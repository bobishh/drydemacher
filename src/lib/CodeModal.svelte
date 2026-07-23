<script lang="ts">
  import Window from './Window.svelte';
  import CodePanel from './CodePanel.svelte';
  import MacroDiffPanel from './MacroDiffPanel.svelte';
  import {
    seedCodeModalDraftField,
    shouldReseedCodeModalDraftFields,
  } from './codeModalDraftFields';
  import { composeMacroDiffPanelModel } from './macroDiffPanel';
  import type { SessionCodeDiffView } from './sessionActivity';
  import {
    canInsertVerifyTemplate,
    hasVerifyClause,
    insertVerifyTemplate,
    looksLikeEckyModelSource,
  } from './verifyTemplate';
  import { formatBackendError } from './tauri/client';

  /** Parse a line number (1-based) from an error message or object. */
  function parseErrorLine(error: unknown): number | null {
    // Check direct fields on AppError-shaped objects
    const startLine = (error as { startLine?: number | null } | null)?.startLine;
    const ctxStart = (error as { diagnosticContext?: { startLine?: number | null } | null } | null)
      ?.diagnosticContext?.startLine;
    const direct = startLine ?? ctxStart;
    if (typeof direct === 'number' && direct > 0) return direct;
    // Fallback: scan formatted text for 'line N'
    const text = formatBackendError(error);
    const match = text.match(/line\s+(\d+)/i);
    return match ? parseInt(match[1], 10) : null;
  }

  type CodeModalCommitPayload = {
    code: string;
    title: string;
    versionName: string;
  };

  type CodeModalMode = 'version' | 'sketch-preview' | 'docs-snippet';

  let {
    code = $bindable(''),
    mode = 'version',
    sourceLanguage = null,
    macroDiffView = null,
    title,
    draftScopeKey = '',
    defaultTitle = '',
    defaultVersionName = '',
    z = 0,
    hidden = false,
    focused = true,
    onclose,
    onApply,
    onCommit,
  }: {
    code?: string;
    mode?: CodeModalMode;
    sourceLanguage?: string | null;
    macroDiffView?: SessionCodeDiffView | null;
    title: string;
    draftScopeKey?: string;
    defaultTitle?: string;
    defaultVersionName?: string;
    z?: number;
    hidden?: boolean;
    focused?: boolean;
    onclose: () => void;
    onApply?: (code: string) => Promise<unknown> | unknown;
    onCommit?: (payload: CodeModalCommitPayload) => Promise<void> | void;
  } = $props();

  let x = $state(60);
  let y = $state(40);
  let width = $state(960);
  let height = $state(620);

  let copyState = $state<'idle' | 'copied'>('idle');
  let verifyState = $state<'idle' | 'inserted' | 'exists'>('idle');
  let commitState = $state<'idle' | 'applying' | 'committing'>('idle');
  let commitError = $state('');
  let errorLine = $state<number | null>(null);
  let draftTitle = $state('');
  let draftVersionName = $state('');
  let seededDraftScopeKey = $state('');
  const canMutateVersion = $derived(mode === 'version');
  const effectiveDraftScopeKey = $derived(
    draftScopeKey || `${mode}:${title}:${defaultTitle}:${defaultVersionName}:${sourceLanguage ?? ''}`,
  );
  const macroDiffModel = $derived.by(() =>
    canMutateVersion && macroDiffView ? composeMacroDiffPanelModel(macroDiffView) : null,
  );

  $effect(() => {
    if (!shouldReseedCodeModalDraftFields(seededDraftScopeKey, effectiveDraftScopeKey, commitState)) return;
    draftTitle = seedCodeModalDraftField(defaultTitle, title);
    draftVersionName = seedCodeModalDraftField(defaultVersionName, 'V-manual');
    seededDraftScopeKey = effectiveDraftScopeKey;
    commitError = '';
    errorLine = null;
    verifyState = 'idle';
  });



  async function copyCode() {
    try {
      await navigator.clipboard.writeText(code);
      copyState = 'copied';
      setTimeout(() => copyState = 'idle', 2000);
    } catch (e: unknown) {
      console.error('Failed to copy code:', e);
    }
  }

  async function handleApply() {
    if (!onApply || commitState !== 'idle') return;
    commitState = 'applying';
    commitError = '';
    errorLine = null;
    try {
      await onApply(code);
    } catch (e: unknown) {
      console.error('Failed to apply code:', e);
      commitError = formatBackendError(e);
      errorLine = parseErrorLine(e);
    } finally {
      commitState = 'idle';
    }
  }

  function commitPayload(): CodeModalCommitPayload {
    return {
      code,
      title: draftTitle.trim() || defaultTitle || title || 'Manual Edit',
      versionName: draftVersionName.trim() || defaultVersionName || 'V-manual',
    };
  }

  async function handleCommit() {
    if (!onCommit || commitState !== 'idle') return;
    commitState = 'committing';
    commitError = '';
    errorLine = null;
    try {
      await onCommit(commitPayload());
    } catch (e: unknown) {
      console.error('Failed to commit code:', e);
      commitError = formatBackendError(e);
      errorLine = parseErrorLine(e);
    } finally {
      commitState = 'idle';
    }
  }

  function handleCodeChange(nextCode: string) {
    code = nextCode;
    verifyState = 'idle';
  }

  function handleInsertVerify() {
    if (canInsertVerifyTemplate(code)) {
      code = insertVerifyTemplate(code);
      verifyState = 'inserted';
      return;
    }
    if (hasVerifyClause(code)) {
      verifyState = 'exists';
    }
  }
