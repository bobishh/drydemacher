import { test, expect } from '@playwright/test';

function installProjectSwitcherMocks(options?: {
  history?: Array<Record<string, unknown>>;
  inventory?: Array<Record<string, unknown>>;
  inventoryError?: { message: string; details?: string };
  deletedProjectPages?: Record<string, {
    items: Array<Record<string, unknown>>;
    nextBefore: string | null;
    hasMore: boolean;
  }>;
  latestVersions?: Record<string, Record<string, unknown> | null>;
  messagePages?: Record<string, Array<Record<string, unknown>>>;
  latestVersionDelayMs?: number;
}) {
  const history = options?.history ?? [];
  const inventory = options?.inventory ?? [];
  const inventoryError = options?.inventoryError ?? null;
  const deletedProjectPages = options?.deletedProjectPages ?? {};
  const latestVersions = options?.latestVersions ?? {};
  const messagePages = options?.messagePages ?? {};
  const latestVersionDelayMs = options?.latestVersionDelayMs ?? 0;

  return async ({ page }: { page: import('@playwright/test').Page }) => {
    await page.addInitScript(
      ({ history, inventory, inventoryError, deletedProjectPages, latestVersions, messagePages, latestVersionDelayMs }) => {
        const mockWindow = window as any;
        localStorage.clear();
        mockWindow.__PROJECTS_CALLS__ = [];
        let mutableHistory = structuredClone(history);
        let mutableInventory = structuredClone(inventory);
        let mutableDeletedProjectPages = structuredClone(deletedProjectPages);

        const config = {
          engines: [{ id: 'mock', name: 'Mock', provider: 'openai', apiKey: '', model: 'mock', baseUrl: '', enabled: true }],
          selectedEngineId: 'mock',
          freecadCmd: '',
          assets: [],
          microwave: { humId: null, dingId: null, muted: true },
          voice: { sttLanguageCode: 'en-US' },
          mcp: { mode: 'passive', autoAgents: [] },
          hasSeenOnboarding: true,
          connectionType: null,
          defaultEngineKind: 'freecad',
          defaultSourceLanguage: 'legacyPython',
          defaultGeometryBackend: 'freecad',
          maxGenerationAttempts: 3,
          maxVerifyAttempts: 0,
        };

        window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
        window.__TAURI_INTERNALS__.metadata = {};
        window.__TAURI_INTERNALS__.transformCallback = (callback: unknown) => {
          const id = Math.floor(Math.random() * 1_000_000_000);
          (window as any)[`_${id}`] = callback;
          return id;
        };
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: Record<string, unknown>) => {
          mockWindow.__PROJECTS_CALLS__.push({ cmd, args });
          if (cmd === 'get_config') return structuredClone(config);
          if (cmd === 'save_config') return null;
          if (cmd === 'get_runtime_capabilities') {
            return {
              freecad: { available: true, detail: 'Ready', path: '/mock/freecadcmd' },
              build123d: { available: true, detail: 'Ready', path: '/mock/python3' },
              mesh: { available: true, detail: 'Ready', path: '/mock/mesh' },
              recommendedAuthoringContext: {
                engineKind: 'freecad',
                sourceLanguage: 'legacyPython',
                geometryBackend: 'freecad',
              },
            };
          }
          if (cmd === 'get_animal_cap_catalog') {
            return { schemaVersion: 1, generatedAt: 0, entries: [] };
          }
          if (cmd === 'list_installed_component_package_headers') return [];
          if (cmd === 'get_history') return structuredClone(mutableHistory);
          if (cmd === 'get_inventory') {
            if (inventoryError) {
              throw { code: 'persistence', message: inventoryError.message, details: inventoryError.details };
            }
            return structuredClone(mutableInventory);
          }
          if (cmd === 'finalize_thread') {
            const id = String(args?.id ?? '');
            const project = mutableHistory.find((item: any) => item.id === id);
            if (project) {
              mutableHistory = mutableHistory.filter((item: any) => item.id !== id);
              mutableInventory = [
                {
                  ...project,
                  status: 'finalized',
                  finalizedAt: Date.UTC(2026, 6, 25),
                },
                ...mutableInventory.filter((item: any) => item.id !== id),
              ];
            }
            return null;
          }
          if (cmd === 'get_deleted_messages') return [];
          if (cmd === 'get_deleted_threads_page') {
            const cursor = String(args?.before ?? 'first');
            return structuredClone(
              mutableDeletedProjectPages[cursor] ?? {
                items: [],
                nextBefore: null,
                hasMore: false,
              },
            );
          }
          if (cmd === 'restore_deleted_thread') {
            const id = String(args?.id ?? '');
            for (const page of Object.values(mutableDeletedProjectPages) as any[]) {
              const project = page.items.find((item: any) => item.id === id);
              if (!project) continue;
              page.items = page.items.filter((item: any) => item.id !== id);
              mutableHistory = [
                {
                  id: project.id,
                  title: project.title,
                  summary: project.summary,
                  updatedAt: project.updatedAt,
                  messages: [],
                  versionCount: project.versionCount,
                  pendingCount: 0,
                  queuedCount: 0,
                  errorCount: 0,
                  status: 'active',
                  finalizedAt: null,
                  pendingConfirm: null,
                },
                ...mutableHistory,
              ];
              break;
            }
            return null;
          }
          if (cmd === 'get_deleted_thread_preview') return null;
          if (cmd === 'get_last_design') return null;
          if (cmd === 'get_active_agent_sessions') return [];
          if (cmd === 'get_agent_terminal_snapshots') return [];
          if (cmd === 'get_mcp_server_status') return [];
          if (cmd === 'get_mess_stl_path') return '/mock/mess.stl';
          if (cmd === 'get_default_macro') return '# mock macro';
          if (cmd === 'get_thread_latest_version') {
            if (latestVersionDelayMs > 0) {
              await new Promise((resolve) => setTimeout(resolve, latestVersionDelayMs));
            }
            return structuredClone(latestVersions[String(args?.threadId ?? '')] ?? null);
          }
          if (cmd === 'get_thread_messages_page') {
            const threadMessages = messagePages[String(args?.threadId ?? '')];
            if (threadMessages) {
              return {
                messages: structuredClone(threadMessages),
                hasMore: false,
                nextBefore: null,
              };
            }
            return {
              messages: [],
              hasMore: false,
              nextBefore: null,
            };
          }
          return null;
        };
      },
      {
        history,
        inventory,
        inventoryError,
        deletedProjectPages,
        latestVersions,
        messagePages,
        latestVersionDelayMs,
      },
    );
  };
}

