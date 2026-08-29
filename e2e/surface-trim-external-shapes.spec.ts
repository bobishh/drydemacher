import { expect, test, type Page } from '@playwright/test';

type MockConfig = {
  loopError?: string;
  regionError?: string;
  applyError?: string;
  capError?: string;
  applyDelayMs?: number;
};

type InvokeCall = {
  cmd: string;
  args?: Record<string, unknown>;
};

type CaptureSurfaceAnchor = {
  sourceMeshContentDigest: string;
  triangleIndex: number;
  barycentric: [number, number, number];
  sourcePosition: [number, number, number];
  sourceNormal: [number, number, number];
};

const STORAGE_KEY = '__surfaceTrimExternalShapeSources__';
const THREAD_ID = 'capture-thread';
const SOURCE_DIGEST = 'sha256:source-1';
const MESH_DIGEST = 'sha256:mesh-1';

function makeAnchor(
  triangleIndex: number,
  sourcePosition: [number, number, number],
  barycentric: [number, number, number],
): CaptureSurfaceAnchor {
  return {
    sourceMeshContentDigest: MESH_DIGEST,
    triangleIndex,
    barycentric,
    sourcePosition,
    sourceNormal: [0, 0, 1],
  };
}

function buildPathResponse(
  fromAnchor: CaptureSurfaceAnchor,
  toAnchor: CaptureSurfaceAnchor,
  pathMode: string,
  previewId: number,
) {
  return {
    previewId,
    path: {
      sourceMeshContentDigest: MESH_DIGEST,
      sourceMeshTriangles: 4,
      pathMode,
      startTriangleIndex: fromAnchor.triangleIndex,
      endTriangleIndex: toAnchor.triangleIndex,
      totalCost: 1,
      triangleCorridor: [fromAnchor.triangleIndex, toAnchor.triangleIndex],
      edgeSegments: [],
      continuousPolyline: [
        { sourcePosition: fromAnchor.sourcePosition, sharedEdge: null },
        { sourcePosition: toAnchor.sourcePosition, sharedEdge: null },
      ],
      diagnostics: {
        sourceMeshContentDigest: MESH_DIGEST,
        pathMode,
        schemaVersion: 1,
        triangles: 4,
        connectedComponents: 1,
        boundaryEdges: 0,
        nonManifoldEdges: 0,
        anchorStartTriangle: fromAnchor.triangleIndex,
        anchorEndTriangle: toAnchor.triangleIndex,
      },
    },
  };
}

function buildBinaryTetrahedronStl() {
  const header = new Uint8Array(80);
  const triangleCount = new Uint32Array([4]);
  const triangles = new Float32Array([
    0, 0, 1, 0, 0, 0, 120, 0, 0, 0, 120, 0, 0, 0, 120, 0, 0, 0,
    0.57735026, 0.57735026, 0.57735026, 0, 0, 0, 120, 0, 0, 0, 0, 120, 0, 120, 0, 0,
    0.57735026, 0.57735026, 0.57735026, 0, 0, 0, 120, 0, 0, 0, 0, 120, 0, 0, 0, 120,
    -1, 0, 0, 120, 0, 0, 0, 120, 0, 0, 0, 0, 0, 0, 120, 0, 0, 0,
  ]);
  const attributes = new Uint16Array([0, 0, 0, 0]);

  const bytes = new Uint8Array(84 + 50 * 4);
  bytes.set(header, 0);
  bytes.set(new Uint8Array(triangleCount.buffer), 80);

  let offset = 84;
  for (let triangle = 0; triangle < 4; triangle += 1) {
    const start = triangle * 12;
    const normal = triangles.slice(start, start + 3);
    const v1 = triangles.slice(start + 3, start + 6);
    const v2 = triangles.slice(start + 6, start + 9);
    const v3 = triangles.slice(start + 9, start + 12);
    const data = new DataView(bytes.buffer, offset, 50);
    let cursor = 0;
    for (const value of [...normal, ...v1, ...v2, ...v3]) {
      data.setFloat32(cursor, value, true);
      cursor += 4;
    }
    data.setUint16(cursor, attributes[triangle], true);
    offset += 50;
  }

  return Buffer.from(bytes.buffer);
}

