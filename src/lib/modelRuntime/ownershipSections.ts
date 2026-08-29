import type {
  DesignParams,
  ArtifactBundle,
  ModelManifest,
  ResolvedUiField,
  UiField,
} from '../types/domain';
import type { ContextSelectionTarget } from './contextualEditing';
import type { MaterializedSemanticControl } from './semanticControls';

export const OWNERSHIP_DENSE_THRESHOLD = 6;

export type ParameterOwnershipSection = {
  sectionId: string;
  label: string;
  partIds: string[];
  fields: ResolvedUiField[];
  visibleFields: ResolvedUiField[];
  collapsed: boolean;
  selected: boolean;
};

type OwnershipInput = {
  manifest: ModelManifest | null;
  fields: ResolvedUiField[];
  selectedTarget: ContextSelectionTarget | null;
  searchQuery: string;
};

function matchesSearch(field: ResolvedUiField, query: string): boolean {
  if (!query) return true;
  return `${field.key} ${field.label}`.toLowerCase().includes(query);
}

function exactSelectionKeys(target: ContextSelectionTarget | null): Set<string> {
  if (!target || target.kind === 'global') return new Set();
  return new Set(target.parameterKeys || []);
}

export function buildOwnershipSections({
  manifest,
  fields,
  selectedTarget,
  searchQuery,
}: OwnershipInput): ParameterOwnershipSection[] {
  if (!manifest || fields.length === 0) return [];

  const fieldsByKey = new Map(fields.map((field) => [field.key, field]));
  const claims = new Map<string, string[]>();
  for (const part of manifest.parts || []) {
    for (const key of part.parameterKeys || []) {
      if (!fieldsByKey.has(key)) continue;
      const owners = claims.get(key) ?? [];
      if (!owners.includes(part.partId)) owners.push(part.partId);
      claims.set(key, owners);
    }
  }

  const query = searchQuery.trim().toLowerCase();
  const selectedKeys = exactSelectionKeys(selectedTarget);
  const selectedPartId = selectedTarget?.partId ?? null;
  const hasContextSelection = Boolean(selectedTarget && selectedTarget.kind !== 'global');
  const sections: ParameterOwnershipSection[] = [];

  const addSection = (sectionId: string, label: string, partIds: string[], ownedFields: ResolvedUiField[]) => {
    if (ownedFields.length === 0) return;
    const selected =
      selectedKeys.size > 0 &&
      ownedFields.some((field) => selectedKeys.has(field.key)) &&
      (!selectedPartId || partIds.length === 0 || partIds.includes(selectedPartId));
    const matchedFields = ownedFields.filter((field) => matchesSearch(field, query));
    if (query && matchedFields.length === 0) return;
    const exactFields = selectedKeys.size > 0
      ? ownedFields.filter((field) => selectedKeys.has(field.key))
      : ownedFields;
    sections.push({
      sectionId,
      label,
      partIds,
      fields: ownedFields,
      visibleFields: query ? matchedFields : selected ? exactFields : ownedFields,
      collapsed: query
        ? false
        : hasContextSelection
          ? !selected
          : ownedFields.length > OWNERSHIP_DENSE_THRESHOLD,
      selected,
    });
  };

  const modelFields = fields.filter((field) => (claims.get(field.key)?.length ?? 0) !== 1);
  addSection('model:parameters', 'Model Params', [], modelFields);

  for (const part of manifest.parts || []) {
    const partFields = (part.parameterKeys || [])
      .filter((key) => claims.get(key)?.length === 1)
      .map((key) => fieldsByKey.get(key))
      .filter((field): field is ResolvedUiField => Boolean(field));
    const partFieldKeys = new Set(partFields.map((field) => field.key));
    const allocatedKeys = new Set<string>();
    const namedGroups = (manifest.parameterGroups || [])
      .filter(
        (group) =>
          group.groupId !== `part:${part.partId}` &&
          group.groupId !== 'model:parameters' &&
          (group.partIds || []).length === 1 &&
          (group.partIds || []).includes(part.partId),
      )
      .sort(
        (left, right) =>
          (left.order ?? Number.MAX_SAFE_INTEGER) - (right.order ?? Number.MAX_SAFE_INTEGER) ||
          left.groupId.localeCompare(right.groupId),
      );

    for (const group of namedGroups) {
      const groupFields = (group.parameterKeys || [])
        .filter((key) => partFieldKeys.has(key) && !allocatedKeys.has(key))
        .map((key) => fieldsByKey.get(key))
        .filter((field): field is ResolvedUiField => Boolean(field));
      for (const field of groupFields) allocatedKeys.add(field.key);
      addSection(group.groupId, group.label || group.groupId, [part.partId], groupFields);
    }

    const remainingPartFields = partFields.filter((field) => !allocatedKeys.has(field.key));
    addSection(
      `part:${part.partId}`,
      part.label || part.partId,
      [part.partId],
      remainingPartFields,
    );
  }

  if (!hasContextSelection) return sections;
  return [...sections].sort((left, right) => Number(right.selected) - Number(left.selected));
}

type ProvenanceControlsInput = {
  manifest: ModelManifest | null;
  runtime?: Pick<ArtifactBundle, 'engineKind' | 'sourceLanguage'> | null;
  fields: UiField[];
  parameters: DesignParams;
  target: ContextSelectionTarget | null;
};

function primitiveKind(field: UiField): MaterializedSemanticControl['kind'] {
  if (field.type === 'checkbox') return 'toggle';
  if (field.type === 'select' || field.type === 'image') return 'choice';
  return 'number';
}

export function provenanceOverlayControls({
  manifest,
  runtime = null,
  fields,
  parameters,
  target,
}: ProvenanceControlsInput): MaterializedSemanticControl[] {
  const generatedEcky =
    manifest?.sourceKind === 'generated' &&
    (
      manifest.sourceLanguage === 'ecky' ||
      manifest.engineKind === 'ecky' ||
      runtime?.sourceLanguage === 'ecky' ||
      runtime?.engineKind === 'ecky'
    );
  if (!generatedEcky || !target || target.kind === 'global' || !target.editable) return [];

  const exactKeys = new Set(target.parameterKeys || []);
  if (exactKeys.size === 0) return [];

  return fields
    .filter((field) => exactKeys.has(field.key))
    .map((field, order) => ({
      primitiveId: `ast-param:${field.key}`,
      label: field.label || field.key,
      kind: primitiveKind(field),
      source: 'generated',
      editable: !field.frozen,
      partIds: target.partId ? [target.partId] : [],
      order,
      rawField: field,
      bindings: [{ parameterKey: field.key, scale: 1, offset: 0, min: null, max: null }],
      value: parameters[field.key] ?? null,
    }));
}

export function provenanceOverlayPatch(
  controls: MaterializedSemanticControl[],
  primitiveId: string,
  value: MaterializedSemanticControl['value'],
): DesignParams {
  const control = controls.find((candidate) => candidate.primitiveId === primitiveId);
  const key = control?.rawField?.key;
  return key ? { [key]: value } : {};
}
