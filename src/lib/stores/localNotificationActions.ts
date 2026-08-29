import { writable } from 'svelte/store';

export type LocalNotificationAction = {
  label: string;
  onclick: () => void;
};

export type LocalNotificationCard = {
  eventId: string;
  threadId: string | null;
  actorLabel: string;
  summary: string;
  detail: string | null;
  severity: 'info' | 'warning' | 'error' | 'question';
  state: 'active' | 'resolved' | 'failed';
  requiresAttention: boolean;
  actions: LocalNotificationAction[];
};

type LocalNotificationTimerHandle = ReturnType<typeof globalThis.setTimeout> | number;

export type LocalNotificationClock = {
  setTimeout: (callback: () => void, delayMs: number) => LocalNotificationTimerHandle;
  clearTimeout: (handle: LocalNotificationTimerHandle) => void;
};

const systemClock: LocalNotificationClock = {
  setTimeout(callback, delayMs) {
    const handle = globalThis.setTimeout(callback, delayMs);
    if (typeof handle === 'object' && typeof handle.unref === 'function') handle.unref();
    return handle;
  },
  clearTimeout: (handle) => globalThis.clearTimeout(handle),
};

function notificationDurationMs(card: LocalNotificationCard): number | null {
  if (card.severity === 'warning') return 12_000;
  return 8_000;
}

export function createLocalNotificationStore(clock: LocalNotificationClock = systemClock) {
  const store = writable<LocalNotificationCard | null>(null);
  let timer: LocalNotificationTimerHandle | null = null;
  let currentEventId: string | null = null;

  function set(card: LocalNotificationCard | null) {
    if (card && card.eventId === currentEventId) {
      store.set(card);
      return;
    }
    if (timer !== null) {
      clock.clearTimeout(timer);
      timer = null;
    }
    currentEventId = card?.eventId ?? null;
    store.set(card);
    if (!card) return;
    const ttlMs = notificationDurationMs(card);
    if (ttlMs === null) return;
    timer = clock.setTimeout(() => {
      timer = null;
      currentEventId = null;
      store.set(null);
    }, ttlMs);
  }

  return { subscribe: store.subscribe, set };
}

export const localNotificationActionsStore = createLocalNotificationStore();
