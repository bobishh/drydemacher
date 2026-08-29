import { writable } from 'svelte/store';

import type {
  AgentActivityEvent,
  AgentActivityActorKind,
  AgentActivitySeverity,
  AgentActivityState,
} from '../tauri/contracts';

export type AgentNotificationClock = {
  now: () => number;
  setInterval: (callback: () => void, delayMs: number) => AgentNotificationTimerHandle;
  clearInterval: (handle: AgentNotificationTimerHandle) => void;
};

type AgentNotificationTimerHandle = ReturnType<typeof globalThis.setInterval> | number;

export type AgentNotificationCard = {
  eventId: string;
  lifecycleKey: string | null;
  eventIds: string[];
  threadId: string | null;
  actorKind: AgentActivityActorKind;
  actorLabel: string;
  summary: string;
  detail: string | null;
  severity: AgentActivitySeverity;
  state: AgentActivityState;
  requiresAttention: boolean;
  occurredAt: number;
  visible: boolean;
  paused: boolean;
  sourceEvents: AgentActivityEvent[];
  visibleElapsedMs: number;
  terminalElapsedMs: number;
  remainingMs: number | null;
};

export type AgentNotificationSnapshot = {
  visibleCards: AgentNotificationCard[];
  queuedCards: AgentNotificationCard[];
  allCards: AgentNotificationCard[];
};

type AgentNotificationTimerState = {
  eventKey: string;
  eventIds: string[];
  sourceEvents: AgentActivityEvent[];
  latestEvent: AgentActivityEvent;
  threadId: string | null;
  actorKind: AgentActivityActorKind;
  actorLabel: string;
  summary: string;
  detail: string | null;
  severity: AgentActivitySeverity;
  state: AgentActivityState;
  requiresAttention: boolean;
  occurredAt: number;
  visible: boolean;
  dismissed: boolean;
  visibleElapsedMs: number;
  visibleRunningSince: number | null;
  terminalElapsedMs: number;
  terminalRunningSince: number | null;
  terminalDurationMs: number | null;
  hovered: boolean;
  focused: boolean;
  firstSeenAt: number;
  firstCursor: number;
};

export type AgentNotificationStore = {
  subscribe: ReturnType<typeof writable<AgentNotificationSnapshot>>['subscribe'];
  ingest: (event: AgentActivityEvent | AgentActivityEvent[]) => void;
  ingestPriority: (event: AgentActivityEvent | AgentActivityEvent[]) => void;
  dismiss: (eventId: string) => void;
  setHoveredEventId: (eventId: string | null) => void;
  setFocusedEventId: (eventId: string | null) => void;
  setDocumentVisible: (visible: boolean) => void;
  snapshot: () => AgentNotificationSnapshot;
  clear: () => void;
  stop: () => void;
};

type AgentNotificationRuntimeState = {
  documentVisible: boolean;
  cards: AgentNotificationTimerState[];
};

const MAX_VISIBLE_CARDS = 4;
const MAX_RETAINED_CARDS = 512;
const MAX_SOURCE_EVENTS_PER_CARD = 64;
const MIN_VISIBLE_MS = 2_000;
const RESOLVED_NOTIFICATION_TTL_MS = 8_000;
const WARNING_NOTIFICATION_TTL_MS = 12_000;

const systemClock: AgentNotificationClock = {
  now: () => Date.now(),
  setInterval: (callback, delayMs) => {
    const handle = setInterval(callback, delayMs);
    if (typeof handle === 'object' && typeof handle.unref === 'function') {
      handle.unref();
    }
    return handle;
  },
  clearInterval: (handle) => clearInterval(handle),
};

function sortCards(cards: AgentNotificationTimerState[]): AgentNotificationTimerState[] {
  return [...cards].sort((left, right) => {
    if (left.firstCursor !== right.firstCursor) return left.firstCursor - right.firstCursor;
    if (left.firstSeenAt !== right.firstSeenAt) return left.firstSeenAt - right.firstSeenAt;
    return left.latestEvent.eventId.localeCompare(right.latestEvent.eventId);
  });
}

function notificationDurationMs(event: AgentActivityEvent): number | null {
  if (event.severity === 'warning') return WARNING_NOTIFICATION_TTL_MS;
  return RESOLVED_NOTIFICATION_TTL_MS;
}

