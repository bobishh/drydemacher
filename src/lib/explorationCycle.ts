import type { CyclePacket, CyclePhase, CycleStatus } from './tauri/contracts';

/** UI projection of Rust-owned exploration state. */
export type ExplorationPhase = CyclePhase | Exclude<CycleStatus, 'active'>;

export type ExplorationCycleStatus = CycleStatus;

export type ExplorationBudget = {
  limit: number;
  used: number;
  remaining: number;
};

export type ExplorationPendingQuestion = {
  question: string;
  blockedDecision: string;
  cycleId: string;
  currentVersionId: string | null;
};

export type ExplorationCyclePacket = {
  cycleId: string;
  threadId: string;
  baseVersionId: string;
  currentVersionId: string | null;
  chosenVersionId: string | null;
  objective: string;
  acceptanceCriteria: string[];
  hardConstraints: string[];
  softPreferences: string[];
  phase: ExplorationPhase;
  status: ExplorationCycleStatus;
  budget: ExplorationBudget;
  hypothesis: string | null;
  pendingQuestion: ExplorationPendingQuestion | null;
  lastEvidenceRef: string | null;
  promptVersion: string;
};

export type ExplorationCycleUiCopy = {
  phase: string;
  budget: string;
  hypothesis: string | null;
  pendingQuestion: string | null;
  runningBuild: null;
  pendingBuilds: null;
};

export type VersionComparison = {
  leftVersionId: string | null;
  rightVersionId: string | null;
  sameVersion: boolean;
};

/** Flatten generated Rust state for rendering. No transport healing or lifecycle decisions. */
export function projectExplorationCyclePacket(packet: CyclePacket): ExplorationCyclePacket {
  const { state, definition } = packet;
  const phase: ExplorationPhase = state.phase === 'idle' && state.status !== 'active'
    ? state.status
    : state.phase;
  const pendingQuestion = state.pendingQuestion
    ? {
        question: state.pendingQuestion,
        blockedDecision: '',
        cycleId: state.cycleId,
        currentVersionId: state.currentVersionId || null,
      }
    : null;

  return {
    cycleId: state.cycleId,
    threadId: state.threadId,
    baseVersionId: packet.baseVersionId,
    currentVersionId: state.currentVersionId || null,
    chosenVersionId: state.chosenVersionId ?? null,
    objective: definition.objective,
    acceptanceCriteria: definition.acceptanceCriteria ?? [],
    hardConstraints: definition.hardConstraints ?? [],
    softPreferences: definition.softPreferences ?? [],
    phase,
    status: state.status,
    budget: {
      limit: state.budget,
      used: state.budgetUsed,
      remaining: Math.max(0, state.budget - state.budgetUsed),
    },
    hypothesis: packet.hypothesis ?? null,
    pendingQuestion,
    lastEvidenceRef: state.lastEvidenceRef ?? null,
    promptVersion: packet.promptVersion,
  };
}

/** Compact copy for Ecky bubble/status projections. */
export function explorationCycleUiCopy(packet: ExplorationCyclePacket): ExplorationCycleUiCopy {
  return {
    phase: packet.phase.toUpperCase(),
    budget: `${packet.budget.remaining} BUILDS REMAINING`,
    hypothesis: packet.hypothesis,
    pendingQuestion: packet.pendingQuestion?.question ?? null,
    runningBuild: null,
    pendingBuilds: null,
  };
}

/** Compare ordinary immutable version IDs. No candidate/promotion tier exists. */
export function compareVersionRefs(refs: readonly string[]): VersionComparison {
  const normalized = refs
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
  const leftVersionId = normalized[0] ?? null;
  const rightVersionId = normalized[1] ?? null;
  return {
    leftVersionId,
    rightVersionId,
    sameVersion: leftVersionId !== null && rightVersionId !== null && leftVersionId === rightVersionId,
  };
}
