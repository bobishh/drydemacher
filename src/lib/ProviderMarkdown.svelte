<script lang="ts">
  import { tick } from 'svelte';
  import { renderProviderMarkdown } from './providerMarkdown';
  import {
    formatProviderMermaidError,
    renderProviderMermaid,
  } from './providerMermaid';
  import type { ProviderCodeReference } from './providerMessagePresentation';

  let {
    content,
    onOpenCodeReference,
  }: {
    content: string;
    onOpenCodeReference?: (reference: ProviderCodeReference) => Promise<void> | void;
  } = $props();

  let root = $state<HTMLElement | null>(null);
  let renderGeneration = 0;
  const rendered = $derived(renderProviderMarkdown(content));

  function showMermaidError(host: HTMLElement, error: unknown) {
    const message = document.createElement('div');
    message.className = 'provider-mermaid-error';
    message.setAttribute('role', 'alert');
    message.textContent = formatProviderMermaidError(error);
    host.replaceChildren(message);
    host.classList.add('provider-mermaid--error');
  }

  async function renderDiagrams(generation: number) {
    await tick();
    if (!root || generation !== renderGeneration) return;
    for (const diagram of rendered.diagrams) {
      const host = root.querySelector<HTMLElement>(`[data-provider-mermaid-id="${diagram.id}"]`);
      if (!host) continue;
      try {
        const svg = await renderProviderMermaid(diagram.source);
        if (generation !== renderGeneration || !host.isConnected) return;
        host.innerHTML = svg;
        host.classList.add('provider-mermaid--rendered');
      } catch (error) {
        if (generation !== renderGeneration || !host.isConnected) return;
        showMermaidError(host, error);
      }
    }
  }

  function handleClick(event: MouseEvent) {
    const target = event.target instanceof Element ? event.target : null;
    const button = target?.closest<HTMLButtonElement>('button[data-provider-code-reference]');
    if (!button || !root?.contains(button)) return;
    const index = Number.parseInt(button.dataset.providerCodeReference ?? '', 10);
    const reference = rendered.codeReferences[index];
    if (!reference) return;
    event.preventDefault();
    void onOpenCodeReference?.(reference);
  }

  function providerMarkdownActions(node: HTMLElement) {
    node.addEventListener('click', handleClick);
    return {
      destroy() {
        node.removeEventListener('click', handleClick);
      },
    };
  }

  $effect(() => {
    rendered;
    root;
    const generation = ++renderGeneration;
    void renderDiagrams(generation);
    return () => {
      if (renderGeneration === generation) renderGeneration += 1;
    };
  });
</script>

<div class="provider-markdown" bind:this={root} use:providerMarkdownActions>
  {@html rendered.html}
</div>

