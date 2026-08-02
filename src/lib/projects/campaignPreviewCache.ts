import type { ArtifactBundle } from '../types/domain';

export type CampaignEditedPreviewIdentity = {
  source: string;
  runtimeDigest: string;
  backend: string;
};

function cacheKey(identity: CampaignEditedPreviewIdentity): string {
  return JSON.stringify([identity.source, identity.runtimeDigest, identity.backend]);
}

/** Immutable per-source cache. Failures intentionally never enter this cache. */
export function createCampaignPreviewCache() {
  const entries = new Map<string, ArtifactBundle>();

  return {
    get(identity: CampaignEditedPreviewIdentity): ArtifactBundle | null {
      return entries.get(cacheKey(identity)) ?? null;
    },
    put(identity: CampaignEditedPreviewIdentity, artifact: ArtifactBundle): void {
      entries.set(cacheKey(identity), artifact);
    },
  };
}
