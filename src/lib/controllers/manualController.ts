import { get } from 'svelte/store';
import { writable } from 'svelte/store';
import { convertFileSrc } from '@tauri-apps/api/core';
import { workingCopy } from '../stores/workingCopy';
import { activeThreadIdStore as activeThreadId, activeVersionId, config } from '../stores/domainState';
import { refreshHistory, rememberCommittedVersionMessage } from '../stores/history';
import { session } from '../stores/sessionStore';
import { startMicrowaveHum, stopMicrowaveHum, ensureContext } from '../audio/microwave';
import { paramPanelState } from '../stores/paramPanelState';
import { resolveParamApplySource } from './paramApplySource';
import { recordSessionActivityEvent } from '../stores/sessionActivityStore';
import { persistLastSessionSnapshot } from '../modelRuntime/sessionSnapshot';
import { buildImportedSyntheticDesign } from '../modelRuntime/importedRuntime';
import { getRenderableRuntimeBundle, inspectRuntimeBundle } from '../modelRuntime/runtimeBundle';
import { ensureSemanticManifest } from '../modelRuntime/semanticControls';
import { confirmAction } from '../ui/confirmAction';
import type {
  ArtifactBundle,
  DesignOutput,
  DesignParams,
  MacroDialect,
  ModelManifest,
  ParamValue,
  PostProcessingSpec,
  SourceLanguage,
  GeometryBackend,
  UiField,
  UiSpec,
} from '../types/domain';
import {
  addManualVersion,
  applyImportedModel,
  formatBackendError,
  getModelManifest,
  parseMacroParams,
  renderModel,
  saveProjectSource,
  saveModelManifest,
} from '../tauri/client';
import { closeWindow as closeWindowStore } from '../stores/windowStore';
import type { WorkingCopyState } from '../stores/workingCopy';
import { pendingImageGeometry, pendingImageGeometryStatus } from '../imageGeometryPending';
import { LatestTaskGate } from './latestTaskGate';

const latestParamRenderGate = new LatestTaskGate();
let latestAppliedParamDraft: AppliedParamDraft | null = null;
type ManualApplyQueueState = { running: boolean; pending: number };
const manualApplyQueueState = writable<ManualApplyQueueState>({ running: false, pending: 0 });
let manualApplyQueue: Promise<void> = Promise.resolve();
let runningManualApply = false;
let queuedManualApply = 0;
const updateManualApplyQueueState = () =>
  manualApplyQueueState.set({
    running: runningManualApply,
    pending: queuedManualApply,
  });

export const manualApplyQueueStateStore = manualApplyQueueState;

function queueManualApply<T>(task: () => Promise<T>): Promise<T> {
  queuedManualApply += 1;
  updateManualApplyQueueState();
  let resolve: (value: T) => void;
  let reject: (reason?: unknown) => void;
  const result = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  const run = async () => {
    queuedManualApply = Math.max(0, queuedManualApply - 1);
    runningManualApply = true;
    updateManualApplyQueueState();
    try {
      const value = await task();
      resolve(value);
    } catch (error) {
      reject(error);
    } finally {
      runningManualApply = false;
      updateManualApplyQueueState();
    }
  };
  const safeRun = async () => {
    try {
      await run();
    } catch {
      /* no-op: run handles per-task outcome and resolves caller promise */
    }
  };
  manualApplyQueue = manualApplyQueue.then(safeRun, safeRun);
  return result;
}

type ManualCommitOptions = {
  targetThreadId?: string | null;
  activateTargetOnSuccess?: boolean;
  successStatus?: string;
  versionName?: string | null;
};

export type ManualVersionCommitInput = {
  code: string;
  title?: string | null;
  versionName?: string | null;
};

type AppliedParamDraft = {
  signature: string;
  renderableBundle: ArtifactBundle;
  modelManifest: ModelManifest | null;
};

export function shouldPreserveWorkingCopyMacroDraft(
  workingCopyState: Pick<WorkingCopyState, 'macroCode' | 'dirty'>,
  committedMacroCode: string,
): boolean {
  return workingCopyState.dirty && workingCopyState.macroCode !== committedMacroCode;
}

function restoreWorkingCopyMacroDraftIfNeeded(
  previousWorkingCopy: Pick<WorkingCopyState, 'macroCode' | 'dirty'>,
  committedMacroCode: string,
) {
  if (!shouldPreserveWorkingCopyMacroDraft(previousWorkingCopy, committedMacroCode)) return;
  workingCopy.patch({
    macroCode: previousWorkingCopy.macroCode,
    dirty: true,
  });
}

function changedParamKeys(before: DesignParams, after: DesignParams): string[] {
  const keys = new Set([...Object.keys(before), ...Object.keys(after)]);
  return [...keys].filter((key) => stableJson(before[key]) !== stableJson(after[key])).sort();
}

// Exported for the once-per-action emission tests (session-collab-visibility 5.7).
export function recordParamsChanged(input: {
  threadId: string | null;
  versionId: string | null;
  before: DesignParams;
  after: DesignParams;
  persist: boolean;
}) {
  const keys = changedParamKeys(input.before, input.after);
  if (keys.length === 0) return;
  recordSessionActivityEvent({
    threadId: input.threadId,
    versionId: input.versionId,
    kind: 'params_changed',
    title: input.persist ? 'Parameter commit requested' : 'Parameters applied',
    summary: `${keys.length} parameter${keys.length === 1 ? '' : 's'} changed: ${keys.join(', ')}`,
    severity: 'info',
    diffs: keys.map((key) => ({
      kind: 'params',
      key,
      before: stableJson(input.before[key]),
      after: stableJson(input.after[key]),
    })),
  });
}

function recordRenderEvent(input: {
  threadId: string | null;
  versionId: string | null;
  kind: 'render_started' | 'render_succeeded' | 'render_failed';
  title: string;
  summary: string;
  severity: 'info' | 'success' | 'error';
  raw?: unknown;
}) {
  recordSessionActivityEvent(input);
}

