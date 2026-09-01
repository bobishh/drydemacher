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

test('Given an accepted provider turn before its first event When projected Then receipt activity is visible', () => {
  const projected = projectProviderTurnMessages({
    providerId: 'codex',
    providerLabel: 'Codex',
    externalConversationId: 'codex-7',
    activeTurnId: 'turn-starting',
    phase: 'active',
    messages: [],
  });

  assert.equal(projected.length, 1);
  assert.equal(projected[0].id, 'provider-working:codex:codex-7:turn-starting');
  assert.deepEqual(projected[0].providerActivity, {
    providerLabel: 'Codex',
    summary: 'THINKING · Message received. Starting work.',
    phase: 'active',
    items: ['THINKING · Message received. Starting work.'],
  });
});

test('Given provider thinking arrives after receipt When projected Then exact provider activity replaces fallback', () => {
  const projected = projectProviderTurnMessages({
    providerId: 'agy',
    providerLabel: 'Agy',
    externalConversationId: 'agy-8',
    activeTurnId: 'turn-thinking',
    phase: 'active',
    messages: [
      {
        id: 'thinking-1',
        role: 'assistant',
        content: 'THINKING · Inspecting current constraints.',
        status: 'working',
        timestamp: 10,
        providerEventKind: 'activity',
      },
    ],
  });

  assert.deepEqual(projected[0].providerActivity?.items, [
    'THINKING · Inspecting current constraints.',
  ]);
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
