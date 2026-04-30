import { describe, expect, it } from 'vitest';

import { detectFormat, mimeFor } from './detectFormat';

describe('detectFormat', () => {
  it('maps .docx to docx', () => {
    expect(detectFormat('thesis.docx')).toBe('docx');
  });

  it('maps .md and .markdown to markdown', () => {
    expect(detectFormat('notes.md')).toBe('markdown');
    expect(detectFormat('NOTES.markdown')).toBe('markdown');
  });

  it('maps .html and .htm to html', () => {
    expect(detectFormat('a.html')).toBe('html');
    expect(detectFormat('a.htm')).toBe('html');
  });

  it('maps .rtf and .odt to their formats', () => {
    expect(detectFormat('a.rtf')).toBe('rtf');
    expect(detectFormat('a.odt')).toBe('odt');
  });

  it('is case-insensitive on the extension', () => {
    expect(detectFormat('DOC.DOCX')).toBe('docx');
    expect(detectFormat('Hello.MD')).toBe('markdown');
  });

  it('returns null for unknown extensions', () => {
    expect(detectFormat('image.png')).toBeNull();
    expect(detectFormat('archive.zip')).toBeNull();
  });

  it('returns null when there is no extension', () => {
    expect(detectFormat('README')).toBeNull();
    expect(detectFormat('')).toBeNull();
  });

  it('returns null when the dot is the final character', () => {
    expect(detectFormat('weird.')).toBeNull();
  });

  it('uses the last dot when the filename has multiple', () => {
    expect(detectFormat('thesis.draft.docx')).toBe('docx');
  });
});

describe('mimeFor', () => {
  it('returns the OOXML word-document MIME for docx', () => {
    expect(mimeFor('docx')).toBe(
      'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    );
  });

  it('returns text/markdown for markdown', () => {
    expect(mimeFor('markdown')).toBe('text/markdown');
  });

  it('returns text/html for html', () => {
    expect(mimeFor('html')).toBe('text/html');
  });

  it('returns the RTF and ODT MIME types', () => {
    expect(mimeFor('rtf')).toBe('application/rtf');
    expect(mimeFor('odt')).toBe('application/vnd.oasis.opendocument.text');
  });
});
