import assert from 'node:assert/strict';
import test from 'node:test';
import { braceletFixture, evaluateBraceletGate, evaluateBraceletIncremental } from './benchmark_direct_occt_bracelet.mjs';
import { existsSync } from 'node:fs';

function sample(label, policy, totalElapsedMs, booleanMs, overrides = {}) {
  return {
    label,
    validity: true,
    bounds: { xMin: 0, yMin: 0, zMin: 0, xMax: 10, yMax: 10, zMax: 10 },
    signedVolume: 100,
    components: 3,
    parts: 3,
    topologyDigest: 'topology',
    rawErrors: [],
    step: { exists: true, digest: 'step' },
    stl: { exists: true, digest: 'stl' },
    perPart: {
      'daughter-flower-body': {},
      'daughter-flower-center-lid': {},
      'daughter-tpu-breakaway-strap': {},
    },
    stage: {
      totalElapsedMs,
      parallelPolicy: policy,
      workerBudget: 18,
      peakTotalAllocatedCpuUnits: 18,
      serialBooleanCount: policy === 'outer-only' ? 4 : 0,
      parallelBooleanCount: policy === 'adaptive' ? 4 : 0,
      meshBooleanCount: policy === 'adaptive' ? 2 : 0,
      tessellatedStepPartCount: policy === 'adaptive' ? 2 : 0,
      stages: [{ name: 'boolean', elapsedMs: booleanMs }],
      parts: [],
      partialBooleanGroups: [],
    },
    ...overrides,
  };
}

test('BDD Given frozen bracelet source When acceptance loads Then fixture is tracked', () => {
  assert.equal(existsSync(braceletFixture), true);
});

test('BDD Given immutable analytic baseline When three adaptive hybrid samples are 3x faster Then bracelet release gate passes', () => {
  const adaptive = [1, 2, 3].map(index => sample(`adaptive-${index}`, 'adaptive', 22_000, 20_000));
  const gate = evaluateBraceletGate({
    adaptive,
    historicalBaseline: { nativeMs: 69_669, booleanMs: 64_133 },
  });
  assert.equal(gate.passed, true);
  assert.ok(gate.nativeSpeedup >= 3);
  assert.ok(gate.booleanSpeedup >= 3);
});

test('BDD Given current hybrid samples When artifact bytes diverge Then bracelet release gate fails', () => {
  const adaptive = [1, 2, 3].map(index => sample(`adaptive-${index}`, 'adaptive', 22_000, 20_000));
  adaptive[2].stl.digest = 'different';
  assert.throws(
    () => evaluateBraceletGate({ adaptive }),
    /bracelet STL bytes changed across adaptive samples/,
  );
});

test('BDD Given cached bracelet variants When unrelated parameters change Then only required partial closure runs', () => {
  const seed = sample('seed', 'adaptive', 20_000, 18_000);
  const identical = sample('identical', 'adaptive', 1_000, 0, {
    stage: { ...seed.stage, totalElapsedMs: 1_000, serialBooleanCount: 0, parallelBooleanCount: 0 },
  });
  const bodyEdit = sample('body', 'adaptive', 4_000, 2_000, {
    stage: { ...seed.stage, totalElapsedMs: 4_000, parts: [
      { partId: 'daughter-flower-center-lid', cacheHit: true, executedCommandCount: 0 },
    ] },
  });
  const threadEdit = sample('thread', 'adaptive', 8_000, 5_000, {
    stage: { ...seed.stage, totalElapsedMs: 8_000, partialBooleanGroups: [
      { key: 'decorated-dome', cacheHit: true, recomputeCount: 0 },
    ] },
  });
  const decorationEdit = sample('decoration', 'adaptive', 9_000, 6_000, {
    stage: { ...seed.stage, totalElapsedMs: 9_000, partialBooleanGroups: [
      { key: 'operand-pair-0', cacheHit: true, recomputeCount: 0 },
      { key: 'decorated-dome', cacheHit: false, recomputeCount: 1 },
    ] },
  });
  assert.deepEqual(evaluateBraceletIncremental({ seed, identical, bodyEdit, threadEdit, decorationEdit }), { passed: true });
});
