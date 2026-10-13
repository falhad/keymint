import { useCallback, useEffect, useState } from "react";
import { api, errorMessage, events } from "../lib/ipc";
import { applyTheme } from "../lib/theme";
import type { Config } from "../lib/types";

/** Editable copy of the saved config, with explicit Save / Revert. */
export function useDraft() {
  const [saved, setSaved] = useState<Config | null>(null);
  const [draft, setDraft] = useState<Config | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api
      .getConfig()
      .then((c) => {
        setSaved(c);
        setDraft(c);
        applyTheme(c.theme);
      })
      .catch((e) => setError(errorMessage(e)));
    const off = events.onConfigChanged((c) => setSaved(c));
    return () => void off.then((f) => f());
  }, []);

  const update = useCallback((fn: (c: Config) => Config) => {
    setDraft((d) => (d ? fn(d) : d));
    setError(null);
  }, []);

  const save = useCallback(async () => {
    if (!draft) return;
    setBusy(true);
    try {
      const next = await api.saveConfig(draft);
      setSaved(next);
      setDraft(next);
      applyTheme(next.theme);
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }, [draft]);

  const revert = useCallback(() => {
    setDraft(saved);
    setError(null);
  }, [saved]);

  const dirty = JSON.stringify(saved) !== JSON.stringify(draft);
  return { draft, update, save, revert, dirty, busy, error };
}

export type Draft = NonNullable<ReturnType<typeof useDraft>["draft"]>;
export type UpdateFn = ReturnType<typeof useDraft>["update"];
