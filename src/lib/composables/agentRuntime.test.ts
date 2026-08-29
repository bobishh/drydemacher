import test from 'node:test';
import assert from 'node:assert/strict';
import { createAgentRuntime } from './agentRuntime';

test('Given an active thread When wake succeeds Then it routes current target', async () => {
  const calls: string[] = [];
  const runtime = createAgentRuntime({
    hasIpc: () => true,
    setError: () => {},
  });

  await runtime.runControl('wake', 'thread-1', { messageId: 'message-1', modelId: 'model-1' }, async (threadId, messageId, modelId) => {
    calls.push(`wake:${threadId}:${messageId}:${modelId}`);
  });

  assert.deepEqual(calls, ['wake:thread-1:message-1:model-1']);
});
