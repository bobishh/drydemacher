import { expect, test, type Page } from '@playwright/test';

const source = `(model
  (tag-face mounting :faces "bottom" bracket)
  (tag-face load-pad :faces "top" bracket)
  (part bracket (box 10 10 10))
  (analysis bracket-static
    (linear-static :part bracket)
    (material aluminum :young-modulus 68900MPa :poisson-ratio 0.33 :density 2700kg-per-m3 :yield-strength 276MPa)
    (volume-mesh :element tet4 :size 2mm)
    (fixed :faces (tag mounting))
    (surface-force :faces (tag load-pad) :total [0N 0N -100N])
    (solve :method sparse-direct)))`;

const stl = `solid bracket
facet normal 0 0 1
outer loop
vertex 0 0 0
vertex 10 0 0
vertex 0 10 0
endloop
endfacet
endsolid bracket`;

function float64Le(values: number[]): Buffer {
  const buffer = Buffer.alloc(values.length * 8);
  values.forEach((value, index) => buffer.writeDoubleLE(value, index * 8));
  return buffer;
}

function uint32Le(values: number[]): Buffer {
  const buffer = Buffer.alloc(values.length * 4);
  values.forEach((value, index) => buffer.writeUInt32LE(value, index * 4));
  return buffer;
}

type FemFailureMode = 'runtime-unavailable' | 'invalid-mesh' | 'cancelled' | 'corrupt-result';

