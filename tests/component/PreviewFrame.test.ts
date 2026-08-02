import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import PreviewFrame from '../../src/lib/PreviewFrame.svelte';

// T1 — harness proof: mount one existing small Svelte component in isolation
// and assert on its prop-driven render branches (the frontend-testing spec's
// "mount a single component with props and assert on its rendered DOM" case).
// PreviewFrame.svelte is chosen as the smallest pure-presentational component
// (no Tauri invoke, no global stores, props + runes only).
describe('PreviewFrame', () => {
  it('renders the empty placeholder when no src is provided', () => {
    const { container, getByText } = render(PreviewFrame, {
      props: { alt: 'a model preview' },
    });

    // empty branch -> figcaption shows the default label, no <img>
    expect(getByText('NO PREVIEW')).toBeTruthy();
    expect(container.querySelector('img')).toBeNull();
    expect(
      container.querySelector('[data-preview-state]')?.getAttribute('data-preview-state'),
    ).toBe('empty');
  });

  it('renders the image when a src is provided', () => {
    const { container } = render(PreviewFrame, {
      props: { src: '/preview.png', alt: 'a model preview' },
    });

    const img = container.querySelector('img');
    expect(img).not.toBeNull();
    expect(img?.getAttribute('src')).toBe('/preview.png');
    expect(img?.getAttribute('alt')).toBe('a model preview');
    expect(
      container.querySelector('[data-preview-state]')?.getAttribute('data-preview-state'),
    ).toBe('ready');
  });

  it('shows the loading caption when state is loading', () => {
    const { getByText } = render(PreviewFrame, {
      props: { src: '/preview.png', alt: 'a model preview', state: 'loading' },
    });

    expect(getByText('LOADING PREVIEW...')).toBeTruthy();
  });
});
