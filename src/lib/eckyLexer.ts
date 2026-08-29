/** Structural authoring forms. */
const ECKY_FORMS = new Set([
  'model', 'part', 'feature', 'params', 'build', 'shape', 'result', 'verify', 'tag', 'metric', 'expect',
  'define', 'define-component', 'define-syntax', 'let', 'let*', 'lambda', 'if', 'cond', 'when', 'unless',
  'begin', 'quote', 'meta', 'ports', 'port', 'frame', 'place-component', 'port-ref',
]);

/** Parameter/signature entry kinds. */
const ECKY_PARAM_KINDS = new Set(['number', 'toggle', 'select', 'image', 'option', 'text']);

/** Geometry operations (mirrors `ecky/cad` exports). */
const ECKY_CAD_OPS = new Set([
  'hole', 'compound', 'fuse', 'cut', 'common', 'box', 'sphere', 'cylinder', 'cone', 'circle', 'ring',
  'rectangle', 'rounded-rect', 'rounded-polygon', 'polygon', 'extrude', 'revolve', 'loft', 'sweep',
  'helical-ridge', 'shell', 'offset', 'offset-rounded', 'fillet', 'chamfer', 'taper', 'translate', 'rotate',
  'scale', 'mirror', 'sampled-radial-loft', 'bezier-path', 'bspline', 'path', 'polyline', 'profile',
  'make-face', 'union', 'difference', 'intersection', 'xor', 'linear-array', 'radial-array', 'grid-array',
  'arc-array', 'text', 'svg', 'import-stl', 'path-frame', 'plane', 'location', 'place', 'clip-box', 'twist',
  'repeat', 'repeat-union', 'repeat-compound', 'repeat-pick', 'for-union', 'for-compound', 'wall-pattern',
  'instance',
]);

/** Core helpers (mirrors `ecky/core` exports plus common scheme math). */
const ECKY_HELPERS = new Set([
  'vec2', 'vec3', 'start', 'end', 'xy', 'yz', 'xz', 'zip', 'enumerate', 'flat-map', 'concat-map', 'linspace',
  'pi', 'tau', 'clamp', 'lerp', 'invlerp', 'remap', 'deg', 'rad', 'deg->rad', 'rad->deg', 'smoothstep',
  'square', 'cube', 'hash01', 'hash-signed', 'noise2', 'fbm2', 'voronoi2', 'cell-distance2', 'jitter2',
  'jittered-grid', 'polar-points', 'organic-loop', 'wave-loop', 'superellipse-point', 'voronoi-cells',
  'lorenz-points', 'rossler-points', 'logistic-bifurcation-points', 'henon-points', 'map', 'filter', 'fold',
  'foldl', 'foldr', 'range', 'append', 'reverse', 'list', 'cons', 'car', 'cdr', 'apply', 'min', 'max', 'abs',
  'sqrt', 'sin', 'cos', 'tan', 'atan', 'atan2', 'floor', 'ceiling', 'round', 'expt', 'modulo', 'not', 'and', 'or',
]);

/** Heads whose next symbol is a user-given name worth its own color. */
const ECKY_NAMING_FORMS = new Set(['part', 'feature', 'define-component', 'define', 'shape', 'port']);

export type EckyTokenKind =
  | 'comment' | 'keyword' | 'kind' | 'op' | 'helper' | 'name' | 'call' | 'string' | 'number' | 'atom'
  | 'symbol' | 'paren1' | 'paren2' | 'paren3';

export type EckyLexState = { depth: number; afterOpen: boolean; expectName: boolean };
export type EckyLexToken = { text: string; kind: EckyTokenKind | null };

export function createEckyLexState(): EckyLexState {
  return { depth: 0, afterOpen: false, expectName: false };
}

function isSymbolChar(ch: string): boolean {
  return /[A-Za-z0-9_?!+\-*/<>=.$]/.test(ch);
}

function classifySymbol(symbol: string, state: EckyLexState): EckyTokenKind {
  const head = state.afterOpen;
  state.afterOpen = false;
  if (state.expectName) {
    state.expectName = false;
    return 'name';
  }
  if (ECKY_FORMS.has(symbol)) {
    if (head && ECKY_NAMING_FORMS.has(symbol)) state.expectName = true;
    return 'keyword';
  }
  if (ECKY_PARAM_KINDS.has(symbol)) {
    if (head) state.expectName = true;
    return 'kind';
  }
  if (ECKY_CAD_OPS.has(symbol)) return 'op';
  if (ECKY_HELPERS.has(symbol)) return 'helper';
  return head ? 'call' : 'symbol';
}

/**
 * Scan one token at `start`. `end` always advances, so callers can safely
 * render malformed source without a stalled lexer.
 */
export function scanEckyToken(source: string, start: number, state: EckyLexState): EckyLexToken & { end: number } {
  const next = source[start];
  if (!next) return { text: '', kind: null, end: start };

  if (/\s/.test(next)) {
    let end = start + 1;
    while (end < source.length && /\s/.test(source[end]!)) end += 1;
    return { text: source.slice(start, end), kind: null, end };
  }
  if (next === ';') {
    const lineEnd = source.indexOf('\n', start);
    const end = lineEnd === -1 ? source.length : lineEnd;
    return { text: source.slice(start, end), kind: 'comment', end };
  }
  if (next === '(') {
    state.depth += 1;
    state.afterOpen = true;
    state.expectName = false;
    return { text: next, kind: `paren${((state.depth - 1) % 3) + 1}` as EckyTokenKind, end: start + 1 };
  }
  if (next === ')') {
    const depth = state.depth;
    state.depth = Math.max(0, state.depth - 1);
    state.afterOpen = false;
    state.expectName = false;
    return { text: next, kind: `paren${((Math.max(1, depth) - 1) % 3) + 1}` as EckyTokenKind, end: start + 1 };
  }
  if (next === '"') {
    let end = start + 1;
    let escaped = false;
    while (end < source.length && source[end] !== '\n') {
      const ch = source[end++]!;
      if (escaped) escaped = false;
      else if (ch === '\\') escaped = true;
      else if (ch === '"') break;
    }
    state.afterOpen = false;
    return { text: source.slice(start, end), kind: 'string', end };
  }
  if (next === ':') {
    let end = start + 1;
    while (end < source.length && isSymbolChar(source[end]!)) end += 1;
    state.afterOpen = false;
    return { text: source.slice(start, end), kind: 'atom', end };
  }
  if (next === '#') {
    let end = start + 1;
    while (end < source.length && /[A-Za-z:]/.test(source[end]!)) end += 1;
    state.afterOpen = false;
    return { text: source.slice(start, end), kind: 'atom', end };
  }
  const number = source.slice(start).match(/^[+-]?(?:\d+(?:\.\d+)?|\.\d+)/)?.[0];
  if (number) {
    state.afterOpen = false;
    return { text: number, kind: 'number', end: start + number.length };
  }
  if (isSymbolChar(next)) {
    let end = start + 1;
    while (end < source.length && isSymbolChar(source[end]!)) end += 1;
    const text = source.slice(start, end);
    return { text, kind: classifySymbol(text, state), end };
  }
  state.afterOpen = false;
  return { text: next, kind: null, end: start + 1 };
}

/** Tokenize complete Ecky source. Concatenated token text always equals input. */
export function lexEcky(source: string): EckyLexToken[] {
  const state = createEckyLexState();
  const tokens: EckyLexToken[] = [];
  for (let start = 0; start < source.length;) {
    const { end, ...token } = scanEckyToken(source, start, state);
    tokens.push(token);
    start = end;
  }
  return tokens;
}
