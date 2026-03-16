<script lang="ts">
  import PromptPanel from './lib/PromptPanel.svelte';
  import Viewer from './lib/Viewer.svelte';
  import VertexGenie from './lib/VertexGenie.svelte';
  import DrawingOverlay from './lib/DrawingOverlay.svelte';
  import ParamPanel from './lib/ParamPanel.svelte';
  import ConfigPanel from './lib/ConfigPanel.svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { writeTextFile } from '@tauri-apps/plugin-fs';
  import { onMount, tick } from 'svelte';
  import { get } from 'svelte/store';

  import HistoryPanel from './lib/HistoryPanel.svelte';
  import CodeModal from './lib/CodeModal.svelte';
  import DeletedModels from './lib/DeletedModels.svelte';
  import InventoryPanel from './lib/InventoryPanel.svelte';
  import ImportEnrichmentModal from './lib/ImportEnrichmentModal.svelte';
  import AgentTerminalSurface from './lib/AgentTerminalSurface.svelte';
  import Window from './lib/Window.svelte';
  import {
    activeMicrowaveCount,
    setMuted,
    setAudibleThread,
    startMicrowaveHum,
    stopMicrowaveAudio,
    stopMicrowaveHum,
  } from './lib/audio/microwave';
  import { onboarding } from './lib/stores/onboarding';
  import { session } from './lib/stores/sessionStore';
  import { startCookingPhraseLoop, stopPhraseLoop } from './lib/stores/phraseEngine';
  import { handleGenerate, initOrchestrator, isQuestionIntent } from './lib/controllers/requestOrchestrator';
  import { handleParamChange, commitManualVersion, forkManualVersion, stageParamChange } from './lib/controllers/manualController';
  import { loadFromHistory, deleteThread, renameThread, createNewThread, forkDesign, deleteVersion, loadVersion, refreshHistory, finalizeThread } from './lib/stores/history';
  import { workingCopy, isDirty } from './lib/stores/workingCopy';
  import { history, activeThreadId, activeVersionId, config, availableModels, isLoadingModels, freecadAvailable } from './lib/stores/domainState';
  import { sidebarWidth, historyHeight, dialogueHeight, showCodeModal, selectedCode, selectedTitle, currentView } from './lib/stores/viewState';
  import { boot, saveConfig, fetchModels } from './lib/boot/restore';
  import { requestQueue, allRequests, activeRequests, activeRequestCount, currentActiveRequest, activeThreadBusy, activeThreadRequests } from './lib/stores/requestQueue';
  import { nowSeconds } from './lib/stores/timeEngine';
  import { liveApply, paramPanelState } from './lib/stores/paramPanelState';
  import { persistLastSessionSnapshot } from './lib/modelRuntime/sessionSnapshot';
  import { getRenderableRuntimeBundle } from './lib/modelRuntime/runtimeBundle';
  import {
    deriveMascotStateForThreadAgent,
    derivePrimaryAgentId,
    promptBelongsToPrimaryAgent,
    resolveQueuedPromptDelivery,
    usesActiveMcpMode,
  } from './lib/agents/state';
  import {
    agentTerminalSessionKey,
    buildAgentTerminalKeyInput,
    buildAgentTerminalLineInput,
    hasRenderableTerminal,
    mergeAgentTerminalSnapshot,
    pickAgentTerminalAttention,
    pickVisibleAgentTerminal,
  } from './lib/agents/terminal';
  import {
    isThreadAgentBusy,
    resolveActiveMcpBubble,
    resolveTerminalActivityMeta,
  } from './lib/agents/activity';
  import {
    chooseViewportCaptureMode,
    rememberTargetCameraState,
    rememberTargetScreenshot,
    resolveFallbackScreenshotSource,
    viewportTargetKey,
    type ViewportScreenshotCapture,
  } from './lib/agents/screenshot';
  import {
    buildImportedParams,
    buildImportedPreviewTransforms,
    buildImportedUiSpec,
    type ImportedPreviewTransform,
  } from './lib/modelRuntime/importedRuntime';
  import {
    buildSemanticPatch,
    ensureSemanticManifest,
    materializeControlViews,
  } from './lib/modelRuntime/semanticControls';
  import {
    buildContextSelectionTargets,
    createGlobalContextTarget,
    deriveSelectedPartId,
    pickContextAdvisories,
    pickContextControls,
    resolveMeasurementCallout,
    resolveActiveContextViewId,
    resolveContextSelectionTarget,
    type MeasurementControlFocus,
    type ContextSelectionTarget,
  } from './lib/modelRuntime/contextualEditing';
  import {
    buildGenerateFromConceptPrompt,
    cycleConceptPreviewMessageId,
    listConceptPreviewMessages,
    reconcileConceptPreviewUiState,
    resolveEffectiveConceptPreviewMessage,
    type ConceptPreviewUiState,
    type ViewportPresentationMode,
  } from './lib/viewportBlueprint';
  import {
    addImportedModelVersion,
    exportFile,
    formatBackendError,
    getActiveAgentSessions,
    getAgentSessionTrace,
    getAgentTerminalSnapshots,
    getThread,
    getThreadAgentState,
    getModelManifest,
    importFcstd,
    rejectAgentViewportScreenshot,
    renderModel,
    restartPrimaryAutoAgent,
    resizeAgentTerminal,
    resolveAgentConfirm,
    resolveAgentPrompt,
    resolveAgentViewportScreenshot,
    saveConfig as persistBackendConfig,
    sendAgentTerminalInput,
    stopPrimaryAutoAgent,
    wakePrimaryAutoAgent,
    saveModelManifest,
    type PostProcessingSpec,
    type ThreadAgentState,
  } from './lib/tauri/client';
  import { listen } from '@tauri-apps/api/event';
  import type {
    AgentSession,
    AgentSessionTraceEntry,
    AgentTerminalInput,
    AgentTerminalSnapshot,
    Attachment,
    DesignParams,
    GenieTraits,
    Message,
    ParamValue,
    Request,
    UiField,
    UiSpec,
    ViewerAsset,
    ViewportCameraState,
  } from './lib/types/domain';
  import type { MaterializedSemanticView } from './lib/modelRuntime/semanticControls';

  type ViewerHandle = {
    captureScreenshot: (overlayCanvas?: HTMLCanvasElement | null) => string | null;
    captureScreenshotDetails: (overlayCanvas?: HTMLCanvasElement | null) => {
      dataUrl: string;
      width: number;
      height: number;
      camera: ViewportCameraState;
    } | null;
    getCameraState: () => ViewportCameraState | null;
    setCameraState: (camera: ViewportCameraState | null) => void;
  };

  type DrawingOverlayHandle = {
    hasDrawing: () => boolean;
    getCanvas: () => HTMLCanvasElement | null;
    clear: () => void;
  };

  type ThreadPhase = Request['phase'] | 'idle' | 'booting';
  type ViewerBusyPhase = 'generating' | 'repairing' | 'rendering' | 'committing' | null;
  type ViewerCaptureDetails = {
    dataUrl: string;
    width: number;
    height: number;
    camera: ViewportCameraState;
  };
  type AgentViewportScreenshotEvent = {
    requestId: string;
    threadId: string;
    messageId: string;
    modelId?: string | null;
    previewStlPath: string;
    viewerAssets: ViewerAsset[];
    includeOverlays: boolean;
    camera?: ViewportCameraState | null;
  };
  type PendingViewportScreenshotChoice = AgentViewportScreenshotEvent & {
    message: string;
    buttons: string[];
  };
  type HiddenViewerSpec = {
    requestId: string;
    targetKey: string;
    stlUrl: string;
    viewerAssets: ViewerAsset[];
  };

  function defaultConceptPreviewUiState(): ConceptPreviewUiState {
    return {
      pinnedMessageId: null,
      lastAutoPinnedMessageId: null,
      mode: 'model',
    };
  }

  function sameConceptPreviewUiState(
    left: ConceptPreviewUiState,
    right: ConceptPreviewUiState,
  ): boolean {
    return (
      left.pinnedMessageId === right.pinnedMessageId &&
      left.lastAutoPinnedMessageId === right.lastAutoPinnedMessageId &&
      left.mode === right.mode
    );
  }

  function formatAgentPhase(phase: string): string {
    return phase.replace(/_/g, ' ').toUpperCase();
  }

  function shouldSuppressOnboardingForAutomation(): boolean {
    if (typeof navigator === 'undefined') return false;
    return Boolean(navigator.webdriver);
  }

  function mapAgentPhaseToViewerBusy(session: AgentSession | null): ViewerBusyPhase {
    switch (session?.phase) {
      case 'rendering':
      case 'restoring_version':
        return 'rendering';
      case 'saving_version':
        return 'committing';
      case 'patching_params':
      case 'patching_macro':
      case 'reading':
      case 'resolving':
        return 'generating';
      default:
        return null;
    }
  }

  function formatAgentOriginLabel(origin: Message['agentOrigin'] | null | undefined): string | null {
    if (!origin) return null;
    const host = origin.hostLabel?.trim() || origin.agentLabel?.trim() || 'Agent';
    const model = origin.llmModelLabel?.trim() || origin.llmModelId?.trim() || '';
    if (!model || model.toLowerCase() === host.toLowerCase()) {
      return host;
    }
    return `${host} · ${model}`;
  }

  function isActiveRequestPhase(phase: Request['phase']): boolean {
    return !['success', 'error', 'canceled'].includes(phase);
  }

  function isModelBusyRequestPhase(phase: Request['phase']): boolean {
    return ['generating', 'repairing', 'queued_for_render', 'rendering', 'committing'].includes(phase);
  }

  function requestMatchesViewerTarget(
    request: Request,
    threadId: string | null,
    messageId: string | null,
    modelId: string | null,
  ): boolean {
    if (!threadId || request.threadId !== threadId) return false;
    if (modelId) {
      if (request.baseModelId) return request.baseModelId === modelId;
      if (messageId && request.baseMessageId) return request.baseMessageId === messageId;
      return false;
    }
    if (messageId) {
      if (request.baseMessageId) return request.baseMessageId === messageId;
      return false;
    }
    return true;
  }

  function sessionMatchesViewerTarget(
    candidate: AgentSession,
    threadId: string | null,
    messageId: string | null,
    modelId: string | null,
  ): boolean {
    if (!threadId || candidate.threadId !== threadId) return false;
    if (modelId) {
      if (candidate.modelId) return candidate.modelId === modelId;
      if (messageId && candidate.messageId) return candidate.messageId === messageId;
      return false;
    }
    if (messageId) {
      if (candidate.messageId) return candidate.messageId === messageId;
      return false;
    }
    return true;
  }

  // Local reactive aliases for templates
  const phase = $derived($session.phase);
  const status = $derived($session.status);
  const error = $derived($session.error);
  const stlUrl = $derived($session.stlUrl);
  const activeArtifactBundle = $derived($session.artifactBundle);
  const sessionModelManifest = $derived($session.modelManifest);
  let selectedContextTargetId = $state<string | null>(null);
  let sharedContextSearchQuery = $state('');
  let focusedMeasurementControl = $state<MeasurementControlFocus | null>(null);
  let lastViewportContextKey = $state<string | null>(null);

  const isBooting = $derived(phase === 'booting');
  const isQuestionFlow = $derived(phase === 'answering');
  const freecadMissing = $derived(!isBooting && $freecadAvailable === false);
  const isActiveMcpMode = $derived(usesActiveMcpMode($config.connectionType, $config.mcp.mode));

  type DialogueState =
    | { mode: 'generate' }
    | { mode: 'mcp-idle' }
    | { mode: 'agent-reply'; requestId: string; agentLabel: string };

  const dialogueState = $derived.by<DialogueState>(() => {
    if (activePendingAgentPrompt) {
      return {
        mode: 'agent-reply',
        requestId: activePendingAgentPrompt.requestId,
        agentLabel: activePendingAgentPrompt.agentLabel,
      };
    }
    if (isActiveMcpMode) return { mode: 'mcp-idle' };
    return { mode: 'generate' };
  });
  const viewerAssets = $derived.by<ViewerAsset[]>(() => {
    const assets = activeArtifactBundle?.viewerAssets || [];
    return assets.map((asset) => ({
      ...asset,
      path: toAssetUrl(asset.path),
    }));
  });
  const hasRenderableModel = $derived.by(() =>
    Boolean($activeThreadId && ((stlUrl || '').trim() || viewerAssets.length > 0)),
  );
  const effectiveUiSpec = $derived.by<UiSpec>(() => {
    if (($paramPanelState.uiSpec?.fields || []).length > 0) {
      return $paramPanelState.uiSpec;
    }
    return buildImportedUiSpec(sessionModelManifest);
  });
  const effectiveParameters = $derived.by<DesignParams>(() =>
    buildImportedParams(sessionModelManifest, $paramPanelState.params || {}, effectiveUiSpec),
  );
  const activeModelManifest = $derived.by(() =>
    ensureSemanticManifest(sessionModelManifest, effectiveUiSpec, effectiveParameters),
  );
  const contextSelectionTargets = $derived.by<ContextSelectionTarget[]>(() =>
    buildContextSelectionTargets(activeModelManifest),
  );
  const selectedTarget = $derived.by<ContextSelectionTarget | null>(() =>
    resolveContextSelectionTarget(
      activeModelManifest,
      contextSelectionTargets,
      selectedContextTargetId,
      $session.selectedPartId,
    ),
  );
  const selectedPartId = $derived(deriveSelectedPartId(selectedTarget));
  const importedPreviewTransforms = $derived.by<Record<string, ImportedPreviewTransform>>(() =>
    buildImportedPreviewTransforms(activeModelManifest, effectiveParameters),
  );
  const overlayPreviewOnly = $derived.by(() => {
    if (!(activeModelManifest?.sourceKind === 'importedFcstd' && overlaySelectedPart?.editable)) {
      return false;
    }
    if (!selectedPartId) return false;
    const preview = importedPreviewTransforms[selectedPartId];
    if (!preview) return false;
    return (
      Math.abs(preview.scale.x - 1) > 0.001 ||
      Math.abs(preview.scale.y - 1) > 0.001 ||
      Math.abs(preview.scale.z - 1) > 0.001
    );
  });
  const overlaySelectedPart = $derived.by(() => {
    if (!selectedPartId || !activeModelManifest?.parts?.length) return null;
    return activeModelManifest.parts.find((part) => part.partId === selectedPartId) ?? null;
  });
  const availableControlViews = $derived.by<MaterializedSemanticView[]>(() =>
    materializeControlViews(activeModelManifest, effectiveUiSpec, effectiveParameters),
  );
  let activeControlViewId = $state<string | null>(null);
  $effect(() => {
    activeControlViewId = resolveActiveContextViewId(
      availableControlViews,
      selectedTarget,
      activeControlViewId,
    );
  });
  const activeControlView = $derived.by(() =>
    availableControlViews.find((view) => view.viewId === activeControlViewId) ??
    availableControlViews[0] ??
    null,
  );
  const overlayControls = $derived.by(() => {
    if (!activeControlView) return [];
    return pickContextControls(activeControlView, selectedTarget);
  });
  const overlayAdvisories = $derived.by(() =>
    pickContextAdvisories(activeControlView, selectedTarget),
  );
  const activeMeasurementCallout = $derived.by(() =>
    resolveMeasurementCallout(
      activeModelManifest,
      activeArtifactBundle,
      contextSelectionTargets,
      focusedMeasurementControl,
      selectedTarget,
    ),
  );
  const suppressViewportBusyUi = $derived($showCodeModal);
  let showEnrichmentModal = $state(false);
  const enrichmentManifest = $derived.by(() => {
    if (!showEnrichmentModal) return null;
    const m = sessionModelManifest;
    if (!m || m.sourceKind !== 'importedFcstd') return null;
    if (m.enrichmentState?.status !== 'pending') return null;
    return m;
  });
  const localViewportRequests = $derived.by<Request[]>(() => {
    const threadId = $activeThreadId;
    const messageId = $activeVersionId;
    const modelId = activeArtifactBundle?.modelId ?? null;
    return $activeThreadRequests.filter(
      (request) =>
        isActiveRequestPhase(request.phase) &&
        requestMatchesViewerTarget(request, threadId, messageId, modelId),
    );
  });
  const externalViewerSession = $derived.by<AgentSession | null>(() => {
    const threadId = $activeThreadId;
    const messageId = $activeVersionId;
    const modelId = activeArtifactBundle?.modelId ?? null;
    if (!threadId && !messageId && !modelId) return null;

    return (
      activeAgentSessions.find((candidate) =>
        sessionMatchesViewerTarget(candidate, threadId, messageId, modelId),
      ) ??
      null
    );
  });
  const externalViewerBusyPhase = $derived.by<ViewerBusyPhase>(() =>
    mapAgentPhaseToViewerBusy(externalViewerSession),
  );
  const manualViewerBusyPhase = $derived.by<ViewerBusyPhase>(() => {
    if (
      $session.isManual &&
      phase === 'rendering' &&
      $session.manualThreadId === $activeThreadId &&
      ($session.manualMessageId ?? null) === ($activeVersionId ?? null)
    ) {
      return 'rendering';
    }
    return null;
  });
  const showViewerBusyMask = $derived.by(() => {
    if (suppressViewportBusyUi) return false;
    if (localViewportRequests.some((request) => isModelBusyRequestPhase(request.phase))) return true;
    if (manualViewerBusyPhase === 'rendering') return true;
    return externalViewerBusyPhase === 'rendering' || externalViewerBusyPhase === 'committing';
  });
  const localViewerBusyPhase = $derived.by<ViewerBusyPhase>(() => {
    if (localViewportRequests.some((request) => request.phase === 'committing')) return 'committing';
    if (
      localViewportRequests.some((request) =>
        ['queued_for_render', 'rendering'].includes(request.phase),
      )
    ) {
      return 'rendering';
    }
    if (localViewportRequests.some((request) => request.phase === 'repairing')) return 'repairing';
    if (localViewportRequests.some((request) => request.phase === 'generating')) return 'generating';
    if (manualViewerBusyPhase === 'rendering') return 'rendering';
    return null;
  });
  const viewerBusyPhase = $derived.by<ViewerBusyPhase>(() => {
    return localViewerBusyPhase ?? externalViewerBusyPhase;
  });
  const viewerBusyText = $derived.by<string | null>(() => {
    switch (localViewerBusyPhase) {
      case 'repairing':
        return $session.repairMessage || 'Reweaving the geometry lattice.';
      case 'rendering':
        return 'Stabilizing the geometry into manufacturable solids.';
      case 'committing':
        return 'Finalizing the artifact and sealing it into the thread.';
      case 'generating':
        return $session.cookingPhrase || 'Preparing the next transformation.';
      default:
        if (!externalViewerSession || !externalViewerBusyPhase) return null;
        if (externalViewerSession.statusText.trim()) return externalViewerSession.statusText;
        switch (externalViewerBusyPhase) {
          case 'rendering':
            return `External agent ${externalViewerSession.agentLabel} is updating the model.`;
          case 'committing':
            return `External agent ${externalViewerSession.agentLabel} is saving a version.`;
          case 'generating':
            return `External agent ${externalViewerSession.agentLabel} is preparing an update.`;
          default:
            return null;
        }
    }
  });
  const externalViewerStatusText = $derived.by<string | null>(() => {
    if (!externalViewerSession || !externalViewerBusyPhase) return null;
    if (externalViewerSession.statusText.trim()) return externalViewerSession.statusText;
    switch (externalViewerBusyPhase) {
      case 'rendering':
        return `External agent ${externalViewerSession.agentLabel} is updating the model.`;
      case 'committing':
        return `External agent ${externalViewerSession.agentLabel} is saving a version.`;
      case 'generating':
        return `External agent ${externalViewerSession.agentLabel} is preparing an update.`;
      default:
        return null;
    }
  });

  let viewerComponent = $state<ViewerHandle | null>(null);
  let hiddenViewerComponent = $state<ViewerHandle | null>(null);
  let drawingOverlay = $state<DrawingOverlayHandle | null>(null);
  let viewportAreaEl = $state<HTMLElement | null>(null);
  let blueprintImageEl = $state<HTMLImageElement | null>(null);
  let hiddenViewerSpec = $state<HiddenViewerSpec | null>(null);
  let visibleViewerLoadNonce = $state(0);
  let hiddenViewerLoadNonce = $state(0);
  let cameraStateByTarget = $state<Record<string, ViewportCameraState>>({});
  let lastLiveScreenshotByTarget = $state<Record<string, ViewportScreenshotCapture>>({});
  let conceptPreviewUiByThread = $state<Record<string, ConceptPreviewUiState>>({});
  let drawMode = $state(false);
  let lastAssistantMessageId = $state<string | null>(null);
  let lastAdvisorBubble = $state('');
  let lastAdvisorQuestion = $state('');
  let dismissedBubbleText = $state('');
  let agentControlBusy = $state(false);

  let activeAgentSessions = $state<AgentSession[]>([]);
  let agentSessionTraces = $state<Record<string, AgentSessionTraceEntry[]>>({});
  let agentTerminalSnapshots = $state<Record<string, AgentTerminalSnapshot>>({});
  let threadAgentState = $state<ThreadAgentState | null>(null);
  let genieWakeUpCount = $state(0);
  let lastAgentConnectionState = 'none';
  let threadAgentPollInterval: ReturnType<typeof setInterval> | null = null;
  let showAgentTerminalWindow = $state(false);
  let agentTerminalInput = $state('');
  let agentTerminalWindowX = $state(320);
  let agentTerminalWindowY = $state(72);
  let agentTerminalWindowWidth = $state(760);
  let agentTerminalWindowHeight = $state(460);
  let agentTerminalSurface = $state<{ focusTerminal: () => void } | null>(null);
  let lastAgentTerminalFocusKey = $state('');
  let activeMcpMicrowaveKey = $state('');
  let ownsMcpPhraseLoop = $state(false);

  type PendingAgentPrompt = { requestId: string; message: string | null; agentLabel: string; sessionId: string };
  let pendingAgentPrompt = $state<PendingAgentPrompt | null>(null);
  let pendingViewportScreenshotChoices = $state<PendingViewportScreenshotChoice[]>([]);
  const activePendingAgentPrompt = $derived.by<PendingAgentPrompt | null>(() => {
    if (!pendingAgentPrompt) return null;
    if ($config.connectionType !== 'mcp' || $config.mcp.mode !== 'active') {
      return pendingAgentPrompt;
    }
    return promptBelongsToPrimaryAgent(
      $config.mcp.autoAgents ?? [],
      $config.mcp.primaryAgentId ?? null,
      pendingAgentPrompt.agentLabel,
    )
      ? pendingAgentPrompt
      : null;
  });

  type QueuedMessage = { id: string; text: string; attachments?: Attachment[]; status: 'queued' | 'delivered' };
  let queuedUserMessages = $state<QueuedMessage[]>([]);

  // Auto-deliver the first queued message when agent calls request_user_prompt
  $effect(() => {
    if (!activePendingAgentPrompt) return;
    const nextDelivery = resolveQueuedPromptDelivery(queuedUserMessages);
    if (!nextDelivery.delivery) return;
    const requestId = activePendingAgentPrompt.requestId;
    queuedUserMessages = nextDelivery.queuedMessages;
    if (pendingAgentPrompt?.requestId === requestId) {
      pendingAgentPrompt = null;
    }
    void resolveAgentPrompt({
      requestId,
      promptText: nextDelivery.delivery.text,
      attachments: nextDelivery.delivery.attachments ?? [],
    }).catch(() => {});
  });

  $effect(() => {
    const nextConnectionState = threadAgentState?.connectionState ?? 'none';
    if (
      ['waking', 'waiting', 'active'].includes(nextConnectionState) &&
      !['waking', 'waiting', 'active'].includes(lastAgentConnectionState)
    ) {
      genieWakeUpCount++;
    }
    lastAgentConnectionState = nextConnectionState;
  });

  function upsertAgentTerminalSnapshot(snapshot: AgentTerminalSnapshot) {
    if (!hasRenderableTerminal(snapshot)) {
      const { [snapshot.agentId]: _discarded, ...rest } = agentTerminalSnapshots;
      agentTerminalSnapshots = rest;
      return;
    }
    const previous = agentTerminalSnapshots[snapshot.agentId] ?? null;
    agentTerminalSnapshots = {
      ...agentTerminalSnapshots,
      [snapshot.agentId]: mergeAgentTerminalSnapshot(previous, snapshot),
    };
  }

  function replaceAgentSessionTrace(sessionId: string, entries: AgentSessionTraceEntry[]) {
    agentSessionTraces = {
      ...agentSessionTraces,
      [sessionId]: entries.slice(-64),
    };
  }

  function upsertAgentSessionTraceEntry(entry: AgentSessionTraceEntry) {
    const existing = agentSessionTraces[entry.sessionId] ?? [];
    const duplicate = existing.some(
      (candidate) =>
        candidate.timestamp === entry.timestamp &&
        candidate.kind === entry.kind &&
        candidate.summary === entry.summary,
    );
    if (duplicate) return;
    replaceAgentSessionTrace(entry.sessionId, [...existing, entry]);
  }

  function shortSessionId(sessionId: string | null | undefined): string {
    if (!sessionId) return 'NO SESSION';
    return sessionId.slice(0, 8);
  }

  async function sendAgentTerminalPayload(
    payload: AgentTerminalInput | null,
    options?: { clearComposer?: boolean; refocusTerminal?: boolean },
  ) {
    if (!payload) return;
    try {
      await sendAgentTerminalInput(payload);
      if (options?.clearComposer) {
        agentTerminalInput = '';
      }
      if (options?.refocusTerminal) {
        await tick();
        agentTerminalSurface?.focusTerminal();
      }
    } catch (error) {
      session.setError(`Agent Terminal Error: ${formatBackendError(error)}`);
    }
  }

  async function submitAgentTerminalInput(forceEnter = false) {
    if (!visibleAgentTerminal) return;
    const payload = forceEnter
      ? buildAgentTerminalKeyInput(visibleAgentTerminal.agentId, {
          key: 'Enter',
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
          metaKey: false,
        })
      : buildAgentTerminalLineInput(visibleAgentTerminal.agentId, agentTerminalInput, true);
    await sendAgentTerminalPayload(payload, {
      clearComposer: !forceEnter,
      refocusTerminal: true,
    });
  }

  async function handleAgentTerminalRawInput(data: string) {
    if (!visibleAgentTerminal?.active || !data.length) return;
    await sendAgentTerminalPayload(
      {
        agentId: visibleAgentTerminal.agentId,
        text: data,
        key: null,
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
        submit: false,
      },
      { refocusTerminal: false },
    );
  }

  async function handleAgentTerminalResize(agentId: string, cols: number, rows: number) {
    try {
      await resizeAgentTerminal(agentId, cols, rows);
    } catch (error) {
      session.setError(`Agent Terminal Error: ${formatBackendError(error)}`);
    }
  }

  async function handleNudgeAgentPromptRearm() {
    if (!visibleAgentTerminal?.active) return;
    await sendAgentTerminalPayload(
      buildAgentTerminalLineInput(
        visibleAgentTerminal.agentId,
        'Call `request_user_prompt` now so Ecky can queue the next user message.',
        true,
      ),
      { refocusTerminal: true },
    );
  }

  // Wake animation fires when the selected active agent asks for a prompt.
  // No startup pre-waking — genie should be idle until the primary agent actually greets.

  let isResizingWidth = $state(false);
  let isResizingHeight = $state(false);
  let isResizingHistory = $state(false);

  // Initialize async design orchestrator
  initOrchestrator({
    get viewerComponent() { return viewerComponent; },
    openCodeModalManual: (data) => {
      selectedCode.set($workingCopy.macroCode);
      selectedTitle.set($workingCopy.title || data.title);
      showCodeModal.set(true);
    },
    getDrawingCanvas: () => drawingOverlay?.hasDrawing() ? drawingOverlay.getCanvas() : null,
    clearDrawing: () => { drawingOverlay?.clear(); drawMode = false; },
  });

  type ViewerLoadWaiter = {
    targetNonce: number;
    resolve: () => void;
    reject: (error: Error) => void;
    timer: ReturnType<typeof setTimeout>;
  };
  let visibleViewerWaiters: ViewerLoadWaiter[] = [];
  let hiddenViewerWaiters: ViewerLoadWaiter[] = [];

  function settleViewerLoadWaiters(
    waiters: ViewerLoadWaiter[],
    currentNonce: number,
  ): ViewerLoadWaiter[] {
    const pending: ViewerLoadWaiter[] = [];
    for (const waiter of waiters) {
      if (currentNonce >= waiter.targetNonce) {
        clearTimeout(waiter.timer);
        waiter.resolve();
      } else {
        pending.push(waiter);
      }
    }
    return pending;
  }

  function waitForViewerLoad(
    kind: 'visible' | 'hidden',
    previousNonce: number,
    timeoutMs = 12000,
  ): Promise<void> {
    const currentNonce = kind === 'visible' ? visibleViewerLoadNonce : hiddenViewerLoadNonce;
    if (currentNonce > previousNonce) {
      return Promise.resolve();
    }
    return new Promise<void>((resolve, reject) => {
      const waiter: ViewerLoadWaiter = {
        targetNonce: previousNonce + 1,
        resolve,
        reject,
        timer: setTimeout(() => {
          if (kind === 'visible') {
            visibleViewerWaiters = visibleViewerWaiters.filter((candidate) => candidate !== waiter);
          } else {
            hiddenViewerWaiters = hiddenViewerWaiters.filter((candidate) => candidate !== waiter);
          }
          reject(new Error(`Timed out waiting for the ${kind} viewer to load.`));
        }, timeoutMs),
      };
      if (kind === 'visible') {
        visibleViewerWaiters = [...visibleViewerWaiters, waiter];
      } else {
        hiddenViewerWaiters = [...hiddenViewerWaiters, waiter];
      }
    });
  }

  function handleVisibleViewerLoaded() {
    visibleViewerLoadNonce += 1;
    visibleViewerWaiters = settleViewerLoadWaiters(visibleViewerWaiters, visibleViewerLoadNonce);
  }

  function handleHiddenViewerLoaded() {
    hiddenViewerLoadNonce += 1;
    hiddenViewerWaiters = settleViewerLoadWaiters(hiddenViewerWaiters, hiddenViewerLoadNonce);
  }

  function handleVisibleViewerCameraChange(nextCamera: ViewportCameraState) {
    if (!currentViewportTargetKey) return;
    cameraStateByTarget = rememberTargetCameraState(
      cameraStateByTarget,
      currentViewportTargetKey,
      nextCamera,
      true,
    );
  }

  function liveOverlayCanvas(includeOverlays: boolean): HTMLCanvasElement | null {
    if (!includeOverlays) return null;
    return drawingOverlay?.hasDrawing() ? drawingOverlay.getCanvas() : null;
  }

  function currentVisibleTargetRef() {
    const threadId = get(activeThreadId);
    const messageId = get(activeVersionId);
    const modelId = get(session).artifactBundle?.modelId ?? null;
    if (!threadId || !messageId) return null;
    return { threadId, messageId, modelId };
  }

  function viewerAssetsToUrls(assets: ViewerAsset[]): ViewerAsset[] {
    return assets.map((asset) => ({
      ...asset,
      path: toAssetUrl(asset.path),
    }));
  }

  function rememberVisibleViewportCapture(capture: ViewportScreenshotCapture) {
    if (!capture.threadId || !capture.messageId) return;
    const key = viewportTargetKey(capture.threadId, capture.messageId);
    lastLiveScreenshotByTarget = rememberTargetScreenshot(lastLiveScreenshotByTarget, key, capture);
    cameraStateByTarget = rememberTargetCameraState(
      cameraStateByTarget,
      key,
      capture.camera,
      true,
    );
  }

  async function sendViewportScreenshotReply(
    requestId: string,
    capture: ViewportScreenshotCapture,
  ) {
    await resolveAgentViewportScreenshot({
      requestId,
      dataUrl: capture.dataUrl,
      width: capture.width,
      height: capture.height,
      camera: capture.camera,
      source: capture.source ?? 'visible-live',
      threadId: capture.threadId ?? '',
      messageId: capture.messageId ?? '',
      modelId: capture.modelId ?? null,
      includeOverlays: capture.includeOverlays ?? false,
    });
  }

  async function rejectViewportScreenshotReply(requestId: string, error: unknown) {
    const message = typeof error === 'string' ? error : formatBackendError(error);
    try {
      await rejectAgentViewportScreenshot(requestId, message);
    } catch {
      // Ignore races with timeout cleanup on the backend side.
    }
  }

  function captureVisibleViewport(
    request: AgentViewportScreenshotEvent,
    source: string,
  ): ViewportScreenshotCapture | null {
    const visibleTarget = currentVisibleTargetRef();
    if (!viewerComponent || !visibleTarget) return null;
    const details = viewerComponent.captureScreenshotDetails(
      liveOverlayCanvas(request.includeOverlays),
    );
    if (!details) return null;
    const capture: ViewportScreenshotCapture = {
      dataUrl: details.dataUrl,
      width: details.width,
      height: details.height,
      camera: details.camera,
      capturedAt: Date.now(),
      source,
      threadId: visibleTarget.threadId,
      messageId: visibleTarget.messageId,
      modelId: visibleTarget.modelId,
      includeOverlays: request.includeOverlays,
    };
    rememberVisibleViewportCapture(capture);
    return capture;
  }

  async function captureHiddenTarget(
    request: AgentViewportScreenshotEvent,
    source: string,
  ): Promise<ViewportScreenshotCapture> {
    const targetKey = viewportTargetKey(request.threadId, request.messageId);
    const previousNonce = hiddenViewerLoadNonce;
    hiddenViewerSpec = null;
    await tick();
    hiddenViewerSpec = {
      requestId: request.requestId,
      targetKey,
      stlUrl: toAssetUrl(request.previewStlPath),
      viewerAssets: viewerAssetsToUrls(request.viewerAssets ?? []),
    };
    await waitForViewerLoad('hidden', previousNonce);
    if (!hiddenViewerComponent) {
      throw new Error('Hidden viewer is unavailable.');
    }
    hiddenViewerComponent.setCameraState(request.camera ?? null);
    const details = hiddenViewerComponent.captureScreenshotDetails();
    if (!details) {
      throw new Error('Failed to capture the hidden target preview.');
    }
    return {
      dataUrl: details.dataUrl,
      width: details.width,
      height: details.height,
      camera: details.camera,
      capturedAt: Date.now(),
      source,
      threadId: request.threadId,
      messageId: request.messageId,
      modelId: request.modelId ?? null,
      includeOverlays: false,
    };
  }

  async function switchToViewportTarget(request: AgentViewportScreenshotEvent) {
    const previousNonce = visibleViewerLoadNonce;
    const thread = await getThread(request.threadId);
    history.update((items) => {
      const nextThread = { ...thread, messages: thread.messages };
      return items.some((candidate) => candidate.id === thread.id)
        ? items.map((candidate) => (candidate.id === thread.id ? nextThread : candidate))
        : [nextThread, ...items];
    });
    const targetMessage =
      thread.messages.find(
        (message) =>
          message.id === request.messageId &&
          message.role === 'assistant' &&
          Boolean(message.output || message.artifactBundle),
      ) ?? null;
    if (!targetMessage) {
      throw new Error(`Target version ${request.messageId} is unavailable for screenshot capture.`);
    }
    activeThreadId.set(thread.id);
    currentView.set('workbench');
    await loadVersion(targetMessage);
    await waitForViewerLoad('visible', previousNonce);
  }

  async function processViewportScreenshotChoice(
    request: AgentViewportScreenshotEvent,
    choice: string,
  ) {
    const normalizedChoice = choice.trim().toLowerCase();
    if (normalizedChoice === 'cancel') {
      await rejectViewportScreenshotReply(request.requestId, 'Viewport screenshot cancelled by the user.');
      return;
    }

    if (normalizedChoice === 'current view') {
      const capture = captureVisibleViewport(request, 'current-view-mismatch');
      if (!capture) {
        await rejectViewportScreenshotReply(
          request.requestId,
          'Current view capture is unavailable because the workbench viewport is not visible.',
        );
        return;
      }
      await sendViewportScreenshotReply(request.requestId, capture);
      return;
    }

    if (normalizedChoice === 'switch & capture') {
      await switchToViewportTarget(request);
      const capture = captureVisibleViewport(request, 'switched-visible');
      if (!capture) {
        throw new Error('Switched to the target but failed to capture the visible viewport.');
      }
      await sendViewportScreenshotReply(request.requestId, capture);
      return;
    }

    if (normalizedChoice === 'fallback preview') {
      const targetKey = viewportTargetKey(request.threadId, request.messageId);
      const fallback = resolveFallbackScreenshotSource(lastLiveScreenshotByTarget, targetKey);
      if (fallback.kind === 'cached-live') {
        await sendViewportScreenshotReply(request.requestId, {
          ...fallback.capture,
          source: fallback.capture.source ?? 'cached-live',
          threadId: fallback.capture.threadId ?? request.threadId,
          messageId: fallback.capture.messageId ?? request.messageId,
          modelId: fallback.capture.modelId ?? request.modelId ?? null,
          includeOverlays: fallback.capture.includeOverlays ?? true,
        });
        return;
      }
      const capture = await captureHiddenTarget(request, request.camera ? 'hidden-target' : 'hidden-preview');
      await sendViewportScreenshotReply(request.requestId, capture);
      return;
    }

    await rejectViewportScreenshotReply(
      request.requestId,
      `Unsupported viewport screenshot choice: ${choice}`,
    );
  }

  async function handleViewportScreenshotEvent(request: AgentViewportScreenshotEvent) {
    try {
      const mode = chooseViewportCaptureMode({
        currentView: get(currentView),
        currentThreadId: get(activeThreadId),
        currentMessageId: get(activeVersionId),
        requestedThreadId: request.threadId,
        requestedMessageId: request.messageId,
        cameraOverride: request.camera ?? null,
        hasVisibleViewer: Boolean(
          viewerComponent &&
            get(currentView) === 'workbench' &&
            get(activeThreadId) &&
            get(activeVersionId),
        ),
      });

      if (mode === 'visible-live') {
        const capture = captureVisibleViewport(request, 'visible-live');
        if (!capture) {
          throw new Error('Visible viewport capture is unavailable.');
        }
        await sendViewportScreenshotReply(request.requestId, capture);
        return;
      }

      if (mode === 'hidden-target') {
        const capture = await captureHiddenTarget(request, 'hidden-target');
        await sendViewportScreenshotReply(request.requestId, capture);
        return;
      }

      const requestedLabel = `${request.threadId} / ${request.messageId}`;
      const message =
        'Agent wants a visual check, but the requested target is not the current live viewport. ' +
        `Choose what to send back for ${requestedLabel}.`;
      const nextChoice: PendingViewportScreenshotChoice = {
        ...request,
        message,
        buttons: ['Current View', 'Switch & Capture', 'Fallback Preview', 'Cancel'],
      };
      if (!pendingViewportScreenshotChoices.find((item) => item.requestId === request.requestId)) {
        pendingViewportScreenshotChoices = [...pendingViewportScreenshotChoices, nextChoice];
      }
    } catch (error) {
      await rejectViewportScreenshotReply(request.requestId, error);
    }
  }

  // Shut down audio context when idle for 2s
  let idleTimeout: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    const hasAudioActivity = $activeRequestCount > 0 || $activeMicrowaveCount > 0;
    if (hasAudioActivity) {
      if (idleTimeout) {
        clearTimeout(idleTimeout);
        idleTimeout = null;
        console.info('[Microwave] idle shutdown canceled', {
          activeRequests: $activeRequestCount,
          activeMicrowaves: $activeMicrowaveCount,
        });
      }
      return;
    }

    if (!idleTimeout) {
      console.info('[Microwave] idle shutdown scheduled');
      idleTimeout = setTimeout(() => {
        const stillActive = get(activeRequestCount) > 0 || get(activeMicrowaveCount) > 0;
        if (stillActive) {
          console.info('[Microwave] idle shutdown skipped due to renewed activity', {
            activeRequests: get(activeRequestCount),
            activeMicrowaves: get(activeMicrowaveCount),
          });
          idleTimeout = null;
          return;
        }
        console.info('[Microwave] idle shutdown closing audio context');
        stopMicrowaveAudio(true);
        idleTimeout = null;
      }, 2000);
    }
  });

  $effect(() => {
    if (
      !isBooting &&
      !$config.hasSeenOnboarding &&
      !$onboarding.isActive &&
      !shouldSuppressOnboardingForAutomation()
    ) {
      onboarding.start();
    }
  });

  // Wire thread changes to audio focus
  $effect(() => {
    setAudibleThread($activeThreadId);
  });

  $effect(() => {
    const nextMicrowaveKey =
      isActiveMcpMode && activeMcpBusy && threadAgentState?.sessionId
        ? `__mcp__:${threadAgentState.sessionId}`
        : '';
    if (activeMcpMicrowaveKey && activeMcpMicrowaveKey !== nextMicrowaveKey) {
      stopMicrowaveHum(activeMcpMicrowaveKey);
    }
    if (nextMicrowaveKey && nextMicrowaveKey !== activeMcpMicrowaveKey) {
      startMicrowaveHum(nextMicrowaveKey, $config, $activeThreadId ?? null);
    }
    activeMcpMicrowaveKey = nextMicrowaveKey;
  });

  $effect(() => {
    const localThinkingActive = ['classifying', 'generating', 'answering'].includes(activeThreadHighestPhase);
    const shouldOwnPhraseLoop = isActiveMcpMode && activeMcpBusy && !localThinkingActive;
    if (shouldOwnPhraseLoop && !ownsMcpPhraseLoop) {
      startCookingPhraseLoop();
      ownsMcpPhraseLoop = true;
      return;
    }
    if (!shouldOwnPhraseLoop && ownsMcpPhraseLoop) {
      if (!localThinkingActive) {
        stopPhraseLoop();
      }
      ownsMcpPhraseLoop = false;
    }
  });


  function formatCookingTime(s: number) {
    const m = Math.floor(s / 60);
    const sec = s % 60;
    return `${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}`;
  }

  // --- Agent confirmation requests ---
  type AgentConfirmItem = { requestId: string; message: string; buttons: string[]; agentLabel: string };
  let pendingConfirms = $state<AgentConfirmItem[]>([]);
  const activeViewportScreenshotChoice = $derived(pendingViewportScreenshotChoices[0] ?? null);

  async function answerConfirm(requestId: string, choice: string) {
    pendingConfirms = pendingConfirms.filter(c => c.requestId !== requestId);
    try { await resolveAgentConfirm(requestId, choice); } catch { /* already timed out */ }
  }

  async function answerViewportScreenshotChoice(requestId: string, choice: string) {
    const request =
      pendingViewportScreenshotChoices.find((item) => item.requestId === requestId) ?? null;
    pendingViewportScreenshotChoices = pendingViewportScreenshotChoices.filter(
      (item) => item.requestId !== requestId,
    );
    if (!request) return;
    try {
      await processViewportScreenshotChoice(request, choice);
    } catch (error) {
      await rejectViewportScreenshotReply(requestId, error);
    }
  }

  async function answerAgentPrompt(requestId: string, promptText: string, attachments: Attachment[]) {
    pendingAgentPrompt = null;
    try {
      await resolveAgentPrompt({
        requestId,
        promptText,
        attachments,
      });
    } catch { /* already timed out */ }
    void refreshThreadAgentState();
  }

  async function handleDialogueSubmit(prompt: string, attachments: Attachment[]) {
    switch (dialogueState.mode) {
      case 'agent-reply': await answerAgentPrompt(dialogueState.requestId, prompt, attachments); break;
      case 'generate':    await handleGenerate(prompt, attachments); break;
      case 'mcp-idle': {
        try {
          await persistBackendConfig(get(config));
        } catch (e) {
          session.setError(`Config Save Error: ${formatBackendError(e)}`);
          break;
        }
        queuedUserMessages = [
          ...queuedUserMessages,
          {
            id: crypto.randomUUID(),
            text: prompt,
            attachments,
            status: 'queued',
          },
        ];
        try {
          await wakePrimaryAutoAgent($activeThreadId);
          await refreshThreadAgentState();
        } catch (e) {
          session.setError(`Agent Wake Error: ${formatBackendError(e)}`);
        }
        break;
      }
    }
  }

  function updateActiveThreadConceptPreviewState(
    updater: (state: ConceptPreviewUiState) => ConceptPreviewUiState,
  ) {
    const threadId = $activeThreadId;
    if (!threadId) return;
    const previous = conceptPreviewUiByThread[threadId] ?? defaultConceptPreviewUiState();
    const next = updater(previous);
    if (sameConceptPreviewUiState(previous, next)) return;
    conceptPreviewUiByThread = {
      ...conceptPreviewUiByThread,
      [threadId]: next,
    };
  }

  function setViewportPresentationMode(mode: ViewportPresentationMode) {
    updateActiveThreadConceptPreviewState((state) => ({ ...state, mode }));
  }

  function pinConceptPreviewMessageId(
    messageId: string | null,
    mode: ViewportPresentationMode | null = null,
  ) {
    updateActiveThreadConceptPreviewState((state) => ({
      ...state,
      pinnedMessageId: messageId,
      mode: mode ?? state.mode,
    }));
  }

  function openConceptPreviewInViewport(message: Message) {
    pinConceptPreviewMessageId(message.id, 'blueprint');
  }

  function pinConceptPreviewFromMessage(message: Message) {
    pinConceptPreviewMessageId(message.id);
  }

  function pickOtherConceptPreview() {
    const nextMessageId = cycleConceptPreviewMessageId(
      activeThread?.messages ?? [],
      effectiveConceptPreviewMessage?.id ?? activeThreadConceptPreviewState.pinnedMessageId,
    );
    if (!nextMessageId) return;
    pinConceptPreviewMessageId(nextMessageId, 'blueprint');
  }

  async function waitForBlueprintImageReady(): Promise<HTMLImageElement | null> {
    await tick();
    const image = blueprintImageEl;
    if (!image) return null;
    if (image.complete && image.naturalWidth > 0 && image.naturalHeight > 0) {
      return image;
    }
    await new Promise<void>((resolve) => {
      const cleanup = () => {
        image.removeEventListener('load', handleLoad);
        image.removeEventListener('error', handleLoad);
      };
      const handleLoad = () => {
        cleanup();
        resolve();
      };
      image.addEventListener('load', handleLoad, { once: true });
      image.addEventListener('error', handleLoad, { once: true });
      window.setTimeout(() => {
        cleanup();
        resolve();
      }, 500);
    });
    return blueprintImageEl;
  }

  async function captureConceptPreviewCompositeDataUrl(): Promise<string | null> {
    const conceptMessage = effectiveConceptPreviewMessage;
    if (!conceptMessage?.imageData) return null;

    const image = await waitForBlueprintImageReady();
    const overlayCanvas = drawingOverlay?.hasDrawing() ? drawingOverlay.getCanvas() : null;
    if (!image || !overlayCanvas || !viewportAreaEl) {
      return conceptMessage.imageData;
    }

    const viewportRect = viewportAreaEl.getBoundingClientRect();
    const imageRect = image.getBoundingClientRect();
    if (
      viewportRect.width <= 0 ||
      viewportRect.height <= 0 ||
      imageRect.width <= 0 ||
      imageRect.height <= 0
    ) {
      return conceptMessage.imageData;
    }

    const naturalWidth = image.naturalWidth || Math.round(imageRect.width);
    const naturalHeight = image.naturalHeight || Math.round(imageRect.height);
    if (naturalWidth <= 0 || naturalHeight <= 0) {
      return conceptMessage.imageData;
    }

    const overlayScaleX = overlayCanvas.width / viewportRect.width;
    const overlayScaleY = overlayCanvas.height / viewportRect.height;
    const sourceX = Math.max(0, (imageRect.left - viewportRect.left) * overlayScaleX);
    const sourceY = Math.max(0, (imageRect.top - viewportRect.top) * overlayScaleY);
    const sourceWidth = Math.min(
      overlayCanvas.width - sourceX,
      imageRect.width * overlayScaleX,
    );
    const sourceHeight = Math.min(
      overlayCanvas.height - sourceY,
      imageRect.height * overlayScaleY,
    );

    if (sourceWidth <= 0 || sourceHeight <= 0) {
      return conceptMessage.imageData;
    }

    const composed = document.createElement('canvas');
    composed.width = Math.max(1, Math.round(naturalWidth));
    composed.height = Math.max(1, Math.round(naturalHeight));
    const context = composed.getContext('2d');
    if (!context) return conceptMessage.imageData;

    context.drawImage(image, 0, 0, composed.width, composed.height);
    context.drawImage(
      overlayCanvas,
      sourceX,
      sourceY,
      sourceWidth,
      sourceHeight,
      0,
      0,
      composed.width,
      composed.height,
    );
    return composed.toDataURL('image/png');
  }

  async function resolveBlueprintPromptImageOverride(): Promise<string | null> {
    if (!showBlueprintViewport || !effectiveConceptPreviewMessage) return null;
    return captureConceptPreviewCompositeDataUrl();
  }

  async function generateFromConceptPreview() {
    if (!effectiveConceptPreviewMessage) return;
    const imageDataOverride =
      (await resolveBlueprintPromptImageOverride()) ?? effectiveConceptPreviewMessage.imageData ?? null;
    await handleGenerate(
      buildGenerateFromConceptPrompt(effectiveConceptPreviewMessage),
      [],
      { imageDataOverride },
    );
  }

  async function handlePromptPanelSubmit(prompt: string, attachments: Attachment[]) {
    if (dialogueState.mode !== 'generate') {
      await handleDialogueSubmit(prompt, attachments);
      return;
    }
    const imageDataOverride = await resolveBlueprintPromptImageOverride();
    await handleGenerate(prompt, attachments, { imageDataOverride });
  }

  onMount(() => {
    void boot();
    // Initial fetch of agent sessions (push events only fire on changes, not on load)
    void getActiveAgentSessions().then(sessions => { activeAgentSessions = sessions; }).catch(() => {});
    void getAgentTerminalSnapshots()
      .then((snapshots) => {
        agentTerminalSnapshots = Object.fromEntries(
          snapshots
            .filter(hasRenderableTerminal)
            .map((snapshot) => [snapshot.agentId, snapshot]),
        );
      })
      .catch(() => {});
    threadAgentPollInterval = setInterval(() => void refreshThreadAgentState(), 1000);

    const unlisten = listen<AgentConfirmItem>('agent-confirm-request', (event) => {
      const item = event.payload;
      if (!pendingConfirms.find(c => c.requestId === item.requestId)) {
        pendingConfirms = [...pendingConfirms, item];
      }
    });
    const unlistenPrompt = listen<PendingAgentPrompt>('agent-prompt-request', (event) => {
      pendingAgentPrompt = event.payload;
      void refreshThreadAgentState();
    });
    const unlistenViewportScreenshot = listen<AgentViewportScreenshotEvent>(
      'agent-viewport-screenshot-request',
      (event) => {
        void handleViewportScreenshotEvent(event.payload);
      },
    );
    const unlistenHistory = listen('history-updated', async () => {
      await refreshHistory();
      void refreshThreadAgentState();
    });
    const unlistenSessions = listen<AgentSession[]>('agent-sessions-changed', (event) => {
      activeAgentSessions = event.payload;
      void refreshThreadAgentState();
    });
    const unlistenTerminal = listen<AgentTerminalSnapshot>('agent-terminal-updated', (event) => {
      upsertAgentTerminalSnapshot(event.payload);
    });
    const unlistenTrace = listen<AgentSessionTraceEntry>('agent-session-trace-updated', (event) => {
      upsertAgentSessionTraceEntry(event.payload);
      if (event.payload.sessionId === threadAgentState?.sessionId) {
        void refreshThreadAgentState();
      }
    });
    return () => {
      if (threadAgentPollInterval) clearInterval(threadAgentPollInterval);
      void unlisten.then(fn => fn());
      void unlistenPrompt.then(fn => fn());
      void unlistenViewportScreenshot.then(fn => fn());
      void unlistenHistory.then(fn => fn());
      void unlistenSessions.then(fn => fn());
      void unlistenTerminal.then(fn => fn());
      void unlistenTrace.then(fn => fn());
    };
  });

  const activeThread = $derived($history.find(t => t.id === $activeThreadId));
  const activeThreadConceptPreviewState = $derived.by<ConceptPreviewUiState>(() => {
    if (!$activeThreadId) return defaultConceptPreviewUiState();
    return conceptPreviewUiByThread[$activeThreadId] ?? defaultConceptPreviewUiState();
  });
  const conceptPreviewMessages = $derived.by<Message[]>(() =>
    listConceptPreviewMessages(activeThread?.messages ?? []),
  );
  const effectiveConceptPreviewMessage = $derived.by<Message | null>(() =>
    resolveEffectiveConceptPreviewMessage(
      activeThread?.messages ?? [],
      activeThreadConceptPreviewState.pinnedMessageId,
    ),
  );
  const viewportPresentationMode = $derived.by<ViewportPresentationMode>(() =>
    activeThreadConceptPreviewState.mode,
  );
  const showBlueprintViewport = $derived.by(
    () => viewportPresentationMode === 'blueprint' && !!effectiveConceptPreviewMessage,
  );
  const blueprintAttentionVisible = $derived.by(
    () => hasRenderableModel && !!effectiveConceptPreviewMessage && viewportPresentationMode !== 'blueprint',
  );
  $effect(() => {
    const threadId = $activeThreadId;
    if (!threadId) return;
    const previous = conceptPreviewUiByThread[threadId] ?? defaultConceptPreviewUiState();
    const next = reconcileConceptPreviewUiState({
      messages: activeThread?.messages ?? [],
      previous,
      hasModel: hasRenderableModel,
    }).nextState;
    if (sameConceptPreviewUiState(previous, next)) return;
    conceptPreviewUiByThread = {
      ...conceptPreviewUiByThread,
      [threadId]: next,
    };
  });
  $effect(() => {
    if (!showBlueprintViewport) {
      blueprintImageEl = null;
    }
  });
  const activeVersionMessage = $derived.by<Message | null>(() => {
    if (!activeThread) return null;
    return (
      activeThread.messages.find(
        (message) =>
          message.id === $activeVersionId &&
          message.role === 'assistant' &&
          Boolean(message.output || message.artifactBundle),
      ) ?? null
    );
  });
  const currentViewportTargetKey = $derived.by<string | null>(() => {
    if (!$activeThreadId || !$activeVersionId) return null;
    return viewportTargetKey($activeThreadId, $activeVersionId);
  });
  const persistedViewportCameraState = $derived.by<ViewportCameraState | null>(() => {
    if (!currentViewportTargetKey) return null;
    return cameraStateByTarget[currentViewportTargetKey] ?? null;
  });
  const activeVersionAgentLabel = $derived(formatAgentOriginLabel(activeVersionMessage?.agentOrigin));
  const agentTerminalSnapshotList = $derived.by<AgentTerminalSnapshot[]>(() =>
    Object.values(agentTerminalSnapshots),
  );
  const primaryAgentId = $derived.by<string | null>(() =>
    derivePrimaryAgentId($config.mcp.autoAgents ?? [], $config.mcp.primaryAgentId ?? null),
  );
  const visibleAgentTerminal = $derived.by<AgentTerminalSnapshot | null>(() =>
    pickVisibleAgentTerminal(agentTerminalSnapshotList, primaryAgentId),
  );
  const activeAgentTerminalAttention = $derived.by<AgentTerminalSnapshot | null>(() =>
    pickAgentTerminalAttention(agentTerminalSnapshotList, primaryAgentId),
  );
  const activeMcpBusy = $derived.by<boolean>(() => isThreadAgentBusy(threadAgentState));
  const activeMcpBubbleSummary = $derived.by<string>(() =>
    resolveActiveMcpBubble({
      threadAgentState,
      visibleAgentTerminal,
      cookingPhrase: $session.cookingPhrase,
      nowSecs: $nowSeconds,
    }),
  );
  const activeAgentTerminalMetaSummary = $derived.by<string>(() =>
    resolveTerminalActivityMeta({
      threadAgentState,
      visibleAgentTerminal,
      nowSecs: $nowSeconds,
    }),
  );
  const activeAgentSessionTrace = $derived.by<AgentSessionTraceEntry[]>(() => {
    const sessionId = threadAgentState?.sessionId;
    return sessionId ? (agentSessionTraces[sessionId] ?? []) : [];
  });
  const latestAgentSessionTrace = $derived.by<AgentSessionTraceEntry | null>(() =>
    activeAgentSessionTrace.length ? activeAgentSessionTrace[activeAgentSessionTrace.length - 1] : null,
  );
  const eckyTraits = $derived<Partial<GenieTraits>>(activeThread?.genieTraits || {});
  const eckyIntensity = $derived(1.0 + Math.max(0, ($activeRequestCount - 1) * 0.25));

  function hasTauriIpc(): boolean {
    if (typeof window === 'undefined') return false;
    return typeof (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ === 'object';
  }

  async function refreshThreadAgentState() {
    if (!hasTauriIpc() || !isActiveMcpMode || !$activeThreadId) {
      threadAgentState = null;
      return;
    }
    try {
      const nextState = await getThreadAgentState($activeThreadId);
      threadAgentState = nextState;
      const sessionId = nextState?.sessionId ?? null;
      if (sessionId && !agentSessionTraces[sessionId]) {
        try {
          replaceAgentSessionTrace(sessionId, await getAgentSessionTrace(sessionId));
        } catch {
          // Ignore trace refresh failures; polling will try again later.
        }
      }
    } catch {
      threadAgentState = null;
    }
  }

  $effect(() => {
    if (!isActiveMcpMode || !$activeThreadId) {
      threadAgentState = null;
      return;
    }
    void refreshThreadAgentState();
  });

  const inFlightByThread = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const req of $allRequests) {
      if (!req?.threadId) continue;
      if (['success', 'error', 'canceled'].includes(req.phase)) continue;
      counts[req.threadId] = (counts[req.threadId] || 0) + 1;
    }
    return counts;
  });

  const latestAssistantMessage = $derived.by(() => {
    if (!activeThread?.messages?.length) return null;
    return [...activeThread.messages].reverse().find(m => m.role === 'assistant' && m.status !== 'pending') ?? null;
  });

  const assistantBubble = $derived.by(() => {
    if (!latestAssistantMessage) return '';
    const out = latestAssistantMessage.output;
    return out?.response || (out?.title ? `Generated: ${out.title}` : latestAssistantMessage.content) || '';
  });

  const assistantFresh = $derived.by(() => {
    if (!latestAssistantMessage?.timestamp) return false;
    return $nowSeconds - latestAssistantMessage.timestamp <= 300;
  });

  $effect(() => {
    const msgId = latestAssistantMessage?.id;
    if (msgId && msgId !== lastAssistantMessageId) {
      lastAssistantMessageId = msgId;
      if (assistantFresh) {
        lastAdvisorBubble = assistantBubble;
        dismissedBubbleText = '';
      } else {
        lastAdvisorBubble = '';
        dismissedBubbleText = '';
      }
    }
  });

  const activeThreadHighestPhase = $derived.by<ThreadPhase>(() => {
    if (phase === 'booting') return 'booting';

    const activeRequests = $activeThreadRequests.filter(
      (request) => !['success', 'error', 'canceled'].includes(request.phase),
    );
    const activePhases = activeRequests.map((request) => request.phase);
    if (activePhases.some((requestPhase) => ['rendering', 'queued_for_render', 'committing'].includes(requestPhase))) {
      return 'rendering';
    }
    if (activePhases.some((requestPhase) => requestPhase === 'repairing')) return 'repairing';
    if (activePhases.some((requestPhase) => requestPhase === 'generating')) return 'generating';
    if (activePhases.some((requestPhase) => requestPhase === 'answering')) return 'answering';
    if (activePhases.some((requestPhase) => requestPhase === 'classifying')) return 'classifying';

    const threadErrors = $activeThreadRequests.filter((request) => request.phase === 'error' && request.error);
    if (threadErrors.length > 0) return 'error';

    return 'idle';
  });

  const activeConfirm = $derived(pendingConfirms[0] ?? null);
  const threadAgentMascot = $derived.by(() => deriveMascotStateForThreadAgent(threadAgentState));

  const genieMode = $derived.by(() => {
    if ($onboarding.isActive) return 'speaking';
    if (activeViewportScreenshotChoice) return 'speaking';
    if (activeConfirm) return 'speaking';
    if (activeAgentTerminalAttention) return 'speaking';
    if (activePendingAgentPrompt) return 'speaking';
    if (threadAgentState?.connectionState === 'waking') return 'waking';
    if (threadAgentState?.connectionState === 'sleeping') return 'idle';
    if (threadAgentState?.connectionState === 'waiting') return 'light';
    if (threadAgentState?.connectionState === 'active') return threadAgentMascot.mode;
    if (threadAgentState?.connectionState === 'error') return 'error';
    const atPhase = activeThreadHighestPhase;
    if (atPhase === 'error') return 'error';
    if (atPhase === 'repairing') return 'repairing';
    if (atPhase === 'classifying') return 'light';
    if (atPhase === 'rendering') return 'rendering';
    if (atPhase === 'generating' || atPhase === 'answering') return 'thinking';
    if (assistantFresh && !dismissedBubbleText && lastAdvisorBubble) return 'speaking';
    if (threadAgentState?.connectionState === 'disconnected') return 'idle';
    return 'idle';
  });

  const genieBubble = $derived.by(() => {
    if ($onboarding.isActive) return $onboarding.text;
    if (activeViewportScreenshotChoice) return activeViewportScreenshotChoice.message;
    if (activeConfirm) return activeConfirm.message;
    if (activeAgentTerminalAttention) {
      return (
        activeAgentTerminalAttention.summary ||
        `${activeAgentTerminalAttention.agentLabel} needs terminal input.`
      );
    }

    let raw = '';
    if (activePendingAgentPrompt) {
      raw =
        activePendingAgentPrompt.message ||
        `${activePendingAgentPrompt.agentLabel} is waiting for your input`;
    } else if (threadAgentState?.connectionState === 'active') {
      raw = activeMcpBubbleSummary;
    } else if (threadAgentState?.latestTraceSummary) {
      raw = threadAgentState.latestTraceSummary;
    } else if (threadAgentMascot.bubble) {
      raw = threadAgentMascot.bubble;
    } else {
      const atPhase = activeThreadHighestPhase;
      const threadError =
        atPhase === 'error'
          ? [...$activeThreadRequests].reverse().find((request) => request.phase === 'error' && request.error)
              ?.error
          : null;
      raw = threadError ||
            (atPhase === 'repairing' ? $session.repairMessage : null) ||
            (['classifying', 'generating', 'answering'].includes(atPhase) ? $session.cookingPhrase : null) ||
            lastAdvisorBubble || '';
    }
    return (dismissedBubbleText === raw) ? '' : raw;
  });

  // Reset dismiss state and waking message when a new agent prompt arrives
  $effect(() => {
    if (activePendingAgentPrompt?.requestId) {
      dismissedBubbleText = '';
    }
  });

  const genieActions = $derived.by(() => {
    if ($onboarding.isActive) {
      return [
        { label: 'NEXT', onclick: () => onboarding.next() },
        { label: 'SKIP', onclick: () => onboarding.skip() }
      ];
    }
    if (activeViewportScreenshotChoice) {
      return activeViewportScreenshotChoice.buttons.map((button) => ({
        label: button,
        onclick: () => answerViewportScreenshotChoice(activeViewportScreenshotChoice.requestId, button),
      }));
    }
    if (activeConfirm) {
      return activeConfirm.buttons.map(btn => ({
        label: btn,
        onclick: () => answerConfirm(activeConfirm.requestId, btn),
      }));
    }
    if (activeAgentTerminalAttention) {
      return [
        {
          label: 'OPEN TERMINAL',
          onclick: () => {
            showAgentTerminalWindow = true;
          },
        },
      ];
    }
    if (isActiveMcpMode && $activeThreadId && threadAgentState?.connectionState !== 'none') {
      const connectionState = threadAgentState?.connectionState;
      if (!connectionState) return null;
      const actions: Array<{ label: string; onclick: () => void }> = [];
      if (visibleAgentTerminal) {
        actions.push({
          label: 'OPEN TERMINAL',
          onclick: () => {
            showAgentTerminalWindow = true;
          },
        });
      }
      if (connectionState === 'sleeping') {
        actions.push({
          label: 'WAKE AGENT',
          onclick: () => {
            void handleWakePrimaryAgent();
          },
        });
      } else {
        if (threadAgentState?.awaitingPromptRearm && visibleAgentTerminal?.active) {
          actions.push({
            label: 'NUDGE AGENT',
            onclick: () => {
              void handleNudgeAgentPromptRearm();
            },
          });
        }
        actions.push({
          label: 'RESTART AGENT',
          onclick: () => {
            void handleRestartPrimaryAgent();
          },
        });
        actions.push({
          label: 'STOP AGENT',
          onclick: () => {
            void handleStopPrimaryAgent();
          },
        });
      }
      return actions;
    }
    return null;
  });

  async function handleWakePrimaryAgent() {
    if (!$activeThreadId || agentControlBusy) return;
    agentControlBusy = true;
    try {
      await wakePrimaryAutoAgent($activeThreadId);
      await refreshThreadAgentState();
    } catch (e: unknown) {
      session.setError(`Agent Wake Error: ${formatBackendError(e)}`);
    } finally {
      agentControlBusy = false;
    }
  }

  async function handleStopPrimaryAgent() {
    if (!$activeThreadId || agentControlBusy) return;
    agentControlBusy = true;
    try {
      await stopPrimaryAutoAgent($activeThreadId);
      await refreshThreadAgentState();
    } catch (e: unknown) {
      session.setError(`Agent Stop Error: ${formatBackendError(e)}`);
    } finally {
      agentControlBusy = false;
    }
  }

  async function handleRestartPrimaryAgent() {
    if (!$activeThreadId || agentControlBusy) return;
    agentControlBusy = true;
    try {
      await restartPrimaryAutoAgent($activeThreadId);
      await refreshThreadAgentState();
    } catch (e: unknown) {
      session.setError(`Agent Restart Error: ${formatBackendError(e)}`);
    } finally {
      agentControlBusy = false;
    }
  }

  async function toggleMicrowaveMute() {
    const currentConfig = get(config);
    const newMuted = !(currentConfig?.microwave?.muted);
    const nextConfig = {
      ...currentConfig,
      microwave: {
        ...(currentConfig?.microwave || { humId: null, dingId: null }),
        muted: newMuted,
      },
    };
    config.set(nextConfig);
    setMuted(newMuted, nextConfig);
    await saveConfig();
  }

  function applyCompletedRequest(req: Request) {
    if (!req?.result) return;
    const { design, threadId, messageId, stlUrl: reqStlUrl, artifactBundle, modelManifest } =
      req.result;
    const renderableBundle = getRenderableRuntimeBundle(artifactBundle ?? null, design?.postProcessing ?? null);
    if (design) {
      workingCopy.loadVersion(design, messageId);
      paramPanelState.hydrateFromVersion(design, messageId);
    }
    if (threadId) {
      activeThreadId.set(threadId);
      activeVersionId.set(messageId);
    }
    if (reqStlUrl) {
      session.setStlUrl(reqStlUrl);
    }
    if (renderableBundle || modelManifest) {
      session.setModelRuntime(renderableBundle ?? null, modelManifest ?? null);
    }
    void persistLastSessionSnapshot({
      design: design ?? null,
      threadId,
      messageId,
      artifactBundle: renderableBundle ?? null,
      modelManifest: modelManifest ?? null,
    });
    requestQueue.setActive(req.id);
  }

  function dismissRequest(id: string) {
    requestQueue.remove(id);
  }

  function retryRequest(req: Request) {
    void handleGenerate(req.prompt, req.attachments);
    requestQueue.remove(req.id);
  }

  function cancelRequest(id: string) {
    requestQueue.cancel(id);
  }

  function phaseLabel(phase: Request['phase']) {
    const labels: Partial<Record<Request['phase'], string>> = {
      classifying: 'ROUTING',
      generating: 'LLM',
      queued_for_render: 'QUEUED',
      rendering: 'FREECAD',
      committing: 'SAVING',
      success: 'DONE',
      error: 'ERROR',
      canceled: 'CANCELED',
    };
    return labels[phase] || phase.toUpperCase();
  }

  function toAssetUrl(path: string | null | undefined): string {
    if (!path) return '';
    try {
      return convertFileSrc(path);
    } catch {
      return path;
    }
  }

  async function exportSTL() {
    if (!stlUrl) return;
    try {
      const path = await save({ filters: [{ name: 'STL 3D Model', extensions: ['stl'] }], defaultPath: 'design.stl' });
      if (typeof path === 'string') {
        let rawPath = decodeURIComponent(stlUrl.split('?')[0].replace('asset://localhost/', '/'));
        if (!rawPath.startsWith('/') && rawPath.match(/^[a-zA-Z]:/)) {} else if (!rawPath.startsWith('/')) { rawPath = '/' + rawPath; }
        await exportFile(rawPath, path);
      }
    } catch (e: unknown) {
      session.setError(`Export Error: ${formatBackendError(e)}`);
    }
  }

  async function exportFCStd() {
    const bundle = $session.artifactBundle;
    if (!bundle?.fcstdPath) return;
    try {
      const path = await save({ filters: [{ name: 'FreeCAD Document', extensions: ['FCStd'] }], defaultPath: 'design.FCStd' });
      if (typeof path === 'string') {
        await exportFile(bundle.fcstdPath, path);
      }
    } catch (e: unknown) {
      session.setError(`Export Error: ${formatBackendError(e)}`);
    }
  }

  function dismissGenie() {
    if (genieBubble) dismissedBubbleText = genieBubble;
  }

  function dismissError() {
    session.setError(null);
  }

  $effect(() => {
    if (showAgentTerminalWindow && !visibleAgentTerminal) {
      showAgentTerminalWindow = false;
    }
  });

  $effect(() => {
    const nextKey = `${$activeThreadId ?? ''}:${$activeVersionId ?? ''}:${activeArtifactBundle?.modelId ?? ''}`;
    if (nextKey === lastViewportContextKey) return;
    lastViewportContextKey = nextKey;
    selectedContextTargetId = null;
    sharedContextSearchQuery = '';
    focusedMeasurementControl = null;
  });

  $effect(() => {
    if (activeModelManifest) return;
    selectedContextTargetId = null;
    sharedContextSearchQuery = '';
    focusedMeasurementControl = null;
  });

  $effect(() => {
    const snapshot = visibleAgentTerminal;
    const focusKey = showAgentTerminalWindow && snapshot?.active
      ? `${agentTerminalSessionKey(snapshot)}:${snapshot.active}`
      : '';
    if (!focusKey) {
      lastAgentTerminalFocusKey = '';
      return;
    }
    if (focusKey === lastAgentTerminalFocusKey) return;
    lastAgentTerminalFocusKey = focusKey;
    void tick().then(() => {
      agentTerminalSurface?.focusTerminal();
    });
  });

  function handleTargetSelect(target: ContextSelectionTarget | null) {
    const nextTarget = target ?? createGlobalContextTarget(activeModelManifest);
    const partId = deriveSelectedPartId(nextTarget);
    selectedContextTargetId = nextTarget?.targetId ?? null;
    focusedMeasurementControl = null;
    session.setSelectedPartId(partId);
    void persistLastSessionSnapshot({ selectedPartId: partId });
  }

  function handlePartSelect(partId: string | null) {
    if (!partId) {
      handleTargetSelect(null);
      return;
    }
    const nextTarget =
      contextSelectionTargets.find((target) => target.kind === 'part' && target.partId === partId) ??
      resolveContextSelectionTarget(activeModelManifest, contextSelectionTargets, null, partId);
    handleTargetSelect(nextTarget);
  }

  function handleSemanticControlChange(primitiveId: string, value: ParamValue) {
    const nextParams = buildSemanticPatch(activeModelManifest, primitiveId, value, effectiveUiSpec);
    if (Object.keys(nextParams).length === 0) return;
    if ($liveApply) {
      void handleParamChange(nextParams);
      return;
    }
    stageParamChange(nextParams);
  }

  function handleSelectControlView(viewId: string | null) {
    activeControlViewId = viewId;
  }

  async function handleImportFcstd(sourcePath: string) {
    try {
      session.setError(null);
      session.setStatus('Importing FCStd...');
      const bundle = await importFcstd(sourcePath);
      const rawManifest = await getModelManifest(bundle.modelId);
      const importedUiSpec = buildImportedUiSpec(rawManifest);
      const importedParams = buildImportedParams(rawManifest, {}, importedUiSpec);
      const manifest = ensureSemanticManifest(rawManifest, importedUiSpec, importedParams) ?? rawManifest;
      const threadId = crypto.randomUUID();
      const importedName = sourcePath.split(/[\\/]/).pop() || 'model.FCStd';
      const title =
        manifest.document.documentLabel ||
        manifest.document.documentName ||
        importedName.replace(/\.fcstd$/i, '');
      const messageId = await addImportedModelVersion({
        threadId,
        title,
        artifactBundle: bundle,
        modelManifest: manifest,
      });
      await saveModelManifest(bundle.modelId, manifest, messageId);
      activeThreadId.set(threadId);
      activeVersionId.set(messageId);
      workingCopy.reset();
      paramPanelState.reset();
      session.setStlUrl(toAssetUrl(bundle.previewStlPath));
      session.setModelRuntime(bundle, manifest);
      await refreshHistory();
      await persistLastSessionSnapshot({
        design: null,
        threadId,
        messageId,
        artifactBundle: bundle,
        modelManifest: manifest,
        selectedPartId: null,
      });
      session.setStatus(`Imported FCStd: ${importedName}`);
      currentView.set('workbench');
      if (manifest.enrichmentState?.status === 'pending') {
        showEnrichmentModal = true;
      }
    } catch (e: unknown) {
      session.setError(`FCStd Import Error: ${formatBackendError(e)}`);
    }
  }

  function startResizingWidth(e: MouseEvent) {
    isResizingWidth = true;
    e.preventDefault();
  }

  function startResizingHeight(e: MouseEvent) {
    isResizingHeight = true;
    e.preventDefault();
  }

  function startResizingHistory(e: MouseEvent) {
    isResizingHistory = true;
    e.preventDefault();
  }

  function handleMouseMove(e: MouseEvent) {
    if (isResizingWidth) {
      $sidebarWidth = Math.max(250, Math.min(e.clientX, window.innerWidth - 300));
    } else if (isResizingHeight) {
      $dialogueHeight = Math.max(120, Math.min(window.innerHeight - e.clientY, window.innerHeight - 150));
    } else if (isResizingHistory) {
      const sidebarRect = document.querySelector('.sidebar')?.getBoundingClientRect();
      if (sidebarRect) {
        const heightFromBottom = sidebarRect.bottom - e.clientY;
        $historyHeight = Math.max(100, Math.min(heightFromBottom, sidebarRect.height - 100));
      }
    }
  }

  function stopResizing() {
    isResizingWidth = false;
    isResizingHeight = false;
    isResizingHistory = false;
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') stopResizing();
  }
