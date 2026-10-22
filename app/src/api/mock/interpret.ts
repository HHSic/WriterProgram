// 작품 해석 in the stand-in: the sample chapter 「비에 젖은 손님」 comes with a
// reading, one of whose links reaches back into 「문 닫는 시간」, so 해석 모드
// can be tried in the browser preview. The writer's changes are kept in memory.

import type { JSONContent } from '@tiptap/core';
import type { Backend, Interpretation, InterpretIncoming, InterpretPlace, InterpretRemote, InterpretSpot, InterpretView } from '../types';
import { clone, doc, now, project } from './state';

/** Each top-level block's text as the editor counts it (line breaks as "\n"). */
function blockTexts(body: JSONContent): string[] {
  return (body.content ?? []).map((b) =>
    b.type === 'paragraph' ? (b.content ?? []).map((i) => (i.type === 'text' ? (i.text ?? '') : '\n')).join('') : '',
  );
}

/** Like crates/core/src/reading.rs `find`: the given paragraph first, then anywhere. */
function place(texts: string[], id: string, block: number, quote: string): InterpretPlace | null {
  const at = (b: number) => {
    const i = texts[b].indexOf(quote);
    return i < 0 ? null : { id, block: b, start: i, end: i + quote.length };
  };
  if (block < texts.length) {
    const found = at(block);
    if (found) return found;
  }
  for (let b = 0; b < texts.length; b += 1) {
    const found = at(b);
    if (found) return found;
  }
  return null;
}

function sample(docId: string, first: string | undefined): Interpretation {
  return {
    doc: docId,
    rev: '',
    made: now(),
    by: 'Claude',
    rhythm: '짧은 문장으로 닫힌 시간(삼 년)을 세우고, 손님이 들어오는 대목에서 처음 길게 잇는다. 봉투 문장은 끊어 세울 수도 있다.',
    summary: '닫으려던 문을 다시 여는 이야기다. 서하가 지켜 온 규칙(열한 시 폐점)이 처음 깨지는 밤이고, 그 틈으로 할머니의 시간이 들어온다.',
    scenes: [
      { block: 0, title: '닫히지 않은 문', purpose: '삼 년 동안 지켜 온 폐점 시간을 처음 어기게 만든다. 이 장면이 없으면 손님은 그냥 손님이다.' },
      { block: 5, title: '대여 카드', purpose: '손님이 가져온 것이 책이 아니라 기록이라는 걸 보여 주며 질문을 과거로 돌린다.' },
    ],
    sentences: [
      {
        id: 's1', block: 0, quote: '이 시간에 문을 여는 사람은 없었다.', roles: ['긴장', '정보'],
        meaning: '서점이 오래 바깥과 끊겨 있었다는 것.', intent: '종소리를 사건으로 만든다. 평범한 손님이 아니라는 예고.',
        links: [{ to: 's4', kind: '대비', note: '문을 여는 사람이 없던 곳에 들어온 사람' }],
      },
      {
        id: 's2', block: 0, quote: '적어도 지난 삼 년 동안은 그랬다.', roles: ['복선', '시점'],
        meaning: '삼 년 전에 무언가가 있었다. 서하는 그 해를 세고 있다.', intent: '말하지 않은 과거를 숫자 하나로 남긴다.',
        rhythm: { kind: '깨달음', note: '앞 문장을 짧게 한 번 더 받아 삼 년이라는 숫자만 남긴다.' },
        links: [{ to: 's5', kind: '회수', note: '바랜 대여 카드가 그 이전의 시간을 꺼낸다' }],
      },
      {
        id: 's3', block: 1, quote: '품에 안은 종이봉투만은 이상하리만치 말라 있었다', roles: ['이미지', '긴장'],
        meaning: '남자에게는 젖은 자기 몸보다 봉투가 중요하다.', intent: '봉투를 장면의 중심에 놓는다. 독자가 봉투를 보게 한다.',
        rhythm: {
          kind: '숨을 모으는 대목',
          note: '젖은 남자에서 마른 봉투로 시선을 한 문장 안에서 옮기며 봉투를 끝에 세운다.',
          advice: "'젖어 있었고,'에서 끊어 '품에 안은 종이봉투만은 이상하리만치 말라 있었다.'를 따로 세우면 봉투가 더 또렷해진다. 지금도 맞으니 작가가 고른다.",
        },
        links: [{ to: 's5', kind: '회수', note: '봉투 안에서 나온 것' }],
      },
      {
        id: 's4', block: 3, quote: '그런데 들어오세요.', roles: ['인물', '전환'],
        meaning: '규칙보다 앞서는 무언가가 서하에게 있다.', intent: '서하가 처음으로 규칙을 깬다. 이야기가 움직이기 시작하는 자리.',
        links: [
          { to: 's1', kind: '대비', note: '' },
          ...(first
            ? [{ to: '', kind: '대비', note: '1화에서 매일 지키던 폐점 시간을 처음 어긴다', doc: first, quote: '서하는 매일 밤 열한 시에 서점 문을 닫았다.' }]
            : []),
        ],
      },
      {
        id: 's5', block: 5, quote: '누렇게 바랜 칸마다 같은 이름이 적혀 있었다.', roles: ['회수', '복선'],
        meaning: '한 사람이 오래 같은 일을 되풀이했다.', intent: '이름이 누구인지 묻게 만들고 장면을 닫는다.',
        links: [{ to: 's2', kind: '회수', note: '' }, { to: 's3', kind: '회수', note: '' }],
      },
      {
        id: 's6', block: 2, quote: '영업, 끝났나요?', roles: ['관계'],
        meaning: '손님은 답을 알면서 묻는다.', intent: '서하에게 거절할 기회를 준다.',
        links: [], unclear: '바로 다음 대사와 하는 일이 겹친다. 남자가 들어올 이유를 이 대사가 더 보태는지 작가가 볼 만하다.',
      },
    ],
  };
}