function stableJson(value: unknown): string {
  if (Array.isArray(value)) {
    return `[${value.map(stableJson).join(',')}]`;
  }
  if (value && typeof value === 'object') {
    return `{${Object.entries(value as Record<string, unknown>)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([key, entry]) => `${JSON.stringify(key)}:${stableJson(entry)}`)
      .join(',')}}`;
  }
  return JSON.stringify(value);
}

function paramDraftSignature(input: {
  threadId: string | null;
  targetVersionId: string | null;
  macroCode: string;
  params: DesignParams;
  macroDialect: MacroDialect | null | undefined;
  geometryBackend: string | null | undefined;
  postProcessing: PostProcessingSpec | null | undefined;
}): string {
  return stableJson({
    threadId: input.threadId ?? null,
    targetVersionId: input.targetVersionId ?? null,
    macroCode: input.macroCode,
    params: input.params,
    macroDialect: input.macroDialect ?? null,
    geometryBackend: input.geometryBackend ?? null,
    postProcessing: input.postProcessing ?? null,
  });
}

async function commitRenderedParamDraft(input: {
  snapshotThreadId: string;
  codeToUse: string;
  currentParams: DesignParams;
  uiSpec: UiSpec;
  postProcessing: PostProcessingSpec | null;
  title: string;
  versionName: string;
  workingMacroDialect: MacroDialect | null | undefined;
  workingSourceLanguage: SourceLanguage | null | undefined;
  workingGeometryBackend: GeometryBackend | null | undefined;
  draft: AppliedParamDraft;
}) {
  const committedDesign = buildManualDesign({
    title: input.title,
    versionName: input.versionName,
    response: 'Parameter version committed.',
    macroCode: input.codeToUse,
    bundle: input.draft.renderableBundle,
    uiSpec: input.uiSpec,
    params: input.currentParams,
    postProcessing: input.postProcessing,
    workingMacroDialect: input.workingMacroDialect,
  });
  const newMsgId = await addManualVersion({
    threadId: input.snapshotThreadId,
    title: input.title,
    versionName: input.versionName,
    macroCode: input.codeToUse,
    sourceLanguage: input.draft.renderableBundle.sourceLanguage || input.workingSourceLanguage || null,
    geometryBackend: input.draft.renderableBundle.geometryBackend || input.workingGeometryBackend || null,
    parameters: input.currentParams,
    uiSpec: input.uiSpec,
    postProcessing: input.postProcessing,
    artifactBundle: input.draft.renderableBundle,
    modelManifest: input.draft.modelManifest,
  });

  rememberCommittedVersionMessage(input.snapshotThreadId, input.title, {
    id: newMsgId,
    role: 'assistant',
    content: committedDesign.response,
    status: 'success',
    output: committedDesign,
    usage: null,
    artifactBundle: input.draft.renderableBundle,
    modelManifest: input.draft.modelManifest,
    agentOrigin: null,
    imageData: null,
    visualKind: null,
    attachmentImages: [],
    timestamp: Date.now() / 1000,
  });

  activeVersionId.set(newMsgId);
  const previousWorkingCopy = get(workingCopy);
  workingCopy.loadVersion(committedDesign, newMsgId);
  restoreWorkingCopyMacroDraftIfNeeded(previousWorkingCopy, committedDesign.macroCode);
  paramPanelState.hydrateFromVersion(committedDesign, newMsgId);
  await persistLastSessionSnapshot({
    design: committedDesign,
    threadId: input.snapshotThreadId,
    messageId: newMsgId,
    artifactBundle: input.draft.renderableBundle,
    modelManifest: input.draft.modelManifest,
    selectedPartId: null,
  });
  await refreshHistory();
  recordSessionActivityEvent({
    threadId: input.snapshotThreadId,
    versionId: newMsgId,
    kind: 'version_committed',
    title: 'Parameter version committed',
    summary: 'Parameter version committed from applied draft.',
    severity: 'success',
  });
  session.setStatus('Parameter version committed.');
}

async function runManualCommitHousekeeping(
  modelId: string,
  shouldSaveManifest: boolean,
  snapshotThreadId: string,
  committedDesign: DesignOutput,
  newMsgId: string,
  artifactBundle: ArtifactBundle,
  modelManifest: ModelManifest | null,
  shouldPersistSnapshot: boolean,
) {
  if (shouldSaveManifest && modelManifest) {
    await saveModelManifest(modelId, modelManifest, newMsgId);
  }

  await refreshHistory();

  if (!shouldPersistSnapshot) return;

  await persistLastSessionSnapshot({
    design: committedDesign,
    threadId: snapshotThreadId,
    messageId: newMsgId,
    artifactBundle,
    modelManifest,
    selectedPartId: null,
  });
}

function toAssetUrl(path: string | null | undefined): string {
  if (!path) return '';
  try {
    return convertFileSrc(path);
  } catch {
    return path;
  }
}

function workingCopyBackendLabel(design: Pick<DesignOutput, 'geometryBackend' | 'sourceLanguage' | 'macroDialect'>): string {
  if (design.macroDialect === 'ecky' || design.sourceLanguage === 'ecky') {
    return 'Ecky';
  }
  return 'FreeCAD';
}

function fallbackParamValue(field: UiField): ParamValue {
  switch (field.type) {
    case 'checkbox':
      return false;
    case 'select':
      return field.options[0]?.value ?? '';
    case 'image':
      return '';
    case 'range':
    case 'number':
      return typeof field.min === 'number' ? field.min : 0;
  }
}

