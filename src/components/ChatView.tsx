import { useEffect, useRef, useState } from "react";
import { Footer } from "./Footer";
import { Steps, type RunProgress } from "./ResultView";
import { WorkingAnimation } from "./WorkingAnimation";

/** A chat message as shown; `content` is what is sent, `shown` what the user typed. */
export interface UiMessage {
  role: "user" | "assistant";
  content: string;
  shown?: string;
}

interface Props {
  messages: UiMessage[];
  busy: boolean;
  error: string | null;
  progress: RunProgress;
  /** Selected text offered as context for the first message. */
  context: string | null;
  notice: string | null;
  onSend: (text: string) => void;
  onRetry: () => void;
  onNewChat: () => void;
  onDropContext: () => void;
  onCopy: (text: string) => void;
  onPaste: (text: string) => void;
  onBack: () => void;
}

export function ChatView(p: Props) {
  const [draft, setDraft] = useState("");
  const input = useRef<HTMLTextAreaElement>(null);
  const end = useRef<HTMLDivElement>(null);

  useEffect(() => {
    input.current?.focus();
  }, [p.busy]);
  useEffect(() => {
    end.current?.scrollIntoView({ block: "end" });
  }, [p.messages.length, p.progress.text, p.busy, p.error]);

  // The input is disabled while waiting, so Esc must be caught at window level to cancel.
  const { busy, onBack } = p;
  useEffect(() => {
    if (!busy) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onBack();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [busy, onBack]);

  const send = () => {
    const text = draft.trim();
    if (!text || p.busy) return;
    setDraft("");
    p.onSend(text);
  };

  return (
    <>
      <div className="flex items-center gap-2 border-b border-line px-4 py-3">
        <span className="font-medium">Ask AI</span>
        {p.progress.provider && (
          <span className="text-xs text-muted">
            {p.progress.provider}
            {p.progress.model ? ` · ${p.progress.model}` : ""}
          </span>
        )}
        <span className="flex-1" />
        {p.context && p.messages.length === 0 && (
          <button
            type="button"
            title="Your selected text is sent with the first message. Click to leave it out."
            onClick={p.onDropContext}
            className="rounded-full bg-accent/15 px-2.5 py-0.5 text-xs text-accent"
          >
            Using {p.context.length} selected chars ✕
          </button>
        )}
      </div>

      <div className="flex-1 space-y-3 overflow-y-auto px-4 py-3">
        {p.messages.length === 0 && !p.busy && (
          <p className="pt-8 text-center text-muted">Ask anything. Press Enter to send.</p>
        )}
        {p.messages.map((m, i) =>
          m.role === "user" ? (
            <div key={i} className="ml-10 whitespace-pre-wrap break-words rounded-xl bg-accent/15 px-3 py-2 select-text">
              {m.shown ?? m.content}
            </div>
          ) : (
            <div key={i} className="mr-6 space-y-1">
              <pre className="whitespace-pre-wrap break-words font-sans">{m.content}</pre>
              <div className="flex gap-3 text-xs text-muted">
                <button type="button" className="hover:text-fg" onClick={() => p.onCopy(m.content)}>
                  Copy
                </button>
                <button type="button" className="hover:text-fg" onClick={() => p.onPaste(m.content)}>
                  Paste into app
                </button>
              </div>
            </div>
          ),
        )}
        {p.busy && (
          <div className="mr-6 space-y-2">
            {p.progress.text ? (
              <pre className="whitespace-pre-wrap break-words font-sans">
                {p.progress.text}
                <span className="ml-0.5 inline-block h-4 w-1.5 translate-y-0.5 animate-pulse bg-accent" />
              </pre>
            ) : (
              <div className="space-y-2">
                <WorkingAnimation />
                <Steps progress={p.progress} />
              </div>
            )}
          </div>
        )}
        {p.error && (
          <div className="space-y-2">
            <p className="whitespace-pre-wrap text-danger">{p.error}</p>
            <button type="button" onClick={p.onRetry} className="rounded-md border border-line px-3 py-1 hover:bg-surface-2">
              Retry
            </button>
          </div>
        )}
        <div ref={end} />
      </div>

      <div className="border-t border-line p-3">
        <textarea
          ref={input}
          rows={2}
          value={draft}
          disabled={p.busy}
          placeholder={p.busy ? "Waiting for the answer…" : "Message"}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              send();
            } else if (e.key === "Escape") {
              e.preventDefault();
              p.onBack();
            } else if (e.ctrlKey && e.key.toLowerCase() === "n") {
              e.preventDefault();
              p.onNewChat();
            }
          }}
          className="w-full resize-none rounded-lg border border-line bg-transparent p-2.5 outline-none focus:border-accent disabled:opacity-60"
        />
      </div>
      <Footer
        status={p.notice}
        hints={[
          { keys: "↵", label: "Send", onClick: send, disabled: p.busy || !draft.trim() },
          { keys: "Ctrl N", label: "New chat", onClick: p.onNewChat },
          { keys: "Esc", label: p.busy ? "Cancel" : "Back", onClick: p.onBack },
        ]}
      />
    </>
  );
}
