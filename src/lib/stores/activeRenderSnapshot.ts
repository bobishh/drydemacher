import { derived, writable } from 'svelte/store';

import type { ArtifactBundle, AuthoringTargetRef, DesignOutput, ModelManifest } from '../types/domain';
import { activeVersionId } from './domainState';
import { paramPanelState } from './paramPanelState';
import { session } from './sessionStore';
import { workingCopy } from './workingCopy';

export type ActiveRenderSnapshot = Readonly<{
  snapshotId: string;
  threadId: string;
  messageId: string | null;
  design: DesignOutput;
  artifactBundle: ArtifactBundle;
  modelManifest: ModelManifest;
  selectedPartId: string | null;
  stlUrl: string;
  status: string;
  targetRef: AuthoringTargetRef | null;
}>;

export type ActiveRenderSnapshotInput = Omit<ActiveRenderSnapshot, 'snapshotId' | 'targetRef'> & {
  snapshotId?: string | null;
  eventModelId?: string | null;
  targetRef?: AuthoringTargetRef | null;
};

export class RenderSnapshotMismatch extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'RenderSnapshotMismatch';
  }
}

function stableJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.entries(value as Record<string, unknown>)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, entry]) => `${JSON.stringify(key)}:${stableJson(entry)}`)
      .join(',')}}`;
  }
  return JSON.stringify(value);
}

function fnv1a(value: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, '0');
}

function assertMatching(label: string, left: string | null | undefined, right: string | null | undefined) {
  if (left && right && left !== right) {
    throw new RenderSnapshotMismatch(`${label} mismatch: ${left} != ${right}`);
  }
}

export function buildActiveRenderSnapshot(input: ActiveRenderSnapshotInput): ActiveRenderSnapshot {
  assertMatching(
    'Render snapshot modelId',
    input.artifactBundle.modelId,
    input.modelManifest.modelId,
  );
  assertMatching('Render event modelId', input.eventModelId, input.artifactBundle.modelId);
  assertMatching(
    'Render snapshot sourceLanguage',
    input.design.sourceLanguage,
    input.artifactBundle.sourceLanguage,
  );
  assertMatching(
    'Render manifest sourceLanguage',
    input.artifactBundle.sourceLanguage,
    input.modelManifest.sourceLanguage,
  );
  assertMatching(
    'Render snapshot geometryBackend',
    input.design.geometryBackend,
    input.artifactBundle.geometryBackend,
  );
  assertMatching(
    'Render manifest geometryBackend',
    input.artifactBundle.geometryBackend,
    input.modelManifest.geometryBackend,
  );

  const snapshotId = input.snapshotId?.trim() || `frontend-${fnv1a(stableJson({
    threadId: input.threadId,
    messageId: input.messageId,
    source: input.design.macroCode,
    params: input.design.initialParams,
    postProcessing: input.design.postProcessing ?? null,
    backend: input.design.geometryBackend,
    modelId: input.artifactBundle.modelId,
    contentHash: input.artifactBundle.contentHash,
  }))}`;

  return Object.freeze({
    snapshotId,
    threadId: input.threadId,
    messageId: input.messageId,
    design: input.design,
    artifactBundle: input.artifactBundle,
    modelManifest: input.modelManifest,
    selectedPartId: input.selectedPartId,
    stlUrl: input.stlUrl,
    status: input.status,
    targetRef: input.targetRef ?? null,
  });
}

const snapshotStore = writable<ActiveRenderSnapshot | null>(null);

export const activeRenderSnapshot = {
  subscribe: snapshotStore.subscribe,
  clear: () => snapshotStore.set(null),
};

export const activeRenderDesign = derived(snapshotStore, (snapshot) => snapshot?.design ?? null);
export const activeRenderParams = derived(
  snapshotStore,
  (snapshot) => snapshot?.design.initialParams ?? {},
);
export const activeRenderRuntime = derived(snapshotStore, (snapshot) => snapshot ? ({
  artifactBundle: snapshot.artifactBundle,
  modelManifest: snapshot.modelManifest,
  stlUrl: snapshot.stlUrl,
}) : null);
export const activeRenderTarget = derived(snapshotStore, (snapshot) => snapshot ? ({
  threadId: snapshot.threadId,
  messageId: snapshot.messageId,
  snapshotId: snapshot.snapshotId,
  targetRef: snapshot.targetRef,
}) : null);

export function hydrateActiveRenderSnapshot(input: ActiveRenderSnapshotInput): ActiveRenderSnapshot {
  const snapshot = buildActiveRenderSnapshot(input);

  activeVersionId.set(snapshot.messageId);
  workingCopy.loadVersion(snapshot.design, snapshot.messageId);
  paramPanelState.hydrateFromVersion(snapshot.design, snapshot.messageId);
  session.update((current) => ({
    ...current,
    status: snapshot.status,
    stlUrl: snapshot.stlUrl,
    runtimeRevision: current.runtimeRevision + 1,
    artifactBundle: snapshot.artifactBundle,
    modelManifest: snapshot.modelManifest,
    selectedPartId: snapshot.selectedPartId,
  }));

  // Publish authority last. Compatibility stores above are projections.
  snapshotStore.set(snapshot);
  return snapshot;
}
