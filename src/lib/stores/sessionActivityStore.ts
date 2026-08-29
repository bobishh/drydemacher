import { get, writable } from 'svelte/store';
import {
  appendSessionEvent,
  mapAgentActivityEventToSessionEvent,
  type AgentActivityEventLike,
  type SessionActor,
  type SessionEvent,
} from '../sessionActivity';

export const sessionActivityEvents = writable<SessionEvent[]>([]);

const SESSION_ACTIVITY_MAX_EVENTS = 2_048;

let eventSeq = 0;
const seenSessionEventIds = new Set<string>();

export type SessionActivityEventInput = Omit<SessionEvent, 'id' | 'sessionId' | 'timestamp' | 'actor'> &
  Partial<Pick<SessionEvent, 'id' | 'sessionId' | 'timestamp' | 'actor'>>;

export function recordSessionActivityEvent(input: SessionActivityEventInput): SessionEvent {
  const now = input.timestamp ?? Date.now();
  const actor: SessionActor = input.actor ?? { kind: 'system', id: 'ecky' };
  const event: SessionEvent = {
    ...input,
    id: input.id ?? `session-event:${now}:${eventSeq++}`,
    sessionId: input.sessionId ?? 'local-session',
    cursor: input.cursor ?? null,
    lifecycleKey: input.lifecycleKey ?? null,
    timestamp: now,
    actor,
  };
  seenSessionEventIds.add(event.id);
  sessionActivityEvents.update((events) =>
    appendSessionEvent(events, event).slice(-SESSION_ACTIVITY_MAX_EVENTS),
  );
  return event;
}

export function ingestAgentActivitySessionEvent(input: AgentActivityEventLike): SessionEvent | null {
  if (seenSessionEventIds.has(input.eventId)) return null;
  const event = mapAgentActivityEventToSessionEvent(input);
  seenSessionEventIds.add(event.id);
  sessionActivityEvents.update((events) =>
    appendSessionEvent(events, event).slice(-SESSION_ACTIVITY_MAX_EVENTS),
  );
  return event;
}

export function ingestAgentActivitySessionEvents(inputs: AgentActivityEventLike[]): SessionEvent[] {
  const ingested: SessionEvent[] = [];
  for (const input of inputs) {
    const event = ingestAgentActivitySessionEvent(input);
    if (event) ingested.push(event);
  }
  return ingested;
}

export function clearSessionActivityEvents() {
  eventSeq = 0;
  seenSessionEventIds.clear();
  sessionActivityEvents.set([]);
}

export function currentSessionActivityEvents(): SessionEvent[] {
  return get(sessionActivityEvents);
}
