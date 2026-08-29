import {
  commands,
  type ActiveProjectNavigation,
  type AgentDraftProjection,
  type AppError,
  type AppLogEntry,
  type CampaignRun,
  type CampaignStepPayload,
  type CampaignSummary,
  type CreateCampaignRunInput,
  type CaptureSessionInfo,
  type CaptureSessionState,
  type CaptureRun,
  type ExternalShapeSource,
  type ApplyExternalShapePlaneCropRequest,
  type ApplyExternalShapePlaneCropResult,
  type RemoveExternalShapePlaneCropRequest,
  type RemoveExternalShapePlaneCropResult,
  type SurfaceTrimPathPreviewRequest,
  type SurfaceTrimPathPreviewResponse,
  type SurfaceTrimLoopPreviewRequest,
  type SurfaceTrimLoopPreviewResponse,
  type SurfaceTrimRegionPreviewRequest,
  type SurfaceTrimRegionPreviewResponse,
  type ApplySurfaceTrimRequest,
  type ApplySurfaceTrimResult,
  type RemoveSurfaceTrimRequest,
  type RemoveSurfaceTrimResult,
  type CaptureReconstructionGuide,
  type CaptureReconstructionGuideState,
  type CaptureGuideSourceMesh,
  type CaptureGuideContext,
  type ReopenedCaptureRun,
  type QueuedCaptureGuidedReconstruction,
  type FemCancelResponse,
  type FemConvergenceRequest,
  type FemConvergenceResponse,
  type FemMeshPreviewResponse,
  type FemResultReadRequest,
  type FemResultReadResponse,
  type FemRunResponse,
  type FemStudyRequest,
  type FemStudyValidationResponse,
  type FemVtuExportResponse,
  type DeletedThreadsPage,
  type DenseTopologyItem,
  type DenseTopologyKind,
  type DenseTopologyPage,
  type MissionCoreIrEvaluation,
  type Result,
  type AgentActivityCatchUp,
  type ThreadWindowLayout,
  type WebContentRecoveryState,
  type AgyMessagePage,
  type AgyMessagePageInput,
  type AgyPromptInput,
  type AgyProviderSnapshot,
  type AgyStopInput,
  type ProviderWriterActivationInput,
  type CodexMessagePage,
  type CodexMessagePageInput,
  type CodexPromptInput,
  type CodexSteerInput,
  type CodexStopInput,
  type CodexTakeoverSnapshot,
} from './contracts';
import {
  normalizeArtifactBundle,
  normalizeAttachment,
  normalizeConfig,
  normalizeDeletedMessage,
  normalizeDesignOutput,
  normalizeLastDesignSnapshot,
  normalizeModelManifest,
  normalizeMessage,
  normalizeParsedParamsResult,
  normalizeRuntimeCapabilities,
  normalizeThreadMessagesPage,
  normalizeThread,
  normalizeUsageSummary,
  toContractAttachment,
  toContractDesignOutput,
  toContractLastDesignSnapshot,
  toContractUsageSummary,
  toContractUiSpec,
  type AgentSession,
  type AgentTerminalInput,
  type AgentTerminalSnapshot,
  type ArtifactBundle,
  type AppConfig,
  type Attachment,
  type DeletedMessage,
  type DesignOutput,
  type DesignParams,
  type EngineKind,
  type FinalizeStatus,
  type GeometryBackend,
  type MessageStatus,
  type GenerateOutput,
  type IntentDecision,
  type LastDesignSnapshot,
  type MacroDialect,
  type Message,
  type ModelManifest,
  type McpServerStatus,
  type ParsedParamsResult,
  type RuntimeCapabilities,
  type StructuralVerificationResult,
  type VisualVerificationResult,
  type SourceLanguage,
  type Thread,
  type ThreadMessagesPage,
  type UiSpec,
  type UsageSummary,
  type ViewportCameraState,
} from '../types/domain';
import { resolveSketchPreviewDraftScopeId } from '../sketchPreviewDraftStore';
import {
  decodeAuthoringGraph,
  type AuthoringGraph,
  type AuthoringGraphRequest,
} from '../authoringGraph';
import type {
  ComponentPackage,
  ComponentPackageHeader,
  BrepHiddenLineProjectionRequest,
  BrepHiddenLineProjectionResponse,
  ClearSketchPreviewDraftRequest,
  ExportPartInput,
  FreecadLibraryImportRequest,
  FreecadLibraryItem,
  FreecadLibrarySearchRequest,
  InstalledComponentPackage,
  LoadSketchPreviewDraftRequest,
  PostProcessingSpec,
  PromptTranscription,
  QueueAgentPromptInput,
  RasterTraceRequest,
  RasterTraceResponse,
  RejectViewportScreenshotInput,
  ResolveAgentPromptInput,
  ResolveViewportScreenshotInput,
  SketchAcceptedBrepComponentPackageRequest,
  SketchBrepCandidateRequest,
  SketchBrepCandidateAcceptRequest,
  SketchBrepCandidateAcceptResponse,
  SketchBrepCandidateResponse,
  SketchDraftRequest,
  SketchDraftSource,
  SketchDocument,
  SaveSketchPreviewDraftRequest,
  SketchPreviewDraft,
  SketchPreviewHullRequest,
  SketchSuggestionRequest,
  SketchSuggestionResponse,
  TranscribePromptAudioInput,
  VersionPreviewRuntime,
} from './contracts';

export type {
  FemResultReadRequest,
  FemResultReadResponse,
  FemRunResponse,
  FemConvergenceRequest,
  FemConvergenceResponse,
  FemMeshPreviewResponse,
  FemStudyRequest,
  FemStudyValidationResponse,
  FemVtuExportResponse,
  CodexMessagePage,
  CodexTakeoverSnapshot,
  AgyMessagePage,
  AgyProviderSnapshot,
};

export async function activateProviderWriter(
  input: ProviderWriterActivationInput,
): Promise<void> {
  await invokeCommand(commands.activateProviderWriter(input));
}

export async function getAgyProvider(
  eckyThreadId: string,
): Promise<AgyProviderSnapshot | null> {
  return invokeCommand(commands.getAgyProvider(eckyThreadId));
}

export async function getAgyProviderMessages(
  input: AgyMessagePageInput,
): Promise<AgyMessagePage> {
  return invokeCommand(commands.getAgyProviderMessages(input));
}

export async function sendAgyProviderPrompt(
  input: AgyPromptInput,
): Promise<AgyProviderSnapshot> {
  return invokeCommand(commands.sendAgyProviderPrompt(input));
}

export async function dispatchAgyPromptQueue(
  eckyThreadId: string,
): Promise<AgyProviderSnapshot> {
  return invokeCommand(commands.dispatchAgyPromptQueue(eckyThreadId));
}

export async function stopAgyProvider(
  input: AgyStopInput,
): Promise<AgyProviderSnapshot> {
  return invokeCommand(commands.stopAgyProvider(input));
}

export async function retryAgyQueuedPrompt(
  eckyThreadId: string,
  queueId: string,
): Promise<AgyProviderSnapshot> {
  return invokeCommand(commands.retryAgyQueuedPrompt(eckyThreadId, queueId));
}

