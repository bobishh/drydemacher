import { expect, test, type Page } from '@playwright/test';

declare global {
  interface Window {
    __AGENT_ACTIVITY_CALLS__?: Array<{ cmd: string; args?: Record<string, unknown> }>;
    __EMIT_AGENT_ACTIVITY__?: (payload: Record<string, unknown>) => void;
  }
}

async function installAgentActivityMocks(
  page: Page,
  fixture: 'default' | 'project-folder-consolidation' = 'default',
) {
  await page.route(/\/mock\/.*\.stl(?:\?.*)?$/, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'model/stl',
      body: 'solid test\nendsolid test',
    });
  });
  await page.addInitScript(({ fixture }) => {
    const defaultEvents = [
      ['event-1', 1, 'thread-active', 'Inspecting model', 'active', 'info', false, 'inspect'],
      ['event-2', 2, 'thread-background', 'Validating background design', 'active', 'info', false, 'validate'],
      ['event-3', 3, 'thread-active', 'Inspection complete', 'resolved', 'success', false, 'inspect'],
      ['event-4', 4, 'thread-background', 'Needs confirmation', 'active', 'question', true, 'question'],
      ['event-5', 5, 'thread-active', 'Saved version', 'resolved', 'success', false, 'save'],
      ['event-6', 6, 'thread-background', 'Provider rejected request', 'failed', 'error', true, 'provider'],
    ].map(([eventId, cursor, threadId, summary, state, severity, requiresAttention, lifecycle]) => ({
      eventId,
      cursor,
      sessionId: `session-${threadId}`,
      threadId,
      messageId: null,
      versionId: null,
      actor: { kind: 'agent', id: `agent-${threadId}`, label: threadId === 'thread-active' ? 'Codex' : 'Background agent' },
      kind: 'trace',
      lifecycleKey: `${threadId}:${lifecycle}`,
      phase: state === 'active' ? 'working' : state === 'failed' ? 'error' : 'idle',
      summary,
      detail: eventId === 'event-6' ? 'HTTP 429' : null,
      severity,
      state,
      requiresAttention,
      occurredAt: 1_800_000_000_000 + Number(cursor),
      raw: eventId === 'event-6' ? '{"error":"quota exceeded"}' : null,
    }));
    const projectFolderEvents = [
      ['folder-tool', 1, 'tool_start', 'Applying project source'],
      ['folder-backend', 2, 'backend_resolved', 'Resolved render backend'],
      ['folder-heal', 3, 'auto_heal_applied', 'Reconciled parameters'],
      ['folder-commit', 4, 'tool_success', 'Committed rendered source'],
    ].map(([eventId, cursor, kind, summary]) => ({
      eventId,
      cursor,
      sessionId: 'project-folder-watcher',
      threadId: 'thread-active',
      messageId: null,
      versionId: null,
      actor: { kind: 'agent', id: 'project-folder-watcher', label: 'folder-sync' },
      kind: 'trace',
      lifecycleKey: `trace:project-folder-watcher:thread-active:none:${kind}`,
      phase: kind === 'tool_success' ? 'idle' : 'working',
      summary,
      detail: null,
      severity: kind === 'tool_success' ? 'success' : 'info',
      state: kind === 'tool_success' ? 'resolved' : 'active',
      requiresAttention: false,
      occurredAt: 1_800_000_000_000 + Number(cursor),
      raw: JSON.stringify({ kind }),
    }));
    const projectFolderError = {
      eventId: 'provider-error', cursor: 5, sessionId: 'codex-session', threadId: 'thread-background',
      messageId: null, versionId: null,
      actor: { kind: 'agent', id: 'codex', label: 'Codex' }, kind: 'trace',
      lifecycleKey: 'trace:codex:thread-background:none:tool_error', phase: 'error',
      summary: 'Provider rejected request', detail: 'HTTP 429', severity: 'error', state: 'failed',
      requiresAttention: true, occurredAt: 1_800_000_000_005,
      raw: JSON.stringify({ kind: 'tool_error', body: 'quota exceeded' }),
    };
    const events = fixture === 'project-folder-consolidation'
      ? [...projectFolderEvents, projectFolderError]
      : defaultEvents;

    window.__AGENT_ACTIVITY_CALLS__ = [];
    window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
    const handlers = new Map<string, number[]>();
    let nextCallbackId = 1;
    window.__TAURI_INTERNALS__.transformCallback = (callback: unknown) => {
      const id = nextCallbackId++;
      (window as unknown as Record<string, unknown>)[`_${id}`] = callback;
      return id;
    };
    window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: Record<string, unknown>) => {
      window.__AGENT_ACTIVITY_CALLS__?.push({ cmd, args });
      if (cmd === 'plugin:event|listen') {
        const event = String(args?.event ?? '');
        const handler = Number(args?.handler);
        handlers.set(event, [...(handlers.get(event) ?? []), handler]);
        return handler;
      }
      if (cmd === 'plugin:event|unlisten') return null;
      if (cmd === 'get_agent_activity') return { events, latestCursor: events.length };
      if (cmd === 'project_folder_render_activity') {
        return fixture === 'project-folder-consolidation'
          ? [{ slug: 'dryer', threadId: 'thread-active', sourceDigest: 'sha256:dryer' }]
          : [];
      }
      if (cmd === 'get_config') {
        return {
          engines: [], selectedEngineId: '', freecadCmd: '', assets: [],
          microwave: { humId: null, dingId: null, muted: true },
          voice: { sttLanguageCode: 'en-US' },
          mcp: { mode: 'active', primaryAgentId: null, autoAgents: [], promptTimeoutSecs: 1800 },
          hasSeenOnboarding: true, connectionType: 'mcp', defaultEngineKind: 'ecky',
          defaultSourceLanguage: 'ecky', defaultGeometryBackend: 'mesh', maxGenerationAttempts: 1,
          maxVerifyAttempts: 0,
        };
      }
      if (cmd === 'get_runtime_capabilities') {
        return {
          freecad: { available: true, detail: 'Ready', path: '/mock/freecadcmd' },
          build123d: { available: true, detail: 'Ready', path: '/mock/python3' },
          mesh: { available: true, detail: 'bundled', path: null },
          recommendedAuthoringContext: { engineKind: 'ecky', sourceLanguage: 'ecky', geometryBackend: 'mesh' },
        };
      }
      if (cmd === 'get_history') return [];
      if (cmd === 'get_thread') {
        return { id: String(args?.id), title: 'Background project', status: 'active', messages: [], versionCount: 0, errorCount: 0, createdAt: 1, updatedAt: 2 };
      }
      if (cmd === 'get_last_design') {
        return {
          design: { title: 'Agent notifications', sourceLanguage: 'ecky', geometryBackend: 'mesh', macroCode: '(solid test)', initialParams: {} },
          threadId: 'thread-active', messageId: 'message-active', selectedPartId: null,
          artifactBundle: {
            modelId: 'agent-notification-model', sourceKind: 'generated', engineKind: 'ecky',
            sourceLanguage: 'ecky', geometryBackend: 'mesh', contentHash: 'agent-notification-hash',
            artifactVersion: 1, manifestPath: '/mock/manifest.json', macroPath: '/mock/source.ecky',
            modelStlPath: '/mock/model.stl', viewerAssets: [],
          },
          modelManifest: {
            modelId: 'agent-notification-model', sourceKind: 'generated', engineKind: 'ecky',
            sourceLanguage: 'ecky', geometryBackend: 'mesh', contentHash: 'agent-notification-hash',
            artifactVersion: 1, manifestPath: '/mock/manifest.json', macroPath: '/mock/source.ecky', parts: [],
          },
        };
      }
      if (cmd === 'get_thread_messages_page') return { messages: [], nextBefore: null, hasMore: false };
      if (cmd === 'get_thread_latest_version') return null;
      if (cmd === 'get_default_macro') return '(solid blank)';
      if (cmd === 'get_active_agent_sessions' || cmd === 'get_agent_terminal_snapshots') return [];
      return null;
    };
    window.__EMIT_AGENT_ACTIVITY__ = (payload) => {
      for (const handler of handlers.get('agent-activity-event') ?? []) {
        const callback = (window as unknown as Record<string, unknown>)[`_${handler}`];
        if (typeof callback === 'function') {
          (callback as (event: unknown) => void)({ event: 'agent-activity-event', payload });
        }
      }
    };
  }, { fixture });
}

