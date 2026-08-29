import type { Advisory } from '../types/domain';

export type AdvisoryControlLabel = {
  primitiveId: string;
  label: string;
};

export type AdvisorySummary = Advisory & {
  count: number;
  affectedLabels: string[];
};

export function summarizeAdvisories(
  advisories: Advisory[],
  controls: AdvisoryControlLabel[],
): AdvisorySummary[] {
  const controlLabels = new Map(controls.map((control) => [control.primitiveId, control.label]));
  const summaries = new Map<string, AdvisorySummary>();

  for (const advisory of advisories) {
    const isManual = advisory.advisoryId.startsWith('advisory-manual-');
    const groupKey = isManual
      ? advisory.advisoryId
      : `${advisory.severity}\u0000${advisory.label}\u0000${advisory.message}`;
    const primitiveIds = [...new Set(advisory.primitiveIds || [])];
    const affectedLabels = primitiveIds
      .map((primitiveId) => controlLabels.get(primitiveId) || primitiveId)
      .filter(Boolean);
    const existing = summaries.get(groupKey);

    if (!existing) {
      summaries.set(groupKey, {
        ...advisory,
        primitiveIds,
        count: 1,
        affectedLabels: [...new Set(affectedLabels)],
      });
      continue;
    }

    existing.count += 1;
    existing.primitiveIds = [...new Set([...(existing.primitiveIds || []), ...primitiveIds])];
    existing.affectedLabels = [...new Set([...existing.affectedLabels, ...affectedLabels])];
  }

  return [...summaries.values()];
}
