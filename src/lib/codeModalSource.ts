export type CodeModalSourceAuthority = 'bound' | 'draft';

export function resolveCodeModalSource(input: {
  activeRenderSource?: string;
  boundSource: string;
  isActiveRenderDraft: boolean;
}): { source: string; authority: CodeModalSourceAuthority } {
  if (input.isActiveRenderDraft && typeof input.activeRenderSource === 'string') {
    return {
      source: input.activeRenderSource,
      authority: 'draft',
    };
  }
  return {
    source: input.boundSource,
    authority: 'bound',
  };
}
