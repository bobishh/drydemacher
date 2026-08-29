<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onDestroy } from 'svelte';
  import {
    cancelFemStudy,
    exportFemResultVtu,
    formatBackendError,
    getCachedFemConvergence,
    previewFemMesh,
    runFemConvergence,
    runFemStudy,
    validateFemStudy,
    type FemConvergenceResponse,
    type FemMeshPreviewResponse,
    type FemRunResponse,
    type FemStudyRequest,
    type FemStudyValidationResponse,
  } from './tauri/client';
  import type { FemDisplayOptions, FemFieldKind } from './femDisplay';
  import { safeSaveDialog } from './safeSaveDialog';

  type FemProgress = {
    stage: string;
    elapsedMs: number;
    nodeCount: number | null;
    tet4CellCount: number | null;
    detail: string;
    cancellationBoundary: boolean;
  };

  let {
    modelId,
    source,
    onResultChange,
    onMeshChange,
    onDisplayChange,
  }: {
    modelId: string | null;
    source: string;
    onResultChange?: (result: FemRunResponse | null) => void;
    onMeshChange?: (mesh: FemMeshPreviewResponse | null) => void;
    onDisplayChange?: (display: FemDisplayOptions) => void;
  } = $props();

  const analysisNames = $derived(
    Array.from(source.matchAll(/\(analysis\s+([^\s()]+)/g), (match) => match[1]),
  );
  let selectedAnalysis = $state('');
  let validation = $state<FemStudyValidationResponse | null>(null);
  let result = $state<FemRunResponse | null>(null);
  let resultSource = $state('');
  let meshPreview = $state<FemMeshPreviewResponse | null>(null);
  let meshPreviewSource = $state('');
  let convergence = $state<FemConvergenceResponse | null>(null);
  let meshSizes = $state<[number, number, number]>([4, 2, 1]);
  let progress = $state<FemProgress | null>(null);
  let errorText = $state('');
  let exportEvidence = $state('');
  let runningJobId = $state<string | null>(null);
  let field = $state<FemFieldKind>('vonMises');
  let deformationScale = $state(1);
  let showMesh = $state(true);
  let showOutline = $state(true);
  let clipFraction = $state(1);
  let unlisten: (() => void) | null = null;
  let convergenceRestoreGeneration = 0;
  const resultIsStale = $derived(Boolean(result && resultSource !== source));
  const meshPreviewIsStale = $derived(Boolean(meshPreview && meshPreviewSource !== source));

  $effect(() => {
    if (!analysisNames.includes(selectedAnalysis)) {
      selectedAnalysis = analysisNames[0] ?? '';
      validation = null;
      result = null;
      resultSource = '';
      meshPreview = null;
      meshPreviewSource = '';
      convergence = null;
      onResultChange?.(null);
      onMeshChange?.(null);
    }
  });

  $effect(() => {
    const authored = source.match(/\(volume-mesh\b[^)]*?:size\s+([0-9]+(?:\.[0-9]+)?)mm\b/);
    const size = authored ? Number(authored[1]) : 2;
    if (Number.isFinite(size) && size > 0) meshSizes = [size * 2, size, size / 2];
  });

  $effect(() => {
    const currentModelId = modelId;
    const currentSource = source;
    const currentAnalysis = selectedAnalysis;
    const currentMeshSizes = [...meshSizes];
    const generation = ++convergenceRestoreGeneration;
    convergence = null;
    if (!currentModelId || !currentAnalysis) return;

    void getCachedFemConvergence({
      study: {
        ...request(nextJobId('convergence-cache')),
        modelId: currentModelId,
        source: currentSource,
        analysisName: currentAnalysis,
      },
      meshSizesMm: currentMeshSizes,
      displacementRelativeTolerance: 0.03,
      stressRelativeTolerance: 0.05,
    }).then((cached) => {
      if (generation === convergenceRestoreGeneration) convergence = cached;
    }).catch((error) => {
      if (generation === convergenceRestoreGeneration) errorText = formatBackendError(error);
    });
  });

  $effect(() => {
    onDisplayChange?.({ field, deformationScale, showMesh, showOutline, clipFraction });
  });

  void listen<{ jobId: string; progress: FemProgress }>('fem-progress', ({ payload }) => {
    if (payload.jobId === runningJobId) progress = payload.progress;
  }).then((stop) => { unlisten = stop; });

  onDestroy(() => unlisten?.());

  function request(jobId: string): FemStudyRequest {
    return {
      jobId,
      modelId: modelId ?? '',
      source,
      analysisName: selectedAnalysis,
      budgets: {
        boundaryTriangles: 250_000,
        tet4Cells: 500_000,
        nodes: 150_000,
        dofs: 450_000,
        sparseNonzeros: 30_000_000,
        resultBytes: 128 * 1024 * 1024,
        convergenceLevels: 3,
      },
      control: {
        envelopeMm: 0.1,
        minimumScaledJacobian: 1.0e-6,
        maximumRuntimeMs: 10 * 60 * 1000,
        relativeSolverTolerance: 1.0e-8,
      },
    };
  }

  function nextJobId(kind: string): string {
    return `fem-${kind}-${Date.now()}-${Math.floor(Math.random() * 1_000_000)}`;
  }

  async function validateStudy() {
    errorText = '';
    validation = null;
    try {
      validation = await validateFemStudy(request(nextJobId('validate')));
    } catch (error) {
      errorText = formatBackendError(error);
    }
  }

  async function runStudy() {
    errorText = '';
    result = null;
    onResultChange?.(null);
    const jobId = nextJobId('solve');
    runningJobId = jobId;
    progress = {
      stage: 'resolve', elapsedMs: 0, nodeCount: null, tet4CellCount: null,
      detail: 'Resolving authored study.', cancellationBoundary: true,
    };
    try {
      validation = await validateFemStudy(request(jobId));
      result = await runFemStudy(request(jobId));
      resultSource = source;
      onResultChange?.(result);
    } catch (error) {
      errorText = formatBackendError(error);
    } finally {
      runningJobId = null;
    }
  }

  async function previewMesh() {
    errorText = '';
    meshPreview = null;
    onMeshChange?.(null);
    const jobId = nextJobId('mesh');
    runningJobId = jobId;
    progress = {
      stage: 'resolve', elapsedMs: 0, nodeCount: null, tet4CellCount: null,
      detail: 'Resolving authored study.', cancellationBoundary: true,
    };
    try {
      validation = await validateFemStudy(request(jobId));
      meshPreview = await previewFemMesh(request(jobId));
      meshPreviewSource = source;
      onMeshChange?.(meshPreview);
    } catch (error) {
      errorText = formatBackendError(error);
    } finally {
      runningJobId = null;
    }
  }

  async function runConvergenceStudy() {
    errorText = '';
    convergence = null;
    const jobId = nextJobId('convergence');
    runningJobId = jobId;
    progress = {
      stage: 'resolve', elapsedMs: 0, nodeCount: null, tet4CellCount: null,
      detail: 'Starting coarse-to-fine convergence sequence.', cancellationBoundary: true,
    };
    try {
      convergence = await runFemConvergence({
        study: request(jobId),
        meshSizesMm: [...meshSizes],
        displacementRelativeTolerance: 0.03,
        stressRelativeTolerance: 0.05,
      });
    } catch (error) {
      errorText = formatBackendError(error);
    } finally {
      runningJobId = null;
    }
  }

  function statusLabel(value: string): string {
    return value.replace(/([a-z])([A-Z])/g, '$1 $2').toUpperCase();
  }

  async function cancelStudy() {
    if (!runningJobId) return;
    try {
      await cancelFemStudy(runningJobId);
    } catch (error) {
      errorText = formatBackendError(error);
    }
  }

  async function exportVtu() {
    if (!result || resultIsStale) return;
    errorText = '';
    exportEvidence = '';
    try {
      const targetPath = await safeSaveDialog({
        defaultPath: `${selectedAnalysis || 'fem-result'}.vtu`,
        filters: [{ name: 'VTK Unstructured Grid', extensions: ['vtu'] }],
      });
      if (!targetPath) return;
      const exported = await exportFemResultVtu({
        analysisIdentityDigest: result.analysisIdentityDigest,
        solutionDigest: result.solutionDigest,
        maximumResultBytes: 512 * 1024 * 1024,
      }, targetPath);
      exportEvidence = `VTU ${exported.byteLength.toLocaleString()} B · ${exported.sha256.slice(0, 20)}…`;
    } catch (error) {
      errorText = formatBackendError(error);
    }
  }
</script>

<section class="analysis-panel" data-testid="analysis-panel">
  <header class="analysis-panel__header">
    <div>
      <span class="analysis-panel__eyebrow">NATIVE TET4</span>
      <h2>Structural Analysis</h2>
    </div>
    <span class:analysis-panel__state--ready={Boolean((result?.decisionReady && !resultIsStale) || convergence)} class="analysis-panel__state">
      {runningJobId ? 'RUNNING' : resultIsStale || meshPreviewIsStale ? 'STALE' : result ? (result.decisionReady ? 'READY' : 'REVIEW') : convergence ? 'EVIDENCE' : 'IDLE'}
    </span>
  </header>

  {#if !modelId}
    <p class="analysis-panel__empty">Render a saved model before analysis.</p>
  {:else if analysisNames.length === 0}
    <p class="analysis-panel__empty">No authored <code>(analysis ...)</code> study in current source.</p>
  {:else}
    <label class="analysis-panel__field">
      <span>Study</span>
      <select bind:value={selectedAnalysis} disabled={Boolean(runningJobId)} aria-label="FEM study">
        {#each analysisNames as name}<option value={name}>{name}</option>{/each}
      </select>
    </label>

    <div class="analysis-panel__actions">
      <button type="button" title="Validate units, BRep face tags, domain, loads, supports, and runtime identity without solving" disabled={Boolean(runningJobId)} onclick={validateStudy}>VALIDATE</button>
      <button type="button" title="Generate and inspect the native Tet4 volume mesh, quality, and face-group coverage without solving" disabled={Boolean(runningJobId)} onclick={previewMesh}>MESH PREVIEW</button>
      <button type="button" title="Generate native Tet4 mesh, solve linear statics, verify equilibrium, and publish immutable result arrays" disabled={Boolean(runningJobId)} onclick={runStudy}>MESH + SOLVE</button>
      <button type="button" title="Run three explicit coarse-to-fine meshes and keep displacement and stress convergence decisions separate" disabled={Boolean(runningJobId)} onclick={runConvergenceStudy}>RUN CONVERGENCE</button>
      {#if runningJobId}<button class="analysis-panel__cancel" type="button" title="Request cancellation at next safe pipeline boundary" onclick={cancelStudy}>CANCEL</button>{/if}
    </div>

    {#if progress}
      <div class="analysis-panel__progress" aria-live="polite">
        <strong>{progress.stage.toUpperCase()}</strong>
        <span>{progress.detail}</span>
        <small>{(progress.elapsedMs / 1000).toFixed(1)} s{progress.tet4CellCount ? ` · ${progress.tet4CellCount.toLocaleString()} Tet4` : ''}</small>
      </div>
    {/if}

    {#if errorText}<pre class="analysis-panel__error" role="alert">{errorText}</pre>{/if}
    {#if resultIsStale || meshPreviewIsStale}
      <pre class="analysis-panel__warning" data-testid="fem-stale-warning">SOURCE CHANGED · OLD FEM EVIDENCE IS NOT CURRENT. RUN AGAIN.</pre>
    {/if}

    {#if validation}
      <details open={!result}>
        <summary>DOMAIN · {validation.boundaryTriangleCount.toLocaleString()} SURFACE TRIANGLES</summary>
        <dl>
          <div><dt>Part</dt><dd>{validation.partId}</dd></div>
          <div><dt>Face groups</dt><dd>{validation.faceGroupCount}</dd></div>
          <div><dt>Boundary digest</dt><dd title={validation.boundaryDigest}>{validation.boundaryDigest.slice(0, 20)}…</dd></div>
        </dl>
        {#if validation.decisionReadinessError}<p class="analysis-panel__warning">{validation.decisionReadinessError}</p>{/if}
      </details>
    {/if}

    {#if meshPreview}
      <div class="analysis-panel__mesh" data-testid="fem-mesh-summary">
        <strong>{meshPreview.tet4CellCount.toLocaleString()} TET4</strong>
        <span>{meshPreview.nodeCount.toLocaleString()} NODES · {meshPreview.boundaryTriangleCount.toLocaleString()} SURFACE</span>
        <span>JACOBIAN {meshPreview.minimumScaledJacobian.toPrecision(4)} · RADIUS {meshPreview.minimumRadiusRatio.toPrecision(4)}</span>
        <span>{meshPreview.faceGroupCount} FACE GROUPS · {meshPreview.connectedComponentCount} COMPONENT</span>
      </div>
    {/if}

    <details>
      <summary>CONVERGENCE SIZES</summary>
      <div class="analysis-panel__sizes">
        {#each meshSizes as size, index}
          <label><span>{['COARSE', 'BASE', 'FINE'][index]}</span><input aria-label={`FEM ${['coarse', 'base', 'fine'][index]} mesh size`} type="number" min="0.000001" step="any" bind:value={meshSizes[index]} /> mm</label>
        {/each}
      </div>
    </details>

    {#if convergence}
      <div class="analysis-panel__convergence-status" data-testid="fem-convergence-status">
        <strong>SEQUENCE · {statusLabel(convergence.sequenceStatus)}</strong>
        <strong>DISPLACEMENT · {statusLabel(convergence.displacementStatus)}</strong>
        <strong>STRESS · {statusLabel(convergence.stressStatus)}</strong>
      </div>
      <div class="analysis-panel__table-wrap" data-testid="fem-convergence-table">
        <table>
          <thead><tr><th>SIZE</th><th>STATE</th><th>TET4</th><th>DISP.</th><th>ΔU</th><th>STRESS</th><th>Δσ</th></tr></thead>
          <tbody>
            {#each convergence.levels as level}
              <tr>
                <td>{level.meshSizeMm.toFixed(3)} mm</td>
                <td>
                  {statusLabel(level.status)}
                  {#if level.error}<small class="analysis-panel__level-error">{level.error}</small>{/if}
                </td>
                <td>{level.tet4CellCount?.toLocaleString() ?? '—'}</td>
                <td>{level.maximumDisplacementMm?.toPrecision(4) ?? '—'}</td>
                <td>{level.displacementRelativeDelta === null ? '—' : `${(level.displacementRelativeDelta * 100).toFixed(2)}%`}</td>
                <td>{level.maximumVonMisesMpa?.toPrecision(4) ?? '—'}</td>
                <td>{level.stressRelativeDelta === null ? '—' : `${(level.stressRelativeDelta * 100).toFixed(2)}%`}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    {#if result}
      <div class="analysis-panel__result-actions">
        <button type="button" title="Export current immutable Tet4 mesh, displacement, and stress arrays as VTU" disabled={resultIsStale} onclick={exportVtu}>EXPORT VTU</button>
        {#if exportEvidence}<span data-testid="fem-vtu-evidence">{exportEvidence}</span>{/if}
      </div>
      <div class="analysis-panel__results" data-testid="fem-result-summary">
        <div><span>MAX DISP.</span><strong>{result.summary.maximumDisplacementMm.toPrecision(4)} mm</strong></div>
        <div><span>MAX VON MISES</span><strong>{result.summary.maximumVonMisesMpa.toPrecision(4)} MPa</strong></div>
        <div><span>MIN SAFETY</span><strong>{result.summary.minimumYieldSafetyFactor?.toPrecision(4) ?? '∞'}</strong></div>
        <div><span>MASS</span><strong>{result.summary.massKg.toPrecision(4)} kg</strong></div>
        <div><span>EQUILIBRIUM</span><strong>{result.summary.equilibriumRelativeImbalance.toExponential(2)}</strong></div>
        <div><span>SOLVER RESIDUAL</span><strong>{result.summary.solverRelativeResidual.toExponential(2)}</strong></div>
        <div><span>MESH</span><strong>{result.summary.nodeCount.toLocaleString()} N · {result.summary.tet4CellCount.toLocaleString()} T</strong></div>
      </div>
      {#if result.decisionReadinessError}<pre class="analysis-panel__warning">{result.decisionReadinessError}</pre>{/if}
      <details open>
        <summary>EXTREMA · UNAVERAGED FOR STRESS</summary>
        <dl data-testid="fem-extrema">
          {#each result.summary.extrema as extremum}
            <div>
              <dt>{statusLabel(extremum.fieldKind)}</dt>
              <dd title={`${extremum.coordinateMm.join(', ')} mm · ${extremum.meshContentDigest}`}>
                {extremum.value.toPrecision(5)} {extremum.unit} · {extremum.elementId === null ? `NODE ${extremum.nodeId}` : `ELEMENT ${extremum.elementId}`}
              </dd>
            </div>
          {/each}
        </dl>
      </details>
      <details open={result.acceptanceEvaluations.some((evaluation) => evaluation.status !== 'passed')}>
        <summary>ACCEPTANCE · {result.acceptanceEvaluations.length} AUTHORED METRICS</summary>
        <dl data-testid="fem-acceptance-evidence">
          {#each result.acceptanceEvaluations as evaluation}
            <div>
              <dt>{evaluation.metricId} · {statusLabel(evaluation.status)}</dt>
              <dd title={evaluation.detail}>
                {evaluation.observed === null ? '—' : evaluation.observed.toPrecision(5)} {evaluation.unit} · LIMIT {evaluation.threshold.toPrecision(5)}
                <details class="analysis-panel__trace" data-testid={`fem-acceptance-trace-${evaluation.metricId}`}>
                  <summary>TRACE</summary>
                  <small>RESULT {evaluation.resultDigest}</small>
                  <small>MESH {evaluation.meshContentDigest}</small>
                  <small>GEOMETRY {evaluation.evidenceChain.analysisGeometryDigest}</small>
                  <small>INPUTS {evaluation.evidenceChain.inputEvidenceIds.join(', ') || 'MISSING'}</small>
                  <small>APPLICABILITY {evaluation.evidenceChain.applicabilityCheckIds.join(', ') || 'MISSING'}</small>
                  <small>CONVERGENCE {statusLabel(evaluation.evidenceChain.convergenceStatus ?? 'pending')}</small>
                  <small>SENSITIVITY {evaluation.evidenceChain.sensitivityResultDigests.length ? 'RECORDED' : 'MISSING'}</small>
                  <small>VALIDATION {evaluation.evidenceChain.validationEvidenceIds.join(', ') || 'MISSING'}</small>
                  {#each evaluation.evidenceChain.gaps as gap}<small class="analysis-panel__trace-gap">{gap}</small>{/each}
                </details>
              </dd>
            </div>
          {/each}
        </dl>
      </details>
      <details data-testid="fem-engineering-evidence">
        <summary>ENGINEERING EVIDENCE · {result.engineeringEvidence.verificationLayers.length} LAYERS</summary>
        <div class="analysis-panel__evidence-question">
          <strong>{result.engineeringEvidence.question.statement}</strong>
          <span>{result.engineeringEvidence.question.decision}</span>
        </div>
        <dl>
          {#each result.engineeringEvidence.verificationLayers as layer}
            <div>
              <dt>{statusLabel(layer.layer)} · {statusLabel(layer.status)}</dt>
              <dd title={layer.detail}>{layer.evidenceIds.length ? layer.evidenceIds.join(', ') : '—'}</dd>
            </div>
          {/each}
        </dl>
        <div class="analysis-panel__evidence-subhead">INPUT PROVENANCE</div>
        <dl>
          {#each result.engineeringEvidence.inputs as input}
            <div>
              <dt>{input.inputName} · {statusLabel(input.authority)}</dt>
              <dd title={`${input.evidenceId} · ${input.subject}`}>{input.source}</dd>
            </div>
          {/each}
        </dl>
        <div class="analysis-panel__evidence-subhead">IDEALIZATION</div>
        <dl>
          <div>
            <dt>IDEALIZATION</dt>
            <dd title={`${result.engineeringEvidence.idealization.artifactDigest} · manufacturing ${result.engineeringEvidence.idealization.manufacturingGeometryDigest}`}>
              {result.engineeringEvidence.idealization.kind === 'defeaturedSolid' ? 'DEFEATURED' : 'EXACT'} · {result.engineeringEvidence.idealization.acceptedByUser ? 'ACCEPTED' : 'PENDING'} · {result.engineeringEvidence.idealization.justification}
            </dd>
          </div>
        </dl>
      </details>
      <details>
        <summary>REACTIONS · {result.supportReactions.length} SUPPORT GROUPS</summary>
        <dl>
          {#each result.supportReactions as reaction}
            <div><dt>{reaction.name}</dt><dd>{reaction.resultantN.map(value => value.toPrecision(3)).join(' · ')} N</dd></div>
          {/each}
        </dl>
      </details>
      <details>
        <summary>VIEW · PREVIEW ONLY</summary>
        <div class="analysis-panel__view">
          <label><span>Field</span><select bind:value={field} aria-label="FEM result field"><option value="vonMises">Von Mises stress</option><option value="displacement">Displacement</option></select></label>
          <label><span>Deformation × {deformationScale.toFixed(1)}</span><input aria-label="FEM deformation scale" type="range" min="0" max="20" step="0.5" bind:value={deformationScale} /></label>
          <label><span>Clip {Math.round(clipFraction * 100)}%</span><input aria-label="FEM clip fraction" type="range" min="0.05" max="1" step="0.05" bind:value={clipFraction} /></label>
          <label class="analysis-panel__check"><input type="checkbox" bind:checked={showMesh} /> TET4 EDGES</label>
          <label class="analysis-panel__check"><input type="checkbox" bind:checked={showOutline} /> UNDEFORMED OUTLINE</label>
        </div>
      </details>
    {/if}
  {/if}
</section>

<style>
  .analysis-panel { height: 100%; min-height: 0; overflow: hidden auto; padding: 14px; background: var(--bg-100); color: var(--text); font-family: var(--font-mono); }
  .analysis-panel__header { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; padding-bottom: 12px; overflow: hidden; border-bottom: 1px solid var(--bg-300); }
  .analysis-panel__eyebrow { color: var(--secondary); font-size: .62rem; letter-spacing: .14em; }
  h2 { margin: 3px 0 0; font-size: 1rem; font-weight: 650; letter-spacing: .04em; }
  .analysis-panel__state { padding: 4px 7px; overflow: hidden; border: 1px solid var(--bg-300); color: var(--text-dim); font-size: .62rem; }
  .analysis-panel__state--ready { border-color: var(--primary); color: var(--primary); }
  .analysis-panel__empty { color: var(--text-dim); font-size: .78rem; line-height: 1.5; }
  .analysis-panel__field { display: grid; gap: 5px; margin: 14px 0 10px; color: var(--text-dim); font-size: .66rem; letter-spacing: .08em; text-transform: uppercase; }
  select, button { border: 1px solid var(--bg-300); border-radius: 0; background: var(--bg-200); color: var(--text); font: inherit; }
  select { min-width: 0; padding: 8px; }
  .analysis-panel__actions { display: flex; flex-wrap: wrap; gap: 7px; overflow: hidden; }
  button { padding: 8px 10px; cursor: pointer; color: var(--primary); font-size: .68rem; letter-spacing: .06em; }
  button:hover:not(:disabled), button:focus-visible { border-color: var(--primary); }
  button:disabled { cursor: not-allowed; opacity: .45; }
  .analysis-panel__cancel { color: var(--secondary); }
  .analysis-panel__progress, details, .analysis-panel__results { margin-top: 12px; overflow: hidden; border: 1px solid var(--bg-300); background: color-mix(in srgb, var(--bg-200) 78%, transparent); }
  .analysis-panel__mesh { display: grid; gap: 5px; margin-top: 12px; padding: 10px; overflow: hidden; border: 1px solid var(--primary); background: color-mix(in srgb, var(--primary) 7%, var(--bg-100)); font-size: .65rem; }
  .analysis-panel__mesh strong { color: var(--primary); }
  .analysis-panel__mesh span { overflow: hidden; color: var(--text-dim); text-overflow: ellipsis; white-space: nowrap; }
  .analysis-panel__sizes { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 6px; padding: 0 10px 10px; overflow: hidden; }
  .analysis-panel__sizes label { display: grid; grid-template-columns: 1fr auto; gap: 4px; min-width: 0; color: var(--text-dim); font-size: .58rem; }
  .analysis-panel__sizes label span { grid-column: 1 / -1; }
  .analysis-panel__sizes input { min-width: 0; width: 100%; border: 1px solid var(--bg-300); border-radius: 0; background: var(--bg-100); color: var(--text); font: inherit; }
  .analysis-panel__convergence-status { display: grid; gap: 5px; margin-top: 12px; padding: 9px; overflow: hidden; border: 1px solid var(--bg-300); color: var(--secondary); font-size: .62rem; }
  .analysis-panel__table-wrap { margin-top: 6px; overflow: auto; border: 1px solid var(--bg-300); }
  table { width: 100%; border-collapse: collapse; font-size: .58rem; white-space: nowrap; }
  th, td { padding: 6px; border-right: 1px solid var(--bg-300); border-bottom: 1px solid var(--bg-300); text-align: right; }
  th:first-child, td:first-child { text-align: left; }
  th { color: var(--text-dim); font-weight: 500; }
  .analysis-panel__level-error { display: block; max-width: 24rem; overflow: hidden; color: var(--secondary); text-align: left; text-overflow: ellipsis; white-space: nowrap; }
  .analysis-panel__progress { display: grid; gap: 4px; padding: 10px; font-size: .7rem; }
  .analysis-panel__progress strong { color: var(--primary); }
  .analysis-panel__progress small { color: var(--text-dim); }
  details summary { padding: 8px 10px; cursor: pointer; color: var(--secondary); font-size: .66rem; letter-spacing: .05em; }
  dl { margin: 0; padding: 0 10px 10px; }
  dl div { display: flex; justify-content: space-between; gap: 10px; padding-top: 5px; overflow: hidden; }
  dt { color: var(--text-dim); } dd { margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .analysis-panel__results { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .analysis-panel__result-actions { display: flex; align-items: center; gap: 8px; margin-top: 12px; overflow: hidden; }
  .analysis-panel__result-actions span { overflow: hidden; color: var(--text-dim); font-size: .58rem; text-overflow: ellipsis; white-space: nowrap; }
  .analysis-panel__results div { display: grid; gap: 4px; min-width: 0; padding: 10px; overflow: hidden; border-right: 1px solid var(--bg-300); border-bottom: 1px solid var(--bg-300); }
  .analysis-panel__results span { color: var(--text-dim); font-size: .58rem; letter-spacing: .07em; }
  .analysis-panel__results strong { overflow: hidden; color: var(--text); font-size: .74rem; text-overflow: ellipsis; white-space: nowrap; }
  .analysis-panel__error, .analysis-panel__warning { margin: 12px 0 0; padding: 9px; overflow: auto; border: 1px solid var(--secondary); background: color-mix(in srgb, var(--secondary) 8%, var(--bg-100)); color: var(--secondary); font: .67rem/1.45 var(--font-mono); white-space: pre-wrap; }
  .analysis-panel__view { display: grid; gap: 9px; padding: 0 10px 10px; overflow: hidden; }
  .analysis-panel__view label { display: grid; gap: 4px; min-width: 0; color: var(--text-dim); font-size: .62rem; }
  .analysis-panel__view input[type='range'] { width: 100%; accent-color: var(--primary); }
  .analysis-panel__view .analysis-panel__check { grid-template-columns: auto 1fr; align-items: center; color: var(--text); }
  .analysis-panel__evidence-question { display: grid; gap: 4px; padding: 0 10px 8px; overflow: hidden; }
  .analysis-panel__evidence-question strong { color: var(--text); font-size: .7rem; }
  .analysis-panel__evidence-question span, .analysis-panel__evidence-subhead { color: var(--text-dim); font-size: .6rem; }
  .analysis-panel__evidence-subhead { padding: 8px 10px 0; border-top: 1px solid var(--bg-300); letter-spacing: .07em; }
  .analysis-panel__trace { margin: 5px 0 0; border: 0; background: transparent; text-align: left; white-space: normal; }
  .analysis-panel__trace summary { padding: 2px 0; color: var(--primary); font-size: .58rem; }
  .analysis-panel__trace small { display: block; overflow: hidden; color: var(--text-dim); font-size: .54rem; text-overflow: ellipsis; white-space: nowrap; }
  .analysis-panel__trace .analysis-panel__trace-gap { color: var(--secondary); }
</style>