function canRunTerminalTimer(card: AgentNotificationTimerState, documentVisible: boolean): boolean {
  if (!documentVisible) return false;
  if (!card.visible) return false;
  return !card.hovered && !card.focused;
}

function canRunVisibleTimer(card: AgentNotificationTimerState, documentVisible: boolean): boolean {
  if (!documentVisible) return false;
  if (!card.visible) return false;
  return !card.hovered && !card.focused;
}

function refreshTimerState(card: AgentNotificationTimerState, now: number) {
  if (card.terminalRunningSince !== null) {
    card.terminalElapsedMs += Math.max(0, now - card.terminalRunningSince);
    card.terminalRunningSince = now;
  }
  if (card.visibleRunningSince !== null) {
    card.visibleElapsedMs += Math.max(0, now - card.visibleRunningSince);
    card.visibleRunningSince = now;
  }
}

function createCard(event: AgentActivityEvent, now: number, visible: boolean): AgentNotificationTimerState {
  const terminalDuration = notificationDurationMs(event);

  return {
    eventKey: event.lifecycleKey ?? event.eventId,
    eventIds: [event.eventId],
    sourceEvents: [event],
    latestEvent: event,
    threadId: event.threadId ?? null,
    actorKind: event.actor.kind,
    actorLabel: event.actor.label,
    summary: event.summary,
    detail: event.detail ?? null,
    severity: event.severity,
    state: event.state,
    requiresAttention: event.requiresAttention,
    occurredAt: event.occurredAt,
    visible,
    dismissed: false,
    visibleElapsedMs: 0,
    visibleRunningSince: null,
    terminalElapsedMs: 0,
    terminalRunningSince: null,
    terminalDurationMs: terminalDuration,
    hovered: false,
    focused: false,
    firstSeenAt: now,
    firstCursor: event.cursor,
  };
}

function updateCardFromEvent(card: AgentNotificationTimerState, event: AgentActivityEvent) {
  const previousTerminalDurationMs = card.terminalDurationMs;
  const nextTerminalDurationMs = notificationDurationMs(event);
  card.eventIds.push(event.eventId);
  card.sourceEvents.push(event);
  if (card.eventIds.length > MAX_SOURCE_EVENTS_PER_CARD) {
    card.eventIds.splice(0, card.eventIds.length - MAX_SOURCE_EVENTS_PER_CARD);
  }
  if (card.sourceEvents.length > MAX_SOURCE_EVENTS_PER_CARD) {
    card.sourceEvents.splice(0, card.sourceEvents.length - MAX_SOURCE_EVENTS_PER_CARD);
  }
  card.latestEvent = event;
  card.threadId = event.threadId ?? null;
  card.actorKind = event.actor.kind;
  card.actorLabel = event.actor.label;
  card.summary = event.summary;
  card.detail = event.detail ?? null;
  card.severity = event.severity;
  card.state = event.state;
  card.requiresAttention = event.requiresAttention;
  card.occurredAt = event.occurredAt;
  card.terminalDurationMs = nextTerminalDurationMs;
  if (nextTerminalDurationMs === null) {
    card.terminalElapsedMs = 0;
    card.terminalRunningSince = null;
  } else if (previousTerminalDurationMs === null) {
    card.terminalElapsedMs = 0;
    card.terminalRunningSince = null;
  }
}

function cardWantsTerminalTimer(card: AgentNotificationTimerState): boolean {
  return card.terminalDurationMs !== null && !card.dismissed;
}

function cardCanStayVisible(card: AgentNotificationTimerState): boolean {
  if (card.dismissed) return false;
  if (!card.visible) return true;
  if (card.visibleElapsedMs < MIN_VISIBLE_MS) return true;
  const duration = card.terminalDurationMs;
  if (duration === null) return true;
  return card.terminalElapsedMs < duration;
}

function setCardVisible(card: AgentNotificationTimerState, now: number, documentVisible: boolean) {
  card.visible = true;
  if (card.visibleRunningSince === null) {
    card.visibleRunningSince = canRunVisibleTimer(card, documentVisible) ? now : null;
  }
  if (cardWantsTerminalTimer(card) && card.terminalRunningSince === null) {
    card.terminalRunningSince = canRunTerminalTimer(card, documentVisible) ? now : null;
  }
}

function syncRecord(card: AgentNotificationTimerState, now: number) {
  refreshTimerState(card, now);
}

