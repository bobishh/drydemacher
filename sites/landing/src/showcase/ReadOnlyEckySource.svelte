<script lang="ts">
  import { lexEcky, type EckyLexToken } from '../../../../src/lib/eckyLexer';

  type SourceLine = { tokens: EckyLexToken[] };

  let { code, label }: { code: string; label: string } = $props();

  /**
   * Keep lexer spans intact while distributing whitespace over visual lines.
   * This preserves leading whitespace and the final empty line after a trailing
   * newline without rendering source through HTML.
   */
  function linesFor(source: string): SourceLine[] {
    const lines: SourceLine[] = [{ tokens: [] }];
    for (const token of lexEcky(source)) {
      let remaining = token.text;
      while (true) {
        const newline = remaining.indexOf('\n');
        if (newline === -1) {
          if (remaining) lines[lines.length - 1]!.tokens.push({ ...token, text: remaining });
          break;
        }
        const beforeNewline = remaining.slice(0, newline);
        if (beforeNewline) lines[lines.length - 1]!.tokens.push({ ...token, text: beforeNewline });
        lines.push({ tokens: [] });
        remaining = remaining.slice(newline + 1);
      }
    }
    return lines;
  }

  function tokenClass(kind: EckyLexToken['kind']): string {
    if (!kind) return '';
    return `source-token source-token--${kind.replace('paren', 'paren-')}`;
  }

  const lines = $derived(linesFor(code));
</script>

<div
  class="source-view"
  data-testid="case-source"
  data-source-length={code.length}
  role="region"
  aria-label={label}
>
  {#each lines as line, index}
    <div class="source-line">
      <span class="source-line-number" data-testid="source-line-number" aria-hidden="true">{index + 1}</span>
      <code class="source-line-code">{#each line.tokens as token}{#if token.kind}<span class={tokenClass(token.kind)}>{token.text}</span>{:else}{token.text}{/if}{/each}</code>
    </div>
  {/each}
</div>

<style>
  .source-view {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
    background: var(--bg-100);
    color: var(--text);
    font: 13px/1.55 var(--font-mono, monospace);
    outline: none;
  }

  .source-line {
    display: grid;
    grid-template-columns: 4ch max-content;
    min-width: max-content;
    min-height: 1.55em;
  }

  .source-line-number {
    padding-right: 1.1ch;
    color: var(--text-dim);
    text-align: right;
    user-select: none;
  }

  .source-line-code {
    white-space: pre;
    font: inherit;
  }

  .source-token--comment { color: #6e7b95; font-style: italic; }
  .source-token--keyword { color: #d4a04f; font-weight: 700; }
  .source-token--kind { color: #d98f70; font-weight: 700; }
  .source-token--op { color: #62b6ab; }
  .source-token--helper { color: #a98fd1; }
  .source-token--name { color: #f0d49a; font-weight: 700; }
  .source-token--call { color: #e2c089; }
  .source-token--number { color: #7db2d7; }
  .source-token--string { color: #8ebf86; }
  .source-token--atom { color: #cf8d5a; }
  .source-token--symbol { color: #d7deea; }
  .source-token--paren-1 { color: #8a93ad; }
  .source-token--paren-2 { color: #7fa3a0; }
  .source-token--paren-3 { color: #9d8fbd; }
</style>
