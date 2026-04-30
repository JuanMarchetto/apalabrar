// Phase 5.11 — file extension → editor-bridge `DocFormat` mapping.
//
// Used by `EditorShell`'s file picker to dispatch `core.openDoc(bytes,
// format)`. Pulled out of the component so the lookup is unit-testable
// without rendering Solid.

import type { DocFormat } from '@apalabrar/editor-bridge';

const FORMAT_BY_EXT: Readonly<Record<string, DocFormat>> = {
  docx: 'docx',
  md: 'markdown',
  markdown: 'markdown',
  html: 'html',
  htm: 'html',
  rtf: 'rtf',
  odt: 'odt',
};

/**
 * Map a filename's extension to the corresponding `DocFormat`. The
 * lookup is case-insensitive (`.DOCX` works the same as `.docx`).
 * Returns `null` for unknown extensions; the caller surfaces a
 * "this format isn't supported" error to the user.
 */
export function detectFormat(filename: string): DocFormat | null {
  const dot = filename.lastIndexOf('.');
  if (dot < 0 || dot === filename.length - 1) return null;
  const ext = filename.slice(dot + 1).toLowerCase();
  return FORMAT_BY_EXT[ext] ?? null;
}

/** MIME type for a `DocFormat` — used when emitting download blobs. */
export function mimeFor(format: DocFormat): string {
  switch (format) {
    case 'docx':
      return 'application/vnd.openxmlformats-officedocument.wordprocessingml.document';
    case 'markdown':
      return 'text/markdown';
    case 'html':
      return 'text/html';
    case 'rtf':
      return 'application/rtf';
    case 'odt':
      return 'application/vnd.oasis.opendocument.text';
  }
}
