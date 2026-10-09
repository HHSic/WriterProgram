// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import type { Editor } from '@tiptap/core';
import { Schema } from '@tiptap/pm/model';
import { EditorState, TextSelection } from '@tiptap/pm/state';
import { history, undo } from '@tiptap/pm/history';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { openSymbols, SymbolsDialog } from './Symbols';

const app = vi.hoisted(() => ({ editor: null as Editor | null }));
vi.mock('../store', () => ({ useApp: { getState: () => app }, openDialog: vi.fn(), closeDialog: vi.fn() }));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root;
beforeEach(() => {
  app.editor = null;
  localStorage.clear();
  const host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});
afterEach(() => {
  act(() => root.unmount());
  document.body.replaceChildren();
});

function click(text: string) {
  const button = [...document.querySelectorAll('button')].find((b) => b.textContent === text);
  if (!button) throw new Error(`Missing button: ${text}`);
  act(() => button.click());
}
function show() {
  openSymbols();
  act(() => root.render(<SymbolsDialog />));
}

it('replaces the manuscript selection with one complete face and supports undo', () => {
  const schema = new Schema({ nodes: { doc: { content: 'paragraph+' }, paragraph: { content: 'text*' }, text: {} } });
  const doc = schema.node('doc', null, [schema.node('paragraph', null, schema.text('abcd'))]);
  let state = EditorState.create({ doc, selection: TextSelection.create(doc, 2, 4), plugins: [history()] });
  const dispatch = (tr: Parameters<EditorState['apply']>[0]) => { state = state.apply(tr); };
  app.editor = { isEditable: true, isDestroyed: false, get state() { return state; }, view: { dispatch } } as unknown as Editor;
  show();
  click('텍스트 이모티콘');
  click('(^_^)');
  expect(state.doc.textContent).toBe('a(^_^)d');
  expect(state.selection.from).toBe(7);
  expect(undo(state, dispatch)).toBe(true);
  expect(state.doc.textContent).toBe('abcd');
});

it('keeps internal spaces, caret position and whole faces in recent choices', () => {
  const field = document.createElement('textarea');
  field.value = '앞선택뒤';
  document.body.append(field);
  field.focus();
  field.setSelectionRange(1, 3);
  const input = vi.fn();
  field.addEventListener('input', input);
  show();
  click('텍스트 이모티콘');
  click('인사·응원');
  const face = '(づ￣ ￣)づ';
  click(face);
  expect(field.value).toBe(`앞${face}뒤`);
  expect(field.selectionStart).toBe(1 + face.length);
  click('(^_^)/');
  expect(field.value).toBe(`앞${face}(^_^)/뒤`);
  expect(input).toHaveBeenCalledTimes(2);
  click('최근 쓴 문자');
  expect(document.querySelector('.emoticon-grid')).not.toBeNull();
  click(face);
  expect(field.value).toBe(`앞${face}(^_^)/${face}뒤`);
  expect(JSON.parse(localStorage.getItem('wp.recentSymbols')!)).toEqual([face, '(^_^)/']);
});

it('does not insert into a locked editor', () => {
  const dispatch = vi.fn();
  app.editor = { isEditable: false, isDestroyed: false, view: { dispatch } } as unknown as Editor;
  show();
  click('텍스트 이모티콘');
  click('^_^');
  expect(dispatch).not.toHaveBeenCalled();
  expect(document.body.textContent).toContain('넣을 곳이 없습니다');
  expect(localStorage.getItem('wp.recentSymbols')).toBeNull();
});

it('inserts Unicode faces without normalizing combining marks and reuses them from recent choices', () => {
  const field = document.createElement('textarea');
  field.value = '앞뒤';
  document.body.append(field);
  field.focus();
  field.setSelectionRange(1, 1);
  show();
  click('텍스트 이모티콘');
  click('유니코드 표정');
  expect(document.body.textContent).toContain('글꼴을 확인하세요');
  const face = '(ง •̀_•́)ง';
  click(face);
  expect(field.value).toBe(`앞${face}뒤`);
  expect(field.selectionStart).toBe(1 + face.length);
  click('유니코드 기호');
  click('☺');
  click('최근 쓴 문자');
  click(face);
  expect(field.value).toBe(`앞${face}☺${face}뒤`);
  expect(JSON.parse(localStorage.getItem('wp.recentSymbols')!)).toEqual([face, '☺']);
});
