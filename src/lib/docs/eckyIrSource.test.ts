import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';

import { projectSplitBook } from '../../../scripts/ecky_ir_source';

const root = process.cwd();

test('published campaign exactly generates every split teaching level', () => {
  const canonical = fs.readFileSync(path.join(root, 'public/tutorials/ecky-campaign.md'), 'utf8');
  const index = fs.readFileSync(path.join(root, 'docs/books/ecky-ir/index.md'), 'utf8');
  const projected = projectSplitBook(canonical, index);

  assert.equal(projected.length, 6);
  for (const chapter of projected) {
    const actual = fs.readFileSync(path.join(root, 'docs/books/ecky-ir', chapter.relativePath), 'utf8');
    assert.equal(actual, chapter.markdown, `${chapter.relativePath} drifted from campaign Markdown`);
  }
});
