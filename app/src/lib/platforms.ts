// 연재 플랫폼: how each serial platform counts characters for its minimums,
// the minimum used as the chapter goal, and how text is best pasted into its
// editor. Sources and what is only estimated are in docs/platforms.md; the
// rule a project uses is copied into project.json so the writer can change it.

import type { CountRule, Counts, Platform, ProjectInfo } from '../api/types';
import { countByRule } from '../editor/counts';

/** How text goes on the clipboard: text only, or text and paragraphs as HTML. */
export type PasteHtml = 'none' | 'p';

export interface PasteStyle {
  /** An empty line between paragraphs. */
  blankLine: boolean;
  /** Also put HTML (`<p>` a paragraph) on the clipboard. */
  html: PasteHtml;
}

export interface PlatformPreset {
  id: string;
  name: string;
  rule: CountRule;
  /** Chapter goal when the platform is picked; null for none. */
  minimum: number | null;
  /** The minimum in a sentence, with what it is for. */
  minimumNote: string;
  /** False when the counting rule is not from the platform itself (docs/platforms.md). */
  ruleConfirmed: boolean;
  paste: PasteStyle;
  /** One sentence about pasting, shown under the clipboard choice; may be empty. */
  pasteNote: string;
}

const RULE_OFF: CountRule = { spaces: false, skipMarks: false, wideTwice: false, htmlEscapes: false };
const WITH_SPACES: CountRule = { ...RULE_OFF, spaces: true };

export const PLATFORMS: PlatformPreset[] = [
  {
    id: 'munpia',
    name: '문피아',
    rule: WITH_SPACES,
    minimum: 4000,
    minimumNote: '유료 연재는 한 회 공백 포함 4,000자 이상으로 알려져 있습니다 (예전 5,000자).',
    ruleConfirmed: false,
    paste: { blankLine: true, html: 'p' },
    pasteNote: '',
  },
  {
    id: 'novelpia',
    name: '노벨피아',
    rule: { spaces: false, skipMarks: true, wideTwice: true, htmlEscapes: true },
    minimum: 3000,
    minimumNote: '플러스 연재와 공모전은 한 회 공백 제외 3,000자 이상입니다.',
    ruleConfirmed: true,
    paste: { blankLine: true, html: 'none' },
    pasteNote: '노벨피아 글쓰기 설정의 ‘텍스트 붙여넣기’와 같은 결과입니다.',
  },
  {
    id: 'kakaopage',
    name: '카카오페이지',
    rule: WITH_SPACES,
    minimum: 5000,
    minimumNote: '계약 작품은 한 회 공백 포함 5,000자 안팎이 많습니다.',
    ruleConfirmed: false,
    paste: { blankLine: true, html: 'none' },
    pasteNote: '담당자에게 파일로 보낼 때는 한글이나 Word로 내보내세요.',
  },
  {
    id: 'naver',
    name: '네이버 시리즈·웹소설',
    rule: WITH_SPACES,
    minimum: 4000,
    minimumNote: '챌린지리그는 한 회 4,000자 이상을 권합니다. 정식 연재는 더 깁니다.',
    ruleConfirmed: false,
    paste: { blankLine: true, html: 'p' },
    pasteNote: '',
  },
  {
    id: 'ridi',
    name: '리디',
    rule: WITH_SPACES,
    minimum: 3000,
    minimumNote: '리디 공모(RI-START)는 한 회 공백 포함 3,000자 이상입니다.',
    ruleConfirmed: true,
    paste: { blankLine: true, html: 'none' },
    pasteNote: '리디는 원고를 파일(txt·docx·한글)로 받습니다. 붙여 넣기보다 내보내기를 쓰세요.',
  },
];

export const CUSTOM_ID = 'custom';

export function presetOf(id: string | null | undefined): PlatformPreset | null {
  return PLATFORMS.find((p) => p.id === id) ?? null;
}

/** The platform a project counts by: web novels only. */
export function platformOf(project: Pick<ProjectInfo, 'kind' | 'platform'>): Platform | null {
  return project.kind === 'webnovel' ? project.platform : null;
}

/** Screen name of a project's platform: "노벨피아", or "내 기준" for one set by hand. */
export function platformName(platform: Platform): string {
  return presetOf(platform.id)?.name ?? '내 기준';
}

export function sameRule(a: CountRule, b: CountRule): boolean {
  return a.spaces === b.spaces && a.skipMarks === b.skipMarks && a.wideTwice === b.wideTwice && a.htmlEscapes === b.htmlEscapes;
}

/** The rule chapter goals count by: the platform's, or 공백 포함/제외. */
export function goalRule(project: Pick<ProjectInfo, 'kind' | 'platform' | 'goal'>): CountRule {
  return platformOf(project)?.rule ?? { ...RULE_OFF, spaces: project.goal.countSpaces };
}

/** Characters counted for chapter goals. */
export function goalChars(project: Pick<ProjectInfo, 'kind' | 'platform' | 'goal'>, counts: Counts): number {
  return countByRule(counts, goalRule(project));
}

/** The rule in a sentence or two, e.g. "띄어쓰기도 글자로 셉니다." */
export function ruleText(rule: CountRule): string {
  const marks = '. , ! ? \' "';
  const base = rule.spaces
    ? rule.skipMarks
      ? `띄어쓰기는 세고 ${marks}는 빼고 셉니다.`
      : '띄어쓰기도 글자로 셉니다.'
    : rule.skipMarks
      ? `띄어쓰기와 ${marks}는 빼고 셉니다.`
      : '띄어쓰기는 빼고 셉니다.';
  const extras = [rule.wideTwice && '이모지는 2자', rule.htmlEscapes && '< > &는 4~5자'].filter(Boolean);
  return extras.length ? `${base} ${extras.join(', ')}로 셉니다.` : base;
}

/** What the clipboard choice is for a project with no remembered choice. */
export function defaultPaste(project: Pick<ProjectInfo, 'kind' | 'platform'>): PasteStyle {
  const preset = presetOf(platformOf(project)?.id);
  return preset ? { ...preset.paste } : { blankLine: project.kind === 'webnovel', html: 'none' };
}
