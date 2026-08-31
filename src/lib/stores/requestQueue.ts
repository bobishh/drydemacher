import { writable, derived, get } from 'svelte/store';
import { estimateBase64Bytes, profileLog } from '../debug/profiler';
import type { Attachment, Request, RequestPatch, RequestPhase } from '../types/domain';

export interface QueuedRequest extends Request {}

type RequestQueueState = {
  byId: Record<string, QueuedRequest>;
  order: string[];
  activeId: string | null;
};

const TERMINAL_PHASES: RequestPhase[] = ['success', 'error', 'canceled'];
const MODEL_ACTIVE_PHASES: RequestPhase[] = [
  'generating',
  'repairing',
  'queued_for_render',
  'rendering',
  'committing',
];

function isTerminalPhase(phase: RequestPhase): boolean {
  return TERMINAL_PHASES.includes(phase);
}

function isModelActivePhase(phase: RequestPhase): boolean {
  return MODEL_ACTIVE_PHASES.includes(phase);
}

/** Apply one lifecycle-safe patch. Exported for focused unit tests. */
export function applyRequestPatch<T extends Request>(
  existing: T,
  changes: RequestPatch,
  now = Date.now(),
): T {
  const nextPhase = 'phase' in changes ? changes.phase : undefined;
  if (nextPhase && isTerminalPhase(existing.phase) && nextPhase !== existing.phase) {
    throw new Error(`Cannot transition terminal request ${existing.id} from ${existing.phase} to ${nextPhase}`);
  }
  if (nextPhase === 'success' && (!('result' in changes) || !changes.result)) {
    throw new Error('Successful request requires result payload');
  }
  if (nextPhase === 'error' && (!('error' in changes) || !changes.error?.trim())) {
    throw new Error('Failed request requires error payload');
  }
  if (nextPhase === 'success' && 'error' in changes && changes.error != null) {
    throw new Error('Successful request cannot carry error payload');
  }
  if (nextPhase === 'error' && 'result' in changes && changes.result != null) {
    throw new Error('Failed request cannot carry result payload');
  }
  const merged = { ...existing, ...changes } as T;
  if (nextPhase === 'success') merged.error = null;
  if (nextPhase === 'error') merged.result = null;
  if (nextPhase === 'canceled') {
    merged.result = null;
    merged.error = null;
  }
  if (merged.phase === 'success' && !merged.result) {
    throw new Error('Successful request requires result payload');
  }
  if (merged.phase === 'success' && merged.error !== null) {
    throw new Error('Successful request cannot carry error payload');
  }
  if (merged.phase === 'error' && merged.result !== null) {
    throw new Error('Failed request cannot carry result payload');
  }
  if (merged.phase === 'error' && !merged.error?.trim()) {
    throw new Error('Failed request requires error payload');
  }
  if (merged.phase === 'canceled' && (merged.result !== null || merged.error !== null)) {
    throw new Error('Canceled request cannot carry terminal payload');
  }
  if (nextPhase && isTerminalPhase(nextPhase) && merged.cookingStartTime && !changes.cookingElapsed) {
    merged.cookingElapsed = Math.max(0, Math.floor(now / 1000) - Math.floor(merged.cookingStartTime / 1000));
  }
  return merged;
}

function queueStats(byId: Record<string, QueuedRequest>) {
  const requests = Object.values(byId);
  const terminal = requests.filter(r => isTerminalPhase(r.phase)).length;
  const active = requests.length - terminal;
  const screenshotBytes = requests.reduce((sum, r) => sum + estimateBase64Bytes(r.screenshot), 0);
  return {
    requests: requests.length,
    active,
    terminal,
    screenshotMb: Number((screenshotBytes / (1024 * 1024)).toFixed(2)),
  };
}

