/** Pure UI projection for backend-owned authored verification evidence. */

import type {
  AuthoredVerifyCheck,
  AuthoredVerifyCheckStatus,
  AuthoredVerifyValue,
  StructuralVerificationResult,
} from '../types/domain';

export type AuthoredVerifyChipTone = 'green' | 'red' | 'amber' | 'neutral';

export type AuthoredVerifyChip = {
  id: string;
  label: string;
  status: AuthoredVerifyCheckStatus;
  tone: AuthoredVerifyChipTone;
  message: string;
  stableNodeId: string | null;
};

export function deriveAuthoredVerifyChips(
  result: StructuralVerificationResult | null | undefined,
): AuthoredVerifyChip[] {
  return (result?.authoredVerifyChecks ?? []).map((check) => {
    const label = normalizeAuthoredVerifyTag(check);
    const stableNodeId = check.stableNodeId ?? null;
    return {
      id: stableNodeId ?? `authored-verify:${label}`,
      label,
      status: check.status,
      tone: authoredVerifyTone(check),
      message: formatAuthoredVerifyChipMessage(check),
      stableNodeId,
    };
  });
}

function authoredVerifyTone(check: AuthoredVerifyCheck): AuthoredVerifyChipTone {
  if (check.status === 'passed') return 'green';
  if (check.status === 'skipped') return 'neutral';
  if (check.status === 'failed' && check.severity === 'warning') return 'amber';
  return 'red';
}

function normalizeAuthoredVerifyTag(check: AuthoredVerifyCheck): string {
  return `${check.tag ?? ''}`.trim() || 'verify';
}

function formatAuthoredVerifyChipMessage(check: AuthoredVerifyCheck): string {
  const intent = `${check.intent ?? ''}`.trim();
  if (check.status === 'skipped') {
    const reason = `${check.skipReason ?? check.message}`.trim();
    const condition = `${check.condition ?? ''}`.trim();
    return [intent, condition ? `when ${condition}: false` : '', reason].filter(Boolean).join(' — ');
  }
  const expected = formatAuthoredVerifyValue(check.expected);
  const actual = formatAuthoredVerifyValue(check.actual);
  const comparator = `${check.comparator ?? ''}`.trim();
  if (!expected || !actual || !comparator) {
    return [intent, check.message].filter(Boolean).join(' — ');
  }

  const metric = [`${check.metricSource ?? ''}`.trim(), `${check.metricKey ?? ''}`.trim()]
    .filter(Boolean)
    .join(' ');
  const evidence = `${metric ? `${metric} ` : ''}expected ${comparator} ${expected}; actual ${actual}`;
  return [intent, evidence].filter(Boolean).join(' — ');
}

function formatAuthoredVerifyValue(value: AuthoredVerifyValue | null | undefined): string | null {
  if (!value) return null;
  switch (value.kind) {
    case 'number':
      return Number.isFinite(value.value) ? `${value.value}` : null;
    case 'boolean':
      return value.value ? 'true' : 'false';
    case 'text':
      return value.value;
    default:
      return null;
  }
}
