import { expect, test, type Page } from '@playwright/test';

const MOCK_STL = `solid mock
facet normal 0 0 0
outer loop
vertex 0 0 0
vertex 1 0 0
vertex 0 1 0
endloop
endfacet
endsolid mock
`;

const MOCK_STL_OFFSCREEN = `solid background
facet normal 0 0 0
outer loop
vertex 0 0 -1
vertex 1 0 -1
vertex 0 1 -1
endloop
endfacet
endsolid background
`;

const config = {
  engines: [{ id: 'mock', name: 'Mock', provider: 'mock', apiKey: '', baseUrl: '' }],
  selectedEngineId: 'mock',
  freecadCmd: '',
  assets: [],
  microwave: { muted: true },
  voice: { sttLanguageCode: 'en-US' },
  mcp: { port: null, maxSessions: null, mode: 'passive', primaryAgentId: null, promptTimeoutSecs: 1800, autoAgents: [] },
  hasSeenOnboarding: true,
  defaultEngineKind: 'freecad',
  defaultSourceLanguage: 'legacyPython',
  defaultGeometryBackend: 'freecad',
  maxGenerationAttempts: 3,
  maxVerifyAttempts: 1,
};

const runtimeCapabilities = {
  freecad: { available: true, detail: 'Ready at /mock/freecadcmd', path: '/mock/freecadcmd' },
  build123d: { available: true, detail: 'Ready at /mock/python3', path: '/mock/python3' },
  mesh: { available: true, detail: 'bundled', path: null },
  recommendedAuthoringContext: {
    engineKind: 'freecad',
    sourceLanguage: 'legacyPython',
    geometryBackend: 'freecad',
  },
};

const artifactBundle = {
  modelId: 'context-model',
  sourceKind: 'generated',
  engineKind: 'freecad',
  sourceLanguage: 'legacyPython',
  geometryBackend: 'freecad',
  contentHash: 'context-hash',
  artifactVersion: 1,
  fcstdPath: '/mock/context/model.FCStd',
  manifestPath: '/mock/context/manifest.json',
  macroPath: '/mock/context/source.FCMacro',
  modelStlPath: '/mock/context/model.stl',
  viewerAssets: [],
  edgeTargets: [],
  faceTargets: [
    {
      targetId: 'low:face:top',
      durableTargetId: 'low:node:low-node:face:top',
      canonicalTargetId: 'low:face:top:canonical',
      aliasIds: ['low:face:top:alias'],
      partId: 'low',
      viewerNodeId: 'low-node',
      label: 'Low Top Face',
      editable: true,
      center: { x: 0.33, y: 0.33, z: 0 },
      normal: [0, 0, 1],
      area: 100,
    },
  ],
};

const modelManifest = {
  schemaVersion: 2,
  modelId: 'context-model',
  sourceKind: 'generated',
  document: {
    documentName: 'Context Controls',
    documentLabel: 'Context Controls',
    sourcePath: null,
    objectCount: 2,
    warnings: [],
  },
  parts: [
    {
      partId: 'low',
      freecadObjectName: 'Low',
      label: 'Low',
      kind: 'Part::Feature',
      semanticRole: 'body',
      viewerAssetPath: '/mock/context/low.stl',
      viewerNodeIds: ['low-node'],
      parameterKeys: ['low_width'],
      editable: true,
      bounds: null,
      volume: null,
      area: null,
    },
    {
      partId: 'nose',
      freecadObjectName: 'Nose',
      label: 'Nose',
      kind: 'Part::Feature',
      semanticRole: 'connector',
      viewerAssetPath: '/mock/context/nose.stl',
      viewerNodeIds: ['nose-node'],
      parameterKeys: [],
      editable: true,
      bounds: null,
      volume: null,
      area: null,
    },
  ],
  parameterGroups: [],
  controlPrimitives: [],
  controlRelations: [],
  controlViews: [],
  advisories: [],
  selectionTargets: [
    {
      targetId: 'low:face:top',
      durableTargetId: 'low:node:low-node:face:top',
      canonicalTargetId: 'low:face:top:canonical',
      aliasIds: ['low:face:top:alias'],
      partId: 'low',
      viewerNodeId: 'low-node',
      label: 'Low Top Face',
      kind: 'face',
      editable: true,
      parameterKeys: ['low_width'],
      primitiveIds: [],
      viewIds: [],
    },
  ],
  measurementAnnotations: [],
  warnings: [],
  enrichmentState: { status: 'none', proposals: [] },
};

const design = {
  title: 'Context Controls',
  versionName: 'V1',
  response: '',
  interactionMode: 'design',
  macroCode: '# context controls',
  sourceLanguage: 'legacyPython',
  geometryBackend: 'freecad',
  uiSpec: {
    fields: [
      { type: 'number', key: 'hose_od', label: 'Hose OD' },
      { type: 'number', key: 'low_width', label: 'Low Width' },
    ],
  },
  initialParams: { hose_od: 19, low_width: 42 } as Record<string, number>,
  postProcessing: null,
};

