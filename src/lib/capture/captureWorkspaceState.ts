import type { ArtifactBundle, ModelManifest } from '../types/domain';
import type { CaptureCropBounds, CaptureMeshPreview } from '../tauri/contracts';

export type CaptureWorkspaceState =
  | { phase: 'pairing'; sessionToken: string; runId: string; pairingUrl: string; trustUrl: string; guidance: string; cameraStatus: string; acceptedFrameCount: number; meshPreview: null; preview: null; error: null }
  | { phase: 'capturing'; sessionToken: string; runId: string; pairingUrl: string; trustUrl: string; guidance: string; cameraStatus: string; acceptedFrameCount: number; meshPreview: null; preview: null; error: null }
  | { phase: 'reconstructing'; sessionToken: string; runId: string; pairingUrl: string; trustUrl: string; guidance: string; cameraStatus: string; acceptedFrameCount: number; progress: number; meshPreview: null; preview: null; error: null }
  | { phase: 'preview'; sessionToken: string; runId: string; pairingUrl: string; trustUrl: string; guidance: string; cameraStatus: string; acceptedFrameCount: number; meshPreview: CaptureMeshPreview; preview: { bundle: ArtifactBundle; manifest: ModelManifest; applied: boolean; cropBounds: CaptureCropBounds | null } | null; error: null }
  | { phase: 'failed'; sessionToken: string; runId: string; guidance: string; cameraStatus: string; error: string; acceptedFrameCount: number; meshPreview: CaptureMeshPreview | null; preview: null }
  | { phase: 'cancelled'; sessionToken: ''; runId: ''; guidance: string; cameraStatus: string; error: null; acceptedFrameCount: 0; meshPreview: null; preview: null };
export type CaptureWorkspacePreview = { bundle: ArtifactBundle; manifest: ModelManifest; applied: boolean; cropBounds: CaptureCropBounds | null };

export type CaptureWorkspaceAction =
  | { type: 'start'; sessionToken: string; runId: string; pairingUrl: string; trustUrl: string }
  | { type: 'capture'; acceptedFrameCount: number; guidance?: string; cameraStatus?: string }
  | { type: 'reconstruct'; progress?: number; acceptedFrameCount?: number }
  | { type: 'preview'; meshPreview: CaptureMeshPreview; acceptedFrameCount?: number }
  | { type: 'previewPrepared'; bundle: ArtifactBundle; manifest: ModelManifest }
  | { type: 'fail'; error: string }
  | { type: 'resume' }
  | { type: 'cancel' }
  | { type: 'patch'; patch: Partial<{ phase: CaptureWorkspaceState['phase']; sessionToken: string; runId: string; pairingUrl: string; trustUrl: string; guidance: string; cameraStatus: string; acceptedFrameCount: number; progress: number; meshPreview: CaptureMeshPreview | null; preview: CaptureWorkspacePreview | null; error: string | null }> };

export function createCaptureWorkspaceState(): CaptureWorkspaceState {
  return { phase: 'pairing', sessionToken: '', runId: '', pairingUrl: 'No pairing session yet', trustUrl: '', guidance: 'PAIR PHONE', cameraStatus: 'Camera permission pending', acceptedFrameCount: 0, meshPreview: null, preview: null, error: null };
}

function base(state: CaptureWorkspaceState) {
  return { sessionToken: state.sessionToken, runId: state.runId, pairingUrl: 'pairingUrl' in state ? state.pairingUrl : 'No pairing session yet', trustUrl: 'trustUrl' in state ? state.trustUrl : '', acceptedFrameCount: state.acceptedFrameCount, guidance: state.guidance, cameraStatus: state.cameraStatus };
}