async function routeSurfaceTrimStl(page: Page) {
  await page.route('**/*.stl*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'model/stl',
      body: buildBinaryTetrahedronStl(),
    });
  });
}

async function installSurfaceTrimMocks(page: Page, config: MockConfig = {}) {
  await page.addInitScript(({ config, storageKey, threadId, sourceDigest, meshDigest }) => {
    const THREAD_ID = threadId;
    const SOURCE_DIGEST = sourceDigest;
    const MESH_DIGEST = meshDigest;
    const mockWindow = window as typeof window & {
      __CAPTURE_CALLS__?: InvokeCall[];
      __TAURI_INTERNALS__?: { invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> };
    };

    const defaultSources = [
      {
        nodeId: 11,
        partKey: 'head',
        path: '/tmp/donor.stl',
        displayName: 'donor.stl',
        sourceDigest: 'sha256:source-1',
        contentDigest: 'sha256:mesh-1',
        byteLength: 1200,
        exists: true,
        planeCrops: [],
        surfaceTrims: [],
      },
    ];

    const readPersistedSources = () => {
      const persisted = window.localStorage.getItem(storageKey);
      if (!persisted) {
        return null;
      }

      try {
        return JSON.parse(persisted) as Array<Record<string, unknown>>;
      } catch {
        return null;
      }
    };

    const mapLoopSegments = (loopAnchors: Array<Record<string, unknown>>) => loopAnchors.slice(0, 3).map((fromAnchor, segmentIndex) => {
      const toAnchor = loopAnchors[(segmentIndex + 1) % 3];
      return {
        segmentIndex,
        fromTriangleIndex: fromAnchor.triangleIndex,
        toTriangleIndex: toAnchor.triangleIndex,
        trianglePath: [fromAnchor.triangleIndex, toAnchor.triangleIndex],
        edgeSegments: [],
        continuousPolyline: [
          { sourcePosition: fromAnchor.sourcePosition, sharedEdge: null },
          { sourcePosition: toAnchor.sourcePosition, sharedEdge: null },
        ],
      };
    });

    let externalShapeSources = readPersistedSources() ?? defaultSources;
    let canonicalSource = externalShapeSources.some((source) => ((source.surfaceTrims as unknown[]) ?? []).length > 0)
      ? '(model (part head (surface-trim (import-stl "/tmp/donor.stl") :schema-version 1 :source-digest "sha256:mesh-1" :loop ((mesh-anchor 0 1 0 0) (mesh-anchor 1 1 0 0) (mesh-anchor 2 1 0 0)) :keep-seed (mesh-anchor 3 1 0 0) :path-mode "shortest" :cap "flat")))'
      : '(model (part head (import-stl "/tmp/donor.stl")))';
    let pendingExternalShapeSources: Array<Record<string, unknown>> | null = null;
    let pendingCanonicalSource: string | null = null;
    let previewIdCounter = 0;

    mockWindow.__CAPTURE_CALLS__ = [];
    mockWindow.__TAURI_INTERNALS__ = mockWindow.__TAURI_INTERNALS__ || {};
    mockWindow.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: Record<string, unknown>) => {
      mockWindow.__CAPTURE_CALLS__?.push({ cmd, args });

      if (cmd === 'get_config') {
        return {
          engines: [],
          selectedEngineId: '',
          freecadCmd: '',
          assets: [],
          microwave: { humId: null, dingId: null, muted: true },
          voice: { sttLanguageCode: 'en-US' },
          mcp: { port: null, maxSessions: null, mode: 'active', primaryAgentId: null, promptTimeoutSecs: 1800, eckyAstAuthoring: false, autoAgents: [] },
          hasSeenOnboarding: true,
          connectionType: 'mcp',
          defaultEngineKind: 'ecky',
          defaultSourceLanguage: 'ecky',
          defaultGeometryBackend: 'mesh',
          maxGenerationAttempts: 1,
          maxVerifyAttempts: 0,
        };
      }

      if (cmd === 'save_config') return null;
      if (cmd === 'get_runtime_capabilities') {
        return {
          freecad: { available: true, detail: 'Ready', path: '/mock/freecadcmd' },
          build123d: { available: true, detail: 'Ready', path: '/mock/python3' },
          mesh: { available: true, detail: 'bundled', path: null },
          recommendedAuthoringContext: { engineKind: 'ecky', sourceLanguage: 'ecky', geometryBackend: 'mesh' },
        };
      }
      if (cmd === 'get_history') {
        return [{
          id: THREAD_ID,
          title: 'Finger fixture',
          summary: '',
          updatedAt: 50,
          messages: [],
          genieTraits: null,
          versionCount: 0,
          pendingCount: 0,
          queuedCount: 0,
          errorCount: 0,
          status: 'active',
          finalizedAt: null,
          pendingConfirm: null,
        }];
      }
      if (cmd === 'get_last_design') return null;
      if (cmd === 'get_project_source') {
        return {
          threadId: THREAD_ID,
          slug: 'finger-fixture',
          folder: '/tmp/finger-fixture',
          file: '/tmp/finger-fixture/model.ecky',
          source: canonicalSource,
        };
      }
      if (cmd === 'list_external_shape_sources') return structuredClone(externalShapeSources);
      if (cmd === 'get_default_macro') return '(solid blank)';
      if (cmd === 'get_active_agent_sessions') return [];
      if (cmd === 'get_agent_terminal_snapshots') return [];
      if (cmd === 'get_app_logs') return [];
      if (cmd === 'get_thread_agent_state') {
        return {
          connectionState: 'disconnected',
          sessions: [],
          primaryAgentLabel: null,
          statusText: '',
          phase: null,
          busy: false,
          agentLabel: null,
          activityLabel: '',
          sessionId: null,
        };
      }

      if (cmd === 'preview_external_shape_surface_trim_path') {
        const request = (args?.request ?? {}) as Record<string, unknown>;
        previewIdCounter += 1;
        return {
          previewId: request.previewId ?? previewIdCounter,
          path: buildPathResponse(
            request.fromAnchor as CaptureSurfaceAnchor,
            request.toAnchor as CaptureSurfaceAnchor,
            request.pathMode as string,
            Number(request.previewId ?? previewIdCounter),
          ).path,
        };
      }

      if (cmd === 'preview_external_shape_surface_trim_loop') {
        if (config.loopError) {
          throw new Error(config.loopError);
        }

        const request = (args?.request ?? {}) as Record<string, unknown>;
        const loopAnchors = (request.loopAnchors ?? []) as Array<Record<string, unknown>>;
        previewIdCounter += 1;
        return {
          previewId: request.previewId ?? previewIdCounter,
          sourceMeshContentDigest: MESH_DIGEST,
          pathMode: request.pathMode ?? 'shortest',
          loopTrianglePath: [0, 1, 2],
          loopSegments: mapLoopSegments(loopAnchors),
        };
      }

      if (cmd === 'preview_external_shape_surface_trim_region') {
        if (config.regionError) {
          throw new Error(config.regionError);
        }

        const request = (args?.request ?? {}) as Record<string, unknown>;
        if (request.capMode === 'flat' && config.capError) {
          throw new Error(config.capError);
        }
        const loopAnchors = (request.loopAnchors ?? []) as Array<Record<string, unknown>>;
        const keepSeed = request.keepSeed as Record<string, unknown>;
        previewIdCounter += 1;
        return {
          previewId: request.previewId ?? previewIdCounter,
          preview: {
            sourceMeshContentDigest: MESH_DIGEST,
            pathMode: request.pathMode ?? 'shortest',
            loopSegmentCount: 3,
            loopTrianglePath: [0, 1, 2],
            keepSeedTriangleIndex: keepSeed.triangleIndex,
            retainedTriangleIndices: [0, 1],
            retainedTriangleCount: 2,
            excludedTriangleCount: 2,
            loopSegments: mapLoopSegments(loopAnchors),
          },
          topology: {
            retainedArea: 1,
            outputVertexCount: 4,
            outputTriangleCount: 4,
            duplicatePositionCount: 0,
            boundaryEdgeCount: 0,
            nonManifoldEdgeCount: 0,
            orientationMismatchCount: 0,
            invalidCutVertexDegreeCount: 0,
            closedBoundaryLoops: [],
            openBoundaryChains: [],
          },
          capReports: [
            {
              mode: request.capMode ?? 'open',
              boundaryPointCount: 3,
              addedVertexCount: 0,
              addedTriangleCount: 1,
              maxPlanarityDeviation: 0,
              rmsPlanarityDeviation: 0,
              explicitlyOpen: false,
            },
          ],
          capPreview: request.capMode === 'open'
            ? null
            : {
                vertices: [[0, 0, 0], [120, 0, 0], [0, 120, 0]],
                triangles: [[0, 1, 2]],
              },
        };
      }

      if (cmd === 'apply_external_shape_surface_trim') {
        if (config.applyError) {
          throw new Error(config.applyError);
        }
        if ((config.applyDelayMs ?? 0) > 0) {
          await new Promise((resolve) => {
            setTimeout(resolve, config.applyDelayMs);
          });
        }

        const request = (args?.request ?? {}) as Record<string, unknown>;
        const loopAnchors = (request.loopAnchors ?? []) as Array<Record<string, unknown>>;
        const keepSeed = request.keepSeed as Record<string, unknown>;
        const surfaceTrim = {
          nodeId: 31,
          schemaVersion: 1,
          sourceDigest: MESH_DIGEST,
          loopAnchors: loopAnchors.map((anchor) => ({
            triangleIndex: anchor.triangleIndex,
            barycentric: anchor.barycentric,
            sourcePosition: anchor.sourcePosition,
            sourceNormal: anchor.sourceNormal,
          })),
          keepSeed: {
            triangleIndex: keepSeed.triangleIndex,
            barycentric: keepSeed.barycentric,
            sourcePosition: keepSeed.sourcePosition,
            sourceNormal: keepSeed.sourceNormal,
          },
          pathMode: request.pathMode ?? 'shortest',
          capMode: request.capMode ?? 'open',
        };

        pendingExternalShapeSources = externalShapeSources.map((source) => ({
          ...source,
          nodeId: 21,
          sourceDigest: 'sha256:source-2',
          surfaceTrims: [surfaceTrim],
        }));
        pendingCanonicalSource = '(model (part head (surface-trim (import-stl "/tmp/donor.stl") :schema-version 1 :source-digest "sha256:mesh-1" :loop ((mesh-anchor 0 1 0 0) (mesh-anchor 1 1 0 0) (mesh-anchor 2 1 0 0)) :keep-seed (mesh-anchor 3 1 0 0) :path-mode "shortest" :cap "flat")))';

        return {
          source: pendingCanonicalSource,
          sourceDigest: 'sha256:source-2',
          trimNodeId: 31,
          pointCount: 3,
          pathMode: request.pathMode ?? 'shortest',
          capMode: request.capMode ?? 'open',
          topology: {
            retainedArea: 1,
            outputVertexCount: 4,
            outputTriangleCount: 4,
            duplicatePositionCount: 0,
            boundaryEdgeCount: 0,
            nonManifoldEdgeCount: 0,
            orientationMismatchCount: 0,
            invalidCutVertexDegreeCount: 0,
            closedBoundaryLoops: [],
            openBoundaryChains: [],
          },
          capReports: [
            {
              mode: request.capMode ?? 'open',
              boundaryPointCount: 3,
              addedVertexCount: 0,
              addedTriangleCount: 1,
              maxPlanarityDeviation: 0,
              rmsPlanarityDeviation: 0,
              explicitlyOpen: false,
            },
          ],
        };
      }

      if (cmd === 'remove_external_shape_surface_trim') {
        const request = (args?.request ?? {}) as Record<string, unknown>;
        pendingExternalShapeSources = externalShapeSources.map((source) => ({
          ...source,
          sourceDigest: 'sha256:source-3',
          surfaceTrims: ((source.surfaceTrims as Array<Record<string, unknown>> | undefined) ?? [])
            .filter((trim) => trim.nodeId !== request.trimNodeId),
        }));
        pendingCanonicalSource = '(model (part head (import-stl "/tmp/donor.stl")))';

        return {
          source: pendingCanonicalSource,
          sourceDigest: 'sha256:source-3',
          removedTrimNodeId: request.trimNodeId ?? null,
        };
      }

      if (cmd === 'render_model') {
        return {
          modelId: 'surface-trim-model',
          sourceKind: 'generated',
          sourceLanguage: 'ecky',
          geometryBackend: 'mesh',
          engineKind: 'ecky',
          contentHash: 'sha256:render-1',
          fcstdPath: '/tmp/surface-trim.FCStd',
          manifestPath: '/tmp/surface-trim-manifest.json',
          modelStlPath: '/tmp/surface-trim-model.stl',
          viewerAssets: [],
          edgeTargets: [],
          faceTargets: [],
          calloutAnchors: [],
          measurementGuides: [],
        };
      }
      if (cmd === 'get_model_manifest') {
        return {
          modelId: String(args?.modelId ?? 'surface-trim-model'),
          sourceKind: 'generated',
          sourceDigest: 'sha256:render-1',
          engineKind: 'ecky',
          sourceLanguage: 'ecky',
          geometryBackend: 'mesh',
          document: {
            documentName: 'Surface Trim',
            documentLabel: 'Surface Trim',
            objectCount: 1,
            warnings: [],
          },
          parts: [],
          parameterGroups: [],
          controlPrimitives: [],
          controlRelations: [],
          controlViews: [],
          previewViews: [],
          advisories: [],
          selectionTargets: [],
          measurementAnnotations: [],
          taggedAnchors: {},
          analysisDeclarations: [],
          warnings: [],
          enrichmentState: { status: 'none', proposals: [] },
        };
      }
      if (cmd === 'save_model_manifest') return null;
      if (cmd === 'path_exists') return true;
      if (cmd === 'path_size') return 1024;

      if (cmd === 'save_project_source') {
        canonicalSource = (args?.source as string) ?? pendingCanonicalSource ?? canonicalSource;
        if (pendingExternalShapeSources) {
          externalShapeSources = pendingExternalShapeSources;
          window.localStorage.setItem(storageKey, JSON.stringify(externalShapeSources));
        }
        pendingExternalShapeSources = null;
        pendingCanonicalSource = null;
        return {
          threadId: THREAD_ID,
          slug: 'finger-fixture',
          folder: '/tmp/finger-fixture',
          file: '/tmp/finger-fixture/model.ecky',
          source: canonicalSource,
        };
      }

      if (cmd === 'project_folder_status') {
        return { status: 'clean' };
      }

      return null;
    };
  }, {
    config,
    storageKey: STORAGE_KEY,
    threadId: THREAD_ID,
    sourceDigest: SOURCE_DIGEST,
    meshDigest: MESH_DIGEST,
  });
}

