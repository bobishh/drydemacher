import { expect, test, type Page } from '@playwright/test';
import {
  partialSymmetricMechanicalInsert,
  partialSymmetricMechanicalInsertStl,
} from './fixtures/partial-symmetric-mechanical-insert.js';

type GuideMockMode = 'happy' | 'degenerate-frame' | 'stale-source' | 'source-divergence' | 'comparison'
  | 'ambiguous-plan' | 'noisy-fit' | 'unsupported-primitive';

async function installGuidedCaptureFixture(page: Page, mode: GuideMockMode) {
  await page.addInitScript(({ fixture, mode }) => {
    const mockWindow = window as typeof window & {
      __CAPTURE_GUIDE_CALLS__?: unknown[];
      __CAPTURE_CANONICAL_GUIDE__?: Record<string, any>;
    };
    mockWindow.__CAPTURE_GUIDE_CALLS__ = [];
    const generatedArtifactBundle = {
      modelId: 'generated-model-1', sourceKind: 'generated', contentHash: 'generated-content', artifactVersion: 1,
      fcstdPath: '', manifestPath: '/fixtures/generated-manifest.json',
      modelStlPath: '/fixtures/generated-brep.stl', viewerAssets: [], exportArtifacts: [],
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
    (window.__TAURI_INTERNALS__ as any).convertFileSrc = (path: string) => path;
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
        modelStlPath: '/fixtures/partial-symmetric-mechanical-insert.stl', viewerAssets: [], exportArtifacts: [],
        geometryBackend: 'mesh', sourceLanguage: 'ecky', engineKind: 'ecky',
      }, modelManifest: {
        schemaVersion: 1, modelId: 'partial-insert-mesh', sourceKind: 'generated', engineKind: 'ecky',
        sourceLanguage: 'ecky', geometryBackend: 'mesh',
        document: { documentName: 'Partial insert', documentLabel: 'Partial insert', sourcePath: '/fixtures/partial-symmetric-mechanical-insert.stl', objectCount: 1, warnings: [] },
        parts: [], parameterGroups: [], controlPrimitives: [], controlRelations: [], controlViews: [],
        advisories: [], selectionTargets: [], measurementAnnotations: [], warnings: [], enrichmentState: { status: 'none', proposals: [] },
      } };
      if (cmd === 'get_capture_reconstruction_guide') return null;
      if (cmd === 'ensure_capture_reconstruction_guide') {
        const created = !mockWindow.__CAPTURE_CANONICAL_GUIDE__;
        if (created) {
          mockWindow.__CAPTURE_CANONICAL_GUIDE__ = {
            schemaVersion: 1, guideId: 'guide-server-1', revision: 1,
            captureRunId: 'partial-insert-capture', targetThreadId: 'thread-owner', targetMessageId: null,
            targetSourceDigest: 'sha256:target-source', targetVersionId: null,
            sourceMesh: {
              artifactDigest: fixture.artifactDigest, contentDigest: fixture.contentDigest,
              selection: 'raw', cropDigest: null, triangleCount: fixture.triangles.length,
              sourceBounds: { min: [0, 0, 0], max: [40, 30, 18] },
            },
            calibration: { sourceUnits: 'sourceUnit', millimetresPerSourceUnit: 1, method: { kind: 'knownDistance' }, measurements: [], residualMm: 0 },
            reconstructionFrame: { originMm: [0, 0, 0], xAxis: [1, 0, 0], yAxis: [0, 1, 0], zAxis: [0, 0, 1], sourceLandmarkIds: [] },
            landmarks: [], evidenceComputationPolicy: { neighborhoodRadiusMm: 2, maxNeighborhoodTriangles: 64 },
            surfaceNeighborhoods: [], primitiveCandidates: [], primitiveHypotheses: [], surfaceRegions: [],
            regionAdjacency: [], reconstructedProfiles: [], authoredConstraints: [],
            constraintGraph: { dimensions: [], relations: [], contentDigest: '' },
            featurePlanCandidates: [], selectedFeaturePlanId: null, stageBypasses: [],
            reconstructionReadiness: { ready: false, stages: [], missingStages: [], ambiguousStages: [], selectedFeaturePlanId: null, detail: '' },
            featureExpectations: [], measurements: [], axes: [], planes: [], profiles: [], ignoredRegions: [],
            remapProposals: [], symmetryCompletion: { kind: 'none' }, instruction: '', evidenceViews: [],
            canonicalDigest: 'sha256:guide-1',
          };
        }
        return { guide: structuredClone(mockWindow.__CAPTURE_CANONICAL_GUIDE__), state: { status: 'draft' }, created };
      }
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
        const guide = JSON.parse(JSON.stringify(args?.guide ?? {})) as Record<string, unknown>;
        const saved = {
          ...guide,
          revision: Number(args?.expectedRevision ?? 0) + 1,
          canonicalDigest: `sha256:guide-${Number(args?.expectedRevision ?? 0) + 1}`,
        };
        mockWindow.__CAPTURE_CANONICAL_GUIDE__ = JSON.parse(JSON.stringify(saved));
        return saved;
      }
      if (cmd === 'apply_capture_guide_edit') {
        const input = args?.input as {
          expectedRevision: number;
          edit: Record<string, any> & { kind: string };
        };
        let guide = JSON.parse(JSON.stringify(mockWindow.__CAPTURE_CANONICAL_GUIDE__ ?? {})) as Record<string, any>;
        const baseRevision = Number(guide.revision ?? 0);
        if (input.edit.kind === 'addLandmark') {
          const ordinal = guide.landmarks.length + 1;
          guide.landmarks.push({
            landmarkId: `landmark-${ordinal}`, label: `${input.edit.role} ${ordinal}`, role: input.edit.role,
            anchor: input.edit.anchor, localPositionMm: input.edit.anchor.sourcePosition,
            localNormal: input.edit.anchor.sourceNormal, uncertaintyMm: null,
          });
        } else if (input.edit.kind === 'updateLandmark') {
          const landmark = guide.landmarks.find((item: any) => item.landmarkId === input.edit.landmarkId);
          if (!landmark) throw new Error(`Capture landmark '${input.edit.landmarkId}' does not exist.`);
          landmark.label = input.edit.label;
          landmark.role = input.edit.role;
        } else if (input.edit.kind === 'deleteLandmark') {
          guide.landmarks = guide.landmarks.filter((item: any) => item.landmarkId !== input.edit.landmarkId);
          for (const profile of guide.profiles) {
            profile.landmarkIds = profile.landmarkIds.filter((id: string) => id !== input.edit.landmarkId);
          }
        } else if (input.edit.kind === 'replaceDraft') {
          guide = JSON.parse(JSON.stringify(input.edit.guide));
        } else if (input.edit.kind === 'configureProfile') {
          const profile = guide.profiles.find((item: any) => item.profileId === input.edit.profileId);
          Object.assign(profile, {
            label: input.edit.label, kind: input.edit.profileKind, operationHint: input.edit.operationHint,
            supportPlaneId: input.edit.supportPlaneId, featureLabel: input.edit.featureLabel, fitRole: input.edit.fitRole,
          });
        } else if (input.edit.kind === 'reorderProfileLandmark') {
          const profile = guide.profiles.find((item: any) => item.profileId === input.edit.profileId);
          const sourceIndex = profile.landmarkIds.indexOf(input.edit.landmarkId);
          const [moved] = profile.landmarkIds.splice(sourceIndex, 1);
          profile.landmarkIds.splice(input.edit.targetIndex, 0, moved);
        } else if (input.edit.kind === 'updateFeatureExpectation') {
          const expectation = guide.featureExpectations.find((item: any) => item.expectationId === input.edit.expectationId);
          Object.assign(expectation, input.edit);
          delete expectation.kind;
        } else if (input.edit.kind === 'selectFeaturePlan') {
          guide.selectedFeaturePlanId = input.edit.planId;
        }
        guide.revision = baseRevision + 1;
        guide.canonicalDigest = `sha256:guide-${guide.revision}`;
        mockWindow.__CAPTURE_CANONICAL_GUIDE__ = JSON.parse(JSON.stringify(guide));
        return {
          guide,
          state: { status: 'draft' },
          baseRevision,
          expectedRevisionMatched: input.expectedRevision === baseRevision,
          sourceDigestMatched: true,
          rawEvidence: [],
        };
      }
      if (cmd === 'validate_capture_guide_intent') {
        const input = args?.input as Record<string, any>;
        const guide = JSON.parse(JSON.stringify(mockWindow.__CAPTURE_CANONICAL_GUIDE__ ?? {})) as Record<string, any>;
        const baseRevision = Number(guide.revision ?? 0);
        const byRole = (role: string) => guide.landmarks.filter((landmark: any) => landmark.role === role);
        const calibration = byRole('calibrationEndpoint').slice(0, 2);
        const frameDirections = byRole('frameDirection').slice(0, 2);
        const frameOrigin = byRole('frameOrigin')[0];
        const symmetrySamples = byRole('symmetrySample');
        const profileVertices = byRole('profileVertex');
        const existingProfile = guide.profiles.find((profile: any) => profile.profileId === 'profile-1');
        const profileIds = profileVertices.map((landmark: any) => landmark.landmarkId);
        const orderedProfileIds = [
          ...(existingProfile?.landmarkIds ?? []).filter((id: string) => profileIds.includes(id)),
          ...profileIds.filter((id: string) => !existingProfile?.landmarkIds.includes(id)),
        ];
        guide.calibration = {
          sourceUnits: 'sourceUnit',
          millimetresPerSourceUnit: guide.calibration.millimetresPerSourceUnit || 1,
          method: { kind: 'knownDistance' },
          measurements: [{
            measurementId: 'calibration-1', label: 'Known calibration distance',
            firstLandmarkId: calibration[0].landmarkId, secondLandmarkId: calibration[1].landmarkId,
            knownDistanceMm: input.knownDistanceMm, fittedDistanceMm: 0, residualMm: 0,
            acceptedToleranceMm: 0.1,
          }],
          residualMm: 0,
        };
        guide.reconstructionFrame.sourceLandmarkIds = [
          frameOrigin.landmarkId,
          frameDirections[0].landmarkId,
          frameDirections[1].landmarkId,
        ];
        guide.planes = [{
          planeId: 'symmetry-plane-1', label: 'Primary symmetry plane', role: 'symmetry',
          landmarkIds: symmetrySamples.map((landmark: any) => landmark.landmarkId),
          originMm: [0, 0, 0], normal: [1, 0, 0],
          fit: { rmsMm: 0, maxMm: 0, toleranceMm: 0.25 },
        }];
        guide.profiles = [{
          profileId: 'profile-1', label: existingProfile?.label ?? 'Ordered reconstruction profile',
          kind: existingProfile?.kind ?? 'closed', supportPlaneId: existingProfile?.supportPlaneId ?? 'symmetry-plane-1',
          landmarkIds: orderedProfileIds, operationHint: existingProfile?.operationHint ?? 'extrude',
          featureLabel: existingProfile?.featureLabel ?? 'insert-body', fitRole: existingProfile?.fitRole ?? 'outer-envelope',
        }];
        const existingExpectation = (id: string) => guide.featureExpectations.find((item: any) => item.expectationId === id);
        guide.featureExpectations = [
          existingExpectation('expectation-symmetry-face') ?? {
            expectationId: 'expectation-symmetry-face', guideItemIds: ['symmetry-plane-1'],
            label: 'Exact symmetry support face', expectedGeometryKind: 'plane', requiredBrepTopologyKind: 'face',
            cardinality: 'one', partId: 'insert-body', instancePath: null,
            expectedAuthoredSelector: { kind: 'tag', name: 'symmetry-face' }, requiredForAcceptance: true,
            positionToleranceMm: 0.25, normalToleranceDeg: 1, radialToleranceMm: null,
          },
          existingExpectation('expectation-profile-edges') ?? {
            expectationId: 'expectation-profile-edges', guideItemIds: ['profile-1'],
            label: 'Exact ordered profile edges', expectedGeometryKind: 'profile', requiredBrepTopologyKind: 'orderedEdges',
            cardinality: 'oneOrMore', partId: 'insert-body', instancePath: null,
            expectedAuthoredSelector: { kind: 'tag', name: 'profile-edges' }, requiredForAcceptance: true,
            positionToleranceMm: 0.25, normalToleranceDeg: null, radialToleranceMm: null,
          },
        ];
        guide.symmetryCompletion = { kind: 'half', planeId: 'symmetry-plane-1' };
        guide.measurements = [{
          measurementId: 'feature-depth', label: 'Feature depth', landmarkIds: [], value: input.featureDepthMm,
          unit: 'mm', fitCritical: true, authoredParameterName: 'feature-depth', constraintKind: 'extent',
        }];
        guide.instruction = String(input.instruction ?? '').trim();
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
        const evaluated: Record<string, any> = {
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
        const rawEvidence = mode === 'noisy-fit'
          ? ['Circle fit maximum residual 1.8 mm exceeds tolerance 0.25 mm.']
          : mode === 'degenerate-frame'
            ? ['Frame evidence is degenerate: origin, X, and Y landmarks are collinear.']
            : [];
        if (rawEvidence.length > 0) {
          evaluated.reconstructionReadiness.ready = false;
          evaluated.reconstructionReadiness.detail = rawEvidence[0];
        }
        evaluated.revision = baseRevision + 1;
        evaluated.canonicalDigest = `sha256:guide-${evaluated.revision}`;
        mockWindow.__CAPTURE_CANONICAL_GUIDE__ = JSON.parse(JSON.stringify(evaluated));
        const state = mode === 'stale-source'
          ? { status: 'stale', reason: 'Guide is stale: selected crop/source mesh digest changed.' }
          : evaluated.reconstructionReadiness.ready && rawEvidence.length === 0
            ? { status: 'ready' }
            : { status: 'draft' };
        return {
          guide: evaluated,
          state,
          baseRevision,
          expectedRevisionMatched: input.expectedRevision === baseRevision,
          sourceDigestMatched: mode !== 'stale-source',
          rawEvidence: mode === 'stale-source' ? [state.reason] : rawEvidence,
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
  await page.route('**/fixtures/model.stl*', route => route.fulfill({
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
  await expect.poll(async () => page.evaluate(() => (
    window as typeof window & { __CAPTURE_GUIDE_CALLS__?: Array<{ cmd: string; args?: any }> }
  ).__CAPTURE_GUIDE_CALLS__?.filter(call =>
    call.cmd === 'apply_capture_guide_edit' &&
    ['updateLandmark', 'deleteLandmark'].includes(call.args?.input?.edit?.kind)
  ).length ?? 0)).toBe(2);
  await page.getByRole('button', { name: 'UNDO GUIDE EDIT' }).click();
  await expect.poll(async () => page.evaluate(() => (
    window as typeof window & { __CAPTURE_GUIDE_CALLS__?: Array<{ cmd: string; args?: any }> }
  ).__CAPTURE_GUIDE_CALLS__?.filter(call =>
    call.cmd === 'apply_capture_guide_edit' && call.args?.input?.edit?.kind === 'replaceDraft'
  ).length ?? 0)).toBe(1);
  expect(await page.evaluate(() => {
    const calls = (window as typeof window & { __CAPTURE_GUIDE_CALLS__?: Array<{ cmd: string; args?: any }> }).__CAPTURE_GUIDE_CALLS__ ?? [];
    const undo = calls.find(call => call.cmd === 'apply_capture_guide_edit' && call.args?.input?.edit?.kind === 'replaceDraft');
    return undo?.args?.input?.edit?.guide?.landmarks?.length ?? -1;
  })).toBe(11);
  await expect(page.locator('.capture-panel__guide-point')).toHaveCount(11);
  await expect(page.getByLabel('Landmark 1 label')).toHaveValue('calibration datum A');
  await page.getByRole('button', { name: 'VALIDATE GUIDE' }).click();
  await expect.poll(async () => page.evaluate(() => (
    window as typeof window & { __CAPTURE_GUIDE_CALLS__?: Array<{ cmd: string }> }
  ).__CAPTURE_GUIDE_CALLS__?.filter(
    call => call.cmd === 'validate_capture_guide_intent',
  ).length ?? 0)).toBe(1);

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
    return (window as typeof window & {
      __CAPTURE_CANONICAL_GUIDE__?: Record<string, unknown>;
    }).__CAPTURE_CANONICAL_GUIDE__ as {
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
