import { get, writable, type Readable } from 'svelte/store';
import { requestQueue } from './requestQueue';
import type { ArtifactBundle, ModelManifest } from '../types/domain';
import type { AppError } from '../tauri/contracts';

export type SessionError = string | AppError;

export type SessionPhase =
  | 'booting'
  | 'idle'
  | 'classifying'
  | 'answering'
  | 'generating'
  | 'rendering'
  | 'repairing'
  | 'error';

export type SessionRuntime =
  | { kind: 'empty' }
  | { kind: 'geometryOnly'; stlUrl: string }
  | { kind: 'artifact'; stlUrl: string; artifactBundle: ArtifactBundle }
  | {
      kind: 'model';
      stlUrl: string;
      artifactBundle: ArtifactBundle;
      modelManifest: ModelManifest;
      selectedPartId: string | null;
    };

interface SessionCoreState {
  phase: SessionPhase;
  status: string;
  error: SessionError | null;
  globalError: SessionError | null;
  runtime: SessionRuntime;
  runtimeRevision: number;
  repairMessage: string;
  cookingPhrase: string;
}

export interface SessionState extends SessionCoreState {
  stlUrl: string | null;
  artifactBundle: ArtifactBundle | null;
  modelManifest: ModelManifest | null;
  selectedPartId: string | null;
}

function projectRuntime(runtime: SessionRuntime) {
  switch (runtime.kind) {
    case 'empty':
      return {
        stlUrl: null,
        artifactBundle: null,
        modelManifest: null,
        selectedPartId: null,
      };
    case 'geometryOnly':
      return {
        stlUrl: runtime.stlUrl,
        artifactBundle: null,
        modelManifest: null,
        selectedPartId: null,
      };
    case 'artifact':
      return {
        stlUrl: runtime.stlUrl,
        artifactBundle: runtime.artifactBundle,
        modelManifest: null,
        selectedPartId: null,
      };
    case 'model':
      return {
        stlUrl: runtime.stlUrl,
        artifactBundle: runtime.artifactBundle,
        modelManifest: runtime.modelManifest,
        selectedPartId: runtime.selectedPartId,
      };
  }
}

function projectState(state: SessionCoreState): SessionState {
  return Object.freeze({ ...state, ...projectRuntime(state.runtime) });
}

function requireStlUrl(url: string): string {
  if (!url.trim()) throw new Error('session runtime requires non-empty stlUrl');
  return url;
}

function validateRuntime(runtime: SessionRuntime): SessionRuntime {
  if (runtime.kind === 'empty') return runtime;
  requireStlUrl(runtime.stlUrl);

  if (runtime.kind === 'model') {
    if (runtime.artifactBundle.modelId !== runtime.modelManifest.modelId) {
      throw new Error('session runtime modelId mismatch');
    }
    if (
      runtime.selectedPartId !== null
      && !(runtime.modelManifest.parts ?? []).some((part) => part.partId === runtime.selectedPartId)
    ) {
      throw new Error(`session runtime selected part not found: ${runtime.selectedPartId}`);
    }
  }

  return runtime;
}

