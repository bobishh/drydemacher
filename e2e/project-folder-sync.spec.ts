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
    };
    __emitTauriEvent?: (event: string, payload: unknown) => void;
  }
}

function projectFolderMockScript() {
  (window as any).__PROJECT_FOLDER_CALLS__ = [];
  (window as any).__PROJECT_FOLDER_STATE__ = {
    status: 'missing',
    applyError: null,
    applied: false,
  };

  window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
  const eventHandlers = new Map<string, number>();
  let nextCallbackId = 1;
  window.__TAURI_INTERNALS__.transformCallback = (callback: unknown) => {
    const callbackId = nextCallbackId++;
    (window as unknown as Record<string, unknown>)[`_${callbackId}`] = callback;
    return callbackId;
  };
  window.__emitTauriEvent = (event, payload) => {
    const callbackId = eventHandlers.get(event);
    const callback = callbackId
      ? (window as unknown as Record<string, unknown>)[`_${callbackId}`]
      : null;
    if (typeof callback === 'function') {
      callback({ event, id: callbackId, payload });
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
      eventHandlers.set(String(args?.event ?? ''), handler);
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
    if (cmd === 'get_history') return [];
    if (cmd === 'get_last_design') return null;
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
        previewStlPath: '/mock.stl',
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
          previewStlSizeBytes: 1024,
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
    if (cmd === 'project_folder_apply') {
      const state = (window as any).__PROJECT_FOLDER_STATE__;
      if (state.applyError) {
        throw state.applyError;
      }
      state.applied = true;
      state.status = 'clean';
      return {
        stateBefore: 'fileChanged',
        noOp: false,
        threadId: 'mock-thread-1',
        messageId: 'mock-msg-2',
        modelId: 'mock-model-2',
        manifest: { ...manifest, messageId: 'mock-msg-2', sourceDigest: 'sha256:edited' },
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
  await page.getByRole('button', { name: 'PARAMS' }).click();
  await expect(page.locator('.param-panel')).toBeVisible({ timeout: 10000 });
}

test.describe('Project folder mirror (filesystem-project-mirror T5.2/T5.3)', () => {
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

  test('Given an exported folder with an external edit When Apply runs Then a new version is committed and the chip goes clean', async ({
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
        { kind: 'applyFailed', slug: 'bracket-abc12345', error: 'simulated edit pending' },
      ]);
    });

    await expect(chip).toContainText(/changed/i, { timeout: 10000 });
    await expect(chip.getByRole('button', { name: /APPLY/i })).toBeVisible();

    await chip.getByRole('button', { name: /APPLY/i }).click();

    await expect.poll(() =>
      page.evaluate(() =>
        (window.__PROJECT_FOLDER_CALLS__ ?? []).some((c) => c.cmd === 'project_folder_apply'),
      ),
    ).toBeTruthy();
    await expect.poll(() =>
      page.evaluate(() => (window as any).__PROJECT_FOLDER_STATE__?.applied === true),
    ).toBe(true);

    // After a successful apply the chip refreshes to clean.
    await expect(chip).toContainText(/clean/i, { timeout: 10000 });
  });

  test('Given a stale folder When Apply runs without force Then it refuses and surfaces the exact reason', async ({
    page,
  }) => {
    await bootIntoBracketWorkspace(page);

    const chip = page.getByTestId('project-folder-chip');
    await expect(chip).toBeVisible({ timeout: 10000 });

    await chip.getByRole('button', { name: /EXPORT/i }).click();
    await page.evaluate(() => {
      (window as any).__PROJECT_FOLDER_STATE__.status = 'threadAdvanced';
      (window as any).__PROJECT_FOLDER_STATE__.applyError = {
        code: 'validation',
        message: 'Project folder is stale: thread advanced past the exported version.',
        details: 'Run project_folder_export to refresh the folder.',
      };
    });
    await page.evaluate(() => {
      window.__emitTauriEvent?.('project-folder-sync', []);
    });

    await expect(chip).toContainText(/stale|advanced/i, { timeout: 10000 });

    // Apply must be offered (the user can still attempt it) but the backend
    // refuses; the raw error text is surfaced verbatim, not a generic message.
    await chip.getByRole('button', { name: /APPLY/i }).click();

    await expect.poll(() =>
      page.evaluate(() =>
        (window.__PROJECT_FOLDER_CALLS__ ?? []).some((c) => c.cmd === 'project_folder_apply'),
      ),
    ).toBeTruthy();
    await expect(chip).toContainText(/stale: thread advanced past the exported version/i);
    // Re-export is the documented remediation and must be available.
    await expect(chip.getByRole('button', { name: /RE-EXPORT/i })).toBeVisible();
  });
});
