import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { FloatingSelectionToolbar } from './FloatingSelectionToolbar';

afterEach(cleanup);

const baseProps = {
  visible: true,
  position: { x: 100, y: 200 },
  bold: false,
  italic: false,
  underline: false,
} as const;

describe('FloatingSelectionToolbar', () => {
  it('renders nothing when visible is false', () => {
    render(() => <FloatingSelectionToolbar {...baseProps} visible={false} />);
    expect(screen.queryByRole('toolbar')).toBeNull();
  });

  it('renders a toolbar with accessible name when visible', () => {
    render(() => <FloatingSelectionToolbar {...baseProps} />);
    expect(
      screen.getByRole('toolbar', { name: 'Selection formatting' }),
    ).toBeInTheDocument();
  });

  it('applies position via inline style (left/top in px)', () => {
    render(() => (
      <FloatingSelectionToolbar
        {...baseProps}
        position={{ x: 42, y: 84 }}
      />
    ));
    const toolbar = screen.getByRole('toolbar');
    expect(toolbar.style.left).toBe('42px');
    expect(toolbar.style.top).toBe('84px');
  });

  it('reflects bold/italic/underline state via aria-pressed', () => {
    render(() => (
      <FloatingSelectionToolbar
        {...baseProps}
        bold
        italic
        underline={false}
      />
    ));
    expect(screen.getByLabelText('Bold')).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByLabelText('Italic')).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByLabelText('Underline'))
      .toHaveAttribute('aria-pressed', 'false');
  });

  it('fires onToggleBold when Bold is clicked', () => {
    const onToggleBold = vi.fn();
    render(() => (
      <FloatingSelectionToolbar
        {...baseProps}
        onToggleBold={onToggleBold}
      />
    ));
    fireEvent.click(screen.getByLabelText('Bold'));
    expect(onToggleBold).toHaveBeenCalledTimes(1);
  });

  it('fires onToggleItalic when Italic is clicked', () => {
    const onToggleItalic = vi.fn();
    render(() => (
      <FloatingSelectionToolbar
        {...baseProps}
        onToggleItalic={onToggleItalic}
      />
    ));
    fireEvent.click(screen.getByLabelText('Italic'));
    expect(onToggleItalic).toHaveBeenCalledTimes(1);
  });

  it('fires onToggleUnderline when Underline is clicked', () => {
    const onToggleUnderline = vi.fn();
    render(() => (
      <FloatingSelectionToolbar
        {...baseProps}
        onToggleUnderline={onToggleUnderline}
      />
    ));
    fireEvent.click(screen.getByLabelText('Underline'));
    expect(onToggleUnderline).toHaveBeenCalledTimes(1);
  });

  it('fires onLink when Insert link is clicked', () => {
    const onLink = vi.fn();
    render(() => <FloatingSelectionToolbar {...baseProps} onLink={onLink} />);
    fireEvent.click(screen.getByLabelText('Insert link'));
    expect(onLink).toHaveBeenCalledTimes(1);
  });

  it('fires onComment when Add comment is clicked', () => {
    const onComment = vi.fn();
    render(() => <FloatingSelectionToolbar {...baseProps} onComment={onComment} />);
    fireEvent.click(screen.getByLabelText('Add comment'));
    expect(onComment).toHaveBeenCalledTimes(1);
  });

  it('treats undefined callbacks as no-ops (does not throw on click)', () => {
    render(() => <FloatingSelectionToolbar {...baseProps} />);
    expect(() => {
      fireEvent.click(screen.getByLabelText('Bold'));
      fireEvent.click(screen.getByLabelText('Italic'));
      fireEvent.click(screen.getByLabelText('Underline'));
      fireEvent.click(screen.getByLabelText('Insert link'));
      fireEvent.click(screen.getByLabelText('Add comment'));
    }).not.toThrow();
  });

  it('merges class prop into the root toolbar', () => {
    render(() => <FloatingSelectionToolbar {...baseProps} class='shadow-2xl' />);
    const toolbar = screen.getByRole('toolbar');
    expect(toolbar.className).toContain('shadow-2xl');
    expect(toolbar.className).toContain('absolute');
  });
});
