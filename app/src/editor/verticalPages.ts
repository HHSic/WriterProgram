// Page gaps are view decorations, never manuscript nodes or undo entries.
import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
export const verticalPagesKey = new PluginKey<DecorationSet>('verticalPages');
export const VerticalPages = Extension.create({
  name:'verticalPages',
  addProseMirrorPlugins() { return [new Plugin({ key:verticalPagesKey,
    state:{ init:()=>DecorationSet.empty, apply:(tr,old)=>tr.getMeta(verticalPagesKey) ?? old.map(tr.mapping,tr.doc) },
    props:{ decorations:state=>verticalPagesKey.getState(state) },
  })]; },
});
export function pageGap(pos:number,height:number,inline:boolean) {
  return Decoration.widget(pos,()=>{
    const el=document.createElement(inline?'span':'div');
    el.className='editor-page-gap'; el.contentEditable='false'; el.setAttribute('aria-hidden','true');
    el.style.height=`${Math.max(0,height)}px`;
    return el;
  },{side:-1,ignoreSelection:true});
}
