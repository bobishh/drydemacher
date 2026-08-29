import { expect, test, type Page } from '@playwright/test';

/**
 * BDD acceptance for filesystem-project-mirror T5.2/T5.3.
 *
 * Drives the project-folder mirror UI through the two spec scenarios that
 * matter for the app shell:
 *   - happy path: fileChanged -> Apply -> new committed version -> clean
 *   - conflict path: threadAdvanced -> Apply refuses with the exact reason
 *
 * The Tauri boundary is mocked through __TAURI_INTERNALS__.invoke; the
 * `project-folder-sync` event is replayed through __emitTauriEvent so the
 * status chip refreshes exactly as it does against the real watcher.
 */

declare global {
  interface Window {
    __PROJECT_FOLDER_CALLS__?: Array<{ cmd: string; args?: Record<string, unknown> }>;
    __PROJECT_FOLDER_STATE__?: {
      status: 'missing' | 'clean' | 'fileChanged' | 'threadAdvanced' | 'conflict';
      applyError?: { code: string; message: string; details?: string } | null;
      applied?: boolean;
      renderActivity?: Array<{ slug: string; threadId: string }>;
      timelineRows?: Array<Record<string, unknown>>;
    };
    __PROJECT_FOLDER_HOLD_BOOT__?: boolean;
    __emitTauriEvent?: (event: string, payload: unknown) => void;
  }
}

