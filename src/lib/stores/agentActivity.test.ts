import assert from 'node:assert/strict';
import test from 'node:test';

import {
  connectAgentActivityIngestion,
  createAgentActivityIngestionStore,
  type AgentActivityEvent,
} from './agentActivity';

function event(overrides: Partial<AgentActivityEvent>): AgentActivityEvent {
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
    state: overrides.state ?? 'active',
    requiresAttention: overrides.requiresAttention ?? false,
    occurredAt: overrides.occurredAt ?? 1,
    raw: overrides.raw ?? null,
  };
}

test('agent activity ingestion dedupes live pushes and heals cursor gaps from catch-up', () => {
  const store = createAgentActivityIngestionStore();

  store.ingestPush(event({ cursor: 1, eventId: 'event-1', summary: 'one' }));
  store.ingestPush(event({ cursor: 3, eventId: 'event-3', summary: 'three' }));
  store.ingestCatchUp([
    event({ cursor: 2, eventId: 'event-2', summary: 'two' }),
    event({ cursor: 3, eventId: 'event-3', summary: 'three' }),
  ]);

  assert.deepEqual(
    store.snapshot().events.map((item) => item.eventId),
    ['event-1', 'event-2', 'event-3'],
  );
  assert.equal(store.snapshot().latestCursor, 3);
  assert.equal(store.snapshot().contiguousCursor, 3);
});

test('connectAgentActivityIngestion subscribes before catch-up and keeps both push and catch-up events', async () => {
  const store = createAgentActivityIngestionStore();
  let liveHandler: ((event: { payload: AgentActivityEvent }) => void) | null = null;

  const connection = await connectAgentActivityIngestion(
    {
      listen: async (_eventName, handler) => {
        liveHandler = handler;
        return async () => {};
      },
      getAgentActivity: async (afterCursor) => {
        assert.equal(afterCursor, null);
        liveHandler?.({
          payload: event({ cursor: 1, eventId: 'event-1', summary: 'one' }),
        });
        return {
          events: [event({ cursor: 2, eventId: 'event-2', summary: 'two' })],
          latestCursor: 2,
          oldestCursor: 1,
          hasMore: false,
          droppedCount: 0,
          retainedBytes: 512,
        };
      },
    },
    store,
  );

  assert.deepEqual(
    connection.store.snapshot().events.map((item) => item.eventId),
    ['event-1', 'event-2'],
  );
  assert.equal(connection.store.snapshot().latestCursor, 2);
  assert.equal(connection.store.snapshot().contiguousCursor, 2);
  await connection.disconnect();
});

test('connectAgentActivityIngestion reports recovery failure and retries catch-up without snapshot fallback', async () => {
  const store = createAgentActivityIngestionStore();
  const retryCallbacks: Array<() => void> = [];
  const errors: unknown[] = [];
  let attempts = 0;

  const connection = await connectAgentActivityIngestion(
    {
      listen: async () => async () => {},
      getAgentActivity: async (afterCursor) => {
        attempts += 1;
        assert.equal(afterCursor, null);
        if (attempts === 1) throw new Error('provider body: cursor journal unavailable');
        return {
          events: [event({ cursor: 1, eventId: 'recovered' })],
          latestCursor: 1,
          oldestCursor: 1,
          hasMore: false,
          droppedCount: 0,
          retainedBytes: 256,
        };
      },
    },
    store,
    {
      onRecoveryError: (error) => errors.push(error),
      scheduleRetry: (callback) => {
        retryCallbacks.push(callback);
        return retryCallbacks.length;
      },
      cancelRetry: () => {},
    },
  );

  assert.equal(errors.length, 1);
  assert.match(String(errors[0]), /provider body: cursor journal unavailable/);
  assert.equal(retryCallbacks.length, 1);

  retryCallbacks[0]();
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(store.snapshot().events.map((item) => item.eventId), ['recovered']);
  assert.equal(attempts, 2);
  await connection.disconnect();
});

test('activity store accepts a compacted cursor floor and bounds retained events', () => {
  const store = createAgentActivityIngestionStore();
  const retained = Array.from({ length: 2_500 }, (_, index) =>
    event({ cursor: 10_000 + index, eventId: `retained-${index}` }),
  );

  store.ingestCatchUp(retained, 10_000);

  const snapshot = store.snapshot();
  assert.equal(snapshot.events.length, 2_048);
  assert.equal(snapshot.events[0].cursor, 10_452);
  assert.equal(snapshot.events.at(-1)?.cursor, 12_499);
  assert.equal(snapshot.contiguousCursor, 12_499);
});