function dryerContextFixture(exactKeys: string[] = ['dryer_param_2']) {
  const fields = Array.from({ length: 49 }, (_, index) => ({
    type: 'number',
    key: `dryer_param_${index}`,
    label: `Dryer Param ${index}`,
  }));
  const parts = [
    { partId: 'low', freecadObjectName: 'Low', label: 'Drum', kind: 'solid', semanticRole: 'drum', viewerNodeIds: ['low-node'], parameterKeys: fields.slice(0, 12).map((field) => field.key), editable: true },
    { partId: 'shell', freecadObjectName: 'Shell', label: 'Shell', kind: 'solid', semanticRole: 'enclosure', viewerNodeIds: ['shell-node'], parameterKeys: [fields[0]!.key, ...fields.slice(12, 30).map((field) => field.key)], editable: true },
    { partId: 'air-path', freecadObjectName: 'AirPath', label: 'Air Path', kind: 'solid', semanticRole: 'duct', viewerNodeIds: ['air-node'], parameterKeys: fields.slice(30).map((field) => field.key), editable: true },
  ];
  const manifest = {
    ...modelManifest,
    modelId: 'dryer-context-model',
    engineKind: 'ecky',
    sourceLanguage: 'ecky',
    geometryBackend: 'mesh',
    parts,
    parameterGroups: [],
    controlPrimitives: [],
    controlViews: [],
    selectionTargets: [{
      ...modelManifest.selectionTargets[0],
      label: exactKeys.length ? 'Drum Bore' : 'Ambiguous Drum Face',
      parameterKeys: exactKeys,
    }],
  };
  return {
    artifactBundle: {
      ...artifactBundle,
      modelId: 'dryer-context-model',
      engineKind: 'ecky',
      sourceLanguage: 'ecky',
      geometryBackend: 'mesh',
    },
    modelManifest: manifest,
    design: {
      ...design,
      title: 'Filament Dryer',
      macroCode: '(model (params) (part low (box 1 1 1)))',
      engineKind: 'ecky',
      sourceLanguage: 'ecky',
      geometryBackend: 'mesh',
      uiSpec: { fields },
      initialParams: Object.fromEntries(fields.map((field, index) => [field.key, index + 1])),
    },
  };
}

const authoringGraph = {
  sourceDigest: 'sha256:context-source',
  coreDigest: 'sha256:context-core',
  artifactDigest: 'sha256:context-artifact',
  astNodes: [
    {
      path: 'parts.low',
      stableNodeKey: 'part:low',
      kind: 'part',
      valueKind: 'form',
      operation: 'box',
      partId: 'low',
      sourceAddressable: true,
      editableOps: ['replace'],
      childPaths: [],
      inputPorts: [],
    },
    {
      path: 'parameters.low_width',
      stableNodeKey: 'part:low/param:low_width',
      kind: 'parameter',
      valueKind: 'number',
      partId: 'low',
      sourceAddressable: true,
      editableOps: ['replace'],
      childPaths: [],
      inputPorts: [],
    },
  ],
  features: [
    {
      featureId: 'feature:low-box',
      kind: 'box',
      label: 'Low box',
      sourcePath: 'parts.low',
      sourceStableNodeKey: 'part:low',
      dependencyIds: ['low_width'],
      outputIds: ['output:low'],
      targetIds: ['low:face:top'],
    },
  ],
  dependencies: [
    {
      parameterKey: 'low_width',
      parameterStableNodeKey: 'part:low/param:low_width',
      dependentSourcePaths: ['parts.low'],
      affectedStableNodeKeys: ['part:low'],
      impactedPartIds: ['low'],
      featureIds: ['feature:low-box'],
      targetIds: ['low:face:top'],
    },
  ],
  constraints: [],
  targets: [
    {
      targetId: 'low:face:top',
      durableTargetId: 'low:node:low-node:face:top',
      canonicalTargetId: 'low:face:top:canonical',
      aliasIds: ['low:face:top:alias'],
      partId: 'low',
      viewerNodeId: 'low-node',
      label: 'Low Top Face',
      kind: 'face',
      parameterKeys: ['low_width'],
      primitiveIds: [],
      featureIds: ['feature:low-box'],
      sourceStableNodeKeys: ['part:low'],
      editable: true,
      nonEditableReason: null as string | null,
    },
  ],
  handles: [],
};