test.describe('Projects', () => {
  test('shows search input', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();
    const searchInput = page.locator('[data-window-id="projects"] .search-input');
    await expect(searchInput).toBeVisible();
    await expect(searchInput).toHaveAttribute('placeholder', 'Search...');
  });

  test('new project button opens chooser', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();
    await page.locator('[data-window-id="projects"]').getByRole('button', { name: '+ NEW' }).click();
    await expect(page.getByRole('dialog', { name: /Start New Project/i })).toBeVisible();
  });

  test('Given blank project starts When code window opens Then editor stays empty', async ({ page }) => {
    await installProjectSwitcherMocks()({ page });

    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();
    await page.locator('[data-window-id="projects"]').getByRole('button', { name: '+ NEW' }).click();
    await page.getByRole('button', { name: 'Blank Project' }).click();

    const viewportCodeButton = page.getByTestId('workbench-bottom-dock').getByRole('button', { name: /CODE/i });
    await expect(viewportCodeButton).toBeVisible();
    await expect(viewportCodeButton).toBeEnabled();

    await viewportCodeButton.click();

    await expect(page.getByText(/MACRO INSPECTOR:/i)).toBeVisible();
    const editor = page.locator('.cm-content').first();
    await expect(editor).toBeVisible();
    await expect.poll(async () => (await editor.innerText()).trim()).toBe('');
    await expect(editor).not.toContainText('# mock macro');
  });

  test('shows no project cards when no history', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();
    await expect(page.locator('[data-window-id="projects"] .project-card')).toHaveCount(0);
  });

  test('Given more than six projects When projects opens Then thumbnail fetch warms every card', async ({ page }) => {
    await installProjectSwitcherMocks({
      history: Array.from({ length: 8 }, (_, index) => ({
        id: `thread-${index + 1}`,
        title: `Preview thread ${index + 1}`,
        summary: '',
        updatedAt: Date.UTC(2026, 5, index + 1),
        messages: [],
        genieTraits: null,
        versionCount: 1,
        pendingCount: 0,
        queuedCount: 0,
        errorCount: 0,
        status: 'ready',
        finalizedAt: null,
        pendingConfirm: null,
      })),
    })({ page });

    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();

    const projectsWindow = page.locator('[data-window-id="projects"]');
    await expect(projectsWindow.locator('.project-card')).toHaveCount(8);
    await expect
      .poll(async () =>
        page.evaluate(() =>
          ((window as any).__PROJECTS_CALLS__ as Array<{ cmd: string }>).filter(
            (entry) => entry.cmd === 'get_thread_latest_version',
          ).length,
        ),
      )
      .toBe(8);
  });

  test('Given latest version lacks thumbnail When older page has preview Then project card displays preview image', async ({ page }) => {
    const previewImage = `data:image/png;base64,${btoa('older-preview')}`;
    await installProjectSwitcherMocks({
      history: [
        {
          id: 'thread-preview',
          title: 'Paged Preview Thread',
          summary: '',
          updatedAt: Date.UTC(2026, 5, 1),
          messages: [],
          genieTraits: null,
          versionCount: 2,
          pendingCount: 0,
          queuedCount: 0,
          errorCount: 0,
          status: 'ready',
          finalizedAt: null,
          pendingConfirm: null,
        },
      ],
      latestVersions: {
        'thread-preview': {
          id: 'latest-no-image',
          role: 'assistant',
          status: 'success',
          artifactBundle: { previewStlPath: '/mock/latest.stl' },
          imageData: null,
        },
      },
      messagePages: {
        'thread-preview': [
          {
            id: 'older-with-image',
            role: 'assistant',
            status: 'success',
            artifactBundle: { previewStlPath: '/mock/older.stl' },
            imageData: previewImage,
          },
        ],
      },
    })({ page });

    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();

    const card = page.locator('[data-window-id="projects"] .project-card').filter({ hasText: 'Paged Preview Thread' });
    await expect(card.locator('.card-thumb img')).toHaveAttribute('src', previewImage);
    await expect(card.getByText('NO PREVIEW')).toHaveCount(0);
  });

  test('Given a project starts without preview When preview event arrives Then card replaces NO PREVIEW', async ({ page }) => {
    const previewImage = `data:image/png;base64,${btoa('fresh-preview')}`;
    await installProjectSwitcherMocks({
      history: [
        {
          id: 'thread-preview',
          title: 'Event Preview Thread',
          summary: '',
          updatedAt: Date.UTC(2026, 5, 2),
          messages: [],
          genieTraits: null,
          versionCount: 1,
          pendingCount: 0,
          queuedCount: 0,
          errorCount: 0,
          status: 'ready',
          finalizedAt: null,
          pendingConfirm: null,
        },
      ],
      latestVersions: {
        'thread-preview': {
          id: 'version-1',
          role: 'assistant',
          status: 'success',
          artifactBundle: { previewStlPath: '/mock/version-1.stl' },
          imageData: null,
        },
      },
    })({ page });

    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();

    const card = page.locator('[data-window-id="projects"] .project-card').filter({ hasText: 'Event Preview Thread' });
    await expect(card.getByText('NO PREVIEW')).toBeVisible();

    await page.evaluate(({ previewImage }) => {
      window.dispatchEvent(
        new CustomEvent('ecky:version-preview-updated', {
          detail: {
            threadId: 'thread-preview',
            messageId: 'version-1',
            imageData: previewImage,
          },
        }),
      );
    }, { previewImage });

    await expect(card.locator('.card-thumb img')).toHaveAttribute('src', previewImage);
    await expect(card.getByText('NO PREVIEW')).toHaveCount(0);
  });

  test('Given an active project When it is completed Then the same project and full history appear in Completed', async ({ page }) => {
    await installProjectSwitcherMocks({
      history: [
        {
          id: 'project-same-id',
          title: 'Bike light enclosure',
          summary: 'Three successful iterations',
          updatedAt: Date.UTC(2026, 6, 24),
          messages: [],
          genieTraits: null,
          versionCount: 3,
          pendingCount: 0,
          queuedCount: 0,
          errorCount: 0,
          status: 'active',
          finalizedAt: null,
          pendingConfirm: null,
        },
      ],
    })({ page });

    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();

    const projectsWindow = page.locator('[data-window-id="projects"]');
    const activeCard = projectsWindow.locator('.project-card').filter({ hasText: 'Bike light enclosure' });
    await activeCard.getByRole('button', { name: 'MORE ACTIONS' }).click();
    await activeCard.getByRole('button', { name: 'COMPLETE' }).click();
    await projectsWindow.getByRole('button', { name: 'COMPLETED' }).click();

    const completedCard = projectsWindow.locator('.project-card').filter({ hasText: 'Bike light enclosure' });
    await expect(completedCard).toHaveAttribute('data-project-id', 'project-same-id');
    await expect(completedCard).toContainText('3 versions');

    const calls = await page.evaluate(() => (window as any).__PROJECTS_CALLS__);
    expect(calls).toContainEqual({
      cmd: 'finalize_thread',
      args: expect.objectContaining({ id: 'project-same-id' }),
    });
  });

  test('Given a deleted project When it is recovered Then the same project ID returns to Active', async ({ page }) => {
    await installProjectSwitcherMocks({
      deletedProjectPages: {
        first: {
          items: [
            {
              id: 'deleted-project-id',
              title: 'Recovered enclosure',
              summary: 'Deleted by mistake',
              updatedAt: Date.UTC(2026, 6, 23),
              deletedAt: Date.UTC(2026, 6, 24),
              versionCount: 4,
            },
          ],
          nextBefore: null,
          hasMore: false,
        },
      },
    })({ page });

    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();

    const projectsWindow = page.locator('[data-window-id="projects"]');
    await projectsWindow.getByRole('button', { name: 'TRASH' }).click();
    const deletedCard = projectsWindow.locator('.project-card').filter({ hasText: 'Recovered enclosure' });
    await expect(deletedCard).toHaveAttribute('data-project-id', 'deleted-project-id');
    await deletedCard.getByRole('button', { name: 'RECOVER' }).click();

    await projectsWindow.getByRole('button', { name: 'ACTIVE' }).click();
    const restoredCard = projectsWindow.locator('.project-card').filter({ hasText: 'Recovered enclosure' });
    await expect(restoredCard).toHaveAttribute('data-project-id', 'deleted-project-id');

    const calls = await page.evaluate(() => (window as any).__PROJECTS_CALLS__);
    expect(calls).toContainEqual({
      cmd: 'restore_deleted_thread',
      args: { id: 'deleted-project-id' },
    });
  });

  test('Given more deleted projects exist When Trash opens Then it loads 24 and requests the next cursor on demand', async ({ page }) => {
    const firstItems = Array.from({ length: 24 }, (_, index) => ({
      id: `deleted-${index + 1}`,
      title: `Deleted project ${index + 1}`,
      summary: '',
      updatedAt: Date.UTC(2026, 6, 1),
      deletedAt: Date.UTC(2026, 6, 24),
      versionCount: 1,
    }));
    await installProjectSwitcherMocks({
      deletedProjectPages: {
        first: {
          items: firstItems,
          nextBefore: 'cursor-24',
          hasMore: true,
        },
        'cursor-24': {
          items: [
            {
              id: 'deleted-25',
              title: 'Deleted project 25',
              summary: '',
              updatedAt: Date.UTC(2026, 6, 1),
              deletedAt: Date.UTC(2026, 6, 23),
              versionCount: 2,
            },
          ],
          nextBefore: null,
          hasMore: false,
        },
      },
    })({ page });

    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();

    const projectsWindow = page.locator('[data-window-id="projects"]');
    await projectsWindow.getByRole('button', { name: 'TRASH' }).click();
    await expect(projectsWindow.locator('.project-card')).toHaveCount(24);
    await expect(projectsWindow.getByText('Deleted project 25', { exact: true })).toHaveCount(0);

    await projectsWindow.getByRole('button', { name: 'LOAD MORE' }).click();
    await expect(projectsWindow.locator('.project-card')).toHaveCount(25);
    await expect(projectsWindow.getByText('Deleted project 25', { exact: true })).toBeVisible();

    const pageCalls = await page.evaluate(() =>
      ((window as any).__PROJECTS_CALLS__ as Array<{ cmd: string; args?: Record<string, unknown> }>)
        .filter((entry) => entry.cmd === 'get_deleted_threads_page'),
    );
    expect(pageCalls).toEqual([
      { cmd: 'get_deleted_threads_page', args: { before: null, limit: 24 } },
      { cmd: 'get_deleted_threads_page', args: { before: 'cursor-24', limit: 24 } },
    ]);
  });

  test('Given project navigation When reusable assets are needed Then Library is a separate window', async ({ page }) => {
    await installProjectSwitcherMocks()({ page });

    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();

    const projectsWindow = page.locator('[data-window-id="projects"]');
    await expect(projectsWindow.getByRole('button', { name: 'PACKAGES' })).toHaveCount(0);
    await expect(projectsWindow.getByRole('button', { name: 'ACTIVE' })).toBeVisible();
    await expect(projectsWindow.getByRole('button', { name: 'COMPLETED' })).toBeVisible();

    await page.getByRole('button', { name: 'LIBRARY' }).click();
    const libraryWindow = page.locator('[data-window-id="library"]');
    await expect(libraryWindow).toBeVisible();
    await expect(libraryWindow.getByRole('button', { name: 'COMPONENT PACKAGES' })).toBeVisible();
    await expect(libraryWindow.getByRole('button', { name: 'FREECAD PARTS' })).toBeVisible();
    await expect(libraryWindow.getByRole('button', { name: 'CATALOG' })).toBeVisible();
  });

  test('Given Projects is open When viewport narrows Then window and compact card remain fully reachable', async ({ page }) => {
    await installProjectSwitcherMocks({
      history: [
        {
          id: 'responsive-project',
          title: 'Responsive enclosure',
          summary: 'Compact card proof',
          updatedAt: Date.UTC(2026, 6, 25),
          messages: [],
          genieTraits: null,
          versionCount: 2,
          pendingCount: 0,
          queuedCount: 0,
          errorCount: 0,
          status: 'active',
          finalizedAt: null,
          pendingConfirm: null,
        },
      ],
    })({ page });

    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();
    await page.setViewportSize({ width: 320, height: 700 });

    const projectsWindow = page.locator('[data-window-id="projects"]');
    await expect.poll(async () =>
      projectsWindow.evaluate((element) => {
        const rect = element.getBoundingClientRect();
        return {
          left: Math.round(rect.left),
          top: Math.round(rect.top),
          right: Math.round(rect.right),
          bottom: Math.round(rect.bottom),
        };
      }),
    ).toEqual({ left: 0, top: 80, right: 320, bottom: 580 });

    const card = projectsWindow.locator('.project-card').filter({ hasText: 'Responsive enclosure' });
    await expect(card).toBeVisible();
    await expect(card.getByRole('button', { name: 'MORE ACTIONS' })).toBeVisible();
  });

  test.describe('Completed projects', () => {
    test.beforeEach(
      installProjectSwitcherMocks({
        inventory: [
          {
            id: 'completed-1',
            title: 'Tradescantia zebrina pot',
            summary: 'twisted wall pot',
            updatedAt: Date.UTC(2026, 4, 22),
            messages: [],
            genieTraits: null,
            versionCount: 3,
            pendingCount: 0,
            queuedCount: 0,
            errorCount: 0,
            status: 'finalized',
            finalizedAt: Date.UTC(2026, 4, 22),
            pendingConfirm: null,
          },
        ],
        latestVersionDelayMs: 3000,
      }),
    );

    test('Given slow preview metadata, when Completed opens, then cards render without waiting', async ({ page }) => {
      await page.goto('/');
      await page.getByRole('button', { name: 'PROJECTS' }).click();
      await page.locator('[data-window-id="projects"]').getByRole('button', { name: 'COMPLETED' }).click();

      const projectsWindow = page.locator('[data-window-id="projects"]');
      await expect(projectsWindow.locator('.project-card')).toHaveCount(1);
      await expect(projectsWindow.getByText('Tradescantia zebrina pot')).toBeVisible();

      const calls = await page.evaluate(() => (window as any).__PROJECTS_CALLS__ as Array<{ cmd: string }>);
      expect(calls.filter((entry) => entry.cmd === 'get_thread_messages_page')).toHaveLength(0);
    });
  });

  test('Given completed-project loading fails, when Completed opens, then raw error shows', async ({ page }) => {
    await installProjectSwitcherMocks({
      inventoryError: {
        message: 'Inventory query failed',
        details: 'database is locked',
      },
    })({ page });

    await page.goto('/');
    await page.getByRole('button', { name: 'PROJECTS' }).click();
    await page.locator('[data-window-id="projects"]').getByRole('button', { name: 'COMPLETED' }).click();

    const projectsWindow = page.locator('[data-window-id="projects"]');
    await expect(projectsWindow.getByText('COMPLETED LOAD ERROR')).toBeVisible();
    await expect(projectsWindow.getByText('Inventory query failed')).toBeVisible();
    await expect(projectsWindow.getByText('database is locked')).toBeVisible();
  });
});
