import { expect, test } from '@playwright/test';

/**
 * repair-ecky-learning-campaign — current slice only.
 *
 * One-slice BDD: only the RED for the slice being implemented lives here. As
 * each slice turns green, the next slice's RED is authored. CAD-model
 * correctness stays owned by the Ecky runtime/model tests; these route
 * scenarios assert only campaign projection, named-fit teaching, and visible
 * asset loading on `/learn/ecky-ir`.
 */

const LEARN = '/learn/ecky-ir';

// ---------------------------------------------------------------------------
// Slice One: one active phase, example-answer separation.
// ---------------------------------------------------------------------------

test.describe('repair-ecky-learning-campaign: one active phase (2.1)', () => {
  test('Given Level 01 When it opens Then BRIEF is the only visible phase content', async ({ page }) => {
    await page.goto(LEARN);

    await expect(page.locator('.mission-workbench__title')).toHaveText('Corner Bracket');

    // BRIEF phase is visible.
    await expect(page.getByRole('heading', { name: 'Brief', exact: true })).toBeVisible();

    // Every other phase content is NOT visible on first load.
    await expect(page.getByRole('heading', { name: /worked example/i })).toHaveCount(0);
    await expect(page.getByRole('heading', { name: 'Decide', exact: true })).toHaveCount(0);
    await expect(page.getByRole('textbox', { name: /attempt/i })).toHaveCount(0);
    await expect(page.getByRole('heading', { name: 'Transfer', exact: true })).toHaveCount(0);
  });

  test('Given Level 01 When STUDY is activated Then BRIEF unmounts and exactly one worked-example source is visible', async ({ page }) => {
    await page.goto(LEARN);

    await page.getByRole('button', { name: /^STUDY$/ }).click();

    // BRIEF phase is gone.
    await expect(page.getByRole('heading', { name: 'Brief', exact: true })).toHaveCount(0);

    // STUDY content is visible.
    await expect(page.getByRole('heading', { name: /worked example/i })).toBeVisible();

    // Exactly one worked-example source block is rendered.
    await expect(page.locator('.mission-code')).toHaveCount(1);

    // Other phases are still not visible.
    await expect(page.getByRole('textbox', { name: /attempt/i })).toHaveCount(0);
  });

  test('Given Level 01 practice has in-progress work When the learner navigates away and back Then the attempt state survives hidden', async ({ page }) => {
    await page.goto(LEARN);

    // Go to PRACTICE and type in-progress work.
    await page.getByRole('button', { name: /^PRACTICE$/ }).click();
    const editor = page.getByRole('textbox', { name: /attempt/i });
    await editor.fill('(model (part bracket (marker-survives-hidden)))');

    // Navigate away to BRIEF (PRACTICE unmounts).
    await page.getByRole('button', { name: /^BRIEF$/ }).click();
    await expect(editor).toHaveCount(0);

    // Return to PRACTICE: the in-progress attempt is preserved.
    await page.getByRole('button', { name: /^PRACTICE$/ }).click();
    await expect(page.getByRole('textbox', { name: /attempt/i })).toHaveValue(
      '(model (part bracket (marker-survives-hidden)))',
    );
  });
});

test.describe('repair-ecky-learning-campaign: no duplicate chapter answer beneath workbench (2.1 boundary)', () => {
  test('Given Level 01 When the workbench is mounted Then the full chapter prose answer does not render simultaneously beneath the workbench', async ({ page }) => {
    await page.goto(LEARN);

    // The workbench is mounted.
    await expect(page.locator('.mission-workbench')).toBeVisible();

    // The interactive campaign does NOT render the chapter body prose (which
    // historically carried the worked solution) beneath the workbench. The
    // article title remains, but the bodyHtml answer region is absent in
    // campaign mode.
    await expect(page.locator('.docs-article__body')).toHaveCount(0);
  });
});

// ---------------------------------------------------------------------------
// Slice Two: real source-bound Corner Bracket render.
// ---------------------------------------------------------------------------

