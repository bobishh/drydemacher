import { fireEvent, render } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import AgentNotificationCenter from '../../src/lib/AgentNotificationCenter.svelte';
import {
  clearAgentNotificationsStore,
  agentNotificationsStore,
} from '../../src/lib/stores/agentNotifications';
import { localNotificationActionsStore } from '../../src/lib/stores/localNotificationActions';
import { historyStore } from '../../src/lib/stores/domainState';

describe('AgentNotificationCenter', () => {
  it('renders all threads in one stack and routes a card to its project dialogue', async () => {
    const openThreadDialogue = vi.fn();
    const openActivityEvent = vi.fn();

    clearAgentNotificationsStore();
    historyStore.set([
      { id: 'thread-active', title: 'Hydrant Cap' },
      { id: 'thread-background', title: 'Valve Body' },
    ] as never);

    agentNotificationsStore.ingest([
      {
        eventId: 'event-1',
        cursor: 1,
        sessionId: 'session-1',
        threadId: 'thread-active',
        messageId: null,
        versionId: null,
        actor: { kind: 'agent', id: 'agent-1', label: 'Alpha' },
        kind: 'trace',
        lifecycleKey: null,
        phase: null,
        summary: 'active thread card',
        detail: 'active detail',
        severity: 'info',
        state: 'resolved',
        requiresAttention: false,
        occurredAt: 1,
        raw: null,
      },
      {
        eventId: 'event-3',
        cursor: 3,
        sessionId: 'session-1',
        threadId: 'thread-background',
        messageId: null,
        versionId: null,
        actor: { kind: 'agent', id: 'agent-2', label: 'Beta' },
        kind: 'trace',
        lifecycleKey: null,
        phase: 'error',
        summary: 'background failure',
        detail: 'raw provider body',
        severity: 'error',
        state: 'failed',
        requiresAttention: true,
        occurredAt: 3,
        raw: null,
      },
      {
        eventId: 'event-2',
        cursor: 2,
        sessionId: 'session-1',
        threadId: 'thread-background',
        messageId: null,
        versionId: null,
        actor: { kind: 'agent', id: 'agent-2', label: 'Beta' },
        kind: 'trace',
        lifecycleKey: null,
        phase: null,
        summary: 'background thread card',
        detail: 'background detail',
        severity: 'question',
        state: 'active',
        requiresAttention: true,
        occurredAt: 2,
        raw: null,
      },
    ]);

    localNotificationActionsStore.set({
      eventId: 'local-folder-error',
      threadId: 'thread-active',
      actorLabel: 'ECKY',
      summary: 'SOURCE APPLY FAILED',
      detail: 'active thread card',
      severity: 'error',
      state: 'failed',
      requiresAttention: true,
      actions: [],
    });

    const { getByRole, getAllByRole, getByText, getAllByText, queryByText } = render(AgentNotificationCenter, {
      props: {
        activeThreadId: 'thread-active',
        onOpenThreadDialogue: openThreadDialogue,
        onOpenActivityEvent: openActivityEvent,
      },
    });

    expect(getByText('Hydrant Cap')).not.toBeNull();
    expect(getAllByText('Valve Body')).toHaveLength(2);
    expect(queryByText('thread-active')).toBeNull();
    expect(getByText('active thread card')).not.toBeNull();
    expect(getByText('background thread card')).not.toBeNull();
    expect(queryByText('SOURCE APPLY FAILED')).toBeNull();
    expect(getAllByRole('button', { name: /open project dialogue/i })).toHaveLength(1);

    await fireEvent.click(getByRole('button', { name: /open project dialogue for valve body/i }));
    expect(openThreadDialogue).toHaveBeenCalledWith('thread-background');

    await fireEvent.click(getByRole('button', { name: /open activity details for hydrant cap/i }));
    expect(openActivityEvent).toHaveBeenCalledWith('event-1', 'thread-active', false);

    await fireEvent.click(getByRole('button', { name: /open activity details for valve body/i }));
    expect(openActivityEvent).toHaveBeenCalledWith('event-3', 'thread-background', false);

    expect(getByRole('status')).not.toBeNull();
    expect(getByRole('alert')).not.toBeNull();
    localNotificationActionsStore.set(null);
    historyStore.set([]);
  });
});
