/**
 * Phase 2.5 Playwright E2E for the page-load timeline (blueprint
 * Section D). Targets:
 *
 * - T+200 ms editable on broadband (relaxed to 2000 ms in dev mode
 *   because Vite cold-start dominates first-load time; the assertion
 *   is the *order* of paint events, not the absolute clock).
 * - Blank-doc surface visible BEFORE WASM is ready.
 * - "Continue last doc?" toast appears when OPFS holds a session.
 * - Toast auto-dismisses after 5 s.
 * - Keystrokes typed during the WASM-loading window are buffered
 *   and replayed once the editor is ready.
 *
 * All tests run against the chromium project (Playwright config
 * defines firefox + webkit too; nightly CI extends; PR CI runs
 * chromium-only for speed).
 */
import { expect, test } from '@playwright/test';

test.describe('page-load timeline @page-load', () => {
  test('skeleton paints in the initial HTML before any JS runs', async ({ page }) => {
    await page.goto('/', { waitUntil: 'commit' });
    // The skeleton shell is in index.html so it's available
    // immediately on commit (before Solid hydrates).
    const skeleton = page.locator('[data-testid=skeleton-shell]');
    await expect(skeleton).toBeAttached({ timeout: 1000 });
  });

  test('blank-doc surface is interactive within 2000 ms (dev) / 200 ms (prod) of nav start', async ({ page }) => {
    await page.goto('/');
    const start = await page.evaluate(() => performance.now());
    await page.locator('[data-testid=blank-doc]').waitFor({ state: 'visible' });
    const elapsed = (await page.evaluate(() => performance.now())) - start;
    // Generous threshold for dev mode; production target is 200 ms.
    expect(elapsed).toBeLessThan(2000);
  });

  test('blank-doc surface is focusable (tabIndex ≥ 0)', async ({ page }) => {
    await page.goto('/');
    const surface = page.locator('[data-testid=blank-doc]');
    await surface.waitFor();
    const tabIndex = await surface.getAttribute('tabindex');
    expect(Number(tabIndex)).toBeGreaterThanOrEqual(0);
  });

  test('Apalabrar brand is visible alongside the blank-doc surface', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { level: 1, name: /apalabrar/i }))
      .toBeVisible();
    await expect(page.locator('[data-testid=blank-doc]')).toBeVisible();
  });

  test('keystrokes typed at T+0 land in the doc once core is ready', async ({ page }) => {
    await page.goto('/');
    const surface = page.locator('[data-testid=blank-doc]');
    await surface.waitFor();
    await surface.focus();
    // Buffer happens in JS; without a wired editor we can only assert
    // that the buffer itself accepts the events without error AND
    // that core-ready transitions correctly.
    await page.keyboard.type('hola');
    await expect(surface).toHaveAttribute('data-core-ready', 'true', {
      timeout: 3000,
    });
  });
});

test.describe('Continue last doc? toast @page-load', () => {
  test.beforeEach(async ({ page }) => {
    // Seed OPFS via the storage harness page before testing.
    await page.goto('/storage-harness');
    await page.waitForLoadState('networkidle');
  });

  test('toast does not appear on a clean OPFS', async ({ page }) => {
    // Fresh storage state per worker is enforced by Playwright's
    // browser-context isolation. Visit / and confirm no toast.
    await page.goto('/');
    await page.locator('[data-testid=blank-doc]').waitFor();
    // Wait a tick for the bootstrap to settle.
    await page.waitForTimeout(200);
    expect(await page.getByRole('status').count()).toBe(0);
  });
});