test('Given folder render trace noise fills capacity When another agent fails Then one source card and the foreign error stay visible', async ({ page }) => {
  await installAgentActivityMocks(page, 'project-folder-consolidation');
  await page.goto('/');

  const center = page.locator('.genie-layer .agent-notification-center');
  await expect(center.locator('.agent-card')).toHaveCount(2);
  await expect(center.locator('.agent-card').filter({ hasText: 'SOURCE RENDERING' })).toHaveCount(1);
  await expect(center.locator('.agent-card').filter({ hasText: 'Provider rejected request' })).toHaveCount(1);
  await expect(center.locator('.agent-card').filter({ hasText: 'Applying project source' })).toHaveCount(0);
  await expect(center.locator('.agent-card').filter({ hasText: 'Resolved render backend' })).toHaveCount(0);
});

test('Given a declared cross-thread FEM job When it progresses and completes Then Ecky bubble expands and becomes a notification', async ({ page }) => {
  await installAgentActivityMocks(page);
  await page.goto('/');

  await page.evaluate(() => {
    window.__EMIT_AGENT_ACTIVITY__?.({
      eventId: 'long-task-1', cursor: 7, sessionId: 'fem-session', threadId: 'thread-background',
      messageId: 'message-fem', versionId: null,
      actor: { kind: 'agent', id: 'fem', label: 'FEM' }, kind: 'trace',
      lifecycleKey: 'long-task:topology-1', phase: 'solve', summary: 'Topology optimization',
      detail: 'Factoring load case 2/5', severity: 'info', state: 'active', requiresAttention: false,
      occurredAt: Date.now() - 12_000,
      raw: JSON.stringify({ kind: 'long_task_progress', taskId: 'topology-1', expectedDurationMs: 600000, stage: 'SOLVE', progressCurrent: 33, progressTotal: 120, jobId: 'fem-job-1', cancellable: true }),
    });
  });

  const bubble = page.getByTestId('long-task-bubble');
  await expect(bubble).toBeVisible();
  await expect(bubble).toContainText('Topology optimization');
  await expect(bubble).toContainText('SOLVE');
  await expect(page.locator('.agent-card[data-event-id="long-task-1"]')).toHaveCount(0);

  await bubble.click();
  await expect(page.getByTestId('long-task-details')).toContainText('Factoring load case 2/5');
  await expect(page.getByTestId('long-task-details')).toContainText('33 / 120');
  await expect(page.getByRole('button', { name: 'Cancel FEM job' })).toBeVisible();

  await page.evaluate(() => {
    window.__EMIT_AGENT_ACTIVITY__?.({
      eventId: 'long-task-2', cursor: 8, sessionId: 'fem-session', threadId: 'thread-background',
      messageId: 'message-fem', versionId: 'version-fem',
      actor: { kind: 'agent', id: 'fem', label: 'FEM' }, kind: 'trace',
      lifecycleKey: 'long-task:topology-1', phase: 'idle', summary: 'Topology optimization complete',
      detail: 'Published verified analysis', severity: 'success', state: 'resolved', requiresAttention: false,
      occurredAt: Date.now(), raw: JSON.stringify({ kind: 'long_task_finished', taskId: 'topology-1' }),
    });
  });

  await expect(bubble).toHaveCount(0);
  await expect(page.locator('.agent-card[data-event-id="long-task-2"]')).toBeVisible();
  await expect(page.locator('.agent-card[data-event-id="long-task-2"]')).toContainText('Topology optimization complete');
});

