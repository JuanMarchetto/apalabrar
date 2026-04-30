// Phase 4.7 RED — SuggestionPill visual contract.

import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { SuggestionPill } from './SuggestionPill';

afterEach(cleanup);

const baseProps = {
  id: 's-1',
  author: 'alice',
  replacement: 'WORLD',
  state: 'pending' as const,
};

describe('SuggestionPill', () => {
  it('renders an article landmark with the suggestion id as data attribute', () => {
    render(() => <SuggestionPill {...baseProps} />);
    const article = screen.getByRole('article');
    expect(article).toHaveAttribute('data-suggestion-id', 's-1');
  });

  it('shows the author and the proposed replacement text', () => {
    render(() => <SuggestionPill {...baseProps} />);
    expect(screen.getByText('alice')).toBeInTheDocument();
    expect(screen.getByText('WORLD')).toBeInTheDocument();
  });

  it('renders Accept and Reject buttons when state is pending', () => {
    render(() => <SuggestionPill {...baseProps} />);
    expect(screen.getByRole('button', { name: 'Accept' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reject' })).toBeInTheDocument();
  });

  it('clicking Accept fires onAccept with the suggestion id', () => {
    const onAccept = vi.fn();
    render(() => <SuggestionPill {...baseProps} onAccept={onAccept} />);
    fireEvent.click(screen.getByRole('button', { name: 'Accept' }));
    expect(onAccept).toHaveBeenCalledWith('s-1');
  });

  it('clicking Reject fires onReject with the suggestion id', () => {
    const onReject = vi.fn();
    render(() => <SuggestionPill {...baseProps} onReject={onReject} />);
    fireEvent.click(screen.getByRole('button', { name: 'Reject' }));
    expect(onReject).toHaveBeenCalledWith('s-1');
  });

  it('disables Accept and Reject buttons when state is not pending', () => {
    render(() => <SuggestionPill {...baseProps} state='accepted' />);
    expect(screen.getByRole('button', { name: 'Accept' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Reject' })).toBeDisabled();
  });
});
