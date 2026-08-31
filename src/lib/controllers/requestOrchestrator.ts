import { get } from 'svelte/store';
import { convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { workingCopy } from '../stores/workingCopy';
import { activeThreadIdStore as activeThreadId, activeVersionId, config } from '../stores/domainState';
import { createNewThread, rememberCommittedVersionMessage } from '../stores/history';
import { requestQueue } from '../stores/requestQueue';
import { session, syncSessionPhaseFromQueue } from '../stores/sessionStore';
import { paramPanelState } from '../stores/paramPanelState';
import { ensureContext, startRequestHum, stopRequestHum } from '../audio/microwave';
import { startCookingPhraseLoop, stopPhraseLoop } from '../stores/phraseEngine';
import { getRenderableRuntimeBundle } from '../modelRuntime/runtimeBundle';
import type {
  AppConfig,
  Attachment,
  DesignOutput,
  Request,
  StructuralVerificationResult,
} from '../types/domain';
import { estimateBase64Bytes, profileLog } from '../debug/profiler';
import { formatBackendError, startExplorationRun } from '../tauri/client';
import { hydrateActiveRenderSnapshot } from '../stores/activeRenderSnapshot';
import type { AppError } from '../tauri/contracts';

function toSessionError(error: unknown): string | AppError {
  if (
    error && typeof error === 'object' && 'code' in error && 'message' in error &&
    typeof (error as { message?: unknown }).message === 'string'
  ) return error as AppError;
  return formatBackendError(error);
}

function toAssetUrl(path: string | null | undefined): string {
  if (!path) return '';
  try { return convertFileSrc(path); } catch { return path; }
}

export type GenerationProjectionGuard = {
  requestId: string;
  requestThreadId: string;
  latestThreadRequestId: string | null;
  activeThreadId: string | null;
};

/** Only newest request for active thread may update visible runtime. */
export function canPublishGenerationProjection({
  requestId,
  requestThreadId,
  latestThreadRequestId,
  activeThreadId,
}: GenerationProjectionGuard): boolean {
  return latestThreadRequestId === requestId && activeThreadId === requestThreadId;
}

type ViewerRef = {
  captureScreenshot: (overlayCanvas?: HTMLCanvasElement | null) => string | null;
};

type OrchestratorUiDeps = {
  viewerComponent?: ViewerRef | null;
  getDrawingCanvas?: (() => HTMLCanvasElement | null) | null;
  clearDrawing?: (() => void) | null;
};

type GenerateSubmissionOptions = {
  imageDataOverride?: string | null;
  uiDeps?: OrchestratorUiDeps;
  buildMode?: 'interactive' | 'controller';
};

export type ExplorationRunProgressEvent = {
  requestId: string;
  threadId: string;
  phase: string;
  attempt: number;
  maxAttempts: number;
  runningBuilds: number;
  pendingBuilds: number;
  currentVersionId?: string | null;
  summary: string;
  rawError?: string | null;
};

export type ExplorationRunProgressProjection = {
  requestPhase: Request['phase'];
  buildQueueState: Request['buildQueueState'];
  attempt: number;
  maxAttempts: number;
  copy: string;
};

/** Pure backend-progress to existing request/session projection mapping. */
export function projectExplorationRunProgress(
  progress: ExplorationRunProgressEvent,
): ExplorationRunProgressProjection {
  const phase = `${progress.phase ?? ''}`.trim().toLowerCase();
  const attempt = Math.max(1, Number(progress.attempt) || 1);
  const maxAttempts = Math.max(attempt, Number(progress.maxAttempts) || attempt);
  const requestPhase: Request['phase'] = phase === 'building'
    ? attempt > 1 ? 'repairing' : 'generating'
    : phase === 'verifying' || phase === 'deciding'
      ? 'rendering'
      : phase === 'awaitinginput' || phase === 'awaiting_input' ? 'classifying' : 'classifying';
  const pending = Math.max(0, Number(progress.pendingBuilds) || 0);
  const running = Math.max(0, Number(progress.runningBuilds) || 0);
  const buildQueueState: Request['buildQueueState'] = running > 0
    ? 'running'
    : pending > 0 || phase === 'queued' ? 'pending' : 'running';
  const summary = `${progress.summary ?? ''}`.trim() || phase.toUpperCase() || 'EXPLORATION RUNNING';
  const copy = `${summary} · RUNNING ${running} · PENDING ${pending}`;
  return { requestPhase, buildQueueState, attempt, maxAttempts, copy };
}

export type ExplorationRunTerminalProjection = {
  requestPhase: 'success' | 'error' | 'canceled';
  copy: string;
  error: string | null;
};

/** Keep terminal backend outcomes distinct without making frontend decisions. */
export function projectExplorationRunTerminal(
  phase: string,
  rawError: string | null | undefined,
  responseText: string | null | undefined = null,
): ExplorationRunTerminalProjection {
  const normalized = `${phase ?? ''}`.trim().toLowerCase();
  if (normalized === 'completed' && !rawError?.trim()) {
    return { requestPhase: 'success', copy: responseText?.trim() || 'EXPLORATION COMPLETE', error: null };
  }
  if ((normalized === 'awaitinginput' || normalized === 'awaiting_input') && !rawError?.trim()) {
    return { requestPhase: 'success', copy: responseText?.trim() || 'EXPLORATION AWAITING INPUT', error: null };
  }
  if (normalized === 'stopped') {
    return { requestPhase: 'canceled', copy: 'EXPLORATION STOPPED', error: null };
  }
  if (normalized === 'superseded') {
    return { requestPhase: 'canceled', copy: 'EXPLORATION SUPERSEDED BY NEWER INPUT', error: null };
  }
  if (normalized === 'interrupted') {
    return { requestPhase: 'canceled', copy: 'EXPLORATION INTERRUPTED', error: null };
  }
  const error = rawError?.trim() || `Exploration run ended in ${phase}.`;
  return { requestPhase: 'error', copy: `EXPLORATION FAILED · ${error}`, error };
}

function buildWorkingDesignSnapshot(): DesignOutput | null {
  const copy = get(workingCopy);
  const panel = get(paramPanelState);
  if (!copy.macroCode) return null;
  return {
    title: copy.title || 'Untitled Design',
    versionName: copy.versionName || 'Working Copy',
    response: '',
    interactionMode: 'design',
    macroCode: copy.macroCode,
    macroDialect: copy.macroDialect ?? 'legacy',
    engineKind: copy.engineKind ?? 'freecad',
    sourceLanguage: copy.sourceLanguage ?? 'legacyPython',
    geometryBackend: copy.geometryBackend ?? 'freecad',
    uiSpec: panel.uiSpec || { fields: [] },
    initialParams: panel.params || {},
    postProcessing: copy.postProcessing ?? null,
  };
}

function stopRequestHumFor(requestId: string, success: boolean, currentConfig: AppConfig, threadId: string) {
  const queue = get(requestQueue);
  const activeIds = queue.order.filter((id) => {
    const request = queue.byId[id];
    return request && request.threadId === threadId && !['success', 'error', 'canceled'].includes(request.phase);
  });
  stopRequestHum(requestId, success, currentConfig, Math.max(0, activeIds.indexOf(requestId)));
}

type ExplorationRunResult = {
  requestId: string;
  threadId: string;
  phase: string;
  messageId: string;
  design?: DesignOutput | null;
  artifactBundle?: import('../types/domain').ArtifactBundle | null;
  modelManifest?: import('../types/domain').ModelManifest | null;
  structuralVerification?: StructuralVerificationResult | null;
  message?: import('../types/domain').Message | null;
  snapshotId?: string | null;
  responseText?: string | null;
  rawError?: string | null;
  publicationAllowed: boolean;
};

function latestRequestIdForThread(threadId: string): string | null {
  const queue = get(requestQueue);
  return [...queue.order].reverse().find((id) => queue.byId[id]?.threadId === threadId) ?? null;
}

async function projectExplorationRun(requestId: string, currentConfig: AppConfig, output: ExplorationRunResult) {
  const terminal = projectExplorationRunTerminal(output.phase, output.rawError, output.responseText);
  if (terminal.requestPhase !== 'success') {
    if (terminal.requestPhase === 'error') {
      if (get(activeThreadId) === output.threadId) session.setError(toSessionError(terminal.error));
      requestQueue.patch(requestId, { phase: 'error', error: terminal.error ?? terminal.copy });
    } else {
      if (get(activeThreadId) === output.threadId) session.setStatus(terminal.copy);
      requestQueue.patch(requestId, { phase: 'canceled' });
    }
    stopRequestHumFor(requestId, false, currentConfig, output.threadId);
    syncSessionPhaseFromQueue();
    return;
  }

  const design = output.design ?? null;
  const artifactBundle = output.artifactBundle ?? null;
  const modelManifest = output.modelManifest ?? null;
  if (output.publicationAllowed && design && artifactBundle && modelManifest && !output.snapshotId) {
    throw new Error('Exploration run returned renderable runtime without backend snapshotId.');
  }
  const result = {
    design,
    threadId: output.threadId,
    messageId: output.messageId,
    stlUrl: toAssetUrl(artifactBundle?.modelStlPath),
    artifactBundle,
    modelManifest,
    structuralVerification: output.structuralVerification ?? null,
  };
  requestQueue.patch(requestId, { phase: 'success', result });

  const mayPublish = output.publicationAllowed && canPublishGenerationProjection({
    requestId,
    requestThreadId: output.threadId,
    latestThreadRequestId: latestRequestIdForThread(output.threadId),
    activeThreadId: get(activeThreadId),
  });
  if (mayPublish && output.message) {
    rememberCommittedVersionMessage(output.threadId, output.design?.title ?? 'Design', output.message);
  }
  if (mayPublish && design && artifactBundle && modelManifest) {
    const renderableBundle = getRenderableRuntimeBundle(
      artifactBundle,
      design.postProcessing ?? null,
      design.initialParams ?? {},
    ) ?? artifactBundle;
    activeThreadId.set(output.threadId);
    hydrateActiveRenderSnapshot({
      snapshotId: output.snapshotId!,
      threadId: output.threadId,
      messageId: output.messageId,
      design,
      artifactBundle: renderableBundle,
      modelManifest,
      selectedPartId: null,
      stlUrl: toAssetUrl(renderableBundle.modelStlPath),
      status: output.responseText?.trim() || design.response?.trim() || 'Design synthesized successfully.',
      targetRef: { kind: 'savedVersion', threadId: output.threadId, messageId: output.messageId },
    });
  }
  if (get(activeThreadId) === output.threadId) {
    session.setStatus(output.responseText?.trim() || design?.response?.trim() || 'Design synthesized successfully.');
  }
  stopRequestHumFor(requestId, true, currentConfig, output.threadId);
  syncSessionPhaseFromQueue();
}

/** Submit intent to Rust-owned exploration runner; frontend owns projection only. */
export async function handleGenerate(initialPrompt: string, attachments: Attachment[] = [], options: GenerateSubmissionOptions = {}): Promise<string> {
  session.setError(null);
  const currentConfig = get(config);
  if (!get(activeThreadId)) {
    const createdThreadId = await createNewThread({ mode: 'blank' });
    if (!createdThreadId) throw new Error('Failed to open a design thread for generation.');
  }
  const threadId = get(activeThreadId);
  if (!threadId) throw new Error('Generation requires an active thread.');
  const buildMode = options.buildMode ?? 'interactive';
  const baseVersionId = get(activeVersionId);
  const requestId = requestQueue.submit(
    initialPrompt,
    attachments,
    threadId,
    baseVersionId,
    get(session).artifactBundle?.modelId ?? null,
    buildMode,
  );
  requestQueue.setActive(requestId);

  const overlay = options.uiDeps?.getDrawingCanvas?.() ?? null;
  const imageData = options.imageDataOverride ?? (options.uiDeps?.viewerComponent && get(session).stlUrl
    ? options.uiDeps.viewerComponent.captureScreenshot(overlay)
    : null);
  options.uiDeps?.clearDrawing?.();
  if (imageData) requestQueue.patch(requestId, { screenshot: imageData });
  requestQueue.patch(requestId, { cookingStartTime: Date.now() });
  syncSessionPhaseFromQueue();
  ensureContext();
  startCookingPhraseLoop();
  startRequestHum(requestId, currentConfig, threadId);
  profileLog('generate.submit', {
    requestId,
    threadId,
    promptChars: initialPrompt.length,
    attachments: attachments.length,
    screenshotMb: Number((estimateBase64Bytes(imageData) / (1024 * 1024)).toFixed(2)),
  });

  void (async () => {
    const request = get(requestQueue).byId[requestId];
    if (!request || request.phase === 'canceled') return;
    let unlisten: (() => void) | null = null;
    try {
      unlisten = await listen<ExplorationRunProgressEvent>('exploration-run-progress', (event) => {
        if (event.payload.requestId !== requestId || event.payload.threadId !== threadId) return;
        const projection = projectExplorationRunProgress(event.payload);
        requestQueue.patch(requestId, {
          phase: projection.requestPhase,
          buildQueueState: projection.buildQueueState,
          attempt: projection.attempt,
          maxAttempts: projection.maxAttempts,
          lightResponse: projection.copy,
        });
        if (get(activeThreadId) === threadId) session.setStatus(projection.copy);
        syncSessionPhaseFromQueue();
      });
    } catch (error) {
      console.warn('[Orchestrator] exploration progress subscription unavailable:', error);
    }
    try {
      const workingDesign = buildWorkingDesignSnapshot();
      const output = await startExplorationRun({
        requestId,
        threadId,
        prompt: initialPrompt,
        attachments,
        imageData,
        parentMacroCode: get(workingCopy).macroCode || null,
        workingDesign,
        baseVersionId,
        kind: buildMode,
        options: {
          engineKind: workingDesign?.engineKind ?? currentConfig.defaultEngineKind,
          sourceLanguage: workingDesign?.sourceLanguage ?? currentConfig.defaultSourceLanguage,
          geometryBackend: workingDesign?.geometryBackend ?? currentConfig.defaultGeometryBackend,
        },
        acceptanceCriteria: [],
        hardConstraints: [],
        softPreferences: [],
      });
      await projectExplorationRun(requestId, currentConfig, output as ExplorationRunResult);
    } finally {
      unlisten?.();
    }
  })().catch((error) => {
    const message = formatBackendError(error);
    if (get(activeThreadId) === threadId) session.setError(toSessionError(error));
    requestQueue.patch(requestId, { phase: 'error', error: message });
    stopRequestHumFor(requestId, false, currentConfig, threadId);
    syncSessionPhaseFromQueue();
  }).finally(() => {
    stopPhraseLoop();
  });
  return requestId;
}