<style>
  .provider-markdown {
    max-width: 100%;
    overflow: hidden;
    white-space: normal;
    overflow-wrap: anywhere;
  }

  .provider-markdown :global(h1),
  .provider-markdown :global(h2),
  .provider-markdown :global(h3),
  .provider-markdown :global(h4),
  .provider-markdown :global(h5),
  .provider-markdown :global(h6) {
    margin: 10px 0 5px;
    color: var(--secondary);
    font-family: var(--font-mono);
    line-height: 1.25;
  }

  .provider-markdown :global(h1) { font-size: 1.15rem; }
  .provider-markdown :global(h2) { font-size: 1rem; }
  .provider-markdown :global(h3) { font-size: 0.88rem; }
  .provider-markdown :global(h4),
  .provider-markdown :global(h5),
  .provider-markdown :global(h6) { font-size: 0.76rem; }

  .provider-markdown :global(h1:first-child),
  .provider-markdown :global(h2:first-child),
  .provider-markdown :global(h3:first-child),
  .provider-markdown :global(p:first-child) {
    margin-top: 0;
  }

  .provider-markdown :global(p) {
    margin: 0 0 7px;
  }

  .provider-markdown :global(p:last-child) {
    margin-bottom: 0;
  }

  .provider-markdown :global(ul),
  .provider-markdown :global(ol) {
    margin: 5px 0 8px;
    padding-left: 22px;
  }

  .provider-markdown :global(li + li) {
    margin-top: 2px;
  }

  .provider-markdown :global(blockquote) {
    margin: 7px 0;
    border-left: 2px solid var(--primary);
    padding: 4px 9px;
    background: color-mix(in srgb, var(--primary) 8%, var(--bg-100));
    color: var(--text-dim);
  }

  .provider-markdown :global(pre) {
    max-width: 100%;
    overflow: auto;
    margin: 7px 0;
    border: 1px solid var(--bg-300);
    border-radius: 0;
    padding: 8px;
    background: var(--bg-100);
    white-space: pre;
  }

  .provider-markdown :global(code) {
    border: 1px solid color-mix(in srgb, var(--secondary) 34%, var(--bg-300));
    border-radius: 0;
    padding: 1px 4px;
    background: color-mix(in srgb, var(--secondary) 8%, var(--bg-100));
    color: var(--secondary);
    font-family: var(--font-mono);
    font-size: 0.92em;
  }

  .provider-markdown :global(pre code) {
    border: 0;
    padding: 0;
    background: transparent;
    color: var(--text);
  }

  .provider-markdown :global(a) {
    color: var(--primary);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .provider-markdown :global(hr) {
    margin: 9px 0;
    border: 0;
    border-top: 1px solid var(--bg-300);
  }

  .provider-markdown :global(table) {
    display: block;
    max-width: 100%;
    overflow-x: auto;
    margin: 7px 0;
    border-collapse: collapse;
  }

  .provider-markdown :global(th),
  .provider-markdown :global(td) {
    border: 1px solid var(--bg-300);
    padding: 4px 7px;
    text-align: left;
    white-space: nowrap;
  }

  .provider-markdown :global(th) {
    background: color-mix(in srgb, var(--secondary) 10%, var(--bg-100));
    color: var(--secondary);
  }

  .provider-markdown :global(img) {
    display: block;
    max-width: 100%;
    height: auto;
    margin: 7px 0;
    border: 1px solid var(--bg-300);
    border-radius: 0;
  }

  .provider-markdown :global(.provider-math) {
    display: inline-block;
    max-width: 100%;
    color: var(--text);
    vertical-align: baseline;
  }

  .provider-markdown :global(.provider-math--display) {
    display: block;
    overflow-x: auto;
    overflow-y: hidden;
    margin: 6px 0;
  }

  .provider-markdown :global(.provider-math--display .katex-display) {
    margin: 0;
    text-align: left;
  }

  .provider-markdown :global(.provider-code-reference) {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    max-width: 100%;
    overflow: hidden;
    border: 1px solid var(--primary);
    border-radius: 0;
    padding: 2px 6px;
    background: color-mix(in srgb, var(--primary) 12%, var(--bg-100));
    color: var(--primary);
    font: inherit;
    font-family: var(--font-mono);
    cursor: pointer;
    vertical-align: baseline;
  }

  .provider-markdown :global(.provider-code-reference:hover),
  .provider-markdown :global(.provider-code-reference:focus-visible) {
    background: color-mix(in srgb, var(--primary) 22%, var(--bg-100));
    outline: 1px solid var(--secondary);
    outline-offset: 1px;
  }

  .provider-markdown :global(.provider-code-reference span) {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .provider-markdown :global(.provider-code-reference small) {
    color: var(--text-dim);
    font-size: 0.52rem;
    white-space: nowrap;
  }

  .provider-markdown :global(.provider-mermaid) {
    max-width: 100%;
    overflow: auto;
    margin: 8px 0;
    border: 1px solid color-mix(in srgb, var(--secondary) 55%, var(--bg-300));
    border-radius: 0;
    padding: 8px;
    background: var(--bg-100);
  }

  .provider-markdown :global(.provider-mermaid svg) {
    display: block;
    max-width: 100%;
    height: auto;
    margin: 0 auto;
  }

  .provider-markdown :global(.provider-mermaid-pending) {
    color: var(--text-dim);
    font-family: var(--font-mono);
    font-size: 0.6rem;
    letter-spacing: 0.08em;
  }

  .provider-markdown :global(.provider-mermaid--error) {
    border-color: var(--danger, #ff6b6b);
  }

  .provider-markdown :global(.provider-mermaid-error) {
    color: var(--danger, #ff6b6b);
    font-family: var(--font-mono);
    font-size: 0.62rem;
    white-space: pre-wrap;
  }
</style>