async function installContextMocks(
  page: Page,
  overrides?: {
    artifactBundle?: typeof artifactBundle;
    modelManifest?: typeof modelManifest;
    design?: typeof design;
    authoringGraph?: typeof authoringGraph;
    authoringGraphDelayMs?: number;
    semanticControlError?: string;
  },
) {
  const bundle = overrides?.artifactBundle ?? artifactBundle;
  const manifest = overrides?.modelManifest ?? modelManifest;
  const mockedDesign = overrides?.design ?? design;
  const graph = overrides?.authoringGraph ?? authoringGraph;
  const graphDelayMs = overrides?.authoringGraphDelayMs ?? 0;
  const semanticControlError = overrides?.semanticControlError ?? null;
  await page.route(/\/mock\/context\/.*\.stl(?:\?.*)?$/, async (route) => {
    const body = route.request().url().includes('offscreen') ? MOCK_STL_OFFSCREEN : MOCK_STL;
    await route.fulfill({ status: 200, contentType: 'model/stl', body });
  });

  await page.addInitScript(({ config, runtimeCapabilities, artifactBundle, modelManifest, design, authoringGraph, authoringGraphDelayMs, semanticControlError }) => {
    (window as any).__CONTEXT_CALLS__ = [];
    window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
    let nextCallbackId = 1;
    window.__TAURI_INTERNALS__.transformCallback = (callback) => {
      const callbackId = nextCallbackId++;
      (window as any)[`_${callbackId}`] = callback;
      return callbackId;
    };
    window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
      (window as any).__CONTEXT_CALLS__.push({ cmd, args });
      if (cmd === 'plugin:event|listen') return Number(args?.handler ?? 0);
      if (cmd === 'plugin:event|unlisten') return null;
      if (cmd === 'get_config') return config;
      if (cmd === 'create_design_thread') {
        return {
          threadId: 'thread-context',
          sourceDocument: { folder: '/mock/context-controls', file: '/mock/context-controls/model.ecky', source: '(model)' },
          initialVersionId: null, snapshotId: null, parserMatched: null, initialVersionError: null,
          workspace: {
            thread: { id: 'thread-context', title: 'Untitled design', summary: '', updatedAt: 1, versionCount: 0, pendingCount: 0, queuedCount: 0, errorCount: 0, status: 'active', engineKind: 'ecky', sourceLanguage: 'ecky', geometryBackend: 'mesh' },
            messagesPage: { messages: [], nextBefore: null, hasMore: false }, selectedVersion: null, requestedMessageFound: false,
          },
        };
      }
      if (cmd === 'get_runtime_capabilities') return runtimeCapabilities;
      if (cmd === 'get_history') return [];
      if (cmd === 'get_last_design') return null;
      if (cmd === 'get_default_macro') return '';
      if (cmd === 'check_freecad') return true;
      if (cmd === 'start_exploration_run') {
        return {
          run: {
            requestId: args.input.requestId,
            threadId: args.input.threadId,
            cycleId: 'cycle-context',
            phase: 'completed',
            messageId: 'msg-context',
            design,
            artifactBundle,
            modelManifest,
            structuralVerification: null,
            usage: null,
            responseText: 'Context controls ready.',
            rawError: null,
            publicationAllowed: true,
          },
          message: {
            id: 'msg-context',
            role: 'assistant',
            content: 'Context controls ready.',
            status: 'success',
            output: design,
            artifactBundle,
            modelManifest,
            timestamp: 100,
          },
          snapshotId: 'snapshot-context',
        };
      }
      if (cmd === 'init_generation_attempt') return 'msg-context';
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
          design,
          threadId: 'thread-context',
          messageId: 'msg-context',
          usage: null,
        };
      }
      if (cmd === 'render_model') return artifactBundle;
      if (cmd === 'get_model_manifest') return modelManifest;
      if (cmd === 'get_authoring_graph') {
        if (authoringGraphDelayMs > 0) {
          await new Promise((resolve) => setTimeout(resolve, authoringGraphDelayMs));
        }
        return authoringGraph;
      }
      if (cmd === 'apply_semantic_control_value') {
        if (semanticControlError) {
          throw { code: 'validation', message: semanticControlError };
        }
        const primitiveId = args?.input?.primitiveId;
        return {
          parameterPatch: primitiveId === 'ast-param:dryer_param_2'
            ? { dryer_param_2: args?.input?.value }
            : {},
          changedParameterKeys: primitiveId === 'ast-param:dryer_param_2' ? ['dryer_param_2'] : [],
          appliedPrimitiveIds: primitiveId ? [primitiveId] : [],
        };
      }
      if (cmd === 'get_thread') {
        return {
          id: args.id,
          title: 'Context Controls',
          summary: '',
          messages: [],
          updatedAt: 100,
          versionCount: 1,
          pendingCount: 0,
          queuedCount: 0,
          errorCount: 0,
          status: 'active',
          engineKind: 'freecad',
          sourceLanguage: 'legacyPython',
          geometryBackend: 'freecad',
        };
      }
      if (cmd === 'get_thread_latest_version' || cmd === 'get_thread_message_version') {
        return {
          id: 'msg-context',
          role: 'assistant',
          content: 'Context Controls',
          status: 'success',
          output: design,
          artifactBundle,
          modelManifest,
          timestamp: 100,
        };
      }
      if (cmd === 'get_thread_messages_page') {
        return {
          messages: [],
          nextBefore: null,
          hasMore: false,
        };
      }
      if (cmd === 'verify_generated_model') {
        return {
          passed: true,
          summary: 'Structural checks passed.',
          issues: [],
          metrics: {
            partCount: 2,
            modelStlSizeBytes: 1024,
            totalVolume: 1000,
            totalArea: 500,
            bbox: { xMin: 0, yMin: 0, zMin: 0, xMax: 10, yMax: 10, zMax: 10 },
          },
          verifierStatus: 'ok',
          verifierSource: 'mock',
        };
      }
      if (cmd === 'verify_render') {
        return {
          passed: true,
          summary: 'Visual checks passed.',
          issues: [],
          usage: null,
        };
      }
      if (cmd === 'finalize_generation_attempt') return null;
      if (cmd === 'save_last_design') return null;
      if (cmd === 'save_config') return null;
      if (cmd === 'get_active_agent_sessions') return [];
      if (cmd === 'get_agent_activity') return { events: [], latestCursor: 0, hasMore: false };
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
  }, { config, runtimeCapabilities, artifactBundle: bundle, modelManifest: manifest, design: mockedDesign, authoringGraph: graph, authoringGraphDelayMs: graphDelayMs, semanticControlError });
}

async function selectViewerTarget(page: Page, expectedPanelText: string) {
  await expect(page.locator('.viewer-shell canvas')).toBeVisible();
  const bounds = await page.locator('.viewer-host').first().boundingBox();
  expect(bounds).not.toBeNull();
  if (!bounds) throw new Error('viewer bounds missing');

  await page.waitForTimeout(250);
  for (const yRatio of [0.45, 0.53, 0.61]) {
    for (const xRatio of [0.36, 0.5, 0.64]) {
      await page.mouse.click(bounds.x + bounds.width * xRatio, bounds.y + bounds.height * yRatio);
      const panelText = (await page.locator('.param-panel').allTextContents())[0] ?? '';
      if (panelText.includes(expectedPanelText)) return;
    }
  }
}

async function waitForContextRender(page: Page) {
  try {
    await expect.poll(() => page.evaluate(() =>
      (window as any).__CONTEXT_CALLS__.some((entry: { cmd: string }) =>
        entry.cmd === 'get_model_manifest' || entry.cmd === 'start_exploration_run'),
    ), { timeout: 15000 }).toBe(true);
  } catch {
    const commands = await page.evaluate(() =>
      (window as any).__CONTEXT_CALLS__.map((entry: { cmd: string }) => entry.cmd),
    );
    throw new Error(`Context render did not finish. Commands: ${commands.join(', ')}`);
  }
}

