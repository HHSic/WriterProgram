// A number box for 원고 서식 (mm, pt, %) that keeps what is typed until it is a number.

import { useEffect, useState } from 'react';

export function Num({
  value,
  onChange,
  label,
  step = 1,
  allowNegative = false,
  onFocus,
  onBlur,
}: {
  value: number;
  onChange: (v: number) => void;
  label: string;
  step?: number;
  allowNegative?: boolean;
  onFocus?: () => void;
  onBlur?: () => void;
}) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);
  return (
    <input
      className="num"
      type="number"
      step={step}
      min={allowNegative ? undefined : 0}
      value={text}
      aria-label={label}
      onFocus={onFocus}
      onBlur={onBlur}
      onMouseEnter={onFocus}
      onMouseLeave={onBlur}
      onChange={(e) => {
        setText(e.target.value);
        const v = Number.parseFloat(e.target.value);
        if (Number.isFinite(v)) onChange(v);
      }}
    />
  );
}
