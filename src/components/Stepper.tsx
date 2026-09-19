// ua — spoofer-tab stepper: −/+ buttons around a text input that clamps to
// [min,max] and rounds to step. Optional `format` for display.
import { useEffect, useState } from "react";

export function Stepper(props: {
  value: number;
  min: number;
  max: number;
  step?: number;
  disabled?: boolean;
  format?: (v: number) => string;
  onChange: (v: number) => void;
}) {
  const { value, min, max, disabled, format, onChange } = props;
  const step = props.step ?? 1;
  const [focused, setFocused] = useState(false);
  const [text, setText] = useState<string>(String(value));

  useEffect(() => {
    if (!focused) setText(String(value));
  }, [value, focused]);

  const clamp = (v: number) => {
    if (Number.isNaN(v)) return value;
    const rounded = Math.round(v / step) * step;
    const r = Math.min(max, Math.max(min, rounded));
    return parseFloat(r.toFixed(6));
  };

  const commit = (raw: string) => {
    const v = clamp(parseFloat(raw));
    onChange(v);
    setText(String(v));
  };

  const bump = (dir: number) => onChange(clamp(value + dir * step));

  // Display: while focused show the raw text being typed; otherwise the formatted value.
  const display = focused ? text : format ? format(value) : String(value);

  return (
    <div className="stepper">
      <button type="button" disabled={disabled || value <= min} onClick={() => bump(-1)} aria-label="decrease">−</button>
      <input
        className="count-input num-input"
        type="text"
        inputMode="decimal"
        value={display}
        disabled={disabled}
        onChange={(e) => setText(e.target.value)}
        onFocus={() => { setFocused(true); setText(String(value)); }}
        onBlur={(e) => { setFocused(false); commit(e.target.value); }}
        onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
      />
      <button type="button" disabled={disabled || value >= max} onClick={() => bump(1)} aria-label="increase">+</button>
    </div>
  );
}