async function installFemFixture(
  page: Page,
  failure = false,
  convergenceFailure = false,
  failureMode?: FemFailureMode,
) {
  const arrays = new Map<string, Buffer>([
    ['nodes.bin', float64Le([0, 0, 0, 10, 0, 0, 0, 10, 0, 0, 0, 10])],
    ['boundary.bin', uint32Le([1, 2, 3, 0, 3, 2, 0, 1, 3, 0, 2, 1])],
    ['displacement.bin', float64Le([0, 0, 0, 0.01, 0, -0.02, 0, 0.01, -0.03, 0, 0, -0.04])],
    ['stress.bin', float64Le([0, 12, 31, 48])],
  ]);
  await page.route(/.*\/mock\/(?:preview\.stl|fem\/[^?]+).*/, async (route) => {
    const url = route.request().url();
    if (url.includes('preview.stl')) {
      await route.fulfill({ status: 200, contentType: 'model/stl', body: stl });
      return;
    }
    const name = [...arrays.keys()].find((candidate) => url.includes(candidate));
    await route.fulfill(name
      ? { status: 200, contentType: 'application/octet-stream', body: arrays.get(name)! }
      : { status: 404, body: 'missing FEM fixture' });
  });
  await page.addInitScript(({ source, failure, convergenceFailure, failureMode }) => {
    let cancellationRequested = false;
    const artifactBundle = {
      modelId: 'fem-bracket', sourceKind: 'generated', engineKind: 'ecky', sourceLanguage: 'ecky', geometryBackend: 'mesh',
      contentHash: 'sha256:artifact', artifactVersion: 1, fcstdPath: '', manifestPath: '/mock/manifest.json',
      previewStlPath: '/mock/preview.stl', viewerAssets: [], exportArtifacts: [],
    };
    const modelManifest = {
      schemaVersion: 1, modelId: 'fem-bracket', sourceKind: 'generated', engineKind: 'ecky', sourceLanguage: 'ecky', geometryBackend: 'mesh',
      sourceDigest: 'sha256:source', document: { documentName: 'Bracket', documentLabel: 'Bracket', objectCount: 1, warnings: [] },
      parts: [], parameterGroups: [], controlPrimitives: [], controlRelations: [], controlViews: [], selectionTargets: [],
      advisories: [], measurementAnnotations: [], warnings: [], enrichmentState: { status: 'none', proposals: [] },
    };
    const design = {
      title: 'Bracket', versionName: 'FEM', response: '', interactionMode: 'design', macroCode: source,
      sourceLanguage: 'ecky', geometryBackend: 'mesh', engineKind: 'ecky', uiSpec: { fields: [] }, initialParams: {}, postProcessing: null,
    };
    const message = { id: 'fem-message', role: 'assistant', content: 'Bracket', status: 'success', timestamp: 1, output: design, artifactBundle, modelManifest };
    const validation = {
      jobId: 'validate', modelId: 'fem-bracket', analysisName: 'bracket-static', partId: 'bracket',
      sourceDigest: 'sha256:source', sourceGeometryDigest: 'sha256:geometry', boundaryDigest: 'sha256:boundary',
      boundaryNodeCount: 8, boundaryTriangleCount: 12, faceGroupCount: 6, decisionReadinessError: null,
    };
    const meshPreview = {
      jobId: 'mesh', modelId: 'fem-bracket', analysisName: 'bracket-static', analysisIdentityDigest: 'sha256:analysis',
      meshContentDigest: 'sha256:mesh', sourceBoundaryDigest: 'sha256:boundary', manifestPath: '/mock/fem/mesh-manifest.json',
      arrays: [
        { name: 'nodesMm', path: '/mock/fem/nodes.bin', scalarType: 'float64Le', shape: [4, 3], byteLength: 96, sha256: 'sha256:nodes' },
        { name: 'boundaryTriangles', path: '/mock/fem/boundary.bin', scalarType: 'uint32Le', shape: [4, 3], byteLength: 48, sha256: 'sha256:boundary-array' },
      ],
      nodeCount: 4, tet4CellCount: 1, boundaryTriangleCount: 4, faceGroupCount: 6,
      minimumScaledJacobian: 0.21, minimumRadiusRatio: 0.18, connectedComponentCount: 1,
      boundaryAreaMm2ByGroup: [100, 100, 100, 100, 100, 100],
    };
    const result = {
      jobId: 'solve', modelId: 'fem-bracket', analysisName: 'bracket-static', analysisIdentityDigest: 'sha256:analysis',
      solutionDigest: 'sha256:solution', resultDigest: 'sha256:result', meshContentDigest: 'sha256:mesh',
      sourceDigest: 'sha256:source', sourceBoundaryDigest: 'sha256:boundary', decisionReady: true, decisionReadinessError: null,
      manifestPath: '/mock/fem/manifest.json',
      arrays: [
        { name: 'nodesMm', path: '/mock/fem/nodes.bin', scalarType: 'float64Le', shape: [4, 3], byteLength: 96, sha256: 'sha256:nodes' },
        { name: 'boundaryTriangles', path: '/mock/fem/boundary.bin', scalarType: 'uint32Le', shape: [4, 3], byteLength: 48, sha256: 'sha256:boundary-array' },
        { name: 'displacementMm', path: '/mock/fem/displacement.bin', scalarType: 'float64Le', shape: [4, 3], byteLength: 96, sha256: 'sha256:displacement' },
        { name: 'nodalDisplayVonMisesMpa', path: '/mock/fem/stress.bin', scalarType: 'float64Le', shape: [4], byteLength: 32, sha256: 'sha256:stress' },
      ],
      summary: { maximumDisplacementMm: 0.04, maximumVonMisesMpa: 48, maximumPrincipalStressMpa: 51,
        volumeMm3: 1000, massKg: 0.0027, minimumYieldSafetyFactor: 5.75, equilibriumRelativeImbalance: 2e-12,
        solverRelativeResidual: 3e-13,
        minimumScaledJacobian: 0.21, nodeCount: 4, tet4CellCount: 1,
        extrema: [
          { fieldKind: 'displacementMagnitude', value: 0.04, unit: 'mm', nodeId: 3, elementId: null, coordinateMm: [0, 0, 10], meshContentDigest: 'sha256:mesh', sourceBoundaryDigest: 'sha256:boundary' },
          { fieldKind: 'vonMisesStress', value: 48, unit: 'MPa', nodeId: null, elementId: 0, coordinateMm: [2.5, 2.5, 2.5], meshContentDigest: 'sha256:mesh', sourceBoundaryDigest: 'sha256:boundary' },
          { fieldKind: 'principalStressMaximum', value: 51, unit: 'MPa', nodeId: null, elementId: 0, coordinateMm: [2.5, 2.5, 2.5], meshContentDigest: 'sha256:mesh', sourceBoundaryDigest: 'sha256:boundary' },
        ] },
      supportReactions: [{ name: 'mounting', faceGroupIndices: [0], resultantN: [0, 0, 100] }],
      engineeringEvidence: {
        question: { questionId: 'bracket-strength', statement: 'Does bracket remain elastic?', decision: 'accept or revise', acceptanceMetricIds: ['stress-limit'] },
        idealization: { artifactDigest: 'sha256:idealization', kind: 'exactSolid', sourceGeometryDigest: 'sha256:geometry', analysisGeometryDigest: 'sha256:geometry', manufacturingGeometryDigest: 'sha256:geometry', affectedTopologyIds: [], justification: 'Exact connected solid.', expectedInfluencePercent: 0, acceptedByUser: true },
        inputs: [
          { inputName: 'aluminum', evidenceId: 'material-source', subject: 'material', source: 'qualified material record', authority: 'recordedSource', uncertaintyPercent: 0, decisionCritical: true },
          { inputName: 'applied-load', evidenceId: 'load-source', subject: 'load', source: 'accepted load case', authority: 'userAccepted', uncertaintyPercent: 0, decisionCritical: true },
          { inputName: 'mounting', evidenceId: 'support-source', subject: 'support', source: 'accepted fixture', authority: 'userAccepted', uncertaintyPercent: 0, decisionCritical: true },
        ],
        assumptions: [{ assumptionId: 'small-strain', category: 'physics', statement: 'Small displacement linear elasticity.', status: 'accepted', evidenceIds: ['material-source', 'load-source', 'support-source'] }],
        applicability: [{ checkId: 'elastic-range', kind: 'elasticRange', status: 'pass', observed: 0.174, limit: 1, unit: 'yieldRatio', evidenceIds: [], detail: 'Within declared elastic range.' }],
        sensitivity: null,
        validationEvidence: [{ validationId: 'calc-bracket-v1', kind: 'differentialSolver', source: 'offline CalculiX 2.21 golden', resultDigest: 'sha256:calculix' }],
        verificationLayers: [
          { layer: 'analyticalUnit', status: 'passed', evidenceIds: ['tet4-patch-suite'], detail: 'Tet4 analytical/unit proof passed.' },
          { layer: 'differentialSolver', status: 'passed', evidenceIds: ['calc-bracket-v1'], detail: 'Independent solver comparison recorded.' },
          { layer: 'meshConvergence', status: 'pending', evidenceIds: [], detail: 'No current convergence sequence is attached.' },
          { layer: 'physicalReference', status: 'missing', evidenceIds: [], detail: 'No physical or qualified-reference validation is recorded.' },
        ],
      },
      acceptanceEvaluations: [{ studyName: 'bracket-static', metricId: 'stress-limit', field: 'von-mises-stress', status: 'passed', observed: 48, unit: 'MPa', threshold: 200, comparison: 'lessThanOrEqual', meshSizeMm: 2, nodeId: null, elementId: 0, coordinateMm: [2.5, 2.5, 2.5], analysisIdentityDigest: 'sha256:analysis', meshContentDigest: 'sha256:mesh', resultDigest: 'sha256:result', convergenceStatus: null,
        evidenceChain: { sourceGeometryDigest: 'sha256:geometry', analysisGeometryDigest: 'sha256:geometry', idealizationAccepted: true, inputEvidenceIds: ['material-source', 'load-source', 'support-source'], applicabilityCheckIds: ['elastic-range'], convergenceStatus: null, sensitivityResultDigests: [], validationEvidenceIds: ['calc-bracket-v1'], gaps: ['mesh convergence is not attached'] },
        detail: 'FEM acceptance metric stress-limit passed.' }],
    };
    const convergence = {
      jobId: 'convergence', modelId: 'fem-bracket', analysisName: 'bracket-static',
      sequenceStatus: 'completed',
      displacementStatus: 'converged', stressStatus: 'suspectedSingularity',
      acceptanceEvaluations: [],
      levels: [
        { meshSizeMm: 4, status: 'completed', error: null, analysisIdentityDigest: 'sha256:a1', solutionDigest: 'sha256:s1', resultDigest: 'sha256:r1', meshContentDigest: 'sha256:m1', nodeCount: 40, tet4CellCount: 90, minimumScaledJacobian: 0.18, equilibriumRelativeImbalance: 2e-10, solverRelativeResidual: 1e-11, maximumDisplacementMm: 0.035, maximumVonMisesMpa: 40, displacementRelativeDelta: null, stressRelativeDelta: null },
        { meshSizeMm: 2, status: 'completed', error: null, analysisIdentityDigest: 'sha256:a2', solutionDigest: 'sha256:s2', resultDigest: 'sha256:r2', meshContentDigest: 'sha256:m2', nodeCount: 120, tet4CellCount: 360, minimumScaledJacobian: 0.17, equilibriumRelativeImbalance: 4e-11, solverRelativeResidual: 2e-12, maximumDisplacementMm: 0.039, maximumVonMisesMpa: 46, displacementRelativeDelta: 0.1026, stressRelativeDelta: 0.1304 },
        { meshSizeMm: 1, status: 'completed', error: null, analysisIdentityDigest: 'sha256:a3', solutionDigest: 'sha256:s3', resultDigest: 'sha256:r3', meshContentDigest: 'sha256:m3', nodeCount: 430, tet4CellCount: 1440, minimumScaledJacobian: 0.16, equilibriumRelativeImbalance: 8e-12, solverRelativeResidual: 5e-13, maximumDisplacementMm: 0.04, maximumVonMisesMpa: 55, displacementRelativeDelta: 0.025, stressRelativeDelta: 0.1636 },
      ],
    };
    (window as any).__FEM_CALLS__ = [];
    window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
    (window.__TAURI_INTERNALS__ as any).convertFileSrc = (path: string) => path;
    window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: Record<string, any>) => {
      (window as any).__FEM_CALLS__.push({ cmd, args });
      if (cmd === 'get_config') return { engines: [], selectedEngineId: '', freecadCmd: '', assets: [], microwave: { muted: true }, voice: { sttLanguageCode: 'en-US' },
        mcp: { port: null, maxSessions: null, mode: 'passive', primaryAgentId: null, promptTimeoutSecs: 1800, eckyAstAuthoring: false, autoAgents: [] },
        hasSeenOnboarding: true, connectionType: 'mcp', defaultEngineKind: 'ecky', defaultSourceLanguage: 'ecky', defaultGeometryBackend: 'mesh', maxGenerationAttempts: 1, maxVerifyAttempts: 0 };
      if (cmd === 'get_runtime_capabilities') return { freecad: { available: false, detail: 'missing', path: null }, build123d: { available: false, detail: 'missing', path: null }, mesh: { available: true, detail: 'bundled', path: null }, recommendedAuthoringContext: { engineKind: 'ecky', sourceLanguage: 'ecky', geometryBackend: 'mesh' } };
      if (cmd === 'get_history') return [{ id: 'fem-thread', title: 'Bracket', summary: '', messages: [], updatedAt: 1, versionCount: 1, pendingCount: 0, queuedCount: 0, errorCount: 0, status: 'active', engineKind: 'ecky', sourceLanguage: 'ecky', geometryBackend: 'mesh' }];
      if (cmd === 'get_last_design') return { design, threadId: 'fem-thread', messageId: 'fem-message', artifactBundle, modelManifest, selectedPartId: null };
      if (cmd === 'get_thread_latest_version' || cmd === 'get_thread_message_version') return message;
      if (cmd === 'get_thread_messages_page') return { messages: [message], nextBefore: null, hasMore: false };
      if (cmd === 'get_default_macro') return source;
      if (cmd === 'get_thread_agent_state') return { connectionState: 'disconnected', sessions: [], primaryAgentLabel: null, statusText: '', phase: null, busy: false };
      if (cmd === 'get_active_agent_sessions' || cmd === 'get_agent_terminal_snapshots' || cmd === 'list_installed_component_package_headers') return [];
      if (cmd === 'get_thread_window_layout') return null;
      if (cmd === 'save_thread_window_layout') return null;
      if (cmd === 'plugin:fs|exists') return true;
      if (cmd === 'plugin:fs|size') return 1024;
      if (cmd === 'render_model') return { ...artifactBundle, modelId: 'fem-bracket-edited', contentHash: 'sha256:artifact-edited' };
      if (cmd === 'get_model_manifest') return { ...modelManifest, modelId: args?.modelId ?? 'fem-bracket-edited', sourceDigest: 'sha256:source-edited' };
      if (cmd === 'verify_generated_model') return { passed: true, summary: 'Checks passed.', issues: [], metrics: { partCount: 1 }, verifierStatus: 'ok', verifierSource: 'mock' };
      if (cmd === 'verify_render') return { passed: true, summary: 'Visual checks passed.', issues: [], usage: null };
      if (cmd === 'save_model_manifest' || cmd === 'save_last_design' || cmd === 'update_version_runtime') return null;
      if (cmd === 'validate_fem_study') return { ...validation, jobId: args?.request?.jobId ?? 'validate' };
      if (cmd === 'preview_fem_mesh') {
        if (failureMode === 'runtime-unavailable') throw { code: 'notFound', message: 'Pinned fTetWild runtime unavailable', details: 'runtime manifest missing at bundled runtime root.' };
        if (failureMode === 'invalid-mesh') throw { code: 'validation', message: 'fTetWild volume mesh rejected', details: 'cell 17 has non-positive signed volume; no artifact published.' };
        return { ...meshPreview, jobId: args?.request?.jobId ?? 'mesh' };
      }
      if (cmd === 'run_fem_study') {
        await new Promise((resolve) => setTimeout(resolve, 180));
        if (failure) throw { code: 'validation', message: 'FEM linear-static solve failed', details: 'Faer sparse Cholesky rejected singular matrix; constraint rank 5/6.' };
        if (failureMode === 'cancelled' && cancellationRequested) throw { code: 'conflict', message: "FEM study 'bracket-static' was cancelled at factorization boundary", details: 'No result artifact was published.' };
        if (failureMode === 'corrupt-result') return {
          ...result,
          arrays: result.arrays.map((array) => array.name === 'nodalDisplayVonMisesMpa'
            ? { ...array, path: '/mock/fem/corrupt.bin' }
            : array),
          jobId: args?.request?.jobId ?? 'solve',
        };
        return { ...result, jobId: args?.request?.jobId ?? 'solve' };
      }
      if (cmd === 'safe_save_dialog') return '/tmp/bracket-static.vtu';
      if (cmd === 'export_fem_result_vtu') return { path: args?.targetPath, byteLength: 2048, sha256: 'sha256:vtu-export', resultDigest: 'sha256:result' };
      if (cmd === 'run_fem_convergence') {
        if (convergenceFailure) return {
          ...convergence,
          jobId: args?.request?.study?.jobId ?? 'convergence',
          sequenceStatus: 'failed', displacementStatus: 'failed', stressStatus: 'failed',
          levels: [
            convergence.levels[0],
            { meshSizeMm: 2, status: 'failed', error: 'Faer factorization failed at refinement level 2', analysisIdentityDigest: null, solutionDigest: null, resultDigest: null, meshContentDigest: null, nodeCount: null, tet4CellCount: null, minimumScaledJacobian: null, equilibriumRelativeImbalance: null, solverRelativeResidual: null, maximumDisplacementMm: null, maximumVonMisesMpa: null, displacementRelativeDelta: null, stressRelativeDelta: null },
          ],
        };
        return { ...convergence, jobId: args?.request?.study?.jobId ?? 'convergence' };
      }
      if (cmd === 'cancel_fem_study') {
        cancellationRequested = true;
        return { jobId: args?.jobId, cancellationRequested: true };
      }
      return null;
    };
  }, { source, failure, convergenceFailure, failureMode });
}

