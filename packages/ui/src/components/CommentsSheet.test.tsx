// Phase 4.6 RED — CommentsSheet visibility behaviour.
//
// The component is a controlled wrapper around kobalte Dialog. Tests
// drive the `open` prop and assert that:
//   - children render only when open=true
//   - onOpenChange fires on Esc / dismiss
//   - the dialog has an accessible name (label prop)

import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { CommentsSheet } from './CommentsSheet';

afterEach(cleanup);

describe('CommentsSheet', () => {
  it('does not render its children when open is false', () => {
    render(() => (
      <CommentsSheet open={false}>
        <div data-testid='sheet-body'>body</div>
      </CommentsSheet>
    ));
    expect(screen.queryByTestId('sheet-body')).toBeNull();
  });

  it('renders its children when open is true', () => {
    render(() => (
      <CommentsSheet open={true}>
        <div data-testid='sheet-body'>body</div>
      </CommentsSheet>
    ));
    expect(screen.getByTestId('sheet-body')).toBeInTheDocument();
  });

  it('exposes an accessible label (default "Comments")', () => {
    render(() => (
      <CommentsSheet open={true}>
        <div>x</div>
      </CommentsSheet>
    ));
    expect(screen.getByRole('dialog', { name: 'Comments' })).toBeInTheDocument();
  });

  it('uses the provided label prop when given', () => {
    render(() => (
      <CommentsSheet open={true} label='Comentarios'>
        <div>x</div>
      </CommentsSheet>
    ));
    expect(screen.getByRole('dialog', { name: 'Comentarios' })).toBeInTheDocument();
  });

  it('fires onOpenChange(false) when the user presses Escape', () => {
    const onOpenChange = vi.fn();
    render(() => (
      <CommentsSheet open={true} onOpenChange={onOpenChange}>
        <div>x</div>
      </CommentsSheet>
    ));
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
