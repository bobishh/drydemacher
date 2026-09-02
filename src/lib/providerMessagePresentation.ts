export type ProviderCodeReference = {
  kind: 'codeReference';
  label: string;
  path: string;
  line: number;
};

export type ProviderMessageSegment =
  | { kind: 'text'; text: string }
  | { kind: 'math'; latex: string; displayMode: boolean }
  | ProviderCodeReference;

export type ProviderMessagePresentation = {
  text: string;
  segments: ProviderMessageSegment[];
};

const DEBUG_ID_LINE = /^\s*`?(?:messageId|modelId)\s*:\s*[^\s`\\]+`?\\?\s*$/i;
const MARKDOWN_LINK = /\[([^\]\n]+)\]\(([^)\n]+)\)/g;

type MathDelimiter = {
  open: string;
  close: string;
  displayMode: boolean;
  multiline: boolean;
};

const MATH_DELIMITERS: MathDelimiter[] = [
  { open: '$$', close: '$$', displayMode: true, multiline: true },
  { open: '\\[', close: '\\]', displayMode: true, multiline: true },
  { open: '\\(', close: '\\)', displayMode: false, multiline: false },
  { open: '$', close: '$', displayMode: false, multiline: false },
];

export function providerMessageText(content: string): string {
  return content
    .replace(/\r\n/g, '\n')
    .split('\n')
    .filter((line) => !DEBUG_ID_LINE.test(line))
    .join('\n')
    .trimEnd();
}

export function parseProviderCodeReference(label: string, target: string): ProviderCodeReference | null {
  let decoded: string;
  try {
    decoded = decodeURIComponent(target);
  } catch {
    return null;
  }
  const matched = decoded.match(/^(\/.*\/model\.ecky)(?::(\d+))?$/i);
  if (!matched) return null;
  const line = matched[2] ? Number.parseInt(matched[2], 10) : 1;
  if (!Number.isSafeInteger(line) || line <= 0) return null;
  return {
    kind: 'codeReference',
    label,
    path: matched[1],
    line,
  };
}

export type ProviderMathToken = {
  raw: string;
  latex: string;
  displayMode: boolean;
};

export function providerMathTokenAtStart(text: string): ProviderMathToken | null {
  const delimiter = delimiterAt(text, 0);
  if (!delimiter) return null;
  const latexStart = delimiter.open.length;
  const close = closingDelimiterIndex(text, latexStart, delimiter);
  if (close < 0) return null;
  return {
    raw: text.slice(0, close + delimiter.close.length),
    latex: text.slice(latexStart, close),
    displayMode: delimiter.displayMode,
  };
}

function isEscaped(text: string, index: number): boolean {
  let slashCount = 0;
  for (let cursor = index - 1; cursor >= 0 && text[cursor] === '\\'; cursor -= 1) {
    slashCount += 1;
  }
  return slashCount % 2 === 1;
}

function delimiterAt(text: string, index: number): MathDelimiter | null {
  if (isEscaped(text, index)) return null;
  for (const delimiter of MATH_DELIMITERS) {
    if (!text.startsWith(delimiter.open, index)) continue;
    if (delimiter.open === '$' && text.startsWith('$$', index)) continue;
    if (!text[index + delimiter.open.length] || /\s/.test(text[index + delimiter.open.length])) continue;
    return delimiter;
  }
  return null;
}

function closingDelimiterIndex(text: string, start: number, delimiter: MathDelimiter): number {
  let cursor = start;
  while (cursor < text.length) {
    const matched = text.indexOf(delimiter.close, cursor);
    if (matched < 0) return -1;
    if (!delimiter.multiline && text.slice(start, matched).includes('\n')) return -1;
    if (!isEscaped(text, matched)) {
      const latex = text.slice(start, matched);
      const previous = text[matched - 1] ?? '';
      const next = text[matched + delimiter.close.length] ?? '';
      const validDollarClose = delimiter.close !== '$'
        || (!/\s/.test(previous) && !/\d/.test(next));
      if (latex.trim() && validDollarClose) return matched;
    }
    cursor = matched + delimiter.close.length;
  }
  return -1;
}

function mathSegments(text: string): ProviderMessageSegment[] {
  const segments: ProviderMessageSegment[] = [];
  let textStart = 0;
  let cursor = 0;
  while (cursor < text.length) {
    const delimiter = delimiterAt(text, cursor);
    if (!delimiter) {
      cursor += 1;
      continue;
    }
    const latexStart = cursor + delimiter.open.length;
    const close = closingDelimiterIndex(text, latexStart, delimiter);
    if (close < 0) {
      cursor = latexStart;
      continue;
    }
    if (cursor > textStart) segments.push({ kind: 'text', text: text.slice(textStart, cursor) });
    segments.push({
      kind: 'math',
      latex: text.slice(latexStart, close),
      displayMode: delimiter.displayMode,
    });
    cursor = close + delimiter.close.length;
    textStart = cursor;
  }
  if (textStart < text.length) segments.push({ kind: 'text', text: text.slice(textStart) });
  return segments;
}

function appendTextSegments(segments: ProviderMessageSegment[], text: string) {
  if (!text) return;
  segments.push(...mathSegments(text));
}

export function providerMessagePresentation(content: string): ProviderMessagePresentation {
  const text = providerMessageText(content);
  const segments: ProviderMessageSegment[] = [];
  let offset = 0;
  for (const matched of text.matchAll(MARKDOWN_LINK)) {
    const index = matched.index ?? 0;
    const reference = parseProviderCodeReference(matched[1], matched[2]);
    if (!reference) continue;
    if (index > offset) appendTextSegments(segments, text.slice(offset, index));
    segments.push(reference);
    offset = index + matched[0].length;
  }
  if (offset < text.length) appendTextSegments(segments, text.slice(offset));
  return { text, segments };
}