function mergeFieldWithExisting(parsedField: UiField, existingField: UiField | undefined): UiField {
  if (!existingField || existingField.type !== parsedField.type) {
    return parsedField;
  }

  switch (parsedField.type) {
    case 'checkbox':
      {
        const existing = existingField;
        return {
          ...parsedField,
          label: existing.label || parsedField.label,
          frozen: existing.frozen ?? parsedField.frozen,
        };
      }
    case 'select':
      {
        const existing = existingField as Extract<UiField, { type: 'select' }>;
        return {
          ...parsedField,
          label: existing.label || parsedField.label,
          frozen: existing.frozen ?? parsedField.frozen,
          options:
            existing.options?.length > 0 ? existing.options : parsedField.options,
        };
      }
    case 'image':
      {
        const existing = existingField as Extract<UiField, { type: 'image' }>;
        return {
          ...parsedField,
          label: existing.label || parsedField.label,
          frozen: existing.frozen ?? parsedField.frozen,
        };
      }
    case 'range':
    case 'number':
      {
        const existing = existingField as Extract<UiField, { type: 'range' | 'number' }>;
        return {
          ...parsedField,
          label: existing.label || parsedField.label,
          frozen: existing.frozen ?? parsedField.frozen,
          min: existing.min ?? parsedField.min,
          max: existing.max ?? parsedField.max,
          step: existing.step ?? parsedField.step,
          minFrom: existing.minFrom ?? parsedField.minFrom,
          maxFrom: existing.maxFrom ?? parsedField.maxFrom,
        };
      }
  }
}

function coerceParamValue(field: UiField, currentValue: ParamValue | undefined, parsedValue: ParamValue | undefined): ParamValue {
  const candidate = currentValue ?? parsedValue;

  switch (field.type) {
    case 'checkbox':
      if (typeof candidate === 'boolean') return candidate;
      return typeof parsedValue === 'boolean' ? parsedValue : false;
    case 'image':
      if (typeof candidate === 'string') return candidate;
      return typeof parsedValue === 'string' ? parsedValue : '';
    case 'select': {
      const optionValues = new Set((field.options || []).map((option) => option.value));
      if (typeof candidate === 'string' && optionValues.has(candidate)) return candidate;
      if (typeof parsedValue === 'string' && optionValues.has(parsedValue)) return parsedValue;
      return field.options[0]?.value ?? '';
    }
    case 'range':
    case 'number':
      if (typeof candidate === 'number' && Number.isFinite(candidate)) return candidate;
      if (typeof parsedValue === 'number' && Number.isFinite(parsedValue)) return parsedValue;
      return fallbackParamValue(field);
  }
}

async function reconcileManualControls(
  editedCode: string,
  currentUiSpec: UiSpec,
  currentParams: DesignParams,
): Promise<{ uiSpec: UiSpec; params: DesignParams; parserMatched: boolean }> {
  try {
    const parsed = await parseMacroParams(editedCode);
    if (!parsed.fields.length) {
      return { uiSpec: currentUiSpec, params: currentParams, parserMatched: false };
    }

    const existingByKey = new Map(currentUiSpec.fields.map((field) => [field.key, field]));
    const nextFields = parsed.fields.map((field) =>
      mergeFieldWithExisting(field, existingByKey.get(field.key)),
    );
    const nextParams: DesignParams = {};
    for (const field of nextFields) {
      nextParams[field.key] = coerceParamValue(field, currentParams[field.key], parsed.params[field.key]);
    }

    return {
      uiSpec: { fields: nextFields },
      params: nextParams,
      parserMatched: true,
    };
  } catch (error) {
    console.warn('[ManualController] Failed to reconcile controls from edited macro:', error);
    return { uiSpec: currentUiSpec, params: currentParams, parserMatched: false };
  }
}

export async function handleParamChange(
  newParams: DesignParams,
  forcedCode: string | null = null,
  persist: boolean = false
): Promise<boolean> {
  return queueManualApply(() => doHandleParamChange(newParams, forcedCode, persist));
}

