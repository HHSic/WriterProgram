// 설정집 cards, their kinds, and where their names appear.

import type { SearchMatch } from './search-export';

/** 설정집 분류 (crates/core/src/cards.rs). */
export interface CardType {
  id: string;
  name: string;
  fields: string[];
}

export interface CardSummary {
  id: string;
  cardType: string;
  name: string;
  aliases: string[];
  highlight: boolean;
  summary: string;
}

export interface Card {
  id: string;
  cardType: string;
  name: string;
  aliases: string[];
  /** [name, value] pairs, in order. */
  fields: [string, string][];
  highlight: boolean;
  created: string;
  description: string;
}

export interface Appearance {
  docId: string;
  count: number;
  samples: SearchMatch[];
}

/** Where a card's names appear, and how many chapters could not be read. */
export interface Appearances {
  places: Appearance[];
  skipped: number;
}
