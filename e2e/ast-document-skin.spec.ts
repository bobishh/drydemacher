import { expect, test, type Page } from '@playwright/test';

/**
 * BDD acceptance for filesystem-project-mirror task 6.1: the document-skin
 * ("literate") renderer is an alternate LAYOUT over the existing AstMap
 * projection — same stable node ids, same patch intents, nested-document
 * presentation instead of the spatial scene. It must not be a separate
 * editor with its own identity model (per fsm design "Literate projection
 * note" and macro-ast-map-editor design).
 *
 * Two scenarios per the macro-ast-map-editor BDD proof rule (one happy path,
 * one pending state):
 *   - happy: the document skin renders the SAME node ids as the spatial map
 *     for the same macro, and an inline param edit in DOC view emits the
 *     identical render patch (source/preview update).
 *   - pending: an empty macro shows a pending document state without fake
 *     content and without crashing.
 */

declare global {
  interface Window {
    __AST_DOC_CALLS__?: Array<{ cmd: string; args?: Record<string, unknown> }>;
  }
}

function astDocumentSkinMockScript() {
  (window as any).__AST_DOC_CALLS__ = [];

  window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
  window.__TAURI_INTERNALS__.metadata = {};
  window.__TAURI_INTERNALS__.transformCallback = (callback: unknown) => {
    const id = Math.floor(Math.random() * 1_000_000_000);
    (window as any)[`_${id}`] = callback;
    return id;
  };

  window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: Record<string, unknown>) => {
    (window as any).__AST_DOC_CALLS__.push({ cmd, args });
    if (cmd === 'plugin:event|listen') return Number(args?.handler ?? 1);
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
      const prompt = `${args?.prompt ?? ''}`;
      // Seeded macro: a model with a part carrying two numeric params.
      return {
        threadId: args?.threadId || 'mock-thread-1',
        messageId: 'mock-msg-1',
        usage: null,
        design: {
          title: 'Document Bracket',
          versionName: 'V1',
          interactionMode: 'design',
          macroCode:
            '(model\n' +
            '  (part body (box width height depth))\n' +
            ')\n',
          sourceLanguage: 'legacyPython',
          geometryBackend: 'freecad',
          engineKind: 'freecad',
          uiSpec: {
            fields: [
              { type: 'number', key: 'width', label: 'Width' },
              { type: 'number', key: 'height', label: 'Height' },
            ],
          },
          initialParams: { width: 10, height: 20 },
          postProcessing: null,
        },
      };
    }
    if (cmd === 'macro_ast_source_map') {
      // Byte-accurate identity for the seeded macro so the shared projection
      // assigns stable node ids in both map and document views.
      const code = '(model\n  (part body (box width height depth))\n)\n';
      const partStart = code.indexOf('(part body');
      return [
        { id: 'model', kind: 'model', label: 'model', startByte: 0, endByte: code.length },
        {
          id: 'part:body',
          kind: 'part',
          label: 'body',
          startByte: partStart,
          endByte: partStart + '(part body (box width height depth))'.length,
        },
      ];
    }
    if (cmd === 'render_model') {
      // Failure-path hook: the DOC-view failure test arms this to prove the
      // document skin surfaces raw backend errors through the shared patch
      // + error flow (no generic "check API" masking).
      if ((window as any).__AST_DOC_RENDER_FAIL__) {
        throw {
          code: 'render',
          message: 'width 999 is out of range',
          details: 'Parameter width must be <= 100.',
        };
      }
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
        document: {
          documentName: 'Document Bracket',
          documentLabel: 'Document Bracket',
          objectCount: 0,
          warnings: [],
        },
        parts: [{ partId: 'body', label: 'body', kind: 'solid', editable: true, parameterKeys: ['width', 'height'] }],
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
        title: 'Document Bracket',
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
    return null;
  };
}