function canonicalizePatchedState(
  state: CaptureWorkspaceState,
  patch: Extract<CaptureWorkspaceAction, { type: 'patch' }>['patch'],
): CaptureWorkspaceState {
  const raw = { ...state, ...patch };
  const common = base(raw as CaptureWorkspaceState);
  switch (raw.phase) {
    case 'pairing':
      return { ...common, phase: 'pairing', meshPreview: null, preview: null, error: null };
    case 'capturing':
      return { ...common, phase: 'capturing', meshPreview: null, preview: null, error: null };
    case 'reconstructing':
      return {
        ...common,
        phase: 'reconstructing',
        progress: Math.max(0, Math.min(1, 'progress' in raw && typeof raw.progress === 'number' ? raw.progress : 0)),
        meshPreview: null,
        preview: null,
        error: null,
      };
    case 'preview':
      if (!raw.meshPreview) throw new Error('Capture preview requires mesh');
      return {
        ...common,
        phase: 'preview',
        meshPreview: raw.meshPreview,
        preview: raw.preview ?? null,
        error: null,
      };
    case 'failed': {
      const error = raw.error?.trim();
      if (!error) throw new Error('Failed capture requires error');
      return {
        phase: 'failed',
        sessionToken: raw.sessionToken,
        runId: raw.runId,
        guidance: raw.guidance,
        cameraStatus: raw.cameraStatus,
        error,
        acceptedFrameCount: raw.acceptedFrameCount,
        meshPreview: raw.meshPreview,
        preview: null,
      };
    }
    case 'cancelled':
      return reduceCaptureWorkspace(state, { type: 'cancel' });
  }
}

export function reduceCaptureWorkspace(state: CaptureWorkspaceState, action: CaptureWorkspaceAction): CaptureWorkspaceState {
  if (action.type === 'patch') {
    return canonicalizePatchedState(state, action.patch);
  }
  if (action.type === 'start') return { phase: 'capturing', sessionToken: action.sessionToken, runId: action.runId, pairingUrl: action.pairingUrl, trustUrl: action.trustUrl, guidance: 'OPEN LINK ON PHONE', cameraStatus: 'Waiting for phone camera', acceptedFrameCount: 0, meshPreview: null, preview: null, error: null };
  if (action.type === 'capture' && (state.phase === 'capturing' || state.phase === 'pairing')) return { ...state, phase: 'capturing', acceptedFrameCount: action.acceptedFrameCount, guidance: action.guidance ?? state.guidance, cameraStatus: action.cameraStatus ?? state.cameraStatus, error: null };
  if (action.type === 'reconstruct' && (state.phase === 'capturing' || state.phase === 'preview' || state.phase === 'reconstructing')) return { ...base(state), phase: 'reconstructing', progress: action.progress ?? 0, meshPreview: null, preview: null, error: null };
  if (action.type === 'preview' && (state.phase === 'reconstructing' || state.phase === 'capturing')) return { ...base(state), phase: 'preview', meshPreview: action.meshPreview, preview: null, error: null, guidance: 'PREPARING PREVIEW' };
  if (action.type === 'previewPrepared' && state.phase === 'preview') return { ...state, preview: { bundle: action.bundle, manifest: action.manifest, applied: false, cropBounds: null }, guidance: 'INSPECT MESH', cameraStatus: 'Preview ready inside Capture window' };
  if (action.type === 'fail' && state.phase !== 'cancelled') return { phase: 'failed', sessionToken: state.sessionToken, runId: state.runId, guidance: 'CAPTURE FAILED', cameraStatus: action.error, error: action.error, acceptedFrameCount: state.acceptedFrameCount, meshPreview: state.meshPreview, preview: null };
  if (action.type === 'resume' && (state.phase === 'failed' || state.phase === 'cancelled')) return createCaptureWorkspaceState();
  if (action.type === 'cancel') return { phase: 'cancelled', sessionToken: '', runId: '', guidance: 'PAIR PHONE', cameraStatus: 'Session cancelled', error: null, acceptedFrameCount: 0, meshPreview: null, preview: null };
  throw new Error(`Invalid capture transition: ${state.phase} -> ${action.type}`);
}
