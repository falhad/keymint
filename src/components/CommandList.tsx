import { useEffect, useRef } from "react";
import type { CommandManifest } from "../lib/types";
import { Badge } from "./Badge";

interface Props {
  items: CommandManifest[];
  selected: number;
  onHover: (i: number) => void;
  onPick: (cmd: CommandManifest) => void;
}

export function CommandList({ items, selected, onHover, onPick }: Props) {
  const ref = useRef<HTMLLIElement>(null);
  useEffect(() => {
    ref.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  if (items.length === 0) {
    return <div className="grid flex-1 place-items-center text-muted">No matching commands</div>;
  }
  return (
    <ul role="listbox" className="flex-1 overflow-y-auto p-2">
      {items.map((cmd, i) => (
        <li
          key={cmd.id}
          ref={i === selected ? ref : undefined}
          role="option"
          aria-selected={i === selected}
          onMouseMove={() => i !== selected && onHover(i)}
          onClick={() => onPick(cmd)}
          className={`flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2 ${
            i === selected ? "bg-surface-2" : ""
          }`}
        >
          <Badge title={cmd.title} category={cmd.category} />
          <span className="min-w-0 flex-1">
            <span className="block truncate">{cmd.title}</span>
            {cmd.subtitle && <span className="block truncate text-xs text-muted">{cmd.subtitle}</span>}
          </span>
          {cmd.slash && <span className="text-xs text-muted">/{cmd.slash}</span>}
          {cmd.shortcut && <kbd>{cmd.shortcut}</kbd>}
          <span className="text-xs text-muted">{cmd.category}</span>
        </li>
      ))}
    </ul>
  );
}
