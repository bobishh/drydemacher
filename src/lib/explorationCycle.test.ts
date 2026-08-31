import assert from 'node:assert/strict';
import test from 'node:test';
import type { CyclePacket } from './tauri/contracts';
import {
  compareVersionRefs,
  explorationCycleUiCopy,
  projectExplorationCyclePacket,
} from './explorationCycle';

function cyclePacket(overrides: Partial<CyclePacket> = {}): CyclePacket {
  return {
    state: {
      cycleId: 'cycle-1',
      threadId: 'thread-1',
      phase: 'awaitingInput',
      status: 'active',
      currentVersionId: 'version-b',
      chosenVersionId: null,
      pendingQuestion: 'Keep the mounting face fixed?',
      lastEvidenceRef: 'evidence-1',
      budget: 4,
      budgetUsed: 2,
    },
    baseVersionId: 'version-a',
    definition: {
      objective: 'Repair the wall bracket',
      acceptanceCriteria: ['minimum wall 4mm'],
      hardConstraints: ['keep mounting holes'],
      softPreferences: ['minimize mass'],
    },
    hypothesis: 'The red draft has an unclosed expression.',
    lastVerification: null,
    lastRoute: null,
    eventCount: 4,
    promptVersion: 'exploration-v1',
    ...overrides,
  };
}

test('projects the generated camelCase Rust packet without creating lifecycle state', () => {
  const packet = projectExplorationCyclePacket(cyclePacket());

  assert.deepEqual(packet, {
    cycleId: 'cycle-1',
    threadId: 'thread-1',
    baseVersionId: 'version-a',
    currentVersionId: 'version-b',
    chosenVersionId: null,
    objective: 'Repair the wall bracket',
    acceptanceCriteria: ['minimum wall 4mm'],
    hardConstraints: ['keep mounting holes'],
    softPreferences: ['minimize mass'],
    phase: 'awaitingInput',
    status: 'active',
    budget: { limit: 4, used: 2, remaining: 2 },
    hypothesis: 'The red draft has an unclosed expression.',
    pendingQuestion: {
      question: 'Keep the mounting face fixed?',
      blockedDecision: '',
      cycleId: 'cycle-1',
      currentVersionId: 'version-b',
    },
    lastEvidenceRef: 'evidence-1',
    promptVersion: 'exploration-v1',
  });
});

test('does not heal legacy transport field names in the frontend', () => {
  const canonical = cyclePacket();
  const state = canonical.state as unknown as Record<string, unknown>;
  delete state.lastEvidenceRef;
  state[['last', 'evidence', 'ref'].join('_')] = 'legacy-evidence';

  const packet = projectExplorationCyclePacket(canonical);

  assert.equal(packet.lastEvidenceRef, null);
});

test('maps idle terminal controller states to terminal UI phases', () => {
  for (const status of ['completed', 'stopped', 'interrupted'] as const) {
    const packet = projectExplorationCyclePacket(cyclePacket({
      state: {
        ...cyclePacket().state,
        phase: 'idle',
        status,
        pendingQuestion: null,
      },
    }));
    assert.equal(packet.status, status);
    assert.equal(packet.phase, status);
  }
});

test('projects controller phase and budget into compact UI copy', () => {
  const packet = projectExplorationCyclePacket(cyclePacket({
    state: {
      ...cyclePacket().state,
      phase: 'building',
      pendingQuestion: null,
    },
  }));

  assert.deepEqual(explorationCycleUiCopy(packet), {
    phase: 'BUILDING',
    budget: '2 BUILDS REMAINING',
    hypothesis: 'The red draft has an unclosed expression.',
    pendingQuestion: null,
    runningBuild: null,
    pendingBuilds: null,
  });
});

test('compares ordinary immutable version refs without candidate or promotion semantics', () => {
  assert.deepEqual(compareVersionRefs(['version-a', 'version-b', 'version-a']), {
    leftVersionId: 'version-a',
    rightVersionId: 'version-b',
    sameVersion: false,
  });
  assert.deepEqual(compareVersionRefs(['version-c', 'version-c']), {
    leftVersionId: 'version-c',
    rightVersionId: 'version-c',
    sameVersion: true,
  });
  assert.deepEqual(compareVersionRefs(['version-c']), {
    leftVersionId: 'version-c',
    rightVersionId: null,
    sameVersion: false,
  });
});
