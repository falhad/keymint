import { useState } from "react";

const MODIFIER_CODES = /^(Control|Shift|Alt|Meta|OS)(Left|Right)?$/;

/** Turn a keydown into a global-shortcut accelerator such as `Ctrl+Alt+G`. */
export function toAccelerator(e: React.KeyboardEvent): string | null {
  if (MODIFIER_CODES.test(e.code)) return null;
  const key = e.code.startsWith("Key")
    ? e.code.slice(3)
    : e.code.startsWith("Digit")
      ? e.code.slice(5)
      : e.code;
  const isFunctionKey = /^F\d{1,2}$/.test(key);
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
  // A bare letter as a global hotkey would hijack typing everywhere.
  if (mods.length === 0 && !isFunctionKey) return null;
  return [...mods, key].join("+");
}

interface Props {
  value: string;
  onChange: (accelerator: string) => void;
  disabled?: boolean;
}

export function ShortcutRecorder({ value, onChange, disabled }: Props) {
  const [recording, setRecording] = useState(false);
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={() => setRecording(true)}
      onBlur={() => setRecording(false)}
      onKeyDown={(e) => {
        if (!recording) return;
        e.preventDefault();
        if (e.key === "Escape") return setRecording(false);
        if (e.key === "Backspace" || e.key === "Delete") {
          onChange("");
          return setRecording(false);
        }
        const accel = toAccelerator(e);
        if (accel) {
          onChange(accel);
          setRecording(false);
        }
      }}
      className={`min-w-36 rounded-md border px-2.5 py-1.5 text-left disabled:opacity-50 ${
        recording ? "border-accent text-accent" : "border-line"
      }`}
    >
      {recording ? "Press keys… (Esc cancel, ⌫ clear)" : value || <span className="text-muted">Not set</span>}
    </button>
  );
}
