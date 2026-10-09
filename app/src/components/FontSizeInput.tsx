import { useEffect, useState } from 'react';

/** A finite pixel size from 1 upward, with no maximum; keep partial typing intact. */
export function FontSizeInput({ value, onChange, label }: { value: number; onChange: (size: number) => void; label: string }) {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  return <input className="font-size-input" type="number" min={1} step="any" inputMode="decimal"
    aria-label={label} value={draft}
    onChange={(e) => {
      setDraft(e.target.value);
      const next = e.target.valueAsNumber;
      if (Number.isFinite(next) && next >= 1) onChange(next);
    }}
    onBlur={() => {
      const next = Number(draft);
      if (draft.trim() && Number.isFinite(next)) { const size = Math.max(1, next); setDraft(String(size)); if (size !== value) onChange(size); }
      else setDraft(String(value));
    }}
    onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); e.currentTarget.blur(); } }} />;
}
