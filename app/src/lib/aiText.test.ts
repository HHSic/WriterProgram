import { describe, expect, it } from 'vitest';
import { addUsage, sizeText, usageText } from './aiText';

describe('AI sentences', () => {
  it('says how much will be sent', () => {
    expect(sizeText(5123)).toBe('보낼 글은 모두 약 5,100자입니다.');
    expect(sizeText(840, 3)).toBe('회차 3개를 하나씩 보냅니다. 보낼 글은 모두 약 840자입니다.');
  });

  it('says what was used, without prices', () => {
    expect(usageText({ inputTokens: 3200, outputTokens: 180 }, 5123)).toBe(
      '보낸 글 약 5,100자, AI 회사 기준으로 읽은 양 3,200토큰, 답한 양 180토큰을 썼습니다. 요금은 AI 회사 계정에서 나갑니다.',
    );
    expect(usageText(null, 900)).toBe('보낸 글 약 900자. 쓴 양은 AI 회사 사용 내역에서 볼 수 있습니다.');
    expect(usageText({ inputTokens: 10, outputTokens: 2 }, 20)).not.toMatch(/원|\$/);
  });

  it('adds up several requests', () => {
    expect(addUsage([{ inputTokens: 1, outputTokens: 2 }, null, { inputTokens: 3, outputTokens: null }])).toEqual({
      inputTokens: 4,
      outputTokens: 2,
    });
    expect(addUsage([null, null])).toBeNull();
  });
});
