import { useEffect, useState } from 'react';

/** A finite size with no maximum; keep partial typing intact. */
export function FontSizeInput({ value, onChange, label, min = 1 }: { value: number; onChange: (size: number) => void; label: string; min?: number }) {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  return <input className="font-size-input" type="number" min={min} step="any" inputMode="decimal"
    aria-label={label} value={draft}
    onChange={(e) => {
      setDraft(e.target.value);
      const next = e.target.valueAsNumber;
      if (Number.isFinite(next) && next >= min) onChange(next);
    }}
    onBlur={() => {
      const next = Number(draft);
      if (draft.trim() && Number.isFinite(next)) { const size = Math.max(min, next); setDraft(String(size)); if (size !== value) onChange(size); }
      else setDraft(String(value));
    }}
    onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); e.currentTarget.blur(); } }} />;
}
