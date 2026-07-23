import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const canonicalSource = readFileSync(
  fileURLToPath(new URL('../src/models/iphone-17e-voronoi-web.ecky', import.meta.url)),
  'utf8',
);

test.describe('Landing parametric case workbench', () => {
  test('Given two real case patterns When the case study loads Then pattern artifacts replace fake app chrome', async ({ page }) => {
    await page.goto('/');

    const caseStudy = page.locator('#case-study');
    await expect(caseStudy).toBeVisible();

    const workbench = caseStudy.getByTestId('case-workbench');
    await expect(workbench).toBeVisible();
    await expect(caseStudy.getByRole('heading', { name: "Write Lisp you don't understand." })).toBeVisible();
    await expect(caseStudy.getByText(/gaslight an LLM until it produces something useful/i)).toBeVisible();
    await expect(caseStudy.getByText(/funny plastic hat/i)).toBeVisible();
    await expect(workbench.getByText('iPhone 17e', { exact: true })).toBeVisible();
    await expect(workbench.getByRole('combobox', { name: 'Phone model' })).toHaveCount(0);
    await expect(workbench.getByRole('button', { name: 'SEE CODE' })).toBeVisible();
    await expect(workbench.getByRole('textbox')).toHaveCount(0);
    await expect(workbench.getByText('SESSION HISTORY')).toHaveCount(0);
    await expect(workbench.getByText('30 NAMED PARAMETERS')).toHaveCount(0);
    await expect(workbench.getByText('2 VERIFICATION CLAUSES')).toHaveCount(0);
    await expect(workbench.getByText('6,750-TRIANGLE STL EXPORT')).toHaveCount(0);
    await expect(workbench.getByText('SOURCE: ECKY')).toHaveCount(0);
    await expect(workbench.getByText('REAL STL EXPORT')).toHaveCount(0);

    const patterns = workbench.getByRole('group', { name: 'Case pattern' });
    await expect(patterns.getByRole('button', { name: 'VORONOI WEB' })).toHaveAttribute('aria-pressed', 'true');
    await expect(patterns.getByRole('button', { name: 'CELL GRID' })).toHaveAttribute('aria-pressed', 'false');
    await expect(workbench).toHaveAttribute('data-selected-variant', 'voronoi-web');

    const older = workbench.getByRole('group', { name: 'Earlier case versions' });
    await expect(older.getByRole('button')).toHaveCount(3);

    const viewer = workbench.locator('.viewer canvas');
    await expect(viewer).toBeVisible();
    await expect.poll(async () => workbench.locator('.viewer-load').count(), {
      message: 'real iPhone STL finished loading',
      timeout: 15_000,
    }).toBe(0);

    const caseDownload = caseStudy.getByRole('link', { name: 'DOWNLOAD CASE STL' });
    await expect(caseDownload).toHaveAttribute('download', 'iphone-17e-voronoi-web.stl');
    await expect(caseStudy.getByRole('link', { name: 'DOWNLOAD .ECKY' })).toHaveAttribute(
      'download',
      'iphone-17e-voronoi-web.ecky',
    );
    await expect(caseStudy.getByRole('link', { name: /INSERT|BUNDLE/ })).toHaveCount(0);

    await patterns.getByRole('button', { name: 'CELL GRID' }).click();
    await expect(workbench).toHaveAttribute('data-selected-variant', 'cell-grid');
    await expect(patterns.getByRole('button', { name: 'CELL GRID' })).toHaveAttribute('aria-pressed', 'true');
    await expect.poll(async () => workbench.locator('.viewer-load').count(), {
      message: 'cell-grid STL finished loading',
      timeout: 15_000,
    }).toBe(0);
    await expect(caseDownload).toHaveAttribute('download', 'iphone-17e-cell-grid.stl');

    await older.getByRole('button', { name: /PERFORATED/ }).click();
    await expect(workbench).toHaveAttribute('data-selected-variant', 'old-perforated');
    await expect(caseDownload).toHaveAttribute('download', 'iphone-17e-old-perforated.stl');
    await expect.poll(async () => workbench.locator('.viewer-load').count(), {
      message: 'earlier perforated STL finished loading',
      timeout: 15_000,
    }).toBe(0);
  });

  test('Given canonical source When CODE opens Then the complete highlighted source is readable, copyable, and keyboard dismissible', async ({ page, context }) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(`pageerror: ${error.message}`));
    page.on('console', (message) => {
      if (message.type() === 'error') errors.push(message.text());
    });
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await page.goto('/');

    const codeButton = page.getByTestId('case-workbench').getByRole('button', { name: 'SEE CODE' });
    await codeButton.click();

    const inspector = page.getByRole('dialog', { name: 'Macro Inspector: iPhone 17e — Voronoi web' });
    await expect(inspector).toBeVisible();
    // Static source: inspectable, highlighted, and deliberately not editable.
    const source = inspector.getByTestId('case-source');
    await expect(source).toBeVisible();
    await expect(source.getByTestId('source-line-number').first()).toHaveText('1');
    await expect(source.locator('.source-token--keyword').filter({ hasText: 'model' })).toBeVisible();
    await expect(source.locator('.source-token--number').filter({ hasText: '146.71' })).toBeVisible();
    await expect(source.locator('.source-token--comment').filter({ hasText: 'Print/backend controls' })).toBeVisible();
    await expect(inspector.getByRole('textbox')).toHaveCount(0);
    await expect(inspector.getByRole('button', { name: 'APPLY' })).toHaveCount(0);
    await expect(source).toHaveAttribute(
      'data-source-length',
      String(canonicalSource.length),
    );

    await inspector.getByRole('button', { name: 'COPY CODE' }).click();
    await expect(inspector.getByRole('button', { name: 'COPIED' })).toBeVisible();
    await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toBe(canonicalSource);

    await page.keyboard.press('Escape');
    await expect(inspector).toBeHidden();
    await expect(codeButton).toBeFocused();
    expect(errors, 'source inspector opens without browser errors').toEqual([]);
  });

  test('Given the case pattern is ready When the visitor pauses Then the front inspection view stays stable', async ({ page }) => {
    await page.goto('/');

    const workbench = page.getByTestId('case-workbench');
    await expect.poll(async () => workbench.locator('.viewer-load').count(), {
      message: 'real iPhone STL finished loading',
      timeout: 15_000,
    }).toBe(0);

    const canvas = workbench.locator('.viewer canvas');
    const firstFrame = await canvas.screenshot();
    await page.waitForTimeout(700);
    const secondFrame = await canvas.screenshot();
    expect(secondFrame, 'pattern does not rotate away before the visitor drags').toEqual(firstFrame);
  });

  test('Given the STL request fails When the viewer settles Then raw asset context replaces pending and retry recovers', async ({ page }) => {
    await page.route('**/*.stl', (route) => route.abort('failed'));
    await page.goto('/');

    const workbench = page.getByTestId('case-workbench');
    const failure = workbench.getByRole('alert');
    await expect(failure).toContainText('iphone-17e-voronoi-web');
    await expect(workbench.locator('.viewer-load')).toHaveCount(0);

    await page.unroute('**/*.stl');
    await workbench.getByRole('button', { name: 'RETRY STL' }).click();
    await expect(failure).toBeHidden();
    await expect.poll(async () => workbench.locator('.viewer-load').count(), {
      message: 'retry loaded the real STL',
      timeout: 15_000,
    }).toBe(0);
  });

  test('Given a narrow viewport When the vignette and source inspector open Then layout stays inside the page', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/');

    const workbench = page.getByTestId('case-workbench');
    const viewer = workbench.locator('.viewer');
    await expect(viewer).toBeVisible();
    const viewerBox = await viewer.boundingBox();
    expect(viewerBox?.width, 'responsive viewer width').toBeGreaterThan(0);
    expect(viewerBox?.width, 'responsive viewer fits narrow page').toBeLessThanOrEqual(390);

    const codeButton = workbench.getByRole('button', { name: 'SEE CODE' });
    await codeButton.click();
    const inspector = page.getByRole('dialog', { name: 'Macro Inspector: iPhone 17e — Voronoi web' });
    await expect(inspector).toBeVisible();

    const overflow = await page.evaluate(() => ({
      body: document.body.scrollWidth - document.body.clientWidth,
      root: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    }));
    expect(overflow.body).toBeLessThanOrEqual(0);
    expect(overflow.root).toBeLessThanOrEqual(0);
    const dialogBox = await inspector.boundingBox();
    expect(dialogBox?.x).toBeGreaterThanOrEqual(0);
    expect(dialogBox?.y).toBeGreaterThanOrEqual(0);
    expect((dialogBox?.x ?? 0) + (dialogBox?.width ?? 0)).toBeLessThanOrEqual(390);
    expect((dialogBox?.y ?? 0) + (dialogBox?.height ?? 0)).toBeLessThanOrEqual(844);
    await expect.poll(() => page.evaluate(() => Boolean(document.querySelector('dialog:modal')))).toBe(true);
    await expect.poll(() => page.evaluate(() => getComputedStyle(document.documentElement).overflow)).toBe('hidden');

    for (let index = 0; index < 5; index += 1) await page.keyboard.press('Tab');
    await expect.poll(() => page.evaluate(() => Boolean(document.activeElement?.closest('dialog')))).toBe(true);
    await expect(page.getByRole('button', { name: 'CLOSE CODE' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'DOWNLOAD SOURCE' })).toBeVisible();

    await page.keyboard.press('Escape');
    await expect(inspector).toBeHidden();
    await expect(codeButton).toBeFocused();
    await expect.poll(() => page.evaluate(() => getComputedStyle(document.documentElement).overflow)).not.toBe('hidden');
  });
});
