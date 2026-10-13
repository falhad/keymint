// Mirrors the serde types in crates/keymint-core. Keep in sync by hand.

export type InputKind = "none" | "selection" | "text" | "optional-text";

export interface CommandManifest {
  id: string;
  title: string;
  subtitle: string;
  keywords: string[];
  icon: string;
  category: string;
  slash: string | null;
  input: InputKind;
  replaces_selection: boolean;
  shortcut: string | null;
}

export type Action =
  | { type: "open-settings" }
  | { type: "show-history" }
  | { type: "open-chat"; question: string | null }
  | { type: "open-url"; url: string }
  | { type: "quit" };

export type CommandOutput =
  | { type: "text"; text: string }
  | { type: "done" }
  | { type: "action"; action: Action };

export type ProviderKind = "openai" | "anthropic" | "ollama" | "openrouter" | "ninerouter";
export type Theme = "system" | "light" | "dark";

export interface ProviderConfig {
  base_url: string;
  model: string;
}

export interface CustomPrompt {
  id: string;
  title: string;
  prompt: string;
}

export interface ShellCommandDef {
  id: string;
  title: string;
  program: string;
  args: string[];
  uses_input: boolean;
  timeout_secs: number;
}

export interface UrlCommandDef {
  id: string;
  title: string;
  url: string;
}

export interface Config {
  version: number;
  theme: Theme;
  shortcut: string;
  launch_at_startup: boolean;
  hide_on_blur: boolean;
  auto_replace: boolean;
  history_limit: number;
  chat_system_prompt: string;
  target_language: string;
  default_provider: ProviderKind;
  providers: Record<ProviderKind, ProviderConfig>;
  custom_prompts: CustomPrompt[];
  shell_commands: ShellCommandDef[];
  url_commands: UrlCommandDef[];
  command_shortcuts: Record<string, string>;
}

export interface ProviderStatus {
  kind: ProviderKind;
  label: string;
  requires_key: boolean;
  has_key: boolean;
}

export interface PlatformInfo {
  os: "macos" | "windows" | "linux";
  session: "wayland" | "x11" | "native";
  global_shortcut: boolean;
  auto_capture: boolean;
  auto_replace: boolean;
  notes: string[];
}

export type ReplaceOutcome = "pasted" | "copied";

export interface OpenPayload {
  selection: string | null;
  run: string | null;
}

export type Progress =
  | { type: "connecting"; provider: string; model: string }
  | { type: "thinking" }
  | { type: "writing" }
  | { type: "delta"; text: string };

export interface ProgressEvent {
  run_id: number;
  event: Progress;
}

export interface HistoryEntry {
  id: number;
  command_id: string;
  command_title: string;
  input: string;
  output: string;
  timestamp: number;
}

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

export interface SlashResult {
  items: CommandManifest[];
  /** Text typed after a complete slash command name. */
  args: string;
}
