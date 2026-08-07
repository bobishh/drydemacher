import { expect, test, type Page } from '@playwright/test';
import {
  partialSymmetricMechanicalInsert,
  partialSymmetricMechanicalInsertStl,
} from './fixtures/partial-symmetric-mechanical-insert.js';

type GuideMockMode = 'happy' | 'degenerate-frame' | 'stale-source' | 'source-divergence' | 'comparison'
  | 'ambiguous-plan' | 'noisy-fit' | 'unsupported-primitive';

async function installGuidedCaptureFixture(page: Page, mode: GuideMockMode) {
  await page.addInitScript(({ fixture, mode }) => {
    const mockWindow = window as typeof window & { __CAPTURE_GUIDE_CALLS__?: unknown[] };
    mockWindow.__CAPTURE_GUIDE_CALLS__ = [];
    const generatedArtifactBundle = {
      modelId: 'generated-model-1', sourceKind: 'generated', contentHash: 'generated-content', artifactVersion: 1,
      fcstdPath: '', manifestPath: '/fixtures/generated-manifest.json',
      previewStlPath: '/fixtures/generated-brep.stl', viewerAssets: [], exportArtifacts: [],
      geometryBackend: 'mesh', sourceLanguage: 'ecky', engineKind: 'ecky',
    };
    const generatedManifest = {
      schemaVersion: 1, modelId: 'generated-model-1', sourceKind: 'generated', engineKind: 'ecky',
      sourceLanguage: 'ecky', geometryBackend: 'mesh',
      document: { documentName: 'Generated insert', documentLabel: 'Generated insert', sourcePath: '/fixtures/generated.ecky', objectCount: 1, warnings: [] },
      parts: [], parameterGroups: [], controlPrimitives: [], controlRelations: [], controlViews: [],
      advisories: [], selectionTargets: [], measurementAnnotations: [], warnings: [], enrichmentState: { status: 'none', proposals: [] },
    };
    const generatedMessage = {
      id: 'generated-message-1', role: 'assistant', content: 'Generated guided BRep', status: 'success', timestamp: 2,
      output: {
        title: 'Generated insert', description: '', macroCode: '(model)', versionName: 'Guided BRep',
        parameters: {}, sourceLanguage: 'ecky', geometryBackend: 'mesh', engineKind: 'ecky',
      },
      artifactBundle: generatedArtifactBundle,
      modelManifest: generatedManifest,
    };
    const ownerThread = {
      id: 'thread-owner', title: 'Insert', updatedAt: 2, versionCount: 1, pendingCount: 0,
      queuedCount: 0, errorCount: 0, status: 'ready', summary: 'Guided insert', messages: [generatedMessage],
    };
    window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
    window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: Record<string, unknown>) => {
      mockWindow.__CAPTURE_GUIDE_CALLS__?.push({ cmd, args });
      if (cmd === 'get_config') return {
        engines: [], selectedEngineId: '', freecadCmd: '', assets: [],
        microwave: { humId: null, dingId: null, muted: true }, voice: { sttLanguageCode: 'en-US' },
        mcp: { port: null, maxSessions: null, mode: 'active', primaryAgentId: null, promptTimeoutSecs: 1800, eckyAstAuthoring: false, autoAgents: [] },
        hasSeenOnboarding: true, connectionType: 'mcp', defaultEngineKind: 'ecky',
        defaultSourceLanguage: 'ecky', defaultGeometryBackend: 'mesh', maxGenerationAttempts: 1, maxVerifyAttempts: 0,
      };
      if (cmd === 'get_runtime_capabilities') return {
        freecad: { available: true, detail: 'Ready', path: '/mock/freecadcmd' },
        build123d: { available: true, detail: 'Ready', path: '/mock/python3' },
        mesh: { available: true, detail: 'bundled', path: null },
        recommendedAuthoringContext: { engineKind: 'ecky', sourceLanguage: 'ecky', geometryBackend: 'mesh' },
      };
      if (cmd === 'start_capture_session' || cmd === 'get_capture_session_status') return {
        sessionId: 'partial-insert-capture', pairingToken: 'partial-insert-token', pairingUrl: '', trustUrl: '',
        targetThreadId: 'thread-owner', targetMessageId: null,
        protocolVersion: 1, clientCapabilities: {}, state: 'preview', createdAt: 1, expiresAt: 999999,
        acceptedFrameCount: 24, reconstructionProgress: 1,
        meshPreview: { stlPath: '/fixtures/partial-symmetric-mechanical-insert.stl', triangleCount: fixture.triangles.length,
          boundsMm: fixture.expectedBrepEnvelopeMm, scaleLabel: 'source coordinates', warnings: [] },
      };
      if (cmd === 'get_history') return mode === 'comparison' ? [ownerThread] : [];
      if (cmd === 'get_last_design') return null;
      if (cmd === 'get_default_macro') return '(solid blank)';
      if (cmd === 'get_active_agent_sessions' || cmd === 'get_agent_terminal_snapshots') return [];
      if (cmd === 'get_thread_agent_state') return { connectionState: 'disconnected', sessions: [], primaryAgentLabel: null, statusText: '', phase: null, busy: false, agentLabel: null, activityLabel: '', sessionId: null };
      if (cmd === 'get_thread_latest_version') return mode === 'comparison' ? generatedMessage : null;
      if (cmd === 'get_thread_message_version') return mode === 'comparison' ? generatedMessage : null;
      if (cmd === 'get_thread_messages_page') return {
        messages: mode === 'comparison' ? [generatedMessage] : [], nextBefore: null, hasMore: false,
      };
      if (cmd === 'get_thread') return ownerThread;
      if (cmd === 'list_capture_runs') return mode === 'comparison' ? [{
        id: 'partial-insert-capture', targetThreadId: 'thread-owner', targetMessageId: null,
        title: 'Insert capture', state: 'preview', createdAt: 1, updatedAt: 2, acceptedFrameCount: 24,
        meshPreview: { stlPath: '/fixtures/partial-symmetric-mechanical-insert.stl', triangleCount: fixture.triangles.length,
          boundsMm: fixture.expectedBrepEnvelopeMm, scaleLabel: 'source coordinates', warnings: [] },
        derivedStlPath: null, cropBounds: null, previewScale: 1, targetSource: '', targetSourceLanguage: 'ecky',
        startedFromEmpty: false, rawError: null, reconstructionGuide: null, reconstructionGuideState: null,
        guidedReconstructionMessageId: 'generated-message-1', guidedReconstructionModelId: 'generated-model-1',
        guidedReconstructionResult: {
          guideId: 'guide-comparison', guideRevision: 2, guideCanonicalDigest: 'sha256:guide',
          sourceMeshArtifactDigest: fixture.artifactDigest, sourceMeshContentDigest: fixture.contentDigest,
          targetSourceDigest: 'sha256:target-source', targetVersionId: null,
          generatedSourceDigest: 'sha256:generated', geometryDigest: 'sha256:geometry', assumptions: [],
          inferredRegions: ['three mirrored quarters from X/Y symmetry'],
          correspondences: [{
            expectationId: 'expectation-profile-edges', guideItemIds: ['profile-visible-quarter'],
            partId: 'insert', instancePath: null, authoredSelector: { kind: 'binding', name: 'mating_rail_edges' },
            selectorCardinality: 'oneOrMore', brepTargetKind: 'orderedEdges',
            canonicalTargetIds: ['edge:1'], durableTargetIds: ['durable:edge:1'], sourceStableNodeKeys: ['profile-node'],
            sourceGeometryDigest: 'sha256:geometry', relation: 'profiles',
            residual: { metric: 'orderedProfileDistance', maximum: 0.14, rms: 0.07, unit: 'mm', components: [] },
            status: 'satisfied',
          }],
        },
        guidedReconstructionDeviation: {
          schemaVersion: 1, guideId: 'guide-comparison', guideRevision: 2,
          guideCanonicalDigest: 'sha256:guide', sourceMeshContentDigest: fixture.contentDigest,
          generatedGeometryDigest: 'sha256:geometry', parts: [], sourceVertexCount: 12, sampleCount: 4,
          maximumMm: 0.18, rmsMm: 0.09, percentile95Mm: 0.17, outlierThresholdMm: 0.2,
          outlierCount: 1, evidenceScope: 'observedRegionOnly',
          displaySamples: [
            { sourceVertexIndex: 0, localPositionMm: [0, 0, 0], distanceMm: 0.04 },
            { sourceVertexIndex: 1, localPositionMm: [20, 0, 0], distanceMm: 0.12 },
            { sourceVertexIndex: 2, localPositionMm: [20, 15, 0], distanceMm: 0.4 },
          ],
          contentDigest: 'sha256:deviation',
        },
      }] : [];
      if (cmd === 'prepare_capture_preview') return { artifactBundle: {
        modelId: 'partial-insert-mesh', sourceKind: 'generated', contentHash: fixture.contentDigest, artifactVersion: 1,
        fcstdPath: '', manifestPath: '/fixtures/partial-insert-manifest.json',
        previewStlPath: '/fixtures/partial-symmetric-mechanical-insert.stl', viewerAssets: [], exportArtifacts: [],
        geometryBackend: 'mesh', sourceLanguage: 'ecky', engineKind: 'ecky',
      }, modelManifest: {
        schemaVersion: 1, modelId: 'partial-insert-mesh', sourceKind: 'generated', engineKind: 'ecky',
        sourceLanguage: 'ecky', geometryBackend: 'mesh',
        document: { documentName: 'Partial insert', documentLabel: 'Partial insert', sourcePath: '/fixtures/partial-symmetric-mechanical-insert.stl', objectCount: 1, warnings: [] },
        parts: [], parameterGroups: [], controlPrimitives: [], controlRelations: [], controlViews: [],
        advisories: [], selectionTargets: [], measurementAnnotations: [], warnings: [], enrichmentState: { status: 'none', proposals: [] },
      } };
      if (cmd === 'get_capture_reconstruction_guide') return null;
      if (cmd === 'get_capture_guide_context') return {
        sourceMesh: {
          artifactDigest: fixture.artifactDigest,
          contentDigest: fixture.contentDigest,
          selection: 'raw',
          cropDigest: null,
          triangleCount: fixture.triangles.length,
          sourceBounds: { min: [0, 0, 0], max: [40, 30, 18] },
        },
        targetSourceDigest: 'sha256:target-source',
        targetVersionId: null,
      };
      if (cmd === 'save_capture_reconstruction_guide') {
        const guideState = args?.guideState as { status?: string } | undefined;
        if (guideState?.status === 'ready' && mode === 'degenerate-frame') throw new Error('Frame evidence is degenerate: origin, X, and Y landmarks are collinear.');
        if (guideState?.status === 'ready' && mode === 'stale-source') throw new Error('Guide is stale: selected crop/source mesh digest changed.');
        const guide = structuredClone(args?.guide as Record<string, unknown>);
        return {
          ...guide,
          revision: Number(args?.expectedRevision ?? 0) + 1,
          canonicalDigest: `sha256:guide-${Number(args?.expectedRevision ?? 0) + 1}`,
        };
      }
      if (cmd === 'evaluate_capture_reconstruction_guide') {
        if (mode === 'noisy-fit') {
          throw new Error('Circle fit maximum residual 1.8 mm exceeds tolerance 0.25 mm.');
        }
        const guide = structuredClone(args?.guide as Record<string, unknown>) as Record<string, any>;
        const selected = guide.selectedFeaturePlanId as string | null | undefined;
        const ambiguous = mode === 'ambiguous-plan' && !selected;
        const unsupported = mode === 'unsupported-primitive';
        const plans = ambiguous ? [
          { planId: 'plan:profile-1:extrude', label: 'Observed profile extrude', operations: [], supportingEvidenceIds: ['profile-1'], rejectingEvidence: [], score: 0.91, status: 'needsConfirmation' },
          { planId: 'plan:profile-1:revolve', label: 'Observed profile revolve', operations: [], supportingEvidenceIds: ['profile-1'], rejectingEvidence: [], score: 0.88, status: 'needsConfirmation' },
        ] : [{
          planId: selected ?? 'plan:profile-1:extrude', label: 'Observed profile extrude', operations: [],
          supportingEvidenceIds: ['profile-1'], rejectingEvidence: [], score: 0.91, status: 'supported',
        }];
        const primitiveHypotheses = unsupported ? [{
          hypothesisId: 'hypothesis:bore:cylinder:rejected', guideItemIds: ['bore-1'], kind: 'cylinder',
          status: 'rejected', candidateId: null, domain: { parameterName: 'axisMm', minimum: 0, maximum: 12, observedOnly: true },
          fit: { rmsMm: 0.9, maxMm: 1.7, toleranceMm: 0.25 }, reason: 'Cylinder evidence is degenerate and over tolerance.',
        }] : [];
        const ready = !ambiguous && !unsupported;
        return {
          ...guide,
          primitiveHypotheses,
          featurePlanCandidates: plans,
          selectedFeaturePlanId: ready ? (selected ?? plans[0].planId) : null,
          reconstructionReadiness: {
            ready,
            stages: [{
              stage: unsupported ? 'primitiveFit' : 'featurePlan',
              status: unsupported ? 'missing' : (ambiguous ? 'ambiguous' : 'satisfied'),
              affectedEvidenceIds: unsupported ? ['bore-1'] : plans.map(plan => plan.planId),
              detail: unsupported ? 'Unsupported cylinder fit blocks reconstruction.' : 'One evidence-backed feature plan must be selected.',
            }],
            missingStages: unsupported ? ['primitiveFit'] : [],
            ambiguousStages: ambiguous ? ['featurePlan'] : [],
            selectedFeaturePlanId: ready ? (selected ?? plans[0].planId) : null,
            detail: ready ? 'Deterministic reconstruction stack is ready.' : 'Explicit resolution required.',
          },
        };
      }
      if (cmd === 'queue_capture_guided_reconstruction') {
        if (mode === 'source-divergence') {
          throw new Error('Capture target source diverged: expected sha256:target-source, found sha256:changed-source.');
        }
        return {
          requestId: 'capture-guide:sha256:request',
          threadId: 'thread-owner',
          messageId: 'guided-message-1',
        };
      }
      return null;
    };
  }, { fixture: partialSymmetricMechanicalInsert, mode });
  await page.route('**/partial-symmetric-mechanical-insert.stl*', route => route.fulfill({
    status: 200, contentType: 'model/stl', body: partialSymmetricMechanicalInsertStl(),
  }));
  await page.route('**/generated-brep.stl*', route => route.fulfill({
    status: 200, contentType: 'model/stl', body: partialSymmetricMechanicalInsertStl(),
  }));
}

