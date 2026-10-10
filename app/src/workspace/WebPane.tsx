// 앱 안 브라우저 (시제품): a web page in a tab of the middle column. The page
// itself is a separate webview the app lays over the empty area below the bar
// (app/src-tauri/src/browser.rs); this keeps it in the right place, and hides
// it while its tab is in the back or a menu or dialog is open, since it always
// draws on top of the app.

import { useEffect, useRef, useState, type FormEvent } from 'react';
import { api } from '../api';
import type { Bounds } from '../api/types';
import { Icon } from '../components/Icon';
import { useMenuBounds } from '../components/Menu';
import { clipPage, rememberWebPage, useApp, webPages } from '../store';

const HOME = 'https://www.google.com/';

export function WebPane({ id, active }: { id: string; active: boolean }) {
  const projectId = useApp((s) => s.overview!.project.id);
  const covered = useApp((s) => s.dialog !== null || s.busy !== null);
  const menuBounds = useMenuBounds();
  const label = `web-${id}`;
  const saved = webPages(projectId)[id];
  const [url, setUrl] = useState(saved?.url || HOME);
  const [typed, setTyped] = useState(saved?.url || HOME);
  const [loading, setLoading] = useState(false);
  const [editing, setEditing] = useState(false);
  const host = useRef<HTMLDivElement>(null);
  const opened = useRef(false);
  const mounted = useRef(true);
  const placement = useRef<{at:Bounds; show:boolean}|null>(null);
  const [preview,setPreview] = useState<string|null>(null);
  const previewGeneration = useRef(0);
  const capture = () => {
    const generation = ++previewGeneration.current;
    void api.browserPreview(label).then(image => { if(generation === previewGeneration.current) setPreview(image); }).catch(() => {});
  };
  const visible = active && !covered && api.isDesktop;

  // Address, title and loading from the page.
  useEffect(
    () =>
      api.onBrowserPage((e) => {
        if (e.label !== label) return;
        if (e.url) {
          setUrl(e.url);
          if (!editing) setTyped(e.url);
          rememberWebPage(projectId, id, { url: e.url });
        }
        if (e.title !== undefined) rememberWebPage(projectId, id, { title: e.title });
        if (e.loading !== undefined) { setLoading(e.loading); if(!e.loading) capture(); }
      }),
    [label, projectId, id, editing],
  );

  // Keep the page over the area, or out of sight.
  useEffect(() => {
    if (!api.isDesktop) return;
    const el = host.current;
    if (!el) return;
    let last = '';
    const place = () => {
      const r = el.getBoundingClientRect();
      const at: Bounds = { x: Math.round(r.left), y: Math.round(r.top), w: Math.round(r.width), h: Math.round(r.height) };
      const overlap = menuBounds && menuBounds.left < r.right && menuBounds.right > r.left && menuBounds.top < r.bottom && menuBounds.bottom > r.top;
      const show = visible && !overlap && at.w > 0 && at.h > 0;
      placement.current = {at,show};
      const key = `${show}:${at.x},${at.y},${at.w},${at.h}`;
      if (key === last) return;
      last = key;
      if (show && !opened.current) {
        opened.current = true;
        void api.browserOpen(label, url, at).then(async () => {
          if(!mounted.current) { await api.browserClose(label); return; }
          const latest = placement.current;
          if(latest) await api.browserBounds(label,latest.at,latest.show);
          capture();
        }).catch(() => {
          opened.current = false;
        });
      } else if (opened.current) {
        void api.browserBounds(label, at, show).catch(() => { last = ''; });
      }
    };
    place();
    const observer = new ResizeObserver(place);
    observer.observe(el);
    observer.observe(document.body);
    window.addEventListener('resize', place);
    // Panels opening beside it move it without resizing it.
    const timer = setInterval(place, 400);
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', place);
      clearInterval(timer);
    };
    // `url` only matters for the first opening.
  }, [label, visible, menuBounds]); // eslint-disable-line react-hooks/exhaustive-deps

  // Closing the tab closes the page.
  useEffect(
    () => () => {
      mounted.current = false;
      previewGeneration.current++;
      if (opened.current) void api.browserClose(label);
    },
    [label],
  );

  const go = async (e: FormEvent) => {
    e.preventDefault();
    setEditing(false);
    try {
      const next = await api.browserNavigate(label, typed);
      setTyped(next);
    } catch {
      // Not open yet: it opens with this address.
      setUrl(typed);
    }
  };

  return (
    <div className="web-pane">
      <form className="web-bar" onSubmit={(e) => void go(e)}>
        <button type="button" className="icon-btn tiny" aria-label="뒤로" title="뒤로" onClick={() => void api.browserStep(label, 'back')}>
          <Icon name="back" size={15} />
        </button>
        <button type="button" className="icon-btn tiny" aria-label="앞으로" title="앞으로" onClick={() => void api.browserStep(label, 'forward')}>
          <Icon name="forward" size={15} />
        </button>
        <button
          type="button"
          className={`icon-btn tiny${loading ? ' spinning' : ''}`}
          aria-label="새로 고침"
          title="새로 고침"
          onClick={() => void api.browserStep(label, 'reload')}
        >
          <Icon name="reload" size={15} />
        </button>
        <input
          className="web-address"
          value={typed}
          aria-label="주소 또는 검색어"
          placeholder="주소 또는 검색어"
          spellCheck={false}
          onFocus={(e) => {
            setEditing(true);
            e.target.select();
          }}
          onBlur={() => {
            setEditing(false);
            setTyped(url);
          }}
          onChange={(e) => setTyped(e.target.value)}
        />
        <button type="button" className="btn small" title="이 페이지(고른 글이 있으면 그 글까지)를 메모함에 보관" onClick={() => void clipPage(label)}>
          자료로 보관
        </button>
      </form>
      <div ref={host} className="web-host">
        {api.isDesktop && !preview && covered && <p className="pane-message">열어 둔 웹 페이지는 이 창을 닫으면 계속 사용할 수 있습니다.</p>}
        {api.isDesktop && preview && <img className="web-preview" src={preview} alt="열어 둔 웹 페이지" draggable={false}/> }
        {!api.isDesktop && <p className="pane-message">앱 안 브라우저는 데스크톱 앱에서만 보입니다.</p>}
      </div>
    </div>
  );
}