async function doHandleParamChange(
  newParams: DesignParams,
  forcedCode: string | null = null,
  persist: boolean = false
): Promise<boolean> {
  console.log('[ManualController] handleParamChange start', { newParams, persist });
  session.setError(null);
  const wc = get(workingCopy);
  const panel = get(paramPanelState);
  const snapshotThreadId = get(activeThreadId);
  const applySource = resolveParamApplySource({
    forcedCode,
    workingMacroCode: wc.macroCode,
    panelVersionId: panel.versionId,
    sourceVersionId: wc.sourceVersionId,
    activeVersionId: get(activeVersionId),
  });
  const targetVersionId =
    applySource.ok || applySource.reason === 'missing-macro-code'
      ? applySource.targetVersionId
      : wc.sourceVersionId || get(activeVersionId) || panel.versionId;
  if (!applySource.ok && applySource.reason === 'stale-panel-source-version-mismatch') {
    console.warn('[ManualController] Stale parameter panel source mismatch', applySource);
    if (get(activeThreadId) === snapshotThreadId) {
      session.setError(
        `Apply Failed: parameter panel is stale for active source (${applySource.panelVersionId} != ${applySource.sourceVersionId}). Reload the active version.`,
      );
    }
    return false;
  }

  const currentParams = forcedCode ? { ...newParams } : { ...panel.params, ...newParams };
  const renderToken = latestParamRenderGate.reserve(snapshotThreadId ?? '__detached__');
  recordParamsChanged({
    threadId: snapshotThreadId,
    versionId: targetVersionId,
    before: panel.params,
    after: currentParams,
    persist,
  });
  
  // 1. Update workingCopy immediately for UI responsiveness
  paramPanelState.setParams(currentParams);
  workingCopy.patch({ params: currentParams });

  const codeToUse = applySource.ok ? applySource.code : '';
  const pendingImages = pendingImageGeometry(codeToUse, panel.uiSpec, currentParams);
  if (pendingImages.length > 0) {
    if (get(activeThreadId) === snapshotThreadId) {
      session.setStatus(pendingImageGeometryStatus(pendingImages));
    }
    return true;
  }
  if (!codeToUse) {
    const currentSession = get(session);
    const importedDesign = buildImportedSyntheticDesign(
      currentSession.modelManifest,
      currentParams,
      panel.uiSpec,
    );
    if (!importedDesign) {
      console.warn('[ManualController] No macroCode or imported component runtime');
      if (get(activeThreadId) === snapshotThreadId) {
        session.setError('Apply Failed: active version has no executable source or imported component runtime.');
      }
      return false;
    }

    const sourceBundle = currentSession.artifactBundle;
    const sourceManifest = currentSession.modelManifest;
    paramPanelState.setParams(importedDesign.initialParams);
    paramPanelState.setUiSpec(importedDesign.uiSpec);
    workingCopy.patch({
      title: importedDesign.title,
      versionName: importedDesign.versionName,
      uiSpec: importedDesign.uiSpec,
      params: importedDesign.initialParams,
    });
    if (!sourceBundle || !sourceManifest) {
      session.setError('Imported Apply Failed: imported component runtime is not loaded.');
      return false;
    }

    let persistedImportedMessageId: string | null = null;
    try {
      startMicrowaveHum('__manual__', get(config), snapshotThreadId);
      session.setStatus('Applying imported FreeCAD component bindings...');
      recordRenderEvent({
        threadId: snapshotThreadId,
        versionId: targetVersionId,
        kind: 'render_started',
        title: 'Imported component apply started',
        summary: 'Applying imported FreeCAD component bindings.',
        severity: 'info',
      });
      const nextBundle = await applyImportedModel(
        sourceBundle,
        sourceManifest,
        importedDesign.initialParams,
        null,
      );
      const rawNextManifest = await getModelManifest(nextBundle.modelId);
      const nextManifest = ensureSemanticManifest(
        rawNextManifest,
        importedDesign.uiSpec,
        importedDesign.initialParams,
        sourceManifest,
      ) ?? rawNextManifest;
      if (!latestParamRenderGate.isCurrent(renderToken)) return false;
      if (get(activeThreadId) !== snapshotThreadId) return false;

      session.setStlUrl(toAssetUrl(nextBundle.modelStlPath));
      session.setModelRuntime(nextBundle, nextManifest);
      if (JSON.stringify(nextManifest) !== JSON.stringify(rawNextManifest)) {
        await saveModelManifest(nextBundle.modelId, nextManifest, null);
      }
      recordRenderEvent({
        threadId: snapshotThreadId,
        versionId: targetVersionId,
        kind: 'render_succeeded',
        title: 'Imported component apply succeeded',
        summary: 'Imported FreeCAD component bindings applied.',
        severity: 'success',
        raw: { modelId: nextBundle.modelId, modelStlPath: nextBundle.modelStlPath },
      });

      if (persist && snapshotThreadId) {
        const newMsgId = await addManualVersion({
          threadId: snapshotThreadId,
          title: importedDesign.title,
          versionName: importedDesign.versionName,
          macroCode: importedDesign.macroCode,
          sourceLanguage: importedDesign.sourceLanguage,
          geometryBackend: importedDesign.geometryBackend,
          parameters: importedDesign.initialParams,
          uiSpec: importedDesign.uiSpec,
          postProcessing: importedDesign.postProcessing ?? null,
          artifactBundle: nextBundle,
          modelManifest: nextManifest,
        });
        persistedImportedMessageId = newMsgId;
        activeVersionId.set(newMsgId);
        workingCopy.loadVersion(importedDesign, newMsgId);
        paramPanelState.hydrateFromVersion(importedDesign, newMsgId);
        await refreshHistory();
      }
      await persistLastSessionSnapshot({
        design: importedDesign,
        threadId: snapshotThreadId,
        messageId: persist ? get(activeVersionId) : targetVersionId,
        artifactBundle: nextBundle,
        modelManifest: nextManifest,
        selectedPartId: null,
      });
      session.setStatus(
        persist
          ? 'Imported component appended as new version.'
          : 'Imported component preview updated.',
      );
      return true;
    } catch (error) {
      const rawError = formatBackendError(error);
      console.error('[ManualController] apply_imported_model error:', rawError, error);
      if (persist && snapshotThreadId && !persistedImportedMessageId) {
        try {
          const failedMessageId = await addManualVersion({
            threadId: snapshotThreadId,
            title: importedDesign.title,
            versionName: importedDesign.versionName,
            macroCode: importedDesign.macroCode,
            sourceLanguage: importedDesign.sourceLanguage,
            geometryBackend: importedDesign.geometryBackend,
            parameters: importedDesign.initialParams,
            uiSpec: importedDesign.uiSpec,
            postProcessing: importedDesign.postProcessing ?? null,
            artifactBundle: null,
            modelManifest: null,
            status: 'error',
            errorMessage: rawError,
          });
          activeVersionId.set(failedMessageId);
          await refreshHistory();
        } catch (persistenceError) {
          console.error('[ManualController] failed imported version persistence error:', persistenceError);
        }
      }
      recordRenderEvent({
        threadId: snapshotThreadId,
        versionId: targetVersionId,
        kind: 'render_failed',
        title: 'Imported component apply failed',
        summary: rawError,
        severity: 'error',
        raw: error,
      });
      session.setError(`Imported Apply Failed: ${rawError}`);
      return false;
    } finally {
      if (latestParamRenderGate.isCurrent(renderToken)) {
        stopMicrowaveHum('__manual__');
      }
    }
  }

  const currentDraftSignature = paramDraftSignature({
    threadId: snapshotThreadId ?? null,
    targetVersionId: targetVersionId ?? null,
    macroCode: codeToUse,
    params: currentParams,
    macroDialect: wc.macroDialect ?? null,
    geometryBackend: wc.geometryBackend ?? null,
    postProcessing: wc.postProcessing ?? null,
  });
  if (persist && snapshotThreadId && latestAppliedParamDraft?.signature === currentDraftSignature) {
    session.setStatus('Appending applied parameter draft...');
    try {
      await commitRenderedParamDraft({
        snapshotThreadId,
        codeToUse,
        currentParams,
        uiSpec: panel.uiSpec,
        postProcessing: wc.postProcessing ?? null,
        title:
          wc.title ||
          latestAppliedParamDraft.modelManifest?.document?.documentLabel ||
          latestAppliedParamDraft.modelManifest?.document?.documentName ||
          'Parameter Apply',
        versionName: wc.versionName || 'Param Apply',
        workingMacroDialect: wc.macroDialect,
        workingSourceLanguage: wc.sourceLanguage,
        workingGeometryBackend: wc.geometryBackend,
        draft: latestAppliedParamDraft,
      });
      latestAppliedParamDraft = null;
    } catch (e) {
      console.error('[ManualController] cached append failed:', formatBackendError(e), e);
      if (get(activeThreadId) === snapshotThreadId) {
        session.setError(`Apply Failed: ${formatBackendError(e)}`);
      }
      return false;
    }
    return true;
  }

  ensureContext();

  session.setStatus(`Executing ${workingCopyBackendLabel(wc)} engine...`);
  let persistedParamMessageId: string | null = null;
  try {
    const currentConfig = get(config);
    startMicrowaveHum('__manual__', currentConfig, snapshotThreadId);

    console.log('[ManualController] Invoking render_model with', { parameters: currentParams });
    recordRenderEvent({
      threadId: snapshotThreadId,
      versionId: targetVersionId,
      kind: 'render_started',
      title: 'Parameter render started',
      summary: `Rendering ${workingCopyBackendLabel(wc)} parameter draft.`,
      severity: 'info',
    });
    const bundle = await renderModel(
      codeToUse,
      currentParams,
      wc.macroDialect ?? null,
      wc.geometryBackend ?? null,
      wc.postProcessing ?? null,
      get(session).modelManifest,
    );
    const runtime = await inspectRuntimeBundle(
      bundle,
      undefined,
      wc.postProcessing ?? null,
      currentParams,
    );
    const renderableBundle =
      runtime.bundle ??
      getRenderableRuntimeBundle(bundle, wc.postProcessing ?? null, currentParams) ??
      bundle;
    const rawManifest = await getModelManifest(bundle.modelId);
    const previousManifest = get(session).modelManifest;
    const manifest =
      ensureSemanticManifest(rawManifest, panel.uiSpec, currentParams, previousManifest) ??
      rawManifest;
    const manifestChanged = JSON.stringify(manifest) !== JSON.stringify(rawManifest);
    if (manifestChanged && !persist) {
      await saveModelManifest(bundle.modelId, manifest, null);
    }

    if (!latestParamRenderGate.isCurrent(renderToken)) {
      return false;
    }

    if (get(activeThreadId) === snapshotThreadId) {
      session.setStlUrl(toAssetUrl(renderableBundle.modelStlPath));
      session.setModelRuntime(renderableBundle, manifest);
      recordRenderEvent({
        threadId: snapshotThreadId,
        versionId: targetVersionId,
        kind: 'render_succeeded',
        title: 'Parameter render succeeded',
        summary: 'Parameter draft rendered.',
        severity: 'success',
        raw: { modelId: renderableBundle.modelId, modelStlPath: renderableBundle.modelStlPath },
      });
    }

    if (get(activeThreadId) === snapshotThreadId) {
      const draftDesign = buildManualDesign({
        title: wc.title || manifest.document?.documentLabel || manifest.document?.documentName || 'Parameter Apply',
        versionName: wc.versionName || 'Param Apply',
        response: 'Parameters applied.',
        macroCode: codeToUse,
        bundle: renderableBundle,
        uiSpec: panel.uiSpec,
        params: currentParams,
        postProcessing: wc.postProcessing ?? null,
        workingMacroDialect: wc.macroDialect,
      });
      await persistLastSessionSnapshot({
        design: draftDesign,
        threadId: snapshotThreadId,
        messageId: targetVersionId ?? null,
        artifactBundle: renderableBundle,
        modelManifest: manifest,
        selectedPartId: null,
      });
      if (!persist) {
        latestAppliedParamDraft = {
          signature: currentDraftSignature,
          renderableBundle,
          modelManifest: manifest,
        };
      }
    }

    if (persist && snapshotThreadId) {
      latestAppliedParamDraft = null;
      const committedTitle = wc.title || manifest.document?.documentLabel || manifest.document?.documentName || 'Parameter Apply';
      const committedVersionName = wc.versionName || 'Param Apply';
      const committedDesign = buildManualDesign({
        title: committedTitle,
        versionName: committedVersionName,
        response: 'Parameter version appended.',
        macroCode: codeToUse,
        bundle: renderableBundle,
        uiSpec: panel.uiSpec,
        params: currentParams,
        postProcessing: wc.postProcessing ?? null,
        workingMacroDialect: wc.macroDialect,
      });
      const newMsgId = await addManualVersion({
        threadId: snapshotThreadId,
        title: committedTitle,
        versionName: committedVersionName,
        macroCode: codeToUse,
        sourceLanguage: renderableBundle.sourceLanguage || wc.sourceLanguage || null,
        geometryBackend: renderableBundle.geometryBackend || wc.geometryBackend || null,
        parameters: currentParams,
        uiSpec: panel.uiSpec,
        postProcessing: wc.postProcessing ?? null,
        artifactBundle: renderableBundle,
        modelManifest: manifest,
      });
      persistedParamMessageId = newMsgId;
      if (manifestChanged) {
        await saveModelManifest(bundle.modelId, manifest, newMsgId);
      }

      rememberCommittedVersionMessage(snapshotThreadId, committedTitle, {
        id: newMsgId,
        role: 'assistant',
        content: committedDesign.response,
        status: 'success',
        output: committedDesign,
        usage: null,
        artifactBundle: renderableBundle,
        modelManifest: manifest,
        agentOrigin: null,
        imageData: null,
        visualKind: null,
        attachmentImages: [],
        timestamp: Date.now() / 1000,
      });

      if (latestParamRenderGate.isCurrent(renderToken) && get(activeThreadId) === snapshotThreadId) {
        activeVersionId.set(newMsgId);
        workingCopy.loadVersion(committedDesign, newMsgId);
        restoreWorkingCopyMacroDraftIfNeeded(wc, committedDesign.macroCode);
        paramPanelState.hydrateFromVersion(committedDesign, newMsgId);
        await persistLastSessionSnapshot({
          design: committedDesign,
          threadId: snapshotThreadId,
          messageId: newMsgId,
          artifactBundle: renderableBundle,
          modelManifest: manifest,
          selectedPartId: null,
        });
        await refreshHistory();
        recordSessionActivityEvent({
          threadId: snapshotThreadId,
          versionId: newMsgId,
          kind: 'version_committed',
          title: 'Parameter version appended',
          summary: 'Parameter version appended.',
          severity: 'success',
        });
        session.setStatus('Parameter version appended.');
      }
    } else if (latestParamRenderGate.isCurrent(renderToken) && get(activeThreadId) === snapshotThreadId) {
      session.setStatus('Parameter preview applied.');
    }
  } catch (e) {
    const rawError = formatBackendError(e);
    console.error('[ManualController] render_model error:', rawError, e);
    if (persist && snapshotThreadId && codeToUse && !persistedParamMessageId) {
      try {
        const failedMessageId = await addManualVersion({
          threadId: snapshotThreadId,
          title: wc.title || 'Parameter Apply',
          versionName: wc.versionName || 'Param Apply',
          macroCode: codeToUse,
          sourceLanguage: wc.sourceLanguage || null,
          geometryBackend: wc.geometryBackend || null,
          parameters: currentParams,
          uiSpec: panel.uiSpec,
          postProcessing: wc.postProcessing ?? null,
          artifactBundle: null,
          modelManifest: null,
          status: 'error',
          errorMessage: rawError,
        });
        activeVersionId.set(failedMessageId);
        await refreshHistory();
      } catch (persistenceError) {
        console.error('[ManualController] failed parameter version persistence error:', persistenceError);
      }
    }
    if (latestParamRenderGate.isCurrent(renderToken) && get(activeThreadId) === snapshotThreadId) {
      recordRenderEvent({
        threadId: snapshotThreadId,
        versionId: targetVersionId,
        kind: 'render_failed',
        title: 'Parameter render failed',
        summary: rawError,
        severity: 'error',
        raw: e,
      });
      session.setError(
        e && typeof e === 'object' && 'code' in e && 'message' in e
          ? (e as import('../tauri/contracts').AppError)
          : `Render Error: ${rawError}`,
      );
    }
    return false;
  } finally {
    if (latestParamRenderGate.isCurrent(renderToken)) {
      stopMicrowaveHum('__manual__');
    }
  }
  return true;
}