test('Given a model contains an analysis When ordinary preview opens Then no FEM worker operation starts', async ({ page }) => {
  await installFemFixture(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  const calls = await page.evaluate(() => (window as any).__FEM_CALLS__ as Array<{ cmd: string }>);
  expect(calls.filter((call) => [
    'validate_fem_study',
    'preview_fem_mesh',
    'run_fem_study',
    'run_fem_convergence',
  ].includes(call.cmd))).toEqual([]);
});

test('Given a valid authored study When mesh preview runs Then real Tet4 boundary appears without solving', async ({ page }) => {
  await installFemFixture(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'MESH PREVIEW' }).click();
  await expect(page.getByTestId('fem-mesh-summary')).toContainText('1 TET4');
  await expect(page.locator('.viewer-host').first()).toHaveAttribute('data-fem-mesh-overlay-visible', 'true');
  const calls = await page.evaluate(() => (window as any).__FEM_CALLS__);
  expect(calls.some((call: { cmd: string }) => call.cmd === 'preview_fem_mesh')).toBe(true);
  expect(calls.some((call: { cmd: string }) => call.cmd === 'run_fem_study')).toBe(false);
});

test('Given authored face tags and linear-static study When native FEM runs Then verified summary and preview-only field appear', async ({ page }) => {
  await installFemFixture(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await expect(page.getByLabel('FEM study')).toHaveValue('bracket-static');
  await page.getByRole('button', { name: 'MESH + SOLVE' }).click();
  await expect(page.getByText('RUNNING', { exact: true })).toBeVisible();
  await expect(page.getByTestId('fem-result-summary')).toContainText('48.00 MPa');
  await expect(page.getByTestId('fem-extrema')).toContainText('ELEMENT 0');
  await expect(page.getByTestId('fem-acceptance-evidence')).toContainText('stress-limit · PASSED');
  await expect(page.getByTestId('fem-result-legend')).toContainText('PREVIEW ONLY');
  await expect(page.locator('.viewer-host').first()).toHaveAttribute('data-fem-overlay-visible', 'true');
});

test('Given numerical solve succeeds When engineering evidence opens Then verification layers and input provenance stay separate', async ({ page }) => {
  await installFemFixture(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'MESH + SOLVE' }).click();
  const evidence = page.getByTestId('fem-engineering-evidence');
  await expect(evidence).toContainText('Does bracket remain elastic?');
  await expect(evidence).toContainText('DIFFERENTIAL SOLVER · PASSED');
  await expect(evidence).toContainText('MESH CONVERGENCE · PENDING');
  await expect(evidence).toContainText('PHYSICAL REFERENCE · MISSING');
  await expect(evidence).toContainText('IDEALIZATION EXACT · ACCEPTED');
  await expect(evidence).toContainText('accepted load case');
  await expect(evidence).not.toContainText('SAFE');
  await page.getByText('ACCEPTANCE · 1 AUTHORED METRICS').click();
  await page.getByTestId('fem-acceptance-trace-stress-limit').getByText('TRACE').click();
  await expect(page.getByTestId('fem-acceptance-trace-stress-limit')).toContainText('INPUTS material-source, load-source, support-source');
  await expect(page.getByTestId('fem-acceptance-trace-stress-limit')).toContainText('VALIDATION calc-bracket-v1');
});

