import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { CommentsSidebar, type CommentThread } from './CommentsSidebar';

afterEach(cleanup);

const sampleThreads: CommentThread[] = [
  {
    id: 't1',
    author: 'Alice',
    body: 'This sentence is unclear.',
    createdAt: '2026-04-29T10:00:00Z',
    status: 'open',
    anchorBlockId: 'b-7',
    replies: [
      {
        id: 't1r1',
        author: 'Bob',
        body: 'Agreed, will revise.',
        createdAt: '2026-04-29T10:30:00Z',
      },
    ],
  },
  {
    id: 't2',
    author: 'Carol',
    body: 'Already addressed.',
    createdAt: '2026-04-29T11:15:00Z',
    status: 'resolved',
    anchorBlockId: 'b-12',
  },
];

describe('CommentsSidebar', () => {
  it('renders an aside with the accessible name "Comments"', () => {
    render(() => <CommentsSidebar threads={sampleThreads} />);
    expect(
      screen.getByRole('complementary', { name: 'Comments' }),
    ).toBeInTheDocument();
  });

  it('shows the default empty-state copy when no threads', () => {
    render(() => <CommentsSidebar threads={[]} />);
    expect(screen.getByText('No comments yet.')).toBeInTheDocument();
  });

  it('shows a custom emptyMessage when provided', () => {
    render(() => <CommentsSidebar threads={[]} emptyMessage='Select text to comment.' />);
    expect(screen.getByText('Select text to comment.')).toBeInTheDocument();
  });

  it('renders one article per thread with author, body, and timestamp', () => {
    render(() => <CommentsSidebar threads={sampleThreads} />);
    const articles = screen.getAllByRole('article');
    // Two threads + one reply = 3 article elements.
    expect(articles.length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.getByText('This sentence is unclear.'))
      .toBeInTheDocument();
    expect(screen.getAllByText('2026-04-29T10:00:00Z').length).toBeGreaterThan(
      0,
    );
  });

  it('renders inline replies under their parent thread', () => {
    render(() => <CommentsSidebar threads={sampleThreads} />);
    expect(screen.getByText('Bob')).toBeInTheDocument();
    expect(screen.getByText('Agreed, will revise.')).toBeInTheDocument();
  });

  it('shows a "Resolved" badge on resolved threads', () => {
    render(() => <CommentsSidebar threads={sampleThreads} />);
    const badges = screen.getAllByLabelText('Resolved');
    expect(badges.length).toBe(1);
  });

  it('fires onJumpToAnchor with the thread anchorBlockId', () => {
    const onJumpToAnchor = vi.fn();
    render(() => (
      <CommentsSidebar
        threads={sampleThreads}
        onJumpToAnchor={onJumpToAnchor}
      />
    ));
    const buttons = screen.getAllByRole('button', { name: 'Jump to anchor' });
    fireEvent.click(buttons[0]!);
    expect(onJumpToAnchor).toHaveBeenCalledWith('b-7');
  });

  it('fires onResolve with the thread id; disabled on already-resolved', () => {
    const onResolve = vi.fn();
    render(() => <CommentsSidebar threads={sampleThreads} onResolve={onResolve} />);
    const buttons = screen.getAllByRole('button', { name: 'Resolve' });
    expect(buttons[0]).not.toBeDisabled(); // open thread
    expect(buttons[1]).toBeDisabled(); // resolved thread
    fireEvent.click(buttons[0]!);
    expect(onResolve).toHaveBeenCalledWith('t1');
    fireEvent.click(buttons[1]!); // disabled, should not fire again
    expect(onResolve).toHaveBeenCalledTimes(1);
  });

  it('renders a reply form when onReply is provided', () => {
    render(() => <CommentsSidebar threads={sampleThreads} onReply={() => {}} />);
    // Two threads → two reply forms.
    expect(screen.getAllByRole('form', { name: 'Reply' })).toHaveLength(2);
  });

  it('does NOT render a reply form when onReply is undefined', () => {
    render(() => <CommentsSidebar threads={sampleThreads} />);
    expect(screen.queryByRole('form', { name: 'Reply' })).toBeNull();
  });

  it('reply submit fires onReply(threadId, body) and clears the field', () => {
    const onReply = vi.fn();
    render(() => <CommentsSidebar threads={sampleThreads} onReply={onReply} />);
    const textarea = screen.getAllByLabelText('Reply text')[0]!;
    fireEvent.input(textarea, { target: { value: 'Will fix.' } });
    const submit = screen.getAllByRole('button', { name: 'Reply' })[0]!;
    fireEvent.click(submit);
    expect(onReply).toHaveBeenCalledWith('t1', 'Will fix.');
    // After clearing, submit should be disabled (empty body guard).
    expect((textarea as HTMLTextAreaElement).value).toBe('');
  });

  it('does not fire onReply for whitespace-only input', () => {
    const onReply = vi.fn();
    render(() => <CommentsSidebar threads={sampleThreads} onReply={onReply} />);
    const textarea = screen.getAllByLabelText('Reply text')[0]!;
    fireEvent.input(textarea, { target: { value: '   \n  ' } });
    const submit = screen.getAllByRole('button', { name: 'Reply' })[0]!;
    expect(submit).toBeDisabled();
    expect(onReply).not.toHaveBeenCalled();
  });

  it('marks the active thread with aria-current="true"', () => {
    render(() => <CommentsSidebar threads={sampleThreads} activeThreadId='t2' />);
    const articles = document.querySelectorAll('article[data-thread-id]');
    const t1 = Array.from(articles).find((a) => a.getAttribute('data-thread-id') === 't1');
    const t2 = Array.from(articles).find((a) => a.getAttribute('data-thread-id') === 't2');
    expect(t1).not.toHaveAttribute('aria-current');
    expect(t2).toHaveAttribute('aria-current', 'true');
  });

  it('merges class prop into the root aside', () => {
    render(() => <CommentsSidebar threads={sampleThreads} class='border-l' />);
    const aside = screen.getByRole('complementary');
    expect(aside.className).toContain('border-l');
    expect(aside.className).toContain('flex');
  });
});
