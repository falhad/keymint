import { useEffect, useRef, useState } from "react";
import type { CommandManifest, PlatformInfo } from "../lib/types";
import { Badge } from "./Badge";
import { Footer, type Hint } from "./Footer";
import { WorkingAnimation } from "./WorkingAnimation";

export type RunState =
  | { kind: "running" }
  | { kind: "text"; text: string }
  | { kind: "error"; message: string };

/** What the AI side reported so far for the current run. */
export interface RunProgress {
  stage: "connecting" | "thinking" | "writing";
  provider?: string;
  model?: string;
  /** Streamed text received so far. */
  text: string;
}

export const INITIAL_PROGRESS: RunProgress = { stage: "connecting", text: "" };

interface Props {
  cmd: CommandManifest;
  state: RunState;
  progress: RunProgress;
  canReplace: boolean;
  platform: PlatformInfo | null;
  notice: string | null;
  onCopy: () => void;
  onRetry: () => void;
  onReplace: () => void;
  onEditInput: () => void;
  onOpenSettings: () => void;
  onBack: () => void;
}

const STAGES = ["connecting", "thinking", "writing"] as const;

function isAi(cmd: CommandManifest) {
  return cmd.category === "AI" || cmd.category === "Custom Prompt";
}

export function Steps({ progress }: { progress: RunProgress }) {
  const active = STAGES.indexOf(progress.stage);
  const target = progress.provider ? `${progress.provider}${progress.model ? ` · ${progress.model}` : ""}` : "the AI provider";
  const labels = [`Sending request to ${target}`, "Waiting for the model to start", "Receiving the reply"];
  return (
    <ol className="space-y-1.5 text-sm">
      {labels.map((label, i) => {
        const done = i < active;
        const current = i === active;
        return (
          <li key={label} className={`flex items-center gap-2 ${done || current ? "" : "text-muted"}`}>
            <span
              className={`grid size-4 place-items-center rounded-full text-[10px] ${
                done ? "bg-accent text-accent-fg" : current ? "border-2 border-accent" : "border border-line"
              } ${current ? "animate-pulse" : ""}`}
            >
              {done ? "✓" : ""}
            </span>
            {label}
          </li>
        );
      })}
    </ol>
  );
}

function Running({ cmd, progress }: { cmd: CommandManifest; progress: RunProgress }) {
  const [seconds, setSeconds] = useState(0);
  useEffect(() => {
    setSeconds(0);
    const t = setInterval(() => setSeconds((s) => s + 1), 1000);
    return () => clearInterval(t);
  }, []);

  // Keep the newest streamed text in view.
  const tail = useRef<HTMLDivElement>(null);
  useEffect(() => {
    tail.current?.scrollIntoView({ block: "end" });
  }, [progress.text]);

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-4">
        <WorkingAnimation />
        <span className="text-xs text-muted">{seconds}s</span>
      </div>
      {isAi(cmd) ? <Steps progress={progress} /> : <p className="text-sm text-muted">Working…</p>}
      {progress.text && (
        <pre className="whitespace-pre-wrap break-words border-t border-line pt-3 font-sans">
          {progress.text}
          <span className="ml-0.5 inline-block h-4 w-1.5 translate-y-0.5 animate-pulse bg-accent" />
        </pre>
      )}
      <div ref={tail} />
    </div>
  );
}

export function ResultView(p: Props) {
  const done = p.state.kind === "text";
  const replaceLabel = p.platform?.auto_replace ? "Replace Selection" : "Replace (copy)";

  const hints: Hint[] = [{ keys: "Esc", label: p.state.kind === "running" ? "Cancel" : "Back", onClick: p.onBack }];
  if (p.state.kind !== "running") hints.unshift({ keys: "Ctrl R", label: "Retry", onClick: p.onRetry });
  if (done) {
    hints.unshift({ keys: "Ctrl E", label: "Edit input", onClick: p.onEditInput });
    if (p.canReplace) hints.unshift({ keys: "Ctrl ↵", label: replaceLabel, onClick: p.onReplace });
    hints.unshift({ keys: "↵", label: "Copy", onClick: p.onCopy });
  }

  return (
    <>
      <div className="flex items-center gap-3 px-4 pt-3">
        <Badge title={p.cmd.title} category={p.cmd.category} />
        <span className="font-medium">{p.cmd.title}</span>
        {p.progress.provider && (
          <span className="ml-auto truncate text-xs text-muted">
            {p.progress.provider}
            {p.progress.model ? ` · ${p.progress.model}` : ""}
          </span>
        )}
      </div>
      <div className="m-3 flex-1 overflow-y-auto rounded-lg border border-line p-3">
        {p.state.kind === "running" && <Running cmd={p.cmd} progress={p.progress} />}
        {p.state.kind === "text" && <pre className="whitespace-pre-wrap break-words font-sans">{p.state.text}</pre>}
        {p.state.kind === "error" && (
          <div className="space-y-3">
            <p className="whitespace-pre-wrap text-danger">{p.state.message}</p>
            {/api key|provider|ollama|network/i.test(p.state.message) && (
              <button
                type="button"
                onClick={p.onOpenSettings}
                className="rounded-md bg-accent px-3 py-1.5 text-accent-fg"
              >
                Open Settings
              </button>
            )}
          </div>
        )}
      </div>
      <Footer status={p.notice} hints={hints} />
    </>
  );
}
