import test from 'node:test';
import assert from 'node:assert/strict';
import { createAgentRuntime } from './agentRuntime';

test('Given no active thread When polling Then no agent-state request is made and state clears', async () => {
  let calls = 0;
  let state: string | null = 'old';
  const runtime = createAgentRuntime({
    hasIpc: () => true,
    getState: async () => { calls += 1; return 'next'; },
    setState: (next) => { state = next; },
    setError: () => {},
  });

  await runtime.refresh(null);

  assert.equal(calls, 0);
  assert.equal(state, null);
});

test('Given an active thread When wake succeeds Then it routes current target and refreshes state', async () => {
  const calls: string[] = [];
  const runtime = createAgentRuntime({
    hasIpc: () => true,
    getState: async (threadId) => { calls.push(`state:${threadId}`); return 'active'; },
    setState: () => {},
    setError: () => {},
  });

  await runtime.runControl('wake', 'thread-1', { messageId: 'message-1', modelId: 'model-1' }, async (threadId, messageId, modelId) => {
    calls.push(`wake:${threadId}:${messageId}:${modelId}`);
  });

  assert.deepEqual(calls, ['wake:thread-1:message-1:model-1', 'state:thread-1']);
});
