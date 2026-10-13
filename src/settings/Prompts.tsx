import type { Draft, UpdateFn } from "./useDraft";
import { Button, Card, Field, TextArea, TextInput, uniqueId } from "./ui";

export function Prompts({ cfg, update }: { cfg: Draft; update: UpdateFn }) {
  const set = (i: number, patch: Partial<Draft["custom_prompts"][number]>) =>
    update((c) => ({ ...c, custom_prompts: c.custom_prompts.map((p, j) => (j === i ? { ...p, ...patch } : p)) }));

  const add = () =>
    update((c) => ({
      ...c,
      custom_prompts: [
        ...c.custom_prompts,
        { id: uniqueId("new-prompt", c.custom_prompts.map((p) => p.id)), title: "New prompt", prompt: "" },
      ],
    }));

  return (
    <>
      <h2 className="text-lg font-semibold">Custom prompts</h2>
      <p className="text-sm text-muted">
        Each prompt becomes a command in the palette. Write the instructions; the selected text is sent as the message.
        Use <code>{"{{language}}"}</code> for your target language.
      </p>
      {cfg.custom_prompts.map((p, i) => (
        <Card key={i}>
          <Field label="Title">
            <TextInput
              value={p.title}
              onChange={(e) => set(i, { title: e.target.value })}
            />
          </Field>
          <Field label="Instructions">
            <TextArea rows={4} value={p.prompt} placeholder="e.g. Rewrite this as a polite customer-support reply." onChange={(e) => set(i, { prompt: e.target.value })} />
          </Field>
          <div className="flex items-center">
            <span className="flex-1 text-xs text-muted">
              Command id: <code>prompt.{p.id}</code>
            </span>
            <Button variant="danger" onClick={() => update((c) => ({ ...c, custom_prompts: c.custom_prompts.filter((_, j) => j !== i) }))}>
              Delete
            </Button>
          </div>
        </Card>
      ))}
      <Button onClick={add}>Add prompt</Button>
    </>
  );
}