async function pickScanEvidence(
  page: Page,
  roleButton: string,
  positions: Array<{ x: number; y: number }>,
) {
  await page.getByRole('button', { name: roleButton }).click();
  const canvas = page.locator('[data-testid="capture-preview-viewport"] canvas');
  const box = await canvas.boundingBox();
  if (!box) throw new Error('capture canvas missing');
  for (const position of positions) {
    await canvas.click({ position: { x: box.width * position.x, y: box.height * position.y } });
  }
}

async function authorMinimumMechanicalGuide(page: Page) {
  const observedTriangle = { x: 0.46, y: 0.60 };
  await pickScanEvidence(page, 'PICK CALIBRATION ENDPOINT', [observedTriangle, observedTriangle]);
  await pickScanEvidence(page, 'PICK FRAME ORIGIN', [{ x: 0.46, y: 0.60 }]);
  await pickScanEvidence(page, 'PICK FRAME DIRECTION', [observedTriangle, observedTriangle]);
  await pickScanEvidence(page, 'PICK SYMMETRY PLANE', [observedTriangle, observedTriangle, observedTriangle]);
  await pickScanEvidence(page, 'PICK PROFILE VERTEX', [observedTriangle, observedTriangle, observedTriangle]);
  await expect(page.locator('.capture-panel__guide-point')).toHaveCount(11);
  await page.getByLabel('Reconstruction instruction').fill(
    'Reconstruct observed scan evidence as constrained parametric geometry; preserve stated symmetry and exact BRep targets.',
  );
}

