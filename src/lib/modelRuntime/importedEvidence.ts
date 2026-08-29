import type { ManifestBounds, ModelManifest } from '../types/domain';

export function isForeignCadEvidence(manifest: ModelManifest | null): manifest is ModelManifest {
  return manifest?.sourceKind === 'importedFcstd' || manifest?.sourceKind === 'importedStep';
}

function dimensions(bounds: ManifestBounds | null | undefined): [number, number, number] | null {
  if (!bounds) return null;
  return [
    bounds.xMax - bounds.xMin,
    bounds.yMax - bounds.yMin,
    bounds.zMax - bounds.zMin,
  ];
}

function formatDimensions(value: [number, number, number] | null): string {
  if (!value) return 'UNKNOWN';
  return `${value.map((dimension) => Number(dimension.toFixed(3))).join(' × ')} mm`;
}

export function buildImportedEvidence(manifest: ModelManifest): string {
  const parts = manifest.parts ?? [];
  const overallBounds = parts.reduce<ManifestBounds | null>((combined, part) => {
    if (!part.bounds) return combined;
    if (!combined) return { ...part.bounds };
    return {
      xMin: Math.min(combined.xMin, part.bounds.xMin),
      xMax: Math.max(combined.xMax, part.bounds.xMax),
      yMin: Math.min(combined.yMin, part.bounds.yMin),
      yMax: Math.max(combined.yMax, part.bounds.yMax),
      zMin: Math.min(combined.zMin, part.bounds.zMin),
      zMax: Math.max(combined.zMax, part.bounds.zMax),
    };
  }, null);
  const lines = [
    `IMPORTED ${manifest.sourceKind === 'importedFcstd' ? 'FCSTD' : 'STEP'} — READ ONLY`,
    `FILE  ${manifest.document.sourcePath ?? 'UNKNOWN'}`,
    `PRINT SIZE  ${formatDimensions(dimensions(overallBounds))}`,
    `PARTS  ${manifest.parts?.length ?? 0}`,
    '',
  ];
  parts.forEach((part, index) => {
    lines.push(`${index + 1}. ${part.label || part.freecadObjectName || part.partId}`);
    lines.push(`   TYPE  ${part.kind}`);
    lines.push(`   SIZE  ${formatDimensions(dimensions(part.bounds))}`);
    if (part.volume != null) lines.push(`   VOLUME  ${Number(part.volume.toFixed(3))} mm³`);
    if (part.area != null) lines.push(`   AREA  ${Number(part.area.toFixed(3))} mm²`);
  });
  const warnings = [...new Set([...(manifest.document.warnings ?? []), ...(manifest.warnings ?? [])])];
  if (warnings.length) lines.push('', 'WARNINGS', ...warnings.map((warning) => `- ${warning}`));
  return lines.join('\n');
}