export function stageParamChange(newParams: DesignParams) {
  const panel = get(paramPanelState);
  const currentParams = { ...panel.params, ...newParams };
  paramPanelState.setParams(currentParams);
  workingCopy.patch({ params: currentParams });
  session.setStatus('Parameters staged. Apply to rerender.');
}

function buildManualDesign(input: {
  title: string;
  versionName: string;
  response: string;
  macroCode: string;
  bundle: ArtifactBundle;
  uiSpec: UiSpec;
  params: DesignParams;
  postProcessing: PostProcessingSpec | null;
  workingMacroDialect: MacroDialect | null | undefined;
}): DesignOutput {
  return {
    title: input.title,
    versionName: input.versionName,
    response: input.response,
    interactionMode: "design",
    macroCode: input.macroCode,
    macroDialect:
      input.bundle.engineKind === 'ecky'
        ? 'ecky'
        : input.workingMacroDialect ?? 'legacy',
    sourceLanguage: input.bundle.sourceLanguage === 'build123d'
      ? 'ecky'
      : input.bundle.sourceLanguage || (input.bundle.engineKind === 'ecky' ? 'ecky' : 'legacyPython'),
    geometryBackend: input.bundle.geometryBackend === 'build123d'
      ? 'mesh'
      : input.bundle.geometryBackend || (input.bundle.engineKind === 'ecky' ? 'mesh' : 'freecad'),
    engineKind: input.bundle.engineKind === 'build123d' ? 'ecky' : input.bundle.engineKind,
    uiSpec: input.uiSpec,
    initialParams: input.params,
    postProcessing: input.postProcessing ?? null,
  };
}