async function bootSeededMacro(page: Page, prompt: string) {
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
  await page.addInitScript(astDocumentSkinMockScript);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', prompt);
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');
  await page.getByTestId('workbench-bottom-dock').getByRole('button', { name: 'PARAMS' }).click();
  await expect(page.locator('.param-panel')).toBeVisible({ timeout: 10000 });
  // The app may auto-open the code window on some generation paths; dismiss
  // it so it cannot intercept the New Params nav click (a real, legitimately
  // openable window — not a bug, just stacking the test must navigate past).
  const codeClose = page.locator('.window[data-window-id="code"] .window-close');
  if (await codeClose.count()) {
    await codeClose.first().click().catch(() => {});
  }
  await page.getByRole('button', { name: 'new params', exact: true }).click();
  await expect(page.locator('.macro-ast-map-shell')).toBeVisible();
}

async function readAstNodeIds(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    return Array.from(document.querySelectorAll('[data-node-id]'))
      .map((el) => el.getAttribute('data-node-id'))
      .filter((id): id is string => Boolean(id));
  });
}

test.describe('Macro AST document skin (filesystem-project-mirror 6.1)', () => {
  test('Given a seeded macro When the document skin opens Then it shows the same node ids as the map and an inline edit emits the shared param patch', async ({
    page,
  }) => {
    await bootSeededMacro(page, 'make a seeded macro');

    // Capture node ids from the spatial map (the existing projection).
    const mapIds = await readAstNodeIds(page);
    expect(mapIds.length).toBeGreaterThan(0);

    // Switch to the document skin.
    await page.getByTestId('ast-view-doc').click();
    await expect(page.locator('.macro-ast-document')).toBeVisible();
    await expect(page.locator('.macro-ast-map-shell')).toHaveCount(0);

    const docIds = await readAstNodeIds(page);
    // Same stable node ids — the document skin reuses the one projection,
    // it is not a separate editor with its own identity model.
    expect(docIds.sort()).toEqual(mapIds.sort());

    // The document skin renders an inline control for the `width` param
    // (shared ParamPanelControlField). Editing it must emit the SAME patch
    // the spatial map emits (render_model via Apply) — proving intent parity.
    const widthInput = page
      .locator('.macro-ast-document [data-param-key="width"] input[type="number"]')
      .first();
    await expect(widthInput).toBeVisible();
    await widthInput.fill('42');
    const applyBtn = page.locator('.param-panel .apply-btn');
    await expect(applyBtn).toBeEnabled();
    await applyBtn.click();

    await expect.poll(() =>
      page.evaluate(() =>
        (window.__AST_DOC_CALLS__ ?? []).some(
          (c) => c.cmd === 'render_model' && (c.args?.parameters as any)?.width === 42,
        ),
      ),
    ).toBeTruthy();
  });

  test('Given a seeded macro When an inline edit in DOC view fails to render Then the raw backend error surfaces', async ({
    page,
  }) => {
    await bootSeededMacro(page, 'make a seeded macro');

    await page.getByTestId('ast-view-doc').click();
    await expect(page.locator('.macro-ast-document')).toBeVisible();

    // Arm the mock so the next render fails with a raw, specific error body.
    await page.evaluate(() => {
      (window as any).__AST_DOC_RENDER_FAIL__ = true;
    });

    const widthInput = page
      .locator('.macro-ast-document [data-param-key="width"] input[type="number"]')
      .first();
    await expect(widthInput).toBeVisible();
    await widthInput.fill('999');
    const applyBtn = page.locator('.param-panel .apply-btn');
    await expect(applyBtn).toBeEnabled();
    await applyBtn.click();

    // The document skin shares the map's patch + error path: the raw
    // backend error surfaces verbatim (no generic "check API" message) in
    // the Ecky session bubble.
    await expect(page.getByTestId('genie-session-bubble')).toContainText(
      /width 999 is out of range/i,
      { timeout: 10000 },
    );
  });
});
