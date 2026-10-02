import { describe, expect, it } from 'vitest';
import { splitSentences } from './sentences';

const split = (text: string) => splitSentences(text).map((s) => text.slice(s.start, s.end));

describe('splitSentences', () => {
  it('splits on full stops, question and exclamation marks', () => {
    expect(split('비가 왔다. 그는 우산을 폈다! 어디로 가지?')).toEqual(['비가 왔다.', '그는 우산을 폈다!', '어디로 가지?']);
  });

  it('keeps a run of marks together', () => {
    expect(split('정말?! 말도 안 돼...  그래도 간다.')).toEqual(['정말?!', '말도 안 돼...', '그래도 간다.']);
  });

  it('splits after ellipses', () => {
    expect(split('그건… 아니, 됐어…… 가자.')).toEqual(['그건…', '아니, 됐어……', '가자.']);
  });

  it('keeps closing quotes with the dialogue', () => {
    expect(split('“어디 가?” 서하가 물었다. “집에.”')).toEqual(['“어디 가?”', '서하가 물었다.', '“집에.”']);
    expect(split('"안녕." 그가 웃었다.')).toEqual(['"안녕."', '그가 웃었다.']);
    expect(split("‘정말일까?’ 그녀는 생각했다.")).toEqual(['‘정말일까?’', '그녀는 생각했다.']);
  });

  it('reads a quote and its 하고 / 라며 as one sentence', () => {
    expect(split('“가자!” 하고 외쳤다. 다들 일어섰다.')).toEqual(['“가자!” 하고 외쳤다.', '다들 일어섰다.']);
    expect(split('“설마요?”라며 웃었다.')).toEqual(['“설마요?”라며 웃었다.']);
  });

  it('does not split numbers or marks inside a word', () => {
    expect(split('키가 3.5센티미터 컸다. 1.2배 빨랐다.')).toEqual(['키가 3.5센티미터 컸다.', '1.2배 빨랐다.']);
    expect(split('www.example.com에 들어갔다.')).toEqual(['www.example.com에 들어갔다.']);
  });

  it('ends a sentence at a line break', () => {
    expect(split('첫 줄\n둘째 줄. 셋째\n')).toEqual(['첫 줄', '둘째 줄.', '셋째']);
  });

  it('keeps the last sentence without a mark', () => {
    expect(split('끝나지 않은 문장')).toEqual(['끝나지 않은 문장']);
  });

  it('gives offsets into the text, without surrounding spaces', () => {
    expect(splitSentences('  가. 나.  ')).toEqual([
      { start: 2, end: 4 },
      { start: 5, end: 7 },
    ]);
  });

  it('finds nothing in an empty or blank text', () => {
    expect(splitSentences('')).toEqual([]);
    expect(splitSentences('  \n ')).toEqual([]);
  });
});
