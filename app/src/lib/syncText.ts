// Sentences about keeping a project in step with a drive (store/drives.ts).

/** What a file in the project folder is, by where it sits, in the writer's words. */
const KINDS: [prefix: string, label: string][] = [
  ['manuscript/', '회차 파일'],
  ['planning/', '기획 문서'],
  ['cards/', '설정 카드'],
  ['notes/', '메모'],
  ['.snapshots/', '기록'],
  ['.trash/', '휴지통 파일'],
  ['.journal/', '창작 일지 파일'],
  ['.exchanges/', '교정본 주고받기 파일'],
  ['project.json', '작품 구조 파일'],
];

/** "회차 파일 30개, 기록 5개": how many files of each kind, the kinds in a fixed order. */
export function filesText(paths: string[]): string {
  const counts = new Map<string, number>();
  for (const path of paths) {
    const label = KINDS.find(([prefix]) => path.startsWith(prefix))?.[1] ?? '다른 파일';
    counts.set(label, (counts.get(label) ?? 0) + 1);
  }
  const order = [...KINDS.map(([, label]) => label), '다른 파일'];
  return order
    .filter((label) => counts.has(label))
    .map((label) => `${label} ${counts.get(label)}개`)
    .join(', ');
}

/**
 * The question for removals a pass held back because too many went at once:
 * "이 기기에서 회차 파일 32개가 없어져 드라이브에서도 지우려 합니다. 맞나요?"
 */
export function removalQuestion(there: string[], here: string[]): string {
  const parts: string[] = [];
  if (there.length) parts.push(`이 기기에서 ${filesText(there)}가 없어져 드라이브에서도 지우려 합니다.`);
  if (here.length) parts.push(`드라이브에서 ${filesText(here)}가 없어져 이 기기에서도 지우려 합니다.`);
  return parts.length ? `${parts.join(' ')} 맞나요?` : '';
}

/** What 되살리기 does for removals held back, by direction. */
export function keepText(there: string[], here: string[]): string {
  if (there.length && here.length) return '되살리기를 고르면 남아 있는 쪽의 파일로 양쪽을 다시 채웁니다.';
  if (here.length) return '되살리기를 고르면 이 기기에 남은 파일을 드라이브에 다시 올립니다.';
  return '되살리기를 고르면 드라이브에 남은 파일을 이 기기로 다시 가져옵니다.';
}