async function openSurfaceTrimPage(page: Page) {
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Work with external shapes' }).click();
  await expect(page.getByRole('tab', { name: /^IMPORT$/ })).toBeVisible();
  await expect(page.getByRole('tab', { name: /^CROP$/ })).toBeVisible();
  await page.getByRole('tab', { name: /^CROP$/ }).click();
}

async function clearInvokeLog(page: Page) {
  await page.evaluate(() => {
    (window as any).__CAPTURE_CALLS__ = [];
  });
}

async function getInvokeCalls(page: Page): Promise<InvokeCall[]> {
  return page.evaluate(() => ((window as any).__CAPTURE_CALLS__ ?? []) as InvokeCall[]);
}

async function clickViewerPoint(page: Page, xRatio: number, yRatio: number) {
  const canvas = page.locator('[role="dialog"][data-window-id="capture"] .viewer-host canvas');
  await expect(canvas).toBeVisible();
  const bounds = await canvas.boundingBox();
  if (!bounds) throw new Error('Capture Viewer canvas has no layout bounds.');
  await canvas.click({
    position: {
      x: bounds.width * xRatio,
      y: bounds.height * yRatio,
    },
  });
}

async function openCropSurfaceTrace(page: Page) {
  await expect(page.getByRole('button', { name: /^CUT PLANE$/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /^TRACE SURFACE$/ })).toBeVisible();
  await page.getByRole('button', { name: /^TRACE SURFACE$/ }).click();
}

