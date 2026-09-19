// Nl — editor stepper: <input type=number>. Live-commits while typing when the
// value is within [min,max]; onBlur clamps and reverts empty/NaN; Enter blurs.
import { useEffect, useState } from "react";

export function NumStepper(props: {
  value: number;
  min: number;
  max: number;
  step?: number;
  suffix?: string;
  ariaLabel?: string;
  disabled?: boolean;
  onCommit: (v: number) => void;
}) {
  const { value, min, max, suffix, ariaLabel, disabled, onCommit } = props;
  const step = props.step ?? 1;
  const [text, setText] = useState<string>(String(value));

  useEffect(() => { setText(String(value)); }, [value]);

  const onChange = (raw: string) => {
    setText(raw);
    const d = parseFloat(raw);
    if (!Number.isNaN(d) && d >= min && d <= max) onCommit(d);
  };

  const onBlur = () => {
    const d = parseFloat(text);
    if (text.trim() === "" || Number.isNaN(d)) { setText(String(value)); return; }
    const clamped = Math.min(max, Math.max(min, d));
    onCommit(clamped);
    setText(String(clamped));
  };

  return (
    <div className="num-field">
      <input
        className="count-input num-input"
        type="number"
        min={min}
        max={max}
        step={step}
        value={text}
        aria-label={ariaLabel}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
        onBlur={onBlur}
        onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
      />
      {suffix && <span className="num-suffix">{suffix}</span>}
    </div>
  );
}
