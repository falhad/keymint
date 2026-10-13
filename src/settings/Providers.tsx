import { useCallback, useEffect, useState } from "react";
import { api, errorMessage } from "../lib/ipc";
import type { ProviderKind, ProviderStatus } from "../lib/types";
import type { Draft, UpdateFn } from "./useDraft";
import { Button, Card, Field, TextInput } from "./ui";

const DEFAULT_URLS: Record<ProviderKind, string> = {
  openai: "https://api.openai.com/v1",
  anthropic: "https://api.anthropic.com/v1",
  ollama: "http://localhost:11434",
  openrouter: "https://openrouter.ai/api/v1",
  ninerouter: "http://localhost:20128/v1",
};

export function Providers({ cfg, update }: { cfg: Draft; update: UpdateFn }) {
  const [status, setStatus] = useState<ProviderStatus[]>([]);
  const [statusError, setStatusError] = useState<string | null>(null);

  const reload = useCallback(() => {
    api
      .providerStatus()
      .then((s) => {
        setStatus(s);
        setStatusError(null);
      })
      .catch((e) => setStatusError(errorMessage(e)));
  }, []);
  useEffect(reload, [reload]);

  return (
    <>
      <h2 className="text-lg font-semibold">AI providers</h2>
      <p className="text-sm text-muted">
        API keys are stored in your operating system's keychain and are never written to the settings file.
        Saving a key or testing a provider applies immediately; the default provider, model and URL need <b>Save</b>.
      </p>
      {statusError && <p className="text-sm text-danger">Keychain unavailable: {statusError}</p>}
      {status.map((s) => (
        <ProviderCard key={s.kind} status={s} cfg={cfg} update={update} onKeyChange={reload} />
      ))}
    </>
  );
}

function ProviderCard({
  status,
  cfg,
  update,
  onKeyChange,
}: {
  status: ProviderStatus;
  cfg: Draft;
  update: UpdateFn;
  onKeyChange: () => void;
}) {
  const kind = status.kind;
  const pc = cfg.providers[kind];
  const [key, setKey] = useState("");
  const [message, setMessage] = useState<{ text: string; ok: boolean } | null>(null);
  const [busy, setBusy] = useState(false);
  const [models, setModels] = useState<string[]>([]);

  const patch = (p: Partial<typeof pc>) =>
    update((c) => ({ ...c, providers: { ...c.providers, [kind]: { ...c.providers[kind], ...p } } }));

  const run = async (fn: () => Promise<string | void>) => {
    setBusy(true);
    try {
      const text = await fn();
      setMessage(text ? { text, ok: true } : null);
    } catch (e) {
      setMessage({ text: errorMessage(e), ok: false });
    } finally {
      setBusy(false);
    }
  };

  const saveKey = () =>
    run(async () => {
      await api.setApiKey(kind, key);
      setKey("");
      onKeyChange();
      return "API key saved";
    });

  return (
    <Card>
      <div className="flex items-center gap-3">
        <label className="flex cursor-pointer items-center gap-2 font-medium">
          <input
            type="radio"
            name="default-provider"
            checked={cfg.default_provider === kind}
            onChange={() => update((c) => ({ ...c, default_provider: kind }))}
            className="accent-[var(--accent)]"
          />
          {status.label}
        </label>
        {cfg.default_provider === kind && <span className="rounded-full bg-accent/15 px-2 py-0.5 text-xs text-accent">Default</span>}
        <span className="flex-1" />
        {status.requires_key ? (
          <span className={`text-xs ${status.has_key ? "text-accent" : "text-muted"}`}>
            {status.has_key ? "Key saved" : "No key"}
          </span>
        ) : (
          <span className="text-xs text-muted">No key needed</span>
        )}
      </div>

      <div className="grid grid-cols-2 gap-3">
        <Field label="Model">
          <TextInput value={pc.model} list={`models-${kind}`} onChange={(e) => patch({ model: e.target.value })} />
          <datalist id={`models-${kind}`}>
            {models.map((m) => (
              <option key={m} value={m} />
            ))}
          </datalist>
        </Field>
        <Field label="Base URL">
          <TextInput value={pc.base_url} placeholder={DEFAULT_URLS[kind]} onChange={(e) => patch({ base_url: e.target.value })} />
        </Field>
      </div>

      {status.requires_key && (
        <Field label="API key">
          <div className="flex gap-2">
            <TextInput
              type="password"
              autoComplete="off"
              value={key}
              placeholder={status.has_key ? "•••••••• (enter a new key to replace)" : "Paste your API key"}
              onChange={(e) => setKey(e.target.value)}
            />
            <Button onClick={() => void saveKey()} disabled={busy || !key.trim()}>
              Save key
            </Button>
            {status.has_key && (
              <Button
                variant="danger"
                disabled={busy}
                onClick={() =>
                  void run(async () => {
                    await api.deleteApiKey(kind);
                    onKeyChange();
                    return "API key removed";
                  })
                }
              >
                Remove
              </Button>
            )}
          </div>
        </Field>
      )}

      <div className="flex items-center gap-2">
        <Button
          disabled={busy}
          onClick={() =>
            void run(async () => {
              const reply = await api.testProvider(kind);
              return `Connected. Reply: ${reply.slice(0, 60)}`;
            })
          }
        >
          Test connection
        </Button>
        {kind !== "anthropic" && (
          <Button
            disabled={busy}
            onClick={() =>
              void run(async () => {
                const found = await api.listModels(kind);
                setModels(found);
                return found.length ? `${found.length} models found` : "No models found";
              })
            }
          >
            Find models
          </Button>
        )}
        <span className={`min-w-0 flex-1 truncate text-xs ${message?.ok ? "text-muted" : "text-danger"}`}>{message?.text}</span>
      </div>
      <p className="text-xs text-muted">Test uses the saved settings, so press Save first after changing the model or URL.</p>
    </Card>
  );
}