test('Given a current solved study When VTU export runs Then immutable mesh and field artifact is exported', async ({ page }) => {
  await installFemFixture(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'MESH + SOLVE' }).click();
  await page.getByRole('button', { name: 'EXPORT VTU' }).click();
  await expect(page.getByTestId('fem-vtu-evidence')).toContainText('2,048 B');
  const calls = await page.evaluate(() => (window as any).__FEM_CALLS__ as Array<{ cmd: string }>);
  expect(calls.some((call) => call.cmd === 'export_fem_result_vtu')).toBe(true);
});

test('Given a singular study When solve fails Then raw backend factorization detail remains visible', async ({ page }) => {
  await installFemFixture(page, true);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'MESH + SOLVE' }).click();
  await expect(page.getByRole('alert')).toContainText('Faer sparse Cholesky rejected singular matrix');
  await expect(page.getByTestId('fem-result-summary')).toHaveCount(0);
});

test('Given explicit coarse-to-fine sizes When convergence runs Then displacement and stress stay separate', async ({ page }) => {
  await installFemFixture(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'RUN CONVERGENCE' }).click();
  const table = page.getByTestId('fem-convergence-table');
  await expect(table).toContainText('4.000 mm');
  await expect(table).toContainText('1,440');
  await expect(page.getByTestId('fem-convergence-status')).toContainText('DISPLACEMENT · CONVERGED');
  await expect(page.getByTestId('fem-convergence-status')).toContainText('STRESS · SUSPECTED SINGULARITY');
});