</script>

<Window 
  windowId="code"
  title={`MACRO INSPECTOR: ${title}`} 
  {onclose} 
  {z}
  {hidden}
  {focused}
  bind:x 
  bind:y 
  bind:width 
  bind:height
>
  <div class="code-modal-content">
    {#if canMutateVersion}
      <div class="code-modal-topbar">
        <div class="commit-fields">
          <label class="commit-field commit-field-title">
            <span class="commit-field__label">Title</span>
            <input
              class="commit-input"
              bind:value={draftTitle}
              placeholder="Title"
              aria-label="Version title"
              disabled={commitState !== 'idle'}
            />
          </label>
          <label class="commit-field commit-field-version">
            <span class="commit-field__label">Version</span>
            <input
              class="commit-input commit-input-version"
              bind:value={draftVersionName}
              placeholder="Version"
              aria-label="Version name"
              disabled={commitState !== 'idle'}
            />
          </label>
        </div>
        <div class="commit-actions">
          <button
            class="btn btn-secondary"
            onclick={handleApply}
            disabled={!onApply || commitState !== 'idle'}
            title="Render code changes without creating a history version"
          >
            {#if commitState === 'applying'}
              APPLYING...
            {:else}
              APPLY
            {/if}
          </button>
          <button
            class="btn btn-primary"
            onclick={handleCommit}
            disabled={!onCommit || commitState !== 'idle'}
            title="Save changes as a new version in history"
          >
            {#if commitState === 'committing'}
              COMMITTING...
            {:else}
              COMMIT VERSION
            {/if}
          </button>
        </div>
      </div>
    {/if}
    <div class="code-editor-area">
      <CodePanel
        code={code}
        {sourceLanguage}
        highlightLine={errorLine}
        onchange={handleCodeChange}
      />
    </div>
    <MacroDiffPanel model={macroDiffModel} />
    <div class="code-modal-footer">
      <div class="footer-left">
        <button class="btn btn-secondary" onclick={copyCode}>
          {copyState === 'copied' ? 'COPIED!' : 'COPY CODE'}
        </button>
        {#if canMutateVersion && looksLikeEckyModelSource(code)}
          <button
            class="btn btn-secondary"
            onclick={handleInsertVerify}
            disabled={hasVerifyClause(code)}
            title={hasVerifyClause(code) ? 'This source already contains a verify clause.' : 'Append a top-level verify template to this Ecky model.'}
          >
            {#if verifyState === 'inserted'}
              VERIFY INSERTED
            {:else if hasVerifyClause(code)}
              VERIFY EXISTS
            {:else}
              INSERT VERIFY
            {/if}
          </button>
        {/if}
        {#if commitError}
          <div class="commit-error" title={commitError}>{commitError}</div>
        {/if}
      </div>
    </div>
  </div>
</Window>

<style>
  .code-modal-content {
    width: 100%;
    height: 100%;
    background: var(--bg);
    display: flex;
    flex-direction: column;
  }

  .code-modal-topbar {
    flex: 0 0 auto;
    padding: 10px 12px;
    background: color-mix(in srgb, var(--bg-100) 92%, var(--primary) 8%);
    border-bottom: 1px solid var(--bg-300);
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 12px;
    overflow: hidden;
  }

  .code-editor-area {
    flex: 1;
    min-height: 0;
  }

  .code-modal-footer {
    padding: 12px;
    background: var(--bg-100);
    border-top: 1px solid var(--bg-300);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .footer-left {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
  }

  .commit-error {
    max-width: 480px;
    padding: 8px 10px;
    border: 1px solid color-mix(in srgb, var(--red) 72%, var(--bg-300));
    background: color-mix(in srgb, var(--red) 14%, var(--bg-100));
    color: var(--text);
    font-size: 0.72rem;
    line-height: 1.35;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .commit-actions {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: flex-end;
    min-width: 0;
    flex: 0 0 auto;
  }

  .commit-fields {
    flex: 1 1 auto;
    display: grid;
    grid-template-columns: minmax(240px, 1fr) 140px;
    gap: 10px;
    min-width: 0;
  }

  .commit-field {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .commit-field__label {
    color: var(--text-dim);
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .commit-input {
    min-width: 0;
    width: 100%;
    height: 34px;
    border: 1px solid var(--bg-300);
    background: var(--bg);
    color: var(--text);
    padding: 0 10px;
    font-size: 0.72rem;
    font-family: inherit;
  }

  .commit-input-version {
    width: 100%;
  }
</style>
