<script lang="ts">
  import { get } from 'svelte/store';
  import Modal from './Modal.svelte';
  import {
    applySemanticManifestEdit,
    formatBackendError,
  } from './tauri/client';
  import { buildImportedSyntheticDesign } from './modelRuntime/importedRuntime';
  import { activeThreadIdStore as activeThreadId, historyStore as history } from './stores/domainState';
  import { session } from './stores/sessionStore';
  import type {
    DesignParams,
    EnrichmentProposal,
    EnrichmentStatus,
    ModelManifest,
    PartBinding,
  } from './types/domain';

  let {
    manifest,
    activeVersionId = null,
    onclose,
    ondone,
    onSelectPart,
  }: {
    manifest: ModelManifest;
    activeVersionId?: string | null;
    onclose: () => void;
    ondone: (updatedManifest: ModelManifest) => void;
    onSelectPart?: (partId: string | null) => void;
  } = $props();

  let proposalStatuses = $state<Record<string, EnrichmentStatus>>({});
  let saving = $state(false);
  let step = $state<'review' | 'done'>('review');

  const proposals = $derived<EnrichmentProposal[]>(
    manifest.enrichmentState?.proposals || [],
  );

  const localProposals = $derived<EnrichmentProposal[]>(
    proposals.map((p) => ({
      ...p,
      status: proposalStatuses[p.proposalId] ?? p.status,
    })),
  );

  const parts = $derived<PartBinding[]>(manifest.parts || []);

  const acceptedCount = $derived(
    localProposals.filter((p) => p.status === 'accepted').length,
  );

  const rejectedCount = $derived(
    localProposals.filter((p) => p.status === 'rejected').length,
  );

  function labelPartIds(partIds: string[] | undefined): string {
    if (!partIds?.length || !parts.length) return 'No parts';
    return partIds
      .map((id) => parts.find((p) => p.partId === id)?.label || id)
      .join(', ');
  }

  function acceptAll() {
    const next: Record<string, EnrichmentStatus> = {};
    for (const p of proposals) next[p.proposalId] = 'accepted';
    proposalStatuses = next;
  }

  function rejectAll() {
    const next: Record<string, EnrichmentStatus> = {};
    for (const p of proposals) next[p.proposalId] = 'rejected';
    proposalStatuses = next;
  }

  function toggleProposal(proposalId: string) {
    const current = proposalStatuses[proposalId] ?? proposals.find((p) => p.proposalId === proposalId)?.status ?? 'pending';
    proposalStatuses = {
      ...proposalStatuses,
      [proposalId]: current === 'accepted' ? 'pending' : 'accepted',
    };
  }

  function statusOf(proposalId: string): EnrichmentStatus {
    return proposalStatuses[proposalId] ?? proposals.find((p) => p.proposalId === proposalId)?.status ?? 'pending';
  }

  async function commitChanges() {
    if (saving) return;
    saving = true;

    try {
      const versionMessageId = activeVersionId;
      const result = await applySemanticManifestEdit({
        modelId: manifest.modelId,
        messageId: versionMessageId,
        edit: {
          action: 'setProposalStatuses',
          entries: localProposals.map((proposal) => ({
            proposalId: proposal.proposalId,
            status: proposal.status,
          })),
        },
      });
      const nextManifest = result.manifest;

      const threadId = get(activeThreadId);
      if (threadId && versionMessageId) {
        const nextOutput = buildImportedSyntheticDesign(nextManifest, {}, null);
        history.update((threads) =>
          threads.map((thread) => {
            if (thread.id !== threadId || !thread.messages?.length) return thread;
            return {
              ...thread,
              messages: thread.messages.map((msg) =>
                msg.id === versionMessageId
                  ? { ...msg, output: nextOutput ?? msg.output ?? null, modelManifest: nextManifest }
                  : msg,
              ),
            };
          }),
        );
      }

      const currentSession = get(session);
      session.setModelRuntime(currentSession.artifactBundle, nextManifest);

      ondone(nextManifest);
      onclose();
    } catch (e: unknown) {
      session.setError(`Enrichment Commit Failed: ${formatBackendError(e)}`);
    } finally {
      saving = false;
    }
  }
</script>

