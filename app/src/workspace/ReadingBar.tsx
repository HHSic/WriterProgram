// The small bar over a chapter while it is read aloud (소리 내어 읽기):
// 일시정지 / 계속, 멈춤 and the speed. Also where the "no Korean voice"
// message shows.

import type { Editor } from '@tiptap/core';
import { Icon } from '../components/Icon';
import { READ_RATES } from '../lib/view';
import { closeReadingNotice, pauseReading, resumeReading, setView, stopReading, useApp } from '../store';

/** How to get a Korean voice on Windows (also shown in 보기 설정). */
export const NO_VOICE_HELP = 'Windows 설정 → 시간 및 언어 → 음성 → ‘음성 추가’에서 한국어를 설치한 뒤 앱을 다시 여세요.';

export function rateLabel(rate: number): string {
  return `${rate.toFixed(1)}배`;
}

export function ReadingBar({ editor }: { editor: Editor }) {
  const status = useApp((s) => (s.reading?.editor === editor ? s.reading.status : null));
  const rate = useApp((s) => s.view.readRate);
  if (!status) return null;

  if (status === 'noVoice') {
    return (
      <div className="reading-bar notice" role="alert">
        <div className="reading-notice">
          <strong>한국어 목소리가 없어 읽을 수 없음</strong>
          <span>{NO_VOICE_HELP}</span>
        </div>
        <button type="button" className="icon-btn" aria-label="닫기" title="닫기" onClick={closeReadingNotice}>
          <Icon name="close" size={16} />
        </button>
      </div>
    );
  }

  const reading = status === 'reading';
  return (
    <div className="reading-bar" role="toolbar" aria-label="소리 내어 읽기">
      <span className={`reading-state${reading ? ' on' : ''}`} role="status">
        <Icon name="speaker" size={15} />
        {reading ? '읽는 중' : '일시정지됨'}
      </span>
      <button
        type="button"
        className="btn small"
        // Keep the cursor where it is in the text.
        onMouseDown={(e) => e.preventDefault()}
        onClick={reading ? pauseReading : resumeReading}
      >
        {reading ? '일시정지' : '계속'}
      </button>
      <button
        type="button"
        className="btn small"
        title="멈추고 읽던 문장으로 커서 옮기기 (Esc)"
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => stopReading()}
      >
        멈춤
      </button>
      <label className="reading-rate" title="빠르기 (다음 문장부터)">
        <span>빠르기</span>
        <select value={rate} onChange={(e) => setView({ readRate: Number(e.target.value) })}>
          {READ_RATES.map((r) => (
            <option key={r} value={r}>
              {rateLabel(r)}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
