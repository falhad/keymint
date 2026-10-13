import type { ShellCommandDef } from "../lib/types";
import type { Draft, UpdateFn } from "./useDraft";
import { Button, Card, Field, TextInput, Toggle, uniqueId } from "./ui";

export function Commands({ cfg, update }: { cfg: Draft; update: UpdateFn }) {
  return (
    <>
      <h2 className="text-lg font-semibold">Custom commands</h2>
      <UrlSection cfg={cfg} update={update} />
      <ShellSection cfg={cfg} update={update} />
    </>
  );
}

function UrlSection({ cfg, update }: { cfg: Draft; update: UpdateFn }) {
  const set = (i: number, patch: Partial<Draft["url_commands"][number]>) =>
    update((c) => ({ ...c, url_commands: c.url_commands.map((u, j) => (j === i ? { ...u, ...patch } : u)) }));

  return (
    <section className="space-y-3">
      <h3 className="font-medium">Web links</h3>
      <p className="text-sm text-muted">
        Open a web address. <code>{"{{query}}"}</code> is replaced with the text you type or select. Only http(s) links are allowed.
      </p>
      {cfg.url_commands.map((u, i) => (
        <Card key={i}>
          <div className="grid grid-cols-2 gap-3">
            <Field label="Title">
              <TextInput
                value={u.title}
                onChange={(e) => set(i, { title: e.target.value })}
              />
            </Field>
            <Field label="URL">
              <TextInput value={u.url} placeholder="https://example.com/?q={{query}}" onChange={(e) => set(i, { url: e.target.value })} />
            </Field>
          </div>
          <Button variant="danger" onClick={() => update((c) => ({ ...c, url_commands: c.url_commands.filter((_, j) => j !== i) }))}>
            Delete
          </Button>
        </Card>
      ))}
      <Button
        onClick={() =>
          update((c) => ({
            ...c,
            url_commands: [...c.url_commands, { id: uniqueId("new-link", c.url_commands.map((u) => u.id)), title: "New link", url: "https://" }],
          }))
        }
      >
        Add web link
      </Button>
    </section>
  );
}

function ShellSection({ cfg, update }: { cfg: Draft; update: UpdateFn }) {
  const set = (i: number, patch: Partial<ShellCommandDef>) =>
    update((c) => ({ ...c, shell_commands: c.shell_commands.map((s, j) => (j === i ? { ...s, ...patch } : s)) }));

  return (
    <section className="space-y-3 pt-4">
      <h3 className="font-medium">Programs</h3>
      <p className="text-sm text-muted">
        Run a program and show its output. Arguments are passed directly (no shell), one per line.
        <code>{" {{text}}"}</code> in an argument is replaced with the selected text. Only commands you add here can run.
      </p>
      {cfg.shell_commands.map((s, i) => (
        <Card key={i}>
          <div className="grid grid-cols-2 gap-3">
            <Field label="Title">
              <TextInput
                value={s.title}
                onChange={(e) => set(i, { title: e.target.value })}
              />
            </Field>
            <Field label="Program">
              <TextInput value={s.program} placeholder="git" onChange={(e) => set(i, { program: e.target.value })} />
            </Field>
          </div>
          <Field label="Arguments (one per line)">
            <textarea
              rows={3}
              value={s.args.join("\n")}
              placeholder={"status\n--short"}
              onChange={(e) => set(i, { args: e.target.value === "" ? [] : e.target.value.split("\n") })}
              className="w-full resize-y rounded-md border border-line bg-transparent px-2.5 py-1.5 font-mono text-xs outline-none focus:border-accent"
            />
          </Field>
          <div className="flex items-center gap-4">
            <Toggle label="Uses selected text" checked={s.uses_input} onChange={(v) => set(i, { uses_input: v })} />
            <label className="flex items-center gap-2 text-sm">
              Timeout (s)
              <TextInput
                type="number"
                min={1}
                value={s.timeout_secs}
                onChange={(e) => set(i, { timeout_secs: Math.max(1, Number(e.target.value) || 1) })}
                className="!w-20"
              />
            </label>
            <span className="flex-1" />
            <Button variant="danger" onClick={() => update((c) => ({ ...c, shell_commands: c.shell_commands.filter((_, j) => j !== i) }))}>
              Delete
            </Button>
          </div>
        </Card>
      ))}
      <Button
        onClick={() =>
          update((c) => ({
            ...c,
            shell_commands: [
              ...c.shell_commands,
              { id: uniqueId("new-program", c.shell_commands.map((s) => s.id)), title: "New program", program: "", args: [], uses_input: false, timeout_secs: 30 },
            ],
          }))
        }
      >
        Add program
      </Button>
    </section>
  );
}
