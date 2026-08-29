import { get } from 'svelte/store';

import { activeThreadIdStore as activeThreadId, activeVersionId } from '../stores/domainState';
import { paramPanelState } from '../stores/paramPanelState';
import { session } from '../stores/sessionStore';
import { workingCopy } from '../stores/workingCopy';
import { activeRenderSnapshot } from '../stores/activeRenderSnapshot';
import { buildImportedSyntheticDesign } from './importedRuntime';
import { saveLastDesign } from '../tauri/client';
import type { AuthoringTargetRef, DesignOutput, LastDesignSnapshot } from '../types/domain';

type TauriWindow = Window & {
  __TAURI_INTERNALS__?: {
    invoke?: unknown;
  };
};

function hasTauriRuntime(): boolean {
  return (
    typeof window !== 'undefined' &&
    (window as TauriWindow).__TAURI_INTERNALS__ !== undefined &&
    typeof (window as TauriWindow).__TAURI_INTERNALS__ === 'object'
  );
}

function buildWorkingDesign(): DesignOutput | null {
  const current = get(workingCopy);
  if (!current.macroCode.trim()) {
    const currentSession = get(session);
    const panel = get(paramPanelState);
    return buildImportedSyntheticDesign(currentSession.modelManifest, panel.params, panel.uiSpec);
  }

  return {
    title: current.title,
    versionName: current.versionName,
    response: '',
    interactionMode: 'design',
    macroCode: current.macroCode,
    macroDialect: current.macroDialect ?? 'legacy',
    sourceLanguage: current.sourceLanguage ?? 'legacyPython',
    geometryBackend: current.geometryBackend ?? 'freecad',
    uiSpec: current.uiSpec,
    initialParams: current.params,
    postProcessing: current.postProcessing ?? null,
  };
}

export function resolveRestartTargetRef(
  explicitTargetRef: AuthoringTargetRef | null | undefined,
  activeTargetRef: AuthoringTargetRef | null | undefined,
  threadId: string | null,
  messageId: string | null,
): AuthoringTargetRef | null {
  if (explicitTargetRef !== undefined) return explicitTargetRef;
  const activeTargetMatches = activeTargetRef && threadId && (
    activeTargetRef.kind === 'savedVersion'
      ? activeTargetRef.threadId === threadId && activeTargetRef.messageId === messageId
      : activeTargetRef.kind === 'draft'
        ? activeTargetRef.threadId === threadId && activeTargetRef.previewId === messageId
        : activeTargetRef.threadId === threadId
  );
  if (activeTargetMatches) return activeTargetRef;
  if (!threadId || !messageId) return null;
  return {
    kind: 'savedVersion',
    threadId,
    messageId,
  };
}

export async function persistLastSessionSnapshot(
  overrides: Partial<LastDesignSnapshot> = {},
): Promise<void> {
  if (!hasTauriRuntime()) {
    return;
  }

  const currentSession = get(session);
  const currentSnapshot = get(activeRenderSnapshot);
  const threadId = overrides.threadId !== undefined ? overrides.threadId : get(activeThreadId);
  const messageId = overrides.messageId !== undefined ? overrides.messageId : get(activeVersionId);
  const baseManifest = overrides.modelManifest ?? currentSession.modelManifest;
  const candidateSelectedPartId = overrides.selectedPartId ?? currentSession.selectedPartId;
  const selectedPartId =
    candidateSelectedPartId &&
    baseManifest?.parts?.some((part) => part.partId === candidateSelectedPartId)
      ? candidateSelectedPartId
      : null;

  const snapshot: LastDesignSnapshot = {
    design: overrides.design !== undefined ? overrides.design : buildWorkingDesign(),
    threadId,
    messageId,
    artifactBundle:
      overrides.artifactBundle !== undefined
        ? overrides.artifactBundle
        : currentSession.artifactBundle,
    modelManifest: baseManifest ?? null,
    selectedPartId,
    targetRef: resolveRestartTargetRef(
      overrides.targetRef,
      currentSnapshot?.targetRef,
      threadId,
      messageId,
    ),
  };

  // Saved versions and durable drafts restore by identity. Sending their full
  // runtime aggregate back through IPC duplicates SQLite/native/WebContent
  // memory and can kill WebContent on dense models.
  const persistedSnapshot: LastDesignSnapshot = threadId && messageId
    ? {
        design: null,
        threadId,
        messageId,
        artifactBundle: null,
        modelManifest: null,
        selectedPartId,
        targetRef: snapshot.targetRef,
      }
    : snapshot;

  if (!snapshot.threadId && !snapshot.messageId && !snapshot.design && !snapshot.artifactBundle && !snapshot.modelManifest) {
    await clearLastSessionSnapshot();
    return;
  }

  try {
    await saveLastDesign(persistedSnapshot);
  } catch (error) {
    console.warn('[SessionSnapshot] Failed to persist last snapshot:', error);
  }
}

export async function clearLastSessionSnapshot(): Promise<void> {
  if (!hasTauriRuntime()) {
    return;
  }

  try {
    await saveLastDesign(null);
  } catch (error) {
    console.warn('[SessionSnapshot] Failed to clear last snapshot:', error);
  }
}