</script>

<svelte:window onmousemove={handleMouseMove} onmouseup={stopResizing} onblur={stopResizing} onkeydown={handleGlobalKeydown} />

<div class="app-page" role="application">
  {#if $onboarding.isActive}
    <div class="onboarding-backdrop"></div>
  {/if}
  <div class="app-overlay-actions">
    {#if $currentView === 'workbench'}
      {#if $activeRequestCount > 0}
        <button class="overlay-icon-btn" onclick={toggleMicrowaveMute} title="Toggle Cafeteria Hum">
          {$config?.microwave?.muted ? '🔇' : '🔊'}
        </button>
      {/if}
      {#if visibleAgentTerminal}
        <button
          class="overlay-icon-btn terminal-overlay-btn"
          class:terminal-overlay-btn-attention={visibleAgentTerminal.attentionRequired}
          onclick={() => {
            showAgentTerminalWindow = true;
          }}
          title={
            visibleAgentTerminal.attentionRequired
              ? `${visibleAgentTerminal.agentLabel} needs terminal input`
              : `Open ${visibleAgentTerminal.agentLabel} terminal`
          }
        >
          >_
        </button>
      {/if}
      <button class="overlay-icon-btn" class:draw-active={drawMode} onclick={() => drawMode = !drawMode} title={drawMode ? 'Exit Draw Mode' : 'Draw Annotations'}>
        ✏️
      </button>
      <button class="settings-overlay-btn" onclick={() => currentView.set('config')} title="Configuration">⚙️</button>
      <button class="settings-overlay-btn" onclick={() => currentView.set('trash')} title="Trash">🗑️</button>
      <button class="settings-overlay-btn" onclick={() => currentView.set('inventory')} title="Inventory">📦</button>
    {:else}
      <button class="settings-overlay-btn" onclick={() => currentView.set('workbench')} title="Close">×</button>
    {/if}
  </div>

  <div class="app-container">
    {#if $currentView === 'config'}
      <ConfigPanel
        bind:config={$config}
        availableModels={$availableModels}
        isLoadingModels={$isLoadingModels}
        onfetch={fetchModels}
        onsave={saveConfig}
      />
    {:else if $currentView === 'trash'}
      <DeletedModels />
    {:else if $currentView === 'inventory'}
      <InventoryPanel />
    {:else}
      <div class="workbench">
        <aside class="sidebar" style="width: {$sidebarWidth}px">
          <div class="sidebar-section flex-1" class:onboarding-highlight={$onboarding.target === 'params'}>
            <div class="pane-header">TUNABLE PARAMETERS</div>
            <div class="sidebar-content scrollable">
              <ParamPanel 
                uiSpec={effectiveUiSpec} 
                modelManifest={activeModelManifest}
                controlViews={availableControlViews}
                activeControlViewId={activeControlViewId}
                selectedTarget={selectedTarget}
                bind:searchQuery={sharedContextSearchQuery}
                onControlFocusChange={(focus) => focusedMeasurementControl = focus}
                onSelectControlView={handleSelectControlView}
                onSemanticChange={handleSemanticControlChange}
                selectedPartId={selectedPartId}
                onSelectPart={handlePartSelect}
                onspecchange={(spec, newParams) => {
                  paramPanelState.setUiSpec(spec);
                  workingCopy.patch({ uiSpec: spec });
                  if (newParams) {
                    paramPanelState.setParams(newParams);
                    workingCopy.patch({ params: newParams });
                  }
                }}
                parameters={effectiveParameters} 
                macroCode={$paramPanelState.macroCode}
                onchange={handleParamChange} 
                activeVersionId={$paramPanelState.versionId}
                messageId={$activeVersionId}
              />
            </div>
          </div>
          <div class="resizer-v" role="slider" aria-label="Resize history" aria-orientation="vertical" aria-valuenow={$historyHeight} tabindex="0" onmousedown={startResizingHistory} onkeydown={(e) => {
            if (e.key === 'ArrowUp') historyHeight.set($historyHeight + 10);
            if (e.key === 'ArrowDown') historyHeight.set($historyHeight - 10);
          }}></div>
          <div class="sidebar-section" style="height: {$historyHeight}px" class:onboarding-highlight={$onboarding.target === 'history'}>
            <div class="pane-header">THREAD HISTORY</div>
            <div class="sidebar-content scrollable">
              <HistoryPanel history={$history} activeThreadId={$activeThreadId}
                inFlightByThread={inFlightByThread}
                activeAgentSessions={activeAgentSessions}
                onSelect={loadFromHistory}
                onDelete={deleteThread}
                onRename={renameThread}
                onNew={createNewThread}
                onImportFcstd={handleImportFcstd}
                onFinalize={finalizeThread}
              />
            </div>
          </div>
        </aside>

        <div class="resizer-w" role="slider" aria-label="Resize sidebar" aria-orientation="horizontal" aria-valuenow={$sidebarWidth} tabindex="0" onmousedown={startResizingWidth} onkeydown={(e) => {
          if (e.key === 'ArrowLeft') sidebarWidth.set($sidebarWidth - 10);
          if (e.key === 'ArrowRight') sidebarWidth.set($sidebarWidth + 10);
        }}></div>

        <div class="main-workbench">
          <main
            class="viewport-area"
            role="presentation"
            bind:this={viewportAreaEl}
            class:onboarding-highlight={$onboarding.target === 'viewport'}
          >
            <div class="viewer-shell" class:viewer-shell--occluded={showBlueprintViewport}>
              <Viewer
                bind:this={viewerComponent}
                stlUrl={$activeThreadId ? stlUrl : null}
                viewerAssets={viewerAssets}
                edgeTargets={activeArtifactBundle?.edgeTargets ?? []}
                selectionTargets={contextSelectionTargets}
                selectedTarget={selectedTarget}
                searchQuery={sharedContextSearchQuery}
                persistedCameraState={persistedViewportCameraState}
                selectedPartId={selectedPartId}
                overlayPartLabel={selectedTarget?.label ?? overlaySelectedPart?.label ?? null}
                overlayPartEditable={selectedTarget?.editable ?? overlaySelectedPart?.editable ?? false}
                overlayPreviewOnly={overlayPreviewOnly}
                overlayControls={overlayControls}
                overlayAdvisories={overlayAdvisories}
                activeMeasurementCallout={activeMeasurementCallout}
                previewTransforms={importedPreviewTransforms}
                onOverlayChange={handleSemanticControlChange}
                onControlFocusChange={(focus) => focusedMeasurementControl = focus}
                onSearchQueryChange={(query) => sharedContextSearchQuery = query}
                onSelectTarget={handleTargetSelect}
                onCameraStateChange={handleVisibleViewerCameraChange}
                onModelLoaded={handleVisibleViewerLoaded}
                isGenerating={viewerBusyPhase === 'generating' || viewerBusyPhase === 'repairing'}
                hideModelWhileBusy={showViewerBusyMask}
                busyPhase={viewerBusyPhase}
                busyText={viewerBusyText}
              />
            </div>
            {#if showBlueprintViewport && effectiveConceptPreviewMessage?.imageData}
              <section class="viewport-blueprint" aria-label="Concept preview">
                <div class="viewport-blueprint__frame">
                  <div class="viewport-blueprint__header">
                    <div class="viewport-blueprint__eyebrow">CONCEPT PREVIEW</div>
                    <div class="viewport-blueprint__warning">NOT A 3D MODEL</div>
                  </div>
                  <div class="viewport-blueprint__body">
                    <div class="viewport-blueprint__image-wrap">
                      <img
                        bind:this={blueprintImageEl}
                        src={effectiveConceptPreviewMessage.imageData || ''}
                        alt="Assistant concept preview"
                        class="viewport-blueprint__image"
                      />
                    </div>
                    {#if effectiveConceptPreviewMessage.content.trim()}
                      <div class="viewport-blueprint__note">
                        {effectiveConceptPreviewMessage.content}
                      </div>
                    {/if}
                  </div>
                  <div class="viewport-blueprint__actions">
                    <button
                      class="btn btn-xs btn-secondary"
                      onclick={() => setViewportPresentationMode('model')}
                      disabled={!hasRenderableModel}
                    >
                      OPEN MODEL
                    </button>
                    <button
                      class="btn btn-xs btn-primary"
                      onclick={() => void generateFromConceptPreview()}
                      disabled={$activeThreadBusy || freecadMissing}
                    >
                      GENERATE 3D FROM THIS
                    </button>
                    <button
                      class="btn btn-xs btn-secondary"
                      onclick={pickOtherConceptPreview}
                      disabled={conceptPreviewMessages.length < 2}
                    >
                      PICK OTHER PREVIEW
                    </button>
                  </div>
                </div>
              </section>
            {/if}
            <DrawingOverlay bind:this={drawingOverlay} active={drawMode} />
            <div class="hidden-viewer-host" aria-hidden="true">
              <Viewer
                bind:this={hiddenViewerComponent}
                stlUrl={hiddenViewerSpec?.stlUrl ?? null}
                viewerAssets={hiddenViewerSpec?.viewerAssets ?? []}
                edgeTargets={[]}
                selectionTargets={[]}
                selectedTarget={null}
                searchQuery=""
                selectedPartId={null}
                overlayPartLabel={null}
                overlayPartEditable={false}
                overlayPreviewOnly={false}
                overlayControls={[]}
                overlayAdvisories={[]}
                activeMeasurementCallout={null}
                previewTransforms={{}}
                onControlFocusChange={() => { focusedMeasurementControl = null; }}
                onSearchQueryChange={() => {}}
                onSelectTarget={() => {}}
                onCameraStateChange={() => {}}
                onModelLoaded={handleHiddenViewerLoaded}
                isGenerating={false}
                hideModelWhileBusy={false}
                busyPhase={null}
                busyText={null}
              />
            </div>
            <div class="genie-layer" class:onboarding-active={$onboarding.isActive}>
              <VertexGenie 
                mode={genieMode} 
                bubble={genieBubble} 
                onDismiss={dismissGenie} 
                actions={genieActions} 
                traits={eckyTraits} 
                intensity={eckyIntensity} 
                wakeUp={genieWakeUpCount}
                agentConnected={
                  threadAgentMascot.connected ||
                  !!visibleAgentTerminal?.active ||
                  !!activePendingAgentPrompt ||
                  !!activeConfirm ||
                  !!activeViewportScreenshotChoice
                }
              />
            </div>

            {#if error}
              <div
                class="error-banner"
                role="button"
                tabindex="0"
                aria-label="Dismiss error"
                onclick={dismissError}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    dismissError();
                  }
                }}
              >
                <div class="error-banner__label">ERROR</div>
                <div class="error-banner__body">{error}</div>
                <button
                  class="error-banner__dismiss"
                  onclick={(e) => {
                    e.stopPropagation();
                    dismissError();
                  }}
                  title="Dismiss error"
                >
                  ✕
                </button>
              </div>
            {/if}

            {#if freecadMissing}
              <div class="freecad-missing-banner">
                <div class="freecad-missing-banner__label">FREECAD NOT FOUND</div>
                <div class="freecad-missing-banner__body">FreeCAD is required to generate models. Install it or set the path in <button class="freecad-missing-banner__settings-link" onclick={() => currentView.set('config')}>Settings</button>.</div>
              </div>
            {/if}

            {#if $activeThreadRequests.length > 0}
              <div class="cafeteria-strip">
                {#each $activeThreadRequests as req (req.id)}
                  <div class="microwave-unit" 
                    class:mw-active={!['success','error','canceled'].includes(req.phase)} 
                    class:mw-success={req.phase === 'success' && !req.isQuestion} 
                    class:mw-thinking-result={req.phase === 'success' && req.isQuestion}
                    class:mw-error={req.phase === 'error'} 
                    class:mw-canceled={req.phase === 'canceled'}
                    class:mw-routing={req.phase === 'classifying'}
                    onclick={() => { if (req.phase === 'success') applyCompletedRequest(req); }}
                    role="button"
                    tabindex="0"
                    onkeydown={(e) => { if (req.phase === 'success' && (e.key === 'Enter' || e.key === ' ')) applyCompletedRequest(req); }}
                    >
                    <div class="mw-glass" class:mw-pulse={req.phase === 'generating' || req.phase === 'repairing' || req.phase === 'rendering' || req.phase === 'classifying'}></div>

                    {#if req.screenshot}
                      <img src={req.screenshot} class="mw-screenshot" alt="Snapshot" />
                    {/if}

                    <div class="mw-display">
                      <div class="mw-phase">{req.isQuestion && req.phase === 'success' ? 'ADVICE' : phaseLabel(req.phase)}</div>
                      {#if req.phase === 'classifying'}
                        <div class="mw-routing-indicator">INTENT CHECK...</div>
                      {:else if req.isQuestion && req.phase === 'success'}
                        <div class="mw-advice-ready">READY</div>
                      {:else}
                        <div class="mw-timer">
                          {formatCookingTime(['success', 'error', 'canceled'].includes(req.phase) ? req.cookingElapsed : Math.max(0, $nowSeconds - Math.floor((req.cookingStartTime || Date.now()) / 1000)))}
                        </div>
                      {/if}
                      <div class="mw-prompt" title={req.prompt}>{req.prompt.slice(0, 28)}{req.prompt.length > 28 ? '…' : ''}</div>
                    </div>

                    {#if !['success', 'error', 'canceled'].includes(req.phase)}
                      <div class="mw-actions">
                        <button class="mw-btn mw-btn-cancel" onclick={(e) => { e.stopPropagation(); cancelRequest(req.id); }} title="Cancel">⏹</button>
                      </div>
                    {:else if req.phase === 'success'}
                      <div class="mw-actions">
                        <button class="mw-btn" onclick={(e) => { e.stopPropagation(); dismissRequest(req.id); }} title="Dismiss">✕</button>
                      </div>
                    {:else if req.phase === 'error' || req.phase === 'canceled'}
                      <div class="mw-actions">
                        <button class="mw-btn mw-btn-retry" onclick={(e) => { e.stopPropagation(); retryRequest(req); }} title="Retry">🔄</button>
                        <button class="mw-btn" onclick={(e) => { e.stopPropagation(); dismissRequest(req.id); }} title="Dismiss">✕</button>
                      </div>
                    {/if}
                    </div>                {/each}
              </div>
            {/if}
            

            {#if $activeThreadId && ($workingCopy.macroCode || stlUrl || effectiveConceptPreviewMessage)}
              <div class="viewport-overlay">
                {#if effectiveConceptPreviewMessage}
                  <div class="viewport-mode-panel">
                    <div class="viewport-mode-label">VIEWPORT MODE</div>
                    <div
                      class="viewport-mode-toggle"
                      class:viewport-mode-toggle--attention={blueprintAttentionVisible}
                    >
                      <button
                        class="viewport-mode-btn"
                        class:viewport-mode-btn--active={viewportPresentationMode === 'model'}
                        onclick={() => setViewportPresentationMode('model')}
                        disabled={!hasRenderableModel}
                      >
                        MODEL
                      </button>
                      <button
                        class="viewport-mode-btn"
                        class:viewport-mode-btn--active={viewportPresentationMode === 'blueprint'}
                        onclick={() => setViewportPresentationMode('blueprint')}
                      >
                        BLUEPRINT
                      </button>
                    </div>
                    {#if blueprintAttentionVisible}
                      <div class="viewport-mode-hint">CONCEPT PREVIEW READY</div>
                    {/if}
                  </div>
                {/if}
                {#if activeVersionAgentLabel}
                  <div class="agent-origin-chip" title={`Current model authored by ${activeVersionAgentLabel}`}>
                    {activeVersionAgentLabel}
                  </div>
                {/if}
                <div class="export-actions">
                  <button class="btn btn-xs btn-secondary" onclick={forkDesign} disabled={showViewerBusyMask} title="Fork this design into a new project">🍴 FORK</button>
                  <button class="btn btn-xs btn-primary" onclick={exportSTL} disabled={!stlUrl || showViewerBusyMask} title="Export as STL">💾 STL</button>
                  <button class="btn btn-xs btn-secondary" onclick={exportFCStd} disabled={!$session.artifactBundle?.fcstdPath || showViewerBusyMask} title="Export as FreeCAD document">💾 FCStd</button>
                </div>
              </div>
            {/if}
          </main>
          
          <div class="resizer-v" role="slider" aria-label="Resize dialogue" aria-orientation="vertical" aria-valuenow={$dialogueHeight} tabindex="0" onmousedown={startResizingHeight} onkeydown={(e) => {
            if (e.key === 'ArrowUp') dialogueHeight.set($dialogueHeight + 10);
            if (e.key === 'ArrowDown') dialogueHeight.set($dialogueHeight - 10);
          }}></div>

          <div class="dialogue-area" style="height: {$dialogueHeight}px" class:onboarding-highlight={$onboarding.target === 'dialogue'}>
            <div class="pane-header dialogue-header">
              DIALOGUE: {activeThread ? activeThread.title : 'New Session'}
            </div>
            <div class="dialogue-content">
              {#key $activeThreadId ?? 'new-thread'}
                <PromptPanel
                  onGenerate={handlePromptPanelSubmit}
                  isGenerating={$activeThreadBusy}
                  freecadMissing={freecadMissing}
                  dialogueState={dialogueState}
                  queuedMessages={queuedUserMessages}
                  messages={activeThread?.messages || []}
                  activeThreadId={$activeThreadId}
                  onOpenConceptPreview={openConceptPreviewInViewport}
                  onPinConceptPreview={pinConceptPreviewFromMessage}
                  pinnedConceptPreviewMessageId={activeThreadConceptPreviewState.pinnedMessageId}
                  onShowCode={(m) => { selectedCode.set(m.output.macroCode); selectedTitle.set(m.output.title); showCodeModal.set(true); }}
                  onDeleteVersion={deleteVersion}
                  bind:activeVersionId={$activeVersionId}
                  onVersionChange={loadVersion}
                />
              {/key}
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>

  {#if isBooting}
    <div class="boot-overlay">
      <div class="boot-overlay__glass"></div>
      <div class="boot-overlay__content">
        <div class="boot-overlay__title">ECKY CAD</div>
        <div class="boot-overlay__ecky">
          <VertexGenie mode="thinking" bubble="" />
        </div>
        <div class="boot-overlay__status">Restoring environment...</div>
      </div>
    </div>
  {/if}

  {#if visibleAgentTerminal}
    <Window
      bind:x={agentTerminalWindowX}
      bind:y={agentTerminalWindowY}
      bind:width={agentTerminalWindowWidth}
      bind:height={agentTerminalWindowHeight}
      minWidth={560}
      minHeight={320}
      title={`${visibleAgentTerminal.agentLabel} Terminal`}
      hidden={!showAgentTerminalWindow}
      onclose={() => {
        showAgentTerminalWindow = false;
      }}
    >
      <div class="agent-terminal-window">
        <div class="agent-terminal-window__meta">
          <div class="agent-terminal-window__status">
            {#if visibleAgentTerminal.active}
              LIVE PTY
            {:else}
              LAST SESSION
            {/if}
          </div>
          {#if activeAgentTerminalMetaSummary}
            <div class="agent-terminal-window__summary">{activeAgentTerminalMetaSummary}</div>
          {/if}
        </div>
        {#if threadAgentState?.sessionId}
          <div class="agent-terminal-window__trace-meta">
            <span>SESSION {shortSessionId(threadAgentState.sessionId)}</span>
            <span>THREAD {activeThread?.title ?? 'UNKNOWN'}</span>
            {#if threadAgentState.providerKind}
              <span>PROVIDER {threadAgentState.providerKind.toUpperCase()}</span>
            {/if}
            {#if threadAgentState.waitingOnPrompt}
              <span>WAITING ON PROMPT</span>
            {:else if threadAgentState.awaitingPromptRearm}
              <span>AWAITING PROMPT RE-ARM</span>
            {:else if activeMcpBusy}
              <span>TURN ACTIVE</span>
            {:else if latestAgentSessionTrace}
              <span>{formatAgentPhase(latestAgentSessionTrace.phase)}</span>
            {/if}
          </div>
        {/if}
        {#if activeAgentSessionTrace.length}
          <div class="agent-terminal-window__trace">
            {#each [...activeAgentSessionTrace].slice(-6).reverse() as trace (`${trace.timestamp}-${trace.kind}-${trace.summary}`)}
              <div class="agent-terminal-window__trace-entry">
                <div class="agent-terminal-window__trace-kind">{trace.kind.replace(/_/g, ' ')}</div>
                <div class="agent-terminal-window__trace-text">{trace.summary}</div>
              </div>
            {/each}
          </div>
        {/if}
        <div class="agent-terminal-window__hint">
          {#if visibleAgentTerminal.active}
            CLICK TERMINAL TO TYPE DIRECTLY. ARROWS, TAB, ESC, CTRL+C AND PASTE GO STRAIGHT TO THE PTY.
          {:else}
            LAST CAPTURED TERMINAL OUTPUT
          {/if}
        </div>
        <div
          class="agent-terminal-window__screen"
          class:agent-terminal-window__screen--live={visibleAgentTerminal.active}
          aria-label={`${visibleAgentTerminal.agentLabel} terminal`}
        >
          <AgentTerminalSurface
            bind:this={agentTerminalSurface}
            snapshot={visibleAgentTerminal}
            visible={showAgentTerminalWindow}
            onRawInput={(data) => void handleAgentTerminalRawInput(data)}
            onResize={({ cols, rows }) =>
              void handleAgentTerminalResize(visibleAgentTerminal.agentId, cols, rows)}
          />
        </div>
        <div class="agent-terminal-window__composer">
          <input
            class="input-mono agent-terminal-window__input"
            bind:value={agentTerminalInput}
            placeholder={`Paste or send a full line to ${visibleAgentTerminal.agentLabel}...`}
            disabled={!visibleAgentTerminal.active}
            onkeydown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault();
                void submitAgentTerminalInput();
              }
            }}
          />
          <button
            class="btn btn-xs btn-secondary"
            onclick={() => void submitAgentTerminalInput(true)}
            disabled={!visibleAgentTerminal.active}
            title="Send Enter"
          >
            ENTER
          </button>
          <button
            class="btn btn-xs btn-primary"
            onclick={() => void submitAgentTerminalInput()}
            disabled={!visibleAgentTerminal.active || !agentTerminalInput.length}
          >
            SEND
          </button>
        </div>
      </div>
    </Window>
  {/if}

  {#if $showCodeModal}
    <CodeModal
      bind:code={$selectedCode}
      title={$selectedTitle}
      onCommit={commitManualVersion}
      onFork={forkManualVersion}
      onclose={() => showCodeModal.set(false)}
    />
  {/if}

  {#if enrichmentManifest}
    <ImportEnrichmentModal
      manifest={enrichmentManifest}
      activeVersionId={$activeVersionId}
      onSelectPart={handlePartSelect}
      onclose={() => showEnrichmentModal = false}
      ondone={(updatedManifest) => {
        session.setModelRuntime($session.artifactBundle, updatedManifest);
        showEnrichmentModal = false;
      }}
    />
  {/if}
</div>

<style>
  .app-page { position: relative; height: 100vh; display: flex; flex-direction: column; background: var(--bg); color: var(--text); }
  .app-container { flex: 1; display: flex; flex-direction: column; min-height: 0; overflow: hidden; }
  .workbench { display: flex; height: 100%; width: 100%; overflow: hidden; }
  .sidebar { display: flex; flex-direction: column; flex-shrink: 0; background: var(--bg-100); border-right: 1px solid var(--bg-300); }
  .sidebar-content { flex: 1; min-height: 0; }
  .main-workbench { flex: 1; display: flex; flex-direction: column; min-width: 0; overflow: hidden; }
  .viewport-area { flex: 1; min-height: 100px; background: #0b0f1a; position: relative; overflow: hidden; }
  .viewer-shell {
    position: absolute;
    inset: 0;
    z-index: 5;
    transition: opacity 180ms ease, filter 180ms ease;
  }
  .viewer-shell--occluded {
    opacity: 0.08;
    filter: saturate(0.45);
    pointer-events: none;
  }
  .viewport-blueprint {
    position: absolute;
    inset: 0;
    z-index: 20;
    padding: 56px 28px 96px;
    display: flex;
    align-items: stretch;
    justify-content: stretch;
    background:
      radial-gradient(circle at 20% 18%, rgba(198, 154, 52, 0.12), transparent 28%),
      linear-gradient(rgba(255, 255, 255, 0.04) 1px, transparent 1px),
      linear-gradient(90deg, rgba(255, 255, 255, 0.04) 1px, transparent 1px),
      linear-gradient(180deg, rgba(7, 11, 19, 0.28), rgba(7, 11, 19, 0.68));
    background-size: auto, 28px 28px, 28px 28px, auto;
    pointer-events: none;
    overflow: hidden;
  }
  .viewport-blueprint__frame {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    gap: 14px;
    padding: 18px;
    border: 1px solid color-mix(in srgb, var(--primary) 48%, var(--bg-300));
    outline: 1px solid color-mix(in srgb, var(--secondary) 18%, transparent);
    outline-offset: -6px;
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--bg-100) 92%, black 8%), color-mix(in srgb, var(--bg-200) 88%, black 12%));
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.03),
      0 18px 42px rgba(0, 0, 0, 0.28);
    pointer-events: auto;
    overflow: hidden;
  }
  .viewport-blueprint__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.12em;
  }
  .viewport-blueprint__eyebrow {
    color: var(--primary);
    font-size: 0.68rem;
    font-weight: 700;
  }
  .viewport-blueprint__warning {
    color: var(--secondary);
    font-size: 0.64rem;
    font-weight: 700;
  }
  .viewport-blueprint__body {
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(220px, 320px);
    gap: 16px;
    overflow: hidden;
  }
  .viewport-blueprint__image-wrap {
    min-width: 0;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid color-mix(in srgb, var(--secondary) 26%, var(--bg-300));
    background:
      linear-gradient(rgba(255, 255, 255, 0.03) 1px, transparent 1px),
      linear-gradient(90deg, rgba(255, 255, 255, 0.03) 1px, transparent 1px),
      color-mix(in srgb, var(--bg-100) 84%, black 16%);
    background-size: 20px 20px, 20px 20px, auto;
    overflow: hidden;
  }
  .viewport-blueprint__image {
    max-width: 100%;
    max-height: 100%;
    width: auto;
    height: auto;
    object-fit: contain;
    filter: contrast(1.02) saturate(0.92);
    image-rendering: auto;
  }
  .viewport-blueprint__note {
    padding: 12px;
    border: 1px solid color-mix(in srgb, var(--primary) 24%, var(--bg-300));
    background: color-mix(in srgb, var(--bg-100) 88%, black 12%);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 0.78rem;
    line-height: 1.6;
    white-space: pre-wrap;
    overflow: auto;
  }
  .viewport-blueprint__actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
  }
  .dialogue-area { flex-shrink: 0; background: var(--bg-100); display: flex; flex-direction: column; border-top: 1px solid var(--bg-300); overflow: hidden; }
  .dialogue-content { flex: 1; min-height: 0; }
  .agent-origin-chip {
    padding: 4px 8px;
    border: 1px solid color-mix(in srgb, var(--primary) 45%, var(--bg-400));
    background: color-mix(in srgb, var(--primary) 10%, var(--bg-200));
    color: var(--primary);
    font-family: var(--font-mono);
    font-size: 0.62rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    pointer-events: none;
  }
  .pane-header { padding: 4px 12px; background: var(--bg-200); border-bottom: 1px solid var(--bg-300); color: var(--secondary); font-size: 0.6rem; font-weight: bold; letter-spacing: 0.1em; text-transform: uppercase; }
  .scrollable { overflow-y: auto; }
  .resizer-w { width: 4px; background: var(--bg-300); cursor: col-resize; z-index: 10; flex-shrink: 0; }
  .resizer-v { height: 4px; background: var(--bg-300); cursor: row-resize; z-index: 10; flex-shrink: 0; }
  .app-overlay-actions { position: absolute; top: 10px; right: 10px; z-index: 150; display: flex; gap: 8px; }
  .overlay-icon-btn, .settings-overlay-btn { width: 34px; height: 34px; background: color-mix(in srgb, var(--bg-100) 90%, transparent); border: 1px solid var(--bg-300); color: var(--text); cursor: pointer; display: flex; align-items: center; justify-content: center; box-shadow: var(--shadow); }
  .overlay-icon-btn:hover, .settings-overlay-btn:hover { border-color: var(--primary); color: var(--primary); }
  .overlay-icon-btn.draw-active { border-color: var(--primary); background: color-mix(in srgb, var(--primary) 25%, var(--bg-100)); box-shadow: 0 0 8px var(--primary); }
  .terminal-overlay-btn { font-family: var(--font-mono); font-size: 0.72rem; letter-spacing: 0.04em; }
  .terminal-overlay-btn-attention { border-color: var(--secondary); color: var(--secondary); box-shadow: 0 0 10px color-mix(in srgb, var(--secondary) 50%, transparent); }
  .genie-layer { position: absolute; left: 10px; top: 10px; z-index: 220; pointer-events: auto; max-width: min(80vw, 420px); }
  .error-banner {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 130;
    max-width: min(46vw, 560px);
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 10px;
    align-items: start;
    padding: 10px 12px;
    border: 1px solid color-mix(in srgb, var(--red) 72%, var(--bg-300));
    background: color-mix(in srgb, var(--bg-100) 88%, black 12%);
    box-shadow: var(--shadow);
    overflow: hidden;
    cursor: pointer;
  }
  .error-banner:focus-visible {
    outline: 1px solid var(--red);
    outline-offset: 1px;
  }
  .error-banner__label {
    color: var(--red);
    font-size: 0.62rem;
    font-weight: bold;
    letter-spacing: 0.12em;
  }
  .error-banner__body {
    color: var(--text);
    font-size: 0.78rem;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .error-banner__dismiss {
    border: 1px solid var(--bg-400);
    background: var(--bg-200);
    color: var(--text-dim);
    width: 24px;
    height: 24px;
    cursor: pointer;
  }
  .error-banner__dismiss:hover { border-color: var(--red); color: var(--text); }

  .freecad-missing-banner {
    position: absolute;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 135;
    max-width: min(80vw, 520px);
    padding: 8px 14px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: var(--bg-200);
    border: 1px solid var(--accent);
    pointer-events: auto;
  }
  .freecad-missing-banner__label {
    color: var(--accent);
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.12em;
  }
  .freecad-missing-banner__body {
    color: var(--text-dim);
    font-size: 0.78rem;
    line-height: 1.4;
  }
  .freecad-missing-banner__settings-link {
    background: none;
    border: none;
    color: var(--primary);
    cursor: pointer;
    padding: 0;
    font-size: inherit;
    text-decoration: underline;
  }

  /* STL Cafeteria — multi-microwave strip */
  .cafeteria-strip { position: absolute; bottom: 48px; left: 12px; right: 12px; z-index: 100; display: flex; gap: 8px; flex-wrap: wrap; pointer-events: auto; }
  .microwave-unit { position: relative; width: 180px; min-height: 72px; background: rgba(10, 14, 24, 0.88); border: 1px solid var(--bg-300); backdrop-filter: blur(8px); display: flex; flex-direction: column; overflow: hidden; transition: all 0.2s ease; }
  .microwave-unit.mw-success, .microwave-unit.mw-thinking-result { cursor: pointer; }
  .microwave-unit.mw-success:hover, .microwave-unit.mw-thinking-result:hover { background: rgba(20, 30, 45, 0.95); box-shadow: 0 0 15px rgba(74, 140, 92, 0.2); transform: translateY(-2px); }
  .microwave-unit.mw-thinking-result:hover { box-shadow: 0 0 15px rgba(139, 231, 255, 0.2); }
  .microwave-unit.mw-active { border-color: var(--primary); }
  .microwave-unit.mw-success { border-color: var(--secondary); }
  .microwave-unit.mw-thinking-result { border-color: #8be7ff; background: rgba(15, 23, 36, 0.95); }
  .microwave-unit.mw-error { border-color: var(--red); }
  .microwave-unit.mw-canceled { border-color: #444; background: rgba(15, 23, 36, 0.6); opacity: 0.75; }
  .microwave-unit.mw-routing { border-color: #4a708b; background: rgba(15, 23, 36, 0.9); }
  .mw-glass { position: absolute; inset: 0; opacity: 0; transition: opacity 0.3s; z-index: 2; pointer-events: none; }
  .mw-glass.mw-pulse { 
    animation: mw-pulse 2.5s infinite; 
    background: linear-gradient(135deg, rgba(74, 140, 92, 0.25), transparent, rgba(200, 166, 32, 0.2)); 
    opacity: 1; 
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }
  .mw-routing .mw-glass.mw-pulse, .mw-thinking-result .mw-glass.mw-pulse {
    background: linear-gradient(135deg, rgba(74, 112, 139, 0.35), transparent, rgba(139, 231, 255, 0.3)); 
    animation-duration: 4s;
  }
  @keyframes mw-pulse { 0%, 100% { opacity: 0.8; } 50% { opacity: 0.4; } }
  
  .mw-screenshot {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0.3;
    filter: grayscale(0.6) contrast(1.2);
    z-index: 1;
    pointer-events: none;
  }

  .mw-display { position: relative; z-index: 5; padding: 8px; display: flex; flex-direction: column; gap: 2px; flex: 1; }
  .mw-phase { font-size: 0.55rem; font-weight: bold; letter-spacing: 0.1em; color: var(--secondary); }
  .mw-routing .mw-phase, .mw-thinking-result .mw-phase { color: #8be7ff; text-shadow: 0 0 10px rgba(139, 231, 255, 0.4); }
  .mw-routing-indicator { font-size: 0.65rem; color: #8be7ff; font-weight: bold; margin: 4px 0; letter-spacing: 0.05em; animation: mw-routing-blink 1.5s infinite; }
  .mw-advice-ready { font-size: 1.1rem; color: #8be7ff; font-weight: bold; margin: 2px 0; text-shadow: 0 0 10px rgba(139, 231, 255, 0.6); }
  @keyframes mw-routing-blink { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
  .mw-timer { font-family: var(--font-mono); font-size: 1.1rem; font-weight: bold; color: var(--primary); text-shadow: 0 0 12px var(--primary); }
  .mw-error .mw-timer { color: var(--red); text-shadow: 0 0 12px var(--red); }
  .mw-canceled .mw-timer { color: #888; text-shadow: none; }
  .mw-success .mw-timer { color: var(--secondary); text-shadow: 0 0 12px var(--secondary); }
  .mw-prompt { font-size: 0.55rem; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mw-actions { display: flex; gap: 4px; padding: 0 8px 6px; position: relative; z-index: 1; }
  .mw-btn { background: var(--bg-300); border: 1px solid var(--bg-400); color: var(--text); font-size: 0.55rem; padding: 2px 6px; cursor: pointer; font-weight: bold; }
  .mw-btn:hover { border-color: var(--primary); color: var(--primary); }
  .mw-btn-cancel:hover { background: var(--red); color: white; border-color: var(--red); }
  .hidden-viewer-host {
    position: fixed;
    left: -200vw;
    top: 0;
    width: 1024px;
    height: 768px;
    pointer-events: none;
    visibility: hidden;
    overflow: hidden;
  }
  .viewport-overlay { position: absolute; bottom: 12px; right: 12px; background: rgba(11, 15, 26, 0.6); backdrop-filter: blur(4px); padding: 8px; border: 1px solid var(--bg-300); z-index: 50; display: flex; flex-direction: column; align-items: flex-end; gap: 8px; }
  .viewport-mode-panel {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
    min-width: 210px;
  }
  .viewport-mode-label {
    color: var(--secondary);
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }
  .viewport-mode-toggle {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 4px;
    padding: 4px;
    border: 1px solid var(--bg-300);
    background: color-mix(in srgb, var(--bg-100) 92%, transparent);
  }
  .viewport-mode-toggle--attention {
    border-color: var(--secondary);
    box-shadow: 0 0 12px color-mix(in srgb, var(--secondary) 26%, transparent);
  }
  .viewport-mode-btn {
    min-width: 0;
    padding: 6px 8px;
    border: 1px solid var(--bg-400);
    background: var(--bg-200);
    color: var(--text-dim);
    font-family: var(--font-mono);
    font-size: 0.66rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .viewport-mode-btn:hover:not(:disabled) {
    border-color: var(--primary);
    color: var(--primary);
  }
  .viewport-mode-btn--active {
    border-color: var(--primary);
    background: color-mix(in srgb, var(--primary) 18%, var(--bg-200));
    color: var(--primary);
  }
  .viewport-mode-btn:disabled {
    cursor: default;
    opacity: 0.5;
  }
  .viewport-mode-hint {
    color: var(--secondary);
    font-size: 0.62rem;
    font-family: var(--font-mono);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    text-align: right;
  }
  .export-actions { display: flex; gap: 4px; }
  @media (max-width: 960px) {
    .viewport-blueprint {
      padding: 48px 16px 96px;
    }
    .viewport-blueprint__body {
      grid-template-columns: minmax(0, 1fr);
    }
    .viewport-blueprint__note {
      max-height: 180px;
    }
    .viewport-overlay {
      left: 12px;
      right: 12px;
      bottom: 12px;
      align-items: stretch;
    }
    .viewport-mode-panel {
      min-width: 0;
    }
    .viewport-mode-hint {
      text-align: left;
    }
  }
  .boot-overlay { position: absolute; inset: 0; z-index: 300; display: flex; align-items: center; justify-content: center; background: var(--bg); }
  .boot-overlay__glass { position: absolute; inset: 0; background: radial-gradient(circle, rgba(74, 140, 92, 0.16), transparent), rgba(8, 12, 20, 0.86); backdrop-filter: blur(18px); }
  .boot-overlay__content { position: relative; z-index: 1; display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 20px; }
  .boot-overlay__title { color: var(--secondary); font-weight: bold; letter-spacing: 0.2em; }
  .boot-overlay__status { color: var(--text-dim); font-size: 0.7rem; }
  .agent-terminal-window { display: flex; flex-direction: column; height: 100%; background: linear-gradient(180deg, color-mix(in srgb, var(--bg-100) 92%, #071019 8%), var(--bg)); overflow: hidden; }
  .agent-terminal-window__meta { display: flex; flex-direction: column; gap: 6px; padding: 10px 12px; border-bottom: 1px solid var(--bg-300); background: color-mix(in srgb, var(--bg-200) 88%, transparent); }
  .agent-terminal-window__status { font-family: var(--font-mono); font-size: 0.65rem; letter-spacing: 0.14em; color: var(--secondary); text-transform: uppercase; }
  .agent-terminal-window__summary { font-size: 0.76rem; color: var(--text-dim); }
  .agent-terminal-window__trace-meta { display: flex; flex-wrap: wrap; gap: 8px 14px; padding: 8px 12px; border-bottom: 1px solid var(--bg-300); background: color-mix(in srgb, var(--bg-200) 80%, transparent); font-family: var(--font-mono); font-size: 0.62rem; letter-spacing: 0.1em; text-transform: uppercase; color: var(--text-dim); }
  .agent-terminal-window__trace { display: flex; flex-direction: column; gap: 6px; max-height: 132px; overflow: auto; padding: 10px 12px; border-bottom: 1px solid var(--bg-300); background: color-mix(in srgb, var(--bg-100) 90%, #071019 10%); }
  .agent-terminal-window__trace-entry { display: grid; grid-template-columns: minmax(108px, auto) minmax(0, 1fr); gap: 10px; align-items: start; font-size: 0.72rem; }
  .agent-terminal-window__trace-kind { font-family: var(--font-mono); letter-spacing: 0.08em; text-transform: uppercase; color: var(--secondary); }
  .agent-terminal-window__trace-text { min-width: 0; color: var(--text-dim); line-height: 1.4; }
  .agent-terminal-window__hint { padding: 8px 12px; border-bottom: 1px solid var(--bg-300); font-family: var(--font-mono); font-size: 0.64rem; letter-spacing: 0.08em; color: color-mix(in srgb, var(--secondary) 88%, #d9e8c9 12%); background: color-mix(in srgb, var(--bg-200) 72%, #071019 28%); text-transform: uppercase; }
  .agent-terminal-window__screen { flex: 1; min-height: 0; min-width: 0; overflow: hidden; background:
      radial-gradient(circle at top, color-mix(in srgb, var(--primary) 10%, transparent), transparent 42%),
      linear-gradient(180deg, rgba(6, 11, 17, 0.96), rgba(3, 8, 14, 0.98)); }
  .agent-terminal-window__screen--live { cursor: text; }
  .agent-terminal-window__composer { display: flex; gap: 8px; padding: 10px 12px; border-top: 1px solid var(--bg-300); background: color-mix(in srgb, var(--bg-200) 84%, transparent); }
  .agent-terminal-window__input { flex: 1; min-width: 0; }
  .flex-1 { flex: 1; }
  .sidebar-section { display: flex; flex-direction: column; min-height: 0; overflow: hidden; }

  /* Onboarding */
  .onboarding-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    z-index: 999;
    pointer-events: all;
  }
  :global(.onboarding-highlight) {
    position: relative !important;
    z-index: 1000 !important;
    box-shadow: 0 0 0 2px var(--primary), 0 0 40px rgba(74, 140, 92, 0.5) !important;
    pointer-events: none;
    background: var(--bg-100);
  }
  :global(.genie-layer.onboarding-active) {
    z-index: 1001 !important;
  }

  /* Agent confirmation stack */
</style>
