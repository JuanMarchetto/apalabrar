// Phase 4.7 — SuggestionsController.
//
// Renders one `<SuggestionPill />` per pending suggestion in the doc
// (plus accepted / rejected when `showResolved` is on) and dispatches
// `AcceptSuggestion` / `RejectSuggestion` EditOps via the injected
// `applyEditOp` callback.

import type { EditOp, Suggestion } from '@apalabrar/editor-bridge';
import { SuggestionPill } from '@apalabrar/ui';
import { type Component, For } from 'solid-js';

export interface SuggestionsControllerProps {
  /** All suggestions (pending + historical) read via `core.suggestions(id)`. */
  suggestions: ReadonlyArray<Suggestion>;
  /**
   * Dispatch an EditOp through the wasm core. In production:
   * `(op) => core.applyEditOp(docId, op)`.
   */
  applyEditOp: (op: EditOp) => void;
  /**
   * When `true`, pills for accepted / rejected suggestions are also
   * rendered. Default `false` (only Pending visible).
   */
  showResolved?: boolean | undefined;
}

export const SuggestionsController: Component<SuggestionsControllerProps> = (props) => {
  const visible = () =>
    props.suggestions.filter((s) => props.showResolved || s.state === 'pending');

  const handleAccept = (id: string) =>
    props.applyEditOp({ kind: 'AcceptSuggestion', suggestion_id: id });

  const handleReject = (id: string) =>
    props.applyEditOp({ kind: 'RejectSuggestion', suggestion_id: id });

  return (
    <div data-testid='suggestions-controller'>
      <For each={visible()}>
        {(s) => (
          <SuggestionPill
            id={s.id}
            author={s.author}
            replacement={s.replacement}
            state={s.state}
            onAccept={handleAccept}
            onReject={handleReject}
          />
        )}
      </For>
    </div>
  );
};