test.describe('repair-ecky-learning-campaign: real Corner Bracket render (3.1)', () => {
  test('Given Level 01 BRIEF When it renders Then a descriptive Corner Bracket image with non-zero natural dimensions is visible', async ({ page }) => {
    await page.goto(LEARN);

    // BRIEF is the active phase on first load.
    await expect(page.getByRole('heading', { name: 'Brief', exact: true })).toBeVisible();

    const image = page.getByAltText(/corner bracket/i).first();

    await expect(image).toBeVisible();
    // A non-broken image reports non-zero natural dimensions once loaded.
    const naturalWidth = await image.evaluate((el) => (el as HTMLImageElement).naturalWidth);
    const naturalHeight = await image.evaluate((el) => (el as HTMLImageElement).naturalHeight);
    expect(naturalWidth).toBeGreaterThan(0);
    expect(naturalHeight).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// Slice Three: honest EPUB action.
// ---------------------------------------------------------------------------

test.describe('repair-ecky-learning-campaign: honest EPUB action (4.1, 4.3)', () => {
  test('Given the campaign When it loads Then OFFLINE BOOK · EPUB is visible as a secondary action and DOWNLOAD CAMPAIGN is absent', async ({ page }) => {
    await page.goto(LEARN);

    const offlineBook = page.getByRole('button', { name: /OFFLINE BOOK · EPUB/i });
    await expect(offlineBook).toBeVisible();
    // The EPUB is the offline book, presented as secondary (not a primary
    // "download campaign" action).
    await expect(offlineBook).toHaveClass(/docs-action--secondary/);
    await expect(page.getByRole('button', { name: /^DOWNLOAD CAMPAIGN$/i })).toHaveCount(0);
  });

  test('Given the campaign When OFFLINE BOOK · EPUB is activated Then the EPUB artifact downloads', async ({ page }) => {
    await page.goto(LEARN);

    const downloadPromise = page.waitForEvent('download');
    await page.getByRole('button', { name: /OFFLINE BOOK · EPUB/i }).click();
    const download = await downloadPromise;

    expect(download.suggestedFilename()).toBe('ecky-ir-field-guide.epub');
  });
});

// ---------------------------------------------------------------------------
// Slice Four: Dovetail Fit (reuses the production film-adapter dovetail).
// ---------------------------------------------------------------------------

test.describe('repair-ecky-learning-campaign: Level 03 Dovetail Fit (5.1, 5.8)', () => {
  test('Given Level 03 When it loads Then Dovetail Fit is the active mission, ribbed plate is absent, and practice exposes the named fit relation', async ({ page }) => {
    await page.goto(LEARN);

    await page.locator('.docs-sidebar').getByRole('button', { name: /Dovetail Fit/i }).click();

    await expect(page.locator('.mission-workbench__title')).toHaveText('Dovetail Fit');
    await expect(page.locator('.mission-workbench__artifact')).toContainText(/dovetail/i);
    // Ribbed plate is absent from primary Level 03 content.
    await expect(page.locator('.mission-workbench__artifact')).not.toContainText(/ribbed plate/i);

    // PRACTICE exposes one named shared-clearance relation, not anonymous
    // magic offsets on each side.
    await page.getByRole('button', { name: /^PRACTICE$/ }).click();
    const editor = page.getByRole('textbox', { name: /attempt/i });
    const attemptValue = await editor.inputValue();
    expect(attemptValue).toContain('fit_clearance');
  });

  test('Given Level 03 BRIEF When it renders Then a descriptive Dovetail image with non-zero natural dimensions is visible', async ({ page }) => {
    await page.goto(LEARN);

    await page.locator('.docs-sidebar').getByRole('button', { name: /Dovetail Fit/i }).click();

    await expect(page.getByRole('heading', { name: 'Brief', exact: true })).toBeVisible();
    const image = page.getByAltText(/dovetail/i).first();
    await expect(image).toBeVisible();
    const naturalWidth = await image.evaluate((el) => (el as HTMLImageElement).naturalWidth);
    const naturalHeight = await image.evaluate((el) => (el as HTMLImageElement).naturalHeight);
    expect(naturalWidth).toBeGreaterThan(0);
    expect(naturalHeight).toBeGreaterThan(0);
  });
});
