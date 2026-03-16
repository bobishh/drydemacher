import type { AgentTerminalSnapshot } from '../types/domain';
import type { ThreadAgentState } from '../tauri/client';

export function isThreadAgentBusy(
  state: ThreadAgentState | null | undefined,
): boolean {
  return Boolean(
    state &&
      state.connectionState === 'active' &&
      state.busy &&
      !state.waitingOnPrompt &&
      !state.awaitingPromptRearm,
  );
}

export function formatAgentActivityElapsed(
  activityStartedAt: number | null | undefined,
  nowSecs: number,
): string | null {
  if (!activityStartedAt) return null;
  const elapsed = Math.max(0, nowSecs - activityStartedAt);
  const hours = Math.floor(elapsed / 3600);
  const minutes = Math.floor((elapsed % 3600) / 60);
  const seconds = elapsed % 60;
  if (hours > 0) {
    return `${hours}h ${String(minutes).padStart(2, '0')}m`;
  }
  return `${minutes}m ${String(seconds).padStart(2, '0')}s`;
}

function withElapsed(
  text: string | null | undefined,
  activityStartedAt: number | null | undefined,
  nowSecs: number,
): string {
  const normalized = `${text ?? ''}`.replace(/\s+/g, ' ').trim();
  if (!normalized) return '';
  const elapsed = formatAgentActivityElapsed(activityStartedAt, nowSecs);
  return elapsed ? `${normalized} · ${elapsed}` : normalized;
}

export function resolveActiveMcpBubble(input: {
  threadAgentState: ThreadAgentState | null | undefined;
  visibleAgentTerminal: AgentTerminalSnapshot | null | undefined;
  cookingPhrase: string | null | undefined;
  nowSecs: number;
}): string {
  const state = input.threadAgentState;
  if (!state) return '';

  const activityStartedAt =
    state.activityStartedAt ??
    input.visibleAgentTerminal?.activityStartedAt ??
    null;
  const activityLabel =
    state.activityLabel?.trim() ||
    input.visibleAgentTerminal?.activityLabel?.trim() ||
    '';
  if (activityLabel) {
    return withElapsed(activityLabel, activityStartedAt, input.nowSecs);
  }

  const cookingPhrase = `${input.cookingPhrase ?? ''}`.trim();
  if (isThreadAgentBusy(state) && cookingPhrase) {
    return withElapsed(cookingPhrase, activityStartedAt, input.nowSecs);
  }

  const sanitizedTerminalSummary = `${input.visibleAgentTerminal?.summary ?? ''}`.trim();
  if (sanitizedTerminalSummary) {
    return withElapsed(sanitizedTerminalSummary, activityStartedAt, input.nowSecs);
  }

  return (
    `${state.latestTraceSummary ?? ''}`.trim() ||
    `${state.statusText ?? ''}`.trim()
  );
}

export function resolveTerminalActivityMeta(input: {
  threadAgentState: ThreadAgentState | null | undefined;
  visibleAgentTerminal: AgentTerminalSnapshot | null | undefined;
  nowSecs: number;
}): string {
  const state = input.threadAgentState;
  const terminal = input.visibleAgentTerminal;
  const base =
    state?.activityLabel?.trim() ||
    terminal?.activityLabel?.trim() ||
    terminal?.summary?.trim() ||
    state?.latestTraceSummary?.trim() ||
    state?.statusText?.trim() ||
    '';
  return withElapsed(
    base,
    state?.activityStartedAt ?? terminal?.activityStartedAt ?? null,
    input.nowSecs,
  );
}
