import assert from 'node:assert/strict';
import test from 'node:test';

import type {
  AgentActivityEvent,
  AgentActivitySeverity,
  AgentActivityState,
} from '../tauri/contracts';
import {
  createAgentNotificationsStore,
  type AgentNotificationSnapshot,
} from './agentNotifications';

type TimerEntry = {
  id: number;
  delayMs: number;
  nextRunAt: number;
  callback: () => void;
};

function buildEvent(overrides: Partial<AgentActivityEvent>): AgentActivityEvent {
  return {
    eventId: overrides.eventId ?? `event-${overrides.cursor ?? 0}`,
    cursor: overrides.cursor ?? 0,
    sessionId: overrides.sessionId ?? 'session-1',
    threadId: overrides.threadId ?? 'thread-1',
    messageId: overrides.messageId ?? null,
    versionId: overrides.versionId ?? null,
    actor:
      overrides.actor ?? {
        kind: 'agent',
        id: 'agent-1',
        label: 'Agent',
      },
    kind: overrides.kind ?? 'trace',
    lifecycleKey: overrides.lifecycleKey ?? null,
    phase: overrides.phase ?? null,
    summary: overrides.summary ?? 'summary',
    detail: overrides.detail ?? null,
    severity: overrides.severity ?? 'info',
    state: overrides.state ?? 'resolved',
    requiresAttention: overrides.requiresAttention ?? false,
    occurredAt: overrides.occurredAt ?? 0,
    raw: overrides.raw ?? null,
  };
}

function createFakeClock(startAt = 0) {
  let nowMs = startAt;
  let nextTimerId = 1;
  const timers = new Map<number, TimerEntry>();

  function runDueTimers(targetMs: number) {
    while (true) {
      let nextEntry: TimerEntry | null = null;
      for (const entry of timers.values()) {
        if (entry.nextRunAt <= targetMs && (!nextEntry || entry.nextRunAt < nextEntry.nextRunAt)) {
          nextEntry = entry;
        }
      }
      if (!nextEntry) break;
      nowMs = nextEntry.nextRunAt;
      nextEntry.callback();
      if (!timers.has(nextEntry.id)) continue;
      nextEntry.nextRunAt += nextEntry.delayMs;
    }
    nowMs = targetMs;
  }

  return {
    now: () => nowMs,
    setInterval(callback: () => void, delayMs: number) {
      const entry: TimerEntry = {
        id: nextTimerId++,
        delayMs,
        nextRunAt: nowMs + delayMs,
        callback,
      };
      timers.set(entry.id, entry);
      return entry.id;
    },
    clearInterval(handle: number | ReturnType<typeof setInterval>) {
      if (typeof handle === 'number') {
        timers.delete(handle);
      }
    },
    advanceBy(deltaMs: number) {
      runDueTimers(nowMs + deltaMs);
    },
  };
}

function visibleIds(snapshot: AgentNotificationSnapshot) {
  return snapshot.visibleCards.map((card) => card.eventId);
}

function queuedIds(snapshot: AgentNotificationSnapshot) {
  return snapshot.queuedCards.map((card) => card.eventId);
}

function latestCard(snapshot: AgentNotificationSnapshot) {
  return snapshot.allCards[snapshot.allCards.length - 1] ?? null;
}

test('agent notifications keep four visible, queue overflow FIFO, and preserve oldest-to-newest order', () => {
  const clock = createFakeClock();
  const store = createAgentNotificationsStore(clock);

  store.ingest([
    buildEvent({ cursor: 1, eventId: 'event-1', occurredAt: 1, summary: 'one' }),
    buildEvent({ cursor: 2, eventId: 'event-2', occurredAt: 2, summary: 'two' }),
    buildEvent({ cursor: 3, eventId: 'event-3', occurredAt: 3, summary: 'three' }),
    buildEvent({ cursor: 4, eventId: 'event-4', occurredAt: 4, summary: 'four' }),
    buildEvent({ cursor: 5, eventId: 'event-5', occurredAt: 5, summary: 'five' }),
    buildEvent({ cursor: 6, eventId: 'event-6', occurredAt: 6, summary: 'six' }),
  ]);

  const snapshot = store.snapshot();
  assert.deepEqual(visibleIds(snapshot), ['event-1', 'event-2', 'event-3', 'event-4']);
  assert.deepEqual(queuedIds(snapshot), ['event-5', 'event-6']);
});

