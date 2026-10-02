// Saved formats (서식) of the writer's own and the page estimate for a format.

import type { Backend, UserPreset } from '../types';
import { CATALOG, mockPages } from './presets';
import { clone, overview } from './state';

let userPresets: UserPreset[] = [];

export const formatMethods = {
  async formatCatalog() {
    return clone({ ...CATALOG, user: userPresets });
  },
  async formatSavePreset(name, format) {
    const trimmed = name.trim();
    if (!trimmed) throw '서식 이름을 적어 주세요';
    if (CATALOG.builtin.some((b) => b.name === trimmed)) throw '기본 서식과 같은 이름은 쓸 수 없음';
    userPresets = [...userPresets.filter((p) => p.name !== trimmed), { name: trimmed, format: { ...clone(format), preset: trimmed } }];
    return clone(userPresets);
  },
  async formatDeletePreset(name) {
    userPresets = userPresets.filter((p) => p.name !== name);
    return clone(userPresets);
  },
  async formatEstimate(root, format) {
    return mockPages(overview(root).total.withSpaces, format);
  },
} satisfies Partial<Backend>;
