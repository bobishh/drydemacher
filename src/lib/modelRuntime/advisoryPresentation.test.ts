import assert from 'node:assert/strict';
import test from 'node:test';
import type { Advisory } from '../types/domain';
import { summarizeAdvisories } from './advisoryPresentation';

test('summarizeAdvisories groups repeated generated warnings and names affected controls', () => {
  const advisories: Advisory[] = [
    {
      advisoryId: 'advisory-gap-a-clearance',
      label: 'Low clearance',
      severity: 'warning',
      primitiveIds: ['gap-a'],
      message: 'Clearance is below the recommended fit range.',
    },
    {
      advisoryId: 'advisory-gap-b-clearance',
      label: 'Low clearance',
      severity: 'warning',
      primitiveIds: ['gap-b'],
      message: 'Clearance is below the recommended fit range.',
    },
  ];

  const summaries = summarizeAdvisories(advisories, [
    { primitiveId: 'gap-a', label: 'Door gap' },
    { primitiveId: 'gap-b', label: 'Latch gap' },
  ]);

  assert.equal(summaries.length, 1);
  assert.equal(summaries[0]?.count, 2);
  assert.deepEqual(summaries[0]?.affectedLabels, ['Door gap', 'Latch gap']);
});

test('summarizeAdvisories keeps manual warnings independently deletable', () => {
  const advisories: Advisory[] = [
    {
      advisoryId: 'advisory-manual-a',
      label: 'Review fit',
      severity: 'warning',
      primitiveIds: ['gap-a'],
      message: 'Check before export.',
    },
    {
      advisoryId: 'advisory-manual-b',
      label: 'Review fit',
      severity: 'warning',
      primitiveIds: ['gap-b'],
      message: 'Check before export.',
    },
  ];

  assert.equal(summarizeAdvisories(advisories, []).length, 2);
});
