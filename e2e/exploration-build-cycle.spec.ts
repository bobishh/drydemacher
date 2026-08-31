import { expect, test, type Page } from '@playwright/test';

const THREAD_ID = 'exploration-thread';
const VERSION_A = 'version-a';
const VERSION_B = 'version-b-red';
const VERSION_C = 'version-c-green';

const runtimeCapabilities = {
  freecad: { available: true, detail: 'Ready', path: '/mock/freecadcmd' },
  build123d: { available: true, detail: 'Ready', path: '/mock/python3' },
  mesh: { available: true, detail: 'bundled', path: null },
  recommendedAuthoringContext: {
    engineKind: 'build123d',
    sourceLanguage: 'build123d',
    geometryBackend: 'build123d',
  },
};

const baseDesign = {
  title: 'Wall Bracket',
  versionName: 'A · Good wall',
  response: 'Good wall rendered.',
  interactionMode: 'design',
  macroCode: 'from build123d import *\nBox(20, 20, 4)',
  macroDialect: 'build123d',
  sourceLanguage: 'build123d',
  geometryBackend: 'build123d',
  uiSpec: { fields: [] },
  initialParams: {},
  postProcessing: null,
};

const redDesign = {
  ...baseDesign,
  versionName: 'B · Red wall draft',
  response: 'Wall draft failed validation.',
  macroCode: 'from build123d import *\nBox(20, 20,',
};

const greenDesign = {
  ...baseDesign,
  versionName: 'C · Repaired wall',
  response: 'Wall repaired.',
  macroCode: 'from build123d import *\nBox(20, 20, 8)',
};

function artifact(version: string) {
  return {
    modelId: `model-${version}`,
    sourceKind: 'generated',
    sourceLanguage: 'build123d',
    geometryBackend: 'build123d',
    contentHash: `hash-${version}`,
    artifactVersion: 1,
    fcstdPath: `/mock/${version}.FCStd`,
    manifestPath: `/mock/${version}.json`,
    modelStlPath: `/mock/${version}.stl`,
    viewerAssets: [],
    exportArtifacts: [],
  };
}

function manifest(version: string) {
  return {
    modelId: `model-${version}`,
    sourceKind: 'generated',
    sourceLanguage: 'build123d',
    geometryBackend: 'build123d',
    document: { documentName: 'Wall Bracket', documentLabel: 'Wall Bracket', objectCount: 1, warnings: [] },
    parts: [],
    parameterGroups: [],
    selectionTargets: [],
    warnings: [],
    enrichmentState: { status: 'none', proposals: [] },
  };
}

function versionMessage(id: string, status: 'success' | 'error', design: typeof baseDesign) {
  const isGood = status === 'success';
  return {
    id,
    role: 'assistant',
    content: isGood ? design.response : 'PARSE_UNEXPECTED_EOF: source ended before expression closed',
    status,
    timestamp: 1,
    output: design,
    artifactBundle: isGood ? artifact(id) : null,
    modelManifest: isGood ? manifest(id) : null,
    usage: null,
  };
}

type ExplorationMockOptions = {
  cyclePacket?: unknown;
};