function projectFolderMockScript() {
  (window as any).__PROJECT_FOLDER_CALLS__ = [];
  (window as any).__PROJECT_FOLDER_STATE__ = {
    status: 'missing',
    applyError: null,
    applied: false,
    renderActivity: [],
    timelineRows: [],
  };

  window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
  const eventHandlers = new Map<string, number[]>();
  let nextCallbackId = 1;
  window.__TAURI_INTERNALS__.transformCallback = (callback: unknown) => {
    const callbackId = nextCallbackId++;
    (window as unknown as Record<string, unknown>)[`_${callbackId}`] = callback;
    return callbackId;
  };
  window.__emitTauriEvent = (event, payload) => {
    for (const callbackId of eventHandlers.get(event) ?? []) {
      const callback = (window as unknown as Record<string, unknown>)[`_${callbackId}`];
      if (typeof callback === 'function') {
        callback({ event, id: callbackId, payload });
      }
    }
  };

  const manifest = {
    schemaVersion: 1,
    projectId: 'proj-mock-1',
    threadId: 'mock-thread-1',
    messageId: 'mock-msg-1',
    modelId: 'mock-model-1',
    sourceDigest: 'sha256:exported',
    exportedAt: 1781200000,
  };

  const statusBody = () => {
    const state = (window as any).__PROJECT_FOLDER_STATE__;
    return {
      state: state.status,
      folder: '/mock/projects/bracket-abc12345',
      manifest,
      fileDigest: state.status === 'fileChanged' || state.status === 'conflict'
        ? 'sha256:edited'
        : 'sha256:exported',
      threadHeadMessageId: state.status === 'threadAdvanced' || state.status === 'conflict'
        ? 'mock-msg-2'
        : 'mock-msg-1',
    };
  };

  window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: Record<string, unknown>) => {
    (window as any).__PROJECT_FOLDER_CALLS__.push({ cmd, args });
    if (cmd === 'plugin:event|listen') {
      const handler = Number(args?.handler);
      const event = String(args?.event ?? '');
      eventHandlers.set(event, [...(eventHandlers.get(event) ?? []), handler]);
      return handler;
    }
    if (cmd === 'plugin:event|unlisten') return null;
    if (cmd === 'get_config') {
      return {
        engines: [{ id: 'mock', name: 'Mock' }],
        selectedEngineId: 'mock',
        hasSeenOnboarding: true,
      };
    }
    if (cmd === 'get_runtime_capabilities') {
      return {
        freecad: { available: true, detail: 'Ready at /mock/freecadcmd', path: '/mock/freecadcmd' },
        build123d: { available: true, detail: 'Ready at /mock/python3', path: '/mock/python3' },
        mesh: { available: true, detail: 'bundled', path: null },
        recommendedAuthoringContext: {
          engineKind: 'freecad',
          sourceLanguage: 'legacyPython',
          geometryBackend: 'freecad',
        },
      };
    }
    if (cmd === 'check_freecad') return true;
    if (cmd === 'get_history') {
      if (((window as any).__PROJECT_FOLDER_STATE__.timelineRows ?? []).length > 0) {
        return [{
          id: 'mock-thread-1',
          title: 'Bracket',
          summary: 'Bracket thread',
          messages: [],
          updatedAt: 1781200100,
          versionCount: 1,
          pendingCount: 0,
          queuedCount: 0,
          errorCount: 0,
          status: 'active',
        }];
      }
      return [];
    }
    if (cmd === 'get_last_design') {
      if (window.__PROJECT_FOLDER_HOLD_BOOT__) {
        return new Promise(() => {});
      }
      return null;
    }
    if (cmd === 'get_default_macro') return '# macro';
    if (cmd === 'init_generation_attempt') return 'mock-msg-1';
    if (cmd === 'classify_intent') {
      return {
        intentMode: 'design',
        response: 'Routing request...',
        finalResponse: '',
        confidence: 0.9,
        usage: null,
      };
    }
    if (cmd === 'generate_design') {
      return {
        threadId: args?.threadId || 'mock-thread-1',
        messageId: 'mock-msg-1',
        usage: null,
        design: {
          title: 'Bracket',
          versionName: 'V1',
          interactionMode: 'design',
          macroCode: 'print("bracket")',
          sourceLanguage: 'legacyPython',
          geometryBackend: 'freecad',
          engineKind: 'freecad',
          uiSpec: { fields: [{ type: 'number', key: 'width', label: 'Width' }] },
          initialParams: { width: 10 },
          postProcessing: null,
        },
      };
    }
    if (cmd === 'render_model') {
      return {
        modelId: 'mock-model-1',
        sourceKind: 'generated',
        sourceLanguage: 'legacyPython',
        geometryBackend: 'freecad',
        engineKind: 'freecad',
        contentHash: 'mock-hash',
        artifactVersion: 1,
        fcstdPath: '/mock.FCStd',
        manifestPath: '/mock/manifest.json',
        macroPath: '/mock.py',
        modelStlPath: '/mock.stl',
        viewerAssets: [],
        calloutAnchors: [],
        measurementGuides: [],
        edgeTargets: [],
        faceTargets: [],
      };
    }
    if (cmd === 'get_model_manifest') {
      return {
        modelId: 'mock-model-1',
        sourceKind: 'generated',
        sourceLanguage: 'legacyPython',
        geometryBackend: 'freecad',
        document: { documentName: 'Bracket', documentLabel: 'Bracket', objectCount: 0, warnings: [] },
        parts: [],
        parameterGroups: [],
        controlPrimitives: [],
        controlRelations: [],
        controlViews: [],
        selectionTargets: [],
        advisories: [],
        measurementAnnotations: [],
        warnings: [],
        enrichmentState: { status: 'none', proposals: [] },
      };
    }
    if (cmd === 'verify_generated_model') {
      return {
        passed: true,
        summary: 'Structural checks passed.',
        issues: [],
        metrics: {
          partCount: 1,
          modelStlSizeBytes: 1024,
          totalVolume: 1000,
          totalArea: 500,
          bbox: { xMin: 0, yMin: 0, zMin: 0, xMax: 10, yMax: 10, zMax: 10 },
        },
        verifierStatus: 'ok',
        verifierSource: 'mock',
      };
    }
    if (cmd === 'get_thread') {
      return {
        id: args?.id,
        title: 'Bracket',
        updatedAt: Date.now() / 1000,
        versionCount: 1,
        pendingCount: 0,
        errorCount: 0,
        summary: '',
        messages: [],
      };
    }
    if (cmd === 'get_thread_messages_page') {
      return {
        messages: (window as any).__PROJECT_FOLDER_STATE__.timelineRows ?? [],
        nextBefore: null,
        hasMore: false,
        observedBytes: 0,
        truncatedFields: [],
      };
    }
    if (cmd === 'save_model_manifest') return null;
    if (cmd === 'add_manual_version') return 'mock-param-version-1';
    if (cmd === 'update_version_runtime') return null;
    if (cmd === 'update_parameters') return null;
    if (cmd === 'update_post_processing') return null;
    if (cmd === 'finalize_generation_attempt') return null;
    if (cmd === 'save_last_design') return null;
    if (cmd === 'save_config') return null;
    if (cmd === 'get_active_agent_sessions') return [];
    if (cmd === 'get_agent_terminal_snapshots') return [];
    if (cmd === 'get_agent_activity') return { events: [], latestCursor: 0 };
    if (cmd === 'get_thread_agent_state') {
      return {
        threadId: args?.threadId ?? null,
        connectionState: 'disconnected',
        sessions: [],
        primaryAgentLabel: null,
        statusText: '',
      };
    }
    // --- filesystem-project-mirror commands (T5.2) ---
    if (cmd === 'project_folder_export') {
      (window as any).__PROJECT_FOLDER_STATE__.status = 'clean';
      return {
        slug: 'bracket-abc12345',
        folder: '/mock/projects/bracket-abc12345',
        manifest,
      };
    }
    if (cmd === 'project_folder_status') {
      return statusBody();
    }
    if (cmd === 'project_folder_render_activity') {
      return (window as any).__PROJECT_FOLDER_STATE__.renderActivity ?? [];
    }
    if (cmd === 'get_project_source') {
      return {
        threadId: String(args?.threadId),
        slug: 'bracket-abc12345',
        folder: '/mock/projects/bracket-abc12345',
        file: '/mock/projects/bracket-abc12345/model.ecky',
        source: '(model (part body (solidify (import-stl "scan.stl"))))',
      };
    }
    if (cmd === 'open_or_create_blank_design_thread') {
      return {
        threadId: 'mock-thread-1',
        slug: 'bracket-abc12345',
        folder: '/mock/projects/bracket-abc12345',
        file: '/mock/projects/bracket-abc12345/model.ecky',
        source: '(model (part body (box 20 20 20)))',
      };
    }
    if (cmd === 'open_project_in_editor' || cmd === 'reveal_project_folder') {
      return {
        slug: 'bracket-abc12345',
        folder: '/mock/projects/bracket-abc12345',
        file: '/mock/projects/bracket-abc12345/model.ecky',
      };
    }
    return null;
  };
}