test('Given refinement level two fails When convergence returns Then level one evidence and raw failure remain visible', async ({ page }) => {
  await installFemFixture(page, false, true);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'RUN CONVERGENCE' }).click();
  await expect(page.getByTestId('fem-convergence-status')).toContainText('SEQUENCE · FAILED');
  const table = page.getByTestId('fem-convergence-table');
  await expect(table).toContainText('90');
  await expect(table).toContainText('Faer factorization failed at refinement level 2');
});

test('Given a solved study When source changes Then old FEM evidence is visibly stale and leaves viewport', async ({ page }) => {
  await installFemFixture(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'MESH + SOLVE' }).click();
  await expect(page.locator('.viewer-host').first()).toHaveAttribute('data-fem-overlay-visible', 'true');

  await page.getByTestId('workbench-bottom-dock').getByRole('button', { name: /CODE/i }).click();
  const editor = page.locator('[data-window-id="code"] .cm-content');
  await editor.click();
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+A' : 'Control+A');
  await page.keyboard.insertText(source.replace('-100N', '-125N'));
  await page.locator('[role="dialog"]').filter({ hasText: 'MACRO INSPECTOR:' }).getByRole('button', { name: 'APPLY', exact: true }).click();
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await expect(page.getByText('STALE', { exact: true })).toBeVisible();
  await expect(page.getByTestId('fem-stale-warning')).toContainText('OLD FEM EVIDENCE IS NOT CURRENT');
  await expect(page.locator('.viewer-host').first()).toHaveAttribute('data-fem-overlay-visible', 'false');
});

