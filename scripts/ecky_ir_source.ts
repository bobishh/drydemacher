import fs from 'node:fs';
import path from 'node:path';

export type SplitBookChapter = {
  title: string;
  relativePath: string;
  markdown: string;
};

const CHAPTER_LINK = /^- \[([^\]]+)\]\((chapters\/[^)]+\.md)\)$/gm;

export function projectSplitBook(canonicalMarkdown: string, indexMarkdown: string): SplitBookChapter[] {
  const chapters: SplitBookChapter[] = [];

  for (const match of indexMarkdown.matchAll(CHAPTER_LINK)) {
    const title = match[1]?.trim();
    const relativePath = match[2]?.trim();
    if (!title || !relativePath) continue;

    chapters.push({
      title,
      relativePath,
      markdown: extractLevelTwoSection(canonicalMarkdown, title),
    });
  }

  if (!chapters.length) {
    throw new Error('Book index contains no linked chapter entries.');
  }

  return chapters;
}

export function syncSplitBook(root: string, check = false): void {
  const bookRoot = path.join(root, 'docs', 'books', 'ecky-ir');
  const canonicalPath = path.join(root, 'public', 'docs', 'ecky-ir.md');
  const indexPath = path.join(bookRoot, 'index.md');
  const chapters = projectSplitBook(
    fs.readFileSync(canonicalPath, 'utf8'),
    fs.readFileSync(indexPath, 'utf8'),
  );
  const drift: string[] = [];

  for (const chapter of chapters) {
    const outputPath = path.join(bookRoot, chapter.relativePath);
    const current = fs.existsSync(outputPath) ? fs.readFileSync(outputPath, 'utf8') : null;
    if (current === chapter.markdown) continue;
    if (check) {
      drift.push(chapter.relativePath);
      continue;
    }
    fs.mkdirSync(path.dirname(outputPath), { recursive: true });
    fs.writeFileSync(outputPath, chapter.markdown);
  }

  if (drift.length) {
    throw new Error(`Split book drifted from public/docs/ecky-ir.md: ${drift.join(', ')}`);
  }
}

function extractLevelTwoSection(markdown: string, title: string): string {
  const normalized = markdown.replace(/\r\n/g, '\n');
  const heading = `## ${title}`;
  const start = normalized.indexOf(heading);
  if (start === -1 || (start > 0 && normalized[start - 1] !== '\n')) {
    throw new Error(`Canonical book is missing chapter heading: ${heading}`);
  }
  const next = normalized.indexOf('\n## ', start + heading.length);
  const end = next === -1 ? normalized.length : next + 1;
  return `${normalized.slice(start, end).trim()}\n`;
}
