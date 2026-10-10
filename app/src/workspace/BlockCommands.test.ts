// @vitest-environment jsdom
import { Editor } from '@tiptap/core';
import { afterEach, expect, it } from 'vitest';
import { manuscriptExtensions } from '../editor/extensions';
import { slashQuery, BLOCK_COMMANDS } from './BlockCommands';
let e:Editor;
afterEach(()=>e?.destroy());
function create(text:string) {e=new Editor({extensions:manuscriptExtensions('◆',{onCardOpen(){},onNoteOpen(){},onNoteAdd(){}}),content:{type:'doc',content:[{type:'paragraph',content:[{type:'text',text}]}]}});e.commands.setTextSelection(text.length+1);return e;}
it('only treats a slash at the beginning of a paragraph as a block command',()=>{
  const e=create('/부제목');expect(slashQuery(e)).toEqual({from:1,to:5,query:'부제목'});
  e.commands.setContent('<p>문장 /제목</p>');e.commands.setTextSelection(7);expect(slashQuery(e)).toBeNull();
  e.commands.setContent('<p>/제목</p>');e.commands.setTextSelection({from:1,to:3});expect(slashQuery(e)).toBeNull();e.setEditable(false);expect(slashQuery(e)).toBeNull();
});
it('applies styles and dispatches synopsis commands without adding metadata to the manuscript',()=>{
  const e=create('/');const events:string[]=[];
  e.view.dom.addEventListener('writer:synopsis',event=>events.push((event as CustomEvent<string>).detail));
  BLOCK_COMMANDS.find(c=>c.label==='부제목')!.run(e);expect(e.state.doc.child(0).attrs.heading).toBe(2);
  BLOCK_COMMANDS.find(c=>c.label==='시놉시스 추가')!.run(e);BLOCK_COMMANDS.find(c=>c.label==='시놉시스 삭제')!.run(e);expect(events).toEqual(['add','remove']);expect(e.getText()).toBe('/');
});
