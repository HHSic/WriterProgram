// 연재 플랫폼 (작품 설정 › 목표, web novels): picking one makes chapter counts
// and goals follow that platform's way of counting and puts its minimum in
// the chapter goal. The rule can be changed by hand (docs/platforms.md).

import type { CountRule, Goal, Platform } from '../../api/types';
import { num } from '../../lib/format';
import { CUSTOM_ID, PLATFORMS, platformName, presetOf, ruleText, sameRule } from '../../lib/platforms';

const RULE_CHECKS: { key: keyof CountRule; label: string }[] = [
  { key: 'spaces', label: '띄어쓰기도 글자로 셈 (공백 포함)' },
  { key: 'skipMarks', label: '마침표, 쉼표, 느낌표, 물음표, 곧은 따옴표(\' ")는 세지 않음' },
  { key: 'wideTwice', label: '이모지는 2자로 셈' },
  { key: 'htmlEscapes', label: '< 와 >는 4자, &는 5자로 셈' },
];

export function PlatformField({
  platform,
  goal,
  onChange,
}: {
  platform: Platform | null;
  goal: Goal;
  onChange: (platform: Platform | null, goal: Goal) => void;
}) {
  const preset = presetOf(platform?.id);

  const pick = (id: string) => {
    if (!id) {
      onChange(null, goal);
      return;
    }
    const next = presetOf(id);
    if (!next) {
      onChange({ id: CUSTOM_ID, rule: platform?.rule ?? { spaces: goal.countSpaces, skipMarks: false, wideTwice: false, htmlEscapes: false } }, goal);
      return;
    }
    onChange({ id, rule: { ...next.rule } }, { ...goal, perDoc: next.minimum ?? goal.perDoc, countSpaces: next.rule.spaces });
  };

  const setRule = (key: keyof CountRule, on: boolean) => {
    if (!platform) return;
    const rule = { ...platform.rule, [key]: on };
    onChange({ ...platform, rule }, { ...goal, countSpaces: rule.spaces });
  };

  const changed = !!preset && !!platform && !sameRule(preset.rule, platform.rule);

  return (
    <div className="field platform-field">
      <span className="field-label">연재 플랫폼</span>
      <select value={platform?.id ?? ''} onChange={(e) => pick(e.target.value)} aria-label="연재 플랫폼">
        <option value="">정하지 않음 (공백 포함·제외로 셈)</option>
        {PLATFORMS.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
        <option value={CUSTOM_ID}>직접 정하기</option>
      </select>
      {platform && (
        <>
          <small className="hint">
            {preset ? `${preset.minimumNote} ` : ''}
            회차 글자 수를 {platformName(platform)}에서 세는 법으로 셉니다. {ruleText(platform.rule)} 줄바꿈과 빈 줄은 세지 않습니다.
          </small>
          {preset && !preset.ruleConfirmed && (
            <small className="hint">
              이 세는 법은 작가들이 확인한 내용을 바탕으로 했습니다. 플랫폼에 올렸을 때 글자 수가 다르면 아래에서 바꿔 주세요.
            </small>
          )}
          <details className="platform-rule" open={!preset || changed}>
            <summary>세는 법 바꾸기{changed ? ' · 바꿈' : ''}</summary>
            {RULE_CHECKS.map((c) => (
              <label key={c.key} className="check">
                <input type="checkbox" checked={platform.rule[c.key]} onChange={(e) => setRule(c.key, e.target.checked)} />
                {c.label}
              </label>
            ))}
            {changed && preset && (
              <button type="button" className="btn ghost" onClick={() => pick(preset.id)}>
                {preset.name} 기본으로 되돌리기
                {preset.minimum ? ` (목표 ${num(preset.minimum)}자)` : ''}
              </button>
            )}
          </details>
        </>
      )}
    </div>
  );
}
