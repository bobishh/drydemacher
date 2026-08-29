export type ProviderCodeReference = {
  kind: 'codeReference';
  label: string;
  path: string;
  line: number;
};

export type ProviderMessageSegment =
  | { kind: 'text'; text: string }
  | ProviderCodeReference;

export type ProviderMessagePresentation = {
  text: string;
  segments: ProviderMessageSegment[];
};

const DEBUG_ID_LINE = /^\s*`?(?:messageId|modelId)\s*:\s*[^\s`\\]+`?\\?\s*$/i;
const MARKDOWN_LINK = /\[([^\]\n]+)\]\(([^)\n]+)\)/g;

export function providerMessageText(content: string): string {
  return content
    .replace(/\r\n/g, '\n')
    .split('\n')
    .filter((line) => !DEBUG_ID_LINE.test(line))
    .join('\n')
    .trimEnd();
}

function parseCodeReference(label: string, target: string): ProviderCodeReference | null {
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

export function providerMessagePresentation(content: string): ProviderMessagePresentation {
  const text = providerMessageText(content);
  const segments: ProviderMessageSegment[] = [];
  let offset = 0;
  for (const matched of text.matchAll(MARKDOWN_LINK)) {
    const index = matched.index ?? 0;
    const reference = parseCodeReference(matched[1], matched[2]);
    if (!reference) continue;
    if (index > offset) segments.push({ kind: 'text', text: text.slice(offset, index) });
    segments.push(reference);
    offset = index + matched[0].length;
  }
  if (offset < text.length) segments.push({ kind: 'text', text: text.slice(offset) });
  if (segments.length === 0 && text) segments.push({ kind: 'text', text });
  return { text, segments };
}
