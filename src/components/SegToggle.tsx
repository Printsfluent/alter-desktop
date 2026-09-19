// Generic segmented toggle (seg-toggle). Options each carry an id, label, optional title.
export interface SegOption<T extends string> {
  id: T;
  label: string;
  title?: string;
}

export function SegToggle<T extends string>(props: {
  options: SegOption<T>[];
  value: T;
  onChange: (id: T) => void;
  disabled?: boolean;
  className?: string;
}) {
  const { options, value, onChange, disabled, className } = props;
  return (
    <div className={`seg-toggle${className ? " " + className : ""}`} role="group">
      {options.map((o) => (
        <button
          key={o.id}
          type="button"
          className={o.id === value ? "active" : ""}
          title={o.title}
          disabled={disabled}
          onClick={() => onChange(o.id)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
