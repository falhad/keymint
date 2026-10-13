import type { ReactNode } from "react";

export interface Hint {
  keys: string;
  label: string;
  onClick?: () => void;
  disabled?: boolean;
}

export function Footer({ hints, status }: { hints: Hint[]; status?: ReactNode }) {
  return (
    <div className="flex items-center gap-3 border-t border-line px-4 py-2 text-xs text-muted">
      <span className="min-w-0 flex-1 truncate">{status}</span>
      {hints.map((h) => (
        <button
          key={h.label}
          type="button"
          disabled={h.disabled}
          onClick={h.onClick}
          className="flex items-center gap-1.5 rounded px-1 py-0.5 enabled:hover:text-fg disabled:opacity-40"
        >
          {h.label} <kbd>{h.keys}</kbd>
        </button>
      ))}
    </div>
  );
}
