import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { GDocsToolbar, type GDocsToolbarProps } from './GDocsToolbar';

afterEach(cleanup);

const baseProps: GDocsToolbarProps = {
  canUndo: true,
  canRedo: true,
  paragraphStyle: 'normal',
  fontFamily: 'Inter',
  fontFamilies: ['Inter', 'Source Serif Pro', 'JetBrains Mono'],
  fontSize: 12,
  fontSizes: [10, 11, 12, 14, 18, 24],
  bold: false,
  italic: false,
  underline: false,
  alignment: 'left',
  list: 'none',
  layoutMode: 'modern',
};

describe('GDocsToolbar', () => {
  it('renders a toolbar with the accessible name "Document toolbar"', () => {
    render(() => <GDocsToolbar {...baseProps} />);
    expect(
      screen.getByRole('toolbar', { name: 'Document toolbar' }),
    ).toBeInTheDocument();
  });

  it('disables Undo / Redo per canUndo / canRedo', () => {
    render(() => <GDocsToolbar {...baseProps} canUndo={false} canRedo={true} />);
    expect(screen.getByLabelText('Undo')).toBeDisabled();
    expect(screen.getByLabelText('Redo')).not.toBeDisabled();
  });

  it('fires onUndo / onRedo when buttons clicked', () => {
    const onUndo = vi.fn();
    const onRedo = vi.fn();
    render(() => <GDocsToolbar {...baseProps} onUndo={onUndo} onRedo={onRedo} />);
    fireEvent.click(screen.getByLabelText('Undo'));
    fireEvent.click(screen.getByLabelText('Redo'));
    expect(onUndo).toHaveBeenCalled();
    expect(onRedo).toHaveBeenCalled();
  });

  it('renders paragraph-style select with all 8 options and reflects value', () => {
    render(() => <GDocsToolbar {...baseProps} paragraphStyle='heading2' />);
    const select = screen.getByLabelText('Paragraph style') as HTMLSelectElement;
    expect(select.value).toBe('heading2');
    expect(select.querySelectorAll('option')).toHaveLength(8);
  });

  it('fires onParagraphStyleChange with the new style', () => {
    const onParagraphStyleChange = vi.fn();
    render(() => (
      <GDocsToolbar
        {...baseProps}
        onParagraphStyleChange={onParagraphStyleChange}
      />
    ));
    fireEvent.change(screen.getByLabelText('Paragraph style'), {
      target: { value: 'heading1' },
    });
    expect(onParagraphStyleChange).toHaveBeenCalledWith('heading1');
  });

  it('fires onFontFamilyChange with the new family string', () => {
    const onFontFamilyChange = vi.fn();
    render(() => (
      <GDocsToolbar
        {...baseProps}
        onFontFamilyChange={onFontFamilyChange}
      />
    ));
    fireEvent.change(screen.getByLabelText('Font family'), {
      target: { value: 'JetBrains Mono' },
    });
    expect(onFontFamilyChange).toHaveBeenCalledWith('JetBrains Mono');
  });

  it('fires onFontSizeChange with a number, not a string', () => {
    const onFontSizeChange = vi.fn();
    render(() => <GDocsToolbar {...baseProps} onFontSizeChange={onFontSizeChange} />);
    fireEvent.change(screen.getByLabelText('Font size'), {
      target: { value: '24' },
    });
    expect(onFontSizeChange).toHaveBeenCalledWith(24);
    expect(typeof onFontSizeChange.mock.calls[0]?.[0]).toBe('number');
  });

  it('reflects bold/italic/underline state via aria-pressed', () => {
    render(() => (
      <GDocsToolbar
        {...baseProps}
        bold
        italic={false}
        underline
      />
    ));
    expect(screen.getByLabelText('Bold'))
      .toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByLabelText('Italic'))
      .toHaveAttribute('aria-pressed', 'false');
    expect(screen.getByLabelText('Underline'))
      .toHaveAttribute('aria-pressed', 'true');
  });

  it('fires bold/italic/underline toggles', () => {
    const onToggleBold = vi.fn();
    const onToggleItalic = vi.fn();
    const onToggleUnderline = vi.fn();
    render(() => (
      <GDocsToolbar
        {...baseProps}
        onToggleBold={onToggleBold}
        onToggleItalic={onToggleItalic}
        onToggleUnderline={onToggleUnderline}
      />
    ));
    fireEvent.click(screen.getByLabelText('Bold'));
    fireEvent.click(screen.getByLabelText('Italic'));
    fireEvent.click(screen.getByLabelText('Underline'));
    expect(onToggleBold).toHaveBeenCalledTimes(1);
    expect(onToggleItalic).toHaveBeenCalledTimes(1);
    expect(onToggleUnderline).toHaveBeenCalledTimes(1);
  });

  it('alignment toggle group reflects current alignment via aria-pressed', () => {
    render(() => <GDocsToolbar {...baseProps} alignment='center' />);
    expect(screen.getByLabelText('Align center'))
      .toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByLabelText('Align left'))
      .toHaveAttribute('aria-pressed', 'false');
    expect(screen.getByLabelText('Justify'))
      .toHaveAttribute('aria-pressed', 'false');
  });

  it('fires onAlignmentChange with the clicked alignment', () => {
    const onAlignmentChange = vi.fn();
    render(() => (
      <GDocsToolbar
        {...baseProps}
        onAlignmentChange={onAlignmentChange}
      />
    ));
    fireEvent.click(screen.getByLabelText('Align right'));
    expect(onAlignmentChange).toHaveBeenCalledWith('right');
  });

  it('list toggle: clicking the active list kind clears to "none"', () => {
    const onListChange = vi.fn();
    render(() => (
      <GDocsToolbar
        {...baseProps}
        list='bullet'
        onListChange={onListChange}
      />
    ));
    expect(screen.getByLabelText('Bulleted list'))
      .toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(screen.getByLabelText('Bulleted list'));
    expect(onListChange).toHaveBeenCalledWith('none');
  });

  it('list toggle: clicking an inactive list kind sets it', () => {
    const onListChange = vi.fn();
    render(() => (
      <GDocsToolbar
        {...baseProps}
        list='none'
        onListChange={onListChange}
      />
    ));
    fireEvent.click(screen.getByLabelText('Numbered list'));
    expect(onListChange).toHaveBeenCalledWith('numbered');
  });

  it('fires onLink and onComment when those buttons are clicked', () => {
    const onLink = vi.fn();
    const onComment = vi.fn();
    render(() => (
      <GDocsToolbar
        {...baseProps}
        onLink={onLink}
        onComment={onComment}
      />
    ));
    fireEvent.click(screen.getByLabelText('Insert link'));
    fireEvent.click(screen.getByLabelText('Add comment'));
    expect(onLink).toHaveBeenCalled();
    expect(onComment).toHaveBeenCalled();
  });

  it('embeds the WordClassicToggle and forwards onLayoutModeChange', () => {
    const onLayoutModeChange = vi.fn();
    render(() => (
      <GDocsToolbar
        {...baseProps}
        layoutMode='modern'
        onLayoutModeChange={onLayoutModeChange}
      />
    ));
    expect(screen.getByRole('radio', { name: 'Modern' })).toBeChecked();
    fireEvent.click(screen.getByRole('radio', { name: 'Classic' }));
    expect(onLayoutModeChange).toHaveBeenCalledWith('classic');
  });

  it('treats undefined callbacks as no-ops (no throws)', () => {
    render(() => <GDocsToolbar {...baseProps} />);
    expect(() => {
      fireEvent.click(screen.getByLabelText('Undo'));
      fireEvent.click(screen.getByLabelText('Bold'));
      fireEvent.click(screen.getByLabelText('Align right'));
      fireEvent.click(screen.getByLabelText('Bulleted list'));
      fireEvent.click(screen.getByLabelText('Insert link'));
    }).not.toThrow();
  });

  it('merges class prop into the root toolbar', () => {
    render(() => <GDocsToolbar {...baseProps} class='sticky top-0' />);
    const toolbar = screen.getByRole('toolbar', { name: 'Document toolbar' });
    expect(toolbar.className).toContain('sticky');
    expect(toolbar.className).toContain('flex');
  });
});
