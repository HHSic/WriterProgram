// @vitest-environment jsdom
import { Editor } from '@tiptap/core';
import { afterEach, expect, it } from 'vitest';
import { manuscriptExtensions } from './extensions';
import { sceneTextPosition } from './sceneNavigation';
let editor: Editor;
afterEach(() => editor?.destroy());
function create() {
  editor = new Editor({ extensions: manuscriptExtensions('◆', { onCardOpen() {}, onNoteOpen() {}, onNoteAdd() {} }),
    content: { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: '첫 문단' }] },
      { type: 'paragraph', content: [{ type: 'text', text: '둘째 문단' }] }] } });
  return editor;
}
it('formats every selected paragraph and restores colour and alignment from JSON', () => {
  const e = create();
  e.commands.setTextSelection({ from: 1, to: 10 });
  e.commands.setParagraphAlign('center');
  e.commands.setTextColor('#2365b8');
  const saved = e.getJSON();
  expect(saved.content?.every((p) => p.attrs?.align === 'center')).toBe(true);
  expect(saved.content?.[0].content?.[0].marks).toContainEqual({ type: 'textColor', attrs: { color: '#2365B8' } });
  e.commands.setContent(saved);
  expect(e.getHTML()).toContain('data-align="center"');
  expect(e.getHTML()).toContain('data-color="#2365B8"');
  expect(e.commands.setTextColor('red; background:url(https://example.com)')).toBe(false);
  e.commands.setTextSelection({ from: 1, to: 10 });
  e.commands.unsetTextColor();
  expect(e.getJSON().content?.[0].content?.[0].marks).toBeUndefined();
});
it('splits at the caret for a page break without repeating the break on later Enter', () => {
  const e = create();
  e.commands.setTextSelection(3);
  e.commands.insertPageBreak();
  expect(e.state.doc.child(0).textContent).toBe('첫 ');
  expect(e.state.doc.child(1).textContent).toBe('문단');
  expect(e.state.doc.child(1).attrs.pageBreakBefore).toBe(true);
  e.commands.splitBlock();
  expect(e.state.doc.child(2).attrs.pageBreakBefore).toBe(false);
  e.commands.undo(); e.commands.undo();
  expect(e.state.doc.child(0).textContent).toBe('첫 문단');
});
it('finds the first actual scene character across blank paragraphs, tabs, marks and line breaks', () => {
  const e = create();
  e.commands.setContent({ type: 'doc', content: [{ type: 'paragraph' }, { type: 'sceneBreak' }, { type: 'paragraph' },
    { type: 'paragraph', content: [{ type: 'text', text: '  \t', marks: [{ type: 'bold' }] }, { type: 'hardBreak' },
      { type: 'text', text: '  장면 첫 글자' }] }] });
  const pos = sceneTextPosition(e.state.doc, 3);
  expect(e.state.doc.textBetween(pos, pos + 1)).toBe('장');
});
it('persists in-body heading styles and returns to normal body on Enter', () => {
  const e=create(); e.commands.setTextSelection(3); e.commands.setParagraphHeading(2);
  expect(e.getHTML()).toContain('data-heading="2"');
  const saved=e.getJSON(); e.commands.setContent(saved);
  expect(e.state.doc.child(0).attrs.heading).toBe(2);
  e.commands.splitBlock(); expect(e.state.doc.child(0).attrs.heading).toBe(2); expect(e.state.doc.child(1).attrs.heading).toBeNull();
  expect(e.commands.setParagraphHeading(9)).toBe(false);
});
