import { expect, test } from '@playwright/test';

const TEXT_ONLY_DRAW_REASON =
  'Selected model is text-only. Image attachments, concept-preview reuse, screenshot capture, and drawing annotations are unavailable.';

async function installTextOnlyDrawMock(page: import('@playwright/test').Page) {
  await page.addInitScript(() => {
    const config = {
      engines: [{
        id: 'text-only',
        name: 'Text-only test engine',
        provider: 'openai',
        apiKey: 'test-key',
        model: 'meta/llama-3.1-70b-instruct',
        lightModel: 'meta/llama-3.1-70b-instruct',
        baseUrl: 'https://integrate.api.nvidia.com/v1',
        enabled: true,
      }],
      selectedEngineId: 'text-only',
      hasSeenOnboarding: true,
      freecadCmd: '',
      assets: [],
      microwave: null,
      mcp: {
        port: null,
        maxSessions: null,
        mode: 'passive',
        primaryAgentId: null,
        promptTimeoutSecs: 1800,
        autoAgents: [],
      },
      connectionType: 'api_key',
      defaultEngineKind: 'build123d',
      defaultSourceLanguage: 'ecky',
      defaultGeometryBackend: 'build123d',
      maxGenerationAttempts: 3,
      maxVerifyAttempts: 1,
    };

    window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
    window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
      if (cmd === 'get_config') return config;
      if (cmd === 'save_config') return null;
      if (cmd === 'get_runtime_capabilities') {
        return {
          freecad: { available: false, detail: 'missing', path: null },
          build123d: { available: true, detail: 'Ready', path: '/mock/python3' },
          mesh: { available: true, detail: 'bundled', path: null },
          recommendedAuthoringContext: {
            engineKind: 'build123d',
            sourceLanguage: 'build123d',
            geometryBackend: 'build123d',
          },
        };
      }
      if (cmd === 'get_history') return [];
      if (cmd === 'get_last_design') return null;
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
      if (cmd === 'check_freecad') return false;
      if (cmd === 'get_mess_stl_path') return '/mock/mess.stl';
      return null;
    };
  });
}

