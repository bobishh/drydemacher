import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render } from '@testing-library/svelte';
import DialogueWindowContent from '../../src/lib/dialogue/DialogueWindowContent.svelte';

describe('DialogueWindowContent', () => {
  it('renders the remembered-layout toolbar and prompt composer', () => {
    const { getByRole, getByText } = render(DialogueWindowContent, {
      props: {
        rememberLayout: true,
        activeThreadId: 'thread-1',
        promptProps: {
          onGenerate: async () => {},
          onShowCode: () => {},
          dialogueState: { mode: 'generate' },
        },
      },
    });

    expect((getByRole('checkbox', { name: 'Remember layout' }) as HTMLInputElement).checked).toBe(true);
    expect(getByText('PROCESS')).not.toBeNull();
  });

  it('forwards remembered-layout changes to App ownership', async () => {
    const onRememberLayoutChange = vi.fn();
    const { getByRole } = render(DialogueWindowContent, {
      props: {
        rememberLayout: false,
        onRememberLayoutChange,
        activeThreadId: null,
        promptProps: { onGenerate: async () => {}, onShowCode: () => {} },
      },
    });

    await fireEvent.click(getByRole('checkbox', { name: 'Remember layout' }));
    expect(onRememberLayoutChange).toHaveBeenCalledWith(true);
  });
});
