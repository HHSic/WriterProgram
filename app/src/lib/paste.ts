// 붙여넣기 서식: what the clipboard export writes so that pasting into a
// platform's editor gives the spacing the writer meant (docs/platforms.md).
// Platform editors are HTML editors; pasted plain text is turned into
// paragraphs by the browser and the editor, which is where one line break can
// become two or three lines. With `html: 'p'` each line goes in as its own
// `<p>` and an empty line as an empty paragraph, the same as pressing Enter.

import type { PasteStyle } from './platforms';

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

/** Text as HTML paragraphs: one `<p>` a line, `<p><br></p>` for an empty line. */
export function pasteHtml(text: string): string {
  return text
    .split('\n')
    .map((line) => (line ? `<p>${escapeHtml(line)}</p>` : '<p><br></p>'))
    .join('');
}

/** Puts `text` on the clipboard, with HTML paragraphs too when the style asks for it. */
export async function writeClipboard(text: string, style: PasteStyle): Promise<void> {
  if (style.html === 'p' && typeof ClipboardItem !== 'undefined' && navigator.clipboard.write) {
    try {
      await navigator.clipboard.write([
        new ClipboardItem({
          'text/plain': new Blob([text], { type: 'text/plain' }),
          'text/html': new Blob([pasteHtml(text)], { type: 'text/html' }),
        }),
      ]);
      return;
    } catch {
      // Some webviews take text only; text alone still pastes.
    }
  }
  await navigator.clipboard.writeText(text);
}

/** The first lines as they will stand in the editor: text lines and empty lines. */
export function previewLines(text: string, max = 12): { text: string; blank: boolean }[] {
  return text
    .split('\n')
    .slice(0, max)
    .map((line) => ({ text: line, blank: line.trim() === '' }));
}

const KEY = 'wp.paste.';

/** The clipboard choice last used in this project, or null. */
export function loadPaste(projectId: string): (PasteStyle & { preset: string }) | null {
  try {
    const raw = localStorage.getItem(KEY + projectId);
    if (!raw) return null;
    const v = JSON.parse(raw) as Partial<PasteStyle & { preset: string }>;
    if (typeof v.blankLine !== 'boolean' || (v.html !== 'none' && v.html !== 'p')) return null;
    return { blankLine: v.blankLine, html: v.html, preset: typeof v.preset === 'string' ? v.preset : '' };
  } catch {
    return null;
  }
}

export function savePaste(projectId: string, style: PasteStyle & { preset: string }) {
  try {
    localStorage.setItem(KEY + projectId, JSON.stringify(style));
  } catch {
    // Remembering is only a convenience.
  }
}
