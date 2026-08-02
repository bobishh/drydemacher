import assert from 'node:assert/strict';
import test from 'node:test';

import { fitRectToViewport } from './windowGeometry';

test('fitRectToViewport shrinks oversized rect and clamps it fully inside viewport', () => {
  assert.deepEqual(
    fitRectToViewport(
      { x: 140, y: 120, width: 1000, height: 700 },
      { width: 400, height: 300 },
      { width: 900, height: 620 },
    ),
    { x: 0, y: 0, width: 900, height: 620 },
  );
});

test('fitRectToViewport keeps size when already valid and only repositions offscreen rect', () => {
  assert.deepEqual(
    fitRectToViewport(
      { x: 320, y: 560, width: 980, height: 260 },
      { width: 350, height: 260 },
      { width: 1180, height: 680 },
    ),
    { x: 200, y: 420, width: 980, height: 260 },
  );
});

test('fitRectToViewport reserves the measured dock bottom inset', () => {
  assert.deepEqual(
    fitRectToViewport(
      { x: 320, y: 560, width: 980, height: 260 },
      { width: 350, height: 260 },
      { width: 1280, height: 720 },
      { bottom: 82 },
    ),
    { x: 300, y: 378, width: 980, height: 260 },
  );
});

test('fitRectToViewport moves a restored rectangle out of the dock-safe area', () => {
  assert.deepEqual(
    fitRectToViewport(
      { x: 100, y: 500, width: 600, height: 300 },
      { width: 400, height: 240 },
      { width: 1000, height: 700 },
      { bottom: 90 },
    ),
    { x: 100, y: 310, width: 600, height: 300 },
  );
});

test('fitRectToViewport clamps drag-end geometry against every safe edge', () => {
  assert.deepEqual(
    fitRectToViewport(
      { x: 900, y: 650, width: 300, height: 200 },
      { width: 200, height: 160 },
      { width: 1000, height: 700 },
      { top: 12, right: 16, bottom: 80, left: 8 },
    ),
    { x: 684, y: 420, width: 300, height: 200 },
  );
});

test('fitRectToViewport shrinks below minimum when safe work area is smaller', () => {
  assert.deepEqual(
    fitRectToViewport(
      { x: 80, y: 80, width: 500, height: 400 },
      { width: 400, height: 300 },
      { width: 300, height: 220 },
      { top: 10, right: 12, bottom: 90, left: 8 },
    ),
    { x: 8, y: 10, width: 280, height: 120 },
  );
});
