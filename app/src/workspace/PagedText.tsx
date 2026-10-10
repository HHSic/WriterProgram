// One continuous editor, with non-persistent gaps and vertically stacked sheets.
import { useLayoutEffect, useRef, useState } from 'react';
import { EditorContent, type Editor } from '@tiptap/react';
import { DecorationSet } from '@tiptap/pm/view';
import type { ManuscriptFormat } from '../api/types';
import type { ViewSettings } from '../lib/view';
import { PAGE_GAP, pageGeometry } from '../lib/pagination';
import { pageGap, verticalPagesKey } from '../editor/verticalPages';

export function PagedText({ editor, format, view }: { editor: Editor; format: ManuscriptFormat; view: ViewSettings }) {
  const sheet = useRef<HTMLDivElement>(null);
  const [layout,setLayout] = useState({ pages:1,height:800,bottom:60 });
  useLayoutEffect(()=>{
    const paper=sheet.current;
    if(!paper) return;
    let alive=true, measuring=false, measuredWidth=0;
    let timer:ReturnType<typeof setTimeout>;
    const dispatch=(value:DecorationSet)=>editor.view.dispatch(editor.state.tr.setMeta(verticalPagesKey,value).setMeta('addToHistory',false));
    const measure=()=>{
      if(!alive||measuring||editor.isDestroyed||editor.view.composing||!paper.clientWidth) return;
      measuring=true;
      const scroll=paper.closest<HTMLElement>('.doc-scroll');
      const caretBefore=editor.view.coordsAtPos(editor.state.selection.head).top;
      const scrollBox=scroll?.getBoundingClientRect();
      const holdCaret=editor.isFocused && scrollBox && caretBefore>=scrollBox.top && caretBefore<=scrollBox.bottom;
      const geometry=pageGeometry(paper.clientWidth,format);
      const height=geometry.top+geometry.height+geometry.bottom,step=height+PAGE_GAP;
      for(const [key,value] of Object.entries({left:geometry.left,right:geometry.right,top:geometry.top,bottom:geometry.bottom})) paper.style.setProperty(`--sheet-${key}`,`${value}px`);
      dispatch(DecorationSet.empty);
      const dom=editor.view.dom, top=dom.getBoundingClientRect().top;
      const gaps:ReturnType<typeof pageGap>[]=[];
      let page=0, added=0;
      // All positions are measured in natural flow before adding decorations.
      editor.state.doc.forEach((node,pos)=>{
        const element=editor.view.nodeDOM(pos) as HTMLElement | null;
        if(!element?.getBoundingClientRect) return;
        const rect=element.getBoundingClientRect();
        const start=rect.top-top;
        const end=rect.bottom-top;
        const nextPage=(at:number,y:number,inline:boolean)=>{
          // Round to the next sheet's content top, including unused space.
          const gap=(page+1)*step-(y+added);
          if(gap>0.5) { gaps.push(pageGap(at,gap,inline)); added+=gap; page++; }
        };
        if(node.attrs.pageBreakBefore && start+added>page*step+1) nextPage(pos,start,false);
        const bottom=page*step+geometry.height;
        if(end+added<=bottom+0.5) return;
        if(!node.isTextblock||node.content.size===0) {
          if(start+added>page*step+1) nextPage(pos,start,false);
          return;
        }
        let offset=0;
        // Locate first character of each overflowing line with logarithmic reads.
        while(offset<node.content.size) {
          const limit=page*step+geometry.height-added;
          const last=editor.view.coordsAtPos(pos+node.nodeSize-1);
          if(last.bottom-top<=limit+0.5) break;
          let lo=offset,hi=node.content.size;
          while(lo<hi) {
            const mid=Math.floor((lo+hi)/2);
            const r=editor.view.coordsAtPos(pos+1+mid);
            if(r.bottom-top>limit+0.5) hi=mid; else lo=mid+1;
          }
          if(lo>=node.content.size) break;
          const r=editor.view.coordsAtPos(pos+1+lo);
          if(r.top-top+added<=page*step+1) { // A font taller than the paper: let it overflow once.
            break;
          }
          if(lo===0) nextPage(pos,start,false);
          else nextPage(pos+1+lo,r.top-top,true);
          offset=lo+1;
        }
      });
      dispatch(DecorationSet.create(editor.state.doc,gaps));
      setLayout({pages:page+1,height,bottom:geometry.bottom});
      if(holdCaret&&scroll) scroll.scrollTop+=editor.view.coordsAtPos(editor.state.selection.head).top-caretBefore;
      measuredWidth=paper.clientWidth; measuring=false;
    };
    const schedule=()=>{ clearTimeout(timer); timer=setTimeout(measure,80); };
    measure(); editor.on('update',schedule);
    const observer=typeof ResizeObserver==='undefined'?null:new ResizeObserver(()=>{if(paper.clientWidth!==measuredWidth) schedule();});
    observer?.observe(paper); paper.addEventListener('compositionend',schedule); window.addEventListener('resize',schedule); void document.fonts?.ready.then(schedule);
    return()=>{alive=false;clearTimeout(timer);observer?.disconnect();editor.off('update',schedule);paper.removeEventListener('compositionend',schedule);window.removeEventListener('resize',schedule);if(!editor.isDestroyed) dispatch(DecorationSet.empty);};
  },[editor,format,view.font,view.fontSize,view.lineHeight,view.letterSpacing,view.paragraphGap,view.indent,view.width]);
  return <><div ref={sheet} className="vertical-pages" style={{minHeight:layout.pages*(layout.height+PAGE_GAP)-PAGE_GAP}}>
    <div className="sheet-backgrounds" aria-hidden="true">{Array.from({length:layout.pages},(_,i)=><div key={i} className="vertical-sheet" data-page={i+1} style={{top:i*(layout.height+PAGE_GAP),height:layout.height}}><span className="sheet-page-number" style={{bottom:Math.max(8,layout.bottom/3)}}>{i+1}</span></div>)}</div>
    <div className="paged-viewport vertical-page-text"><EditorContent editor={editor}/></div>
  </div><p className="page-count" role="status">{layout.pages}쪽 · 세로로 스크롤 · 쪽 나눔: Ctrl+Enter</p></>;
}
