import { exists } from '@tauri-apps/plugin-fs';

import {
  type ArtifactBundle,
  type DesignParams,
  type PostProcessingSpec,
  normalizePostProcessing,
} from '../types/domain';

export type RuntimeBundleAvailability = {
  bundle: ArtifactBundle | null;
  modelAvailable: boolean;
  degradedToModel: boolean;
};

type PathExists = (path: string) => Promise<boolean>;

async function defaultPathExists(path: string): Promise<boolean> {
  return exists(path);
}

async function safePathExists(path: string, pathExists: PathExists): Promise<boolean> {
  try {
    return await pathExists(path);
  } catch {
    return false;
  }
}

function hasDisplacementPostProcessing(
  postProcessing: PostProcessingSpec | null | undefined,
  params: DesignParams | null | undefined = null,
): boolean {
  const normalized = normalizePostProcessing(postProcessing);
  if (!normalized) return false;

  return (normalized.lithophaneAttachments ?? []).some((attachment) => {
    if (attachment.enabled === false) return false;
    if (attachment.source.kind === 'file') {
      return attachment.source.imagePath.trim().length > 0;
    }
    const parameterKey = attachment.source.imageParam.trim();
    if (!parameterKey) return false;
    const parameterValue = params?.[parameterKey];
    return typeof parameterValue === 'string' && parameterValue.trim().length > 0;
  });
}

export function getRenderableRuntimeBundle(
  bundle: ArtifactBundle | null | undefined,
  postProcessing: PostProcessingSpec | null | undefined = null,
  params: DesignParams | null | undefined = null,
): ArtifactBundle | null {
  if (!bundle) return null;
  if (!hasDisplacementPostProcessing(postProcessing, params)) return bundle;
  if (!(bundle.viewerAssets?.length ?? 0)) return bundle;
  return {
    ...bundle,
    viewerAssets: [],
  };
}

export async function inspectRuntimeBundle(
  bundle: ArtifactBundle | null | undefined,
  pathExists: PathExists = defaultPathExists,
  postProcessing: PostProcessingSpec | null | undefined = null,
  params: DesignParams | null | undefined = null,
): Promise<RuntimeBundleAvailability> {
  if (!bundle?.modelStlPath) {
    return {
      bundle: null,
      modelAvailable: false,
      degradedToModel: false,
    };
  }

  const modelAvailable = await safePathExists(bundle.modelStlPath, pathExists);
  if (!modelAvailable) {
    return {
      bundle: null,
      modelAvailable: false,
      degradedToModel: false,
    };
  }

  const renderableBundle = getRenderableRuntimeBundle(bundle, postProcessing, params);
  const viewerAssets = renderableBundle?.viewerAssets ?? [];
  const degradedToModel = Boolean(
    renderableBundle &&
      (bundle.viewerAssets?.length ?? 0) > 0 &&
      (renderableBundle.viewerAssets?.length ?? 0) === 0,
  );

  if (!viewerAssets.length) {
    return {
      bundle: renderableBundle,
      modelAvailable: true,
      degradedToModel,
    };
  }

  const viewerAssetChecks = await Promise.all(
    viewerAssets.map((asset) => safePathExists(asset.path, pathExists)),
  );

  if (viewerAssetChecks.every(Boolean)) {
    return {
      bundle: renderableBundle,
      modelAvailable: true,
      degradedToModel,
    };
  }

  const modelOnlyBundle = renderableBundle
    ? {
        ...renderableBundle,
        viewerAssets: [],
      }
    : null;

  return {
    bundle: modelOnlyBundle,
    modelAvailable: true,
    degradedToModel: true,
  };
}
