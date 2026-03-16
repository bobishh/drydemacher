import { exists } from '@tauri-apps/plugin-fs';

import type { ArtifactBundle, PostProcessingSpec } from '../types/domain';

export type RuntimeBundleAvailability = {
  bundle: ArtifactBundle | null;
  previewAvailable: boolean;
  degradedToPreview: boolean;
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
): boolean {
  return Boolean(postProcessing?.displacement);
}

export function getRenderableRuntimeBundle(
  bundle: ArtifactBundle | null | undefined,
  postProcessing: PostProcessingSpec | null | undefined = null,
): ArtifactBundle | null {
  if (!bundle) return null;
  if (!hasDisplacementPostProcessing(postProcessing)) return bundle;
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
): Promise<RuntimeBundleAvailability> {
  if (!bundle?.previewStlPath) {
    return {
      bundle: null,
      previewAvailable: false,
      degradedToPreview: false,
    };
  }

  const previewAvailable = await safePathExists(bundle.previewStlPath, pathExists);
  if (!previewAvailable) {
    return {
      bundle: null,
      previewAvailable: false,
      degradedToPreview: false,
    };
  }

  const renderableBundle = getRenderableRuntimeBundle(bundle, postProcessing);
  const viewerAssets = renderableBundle?.viewerAssets ?? [];
  const degradedToPreview = Boolean(
    renderableBundle &&
      (bundle.viewerAssets?.length ?? 0) > 0 &&
      (renderableBundle.viewerAssets?.length ?? 0) === 0,
  );

  if (!viewerAssets.length) {
    return {
      bundle: renderableBundle,
      previewAvailable: true,
      degradedToPreview,
    };
  }

  const viewerAssetChecks = await Promise.all(
    viewerAssets.map((asset) => safePathExists(asset.path, pathExists)),
  );

  if (viewerAssetChecks.every(Boolean)) {
    return {
      bundle: renderableBundle,
      previewAvailable: true,
      degradedToPreview,
    };
  }

  const previewOnlyBundle = renderableBundle
    ? {
        ...renderableBundle,
        viewerAssets: [],
      }
    : null;

  return {
    bundle: previewOnlyBundle,
    previewAvailable: true,
    degradedToPreview: true,
  };
}
