import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ChatMessage,
  CommandManifest,
  CommandOutput,
  Config,
  HistoryEntry,
  OpenPayload,
  PlatformInfo,
  ProviderKind,
  ProgressEvent,
  ProviderStatus,
  ReplaceOutcome,
  SlashResult,
} from "./types";

/** Errors from Rust arrive as plain strings. */
export function errorMessage(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}

export const api = {
  search: (query: string) => invoke<CommandManifest[]>("search", { query }),
  searchSlash: (query: string) => invoke<SlashResult>("search_slash", { query }),
  chat: (messages: ChatMessage[], runId: number) => invoke<string>("chat", { messages, runId }),
  listCommands: () => invoke<CommandManifest[]>("list_commands"),
  execute: (id: string, text: string | null, runId: number) =>
    invoke<CommandOutput>("execute", { id, input: { text }, runId }),
  listHistory: () => invoke<HistoryEntry[]>("list_history"),
  deleteHistoryEntry: (id: number) => invoke<void>("delete_history_entry", { id }),
  clearHistory: () => invoke<void>("clear_history"),
  getConfig: () => invoke<Config>("get_config"),
  saveConfig: (config: Config) => invoke<Config>("save_config", { config }),
  providerStatus: () => invoke<ProviderStatus[]>("provider_status"),
  setApiKey: (provider: ProviderKind, key: string) => invoke<void>("set_api_key", { provider, key }),
  deleteApiKey: (provider: ProviderKind) => invoke<void>("delete_api_key", { provider }),
  testProvider: (provider: ProviderKind) => invoke<string>("test_provider", { provider }),
  listModels: (provider: ProviderKind) => invoke<string[]>("list_models", { provider }),
  platformInfo: () => invoke<PlatformInfo>("platform_info"),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  replaceSelection: (text: string) => invoke<ReplaceOutcome>("replace_selection", { text }),
  hidePalette: () => invoke<void>("hide_palette"),
  openSettings: () => invoke<void>("open_settings"),
};

export const events = {
  onPaletteOpen: (cb: (p: OpenPayload) => void): Promise<UnlistenFn> =>
    listen<OpenPayload>("palette:open", (e) => cb(e.payload)),
  onProgress: (cb: (e: ProgressEvent) => void): Promise<UnlistenFn> =>
    listen<ProgressEvent>("command:progress", (e) => cb(e.payload)),
  onConfigChanged: (cb: (c: Config) => void): Promise<UnlistenFn> =>
    listen<Config>("config:changed", (e) => cb(e.payload)),
};

/** Copy from the focused webview first (works on Wayland), fall back to the native clipboard. */
export async function copyToClipboard(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    await api.copyText(text);
  }
}
