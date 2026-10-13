import { useEffect, useState } from "react";
import { api } from "../lib/ipc";
import type { CommandManifest, PlatformInfo } from "../lib/types";
import type { Draft, UpdateFn } from "./useDraft";
import { Card, Field, TextInput } from "./ui";
import { ShortcutRecorder } from "./ShortcutRecorder";

interface Props {
  cfg: Draft;
  update: UpdateFn;
  platform: PlatformInfo | null;
  /** Commands are only known to the registry after saving. */
  dirty: boolean;
}

export function Shortcuts({ cfg, update, platform, dirty }: Props) {
  const [commands, setCommands] = useState<CommandManifest[]>([]);
  const [filter, setFilter] = useState("");

  // Reload after saves so newly created commands appear.
  useEffect(() => {
    if (!dirty) api.listCommands().then(setCommands).catch(console.error);
  }, [dirty]);

  const supported = platform?.global_shortcut !== false;
  const shown = commands.filter((c) => c.title.toLowerCase().includes(filter.toLowerCase()) || c.id.includes(filter.toLowerCase()));

  const setCommandShortcut = (id: string, accel: string) =>
    update((c) => {
      const next = { ...c.command_shortcuts };
      if (accel) next[id] = accel;
      else delete next[id];
      return { ...c, command_shortcuts: next };
    });

  return (
    <>
      <h2 className="text-lg font-semibold">Shortcuts</h2>
      {!supported && (
        <Card>
          <p className="text-sm">
            Your Wayland session does not allow apps to register global shortcuts. Create a custom keyboard shortcut in your
            desktop's settings instead:
          </p>
          <ul className="list-disc space-y-1 pl-5 text-sm">
            <li>
              Open the palette: <code>keymint --toggle</code>
            </li>
            <li>
              Run one command directly: <code>keymint --run ai.fix-grammar</code>
            </li>
          </ul>
          <p className="text-xs text-muted">The command IDs are shown below. Keymint must already be running (it lives in the tray).</p>
        </Card>
      )}
      <Card>
        <Field label="Open Keymint">
          <ShortcutRecorder value={cfg.shortcut} disabled={!supported} onChange={(v) => update((c) => ({ ...c, shortcut: v }))} />
        </Field>
      </Card>

      <h3 className="pt-2 font-medium">Command shortcuts</h3>
      {dirty && <p className="text-xs text-muted">Save your changes to refresh this list.</p>}
      <TextInput placeholder="Filter commands…" value={filter} onChange={(e) => setFilter(e.target.value)} />
      <div className="divide-y divide-line rounded-xl border border-line">
        {shown.map((cmd) => (
          <div key={cmd.id} className="flex items-center gap-3 px-4 py-2">
            <div className="min-w-0 flex-1">
              <div className="truncate">{cmd.title}</div>
              <code className="text-xs text-muted">{cmd.id}</code>
            </div>
            {supported && (
              <ShortcutRecorder value={cfg.command_shortcuts[cmd.id] ?? ""} onChange={(v) => setCommandShortcut(cmd.id, v)} />
            )}
          </div>
        ))}
      </div>
    </>
  );
}
