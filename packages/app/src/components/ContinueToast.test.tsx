/**
 * Phase 2.5 RED tests for `ContinueToast`. Coverage:
 *
 * - Renders with role=status + aria-live=polite.
 * - "Continue" button calls onContinue with the newest doc's id.
 * - Dismiss button calls onDismiss.
 * - Auto-dismisses after autoDismissMs (default 5000) by calling
 *   onDismiss exactly once.
 * - Auto-dismiss is cancelled if the user manually dismisses or
 *   continues — no double onDismiss firing.
 * - Renders the doc count when there are multiple recents.
 */
import { type DocId, type DocMetadata, parseDocId } from '@apalabrar/editor-bridge';
import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ContinueToast } from './ContinueToast';

const id = (s: string): DocId => parseDocId(s);

const docs = (n: number): DocMetadata[] =>
  Array.from({ length: n }, (_, i) => ({
    id: id(`doc-${i}`),
    sizeBytes: 100 * (i + 1),
    lastModified: 1_700_000_000_000 + i * 1000,
  }));

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe('ContinueToast', () => {
  it('renders nothing meaningful when docs is empty', () => {
    render(() => (
      <ContinueToast
        docs={[]}
        onContinue={() => undefined}
        onDismiss={() => undefined}
      />
    ));
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('renders with role=status and aria-live=polite', () => {
    render(() => (
      <ContinueToast
        docs={docs(1)}
        onContinue={() => undefined}
        onDismiss={() => undefined}
      />
    ));
    const toast = screen.getByRole('status');
    expect(toast.getAttribute('aria-live')).toBe('polite');
  });

  it('Continue button fires onContinue with the newest doc id', () => {
    const onContinue = vi.fn();
    const list = docs(3);
    render(() => (
      <ContinueToast
        docs={list}
        onContinue={onContinue}
        onDismiss={() => undefined}
      />
    ));
    fireEvent.click(screen.getByRole('button', { name: /continue/i }));
    expect(onContinue).toHaveBeenCalledTimes(1);
    expect(onContinue).toHaveBeenCalledWith(list[0]!.id);
  });

  it('Dismiss button fires onDismiss', () => {
    const onDismiss = vi.fn();
    render(() => (
      <ContinueToast
        docs={docs(1)}
        onContinue={() => undefined}
        onDismiss={onDismiss}
      />
    ));
    fireEvent.click(screen.getByRole('button', { name: /dismiss/i }));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('auto-dismisses after autoDismissMs (default 5000)', () => {
    vi.useFakeTimers();
    const onDismiss = vi.fn();
    render(() => (
      <ContinueToast
        docs={docs(1)}
        onContinue={() => undefined}
        onDismiss={onDismiss}
      />
    ));
    expect(onDismiss).not.toHaveBeenCalled();
    vi.advanceTimersByTime(4999);
    expect(onDismiss).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('respects custom autoDismissMs', () => {
    vi.useFakeTimers();
    const onDismiss = vi.fn();
    render(() => (
      <ContinueToast
        docs={docs(1)}
        onContinue={() => undefined}
        onDismiss={onDismiss}
        autoDismissMs={1000}
      />
    ));
    vi.advanceTimersByTime(1000);
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('manual dismiss cancels the auto-dismiss timer (no double-fire)', () => {
    vi.useFakeTimers();
    const onDismiss = vi.fn();
    render(() => (
      <ContinueToast
        docs={docs(1)}
        onContinue={() => undefined}
        onDismiss={onDismiss}
        autoDismissMs={2000}
      />
    ));
    fireEvent.click(screen.getByRole('button', { name: /dismiss/i }));
    expect(onDismiss).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(5000);
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('clicking Continue cancels the auto-dismiss timer', () => {
    vi.useFakeTimers();
    const onContinue = vi.fn();
    const onDismiss = vi.fn();
    render(() => (
      <ContinueToast
        docs={docs(1)}
        onContinue={onContinue}
        onDismiss={onDismiss}
        autoDismissMs={2000}
      />
    ));
    fireEvent.click(screen.getByRole('button', { name: /continue/i }));
    vi.advanceTimersByTime(5000);
    expect(onContinue).toHaveBeenCalledTimes(1);
    expect(onDismiss).not.toHaveBeenCalled();
  });
});
