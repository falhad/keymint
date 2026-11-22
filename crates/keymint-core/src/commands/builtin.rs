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

const PR_APPROVED_MESSAGES: &[&str] = &[
    "Approved! 🎉 Clean, readable and easy to follow. Thanks for the great work!",
    "LGTM! 🚀 Nicely done — thanks for taking the time to make this so tidy.",
    "Approved ✅ Great job, this looks solid. Happy to see it merged!",
    "Looks great to me! 🙌 Thanks for the thoughtful PR. Approving.",
    "Approved with a smile 😊 Really appreciate the care you put into this.",
    "Nice work! 👏 Everything checks out on my side. Approved.",
    "Ship it! 🚢 Lovely change, thank you for the effort.",
    "Approved! 💚 Thanks for keeping this focused and well explained.",
    "All good here ✨ Great contribution, approving. Thank you!",
    "LGTM 👍 Thanks for the clear description and clean implementation.",
    "Approved! 🌟 This is exactly what we needed. Thank you for the quick turnaround!",
    "Beautiful work! 🎨 The code reads really well. Approving.",
    "Approved ✅ Thanks for covering the edge cases so carefully.",
    "Love it! 😍 Simple, clear and well tested. Approved.",
    "Great PR! 🙏 I learned something from reading this. Approved.",
    "Approved! 🔥 Solid work, and thanks for the helpful comments in the code.",
    "Looks good to me 👌 Thank you for making this so easy to review.",
    "Approved 🎯 Right to the point, no surprises. Nicely done!",
    "Fantastic job! 🏆 Everything looks in order. Approving.",
    "Approved! 🤝 Thanks for the patience and the clean follow-up fixes.",
    "Well done! 💪 Tidy diff, clear intent. Good to merge.",
    "LGTM ☀️ Appreciate the thoughtful details. Thanks a lot!",
    "Approved! 🧹 Thanks for leaving the code cleaner than you found it.",
    "Great stuff! 🙌 Tests pass, code is clear. Approved.",
    "Approved 🚀 Thanks for the effort, this is a nice improvement.",
];

/// Picks a message at random. `RandomState` is seeded randomly by the OS for every instance,
/// so this needs no extra dependency and does not depend on the clock's resolution.
pub fn pr_approved_message() -> &'static str {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let random = RandomState::new().build_hasher().finish() as usize;
    PR_APPROVED_MESSAGES[random % PR_APPROVED_MESSAGES.len()]
}

/// A command that produces text without any input.
struct SnippetCommand {
    manifest: CommandManifest,
    produce: fn() -> &'static str,
}

#[async_trait]
impl Command for SnippetCommand {
    fn manifest(&self) -> &CommandManifest {
        &self.manifest
    }
    async fn run(&self, _: &CommandContext, _: CommandInput) -> Result<CommandOutput> {
        Ok(CommandOutput::Text { text: (self.produce)().to_string() })
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
            Arc::new(SnippetCommand {
                manifest: CommandManifest::new("text.pr-approved", "PR Approved Message", "Text")
                    .slash("approve")
                    .subtitle("Insert a random friendly pull request approval")
                    .icon("check")
                    .keywords(["pull request", "pr", "lgtm", "review", "approve"])
                    .inserts_result(true),
                produce: pr_approved_message,
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
    fn pr_approved_message_is_from_the_list() {
        assert!(PR_APPROVED_MESSAGES.contains(&pr_approved_message()));
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
