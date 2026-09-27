import { Icon } from '../components/Icon';
import { flushAll } from '../lib/flush';
import { UNTITLED, docNumber } from '../lib/labels';
import { findDoc, openDialog, useApp } from '../store';

export function CenterHead() {
  const ov = useApp((s) => s.overview)!;
  const activeDocId = useApp((s) => s.activeDocId);
  const editor = useApp((s) => s.editor);
  const save = useApp((s) => s.save);
  const rightOpen = useApp((s) => s.rightOpen);
  const place = activeDocId ? findDoc(ov, activeDocId) : null;

  return (
    <div className="col-head center-head">
      <nav className="crumbs" aria-label="현재 위치">
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
      <SaveIndicator state={save.state} error={save.error} />
      <button
        type="button"
        className="icon-btn"
        aria-label="장면 나눔 넣기"
        title="장면 나눔 넣기 (빈 줄에 ***를 쓰고 Enter)"
        disabled={!editor}
        onClick={() => editor?.chain().focus().insertSceneBreak().run()}
      >
        <Icon name="diamond" size={16} />
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