test('Given External Shapes crop, When TRACE SURFACE closes a loop, Then region preview and apply request work', async ({ page }, testInfo) => {
  await routeSurfaceTrimStl(page);
  await installSurfaceTrimMocks(page);
  await openSurfaceTrimPage(page);
  await openCropSurfaceTrace(page);
  await expect(page.locator('[role="dialog"][data-window-id="capture"] .viewer-host[data-crop-box-enabled="false"]')).toHaveCount(1);
  await expect(page.locator('[data-capture-guide-overlay]')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /^BOX$/ })).toHaveCount(0);
  await expect(page.getByRole('button', { name: /^PLANE$/ })).toHaveCount(0);

  await clearInvokeLog(page);
  await clickViewerPoint(page, 0.43, 0.58);
  await clickViewerPoint(page, 0.47, 0.68);
  await clickViewerPoint(page, 0.38, 0.72);

  await page.getByRole('button', { name: /^CLOSE LOOP$/ }).click();
  await expect(page.getByText('CLICK REGION TO KEEP')).toBeVisible();

  const loopCall = (await getInvokeCalls(page)).find((call) => call.cmd === 'preview_external_shape_surface_trim_loop');
  expect(loopCall).toBeTruthy();
  expect(loopCall?.args?.request).toMatchObject({
    schemaVersion: 1,
    threadId: THREAD_ID,
    targetMessageId: null,
    nodeId: 11,
    expectedSourceDigest: SOURCE_DIGEST,
    expectedMeshContentDigest: MESH_DIGEST,
    pathMode: expect.any(String),
    previewId: expect.any(Number),
    loopAnchors: [
      expect.objectContaining({ sourceMeshContentDigest: MESH_DIGEST }),
      expect.objectContaining({ sourceMeshContentDigest: MESH_DIGEST }),
      expect.objectContaining({ sourceMeshContentDigest: MESH_DIGEST }),
    ],
  });

  await clearInvokeLog(page);
  await clickViewerPoint(page, 0.45, 0.64);
  await expect(page.getByText('KEEP 2 TRIANGLES')).toBeVisible();

  const regionCall = (await getInvokeCalls(page)).find((call) => call.cmd === 'preview_external_shape_surface_trim_region');
  expect(regionCall).toBeTruthy();
  expect(regionCall?.args?.request).toMatchObject({
    schemaVersion: 1,
    threadId: THREAD_ID,
    targetMessageId: null,
    nodeId: 11,
    expectedSourceDigest: SOURCE_DIGEST,
    expectedMeshContentDigest: MESH_DIGEST,
    pathMode: expect.any(String),
    capMode: expect.any(String),
    previewId: expect.any(Number),
    loopAnchors: [
      expect.objectContaining({ sourceMeshContentDigest: MESH_DIGEST }),
      expect.objectContaining({ sourceMeshContentDigest: MESH_DIGEST }),
      expect.objectContaining({ sourceMeshContentDigest: MESH_DIGEST }),
    ],
    keepSeed: expect.objectContaining({ sourceMeshContentDigest: MESH_DIGEST }),
  });

  await page.getByRole('button', { name: /^FLAT$/ }).click();
  await expect(page.locator('[role="dialog"][data-window-id="capture"] .viewer-host[data-surface-trim-cap-preview="true"]')).toHaveCount(1);
  await page.getByRole('button', { name: /^APPLY SURFACE TRIM$/ }).click();

  const applyCall = (await getInvokeCalls(page)).find((call) => call.cmd === 'apply_external_shape_surface_trim');
  expect(applyCall).toBeTruthy();
  expect(applyCall?.args?.request).toMatchObject({
    schemaVersion: 1,
    threadId: THREAD_ID,
    targetMessageId: null,
    nodeId: 11,
    expectedSourceDigest: SOURCE_DIGEST,
    expectedMeshContentDigest: MESH_DIGEST,
    replaceTrimNodeId: null,
    pathMode: expect.any(String),
    capMode: 'flat',
    loopAnchors: [
      expect.objectContaining({ sourceMeshContentDigest: MESH_DIGEST }),
      expect.objectContaining({ sourceMeshContentDigest: MESH_DIGEST }),
      expect.objectContaining({ sourceMeshContentDigest: MESH_DIGEST }),
    ],
    keepSeed: expect.objectContaining({ sourceMeshContentDigest: MESH_DIGEST }),
  });
  await expect.poll(async () => (await getInvokeCalls(page)).some((call) => call.cmd === 'save_project_source')).toBe(true);
  const saveCall = (await getInvokeCalls(page)).find((call) => call.cmd === 'save_project_source');
  expect(saveCall?.args?.source).toBe('(model (part head (surface-trim (import-stl "/tmp/donor.stl") :schema-version 1 :source-digest "sha256:mesh-1" :loop ((mesh-anchor 0 1 0 0) (mesh-anchor 1 1 0 0) (mesh-anchor 2 1 0 0)) :keep-seed (mesh-anchor 3 1 0 0) :path-mode "shortest" :cap "flat")))');

  await page.reload();
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Work with external shapes' }).click();
  await page.getByRole('tab', { name: /^CROP$/ }).click();
  await expect(page.getByRole('region', { name: 'Existing surface trims' })).toBeVisible();
  await expect(page.getByText('SURFACE TRIMS')).toBeVisible();
  await expect(page.getByText('3 POINTS')).toBeVisible();
  await expect(page.getByText('FLAT')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Edit surface trim 1' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Remove surface trim 1' })).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath('surface-trim-applied.png') });

  await page.getByRole('button', { name: 'Edit surface trim 1' }).click();
  await expect(page.getByText('PREVIEW READY')).toBeVisible();
  await page.getByRole('button', { name: /^CANCEL$/ }).click();
  await page.getByRole('button', { name: 'Remove surface trim 1' }).click();

  const removeCall = (await getInvokeCalls(page)).find((call) => call.cmd === 'remove_external_shape_surface_trim');
  expect(removeCall).toBeTruthy();
  expect(removeCall?.args?.request).toMatchObject({
    threadId: THREAD_ID,
    targetMessageId: null,
    nodeId: 21,
    trimNodeId: 31,
    expectedSourceDigest: 'sha256:source-2',
  });
});

