import { expect, test } from '@playwright/test';

test.describe('Landing working model workbench', () => {
  test('Given the two freshest bottle-holder threads When the landing loads Then their mating parts replace the stale combined model', async ({ page }) => {
    await page.goto('/');

    const caseStudy = page.locator('#case-study');
    await expect(caseStudy).toBeVisible();

    const workbench = caseStudy.getByTestId('model-workbench');
    await expect(workbench).toBeVisible();
    await expect(caseStudy.getByRole('heading', { name: 'Make parts with AI. Keep the model.' })).toBeVisible();
    await expect(caseStudy.getByText(/inspect or edit the readable .ecky source/i)).toBeVisible();
    await expect(workbench.getByRole('button', { name: 'SEE CODE' })).toBeVisible();

    const models = workbench.getByRole('group', { name: 'Working models' });
    await expect(models.getByRole('button')).toHaveCount(4);
    await expect(models.getByRole('button', { name: /BOTTLE HOLDER/ })).toHaveAttribute('aria-pressed', 'true');
    await expect(models.getByRole('button', { name: /GILLETTE KIT/ })).toHaveAttribute('aria-pressed', 'false');
    await expect(models.getByRole('button', { name: /FILM SCANNER/ })).toHaveAttribute('aria-pressed', 'false');
    await expect(models.getByRole('button', { name: /AIRTAG BRACELET/ })).toHaveCount(0);
    await expect(models.getByRole('button', { name: /PHONE CASE/ })).toHaveAttribute('aria-pressed', 'false');
    await expect(models.getByRole('button', { name: /PRESTA CAP|DOVETAIL BOX|FRAME BRACKET/ })).toHaveCount(0);
    await expect(workbench).toHaveAttribute('data-selected-variant', 'bicycle-bottle-holder');

    const viewer = workbench.locator('.viewer canvas');
    await expect(viewer).toBeVisible();
    await expect.poll(async () => workbench.locator('.viewer-load').count(), {
      message: 'fresh bottle holder and frame rail STLs finished loading',
      timeout: 15_000,
    }).toBe(0);

    const modelDownload = caseStudy.getByRole('link', { name: 'DOWNLOAD BOTTLE HOLDER STL' });
    await expect(modelDownload).toHaveAttribute('download', 'bicycle-bottle-holder.stl');
    await expect(caseStudy.getByRole('link', { name: 'DOWNLOAD BOTTLE HOLDER SOURCE' })).toHaveAttribute(
      'download',
      'bicycle-bottle-holder.ecky',
    );
    await expect(caseStudy.getByRole('link', { name: 'DOWNLOAD FRAME MOUNT RAIL SOURCE' })).toHaveAttribute(
      'download',
      'bottle-holder-frame-mount-rail.ecky',
    );

    await models.getByRole('button', { name: /PHONE CASE/ }).click();
    await expect(workbench).toHaveAttribute('data-selected-variant', 'iphone-case');
    await expect(models.getByRole('button', { name: /PHONE CASE/ })).toHaveAttribute('aria-pressed', 'true');
    await expect.poll(async () => workbench.locator('.viewer-load').count(), {
      message: 'phone case STL finished loading',
      timeout: 15_000,
    }).toBe(0);
    await expect(caseStudy.getByRole('link', { name: 'DOWNLOAD TPU CASE STL' })).toHaveAttribute(
      'download',
      'iphone-17e-voronoi-case.stl',
    );
    await expect(caseStudy.getByRole('link', { name: 'DOWNLOAD INNER PETG ISLAND STL' })).toHaveAttribute(
      'download',
      'iphone-17e-camera-inner-island-petg.stl',
    );
    await expect(caseStudy.getByRole('link', { name: 'DOWNLOAD OUTER PETG SNAP ISLAND STL' })).toHaveAttribute(
      'download',
      'iphone-17e-camera-outer-snap-island-petg.stl',
    );
  });

  test('Given finished print sets When the gallery is read Then every model names its complete mating set', async ({ page }) => {
    await page.goto('/');

    const workbench = page.getByTestId('model-workbench');
    const models = workbench.getByRole('group', { name: 'Working models' });
    await expect(workbench.getByText('PICK A MODEL', { exact: true })).toBeVisible();
    await expect(models.getByRole('button', { name: /BOTTLE HOLDER/ })).toContainText('Two source threads');
    await expect(models.getByRole('button', { name: /GILLETTE KIT/ })).toContainText('Complete 3-print set');
    await expect(models.getByRole('button', { name: /FILM SCANNER/ })).toContainText('Complete 6-print set');
    await expect(models.getByRole('button', { name: /PHONE CASE/ })).toContainText('Complete 3-print set');
    await expect(workbench.getByText(/component/i)).toHaveCount(0);

    await expect(workbench).toHaveAttribute('data-selected-variant', 'bicycle-bottle-holder');
    await expect(page.getByRole('link', { name: 'DOWNLOAD BOTTLE HOLDER STL' })).toHaveAttribute(
      'download',
      'bicycle-bottle-holder.stl',
    );
    await expect(page.getByRole('link', { name: 'DOWNLOAD FRAME MOUNT RAIL STL' })).toHaveAttribute(
      'download',
      'bottle-holder-frame-mount-rail.stl',
    );
  });

  test('Given canonical source When CODE opens Then the complete highlighted source is readable, copyable, and keyboard dismissible', async ({ page, context }) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(`pageerror: ${error.message}`));
    page.on('console', (message) => {
      if (message.type() === 'error') errors.push(message.text());
    });
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await page.goto('/');

    const codeButton = page.getByTestId('model-workbench').getByRole('button', { name: 'SEE CODE' });
    await codeButton.click();

    const inspector = page.getByRole('dialog', { name: 'Macro Inspector: Bicycle bottle holder + frame mount rail' });
    await expect(inspector).toBeVisible();
    // Static source: inspectable, highlighted, and deliberately not editable.
    const source = inspector.getByTestId('case-source');
    await expect(source).toBeVisible();
    await expect(source.getByTestId('source-line-number').first()).toHaveText('1');
    await expect(source.locator('.source-token--keyword').filter({ hasText: 'model' })).toBeVisible();
    await expect(source.locator('.source-token--name').filter({ hasText: 'dovetail_clearance' })).toBeVisible();
    await expect(source.locator('.source-token--string').filter({ hasText: 'BottleCage' })).toBeVisible();
    await expect(inspector.getByRole('textbox')).toHaveCount(0);
    await expect(inspector.getByRole('button', { name: 'APPLY' })).toHaveCount(0);
    await expect.poll(async () => Number(await source.getAttribute('data-source-length'))).toBeGreaterThan(300);

    await inspector.getByRole('button', { name: 'COPY CODE' }).click();
    await expect(inspector.getByRole('button', { name: 'COPIED' })).toBeVisible();
    await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toContain('(part "BottleCage"');

    await page.keyboard.press('Escape');
    await expect(inspector).toBeHidden();
    await expect(codeButton).toBeFocused();
    expect(errors, 'source inspector opens without browser errors').toEqual([]);
  });

  test('Given complete mating models When source opens Then both bottle threads and phone lattice stay inspectable', async ({ page }) => {
    await page.goto('/');

    const workbench = page.getByTestId('model-workbench');
    const models = workbench.getByRole('group', { name: 'Working models' });

    await workbench.getByRole('button', { name: 'SEE CODE' }).click();
    let inspector = page.getByRole('dialog', { name: 'Macro Inspector: Bicycle bottle holder + frame mount rail' });
    let source = inspector.getByTestId('case-source');
    await expect(source).toContainText('(part "BottleCage"');
    await expect(source).toContainText('(repeat-union cell 18');
    await inspector.getByRole('button', { name: 'FRAME MOUNT RAIL SOURCE' }).click();
    source = inspector.getByTestId('case-source');
    await expect(source).toContainText('(part "FrameMountRail"');
    await expect(source).toContainText('(number rail_length');
    await inspector.getByRole('button', { name: 'CLOSE CODE' }).click();

    await models.getByRole('button', { name: /PHONE CASE/ }).click();
    await workbench.getByRole('button', { name: 'SEE CODE' }).click();
    inspector = page.getByRole('dialog', { name: 'Macro Inspector: iPhone 17e — warped-Voronoi TPU + PETG camera island' });
    source = inspector.getByTestId('case-source');
    await expect(source).toContainText('(define-component lattice-strut');
    await expect(source).toContainText('(part iphone-17e-tpu-case');
    await expect(source).toContainText('(part camera-cluster-inner-island-petg');
    await expect(source).toContainText('(part camera-cluster-outer-snap-island-petg');
  });

  test('Given the bottle-holder pair is ready When the visitor pauses Then the inspection view stays stable', async ({ page }) => {
    await page.goto('/');

    const workbench = page.getByTestId('model-workbench');
    await expect.poll(async () => workbench.locator('.viewer-load').count(), {
      message: 'bottle holder STLs finished loading',
      timeout: 15_000,
    }).toBe(0);

    const canvas = workbench.locator('.viewer canvas');
    const firstFrame = await canvas.screenshot();
    await page.waitForTimeout(700);
    const secondFrame = await canvas.screenshot();
    expect(secondFrame, 'model does not rotate away before the visitor drags').toEqual(firstFrame);
  });

  test('Given the STL request fails When the viewer settles Then raw asset context replaces pending and retry recovers', async ({ page }) => {
    await page.route('**/*.stl', (route) => route.abort('failed'));
    await page.goto('/');

    const workbench = page.getByTestId('model-workbench');
    const failure = workbench.getByRole('alert');
    await expect(failure).toContainText(/(?:bicycle-bottle-holder|bottle-holder-frame-mount-rail)\.stl/);
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

    const workbench = page.getByTestId('model-workbench');
    const viewer = workbench.locator('.viewer');
    await expect(viewer).toBeVisible();
    const viewerBox = await viewer.boundingBox();
    expect(viewerBox?.width, 'responsive viewer width').toBeGreaterThan(0);
    expect(viewerBox?.width, 'responsive viewer fits narrow page').toBeLessThanOrEqual(390);

    const codeButton = workbench.getByRole('button', { name: 'SEE CODE' });
    await codeButton.click();
    const inspector = page.getByRole('dialog', { name: 'Macro Inspector: Bicycle bottle holder + frame mount rail' });
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
    await expect(inspector.getByRole('link', { name: 'DOWNLOAD BOTTLE HOLDER SOURCE' })).toBeVisible();

    await page.keyboard.press('Escape');
    await expect(inspector).toBeHidden();
    await expect(codeButton).toBeFocused();
    await expect.poll(() => page.evaluate(() => getComputedStyle(document.documentElement).overflow)).not.toBe('hidden');
  });
});
