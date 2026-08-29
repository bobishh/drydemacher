<script lang="ts">
  type SaveValuesState = 'idle' | 'saving' | 'saved';

  let {
    editing = false,
    applying = false,
    committing = false,
    manualApplyBusy = false,
    manualApplyQueued = 0,
    reading = false,
    undoDepth = 0,
    saveValuesState = 'idle',
    activeVersionId = null,
    onApplyChanges,
    onUndoParams,
    onCommitChanges,
    onSaveValues,
    onSaveFields,
    onCancelEditing,
    onReadFromMacro,
  }: {
    editing?: boolean;
    applying?: boolean;
    committing?: boolean;
    reading?: boolean;
    undoDepth?: number;
    saveValuesState?: SaveValuesState;
    manualApplyBusy?: boolean;
    manualApplyQueued?: number;
    activeVersionId?: string | null;
    onApplyChanges?: () => void;
    onUndoParams?: () => void;
    onCommitChanges?: () => void;
    onSaveValues?: () => void;
    onSaveFields?: () => void;
    onCancelEditing?: () => void;
    onReadFromMacro?: () => void;
  } = $props();
</script>

<div class="panel-actions">
  {#if !editing}
    <div class="apply-actions">
      <button
        class="btn btn-xs btn-primary apply-btn"
        onclick={onApplyChanges}
        title={manualApplyBusy
          ? 'Apply is running'
          : manualApplyQueued > 0
            ? `Apply queued: ${manualApplyQueued}`
            : undefined}
      >
        {#if applying || manualApplyBusy}
          APPLYING...
        {:else if manualApplyQueued > 0}
          APPLY QUEUED ({manualApplyQueued})
        {:else}
          APPLY
        {/if}
      </button>
      <button
        class="btn btn-xs btn-secondary"
        onclick={onUndoParams}
        data-undo-depth={undoDepth}
        data-applying={applying || manualApplyBusy}
        disabled={undoDepth === 0 || applying || manualApplyBusy}
        title="Undo last parameter change and rerender"
      >
        UNDO
      </button>
      <button
        class="btn btn-xs btn-primary"
        onclick={onCommitChanges}
        disabled={!activeVersionId || committing || applying || manualApplyBusy}
        title={activeVersionId ? 'Save current draft as immutable history version' : 'Generate first to commit a version'}
      >
        {#if committing}
          COMMITTING...
        {:else}
          COMMIT
        {/if}
      </button>
      <button
        class="btn btn-xs btn-ghost"
        onclick={onSaveValues}
        disabled={!activeVersionId || saveValuesState === 'saving'}
        title={activeVersionId ? 'Persist current values as defaults for this version' : 'Generate first to persist defaults'}
      >
        {#if saveValuesState === 'saving'}
          SAVING...
        {:else if saveValuesState === 'saved'}
          SAVED
        {:else}
          SAVE VALUES
        {/if}
      </button>
    </div>
  {:else}
    <div class="edit-toolbar-left">
      <button class="btn btn-xs btn-primary" onclick={onSaveFields}>💾 SAVE</button>
      <button class="btn btn-xs btn-ghost" onclick={onCancelEditing}>✕ CANCEL</button>
    </div>
    <button class="btn btn-xs btn-secondary" onclick={onReadFromMacro} title="Auto-detect parameters from macro code" disabled={reading}>
      {#if reading}
        ⏳ READING...
      {:else}
        🔍 READ FROM MACRO
      {/if}
    </button>
  {/if}
</div>

<style>
  .panel-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: space-between;
    align-items: flex-start;
    min-width: 0;
    overflow: hidden;
  }

  .apply-actions {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    min-width: 0;
  }

  .edit-toolbar-left {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    min-width: 0;
  }

</style>
