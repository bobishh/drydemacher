const MANIFEST_VERIFY_TEMPLATE = [
  '  (verify',
  '    (tag body_shell)',
  '    (metric check (manifest has-step))',
  '    (expect check (= true)))',
].join('\n');

function buildClearanceVerifyTemplate(leftPartId: string, rightPartId: string): string {
  const tag = `${leftPartId}_${rightPartId}_gap`;
  return [
    '  (verify',
    `    (tag ${tag})`,
    `    (metric gap (clearance min-distance ${leftPartId} ${rightPartId}))`,
    '    (expect gap (>= 3)))',
  ].join('\n');
}

function extractTopLevelPartIds(code: string): string[] {
  const matches = [...code.matchAll(/^\s*\(part\s+([A-Za-z0-9_.-]+)/gm)];
  const ids: string[] = [];
  for (const match of matches) {
    const id = match[1]?.trim();
    if (!id || ids.includes(id)) continue;
    ids.push(id);
  }
  return ids;
}

export function hasVerifyClause(code: string): boolean {
  return /\(\s*verify\b/.test(code);
}

function hasTopLevelForm(code: string, formName: string): boolean {
  let depth = 0;
  let inString = false;
  let escaped = false;
  let inLineComment = false;

  for (let index = 0; index < code.length; index += 1) {
    const char = code[index];
    if (inLineComment) {
      if (char === '\n') inLineComment = false;
      continue;
    }
    if (inString) {
      if (escaped) escaped = false;
      else if (char === '\\') escaped = true;
      else if (char === '"') inString = false;
      continue;
    }
    if (char === ';') {
      inLineComment = true;
      continue;
    }
    if (char === '"') {
      inString = true;
      continue;
    }
    if (char === '(') {
      if (depth === 0) {
        const tail = code.slice(index + 1);
        const match = tail.match(/^\s*([^\s()]+)/);
        if (match?.[1] === formName) return true;
      }
      depth += 1;
      continue;
    }
    if (char === ')') depth = Math.max(0, depth - 1);
  }

  return false;
}

export function looksLikeEckyModelSource(code: string): boolean {
  return hasTopLevelForm(code, 'model');
}

export function canInsertVerifyTemplate(code: string): boolean {
  return looksLikeEckyModelSource(code) && !hasVerifyClause(code);
}

export function insertVerifyTemplate(code: string): string {
  if (!canInsertVerifyTemplate(code)) return code;

  const trimmed = code.trimEnd();
  const closingIndex = trimmed.lastIndexOf(')');
  if (closingIndex === -1) return code;

  const before = trimmed.slice(0, closingIndex).replace(/\s+$/, '');
  const after = trimmed.slice(closingIndex);
  const partIds = extractTopLevelPartIds(code);
  const verifyTemplate =
    partIds.length >= 2
      ? buildClearanceVerifyTemplate(partIds[0], partIds[1])
      : MANIFEST_VERIFY_TEMPLATE;
  return `${before}\n${verifyTemplate}\n${after}\n`;
}