function createRequestQueue() {
  const { subscribe, set, update } = writable<RequestQueueState>({
    byId: {},
    order: [],
    activeId: null,
  });

  return {
    subscribe,

    submit(
      prompt: string,
      attachments: Attachment[] = [],
      threadId: string | null = null,
      baseMessageId: string | null = null,
      baseModelId: string | null = null,
      buildMode: "interactive" | "controller" = "interactive",
    ): string {
      const id = `req-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
      const request: QueuedRequest = {
        id,
        prompt,
        attachments,
        createdAt: Date.now(),
        phase: 'classifying',
        attempt: 0,
        maxAttempts: 0,
        maxVerifyAttempts: 0,
        isQuestion: false,
        lightResponse: '',
        screenshot: null,
        result: null,
        error: null,
        cookingStartTime: null,
        cookingElapsed: 0,
        threadId,
        baseMessageId,
        baseModelId,
        buildMode,
        buildQueueState: "pending",
      };
      update(q => ({
        ...q,
        byId: { ...q.byId, [id]: request },
        order: [...q.order, id],
        activeId: id,
      }));
      const snapshot = get(requestQueue);
      profileLog('queue.submit', {
        requestId: id,
        threadId,
        ...queueStats(snapshot.byId),
      });
      return id;
    },

    patch(id: string, changes: RequestPatch) {
      update(q => {
        const existing = q.byId[id];
        if (!existing) return q;
        const merged = applyRequestPatch(existing, changes);
        if (isTerminalPhase(merged.phase)) merged.buildQueueState = "finished";
        const next: RequestQueueState = {
          ...q,
          byId: { ...q.byId, [id]: merged },
        };
        const patchedPhase = 'phase' in changes ? changes.phase : undefined;
        if (patchedPhase && (patchedPhase !== existing.phase || isTerminalPhase(patchedPhase))) {
          profileLog('queue.phase', {
            requestId: id,
            from: existing.phase,
            to: patchedPhase,
            ...queueStats(next.byId),
          });
        }
        return next;
      });
    },

    setActive(id: string | null) {
      update(q => ({ ...q, activeId: id }));
    },

    cancel(id: string) {
      update(q => {
        const existing = q.byId[id];
        if (!existing || isTerminalPhase(existing.phase)) return q;
        const canceledRequest = applyRequestPatch(existing, { phase: 'canceled' });
        const next: RequestQueueState = {
          ...q,
          byId: { ...q.byId, [id]: canceledRequest },
        };
        profileLog('queue.cancel', {
          requestId: id,
          ...queueStats(next.byId),
        });
        return next;
      });
    },

    remove(id: string) {
      update(q => {
        const { [id]: _, ...rest } = q.byId;
        const next = {
          byId: rest,
          order: q.order.filter(x => x !== id),
          activeId: q.activeId === id ? (q.order.find(x => x !== id) || null) : q.activeId,
        };
        profileLog('queue.remove', {
          requestId: id,
          ...queueStats(next.byId),
        });
        return next;
      });
    },

    clear() {
      set({ byId: {}, order: [], activeId: null });
    },
  };
}

export const requestQueue = createRequestQueue();

// Derived stores for UI

// All requests in submission order (for the cafeteria strip)
export const allRequests = derived(requestQueue, $q =>
  $q.order.map(id => $q.byId[id]).filter(Boolean)
);

// Requests belonging to the currently active thread
export const activeThreadRequests = derived(
  [requestQueue, activeThreadId],
  ([$q, $tid]) => {
    return $q.order
      .map(id => $q.byId[id])
      .filter(r => r && r.threadId === $tid);
  }
);

// Only in-flight requests
export const activeRequests = derived(requestQueue, $q => 
  $q.order.map(id => $q.byId[id]).filter(r => r && !['success', 'error', 'canceled'].includes(r.phase))
);

export const activeRequestCount = derived(activeRequests, $r => $r.length);

export const llmInFlightCount = derived(requestQueue, $q =>
  Object.values($q.byId).filter(r => r.phase === 'classifying' || r.phase === 'generating').length
);

export const renderQueueCount = derived(requestQueue, $q =>
  Object.values($q.byId).filter(r => r.phase === 'queued_for_render' || r.phase === 'rendering').length
);

export const completedRequests = derived(requestQueue, $q =>
  $q.order.map(id => $q.byId[id]).filter(r => r && r.phase === 'success')
);

export const errorRequests = derived(requestQueue, $q =>
  $q.order.map(id => $q.byId[id]).filter(r => r && r.phase === 'error')
);

export const currentActiveRequest = derived(requestQueue, $q =>
  $q.activeId ? $q.byId[$q.activeId] : null
);

import { activeThreadIdStore as activeThreadId } from './domainState';

/**
 * Returns true if the current active thread has an in-flight (active) request.
 */
export const activeThreadBusy = derived(
  [requestQueue, activeThreadId],
  ([$q, $tid]) => {
    return Object.values($q.byId).some(r => 
      r.threadId === $tid && !['success', 'error', 'canceled'].includes(r.phase)
    );
  }
);

export const activeThreadModelBusy = derived(
  [requestQueue, activeThreadId],
  ([$q, $tid]) => {
    return Object.values($q.byId).some((r) =>
      r.threadId === $tid && isModelActivePhase(r.phase),
    );
  },
);
