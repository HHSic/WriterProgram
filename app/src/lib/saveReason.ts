// Why a save failed, in the writer's words. The Rust side answers a short
// reason (writer_core::Error::user_message); the common ones become a
// sentence that says what is wrong and what the writer can do about it.

const REASONS: [RegExp, string][] = [
  [/디스크 공간 부족|공간이 부족|No space left|disk full/i, '디스크가 가득 찼습니다. 파일을 정리하면 다시 저장됩니다'],
  [/다른 프로그램에 잡혀|being used by another process|sharing violation/i, '다른 프로그램(동기화·백신 등)이 파일을 쓰고 있습니다'],
  [/쓸 권한이 없음|읽기 전용|Access is denied|Permission denied|Read-only/i, '작품 폴더에 쓸 수 없습니다(읽기 전용이거나 다른 프로그램이 잡고 있음)'],
  [/파일을 찾을 수 없음|cannot find the path|No such file/i, '작품 폴더를 찾을 수 없습니다(드라이브가 빠졌거나 폴더가 옮겨짐)'],
];

export function saveReason(text: string): string {
  for (const [pattern, words] of REASONS) if (pattern.test(text)) return words;
  return text;
}

/** "30초 뒤", "1분 뒤" for the next try. */
export function retryIn(ms: number): string {
  const s = Math.max(1, Math.ceil(ms / 1000));
  if (s < 60) return `${s}초 뒤`;
  return `${Math.round(s / 60)}분 뒤`;
}
