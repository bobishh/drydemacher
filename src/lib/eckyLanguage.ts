import {
  HighlightStyle,
  LanguageSupport,
  StreamLanguage,
  StringStream,
  syntaxHighlighting,
} from '@codemirror/language';
import { tags } from '@lezer/highlight';
import {
  createEckyLexState,
  scanEckyToken,
  type EckyLexState,
  type EckyTokenKind,
} from './eckyLexer';

/** CodeMirror adapter over the pure Ecky lexer. */
export function readEckyToken(stream: StringStream, state: EckyLexState): EckyTokenKind | null {
  const token = scanEckyToken(stream.string, stream.pos, state);
  stream.pos = token.end;
  return token.kind;
}

export const eckyLanguage = StreamLanguage.define<EckyLexState>({
  name: 'ecky',
  startState: createEckyLexState,
  copyState: (state) => ({ ...state }),
  token: readEckyToken,
  tokenTable: {
    keyword: tags.keyword,
    kind: tags.className,
    op: tags.typeName,
    helper: tags.macroName,
    name: tags.definition(tags.variableName),
    call: tags.function(tags.variableName),
    comment: tags.comment,
    string: tags.string,
    number: tags.number,
    atom: tags.atom,
    symbol: tags.variableName,
    paren1: tags.bracket,
    paren2: tags.squareBracket,
    paren3: tags.angleBracket,
  },
});

const eckyHighlightStyle = HighlightStyle.define([
  { tag: tags.keyword, class: 'cm-ecky-keyword' },
  { tag: tags.className, class: 'cm-ecky-kind' },
  { tag: tags.typeName, class: 'cm-ecky-op' },
  { tag: tags.macroName, class: 'cm-ecky-helper' },
  { tag: tags.definition(tags.variableName), class: 'cm-ecky-name' },
  { tag: tags.function(tags.variableName), class: 'cm-ecky-call' },
  { tag: tags.comment, class: 'cm-ecky-comment' },
  { tag: tags.string, class: 'cm-ecky-string' },
  { tag: tags.number, class: 'cm-ecky-number' },
  { tag: tags.atom, class: 'cm-ecky-atom' },
  { tag: tags.variableName, class: 'cm-ecky-symbol' },
  { tag: tags.bracket, class: 'cm-ecky-paren-1' },
  { tag: tags.squareBracket, class: 'cm-ecky-paren-2' },
  { tag: tags.angleBracket, class: 'cm-ecky-paren-3' },
]);

export function eckyLanguageSupport(): LanguageSupport {
  return new LanguageSupport(eckyLanguage, [syntaxHighlighting(eckyHighlightStyle)]);
}