test('Given a cancellable FEM job When cancel is clicked and backend fails Then command is sent and raw failure stays visible', async ({ page }) => {
  await installAgentActivityMocks(page);
  await page.goto('/');

  await page.evaluate(() => {
    window.__EMIT_AGENT_ACTIVITY__?.({
      eventId: 'cancel-task-1', cursor: 7, sessionId: 'fem-cancel-session', threadId: 'thread-background',
      messageId: 'message-fem', versionId: null,
      actor: { kind: 'agent', id: 'fem', label: 'FEM' }, kind: 'trace',
      lifecycleKey: 'long-task:cancel-topology', phase: 'mesh', summary: 'Remeshing candidate',
      detail: 'Gmsh HXT', severity: 'info', state: 'active', requiresAttention: false,
      occurredAt: Date.now(),
      raw: JSON.stringify({ kind: 'long_task_started', taskId: 'cancel-topology', expectedDurationMs: 600000, stage: 'MESH', jobId: 'cancel-job', cancellable: true }),
    });
  });

  await page.getByTestId('long-task-bubble').click();
  await page.getByTestId('long-task-details').getByRole('button', { name: 'Cancel FEM job' }).click();
  await expect.poll(async () => (await page.evaluate(() => window.__AGENT_ACTIVITY_CALLS__ ?? []))
    .filter((call) => call.cmd === 'cancel_fem_study').length).toBe(1);

  await page.evaluate(() => {
    window.__EMIT_AGENT_ACTIVITY__?.({
      eventId: 'cancel-task-2', cursor: 8, sessionId: 'fem-cancel-session', threadId: 'thread-background',
      messageId: 'message-fem', versionId: null,
      actor: { kind: 'agent', id: 'fem', label: 'FEM' }, kind: 'trace',
      lifecycleKey: 'long-task:cancel-topology', phase: 'error', summary: 'Remeshing candidate failed',
      detail: 'Gmsh process exited 137', severity: 'error', state: 'failed', requiresAttention: true,
      occurredAt: Date.now(), raw: JSON.stringify({ kind: 'long_task_finished', taskId: 'cancel-topology' }),
    });
  });

  await expect(page.getByTestId('long-task-bubble')).toHaveCount(0);
  const failure = page.locator('.agent-card[data-event-id="cancel-task-2"]');
  await expect(failure).toHaveClass(/agent-card--error/);
  await expect(failure).toContainText('Gmsh process exited 137');
});