test('agent notifications order by backend cursor despite clock skew and lifecycle updates', () => {
  const clock = createFakeClock();
  const store = createAgentNotificationsStore(clock);

  store.ingest([
    buildEvent({ cursor: 3, eventId: 'third', occurredAt: 10, lifecycleKey: 'third-work' }),
    buildEvent({ cursor: 1, eventId: 'first', occurredAt: 30, lifecycleKey: 'first-work' }),
    buildEvent({ cursor: 2, eventId: 'second', occurredAt: 20, lifecycleKey: 'second-work' }),
    buildEvent({ cursor: 4, eventId: 'first-done', occurredAt: 40, lifecycleKey: 'first-work' }),
  ]);

  assert.deepEqual(visibleIds(store.snapshot()), ['first-done', 'second', 'third']);
});

test('agent notifications expire resolved success after eight seconds and warning after twelve', () => {
  const clock = createFakeClock();
  const store = createAgentNotificationsStore(clock);

  store.ingest([
    buildEvent({
      cursor: 1,
      eventId: 'success',
      occurredAt: 1,
      state: 'resolved',
      severity: 'success',
      summary: 'success',
    }),
    buildEvent({
      cursor: 2,
      eventId: 'warning',
      occurredAt: 2,
      state: 'resolved',
      severity: 'warning',
      summary: 'warning',
    }),
  ]);

  clock.advanceBy(7_999);
  assert.deepEqual(visibleIds(store.snapshot()), ['success', 'warning']);
  clock.advanceBy(1);
  assert.deepEqual(visibleIds(store.snapshot()), ['warning']);
  clock.advanceBy(3_999);
  assert.deepEqual(visibleIds(store.snapshot()), ['warning']);
  clock.advanceBy(1);
  assert.deepEqual(visibleIds(store.snapshot()), []);
});

test('active, failed, question, and attention notifications expire into the activity hub', () => {
  const clock = createFakeClock();
  const store = createAgentNotificationsStore(clock);

  store.ingest([
    buildEvent({ cursor: 1, eventId: 'active', state: 'active', severity: 'info' }),
    buildEvent({ cursor: 2, eventId: 'error', state: 'failed', severity: 'error' }),
    buildEvent({ cursor: 3, eventId: 'question', state: 'active', severity: 'question' }),
    buildEvent({ cursor: 4, eventId: 'attention', state: 'resolved', severity: 'info', requiresAttention: true }),
  ]);

  assert.deepEqual(store.snapshot().visibleCards.map((card) => card.remainingMs), [8_000, 8_000, 8_000, 8_000]);
  clock.advanceBy(8_000);
  assert.deepEqual(visibleIds(store.snapshot()), []);
});

test('agent notifications pause hover and hidden time without burning remaining ttl', () => {
  const clock = createFakeClock();
  const store = createAgentNotificationsStore(clock);

  store.ingest(
    buildEvent({
      cursor: 1,
      eventId: 'paused-success',
      occurredAt: 1,
      state: 'resolved',
      severity: 'success',
      summary: 'paused-success',
    }),
  );

  const initialRemaining = store.snapshot().visibleCards[0].remainingMs;
  assert.equal(initialRemaining, 8_000);

  store.setHoveredEventId('paused-success');
  clock.advanceBy(10_000);
  let snapshot = store.snapshot();
  assert.equal(snapshot.visibleCards[0].remainingMs, initialRemaining);

  store.setHoveredEventId(null);
  clock.advanceBy(1000);
  snapshot = store.snapshot();
  assert.equal(snapshot.visibleCards[0].remainingMs, initialRemaining - 1000);

  store.setDocumentVisible(false);
  clock.advanceBy(10_000);
  snapshot = store.snapshot();
  assert.equal(snapshot.visibleCards[0].remainingMs, initialRemaining - 1000);

  store.setDocumentVisible(true);
  clock.advanceBy(1000);
  snapshot = store.snapshot();
  assert.equal(snapshot.visibleCards[0].remainingMs, initialRemaining - 2000);
});

test('queued agent notification receives full ttl after promotion', () => {
  const clock = createFakeClock();
  const store = createAgentNotificationsStore(clock);

  store.ingest([
    buildEvent({ cursor: 1, eventId: 'card-1', occurredAt: 1, state: 'resolved', severity: 'success' }),
    buildEvent({ cursor: 2, eventId: 'card-2', occurredAt: 2, state: 'resolved', severity: 'success' }),
    buildEvent({ cursor: 3, eventId: 'card-3', occurredAt: 3, state: 'resolved', severity: 'success' }),
    buildEvent({ cursor: 4, eventId: 'card-4', occurredAt: 4, state: 'resolved', severity: 'success' }),
    buildEvent({ cursor: 5, eventId: 'card-5', occurredAt: 5, state: 'resolved', severity: 'success' }),
  ]);

  clock.advanceBy(8_000);
  let snapshot = store.snapshot();
  assert.deepEqual(visibleIds(snapshot), ['card-5']);
  assert.equal(snapshot.visibleCards[0].remainingMs, 8_000);

  clock.advanceBy(7_999);
  snapshot = store.snapshot();
  assert.deepEqual(visibleIds(snapshot), ['card-5']);
  assert.equal(snapshot.visibleCards[0].remainingMs, 1);

  clock.advanceBy(1);
  snapshot = store.snapshot();
  assert.equal(visibleIds(snapshot).length, 0);
});

