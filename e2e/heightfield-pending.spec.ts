import { expect, test } from '@playwright/test';

test('Given an empty heightfield image When design is generated Then render waits until image selection and Apply', async ({
  page,
}) => {
  await page.route(/\/heightfield\.stl(?:\?.*)?$/, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'model/stl',
      body: `solid heightfield
facet normal 0 0 1
outer loop
vertex 0 0 0
vertex 1 0 0
vertex 0 1 0
endloop
endfacet
endsolid heightfield
`,
    });
  });

  await page.addInitScript(() => {
    const calls: Array<{ cmd: string; args: unknown }> = [];
    (window as any).__HEIGHTFIELD_CALLS__ = calls;
    (window as any).__TAURI_INTERNALS__ = (window as any).__TAURI_INTERNALS__ || {};
    (window as any).__TAURI_INTERNALS__.invoke = async (cmd: string, args: any) => {
      calls.push({ cmd, args });
      if (cmd === 'get_config') {
        return {
          engines: [{ id: 'mock', name: 'Mock' }],
          selectedEngineId: 'mock',
          hasSeenOnboarding: true,
        };
      }
      if (cmd === 'get_runtime_capabilities') {
        return {
          freecad: { available: false, detail: 'Unavailable', path: null },
          build123d: { available: false, detail: 'Unavailable', path: null },
          mesh: { available: true, detail: 'bundled', path: null },
          recommendedAuthoringContext: {
            engineKind: 'ecky',
            sourceLanguage: 'eckyIrV0',
            geometryBackend: 'eckyRust',
          },
        };
      }
      if (cmd === 'check_freecad') return false;
      if (cmd === 'get_history') return [];
      if (cmd === 'get_last_design') return null;
      if (cmd === 'get_default_macro') return '(model)';
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
      if (cmd === 'init_generation_attempt') return 'heightfield-message';
      if (cmd === 'classify_intent') {
        return {
          intentMode: 'design',
          response: 'Routing request...',
          finalResponse: '',
          confidence: 0.99,
          usage: null,
        };
      }
      if (cmd === 'generate_design') {
        return {
          threadId: args?.threadId || 'heightfield-thread',
          messageId: 'heightfield-message',
          usage: null,
          design: {
            title: 'Heightfield Relief',
            versionName: 'V1',
            interactionMode: 'design',
            macroCode:
              '(model (params (image heightmap "")) (part relief (heightfield heightmap :width 100 :depth 70 :relief-height 4 :base-thickness 1.2)))',
            sourceLanguage: 'eckyIrV0',
            geometryBackend: 'eckyRust',
            uiSpec: {
              fields: [
                {
                  type: 'image',
                  key: 'heightmap',
                  label: 'Height Map',
                  frozen: false,
                },
              ],
            },
            initialParams: { heightmap: '' },
          },
        };
      }
      if (cmd === 'render_model') {
        return {
          modelId: 'heightfield-model',
          sourceKind: 'generated',
          contentHash: 'heightfield-hash',
          fcstdPath: '',
          manifestPath: '/heightfield-manifest.json',
          previewStlPath: '/heightfield.stl',
          viewerAssets: [],
          calloutAnchors: [],
          measurementGuides: [],
          edgeTargets: [],
        };
      }
      if (cmd === 'get_model_manifest') {
        return {
          modelId: 'heightfield-model',
          sourceKind: 'generated',
          sourceLanguage: 'eckyIrV0',
          geometryBackend: 'eckyRust',
          document: {
            documentName: 'Heightfield Relief',
            documentLabel: 'Heightfield Relief',
            objectCount: 1,
            warnings: [],
          },
          parts: [],
          parameterGroups: [],
          controlPrimitives: [],
          controlRelations: [],
          controlViews: [],
          selectionTargets: [],
          taggedAnchors: {},
          advisories: [],
          measurementAnnotations: [],
          warnings: [
            'Mesh evidence: part=relief digest=sha256:heightfield triangles=12 boundaryOrNonManifoldEdges=0 topology=closed',
          ],
          enrichmentState: { status: 'none', proposals: [] },
        };
      }
      if (cmd === 'verify_generated_model') {
        return {
          passed: true,
          summary: 'Checks passed.',
          issues: [],
          metrics: {
            partCount: 1,
            previewStlSizeBytes: 256,
            totalVolume: 1,
            totalArea: 1,
            bbox: { xMin: 0, yMin: 0, zMin: 0, xMax: 1, yMax: 1, zMax: 1 },
          },
          verifierStatus: 'ok',
          verifierSource: 'mock',
        };
      }
      if (cmd === 'get_thread') {
        return {
          id: args?.id,
          title: 'Heightfield Relief',
          updatedAt: Date.now() / 1000,
          versionCount: 1,
          pendingCount: 0,
          errorCount: 0,
          summary: '',
          messages: [],
        };
      }
      if (cmd === 'plugin:dialog|open') return '/tmp/height-map.png';
      if (cmd === 'plugin:dialog|save') return '/tmp/exported-heightfield.stl';
      if (
        cmd === 'export_file' ||
        cmd === 'save_model_manifest' ||
        cmd === 'finalize_generation_attempt' ||
        cmd === 'save_last_design' ||
        cmd === 'save_config'
      ) {
        return null;
      }
      return null;
    };
  });

  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make a heightfield relief');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');

  expect(
    await page.evaluate(
      () =>
        (window as any).__HEIGHTFIELD_CALLS__.filter(
          (entry: { cmd: string }) => entry.cmd === 'render_model',
        ).length,
    ),
  ).toBe(0);

  await page.getByRole('button', { name: 'PARAMS' }).click({ force: true });
  await expect(page.locator('.param-panel')).toBeVisible();
  await expect(page.getByRole('status')).toContainText(
    'Heightfield pending image selection: Height Map',
  );
  await page.getByRole('button', { name: 'RAW', exact: true }).click();
  await page.getByRole('button', { name: 'Select Image...' }).last().click();
  await expect(page.getByRole('button', { name: 'height-map.png' }).last()).toBeVisible();
  await page.getByRole('button', { name: 'APPLY' }).click();

  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any).__HEIGHTFIELD_CALLS__.filter(
            (entry: { cmd: string }) => entry.cmd === 'render_model',
          ).length,
      ),
    )
    .toBe(1);

  await expect(page.getByText(/Mesh evidence: part=relief/)).toContainText(
    'boundaryOrNonManifoldEdges=0 topology=closed',
  );
  await page.getByRole('button', { name: '×' }).click();
  await page.getByRole('button', { name: /EXPORT/ }).click();
  await page.getByRole('button', { name: /^STL/ }).click();
  await expect
    .poll(() =>
      page.evaluate(() =>
        (window as any).__HEIGHTFIELD_CALLS__.find(
          (entry: { cmd: string }) => entry.cmd === 'export_file',
        ),
      ),
    )
    .toMatchObject({
      args: {
        sourcePath: '/heightfield.stl',
        targetPath: '/tmp/exported-heightfield.stl',
      },
    });
});
