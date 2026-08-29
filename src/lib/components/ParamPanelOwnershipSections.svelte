<script lang="ts">
  import ParamPanelControlField from './ParamPanelControlField.svelte';
  import type { DesignParams, ParamValue, ResolvedUiField } from '../types/domain';
  import type { ParameterOwnershipSection } from '../modelRuntime/ownershipSections';

  type CadTone = 'neutral' | 'size' | 'x' | 'y' | 'z' | 'angle' | 'state' | 'mode';
  type RangeProps = { min: number; max: number; step: number };

  let {
    sections,
    parameters,
    highlightedParamKey = null,
    searchActive = false,
    getRangeProps,
    getCadTone,
    onDraftValue,
    onUpdate,
    onPickImage,
    onSetFocusedControl,
    onClearFocusedControl,
  }: {
    sections: ParameterOwnershipSection[];
    parameters: DesignParams;
    highlightedParamKey?: string | null;
    searchActive?: boolean;
    getRangeProps: (field: ResolvedUiField) => RangeProps;
    getCadTone: (field: ResolvedUiField) => CadTone;
    onDraftValue: (key: string, value: ParamValue) => void;
    onUpdate: (key: string, value: ParamValue) => void;
    onPickImage: (key: string) => Promise<void> | void;
    onSetFocusedControl: (primitiveId: string | null, parameterKey: string | null) => void;
    onClearFocusedControl: (event: MouseEvent | FocusEvent) => void;
  } = $props();

  let collapseOverrides = $state<Record<string, boolean>>({});

  function isCollapsed(section: ParameterOwnershipSection): boolean {
    if (searchActive) return false;
    const manualOverride = collapseOverrides[section.sectionId];
    if (manualOverride !== undefined) return manualOverride;
    if (section.selected) return false;
    return section.collapsed;
  }

  function toggle(section: ParameterOwnershipSection) {
    collapseOverrides = {
      ...collapseOverrides,
      [section.sectionId]: !isCollapsed(section),
    };
  }
</script>

{#if sections.length > 0}
  <div class="ownership-sections" data-testid="parameter-ownership-sections">
    {#each sections as section (section.sectionId)}
      {@const collapsed = isCollapsed(section)}
      <section
        class="ownership-section"
        class:ownership-section-selected={section.selected}
        data-testid="parameter-ownership-section"
        data-section-id={section.sectionId}
        data-collapsed={collapsed}
        data-selected={section.selected}
      >
        <button
          type="button"
          class="ownership-section__head"
          aria-expanded={!collapsed}
          aria-label={`Toggle ${section.label} parameters`}
          onclick={() => toggle(section)}
        >
          <span>{section.label}</span>
          <span class="ownership-section__count">{section.fields.length} PARAMS</span>
          <span class="ownership-section__toggle">{collapsed ? 'EXPAND' : 'COLLAPSE'}</span>
        </button>

        {#if !collapsed}
          <div class="ownership-section__fields">
            {#each section.visibleFields as field (field.key)}
              <ParamPanelControlField
                elementId={field.key}
                {field}
                value={parameters[field.key]}
                rangeProps={field.type === 'range' || field.type === 'number' ? getRangeProps(field) : null}
                editable={!field.frozen}
                frozen={field.frozen}
                autoField={field._auto}
                focused={section.selected}
                highlighted={highlightedParamKey === field.key}
                cadTone={getCadTone(field)}
                onDraftValue={(nextValue) => onDraftValue(field.key, nextValue)}
                onUpdate={(nextValue) => onUpdate(field.key, nextValue)}
                onPickImage={() => onPickImage(field.key)}
                onMouseEnter={() => onSetFocusedControl(null, field.key)}
                onMouseLeave={onClearFocusedControl}
                onFocusIn={() => onSetFocusedControl(null, field.key)}
                onFocusOut={onClearFocusedControl}
              />
            {/each}
          </div>
        {/if}
      </section>
    {/each}
  </div>
{:else}
  <div class="ownership-empty">No controls match your search.</div>
{/if}

<style>
  .ownership-sections {
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }

  .ownership-section {
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    min-width: 0;
    border: 1px solid var(--bg-300);
    background: var(--bg-100);
    overflow: hidden;
  }

  .ownership-section-selected {
    border-color: var(--secondary);
    background: color-mix(in srgb, var(--secondary) 7%, var(--bg-100));
  }

  .ownership-section__head {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-width: 0;
    padding: 8px 10px;
    border: 0;
    border-radius: 0;
    background: var(--bg-200);
    color: var(--text);
    font: inherit;
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-align: left;
    text-transform: uppercase;
    cursor: pointer;
    overflow: hidden;
  }

  .ownership-section__head > span:first-child {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .ownership-section__count {
    color: var(--text-dim);
    font-size: 0.55rem;
  }

  .ownership-section__toggle {
    color: var(--secondary);
    font-size: 0.53rem;
  }

  .ownership-section__fields {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 220px), 1fr));
    gap: 12px;
    min-width: 0;
    padding: 10px;
    overflow: hidden;
  }

  .ownership-empty {
    padding: 20px;
    color: var(--text-dim);
    font-size: 0.7rem;
    font-style: italic;
    overflow: hidden;
  }
</style>
