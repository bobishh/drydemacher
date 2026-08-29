import { writable, get } from 'svelte/store';
import type { AgentActivityCatchUp, AgentActivityEvent } from '../tauri/contracts';

export type { AgentActivityCatchUp, AgentActivityEvent };

export type AgentActivityIngestionSnapshot = {
  events: AgentActivityEvent[];
  latestCursor: number;
  contiguousCursor: number;
};

export type AgentActivityTransport = {
  listen: (
    eventName: 'agent-activity-event',
    handler: (event: { payload: AgentActivityEvent }) => void,
  ) => Promise<() => void>;
  getAgentActivity: (afterCursor: number | null) => Promise<AgentActivityCatchUp>;
};

type AgentActivityRetryHandle = ReturnType<typeof globalThis.setTimeout> | number;

export type AgentActivityRecoveryOptions = {
  onRecoveryError?: (error: unknown) => void;
  retryDelayMs?: number;
  scheduleRetry?: (callback: () => void, delayMs: number) => AgentActivityRetryHandle;
  cancelRetry?: (handle: AgentActivityRetryHandle) => void;
};

export type AgentActivityIngestionStore = {
  subscribe: ReturnType<typeof writable<AgentActivityEvent[]>>['subscribe'];
  ingestPush: (event: AgentActivityEvent) => void;
  ingestCatchUp: (events: AgentActivityEvent[], oldestCursor?: number | null) => void;
  snapshot: () => AgentActivityIngestionSnapshot;
  clear: () => void;
};

function sortEvents(events: AgentActivityEvent[]): AgentActivityEvent[] {
  return [...events].sort((left, right) => {
    if (left.cursor !== right.cursor) return left.cursor - right.cursor;
    if (left.occurredAt !== right.occurredAt) return left.occurredAt - right.occurredAt;
    return left.eventId.localeCompare(right.eventId);
  });
}

const AGENT_ACTIVITY_FRONTEND_MAX_EVENTS = 2_048;

function contiguousCursor(events: AgentActivityEvent[], cursorFloor = 0): number {
  let expected = cursorFloor + 1;
  for (const event of events) {
    if (event.cursor !== expected) break;
    expected += 1;
  }
  return expected - 1;
}

export function createAgentActivityIngestionStore(): AgentActivityIngestionStore {
  const store = writable<AgentActivityEvent[]>([]);
  const seenEventIds = new Set<string>();
  let latestCursor = 0;
  let contiguous = 0;
  let cursorFloor = 0;

  function commit(event: AgentActivityEvent): boolean {
    if (seenEventIds.has(event.eventId)) return false;
    seenEventIds.add(event.eventId);
    latestCursor = Math.max(latestCursor, event.cursor);
    store.update((events) => {
      const sorted = sortEvents([...events, event]);
      if (sorted.length <= AGENT_ACTIVITY_FRONTEND_MAX_EVENTS) return sorted;
      const removed = sorted.slice(0, sorted.length - AGENT_ACTIVITY_FRONTEND_MAX_EVENTS);
      const retained = sorted.slice(-AGENT_ACTIVITY_FRONTEND_MAX_EVENTS);
      for (const stale of removed) seenEventIds.delete(stale.eventId);
      cursorFloor = Math.max(cursorFloor, (retained[0]?.cursor ?? 1) - 1);
      return retained;
    });
    contiguous = contiguousCursor(get(store), cursorFloor);
    return true;
  }

  return {
    subscribe: store.subscribe,
    ingestPush(event: AgentActivityEvent) {
      commit(event);
    },
    ingestCatchUp(events: AgentActivityEvent[], oldestCursor?: number | null) {
      if (oldestCursor && oldestCursor > cursorFloor + 1) {
        cursorFloor = oldestCursor - 1;
        contiguous = Math.max(contiguous, cursorFloor);
      }
      for (const event of events) {
        commit(event);
      }
    },
    snapshot() {
      return {
        events: get(store),
        latestCursor,
        contiguousCursor: contiguous,
      };
    },
    clear() {
      seenEventIds.clear();
      latestCursor = 0;
      contiguous = 0;
      cursorFloor = 0;
      store.set([]);
    },
  };
}

export const agentActivityIngestionStore = createAgentActivityIngestionStore();

export function clearAgentActivityIngestionStore() {
  agentActivityIngestionStore.clear();
}

export async function connectAgentActivityIngestion(
  transport: AgentActivityTransport,
  store: AgentActivityIngestionStore = agentActivityIngestionStore,
  options: AgentActivityRecoveryOptions = {},
) {
  let recovering = false;
  let disconnected = false;
  let retryHandle: AgentActivityRetryHandle | null = null;
  const scheduleRetry = options.scheduleRetry ?? ((callback, delayMs) => globalThis.setTimeout(callback, delayMs));
  const cancelRetry = options.cancelRetry ?? ((handle) => globalThis.clearTimeout(handle));

  function queueRetry() {
    if (disconnected || retryHandle !== null) return;
    retryHandle = scheduleRetry(() => {
      retryHandle = null;
      void recover();
    }, options.retryDelayMs ?? 1000);
  }

  async function recover() {
    if (recovering || disconnected) return;
    recovering = true;
    try {
      const snapshot = store.snapshot();
      const afterCursor = snapshot.events.length === 0 ? null : snapshot.contiguousCursor;
      const catchUp = await transport.getAgentActivity(afterCursor);
      store.ingestCatchUp(catchUp.events, catchUp.oldestCursor ?? null);
      if (store.snapshot().contiguousCursor < catchUp.latestCursor) queueRetry();
    } catch (error) {
      options.onRecoveryError?.(error);
      queueRetry();
    } finally {
      recovering = false;
    }
  }

  let unlisten = await transport.listen('agent-activity-event', (event) => {
    store.ingestPush(event.payload);
    if (event.payload.cursor > store.snapshot().contiguousCursor + 1) {
      void recover();
    }
  });

  await recover();

  return {
    store,
    disconnect: async () => {
      disconnected = true;
      if (retryHandle !== null) {
        cancelRetry(retryHandle);
        retryHandle = null;
      }
      await unlisten();
    },
  };
}