export async function removeAgyQueuedPrompt(
  eckyThreadId: string,
  queueId: string,
): Promise<AgyProviderSnapshot> {
  return invokeCommand(commands.removeAgyQueuedPrompt(eckyThreadId, queueId));
}

export async function getCodexTakeover(
  eckyThreadId: string,
): Promise<CodexTakeoverSnapshot | null> {
  return invokeCommand(commands.getCodexTakeover(eckyThreadId));
}

export async function getCodexTakeoverMessages(
  input: CodexMessagePageInput,
): Promise<CodexMessagePage> {
  return invokeCommand(commands.getCodexTakeoverMessages(input));
}

export async function sendCodexTakeoverPrompt(
  input: CodexPromptInput,
): Promise<CodexTakeoverSnapshot> {
  return invokeCommand(commands.sendCodexTakeoverPrompt(input));
}

export async function dispatchCodexPromptQueue(
  eckyThreadId: string,
): Promise<CodexTakeoverSnapshot> {
  return invokeCommand(commands.dispatchCodexPromptQueue(eckyThreadId));
}

export async function steerCodexTakeover(
  input: CodexSteerInput,
): Promise<CodexTakeoverSnapshot> {
  return invokeCommand(commands.steerCodexTakeover(input));
}

export async function stopCodexTakeover(
  input: CodexStopInput,
): Promise<CodexTakeoverSnapshot> {
  return invokeCommand(commands.stopCodexTakeover(input));
}

export async function retryCodexQueuedPrompt(
  eckyThreadId: string,
  queueId: string,
): Promise<CodexTakeoverSnapshot> {
  return invokeCommand(commands.retryCodexQueuedPrompt(eckyThreadId, queueId));
}

export async function removeCodexQueuedPrompt(
  eckyThreadId: string,
  queueId: string,
): Promise<CodexTakeoverSnapshot> {
  return invokeCommand(commands.removeCodexQueuedPrompt(eckyThreadId, queueId));
}

export type AppErrorDiagnosticField = {
  key: string;
  value: string;
};

export type AppErrorDiagnosticContext = {
  detailText: string | null;
  rawTail: string | null;
  stableNodeKey: string | null;
  partKey: string | null;
  operation: string | null;
  startLine: number | null;
  endLine: number | null;
  fields: AppErrorDiagnosticField[];
};

function unwrapResult<T>(result: Result<T, AppError>): T {
  if (result.status === 'ok') {
    return result.data;
  }
  throw result.error;
}

async function invokeCommand<T>(command: Promise<Result<T, AppError>>): Promise<T>;
async function invokeCommand<T, R>(
  command: Promise<Result<T, AppError>>,
  transform: (value: T) => R,
): Promise<R>;
async function invokeCommand<T, R>(
  command: Promise<Result<T, AppError>>,
  transform?: (value: T) => R,
): Promise<T | R> {
  const value = unwrapResult(await command);
  return transform ? transform(value) : value;
}

function isBackendError(error: unknown): error is AppError {
  return Boolean(
    error &&
      typeof error === 'object' &&
      'code' in error &&
      'message' in error &&
      typeof (error as { message?: unknown }).message === 'string',
  );
}

function parseDiagnosticTail(rawDetails: string | null | undefined): {
  detailText: string | null;
  rawTail: string | null;
  fields: AppErrorDiagnosticField[];
} {
  const details = `${rawDetails ?? ''}`.trim();
  if (!details) {
    return { detailText: null, rawTail: null, fields: [] };
  }
  const lines = details
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
  const rawTail = lines.at(-1) ?? null;
  if (!rawTail) {
    return { detailText: details, rawTail: null, fields: [] };
  }
  const fields = rawTail
    .split(/\s+/)
    .map((token) => {
      const equalIndex = token.indexOf('=');
      if (equalIndex <= 0 || equalIndex === token.length - 1) return null;
      return {
        key: token.slice(0, equalIndex),
        value: token.slice(equalIndex + 1),
      } satisfies AppErrorDiagnosticField;
    })
    .filter((field): field is AppErrorDiagnosticField => Boolean(field));
  if (fields.length === 0 || fields.length !== rawTail.split(/\s+/).length) {
    return { detailText: details, rawTail: null, fields: [] };
  }
  const detailText = lines.slice(0, -1).join('\n').trim() || null;
  return { detailText, rawTail, fields };
}

function formatDiagnosticLine(context: AppErrorDiagnosticContext): string | null {
  const parts = [...context.fields.map((field) => `${field.key}=${field.value}`)];
  if (context.partKey && !context.fields.some((field) => field.key === 'part')) {
    parts.unshift(`part=${context.partKey}`);
  }
  if (context.operation && !context.fields.some((field) => field.key === 'op')) {
    parts.push(`op=${context.operation}`);
  }
  if (context.startLine !== null && !context.fields.some((field) => field.key === 'lines')) {
    parts.push(
      context.endLine !== null && context.endLine !== context.startLine
        ? `lines=${context.startLine}-${context.endLine}`
        : `lines=${context.startLine}`,
    );
  }
  return parts.length > 0 ? parts.join(' | ') : null;
}

export function getAppErrorDiagnosticContext(error: unknown): AppErrorDiagnosticContext | null {
  if (!isBackendError(error)) return null;
  const parsed = parseDiagnosticTail(error.details);
  const resolvedParamFields = (error.diagnosticContext?.resolvedParams ?? []).map((param) => ({
    key: param.key,
    value:
      typeof param.value === 'number' || typeof param.value === 'boolean'
        ? `${param.value}`
        : param.value === null
          ? 'null'
          : `${param.value}`,
  }));
  return {
    detailText: parsed.detailText,
    rawTail: parsed.rawTail,
    stableNodeKey: error.stableNodeKey ?? null,
    partKey: error.diagnosticContext?.partKey ?? null,
    operation: error.diagnosticContext?.opName ?? error.operation ?? null,
    startLine: error.diagnosticContext?.startLine ?? error.startLine ?? null,
    endLine: error.diagnosticContext?.endLine ?? error.endLine ?? null,
    fields: resolvedParamFields.length > 0 ? resolvedParamFields : parsed.fields,
  };
}