test('Given part selection When only model-global controls exist Then part panel does not duplicate them', async ({ page }) => {
  await installContextMocks(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  const dismissError = page.getByRole('button', { name: 'Dismiss error' });
  if (await dismissError.count()) {
    await dismissError.click();
  }

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make contextual controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');

  await page.getByRole('button', { name: 'Parameters', exact: true }).click();
  await expect(page.locator('.param-panel')).toBeVisible();

  await page.getByRole('button', { name: 'Low' }).click();
  await expect(page.locator('.param-panel').getByText('Low Width', { exact: true })).toBeVisible();

  await page.getByRole('button', { name: 'Nose' }).click();
  await expect(page.getByText('No semantic controls are mapped to this part yet. Open RAW for fallback.')).toBeVisible();
  await expect(page.getByText('Hose OD')).toHaveCount(0);
});

test('Given mapped film gate context When selected Then Params shows gap-related controls only', async ({ page }) => {
  await installContextMocks(page, {
    design: {
      ...design,
      title: 'Film Adapter Coupon',
      versionName: 'V-film-coupon',
      macroCode: '# film adapter coupon',
      uiSpec: {
        fields: [
          { type: 'number', key: 'film_gap', label: 'Film Gap' },
          { type: 'number', key: 'lens_bore_d', label: 'Lens Bore D' },
        ],
      },
      initialParams: { film_gap: 0.35, lens_bore_d: 59.6 },
    },
    artifactBundle: {
      ...artifactBundle,
      modelId: 'film-coupon-model',
      faceTargets: [
        {
          targetId: 'film_gate:face:slot',
          durableTargetId: 'film_gate:node:film-gate-node:face:slot',
          canonicalTargetId: 'film_gate:face:slot:canonical',
          aliasIds: ['film_gate:face:slot:alias'],
          partId: 'film_gate',
          viewerNodeId: 'film-gate-node',
          label: 'Film Gate Slot Face',
          editable: true,
          center: { x: 0.4, y: 0.4, z: 0.1 },
          normal: [0, 0, 1],
          area: 18,
        },
      ],
    },
    modelManifest: {
      ...modelManifest,
      modelId: 'film-coupon-model',
      parts: [
        {
          partId: 'film_gate',
          freecadObjectName: 'FilmGate',
          label: 'Film Gate',
          kind: 'Part::Feature',
          semanticRole: 'gate',
          viewerAssetPath: '/mock/context/film-gate.stl',
          viewerNodeIds: ['film-gate-node'],
          parameterKeys: ['film_gap'],
          editable: true,
          bounds: null,
          volume: null,
          area: null,
        },
        {
          partId: 'lens_adapter',
          freecadObjectName: 'LensAdapter',
          label: 'Lens Adapter',
          kind: 'Part::Feature',
          semanticRole: 'lens',
          viewerAssetPath: '/mock/context/lens-adapter.stl',
          viewerNodeIds: ['lens-adapter-node'],
          parameterKeys: ['lens_bore_d'],
          editable: true,
          bounds: null,
          volume: null,
          area: null,
        },
      ],
      selectionTargets: [
        {
          targetId: 'film_gate:face:slot',
          durableTargetId: 'film_gate:node:film-gate-node:face:slot',
          canonicalTargetId: 'film_gate:face:slot:canonical',
          aliasIds: ['film_gate:face:slot:alias'],
          partId: 'film_gate',
          viewerNodeId: 'film-gate-node',
          label: 'Film Gate Slot Face',
          kind: 'face',
          editable: true,
          parameterKeys: ['film_gap'],
          primitiveIds: [],
          viewIds: [],
        },
      ],
    },
  });
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  const dismissError = page.getByRole('button', { name: 'Dismiss error' });
  if (await dismissError.count()) {
    await dismissError.click();
  }

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'load film coupon controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');

  await page.getByRole('button', { name: /(PARAMS|Parameters)/i }).click();
  await expect(page.locator('.param-panel')).toBeVisible();
  await page.getByRole('button', { name: 'Film Gate' }).click();

  await expect(page.locator('.param-panel')).toContainText('Film Gap');
  await expect(page.locator('.param-panel')).not.toContainText('Lens Bore D');
  await expect(page.locator('.param-panel .param-list .param-field')).toHaveCount(1);
});

test('Given Params select mode on mapped film gate target When target selected Then panel shows frame and gap controls without helicoid controls', async ({
  page,
}) => {
  await installContextMocks(page, {
    design: {
      ...design,
      title: 'Film Gate Isolation',
      versionName: 'V-film-gate-isolation',
      macroCode: '# film gate isolation',
      uiSpec: {
        fields: [
          { type: 'number', key: 'film_gap', label: 'Film Gap' },
          { type: 'number', key: 'film_frame_width', label: 'Frame Width' },
          { type: 'number', key: 'helicoid_pitch', label: 'Helicoid Pitch' },
          { type: 'number', key: 'helicoid_clearance', label: 'Helicoid Clearance' },
        ],
      },
      initialParams: { film_gap: 0.4, film_frame_width: 13.8, helicoid_pitch: 1.2, helicoid_clearance: 0.25 },
    },
    artifactBundle: {
      ...artifactBundle,
      modelId: 'film-gate-isolation-model',
      faceTargets: [
        {
          targetId: 'film_gate:face:slot',
          durableTargetId: 'film_gate:node:film-gate-node:face:slot',
          canonicalTargetId: 'film_gate:face:slot:canonical',
          aliasIds: ['film_gate:face:slot:alias'],
          partId: 'film_gate',
          viewerNodeId: 'film-gate-node',
          label: 'Film Gate Slot Face',
          editable: true,
          center: { x: 0.42, y: 0.42, z: 0.12 },
          normal: [0, 0, 1],
          area: 21,
        },
      ],
    },
    modelManifest: {
      ...modelManifest,
      modelId: 'film-gate-isolation-model',
      parts: [
        {
          partId: 'film_gate',
          freecadObjectName: 'FilmGate',
          label: 'Film Gate',
          kind: 'Part::Feature',
          semanticRole: 'gate',
          viewerAssetPath: '/mock/context/film-gate.stl',
          viewerNodeIds: ['film-gate-node'],
          parameterKeys: ['film_gap', 'film_frame_width'],
          editable: true,
          bounds: null,
          volume: null,
          area: null,
        },
        {
          partId: 'helicoid_adapter',
          freecadObjectName: 'HelicoidAdapter',
          label: 'Helicoid Adapter',
          kind: 'Part::Feature',
          semanticRole: 'thread',
          viewerAssetPath: '/mock/context/offscreen-helicoid-adapter.stl',
          viewerNodeIds: ['helicoid-adapter-node'],
          parameterKeys: ['helicoid_pitch', 'helicoid_clearance'],
          editable: true,
          bounds: null,
          volume: null,
          area: null,
        },
      ],
      selectionTargets: [
        {
          targetId: 'film_gate:face:slot',
          durableTargetId: 'film_gate:node:film-gate-node:face:slot',
          canonicalTargetId: 'film_gate:face:slot:canonical',
          aliasIds: ['film_gate:face:slot:alias'],
          partId: 'film_gate',
          viewerNodeId: 'film-gate-node',
          label: 'Film Gate Slot Face',
          kind: 'face',
          editable: true,
          parameterKeys: ['film_gap', 'film_frame_width'],
          primitiveIds: [],
          viewIds: [],
        },
      ],
    },
  });

  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  const dismissError = page.getByRole('button', { name: 'Dismiss error' });
  if (await dismissError.count()) {
    await dismissError.click();
  }

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'load film gate isolation controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');

  await page.getByRole('button', { name: /(PARAMS|Parameters)/i }).click();
  await page.getByRole('button', { name: 'SELECT' }).click();
  await selectViewerTarget(page, 'Film Gap');

  await expect(page.locator('.param-panel')).toContainText('Film Gap');
  await expect(page.locator('.param-panel')).toContainText('Frame Width');
  await expect(page.locator('.param-panel')).not.toContainText('Helicoid Pitch');
  await expect(page.locator('.param-panel')).not.toContainText('Helicoid Clearance');
  await expect(page.locator('.param-panel .param-list .param-field')).toHaveCount(2);
  await expect(page.locator('.part-chip.part-chip-active')).toContainText('film gate');
});

