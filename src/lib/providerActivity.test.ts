import assert from 'node:assert/strict';
import test from 'node:test';

import { collapseProviderActivity, projectProviderTurnMessages } from './providerActivity';

test('Given active provider events When projected Then latest event is current and full order is retained', () => {
  const grouped = collapseProviderActivity({
    providerId: 'codex',
    providerLabel: 'Codex',
    externalConversationId: 'codex-7',
    activeTurnId: 'turn-9',
    messages: [
      { id: 'one', role: 'assistant', content: 'THINKING · Inspecting constraints', status: 'working', timestamp: 10 },
      { id: 'two', role: 'assistant', content: 'I am checking the authored fit.', status: 'working', timestamp: 11 },
      { id: 'three', role: 'assistant', content: 'USING TOOL · ecky_ast_inspect', status: 'working', timestamp: 12 },
    ],
  });

  assert.equal(grouped?.id, 'provider-working:codex:codex-7:turn-9');
  assert.equal(grouped?.status, 'working');
  assert.equal(grouped?.content, 'USING TOOL · ecky_ast_inspect');
  assert.deepEqual(grouped?.providerActivity, {
    providerLabel: 'Codex',
    summary: 'USING TOOL · ecky_ast_inspect',
    phase: 'active',
    items: ['THINKING · Inspecting constraints', 'I am checking the authored fit.', 'USING TOOL · ecky_ast_inspect'],
  });
});

test('Given no public provider action When projected Then no placeholder is invented', () => {
  assert.equal(collapseProviderActivity({
    providerId: 'agy',
    providerLabel: 'Agy',
    externalConversationId: 'agy-8',
    activeTurnId: 'turn-1',
    messages: [],
  }), null);
});

test('Given repeated provider events When projected Then arrival sequence stays lossless', () => {
  const grouped = collapseProviderActivity({
    providerId: 'codex',
    providerLabel: 'Codex',
    externalConversationId: 'codex-7',
    activeTurnId: 'turn-10',
    phase: 'interrupted',
    messages: [
      { id: 'one', role: 'assistant', content: 'Read files', status: 'discarded', timestamp: 10 },
      { id: 'two', role: 'assistant', content: 'Read files', status: 'discarded', timestamp: 11 },
    ],
  });

  assert.deepEqual(grouped?.providerActivity?.items, ['Read files', 'Read files']);
  assert.equal(grouped?.providerActivity?.phase, 'interrupted');
});

test('Given active provider speech between actions When projected Then speech stays a normal message and WORKING contains actions only', () => {
  const projected = projectProviderTurnMessages({
    providerId: 'agy',
    providerLabel: 'Agy',
    externalConversationId: 'agy-8',
    activeTurnId: 'turn-11',
    phase: 'active',
    messages: [
      { id: 'action-1', role: 'assistant', content: 'WORKING · inspecting', status: 'working', timestamp: 10, providerEventKind: 'activity' },
      { id: 'speech-1', role: 'assistant', content: 'Сначала проверю ограничения.', status: 'working', timestamp: 11, providerEventKind: 'assistant' },
      { id: 'action-2', role: 'assistant', content: 'USING TOOL · grep_search', status: 'working', timestamp: 12, providerEventKind: 'activity' },
    ],
  });

  assert.equal(projected.length, 2);
  assert.equal(projected[0].providerActivity, undefined);
  assert.equal(projected[0].content, 'Сначала проверю ограничения.');
  assert.deepEqual(projected[1].providerActivity?.items, [
    'WORKING · inspecting',
    'USING TOOL · grep_search',
  ]);
});

test('Given interrupted provider speech between actions When projected Then original event order remains visible', () => {
  const projected = projectProviderTurnMessages({
    providerId: 'agy',
    providerLabel: 'Agy',
    externalConversationId: 'agy-8',
    activeTurnId: 'turn-12',
    phase: 'interrupted',
    messages: [
      { id: 'action-1', role: 'assistant', content: 'WORKING · inspecting', status: 'discarded', timestamp: 10, providerEventKind: 'activity' },
      { id: 'speech-1', role: 'assistant', content: 'Не успел закончить проверку.', status: 'discarded', timestamp: 11, providerEventKind: 'assistant' },
      { id: 'action-2', role: 'assistant', content: 'USING TOOL · grep_search', status: 'discarded', timestamp: 12, providerEventKind: 'activity' },
    ],
  });

  assert.deepEqual(projected.map((message) => message.content), [
    'WORKING · inspecting',
    'Не успел закончить проверку.',
    'USING TOOL · grep_search',
  ]);
  assert.deepEqual(projected.map((message) => Boolean(message.providerActivity)), [true, false, true]);
});
