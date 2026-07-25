import { expect, test, type Page } from '@playwright/test';

type CatalogMockMode = 'ok' | 'error';

async function installAnimalCatalogMocks(page: Page, mode: CatalogMockMode) {
  await page.addInitScript((mockMode) => {
    const mockWindow = window as any;
    mockWindow.__ANIMAL_CAP_CATALOG__ = {
      schemaVersion: 1,
      entries: [{
        id: 'quaternius-pug-presta',
        displayName: 'Pug Presta Valve Cap',
        species: 'Pug',
        state: 'published',
        surfaces: { engine: true, landing: true },
        source: {
          author: 'Quaternius',
          pageUrl: 'https://opengameart.org/content/lowpoly-animated-farm-animal-pack',
          license: 'CC0-1.0',
          licenseUrl: 'https://creativecommons.org/publicdomain/zero/1.0/',
        },
        recipe: {
          boreProfileId: 'presta-blind-bomb-v1',
          boreAxis: 'z',
          uniformScale: 12,
          boreAxisHeightMm: 8.5,
        },
        artifact: {
          verificationStatus: 'passed',
          modelId: 'generated-direct-occt-ee9682ed87c5',
          threadId: '0e018829-ebba-4137-bab9-a32e42b49fcd',
          messageId: '46d93d70-66a3-428c-b5d3-59b8253db103',
          sourcePath: '/mock/catalog/pug-presta.ecky',
          stlPath: '/mock/catalog/pug-presta.stl',
          previewPath: '/mock/catalog/pug-presta.png',
        },
      }],
    };

    window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
    window.__TAURI_INTERNALS__.invoke = async (cmd) => {
      if (cmd === 'get_config') {
        return {
          engines: [],
          selectedEngineId: '',
          freecadCmd: '',
          freecadLibraryRoots: [],
          assets: [],
          microwave: { humId: null, dingId: null, muted: true },
          mcp: {
            port: null,
            maxSessions: null,
            mode: 'passive',
            primaryAgentId: null,
            promptTimeoutSecs: 1800,
            autoAgents: [],
          },
          hasSeenOnboarding: true,
          connectionType: 'api_key',
          defaultEngineKind: 'ecky',
          defaultSourceLanguage: 'ecky',
          defaultGeometryBackend: 'mesh',
          maxGenerationAttempts: 1,
          maxVerifyAttempts: 0,
        };
      }
      if (cmd === 'get_runtime_capabilities') {
        return {
          freecad: { available: true, detail: 'Ready', path: '/mock/freecadcmd' },
          build123d: { available: true, detail: 'Ready', path: '/mock/python3' },
          mesh: { available: true, detail: 'bundled', path: null },
          recommendedAuthoringContext: {
            engineKind: 'ecky',
            sourceLanguage: 'ecky',
            geometryBackend: 'mesh',
          },
        };
      }
      if (cmd === 'get_history') return [];
      if (cmd === 'get_last_design') return null;
      if (cmd === 'get_default_macro') return '';
      if (cmd === 'check_freecad') return true;
      if (cmd === 'get_mess_stl_path') return '/mock/mess.stl';
      if (cmd === 'get_active_agent_sessions') return [];
      if (cmd === 'get_agent_terminal_snapshots') return [];
      if (cmd === 'get_thread_agent_state') {
        return {
          threadId: null,
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
      if (cmd === 'list_installed_component_package_headers') return [];
      if (cmd === 'get_animal_cap_catalog') {
        if (mockMode === 'error') {
          throw {
            code: 'persistence',
            message: 'animal cap catalog failed',
            details: 'raw manifest path missing',
          };
        }
        return mockWindow.__ANIMAL_CAP_CATALOG__;
      }
      return null;
    };
  }, mode);
}

test.describe('Animal cap catalog', () => {
  test('Given a published engine entry When Catalog opens Then its fit and provenance are visible', async ({ page }) => {
    await installAnimalCatalogMocks(page, 'ok');
    await page.goto('/');
    await expect(page.locator('.boot-overlay')).toHaveCount(0);

    await page.getByRole('button', { name: 'LIBRARY' }).click();
    await page.getByRole('button', { name: 'CATALOG' }).click();

    const catalog = page.locator('[data-window-id="library"]');
    await expect(catalog.getByText('Pug Presta Valve Cap')).toBeVisible();
    await expect(catalog.getByText('Pug · CC0-1.0')).toBeVisible();
    await expect(catalog.getByText('presta-blind-bomb-v1')).toBeVisible();
    await expect(catalog.getByText('CC0-1.0')).toBeVisible();
    await expect(catalog.getByText('VERIFIED')).toBeVisible();
  });

  test('Given catalog loading fails When Catalog opens Then raw backend body stays visible', async ({ page }) => {
    await installAnimalCatalogMocks(page, 'error');
    await page.goto('/');
    await expect(page.locator('.boot-overlay')).toHaveCount(0);

    await page.getByRole('button', { name: 'LIBRARY' }).click();
    await page.getByRole('button', { name: 'CATALOG' }).click();

    await expect(page.getByText('animal cap catalog failed')).toBeVisible();
    await expect(page.getByText('raw manifest path missing')).toBeVisible();
  });
});