test('agent notifications dismiss by eventId and do not suppress later identical messages', () => {
  const clock = createFakeClock();
  const store = createAgentNotificationsStore(clock);

  store.ingest(buildEvent({ cursor: 1, eventId: 'message-a', occurredAt: 1, summary: 'same text' }));
  assert.deepEqual(visibleIds(store.snapshot()), ['message-a']);

  store.dismiss('message-a');
  assert.deepEqual(visibleIds(store.snapshot()), []);

  store.ingest(buildEvent({ cursor: 2, eventId: 'message-b', occurredAt: 2, summary: 'same text' }));
  assert.deepEqual(visibleIds(store.snapshot()), ['message-b']);
});

test('agent notifications fold lifecycle updates into one card while retaining source events', () => {
  const clock = createFakeClock();
  const store = createAgentNotificationsStore(clock);

  store.ingest([
    buildEvent({
      cursor: 1,
      eventId: 'build-start',
      lifecycleKey: 'build-1',
      occurredAt: 1,
      state: 'active',
      severity: 'info',
      summary: 'build started',
    }),
    buildEvent({
      cursor: 2,
      eventId: 'build-finish',
      lifecycleKey: 'build-1',
      occurredAt: 2,
      state: 'resolved',
      severity: 'success',
      summary: 'build finished',
    }),
  ]);

  const snapshot = store.snapshot();
  assert.equal(snapshot.allCards.length, 1);
  assert.deepEqual(snapshot.allCards[0].eventIds, ['build-start', 'build-finish']);
  assert.equal(snapshot.allCards[0].summary, 'build finished');
  assert.equal(latestCard(snapshot)?.eventId, 'build-finish');
});

test('active lifecycle notification expires after eight seconds without a terminal event', () => {
  const clock = createFakeClock();
  const store = createAgentNotificationsStore(clock);

  store.ingest(buildEvent({ cursor: 1, eventId: 'work-start', lifecycleKey: 'work', state: 'active' }));
  clock.advanceBy(7_999);
  assert.deepEqual(visibleIds(store.snapshot()), ['work-start']);
  clock.advanceBy(1);
  assert.deepEqual(visibleIds(store.snapshot()), []);
});

test('terminal lifecycle updates preserve elapsed ttl instead of restarting it', () => {
  const clock = createFakeClock();
  const store = createAgentNotificationsStore(clock);

  store.ingest(buildEvent({ cursor: 1, eventId: 'work-start', lifecycleKey: 'work', state: 'active' }));
  store.ingest(buildEvent({ cursor: 2, eventId: 'work-done', lifecycleKey: 'work', state: 'resolved', severity: 'success' }));
  clock.advanceBy(3_000);
  store.ingest(buildEvent({ cursor: 3, eventId: 'work-indexed', lifecycleKey: 'work', state: 'resolved', severity: 'success' }));

  assert.equal(store.snapshot().visibleCards[0].remainingMs, 5_000);
  clock.advanceBy(4_999);
  assert.deepEqual(visibleIds(store.snapshot()), ['work-indexed']);
  clock.advanceBy(1);
  assert.deepEqual(visibleIds(store.snapshot()), []);
});

test('notification projection bounds ordinary cards and lifecycle source events', () => {
  const clock = createFakeClock();
  const store = createAgentNotificationsStore(clock);

  for (let cursor = 1; cursor <= 600; cursor += 1) {
    store.ingest(buildEvent({
      cursor,
      eventId: `ordinary-${cursor}`,
      lifecycleKey: `ordinary-${cursor}`,
      state: 'resolved',
    }));
  }
  assert.equal(store.snapshot().allCards.length, 512);

  store.clear();
  for (let cursor = 1; cursor <= 100; cursor += 1) {
    store.ingest(buildEvent({
      cursor,
      eventId: `lifecycle-${cursor}`,
      lifecycleKey: 'one-lifecycle',
      state: cursor === 100 ? 'failed' : 'active',
      severity: cursor === 100 ? 'error' : 'info',
      requiresAttention: cursor === 100,
    }));
  }
  const card = store.snapshot().allCards[0];
  assert.equal(card.eventIds.length, 64);
  assert.equal(card.sourceEvents.length, 64);
  assert.equal(card.eventId, 'lifecycle-100');
});
