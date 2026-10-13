import { useEffect, useState } from "react";
import { api } from "../lib/ipc";
import type { PlatformInfo } from "../lib/types";
import { Commands } from "./Commands";
import { General } from "./General";
import { Prompts } from "./Prompts";
import { Providers } from "./Providers";
import { Shortcuts } from "./Shortcuts";
import { Button } from "./ui";
import { useDraft } from "./useDraft";

const TABS = ["General", "Providers", "Prompts", "Commands", "Shortcuts"] as const;
type Tab = (typeof TABS)[number];

export function Settings() {
  const { draft, update, save, revert, dirty, busy, error } = useDraft();
  const [tab, setTab] = useState<Tab>("General");
  const [platform, setPlatform] = useState<PlatformInfo | null>(null);

  useEffect(() => {
    api.platformInfo().then(setPlatform).catch(console.error);
  }, []);

  if (!draft) return <div className="p-6 text-muted">{error ?? "Loading…"}</div>;

  return (
    <div className="flex h-full">
      <nav className="w-44 shrink-0 space-y-1 border-r border-line p-3">
        <h1 className="px-2 pb-2 text-base font-semibold">Keymint</h1>
        {TABS.map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTab(t)}
            className={`block w-full rounded-md px-2.5 py-1.5 text-left ${tab === t ? "bg-surface-2 font-medium" : "text-muted hover:bg-surface-2"}`}
          >
            {t}
          </button>
        ))}
      </nav>

      <main className="flex min-w-0 flex-1 flex-col">
        <div className="flex-1 space-y-4 overflow-y-auto p-6">
          {platform && platform.notes.length > 0 && (
            <div className="space-y-1 rounded-lg border border-line bg-surface-2 p-3 text-xs">
              {platform.notes.map((n) => (
                <p key={n}>{n}</p>
              ))}
            </div>
          )}
          {tab === "General" && <General cfg={draft} update={update} />}
          {tab === "Providers" && <Providers cfg={draft} update={update} />}
          {tab === "Prompts" && <Prompts cfg={draft} update={update} />}
          {tab === "Commands" && <Commands cfg={draft} update={update} />}
          {tab === "Shortcuts" && <Shortcuts cfg={draft} update={update} platform={platform} dirty={dirty} />}
        </div>

        <div className="flex items-center gap-3 border-t border-line px-6 py-3">
          <span className="min-w-0 flex-1 truncate text-sm text-danger">{error}</span>
          {dirty && <span className="text-xs text-muted">Unsaved changes</span>}
          <Button onClick={revert} disabled={!dirty || busy}>
            Revert
          </Button>
          <Button variant="primary" onClick={() => void save()} disabled={!dirty || busy}>
            {busy ? "Saving…" : "Save"}
          </Button>
        </div>
      </main>
    </div>
  );
}
