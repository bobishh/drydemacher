<script lang="ts">
  import {
    createSourceActions,
    type SourceActionOutcome,
    type SourceLink,
  } from '../sourceActions';

  let {
    threadId,
    messageId = null,
    importedCad = false,
    baseSource = false,
  }: {
    threadId: string;
    messageId?: string | null;
    importedCad?: boolean;
    baseSource?: boolean;
  } = $props();

  const actions = createSourceActions();

  let openBusy = $state(false);
  let cadBusy = $state(false);
  let revealBusy = $state(false);
  let link = $state<SourceLink | null>(null);
  let error = $state<string | null>(null);

  function accept(result: SourceActionOutcome): void {
    if ('link' in result) {
      link = result.link;
      error = null;
      return;
    }
    error = result.error;
  }

  async function handleOpenCad(): Promise<void> {
    if (cadBusy) return;
    error = null;
    cadBusy = true;
    try {
      accept(await actions.openCadFile(threadId, messageId));
    } finally {
      cadBusy = false;
    }
  }

  async function handleOpen(): Promise<void> {
    if (openBusy) return;
    error = null;
    openBusy = true;
    try {
      accept(await actions.openSourceFile(threadId, messageId));
    } finally {
      openBusy = false;
    }
  }

  async function handleReveal(): Promise<void> {
    if (revealBusy) return;
    error = null;
    revealBusy = true;
    try {
      accept(await actions.revealSourceFolder(threadId, messageId, link));
    } finally {
      revealBusy = false;
    }
  }
</script>

<div class="code-source-actions" data-testid="source-actions">
  <div class="code-source-buttons">
    {#if importedCad}
      <button
        class="btn btn-secondary"
        type="button"
        data-testid="source-open-cad"
        title="Open the copied imported CAD source"
        disabled={cadBusy}
        onclick={handleOpenCad}
      >
        {cadBusy ? 'OPENING…' : 'OPEN CAD'}
      </button>
    {:else}
      <button
        class="btn btn-secondary"
        type="button"
        data-testid="source-open-file"
        title={baseSource ? 'Open the committed base model.ecky in the system editor' : 'Open model.ecky in the system editor'}
        disabled={openBusy}
        onclick={handleOpen}
      >
        {openBusy ? 'OPENING…' : baseSource ? 'OPEN BASE FILE' : 'OPEN FILE'}
      </button>
    {/if}
    <button
      class="btn btn-secondary"
      type="button"
      data-testid="source-reveal-folder"
      title={baseSource ? 'Reveal the committed base source folder' : 'Reveal the source folder in the system file manager'}
      disabled={revealBusy}
      onclick={handleReveal}
    >
      {revealBusy ? 'REVEALING…' : 'REVEAL FOLDER'}
    </button>
  </div>

  {#if link}
    <code class="source-path" data-testid="source-path-file" title={link.file}>{link.file}</code>
    <code class="source-path source-folder" data-testid="source-path-folder" title={link.folder}>{link.folder}</code>
  {/if}

  {#if error}
    <div class="source-error" data-testid="source-error" role="alert">{error}</div>
  {/if}
</div>

<style>
  .code-source-actions,
  .code-source-buttons {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }

  .code-source-actions {
    flex: 1 1 auto;
  }

  .code-source-buttons {
    flex: 0 0 auto;
  }

  .source-path {
    min-width: 0;
    max-width: 260px;
    color: var(--text-dim);
    font-size: 0.62rem;
    white-space: nowrap;
    text-overflow: ellipsis;
    overflow: hidden;
  }

  .source-folder {
    display: none;
  }

  .source-error {
    min-width: 0;
    max-width: 420px;
    padding: 5px 7px;
    border: 1px solid var(--red);
    color: var(--red);
    font-family: var(--font-mono);
    font-size: 0.62rem;
    white-space: normal;
    overflow: auto;
  }

  @container (max-width: 600px) {
    .source-path {
      display: none;
    }
  }
</style>
