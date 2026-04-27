// IME / dead-key composition handler.
//
// Spec: applyComposition is the single source of truth for transforming the
// editor's text state in response to a CompositionEvent. The browser fires
// `compositionstart` when the user begins composing (e.g., presses a dead
// key), `compositionupdate` zero-or-more times as the tentative result
// changes, and `compositionend` once with the final composed string in
// `evt.data`. Only `compositionend` is observable in the persisted text;
// start/update events are tentative and must not mutate the committed state.
//
// Output is normalized to NFC (Unicode Normalization Form C) so that
// `ñ` is always the precomposed U+00F1, never `n` + U+0303 combining tilde.
// This matters for length comparisons, search, and round-trip with OOXML.

export function applyComposition(prev: string, evt: CompositionEvent): string {
  // Tentative events (compositionstart, compositionupdate) never mutate the
  // committed string; only compositionend commits. Empty data on
  // compositionend means the composition was cancelled (e.g., backspace).
  if (evt.type !== 'compositionend' || !evt.data) {
    return prev;
  }
  return (prev + evt.data).normalize('NFC');
}
