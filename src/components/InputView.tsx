import { useEffect, useRef, useState } from "react";
import type { CommandManifest } from "../lib/types";
import { Badge } from "./Badge";
import { Footer } from "./Footer";

interface Props {
  cmd: CommandManifest;
  initial: string;
  onSubmit: (text: string) => void;
  onBack: () => void;
}

/** Shown when a command needs text and nothing was selected. */
export function InputView({ cmd, initial, onSubmit, onBack }: Props) {
  const [text, setText] = useState(initial);
  const ref = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    ref.current?.focus();
    ref.current?.setSelectionRange(initial.length, initial.length);
  }, [initial]);

  const submit = () => text.trim() && onSubmit(text);

  return (
    <>
      <div className="flex items-center gap-3 px-4 pt-3">
        <Badge title={cmd.title} category={cmd.category} />
        <span className="font-medium">{cmd.title}</span>
      </div>
      <textarea
        ref={ref}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            submit();
          } else if (e.key === "Escape") {
            e.preventDefault();
            onBack();
          }
        }}
        placeholder={cmd.input === "text" ? "Type here…" : "Type or paste the text to work on…"}
        className="m-3 flex-1 resize-none rounded-lg border border-line bg-transparent p-3 outline-none focus:border-accent"
      />
      <Footer
        status="No text was selected in the other app"
        hints={[
          { keys: "Esc", label: "Back", onClick: onBack },
          { keys: "↵", label: "Run", onClick: submit, disabled: !text.trim() },
        ]}
      />
    </>
  );
}