test('Given stale digest on apply, When APPLY SURFACE TRIM runs, Then raw error shows and render is skipped', async ({ page }) => {
  await routeSurfaceTrimStl(page);
  await installSurfaceTrimMocks(page, { applyError: 'stale digest: expected sha256:source-old, got sha256:source-2' });
  await openSurfaceTrimPage(page);
  await openCropSurfaceTrace(page);

  await clearInvokeLog(page);
  await clickViewerPoint(page, 0.43, 0.58);
  await clickViewerPoint(page, 0.47, 0.68);
  await clickViewerPoint(page, 0.38, 0.72);
  await page.getByRole('button', { name: /^CLOSE LOOP$/ }).click();
  await clickViewerPoint(page, 0.45, 0.64);
  await page.getByRole('button', { name: /^FLAT$/ }).click();
  await page.getByRole('button', { name: /^APPLY SURFACE TRIM$/ }).click();

  await expect(page.getByText(/stale digest: expected sha256:source-old, got sha256:source-2/i)).toBeVisible();

  const calls = await getInvokeCalls(page);
  expect(calls.some((call) => call.cmd === 'render_model')).toBe(false);
  expect(calls.some((call) => call.cmd === 'save_project_source')).toBe(false);
});

test('Given pending apply, When APPLY SURFACE TRIM runs, Then mutation waits for completion', async ({ page }) => {
  await routeSurfaceTrimStl(page);
  await installSurfaceTrimMocks(page, { applyDelayMs: 300 });
  await openSurfaceTrimPage(page);
  await openCropSurfaceTrace(page);

  await clearInvokeLog(page);
  await clickViewerPoint(page, 0.43, 0.58);
  await clickViewerPoint(page, 0.47, 0.68);
  await clickViewerPoint(page, 0.38, 0.72);
  await page.getByRole('button', { name: /^CLOSE LOOP$/ }).click();
  await clickViewerPoint(page, 0.45, 0.64);
  await page.getByRole('button', { name: /^FLAT$/ }).click();
  await clearInvokeLog(page);

  await page.getByRole('button', { name: /^APPLY SURFACE TRIM$/ }).click();

  await expect(page.getByRole('button', { name: /^APPLYING$/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /^APPLYING$/ })).toBeDisabled();

  const pendingCalls = await getInvokeCalls(page);
  expect(pendingCalls.some((call) => call.cmd === 'render_model')).toBe(false);
  expect(pendingCalls.some((call) => call.cmd === 'save_project_source')).toBe(false);

  await expect(page.getByRole('region', { name: 'Existing surface trims' })).toBeVisible({ timeout: 3000 });
});

