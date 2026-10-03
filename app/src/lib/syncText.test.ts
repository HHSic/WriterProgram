import { describe, expect, it } from 'vitest';
import { filesText, keepText, removalQuestion } from './syncText';

const chapters = (n: number) => Array.from({ length: n }, (_, i) => `manuscript/c${i}.md`);

describe('sentences about many removals at once', () => {
  it('counts files by kind in the writer’s words', () => {
    expect(filesText(chapters(3))).toBe('회차 파일 3개');
    expect(filesText(['.snapshots/a/1.auto.md', ...chapters(2), 'project.json', 'x.txt'])).toBe(
      '회차 파일 2개, 기록 1개, 작품 구조 파일 1개, 다른 파일 1개',
    );
  });

  it('asks about removing on the drive', () => {
    expect(removalQuestion(chapters(32), [])).toBe(
      '이 기기에서 회차 파일 32개가 없어져 드라이브에서도 지우려 합니다. 맞나요?',
    );
  });

  it('asks about removing on this device, and both ways at once', () => {
    expect(removalQuestion([], chapters(20))).toBe(
      '드라이브에서 회차 파일 20개가 없어져 이 기기에서도 지우려 합니다. 맞나요?',
    );
    expect(removalQuestion(chapters(20), ['notes/n.md'])).toBe(
      '이 기기에서 회차 파일 20개가 없어져 드라이브에서도 지우려 합니다. 드라이브에서 메모 1개가 없어져 이 기기에서도 지우려 합니다. 맞나요?',
    );
    expect(removalQuestion([], [])).toBe('');
  });

  it('says where the files come back from', () => {
    expect(keepText(chapters(1), [])).toContain('이 기기로 다시 가져옵니다');
    expect(keepText([], chapters(1))).toContain('드라이브에 다시 올립니다');
  });
});