export async function applyManualCodeDraft(editedCode: string) {
  const wc = get(workingCopy);
  const panel = get(paramPanelState);
  const snapshotThreadId = get(activeThreadId);
  const targetVersionId = panel.versionId || wc.sourceVersionId || get(activeVersionId);

  session.setStatus('Applying code draft...');
  session.setError(null);
  try {
    if (!snapshotThreadId) {
      throw new Error('Code Apply requires a bound design thread.');
    }
    const currentConfig = get(config);
    startMicrowaveHum('__manual__', currentConfig, snapshotThreadId);

    const reconciled = await reconcileManualControls(editedCode, panel.uiSpec, panel.params);
    const nextUiSpec = reconciled.uiSpec;
    const nextParams = reconciled.params;
    recordRenderEvent({
      threadId: snapshotThreadId,
      versionId: targetVersionId,
      kind: 'render_started',
      title: 'Code draft render started',
      summary: 'Rendering edited macro draft.',
      severity: 'info',
    });
    const bundle = await renderModel(
      editedCode,
      nextParams,
      null,
      null,
      wc.postProcessing ?? null,
      get(session).modelManifest,
    );
    const runtime = await inspectRuntimeBundle(
      bundle,
      undefined,
      wc.postProcessing ?? null,
      nextParams,
    );
    const renderableBundle =
      runtime.bundle ??
      getRenderableRuntimeBundle(bundle, wc.postProcessing ?? null, nextParams) ??
      bundle;
    const rawManifest = await getModelManifest(bundle.modelId);
    const previousManifest = get(session).modelManifest;
    const manifest =
      ensureSemanticManifest(rawManifest, nextUiSpec, nextParams, previousManifest) ??
      rawManifest;
    if (JSON.stringify(manifest) !== JSON.stringify(rawManifest)) {
      await saveModelManifest(bundle.modelId, manifest, null);
    }
    await saveProjectSource(snapshotThreadId, editedCode);

    const draftDesign = buildManualDesign({
      title: wc.title || 'Manual Edit',
      versionName: wc.versionName || 'Draft',
      response: 'Code draft applied.',
      macroCode: editedCode,
      bundle,
      uiSpec: nextUiSpec,
      params: nextParams,
      postProcessing: wc.postProcessing ?? null,
      workingMacroDialect: wc.macroDialect,
    });

    if (get(activeThreadId) === snapshotThreadId) {
      session.setStlUrl(toAssetUrl(renderableBundle.modelStlPath));
      session.setModelRuntime(renderableBundle, manifest);
      recordSessionActivityEvent({
        threadId: snapshotThreadId,
        versionId: targetVersionId,
        kind: 'macro_patch_applied',
        title: 'Code draft applied',
        summary: reconciled.parserMatched
          ? 'Code draft applied. Controls resynced from macro.'
          : 'Code draft applied.',
        severity: 'success',
        diffs: [
          {
            kind: 'text',
            label: 'Macro source',
            path: 'macro',
            before: wc.macroCode,
            after: editedCode,
          },
        ],
      });
      recordRenderEvent({
        threadId: snapshotThreadId,
        versionId: targetVersionId,
        kind: 'render_succeeded',
        title: 'Code draft render succeeded',
        summary: 'Edited macro draft rendered.',
        severity: 'success',
        raw: { modelId: renderableBundle.modelId, modelStlPath: renderableBundle.modelStlPath },
      });
      workingCopy.patch({
        macroCode: editedCode,
        macroDialect: draftDesign.macroDialect ?? wc.macroDialect,
        engineKind: draftDesign.engineKind ?? wc.engineKind,
        sourceLanguage: draftDesign.sourceLanguage,
        geometryBackend: draftDesign.geometryBackend,
        uiSpec: nextUiSpec,
        params: nextParams,
      });
      paramPanelState.hydrate({
        versionId: targetVersionId,
        uiSpec: nextUiSpec,
        params: nextParams,
      });
      await persistLastSessionSnapshot({
        design: draftDesign,
        threadId: snapshotThreadId,
        messageId: targetVersionId,
        artifactBundle: renderableBundle,
        modelManifest: manifest,
        selectedPartId: null,
      });
      session.setStatus('Code applied; watcher will append its version.');
    }

    return {
      design: draftDesign,
      artifactBundle: renderableBundle,
      modelManifest: manifest,
      parserMatched: reconciled.parserMatched,
    };
  } catch (e) {
    console.error('[ManualController] applyManualCodeDraft error:', formatBackendError(e), e);
    if (get(activeThreadId) === snapshotThreadId) {
      recordRenderEvent({
        threadId: snapshotThreadId,
        versionId: targetVersionId,
        kind: 'render_failed',
        title: 'Code draft render failed',
        summary: formatBackendError(e),
        severity: 'error',
        raw: e,
      });
      session.setError(`Apply Failed: ${formatBackendError(e)}`);
    }
    throw e;
  } finally {
    stopMicrowaveHum('__manual__');
  }
}

