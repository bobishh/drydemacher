export type CodeModalSourceAuthority = 'bound' | 'draft';

export function resolveCodeModalSource(input: {
  activeRenderSource?: string;
  boundSource: string;
  activeRenderMatchesViewport: boolean;
}): { source: string; authority: CodeModalSourceAuthority } {
  if (input.activeRenderMatchesViewport && typeof input.activeRenderSource === 'string') {
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
