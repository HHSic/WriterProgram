// The in-app browser (prototype).

/** A page in the in-app browser, for 자료로 보관. */
export interface PageClip {
  url: string;
  title: string;
  /** Selected text, if any. */
  text: string;
}

/** Word from the in-app browser: a page's address, title, or loading. */
export interface PageEvent {
  label: string;
  url?: string;
  title?: string;
  loading?: boolean;
}

/** Where a browser tab sits in the window, in CSS pixels. */
export interface Bounds {
  x: number;
  y: number;
  w: number;
  h: number;
}
