import { useState } from "react";
import { api, errorMessage } from "../lib/ipc";
import type { Theme } from "../lib/types";
import type { Draft, UpdateFn } from "./useDraft";
import { Button, Card, Field, TextArea, TextInput, Toggle } from "./ui";

export function General({ cfg, update }: { cfg: Draft; update: UpdateFn }) {
  const [historyMessage, setHistoryMessage] = useState<string | null>(null);
  const clearHistory = () =>
    api
      .clearHistory()
      .then(() => setHistoryMessage("History cleared"))
      .catch((e) => setHistoryMessage(errorMessage(e)));

  return (
    <>
      <h2 className="text-lg font-semibold">General</h2>
      <Card>
        <Field label="Theme">
          <select
            value={cfg.theme}
            onChange={(e) => update((c) => ({ ...c, theme: e.target.value as Theme }))}
            className="rounded-md border border-line bg-surface px-2.5 py-1.5"
          >
            <option value="system">Match system</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </Field>
        <Field label="Translate to" hint="Used by the Translate command and by {{language}} in custom prompts.">
          <TextInput value={cfg.target_language} onChange={(e) => update((c) => ({ ...c, target_language: e.target.value }))} />
        </Field>
      </Card>
      <Card>
        <Field label="Ask AI instructions" hint="The system prompt used for chats started with Ask AI or /ask.">
          <TextArea
            rows={3}
            value={cfg.chat_system_prompt}
            onChange={(e) => update((c) => ({ ...c, chat_system_prompt: e.target.value }))}
          />
        </Field>
      </Card>
      <Card>
        <Field
          label="History size"
          hint="Results are kept in a local file (history.json) together with the text you ran them on. Set 0 to keep nothing."
        >
          <div className="flex items-center gap-3">
            <TextInput
              type="number"
              min={0}
              max={1000}
              value={cfg.history_limit}
              onChange={(e) => update((c) => ({ ...c, history_limit: Math.max(0, Math.min(1000, Number(e.target.value) || 0)) }))}
              className="!w-24"
            />
            <Button variant="danger" onClick={clearHistory}>
              Clear history
            </Button>
            <span className="text-xs text-muted">{historyMessage}</span>
          </div>
        </Field>
      </Card>
      <Card>
        <Toggle
          label="Launch Keymint at login"
          checked={cfg.launch_at_startup}
          onChange={(v) => update((c) => ({ ...c, launch_at_startup: v }))}
        />
        <Toggle
          label="Paste AI results over the selected text automatically"
          checked={cfg.auto_replace}
          onChange={(v) => update((c) => ({ ...c, auto_replace: v }))}
        />
        <Toggle
          label="Hide the palette when it loses focus"
          checked={cfg.hide_on_blur}
          onChange={(v) => update((c) => ({ ...c, hide_on_blur: v }))}
        />
        <p className="text-xs text-muted">Keymint keeps running in the system tray when its windows are closed. Use the tray menu to quit.</p>
      </Card>
    </>
  );
}