test.describe('Workbench control dock toolbar', () => {
  test('Given the workbench loads When keyboard enters the dock Then one named toolbar exposes ordered pressed controls', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.boot-overlay')).toHaveCount(0);

    const toolbar = page.getByRole('toolbar', { name: 'Workbench tools' });
    await expect(toolbar).toHaveAttribute('aria-orientation', 'horizontal');
    await expect(toolbar).toHaveCSS('overflow', 'hidden');

    expect(await toolbar.locator('button').evaluateAll((buttons) => buttons.map((button) => button.dataset.dockId))).toEqual([
      'projects',
      'params',
      'dialogue',
      'code',
      'docs',
      'library',
      'draw',
      'settings',
    ]);

    const names = [
      'Projects',
      'Parameters',
      'Dialogue',
      'Code inspector',
      'Ecky IR docs',
      'Reusable component library',
      'Draw annotations',
      'Settings',
    ];
    for (const name of names) {
      const control = toolbar.getByRole('button', { name, exact: true });
      await expect(control).toHaveCount(1);
      await expect(control).toBeVisible();
    }
    await expect(toolbar.getByRole('button', { name: 'Sketch Workspace', exact: true })).toHaveCount(0);
    await expect(toolbar.locator('.dock-state-marker')).toHaveCount(0);

    const projects = toolbar.getByRole('button', { name: 'Projects', exact: true });
    await expect(projects).toHaveCSS('border-radius', '0px');
    await expect(projects).toHaveAttribute('aria-pressed', 'false');
    await projects.click();
    await expect(projects).toHaveAttribute('aria-pressed', 'true');
    await expect(projects).not.toHaveCSS('box-shadow', /inset/);

    await expect(toolbar.locator('button[tabindex="0"]')).toHaveCount(1);
    await projects.focus();
    await projects.press('ArrowRight');
    await expect(toolbar.getByRole('button', { name: 'Parameters', exact: true })).toBeFocused();
    await page.keyboard.press('End');
    await expect(toolbar.getByRole('button', { name: 'Settings', exact: true })).toBeFocused();
    await page.keyboard.press('ArrowRight');
    await expect(projects).toBeFocused();
    await page.keyboard.press('Home');
    await expect(projects).toBeFocused();
  });

  test('Given the workbench loads Then detached Sketch stays absent from the dock', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.boot-overlay')).toHaveCount(0);

    const sketch = page
      .getByRole('toolbar', { name: 'Workbench tools' })
      .getByRole('button', { name: 'Sketch Workspace', exact: true });
    await expect(sketch).toHaveCount(0);
    await expect(page.locator('#dock-description-sketch')).toHaveCount(0);
    await expect(page.locator('[data-window-id="sketch"]')).toHaveCount(0);
  });

  test('Given a hidden then background window When its launcher repeats Then it opens, focuses, and only closes while focused', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.boot-overlay')).toHaveCount(0);

    const toolbar = page.getByRole('toolbar', { name: 'Workbench tools' });
    const projectsButton = toolbar.getByRole('button', { name: 'Projects', exact: true });
    const dialogueButton = toolbar.getByRole('button', { name: 'Dialogue', exact: true });
    const projectsWindow = page.locator('[data-window-id="projects"]');
    const dialogueWindow = page.locator('[data-window-id="dialogue"]');

    await projectsButton.click();
    await expect(projectsWindow).toBeVisible();
    await expect(projectsWindow).toHaveClass(/window--focused/);
    await expect(projectsButton).toHaveAttribute('aria-pressed', 'true');

    await dialogueButton.click();
    await expect(dialogueWindow).toBeVisible();
    await expect(dialogueWindow).toHaveClass(/window--focused/);
    await expect(projectsWindow).toBeVisible();
    await expect(projectsWindow).not.toHaveClass(/window--focused/);

    await projectsButton.click();
    await expect(projectsWindow).toBeVisible();
    await expect(projectsWindow).toHaveClass(/window--focused/);
    await expect(dialogueWindow).toBeVisible();
    await expect(projectsButton).toHaveAttribute('aria-pressed', 'true');

    await projectsButton.click();
    await expect(projectsWindow).toBeHidden();
    await expect(projectsButton).toHaveAttribute('aria-pressed', 'false');
    await expect(dialogueWindow).toHaveClass(/window--focused/);
  });

  test('Given Dialogue opens at default geometry When composer renders Then its actions stay above the dock', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto('/');
    await expect(page.locator('.boot-overlay')).toHaveCount(0);

    const toolbar = page.getByRole('toolbar', { name: 'Workbench tools' });
    await toolbar.getByRole('button', { name: 'Dialogue', exact: true }).click();
    const dialogueWindow = page.locator('[data-window-id="dialogue"]');
    const actions = dialogueWindow.locator('.prompt-actions');
    await expect(actions).toBeVisible();

    const dockBox = await toolbar.boundingBox();
    const windowBox = await dialogueWindow.boundingBox();
    const actionsBox = await actions.boundingBox();
    expect(dockBox).not.toBeNull();
    expect(windowBox).not.toBeNull();
    expect(actionsBox).not.toBeNull();
    expect(windowBox!.y + windowBox!.height).toBeLessThanOrEqual(dockBox!.y - 8);
    expect(actionsBox!.y + actionsBox!.height).toBeLessThanOrEqual(dockBox!.y - 8);
  });

  test('Given a compact workbench When focus moves through the dock Then tooltip and Dialogue remain contained', async ({ page }) => {
    await page.setViewportSize({ width: 640, height: 480 });
    await page.goto('/');
    await expect(page.locator('.boot-overlay')).toHaveCount(0);

    const toolbar = page.getByRole('toolbar', { name: 'Workbench tools' });
    await expect(toolbar.locator('.dock-group--primary')).toHaveCSS('overflow', 'hidden');
    await expect(toolbar.locator('.dock-group--utility')).toHaveCSS('overflow', 'hidden');
    const code = toolbar.getByRole('button', { name: 'Code inspector', exact: true });
    await code.focus();
    const tooltip = page.getByRole('tooltip');
    await expect(tooltip).toHaveText('Code inspector');
    expect(await tooltip.evaluate((node) => node.closest('[role="toolbar"]') === null)).toBe(true);
    await expect(page.getByTestId('dock-tooltip-layer')).toHaveCSS('overflow', 'hidden');

    const tooltipBox = await tooltip.boundingBox();
    expect(tooltipBox).not.toBeNull();
    expect(tooltipBox!.x).toBeGreaterThanOrEqual(0);
    expect(tooltipBox!.x + tooltipBox!.width).toBeLessThanOrEqual(640);

    await toolbar.getByRole('button', { name: 'Dialogue', exact: true }).click();
    const dialogueWindow = page.locator('[data-window-id="dialogue"]');
    const actions = dialogueWindow.locator('.prompt-actions');
    await expect(actions).toBeVisible();
    const dockBox = await toolbar.boundingBox();
    const windowBox = await dialogueWindow.boundingBox();
    expect(dockBox).not.toBeNull();
    expect(windowBox).not.toBeNull();
    expect(windowBox!.y + windowBox!.height).toBeLessThanOrEqual(dockBox!.y - 8);
  });

  test('Given Draw is unavailable for a text-only model When keyboard activates it Then raw reason stays focusable and no action runs', async ({ page }) => {
    await installTextOnlyDrawMock(page);
    await page.goto('/');
    await expect(page.locator('.boot-overlay')).toHaveCount(0);

    const draw = page
      .getByRole('toolbar', { name: 'Workbench tools' })
      .getByRole('button', { name: 'Draw annotations', exact: true });
    await expect(draw).toHaveAttribute('aria-disabled', 'true');
    await expect(draw).not.toHaveAttribute('disabled');
    await expect(draw).toHaveAttribute('data-state', 'disabled');
    await draw.focus();
    await expect(draw).toBeFocused();
    await expect(page.getByRole('tooltip')).toHaveText(TEXT_ONLY_DRAW_REASON);
    await expect(draw.locator('.dock-state-marker')).toHaveCount(0);

    await draw.press('Enter');
    await draw.press('Space');
    await expect(draw).toHaveAttribute('aria-pressed', 'false');
    await expect(page.locator('.draw-toolbar')).toHaveCount(0);
  });

  test('Given compact width When every tool is identified and Projects creates Then controls stay contained and + NEW remains global', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 560 });
    await page.goto('/');
    await expect(page.locator('.boot-overlay')).toHaveCount(0);

    const toolbar = page.getByRole('toolbar', { name: 'Workbench tools' });
    const identities = [
      { name: 'Projects', label: 'PROJ', tooltip: 'Projects' },
      { name: 'Parameters', label: 'PARAM', tooltip: 'Parameters' },
      { name: 'Dialogue', label: 'TALK', tooltip: 'Dialogue' },
      { name: 'Code inspector', label: 'CODE', tooltip: 'Code inspector' },
      { name: 'Ecky IR docs', label: 'DOCS', tooltip: 'Ecky IR docs' },
      { name: 'Reusable component library', label: 'LIB', tooltip: 'Reusable component library' },
      { name: 'Draw annotations', label: 'DRAW', tooltip: 'Draw annotations' },
      { name: 'Settings', label: 'SET', tooltip: 'Settings' },
    ];

    const toolbarBox = await toolbar.boundingBox();
    expect(toolbarBox).not.toBeNull();
    expect(toolbarBox!.x).toBeGreaterThanOrEqual(0);
    expect(toolbarBox!.x + toolbarBox!.width).toBeLessThanOrEqual(390);

    for (const identity of identities) {
      const control = toolbar.getByRole('button', { name: identity.name, exact: true });
      const label = control.locator('.dock-label');
      await expect(control.locator('.dock-icon')).toBeVisible();
      await expect(label).toHaveText(identity.label);
      await expect(label).toBeHidden();

      const controlBox = await control.boundingBox();
      expect(controlBox).not.toBeNull();
      expect(controlBox!.x).toBeGreaterThanOrEqual(0);
      expect(controlBox!.x + controlBox!.width).toBeLessThanOrEqual(390);

      await control.focus();
      const tooltip = page.getByRole('tooltip');
      await expect(tooltip).toHaveText(identity.tooltip);
      const tooltipBox = await tooltip.boundingBox();
      expect(tooltipBox).not.toBeNull();
      expect(tooltipBox!.x).toBeGreaterThanOrEqual(0);
      expect(tooltipBox!.x + tooltipBox!.width).toBeLessThanOrEqual(390);
    }

    await toolbar.getByRole('button', { name: 'Projects', exact: true }).click();
    const projectsWindow = page.locator('[data-window-id="projects"]');
    await expect(projectsWindow).toBeVisible();
    await projectsWindow.getByRole('button', { name: '+ NEW', exact: true }).click();
    const chooser = page.locator('.modal-backdrop').filter({ hasText: 'Start New Project' });
    await expect(chooser).toBeVisible();
    await expect(projectsWindow.getByText('Start New Project')).toHaveCount(0);
  });
});
