import { describe, expect, it } from 'vitest';
import { VERSION } from './index';

describe('editor-bridge', () => {
  it('exports a pinned version', () => {
    expect(VERSION).toBe('0.0.0');
  });
});
