<script lang="ts" module>
  import type { ParamValue, ResolvedUiField } from './types/domain';

  /**
   * The document skin (filesystem-project-mirror task 6.1) is an alternate
   * LAYOUT over the existing AstMap projection. It shares the one projection
   * built by `buildMacroAstMapProjection` (same stable node ids) and emits
   * the SAME patch intents as `MacroAstMap.svelte`
   * (`onUpdate` / `onDraftValue` / `onApplyMacroCode` / `onControlFocusChange`).
   *
   * It is NOT a separate editor with its own identity model — see the fsm
   * design "Literate projection note" and macro-ast-map-editor design.
   */
  export type AstDocumentCadTone = 'neutral' | 'size' | 'x' | 'y' | 'z' | 'angle' | 'state' | 'mode';
  type RangeLikeField = Extract<ResolvedUiField, { type: 'range' | 'number' }>;
</script>

<script lang="ts">
  import { onDestroy } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import ParamPanelControlField from './components/ParamPanelControlField.svelte';
  import {
    buildMacroAstMapProjection,
    spliceMacroSource,
    type MacroAstMapNode,
  } from './macroAstMap';
  import { macroAstSourceMap } from './tauri/client';
  import type { DesignParams, ModelManifest, UiSpec } from './types/domain';

  let {
    macroCode = '',
    modelManifest = null,
    uiSpec = null,
    parameters = {},
    fields = [],
    highlightedParamKey = null,
    liveApply = false,
    focusNodeId = null,
    onFocusNodeHandled,
    onApplyMacroCode = undefined,
    onDraftValue,
    onUpdate,
    onControlFocusChange,
  }: {
    macroCode?: string;
    modelManifest?: ModelManifest | null;
    uiSpec?: UiSpec | null;
    parameters?: DesignParams;
    fields?: ResolvedUiField[];
    highlightedParamKey?: string | null;
    liveApply?: boolean;
    focusNodeId?: string | null;
    onFocusNodeHandled?: () => void;
    onApplyMacroCode?: (code: string) => Promise<unknown>;
    onDraftValue?: (key: string, value: ParamValue) => void;
    onUpdate?: (key: string, value: ParamValue) => void;
    onControlFocusChange?: (primitiveId: string | null, parameterKey: string | null) => void;
  } = $props();

  let sourceNodes = $state<Awaited<ReturnType<typeof macroAstSourceMap>> | null>(null);
  let documentRoot = $state<HTMLElement | null>(null);

  // Fetch byte-accurate node identity from the existing backend command —
  // the same source the spatial map uses, so ids match across views.
  $effect(() => {
    const code = macroCode;
    if (!code || !code.trim()) {
      sourceNodes = null;
      return;
    }
    let cancelled = false;
    macroAstSourceMap(code)
      .then((nodes) => {
        if (!cancelled) sourceNodes = nodes;
      })
      .catch(() => {
        if (!cancelled) sourceNodes = null;
      });
    return () => {
      cancelled = true;
    };
  });

  // The single shared projection. Node ids here are identical to the ones
  // the spatial map renders for the same macro.
  const projection = $derived.by(() =>
    buildMacroAstMapProjection({
      macroCode,
      modelManifest,
      uiSpec,
      parameters,
      sourceNodes,
    }),
  );
  const fieldByKey = $derived.by(() => new Map(fields.map((field) => [field.key, field])));

  const hasContent = $derived(Boolean(macroCode && macroCode.trim()));

  function collectParamNodes(node: MacroAstMapNode, acc: MacroAstMapNode[] = []): MacroAstMapNode[] {
    if (node.kind === 'param') acc.push(node);
    for (const child of node.children ?? []) collectParamNodes(child, acc);
    return acc;
  }

  // Pending-params state: the macro and its part/structure render, but no
  // editable param node exists yet. Surfaced so the author sees "nothing to
  // tune here" instead of an apparently broken or fake-populated document.
  const paramNodeCount = $derived(hasContent ? collectParamNodes(projection.root).length : 0);

  function parseOptionalNumber(raw: number | '' | undefined): number | undefined {
    if (raw === null || raw === undefined || raw === '') return undefined;
    const number = Number(raw);
    return Number.isFinite(number) ? number : undefined;
  }

  function asNumber(value: ParamValue | undefined, fallback = 0): number {
    const numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : fallback;
  }

  function getRangeProps(field: RangeLikeField) {
    const rawVal = Number(parameters[field.key]);
    const val = Number.isFinite(rawVal) ? rawVal : 0;
    let min = parseOptionalNumber(field.min) ?? 0;
    if (field.minFrom && parameters[field.minFrom] !== undefined) {
      min = asNumber(parameters[field.minFrom], min);
    }
    let max = parseOptionalNumber(field.max) ?? Math.max(200, val * 4);
    if (field.maxFrom && parameters[field.maxFrom] !== undefined) {
      max = asNumber(parameters[field.maxFrom], max);
    }
    if (max < min) max = min;
    if (max === min) max = min + 1;
    const stepCandidate = parseOptionalNumber(field.step) ?? (max - min > 50 ? 1 : 0.1);
    return { min, max, step: Number(stepCandidate.toFixed(3)) };
  }

  function getCadTone(field: ResolvedUiField): AstDocumentCadTone {
    const signature = `${field.key} ${field.label}`.toLowerCase();
    const hasFragment = (...fragments: string[]) =>
      fragments.some((fragment) => signature.includes(fragment));
    if (field.type === 'checkbox') return 'state';
    if (field.type === 'image') return 'state';
    if (field.type === 'select') return 'mode';
    if (hasFragment('diameter') || hasFragment('dia')) return 'size';
    if (hasFragment('radius') || hasFragment('fillet')) return 'size';
    if (hasFragment('width') || hasFragment('x')) return 'x';
    if (hasFragment('depth') || hasFragment('z')) return 'z';
    if (hasFragment('height') || hasFragment('y')) return 'y';
    if (hasFragment('angle')) return 'angle';
    return 'neutral';
  }

  function firstSelectedPath(selection: string | string[] | null): string | null {
    if (!selection) return null;
    return Array.isArray(selection) ? (selection[0] ?? null) : selection;
  }

  function clearFocusedControl() {
    onControlFocusChange?.(null, null);
  }

  // Focus handling mirrors the spatial map: when a focus node id arrives
  // (e.g. from a verify chip or search), scroll the matching document block
  // into view, briefly emphasize it, and report handled. Tracked as reactive
  // state so the focus class is applied via class binding (Svelte-visible)
  // rather than imperative DOM mutation.
  let focusedNodeId = $state<string | null>(null);
  let focusTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const id = focusNodeId;
    if (!id || !documentRoot) {
      onFocusNodeHandled?.();
      return;
    }
    const target = documentRoot.querySelector<HTMLElement>(`[data-node-id="${CSS.escape(id)}"]`);
    if (target) target.scrollIntoView({ block: 'center', behavior: 'smooth' });
    focusedNodeId = id;
    if (focusTimer) clearTimeout(focusTimer);
    focusTimer = setTimeout(() => {
      focusedNodeId = null;
      focusTimer = null;
    }, 1200);
    onFocusNodeHandled?.();
  });

  // Source-backed nodes (model/part/verify) share the spatial map's patch
  // intent for source edits: dblclick hands the scoped source to the same
  // `onApplyMacroCode` flow the map uses (no string splicing in the renderer
  // beyond handing the existing macro to the shared editor).
  function handleNodeDblClick(node: MacroAstMapNode) {
    if (!node.sourceRange || !onApplyMacroCode) return;
    const slice = macroCode.slice(node.sourceRange.startByte, node.sourceRange.endByte);
    void onApplyMacroCode(spliceMacroSource(macroCode, node.sourceRange.startByte, node.sourceRange.endByte, slice));
  }

  function handleNodeKeyDown(event: KeyboardEvent, node: MacroAstMapNode) {
    if ((event.key === 'Enter' || event.key === ' ') && node.sourceRange && onApplyMacroCode) {
      event.preventDefault();
      handleNodeDblClick(node);
    }
  }

  onDestroy(() => {
    if (focusTimer) clearTimeout(focusTimer);
    clearFocusedControl();
  });
