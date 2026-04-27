import * as fc from 'fast-check';
import { describe, expect, it, test } from 'vitest';
import { applyComposition } from './composition';

// --- helpers ----------------------------------------------------------------

const start = (data = ''): CompositionEvent => new CompositionEvent('compositionstart', { data });
const update = (data: string): CompositionEvent =>
  new CompositionEvent('compositionupdate', { data });
const end = (data: string): CompositionEvent => new CompositionEvent('compositionend', { data });

// All target precomposed (NFC) characters: validates that whichever path the
// browser took to compose them, the result is a single code point.
const isSingleCodepoint = (s: string): boolean => [...s].length === 1;

// --- 1: ñ (Spanish) ---------------------------------------------------------

describe('applyComposition — Spanish dead-key compositions', () => {
  it('composes "n" + "˜" into "ñ" as a single NFC code point', () => {
    const result = applyComposition('', end('ñ'));
    expect(result).toBe('ñ');
    expect(isSingleCodepoint(result)).toBe(true);
  });

  // --- 2-6: acutes (á é í ó ú) ---------------------------------------------

  it.each([
    ['a', 'á'],
    ['e', 'é'],
    ['i', 'í'],
    ['o', 'ó'],
    ['u', 'ú'],
  ])('composes "%s" + "´" into "%s" as a single NFC code point', (_letter, expected) => {
    const result = applyComposition('', end(expected));
    expect(result).toBe(expected);
    expect(isSingleCodepoint(result)).toBe(true);
  });

  // --- 7: diaeresis ---------------------------------------------------------

  it('composes "u" + "¨" into "ü" as a single NFC code point', () => {
    const result = applyComposition('', end('ü'));
    expect(result).toBe('ü');
    expect(isSingleCodepoint(result)).toBe(true);
  });
});

describe('applyComposition — Portuguese dead-key compositions', () => {
  // --- 8: ç (PT-BR) ---------------------------------------------------------

  it('composes "c" + "¸" into "ç" as a single NFC code point (PT-BR)', () => {
    const result = applyComposition('', end('ç'));
    expect(result).toBe('ç');
    expect(isSingleCodepoint(result)).toBe(true);
  });

  // --- 9: ã (PT-BR) ---------------------------------------------------------

  it('composes "a" + "˜" into "ã" as a single NFC code point (PT-BR)', () => {
    const result = applyComposition('', end('ã'));
    expect(result).toBe('ã');
    expect(isSingleCodepoint(result)).toBe(true);
  });
});

describe('applyComposition — edge cases', () => {
  // --- 10: empty composition ------------------------------------------------

  it('returns prev string unchanged when compositionend data is empty', () => {
    expect(applyComposition('café', end(''))).toBe('café');
    expect(applyComposition('', end(''))).toBe('');
  });

  // --- 11: insertion when prev has no preceding char -----------------------

  it('inserts the composed char at cursor position when prev is empty', () => {
    const result = applyComposition('', end('é'));
    expect(result).toBe('é');
  });

  // --- 12: backspace cancels a started composition --------------------------

  it('cancels a started composition when compositionend fires with empty data', () => {
    // Backspace during composition: browser fires compositionend with data="".
    let s = 'foo';
    s = applyComposition(s, start('a'));
    s = applyComposition(s, update('á'));
    s = applyComposition(s, end(''));
    expect(s).toBe('foo');
  });

  // --- bonus: NFD input must be normalized to NFC --------------------------
  //
  // Some browsers (notably older Safari) emit decomposed combining marks in
  // evt.data. The contract says the persisted string is always NFC. This is
  // not a separately-listed bullet but is the substantive content of what the
  // gate is verifying — keeping it explicit so the GREEN implementer can see
  // it from the tests alone.

  it('normalizes NFD composition input to NFC ("n" + U+0303 → "ñ")', () => {
    const decomposed = 'ñ'; // 2 code points
    expect([...decomposed].length).toBe(2);

    const result = applyComposition('', end(decomposed));
    expect(result).toBe('ñ');
    expect(isSingleCodepoint(result)).toBe(true);
  });

  // --- bonus: append, not replace ------------------------------------------
  //
  // Composition commits at the cursor (end of prev for v0). The committed
  // characters do NOT replace prev; they extend it. Captures the contract
  // that ñ doesn't mean "the n in prev becomes ñ", it means "the user just
  // committed ñ and we extend prev with it".

  it('appends the composed char after existing text', () => {
    const result = applyComposition('Año nuevo, ', end('mañana'));
    expect(result).toBe('Año nuevo, mañana');
  });
});

// --- property test ---------------------------------------------------------

describe('applyComposition — invariants', () => {
  test('a full compositionstart→update*→end sequence yields the same string as a single end with the final data', () => {
    fc.assert(
      fc.property(
        fc.array(
          fc.string({ minLength: 0, maxLength: 4, unit: fc.constantFrom('a', 'b', 'á', 'ñ', 'ç') }),
          { minLength: 0, maxLength: 5 },
        ),
        fc.string({ minLength: 1, maxLength: 3, unit: fc.constantFrom('a', 'á', 'ñ', 'ç', 'é') }),
        fc.string({ minLength: 0, maxLength: 8, unit: fc.constantFrom('a', ' ', 'A', 'ñ') }),
        (intermediates, finalText, prev) => {
          // Method A: full event sequence the browser would fire.
          let viaSequence = prev;
          viaSequence = applyComposition(viaSequence, start());
          for (const partial of intermediates) {
            viaSequence = applyComposition(viaSequence, update(partial));
          }
          viaSequence = applyComposition(viaSequence, end(finalText));

          // Method B: a single compositionend with the final text.
          const direct = applyComposition(prev, end(finalText));

          // Invariant: only compositionend with non-empty data mutates the
          // committed string. Tentative start/update events leave it untouched.
          return viaSequence === direct;
        },
      ),
      { numRuns: 100 },
    );
  });
});
