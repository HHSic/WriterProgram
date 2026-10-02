// AI with the writer's own API key (crates/ai, app/src-tauri/src/commands/ai.rs).

import type { SearchMatch } from './search-export';

export type AiProvider = 'anthropic' | 'openai' | 'gemini';

/** 회차 요약 or 설정 모순 점검. */
export type AiTask = 'summary' | 'check';

export interface AiModels {
  summary: string;
  check: string;
}

export interface AiCompany {
  provider: AiProvider;
  label: string;
  models: AiModels;
  defaults: AiModels;
  /** A key is in the system's credential store (the key never comes back). */
  hasKey: boolean;
}

export interface AiSettings {
  enabled: boolean;
  provider: AiProvider;
  /** When the writer agreed that text is sent (ISO time). */
  agreed: string | null;
  companies: AiCompany[];
}

export interface AiSettingsPatch {
  enabled?: boolean;
  provider?: AiProvider;
  /** Models for one company; an empty name means its default. */
  models?: [AiProvider, AiModels];
}

/** A name and the stand-in sent in its place (이름 가리기). */
export interface AiSwap {
  name: string;
  standIn: string;
  /** The card's own name. */
  card: string;
}

/** What would be sent for one chapter, before anything is sent. */
export interface AiPreview {
  task: AiTask;
  docId: string;
  title: string;
  instructions: string;
  /** The text exactly as it will be sent, names masked. */
  text: string;
  swaps: AiSwap[];
  /** Setting cards sent along (설정 모순 점검). */
  cards: string[];
  /** Characters sent in all. */
  chars: number;
}

/** What the company says it used, when it says so. */
export interface AiUsage {
  inputTokens: number | null;
  outputTokens: number | null;
}

export interface AiSummary {
  docId: string;
  text: string;
  usage: AiUsage | null;
  sentChars: number;
}

export interface AiFinding {
  quote: string;
  card: string;
  problem: string;
  /** Where the quote is in the chapter; null when it was not found again. */
  place: SearchMatch | null;
}

export interface AiCheck {
  docId: string;
  findings: AiFinding[];
  cards: string[];
  usage: AiUsage | null;
  sentChars: number;
}