async function installExplorationMocks(page: Page, options: ExplorationMockOptions = {}) {
  await page.route(/\/mock\/.*\.stl(?:\?.*)?$/, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'model/stl',
      body: 'solid mock\nendsolid mock\n',
    });
  });

  await page.addInitScript(({ initialThread, runtimeCapabilities, versionA, versionB, versionC, baseDesignInput, redDesignInput, greenDesignInput, cyclePacket }) => {
    const w = window as any;
    const events = new Map<string, number[]>();
    let nextCallbackId = 1;
    let thread: any = structuredClone(initialThread);
    let versionCalls = 0;
    const artifactFor = (version: string) => ({
      modelId: `model-${version}`, sourceKind: 'generated', sourceLanguage: 'build123d', geometryBackend: 'build123d',
      contentHash: `hash-${version}`, artifactVersion: 1, fcstdPath: `/mock/${version}.FCStd`,
      manifestPath: `/mock/${version}.json`, modelStlPath: `/mock/${version}.stl`, viewerAssets: [], exportArtifacts: [],
    });
    const manifestFor = (version: string) => ({
      modelId: `model-${version}`, sourceKind: 'generated', sourceLanguage: 'build123d', geometryBackend: 'build123d',
      document: { documentName: 'Wall Bracket', documentLabel: 'Wall Bracket', objectCount: 1, warnings: [] },
      parts: [], parameterGroups: [], selectionTargets: [], warnings: [], enrichmentState: { status: 'none', proposals: [] },
    });

    const emitProgress = (requestId: string, phase: string, attempt: number, runningBuilds: number, pendingBuilds: number, summary: string, currentVersionId: string | null = null) => {
      w.__emitTauriEvent('exploration-run-progress', {
        requestId,
        threadId: thread.id,
        phase,
        attempt,
        maxAttempts: 2,
        runningBuilds,
        pendingBuilds,
        currentVersionId,
        summary,
        rawError: null,
      });
    };

    const appendVersion = (id: string, status: 'success' | 'error', design: any) => {
      if (thread.messages.some((message: any) => message.id === id)) return id;
      const isGood = status === 'success';
      thread.messages.push({
        id,
        role: 'assistant',
        content: isGood ? design.response : 'PARSE_UNEXPECTED_EOF: source ended before expression closed',
        status,
        timestamp: Date.now(),
        output: structuredClone(design),
        usage: null,
        artifactBundle: isGood ? artifactFor(id) : null,
        modelManifest: isGood ? manifestFor(id) : null,
        errorMessage: isGood ? null : 'PARSE_UNEXPECTED_EOF: source ended before expression closed',
      });
      thread.versionCount = thread.messages.length;
      thread.pendingCount = 0;
      thread.queuedCount = 0;
      thread.errorCount = thread.messages.filter((message: any) => message.status === 'error').length;
      versionCalls += 1;
      w.__EXPLORATION_CALLS__.push({ cmd: 'version-appended', id, status });
      w.__emitTauriEvent('history-updated', { threadId: thread.id, messageId: id });
      return id;
    };

    w.__EXPLORATION_CALLS__ = [];
    w.__emitTauriEvent = (event: string, payload: unknown) => {
      for (const callbackId of events.get(event) ?? []) {
        const callback = w[`_${callbackId}`];
        if (typeof callback === 'function') callback({ event, id: callbackId, payload });
      }
    };
    w.__TAURI_INTERNALS__ = w.__TAURI_INTERNALS__ || {};
    w.__TAURI_INTERNALS__.transformCallback = (callback: unknown) => {
      const callbackId = nextCallbackId++;
      w[`_${callbackId}`] = callback;
      return callbackId;
    };

    const currentVersion = () => thread.messages.at(-1) ?? null;

    w.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: Record<string, unknown>) => {
      w.__EXPLORATION_CALLS__.push({ cmd, args: structuredClone(args ?? {}) });
      if (cmd === 'plugin:event|listen') {
        const event = String(args?.event ?? '');
        const callbackId = Number(args?.handler ?? 0);
        events.set(event, [...(events.get(event) ?? []), callbackId]);
        return callbackId;
      }
      if (cmd === 'plugin:event|unlisten') return null;
      if (cmd === 'get_config') {
        return {
          engines: [{ id: 'api-main', name: 'API Main', provider: 'openai', apiKey: 'sk-live', model: 'gpt-4.1', lightModel: 'gpt-4.1-mini', enabled: true }],
          selectedEngineId: 'api-main',
          freecadCmd: '', assets: [], microwave: { muted: true }, voice: { sttLanguageCode: 'en-US' },
          mcp: { mode: 'passive', autoAgents: [] }, hasSeenOnboarding: true, connectionType: 'api_key',
          defaultEngineKind: 'build123d', defaultSourceLanguage: 'build123d', defaultGeometryBackend: 'build123d',
          maxGenerationAttempts: 1, maxVerifyAttempts: 0,
        };
      }
      if (cmd === 'save_config') return null;
      if (cmd === 'get_runtime_capabilities') return structuredClone(runtimeCapabilities);
      if (cmd === 'get_active_exploration_cycle') return structuredClone(cyclePacket ?? null);
      if (cmd === 'get_history') return [structuredClone(thread)];
      if (cmd === 'get_thread') return structuredClone(thread);
      if (cmd === 'get_thread_messages_page') return { messages: structuredClone(thread.messages), hasMore: false, nextBefore: null };
      if (cmd === 'get_thread_latest_version') return structuredClone(currentVersion());
      if (cmd === 'get_thread_head_version_id') return currentVersion()?.id ?? null;
      if (cmd === 'get_thread_message_version') return structuredClone(thread.messages.find((message: any) => message.id === args?.messageId) ?? null);
      if (cmd === 'get_last_design') {
        return {
          threadId: thread.id,
          messageId: versionA,
          design: baseDesignInput,
          artifactBundle: artifactFor(versionA),
          modelManifest: manifestFor(versionA),
          selectedPartId: null,
        };
      }
      if (cmd === 'get_mess_stl_path') return '/mock/mess.stl';
      if (cmd === 'get_active_agent_sessions' || cmd === 'get_agent_terminal_snapshots' || cmd === 'get_mcp_server_status') return [];
      if (cmd === 'get_agent_activity') {
        return { events: [], latestCursor: 0, oldestCursor: 0, hasMore: false, droppedCount: 0, retainedBytes: 0 };
      }
      if (cmd === 'get_message_attachments') return [];
      if (cmd === 'list_installed_component_package_headers') return [];
      if (cmd === 'start_exploration_run') {
        const input = (args as any)?.input ?? args ?? {};
        const requestId = String(input.requestId ?? 'request-from-ui');
        emitProgress(requestId, 'planning', 0, 1, 0, 'Planning one bounded authoring step.');
        emitProgress(requestId, 'building', 1, 1, 0, 'Building red exploratory draft.', versionA);
        appendVersion(versionB, 'error', redDesignInput);
        emitProgress(requestId, 'verifying', 1, 1, 0, 'Verifying red draft before repair.', versionB);
        // Keep the red pending state observable long enough for the browser
        // assertion to prove last-good viewport projection before repair C.
        await new Promise((resolve) => setTimeout(resolve, 1500));
        appendVersion(versionC, 'success', greenDesignInput);
        emitProgress(requestId, 'deciding', 1, 1, 0, 'Choosing repaired version.', versionC);
        emitProgress(requestId, 'completed', 1, 0, 0, 'Exploration complete.', versionC);
        return {
          run: {
            requestId,
            threadId: thread.id,
            cycleId: 'cycle-1',
            phase: 'completed',
            messageId: versionC,
            design: structuredClone(greenDesignInput),
            artifactBundle: artifactFor(versionC),
            modelManifest: manifestFor(versionC),
            structuralVerification: { passed: true, summary: 'Structural checks passed.', issues: [], metrics: { partCount: 1 }, verifierStatus: 'ok', verifierSource: 'rustStructural' },
            usage: null,
            responseText: 'Wall repaired.',
            rawError: null,
            publicationAllowed: true,
          },
          message: structuredClone(thread.messages.find((message: any) => message.id === versionC)),
          snapshotId: `snapshot-${versionC}`,
        };
      }
      return null;
    };
    w.__EXPLORATION_VERSION_CALLS__ = () => versionCalls;
  }, {
    initialThread: {
      id: THREAD_ID,
      title: 'Wall Bracket Exploration',
      summary: 'Immutable exploration fixture',
      updatedAt: 1,
      versionCount: 1,
      pendingCount: 0,
      queuedCount: 0,
      errorCount: 0,
      status: 'active',
      finalizedAt: null,
      pendingConfirm: null,
      engineKind: 'build123d',
      sourceLanguage: 'build123d',
      geometryBackend: 'build123d',
      messages: [versionMessage(VERSION_A, 'success', baseDesign)],
    },
    runtimeCapabilities,
    versionA: VERSION_A,
    versionB: VERSION_B,
    versionC: VERSION_C,
    baseDesignInput: baseDesign,
    redDesignInput: redDesign,
    greenDesignInput: greenDesign,
    cyclePacket: options.cyclePacket ?? null,
  });
}