test('Given a current result When display controls change Then manufacturing pipeline is untouched', async ({ page }) => {
  await installFemFixture(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'MESH + SOLVE' }).click();
  await expect(page.getByTestId('fem-result-summary')).toBeVisible();
  await page.getByText('VIEW · PREVIEW ONLY').click();
  await page.getByLabel('FEM result field').selectOption('displacement');
  await page.getByLabel('FEM deformation scale').fill('8');
  await page.getByLabel('FEM clip fraction').fill('0.5');
  await page.getByText('TET4 EDGES', { exact: true }).click();
  await page.getByText('UNDEFORMED OUTLINE', { exact: true }).click();
  const calls = await page.evaluate(() => (window as any).__FEM_CALLS__ as Array<{ cmd: string }>);
  expect(calls.filter((call) => ['render_model', 'export_file', 'save_model_manifest'].includes(call.cmd))).toEqual([]);
  await expect(page.getByTestId('fem-result-legend')).toContainText('PREVIEW ONLY · EXPORT GEOMETRY UNCHANGED');
});

test('Given fTetWild runtime is unavailable When mesh preview starts Then raw runtime detail remains visible', async ({ page }) => {
  await installFemFixture(page, false, false, 'runtime-unavailable');
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'MESH PREVIEW' }).click();
  await expect(page.getByRole('alert')).toContainText('Pinned fTetWild runtime unavailable');
  await expect(page.getByRole('alert')).toContainText('runtime manifest missing');
});

