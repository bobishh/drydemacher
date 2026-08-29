import assert from 'node:assert/strict';
import test from 'node:test';

import { projectThreadAgentStateFromSessionEvents } from './presentation';
import type { SessionEvent } from '../sessionActivity';

function agentEvent(overrides: Partial<SessionEvent>): SessionEvent {
  return {
    id: overrides.id ?? 'event-1',
    cursor: overrides.cursor ?? 1,
    sessionId: overrides.sessionId ?? 'session-1',
    threadId: overrides.threadId ?? 'thread-1',
    versionId: overrides.versionId ?? null,
    lifecycleKey: overrides.lifecycleKey ?? 'lifecycle-1',
    actor:
      overrides.actor ?? {
        kind: 'agent',
        id: 'agent-1',
        label: 'Codex',
      },
    kind: overrides.kind ?? 'agent_action_finished',
    title: overrides.title ?? 'Agent activity',
    summary: overrides.summary ?? 'Working the thread.',
    detail: overrides.detail ?? 'Rendering the model.',
    timestamp: overrides.timestamp ?? 10,
    severity: overrides.severity ?? 'info',
    raw: overrides.raw ?? null,
    ...overrides,
  };
}

test('projectThreadAgentStateFromSessionEvents reconstructs current-thread presentation from catch-up', () => {
  const projection = projectThreadAgentStateFromSessionEvents(
    [
      agentEvent({
        cursor: 1,
        summary: 'Booting',
        detail: 'Waking agent.',
        state: 'active' as never,
        phase: 'reading',
        requiresAttention: false as never,
        timestamp: 10,
      }),
      agentEvent({
        cursor: 2,
        summary: 'Need prompt',
        detail: 'Waiting for your next message.',
        state: 'resolved' as never,
        phase: 'waiting_for_user',
        requiresAttention: true as never,
        timestamp: 20,
        actor: {
          kind: 'agent',
          id: 'agent-1',
          label: 'Codex',
        },
      }),
      agentEvent({
        cursor: 3,
        threadId: 'thread-2',
        summary: 'Other thread',
        detail: 'Ignore me.',
        state: 'active' as never,
        phase: 'rendering',
        requiresAttention: false as never,
        timestamp: 30,
      }),
    ],
    'thread-1',
  );

  assert.equal(projection.connectionState, 'waiting');
  assert.equal(projection.busy, false);
  assert.equal(projection.phase, 'waiting_for_user');
  assert.equal(projection.waitingOnPrompt, true);
  assert.equal(projection.activityLabel, 'Need prompt');
  assert.equal(projection.statusText, 'Waiting for your next message.');
  assert.equal(projection.agentLabel, 'Codex');
  assert.equal(projection.sessionId, 'session-1');
  assert.equal(projection.updatedAt, 20);
});

test('projectThreadAgentStateFromSessionEvents keeps connected agent active after resolved work', () => {
  const projected = projectThreadAgentStateFromSessionEvents([
    {
      id: 'resolved-1', cursor: 9, sessionId: 'session-1', threadId: 'thread-1', versionId: null,
      actor: { kind: 'agent', id: 'agent-1', label: 'Codex' }, kind: 'agent_action_finished',
      title: 'done', summary: 'done', phase: 'idle', state: 'resolved', requiresAttention: false,
      timestamp: 9, severity: 'success',
    },
  ], 'thread-1');

  assert.equal(projected.connectionState, 'active');
  assert.equal(projected.busy, false);
});

test('projectThreadAgentStateFromSessionEvents restores runtime metadata without snapshot polling', () => {
  const projected = projectThreadAgentStateFromSessionEvents([
    agentEvent({
      cursor: 10,
      state: 'active',
      phase: 'active',
      raw: JSON.stringify({
        providerKind: 'openai',
        llmModelLabel: 'gpt-5.4-mini',
        busy: true,
        activityLabel: 'Applying AST patch',
        activityStartedAt: 42,
        attentionKind: null,
        waitingOnPrompt: false,
      }),
    }),
  ], 'thread-1');

  assert.equal(projected.providerKind, 'openai');
  assert.equal(projected.llmModelLabel, 'gpt-5.4-mini');
  assert.equal(projected.busy, true);
  assert.equal(projected.activityLabel, 'Applying AST patch');
  assert.equal(projected.activityStartedAt, 42);
});
