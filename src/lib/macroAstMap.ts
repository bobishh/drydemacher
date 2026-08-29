import type { DesignParams, ModelManifest, UiField, UiSpec } from './types/domain';
import { buildOwnershipSections } from './modelRuntime/ownershipSections';
import type { AuthoringGraph, AuthoringGraphInputPort } from './authoringGraph';

export type MacroAstMapNodeKind =
  | 'model'
  | 'part'
  | 'port'
  | 'param'
  | 'verify'
  | 'expression'
  | 'operation'
  | 'readonly';

export type MacroAstSourceRange = { startByte: number; endByte: number };

export type MacroAstMapNode = {
  id: string;
  kind: MacroAstMapNodeKind;
  label: string;
  value: string | number | boolean | null;
  fieldKey?: string;
  syntaxVariant?: string;
  syntaxLabel?: string;
  title?: string;
  inputPorts?: AuthoringGraphInputPort[];
  /** Exact byte range of this node in the macro source, when known. */
  sourceRange?: MacroAstSourceRange;
  children: MacroAstMapNode[];
};

/** Shape of `macro_ast_source_map` results (backend command). */
export type MacroAstSourceMapEntry = {
  id: string;
  kind: string;
  label: string;
  startByte: number;
  endByte: number;
};

export type MacroAstMapProjection = {
  root: MacroAstMapNode;
};

export type MacroAstSearchEntry = {
  nodeId: string;
  ownerNodeId: string;
  nodeKind: MacroAstMapNodeKind;
  label: string;
  ownerLabel: string;
  fieldKey?: string;
  searchText: string;
};

type MacroAstMapInput = {
  macroCode?: string;
  modelManifest?: ModelManifest | null;
  uiSpec?: UiSpec | null;
  parameters?: DesignParams;
  sourceNodes?: MacroAstSourceMapEntry[] | null;
  authoringGraph?: AuthoringGraph | null;
};

type MacroAstPart = {
  partId: string;
  label: string;
  parameterKeys?: string[] | null;
};

function normalizeFieldLabel(field: UiField): string {
  return `${field.label ?? field.key ?? ''}`.trim() || field.key;
}

function formatValue(value: unknown): string {
  if (value === null || value === undefined || value === '') return 'Unset';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return `${value}`;
  try {
    return JSON.stringify(value);
  } catch {
    return `${value}`;
  }
}

function synthesizeParts(fields: UiField[]): MacroAstPart[] {
  return [
    {
      partId: 'macro-part',
      label: 'Parameter Region',
      parameterKeys: fields.map((field) => field.key),
    },
  ];
}

function normalizeSyntaxVariant(value: string | null | undefined): string {
  const normalized = `${value ?? ''}`.trim().toLowerCase();
  return normalized.replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'unknown';
}

function normalizeSearchText(value: string): string {
  return value
    .toLocaleLowerCase()
    .replace(/[^a-z0-9]+/g, ' ')
    .trim()
    .replace(/\s+/g, ' ');
}

const GEOMETRY_VALUE_KINDS = new Set(['Sketch', 'Path', 'Frame', 'Mesh', 'Compound', 'Shape', 'Solid']);

function typedNodeLabel(path: string, operation: string | null | undefined): string {
  const segments = path.split('/').filter(Boolean);
  const bindingIndex = Math.max(segments.lastIndexOf('bindings'), segments.lastIndexOf('shapes'));
  if (bindingIndex >= 0 && segments[bindingIndex + 1]) {
    return segments[bindingIndex + 1]!.replace(/~1/g, '/').replace(/~0/g, '~');
  }
  return operation || segments.at(-1)?.replace(/~1/g, '/').replace(/~0/g, '~') || 'expression';
}

function projectTypedNode(node: AuthoringGraph['astNodes'][number]): MacroAstMapNode | null {
  if (!node.partId || node.kind === 'Part' || node.kind === 'Param' || node.kind === 'Reference') {
    return null;
  }
  const sourceAddressable = node.sourceAddressable;
  const geometryValue = GEOMETRY_VALUE_KINDS.has(node.valueKind);
  const kind: MacroAstMapNodeKind = !sourceAddressable
    ? 'readonly'
    : geometryValue || node.operation
      ? 'operation'
      : 'expression';
  return {
    id: node.stableNodeKey,
    kind,
    label: typedNodeLabel(node.path, node.operation),
    value: node.valueKind,
    syntaxVariant: normalizeSyntaxVariant(node.operation || node.valueKind),
    syntaxLabel: kind === 'readonly' ? 'READ ONLY' : (node.operation || node.valueKind).toUpperCase(),
    title: node.nonEditableReason ?? undefined,
    inputPorts: node.inputPorts,
    children: [],
  };
}

