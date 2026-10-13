use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
    Ollama,
    OpenRouter,
    /// Local OpenAI-compatible router (default http://localhost:20128/v1).
    #[serde(rename = "ninerouter")]
    NineRouter,
}

impl ProviderKind {
    pub const ALL: [ProviderKind; 5] = [
        ProviderKind::OpenAi,
        ProviderKind::Anthropic,
        ProviderKind::Ollama,
        ProviderKind::OpenRouter,
        ProviderKind::NineRouter,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "openai",
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Ollama => "ollama",
            ProviderKind::OpenRouter => "openrouter",
            ProviderKind::NineRouter => "ninerouter",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "OpenAI",
            ProviderKind::Anthropic => "Anthropic",
            ProviderKind::Ollama => "Ollama",
            ProviderKind::OpenRouter => "OpenRouter",
            ProviderKind::NineRouter => "9router",
        }
    }

    pub fn requires_key(self) -> bool {
        !matches!(self, ProviderKind::Ollama)
    }

    pub fn default_base_url(self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "https://api.openai.com/v1",
            ProviderKind::Anthropic => "https://api.anthropic.com/v1",
            ProviderKind::Ollama => "http://localhost:11434",
            ProviderKind::OpenRouter => "https://openrouter.ai/api/v1",
            ProviderKind::NineRouter => "http://localhost:20128/v1",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "gpt-4o-mini",
            ProviderKind::Anthropic => "claude-haiku-5-5",
            ProviderKind::Ollama => "llama3.2",
            ProviderKind::OpenRouter => "openai/gpt-4o-mini",
            ProviderKind::NineRouter => "free-best",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Empty string means "use the provider default".
    #[serde(default)]
    pub base_url: String,
    pub model: String,
}

impl ProviderConfig {
    pub fn defaults_for(kind: ProviderKind) -> Self {
        Self {
            base_url: String::new(),
            model: kind.default_model().to_string(),
        }
    }

    pub fn effective_base_url(&self, kind: ProviderKind) -> String {
        let url = if self.base_url.trim().is_empty() {
            kind.default_base_url()
        } else {
            self.base_url.trim()
        };
        url.trim_end_matches('/').to_string()
    }
}

/// A user-defined AI prompt, exposed as a command (`prompt.<id>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPrompt {
    pub id: String,
    pub title: String,
    /// System prompt. `{{language}}` expands to the configured target language.
    /// The selected / typed text is sent as the user message.
    pub prompt: String,
}

/// A user-defined program, exposed as a command (`shell.<id>`).
/// Arguments are passed as an argv array — never through a shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellCommandDef {
    pub id: String,
    pub title: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// When true, `{{text}}` in args is replaced by the selection/input.
    #[serde(default)]
    pub uses_input: bool,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    30
}

/// A user-defined URL template, exposed as a command (`url.<id>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlCommandDef {
    pub id: String,
    pub title: String,
    /// e.g. `https://www.google.com/search?q={{query}}`. Only http(s) allowed.
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub theme: Theme,
    pub shortcut: String,
    pub launch_at_startup: bool,
    pub hide_on_blur: bool,
    /// Paste AI results straight over the selected text instead of showing them first.
    pub auto_replace: bool,
    /// How many results to keep in history; 0 turns history off.
    pub history_limit: u32,
    /// System prompt for the Ask AI chat.
    pub chat_system_prompt: String,
    pub target_language: String,
    pub default_provider: ProviderKind,
    pub providers: BTreeMap<ProviderKind, ProviderConfig>,
    pub custom_prompts: Vec<CustomPrompt>,
    pub shell_commands: Vec<ShellCommandDef>,
    pub url_commands: Vec<UrlCommandDef>,
    /// command id -> accelerator, e.g. `ai.fix-grammar` -> `Ctrl+Alt+G`.
    pub command_shortcuts: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let providers = ProviderKind::ALL
            .into_iter()
            .map(|k| (k, ProviderConfig::defaults_for(k)))
            .collect();
        Self {
            version: CONFIG_VERSION,
            theme: Theme::System,
            shortcut: default_shortcut().to_string(),
            launch_at_startup: false,
            hide_on_blur: true,
            auto_replace: false,
            history_limit: 50,
            chat_system_prompt: DEFAULT_CHAT_PROMPT.to_string(),
            target_language: "English".to_string(),
            default_provider: ProviderKind::OpenAi,
            providers,
            custom_prompts: Vec::new(),
            shell_commands: Vec::new(),
            url_commands: vec![UrlCommandDef {
                id: "web-search".to_string(),
                title: "Search the Web".to_string(),
                url: "https://duckduckgo.com/?q={{query}}".to_string(),
            }],
            command_shortcuts: BTreeMap::new(),
        }
    }
}

