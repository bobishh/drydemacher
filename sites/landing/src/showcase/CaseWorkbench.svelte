<script lang="ts">
  import { tick } from 'svelte';
  import StlViewer from '../StlViewer.svelte';
  import ReadOnlyEckySource from './ReadOnlyEckySource.svelte';
  import {
    caseShowcaseVariants,
    currentPatternVariants,
    earlierCaseVariants,
    type CaseShowcaseVariant,
  } from './caseShowcaseManifest';

  let selectedId = $state(currentPatternVariants[0]?.id ?? '');
  let codeOpen = $state(false);
  let copyState = $state<'idle' | 'copied' | 'error'>('idle');
  let codeButton: HTMLButtonElement | null = $state(null);
  let sourceDialog: HTMLDialogElement | null = $state(null);
  let closeButton: HTMLButtonElement | null = $state(null);
  let copyResetTimer: ReturnType<typeof setTimeout> | null = null;
  let sourceText = $state('');
  let sourceStatus = $state<'idle' | 'loading' | 'ready' | 'error'>('idle');
  let sourceError = $state('');
  let sourceRequest = 0;

  const selected = $derived(
    caseShowcaseVariants.find((variant) => variant.id === selectedId) ?? currentPatternVariants[0],
  );

  $effect(() => {
    if (!codeOpen) return;
    const previousOverflow = document.documentElement.style.overflow;
    document.documentElement.style.overflow = 'hidden';
    return () => {
      document.documentElement.style.overflow = previousOverflow;
    };
  });

  function selectVariant(id: string) {
    selectedId = id;
  }

  async function openCode() {
    if (!selected) return;
    codeOpen = true;
    copyState = 'idle';
    sourceText = '';
    sourceError = '';
    sourceStatus = 'loading';
    const request = ++sourceRequest;
    const sourceUrl = selected.sourceUrl;
    await tick();
    if (sourceDialog && !sourceDialog.open) sourceDialog.showModal();
    closeButton?.focus();
    try {
      const response = await fetch(sourceUrl);
      if (!response.ok) {
        const body = await response.text();
        throw new Error(`HTTP ${response.status}${body ? ` — ${body}` : ''}`);
      }
      const text = await response.text();
      if (request !== sourceRequest || selected.sourceUrl !== sourceUrl) return;
      sourceText = text;
      sourceStatus = 'ready';
    } catch (error) {
      if (request !== sourceRequest) return;
      sourceStatus = 'error';
      sourceError = `Could not load ${selected.sourceDownloadName}: ${error instanceof Error ? error.message : String(error)}`;
    }
  }

  function closeCode() {
    if (sourceDialog?.open) {
      sourceDialog.close();
      return;
    }
    finalizeCodeClose();
  }

  function finalizeCodeClose() {
    sourceRequest += 1;
    codeOpen = false;
    copyState = 'idle';
    void tick().then(() => codeButton?.focus());
  }

  function handleDialogClick(event: MouseEvent) {
    if (event.target === sourceDialog) closeCode();
  }

  async function copyCode() {
    if (!selected) return;
    if (copyResetTimer) clearTimeout(copyResetTimer);
    try {
      await navigator.clipboard.writeText(sourceText);
      copyState = 'copied';
    } catch {
      copyState = 'error';
    }
    copyResetTimer = setTimeout(() => (copyState = 'idle'), 1800);
  }

  function partDownloadLabel(part: CaseShowcaseVariant['parts'][number]): string {
    return `DOWNLOAD ${part.label}`;
  }
</script>

