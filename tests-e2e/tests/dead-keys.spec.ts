import { expect, test } from '@playwright/test';

// Dead-key composition gate — Spanish (es-AR, es-ES) and Portuguese (pt-BR).
//
// Strategy: dispatch CompositionEvents directly via page.evaluate. We can't
// rely on Playwright's keyboard.press to trigger native dead-key composition
// consistently across browsers + OSes (the layout is OS-level and headless
// runners don't expose es-AR / pt-BR layouts). Driving the JS event surface
// directly verifies that the editor's compositionend handler commits the
// final text — which is what the GO/NO-GO gate is actually about.
//
// The "layout" labels are documentation: the dead-key sequences a user types
// to produce each character on each layout. The events the browser fires are
// the same shape regardless of layout.

const compose = async (
  page: import('@playwright/test').Page,
  finalChar: string,
  intermediates: string[] = [],
) => {
  await page.evaluate(
    ({ finalChar, intermediates }) => {
      const el = document.querySelector<HTMLElement>('[data-testid=composing-editor]');
      if (!el) throw new Error('composing-editor not mounted');
      el.focus();
      el.dispatchEvent(new CompositionEvent('compositionstart'));
      for (const partial of intermediates) {
        el.dispatchEvent(new CompositionEvent('compositionupdate', { data: partial }));
      }
      el.dispatchEvent(new CompositionEvent('compositionend', { data: finalChar }));
    },
    { finalChar, intermediates },
  );
};

test.describe('dead-key composition (es-AR, es-ES, pt-BR)', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/composer');
    await expect(page.getByTestId('composing-editor')).toBeVisible();
  });

  // ─── es-AR layout: ' is the acute dead key ──────────────────────────────

  test('es-AR: ´ + a → á', async ({ page }) => {
    await compose(page, 'á', ['á']);
    await expect(page.getByTestId('composing-editor')).toHaveText('á');
  });

  test('es-AR: ´ + e → é', async ({ page }) => {
    await compose(page, 'é', ['é']);
    await expect(page.getByTestId('composing-editor')).toHaveText('é');
  });

  test('es-AR: ´ + i → í', async ({ page }) => {
    await compose(page, 'í', ['í']);
    await expect(page.getByTestId('composing-editor')).toHaveText('í');
  });

  test('es-AR: ´ + o → ó', async ({ page }) => {
    await compose(page, 'ó', ['ó']);
    await expect(page.getByTestId('composing-editor')).toHaveText('ó');
  });

  test('es-AR: ´ + u → ú', async ({ page }) => {
    await compose(page, 'ú', ['ú']);
    await expect(page.getByTestId('composing-editor')).toHaveText('ú');
  });

  // ─── es-ES layout: ñ is on its own key, ¨ via Shift+´ ────────────────────

  test('es-ES: ñ as a direct key', async ({ page }) => {
    await compose(page, 'ñ');
    await expect(page.getByTestId('composing-editor')).toHaveText('ñ');
  });

  test('es-ES: ¨ + u → ü', async ({ page }) => {
    await compose(page, 'ü', ['ü']);
    await expect(page.getByTestId('composing-editor')).toHaveText('ü');
  });

  // ─── pt-BR (ABNT2) layout: ´ + c → ç, ~ + a → ã ──────────────────────────

  test('pt-BR: ´ + c → ç', async ({ page }) => {
    await compose(page, 'ç', ['ç']);
    await expect(page.getByTestId('composing-editor')).toHaveText('ç');
  });

  test('pt-BR: ~ + a → ã', async ({ page }) => {
    await compose(page, 'ã', ['ã']);
    await expect(page.getByTestId('composing-editor')).toHaveText('ã');
  });

  // ─── full sentence smoke ────────────────────────────────────────────────

  test('typing a full Spanish sentence preserves all accents', async ({ page }) => {
    // "Año nuevo, mañana es el día 1°"
    const sentence = 'Año nuevo, mañana es el día 1°';
    await compose(page, sentence);
    await expect(page.getByTestId('composing-editor')).toHaveText(sentence);
  });
});