test('Given fTetWild returns an invalid volume mesh When preview runs Then raw mesh invariant remains visible', async ({ page }) => {
  await installFemFixture(page, false, false, 'invalid-mesh');
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'MESH PREVIEW' }).click();
  await expect(page.getByRole('alert')).toContainText('fTetWild volume mesh rejected');
  await expect(page.getByRole('alert')).toContainText('cell 17 has non-positive signed volume');
});

test('Given a study is running When cancellation is requested Then cancelled backend detail replaces pending state', async ({ page }) => {
  await installFemFixture(page, false, false, 'cancelled');
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'MESH + SOLVE' }).click();
  await expect(page.getByText('RUNNING', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'CANCEL' }).click();
  await expect(page.getByRole('alert')).toContainText("FEM study 'bracket-static' was cancelled at factorization boundary");
  await expect(page.getByTestId('fem-result-summary')).toHaveCount(0);
});

test('Given a published result references a corrupt array When overlay loads Then exact artifact failure is visible', async ({ page }) => {
  await installFemFixture(page, false, false, 'corrupt-result');
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Structural analysis' }).click();
  await page.getByRole('button', { name: 'MESH + SOLVE' }).click();
  await expect(page.getByRole('alert')).toContainText("FEM array '/mock/fem/corrupt.bin' returned HTTP 404");
  await expect(page.locator('.viewer-host').first()).toHaveAttribute('data-fem-overlay-visible', 'false');
});
