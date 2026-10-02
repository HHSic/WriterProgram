// 머리말 and 꼬리말 (with the page number) in 원고 서식.

import type { HeadAlign, HeadContent, ManuscriptFormat, RunningFoot, RunningHead } from '../../api/types';

const HEAD_CONTENTS: { id: HeadContent; label: string }[] = [
  { id: 'none', label: '없음' },
  { id: 'title', label: '작품 제목' },
  { id: 'chapter', label: '장 제목' },
  { id: 'titleChapter', label: '책처럼: 왼쪽 쪽 작품 제목, 오른쪽 쪽 장 제목' },
  { id: 'author', label: '필명' },
  { id: 'custom', label: '직접 쓰기' },
];

const HEAD_ALIGNS: { id: HeadAlign; label: string }[] = [
  { id: 'left', label: '왼쪽' },
  { id: 'center', label: '가운데' },
  { id: 'right', label: '오른쪽' },
  { id: 'outside', label: '바깥쪽 (책처럼 펼친 면의 양 끝)' },
];

/** 머리말: what goes at the top of the pages, and where. */
export function HeadField({ format, onChange }: { format: ManuscriptFormat; onChange: (patch: Partial<ManuscriptFormat>) => void }) {
  const head = format.header;
  const setHead = (patch: Partial<RunningHead>) => onChange({ header: { ...head, ...patch } });
  const on = head.content !== 'none';
  return (
    <div className="field">
      <span className="field-label">머리말</span>
      <div className="row wrap">
        <select value={head.content} onChange={(e) => setHead({ content: e.target.value as HeadContent })} aria-label="머리말 내용">
          {HEAD_CONTENTS.map((c) => (
            <option key={c.id} value={c.id}>
              {c.label}
            </option>
          ))}
        </select>
        {head.content === 'custom' && (
          <input
            className="grow"
            value={head.text}
            maxLength={100}
            placeholder="머리말에 넣을 글"
            aria-label="머리말 글"
            onChange={(e) => setHead({ text: e.target.value })}
          />
        )}
        {on && (
          <select value={head.align} onChange={(e) => setHead({ align: e.target.value as HeadAlign })} aria-label="머리말 자리">
            {HEAD_ALIGNS.map((a) => (
              <option key={a.id} value={a.id}>
                {a.label}
              </option>
            ))}
          </select>
        )}
      </div>
      {on && format.chapterNewPage && (
        <label className="check">
          <input type="checkbox" checked={head.skipChapterFirst} onChange={(e) => setHead({ skipChapterFirst: e.target.checked })} />
          장이 시작하는 쪽에는 넣지 않기
        </label>
      )}
      {head.content === 'author' && <small className="hint">필명은 작품 설정의 기본 정보에서 적습니다.</small>}
    </div>
  );
}

/** Lands in the same place on some page ('outside' is left on even pages, right on odd). */
function meets(a: HeadAlign, b: HeadAlign): boolean {
  const sides = (x: HeadAlign) => (x === 'outside' ? ['left', 'right'] : [x, x]);
  const [ae, ao] = sides(a);
  const [be, bo] = sides(b);
  return ae === be || ao === bo;
}

/** A place for the 꼬리말 that keeps clear of the page number. */
function clearOf(pageNumber: HeadAlign): HeadAlign {
  return (['left', 'right', 'center'] as HeadAlign[]).find((a) => !meets(a, pageNumber)) ?? 'center';
}

/** 꼬리말: the page number and the writer's own line at the bottom of the pages. */
export function FootField({ format, onChange }: { format: ManuscriptFormat; onChange: (patch: Partial<ManuscriptFormat>) => void }) {
  const foot = format.footer;
  const footOn = foot.text.trim() !== '';
  const numbers = format.pageNumbers;
  const clash = numbers && footOn && meets(foot.align, format.pageNumberAlign);
  const setFoot = (patch: Partial<RunningFoot>) => onChange({ footer: { ...foot, ...patch } });
  return (
    <div className="field">
      <span className="field-label">꼬리말</span>
      <div className="row wrap">
        <label className="check">
          <input
            type="checkbox"
            checked={numbers}
            onChange={(e) => {
              const on = e.target.checked;
              // Turning numbers on moves the 꼬리말 aside if it sits in their place.
              if (on && footOn && meets(foot.align, format.pageNumberAlign)) {
                onChange({ pageNumbers: on, footer: { ...foot, align: clearOf(format.pageNumberAlign) } });
              } else {
                onChange({ pageNumbers: on });
              }
            }}
          />
          쪽 번호
        </label>
        {numbers && (
          <select value={format.pageNumberAlign} onChange={(e) => onChange({ pageNumberAlign: e.target.value as HeadAlign })} aria-label="쪽 번호 자리">
            {HEAD_ALIGNS.map((a) => (
              <option key={a.id} value={a.id} disabled={footOn && meets(a.id, foot.align)}>
                {a.label}
              </option>
            ))}
          </select>
        )}
      </div>
      <div className="row wrap">
        <input
          className="grow"
          value={foot.text}
          maxLength={100}
          placeholder="꼬리말에 넣을 글 (없으면 비워 두세요)"
          aria-label="꼬리말 글"
          onChange={(e) => {
            const text = e.target.value;
            // The first letters typed take a place clear of the page number.
            const align = !footOn && numbers && meets(foot.align, format.pageNumberAlign) ? clearOf(format.pageNumberAlign) : foot.align;
            setFoot({ text, align });
          }}
        />
        {footOn && (
          <select value={foot.align} onChange={(e) => setFoot({ align: e.target.value as HeadAlign })} aria-label="꼬리말 자리">
            {HEAD_ALIGNS.map((a) => (
              <option key={a.id} value={a.id} disabled={numbers && meets(a.id, format.pageNumberAlign)}>
                {a.label}
              </option>
            ))}
          </select>
        )}
      </div>
      {clash && <small className="hint">꼬리말과 쪽 번호가 같은 자리에 있습니다. 한쪽 자리를 바꿔 주세요.</small>}
    </div>
  );
}
