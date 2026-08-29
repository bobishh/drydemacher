import assert from 'node:assert/strict';
import test from 'node:test';

import type { AgentActivityEvent } from '../tauri/contracts';
import { projectLongTasks } from './longTasks';

function event(overrides: Partial<AgentActivityEvent>): AgentActivityEvent {
  return {
    eventId: overrides.eventId ?? 'event-1',
    cursor: overrides.cursor ?? 1,
    sessionId: overrides.sessionId ?? 'session-1',
    threadId: overrides.threadId ?? 'thread-background',
    messageId: overrides.messageId ?? null,
    versionId: overrides.versionId ?? null,
    actor: overrides.actor ?? { kind: 'agent', id: 'fem', label: 'FEM' },
    kind: overrides.kind ?? 'trace',
    lifecycleKey: overrides.lifecycleKey ?? 'long-task:topology-1',
    phase: overrides.phase ?? 'solve',
    summary: overrides.summary ?? 'Bottle cage topology',
    detail: overrides.detail ?? 'load case 2/5',
    severity: overrides.severity ?? 'info',
    state: overrides.state ?? 'active',
    requiresAttention: overrides.requiresAttention ?? false,
    occurredAt: overrides.occurredAt ?? 1_000,
    raw: overrides.raw ?? JSON.stringify({
      kind: 'long_task_progress', taskId: 'topology-1', expectedDurationMs: 600_000,
      stage: 'SOLVE', progressCurrent: 33, progressTotal: 120, jobId: 'fem-job-1', cancellable: true,
    }),
  };
}

test('projects active long tasks globally and removes them on terminal event', () => {
  const active = event({});
  const snapshot = projectLongTasks([active], 13_000);

  assert.equal(snapshot.length, 1);
  assert.equal(snapshot[0].threadId, 'thread-background');
  assert.equal(snapshot[0].elapsedMs, 12_000);
  assert.deepEqual(snapshot[0].progress, { current: 33, total: 120 });
  assert.equal(snapshot[0].jobId, 'fem-job-1');

  const finished = event({
    eventId: 'event-2', cursor: 2, state: 'resolved', phase: 'idle',
    raw: JSON.stringify({ kind: 'long_task_finished', taskId: 'topology-1' }),
  });
  assert.deepEqual(projectLongTasks([active, finished], 14_000), []);
});

test('pairs generic long_action notice and clear by session across distinct trace lifecycle keys', () => {
  const started = event({
    eventId: 'generic-1', sessionId: 'agent-session', lifecycleKey: 'trace:agent:session_activity_set',
    raw: JSON.stringify({ kind: 'session_activity_set' }),
  });
  const cleared = event({
    eventId: 'generic-2', cursor: 2, sessionId: 'agent-session', lifecycleKey: 'trace:agent:session_activity_clear',
    state: 'resolved', phase: 'idle', raw: JSON.stringify({ kind: 'session_activity_clear' }),
  });

  assert.equal(projectLongTasks([started], 2_000).length, 1);
  assert.deepEqual(projectLongTasks([started, cleared], 3_000), []);
});
