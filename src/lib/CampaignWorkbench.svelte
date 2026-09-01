<script lang="ts">
  import CodePanel from './CodePanel.svelte';
  import Viewer from './Viewer.svelte';
  import { toPreviewSrc } from './previewSource';
  import { formatBackendError, renderModel } from './tauri/client';
  import { renderMarkdownFragment } from './docs/eckyIrGuide';
  import type { ArtifactBundle } from './types/domain';
  import type { CampaignRun } from './projects/campaignRunClient';
  import type { CampaignCurrentStepPayload } from './projects/campaignDefinitionClient';
  import type { CampaignRunTransitionAction, TransitionCampaignRunResult } from './tauri/contracts';
  import { createCampaignPreviewCache } from './projects/campaignPreviewCache';

  let {
    campaign,
    render = renderModel,
    run = $bindable(),
    onTransition,
    onClose,
  }: {
    /** Backend-projected current step. No campaign corpus is imported by this component. */
    campaign: CampaignCurrentStepPayload;
    render?: typeof renderModel;
    run: CampaignRun;
    onTransition: (action: CampaignRunTransitionAction) => Promise<TransitionCampaignRunResult>;
    onClose?: () => void;
  } = $props();

  const editedPreviewCache = createCampaignPreviewCache();
  let draft = $state('');
  let loadedDraftKey = $state<string | null>(null);
  let draftSaveTimer: ReturnType<typeof setTimeout> | null = null;
  let renderToken = 0;
  let artifact = $state<ArtifactBundle | null>(null);
  let artifactSource = $state<string | null>(null);
  let renderState = $state<'idle' | 'rendering' | 'ready' | 'error'>('idle');
  let renderError = $state('');
  let checkState = $state<'idle' | 'checking' | 'pass' | 'fail' | 'error'>('idle');
  let checkDetail = $state('');
  let transitionPending = $state(false);
  let transitionError = $state('');

  const active = $derived(campaign.currentStep);
  const activeSource = $derived(active?.source ?? null);
  const activeDraftKey = $derived(active?.source && active.canonicalSourceDigest
    ? `${campaign.definitionId}/${active.id}@${active.canonicalSourceDigest}`
    : null);
  const activeProse = $derived(active ? renderMarkdownFragment(active.prose) : '');
  const isStale = $derived(Boolean(activeSource && artifact && artifactSource !== draft));
  const needsRender = $derived(Boolean(activeSource && (!artifact || artifactSource !== draft)));
  const canGoBack = $derived(Boolean(
    active?.previousStep && run.completedStepIds.includes(active.previousStep.id),
  ));

  $effect(() => {
    if (!active) return;
    if (loadedDraftKey !== activeDraftKey) {
      loadedDraftKey = activeDraftKey;
      draft = activeDraftKey ? run.draftOverridesByStepId[activeDraftKey] ?? activeSource ?? '' : '';
      renderToken += 1;
      artifact = active.canonicalPreview?.artifactBundle ?? null;
      artifactSource = active.canonicalPreview ? activeSource : null;
      renderState = artifact ? 'ready' : 'idle';
      renderError = '';
    }
    checkState = 'idle';
    checkDetail = '';
  });

  function previewIdentity(source: string) {
    return {
      source,
      runtimeDigest: active?.canonicalPreview?.runtimeDigest ?? 'campaign-runtime-unavailable',
      backend: 'mesh',
    };
  }

  async function renderActive() {
    if (renderState === 'rendering') return;
    if (!activeSource) {
      renderState = 'error';
      renderError = 'Canonical lesson source is unavailable.';
      return;
    }
    const source = draft;
    const token = ++renderToken;
    renderState = 'rendering';
    renderError = '';

    // Canonical source is already verified and packaged by the backend.
    if (source === activeSource && active?.canonicalPreview) {
      artifact = active.canonicalPreview.artifactBundle;
      artifactSource = source;
      renderState = 'ready';
      return;
    }

    const identity = previewIdentity(source);
    const cached = editedPreviewCache.get(identity);
    if (cached) {
      artifact = cached;
      artifactSource = source;
      renderState = 'ready';
      return;
    }

    try {
      const next = await render(source, {}, 'ecky', 'mesh', null, null);
      if (token !== renderToken) return;
      editedPreviewCache.put(identity, next);
      artifact = next;
      artifactSource = source;
      renderState = 'ready';
    } catch (error) {
      if (token !== renderToken) return;
      renderState = 'error';
      renderError = formatBackendError(error);
    }
  }

  async function transition(action: CampaignRunTransitionAction) {
    transitionError = '';
    const result = await onTransition(action);
    run = result.run as CampaignRun;
    return result;
  }

  function saveDraft(nextDraft: string) {
    if (!active?.source) return;
    if (draftSaveTimer) clearTimeout(draftSaveTimer);
    draftSaveTimer = setTimeout(() => {
      draftSaveTimer = null;
      void transition({ action: 'saveDraft', draft: nextDraft }).catch((error) => {
        transitionError = formatBackendError(error);
      });
    }, 350);
  }

  async function continueStep() {
    if (!active?.nextStepId || transitionPending) return;
    if (draftSaveTimer) clearTimeout(draftSaveTimer);
    draftSaveTimer = null;
    transitionPending = true;
    try {
      await transition({ action: 'continue', draft: activeSource ? draft : null });
    } catch (error) {
      transitionError = formatBackendError(error);
    } finally {
      transitionPending = false;
    }
  }

  async function goBack() {
    if (!active?.previousStep || !canGoBack || transitionPending) return;
    if (draftSaveTimer) clearTimeout(draftSaveTimer);
    draftSaveTimer = null;
    transitionPending = true;
    try {
      await transition({ action: 'back', draft: activeSource ? draft : null });
    } catch (error) {
      transitionError = formatBackendError(error);
    } finally {
      transitionPending = false;
    }
  }

  async function checkSolution() {
    if (!active || active.kind !== 'challenge' || !active.acceptance) return;
    checkState = 'checking';
    checkDetail = '';
    // Backend owns reference source and Core IR evaluation.
    try {
      const result = await transition({ action: 'checkSolution', candidateSource: draft });
      if (!result.check) throw new Error('Campaign check outcome is missing.');
      if (!result.check.matched) {
        checkState = 'fail';
        checkDetail = 'Not the same Core IR yet.';
        return;
      }
      checkState = 'pass';
      checkDetail = 'Core IR matches.';
    } catch (error) {
      checkState = 'error';
      checkDetail = formatBackendError(error);
    }
  }
