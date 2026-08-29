import assert from 'node:assert/strict';
import test from 'node:test';

import {
  collapseProjectFolderRenderCards,
  shouldProjectAgentNotification,
} from './notificationAggregation';

test('routine project-folder watcher traces stay in activity but do not consume notification capacity', () => {
  assert.equal(shouldProjectAgentNotification({
    sessionId: 'project-folder-watcher',
    actor: { label: 'folder-sync' },
    severity: 'info',
    state: 'active',
    requiresAttention: false,
  }), false);
});

test('project-folder watcher failures remain eligible when local recovery card is unavailable', () => {
  assert.equal(shouldProjectAgentNotification({
    sessionId: 'project-folder-watcher',
    actor: { label: 'folder-sync' },
    severity: 'error',
    state: 'failed',
    requiresAttention: true,
  }), true);
});

test('actor label alone cannot suppress another session notification', () => {
  assert.equal(shouldProjectAgentNotification({
    sessionId: 'agent-session',
    actor: { label: 'folder-sync' },
    severity: 'info',
    state: 'active',
    requiresAttention: false,
  }), true);
});

test('project-folder render lifecycle collapses trace noise into the local card', () => {
  const cards = collapseProjectFolderRenderCards([
    { eventId: 'tool', threadId: 'thread-1', actorLabel: 'folder-sync', local: false, activityKind: 'tool_start' },
    { eventId: 'backend', threadId: 'thread-1', actorLabel: 'folder-sync', local: false, activityKind: 'backend_resolved' },
    { eventId: 'heal', threadId: 'thread-1', actorLabel: 'folder-sync', local: false, activityKind: 'auto_heal_applied' },
    { eventId: 'source', threadId: 'thread-1', actorLabel: 'ECKY', local: true, activityKind: null },
  ], 'thread-1');

  assert.deepEqual(cards.map((card) => card.eventId), ['source']);
});

test('project-folder render lifecycle preserves unrelated activity and errors', () => {
  const cards = collapseProjectFolderRenderCards([
    { eventId: 'other-thread', threadId: 'thread-2', actorLabel: 'folder-sync', local: false, activityKind: 'tool_start' },
    { eventId: 'prompt', threadId: 'thread-1', actorLabel: 'Codex', local: false, activityKind: 'request_user_prompt' },
    { eventId: 'error', threadId: 'thread-1', actorLabel: 'Codex', local: false, activityKind: 'tool_error' },
    { eventId: 'source', threadId: 'thread-1', actorLabel: 'ECKY', local: true, activityKind: null },
  ], 'thread-1');

  assert.deepEqual(cards.map((card) => card.eventId), ['other-thread', 'prompt', 'error', 'source']);
});