export async function commitManualVersion(
  editedCodeOrInput: string | ManualVersionCommitInput,
  titleOverride: string | null = null,
  options: ManualCommitOptions = {},
) {
  const wc = get(workingCopy);
  const panel = get(paramPanelState);
  const editedCode = typeof editedCodeOrInput === 'string' ? editedCodeOrInput : editedCodeOrInput.code;
  const inputTitle =
    typeof editedCodeOrInput === 'string' ? titleOverride : editedCodeOrInput.title ?? titleOverride;
  const inputVersionName =
    typeof editedCodeOrInput === 'string' ? options.versionName : editedCodeOrInput.versionName ?? options.versionName;
  const previousThreadId = get(activeThreadId);
  const snapshotThreadId = options.targetThreadId || previousThreadId || crypto.randomUUID();
  const activateTargetOnSuccess =
    options.activateTargetOnSuccess ??
    (!previousThreadId || snapshotThreadId !== previousThreadId);
  let persistedMessageId: string | null = null;

  session.setStatus("Applying manual edit as a new version...");
  session.setError(null);
  try {
    const currentConfig = get(config);
    startMicrowaveHum('__manual__', currentConfig, snapshotThreadId);
    const reconciled = await reconcileManualControls(editedCode, panel.uiSpec, panel.params);
    const nextUiSpec = reconciled.uiSpec;
    const nextParams = reconciled.params;

    recordRenderEvent({
      threadId: snapshotThreadId,
      versionId: panel.versionId || wc.sourceVersionId || get(activeVersionId),
      kind: 'render_started',
      title: 'Manual version render started',
      summary: 'Rendering appended manual edit.',
      severity: 'info',
    });
    const bundle = await renderModel(
      editedCode,
      nextParams,
      null,
      null,
      wc.postProcessing ?? null,
      get(session).modelManifest,
    );
    const runtime = await inspectRuntimeBundle(
      bundle,
      undefined,
      wc.postProcessing ?? null,
      nextParams,
    );
    const renderableBundle =
      runtime.bundle ??
      getRenderableRuntimeBundle(bundle, wc.postProcessing ?? null, nextParams) ??
      bundle;
    const rawManifest = await getModelManifest(bundle.modelId);
    const previousManifest = get(session).modelManifest;
    const manifest =
      ensureSemanticManifest(rawManifest, nextUiSpec, nextParams, previousManifest) ??
      rawManifest;
    const shouldSaveManifest = JSON.stringify(manifest) !== JSON.stringify(rawManifest);

    const committedTitle = inputTitle || wc.title || "Manual Edit";
    const committedVersionName = inputVersionName?.trim() || wc.versionName || "V-manual";
    const newMsgId = await addManualVersion({
      threadId: snapshotThreadId,
      title: committedTitle,
      versionName: committedVersionName,
      macroCode: editedCode,
      sourceLanguage: bundle.sourceLanguage || wc.sourceLanguage || null,
      geometryBackend: bundle.geometryBackend || wc.geometryBackend || null,
      parameters: nextParams,
      uiSpec: nextUiSpec,
      postProcessing: wc.postProcessing ?? null,
      artifactBundle: bundle,
      modelManifest: manifest,
    });
    persistedMessageId = newMsgId;

    const committedDesign: DesignOutput = {
      title: committedTitle,
      versionName: committedVersionName,
      response: "Manual edit appended as new version.",
      interactionMode: "design",
      macroCode: editedCode,
      macroDialect:
        bundle.engineKind === 'ecky'
          ? 'ecky'
          : wc.macroDialect ?? 'legacy',
      sourceLanguage: bundle.sourceLanguage === 'build123d'
        ? 'ecky'
        : bundle.sourceLanguage || (bundle.engineKind === 'ecky' ? 'ecky' : 'legacyPython'),
      geometryBackend: bundle.geometryBackend === 'build123d'
        ? 'mesh'
        : bundle.geometryBackend || (bundle.engineKind === 'ecky' ? 'mesh' : 'freecad'),
      engineKind: bundle.engineKind === 'build123d' ? 'ecky' : bundle.engineKind,
      uiSpec: nextUiSpec,
      initialParams: nextParams,
      postProcessing: wc.postProcessing ?? null,
    };
    rememberCommittedVersionMessage(snapshotThreadId, committedTitle, {
      id: newMsgId,
      role: 'assistant',
      content: committedDesign.response,
      status: 'success',
      output: committedDesign,
      usage: null,
      artifactBundle: renderableBundle,
      modelManifest: manifest,
      agentOrigin: null,
      imageData: null,
      visualKind: null,
      attachmentImages: [],
      timestamp: Date.now() / 1000,
    });

    if (activateTargetOnSuccess) {
      activeThreadId.set(snapshotThreadId);
      activeVersionId.set(null);
    }

    if (get(activeThreadId) === snapshotThreadId) {
      session.setStlUrl(toAssetUrl(renderableBundle.modelStlPath));
      session.setModelRuntime(renderableBundle, manifest);
      const previousWorkingCopy = get(workingCopy);
      workingCopy.loadVersion(committedDesign, newMsgId);
      restoreWorkingCopyMacroDraftIfNeeded(previousWorkingCopy, committedDesign.macroCode);
      paramPanelState.hydrateFromVersion(committedDesign, newMsgId);
      activeVersionId.set(newMsgId);
      closeWindowStore('code');
      recordRenderEvent({
        threadId: snapshotThreadId,
        versionId: newMsgId,
        kind: 'render_succeeded',
        title: 'Manual version render succeeded',
        summary: 'Appended manual edit rendered.',
        severity: 'success',
        raw: { modelId: renderableBundle.modelId, modelStlPath: renderableBundle.modelStlPath },
      });
      recordSessionActivityEvent({
        threadId: snapshotThreadId,
        versionId: newMsgId,
        kind: 'version_committed',
        title: 'Manual version appended',
        summary: committedDesign.response,
        severity: 'success',
        diffs: [
          {
            kind: 'text',
            label: 'Macro source',
            path: 'macro',
            before: wc.macroCode,
            after: editedCode,
          },
        ],
      });
      session.setStatus(
        options.successStatus ||
          (reconciled.parserMatched
            ? "Manual version appended. Controls resynced from macro."
            : "Manual version appended."),
      );
    }
    stopMicrowaveHum('__manual__');

    const shouldPersistSnapshot = get(activeThreadId) === snapshotThreadId;
    void runManualCommitHousekeeping(
      bundle.modelId,
      shouldSaveManifest,
      snapshotThreadId,
      committedDesign,
      newMsgId,
      renderableBundle,
      manifest,
      shouldPersistSnapshot,
    ).catch((error) => {
      console.warn('[ManualController] post-append housekeeping failed:', error);
    });
  } catch (e) {
    const rawError = formatBackendError(e);
    console.error('[ManualController] appendManualVersion error:', rawError, e);
    if (!persistedMessageId) {
      try {
        const failedMessageId = await addManualVersion({
          threadId: snapshotThreadId,
          title: inputTitle || wc.title || 'Manual Edit',
          versionName: inputVersionName?.trim() || wc.versionName || 'V-manual',
          macroCode: editedCode,
          sourceLanguage: wc.sourceLanguage || null,
          geometryBackend: wc.geometryBackend || null,
          parameters: panel.params,
          uiSpec: panel.uiSpec,
          postProcessing: wc.postProcessing ?? null,
          artifactBundle: null,
          modelManifest: null,
          status: 'error',
          errorMessage: rawError,
        });
        if (activateTargetOnSuccess) activeThreadId.set(snapshotThreadId);
        if (get(activeThreadId) === snapshotThreadId) activeVersionId.set(failedMessageId);
        await refreshHistory();
      } catch (persistenceError) {
        console.error(
          '[ManualController] failed manual version persistence error:',
          formatBackendError(persistenceError),
          persistenceError,
        );
      }
    }
    recordRenderEvent({
      threadId: snapshotThreadId,
      versionId: panel.versionId || wc.sourceVersionId || get(activeVersionId),
      kind: 'render_failed',
      title: 'Manual version render failed',
      summary: rawError,
      severity: 'error',
      raw: e,
    });
    session.setError(`Manual Apply Failed: ${rawError}`);
    stopMicrowaveHum('__manual__');
    throw e;
  }
}

export async function forkManualVersion(
  editedCodeOrInput: string | ManualVersionCommitInput,
  titleOverride: string | null = null,
) {
  const wc = get(workingCopy);
  const label =
    typeof editedCodeOrInput === 'string'
      ? titleOverride || wc.title || 'Manual Edit'
      : editedCodeOrInput.title || titleOverride || wc.title || 'Manual Edit';
  const confirmed = await confirmAction(`Fork "${label}" into a new thread with this code?`);
  if (!confirmed) return;

  await commitManualVersion(editedCodeOrInput, titleOverride, {
    targetThreadId: crypto.randomUUID(),
    activateTargetOnSuccess: true,
    successStatus: 'Forked into a new thread.',
  });
}
