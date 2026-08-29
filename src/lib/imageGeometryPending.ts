import type { DesignParams, UiSpec } from './types/domain';

export type PendingImageGeometry = {
  key: string;
  label: string;
};

export function pendingImageGeometry(
  macroCode: string,
  uiSpec: UiSpec | null | undefined,
  params: DesignParams | null | undefined,
): PendingImageGeometry[] {
  const imageFieldKeys = new Set(
    (uiSpec?.fields ?? []).filter((field) => field.type === 'image').map((field) => field.key),
  );
  const referencedKeys = new Set<string>();
  const pattern = /\((?:protrude|extrude|heightfield)\s+([A-Za-z_#][A-Za-z0-9_#-]*)/g;
  for (const match of macroCode.matchAll(pattern)) {
    if (imageFieldKeys.has(match[1])) referencedKeys.add(match[1]);
  }
  if (referencedKeys.size === 0) return [];

  return (uiSpec?.fields ?? [])
    .filter((field) => field.type === 'image' && referencedKeys.has(field.key))
    .filter((field) => {
      const value = params?.[field.key];
      return value == null || (typeof value === 'string' && value.trim() === '');
    })
    .map((field) => ({ key: field.key, label: field.label || field.key }));
}

export function pendingImageGeometryStatus(pending: PendingImageGeometry[]): string {
  const labels = pending.map((field) => field.label).join(', ');
  return `Image geometry pending selection: ${labels}. Select image, then apply.`;
}
