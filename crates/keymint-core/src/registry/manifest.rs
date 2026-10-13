use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::ai::{AiService, ProgressSink};
use crate::config::Config;
use crate::error::Result;

/// What a command needs from the user before it can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InputKind {
    /// Runs immediately.
    None,
    /// Needs text; uses the captured selection when there is one, otherwise asks.
    Selection,
    /// Always asks for text (pre-filled with the selection if there is one).
    Text,
    /// Uses text typed after a slash command (`/ask why…`) when given, never asks.
    OptionalText,
}

/// Static description of a command: everything the UI needs without running it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandManifest {
    /// Namespaced and stable, e.g. `ai.fix-grammar`, `shell.git-status`.
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub keywords: Vec<String>,
    /// Icon name resolved by the frontend; unknown names fall back to a default.
    pub icon: String,
    pub category: String,
    pub input: InputKind,
    /// Whether the command's text result can replace the selection.
    pub replaces_selection: bool,
    /// Name used after `/` in the palette, e.g. `ste` for `/ste`. Lowercase, no spaces.
    #[serde(default)]
    pub slash: Option<String>,
    /// Accelerator assigned by the user, filled in by the registry.
    #[serde(default)]
    pub shortcut: Option<String>,
}

impl CommandManifest {
    pub fn new(id: impl Into<String>, title: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: String::new(),
            keywords: Vec::new(),
            icon: "command".into(),
            category: category.into(),
            input: InputKind::None,
            replaces_selection: false,
            slash: None,
            shortcut: None,
        }
    }

    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    pub fn icon(mut self, s: impl Into<String>) -> Self {
        self.icon = s.into();
        self
    }
    pub fn slash(mut self, name: impl AsRef<str>) -> Self {
        let name: String = name
            .as_ref()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_whitespace() { '-' } else { c })
            .collect();
        self.slash = (!name.is_empty()).then_some(name);
        self
    }
    pub fn input(mut self, kind: InputKind) -> Self {
        self.input = kind;
        self
    }
    pub fn keywords<I, S>(mut self, words: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.keywords = words.into_iter().map(Into::into).collect();
        self
    }
    pub fn replaces_selection(mut self, yes: bool) -> Self {
        self.replaces_selection = yes;
        self
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CommandInput {
    #[serde(default)]
    pub text: Option<String>,
}

impl CommandInput {
    pub fn text(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }

    pub fn require_text(&self) -> Result<&str> {
        let t = self.text().trim();
        if t.is_empty() {
            Err(crate::error::Error::InvalidInput("This command needs some text to work on".into()))
        } else {
            Ok(self.text())
        }
    }
}

/// Side effects a command asks the host shell to perform. Keeping these as
/// data means the core never depends on a window system or OS APIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Action {
    OpenSettings,
    /// Handled by the palette itself: switch to the history view.
    ShowHistory,
    /// Handled by the palette itself: open the chat view, optionally with a first question.
    OpenChat { question: Option<String> },
    OpenUrl { url: String },
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CommandOutput {
    /// Show text in the result view (Copy / Retry / Replace).
    Text { text: String },
    /// Nothing to show; the palette can close.
    Done,
    /// Ask the host to do something, then close the palette.
    Action { action: Action },
}

/// Services available to a running command. Commands never reach for globals.
pub struct CommandContext {
    pub config: Config,
    pub ai: Arc<AiService>,
    /// Where long-running commands report what they are doing.
    pub progress: ProgressSink,
}

#[async_trait]
pub trait Command: Send + Sync {
    fn manifest(&self) -> &CommandManifest;
    async fn run(&self, ctx: &CommandContext, input: CommandInput) -> Result<CommandOutput>;
}

/// A source of commands. Built-in groups, user-defined commands and future
/// plugins are all providers; the registry is rebuilt from them whenever the
/// configuration changes.
pub trait CommandProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn commands(&self, cfg: &Config) -> Vec<Arc<dyn Command>>;
}
