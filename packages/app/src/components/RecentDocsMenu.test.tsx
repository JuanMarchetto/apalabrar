/**
 * Phase 2.5 RED tests for `RecentDocsMenu`. Coverage:
 *
 * - Empty docs → renders nothing (avoid empty disclosure UI).
 * - Trigger is a button with aria-haspopup="menu".
 * - Click trigger opens; aria-expanded flips true.
 * - Each doc renders as a menuitem.
 * - Click menuitem calls onSelect with that doc's id.
 */
import { type DocId, type DocMetadata, parseDocId } from '@apalabrar/editor-bridge';
import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { RecentDocsMenu } from './RecentDocsMenu';

const id = (s: string): DocId => parseDocId(s);

const docs = (ids: string[]): DocMetadata[] =>
  ids.map((s, i) => ({
    id: id(s),
    sizeBytes: 100 * (i + 1),
    lastModified: 1_700_000_000_000 + i * 1000,
  }));

afterEach(() => cleanup());

describe('RecentDocsMenu', () => {
  it('renders nothing when docs is empty', () => {
    const { container } = render(() => <RecentDocsMenu docs={[]} onSelect={() => undefined} />);
    expect(container.textContent).toBe('');
  });

  it('renders a button trigger with aria-haspopup=menu', () => {
    render(() => <RecentDocsMenu docs={docs(['a'])} onSelect={() => undefined} />);
    const trigger = screen.getByRole('button', { name: /recent/i });
    expect(trigger.getAttribute('aria-haspopup')).toBe('menu');
  });

  it('starts collapsed (aria-expanded=false)', () => {
    render(() => <RecentDocsMenu docs={docs(['a'])} onSelect={() => undefined} />);
    const trigger = screen.getByRole('button', { name: /recent/i });
    expect(trigger.getAttribute('aria-expanded')).toBe('false');
  });

  it('clicking trigger expands the menu (aria-expanded=true) and shows menuitems', () => {
    render(() => (
      <RecentDocsMenu
        docs={docs(['alpha', 'beta', 'gamma'])}
        onSelect={() => undefined}
      />
    ));
    fireEvent.click(screen.getByRole('button', { name: /recent/i }));
    const trigger = screen.getByRole('button', { name: /recent/i });
    expect(trigger.getAttribute('aria-expanded')).toBe('true');
    expect(screen.getAllByRole('menuitem').length).toBe(3);
  });

  it('formats sizes in B, KB, MB ranges', () => {
    const list: DocMetadata[] = [
      { id: id('small'), sizeBytes: 500, lastModified: 1 },
      { id: id('medium'), sizeBytes: 5_000, lastModified: 2 },
      { id: id('large'), sizeBytes: 5_000_000, lastModified: 3 },
    ];
    render(() => <RecentDocsMenu docs={list} onSelect={() => undefined} />);
    fireEvent.click(screen.getByRole('button', { name: /recent/i }));
    const items = screen.getAllByRole('menuitem');
    expect(items[0]!.textContent).toContain('500 B');
    expect(items[1]!.textContent).toContain('4.9 KB');
    expect(items[2]!.textContent).toContain('4.8 MB');
  });

  it('clicking a menuitem calls onSelect with that doc id', () => {
    const onSelect = vi.fn();
    const list = docs(['alpha', 'beta']);
    render(() => <RecentDocsMenu docs={list} onSelect={onSelect} />);
    fireEvent.click(screen.getByRole('button', { name: /recent/i }));
    fireEvent.click(screen.getByRole('menuitem', { name: /beta/i }));
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith(list[1]!.id);
  });
});
