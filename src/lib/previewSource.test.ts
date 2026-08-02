import assert from 'node:assert/strict';
import test from 'node:test';

import { toPreviewSrc } from './previewSource';

test('toPreviewSrc preserves direct preview URLs and normalizes local paths', () => {
  const convert = (path: string) => `asset://localhost/${path}`;

  assert.equal(toPreviewSrc(null, convert), null);
  assert.equal(toPreviewSrc('   ', convert), null);
  assert.equal(toPreviewSrc(' data:image/png;base64,abc ', convert), 'data:image/png;base64,abc');
  assert.equal(toPreviewSrc('blob:preview', convert), 'blob:preview');
  assert.equal(toPreviewSrc('https://example.test/preview.png', convert), 'https://example.test/preview.png');
  assert.equal(toPreviewSrc(' /tmp/preview.png ', convert), 'asset://localhost//tmp/preview.png');
});

test('toPreviewSrc retains a local path when Tauri conversion fails', () => {
  assert.equal(toPreviewSrc('/tmp/preview.png', () => { throw new Error('no Tauri runtime'); }), '/tmp/preview.png');
});