test('Given non-planar Flat cap failure, When FLAT runs, Then raw error shows and apply stays unavailable', async ({ page }) => {
  await routeSurfaceTrimStl(page);
  await installSurfaceTrimMocks(page, {
    capError: 'Surface trim cap failed: PlanarityToleranceExceeded maxDeviation=2.400 tolerance=0.100',
  });
  await openSurfaceTrimPage(page);
  await openCropSurfaceTrace(page);

  await clearInvokeLog(page);
  await clickViewerPoint(page, 0.43, 0.58);
  await clickViewerPoint(page, 0.47, 0.68);
  await clickViewerPoint(page, 0.38, 0.72);
  await page.getByRole('button', { name: /^CLOSE LOOP$/ }).click();
  await clickViewerPoint(page, 0.45, 0.64);
  await clearInvokeLog(page);
  await page.getByRole('button', { name: /^FLAT$/ }).click();

  await expect(page.getByText('Surface trim cap failed: PlanarityToleranceExceeded maxDeviation=2.400 tolerance=0.100')).toBeVisible();
  await expect(page.getByRole('button', { name: /^APPLY SURFACE TRIM$/ })).toBeDisabled();

  const calls = await getInvokeCalls(page);
  expect(calls.some((call) => call.cmd === 'render_model')).toBe(false);
  expect(calls.some((call) => call.cmd === 'save_project_source')).toBe(false);
});

test('Given invalid loop preview, When CLOSE LOOP runs, Then raw error shows and region actions stay unavailable', async ({ page }) => {
  await routeSurfaceTrimStl(page);
  await installSurfaceTrimMocks(page, { loopError: 'invalid loop: non-manifold gap' });
  await openSurfaceTrimPage(page);
  await openCropSurfaceTrace(page);

  await clearInvokeLog(page);
  await clickViewerPoint(page, 0.43, 0.58);
  await clickViewerPoint(page, 0.47, 0.68);
  await clickViewerPoint(page, 0.38, 0.72);
  await page.getByRole('button', { name: /^CLOSE LOOP$/ }).click();

  await expect(page.getByText(/invalid loop: non-manifold gap/i)).toBeVisible();
  await expect(page.getByText('CLICK REGION TO KEEP')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /^APPLY SURFACE TRIM$/ })).toHaveCount(0);

  const calls = await getInvokeCalls(page);
  expect(calls.some((call) => call.cmd === 'preview_external_shape_surface_trim_region')).toBe(false);
});