test('Given Params select mode When mapped lens-bore target selected Then panel shows exactly one relevant lens-bore control and excludes unrelated controls', async ({
  page,
}) => {
  await installContextMocks(page, {
    design: {
      ...design,
      title: 'Lens Bore Isolation',
      versionName: 'V-lens-bore-isolation',
      macroCode: '# lens bore isolation',
      uiSpec: {
        fields: [
          { type: 'number', key: 'film_gap', label: 'Film Gap' },
          { type: 'number', key: 'film_frame_width', label: 'Frame Width' },
          { type: 'number', key: 'lens_bore_d', label: 'Lens Bore D' },
          { type: 'number', key: 'helicoid_pitch', label: 'Helicoid Pitch' },
        ],
      },
      initialParams: { film_gap: 0.4, film_frame_width: 13.8, lens_bore_d: 59.6, helicoid_pitch: 1.2 },
    },
    artifactBundle: {
      ...artifactBundle,
      modelId: 'lens-bore-isolation-model',
      faceTargets: [
        {
          targetId: 'lens_adapter:face:bore',
          durableTargetId: 'lens_adapter:node:lens-adapter-node:face:bore',
          canonicalTargetId: 'lens_adapter:face:bore:canonical',
          aliasIds: ['lens_adapter:face:bore:alias'],
          partId: 'lens_adapter',
          viewerNodeId: 'lens-adapter-node',
          label: 'Lens Bore Face',
          editable: true,
          center: { x: 0.52, y: 0.52, z: 0.12 },
          normal: [0, 0, 1],
          area: 20,
        },
      ],
    },
    modelManifest: {
      ...modelManifest,
      modelId: 'lens-bore-isolation-model',
      parts: [
        {
          partId: 'film_gate',
          freecadObjectName: 'FilmGate',
          label: 'Film Gate',
          kind: 'Part::Feature',
          semanticRole: 'gate',
          viewerAssetPath: '/mock/context/offscreen-film-gate.stl',
          viewerNodeIds: ['film-gate-node'],
          parameterKeys: ['film_gap', 'film_frame_width'],
          editable: true,
          bounds: null,
          volume: null,
          area: null,
        },
        {
          partId: 'lens_adapter',
          freecadObjectName: 'LensAdapter',
          label: 'Lens Adapter',
          kind: 'Part::Feature',
          semanticRole: 'lens',
          viewerAssetPath: '/mock/context/lens-adapter.stl',
          viewerNodeIds: ['lens-adapter-node'],
          parameterKeys: ['lens_bore_d'],
          editable: true,
          bounds: null,
          volume: null,
          area: null,
        },
        {
          partId: 'helicoid_adapter',
          freecadObjectName: 'HelicoidAdapter',
          label: 'Helicoid Adapter',
          kind: 'Part::Feature',
          semanticRole: 'thread',
          viewerAssetPath: '/mock/context/offscreen-helicoid-adapter.stl',
          viewerNodeIds: ['helicoid-adapter-node'],
          parameterKeys: ['helicoid_pitch'],
          editable: true,
          bounds: null,
          volume: null,
          area: null,
        },
      ],
      selectionTargets: [
        {
          targetId: 'lens_adapter:face:bore',
          durableTargetId: 'lens_adapter:node:lens-adapter-node:face:bore',
          canonicalTargetId: 'lens_adapter:face:bore:canonical',
          aliasIds: ['lens_adapter:face:bore:alias'],
          partId: 'lens_adapter',
          viewerNodeId: 'lens-adapter-node',
          label: 'Lens Bore Face',
          kind: 'face',
          editable: true,
          parameterKeys: ['lens_bore_d'],
          primitiveIds: [],
          viewIds: [],
        },
      ],
    },
  });

  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  const dismissError = page.getByRole('button', { name: 'Dismiss error' });
  if (await dismissError.count()) {
    await dismissError.click();
  }

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'load lens bore isolation controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');

  await page.getByRole('button', { name: 'PARAMS' }).click();
  await page.getByRole('button', { name: 'SELECT' }).click();
  await selectViewerTarget(page, 'Lens Bore D');

  await expect(page.locator('.param-panel')).toContainText('Lens Bore D');
  await expect(page.locator('.param-panel')).not.toContainText('Film Gap');
  await expect(page.locator('.param-panel')).not.toContainText('Frame Width');
  await expect(page.locator('.param-panel')).not.toContainText('Helicoid Pitch');
  await expect(page.locator('.param-panel .param-list .param-field')).toHaveCount(1);
  await expect(page.locator('.part-chip.part-chip-active')).toContainText('lens adapter');
});

