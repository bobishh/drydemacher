import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';

import { buildEckyIrBook } from './eckyIrBook';

function fixturePath(...parts: string[]): string {
  return path.join(process.cwd(), ...parts);
}

test('buildEckyIrBook assembles campaign levels into book html', () => {
  const docsMarkdown = bookMarkdownFixture();

  const book = buildEckyIrBook({
    docsMarkdown,
  });

  assert.equal(book.title, 'Ecky Campaign');
  assert.match(book.html, /Table of Contents/i);
  assert.equal(book.chapters.length, 6);
  assert.equal(book.chapters[0]?.title, 'Level 01: Corner Bracket');
  assert.ok(book.chapters.some((chapter) => chapter.title === 'Level 05: Perforated Toothbrush Holder'));
  assert.ok(book.chapters.some((chapter) => chapter.title === 'Level 06: Film Adapter'));
  assert.match(book.html, /Mission:/i);
  assert.match(book.html, /Clear condition:/i);
  assert.match(book.html, /one n-ary <code>difference<\/code>/i);
  assert.doesNotMatch(book.html, /ECKY_AGENT_REFERENCE/);
  assert.ok(book.assets.length > 0);
  assert.match(book.html, /assets\/04-cut-and-join-01\.png/);
  assert.deepEqual(book.assets[0], {
    sourcePath: 'target/book/public/docs/assets/04-cut-and-join-01.png',
    outputPath: 'assets/04-cut-and-join-01.png',
    mediaType: 'image/png',
  });
});

function bookMarkdownFixture(): string {
  return fs.readFileSync(fixturePath('public', 'tutorials', 'ecky-campaign.md'), 'utf8');
}
