<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    projectFolderExport,
    projectFolderStatus,
    projectFolderApply,
    formatBackendError,
    type ProjectFolderStatus,
    type ProjectSyncState,
  } from './tauri/client';

  let {
    threadId = null,
    messageId = null,
    macroCode = '',
  }: {
    threadId?: string | null;
    messageId?: string | null;
    macroCode?: string;
  } = $props();

  type ChipState = ProjectSyncState | 'loading' | 'error';

  let status: ProjectFolderStatus | null = $state(null);
  let chipState: ChipState = $state('loading');
  let lastError: string | null = $state(null);
  let busy: 'export' | null = $state(null);
  let applyBusy: boolean = $state(false);
  let listenCleanups: UnlistenFn[] = [];

  // Avoid re-entrancy and stale races when the active version flips quickly.
  let refreshSeq = 0;

  function hasTauriIpc(): boolean {
    if (typeof window === 'undefined') return false;
    return typeof (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ === 'object';
  }

  async function refresh() {
    if (!hasTauriIpc() || !threadId || !macroCode) {
      status = null;
      chipState = 'loading';
      return;
    }
    const seq = ++refreshSeq;
    try {
      const next = await projectFolderStatus(threadId, messageId);
      if (seq !== refreshSeq) return;
      status = next;
      chipState = next.state;
    } catch (error) {
      if (seq !== refreshSeq) return;
      lastError = formatBackendError(error);
      chipState = 'error';
    }
  }

  async function handleExport() {
    if (!threadId || busy) return;
    busy = 'export';
    lastError = null;
    try {
      await projectFolderExport(threadId, messageId);
      await refresh();
    } catch (error) {
      lastError = formatBackendError(error);
      chipState = 'error';
    } finally {
      busy = null;
    }
  }

  async function handleApplySource() {
    if (!threadId || applyBusy) return;
    applyBusy = true;
    lastError = null;
    try {
      await projectFolderApply(threadId, messageId, true, null, null);
      await refresh();
    } catch (error) {
      lastError = formatBackendError(error);
      chipState = 'error';
    } finally {
      applyBusy = false;
    }
  }

  // Refresh whenever the bound target or its source changes.
  $effect(() => {
    void threadId;
    void messageId;
    void macroCode;
    void refresh();
  });

  onMount(async () => {
    if (!hasTauriIpc()) return;
    try {
      listenCleanups.push(
        await listen('project-folder-sync', () => {
          void refresh();
        }),
      );
      listenCleanups.push(
        await listen('history-updated', () => {
          void refresh();
        }),
      );
    } catch {
      // Listening is best-effort; the chip still refreshes on prop changes
      // and after explicit actions.
    }
  });

  onDestroy(() => {
    for (const cleanup of listenCleanups) {
      try {
        cleanup();
      } catch {
        // ignore
      }
    }
    listenCleanups = [];
  });

  const STATE_LABEL: Record<ProjectSyncState, string> = {
    missing: 'NOT EXPORTED',
    clean: 'FOLDER CLEAN',
    fileChanged: 'FILE CHANGED',
    threadAdvanced: 'FOLDER STALE',
    conflict: 'CONFLICT',
  };

  const STATE_HINT: Record<ProjectSyncState, string> = {
    missing: 'Export to edit model.ecky in your editor',
    clean: 'model.ecky matches this version',
    fileChanged: 'model.ecky edited; watcher will sync it',
    threadAdvanced: 'thread advanced past the export; re-export to refresh',
    conflict: 'file and thread both changed; re-export to refresh',
  };

  let label = $derived(
    chipState === 'loading'
      ? 'FOLDER…'
      : chipState === 'error'
        ? 'FOLDER ERROR'
        : STATE_LABEL[chipState],
  );
  let hint = $derived(
    chipState === 'loading' || chipState === 'error'
      ? ''
      : STATE_HINT[chipState],
  );
</script>

{#if macroCode}
  <div
    class="project-folder-chip"
    class:chip-clean={chipState === 'clean'}
    class:chip-changed={chipState === 'fileChanged'}
    class:chip-stale={chipState === 'threadAdvanced' || chipState === 'conflict'}
    class:chip-error={chipState === 'error' || chipState === 'missing'}
    data-testid="project-folder-chip"
    data-folder-state={chipState}
    title={hint}
  >
    <span class="chip-label" data-folder-state-text>{label}</span>
    {#if lastError}
      <span class="chip-error-text" data-testid="project-folder-error">{lastError}</span>
    {/if}
    <span class="chip-actions">
      {#if chipState === 'missing'}
        <button
          class="chip-btn chip-btn-primary"
          onclick={handleExport}
          disabled={busy !== null}
          title="Export model.ecky to a project folder"
        >EXPORT</button>
      {:else if chipState === 'fileChanged'}
        <button
          class="chip-btn"
          onclick={handleExport}
          disabled={busy !== null}
          title="Overwrite the file with this version's source, discarding the external edit"
        >RE-EXPORT</button>
      {:else if chipState === 'threadAdvanced'}
        <button
          class="chip-btn chip-btn-primary"
          onclick={handleExport}
          disabled={busy !== null}
          title="Refresh the folder onto the current thread head"
        >RE-EXPORT</button>
      {:else if chipState === 'conflict'}
        <button
          class="chip-btn chip-btn-primary"
          onclick={handleApplySource}
          disabled={applyBusy}
          title="Apply the edited folder source on top of current head"
        >APPLY SOURCE</button>
      {:else if chipState === 'clean'}
        <button
          class="chip-btn"
          onclick={handleExport}
          disabled={busy !== null}
          title="Re-export this version's source"
        >RE-EXPORT</button>
      {/if}
    </span>
  </div>
{/if}

<style>
  .project-folder-chip {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-width: 0;
    max-width: 100%;
    padding: 4px 8px;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    color: var(--text-dim);
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    line-height: 1.3;
    overflow: hidden;
  }

  .chip-label {
    flex: 0 0 auto;
    white-space: nowrap;
    text-transform: uppercase;
  }

  .chip-clean {
    color: var(--text-dim);
  }

  .chip-changed {
    border-color: var(--bg-300);
    background: var(--bg-200);
    color: var(--text-dim);
  }

  .chip-stale {
    border-color: var(--primary);
    background: color-mix(in srgb, var(--primary) 14%, var(--bg-200));
    color: var(--primary);
  }

  .chip-error {
    border-color: color-mix(in srgb, var(--primary) 60%, var(--bg-300));
    color: var(--primary);
  }

  .chip-error-text {
    flex: 1 1 160px;
    min-width: 0;
    color: var(--primary);
    font-weight: 600;
    letter-spacing: 0.02em;
    text-transform: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    margin-left: auto;
  }

  .chip-btn {
    padding: 3px 8px;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    color: var(--text-dim);
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    cursor: pointer;
    white-space: nowrap;
  }

  .chip-btn:hover:not(:disabled) {
    border-color: var(--secondary);
    color: var(--secondary);
  }

  .chip-btn-primary {
    border-color: color-mix(in srgb, var(--secondary) 55%, var(--bg-300));
    color: var(--secondary);
  }

  .chip-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
