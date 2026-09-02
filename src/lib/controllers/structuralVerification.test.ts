import assert from 'node:assert/strict';
import test from 'node:test';

import { deriveAuthoredVerifyChips } from './structuralVerification';
import type { StructuralVerificationResult } from '../types/domain';

const RESULT: StructuralVerificationResult = {
  passed: false,
  summary: 'Authored verification failed.',
  issues: [],
  metrics: { partCount: 1, modelStlSizeBytes: 1024, totalVolume: 1000, totalArea: 600, bbox: null },
  verifierStatus: 'ok',
  authoredVerifyChecks: [
    {
      tag: 'step_export',
      status: 'passed',
      severity: 'error',
      message: 'true = true',
      stableNodeId: 'verify:step_export',
      metricSource: 'manifest',
      metricKey: 'has-step',
      comparator: '==',
      expected: { kind: 'boolean', value: true },
      actual: { kind: 'boolean', value: true },
    },
    {
      tag: 'bad_clearance',
      status: 'failed',
      severity: 'error',
      message: '0.12 is below 0.3',
      stableNodeId: null,
      metricSource: 'clearance',
      metricKey: 'min-distance',
      comparator: '>=',
      expected: { kind: 'number', value: 0.3 },
      actual: { kind: 'number', value: 0.12 },
    },
  ],
};

test('projects backend authored checks into stable UI chips', () => {
  assert.deepEqual(deriveAuthoredVerifyChips(RESULT), [
    {
      id: 'verify:step_export',
      label: 'step_export',
      status: 'passed',
      tone: 'green',
      message: 'manifest has-step expected == true; actual true',
      stableNodeId: 'verify:step_export',
    },
    {
      id: 'authored-verify:bad_clearance',
      label: 'bad_clearance',
      status: 'failed',
      tone: 'red',
      message: 'clearance min-distance expected >= 0.3; actual 0.12',
      stableNodeId: null,
    },
  ]);
});

test('projects no chips when backend evidence is absent', () => {
  assert.deepEqual(deriveAuthoredVerifyChips(null), []);
});

test('projects warning failures amber and skipped checks neutral with intent evidence', () => {
  const result: StructuralVerificationResult = {
    ...RESULT,
    passed: true,
    authoredVerifyChecks: [
      {
        tag: 'triangle-budget',
        status: 'failed',
        severity: 'warning',
        intent: 'Keep preview responsive',
        message: '12000 > 10000',
      },
      {
        tag: 'assembly-connected',
        status: 'skipped',
        severity: 'error',
        intent: 'Assembly must be connected',
        condition: 'assembly-preview',
        conditionResult: false,
        skipReason: 'Authored `when` condition resolved false.',
        message: 'Skipped.',
      },
    ],
  };

  assert.deepEqual(deriveAuthoredVerifyChips(result), [
    {
      id: 'authored-verify:triangle-budget',
      label: 'triangle-budget',
      status: 'failed',
      tone: 'amber',
      message: 'Keep preview responsive — 12000 > 10000',
      stableNodeId: null,
    },
    {
      id: 'authored-verify:assembly-connected',
      label: 'assembly-connected',
      status: 'skipped',
      tone: 'neutral',
      message:
        'Assembly must be connected — when assembly-preview: false — Authored `when` condition resolved false.',
      stableNodeId: null,
    },
  ]);
});