<Modal title="IMPORT ENRICHMENT" onclose={onclose}>
  {#if step === 'review'}
    <div class="enrichment-container">
      <div class="enrichment-header">
        <p class="enrichment-intro">
          We detected <strong>{parts.length}</strong> parts in your FCStd file.
          Review the proposed parameter bindings below.
        </p>
        <div class="enrichment-summary">
          <span class="summary-chip summary-accepted">{acceptedCount} ACCEPTED</span>
          <span class="summary-chip summary-rejected">{rejectedCount} REJECTED</span>
          <span class="summary-chip summary-total">{proposals.length} TOTAL</span>
        </div>
      </div>

      <div class="proposal-list">
        {#each localProposals as proposal}
          {@const status = statusOf(proposal.proposalId)}
          <button
            class="proposal-card"
            class:proposal-accepted={status === 'accepted'}
            class:proposal-rejected={status === 'rejected'}
            class:proposal-pending={status === 'pending'}
            onmouseenter={() => onSelectPart?.(proposal.partIds?.[0] ?? null)}
            onmouseleave={() => onSelectPart?.(null)}
            onclick={() => toggleProposal(proposal.proposalId)}
            type="button"
          >
            <div class="proposal-head">
              <div class="proposal-label-row">
                <span class="proposal-label">{proposal.label}</span>
              </div>
              <span class="proposal-status proposal-status-{status}">
                {status.toUpperCase()}
              </span>
            </div>
            <div class="proposal-meta">
              PARTS: {labelPartIds(proposal.partIds)}
            </div>
            <div class="proposal-meta">
              PARAMS: {proposal.parameterKeys?.length ? proposal.parameterKeys.join(', ') : 'No parameter keys'}
            </div>
            <div class="proposal-meta">SOURCE: {proposal.provenance}</div>
          </button>
        {/each}
      </div>

      <div class="enrichment-actions">
        <div class="actions-left">
          <button class="btn btn-xs btn-primary" onclick={acceptAll}>ACCEPT ALL</button>
          <button class="btn btn-xs btn-ghost" onclick={rejectAll}>REJECT ALL</button>
        </div>
        <div class="actions-right">
          <button class="btn btn-xs btn-ghost" onclick={onclose}>CANCEL</button>
          <button class="btn btn-xs btn-primary" onclick={commitChanges} disabled={saving}>
            {#if saving}
              APPLYING...
            {:else}
              APPLY
            {/if}
          </button>
        </div>
      </div>
    </div>
  {/if}
</Modal>

<style>
  .enrichment-container {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
    overflow: hidden;
    max-height: 70vh;
    min-width: 480px;
  }

  .enrichment-header {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .enrichment-intro {
    margin: 0;
    font-size: 0.72rem;
    color: var(--text-dim);
    line-height: 1.5;
  }

  .enrichment-intro strong {
    color: var(--text);
  }

  .enrichment-summary {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .summary-chip {
    padding: 2px 6px;
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
  }

  .summary-accepted {
    color: var(--secondary);
    border-color: color-mix(in srgb, var(--secondary) 45%, var(--bg-300));
  }

  .summary-rejected {
    color: var(--text-dim);
    border-color: color-mix(in srgb, var(--text-dim) 45%, var(--bg-300));
  }

  .summary-total {
    color: var(--primary);
    border-color: color-mix(in srgb, var(--primary) 45%, var(--bg-300));
  }

  .proposal-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow-y: auto;
    max-height: 50vh;
  }

  .proposal-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px;
    border: 1px solid var(--bg-300);
    background: var(--bg-100);
    cursor: pointer;
    text-align: left;
    width: 100%;
    font-family: inherit;
    color: inherit;
    transition: border-color 0.15s;
  }

  .proposal-card:hover {
    border-color: var(--primary);
  }

  .proposal-pending {
    border-color: color-mix(in srgb, var(--primary) 35%, var(--bg-300));
  }

  .proposal-accepted {
    border-color: color-mix(in srgb, var(--secondary) 55%, var(--bg-300));
    background: color-mix(in srgb, var(--secondary) 5%, var(--bg-100));
  }

  .proposal-rejected {
    opacity: 0.55;
  }

  .proposal-head {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    align-items: flex-start;
  }

  .proposal-label-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .proposal-label {
    color: var(--text);
    font-size: 0.74rem;
    font-weight: 700;
  }

  .proposal-meta {
    color: var(--text-dim);
    font-size: 0.64rem;
  }

  .proposal-status {
    padding: 3px 6px;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    color: var(--text-dim);
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    flex-shrink: 0;
  }

  .proposal-status-pending {
    border-color: color-mix(in srgb, var(--primary) 45%, var(--bg-300));
    color: var(--primary);
  }

  .proposal-status-accepted {
    border-color: color-mix(in srgb, var(--secondary) 45%, var(--bg-300));
    color: var(--secondary);
  }

  .proposal-status-rejected {
    border-color: color-mix(in srgb, var(--text-dim) 45%, var(--bg-300));
    color: var(--text-dim);
  }

  .enrichment-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--bg-300);
  }

  .actions-left,
  .actions-right {
    display: flex;
    gap: 6px;
  }
</style>
