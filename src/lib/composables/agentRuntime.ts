export type AgentControl = 'wake' | 'stop' | 'restart';

export type AgentTarget = {
  messageId: string | null;
  modelId: string | null;
};

type AgentRuntimeDeps<T> = {
  hasIpc: () => boolean;
  getState: (threadId: string) => Promise<T>;
  setState: (state: T | null) => void;
  setError: (message: string) => void;
};

/** Agent polling and control routing. App retains UI state and terminal wiring. */
export function createAgentRuntime<T>(deps: AgentRuntimeDeps<T>) {
  async function refresh(threadId: string | null): Promise<void> {
    if (!deps.hasIpc() || !threadId) {
      deps.setState(null);
      return;
    }
    try {
      deps.setState(await deps.getState(threadId));
    } catch {
      deps.setState(null);
    }
  }

  async function runControl(
    control: AgentControl,
    threadId: string | null,
    target: AgentTarget,
    invoke: (threadId: string, messageId: string | null, modelId: string | null) => Promise<void>,
  ): Promise<void> {
    if (!threadId) return;
    try {
      await invoke(threadId, target.messageId, target.modelId);
      await refresh(threadId);
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      deps.setError(`Agent ${control[0].toUpperCase()}${control.slice(1)} Error: ${detail}`);
    }
  }

  return { refresh, runControl };
}
