export type CodeModalCommitState = 'idle' | 'applying' | 'committing' | 'translating';

export function seedCodeModalDraftField(
  value: string | null | undefined,
  fallback: string,
): string {
  const trimmed = value?.trim();
  return trimmed ? trimmed : fallback;
}

export function shouldReseedCodeModalDraftFields(
  previousScopeKey: string,
  nextScopeKey: string,
  commitState: CodeModalCommitState,
): boolean {
  return commitState === 'idle' && previousScopeKey !== nextScopeKey;
}