</script>

<div class="macro-ast-document" bind:this={documentRoot}>
  {#if !hasContent}
    <div class="macro-ast-document__empty" data-testid="ast-document-empty">
      <span class="macro-ast-document__empty-label">PENDING</span>
      <span class="macro-ast-document__empty-hint"
        >No macro source yet. Author a model to populate the document.</span
      >
    </div>
  {:else}
    {@render renderNode(projection.root, 0)}
    {#if paramNodeCount === 0}
      <div class="macro-ast-document__pending-params" data-testid="ast-document-pending-params">
        <span class="macro-ast-document__empty-label">PENDING PARAMS</span>
        <span class="macro-ast-document__empty-hint"
          >Structure rendered; no editable parameters found in this macro yet.</span
        >
      </div>
    {/if}
  {/if}
</div>

{#snippet renderNode(node: MacroAstMapNode, depth: number)}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -- a section is interactive (role="button") exactly when it carries a tabindex; the pairing is conditional but always correct -->
  <section
    class="macro-ast-document__node"
    class:macro-ast-document__node--model={node.kind === 'model'}
    class:macro-ast-document__node--part={node.kind === 'part'}
    class:macro-ast-document__node--port={node.kind === 'port'}
    class:macro-ast-document__node--param={node.kind === 'param'}
    class:macro-ast-document__node--verify={node.kind === 'verify'}
    class:macro-ast-document__node--sourced={Boolean(node.sourceRange && onApplyMacroCode)}
    class:macro-ast-document__node--focused={focusedNodeId === node.id}
    data-node-id={node.id}
    data-node-kind={node.kind}
    data-syntax-variant={node.syntaxVariant}
    style={`--doc-depth: ${depth};`}
    role={node.sourceRange && onApplyMacroCode ? 'button' : 'group'}
    tabindex={node.sourceRange && onApplyMacroCode ? 0 : undefined}
    aria-label={node.label}
    ondblclick={() => handleNodeDblClick(node)}
    onkeydown={(event) => handleNodeKeyDown(event, node)}
  >
    <header class="macro-ast-document__node-header">
      <span class="macro-ast-document__node-kind" aria-hidden="true">{node.kind}</span>
      <span class="macro-ast-document__node-label">{node.label.toLowerCase()}</span>
      {#if node.syntaxLabel}
        <span class="macro-ast-document__node-syntax">{node.syntaxLabel}</span>
      {/if}
      {#if node.sourceRange && onApplyMacroCode}
        <span class="macro-ast-document__node-hint" aria-hidden="true">dblclick: source</span>
      {/if}
    </header>

    {#if node.kind === 'param'}
      {@const field = node.fieldKey ? fieldByKey.get(node.fieldKey) : null}
      {#if field}
        <div class="macro-ast-document__control" data-param-key={field.key}>
          <ParamPanelControlField
            elementId={`doc-${field.key}`}
            {field}
            value={parameters[field.key]}
            rangeProps={field.type === 'range' || field.type === 'number' ? getRangeProps(field) : null}
            editable={!field.frozen}
            frozen={field.frozen}
            autoField={field._auto}
            highlighted={highlightedParamKey === field.key}
            cadTone={getCadTone(field)}
            {liveApply}
            compact={true}
            onDraftValue={(nextValue) => onDraftValue?.(field.key, nextValue)}
            onUpdate={(nextValue) => onUpdate?.(field.key, nextValue)}
            onPickImage={async () => {
              const file = await open({
                multiple: false,
                filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'svg'] }],
              });
              const selected = firstSelectedPath(file);
              if (selected) onUpdate?.(field.key, selected);
            }}
            onMouseEnter={() => onControlFocusChange?.(null, field.key)}
            onMouseLeave={clearFocusedControl}
            onFocusIn={() => onControlFocusChange?.(null, field.key)}
            onFocusOut={clearFocusedControl}
          />
        </div>
      {:else}
        <div class="macro-ast-document__value" aria-hidden="true">{node.value ?? '—'}</div>
      {/if}
    {/if}

    {#if node.children?.length}
      <div class="macro-ast-document__children">
        {#each node.children as child (child.id)}
          {@render renderNode(child, depth + 1)}
        {/each}
      </div>
    {/if}
  </section>
{/snippet}

<style>
  .macro-ast-document {
    display: block;
    width: 100%;
    height: 100%;
    min-height: 0;
    padding: 10px 12px;
    border: 1px solid var(--bg-300);
    background: var(--bg-100);
    color: var(--text-dim);
    font-size: 0.78rem;
    line-height: 1.45;
    overflow: hidden auto;
  }

  .macro-ast-document__empty {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 24px 16px;
    border: 1px dashed color-mix(in srgb, var(--secondary) 45%, var(--bg-300));
    background: var(--bg-200);
    color: var(--text-dim);
  }

  .macro-ast-document__empty-label {
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--secondary);
  }

  .macro-ast-document__empty-hint {
    font-size: 0.74rem;
    color: var(--text-dim);
  }

  .macro-ast-document__pending-params {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 10px;
    padding: 10px 12px;
    border: 1px dashed color-mix(in srgb, var(--secondary) 40%, var(--bg-300));
    background: var(--bg-200);
  }

  .macro-ast-document__node {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
    padding: 8px 10px;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    overflow: hidden;
  }

  .macro-ast-document__node--model {
    border-color: color-mix(in srgb, var(--secondary) 45%, var(--bg-300));
    background: color-mix(in srgb, var(--secondary) 8%, var(--bg-200));
  }

  .macro-ast-document__node--part {
    margin-left: calc(var(--doc-depth, 0) * 12px);
  }

  .macro-ast-document__node--param {
    margin-left: calc(var(--doc-depth, 0) * 12px);
    padding: 6px 8px;
    border-color: color-mix(in srgb, var(--primary) 35%, var(--bg-300));
  }

  .macro-ast-document__node--verify {
    border-color: color-mix(in srgb, var(--primary) 55%, var(--bg-300));
    background: color-mix(in srgb, var(--primary) 8%, var(--bg-200));
  }

  .macro-ast-document__node--sourced {
    cursor: pointer;
  }

  .macro-ast-document__node--sourced:focus-visible {
    outline: 1px solid var(--secondary);
    outline-offset: 1px;
  }

  .macro-ast-document__node--focused {
    border-color: var(--secondary);
    box-shadow: 0 0 0 1px var(--secondary);
  }

  .macro-ast-document__node-header {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }

  .macro-ast-document__node-kind {
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--secondary);
  }

  .macro-ast-document__node-label {
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .macro-ast-document__node-syntax {
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--text-dim);
    border: 1px solid var(--bg-300);
    padding: 1px 5px;
  }

  .macro-ast-document__node-hint {
    margin-left: auto;
    font-size: 0.56rem;
    letter-spacing: 0.08em;
    color: var(--text-dim);
    opacity: 0.7;
  }

  .macro-ast-document__control {
    min-width: 0;
  }

  .macro-ast-document__value {
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }

  .macro-ast-document__children {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
</style>
