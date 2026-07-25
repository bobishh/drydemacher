import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';

type WorkedExampleStage = {
  id: string;
  title: string;
  source: string;
  focus: string[];
};

type WorkedExampleManifest = {
  project: string;
  stages: WorkedExampleStage[];
};

const root = process.cwd();

test('worked example manifest names an ordered, runnable toothbrush-holder progression', () => {
  const manifestPath = path.join(
    root,
    'docs',
    'books',
    'ecky-ir',
    'examples',
    'toothbrush-holder',
    'manifest.json',
  );
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as WorkedExampleManifest;

  assert.equal(manifest.project, 'Perforated toothbrush holder');
  assert.deepEqual(
    manifest.stages.map((stage) => stage.id),
    ['shell', 'drained-base', 'single-cutter', 'repeated-cutters'],
  );

  for (const stage of manifest.stages) {
    assert.ok(stage.title.length > 0);
    assert.ok(stage.focus.length > 0);
    const sourcePath = path.join(root, stage.source);
    assert.equal(path.extname(sourcePath), '.ecky');
    assert.ok(fs.existsSync(sourcePath), `${stage.id} source must exist`);
    assert.match(fs.readFileSync(sourcePath, 'utf8'), /^\s*(?:;;[^\n]*\n)*\(model\b/);
  }
});

test('field guide links every toothbrush-holder checkpoint and teaches one final n-ary cut', () => {
  const book = fs.readFileSync(path.join(root, 'public', 'tutorials', 'ecky-campaign.md'), 'utf8');

  assert.match(book, /^## Level 05: Perforated Toothbrush Holder$/m);
  assert.match(book, /toothbrush-holder\/01-shell\.ecky/);
  assert.match(book, /toothbrush-holder\/02-drained-base\.ecky/);
  assert.match(book, /toothbrush-holder\/03-single-cutter\.ecky/);
  assert.match(book, /fixtures\/cad\/perf\/toothbrush_holder_versions\.ecky/);
  assert.match(book, /one n-ary `difference`/i);
});

test('placement lesson uses the mirror signature accepted by the runtime', () => {
  const book = fs.readFileSync(
    path.join(root, 'docs', 'books', 'ecky-ir', 'ecky-ir-corpus.md'),
    'utf8',
  );

  assert.match(book, /\(mirror 'x 0 \(box 10 10 10\)\)/);
  assert.doesNotMatch(book, /\(mirror :normal \(1 0 0\)/);
});