pub const DEFAULT_CHAT_PROMPT: &str = "You are the assistant built into Keymint, a desktop launcher. Answer clearly and briefly. \
Reply in the language the user writes in. Use plain text; use Markdown only for code blocks or lists when they help.";

pub fn default_shortcut() -> &'static str {
    if cfg!(target_os = "macos") {
        "Alt+Space"
    } else {
        "Ctrl+Space"
    }
}

impl Config {
    pub fn provider(&self, kind: ProviderKind) -> ProviderConfig {
        self.providers
            .get(&kind)
            .cloned()
            .unwrap_or_else(|| ProviderConfig::defaults_for(kind))
    }

    /// Fill gaps left by older or hand-edited files and reject invalid values.
    pub fn normalize(&mut self) {
        for kind in ProviderKind::ALL {
            self.providers
                .entry(kind)
                .or_insert_with(|| ProviderConfig::defaults_for(kind));
            let p = self.providers.get_mut(&kind).expect("just inserted");
            if p.model.trim().is_empty() {
                p.model = kind.default_model().to_string();
            }
        }
        if self.shortcut.trim().is_empty() {
            self.shortcut = default_shortcut().to_string();
        }
        if self.chat_system_prompt.trim().is_empty() {
            self.chat_system_prompt = DEFAULT_CHAT_PROMPT.to_string();
        }
        if self.target_language.trim().is_empty() {
            self.target_language = "English".to_string();
        }
        self.version = CONFIG_VERSION;
    }

    pub fn validate(&self) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for id in self
            .custom_prompts
            .iter()
            .map(|p| ("prompt", &p.id, &p.title))
            .chain(self.shell_commands.iter().map(|s| ("shell", &s.id, &s.title)))
            .chain(self.url_commands.iter().map(|u| ("url", &u.id, &u.title)))
        {
            let (kind, id, title) = id;
            if id.trim().is_empty() || title.trim().is_empty() {
                return Err(Error::Config(format!("A {kind} command has an empty id or title")));
            }
            if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                return Err(Error::Config(format!(
                    "Command id \"{id}\" may only contain letters, digits, '-' and '_'"
                )));
            }
            if !seen.insert((kind, id.clone())) {
                return Err(Error::Config(format!("Duplicate {kind} command id \"{id}\"")));
            }
        }
        for u in &self.url_commands {
            crate::commands::url::check_template(&u.url)?;
        }
        for s in &self.shell_commands {
            if s.program.trim().is_empty() {
                return Err(Error::Config(format!("Shell command \"{}\" has no program", s.title)));
            }
        }
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let mut cfg: Config = serde_json::from_str(&text)
                    .map_err(|e| Error::Config(format!("{} is not valid: {e}", path.display())))?;
                cfg.normalize();
                Ok(cfg)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Atomic write: temp file in the same directory, then rename.
    pub fn save(&self, path: &Path) -> Result<()> {
        write_json_atomic(path, self)
    }
}

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut cfg = Config { theme: Theme::Dark, ..Config::default() };
        cfg.custom_prompts.push(CustomPrompt {
            id: "pirate".into(),
            title: "Pirate".into(),
            prompt: "Talk like a pirate".into(),
        });
        cfg.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), cfg);
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::load(&dir.path().join("nope.json")).unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn partial_file_is_filled_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"theme":"light","providers":{"ollama":{"model":""}}}"#).unwrap();
        let cfg = Config::load(&path).unwrap();
        assert_eq!(cfg.theme, Theme::Light);
        assert_eq!(cfg.provider(ProviderKind::Ollama).model, "llama3.2");
        assert_eq!(cfg.provider(ProviderKind::OpenAi).model, "gpt-4o-mini");
    }

    #[test]
    fn corrupt_file_is_an_error_not_silently_reset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "{ nope").unwrap();
        assert!(Config::load(&path).is_err());
    }

    #[test]
    fn validation_rejects_bad_ids_and_urls() {
        let mut cfg = Config::default();
        cfg.custom_prompts.push(CustomPrompt {
            id: "bad id!".into(),
            title: "x".into(),
            prompt: "p".into(),
        });
        assert!(cfg.validate().is_err());

        let mut cfg = Config::default();
        cfg.url_commands[0].url = "javascript:alert({{query}})".into();
        assert!(cfg.validate().is_err());
        assert!(Config::default().validate().is_ok());
    }
}