export function buildMacroAstSearchIndex(
  projection: MacroAstMapProjection,
): MacroAstSearchEntry[] {
  const entries: MacroAstSearchEntry[] = [];
  const visit = (node: MacroAstMapNode, owner: MacroAstMapNode) => {
    const currentOwner = node.kind === 'part' ? node : owner;
    entries.push({
      nodeId: node.id,
      ownerNodeId: currentOwner.id,
      nodeKind: node.kind,
      label: node.label,
      ownerLabel: currentOwner.label,
      fieldKey: node.fieldKey,
      searchText: normalizeSearchText(
        [
          node.id,
          node.label,
          node.fieldKey,
          node.syntaxLabel,
          currentOwner.id,
          currentOwner.label,
        ]
          .filter(Boolean)
          .join(' '),
      ),
    });
    for (const child of node.children ?? []) visit(child, currentOwner);
  };
  visit(projection.root, projection.root);
  return entries;
}

export function searchMacroAstMap(
  index: readonly MacroAstSearchEntry[],
  query: string,
  limit = 12,
): MacroAstSearchEntry[] {
  const normalizedQuery = normalizeSearchText(query);
  if (!normalizedQuery) return [];
  const compactQuery = normalizedQuery.replace(/\s/g, '');
  const score = (entry: MacroAstSearchEntry) => {
    const label = normalizeSearchText(entry.label);
    const fieldKey = normalizeSearchText(entry.fieldKey ?? '');
    const nodeId = normalizeSearchText(entry.nodeId);
    if ([label, fieldKey, nodeId].includes(normalizedQuery)) return 0;
    if ([label, fieldKey, nodeId].some((value) => value.startsWith(normalizedQuery))) return 1;
    if (entry.searchText.includes(normalizedQuery)) return 2;
    return entry.searchText.replace(/\s/g, '').includes(compactQuery) ? 3 : null;
  };

  return index
    .map((entry) => ({ entry, score: score(entry) }))
    .filter((candidate): candidate is { entry: MacroAstSearchEntry; score: number } =>
      candidate.score !== null,
    )
    .sort(
      (left, right) =>
        left.score - right.score ||
        Number(right.entry.nodeKind === 'param') - Number(left.entry.nodeKind === 'param') ||
        left.entry.label.localeCompare(right.entry.label),
    )
    .slice(0, Math.max(0, limit))
    .map(({ entry }) => entry);
}

function sourceRangeFor(
  entries: Map<string, MacroAstSourceMapEntry>,
  ...ids: string[]
): MacroAstSourceRange | undefined {
  for (const id of ids) {
    const entry = entries.get(id);
    if (entry) return { startByte: entry.startByte, endByte: entry.endByte };
  }
  return undefined;
}

function paramNode(
  idPrefix: string,
  field: UiField,
  value: string | number | boolean | null,
): MacroAstMapNode {
  const fieldSyntaxVariant = normalizeSyntaxVariant(field.type);
  return {
    id: `${idPrefix}/param:${field.key}`,
    kind: 'param',
    label: normalizeFieldLabel(field),
    value,
    fieldKey: field.key,
    syntaxVariant: fieldSyntaxVariant,
    syntaxLabel: fieldSyntaxVariant.toUpperCase(),
    children: [],
  };
}

/**
 * Splices an edited slice back into a base document.
 *
 * `start`/`end` are clamped to `[0, base.length]`; an inverted range
 * (`start > end`) collapses to a zero-width point at the clamped `start` so
 * the result is a pure insertion instead of a throw or a reordering.
 */
/**
 * Locates the id of the part node (direct child of `root`) whose param
 * children include `fieldKey`. Used by focus flows to decide which part must
 * be auto-expanded before a param control can receive focus.
 */
export function findOwningPartId(root: MacroAstMapNode, fieldKey: string | undefined | null): string | null {
  if (!fieldKey) return null;
  for (const part of root.children ?? []) {
    if (part.children?.some((param) => param.fieldKey === fieldKey)) return part.id;
  }
  return null;
}

export function spliceMacroSource(base: string, start: number, end: number, slice: string): string {
  const length = base.length;
  const clampedStart = Math.max(0, Math.min(start, length));
  const clampedEnd = Math.max(clampedStart, Math.min(end, length));
  return base.slice(0, clampedStart) + slice + base.slice(clampedEnd);
}