</script>

<section class="campaign-workbench" aria-label="Ecky build missions">
  {#if active}
    <header class="campaign-workbench__header">
      <div>
        <p class="campaign-workbench__kicker">ECKY IR · CHAPTER {active.missionIndex} / {active.missionCount} · STEP {active.stepIndex}</p>
        <h2>{active.title}</h2>
      </div>
      <div class="campaign-workbench__status" aria-live="polite">{active.stepIndex} / {active.stepCount}</div>
      {#if onClose}<button type="button" class="campaign-workbench__quiet" onclick={onClose}>PROJECTS</button>{/if}
    </header>

    <div class="campaign-workbench__body" class:campaign-workbench__body--explain={active.kind === 'explain'}>
      <div class="campaign-workbench__lesson">
        <div class="campaign-workbench__content">
          <div class="campaign-workbench__prose">{@html activeProse}</div>
          {#if active.kind !== 'explain' && activeSource}
            <div class="campaign-workbench__editor"><CodePanel bind:code={draft} sourceLanguage="ecky" onchange={saveDraft} /></div>
          {:else if active.kind !== 'explain'}
            <pre class="campaign-workbench__error" role="alert">Canonical lesson source is unavailable.</pre>
          {/if}
        </div>

        <div class="campaign-workbench__actions">
          {#if canGoBack}<button type="button" class="campaign-workbench__quiet" onclick={() => void goBack()} disabled={transitionPending}>BACK</button>{/if}
          {#if activeSource}<button type="button" class="campaign-workbench__quiet" onclick={() => void renderActive()} disabled={renderState === 'rendering' || !needsRender}>{renderState === 'rendering' ? 'RENDERING…' : needsRender ? 'RENDER' : 'UP TO DATE'}</button>{/if}
          {#if active.kind === 'challenge'}
            <button type="button" class="campaign-workbench__primary" onclick={() => void checkSolution()} disabled={checkState === 'checking'}>{checkState === 'checking' ? 'CHECKING…' : 'CHECK SOLUTION'}</button>
          {:else if active.nextStepId}
            <button type="button" class="campaign-workbench__primary" onclick={() => void continueStep()} disabled={transitionPending}>{transitionPending ? 'CONTINUING…' : 'CONTINUE'}</button>
          {/if}
        </div>
        {#if active.kind === 'challenge'}<p class:campaign-workbench__result--pass={checkState === 'pass'} class:campaign-workbench__result--fail={checkState === 'fail' || checkState === 'error'} class="campaign-workbench__result" data-testid="mission-acceptance">{checkState === 'idle' ? 'Edit the source, then check its Core IR.' : checkDetail}</p>{/if}
        {#if renderState === 'error'}<pre class="campaign-workbench__error" role="alert">{renderError}</pre>{/if}
        {#if transitionError}<pre class="campaign-workbench__error" role="alert">{transitionError}</pre>{/if}
      </div>

      {#if active.kind !== 'explain'}
        <aside class="campaign-workbench__preview" aria-label="Rendered lesson model">
          {#if artifact}
            {#if isStale}<p class="campaign-workbench__stale" data-testid="campaign-preview-stale">Preview is from last rendered source.</p>{/if}
            {#if renderState === 'rendering'}<p class="campaign-workbench__pending">Rendering edited source…</p>{/if}
            <Viewer modelKey={artifact.modelId} stlUrl={toPreviewSrc(artifact.modelStlPath)} viewerAssets={[]} manifestParts={[]} edgeTargets={[]} faceTargets={[]} selectionTargets={[]} selectedTarget={null} searchQuery="" selectedPartId={null} overlayPartLabel={null} overlayPartEditable={false} overlayPreviewOnly={false} showContextOverlay={false} overlayControls={[]} overlayAdvisories={[]} previewTransforms={{}} viewerMode="orbit" onControlFocusChange={() => {}} onSearchQueryChange={() => {}} onCameraStateChange={() => {}} isGenerating={false} hideModelWhileBusy={false} busyPhase={null} busyText={null} />
          {:else if renderState === 'rendering'}<p>Rendering edited source…</p>
          {:else}<p>Bundled preview unavailable.</p>{/if}
        </aside>
      {/if}
    </div>

  {:else}<p class="campaign-workbench__error">Campaign source is unavailable.</p>{/if}
</section>

<style>
  .campaign-workbench { height:100%; min-height:0; display:grid; grid-template-rows:auto minmax(0,1fr) auto; overflow:hidden; border:1px solid var(--bg-300); background:rgba(10,13,22,.72); }
  .campaign-workbench__header { display:flex; justify-content:space-between; gap:16px; padding:14px; border-bottom:1px solid var(--bg-300); overflow:hidden; }
  .campaign-workbench__kicker,.campaign-workbench__status { color:var(--secondary); font-size:11px; letter-spacing:.08em; } h2 { margin:5px 0; font-size:21px; }
  .campaign-workbench__body { min-height:0; display:grid; grid-template-columns:minmax(0,1fr) minmax(300px,.9fr); overflow:hidden; } .campaign-workbench__body--explain { grid-template-columns:minmax(0,1fr); } .campaign-workbench__body--explain .campaign-workbench__lesson { border-right:0; }
  .campaign-workbench__lesson,.campaign-workbench__preview { min-width:0; min-height:0; padding:14px; overflow:hidden; } .campaign-workbench__lesson { display:grid; grid-template-rows:minmax(0,1fr) auto auto; gap:10px; border-right:1px solid var(--bg-300); } .campaign-workbench__content { min-height:0; overflow:auto; display:grid; align-content:start; gap:14px; }
  .campaign-workbench__editor { height:clamp(280px,48vh,560px); min-height:220px; overflow:hidden; border:1px solid var(--bg-300); } .campaign-workbench__preview { min-height:260px; background:#090c14; position:relative; } .campaign-workbench__preview :global(.viewer-container) { height:100%; }
  .campaign-workbench__prose { color:var(--text); line-height:1.65; max-width:76ch; } .campaign-workbench__prose :global(p) { margin:0 0 10px; } .campaign-workbench__prose :global(ul),.campaign-workbench__prose :global(ol) { margin:0 0 12px; padding-left:22px; } .campaign-workbench__prose :global(code) { color:var(--secondary); }
  .campaign-workbench__actions { position:relative; z-index:3; display:flex; gap:8px; flex-wrap:wrap; } button { border:1px solid var(--bg-300); background:#111524; color:var(--text); padding:9px 12px; font:inherit; font-size:12px; cursor:pointer; } button:hover:not(:disabled) { border-color:var(--secondary); } button:active:not(:disabled) { transform:translateY(1px); } button:disabled { opacity:.45; cursor:not-allowed; } .campaign-workbench__primary { border-color:var(--primary); background:#2d2512; } .campaign-workbench__quiet:not(:disabled) { color:var(--text); }
  .campaign-workbench__result,.campaign-workbench__stale,.campaign-workbench__pending { margin:0; font-size:12px; color:var(--text-dim); } .campaign-workbench__stale { position:absolute; z-index:1; top:14px; left:14px; padding:5px 7px; border:1px solid var(--secondary); background:#16130c; color:var(--secondary); } .campaign-workbench__pending { position:absolute; z-index:1; top:44px; left:14px; }
  .campaign-workbench__result--pass { color:#8fca9a; } .campaign-workbench__result--fail,.campaign-workbench__error { color:#f2a3a3; white-space:pre-wrap; overflow:auto; }
  @media (max-width:720px) { .campaign-workbench__body { grid-template-columns:1fr; grid-template-rows:minmax(310px,1fr) minmax(250px,.8fr); overflow:auto; } .campaign-workbench__lesson { border-right:0; border-bottom:1px solid var(--bg-300); min-height:0; } .campaign-workbench__preview { min-height:250px; } .campaign-workbench__header { overflow:auto; } }
</style>
