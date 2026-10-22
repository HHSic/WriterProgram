import { describe, expect, it } from 'vitest';
import { DecorationSet } from '@tiptap/pm/view';
import { RhythmHighlight, countRhythm, setRhythmMarks } from './rhythm';
import { chapter, fakeEditor } from './testing';

const plugins = RhythmHighlight.config.addProseMirrorPlugins!.call({} as never);
const long = '지갑을 꺼내 엄지로 열었다가 닫고 휴대폰을 만졌다가 놓고 마지막으로 손목에 찬 시계를 풀어 계산대에 올렸다.';

describe('길이 보기', () => {
  it('counts narration sentences, the long ones and the runs of short ones', () => {
    const doc = chapter(long, '가로등이 깜박였다. 개가 그쳤다. 하진은 숨을 멈췄다. 문이 열렸다.', '“어서 와.”', '비가 왔다. 바람이 불었다.');
    const c = countRhythm(doc);
    expect(c.sentences).toBe(7);
    expect(c.long.map((s) => [s.block, s.length])).toEqual([[0, [...long].length]]);
    // The line of dialogue breaks the flow: the last two short ones are not a run.
    expect(c.shortRuns.map((r) => r.length)).toEqual([4]);
    const all = [long, '가로등이 깜박였다.', '개가 그쳤다.', '하진은 숨을 멈췄다.', '문이 열렸다.', '비가 왔다.', '바람이 불었다.'];
    expect(c.mean).toBe(Math.round(all.reduce((n, x) => n + [...x].length, 0) / all.length));
  });

  it('leaves dialogue out but keeps single quotes as narration', () => {
    const c = countRhythm(chapter('“가자.” 그가 말했다.', "'수선 세탁'이라는 간판이 붙어 있었다."));
    expect(c.sentences).toBe(2);
  });

  it('marks long sentences and short runs only while it is on', () => {
    const doc = chapter(long, '가로등이 깜박였다. 개가 그쳤다. 하진은 숨을 멈췄다. 문이 열렸다.');
    const editor = fakeEditor(doc, plugins);
    const classes = () =>
      (plugins[0].getState(editor.state) as { set: DecorationSet }).set
        .find()
        .map((d) => (d as unknown as { type: { attrs: { class: string } } }).type.attrs.class);
    expect(classes()).toEqual([]);
    setRhythmMarks(editor, true);
    expect(classes()).toEqual(['rhythm-long', 'rhythm-short', 'rhythm-short', 'rhythm-short', 'rhythm-short']);
    setRhythmMarks(editor, false);
    expect(classes()).toEqual([]);
  });
});