test('Given workbench idle When Params opens Then prewarmed panel is reused', async ({ page }) => {
  await installContextMocks(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);

  await page.waitForFunction(() => Boolean(document.querySelector('[data-window-id="params"] .param-panel')));
  await expect(page.locator('[data-window-id="params"].window--hidden .param-panel')).toHaveCount(1);

  await page.getByRole('button', { name: /(PARAMS|Parameters)/i }).click();
  await expect(page.locator('[data-window-id="params"] .param-panel')).toBeVisible();

  await page.locator('[data-window-id="params"] .window-close').click();
  await expect(page.locator('[data-window-id="params"].window--hidden .param-panel')).toHaveCount(1);

  await page.getByRole('button', { name: /(PARAMS|Parameters)/i }).click();
  await expect(page.locator('[data-window-id="params"] .param-panel')).toBeVisible();
});

test('Given a large Ecky manifest When Params opens Then deterministic ownership sections replace the flat dump', async ({ page }) => {
  const fixture = dryerContextFixture();
  await installContextMocks(page, fixture as any);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make grouped dryer controls');
  await page.locator('textarea.prompt-input').press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');
  await waitForContextRender(page);
  await page.getByRole('button', { name: /(PARAMS|Parameters)/i }).click();

  const sections = page.getByTestId('parameter-ownership-section');
  await expect(sections).toHaveCount(4);
  await expect(sections.nth(0)).toContainText(/Model Params/i);
  await expect(sections.nth(1)).toContainText(/Drum/i);
  await expect(sections.nth(1)).toContainText('11 PARAMS');
  await expect(sections.nth(1)).toHaveAttribute('data-collapsed', 'true');
  await expect(page.locator('.param-panel > .param-list')).toHaveCount(0);
});

test('Given exact generated Ecky provenance When target is selected Then viewport shows only linked control', async ({ page }) => {
  const fixture = dryerContextFixture(['dryer_param_2']);
  await installContextMocks(page, fixture as any);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make selectable dryer controls');
  await page.locator('textarea.prompt-input').press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');
  await waitForContextRender(page);
  await page.getByRole('button', { name: /(PARAMS|Parameters)/i }).click();
  await page.getByRole('button', { name: 'MESH', exact: true }).click();
  await page.getByRole('button', { name: 'SELECT', exact: true }).click();
  await selectViewerTarget(page, 'Dryer Param 2');

  const overlay = page.locator('.viewer-part-overlay');
  await expect(overlay).toBeVisible();
  await expect(overlay.locator('.viewer-part-overlay__controls .viewer-overlay-control')).toHaveCount(1);
  await expect(overlay).toContainText('Dryer Param 2');
  await expect(overlay).not.toContainText('Dryer Param 3');

  await overlay.locator('.viewer-overlay-input[type="number"]').fill('77');
  await expect.poll(() => page.evaluate(() =>
    (window as any).__CONTEXT_CALLS__.filter(
      (entry: { cmd: string }) => entry.cmd === 'apply_semantic_control_value',
    ),
  )).toEqual([{
    cmd: 'apply_semantic_control_value',
    args: {
      input: {
        threadId: 'thread-context',
        targetMessageId: 'msg-context',
        primitiveId: 'ast-param:dryer_param_2',
        value: 77,
      },
    },
  }]);
  await page.getByRole('button', { name: 'PARAMETERS', exact: true }).click();
  const selectedSection = page.getByTestId('parameter-ownership-section').first();
  await expect(selectedSection).toHaveAttribute('data-selected', 'true');
  await expect(selectedSection).toHaveAttribute('data-collapsed', 'false');
  await expect(selectedSection.locator('.param-field')).toHaveCount(1);
  await expect(selectedSection.locator('input[type="number"]')).toHaveValue('77');
  await expect(page.getByRole('button', { name: 'APPLY', exact: true })).toBeEnabled();
});

test('Given semantic value rejection When overlay changes Then raw backend error is shown and patch is not staged', async ({ page }) => {
  const fixture = dryerContextFixture(['dryer_param_2']);
  await installContextMocks(page, {
    ...fixture,
    semanticControlError: 'semantic primitive stale: raw backend body',
  } as any);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make selectable dryer controls');
  await page.locator('textarea.prompt-input').press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');
  await waitForContextRender(page);
  await page.getByRole('button', { name: /(PARAMS|Parameters)/i }).click();
  await page.getByRole('button', { name: 'MESH', exact: true }).click();
  await page.getByRole('button', { name: 'SELECT', exact: true }).click();
  await selectViewerTarget(page, 'Dryer Param 2');

  const overlay = page.locator('.viewer-part-overlay');
  await overlay.locator('.viewer-overlay-input[type="number"]').fill('77');
  await expect(page.getByText(/semantic primitive stale: raw backend body/).first()).toBeVisible();
  await expect.poll(() => page.evaluate(() =>
    (window as any).__CONTEXT_CALLS__.filter(
      (entry: { cmd: string }) => entry.cmd === 'apply_semantic_control_value',
    ).length,
  )).toBe(1);

  await page.getByRole('button', { name: 'PARAMETERS', exact: true }).click();
  const selectedSection = page.getByTestId('parameter-ownership-section').first();
  await expect(selectedSection.locator('input[type="number"]')).toHaveValue('3');
});

