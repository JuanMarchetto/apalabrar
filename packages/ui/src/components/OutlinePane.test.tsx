import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { type OutlineHeading, OutlinePane } from './OutlinePane';

afterEach(cleanup);

const headings: OutlineHeading[] = [
  { blockId: 'a', level: 1, text: 'Introduction' },
  { blockId: 'b', level: 2, text: 'Background' },
  { blockId: 'c', level: 2, text: 'Method' },
  { blockId: 'd', level: 3, text: 'Sample' },
];

describe('OutlinePane', () => {
  it('renders a nav with the accessible name "Document outline"', () => {
    render(() => <OutlinePane headings={headings} />);
    expect(
      screen.getByRole('navigation', { name: 'Document outline' }),
    ).toBeInTheDocument();
  });

  it('renders one button per heading with the heading text', () => {
    render(() => <OutlinePane headings={headings} />);
    for (const h of headings) {
      expect(screen.getByRole('button', { name: h.text })).toBeInTheDocument();
    }
  });

  it('shows the default empty-state copy when no headings', () => {
    render(() => <OutlinePane headings={[]} />);
    expect(screen.getByText('No headings yet.')).toBeInTheDocument();
    // No buttons rendered.
    expect(screen.queryAllByRole('button')).toHaveLength(0);
  });

  it('shows a custom emptyMessage when provided', () => {
    render(() => <OutlinePane headings={[]} emptyMessage='Add a heading to start.' />);
    expect(screen.getByText('Add a heading to start.')).toBeInTheDocument();
  });

  it('fires onJump with the clicked heading blockId', () => {
    const onJump = vi.fn();
    render(() => <OutlinePane headings={headings} onJump={onJump} />);
    fireEvent.click(screen.getByRole('button', { name: 'Method' }));
    expect(onJump).toHaveBeenCalledWith('c');
  });

  it('marks only the active blockId with aria-current="location"', () => {
    render(() => <OutlinePane headings={headings} activeBlockId='b' />);
    const active = screen.getByRole('button', { name: 'Background' });
    expect(active).toHaveAttribute('aria-current', 'location');
    const inactive = screen.getByRole('button', { name: 'Introduction' });
    expect(inactive).not.toHaveAttribute('aria-current');
  });

  it('exposes the heading level via data-level', () => {
    // Note: aria-level is ARIA-spec-illegal on <button>; we use a data-*
    // attribute instead so consumers (and the styleguide) can read level
    // for styling/analytics. Hierarchy is conveyed visually through the
    // level-dependent indent class.
    render(() => <OutlinePane headings={headings} />);
    expect(screen.getByRole('button', { name: 'Introduction' }))
      .toHaveAttribute('data-level', '1');
    expect(screen.getByRole('button', { name: 'Sample' }))
      .toHaveAttribute('data-level', '3');
  });

  it('applies a level-derived indent class to each button', () => {
    render(() => <OutlinePane headings={headings} />);
    expect(screen.getByRole('button', { name: 'Introduction' }).className)
      .toContain('pl-2');
    expect(screen.getByRole('button', { name: 'Background' }).className)
      .toContain('pl-4');
    expect(screen.getByRole('button', { name: 'Sample' }).className)
      .toContain('pl-6');
  });

  it('treats undefined onJump as a no-op (does not throw on click)', () => {
    render(() => <OutlinePane headings={headings} />);
    expect(() => fireEvent.click(screen.getByRole('button', { name: 'Introduction' }))).not
      .toThrow();
  });

  it('merges the class prop into the root nav element', () => {
    render(() => <OutlinePane headings={headings} class='border-r' />);
    const nav = screen.getByRole('navigation');
    expect(nav.className).toContain('border-r');
    expect(nav.className).toContain('flex'); // root default still present
  });
});