async function openPartialInsertCapture(page: Page, mode: GuideMockMode) {
  await installGuidedCaptureFixture(page, mode);
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0);
  await page.getByRole('button', { name: 'Work with external shapes' }).click();
  await page.getByRole('button', { name: 'START CAPTURE' }).click();
  await expect(page.getByText('4 triangles')).toBeVisible();
  await expect(page.locator('[data-testid="capture-preview-viewport"]')).toHaveAttribute('data-preview-status', 'loaded');
}

test('Given a partial symmetric insert capture When calibration frame symmetry and profile evidence are chosen Then guided CAD can be requested', async ({ page }) => {
  // Given deterministic source triangles, known 40 mm span, and expected 80×60×18 mm BRep envelope.
  await openPartialInsertCapture(page, 'happy');

  // When guide mode is selected, evidence authoring remains attached to the immutable source mesh.
  await page.getByRole('tab', { name: 'GUIDED BREP' }).click();
  await page.getByRole('button', { name: 'START GUIDES' }).click();
  await authorMinimumMechanicalGuide(page);
  await expect(page.locator('[data-testid="capture-guide-overlay"] .capture-guide-overlay__point')).toHaveCount(11);
  await expect(page.getByText('OBSERVED REGION ONLY')).toBeVisible();

  // Draft edits stay inside guide persistence. Delete and generic undo never create model history.
  await page.getByText('LANDMARKS · 11', { exact: true }).click();
  await page.getByLabel('Landmark 1 label').fill('calibration datum A');
  await page.getByLabel('Landmark 1 label').blur();
  await page.getByRole('button', { name: 'Delete landmark 11' }).click();
  await expect(page.locator('.capture-panel__guide-point')).toHaveCount(10);
  await page.getByRole('button', { name: 'UNDO GUIDE EDIT' }).click();
  await expect(page.locator('.capture-panel__guide-point')).toHaveCount(11);
  await expect(page.getByLabel('Landmark 1 label')).toHaveValue('calibration datum A');
  await page.getByRole('button', { name: 'VALIDATE GUIDE' }).click();

  // Ordered profile and exact BRep target contract remain editable, then require revalidation.
  await page.getByText('PROFILE · 3 POINTS', { exact: true }).click();
  await page.getByLabel('Profile kind').selectOption('open');
  await page.getByRole('button', { name: 'Move profile point 3 up' }).click();
  await page.getByText(/^EXACT BREP TARGETS ·/).click();
  await page.getByLabel('expectation-profile-edges selector kind').selectOption('binding');
  await page.getByLabel('expectation-profile-edges selector name').fill('mating_rail_edges');
  await page.getByLabel('expectation-profile-edges selector name').blur();
  await expect(page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' })).toBeDisabled();
  await page.getByRole('button', { name: 'VALIDATE GUIDE' }).click();
  await expect(page.getByText('INFERRED HALF · UNVERIFIED')).toBeVisible();

  // Then only valid evidence enables handoff to the owning capture task.
  await expect(page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' })).toBeEnabled();
  await page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' }).click();
  await expect.poll(async () => page.evaluate(() => (window as typeof window & { __CAPTURE_GUIDE_CALLS__?: Array<{ cmd: string }> }).__CAPTURE_GUIDE_CALLS__?.filter(call => call.cmd === 'queue_capture_guided_reconstruction').length ?? 0)).toBe(1);
  await expect.poll(async () => page.evaluate(() => (window as typeof window & { __CAPTURE_GUIDE_CALLS__?: Array<{ cmd: string }> }).__CAPTURE_GUIDE_CALLS__?.filter(call => call.cmd === 'add_manual_version').length ?? 0)).toBe(0);
  const readyGuide = await page.evaluate(() => {
    const calls = (window as typeof window & {
      __CAPTURE_GUIDE_CALLS__?: Array<{ cmd: string; args?: Record<string, unknown> }>;
    }).__CAPTURE_GUIDE_CALLS__ ?? [];
    return calls
      .filter(call => call.cmd === 'save_capture_reconstruction_guide'
        && (call.args?.guideState as { status?: string } | undefined)?.status === 'ready')
      .at(-1)?.args?.guide as {
        profiles: Array<{ kind: string; landmarkIds: string[] }>;
        featureExpectations: Array<{
          expectationId: string;
          expectedAuthoredSelector: { kind: string; name: string };
        }>;
      };
  });
  expect(readyGuide.profiles[0].kind).toBe('open');
  expect(readyGuide.profiles[0].landmarkIds).toEqual(['landmark-9', 'landmark-11', 'landmark-10']);
  expect(readyGuide.featureExpectations.find(item => item.expectationId === 'expectation-profile-edges')?.expectedAuthoredSelector)
    .toEqual({ kind: 'binding', name: 'mating_rail_edges' });
});

test('Given crop inspection was active When Guided CAD opens Then the gizmo disappears and evidence controls use progressive disclosure', async ({ page }) => {
  await openPartialInsertCapture(page, 'happy');
  await page.getByRole('button', { name: 'BOX CROP' }).click();
  const viewer = page.locator('[data-testid="capture-preview-viewport"] .viewer-host');
  await expect(viewer).toHaveAttribute('data-crop-box-enabled', 'true');

  await page.getByRole('button', { name: 'GUIDED CAD' }).click();

  await expect(viewer).toHaveAttribute('data-crop-box-enabled', 'false');
  await expect(page.getByRole('button', { name: 'BOX CROP' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'PICK CALIBRATION ENDPOINT' }))
    .toHaveAttribute('title', 'Pick two scan points whose physical distance is known.');
  await expect(page.getByRole('button', { name: 'PICK NAMED REFERENCE' })).not.toBeVisible();
  await expect(page.getByText('ADVANCED EVIDENCE')).toBeVisible();
  await expect(page.getByLabel('Reconstruction instruction')).toHaveValue('');
  await expect(page.getByLabel('Reconstruction instruction')).toHaveAttribute(
    'placeholder',
    'Describe only intended geometry, constraints, symmetry, and uncertain regions.',
  );
});

test('Given collinear frame landmarks When guided CAD validates the frame Then the raw degenerate-frame reason blocks handoff', async ({ page }) => {
  await openPartialInsertCapture(page, 'degenerate-frame');

  await page.getByRole('button', { name: 'GUIDED CAD' }).click();
  await authorMinimumMechanicalGuide(page);
  await page.getByRole('button', { name: 'VALIDATE GUIDE' }).click();

  await expect(page.getByRole('alert').filter({ hasText: 'Frame evidence is degenerate: origin, X, and Y landmarks are collinear.' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' })).toBeDisabled();
});

test('Given insufficient scan evidence When guide mode opens Then validation and BRep handoff stay blocked with exact missing evidence', async ({ page }) => {
  await openPartialInsertCapture(page, 'happy');
  await page.getByRole('button', { name: 'GUIDED CAD' }).click();

  await page.getByText(/EVIDENCE REQUIREMENTS MISSING$/).click();
  await expect(page.getByText('Pick two calibration endpoints.')).toBeVisible();
  await expect(page.getByText('Pick X and XY frame directions.')).toBeVisible();
  await expect(page.getByText('Pick at least three ordered profile vertices.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'VALIDATE GUIDE' })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' })).toBeDisabled();
});

test('Given supported scan evidence When geometry is analyzed Then one deterministic feature plan is selected before handoff', async ({ page }) => {
  await openPartialInsertCapture(page, 'happy');
  await page.getByRole('button', { name: 'GUIDED CAD' }).click();
  await authorMinimumMechanicalGuide(page);
  await page.getByRole('button', { name: 'VALIDATE GUIDE' }).click();

  const plan = page.getByRole('region', { name: 'Deterministic reconstruction plan' });
  await expect(plan.getByText('Observed profile extrude')).toBeVisible();
  await expect(plan.getByText('SUPPORTED')).toBeVisible();
  await expect(page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' })).toBeEnabled();
});

test('Given competing feature plans When geometry is analyzed Then handoff waits for explicit plan selection', async ({ page }) => {
  await openPartialInsertCapture(page, 'ambiguous-plan');
  await page.getByRole('button', { name: 'GUIDED CAD' }).click();
  await authorMinimumMechanicalGuide(page);
  await page.getByRole('button', { name: 'VALIDATE GUIDE' }).click();

  await expect(page.getByText('FEATURE PLAN · AMBIGUOUS')).toBeVisible();
  await expect(page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' })).toBeDisabled();
  await page.getByLabel('Select Observed profile extrude').check();
  await page.getByRole('button', { name: 'VALIDATE GUIDE' }).click();
  await expect(page.getByText('FEATURE PLAN · SATISFIED')).toBeVisible();
  await expect(page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' })).toBeEnabled();
});

test('Given noisy over-tolerance evidence When geometry is analyzed Then raw residual blocks handoff', async ({ page }) => {
  await openPartialInsertCapture(page, 'noisy-fit');
  await page.getByRole('button', { name: 'GUIDED CAD' }).click();
  await authorMinimumMechanicalGuide(page);
  await page.getByRole('button', { name: 'VALIDATE GUIDE' }).click();
  await expect(page.getByRole('alert').filter({ hasText: 'Circle fit maximum residual 1.8 mm exceeds tolerance 0.25 mm.' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' })).toBeDisabled();
});

test('Given unsupported primitive evidence When geometry is analyzed Then rejected hypothesis and affected evidence stay visible', async ({ page }) => {
  await openPartialInsertCapture(page, 'unsupported-primitive');
  await page.getByRole('button', { name: 'GUIDED CAD' }).click();
  await authorMinimumMechanicalGuide(page);
  await page.getByRole('button', { name: 'VALIDATE GUIDE' }).click();
  await expect(page.getByText('Cylinder evidence is degenerate and over tolerance.')).toBeVisible();
  await expect(page.getByText('bore-1').first()).toBeVisible();
  await expect(page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' })).toBeDisabled();
});

test('Given source changed after guide validation When BRep handoff starts Then exact source divergence blocks queueing', async ({ page }) => {
  await openPartialInsertCapture(page, 'source-divergence');
  await page.getByRole('button', { name: 'GUIDED CAD' }).click();
  await authorMinimumMechanicalGuide(page);
  await page.getByRole('button', { name: 'VALIDATE GUIDE' }).click();
  await page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' }).click();

  await expect(page.getByRole('alert').filter({
    hasText: 'Capture target source diverged: expected sha256:target-source, found sha256:changed-source.',
  })).toBeVisible();
});

test('Given guide evidence targets an older mesh digest When validation checks the selected source Then stale guide blocks handoff with exact reason', async ({ page }) => {
  await openPartialInsertCapture(page, 'stale-source');

  await page.getByRole('button', { name: 'GUIDED CAD' }).click();
  await authorMinimumMechanicalGuide(page);
  await page.getByRole('button', { name: 'VALIDATE GUIDE' }).click();

  await expect(page.getByRole('alert').filter({ hasText: 'Guide is stale: selected crop/source mesh digest changed.' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'BUILD CAD FROM GUIDE' })).toBeDisabled();
});

test('Given committed guided BRep When comparison opens Then scan ghost, BRep, observed metrics, and inferred regions stay independently controllable', async ({ page }) => {
  await openPartialInsertCapture(page, 'comparison');
  await page.getByRole('button', { name: 'GUIDED CAD' }).click();

  const comparison = page.getByRole('region', { name: 'Scan and generated BRep comparison' });
  await expect(comparison).toBeVisible();
  await expect(comparison.getByText('OBSERVED REGION ONLY')).toBeVisible();
  await expect(comparison.getByText('1 @ 0.2 mm')).toBeVisible();
  await expect(comparison.getByText('orderedProfileDistance · MAX 0.14 · RMS 0.07 mm')).toBeVisible();
  await expect(comparison.getByText('three mirrored quarters from X/Y symmetry')).toBeVisible();
  const viewer = page.locator('[data-testid="capture-preview-viewport"] .viewer-host');
  await expect(viewer).toHaveAttribute('data-capture-comparison-loaded', 'true');
  await expect(viewer).toHaveAttribute('data-capture-deviation-point-count', '3');

  await page.getByLabel('Show reference scan').uncheck();
  await page.getByLabel('Show generated BRep').uncheck();
  await page.getByLabel('Reference scan opacity').fill('0.42');
  await page.getByLabel('Generated BRep opacity').fill('0.64');
  await page.getByLabel('Show deviation colors').uncheck();
  await expect(viewer).toHaveAttribute('data-capture-reference-visible', 'false');
  await expect(viewer).toHaveAttribute('data-capture-generated-visible', 'false');
  await expect(viewer).toHaveAttribute('data-capture-reference-opacity', '0.42');
  await expect(viewer).toHaveAttribute('data-capture-generated-opacity', '0.64');
  await expect(viewer).toHaveAttribute('data-capture-deviation-visible', 'false');

  const visualBoundary = await page.getByTestId('capture-panel').evaluate((panel) => {
    const guideInput = panel.querySelector('.capture-panel__guide input[type="number"]');
    const comparisonPanel = panel.querySelector('.capture-panel__comparison');
    const panelStyle = getComputedStyle(panel);
    return {
      panelOverflow: panelStyle.overflow,
      comparisonOverflow: comparisonPanel ? getComputedStyle(comparisonPanel).overflow : '',
      inputBorderRadius: guideInput ? getComputedStyle(guideInput).borderRadius : '',
      primary: panelStyle.getPropertyValue('--primary').trim(),
      secondary: panelStyle.getPropertyValue('--secondary').trim(),
    };
  });
  expect(visualBoundary.panelOverflow).toBe('hidden');
  expect(visualBoundary.comparisonOverflow).toBe('hidden');
  expect(visualBoundary.inputBorderRadius).toBe('0px');
  expect(visualBoundary.primary).not.toBe('');
  expect(visualBoundary.secondary).not.toBe('');
});
