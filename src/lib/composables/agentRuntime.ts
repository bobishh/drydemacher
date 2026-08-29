export type AgentControl = 'wake' | 'stop' | 'restart';

export type AgentTarget = {
  messageId: string | null;
  modelId: string | null;
};

type AgentRuntimeDeps = {
  hasIpc: () => boolean;
  setError: (message: string) => void;
};

/** Agent control routing. App retains UI state and terminal wiring. */
export function createAgentRuntime(deps: AgentRuntimeDeps) {
  async function runControl(
    control: AgentControl,
    threadId: string | null,
    target: AgentTarget,
    invoke: (threadId: string, messageId: string | null, modelId: string | null) => Promise<void>,
  ): Promise<void> {
    if (!threadId) return;
    try {
      await invoke(threadId, target.messageId, target.modelId);
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      deps.setError(`Agent ${control[0].toUpperCase()}${control.slice(1)} Error: ${detail}`);
    }
  }

  return { runControl };
}
