import { render } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import SessionActivityWindow from '../../src/lib/SessionActivityWindow.svelte';

describe('SessionActivityWindow', () => {
  it('renders structural verification findings without leaking the transport envelope', () => {
    const { getByTestId, getByText, queryByText } = render(SessionActivityWindow, {
      props: {
        events: [{
          id: 'validation-1',
          sessionId: 'mcp-http-secret',
          cursor: null,
          lifecycleKey: null,
          threadId: 'thread-secret',
          versionId: 'preview-secret',
          actor: { kind: 'agent', id: 'agent', label: 'Agent' },
          kind: 'validation_reported',
          title: 'Preview validation reported',
          summary: 'Assembly base is above z=0.',
          timestamp: 1,
          severity: 'error',
          raw: {
            sessionId: 'mcp-http-secret',
            threadId: 'thread-secret',
            previewId: 'preview-secret',
            status: 'failed',
            source: 'structuralVerification',
            items: [{ code: 'GROUND_CONTACT_MISSING', message: 'Assembly base is above z=0.' }],
            authoringLints: [],
          },
        }],
      },
    });

    expect(getByTestId('activity-validation-feedback')).not.toBeNull();
    expect(getByText('GROUND_CONTACT_MISSING')).not.toBeNull();
    expect(queryByText('RAW')).toBeNull();
    expect(queryByText(/mcp-http-secret/)).toBeNull();
  });
});
