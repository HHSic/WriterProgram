// 원고 서식: paper, margins, type, 머리말 and 꼬리말, and the saved presets.

/** 원고 서식 (crates/core/src/format.rs). mm, pt, %. */
export interface ManuscriptFormat {
  preset: string;
  paper: { kind: string; widthMm: number; heightMm: number };
  margins: { top: number; bottom: number; inside: number; outside: number; header: number; footer: number };
  font: string;
  sizePt: number;
  lineSpacing: number;
  letterSpacing: number;
  indent: number;
  blankLineBetween: boolean;
  chapterNewPage: boolean;
  pageNumbers: boolean;
  /** Where the page number sits at the bottom. */
  pageNumberAlign: HeadAlign;
  /** 머리말 (crates/core/src/format.rs RunningHead). */
  header: RunningHead;
  /** 꼬리말: the writer's own line at the bottom, beside the page number. */
  footer: RunningFoot;
  /** Where the first-line indent is left out (crates/core/src/indent.rs). */
  indentRules: IndentRules;
}

export interface IndentRules {
  /** The first paragraph of a chapter. */
  chapterFirst: boolean;
  /** The first paragraph after a scene break. */
  afterScene: boolean;
  /** Paragraphs set in with margins (letters, quotations). */
  margined: boolean;
  /** Dialogue (opening with a quotation mark). */
  dialogue: boolean;
  /** Dialogue as on 원고지: every line one cell in. */
  dialogueHang: boolean;
}

export type HeadContent = 'none' | 'title' | 'chapter' | 'titleChapter' | 'author' | 'custom';
export type HeadAlign = 'left' | 'center' | 'right' | 'outside';

export interface RunningHead {
  content: HeadContent;
  /** For content 'custom'. */
  text: string;
  align: HeadAlign;
  /** Left off each chapter's first page (when chapters start on a new page). */
  skipChapterFirst: boolean;
}

export interface RunningFoot {
  /** Empty when there is no 꼬리말. */
  text: string;
  align: HeadAlign;
}

export interface UserPreset {
  name: string;
  format: ManuscriptFormat;
}

export interface FormatCatalog {
  builtin: { id: string; name: string; format: ManuscriptFormat }[];
  user: UserPreset[];
  fonts: { key: string; label: string }[];
  papers: { key: string; label: string; widthMm: number; heightMm: number }[];
}