/** Each project's readings by chapter, the sample one put in when first asked. */
const readings = new Map<string, Map<string, Interpretation>>();

function readingsOf(root: string, p: ReturnType<typeof project>): Map<string, Interpretation> {
  let all = readings.get(root);
  if (!all) {
    all = new Map();
    readings.set(root, all);
    const docs = [...p.docs.values()];
    const first = docs.find((d) => d.meta.title === '문 닫는 시간')?.meta.id;
    const guest = docs.find((d) => d.meta.title === '비에 젖은 손님');
    if (guest) all.set(guest.meta.id, sample(guest.meta.id, first));
  }
  return all;
}

/** Like crates/core/src/reading.rs `load`. */
function viewOf(root: string, docId: string): InterpretView | null {
  const p = project(root);
  const all = readingsOf(root, p);
  const here = blockTexts(doc(p, docId).body);
  const spot = (other: string, quote: string): [string, InterpretSpot | null] => {
    const d = p.docs.get(other);
    if (!d) return ['', null];
    const found = place(blockTexts(d.body), '', 0, quote);
    return [d.meta.title, found && { doc: other, title: d.meta.title, block: found.block, start: found.start, end: found.end }];
  };
  const incoming: InterpretIncoming[] = [];
  for (const [other, r] of all) {
    if (other === docId) continue;
    for (const s of r.sentences) {
      for (const l of s.links) {
        if (l.doc !== docId || !l.quote) continue;
        const [title, fromSpot] = spot(other, s.quote);
        incoming.push({
          doc: other, title, from: s.id, fromQuote: s.quote, fromSpot,
          kind: l.kind, note: l.note, quote: l.quote, place: place(here, '', 0, l.quote),
        });
      }
    }
  }
  const own = all.get(docId);
  if (!own && !incoming.length) return null;
  const reading = clone(own ?? { doc: docId, rev: '', made: '', by: '', summary: '', scenes: [], sentences: [] });
  const places: InterpretPlace[] = [];
  const gone: string[] = [];
  const remote: InterpretRemote[] = [];
  for (const s of reading.sentences) {
    const found = place(here, s.id, s.block, s.quote);
    if (found) places.push(found);
    else gone.push(s.id);
    s.links.forEach((l, i) => {
      if (!l.doc || l.doc === docId) return;
      const [title, at] = spot(l.doc, l.quote ?? '');
      remote.push({ from: s.id, link: i, doc: l.doc, title, quote: l.quote ?? '', kind: l.kind, note: l.note, spot: at });
    });
  }
  return { reading, read: !!own, places, gone, changed: false, remote, incoming };
}

export const interpretMethods = {
  async interpretLoad(root, docId) {
    return viewOf(root, docId);
  },
  async interpretSave(root, reading) {
    const kept = clone(reading);
    for (const s of kept.sentences) for (const l of s.links) if (l.doc === kept.doc) delete l.doc;
    kept.edited = now();
    readingsOf(root, project(root)).set(kept.doc, kept);
    return viewOf(root, kept.doc);
  },
} satisfies Partial<Backend>;
