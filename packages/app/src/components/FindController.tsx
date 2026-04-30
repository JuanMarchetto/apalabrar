// Phase 4.5 — FindController.
//
// Owns find-state (query, options, matches, current index) and wires
// the parent-controlled `<FindReplaceBar />` to an injected `find`
// function (typically `ApalabrarCore.find` bound to a CoreDocId).
//
// The `find` function is injected (DI) rather than reaching into the
// wasm core directly so component tests can drive it with a fake
// without touching the real runtime.

import type { FindOptions, Match } from '@apalabrar/editor-bridge';
import { FindReplaceBar } from '@apalabrar/ui';
import { type Component, createEffect, createSignal } from 'solid-js';

export interface FindControllerProps {
  /**
   * Search the doc for `needle`. Called when query or options change.
   * Bind this to `ApalabrarCore.find(docId, ...)` in production.
   */
  find: (needle: string, opts: FindOptions) => Match[];
  /** Optional initial query (useful for tests). */
  initialQuery?: string;
  /** Optional close handler (closes the bar). */
  onClose?: () => void;
}

export const FindController: Component<FindControllerProps> = (props) => {
  const [findQuery, setFindQuery] = createSignal(props.initialQuery ?? '');
  const [replaceQuery, setReplaceQuery] = createSignal('');
  const [caseSensitive, setCaseSensitive] = createSignal(false);
  const [wholeWord, setWholeWord] = createSignal(false);
  const [matches, setMatches] = createSignal<Match[]>([]);
  const [currentMatch, setCurrentMatch] = createSignal<number | undefined>(undefined);

  // Recompute matches whenever query or options change. Empty query
  // clears the result set (UI shows "0 of 0") without calling the
  // engine — matches the "don't enumerate cursor positions" rule from
  // the engine's locked semantics.
  createEffect(() => {
    const q = findQuery();
    if (q === '') {
      setMatches([]);
      setCurrentMatch(undefined);
      return;
    }
    const result = props.find(q, {
      caseSensitive: caseSensitive(),
      wholeWord: wholeWord(),
    });
    setMatches(result);
    setCurrentMatch(result.length > 0 ? 0 : undefined);
  });

  const next = () => {
    const m = matches();
    if (m.length === 0) return;
    const cur = currentMatch() ?? -1;
    setCurrentMatch((cur + 1) % m.length);
  };

  const prev = () => {
    const m = matches();
    if (m.length === 0) return;
    const cur = currentMatch() ?? 0;
    setCurrentMatch((cur - 1 + m.length) % m.length);
  };

  return (
    <FindReplaceBar
      findQuery={findQuery()}
      replaceQuery={replaceQuery()}
      matchCount={matches().length}
      currentMatch={currentMatch()}
      caseSensitive={caseSensitive()}
      wholeWord={wholeWord()}
      onFindQueryChange={setFindQuery}
      onReplaceQueryChange={setReplaceQuery}
      onCaseSensitiveChange={setCaseSensitive}
      onWholeWordChange={setWholeWord}
      onNext={next}
      onPrev={prev}
      onClose={props.onClose}
    />
  );
};
