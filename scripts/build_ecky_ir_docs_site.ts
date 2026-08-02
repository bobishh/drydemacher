/**
 * Build the Ecky IR Field Guide as a standalone, server-rendered HTML page.
 *
 * Output: target/book/dist/docs-site/index.html
 *
 * Run:  npm run build:docs-site
 *
 * This replaces the old parchment-style field-guide HTML for the web. The
 * EPUB builder (build_ecky_ir_book.ts) still handles the EPUB artifact; this
 * script handles the themed web page served at /docs/.
 */
import fs from 'node:fs';
import path from 'node:path';

import { parseDocsDocument } from '../src/lib/docs/eckyIrGuide';
import {
  buildDocsClientScript,
  buildDocsSitePages,
} from '../src/lib/docs/eckyIrDocsSite';
import {
  buildChaptersSitePages,
  type StaticChapter,
} from '../src/lib/docs/eckyIrChaptersSite';
import { syncEckyIrContent } from './ecky_ir_content';

const root = process.cwd();
const docsSourcePath = path.join(root, 'public', 'docs', 'ecky-ir.md');
const outputDir = path.join(root, 'target', 'book', 'dist', 'docs-site');

syncEckyIrContent(root);
const docsMarkdown = fs.readFileSync(docsSourcePath, 'utf8');
const doc = parseDocsDocument(docsMarkdown, { assetBasePath: '/docs' });

const pages = buildDocsSitePages(doc, {
  basePath: '/docs',
  rawMarkdownPath: '/docs/ecky-ir.md',
  epubPath: '/docs/ecky-ir-field-guide.epub',
});

const missionFiles = [
  ['mission-01-bracket-enclosure', 'level-01-corner-bracket.md'],
  ['mission-02-bottle-cage-dovetail', 'level-02-bottle-cage-dovetail.md'],
  ['mission-03-wing-propeller-study', 'level-03-printable-wing-propeller.md'],
  ['mission-04-gillette-travel-kit', 'level-04-gillette-travel-kit.md'],
  ['mission-05-iphone-case-fixture', 'level-05-iphone-case-fixture.md'],
  ['mission-06-film-scanner', 'level-06-film-scanner.md'],
] as const;
const chapters: StaticChapter[] = missionFiles.map(([id, file]) => {
  const markdown = fs.readFileSync(path.join(root, 'docs', 'books', 'ecky-ir', 'missions', file), 'utf8');
  const title = markdown.match(/^title:\s*(.+)$/m)?.[1] ?? id;
  return { id, sectionSlug: file.replace(/\.md$/, ''), title, markdown, checkpoints: [] };
});
const chapterPages = buildChaptersSitePages(chapters, {
  basePath: '/docs',
  epubPath: '/docs/ecky-ir-field-guide.epub',
});
for (const [relativePath, html] of chapterPages) pages.set(relativePath, html);

fs.rmSync(outputDir, { recursive: true, force: true });
for (const [relativePath, html] of pages) {
  const outputPath = path.join(outputDir, relativePath);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, html);
}
fs.writeFileSync(path.join(outputDir, 'docs.js'), buildDocsClientScript());

console.log(`Docs site: ${outputDir}`);
console.log(`  ${pages.size} reference and chapter pages rendered`);
