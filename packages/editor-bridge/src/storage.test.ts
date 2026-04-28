// Pure helper tests: doc-id validation + StorageError shape.

import { describe, expect, it } from 'vitest';
import { isDocId, parseDocId, StorageError } from './storage';

describe('parseDocId', () => {
  it('accepts a basic alphanumeric id', () => {
    expect(parseDocId('hello-world_123')).toBe('hello-world_123');
  });

  it('accepts a single-character id', () => {
    expect(parseDocId('a')).toBe('a');
  });

  it('accepts a 128-character id (the upper bound)', () => {
    const id = 'a'.repeat(128);
    expect(parseDocId(id)).toBe(id);
  });

  it('rejects an empty string with invalid-id', () => {
    expect(() => parseDocId('')).toThrow(StorageError);
    try {
      parseDocId('');
    } catch (error) {
      expect((error as StorageError).kind).toBe('invalid-id');
      expect((error as StorageError).id).toBe('');
    }
  });

  it('rejects a 129-character id (one past the upper bound)', () => {
    expect(() => parseDocId('a'.repeat(129))).toThrow(StorageError);
  });

  it('rejects path-traversal patterns', () => {
    expect(() => parseDocId('../etc/passwd')).toThrow(StorageError);
    expect(() => parseDocId('foo/bar')).toThrow(StorageError);
    expect(() => parseDocId('a.b')).toThrow(StorageError);
  });

  it('rejects whitespace and unicode', () => {
    expect(() => parseDocId('hello world')).toThrow(StorageError);
    expect(() => parseDocId('héllo')).toThrow(StorageError);
    expect(() => parseDocId('emoji-🚀')).toThrow(StorageError);
  });

  it('rejects non-string inputs', () => {
    // @ts-expect-error testing runtime guard; static type forbids this
    expect(() => parseDocId(null)).toThrow(StorageError);
    // @ts-expect-error testing runtime guard
    expect(() => parseDocId(123)).toThrow(StorageError);
  });

  it('records the offending id on the StorageError', () => {
    try {
      parseDocId('foo/bar');
    } catch (error) {
      expect(error).toBeInstanceOf(StorageError);
      expect((error as StorageError).kind).toBe('invalid-id');
      expect((error as StorageError).id).toBe('foo/bar');
      expect((error as StorageError).message).toContain('foo/bar');
    }
  });
});

describe('isDocId', () => {
  it('returns true for a valid id', () => {
    expect(isDocId('valid-id')).toBe(true);
  });

  it('returns false for an empty string', () => {
    expect(isDocId('')).toBe(false);
  });

  it('returns false for non-string values', () => {
    expect(isDocId(null)).toBe(false);
    expect(isDocId(undefined)).toBe(false);
    expect(isDocId(42)).toBe(false);
    expect(isDocId({})).toBe(false);
  });
});

describe('StorageError', () => {
  it('is an Error subclass with a kind discriminator', () => {
    const error = new StorageError('quota-exceeded', 'no space');
    expect(error).toBeInstanceOf(Error);
    expect(error.name).toBe('StorageError');
    expect(error.kind).toBe('quota-exceeded');
    expect(error.message).toBe('no space');
    expect(error.id).toBeNull();
  });

  it('preserves the offending id when supplied', () => {
    const error = new StorageError('not-found' as never, 'missing', 'doc-id-here');
    expect(error.id).toBe('doc-id-here');
  });
});
