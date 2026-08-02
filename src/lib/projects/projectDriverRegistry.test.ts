import assert from 'node:assert/strict';
import test from 'node:test';
import { campaignRunProjectDriver, designProjectDriver } from './projectDriverRegistry';

test('Given design and campaign projects When projected for Projects Then each keeps its own identity and summary', () => {
  const design = designProjectDriver.card({
    id: 'thread-a', title: 'Bracket', updatedAt: 10, versionCount: 3,
  });
  const campaign = campaignRunProjectDriver.card({
    id: 'run-b', title: 'Ecky IR build missions', currentStepId: 'mission-01/why-stock', completedStepIds: ['mission-00/start'], updatedAt: 11,
  });

  assert.deepEqual(design, { kind: 'design', id: 'thread-a', title: 'Bracket', updatedAt: 10, progress: '3 versions' });
  assert.deepEqual(campaign, { kind: 'campaignRun', id: 'run-b', title: 'Ecky IR build missions', updatedAt: 11, progress: '1 step complete' });
});