test('Given ambiguous generated Ecky provenance When target is selected Then viewport keeps editable overlay absent', async ({ page }) => {
  const fixture = dryerContextFixture([]);
  await installContextMocks(page, fixture as any);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make ambiguous dryer controls');
  await page.locator('textarea.prompt-input').press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');
  await waitForContextRender(page);
  await page.getByRole('button', { name: /(PARAMS|Parameters)/i }).click();
  await page.getByRole('button', { name: 'MESH', exact: true }).click();
  await page.getByRole('button', { name: 'SELECT', exact: true }).click();
  await selectViewerTarget(page, '__ambiguous_target_has_no_panel_control__');

  await expect(page.locator('.viewer-part-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'PARAMETERS', exact: true }).click();
  await expect(page.locator('[data-testid="parameter-ownership-section"][data-selected="true"]')).toHaveCount(0);
  await expect(page.getByTestId('parameter-ownership-section').first()).toHaveAttribute('data-collapsed', 'true');
});

test('Given Params select mode When viewer face is clicked Then Params focuses exact face control', async ({ page }) => {
  await installContextMocks(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  const dismissError = page.getByRole('button', { name: 'Dismiss error' });
  if (await dismissError.count()) {
    await dismissError.click();
  }

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make contextual controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');

  await page.getByRole('button', { name: 'PARAMS' }).click();
  await page.getByRole('button', { name: 'SELECT' }).click();
  await selectViewerTarget(page, 'Low Width');

  await expect(page.locator('.viewer-part-overlay')).toHaveCount(0);
  await expect(page.locator('.param-panel')).toContainText('Low Width');
  await expect(page.locator('.param-panel')).not.toContainText('Hose OD');
  await expect(page.locator('.param-panel .param-list .param-field')).toHaveCount(1);
  await expect(page.locator('.part-chip.part-chip-active')).toContainText('low');
});

test('Given exact backend provenance When geometry is selected Then source owner, upstream parameter, and output trace focus together', async ({ page }) => {
  await installContextMocks(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make source-aware contextual controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');
  await page.getByRole('button', { name: 'Parameters', exact: true }).click();
  await page.getByRole('button', { name: 'SELECT' }).click();
  await selectViewerTarget(page, 'Low Width');

  const trace = page.getByTestId('viewer-dependency-trace');
  await expect(trace).toContainText('Low Top Face');
  await expect(trace).toContainText('low_width');
  await expect(trace).toContainText('Low box');
  await expect(page.locator('.macro-ast-map-shell')).toBeVisible();
  await expect(page.locator('.macro-ast-node[data-node-id="part:low"]')).toHaveAttribute(
    'data-authoring-selected',
    'true',
  );
  await expect(
    page.locator('.macro-ast-node[data-node-id="part:low/param:low_width"]'),
  ).toHaveAttribute('data-authoring-upstream', 'true');
});

test('Given source graph When AST owner is selected Then mapped model target highlights without rendering whole graph in viewport', async ({ page }) => {
  await installContextMocks(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make source-aware contextual controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');
  await page.getByRole('button', { name: 'Parameters', exact: true }).click();
  await page.getByRole('button', { name: 'new params', exact: true }).click();

  await page.locator('.macro-ast-node[data-node-id="part:low"] .macro-ast-node__header').click();

  await expect(page.locator('.viewer-host')).toHaveAttribute(
    'data-authoring-highlight-targets',
    'low:face:top',
  );
  await expect(page.getByTestId('viewer-dependency-trace')).toContainText('Low Top Face');
  await expect(page.getByTestId('authoring-graph-source')).toContainText('low_width');
  await expect(page.getByTestId('authoring-graph-source')).toContainText('Low Top Face');
  await expect(page.locator('.viewer-host [data-authoring-graph-node]')).toHaveCount(0);
});

test('Given missing backend provenance When geometry is selected Then target stays inspectable and raw reason replaces edit focus', async ({ page }) => {
  await installContextMocks(page, {
    authoringGraph: {
      ...authoringGraph,
      targets: [
        {
          ...authoringGraph.targets[0],
          featureIds: [],
          sourceStableNodeKeys: [],
          editable: false,
          nonEditableReason: 'Kernel face has no exact source provenance after boolean refinement.',
        },
      ],
    },
  });
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make source-aware contextual controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');
  await page.getByRole('button', { name: 'Parameters', exact: true }).click();
  await page.getByRole('button', { name: 'SELECT' }).click();
  await selectViewerTarget(page, 'Low Width');

  const trace = page.getByTestId('viewer-dependency-trace');
  await expect(trace).toHaveAttribute('data-resolution', 'missing');
  await expect(trace).toContainText(
    'Kernel face has no exact source provenance after boolean refinement.',
  );
  await expect(page.locator('.macro-ast-node[data-authoring-selected="true"]')).toHaveCount(0);
});

test('Given authoring graph is pending When geometry is selected Then compact trace reports loading', async ({ page }) => {
  await installContextMocks(page, { authoringGraphDelayMs: 1500 });
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make source-aware contextual controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');
  await page.getByRole('button', { name: 'Parameters', exact: true }).click();
  await page.getByRole('button', { name: 'SELECT' }).click();
  await selectViewerTarget(page, 'Low Width');

  await expect(page.getByTestId('viewer-dependency-trace')).toHaveAttribute(
    'data-resolution',
    'pending',
  );
  await expect(page.getByTestId('viewer-dependency-trace')).toContainText('LOADING SOURCE GRAPH');
});

test('Given Params select mode When viewer click hits unmapped part Then Params shows empty semantic state', async ({ page }) => {
  await installContextMocks(page, {
    artifactBundle: {
      ...artifactBundle,
      faceTargets: [],
    },
    modelManifest: {
      ...modelManifest,
      selectionTargets: [],
    },
  });
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  const dismissError = page.getByRole('button', { name: 'Dismiss error' });
  if (await dismissError.count()) {
    await dismissError.click();
  }

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make contextual controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');

  await page.getByRole('button', { name: 'PARAMS' }).click();
  await page.getByRole('button', { name: 'SELECT' }).click();
  await page.getByRole('button', { name: 'Nose' }).click();
  await expect(page.locator('.param-panel')).toContainText(
    'No semantic controls are mapped to this part yet. Open RAW for fallback.',
  );
  await expect(page.locator('.viewer-shell canvas')).toBeVisible();
  const viewer = page.locator('.viewer-host').first();
  const bounds = await viewer.boundingBox();
  expect(bounds).not.toBeNull();
  if (!bounds) throw new Error('viewer bounds missing');

  await page.mouse.click(bounds.x + bounds.width * 0.94, bounds.y + bounds.height * 0.1);
  await expect(page.locator('.param-panel')).toContainText(
    'No semantic controls are mapped to this part yet. Open RAW for fallback.',
  );
  await expect(page.locator('.part-chip.part-chip-active')).toContainText('nose');
});

test('Given select mode with ambiguous face targets When no face selected Then Params shows pending target message and no fallback controls', async ({
  page,
}) => {
  await installContextMocks(page, {
    artifactBundle: {
      ...artifactBundle,
      faceTargets: [
        {
          targetId: 'low:face:threadA',
          durableTargetId: 'low:node:low-node:face:threadA',
          canonicalTargetId: 'low:face:threadA:canonical',
          aliasIds: ['low:face:threadA:alias'],
          partId: 'low',
          viewerNodeId: 'low-node',
          label: 'Low Thread Face A',
          editable: true,
          center: { x: 0.28, y: 0.35, z: 0.06 },
          normal: [0, 0, 1],
          area: 12,
        },
        {
          targetId: 'low:face:threadB',
          durableTargetId: 'low:node:low-node:face:threadB',
          canonicalTargetId: 'low:face:threadB:canonical',
          aliasIds: ['low:face:threadB:alias'],
          partId: 'low',
          viewerNodeId: 'low-node',
          label: 'Low Thread Face B',
          editable: true,
          center: { x: 0.62, y: 0.38, z: 0.08 },
          normal: [0, 0, 1],
          area: 10,
        },
      ],
    },
    modelManifest: {
      ...modelManifest,
      selectionTargets: [
        {
          targetId: 'low:face:threadA',
          durableTargetId: 'low:node:low-node:face:threadA',
          canonicalTargetId: 'low:face:threadA:canonical',
          aliasIds: ['low:face:threadA:alias'],
          partId: 'low',
          viewerNodeId: 'low-node',
          label: 'Low Thread Face A',
          kind: 'face',
          editable: true,
          parameterKeys: ['low_width'],
          primitiveIds: [],
          viewIds: [],
        },
        {
          targetId: 'low:face:threadB',
          durableTargetId: 'low:node:low-node:face:threadB',
          canonicalTargetId: 'low:face:threadB:canonical',
          aliasIds: ['low:face:threadB:alias'],
          partId: 'low',
          viewerNodeId: 'low-node',
          label: 'Low Thread Face B',
          kind: 'face',
          editable: true,
          parameterKeys: ['low_width'],
          primitiveIds: [],
          viewIds: [],
        },
      ],
    },
  });
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  const dismissError = page.getByRole('button', { name: 'Dismiss error' });
  if (await dismissError.count()) {
    await dismissError.click();
  }

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'show thread face controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');

  await page.getByRole('button', { name: 'PARAMS' }).click();
  await page.getByRole('button', { name: 'SELECT' }).click();
  await expect(page.locator('.param-panel')).toContainText(
    'Multiple face targets found. Select one in viewport; fallback waits for explicit target.',
  );
  await expect(page.locator('.param-panel')).not.toContainText('Low Width');
  await expect(page.locator('.viewer-part-overlay')).toHaveCount(0);
});

test('Given Params measure mode When viewer is clicked and dragged Then Params focus and part selection stay unchanged', async ({ page }) => {
  await installContextMocks(page);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  const dismissError = page.getByRole('button', { name: 'Dismiss error' });
  if (await dismissError.count()) {
    await dismissError.click();
  }

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make contextual controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');

  await page.getByRole('button', { name: 'PARAMS' }).click();
  await page.getByRole('button', { name: 'MEASURE' }).click();
  await expect(page.locator('.viewer-shell canvas')).toBeVisible();
  const viewer = page.locator('.viewer-host').first();
  const bounds = await viewer.boundingBox();
  expect(bounds).not.toBeNull();
  if (!bounds) throw new Error('viewer bounds missing');

  const centerX = bounds.x + bounds.width * 0.5;
  const centerY = bounds.y + bounds.height * 0.5;
  await page.mouse.click(centerX, centerY);
  await page.mouse.move(centerX, centerY);
  await page.mouse.down();
  await page.mouse.move(centerX + 100, centerY + 35, { steps: 12 });
  await page.mouse.up();

  await expect(page.locator('.viewer-part-overlay')).toHaveCount(0);
  await expect(page.locator('.part-chip-active')).toHaveCount(0);
  // Measure mode "keeps parameter focus unchanged" (MEASURE tooltip): entering it
  // from the default global view leaves that view rendered (count 2), and a
  // viewport drag must not select a part or filter the controls. f16917e briefly
  // inverted this to count 0 / lowercase, but that was a test-only edit with no
  // backing implementation or spec and contradicts every sibling assertion.
  await expect(page.locator('.param-panel .param-list .param-field')).toHaveCount(2);
  await expect(page.locator('.param-panel')).toContainText('Hose OD');
  await expect(page.locator('.param-panel')).toContainText('Low Width');
});

test('Given default orbit mode and no selection When user drags viewer Then it does not select a part or open viewport controls', async ({ page }) => {
  await installContextMocks(page, {
    artifactBundle: {
      ...artifactBundle,
      faceTargets: [],
    },
    modelManifest: {
      ...modelManifest,
      selectionTargets: [],
    },
  });
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  const dismissError = page.getByRole('button', { name: 'Dismiss error' });
  if (await dismissError.count()) {
    await dismissError.click();
  }

  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await page.fill('textarea.prompt-input', 'make contextual controls');
  await page
    .locator('textarea.prompt-input')
    .press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter');

  await page.getByRole('button', { name: 'PARAMS' }).click();
  await expect(page.getByRole('button', { name: 'ORBIT' })).toHaveClass(/panel-mode-tab-active/);
  await expect(page.locator('.part-chip-active')).toHaveCount(0);
  await expect(page.locator('.viewer-part-overlay')).toHaveCount(0);
  await expect(page.locator('.viewer-shell canvas')).toBeVisible();
  const viewer = page.locator('.viewer-host').first();
  const bounds = await viewer.boundingBox();
  expect(bounds).not.toBeNull();
  if (!bounds) throw new Error('viewer bounds missing');

  const startX = bounds.x + bounds.width * 0.5;
  const startY = bounds.y + bounds.height * 0.5;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX + 90, startY + 30, { steps: 12 });
  await page.mouse.up();

  await expect(page.locator('.viewer-part-overlay')).toHaveCount(0);
  await expect(page.locator('.part-chip-active')).toHaveCount(0);
});
