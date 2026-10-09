import { FontSizeInput } from '../components/FontSizeInput';
import { BOOK_PAPERS, type BookPreferences } from '../lib/book';

export function ReaderSettings({ preferences, update, fullscreen, onFullscreen, error }: {
  preferences: BookPreferences; update: (patch: Partial<BookPreferences>) => void;
  fullscreen: boolean; onFullscreen: () => void; error: string;
}) {
  const preset = BOOK_PAPERS.find((p) => p.width === preferences.pageWidth && p.height === preferences.pageHeight)?.name ?? 'custom';
  return <aside className="reader-settings" aria-label="책 미리보기 설정">
    <h3>미리보기 설정</h3>
    <button type="button" className="btn" aria-pressed={fullscreen} onClick={onFullscreen}>{fullscreen ? '전체화면 나가기' : '전체화면'}</button>
    {error && <p role="alert">{error}</p>}
    <fieldset><legend>읽기 화면</legend><div className="segmented" role="group" aria-label="읽기 화면">
      {([['auto', '자동'], ['phone', '휴대폰'], ['pc', 'PC']] as const).map(([id, label]) => <button type="button" key={id}
        className={`btn${preferences.device === id ? ' on' : ''}`} aria-pressed={preferences.device === id}
        onClick={() => update({ device: id })}>{label}</button>)}
    </div></fieldset>
    <fieldset><legend>한 쪽 크기</legend>
      <label>종이 <select aria-label="미리보기 종이 크기" value={preset} onChange={(e) => {
        const paper = BOOK_PAPERS.find((p) => p.name === e.target.value);
        if (paper) update({ pageWidth: paper.width, pageHeight: paper.height });
      }}><option value="custom">사용자 지정</option>{BOOK_PAPERS.map((p) => <option key={p.name} value={p.name}>{p.name} ({p.width} × {p.height} mm)</option>)}</select></label>
      <label>가로 <FontSizeInput label="책 가로(mm)" min={20} value={preferences.pageWidth} onChange={(pageWidth) => update({ pageWidth })} /><span>mm</span></label>
      <label>세로 <FontSizeInput label="책 세로(mm)" min={20} value={preferences.pageHeight} onChange={(pageHeight) => update({ pageHeight })} /><span>mm</span></label>
      <button type="button" className="btn" onClick={() => update({ pageWidth: preferences.pageHeight, pageHeight: preferences.pageWidth })}>가로·세로 바꾸기</button>
      <p className="hint">선택한 종이 크기로 쪽을 나누고 화면에 맞춰 축소합니다. PC는 두 쪽을 나란히 보여줍니다.</p>
    </fieldset>
    <fieldset><legend>글자·배경</legend>
      <div className="reader-type-controls" role="group" aria-label="읽기 글자 크기">
        <button type="button" className="btn" aria-label="글자 작게" disabled={preferences.fontSize <= 1}
          onClick={() => update({ fontSize: Math.max(1, preferences.fontSize - 2) })}>가−</button>
        <FontSizeInput label="읽기 글자 크기(px)" value={preferences.fontSize} onChange={(fontSize) => update({ fontSize })} /><span>px</span>
        <button type="button" className="btn" aria-label="글자 크게" onClick={() => update({ fontSize: preferences.fontSize + 2 })}>가+</button>
      </div>
      <label>배경 <select aria-label="읽기 배경" value={preferences.theme} onChange={(e) => update({ theme: e.target.value as BookPreferences['theme'] })}>
        <option value="paper">종이</option><option value="light">밝게</option><option value="dark">어둡게</option>
      </select></label>
    </fieldset>
  </aside>;
}
