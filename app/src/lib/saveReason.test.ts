import { describe, expect, it } from 'vitest';
import { retryIn, saveReason } from './saveReason';

describe('saveReason', () => {
  it('says what is wrong in the writer\'s words', () => {
    expect(saveReason('디스크 공간 부족')).toContain('디스크가 가득 찼습니다');
    expect(saveReason('이 위치에 쓸 권한이 없음')).toContain('작품 폴더에 쓸 수 없습니다');
    expect(saveReason('읽기 전용 위치')).toContain('작품 폴더에 쓸 수 없습니다');
    expect(saveReason('파일이 다른 프로그램에 잡혀 있음')).toContain('다른 프로그램');
    expect(saveReason('파일을 찾을 수 없음')).toContain('작품 폴더를 찾을 수 없습니다');
  });

  it('keeps a reason it does not know', () => {
    expect(saveReason('회차를 찾을 수 없음 · abc')).toBe('회차를 찾을 수 없음 · abc');
  });
});

describe('retryIn', () => {
  it('counts seconds, then minutes', () => {
    expect(retryIn(30000)).toBe('30초 뒤');
    expect(retryIn(1200)).toBe('2초 뒤');
    expect(retryIn(0)).toBe('1초 뒤');
    expect(retryIn(60000)).toBe('1분 뒤');
  });
});
