// Local candidate suggestions for the browser preview; native uses core::discover.
import type { Backend, DiscoveredCandidate, DiscoveredKind } from '../types';
import { findNames } from '../../editor/names';
import { clone, project, doc, now, id } from './state';
const COMMON = ["그녀", "그들", "그것", "이것", "저것", "여자", "남자", "사람", "아이", "모두", "누군가", "아무도", "목소리", "바람", "한숨", "고개", "생각", "마음", "시간", "사실", "감정", "말", "대답", "일", "이름", "장면", "가게", "서점", "학교", "도시", "오늘", "내일", "어제", "아침", "다음", "지금", "갑자기", "우리", "당신", "자신", "상대", "주인", "손님", "할머니", "아버지", "어머니", "선생", "문제", "이유", "기억", "꿈", "눈", "손", "소리", "모습", "침묵", "얼굴", "그곳", "이곳", "저곳", "여기", "거기", "어디", "거리", "앞", "뒤", "안", "밖", "그때", "그날", "계속", "다시", "함께", "조용히"];
const PLACES = ["서점", "도서관", "학교", "왕국", "제국", "대륙", "마을", "항구", "도시", "골목", "카페", "병원", "공원", "서고", "저택", "광장", "동굴", "시장", "지하실", "거리", "숲", "산", "강", "바다", "궁", "성", "역", "탑", "섬"];
const SPEECH = ["말했", "말하", "말을", "물었", "물어", "대답", "외쳤", "외치", "속삭", "웃었", "웃으며", "고개를", "생각했", "돌아보", "바라보", "한숨", "인사", "걸어", "손을", "눈을"];
const TRAVEL = ["갔", "가서", "향했", "향하", "들어", "도착", "살았", "살고", "머물", "떠났", "떠나", "돌아", "지났", "지나", "서 있었", "서있", "만났", "만나", "걸었", "걸어", "찾아"];
export function suggestNames(text: string): Map<string, DiscoveredKind> {
 const out = new Map<string, DiscoveredKind>();
 const valid = (name: string) => [...name].length >= 2 && [...name].length <= 30 && !COMMON.includes(name);
 const strip = (word: string, suffixes: string[]) => suffixes.map(s => word.endsWith(s) ? word.slice(0, -s.length) : '').find(s => [...s].length >= 2);
 const words = [...text.matchAll(/[가-힣A-Za-z][가-힣A-Za-z·\-]{1,39}/gu)];
 words.forEach((word, i) => {
  const raw = word[0], end = word.index + raw.length;
  const tail = [...text.slice(end).split(/[.!?\n]/u)[0]].slice(0, 50).join('');
  const prior = words[i - 1];
  const previous = prior && /^\s*$/u.test(text.slice(prior.index + prior[0].length, word.index)) ? prior[0] : '';
  const person = strip(raw, ['에게는', '한테는', '에게', '한테', '은', '는', '이', '가', '와', '과']);
  if (person && valid(person) && [...person].length <= 12 && !PLACES.some(s => person.endsWith(s)) && SPEECH.some(s => tail.includes(s))) out.set(person, 'person');
  const honor = strip(raw, ['씨는', '씨가', '씨', '님은', '님이', '님', '군은', '군이', '군']) || (['씨', '씨는', '씨가', '님', '님은', '님이', '선생님', '박사', '공작'].includes(raw) ? previous : '');
  if (honor && valid(honor)) out.set(honor, 'person');
  const place = strip(raw, ['에서는', '에서도', '에서', '으로는', '으로', '에선', '에는', '에도', '에', '로는', '로']);
  if (place) {
   const noun = PLACES.find(s => place.endsWith(s));
   const name = noun === place && previous && valid(previous) && !['은', '는', '을', '를', '가', '이', '한', '던', '된', '운'].some(s => previous.endsWith(s)) ? previous + ' ' + place : place;
   if (valid(name) && (noun || TRAVEL.some(s => tail.includes(s)))) out.set(name, 'place');
  }
 });
 return out;
}
export const discoverMethods = {
 async discoverScan(root, docIds) {
  const p = project(root), candidates = new Map<string, DiscoveredCandidate>();
  const docs = p.parts.flatMap(x => x.docs).filter(x => !docIds || docIds.includes(x)).map(x => [x, doc(p, x)] as const);
  let skipped = 0, truncated = false;
  const texts = docs.flatMap(([docId, d]) => {
   if (d.broken) { skipped++; return []; }
   return (d.body.content ?? []).flatMap((n, block) => n.type === 'paragraph' ? [{ docId, block, text: (n.content ?? []).map(i => i.type === 'hardBreak' ? '\n' : i.text ?? '').join('') }] : []);
  });
  for (const { text } of texts) for (const [name, kind] of suggestNames(text)) {
   if (candidates.size >= 500) { truncated = true; break; }
   const card = [...p.cards.values()].find(c => c.name === name || c.aliases.includes(name));
   if (!candidates.has(name)) candidates.set(name, { name, kind, count: 0, existing: card?.id ?? null, samples: [] });
  }
  const names = [...candidates.keys()].sort((a,b) => b.length - a.length);
  const re = names.length ? new RegExp(names.map(n => n.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|'), 'gu') : null;
  if (re) for (const { text, block, docId } of texts) for (const m of findNames(re, text)) {
   const c = candidates.get(m.name)!; c.count++;
   if (c.samples.length < 3) c.samples.push({ docId, hit: { block, start: m.index, end: m.index + m.name.length, before: text.slice(Math.max(0, m.index - 24), m.index), text: m.name, after: text.slice(m.index + m.name.length, m.index + m.name.length + 24) } });
  }
  return { candidates: [...candidates.values()].filter(c => c.count).sort((a,b) => b.count - a.count || a.name.localeCompare(b.name)), skipped, truncated };
 },
 async discoverAdd(root, picks) {
  const p = project(root);
  if (picks.length > 500 || picks.some(x => !x.name.trim() || !p.cardTypes.some(t => t.id === x.kind))) throw '이름과 분류를 확인해 주세요';
  const known = new Set([...p.cards.values()].flatMap(c => [c.name, ...c.aliases])), made: string[] = [];
  for (const pick of picks) {
   const name = pick.name.trim(); if (known.has(name)) continue; known.add(name);
   const card = { id: id(), cardType: pick.kind, name, aliases: [], fields: p.cardTypes.find(t => t.id === pick.kind)!.fields.map(f => [f, ''] as [string,string]), highlight: true, created: now(), description: '' };
   p.cards.set(card.id, clone(card)); made.push(card.id);
  }
  return made;
 },
} satisfies Pick<Backend, 'discoverScan' | 'discoverAdd'>;