export function buildMacroAstMapProjection(input: MacroAstMapInput): MacroAstMapProjection {
  const sourceEntries = new Map(
    (input.sourceNodes ?? []).map((entry) => [entry.id, entry]),
  );
  const fields = Array.isArray(input.uiSpec?.fields)
    ? input.uiSpec.fields.filter((field): field is UiField => Boolean(field))
    : [];
  const fieldByKey = new Map(fields.map((field) => [field.key, field]));
  const manifestParts = input.modelManifest?.parts || [];
  const partById = new Map(manifestParts.map((part) => [part.partId, part]));
  const parts = (input.modelManifest?.parts?.length ? input.modelManifest.parts : synthesizeParts(fields)).map(
    (part) => ({
      partId: part.partId,
      label: `${part.label ?? part.partId}`.trim() || part.partId,
      parameterKeys: Array.isArray(part.parameterKeys) ? [...part.parameterKeys] : [],
    }),
  );

  const ownershipSections = input.modelManifest
    ? buildOwnershipSections({
        manifest: input.modelManifest,
        fields,
        selectedTarget: null,
        searchQuery: '',
      })
    : [];
  const ownershipById = new Map(ownershipSections.map((section) => [section.sectionId, section]));
  const valueOf = (field: UiField) =>
    (input.parameters?.[field.key] ?? null) as string | number | boolean | null;

  const partNodes: MacroAstMapNode[] = parts.map((part, partIndex) => {
    const ownFields = input.modelManifest
      ? ownershipSections
          .filter((section) => section.partIds.includes(part.partId))
          .flatMap((section) => section.fields)
      : part.parameterKeys
          .map((key) => fieldByKey.get(key))
          .filter((field): field is UiField => Boolean(field));
    const typedChildren = (input.authoringGraph?.astNodes ?? [])
      .filter((node) => node.partId === part.partId)
      .map(projectTypedNode)
      .filter((node): node is MacroAstMapNode => Boolean(node));
    return {
      id: `part:${part.partId}`,
      kind: 'part',
      label: part.label || `Part ${partIndex + 1}`,
      value: null,
      sourceRange: sourceRangeFor(
        sourceEntries,
        `part:${part.partId}`,
        `feature:${part.partId}`,
      ),
      syntaxVariant: normalizeSyntaxVariant(partById.get(part.partId)?.kind ?? 'part'),
      syntaxLabel: normalizeSyntaxVariant(partById.get(part.partId)?.kind ?? 'part').toUpperCase(),
      children: [
        ...ownFields.map((field) => paramNode(`part:${part.partId}`, field, valueOf(field))),
        ...typedChildren,
      ],
    };
  });

  const sharedFields = input.modelManifest
    ? (ownershipById.get('model:parameters')?.fields ?? [])
    : [];
  const sharedGroup: MacroAstMapNode[] = sharedFields.length
    ? [
        {
          id: 'shared-params',
          kind: 'part',
          label: 'Model Params',
          value: null,
          syntaxVariant: 'shared',
          syntaxLabel: 'SHARED',
          children: sharedFields.map((field) => paramNode('shared', field, valueOf(field))),
        },
      ]
    : [];
  const verifyNodes: MacroAstMapNode[] = (input.sourceNodes ?? [])
    .filter((entry) => entry.kind === 'verify')
    .map((entry, index) => {
      const tag = `${entry.label || ''}`.trim();
      return {
        // Key by tag so an authored verify chip (`verify:<tag>`) focuses this
        // node; fall back to the source-map id when the clause has no tag.
        id: tag ? `verify:${tag}` : entry.id || `verify:${index}`,
        kind: 'verify' as const,
        label: tag || `verify ${index + 1}`,
        value: null,
        syntaxVariant: 'verify',
        syntaxLabel: 'VERIFY',
        sourceRange: { startByte: entry.startByte, endByte: entry.endByte },
        children: [],
      } satisfies MacroAstMapNode;
    });

  const root: MacroAstMapNode = {
    id: 'macro-root',
    kind: 'model',
    label: 'Macro Root',
    value: null,
    syntaxVariant: 'model',
    syntaxLabel: 'MODEL',
    sourceRange: sourceRangeFor(sourceEntries, 'model'),
    children: [...verifyNodes, ...sharedGroup, ...partNodes],
  };

  return { root };
}