function resumeTimers(card: AgentNotificationTimerState, now: number, documentVisible: boolean) {
  if (canRunTerminalTimer(card, documentVisible) && card.terminalRunningSince === null && cardWantsTerminalTimer(card)) {
    card.terminalRunningSince = now;
  }
  if (canRunVisibleTimer(card, documentVisible) && card.visibleRunningSince === null) {
    card.visibleRunningSince = now;
  }
}

function hideTimers(card: AgentNotificationTimerState) {
  card.terminalRunningSince = null;
  card.visibleRunningSince = null;
}

function projectCard(card: AgentNotificationTimerState, now: number): AgentNotificationCard {
  const visibleElapsedMs =
    card.visibleElapsedMs +
    (card.visibleRunningSince === null ? 0 : Math.max(0, now - card.visibleRunningSince));
  const terminalElapsedMs =
    card.terminalElapsedMs +
    (card.terminalRunningSince === null ? 0 : Math.max(0, now - card.terminalRunningSince));
  const remainingMs =
    card.terminalDurationMs === null
      ? null
      : Math.max(
          0,
          Math.max(MIN_VISIBLE_MS - visibleElapsedMs, card.terminalDurationMs - terminalElapsedMs),
        );

  return {
    eventId: card.latestEvent.eventId,
    lifecycleKey: card.latestEvent.lifecycleKey ?? null,
    eventIds: [...card.eventIds],
    threadId: card.threadId,
    actorKind: card.actorKind,
    actorLabel: card.actorLabel,
    summary: card.summary,
    detail: card.detail,
    severity: card.severity,
    state: card.state,
    requiresAttention: card.requiresAttention,
    occurredAt: card.occurredAt,
    visible: card.visible,
    paused: card.visible ? card.visibleRunningSince === null : false,
    sourceEvents: [...card.sourceEvents],
    visibleElapsedMs,
    terminalElapsedMs,
    remainingMs,
  };
}

export function projectAgentNotifications(
  state: AgentNotificationRuntimeState,
  now: number,
): AgentNotificationSnapshot {
  const cards = sortCards(state.cards.filter((card) => !card.dismissed));
  const visibleCards = cards.filter((card) => card.visible);
  const queuedCards = cards.filter((card) => !card.visible);
  return {
    visibleCards: visibleCards.map((card) => projectCard(card, now)),
    queuedCards: queuedCards.map((card) => projectCard(card, now)),
    allCards: cards.map((card) => projectCard(card, now)),
  };
}

