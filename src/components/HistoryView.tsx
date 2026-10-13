import { useEffect, useRef, useState } from "react";
import type { HistoryEntry } from "../lib/types";
import { Footer } from "./Footer";

interface Props {
  entries: HistoryEntry[];
  notice: string | null;
  onCopy: (text: string) => void;
  onPaste: (text: string) => void;
  onDelete: (id: number) => void;
  onBack: () => void;
}

function ago(timestamp: number): string {
  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - timestamp);
  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)} min ago`;
  if (seconds < 86_400) return `${Math.floor(seconds / 3600)} h ago`;
  return `${Math.floor(seconds / 86_400)} d ago`;
}

function firstLine(text: string): string {
  return text.trim().split("\n", 1)[0] ?? "";
}

export function HistoryView({ entries, notice, onCopy, onPaste, onDelete, onBack }: Props) {
  const [selected, setSelected] = useState(0);
  const current = entries[Math.min(selected, entries.length - 1)];
  const row = useRef<HTMLLIElement>(null);
  useEffect(() => {
    row.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (e.key === "Escape") onBack();
      else if (e.key === "ArrowDown") setSelected((s) => Math.min(s + 1, entries.length - 1));
      else if (e.key === "ArrowUp") setSelected((s) => Math.max(s - 1, 0));
      else if (e.key === "Enter" && current) (mod ? onPaste : onCopy)(current.output);
      else if (e.key === "Delete" && current) {
        onDelete(current.id);
        setSelected((s) => Math.max(0, Math.min(s, entries.length - 2)));
      } else return;
      e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [entries.length, current, onBack, onCopy, onPaste, onDelete]);

  return (
    <>
      <div className="border-b border-line px-4 py-3 font-medium">History</div>
      {entries.length === 0 ? (
        <div className="grid flex-1 place-items-center px-6 text-center text-muted">
          Nothing here yet. Results of commands you run are kept here so you can reuse them.
        </div>
      ) : (
        <div className="flex min-h-0 flex-1">
          <ul role="listbox" className="w-64 shrink-0 overflow-y-auto border-r border-line p-2">
            {entries.map((entry, i) => (
              <li
                key={entry.id}
                ref={i === selected ? row : undefined}
                role="option"
                aria-selected={i === selected}
                onMouseMove={() => i !== selected && setSelected(i)}
                onClick={() => onCopy(entry.output)}
                className={`cursor-pointer rounded-lg px-3 py-2 ${i === selected ? "bg-surface-2" : ""}`}
              >
                <div className="flex items-baseline gap-2">
                  <span className="min-w-0 flex-1 truncate text-sm">{entry.command_title}</span>
                  <span className="shrink-0 text-[11px] text-muted">{ago(entry.timestamp)}</span>
                </div>
                <div className="truncate text-xs text-muted">{firstLine(entry.output)}</div>
              </li>
            ))}
          </ul>
          <div className="min-w-0 flex-1 space-y-3 overflow-y-auto p-3">
            {current && (
              <>
                <pre className="whitespace-pre-wrap break-words font-sans">{current.output}</pre>
                {current.input && (
                  <details className="text-xs text-muted">
                    <summary className="cursor-pointer">Original text</summary>
                    <pre className="mt-1 whitespace-pre-wrap break-words font-sans">{current.input}</pre>
                  </details>
                )}
              </>
            )}
          </div>
        </div>
      )}
      <Footer
        status={notice}
        hints={[
          { keys: "↵", label: "Copy", disabled: !current, onClick: () => current && onCopy(current.output) },
          { keys: "Ctrl ↵", label: "Paste", disabled: !current, onClick: () => current && onPaste(current.output) },
          { keys: "Del", label: "Remove", disabled: !current, onClick: () => current && onDelete(current.id) },
          { keys: "Esc", label: "Back", onClick: onBack },
        ]}
      />
    </>
  );
}
