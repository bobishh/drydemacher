import type { SessionEvent } from '../sessionActivity';

export type ThreadAgentPresentationState = {
  connectionState: 'none' | 'sleeping' | 'waking' | 'waiting' | 'active' | 'disconnected' | 'error';
  agentLabel: string | null;
  llmModelLabel: string | null;
  providerKind?: string | null;
  sessionId?: string | null;
  phase: string | null;
  statusText: string | null;
  busy?: boolean;
  activityLabel?: string | null;
  activityStartedAt?: number | null;
  attentionKind?: string | null;
  waitingOnPrompt?: boolean;
  updatedAt: number | null;
};

function latestEvent(events: SessionEvent[]): SessionEvent | null {
  if (!events.length) return null;
  const sorted = [...events].sort((left, right) => {
    const leftCursor = left.cursor ?? null;
    const rightCursor = right.cursor ?? null;
    if (leftCursor !== null && rightCursor !== null && leftCursor !== rightCursor) {
      return leftCursor - rightCursor;
    }
    if (left.timestamp !== right.timestamp) return left.timestamp - right.timestamp;
    return left.id.localeCompare(right.id);
  });
  return sorted[sorted.length - 1] ?? null;
}

function activityEvents(events: SessionEvent[], threadId: string | null): SessionEvent[] {
  return events.filter((event) => {
    if (event.cursor === null || event.cursor === undefined) return false;
    if (threadId && event.threadId !== threadId) return false;
    return true;
  });
}

function normalizeText(value: string | null | undefined): string | null {
  const text = `${value ?? ''}`.replace(/\s+/g, ' ').trim();
  return text || null;
}

function runtimeMetadata(raw: unknown): Record<string, unknown> {
  if (typeof raw === 'string') {
    try {
      const parsed = JSON.parse(raw);
      return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
        ? parsed as Record<string, unknown>
        : {};
    } catch {
      return {};
    }
  }
  return raw && typeof raw === 'object' && !Array.isArray(raw)
    ? raw as Record<string, unknown>
    : {};
}

function optionalText(value: unknown): string | null {
  return typeof value === 'string' ? normalizeText(value) : null;
}

export function projectThreadAgentStateFromSessionEvents(
  events: SessionEvent[],
  threadId: string | null,
): ThreadAgentPresentationState {
  const threadEvents = activityEvents(events, threadId);
  const latest = latestEvent(threadEvents);

  if (!latest) {
    return {
      connectionState: 'none',
      agentLabel: null,
      llmModelLabel: null,
      providerKind: null,
      sessionId: null,
      phase: null,
      statusText: null,
      busy: false,
      activityLabel: null,
      activityStartedAt: null,
      attentionKind: null,
      waitingOnPrompt: false,
      updatedAt: null,
    };
  }

  const metadata = runtimeMetadata(latest.raw);
  const waitingOnPrompt =
    typeof metadata.waitingOnPrompt === 'boolean'
      ? metadata.waitingOnPrompt
      : Boolean(latest.requiresAttention) || latest.phase === 'waiting_for_user';
  const normalizedPhase = normalizeText(latest.phase)?.toLowerCase() ?? '';
  const connectionState =
    normalizedPhase === 'sleeping'
      ? 'sleeping'
      : normalizedPhase === 'waking'
        ? 'waking'
        : normalizedPhase === 'disconnected'
          ? 'disconnected'
          : normalizedPhase === 'error' || latest.state === 'failed'
        ? 'error'
        : waitingOnPrompt || normalizedPhase === 'waiting'
          ? 'waiting'
          : 'active';

  return {
    connectionState,
    agentLabel: latest.actor.kind === 'agent' ? latest.actor.label : null,
    llmModelLabel: optionalText(metadata.llmModelLabel),
    providerKind: optionalText(metadata.providerKind),
    sessionId: latest.sessionId,
    phase: latest.phase ?? null,
    statusText: normalizeText(latest.detail) ?? normalizeText(latest.summary),
    busy:
      typeof metadata.busy === 'boolean'
        ? metadata.busy && !waitingOnPrompt
        : latest.state === 'active' && !waitingOnPrompt,
    activityLabel: optionalText(metadata.activityLabel) ?? normalizeText(latest.summary),
    activityStartedAt:
      typeof metadata.activityStartedAt === 'number'
        ? metadata.activityStartedAt
        : latest.state === 'active' ? latest.timestamp : null,
    attentionKind: optionalText(metadata.attentionKind) ?? (latest.requiresAttention ? 'prompt' : null),
    waitingOnPrompt,
    updatedAt: latest.timestamp,
  };
}
