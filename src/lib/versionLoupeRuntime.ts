import { getThreadMessageVersion, materializeVersionPreview } from './tauri/client';
import type { Message, ModelManifest, ViewerAsset } from './types/domain';

type VersionLoupeMessage = Pick<Message, 'id' | 'artifactBundle' | 'modelManifest' | 'output'>;

export type VersionLoupeRuntime = {
  previewUrl: string | null;
  viewerAssets: ViewerAsset[];
  modelManifest: ModelManifest | null;
  leaseId: string | null;
  available: boolean;
};

type RuntimeDeps = {
  getThreadMessageVersion?: typeof getThreadMessageVersion;
  materializePreview?: typeof materializeVersionPreview;
};

async function hydrateVersionMessage(
  message: VersionLoupeMessage,
  threadId: string | null,
  loadVersionMessage: typeof getThreadMessageVersion,
): Promise<VersionLoupeMessage> {
  if (!threadId) return message;
  if (message.output && message.artifactBundle && message.modelManifest) return message;
  const hydrated = await loadVersionMessage(threadId, message.id);
  if (!hydrated) return message;
  return hydrated;
}

export async function resolveVersionLoupeRuntime(
  message: VersionLoupeMessage,
  threadId: string | null,
  toAssetUrl: (path: string | null | undefined) => string,
  deps: RuntimeDeps = {},
): Promise<VersionLoupeRuntime> {
  const loadVersionMessage = deps.getThreadMessageVersion ?? getThreadMessageVersion;
  const materializePreview = deps.materializePreview ?? materializeVersionPreview;

  const hydratedMessage = await hydrateVersionMessage(message, threadId, loadVersionMessage);
  if (!threadId || !hydratedMessage.artifactBundle) {
    return {
      previewUrl: null,
      viewerAssets: [],
      modelManifest: null,
      leaseId: null,
      available: false,
    };
  }

  const runtime = await materializePreview(threadId, hydratedMessage.id);
  if (!runtime.artifactBundle.modelStlPath) {
    return {
      previewUrl: null,
      viewerAssets: [],
      modelManifest: runtime.modelManifest,
      leaseId: runtime.leaseId ?? null,
      available: false,
    };
  }

  return {
    previewUrl: toAssetUrl(runtime.artifactBundle.modelStlPath),
    viewerAssets: (runtime.artifactBundle.viewerAssets ?? []).map((asset) => ({
      ...asset,
      path: toAssetUrl(asset.path),
    })),
    modelManifest: runtime.modelManifest,
    leaseId: runtime.leaseId ?? null,
    available: true,
  };
}
