import { fireEvent, render } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import ParamPanelOwnershipSections from '../../src/lib/components/ParamPanelOwnershipSections.svelte';
import type { ParameterOwnershipSection } from '../../src/lib/modelRuntime/ownershipSections';

const field = {
  type: 'number' as const,
  key: 'spool_diameter',
  label: 'Spool Diameter',
  frozen: false,
};

function props(sections: ParameterOwnershipSection[]) {
  return {
    sections,
    parameters: { spool_diameter: 200 },
    getRangeProps: () => ({ min: 0, max: 300, step: 1 }),
    getCadTone: () => 'size' as const,
    onDraftValue: () => {},
    onUpdate: () => {},
    onPickImage: () => {},
    onSetFocusedControl: () => {},
    onClearFocusedControl: () => {},
  };
}

describe('ParamPanelOwnershipSections', () => {
  it('Given selected Model Params is expanded When COLLAPSE is clicked Then manual collapse wins', async () => {
    const section: ParameterOwnershipSection = {
      sectionId: 'model:parameters',
      label: 'Model Params',
      partIds: [],
      fields: [field],
      visibleFields: [field],
      collapsed: false,
      selected: true,
    };
    const view = render(ParamPanelOwnershipSections, { props: props([section]) });
    const header = view.getByRole('button', { name: 'Toggle Model Params parameters' });

    expect(header.getAttribute('aria-expanded')).toBe('true');
    expect(view.getByText('COLLAPSE')).not.toBeNull();
    await fireEvent.click(header);

    expect(header.getAttribute('aria-expanded')).toBe('false');
    expect(view.getByText('EXPAND')).not.toBeNull();
    expect(view.queryByText('Spool Diameter')).toBeNull();
    expect(view.getByTestId('parameter-ownership-section').dataset.collapsed).toBe('true');

    await view.rerender(props([{ ...section }]));
    expect(header.getAttribute('aria-expanded')).toBe('false');
    expect(view.getByText('EXPAND')).not.toBeNull();
  });
});
