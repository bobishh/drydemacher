<script lang="ts">
  import type { Advisory } from '../types/domain';
  import { summarizeAdvisories, type AdvisoryControlLabel } from '../modelRuntime/advisoryPresentation';

  let {
    advisories = [],
    controls = [],
    onDeleteManualAdvisory,
  }: {
    advisories?: Advisory[];
    controls?: AdvisoryControlLabel[];
    onDeleteManualAdvisory?: (advisoryId: string) => void;
  } = $props();

  const summaries = $derived(summarizeAdvisories(advisories, controls));
</script>

{#if summaries.length > 0}
  <div class="warning-stack">
    {#each summaries as advisory}
      <div class="warning-chip" data-severity={advisory.severity}>
        <span>
          {advisory.label}{advisory.count > 1 ? ` (${advisory.count})` : ''}: {advisory.message}
          {#if advisory.affectedLabels.length > 0}
            <span class="warning-chip-context">AFFECTED: {advisory.affectedLabels.join(', ')}</span>
          {/if}
        </span>
        {#if advisory.advisoryId.startsWith('advisory-manual-')}
          <button
            class="btn btn-xs btn-ghost warning-chip-action"
            onclick={() => onDeleteManualAdvisory?.(advisory.advisoryId)}
          >
            DELETE
          </button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .warning-stack {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    overflow: hidden;
  }

  .warning-chip {
    padding: 3px 6px;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    color: var(--text-dim);
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    text-transform: none;
    letter-spacing: normal;
    font-weight: 500;
  }

  .warning-chip[data-severity='warning'] {
    border-color: color-mix(in srgb, var(--primary) 45%, var(--bg-300));
    color: var(--primary);
  }

  .warning-chip-action {
    flex-shrink: 0;
  }

  .warning-chip-context {
    display: block;
    margin-top: 2px;
    color: var(--text-dim);
    font-size: 0.54rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
</style>
