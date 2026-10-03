// Sentences about the creation journal (창작 일지) for 작품 설정.

import type { AnchorResult, JournalReport, JournalSummary } from '../api/types';
import { num } from './format';

/** "2026년 10월 2일" in local time. */
function dateLabel(iso: string): string {
  const d = new Date(iso);
  return `${d.getFullYear()}년 ${d.getMonth() + 1}월 ${d.getDate()}일`;
}

/** "2026년 10월 2일부터 1,234번 저장했습니다. 다른 기기 1대의 기록도 함께 있습니다." */
export function journalSummaryText(s: JournalSummary): string {
  if (!s.since) return '아직 기록이 없습니다. 이 작품을 쓰고 저장하면 쌓이기 시작합니다.';
  let text = `${dateLabel(s.since)}부터 ${num(s.saves)}번 저장했습니다.`;
  const others = s.devices - (s.thisDevice > 0 ? 1 : 0);
  if (others > 0) text += ` 다른 기기 ${num(others)}대의 기록도 함께 있습니다.`;
  return text;
}

/** "마지막 날짜 증명: 2026년 10월 2일" */
export function anchorText(s: JournalSummary): string {
  return s.lastAnchor ? `마지막 날짜 증명: ${dateLabel(s.lastAnchor)}` : '아직 날짜 증명을 받지 않았습니다.';
}

/** What asking for a time stamp now came to. */
export function anchorResultText(r: AnchorResult): string {
  switch (r.state) {
    case 'signed':
      return `${r.signed.join(', ')}에서 날짜 증명을 받았습니다.`;
    case 'unchanged':
      return '지난 날짜 증명 뒤로 바뀐 것이 없어 새로 받지 않았습니다.';
    case 'notYet':
      return '오늘 받은 날짜 증명 뒤로 쓴 양이 적어 아직 받지 않았습니다.';
    case 'enough':
      return '오늘은 날짜 증명을 충분히 받았습니다. 내일 다시 받습니다.';
    case 'ask':
      return '날짜 증명을 받을 때가 되었습니다.';
    case 'noJournal':
      return '아직 기록이 없어 증명할 것이 없습니다.';
    case 'journalOff':
      return '창작 일지가 꺼져 있어 날짜 증명을 받지 않습니다.';
    case 'notAllowed':
      return '날짜 증명 받기가 꺼져 있습니다.';
    case 'offline':
      return '시각 인증 기관에 연결하지 못했습니다. 나중에 다시 시도합니다.';
  }
}

/** What a check found, in one or more sentences. */
export function journalCheckText(r: JournalReport, device: string): string {
  if (!r.files.length) return '확인할 기록이 아직 없습니다.';
  if (r.ok) return '기록이 처음부터 끝까지 이어져 있습니다. 고쳐지거나 빠진 곳이 없습니다.';
  return r.files
    .filter((f) => f.firstBad)
    .map((f) => {
      const [at, problem] = f.firstBad!;
      const whose = f.device === device ? '이 기기' : '다른 기기';
      return problem === 'broken'
        ? `${whose} 기록의 ${num(at)}번째 항목이 앞 항목과 이어지지 않습니다. 그 앞이 고쳐졌거나 빠졌습니다.`
        : `${whose} 기록의 ${num(at)}번째 항목을 읽을 수 없습니다. 고쳐졌거나 쓰다 끊겼습니다.`;
    })
    .join(' ');
}
