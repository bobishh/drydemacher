import { writable } from 'svelte/store';

import type { AgentActivityEvent } from '../tauri/contracts';

type LongTaskRaw = {
  kind?: string;
  taskId?: string;
  expectedDurationMs?: number;
  stage?: string;
  progressCurrent?: number;
  progressTotal?: number;
  jobId?: string;
  cancellable?: boolean;
};

export type LongTask = {
  taskId: string;
  eventId: string;
  sessionId: string;
  threadId: string | null;
  summary: string;
  detail: string | null;
  stage: string;
  startedAt: number;
  updatedAt: number;
  elapsedMs: number;
  expectedDurationMs: number | null;
  progress: { current: number; total: number } | null;
  jobId: string | null;
  cancellable: boolean;
};

function rawPayload(event: AgentActivityEvent): LongTaskRaw {
  if (!event.raw) return {};
  try {
    const value = JSON.parse(event.raw);
    return value && typeof value === 'object' && !Array.isArray(value) ? value as LongTaskRaw : {};
  } catch {
    return {};
  }
}

function occurredAtMs(value: number): number {
  return value >= 1_000_000_000 && value < 1_000_000_000_000 ? value * 1000 : value;
}

function eventKind(event: AgentActivityEvent): string {
  return rawPayload(event).kind ?? '';
}

export function isLongTaskEvent(event: AgentActivityEvent): boolean {
  const kind = eventKind(event);
  return kind.startsWith('long_task_') || kind === 'session_activity_set' || kind === 'session_activity_clear';
}

export function isActiveLongTaskEvent(event: AgentActivityEvent): boolean {
  if (!isLongTaskEvent(event)) return false;
  const kind = eventKind(event);
  return event.state === 'active' && kind !== 'long_task_finished' && kind !== 'session_activity_clear';
}

function taskKey(event: AgentActivityEvent, raw: LongTaskRaw): string {
  if (raw.kind === 'session_activity_set' || raw.kind === 'session_activity_clear') {
    return `session:${event.sessionId}`;
  }
  return raw.taskId?.trim() || event.lifecycleKey?.replace(/^long-task:/, '') || `session:${event.sessionId}`;
}

function terminal(event: AgentActivityEvent, kind: string): boolean {
  return event.state !== 'active' || kind === 'long_task_finished' || kind === 'session_activity_clear';
}

export function projectLongTasks(events: AgentActivityEvent[], now: number): LongTask[] {
  const tasks = new Map<string, Omit<LongTask, 'elapsedMs'>>();
  const ordered = [...events].sort((left, right) => left.cursor - right.cursor);
  for (const event of ordered) {
    if (!isLongTaskEvent(event)) continue;
    const raw = rawPayload(event);
    const key = taskKey(event, raw);
    if (terminal(event, raw.kind ?? '')) {
      tasks.delete(key);
      continue;
    }
    const previous = tasks.get(key);
    const updatedAt = occurredAtMs(event.occurredAt);
    const current = Number(raw.progressCurrent);
    const total = Number(raw.progressTotal);
    tasks.set(key, {
      taskId: key,
      eventId: event.eventId,
      sessionId: event.sessionId,
      threadId: event.threadId ?? null,
      summary: event.summary,
      detail: event.detail ?? null,
      stage: raw.stage?.trim() || event.phase?.trim().toUpperCase() || 'WORKING',
      startedAt: previous?.startedAt ?? updatedAt,
      updatedAt,
      expectedDurationMs: Number.isFinite(raw.expectedDurationMs) ? Number(raw.expectedDurationMs) : null,
      progress: Number.isFinite(current) && Number.isFinite(total) && total > 0
        ? { current, total }
        : previous?.progress ?? null,
      jobId: raw.jobId?.trim() || previous?.jobId || null,
      cancellable: Boolean(raw.cancellable ?? previous?.cancellable),
    });
  }
  return [...tasks.values()]
    .map((task) => ({ ...task, elapsedMs: Math.max(0, now - task.startedAt) }))
    .sort((left, right) => left.startedAt - right.startedAt || left.taskId.localeCompare(right.taskId));
}

export function createLongTasksStore(now: () => number = () => Date.now()) {
  const store = writable<LongTask[]>([]);
  const events: AgentActivityEvent[] = [];
  const seen = new Set<string>();

  function publish() {
    store.set(projectLongTasks(events, now()));
  }

  const interval = globalThis.setInterval(publish, 1000);
  if (typeof interval === 'object' && typeof interval.unref === 'function') interval.unref();

  return {
    subscribe: store.subscribe,
    ingest(input: AgentActivityEvent | AgentActivityEvent[]) {
      for (const event of Array.isArray(input) ? input : [input]) {
        if (seen.has(event.eventId)) continue;
        seen.add(event.eventId);
        events.push(event);
      }
      if (events.length > 4_096) {
        events.splice(0, events.length - 4_096);
        seen.clear();
        for (const event of events) seen.add(event.eventId);
      }
      publish();
    },
    clear() {
      events.length = 0;
      seen.clear();
      publish();
    },
    stop() {
      globalThis.clearInterval(interval);
    },
  };
}

export const longTasksStore = createLongTasksStore();
