<script lang="ts">
  import { cycleTopologyMode, topologyModeLabel, type TopologyMode } from '../viewerDisplayMode';

  let {
    outlineEnabled = true,
    topologyMode = 'mesh',
    selectionMode = 'orbit',
    onViewerDisplayChange,
    onViewerSelectionModeChange,
  }: {
    outlineEnabled?: boolean;
    topologyMode?: TopologyMode;
    selectionMode?: 'orbit' | 'select' | 'measure';
    onViewerDisplayChange?: (display: { outlineEnabled: boolean; topologyMode: TopologyMode }) => void;
    onViewerSelectionModeChange?: (mode: 'orbit' | 'select' | 'measure') => void;
  } = $props();
</script>

<section class="mesh-settings" aria-label="Mesh and viewport settings">
  <div class="section-label">DISPLAY</div>
  <div class="mesh-settings__controls">
    <button
      class="mesh-setting"
      class:mesh-setting-active={outlineEnabled}
      aria-pressed={outlineEnabled}
      onclick={() => onViewerDisplayChange?.({ outlineEnabled: !outlineEnabled, topologyMode })}
      title="Toggle part outlines in the viewport"
    >
      OUTLINE
    </button>
    <button
      class="mesh-setting"
      class:mesh-setting-active={topologyMode !== 'off'}
      aria-pressed={topologyMode !== 'off'}
      onclick={() => onViewerDisplayChange?.({ outlineEnabled, topologyMode: cycleTopologyMode(topologyMode) })}
      title="Cycle topology overlay: off, feature edges, mesh wireframe"
    >
      {topologyModeLabel(topologyMode)}
    </button>
  </div>

  <div class="section-label">INTERACTION</div>
  <div class="mesh-settings__controls">
    {#each ['orbit', 'select', 'measure'] as mode}
      <button
        class="mesh-setting"
        class:mesh-setting-active={selectionMode === mode}
        aria-pressed={selectionMode === mode}
        onclick={() => onViewerSelectionModeChange?.(mode as 'orbit' | 'select' | 'measure')}
      >
        {mode.toUpperCase()}
      </button>
    {/each}
  </div>
</section>

<style>
  .mesh-settings {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border: 1px solid var(--bg-300);
    background: var(--bg-100);
    overflow: hidden;
  }

  .mesh-settings__controls {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    overflow: hidden;
  }

  .mesh-setting {
    padding: 5px 10px;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    color: var(--text-dim);
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    cursor: pointer;
  }

  .mesh-setting-active {
    border-color: var(--secondary);
    background: color-mix(in srgb, var(--secondary) 14%, var(--bg-200));
    color: var(--text);
  }

  .section-label {
    color: var(--secondary);
    font-size: 0.58rem;
    font-weight: bold;
    letter-spacing: 0.12em;
  }
</style>
