import { writable, get } from 'svelte/store';
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

function createSessionStore() {
  const { subscribe, set, update } = writable({
    phase: 'booting' as SessionPhase,
    status: 'System ready.',
    error: null as SessionError | null,
    globalError: null as SessionError | null,
    stlUrl: null as string | null,
    runtimeRevision: 0,
    artifactBundle: null as ArtifactBundle | null,
    modelManifest: null as ModelManifest | null,
    selectedPartId: null as string | null,
    repairMessage: '' as string,
    cookingPhrase: '' as string,
  });

  return {
    subscribe,
    set,
    update,
    setPhase: (p: SessionPhase) => update(s => ({ ...s, phase: p })),
    setStatus: (msg: string) => update(s => ({ ...s, status: msg })),
    setError: (err: SessionError | null) => update(s => ({ ...s, error: err })),
    setGlobalError: (err: SessionError | null) => update(s => ({ ...s, globalError: err })),
    setStlUrl: (url: string | null) =>
      update(s => {
        if (s.stlUrl === url) return s;
        return {
          ...s,
          stlUrl: url,
          runtimeRevision: s.runtimeRevision + 1,
          artifactBundle: url ? s.artifactBundle : null,
          modelManifest: url ? s.modelManifest : null,
          selectedPartId: url ? s.selectedPartId : null,
        };
      }),
    reloadStlUrl: (url: string) =>
      update(s => ({
        ...s,
        stlUrl: url,
        runtimeRevision: s.runtimeRevision + 1,
      })),
    setModelRuntime: (bundle: ArtifactBundle | null, manifest: ModelManifest | null) =>
      update(s => {
        const selectedPartId =
          s.selectedPartId && manifest?.parts?.some((part) => part.partId === s.selectedPartId)
            ? s.selectedPartId
            : null;
        return {
          ...s,
          artifactBundle: bundle,
          modelManifest: manifest,
          selectedPartId,
        };
      }),
    setSelectedPartId: (partId: string | null) => update(s => ({ ...s, selectedPartId: partId })),
    clearModelRuntime: () =>
      update(s => ({ ...s, artifactBundle: null, modelManifest: null, selectedPartId: null })),
    setRepairMessage: (msg: string) => update(s => ({ ...s, repairMessage: msg })),
    setCookingPhrase: (msg: string) => update(s => ({ ...s, cookingPhrase: msg })),
  };
}

export const session = createSessionStore();

// Convenience accessors (backward compat for App.svelte)
export const phase = { subscribe: (fn: (value: SessionPhase) => void) => session.subscribe(s => fn(s.phase)), set: session.setPhase };
export const status = { subscribe: (fn: (value: string) => void) => session.subscribe(s => fn(s.status)), set: session.setStatus };
export const error = { subscribe: (fn: (value: SessionError | null) => void) => session.subscribe(s => fn(s.error)), set: session.setError };
export const stlUrl = { subscribe: (fn: (value: string | null) => void) => session.subscribe(s => fn(s.stlUrl)), set: session.setStlUrl };
export const artifactBundle = { subscribe: (fn: (value: ArtifactBundle | null) => void) => session.subscribe(s => fn(s.artifactBundle)) };
export const modelManifest = { subscribe: (fn: (value: ModelManifest | null) => void) => session.subscribe(s => fn(s.modelManifest)) };
export const selectedPartId = {
  subscribe: (fn: (value: string | null) => void) => session.subscribe(s => fn(s.selectedPartId)),
  set: session.setSelectedPartId,
};
/**
 * Derives session.phase from aggregate request queue state.
 * This is a pure projection of requestQueue state.
 */
export function syncSessionPhaseFromQueue() {
  const q = get(requestQueue);
  const requests = Object.values(q.byId);
  const phases = requests.map(r => r.phase);
  const currentSession = get(session);

  let newPhase: SessionPhase = 'idle';

  if (currentSession.phase === 'booting') {
    newPhase = 'booting';
  } else if (phases.some(p => p === 'rendering' || p === 'queued_for_render' || p === 'committing')) {
    newPhase = 'rendering';
  } else if (phases.some(p => p === 'repairing')) {
    newPhase = 'repairing';
  } else if (phases.some(p => p === 'generating')) {
    newPhase = 'generating';
  } else if (phases.some(p => p === 'answering')) {
    newPhase = 'answering';
  } else if (phases.some(p => p === 'classifying')) {
    newPhase = 'classifying';
  } else {
    newPhase = 'idle';
  }

  session.update(s => ({ 
    ...s, 
    phase: newPhase,
  }));
}

// Automatically keep session phase in sync with the request queue
requestQueue.subscribe(() => {
  syncSessionPhaseFromQueue();
});
