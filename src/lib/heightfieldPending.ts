import type { DesignParams, UiSpec } from './types/domain';

export type PendingHeightfieldImage = {
  key: string;
  label: string;
};

export function pendingHeightfieldImages(
  macroCode: string,
  uiSpec: UiSpec | null | undefined,
  params: DesignParams | null | undefined,
): PendingHeightfieldImage[] {
  const referencedKeys = new Set<string>();
  const pattern = /\(heightfield\s+([A-Za-z_#][A-Za-z0-9_#-]*)/g;
  for (const match of macroCode.matchAll(pattern)) {
    referencedKeys.add(match[1]);
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

export function pendingHeightfieldStatus(pending: PendingHeightfieldImage[]): string {
  const labels = pending.map((field) => field.label).join(', ');
  return `Heightfield pending image selection: ${labels}. Select image, then apply.`;
}