{#if selected}
  <div
    class="case-workbench-wrap"
    data-testid="case-workbench"
    data-selected-variant={selected.id}
  >
    <div class="case-workbench">
      <header class="workbench-header">
        <div class="workbench-file">
          <span class="workbench-file__kicker">CASE STUDY</span>
          <strong>{selected.sourceDownloadName}</strong>
        </div>
        <div class="workbench-controls">
          <div class="phone-static" aria-label="Current phone model">
            <span>DEVICE</span>
            <strong>iPhone 17e</strong>
          </div>
          <button class="workbench-code" type="button" bind:this={codeButton} onclick={openCode}>SEE CODE</button>
        </div>
      </header>

      <div class="pattern-bar" role="group" aria-label="Case pattern">
        <span class="pattern-bar__label">PATTERN</span>
        <div class="pattern-options">
          {#each currentPatternVariants as variant}
            <button
              type="button"
              class:variant-choice--active={variant.id === selected.id}
              class="variant-choice"
              aria-pressed={variant.id === selected.id}
              onclick={() => selectVariant(variant.id)}
            >
              <strong>{variant.label}</strong>
              <span>{variant.note}</span>
            </button>
          {/each}
        </div>
      </div>

      <div class="case-viewport">
        <div class="viewport-label">{selected.title.toUpperCase()}</div>
        <StlViewer
          size={620}
          parts={selected.parts.map(({ url, color }) => ({ url, color }))}
        />
        <div class="viewport-hint">DRAG TO ORBIT</div>
      </div>

      <div class="earlier-versions" role="group" aria-label="Earlier case versions">
        <div class="earlier-heading">
          <span>EARLIER ATTEMPTS</span>
          <small>Same phone. More questionable decisions.</small>
        </div>
        <div class="earlier-list">
          {#each earlierCaseVariants as variant, index}
            <button
              type="button"
              class:earlier-choice--active={variant.id === selected.id}
              class="earlier-choice"
              aria-pressed={variant.id === selected.id}
              onclick={() => selectVariant(variant.id)}
            >
              <span>V{String(index + 1).padStart(2, '0')}</span>
              <strong>{variant.label}</strong>
            </button>
          {/each}
        </div>
      </div>

      {#if codeOpen}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <dialog
          class="source-dialog"
          bind:this={sourceDialog}
          aria-label={`Macro Inspector: ${selected.title}`}
          onclose={finalizeCodeClose}
          onclick={handleDialogClick}
        >
          <div class="source-inspector">
            <header class="source-header">
              <strong>MACRO INSPECTOR: {selected.sourceDownloadName}</strong>
              <button type="button" bind:this={closeButton} onclick={closeCode} aria-label="CLOSE CODE">×</button>
            </header>
            <div class="source-editor">
              {#if sourceStatus === 'ready'}
                <ReadOnlyEckySource code={sourceText} label={`Full source for ${selected.title}`} />
              {:else if sourceStatus === 'error'}
                <div class="source-state source-state--error" role="alert">{sourceError}</div>
              {:else}
                <div class="source-state" role="status">LOADING SAVED SOURCE…</div>
              {/if}
            </div>
            <footer class="source-footer">
              <button type="button" onclick={copyCode} disabled={sourceStatus !== 'ready'}>
                {copyState === 'copied' ? 'COPIED' : copyState === 'error' ? 'COPY FAILED' : 'COPY CODE'}
              </button>
              <a href={selected.sourceUrl} download={selected.sourceDownloadName}>DOWNLOAD SOURCE</a>
            </footer>
          </div>
        </dialog>
      {/if}
    </div>

    <div class="case-downloads" aria-label="Case downloads">
      {#each selected.parts as part}
        <a class="case-download case-download--primary" href={part.url} download={part.downloadName}>
          {partDownloadLabel(part)}
        </a>
      {/each}
      <a class="case-download" href={selected.sourceUrl} download={selected.sourceDownloadName}>DOWNLOAD .ECKY</a>
    </div>
  </div>
{/if}

<style>
  .case-workbench-wrap,
  .case-workbench,
  .case-viewport,
  .earlier-versions,
  .source-inspector,
  .source-editor {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .case-workbench-wrap { width: 100%; }

  .case-workbench {
    position: relative;
    border: 2px solid color-mix(in srgb, var(--secondary) 58%, var(--border-bright));
    background: color-mix(in srgb, var(--bg-100) 94%, #000 6%);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--bg-300) 90%, transparent), 0 20px 60px rgba(0, 0, 0, 0.36);
  }

  .workbench-header {
    min-height: 58px;
    padding: 8px 10px 8px 14px;
    border-bottom: 1px solid var(--bg-300);
    background: var(--bg-200);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    overflow: hidden;
  }

  .workbench-file { display: grid; gap: 2px; min-width: 0; }

  .workbench-file__kicker,
  .phone-static span,
  .pattern-bar__label,
  .earlier-heading span {
    color: var(--text-dim);
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.12em;
  }

  .workbench-file strong {
    color: var(--secondary);
    font-size: 0.76rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .workbench-controls { display: flex; align-items: stretch; gap: 8px; flex: 0 0 auto; }

  .phone-static {
    display: grid;
    gap: 1px;
    min-width: 142px;
    padding: 4px 8px;
    border: 1px solid var(--bg-300);
    background: var(--bg-100);
  }

  .phone-static strong {
    color: var(--text);
    font: 700 0.74rem var(--font-mono);
    text-transform: uppercase;
  }

  .workbench-code,
  .variant-choice,
  .earlier-choice,
  .source-header button,
  .source-footer button,
  .source-footer a,
  .case-download {
    border: 1px solid var(--bg-400);
    background: var(--bg-100);
    color: var(--text);
    font-family: var(--font-mono);
    font-weight: 700;
    cursor: pointer;
  }

  .workbench-code { min-width: 92px; padding: 0 14px; color: var(--secondary); letter-spacing: 0.06em; }

  .workbench-code:hover,
  .workbench-code:focus-visible,
  .source-footer button:hover,
  .source-footer a:hover { border-color: var(--secondary); outline: none; }

  .pattern-bar {
    padding: 10px 12px 12px;
    border-bottom: 1px solid var(--bg-300);
    display: grid;
    grid-template-columns: 74px minmax(0, 1fr);
    align-items: stretch;
    gap: 10px;
    overflow: hidden;
  }

  .pattern-bar__label { padding-top: 9px; color: var(--secondary); }
  .pattern-options { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; min-width: 0; }

  .variant-choice {
    min-width: 0;
    padding: 8px 10px;
    display: grid;
    gap: 3px;
    text-align: left;
    overflow: hidden;
  }

  .variant-choice strong { color: var(--text); font-size: 0.74rem; letter-spacing: 0.07em; }
  .variant-choice span { color: var(--text-dim); font-size: 0.7rem; font-weight: 400; line-height: 1.4; }

  .variant-choice:hover,
  .variant-choice:focus-visible,
  .variant-choice--active { border-color: var(--primary); outline: none; }

  .variant-choice--active { background: color-mix(in srgb, var(--primary) 12%, var(--bg-100)); }
  .variant-choice--active strong { color: var(--primary); }

  .case-viewport {
    position: relative;
    min-height: 580px;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    justify-items: center;
    background: linear-gradient(rgba(74, 140, 92, 0.035) 1px, transparent 1px), linear-gradient(90deg, rgba(74, 140, 92, 0.035) 1px, transparent 1px), #080c17;
    background-size: 20px 20px;
  }

  .viewport-label {
    width: 100%;
    padding: 8px 10px;
    border-bottom: 1px solid var(--bg-300);
    color: var(--text-dim);
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.1em;
  }

  .case-viewport :global(.viewer) { align-self: center; }

  .viewport-hint {
    width: 100%;
    padding: 7px 10px;
    border-top: 1px solid var(--bg-300);
    color: var(--text-dim);
    font-size: 0.68rem;
    letter-spacing: 0.09em;
    text-align: right;
  }

  .earlier-versions {
    border-top: 1px solid var(--bg-300);
    background: var(--bg-200);
    display: grid;
    grid-template-columns: minmax(150px, 0.7fr) minmax(0, 2fr);
    gap: 10px;
    padding: 10px 12px;
  }

  .earlier-heading { display: grid; align-content: center; gap: 3px; }
  .earlier-heading span { color: var(--secondary); }
  .earlier-heading small { color: var(--text-dim); font: 0.68rem/1.4 var(--font-mono); }
  .earlier-list { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 7px; min-width: 0; }

  .earlier-choice { min-width: 0; padding: 7px 8px; display: grid; gap: 2px; text-align: left; overflow: hidden; }
  .earlier-choice span { color: var(--text-dim); font-size: 0.66rem; letter-spacing: 0.08em; }
  .earlier-choice strong { overflow: hidden; color: var(--text); font-size: 0.7rem; letter-spacing: 0.05em; text-overflow: ellipsis; white-space: nowrap; }
  .earlier-choice:hover,
  .earlier-choice:focus-visible,
  .earlier-choice--active { border-color: var(--secondary); outline: none; }
  .earlier-choice--active { background: color-mix(in srgb, var(--secondary) 10%, var(--bg-100)); }

  .case-downloads { display: flex; justify-content: center; gap: 8px; flex-wrap: wrap; padding-top: 18px; overflow: hidden; }
  .case-download { padding: 9px 13px; font-size: 0.72rem; letter-spacing: 0.06em; }
  .case-download:hover,
  .case-download:focus-visible { border-color: var(--primary); color: var(--primary); outline: none; }
  .case-download--primary { border-color: var(--primary); color: var(--primary); background: color-mix(in srgb, var(--primary) 12%, var(--bg-100)); }

  .source-dialog {
    width: min(calc(100vw - 28px), 1100px);
    height: min(calc(100dvh - 28px), 820px);
    max-width: none;
    max-height: none;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    overflow: hidden;
  }
  .source-dialog::backdrop { background: rgba(5, 7, 13, 0.88); backdrop-filter: blur(4px); }
  .source-inspector { width: 100%; height: 100%; border: 2px solid color-mix(in srgb, var(--secondary) 72%, var(--bg-300)); background: var(--bg-100); box-shadow: 0 0 32px color-mix(in srgb, var(--secondary) 18%, transparent); display: grid; grid-template-rows: auto minmax(0, 1fr) auto; }
  .source-header,
  .source-footer { display: flex; align-items: center; gap: 8px; padding: 7px 9px; background: var(--bg-200); overflow: hidden; }
  .source-header { justify-content: space-between; border-bottom: 1px solid var(--bg-300); }
  .source-header strong { min-width: 0; color: var(--secondary); font-size: 0.72rem; letter-spacing: 0.08em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .source-header button { border: 0; background: transparent; padding: 2px 7px; color: var(--text-dim); font-size: 1.1rem; }
  .source-footer { justify-content: flex-end; border-top: 1px solid var(--bg-300); }
  .source-footer button,
  .source-footer a { padding: 7px 10px; font-size: 0.72rem; letter-spacing: 0.06em; }
  .source-footer button:disabled { cursor: wait; opacity: 0.45; }
  .source-state { padding: 14px; color: var(--text-dim); font: 0.74rem/1.5 var(--font-mono); }
  .source-state--error { color: #ef7d7d; }

  @media (max-width: 720px) {
    .workbench-header { align-items: stretch; flex-direction: column; gap: 8px; }
    .workbench-controls { width: 100%; }
    .phone-static { flex: 1; min-width: 0; }
    .pattern-bar { grid-template-columns: 1fr; gap: 7px; }
    .pattern-bar__label { padding-top: 0; }
    .pattern-options { grid-template-columns: 1fr; }
    .case-viewport { min-height: 0; aspect-ratio: 1 / 1.2; }
    .earlier-versions { grid-template-columns: 1fr; }
    .earlier-list { grid-template-columns: 1fr; }
    .source-dialog { width: calc(100vw - 10px); height: calc(100dvh - 10px); }
    .source-footer { justify-content: stretch; }
    .source-footer button,
    .source-footer a { flex: 1; text-align: center; }
  }
</style>
