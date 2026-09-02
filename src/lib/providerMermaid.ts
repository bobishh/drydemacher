import type { MermaidConfig } from 'mermaid';

export const providerMermaidConfig = {
  startOnLoad: false,
  securityLevel: 'strict',
  suppressErrorRendering: true,
  theme: 'base',
  darkMode: true,
  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
  themeVariables: {
    background: '#151b2f',
    primaryColor: '#202a42',
    primaryBorderColor: '#4a8c5c',
    primaryTextColor: '#e9edf7',
    secondaryColor: '#2d2512',
    secondaryBorderColor: '#c8a620',
    secondaryTextColor: '#e9edf7',
    tertiaryColor: '#16213e',
    tertiaryBorderColor: '#4a8c5c',
    tertiaryTextColor: '#e9edf7',
    lineColor: '#c8a620',
    textColor: '#e9edf7',
  },
} satisfies MermaidConfig;

let mermaidEnginePromise: Promise<typeof import('mermaid').default> | null = null;
let renderQueue: Promise<void> = Promise.resolve();
let renderSerial = 0;

async function mermaidEngine() {
  if (!mermaidEnginePromise) {
    mermaidEnginePromise = import('mermaid').then(({ default: mermaid }) => {
      mermaid.initialize(providerMermaidConfig);
      return mermaid;
    });
  }
  return mermaidEnginePromise;
}

export function formatProviderMermaidError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function renderProviderMermaid(source: string): Promise<string> {
  const task = renderQueue.then(async () => {
    const mermaid = await mermaidEngine();
    const id = `eckyProviderMermaid${++renderSerial}`;
    const { svg } = await mermaid.render(id, source);
    return svg;
  });
  renderQueue = task.then(() => undefined, () => undefined);
  return task;
}
