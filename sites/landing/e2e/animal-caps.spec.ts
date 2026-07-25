import { expect, test } from '@playwright/test';

test.describe('Landing animal cap catalog', () => {
  test('Given a published manifest entry When animal caps render Then the real Pug artifacts are selectable', async ({ page }) => {
    await page.goto('/#animal-caps');

    const section = page.locator('#animal-caps');
    await expect(section).toBeVisible();
    await expect(section.getByRole('heading', { name: 'Animals with engineering problems.' })).toBeVisible();

    const workbench = section.getByTestId('animal-cap-workbench');
    await expect(workbench).toHaveAttribute('data-selected-animal-cap', 'quaternius-pug-presta');
    await expect(workbench.getByText('PUG PRESTA VALVE CAP')).toBeVisible();
    await expect(workbench.getByText('PRESTA-BLIND-BOMB-V1')).toBeVisible();
    await expect(workbench.getByText('CC0-1.0')).toBeVisible();
    await expect(workbench.getByText('UNIFORM SCALE 12')).toBeVisible();

    await expect.poll(async () => workbench.locator('.viewer-load').count(), {
      message: 'published Pug STL finished loading',
      timeout: 15_000,
    }).toBe(0);
    await expect(workbench.locator('.viewer canvas')).toBeVisible();
    await expect(section.getByRole('link', { name: 'DOWNLOAD PUG STL' })).toHaveAttribute(
      'download',
      'pug-presta-valve-cap.stl',
    );
    await expect(section.getByRole('link', { name: 'DOWNLOAD PUG SOURCE' })).toHaveAttribute(
      'download',
      'pug-presta-valve-cap.ecky',
    );
  });

  test('Given the published STL fails When viewer settles Then raw asset name and retry replace pending', async ({ page }) => {
    await page.route('**/pug-presta-valve-cap*.stl', (route) => route.abort('failed'));
    await page.goto('/#animal-caps');

    const workbench = page.getByTestId('animal-cap-workbench');
    const failure = workbench.getByRole('alert');
    await expect(failure).toContainText('pug-presta-valve-cap');
    await expect(workbench.locator('.viewer-load')).toHaveCount(0);

    await page.unroute('**/pug-presta-valve-cap*.stl');
    await workbench.getByRole('button', { name: 'RETRY STL' }).click();
    await expect(failure).toBeHidden();
    await expect.poll(async () => workbench.locator('.viewer-load').count(), {
      message: 'Pug retry loaded',
      timeout: 15_000,
    }).toBe(0);
  });
});
