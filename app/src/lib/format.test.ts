import { describe, expect, it } from 'vitest';
import { sizeText } from './format';

describe('sizeText', () => {
  it('reads like the sizes writers see elsewhere', () => {
    expect(sizeText(0)).toBe('0');
    expect(sizeText(300)).toBe('1KB');
    expect(sizeText(12 * 1024)).toBe('12KB');
    expect(sizeText(0.2 * 1024 * 1024)).toBe('0.2MB');
    expect(sizeText(3.1 * 1024 * 1024)).toBe('3.1MB');
    expect(sizeText(41.4 * 1024 * 1024)).toBe('41MB');
    expect(sizeText(1.5 * 1024 * 1024 * 1024)).toBe('1.5GB');
  });
});
