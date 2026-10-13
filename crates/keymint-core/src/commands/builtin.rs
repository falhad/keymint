use std::sync::Arc;

use async_trait::async_trait;

use crate::config::Config;
use crate::error::Result;
use crate::registry::{
    Action, Command, CommandContext, CommandInput, CommandManifest, CommandOutput, CommandProvider,
    InputKind,
};

pub struct BuiltinProvider;

type TextFn = fn(&str) -> String;

/// A command that transforms text locally, no network involved.
struct TextTransform {
    manifest: CommandManifest,
    apply: TextFn,
}

#[async_trait]
impl Command for TextTransform {
    fn manifest(&self) -> &CommandManifest {
        &self.manifest
    }
    async fn run(&self, _: &CommandContext, input: CommandInput) -> Result<CommandOutput> {
        Ok(CommandOutput::Text { text: (self.apply)(input.require_text()?) })
    }
}

/// A command that only asks the host shell to do something.
struct ActionCommand {
    manifest: CommandManifest,
    action: Action,
}

#[async_trait]
impl Command for ActionCommand {
    fn manifest(&self) -> &CommandManifest {
        &self.manifest
    }
    async fn run(&self, _: &CommandContext, _: CommandInput) -> Result<CommandOutput> {
        Ok(CommandOutput::Action { action: self.action.clone() })
    }
}

pub fn word_count(text: &str) -> String {
    let words = text.split_whitespace().count();
    let chars = text.chars().count();
    let lines = text.lines().count();
    format!("{words} words · {chars} characters · {lines} lines")
}

pub fn sort_lines(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    lines.sort_by_key(|l| l.to_lowercase());
    lines.join("\n")
}

impl CommandProvider for BuiltinProvider {
    fn id(&self) -> &'static str {
        "builtin"
    }

    fn commands(&self, _: &Config) -> Vec<Arc<dyn Command>> {
        let transform = |id: &str, slash: &str, title: &str, sub: &str, icon: &str, replaces: bool, apply: TextFn| {
            Arc::new(TextTransform {
                manifest: CommandManifest::new(format!("text.{id}"), title, "Text")
                    .slash(slash)
                    .subtitle(sub)
                    .icon(icon)
                    .input(InputKind::Selection)
                    .replaces_selection(replaces),
                apply,
            }) as Arc<dyn Command>
        };
        vec![
            Arc::new(ActionCommand {
                manifest: CommandManifest::new("app.settings", "Open Settings", "Keymint")
                    .slash("settings")
                    .subtitle("Providers, prompts, shortcuts and appearance")
                    .icon("settings")
                    .keywords(["preferences", "config", "api key"]),
                action: Action::OpenSettings,
            }),
            Arc::new(ActionCommand {
                manifest: CommandManifest::new("app.history", "Show History", "Keymint")
                    .slash("history")
                    .subtitle("Reuse recent results")
                    .icon("clock")
                    .keywords(["recent", "previous", "results", "clipboard"]),
                action: Action::ShowHistory,
            }),
            Arc::new(ActionCommand {
                manifest: CommandManifest::new("app.quit", "Quit Keymint", "Keymint")
                    .slash("quit")
                    .icon("power")
                    .keywords(["exit", "close"]),
                action: Action::Quit,
            }),
            transform("uppercase", "upper", "Uppercase", "Convert text to UPPERCASE", "arrow-up", true, |t| t.to_uppercase()),
            transform("lowercase", "lower", "Lowercase", "Convert text to lowercase", "arrow-down", true, |t| t.to_lowercase()),
            transform("sort-lines", "sort", "Sort Lines", "Sort lines alphabetically", "list", true, sort_lines),
            transform("count", "count", "Count Words", "Words, characters and lines", "hash", false, word_count),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts() {
        assert_eq!(word_count("a b\nc"), "3 words · 5 characters · 2 lines");
    }

    #[test]
    fn sorts_case_insensitively() {
        assert_eq!(sort_lines("b\nA\nc"), "A\nb\nc");
    }

    #[test]
    fn provides_settings_and_quit() {
        let ids: Vec<_> = BuiltinProvider
            .commands(&Config::default())
            .iter()
            .map(|c| c.manifest().id.clone())
            .collect();
        assert!(ids.contains(&"app.settings".to_string()));
        assert!(ids.contains(&"app.quit".to_string()));
    }
}