async function bootIntoBracketWorkspace(page: Page) {
  await page.route(/\/mock\.stl(?:\?.*)?$/, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'model/stl',
      body: `solid mock
facet normal 0 0 0
outer loop
vertex 0 0 0
vertex 1 0 0
vertex 0 1 0
endloop
endfacet
endsolid mock
`,
    });
  });
  await page.addInitScript(projectFolderMockScript);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make a bracket');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');
  await page.getByRole('button', { name: 'Parameters', exact: true }).click();
  await expect(page.locator('.param-panel')).toBeVisible({ timeout: 10000 });
  await expect.poll(() => page.evaluate(() =>
    (window.__PROJECT_FOLDER_CALLS__ ?? []).some(
      (call) => call.cmd === 'plugin:event|listen' && call.args?.event === 'project-folder-sync',
    ),
  )).toBe(true);
}

test.describe('Project folder mirror (filesystem-project-mirror T5.2/T5.3)', () => {
  test('Given an active thread When history changes Then the timeline refreshes without loading the full thread aggregate', async ({ page }) => {
    await bootIntoBracketWorkspace(page);

    const before = await page.evaluate(() => ({
      history: (window.__PROJECT_FOLDER_CALLS__ ?? []).filter((call) => call.cmd === 'get_history').length,
      fullThread: (window.__PROJECT_FOLDER_CALLS__ ?? []).filter((call) => call.cmd === 'get_thread').length,
    }));

    await page.evaluate(() => {
      window.__emitTauriEvent?.('history-updated', {
        threadId: 'mock-thread-1',
        messageId: 'mock-msg-2',
        revision: 2,
        kind: 'messageUpdated',
      });
    });

    await expect.poll(() => page.evaluate(() =>
      (window.__PROJECT_FOLDER_CALLS__ ?? []).filter((call) => call.cmd === 'get_history').length,
    )).toBeGreaterThan(before.history);
    await page.waitForTimeout(100);

    const fullThreadCalls = await page.evaluate(() =>
      (window.__PROJECT_FOLDER_CALLS__ ?? []).filter((call) => call.cmd === 'get_thread').length,
    );
    expect(fullThreadCalls).toBe(before.fullThread);
    await expect(page.locator('.param-panel')).toBeVisible();
  });

  test('Given timeline UI state When fifty invalidations burst Then refresh coalesces and preserves local state', async ({ page }) => {
    await bootIntoBracketWorkspace(page);
    await page.getByRole('button', { name: 'DIALOGUE' }).click();
    const search = page.getByPlaceholder('SEARCH TIMELINE');
    await search.fill('bracket');
    const before = await page.evaluate(() => ({
      pages: (window.__PROJECT_FOLDER_CALLS__ ?? []).filter((call) => call.cmd === 'get_thread_messages_page').length,
      full: (window.__PROJECT_FOLDER_CALLS__ ?? []).filter((call) => call.cmd === 'get_thread').length,
    }));

    await page.evaluate(() => {
      for (let revision = 10; revision < 60; revision += 1) {
        window.__emitTauriEvent?.('history-updated', {
          threadId: 'mock-thread-1',
          messageId: `burst-${revision}`,
          revision,
          kind: 'messageUpdated',
        });
      }
    });

    await expect.poll(() => page.evaluate(() =>
      (window.__PROJECT_FOLDER_CALLS__ ?? []).filter((call) => call.cmd === 'get_thread_messages_page').length,
    )).toBeGreaterThan(before.pages);
    await expect(search).toHaveValue('bracket');
    const after = await page.evaluate(() => ({
      pages: (window.__PROJECT_FOLDER_CALLS__ ?? []).filter((call) => call.cmd === 'get_thread_messages_page').length,
      full: (window.__PROJECT_FOLDER_CALLS__ ?? []).filter((call) => call.cmd === 'get_thread').length,
    }));
    expect(after.pages - before.pages).toBeLessThanOrEqual(2);
    expect(after.full).toBe(before.full);
  });

  test('Given an oversized timeline field When its bounded row arrives Then raw truncation sizes stay visible', async ({ page }) => {
    await bootIntoBracketWorkspace(page);
    await page.getByRole('button', { name: 'DIALOGUE' }).click();
    const pagesBefore = await page.evaluate(() =>
      (window.__PROJECT_FOLDER_CALLS__ ?? []).filter(
        (call) => call.cmd === 'get_thread_messages_page',
      ).length,
    );
    await page.evaluate(() => {
      window.__PROJECT_FOLDER_STATE__!.timelineRows = [{
        id: 'oversized-user-row',
        role: 'user',
        content: 'bounded message preview',
        contentTruncated: true,
        contentObservedBytes: 120000,
        contentAllowedBytes: 8192,
        status: 'success',
        agentOrigin: null,
        timestamp: 1781200100,
        timelineOrder: 42,
        versionSummary: null,
        hasImage: false,
        attachmentCount: 0,
        visualKind: null,
      }];
      window.__emitTauriEvent?.('history-updated', {
        threadId: null,
        messageId: 'oversized-user-row',
        revision: 3000,
        kind: 'messageUpdated',
      });
    });

    await expect.poll(() => page.evaluate(() =>
      (window.__PROJECT_FOLDER_CALLS__ ?? []).filter(
        (call) => call.cmd === 'get_thread_messages_page',
      ).length,
    )).toBeGreaterThan(pagesBefore);
    await expect(page.getByText(/120000 BYTES OBSERVED · 8192 ALLOWED/)).toBeVisible();
  });

  test('Given startup has no restored workspace When a folder render exists Then ordinary Loading remains visible', async ({ page }) => {
    await page.addInitScript(projectFolderMockScript);
    await page.addInitScript(() => {
      window.__PROJECT_FOLDER_HOLD_BOOT__ = true;
      window.__PROJECT_FOLDER_STATE__!.renderActivity = [
        { slug: 'filament-dryer-dc939cfd', threadId: 'mock-thread-1' },
      ];
    });

    await page.goto('/');

    await expect.poll(() => page.evaluate(() =>
      (window.__PROJECT_FOLDER_CALLS__ ?? []).some(
        (call) => call.cmd === 'project_folder_render_activity',
      ),
    )).toBe(true);
    await expect(page.locator('.boot-overlay')).toBeVisible();
    await expect(page.locator('.viewport-transmutation')).toHaveCount(0);
  });

  test('Given startup has no file-sync render When boot remains pending Then ordinary Loading remains visible', async ({ page }) => {
    await page.addInitScript(projectFolderMockScript);
    await page.addInitScript(() => {
      window.__PROJECT_FOLDER_HOLD_BOOT__ = true;
    });

    await page.goto('/');

    await expect(page.locator('.boot-overlay')).toBeVisible();
    await expect(page.locator('.viewport-transmutation')).toHaveCount(0);
  });

  test('Given backend render lock changes When geometry runs Then its single activity signal shows and clears render overlay', async ({ page }) => {
    await bootIntoBracketWorkspace(page);

    await page.evaluate(() => {
      window.__PROJECT_FOLDER_STATE__!.renderActivity = [
        { slug: 'bracket-abc12345', threadId: 'mock-thread-1' },
      ];
      window.__emitTauriEvent?.('geometry-render-activity', { activeCount: 1 });
    });

    await expect(page.locator('.viewport-transmutation')).toBeVisible();
    await expect(page.locator('.viewport-transmutation')).toHaveAttribute('data-phase', 'rendering');
    await expect(page.getByRole('img', { name: 'Rendering geometry.' })).toBeVisible();

    await page.evaluate(() => {
      window.__PROJECT_FOLDER_STATE__!.renderActivity = [];
      window.__emitTauriEvent?.('geometry-render-activity', { activeCount: 0 });
    });

    await expect(page.locator('.viewport-transmutation')).toHaveCount(0);
  });

  test('Given a watcher batch contains multiple folders When active source changes Then its render overlay wins', async ({ page }) => {
    await bootIntoBracketWorkspace(page);

    await page.evaluate(() => {
      window.__emitTauriEvent?.('geometry-render-activity', { activeCount: 1 });
      window.__emitTauriEvent?.('project-folder-sync', [
        { kind: 'detected', slug: 'bracket-abc12345', threadId: 'mock-thread-1' },
        {
          kind: 'applied',
          slug: 'background-part',
          threadId: 'background-thread',
          messageId: 'background-message',
          modelId: 'background-model',
        },
      ]);
    });

    await expect(page.locator('.viewport-transmutation')).toBeVisible();
    await expect(page.locator('.viewport-transmutation')).toHaveAttribute('data-phase', 'rendering');
    await expect(page.locator('.agent-notification-center')).toContainText('bracket-abc12345/model.ecky');
    await expect(page.locator('.agent-notification-center')).not.toContainText('background-part');
  });

  test('Given a foreign folder notification without backend render activity Then render UI stays hidden', async ({ page }) => {
    await bootIntoBracketWorkspace(page);

    await page.evaluate(() => {
      window.__emitTauriEvent?.('project-folder-sync', [
        { kind: 'detected', slug: 'background-part', threadId: 'background-thread' },
      ]);
    });

    await expect(page.locator('.viewport-transmutation')).toHaveCount(0);
    await expect(page.locator('.agent-notification-center')).not.toContainText('background-part');
  });

  test('Given model.ecky changes in an editor When auto-render runs Then one global source card advances from rendering to applied', async ({ page }) => {
    await bootIntoBracketWorkspace(page);

    await page.evaluate(() => {
      for (const [cursor, kind, summary] of [
        [1, 'tool_start', 'Replacing macro source for the active target.'],
        [2, 'backend_resolved', 'Resolved macro render backend.'],
        [3, 'auto_heal_applied', 'Reconciled legacy parameters.'],
      ] as const) {
        window.__emitTauriEvent?.('agent-activity-event', {
          eventId: `folder-sync-trace-${cursor}`,
          cursor,
          sessionId: 'project-folder-watcher',
          threadId: 'mock-thread-1',
          messageId: `mock-draft-${cursor}`,
          versionId: null,
          actor: { kind: 'agent', id: 'project-folder-watcher', label: 'folder-sync' },
          kind: 'trace',
          lifecycleKey: `folder-sync:${kind}`,
          phase: 'rendering',
          summary,
          detail: null,
          severity: 'info',
          state: 'active',
          requiresAttention: false,
          occurredAt: 1_800_000_000_000 + cursor,
          raw: JSON.stringify({ kind }),
        });
      }
      window.__emitTauriEvent?.('project-folder-sync', [
        { kind: 'detected', slug: 'bracket-abc12345', threadId: 'mock-thread-1' },
      ]);
      window.__emitTauriEvent?.('geometry-render-activity', { activeCount: 1 });
    });

    const notice = page.locator('.agent-notification-center .agent-card').filter({ hasText: 'SOURCE RENDERING' });
    await expect(notice).toBeVisible();
    await expect(notice).toContainText('SOURCE RENDERING');
    await expect(notice).toContainText('bracket-abc12345/model.ecky');
    await expect(notice).not.toContainText('mock-thread-1');
    await expect(notice).not.toContainText('FOLDER-SYNC');
    await expect(page.locator('.agent-notification-center .agent-card')).toHaveCount(1);
    await expect(page.locator('.viewport-transmutation')).toBeVisible();
    await expect(page.locator('.viewport-transmutation')).toHaveAttribute('data-phase', 'rendering');

    await page.getByTestId('workbench-bottom-dock').getByRole('button', { name: 'CODE' }).click();
    await expect(page.locator('[role="dialog"]').filter({ hasText: 'MACRO INSPECTOR:' })).toBeVisible();

    await page.evaluate(() => {
      window.__emitTauriEvent?.('project-folder-sync', [
        {
          kind: 'applied',
          slug: 'bracket-abc12345',
          threadId: 'mock-thread-1',
          messageId: 'mock-msg-2',
          modelId: 'mock-model-2',
        },
      ]);
      window.__emitTauriEvent?.('geometry-render-activity', { activeCount: 0 });
    });

    await expect(page.locator('.agent-notification-center .agent-card')).toHaveCount(1);
    await expect(page.locator('.agent-notification-center .agent-card')).toContainText('SOURCE APPLIED');
    await expect(page.locator('.viewport-transmutation')).toHaveCount(0);
    await expect(page.getByTestId('project-folder-notice')).toHaveCount(0);
  });

  test('Given an external source edit fails When the watcher reports it Then the raw error is globally visible', async ({ page }) => {
    await bootIntoBracketWorkspace(page);

    await page.getByTestId('workbench-bottom-dock').getByRole('button', { name: 'CODE' }).click();
    const modal = page.locator('[role="dialog"]').filter({ hasText: 'MACRO INSPECTOR:' });

    await page.evaluate(() => {
      window.__emitTauriEvent?.('project-folder-sync', [
        {
          kind: 'applyFailed',
          slug: 'bracket-abc12345',
          threadId: 'mock-thread-1',
          messageId: 'mock-msg-1',
          error: 'line 17: unexpected closing parenthesis',
        },
      ]);
    });

    const notice = page.locator('.agent-notification-center .agent-card').filter({ hasText: 'SOURCE APPLY FAILED' });
    await expect(notice).toBeVisible();
    await expect(notice).toContainText('line 17: unexpected closing parenthesis');
    await expect(page.getByTestId('project-folder-notice')).toHaveCount(0);
    await expect(modal).toContainText('MACRO INSPECTOR:');
  });

  test('Given a blank UI thread When generation starts Then thread initialization runs before design generation', async ({
    page,
  }) => {
    await bootIntoBracketWorkspace(page);

    const calls = await page.evaluate(() => window.__PROJECT_FOLDER_CALLS__ ?? []);
    const initIndex = calls.findIndex((call) => call.cmd === 'init_generation_attempt');
    const generateIndex = calls.findIndex((call) => call.cmd === 'generate_design');
    expect(initIndex).toBeGreaterThanOrEqual(0);
    expect(generateIndex).toBeGreaterThan(initIndex);
    expect(calls[initIndex]?.args?.threadId).toBe(calls[generateIndex]?.args?.threadId);
  });

  test('Given an exported folder with an external edit When watcher owns sync Then manual Apply is not offered', async ({
    page,
  }) => {
    await bootIntoBracketWorkspace(page);

    const chip = page.getByTestId('project-folder-chip');
    await expect(chip).toBeVisible({ timeout: 10000 });

    // Export the active version to seed the folder, then simulate an external
    // edit by flipping the mocked status to fileChanged and replaying the
    // watcher event.
    await chip.getByRole('button', { name: /EXPORT/i }).click();
    await expect.poll(() =>
      page.evaluate(() =>
        (window.__PROJECT_FOLDER_CALLS__ ?? []).some((c) => c.cmd === 'project_folder_export'),
      ),
    ).toBeTruthy();

    await page.evaluate(() => {
      (window as any).__PROJECT_FOLDER_STATE__.status = 'fileChanged';
    });
    await page.evaluate(() => {
      window.__emitTauriEvent?.('project-folder-sync', [
        {
          kind: 'applyFailed',
          slug: 'bracket-abc12345',
          threadId: 'mock-thread-1',
          messageId: 'mock-msg-1',
          error: 'simulated edit pending',
        },
      ]);
    });

    await expect(chip).toContainText(/changed/i, { timeout: 10000 });
    await expect(chip.getByRole('button', { name: /APPLY/i })).toHaveCount(0);
    await expect(chip.getByRole('button', { name: /RE-EXPORT/i })).toBeVisible();
    expect(
      await page.evaluate(() =>
        (window.__PROJECT_FOLDER_CALLS__ ?? []).some((c) => c.cmd === 'project_folder_apply'),
      ),
    ).toBe(false);
  });

  test('Given a stale folder When watcher owns sync Then manual Apply is not offered and re-export remains', async ({
    page,
  }) => {
    await bootIntoBracketWorkspace(page);

    const chip = page.getByTestId('project-folder-chip');
    await expect(chip).toBeVisible({ timeout: 10000 });

    await chip.getByRole('button', { name: /EXPORT/i }).click();
    await page.evaluate(() => {
      (window as any).__PROJECT_FOLDER_STATE__.status = 'threadAdvanced';
    });
    await page.evaluate(() => {
      window.__emitTauriEvent?.('project-folder-sync', []);
    });

    await expect(chip).toContainText(/stale|advanced/i, { timeout: 10000 });

    await expect(chip.getByRole('button', { name: /APPLY/i })).toHaveCount(0);
    await expect(chip.getByRole('button', { name: /RE-EXPORT/i })).toBeVisible();
    expect(
      await page.evaluate(() =>
        (window.__PROJECT_FOLDER_CALLS__ ?? []).some((c) => c.cmd === 'project_folder_apply'),
      ),
    ).toBe(false);
  });
});
