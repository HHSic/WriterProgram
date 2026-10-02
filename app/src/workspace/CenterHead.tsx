import { Icon } from '../components/Icon';
import { openMenu } from '../components/Menu';
import { flushAll } from '../lib/flush';
import { UNTITLED, docNumber } from '../lib/labels';
import { findDoc, openDialog, openWeb, splitView, toggleReading, unsplit, useApp, webPages } from '../store';
import { marginMenu } from './EditToolbar';
import { openSymbols } from './Symbols';

export function CenterHead() {
  const ov = useApp((s) => s.overview)!;
  const activeDocId = useApp((s) => s.activeDocId);
  const editor = useApp((s) => s.editor);
  const save = useApp((s) => s.save);
  const conflict = useApp((s) => (s.activeDocId ? s.conflicts[s.activeDocId] : undefined));
  const rightOpen = useApp((s) => s.rightOpen);
  const activeCardId = useApp((s) => s.activeCardId);
  const target = useApp((s) => s.activeTarget);
  const split = useApp((s) => (s.panes.length > 1 ? s.split : null));
  const reading = useApp((s) => s.reading !== null && s.reading.status !== 'noVoice' && s.reading.editor === s.editor);
  const card = activeCardId ? ov.cards.find((c) => c.id === activeCardId) : undefined;
  const place = activeDocId ? findDoc(ov, activeDocId) : null;

  return (
    <div className="col-head center-head">
      <nav className="crumbs" aria-label="현재 위치">
        {target?.kind === 'notes' && <strong>메모함</strong>}
        {target?.kind === 'web' && (
          <>
            <span>웹</span>
            <Icon name="chevronRight" size={12} />
            <strong>{webPages(ov.project.id)[target.id]?.title || '새 페이지'}</strong>
          </>
        )}
        {target?.kind === 'table' && (
          <>
            <span>개요 표</span>
            <Icon name="chevronRight" size={12} />
            <strong>{ov.parts.find((p) => p.id === target.id)?.title}</strong>
          </>
        )}
        {activeCardId && (
          <>
            <span>설정집</span>
            <Icon name="chevronRight" size={12} />
            <span>{ov.cardTypes.find((t) => t.id === card?.cardType)?.name}</span>
            <Icon name="chevronRight" size={12} />
            <strong>{card?.name || '이름 없음'}</strong>
          </>
        )}
        {place && (
          <>
            <span>{place.part ? place.part.title : '기획'}</span>
            <Icon name="chevronRight" size={12} />
            <strong>
              {place.number !== null && `${docNumber(ov.project.kind, place.number)} · `}
              {place.doc.title || UNTITLED}
            </strong>
          </>
        )}
      </nav>
      <div className="grow" />
      {conflict ? (
        <span className="save-state conflict" role="status" title="다른 기기에서 고친 내용과 달라 저장을 멈춤">
          <span className="dot-mark" />
          다른 기기에서 바뀜
        </span>
      ) : (
        <SaveIndicator state={save.state} error={save.error} />
      )}
      {activeDocId && (
        <button
          type="button"
          className="icon-btn"
          aria-label="장면 나눔 넣기"
          title="장면 나눔 넣기 (빈 줄에 ***를 쓰고 Enter)"
          disabled={!editor || !editor.isEditable}
          onClick={() => editor?.chain().focus().insertSceneBreak().run()}
        >
          <Icon name="diamond" size={16} />
        </button>
      )}
      {activeDocId && (
        <button
          type="button"
          className="icon-btn"
          aria-label="문단 여백"
          title="문단 여백: 문단을 통째로 들이기 (Ctrl+] / Ctrl+[)"
          disabled={!editor || !editor.isEditable}
          onClick={(e) => editor && openMenu(e, marginMenu(editor), { title: '문단 여백' })}
        >
          <Icon name="indent" size={17} />
        </button>
      )}
      {activeDocId && (
        <button
          type="button"
          className={`icon-btn${reading ? ' on' : ''}`}
          aria-label={reading ? '소리 내어 읽기 멈춤' : '소리 내어 읽기'}
          aria-pressed={reading}
          title={reading ? '소리 내어 읽기 멈춤 (Ctrl+Shift+R)' : '소리 내어 읽기: 커서가 있는 문장부터, 고른 글이 있으면 그 글만 (Ctrl+Shift+R)'}
          disabled={!editor}
          onMouseDown={(e) => e.preventDefault()}
          onClick={toggleReading}
        >
          <Icon name="speaker" size={17} />
        </button>
      )}
      <button
        type="button"
        className="icon-btn"
        aria-label="문자표"
        title="문자표: 특수 문자와 빈칸 넣기 (Ctrl+F10)"
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => openSymbols()}
      >
        <Icon name="symbol" size={17} />
      </button>
      <button
        type="button"
        className="icon-btn"
        aria-label="웹 보기"
        title="웹 보기: 새 탭에 브라우저 열기 (자료 조사)"
        onClick={() => void openWeb()}
      >
        <Icon name="globe" size={17} />
      </button>
      <button
        type="button"
        className={`icon-btn${split ? ' on' : ''}`}
        aria-label="나눠 보기"
        aria-pressed={split !== null}
        title="나눠 보기 (Ctrl+\)"
        onClick={(e) =>
          openMenu(e, [
            { label: '좌우로 나눠 보기', checked: split === 'row', onSelect: () => splitView('row') },
            { label: '위아래로 나눠 보기', checked: split === 'column', onSelect: () => splitView('column') },
            { separator: true },
            { label: '나눠 보기 닫기', disabled: split === null, onSelect: () => void unsplit() },
          ])
        }
      >
        <Icon name={split === 'column' ? 'splitDown' : 'split'} size={17} />
      </button>
      <button type="button" className="icon-btn" aria-label="보기 설정" title="보기 설정" onClick={() => openDialog({ kind: 'view' })}>
        <Icon name="type" size={18} />
      </button>
      <button
        type="button"
        className={`icon-btn${rightOpen ? ' on' : ''}`}
        aria-label={rightOpen ? '오른쪽 패널 닫기' : '오른쪽 패널 열기'}
        aria-pressed={rightOpen}
        title="오른쪽 패널"
        onClick={() => useApp.setState({ rightOpen: !rightOpen })}
      >
        <Icon name="panel" size={18} />
      </button>
    </div>
  );
}

function SaveIndicator({ state, error }: { state: 'saved' | 'saving' | 'error'; error?: string }) {
  if (state === 'error') {
    return (
      <button type="button" className="save-state error" title={`${error ?? ''} · 눌러서 다시 저장`} onClick={() => void flushAll()}>
        <span className="dot-mark" />
        저장하지 못함 · 다시 시도
      </button>
    );
  }
  return (
    <span className={`save-state ${state}`} role="status">
      <span className="dot-mark" />
      {state === 'saving' ? '저장 중' : '저장됨'}
    </span>
  );
}