export function formatBackendError(error: unknown): string {
  if (isBackendError(error)) {
    const context = getAppErrorDiagnosticContext(error);
    const sections = [error.message];
    if (context?.detailText) {
      sections.push(context.detailText);
    } else if (error.details && !context) {
      sections.push(error.details);
    }
    const diagnosticLine = context ? formatDiagnosticLine(context) : null;
    if (diagnosticLine) {
      sections.push(`Context: ${diagnosticLine}`);
    } else if (error.details && !context?.detailText) {
      sections.push(error.details);
    }
    return sections.join('\n');
  }
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

export async function getConfig(): Promise<AppConfig> {
  return invokeCommand(commands.getConfig(), normalizeConfig);
}

export async function getRuntimeCapabilities(): Promise<RuntimeCapabilities> {
  return invokeCommand(commands.getRuntimeCapabilities(), normalizeRuntimeCapabilities);
}

export async function saveConfig(config: AppConfig): Promise<void> {
  await invokeCommand(commands.saveConfig(config));
}

export async function listModels(
  provider: string,
  apiKey: string,
  baseUrl: string,
): Promise<string[]> {
  return invokeCommand(commands.listModels(provider, apiKey, baseUrl));
}

export async function getDesignSystemPrompt(provider?: string | null): Promise<string> {
  return invokeCommand(commands.getDesignSystemPrompt(provider ?? null));
}

export async function listAgentModels(cmd: string): Promise<{ models: string[]; isLive: boolean }> {
  return invokeCommand(commands.listAgentModels(cmd));
}

export async function listProviderModels(provider: string): Promise<{ models: string[]; isLive: boolean }> {
  return invokeCommand(commands.listProviderModels(provider));
}

export async function getHistory(): Promise<Thread[]> {
  return invokeCommand(commands.getHistory(), (threads) => threads.map(normalizeThread));
}

export async function getThread(id: string): Promise<Thread> {
  return invokeCommand(commands.getThread(id), normalizeThread);
}

export async function getThreadLatestVersion(threadId: string): Promise<Message | null> {
  const headId = await invokeCommand(commands.getThreadHeadVersionId(threadId));
  if (headId) {
    const detail = await invokeCommand(commands.getVersionDetail(threadId, headId));
    return detail ? hydrateVersionDetail(threadId, headId, detail) : null;
  }
  const fallback = await invokeCommand(commands.getThreadLatestVersion(threadId));
  if (!fallback) return null;
  return getHydratedVersionDetail(threadId, fallback.id, fallback);
}

export async function getThreadMessageVersion(
  threadId: string,
  messageId: string,
): Promise<Message | null> {
  const detail = await invokeCommand(commands.getVersionDetail(threadId, messageId));
  if (!detail) {
    const message = await invokeCommand(commands.getThreadMessageVersion(threadId, messageId));
    return message ? normalizeMessage(message) : null;
  }
  return hydrateVersionDetail(threadId, messageId, detail);
}

async function getHydratedVersionDetail(
  threadId: string,
  messageId: string,
  fallback: import('./contracts').Message,
): Promise<Message> {
  const detail = await invokeCommand(commands.getVersionDetail(threadId, messageId));
  return detail
    ? hydrateVersionDetail(threadId, messageId, detail)
    : normalizeMessage(fallback);
}

async function hydrateVersionDetail(
  threadId: string,
  messageId: string,
  detail: import('./contracts').VersionDetail,
): Promise<Message> {
  const message = structuredClone(detail.message);
  await hydrateBoundedTopology(
    detail.edgeCount,
    detail.faceCount,
    detail.selectionTargetCount,
    async (kind, cursor) => invokeCommand(
      commands.getDenseTopologyPage(threadId, messageId, kind, cursor, 500),
    ),
    message,
  );
  return normalizeMessage(message);
}

const MAX_INTERACTIVE_TOPOLOGY_TARGETS = 1_000;

async function readTopologyKind(
  kind: DenseTopologyKind,
  loadPage: (kind: DenseTopologyKind, cursor: string | null) => Promise<DenseTopologyPage>,
): Promise<DenseTopologyItem[]> {
  const items: DenseTopologyItem[] = [];
  let cursor: string | null = null;
  do {
    const page = await loadPage(kind, cursor);
    items.push(...page.items);
    cursor = page.nextCursor;
  } while (cursor && items.length < MAX_INTERACTIVE_TOPOLOGY_TARGETS);
  return items;
}

async function hydrateBoundedTopology(
  edgeCount: number,
  faceCount: number,
  selectionTargetCount: number,
  loadPage: (kind: DenseTopologyKind, cursor: string | null) => Promise<DenseTopologyPage>,
  target: {
    artifactBundle?: { edgeTargets?: unknown[]; faceTargets?: unknown[] } | null;
    modelManifest?: { selectionTargets?: unknown[] } | null;
  },
): Promise<void> {
  const total = edgeCount + faceCount + selectionTargetCount;
  if (total === 0 || total > MAX_INTERACTIVE_TOPOLOGY_TARGETS) return;
  const [edges, faces, selections] = await Promise.all([
    edgeCount > 0 ? readTopologyKind('edge', loadPage) : [],
    faceCount > 0 ? readTopologyKind('face', loadPage) : [],
    selectionTargetCount > 0 ? readTopologyKind('selection', loadPage) : [],
  ]);
  if (target.artifactBundle) {
    target.artifactBundle.edgeTargets = edges
      .filter((item) => item.kind === 'edge')
      .map((item) => item.value);
    target.artifactBundle.faceTargets = faces
      .filter((item) => item.kind === 'face')
      .map((item) => item.value);
  }
  if (target.modelManifest) {
    target.modelManifest.selectionTargets = selections
      .filter((item) => item.kind === 'selection')
      .map((item) => item.value);
  }
}

export async function materializeVersionPreview(
  threadId: string,
  messageId: string,
): Promise<VersionPreviewRuntime> {
  const runtime = await invokeCommand(commands.materializeVersionPreview(threadId, messageId));
  return {
    ...runtime,
    artifactBundle: normalizeArtifactBundle(runtime.artifactBundle),
    modelManifest: normalizeModelManifest(runtime.modelManifest),
  };
}

export async function releaseVersionPreview(leaseId: string): Promise<void> {
  await invokeCommand(commands.releaseVersionPreview(leaseId));
}

export async function getThreadMessagesPage(
  threadId: string,
  before: string | null = null,
  limit = 50,
  includeVisualPayloads = false,
): Promise<ThreadMessagesPage> {
  return invokeCommand(
    commands.getThreadMessagesPage(threadId, before, limit, includeVisualPayloads),
    normalizeThreadMessagesPage,
  );
}

export async function getVersionSource(threadId: string, messageId: string): Promise<string | null> {
  const chunks: string[] = [];
  let startByte = 0;
  let expectedDigest: string | null = null;
  while (true) {
    const window = await invokeCommand(
      commands.getVersionSourceWindow(threadId, messageId, startByte, 256 * 1024),
    );
    if (!window) return null;
    if (expectedDigest && window.digest !== expectedDigest) {
      throw new Error(
        `Version source changed during windowed read: expected ${expectedDigest}, received ${window.digest}.`,
      );
    }
    expectedDigest = window.digest;
    chunks.push(window.content);
    if (window.nextStartByte === null) break;
    if (window.nextStartByte <= startByte) {
      throw new Error(`Version source window did not advance beyond byte ${startByte}.`);
    }
    startByte = window.nextStartByte;
  }
  return chunks.join('');
}

export async function deleteThread(id: string): Promise<void> {
  await invokeCommand(commands.deleteThread(id));
}

export async function renameThread(id: string, title: string): Promise<void> {
  await invokeCommand(commands.renameThread(id, title));
}

export async function deleteVersion(messageId: string): Promise<void> {
  await invokeCommand(commands.deleteVersion(messageId));
}

export async function restoreVersion(messageId: string): Promise<void> {
  await invokeCommand(commands.restoreVersion(messageId));
}

export async function getDeletedMessages(): Promise<DeletedMessage[]> {
  return invokeCommand(commands.getDeletedMessages(), (messages) =>
    messages.map(normalizeDeletedMessage),
  );
}

export async function getDeletedThreadsPage(
  before: string | null = null,
  limit = 24,
): Promise<DeletedThreadsPage> {
  return invokeCommand(commands.getDeletedThreadsPage(before, limit));
}

export async function getDeletedThreadPreview(id: string): Promise<string | null> {
  return invokeCommand(commands.getDeletedThreadPreview(id));
}

export async function getThreadPreview(id: string): Promise<string | null> {
  return invokeCommand(commands.getThreadPreview(id));
}

export async function restoreDeletedThread(id: string): Promise<void> {
  await invokeCommand(commands.restoreDeletedThread(id));
}

export async function hideDeletedMessage(messageId: string): Promise<void> {
  await invokeCommand(commands.hideDeletedMessage(messageId));
}

export async function finalizeThread(id: string, messageId: string | null = null): Promise<void> {
  await invokeCommand(commands.finalizeThread(id, messageId));
}

export async function reopenThread(id: string): Promise<void> {
  await invokeCommand(commands.reopenThread(id));
}

export async function getInventory(): Promise<Thread[]> {
  return invokeCommand(commands.getInventory(), (threads) => threads.map(normalizeThread));
}

export async function generateDesign(input: {
  prompt: string;
  threadId: string | null;
  parentMacroCode: string | null;
  workingDesign: DesignOutput | null;
  isRetry: boolean;
  imageData: string | null;
  attachments: Attachment[];
  questionMode: boolean | null;
  followUpQuestion: string | null;
  engineKind?: EngineKind | null;
  sourceLanguage?: SourceLanguage | null;
  geometryBackend?: GeometryBackend | null;
}): Promise<GenerateOutput> {
  const result = await invokeCommand(
    commands.generateDesign(
      input.prompt,
      input.threadId,
      input.parentMacroCode,
      input.workingDesign ? toContractDesignOutput(input.workingDesign) : null,
      input.isRetry,
      input.imageData,
      input.attachments.map(toContractAttachment),
      {
        questionMode: input.questionMode,
        followUpQuestion: input.followUpQuestion,
        engineKind: input.engineKind ?? null,
        sourceLanguage: input.sourceLanguage ?? null,
        geometryBackend: input.geometryBackend ?? null,
      },
    ),
  );
  return {
    design: normalizeDesignOutput(result.design),
    threadId: result.threadId,
    messageId: result.messageId,
    usage: normalizeUsageSummary(result.usage),
  };
}

export async function initGenerationAttempt(input: {
  threadId: string;
  prompt: string;
  attachments: Attachment[];
  imageData: string | null;
}): Promise<string> {
  return invokeCommand(
    commands.initGenerationAttempt(
      input.threadId,
      input.prompt,
      input.attachments.map(toContractAttachment),
      input.imageData,
    ),
  );
}

export async function finalizeGenerationAttempt(input: {
  messageId: string;
  status: FinalizeStatus;
  design?: DesignOutput;
  usage?: UsageSummary | null;
  artifactBundle?: ArtifactBundle | null;
  modelManifest?: ModelManifest | null;
  errorMessage?: string;
  responseText?: string;
}): Promise<void> {
  await invokeCommand(
    commands.finalizeGenerationAttempt(
      input.messageId,
      input.status,
      input.design ? toContractDesignOutput(input.design) : null,
      toContractUsageSummary(input.usage),
      input.artifactBundle ?? null,
      input.modelManifest ?? null,
      input.errorMessage ?? null,
      input.responseText ?? null,
    ),
  );
}

export async function persistStructuralVerification(
  messageId: string,
  structuralVerification: StructuralVerificationResult,
): Promise<void> {
  await invokeCommand(
    commands.persistStructuralVerification(messageId, structuralVerification),
  );
}

export async function classifyIntent(input: {
  prompt: string;
  threadId: string | null;
  context: string | null;
  imageData: string | null;
  attachments: Attachment[];
}): Promise<IntentDecision> {
  const result = await invokeCommand(
    commands.classifyIntent(
      input.prompt,
      input.threadId,
      input.context,
      input.imageData,
      input.attachments.map(toContractAttachment),
    ),
  );
  return {
    ...result,
    usage: normalizeUsageSummary(result.usage),
  };
}

export type { MacroAstSourceNode } from './contracts';

export async function openProjectInEditor(
  threadId: string | null,
  messageId: string | null,
): Promise<import('./contracts').ProjectEditorLink> {
  return invokeCommand(commands.openProjectInEditor(threadId, messageId));
}

export async function openImportedCadSource(
  threadId: string | null,
  messageId: string | null,
): Promise<import('./contracts').ProjectEditorLink> {
  return invokeCommand(commands.openImportedCadSource(threadId, messageId));
}

export async function getProjectSource(
  threadId: string,
): Promise<import('./contracts').ProjectSourceDocument> {
  return invokeCommand(commands.getProjectSource(threadId));
}

export async function listExternalShapeSources(
  threadId: string,
): Promise<ExternalShapeSource[]> {
  return invokeCommand(commands.listExternalShapeSources(threadId));
}

export async function applyExternalShapePlaneCrop(
  request: ApplyExternalShapePlaneCropRequest,
): Promise<ApplyExternalShapePlaneCropResult> {
  return invokeCommand(commands.applyExternalShapePlaneCrop(request));
}

export async function removeExternalShapePlaneCrop(
  request: RemoveExternalShapePlaneCropRequest,
): Promise<RemoveExternalShapePlaneCropResult> {
  return invokeCommand(commands.removeExternalShapePlaneCrop(request));
}

export async function previewExternalShapeSurfaceTrimPath(
  request: SurfaceTrimPathPreviewRequest,
): Promise<SurfaceTrimPathPreviewResponse> {
  return invokeCommand(commands.previewExternalShapeSurfaceTrimPath(request));
}

export async function previewExternalShapeSurfaceTrimLoop(
  request: SurfaceTrimLoopPreviewRequest,
): Promise<SurfaceTrimLoopPreviewResponse> {
  return invokeCommand(commands.previewExternalShapeSurfaceTrimLoop(request));
}

export async function previewExternalShapeSurfaceTrimRegion(
  request: SurfaceTrimRegionPreviewRequest,
): Promise<SurfaceTrimRegionPreviewResponse> {
  return invokeCommand(commands.previewExternalShapeSurfaceTrimRegion(request));
}

export async function applyExternalShapeSurfaceTrim(
  request: ApplySurfaceTrimRequest,
): Promise<ApplySurfaceTrimResult> {
  return invokeCommand(commands.applyExternalShapeSurfaceTrim(request));
}

export async function removeExternalShapeSurfaceTrim(
  request: RemoveSurfaceTrimRequest,
): Promise<RemoveSurfaceTrimResult> {
  return invokeCommand(commands.removeExternalShapeSurfaceTrim(request));
}

export async function saveProjectSource(
  threadId: string,
  source: string,
): Promise<import('./contracts').ProjectSourceDocument> {
  return invokeCommand(commands.saveProjectSource(threadId, source));
}

export async function openOrCreateBlankDesignThread(
  title: string | null = null,
): Promise<import('./contracts').ProjectSourceDocument> {
  return invokeCommand(commands.openOrCreateBlankDesignThread(title));
}

export async function revealProjectFolder(
  threadId: string | null,
  messageId: string | null,
): Promise<import('./contracts').ProjectEditorLink> {
  return invokeCommand(commands.revealProjectFolder(threadId, messageId));
}

export type {
  ProjectFolderExportResult,
  ProjectFolderStatus,
  ProjectFolderApplyResult,
  ProjectFolderRenderActivity,
  ProjectManifest,
  ProjectSyncState,
} from './contracts';

export async function projectFolderExport(
  threadId: string | null,
  messageId: string | null,
): Promise<import('./contracts').ProjectFolderExportResult> {
  return invokeCommand(commands.projectFolderExport(threadId, messageId));
}

export async function projectFolderStatus(
  threadId: string | null,
  messageId: string | null,
): Promise<import('./contracts').ProjectFolderStatus> {
  return invokeCommand(commands.projectFolderStatus(threadId, messageId));
}

export async function projectFolderRenderActivity(): Promise<import('./contracts').ProjectFolderRenderActivity[]> {
  return invokeCommand(commands.projectFolderRenderActivity());
}

export async function projectFolderApply(
  threadId: string | null,
  messageId: string | null,
  force = false,
  title: string | null = null,
  versionName: string | null = null,
): Promise<import('./contracts').ProjectFolderApplyResult> {
  return invokeCommand(
    commands.projectFolderApply(threadId, messageId, force, title, versionName),
  );
}

export async function macroAstSourceMap(macroCode: string): Promise<import('./contracts').MacroAstSourceNode[]> {
  return invokeCommand(commands.macroAstSourceMap(macroCode));
}

export async function renderModel(
  macroCode: string,
  parameters: DesignParams,
  macroDialect?: MacroDialect | null,
  geometryBackend?: GeometryBackend | null,
  postProcessing?: PostProcessingSpec | null,
  previousManifest?: ModelManifest | null,
): Promise<ArtifactBundle> {
  const trace = {
    event: 'render.invoke',
    at: Date.now(),
    macroLength: macroCode.length,
    macroDialect: macroDialect ?? null,
    geometryBackend: geometryBackend ?? null,
    parameterKeys: Object.keys(parameters),
    previousModelId: previousManifest?.modelId ?? null,
    stack: new Error().stack,
  };
  console.warn('[CAD_FLOW][render.invoke]', trace);
  const flow = ((globalThis as any).__ECKY_CAD_FLOW__ ??= []);
  flow.push(trace);
  const bundle = await invokeCommand(
    commands.renderModel(
      macroCode,
      parameters,
      macroDialect ?? null,
      geometryBackend ?? null,
      postProcessing ?? null,
      previousManifest ?? null,
    ),
    normalizeArtifactBundle,
  );
  console.warn('[CAD_FLOW][render.result]', {
    modelId: bundle.modelId,
    modelStlPath: bundle.modelStlPath,
    contentHash: bundle.contentHash,
  });
  return bundle;
}

export type { PostProcessingSpec };

export async function importFcstd(sourcePath: string): Promise<ArtifactBundle> {
  return invokeCommand(commands.importFcstd(sourcePath), normalizeArtifactBundle);
}

export async function searchFreecadLibrary(
  request: FreecadLibrarySearchRequest,
): Promise<FreecadLibraryItem[]> {
  return invokeCommand(commands.searchFreecadLibrary(request));
}

export async function importFreecadLibraryPart(
  request: FreecadLibraryImportRequest,
): Promise<ArtifactBundle> {
  return invokeCommand(commands.importFreecadLibraryPart(request), normalizeArtifactBundle);
}

export async function applyImportedModel(
  artifactBundle: ArtifactBundle,
  manifest: ModelManifest,
  parameters: DesignParams,
  messageId?: string | null,
): Promise<ArtifactBundle> {
  return invokeCommand(
    commands.applyImportedModel(artifactBundle, manifest, parameters, messageId ?? null),
    normalizeArtifactBundle,
  );
}

export async function getModelManifest(modelId: string): Promise<ModelManifest> {
  return invokeCommand(commands.getModelManifest(modelId), normalizeModelManifest);
}

export async function getAuthoringGraph(request: AuthoringGraphRequest): Promise<AuthoringGraph> {
  const graph = await invokeCommand(commands.getAuthoringGraph(request));
  return decodeAuthoringGraph(graph);
}

export async function saveModelManifest(
  modelId: string,
  manifest: ModelManifest,
  messageId?: string | null,
): Promise<void> {
  await invokeCommand(commands.saveModelManifest(modelId, manifest, messageId ?? null));
}

export async function getDefaultMacro(): Promise<string> {
  return invokeCommand(commands.getDefaultMacro());
}

export async function getMessStlPath(): Promise<string> {
  return invokeCommand(commands.getMessStlPath());
}

export async function exportFile(sourcePath: string, targetPath: string): Promise<void> {
  await invokeCommand(commands.exportFile(sourcePath, targetPath));
}

export async function exportEckyMcpSkillZip(targetPath: string): Promise<void> {
  await invokeCommand(commands.exportEckyMcpSkillZip(targetPath));
}

export async function exportDocsBookEpub(targetPath: string): Promise<void> {
  await invokeCommand(commands.exportDocsBookEpub(targetPath));
}

export async function installComponentPackageArchive(
  archivePath: string,
): Promise<InstalledComponentPackage> {
  return invokeCommand(commands.installComponentPackageArchive(archivePath));
}

export async function listInstalledComponentPackageHeaders(): Promise<ComponentPackageHeader[]> {
  return invokeCommand(commands.listInstalledComponentPackageHeaders());
}

export type CopyInlineComponentImportRequest = {
  packageId: string;
  version: string;
  componentId: string;
  authoredSource: string;
};

export type CopyInlineComponentImportResponse = {
  authoredSource: string;
  componentSource: string;
  entrySymbol: string;
  partKey: string;
};

export async function copyInlineComponentImport(
  request: CopyInlineComponentImportRequest,
): Promise<CopyInlineComponentImportResponse> {
  return invokeCommand(commands.componentImportCopyInline(request));
}

export async function suggestSketchFeatures(
  request: SketchSuggestionRequest,
): Promise<SketchSuggestionResponse> {
  return invokeCommand(commands.suggestSketchFeatures(request));
}

export async function generateSketchDraftPreview(
  request: SketchDraftRequest,
): Promise<{ draft: SketchDraftSource; artifactBundle: ArtifactBundle }> {
  const [draft, bundle] = await invokeCommand(commands.generateSketchDraftPreview(request));
  return { draft, artifactBundle: normalizeArtifactBundle(bundle) };
}

export async function generateSketchPreviewHull(
  request: SketchPreviewHullRequest,
): Promise<{ draft: SketchDraftSource; artifactBundle: ArtifactBundle }> {
  const [draft, bundle] = await invokeCommand(commands.generateSketchPreviewHull(request));
  return { draft, artifactBundle: normalizeArtifactBundle(bundle) };
}

export async function traceRasterReference(
  request: RasterTraceRequest,
): Promise<RasterTraceResponse> {
  return invokeCommand(commands.traceRasterReference(request));
}

export async function saveSketchPreviewDraft(input: {
  scopeId?: string | null;
  draftScopeId?: string | null;
  draftSource: SketchDraftSource;
  artifactBundle: ArtifactBundle;
  sketchDocument?: SketchDocument | null;
}): Promise<SketchPreviewDraft> {
  const scopeId = resolveSketchPreviewDraftScopeId(input);
  return invokeCommand(
    commands.saveSketchPreviewDraft({
      scopeId,
      draftSource: input.draftSource,
      artifactBundle: input.artifactBundle,
      sketchDocument: input.sketchDocument ?? null,
    } satisfies SaveSketchPreviewDraftRequest),
  );
}

export async function loadSketchPreviewDraft(input: {
  scopeId?: string | null;
  draftScopeId?: string | null;
}): Promise<SketchPreviewDraft | null> {
  const scopeId = resolveSketchPreviewDraftScopeId(input);
  return invokeCommand(
    commands.loadSketchPreviewDraft({
      scopeId,
    } satisfies LoadSketchPreviewDraftRequest),
  );
}

export async function clearSketchPreviewDraft(input: {
  scopeId?: string | null;
  draftScopeId?: string | null;
}): Promise<void> {
  const scopeId = resolveSketchPreviewDraftScopeId(input);
  await invokeCommand(
    commands.clearSketchPreviewDraft({
      scopeId,
    } satisfies ClearSketchPreviewDraftRequest),
  );
}

export async function analyzeSketchBrepCandidates(
  request: SketchBrepCandidateRequest,
): Promise<SketchBrepCandidateResponse> {
  return invokeCommand(commands.analyzeSketchBrepCandidates(request));
}

export async function acceptSketchBrepCandidateSolution(
  request: SketchBrepCandidateAcceptRequest,
): Promise<Omit<SketchBrepCandidateAcceptResponse, 'artifactBundle'> & { artifactBundle: ArtifactBundle }> {
  const response = await invokeCommand(commands.acceptSketchBrepCandidateSolution(request));
  return {
    ...response,
    artifactBundle: normalizeArtifactBundle(response.artifactBundle),
  };
}

export async function acceptedBrepCandidateToComponentPackage(
  request: SketchAcceptedBrepComponentPackageRequest,
): Promise<ComponentPackage> {
  return invokeCommand(commands.acceptedBrepCandidateToComponentPackage(request));
}

export async function extractBrepHiddenLineProjections(
  request: BrepHiddenLineProjectionRequest,
): Promise<BrepHiddenLineProjectionResponse> {
  return invokeCommand(commands.extractBrepHiddenLineProjections(request));
}

export async function exportMultipartStlZip(
  parts: ExportPartInput[],
  targetPath: string,
  modelName: string,
): Promise<void> {
  await invokeCommand(commands.exportMultipartStlZip(parts, targetPath, modelName));
}

export async function exportMultipart3mf(
  parts: ExportPartInput[],
  targetPath: string,
  modelName: string,
): Promise<void> {
  await invokeCommand(commands.exportMultipart3mf(parts, targetPath, modelName));
}

export async function addManualVersion(input: {
  threadId: string;
  title: string;
  versionName: string;
  macroCode: string;
  sourceLanguage?: SourceLanguage | null;
  geometryBackend?: GeometryBackend | null;
  parameters: DesignParams;
  uiSpec: UiSpec;
  postProcessing?: PostProcessingSpec | null;
  artifactBundle?: ArtifactBundle | null;
  modelManifest?: ModelManifest | null;
  status?: MessageStatus | null;
  errorMessage?: string | null;
}): Promise<string> {
  return invokeCommand(
    commands.addManualVersion({
      threadId: input.threadId,
      title: input.title,
      versionName: input.versionName,
      macroCode: input.macroCode,
      sourceLanguage: input.sourceLanguage ?? null,
      geometryBackend: input.geometryBackend ?? null,
      parameters: input.parameters,
      uiSpec: toContractUiSpec(input.uiSpec),
      postProcessing: input.postProcessing ?? null,
      artifactBundle: input.artifactBundle ?? null,
      modelManifest: input.modelManifest ?? null,
      status: input.status ?? null,
      errorMessage: input.errorMessage ?? null,
    }),
  );
}

export async function addImportedModelVersion(input: {
  threadId: string;
  title: string;
  artifactBundle: ArtifactBundle;
  modelManifest: ModelManifest;
}): Promise<string> {
  return invokeCommand(
    commands.addImportedModelVersion(
      input.threadId,
      input.title,
      input.artifactBundle,
      input.modelManifest,
    ),
  );
}

export async function updateUiSpec(messageId: string, uiSpec: UiSpec): Promise<void> {
  await invokeCommand(commands.updateUiSpec(messageId, toContractUiSpec(uiSpec)));
}

export async function updateParameters(
  messageId: string,
  parameters: DesignParams,
): Promise<void> {
  await invokeCommand(commands.updateParameters(messageId, parameters));
}

export async function repairMissingVersionRuntime(
  messageId: string,
  artifactBundle: ArtifactBundle,
  modelManifest: ModelManifest,
): Promise<void> {
  await invokeCommand(commands.repairMissingVersionRuntime(messageId, artifactBundle, modelManifest));
}

export async function updateVersionPreview(
  messageId: string,
  imageData: string,
  artifactBundle: ArtifactBundle,
): Promise<void> {
  await invokeCommand(commands.updateVersionPreview(messageId, imageData, artifactBundle));
}

export async function parseMacroParams(macroCode: string): Promise<ParsedParamsResult> {
  return normalizeParsedParamsResult(await commands.parseMacroParams(macroCode));
}

export async function listCampaignDefinitions(): Promise<CampaignSummary[]> {
  return invokeCommand(commands.listCampaignDefinitions());
}

export async function getCampaignStep(
  definitionId: string,
  stepId: string,
): Promise<CampaignStepPayload> {
  return invokeCommand(commands.getCampaignStep(definitionId, stepId));
}

export async function checkCampaignStep(
  definitionId: string,
  stepId: string,
  candidateSource: string,
): Promise<MissionCoreIrEvaluation> {
  return invokeCommand(commands.checkCampaignStep(definitionId, stepId, candidateSource));
}

export async function createCampaignRun(input: CreateCampaignRunInput): Promise<CampaignRun> {
  return invokeCommand(commands.createCampaignRun(input));
}

export async function listCampaignRuns(): Promise<CampaignRun[]> {
  return invokeCommand(commands.listCampaignRuns());
}

export async function getCampaignRun(id: string): Promise<CampaignRun> {
  return invokeCommand(commands.getCampaignRun(id));
}

export async function saveCampaignRun(run: CampaignRun): Promise<CampaignRun> {
  return invokeCommand(commands.saveCampaignRun(run));
}

export async function deleteCampaignRun(id: string): Promise<void> {
  await invokeCommand(commands.deleteCampaignRun(id));
}

export async function getActiveProjectNavigation(): Promise<ActiveProjectNavigation | null> {
  return invokeCommand(commands.getActiveProjectNavigation());
}

export async function saveActiveProjectNavigation(
  navigation: ActiveProjectNavigation,
): Promise<ActiveProjectNavigation> {
  return invokeCommand(commands.saveActiveProjectNavigation(navigation));
}

export async function clearActiveProjectNavigation(): Promise<void> {
  await invokeCommand(commands.clearActiveProjectNavigation());
}

export async function getAppWindowLayout(): Promise<ThreadWindowLayout | null> {
  return invokeCommand(commands.getAppWindowLayout());
}

export async function saveAppWindowLayout(layout: ThreadWindowLayout): Promise<void> {
  await invokeCommand(commands.saveAppWindowLayout(layout));
}

export async function uploadAsset(input: {
  sourcePath: string;
  name: string;
  format: string;
}) {
  return invokeCommand(commands.uploadAsset(input.sourcePath, input.name, input.format));
}

export async function saveRecordedAudio(input: { base64Data: string; name: string }) {
  return invokeCommand(commands.saveRecordedAudio(input.base64Data, input.name));
}

export async function transcribePromptAudio(input: TranscribePromptAudioInput): Promise<PromptTranscription> {
  return invokeCommand(commands.transcribePromptAudio(input));
}

export async function getLastDesign(): Promise<LastDesignSnapshot | null> {
  return invokeCommand(commands.getLastDesign(), normalizeLastDesignSnapshot);
}

export async function getWebContentRecoveryState(): Promise<WebContentRecoveryState> {
  return invokeCommand(commands.getWebContentRecoveryState());
}

export async function acknowledgeWebContentRecovery(): Promise<void> {
  await invokeCommand(commands.acknowledgeWebContentRecovery());
}

export type HydratedAgentDraft = Omit<
  AgentDraftProjection,
  'designOutput' | 'artifactBundle' | 'modelManifest'
> & {
  designOutput: DesignOutput;
  artifactBundle: ArtifactBundle;
  modelManifest: ModelManifest;
};

export async function getAgentDraftPreview(
  threadId: string,
  previewId: string,
): Promise<HydratedAgentDraft> {
  const draft = await invokeCommand(commands.getAgentDraftPreview(threadId, previewId));
  const projected = structuredClone(draft);
  await hydrateBoundedTopology(
    draft.edgeCount,
    draft.faceCount,
    draft.selectionTargetCount,
    async (kind, cursor) => invokeCommand(
      commands.getAgentDraftTopologyPage(threadId, previewId, kind, cursor, 500),
    ),
    projected,
  );
  return {
    ...projected,
    designOutput: normalizeDesignOutput(projected.designOutput),
    artifactBundle: normalizeArtifactBundle(projected.artifactBundle),
    modelManifest: normalizeModelManifest(projected.modelManifest),
  };
}

export async function saveLastDesign(snapshot: LastDesignSnapshot | null): Promise<void> {
  await invokeCommand(commands.saveLastDesign(snapshot ? toContractLastDesignSnapshot(snapshot) : null));
}

export async function getActiveAgentSessions(): Promise<AgentSession[]> {
  return invokeCommand(commands.getActiveAgentSessions());
}

export async function getMcpServerStatus(): Promise<McpServerStatus> {
  return invokeCommand(commands.getMcpServerStatus());
}

export async function getAgentTerminalSnapshots(): Promise<AgentTerminalSnapshot[]> {
  return invokeCommand(commands.getAgentTerminalSnapshots());
}

export async function startCaptureSession(
  threadId: string,
  messageId: string | null,
  title: string,
  targetSource: string,
  targetSourceLanguage: string,
  startedFromEmpty: boolean,
): Promise<CaptureSessionInfo> {
  return invokeCommand(commands.startCaptureSession(
    threadId,
    messageId,
    title,
    targetSource,
    targetSourceLanguage,
    startedFromEmpty,
  ));
}

export async function listCaptureRuns(threadId: string): Promise<CaptureRun[]> {
  return invokeCommand(commands.listCaptureRuns(threadId));
}

export async function reopenCaptureRun(runId: string): Promise<ReopenedCaptureRun> {
  return invokeCommand(commands.reopenCaptureRun(runId));
}

export async function adoptLatestCaptureRun(
  threadId: string,
  messageId: string | null,
  title: string,
  targetSource: string,
  targetSourceLanguage: string,
  startedFromEmpty: boolean,
): Promise<ReopenedCaptureRun> {
  return invokeCommand(commands.adoptLatestCaptureRun(
    threadId,
    messageId,
    title,
    targetSource,
    targetSourceLanguage,
    startedFromEmpty,
  ));
}

export async function saveCapturePreviewSettings(
  runId: string,
  cropBounds: import('./contracts').CaptureCropBounds | null,
  previewScale: number,
): Promise<void> {
  await invokeCommand(commands.saveCapturePreviewSettings(runId, cropBounds, previewScale));
}

export async function getCaptureReconstructionGuide(
  runId: string,
): Promise<CaptureReconstructionGuide | null> {
  return invokeCommand(commands.getCaptureReconstructionGuide(runId));
}

export async function getCaptureGuideSourceIdentity(
  runId: string,
): Promise<CaptureGuideSourceMesh> {
  return invokeCommand(commands.getCaptureGuideSourceIdentity(runId));
}

export async function getCaptureGuideContext(
  runId: string,
): Promise<CaptureGuideContext> {
  return invokeCommand(commands.getCaptureGuideContext(runId));
}

export async function saveCaptureReconstructionGuide(
  runId: string,
  expectedRevision: number,
  expectedMeshDigest: string,
  guide: CaptureReconstructionGuide,
  guideState: CaptureReconstructionGuideState,
): Promise<CaptureReconstructionGuide> {
  return invokeCommand(commands.saveCaptureReconstructionGuide(
    runId,
    expectedRevision,
    expectedMeshDigest,
    guide,
    guideState,
  ));
}

export async function evaluateCaptureReconstructionGuide(
  runId: string,
  expectedMeshDigest: string,
  guide: CaptureReconstructionGuide,
): Promise<CaptureReconstructionGuide> {
  return invokeCommand(commands.evaluateCaptureReconstructionGuide(
    runId,
    expectedMeshDigest,
    guide,
  ));
}

export async function queueCaptureGuidedReconstruction(
  runId: string,
  expectedGuideRevision: number,
  expectedTargetSourceDigest: string,
): Promise<QueuedCaptureGuidedReconstruction> {
  return invokeCommand(commands.queueCaptureGuidedReconstruction(
    runId,
    expectedGuideRevision,
    expectedTargetSourceDigest,
  ));
}

export async function showSafeSaveDialog(
  defaultPath: string,
  filterName: string,
  extensions: string[],
): Promise<string | null> {
  return invokeCommand(commands.safeSaveDialog(defaultPath, filterName, extensions));
}

export async function getCaptureSessionStatus(token: string): Promise<CaptureSessionInfo | null> {
  return invokeCommand(commands.getCaptureSessionStatus(token));
}

export async function pairCaptureSession(token: string): Promise<CaptureSessionInfo> {
  return invokeCommand(commands.pairCaptureSession(token));
}

export async function cancelCaptureSession(token: string): Promise<CaptureSessionInfo> {
  return invokeCommand(commands.cancelCaptureSession(token));
}

export async function resumeCaptureSession(token: string): Promise<CaptureSessionInfo> {
  return invokeCommand(commands.resumeCaptureSession(token));
}

export async function retryCaptureReconstruction(token: string): Promise<CaptureSessionInfo> {
  return invokeCommand(commands.retryCaptureReconstruction(token));
}

export async function prepareCapturePreview(
  token: string,
  cropBounds: import('./contracts').CaptureCropBounds | null,
): Promise<{
  artifactBundle: ArtifactBundle;
  modelManifest: ModelManifest;
}> {
  const prepared = await invokeCommand(commands.prepareCapturePreview(token, cropBounds));
  return {
    artifactBundle: normalizeArtifactBundle(prepared.artifactBundle),
    modelManifest: normalizeModelManifest(prepared.modelManifest),
  };
}

export async function validateFemStudy(
  request: FemStudyRequest,
): Promise<FemStudyValidationResponse> {
  return invokeCommand(commands.validateFemStudy(request));
}

export async function runFemStudy(request: FemStudyRequest): Promise<FemRunResponse> {
  return invokeCommand(commands.runFemStudy(request));
}

export async function previewFemMesh(request: FemStudyRequest): Promise<FemMeshPreviewResponse> {
  return invokeCommand(commands.previewFemMesh(request));
}

export async function runFemConvergence(
  request: FemConvergenceRequest,
): Promise<FemConvergenceResponse> {
  return invokeCommand(commands.runFemConvergence(request));
}

export async function getCachedFemConvergence(
  request: FemConvergenceRequest,
): Promise<FemConvergenceResponse | null> {
  return invokeCommand(commands.getCachedFemConvergence(request));
}

export async function cancelFemStudy(jobId: string): Promise<FemCancelResponse> {
  return invokeCommand(commands.cancelFemStudy(jobId));
}

export async function readFemResult(
  request: FemResultReadRequest,
): Promise<FemResultReadResponse> {
  return invokeCommand(commands.readFemResult(request));
}

export async function exportFemResultVtu(
  request: FemResultReadRequest,
  targetPath: string,
): Promise<FemVtuExportResponse> {
  return invokeCommand(commands.exportFemResultVtu(request, targetPath));
}

export async function sendAgentTerminalInput(input: AgentTerminalInput): Promise<void> {
  await invokeCommand(
    commands.sendAgentTerminalInput({
      agentId: input.agentId,
      text: input.text ?? '',
      key: input.key ?? null,
      ctrl: input.ctrl ?? false,
      alt: input.alt ?? false,
      shift: input.shift ?? false,
      meta: input.meta ?? false,
      submit: input.submit ?? false,
    }),
  );
}

export async function resizeAgentTerminal(
  agentId: string,
  cols: number,
  rows: number,
): Promise<void> {
  await invokeCommand(commands.resizeAgentTerminal(agentId, cols, rows));
}

export async function resolveAgentConfirm(requestId: string, choice: string) {
  await invokeCommand(commands.resolveAgentConfirm(requestId, choice));
}

export async function preparePromptAttachments(
  attachments: Attachment[],
): Promise<Attachment[]> {
  if (attachments.length === 0) {
    return [];
  }
  return invokeCommand(
    commands.preparePromptAttachments(attachments.map(toContractAttachment)),
    (value) => value.map(normalizeAttachment),
  );
}

export async function preparePromptWorkspaceCapture(input: {
  dataUrl: string;
  threadId?: string | null;
  name?: string | null;
  explanation?: string | null;
}): Promise<Attachment> {
  return invokeCommand(
    commands.preparePromptWorkspaceCapture({
      dataUrl: input.dataUrl,
      threadId: input.threadId ?? null,
      name: input.name ?? null,
      explanation: input.explanation ?? null,
    }),
    normalizeAttachment,
  );
}

export async function getMessageAttachments(messageId: string): Promise<Attachment[]> {
  return invokeCommand(commands.getMessageAttachments(messageId), (value) =>
    value.map(normalizeAttachment),
  );
}

export async function resolveAgentPrompt(input: {
  requestId: string;
  promptText: string;
  messageIds?: string[];
  messageId?: string | null;
  attachments: Attachment[];
}) {
  await invokeCommand(
    commands.resolveAgentPrompt({
      requestId: input.requestId,
      promptText: input.promptText,
      messageIds: input.messageIds ?? [],
      messageId: input.messageId ?? null,
      attachments: input.attachments.map(toContractAttachment),
    } as ResolveAgentPromptInput),
  );
}

export async function queueAgentPrompt(input: {
  threadId?: string | null;
  promptText: string;
  attachments: Attachment[];
}): Promise<{ threadId: string; messageId: string }> {
  return invokeCommand(
    commands.queueAgentPrompt({
      threadId: input.threadId ?? null,
      promptText: input.promptText,
      attachments: input.attachments.map(toContractAttachment),
    } as QueueAgentPromptInput),
  );
}

export async function resolveAgentViewportScreenshot(input: {
  requestId: string;
  dataUrl: string;
  width: number;
  height: number;
  camera: ViewportCameraState;
  source: string;
  threadId: string;
  messageId: string;
  modelId?: string | null;
  includeOverlays: boolean;
}) {
  await invokeCommand(commands.resolveAgentViewportScreenshot(input as ResolveViewportScreenshotInput));
}

export async function rejectAgentViewportScreenshot(requestId: string, error: string) {
  await invokeCommand(
    commands.rejectAgentViewportScreenshot({
      requestId,
      error,
    } as RejectViewportScreenshotInput),
  );
}

export async function getAgentActivity(afterCursor: number | null): Promise<AgentActivityCatchUp> {
  return invokeCommand(commands.getAgentActivity(afterCursor));
}

export async function getAppLogs(): Promise<AppLogEntry[]> {
  return invokeCommand(commands.getAppLogs());
}

export async function verifyRender(
  originalPrompt: string,
  screenshots: string[],
  referenceImagePaths: string[] = [],
  structuralSummary: string | null = null,
): Promise<VisualVerificationResult> {
  return invokeCommand(commands.verifyRender(originalPrompt, screenshots, referenceImagePaths, structuralSummary));
}

export async function verifyGeneratedModel(
  modelId: string,
  originalPrompt: string,
): Promise<StructuralVerificationResult> {
  return invokeCommand(commands.verifyGeneratedModel(modelId, originalPrompt));
}

export async function getThreadWindowLayout(threadId: string): Promise<ThreadWindowLayout | null> {
  return invokeCommand(commands.getThreadWindowLayout(threadId));
}

export async function saveThreadWindowLayout(threadId: string, layout: ThreadWindowLayout): Promise<void> {
  await invokeCommand(commands.saveThreadWindowLayout(threadId, layout));
}

export type { AppLogEntry };
export type { VisualVerificationResult };
export type { StructuralVerificationResult };