function createSessionStore(): Readable<SessionState> & {
  setPhase: (phase: SessionPhase) => void;
  setStatus: (status: string) => void;
  setError: (error: SessionError | null) => void;
  setGlobalError: (error: SessionError | null) => void;
  setRuntime: (runtime: SessionRuntime) => void;
  clearRuntime: () => void;
  setStlUrl: (url: string | null) => void;
  reloadStlUrl: (url: string) => void;
  setModelRuntime: (bundle: ArtifactBundle | null, manifest: ModelManifest | null) => void;
  setSelectedPartId: (partId: string | null) => void;
  clearModelRuntime: () => void;
  setRepairMessage: (message: string) => void;
  setCookingPhrase: (message: string) => void;
} {
  const core = writable<SessionCoreState>({
    phase: 'booting',
    status: 'System ready.',
    error: null,
    globalError: null,
    runtime: { kind: 'empty' },
    runtimeRevision: 0,
    repairMessage: '',
    cookingPhrase: '',
  });

  const subscribe: Readable<SessionState>['subscribe'] = (run, invalidate) =>
    core.subscribe((state) => run(projectState(state)), invalidate);

  const replaceRuntime = (runtime: SessionRuntime, forceRevision = false) => {
    const valid = validateRuntime(runtime);
    core.update((state) => {
      if (!forceRevision && state.runtime === valid) return state;
      return { ...state, runtime: valid, runtimeRevision: state.runtimeRevision + 1 };
    });
  };

  return {
    subscribe,
    setPhase: (phase) => core.update((state) => ({ ...state, phase })),
    setStatus: (status) => core.update((state) => ({ ...state, status })),
    setError: (error) => core.update((state) => ({ ...state, error })),
    setGlobalError: (globalError) => core.update((state) => ({ ...state, globalError })),
    setRuntime: (runtime) => replaceRuntime(runtime),
    clearRuntime: () => replaceRuntime({ kind: 'empty' }),
    setStlUrl: (url) => {
      if (url === null) {
        replaceRuntime({ kind: 'empty' });
        return;
      }
      const current = get(core).runtime;
      if (current.kind !== 'empty' && current.stlUrl === url) return;
      replaceRuntime({ kind: 'geometryOnly', stlUrl: requireStlUrl(url) });
    },
    reloadStlUrl: (url) => {
      const current = get(core).runtime;
      const stlUrl = requireStlUrl(url);
      replaceRuntime(
        current.kind === 'empty' ? { kind: 'geometryOnly', stlUrl } : { ...current, stlUrl },
        true,
      );
    },
    setModelRuntime: (artifactBundle, modelManifest) => {
      if (artifactBundle === null) {
        if (modelManifest !== null) throw new Error('model manifest requires artifact bundle');
        const current = get(core).runtime;
        replaceRuntime(
          current.kind === 'empty' ? current : { kind: 'geometryOnly', stlUrl: current.stlUrl },
        );
        return;
      }

      const current = get(core).runtime;
      const stlUrl = current.kind === 'empty' ? artifactBundle.modelStlPath : current.stlUrl;
      replaceRuntime(
        modelManifest === null
          ? { kind: 'artifact', stlUrl, artifactBundle }
          : {
              kind: 'model',
              stlUrl,
              artifactBundle,
              modelManifest,
              selectedPartId:
                current.kind === 'model'
                && current.selectedPartId !== null
                && (modelManifest.parts ?? []).some((part) => part.partId === current.selectedPartId)
                  ? current.selectedPartId
                  : null,
            },
      );
    },
    setSelectedPartId: (selectedPartId) => {
      const current = get(core).runtime;
      if (current.kind !== 'model') {
        if (selectedPartId !== null) throw new Error('selected part requires loaded model runtime');
        return;
      }
      replaceRuntime(validateRuntime({ ...current, selectedPartId }));
    },
    clearModelRuntime: () => {
      const current = get(core).runtime;
      replaceRuntime(
        current.kind === 'empty' ? current : { kind: 'geometryOnly', stlUrl: current.stlUrl },
      );
    },
    setRepairMessage: (repairMessage) => core.update((state) => ({ ...state, repairMessage })),
    setCookingPhrase: (cookingPhrase) => core.update((state) => ({ ...state, cookingPhrase })),
  };
}

export const session = createSessionStore();

// Compatibility projections. Runtime aggregate remains sole authority.
export const phase = {
  subscribe: (run: (value: SessionPhase) => void) => session.subscribe((state) => run(state.phase)),
  set: session.setPhase,
};
export const status = {
  subscribe: (run: (value: string) => void) => session.subscribe((state) => run(state.status)),
  set: session.setStatus,
};
export const error = {
  subscribe: (run: (value: SessionError | null) => void) => session.subscribe((state) => run(state.error)),
  set: session.setError,
};
export const stlUrl = {
  subscribe: (run: (value: string | null) => void) => session.subscribe((state) => run(state.stlUrl)),
  set: session.setStlUrl,
};
export const artifactBundle = {
  subscribe: (run: (value: ArtifactBundle | null) => void) => session.subscribe((state) => run(state.artifactBundle)),
};
export const modelManifest = {
  subscribe: (run: (value: ModelManifest | null) => void) => session.subscribe((state) => run(state.modelManifest)),
};
export const selectedPartId = {
  subscribe: (run: (value: string | null) => void) => session.subscribe((state) => run(state.selectedPartId)),
  set: session.setSelectedPartId,
};

/** Derives session.phase from aggregate request queue state. */
export function syncSessionPhaseFromQueue() {
  const requests = Object.values(get(requestQueue).byId);
  const phases = requests.map((request) => request.phase);
  const currentSession = get(session);

  let newPhase: SessionPhase = 'idle';
  if (currentSession.phase === 'booting') newPhase = 'booting';
  else if (phases.some((value) => value === 'rendering' || value === 'queued_for_render' || value === 'committing')) newPhase = 'rendering';
  else if (phases.some((value) => value === 'repairing')) newPhase = 'repairing';
  else if (phases.some((value) => value === 'generating')) newPhase = 'generating';
  else if (phases.some((value) => value === 'answering')) newPhase = 'answering';
  else if (phases.some((value) => value === 'classifying')) newPhase = 'classifying';

  session.setPhase(newPhase);
}

requestQueue.subscribe(syncSessionPhaseFromQueue);
