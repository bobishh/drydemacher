import { expect, test } from '@playwright/test';

/**
 * Landing page e2e — BDD Given/When/Then, user-visible behavior.
 * The landing is a separate static Vite project from the Tauri app, but it
 * reuses the canonical genome from src/lib/genie. These tests gate the two
 * things that can silently break: (a) the landing shell renders with the right
 * structure + CTAs, and (b) the mascot / STL viewer mount without console
 * errors (genome import / Three.js scene build failures would surface as
 * errors on load).
 */

test.describe('Ecky landing', () => {
  test('Given the app mark When favicon renders Then it matches the canonical Ecky face', async ({ page }) => {
    await page.goto('/favicon.svg');

    const silhouette = page.locator('#ecky-silhouette');
    const eyes = page.locator('[data-ecky-feature="eye"]');
    const mouth = page.locator('[data-ecky-feature="mouth"]');

    await expect(silhouette).toBeVisible();
    await expect(eyes).toHaveCount(2);
    await expect(mouth).toHaveCount(1);
    await expect(mouth).toHaveAttribute('points');
    await expect(page.locator('[data-ecky-feature="smile"], [data-ecky-feature="node"]')).toHaveCount(0);
  });

  test('Given no packaged app release When page opens Then the real case leads and app download claims stay absent', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') errors.push(msg.text());
    });

    await page.goto('/');

    await expect(page.locator('.nav')).toContainText('Ecky CAD');
    const hero = page.locator('.hero');
    await expect(hero.getByRole('heading', { name: "Write Lisp you don't understand." })).toBeVisible();
    await expect(hero.getByText(/PROMPT-DRIVEN CAD/)).toBeVisible();
    await expect(hero.getByText(/gaslight an LLM until it produces something useful/i)).toBeVisible();
    await expect(hero.getByTestId('case-workbench')).toBeVisible();

    await expect(page.getByRole('link', { name: 'Download', exact: true })).toHaveCount(0);
    await expect(page.getByRole('link', { name: 'Download ↗', exact: true })).toHaveCount(0);
    await expect(page.getByRole('link', { name: 'Releases ↗', exact: true })).toHaveCount(0);
    await expect(page.locator('a[href*="/releases"]')).toHaveCount(0);
    await expect(hero.getByRole('link', { name: 'Source ↗' })).toHaveAttribute('href', 'https://github.com/bobishh/ecky');
    await expect(hero.getByRole('link', { name: 'Docs' })).toHaveAttribute('href', '/docs/');
    await expect(hero.getByRole('link', { name: 'DOWNLOAD CASE STL' })).toBeVisible();

    expect(errors, 'page opened with no console errors').toEqual([]);
  });

  test('Given mascot zone When page loads Then WebGL canvas mounts without errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(`pageerror: ${err.message}`));
    page.on('console', (msg) => {
      if (msg.type() === 'error') errors.push(msg.text());
    });

    await page.goto('/');
    const canvas = page.locator('.mascot canvas');
    await expect(canvas).toBeVisible();

    await expect.poll(async () => {
      const ok = await canvas.evaluate((el: HTMLCanvasElement) => el.width > 0 && el.height > 0);
      return ok;
    }, { message: 'canvas has nonzero backing size', timeout: 10_000 }).toBe(true);

    expect(errors, 'mascot mounted without genome/renderer errors').toEqual([]);
  });

  test('Given reduced motion When the hero settles Then mascot and scrolling remain still', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/');

    const canvas = page.locator('.mascot canvas');
    await expect(canvas).toBeVisible();
    const firstFrame = await canvas.screenshot();
    await page.waitForTimeout(500);
    const secondFrame = await canvas.screenshot();
    expect(secondFrame).toEqual(firstFrame);
    await expect.poll(() => page.evaluate(() => getComputedStyle(document.documentElement).scrollBehavior)).toBe('auto');
  });

  test('Given measured scope section When scrolled Then six fact cards render', async ({ page }) => {
    await page.goto('/');

    // First grid reports current implementation scope (6 cards).
    const grids = page.locator('.feature-grid');
    await expect(grids).toHaveCount(2);

    const features = grids.first().locator('.feature-card');
    await expect(features).toHaveCount(6);
    await expect(features.first()).toContainText(/54 Core IR operations/);
  });

  test('Given project claims When page opens Then facts, limits, and origin are explicit', async ({ page }) => {
    await page.goto('/');

    await expect(page.locator('.hero-summary')).toContainText(/v0\.0\.1.*pre-release/i);
    await expect(page.getByRole('heading', { name: "Write Lisp you don't understand." })).toBeVisible();
    await expect(page.getByText(/gaslight an LLM until it produces something useful/i)).toBeVisible();
    await expect(page.getByText(/30 named parameters.*2 verification clauses.*6,750-triangle STL/i)).toHaveCount(0);

    const facts = page.locator('.feature-grid').first();
    await expect(facts).toContainText('54 Core IR operations');
    await expect(facts).toContainText('50 run directly on native OCCT');
    await expect(facts).toContainText('3 geometry backends');
    await expect(facts).toContainText('2 authoring paths');
    await expect(facts).not.toContainText('Any model');

    await expect(page.getByRole('heading', { name: 'From Python macros to a constrained CAD language' })).toBeVisible();
    const history = page.locator('.feature-grid').nth(1);
    await expect(history).toContainText('March 2026');
    await expect(history).toContainText('FreeCAD Python macros');
    await expect(history).toContainText('July 2026');
    await expect(history).toContainText('v0.0.1');
  });

  test('Given a 390px viewport When navigation renders Then every link stays on one line', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/');

    const nav = page.locator('.nav');
    const measurements = await nav.evaluate((element) => ({
      height: element.getBoundingClientRect().height,
      links: Array.from(element.querySelectorAll('a')).map((link) => {
        const rect = link.getBoundingClientRect();
        return { text: link.textContent?.trim(), height: rect.height, lineHeight: Number.parseFloat(getComputedStyle(link).lineHeight) };
      }),
    }));

    expect(measurements.height, 'compact mobile navigation').toBeLessThanOrEqual(68);
    for (const link of measurements.links) {
      expect(link.height, `${link.text} remains one line`).toBeLessThanOrEqual(link.lineHeight * 1.25);
    }
  });

  test('Given social crawlers When the landing metadata is read Then a canonical share card is complete', async ({ page, request }) => {
    await page.goto('/');

    await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', 'https://ecky-cad.com/');
    await expect(page.locator('meta[property="og:image"]')).toHaveAttribute('content', 'https://ecky-cad.com/og-image.png');
    await expect(page.locator('meta[name="twitter:card"]')).toHaveAttribute('content', 'summary_large_image');
    await expect(page.locator('meta[name="theme-color"]')).toHaveAttribute('content', '#1a1a2e');

    const robots = await request.get('/robots.txt');
    expect(robots.headers()['content-type']).toContain('text/plain');
    expect(await robots.text()).toContain('Sitemap: https://ecky-cad.com/sitemap.xml');

    const sitemap = await request.get('/sitemap.xml');
    expect(sitemap.headers()['content-type']).toMatch(/xml/);
    expect(await sitemap.text()).toContain('<loc>https://ecky-cad.com/</loc>');

    const indexNowKey = await request.get('/indexnow-key.txt');
    expect(indexNowKey.headers()['content-type']).toContain('text/plain');
    expect((await indexNowKey.text()).trim()).toBe('69a8ca5615ffe76f7e56f6a662beaf6a');
  });

  test('Given the showcase section When page loads Then the live STL viewer mounts the real iPhone case mesh', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(`pageerror: ${err.message}`));
    page.on('console', (msg) => {
      if (msg.type() === 'error') errors.push(msg.text());
    });

    await page.goto('/');
    // The vignette renders the real exported STL; interaction/content are
    // specified in case-workbench.spec.ts.
    const caseStudy = page.locator('#case-study');
    await expect(caseStudy.getByRole('heading', { name: "Write Lisp you don't understand." })).toBeVisible();

    const viewer = caseStudy.locator('.viewer canvas');
    await expect(viewer).toBeVisible();
    await expect.poll(async () => {
      const ok = await viewer.evaluate((el: HTMLCanvasElement) => el.width > 0 && el.height > 0);
      return ok;
    }, { message: 'STL viewer canvas mounted', timeout: 10_000 }).toBe(true);

    // The STL is served (not 404): gate on the loading hint disappearing.
    await expect.poll(async () => {
      const stillLoading = await caseStudy.locator('.viewer-load').count();
      return stillLoading;
    }, { message: 'STL part finished loading', timeout: 15_000 }).toBe(0);

    expect(errors, 'STL viewer loaded the mesh without errors').toEqual([]);
  });

  test('Given repository history When scrolled Then six dated transitions render', async ({ page }) => {
    await page.goto('/');

    await expect(page.getByRole('heading', { name: 'From Python macros to a constrained CAD language' })).toBeVisible();

    const items = page.locator('.feature-grid').nth(1).locator('.feature-card');
    await expect(items).toHaveCount(6);
    await expect(items.first().locator('.status')).toContainText('March 2026');
    await expect(items.last().locator('.status')).toContainText('July 2026');
    await expect(items.last()).toContainText(/Still pre-release/);
  });
});
