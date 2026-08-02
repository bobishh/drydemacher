<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { formatBackendError } from '../tauri/client';

  // thread-source-binding §3.3 — Settings source-root picker for the existing
  // Config.projectsRoot. The bound directory is persisted through the existing
  // save_config command (the caller's onsave). The OS directory picker is
  // dependency-injected so the component is unit-testable under jsdom and the
  // raw filesystem/persist error always renders in-UI (never console-only).
  let {
    projectsRoot = $bindable(''),
    onsave = undefined,
    pickDirectory = undefined,
  }: {
    projectsRoot?: string;
    onsave?: () => Promise<void> | void;
    pickDirectory?: () => Promise<string | null>;
  } = $props();

  const defaultPickDirectory = async (): Promise<string | null> => {
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === 'string' ? selected : null;
  };

  const resolvePicker = (): (() => Promise<string | null>) =>
    pickDirectory ?? defaultPickDirectory;

  let busy = $state(false);
  let error = $state<string | null>(null);

  const displayPath = $derived(projectsRoot && projectsRoot.trim().length > 0 ? projectsRoot : '');

  async function handlePick() {
    if (busy) return;
    error = null;
    busy = true;
    try {
      const selected = await resolvePicker()();
      if (typeof selected !== 'string' || !selected.trim()) return;
      const previous = projectsRoot;
      projectsRoot = selected.trim();
      try {
        if (onsave) await onsave();
      } catch (e) {
        projectsRoot = previous;
        throw e;
      }
    } catch (e: unknown) {
      error = `Source root: ${formatBackendError(e)}`;
    } finally {
      busy = false;
    }
  }

  async function handleClear() {
    if (busy) return;
    const previous = projectsRoot;
    error = null;
    busy = true;
    projectsRoot = '';
    try {
      if (onsave) await onsave();
    } catch (e: unknown) {
      projectsRoot = previous;
      error = `Source root: ${formatBackendError(e)}`;
    } finally {
      busy = false;
    }
  }
</script>

<div class="source-root-field field" data-testid="source-root-field">
  <div class="prompt-header">
    <label for="source-root-input">SOURCE ROOT</label>
    <div class="button-row">
      <button
        class="btn btn-xs btn-ghost"
        type="button"
        data-testid="source-root-pick"
        onclick={handlePick}
        disabled={busy}
      >
        {busy ? '…' : projectsRoot ? 'CHANGE FOLDER' : 'SET FOLDER'}
      </button>
      {#if displayPath}
        <button
          class="btn btn-xs btn-ghost"
          type="button"
          title="Clear source root (use default <app_data>/projects)"
          onclick={handleClear}
          disabled={busy}
        >
          ✕ CLEAR
        </button>
      {/if}
    </div>
  </div>
  <div class="source-root-path" data-testid="source-root-path" title={displayPath}>
    {#if displayPath}
      <code>{displayPath}</code>
    {:else}
      <span class="field-help">DEFAULT — new thread source folders use &lt;app_data&gt;/projects.</span>
    {/if}
  </div>
  <div class="field-help">
    Bound thread source folders (<code>&lt;source-root&gt;/&lt;slug&gt;/model.ecky</code>) are editable working copies; Ecky history stays in SQLite.
  </div>
  {#if error}
    <div class="source-root-error" data-testid="source-root-error" role="alert">{error}</div>
  {/if}
</div>

<style>
  .source-root-field {
    min-width: 0;
  }

  .source-root-path {
    min-width: 0;
    padding: 7px 9px;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    overflow: hidden;
  }

  .source-root-path code {
    display: block;
    min-width: 0;
    overflow: hidden;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    white-space: nowrap;
    text-overflow: ellipsis;
    text-align: left;
    direction: rtl;
    unicode-bidi: plaintext;
  }

  .source-root-error {
    margin-top: 6px;
    padding: 7px 9px;
    border: 1px solid var(--red);
    background: color-mix(in srgb, var(--red) 12%, var(--bg-200));
    color: var(--red);
    font-family: var(--font-mono);
    font-size: 0.7rem;
    white-space: pre-wrap;
    word-break: break-word;
    overflow: hidden;
  }
</style>