test('Given six rapid cross-thread events When workbench loads Then one Ecky stack preserves every event', async ({ page }) => {
  await installAgentActivityMocks(page);
  await page.goto('/');

  const center = page.locator('.genie-layer .agent-notification-center');
  const activityCards = center.locator('.agent-card[data-event-id^="event-"]');
  await expect(center).toBeVisible();
  await expect(page.locator('.genie-bubble')).toHaveCount(0);
  await expect(activityCards).toHaveCount(4);
  await expect(center.locator('[data-event-id="event-3"]')).toHaveClass(/agent-card--active/);
  await expect(center.locator('[data-event-id="event-2"]')).toHaveClass(/agent-card--muted/);
  await expect(center.locator('[data-event-id="event-4"]')).toBeVisible();
  await expect(center.locator('[data-event-id="event-5"]')).toBeVisible();

  await center.locator('[data-event-id="event-4"]').click();
  await expect(center.locator('[data-event-id="event-4"]')).toHaveClass(/agent-card--active/);
  await expect(page.getByTestId('activity-event-list')).toHaveCount(0);
  await expect(page.locator('.prompt-input')).toBeVisible();
  await expect(page.locator('.prompt-input')).toBeFocused();

  await center.locator('[data-event-id="event-3"] .agent-card__action').filter({ hasText: 'DISMISS' }).click();
  await expect(center.locator('[data-event-id="event-6"]')).toBeVisible();
  await center.locator('[data-event-id="event-6"]').click();
  await expect(page.getByTestId('activity-event-list')).toBeVisible();
  await expect(page.getByTestId('activity-event-detail')).toContainText('quota exceeded');

  const calls = await page.evaluate(() => window.__AGENT_ACTIVITY_CALLS__ ?? []);
  expect(calls.filter((call) => call.cmd === 'get_agent_activity')).toHaveLength(1);
  expect(calls.filter((call) => call.cmd === 'get_thread_agent_state')).toHaveLength(0);
});

test('Given activity exists When Ecky is double-clicked Then activity list opens', async ({ page }) => {
  await installAgentActivityMocks(page);
  await page.goto('/');

  await page.getByRole('button', { name: 'Poke Ecky' }).dblclick();

  await expect(page.getByTestId('activity-event-list')).toBeVisible();
  await expect(page.getByTestId('activity-event-list')).toContainText('Provider rejected request');
  await expect(page.getByTestId('activity-event-list')).toContainText('Needs confirmation');
});

test('Given mixed notification states When ttl passes Then every card expires into Activity Hub', async ({ page }) => {
  await page.clock.install();
  await installAgentActivityMocks(page);
  await page.goto('/');

  const center = page.locator('.genie-layer .agent-notification-center');
  const activityCards = center.locator('.agent-card[data-event-id^="event-"]');
  await expect(activityCards).toHaveCount(4);

  await page.clock.fastForward(8_000);

  await expect(center.locator('[data-event-id="event-6"]')).toBeVisible();
  await page.clock.fastForward(8_000);

  await expect(activityCards).toHaveCount(0);
  await expect(center.locator('[data-event-id="event-3"]')).toHaveCount(0);
  await expect(center.locator('[data-event-id="event-5"]')).toHaveCount(0);

  await page.getByRole('button', { name: 'Poke Ecky' }).dblclick();
  await expect(page.getByTestId('activity-event-list')).toContainText('Provider rejected request');
});
