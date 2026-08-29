const HIDDEN_WARNING_PREFIXES = [
  'Feature graph was not carried forward',
  'Hybrid poly BRep bridge:',
];

export function visibleManifestWarnings(warnings: Array<string | null | undefined>): string[] {
  const visible = new Set<string>();

  for (const warning of warnings) {
    const normalized = warning?.trim();
    if (!normalized) continue;
    if (HIDDEN_WARNING_PREFIXES.some((prefix) => normalized.startsWith(prefix))) continue;
    visible.add(normalized);
  }

  return [...visible];
}
