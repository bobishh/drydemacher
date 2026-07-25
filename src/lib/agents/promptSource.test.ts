import { strict as assert } from 'node:assert';
import test from 'node:test';

import { MODEL_SYSTEM_PROMPT } from '../../../server/prompt.js';

test('legacy node prompt uses generated ecky authoring source', () => {
  assert.match(MODEL_SYSTEM_PROMPT, /# Ecky authoring/);
  assert.match(MODEL_SYSTEM_PROMPT, /# Response envelope/);
  assert.doesNotMatch(MODEL_SYSTEM_PROMPT, /You are a CAD Design Agent for FreeCAD/);
  assert.doesNotMatch(MODEL_SYSTEM_PROMPT, /Generate FreeCAD Python macros/);
});
