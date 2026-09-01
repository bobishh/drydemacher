import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const appSource = readFileSync(new URL('../../src/App.svelte', import.meta.url), 'utf8');
const bindingsSource = readFileSync(
  new URL('../../src-tauri/src/bindings.rs', import.meta.url),
  'utf8',
);
const mcpPromptSource = readFileSync(
  new URL('../../src-tauri/src/mcp/handlers/session.rs', import.meta.url),
  'utf8',
);

test('agent prompt reply submits one Rust-owned resolve-or-queue intent', () => {
  const start = appSource.indexOf('async function answerAgentPrompt');
  const end = appSource.indexOf('async function handleDialogueSubmit', start);
  const handler = appSource.slice(start, end);

  assert.match(handler, /submitAgentPromptReply\s*\(/);
  assert.doesNotMatch(handler, /resolveAgentPrompt\s*\(/);
  assert.doesNotMatch(handler, /queueAgentPrompt\s*\(/);
  assert.doesNotMatch(handler, /No pending prompt request/);
  assert.doesNotMatch(handler, /timed out after/);
  assert.match(bindingsSource, /submit_agent_prompt_reply/);
});

test('queued prompt batch selection and attachment loading stay in Rust request path', () => {
  assert.doesNotMatch(appSource, /collectQueuedThreadBatch/);
  assert.doesNotMatch(appSource, /getMessageAttachments/);
  assert.doesNotMatch(appSource, /autoDrainingPromptRequestIds/);
  assert.match(mcpPromptSource, /auto_deliver_queued_prompt_batch/);
  const channelIndex = mcpPromptSource.indexOf('channels.insert(request_id.clone(), tx)');
  const waitIndex = mcpPromptSource.indexOf('state.prompt_waits.lock().unwrap().insert');
  const drainIndex = mcpPromptSource.indexOf('auto_deliver_queued_prompt_batch');
  const eventIndex = mcpPromptSource.indexOf('if !auto_delivered');
  assert.ok(channelIndex >= 0 && channelIndex < waitIndex);
  assert.ok(waitIndex < drainIndex && drainIndex < eventIndex);
  assert.match(mcpPromptSource, /queued prompt auto-delivery failed; preserving live prompt/);
});
