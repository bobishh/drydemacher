import assert from 'node:assert/strict';
import test from 'node:test';

import {
  assessPixels,
  selectCaptureGuidance,
  sharpestCandidateIndex,
  type CaptureMetrics,
} from '../../../src-tauri/assets/capture_metrics.mjs';

function metrics(overrides: Partial<CaptureMetrics> = {}): CaptureMetrics {
  return {
    luminance: 120,
    sharpness: 24,
    subjectCoverage: 0.45,
    borderContact: 0,
    borderSides: 0,
    motion: 2,
    novelty: 20,
    ...overrides,
  };
}

test('instruction priority chooses illumination before motion', () => {
  assert.equal(
    selectCaptureGuidance(metrics({ luminance: 20, subjectCoverage: 0.95, motion: 40 }), 5_000).code,
    'tooDark',
  );
});

test('instruction priority covers every blocking gate', () => {
  assert.equal(selectCaptureGuidance(metrics({ motion: 30 }), 5_000).code, 'moveSlower');
  assert.equal(selectCaptureGuidance(metrics({ novelty: 1 }), 5_000).code, 'newAngle');
  assert.equal(selectCaptureGuidance(metrics(), 500).code, 'holdStill');
  assert.equal(selectCaptureGuidance(metrics(), 5_000).code, 'accepted');
});

test('low absolute sharpness does not permanently block a low-texture object', () => {
  assert.equal(selectCaptureGuidance(metrics({ sharpness: 0.2 }), 5_000).code, 'accepted');
});

test('relative focus selection keeps sharpest candidate from a bounded burst', () => {
  assert.equal(sharpestCandidateIndex([1.2, 4.8, 2.7, 3.1]), 1);
});

test('local Laplacian score ranks a crisp edge above a softened edge', () => {
  const width = 48;
  const height = 48;
  const makeFrame = (soft: boolean) => {
    const rgba = new Uint8ClampedArray(width * height * 4);
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const distance = Math.max(Math.abs(x - width / 2), Math.abs(y - height / 2));
        const value = soft
          ? Math.round(45 + 135 * Math.max(0, Math.min(1, (15 - distance) / 6)))
          : distance <= 12 ? 180 : 45;
        const offset = (y * width + x) * 4;
        rgba[offset] = rgba[offset + 1] = rgba[offset + 2] = value;
        rgba[offset + 3] = 255;
      }
    }
    return rgba;
  };

  const crisp = assessPixels(makeFrame(false), width, height, null, null);
  const soft = assessPixels(makeFrame(true), width, height, null, null);
  assert.ok(crisp.sharpness > soft.sharpness, `${crisp.sharpness} <= ${soft.sharpness}`);
});

test('Safari edge occupancy never claims unsupported physical distance', () => {
  const guidance = selectCaptureGuidance(
    metrics({ subjectCoverage: 0.99, borderContact: 0.9, borderSides: 4, metricDepth: false }),
    5_000,
  );

  assert.notEqual(guidance.code, 'tooClose');
  assert.notEqual(guidance.code, 'tooFar');
});

test('small low-texture Safari subject remains capturable without segmentation', () => {
  const guidance = selectCaptureGuidance(
    metrics({ subjectCoverage: 0.02, sharpness: 0.1, metricDepth: false }),
    5_000,
  );

  assert.equal(guidance.code, 'accepted');
});

test('pixel assessment is deterministic and framing-relative', () => {
  const width = 8;
  const height = 8;
  const rgba = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const index = (y * width + x) * 4;
      const foreground = x >= 2 && x <= 5 && y >= 2 && y <= 5;
      rgba[index] = rgba[index + 1] = rgba[index + 2] = foreground ? 180 : 40;
      rgba[index + 3] = 255;
    }
  }
  const first = assessPixels(rgba, width, height, null, null);
  const second = assessPixels(rgba, width, height, null, null);
  assert.deepEqual(first, second);
  assert.ok(first.subjectCoverage > 0.15 && first.subjectCoverage < 0.8);
  assert.equal(first.metricDepth, false);
});