async function openExplorationThread(page: Page) {
  await page.goto('/');
  await expect(page.locator('.boot-overlay')).toHaveCount(0, { timeout: 15000 });
  await page.getByRole('button', { name: 'PROJECTS' }).click();
  await page.locator(`[data-project-id="${THREAD_ID}"]`).getByRole('button', { name: 'OPEN' }).click();
  await page.getByRole('button', { name: 'DIALOGUE' }).click();
  await expect(page.getByRole('region', { name: 'Prompt panel' })).toBeVisible();
  await expect(page.locator('.trail-version-event').filter({ hasText: 'Good wall rendered.' })).toBeVisible();
}

test.describe('exploration build cycle', () => {
  test('Given a good version When one exploration run repairs red B to green C Then A/B/C remain immutable and C is the head', async ({ page }) => {
    await installExplorationMocks(page);
    await openExplorationThread(page);

    const viewport = page.locator('.viewport-area .viewer-shell');
    const goodViewportKey = await viewport.getAttribute('data-model-key');
    expect(goodViewportKey).toContain(`model-${VERSION_A}`);

    const promptInput = page.getByPlaceholder(/Type a question or design change/i);
    await promptInput.fill('Make the wall thicker, preserving the mounting face.');
    await promptInput.press('Meta+Enter');

    // Red BUILD persists B and makes it head. Viewport remains A, last known good.
    await expect(page.locator('.trail-version-event').filter({ hasText: 'B · Red wall draft' })).toBeVisible({ timeout: 15000 });
    const redVersion = page.locator('.trail-version-event').filter({ hasText: 'B · Red wall draft' });
    await expect(redVersion).toHaveClass(/trail-error/);
    await expect(redVersion).toContainText('PARSE_UNEXPECTED_EOF');
    await expect(page.locator('.trail-version-event').filter({ hasText: 'Good wall rendered.' })).toHaveCount(1);
    await expect(page.locator('.trail-active-version').filter({ hasText: 'B · Red wall draft' })).toBeVisible();
    await expect(viewport).toHaveAttribute('data-model-key', goodViewportKey!);

    // The same backend run appends C after B; frontend does not retry or append a lifecycle row.
    await expect(page.locator('.trail-version-event').filter({ hasText: 'C · Repaired wall' })).toBeVisible({ timeout: 15000 });
    await expect(page.locator('.trail-version-event')).toHaveCount(3);
    await expect(page.locator('.trail-version-event').filter({ hasText: 'C · Repaired wall' })).toHaveClass(/trail-active-version/);
    await expect(viewport).toHaveAttribute('data-model-key', new RegExp(`model-${VERSION_C}`));

    const calls = await page.evaluate(() => (window as any).__EXPLORATION_CALLS__);
    expect(calls.filter((call: any) => call.cmd === 'start_exploration_run')).toHaveLength(1);
    expect(calls.filter((call: any) => call.cmd === 'version-appended').map((call: any) => call.id)).toEqual([VERSION_B, VERSION_C]);
    expect(await page.evaluate(() => (window as any).__EXPLORATION_VERSION_CALLS__())).toBe(2);
    const oldLifecycleCommands = [
      'classify_intent',
      'generate_design',
      'init_generation_attempt',
      'persist_generation_draft',
      'render_model',
      'persist_structural_verification',
      'verify_generated_model',
      'verify_render',
      'finalize_generation_attempt',
    ];
    expect(calls.filter((call: any) => oldLifecycleCommands.includes(call.cmd))).toEqual([]);
    await expect(page.locator('.trail-version-event').filter({ hasText: 'D ·' })).toHaveCount(0);
  });

  test('Given an exploration projection When rendered in the desktop app Then Tactical Midnight styling and bounded layout remain intact', async ({ page }) => {
    await installExplorationMocks(page, {
      cyclePacket: {
        state: {
          cycleId: 'cycle-1',
          threadId: THREAD_ID,
          phase: 'planning',
          status: 'active',
          currentVersionId: VERSION_A,
          chosenVersionId: null,
          pendingQuestion: null,
          lastEvidenceRef: 'evidence-a',
          budget: 3,
          budgetUsed: 1,
        },
        definition: {
          objective: 'Keep mounting face fixed while thickening wall.',
          acceptanceCriteria: ['wall remains printable'],
          hardConstraints: ['mounting face stays fixed'],
          softPreferences: [],
        },
        baseVersionId: VERSION_A,
        hypothesis: 'Increase wall thickness without moving mounting face.',
        promptVersion: 'exploration-v1',
      },
    });
    await page.setViewportSize({ width: 1280, height: 720 });
    await openExplorationThread(page);

    const projection = page.locator('.ecky-cycle-bubble');
    await expect(projection).toBeVisible();
    await expect(projection).toContainText('PLANNING');
    await expect(projection).toContainText('HYPOTHESIS');

    const desktopFacts = await page.evaluate(() => {
      const root = getComputedStyle(document.documentElement);
      const app = document.querySelector<HTMLElement>('.app-page');
      const normalizeColor = (value: string) => {
        const probe = document.createElement('span');
        probe.style.color = value;
        document.body.append(probe);
        const normalized = getComputedStyle(probe).color;
        probe.remove();
        return normalized;
      };
      const selectors = [
        '.app-page',
        '.app-container',
        '.workbench',
        '.main-workbench',
        '.viewport-area',
        '.viewer-shell',
        '[data-testid="workbench-bottom-dock"]',
        '[data-window-id="dialogue"]',
        '.dialogue-content',
        '.prompt-container',
        '.ecky-cycle-bubble',
      ];
      return {
        tokens: {
          bg: root.getPropertyValue('--bg').trim(),
          bg100: root.getPropertyValue('--bg-100').trim(),
          primary: root.getPropertyValue('--primary').trim(),
          secondary: root.getPropertyValue('--secondary').trim(),
        },
        accent: getComputedStyle(document.querySelector('.trail-role')!).color,
        secondary: normalizeColor(root.getPropertyValue('--secondary')),
        radii: selectors.map((selector) => [selector, getComputedStyle(document.querySelector(selector)!).borderRadius]),
        overflows: selectors.map((selector) => [selector, getComputedStyle(document.querySelector(selector)!).overflow]),
        agentStatusBars: document.querySelectorAll('.agent-status-bar, [data-testid="agent-status-bar"]').length,
        horizontalOverflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) > window.innerWidth,
        appRect: app?.getBoundingClientRect().toJSON(),
        viewport: { width: window.innerWidth, height: window.innerHeight },
      };
    });
    expect(desktopFacts.tokens).toEqual({ bg: '#1a1a2e', bg100: '#16213e', primary: '#4a8c5c', secondary: '#c8a620' });
    expect(desktopFacts.accent).toBe(desktopFacts.secondary);
    expect(desktopFacts.agentStatusBars).toBe(0);
    expect(desktopFacts.horizontalOverflow).toBe(false);
    expect(desktopFacts.radii.every(([, radius]) => radius === '0px')).toBe(true);
    expect(desktopFacts.overflows.every(([, overflow]) => overflow === 'hidden')).toBe(true);
    expect(desktopFacts.appRect).toMatchObject({ x: 0, y: 0, width: 1280, height: 720 });

  });
});
