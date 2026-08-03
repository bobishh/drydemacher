import { expect, test } from '@playwright/test';

test.describe('Ecky language reference — paged docs reader', () => {
  test('Given the docs root When opened Then it shows a linked operation index without backend noise', async ({ page }) => {
    await page.goto('/docs/');

    await expect(page).toHaveTitle(/Operation Index · Ecky Language Reference/);
    await expect(page.getByRole('heading', { name: 'Operation Index', level: 1 })).toBeVisible();
    await expect(page.locator('.docs-main__section')).toHaveCount(1);
    await expect(page.getByText(/build123d|ecky-rust|freecad/i)).toHaveCount(0);

    const boxLink = page.locator('.docs-main').getByRole('link', { name: 'box', exact: true });
    await expect(boxLink).toHaveAttribute('href', '/docs/primitive-signatures/#box');
  });

  test('Given a reference page When rendered Then the sidebar contains route links for all sections', async ({ page }) => {
    await page.goto('/docs/primitive-signatures/');

    const toc = page.locator('nav[aria-label="Reference contents"]');
    const links = toc.locator('.docs-toc__link');
    await expect(links).toHaveCount(14);
    await expect(toc.getByRole('link', { name: 'Operation Index' })).toHaveAttribute('href', '/docs/');
    await expect(toc.getByRole('link', { name: 'Verify Clauses' })).toHaveAttribute(
      'href',
      '/docs/verify-clauses/',
    );
    await expect(toc.getByRole('link', { name: 'Primitive Signatures' })).toHaveAttribute(
      'aria-current',
      'page',
    );
  });

  test('Given the operation index When an operation is clicked Then its exact signature opens', async ({ page }) => {
    await page.goto('/docs/');

    const boxLink = page.locator('.docs-main').getByRole('link', { name: 'box', exact: true });
    await boxLink.click();

    await expect(page).toHaveURL('/docs/primitive-signatures/#box');
    await expect(page.getByRole('heading', { name: 'box', level: 3 })).toBeVisible();
  });

  test('Given component docs When opened Then live locks and native STEP boundaries are explicit', async ({ page }) => {
    await page.goto('/docs/components/');

    await expect(
      page.getByRole('heading', { name: 'Live package references', level: 3 }),
    ).toBeVisible();
    await expect(page.locator('.docs-main')).toContainText('No semver ranges');
    await expect(page.locator('.docs-main')).toContainText('ecky.lock.json');
    await expect(page.locator('.docs-main')).toContainText('never calls FreeCAD');
    await expect(page.locator('.docs-main')).toContainText('solidify');
  });

  test('Given a phone viewport When Contents opens Then a section link remains clickable', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/docs/');

    const menu = page.getByRole('button', { name: '☰ Contents', exact: true });
    const toc = page.locator('nav[aria-label="Reference contents"]');
    await expect(menu).toBeVisible();
    await expect(menu).toHaveAttribute('aria-expanded', 'false');
    await expect(toc).toBeHidden();

    await menu.click();
    await expect(menu).toHaveAttribute('aria-expanded', 'true');
    await expect(toc).toBeVisible();

    await toc.getByRole('link', { name: 'Verify Clauses' }).click();
    await expect(page).toHaveURL('/docs/verify-clauses/');
    await expect(page.getByRole('heading', { name: 'Verify Clauses', level: 1 })).toBeVisible();
    await expect(toc).toBeHidden();
  });

  test('Given a phone viewport When Contents is open Then the menu owns vertical scrolling', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 480 });
    await page.goto('/docs/');

    await page.getByRole('button', { name: '☰ Contents', exact: true }).click();
    const toc = page.locator('nav[aria-label="Reference contents"]');
    await expect(toc).toBeVisible();

    const metrics = await toc.evaluate((element) => ({
      clientHeight: element.clientHeight,
      scrollHeight: element.scrollHeight,
      viewportHeight: window.visualViewport?.height ?? window.innerHeight,
      headerHeight: Number.parseFloat(
        getComputedStyle(document.documentElement).getPropertyValue('--header-h'),
      ),
    }));
    expect(metrics.clientHeight).toBeLessThanOrEqual(metrics.viewportHeight - metrics.headerHeight);
    expect(metrics.scrollHeight).toBeGreaterThan(metrics.clientHeight);

    const pageScrollBefore = await page.evaluate(() => window.scrollY);
    await toc.hover();
    await page.mouse.wheel(0, 700);
    await expect.poll(() => toc.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBe(pageScrollBefore);
  });

  test('Given a phone viewport When a long page reloads Then document scrolling remains native', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/docs/');

    for (const selector of ['.docs-shell', '.docs-layout', '.docs-main', '.docs-main__section']) {
      await expect(page.locator(selector)).toHaveCSS('overflow', 'visible');
    }

    await page.mouse.wheel(0, 900);
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(0);

    await page.reload();
    await page.mouse.wheel(0, 900);
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(0);

    const horizontalOverflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(horizontalOverflow).toBeLessThanOrEqual(0);
  });

  test('Given a section page When rendered Then previous and next section links are present', async ({ page }) => {
    await page.goto('/docs/language-overview/');

    const pager = page.locator('nav[aria-label="Section navigation"]');
    await expect(pager.getByRole('link', { name: /Operation Index/ })).toHaveAttribute(
      'href',
      '/docs/',
    );
    await expect(pager.getByRole('link', { name: /Forms and Structure/ })).toHaveAttribute(
      'href',
      '/docs/forms-and-structure/',
    );
  });

  test('Given a direct section URL When opened Then only that section and its code render', async ({ page }) => {
    await page.goto('/docs/primitive-signatures/');

    await expect(page.getByRole('heading', { name: 'Primitive Signatures', level: 1 })).toBeVisible();
    await expect(page.locator('.docs-main__section')).toHaveCount(1);
    await expect(page.getByRole('heading', { name: 'Language Overview', level: 1 })).toHaveCount(0);
    expect(await page.locator('.docs-main pre code').count()).toBeGreaterThan(0);
  });

  test('Given the docs page When rendered Then raw markdown and EPUB links remain present', async ({ page }) => {
    await page.goto('/docs/');

    await expect(page.getByRole('link', { name: /Raw .md/i })).toHaveAttribute(
      'href',
      '/docs/ecky-ir.md',
    );
    await expect(page.getByRole('link', { name: /EPUB/i })).toHaveAttribute(
      'href',
      '/docs/ecky-ir-field-guide.epub',
    );
  });

  test('Given the raw markdown URL When fetched Then it contains linked operations without backend columns', async ({
    request,
  }) => {
    const response = await request.get('/docs/ecky-ir.md');
    expect(response.status()).toBe(200);
    expect(response.headers()['content-type']).toContain('text/markdown');
    const body = await response.text();
    expect(body).toContain('## Operation Index');
    expect(body).toContain('[`box`](#box)');
    expect(body).not.toContain('Available backends');
  });

  test('Given a docs route When fully loaded Then no console or page errors fire', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') errors.push(msg.text());
    });
    page.on('pageerror', (err) => errors.push(err.message));

    await page.goto('/docs/primitive-signatures/');
    await page.waitForLoadState('networkidle');

    expect(errors).toEqual([]);
  });
});
