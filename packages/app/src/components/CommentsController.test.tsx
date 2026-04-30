// Phase 4.6 RED — CommentsController integration.
//
// The controller wires the parent-controlled `<CommentsSidebar />`
// (rendered inside a `<CommentsSheet />`) to the editor-bridge wasm
// core via an injected `applyEditOp` callback. Tests assert the
// dispatch shape (kind + thread_id + author + status / body fields)
// rather than touching wasm.

import type { Comment, EditOp } from '@apalabrar/editor-bridge';
import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { CommentsController } from './CommentsController';

afterEach(cleanup);

const baseThread: Comment = {
  thread_id: 't-1',
  from: 4,
  to: 9,
  body: 'fix this',
  author: 'alice',
  created_at: 1_700_000_000_000,
  status: 'open',
  replies: [],
};

describe('CommentsController', () => {
  it('renders the empty-state message when threads is empty', () => {
    render(() => (
      <CommentsController
        threads={[]}
        author='daisy'
        open={true}
        applyEditOp={vi.fn()}
      />
    ));
    expect(screen.getByText(/no comments yet/i)).toBeInTheDocument();
  });

  it('renders one article per thread inside the sidebar', () => {
    render(() => (
      <CommentsController
        threads={[
          baseThread,
          { ...baseThread, thread_id: 't-2', body: 'second' },
        ]}
        author='daisy'
        open={true}
        applyEditOp={vi.fn()}
      />
    ));
    expect(screen.getAllByRole('article')).toHaveLength(2);
  });

  it('clicking jump-to-anchor fires onScrollToPosition with the thread from-position', () => {
    const onScrollToPosition = vi.fn();
    render(() => (
      <CommentsController
        threads={[baseThread]}
        author='daisy'
        open={true}
        applyEditOp={vi.fn()}
        onScrollToPosition={onScrollToPosition}
      />
    ));
    fireEvent.click(screen.getByRole('button', { name: 'Jump to anchor' }));
    expect(onScrollToPosition).toHaveBeenCalledWith(4);
  });

  it('clicking resolve dispatches SetCommentStatus(thread_id, "resolved")', () => {
    const applyEditOp = vi.fn<(op: EditOp) => void>();
    render(() => (
      <CommentsController
        threads={[baseThread]}
        author='daisy'
        open={true}
        applyEditOp={applyEditOp}
      />
    ));
    fireEvent.click(screen.getByRole('button', { name: 'Resolve' }));
    expect(applyEditOp).toHaveBeenCalledWith({
      kind: 'SetCommentStatus',
      thread_id: 't-1',
      status: 'resolved',
    });
  });

  it('resolved threads disable the Resolve button', () => {
    render(() => (
      <CommentsController
        threads={[{ ...baseThread, status: 'resolved' }]}
        author='daisy'
        open={true}
        applyEditOp={vi.fn()}
      />
    ));
    expect(screen.getByRole('button', { name: 'Resolve' })).toBeDisabled();
  });

  it('submitting a reply dispatches ReplyToComment with body, author, and now()', () => {
    const applyEditOp = vi.fn<(op: EditOp) => void>();
    const now = vi.fn(() => 9_999);
    render(() => (
      <CommentsController
        threads={[baseThread]}
        author='daisy'
        open={true}
        applyEditOp={applyEditOp}
        now={now}
      />
    ));
    fireEvent.input(screen.getByLabelText('Reply text'), {
      target: { value: 'agreed' },
    });
    fireEvent.submit(screen.getByLabelText('Reply'));
    expect(applyEditOp).toHaveBeenCalledWith({
      kind: 'ReplyToComment',
      thread_id: 't-1',
      body: 'agreed',
      author: 'daisy',
      created_at: 9_999,
    });
  });

  it('does not dispatch on empty reply body submit', () => {
    const applyEditOp = vi.fn();
    render(() => (
      <CommentsController
        threads={[baseThread]}
        author='daisy'
        open={true}
        applyEditOp={applyEditOp}
      />
    ));
    fireEvent.submit(screen.getByLabelText('Reply'));
    expect(applyEditOp).not.toHaveBeenCalled();
  });

  it('renders existing replies under the parent thread', () => {
    render(() => (
      <CommentsController
        threads={[
          {
            ...baseThread,
            replies: [
              { id: 'r-1', author: 'bob', body: 'agreed', created_at: 1 },
            ],
          },
        ]}
        author='daisy'
        open={true}
        applyEditOp={vi.fn()}
      />
    ));
    expect(screen.getByText('agreed')).toBeInTheDocument();
    expect(screen.getByText('bob')).toBeInTheDocument();
  });

  it('does not render the sidebar when open is false', () => {
    render(() => (
      <CommentsController
        threads={[baseThread]}
        author='daisy'
        open={false}
        applyEditOp={vi.fn()}
      />
    ));
    expect(screen.queryByText('fix this')).toBeNull();
  });

  it('forwards onOpenChange from the sheet', () => {
    const onOpenChange = vi.fn();
    render(() => (
      <CommentsController
        threads={[baseThread]}
        author='daisy'
        open={true}
        onOpenChange={onOpenChange}
        applyEditOp={vi.fn()}
      />
    ));
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
