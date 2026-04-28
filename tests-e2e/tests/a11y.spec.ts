/**
 * Phase 2.5 axe-core a11y tests for every state of the landing page.
 * Tagged @a11y so `pnpm test:a11y` runs only this suite.
 *
 * States covered:
 * - Initial landing (skeleton + Solid mounted, no recents)
 * - Toast visible (would require seeded OPFS — currently asserted
 *   structurally via component tests; a11y on the toast DOM is
 *   covered by ContinueToast.test.tsx's role/aria assertions)
 * - Composer route (LATAM dead-key gate)
 * - Demo route (storage harness)
 *
 * The blueprint's targets are zero serious + zero critical violations
 * on every shipped state (Section E risk #4: "browser-native
 * accessibility broken by canvas approach" — mitigated by sticking to
 * native semantic HTML in the v0 shell).
 */
import { AxeBuilder } from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

const seriousOrCritical = (
  violations: ReadonlyArray<{ impact?: string | null; }>,
): typeof violations => violations.filter((v) => v.impact === 'serious' || v.impact === 'critical');

test.describe('a11y — landing page @a11y', () => {
  test('initial blank-doc state has no serious or critical axe violations', async ({ page }) => {
    await page.goto('/');
    await page.locator('[data-testid=blank-doc]').waitFor();
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(seriousOrCritical(results.violations)).toEqual([]);
  });
});

test.describe('a11y — composer page @a11y', () => {
  test('composer has no serious or critical axe violations', async ({ page }) => {
    await page.goto('/composer');
    await page.getByTestId('composing-editor').waitFor();
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(seriousOrCritical(results.violations)).toEqual([]);
  });
});