export function createAgentNotificationsStore(
  clock: AgentNotificationClock = systemClock,
): AgentNotificationStore {
  const store = writable<AgentNotificationSnapshot>({ visibleCards: [], queuedCards: [], allCards: [] });
  let currentSnapshot: AgentNotificationSnapshot = { visibleCards: [], queuedCards: [], allCards: [] };
  const state: AgentNotificationRuntimeState = {
    documentVisible: true,
    cards: [],
  };

  function reconcile() {
    const now = clock.now();

    for (const card of state.cards) {
      syncRecord(card, now);
    }

    if (state.cards.length > MAX_RETAINED_CARDS) {
      const protectedCards = state.cards.filter((card) =>
        card.requiresAttention || card.severity === 'error' || card.state === 'active',
      );
      const removable = sortCards(state.cards.filter((card) => !protectedCards.includes(card)));
      const removableBudget = Math.max(0, MAX_RETAINED_CARDS - protectedCards.length);
      state.cards = [
        ...protectedCards,
        ...removable.slice(-removableBudget),
      ];
    }

    let changed = true;
    while (changed) {
      changed = false;

      const ordered = sortCards(state.cards.filter((card) => !card.dismissed));
      const visibleCards = ordered.filter((card) => card.visible);

      for (const card of visibleCards) {
        if (!cardCanStayVisible(card)) {
          card.dismissed = true;
          card.visible = false;
          hideTimers(card);
          changed = true;
          break;
        }
      }
      if (changed) continue;

      const visibleCount = ordered.filter((card) => card.visible).length;
      if (visibleCount < MAX_VISIBLE_CARDS) {
        const nextQueued = ordered.find((card) => !card.visible && !card.dismissed);
        if (nextQueued) {
          nextQueued.visible = true;
          setCardVisible(nextQueued, now, state.documentVisible);
          resumeTimers(nextQueued, now, state.documentVisible);
          changed = true;
        }
      }
    }

    currentSnapshot = projectAgentNotifications(state, now);
    store.set(currentSnapshot);
  }

  function upsertEvent(event: AgentActivityEvent, prioritize = false) {
    if (state.cards.some((card) => card.eventIds.includes(event.eventId))) return;
    const now = clock.now();
    const eventKey = event.lifecycleKey ?? event.eventId;
    let card = state.cards.find((item) => item.eventKey === eventKey && !item.dismissed);

    if (!card) {
      let visibleCount = state.cards.filter((item) => item.visible && !item.dismissed).length;
      if (prioritize && visibleCount >= MAX_VISIBLE_CARDS) {
        const demoted = sortCards(state.cards.filter((item) => item.visible && !item.dismissed))
          .find((item) => !item.requiresAttention);
        if (demoted) {
          syncRecord(demoted, now);
          demoted.visible = false;
          hideTimers(demoted);
          visibleCount -= 1;
        }
      }
      card = createCard(event, now, visibleCount < MAX_VISIBLE_CARDS);
      state.cards.push(card);
    } else {
      syncRecord(card, now);
      updateCardFromEvent(card, event);
    }

    if (card.terminalDurationMs !== null && card.terminalRunningSince === null) {
      card.terminalRunningSince = canRunTerminalTimer(card, state.documentVisible) ? now : null;
    }
    if (card.visible && card.visibleRunningSince === null && state.documentVisible && !card.hovered && !card.focused) {
      card.visibleRunningSince = now;
    }

    reconcile();
  }

  const interval = clock.setInterval(() => {
    reconcile();
  }, 1000);

  return {
    subscribe: store.subscribe,
    ingest(eventOrEvents) {
      const events = Array.isArray(eventOrEvents) ? eventOrEvents : [eventOrEvents];
      for (const event of events) {
        upsertEvent(event);
      }
    },
    ingestPriority(eventOrEvents) {
      const events = Array.isArray(eventOrEvents) ? eventOrEvents : [eventOrEvents];
      for (const event of events) {
        upsertEvent(event, true);
      }
    },
    dismiss(eventId) {
      const now = clock.now();
      for (const card of state.cards) {
        if (card.latestEvent.eventId !== eventId) continue;
        syncRecord(card, now);
        card.dismissed = true;
        card.visible = false;
        hideTimers(card);
      }
      reconcile();
    },
    setHoveredEventId(eventId) {
      const now = clock.now();
      for (const card of state.cards) {
        if (card.latestEvent.eventId !== eventId && card.hovered) {
          syncRecord(card, now);
          card.hovered = false;
          resumeTimers(card, now, state.documentVisible);
        }
      }
      const hovered = state.cards.find((card) => card.latestEvent.eventId === eventId && !card.dismissed);
      if (hovered) {
        syncRecord(hovered, now);
        hovered.hovered = eventId !== null;
        if (eventId === null) {
          resumeTimers(hovered, now, state.documentVisible);
        } else {
          hideTimers(hovered);
        }
      }
      reconcile();
    },
    setFocusedEventId(eventId) {
      const now = clock.now();
      for (const card of state.cards) {
        if (card.latestEvent.eventId !== eventId && card.focused) {
          syncRecord(card, now);
          card.focused = false;
          resumeTimers(card, now, state.documentVisible);
        }
      }
      const focused = state.cards.find((card) => card.latestEvent.eventId === eventId && !card.dismissed);
      if (focused) {
        syncRecord(focused, now);
        focused.focused = eventId !== null;
        if (eventId === null) {
          resumeTimers(focused, now, state.documentVisible);
        } else {
          hideTimers(focused);
        }
      }
      reconcile();
    },
    setDocumentVisible(visible) {
      const now = clock.now();
      for (const card of state.cards) {
        syncRecord(card, now);
      }
      state.documentVisible = visible;
      for (const card of state.cards) {
        if (visible) {
          resumeTimers(card, now, state.documentVisible);
        } else {
          hideTimers(card);
        }
      }
      reconcile();
    },
    snapshot() {
      reconcile();
      return currentSnapshot;
    },
    clear() {
      state.documentVisible = true;
      state.cards = [];
      currentSnapshot = { visibleCards: [], queuedCards: [], allCards: [] };
      store.set(currentSnapshot);
    },
    stop() {
      clock.clearInterval(interval);
    },
  };
}

export const agentNotificationsStore = createAgentNotificationsStore();

export function clearAgentNotificationsStore() {
  agentNotificationsStore.clear();
}
