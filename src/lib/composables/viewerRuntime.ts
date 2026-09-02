import type { AuthoringTargetRef } from '../types/domain';

export type ViewerKind = 'visible' | 'hidden';

type ViewerLoadWaiter = {
  targetNonce: number;
  resolve: () => void;
  reject: (error: Error) => void;
  timer: ReturnType<typeof setTimeout>;
};

function settle(waiters: ViewerLoadWaiter[], nonce: number): ViewerLoadWaiter[] {
  const pending: ViewerLoadWaiter[] = [];
  for (const waiter of waiters) {
    if (nonce >= waiter.targetNonce) {
      clearTimeout(waiter.timer);
      waiter.resolve();
    } else {
      pending.push(waiter);
    }
  }
  return pending;
}

function reject(waiters: ViewerLoadWaiter[], error: Error): ViewerLoadWaiter[] {
  for (const waiter of waiters) {
    clearTimeout(waiter.timer);
    waiter.reject(error);
  }
  return [];
}

/** Viewer load/reload waiter state. UI, stores, and recovery policy stay in App. */
export function createViewerLoadRuntime() {
  const nonces: Record<ViewerKind, number> = { visible: 0, hidden: 0 };
  const waiters: Record<ViewerKind, ViewerLoadWaiter[]> = { visible: [], hidden: [] };

  function waitForLoad(kind: ViewerKind, previousNonce: number, timeoutMs = 12000): Promise<void> {
    if (nonces[kind] > previousNonce) return Promise.resolve();
    return new Promise<void>((resolve, rejectWaiter) => {
      const waiter: ViewerLoadWaiter = {
        targetNonce: previousNonce + 1,
        resolve,
        reject: rejectWaiter,
        timer: setTimeout(() => {
          waiters[kind] = waiters[kind].filter((candidate) => candidate !== waiter);
          rejectWaiter(new Error(`Timed out waiting for the ${kind} viewer to load.`));
        }, timeoutMs),
      };
      waiters[kind] = [...waiters[kind], waiter];
    });
  }

  function markLoaded(kind: ViewerKind): number {
    nonces[kind] += 1;
    waiters[kind] = settle(waiters[kind], nonces[kind]);
    return nonces[kind];
  }

  function markFailed(kind: ViewerKind, message: string): void {
    const label = kind === 'visible' ? 'Visible' : 'Hidden';
    waiters[kind] = reject(waiters[kind], new Error(`${label} viewer failed to load model. ${message}`));
  }

  return {
    waitForLoad,
    markLoaded,
    markFailed,
    loadNonce: (kind: ViewerKind) => nonces[kind],
    pendingCount: (kind: ViewerKind) => waiters[kind].length,
  };
}

export function isMissingViewerArtifactError(message: string): boolean {
  const normalized = message.toLowerCase();
  return normalized.includes('responded with 404') || normalized.includes('not found') || normalized.includes('status 404');
}

export function canRepairSavedVersionRuntime(
  targetRef: AuthoringTargetRef | null | undefined,
  threadId: string | null,
  messageId: string | null,
): boolean {
  if (!threadId || !messageId) return false;
  if (!targetRef || targetRef.threadId !== threadId) return true;
  return targetRef.kind === 'savedVersion' && targetRef.messageId === messageId;
}
