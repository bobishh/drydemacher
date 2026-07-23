import assert from 'node:assert/strict';
import test from 'node:test';

import { shouldApplyDraftPreviewToWorkspace } from './draftPreviewProjection';

test('Given a draft preview for another thread When it arrives Then it does not replace the active workspace', () => {
  assert.equal(
    shouldApplyDraftPreviewToWorkspace({ activeThreadId: 'thread-a', previewThreadId: 'thread-b' }),
    false,
  );
});

test('Given a draft preview for the active thread When it arrives Then it may update the workspace', () => {
  assert.equal(
    shouldApplyDraftPreviewToWorkspace({ activeThreadId: 'thread-a', previewThreadId: 'thread-a' }),
    true,
  );
});

test('Given no active thread When a draft preview arrives Then it does not select one', () => {
  assert.equal(
    shouldApplyDraftPreviewToWorkspace({ activeThreadId: null, previewThreadId: 'thread-b' }),
    false,
  );
});
