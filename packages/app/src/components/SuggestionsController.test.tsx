// Phase 4.7 RED — SuggestionsController integration.
//
// The controller renders one `<SuggestionPill />` per pending
// suggestion and dispatches AcceptSuggestion / RejectSuggestion
// EditOps via injected `applyEditOp`.

import type { EditOp, Suggestion } from '@apalabrar/editor-bridge';
import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { SuggestionsController } from './SuggestionsController';

afterEach(cleanup);

const pending = (id: string, overrides: Partial<Suggestion> = {}): Suggestion => ({
  id,
  from: 0,
  to: 5,
  replacement: 'X',
  state: 'pending',
  author: 'alice',
  created_at: 1_000,
  ...overrides,
});

describe('SuggestionsController', () => {
  it('renders one pill per pending suggestion', () => {
    render(() => (
      <SuggestionsController
        suggestions={[pending('s-1'), pending('s-2')]}
        applyEditOp={vi.fn()}
      />
    ));
    expect(screen.getAllByRole('article')).toHaveLength(2);
  });

  it('renders no pills for an empty suggestions array', () => {
    render(() => <SuggestionsController suggestions={[]} applyEditOp={vi.fn()} />);
    expect(screen.queryAllByRole('article')).toHaveLength(0);
  });

  it('hides accepted suggestions by default', () => {
    render(() => (
      <SuggestionsController
        suggestions={[pending('s-1', { state: 'accepted' })]}
        applyEditOp={vi.fn()}
      />
    ));
    expect(screen.queryAllByRole('article')).toHaveLength(0);
  });

  it('hides rejected suggestions by default', () => {
    render(() => (
      <SuggestionsController
        suggestions={[pending('s-1', { state: 'rejected' })]}
        applyEditOp={vi.fn()}
      />
    ));
    expect(screen.queryAllByRole('article')).toHaveLength(0);
  });

  it('shows resolved suggestions when showResolved is true', () => {
    render(() => (
      <SuggestionsController
        suggestions={[
          pending('s-1', { state: 'accepted' }),
          pending('s-2', { state: 'rejected' }),
        ]}
        applyEditOp={vi.fn()}
        showResolved={true}
      />
    ));
    expect(screen.getAllByRole('article')).toHaveLength(2);
  });

  it('clicking Accept dispatches AcceptSuggestion with the suggestion id', () => {
    const applyEditOp = vi.fn<(op: EditOp) => void>();
    render(() => (
      <SuggestionsController
        suggestions={[pending('s-1')]}
        applyEditOp={applyEditOp}
      />
    ));
    fireEvent.click(screen.getByRole('button', { name: 'Accept' }));
    expect(applyEditOp).toHaveBeenCalledWith({
      kind: 'AcceptSuggestion',
      suggestion_id: 's-1',
    });
  });

  it('clicking Reject dispatches RejectSuggestion with the suggestion id', () => {
    const applyEditOp = vi.fn<(op: EditOp) => void>();
    render(() => (
      <SuggestionsController
        suggestions={[pending('s-1')]}
        applyEditOp={applyEditOp}
      />
    ));
    fireEvent.click(screen.getByRole('button', { name: 'Reject' }));
    expect(applyEditOp).toHaveBeenCalledWith({
      kind: 'RejectSuggestion',
      suggestion_id: 's-1',
    });
  });

  it('renders the author of each pending suggestion', () => {
    render(() => (
      <SuggestionsController
        suggestions={[pending('s-1', { author: 'bob' })]}
        applyEditOp={vi.fn()}
      />
    ));
    expect(screen.getByText('bob')).toBeInTheDocument();
  });

  it('disables buttons on resolved suggestions when showResolved is on', () => {
    render(() => (
      <SuggestionsController
        suggestions={[pending('s-1', { state: 'accepted' })]}
        applyEditOp={vi.fn()}
        showResolved={true}
      />
    ));
    expect(screen.getByRole('button', { name: 'Accept' })).toBeDisabled();
  });
});
