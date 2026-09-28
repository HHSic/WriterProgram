import { describe, expect, it } from 'vitest';
import { colorTokens, hexToTint } from './colors';

describe('screen colors', () => {
  it('reads a hue from a picked color and keeps the paper pale', () => {
    expect(hexToTint('#ffffff').chroma).toBeCloseTo(0, 4);
    const red = hexToTint('#ff0000');
    expect(red.hue).toBeCloseTo(29.2, 0);
    expect(red.chroma).toBe(0.03);
    expect(hexToTint('not a color')).toEqual({ hue: 90, chroma: 0 });
  });

  it('leaves the stylesheet beige alone unless the accent changes', () => {
    expect(colorTokens({ palette: 'beige', accent: 'auto', customColor: '#000000' }, false)).toEqual({});
    expect(colorTokens({ palette: 'beige', accent: 'blue', customColor: '#000000' }, true)).toEqual({
      accent: '#7eaae8',
      'accent-ink': '#14181e',
    });
  });

  it('gives every other palette a full light and dark set with its own accent', () => {
    const light = colorTokens({ palette: 'green', accent: 'auto', customColor: '#000000' }, false);
    const dark = colorTokens({ palette: 'green', accent: 'auto', customColor: '#000000' }, true);
    expect(light.bg).toMatch(/^oklch\(0\.957 /);
    expect(dark.bg).toMatch(/^oklch\(0\.215 /);
    expect(light.accent).toBe('#2f7649');
    expect(dark.accent).toBe('#74b98b');
    expect(Object.keys(light)).toEqual(Object.keys(dark));
  });
});
