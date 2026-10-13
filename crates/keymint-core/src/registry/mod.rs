//! The command registry: the single place every action in Keymint lives.

pub mod frecency;
pub mod manifest;
pub mod search;

use std::collections::HashMap;
use std::sync::Arc;

pub use frecency::Frecency;
pub use manifest::{
    Action, Command, CommandContext, CommandInput, CommandManifest, CommandOutput, CommandProvider,
    InputKind,
};

use crate::config::Config;
use crate::error::{Error, Result};

/// Result of a `/` query.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct SlashResult {
    pub items: Vec<CommandManifest>,
    /// Text typed after a complete slash command name, e.g. `fix this` in `/ste fix this`.
    pub args: String,
}

pub struct Registry {
    commands: Vec<Arc<dyn Command>>,
    by_id: HashMap<String, usize>,
    /// Lowercase slash name -> command index. First registration wins.
    by_slash: HashMap<String, usize>,
    /// Manifests with user shortcuts applied, parallel to `commands`.
    manifests: Vec<CommandManifest>,
}

impl Registry {
    /// Collect commands from every provider. A duplicate id keeps the first
    /// registration so a provider can never silently shadow a built-in.
    pub fn build(providers: &[Box<dyn CommandProvider>], cfg: &Config) -> Self {
        let mut commands: Vec<Arc<dyn Command>> = Vec::new();
        let mut by_id = HashMap::new();
        let mut by_slash = HashMap::new();
        let mut manifests = Vec::new();
        for provider in providers {
            for cmd in provider.commands(cfg) {
                let mut manifest = cmd.manifest().clone();
                if by_id.contains_key(&manifest.id) {
                    continue;
                }
                manifest.shortcut = cfg.command_shortcuts.get(&manifest.id).cloned();
                if let Some(slash) = &manifest.slash {
                    // A taken name just means this command has no slash alias.
                    if by_slash.contains_key(slash) {
                        manifest.slash = None;
                    } else {
                        by_slash.insert(slash.clone(), commands.len());
                    }
                }
                by_id.insert(manifest.id.clone(), commands.len());
                manifests.push(manifest);
                commands.push(cmd);
            }
        }
        Self { commands, by_id, by_slash, manifests }
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn manifests(&self) -> &[CommandManifest] {
        &self.manifests
    }

    pub fn manifest(&self, id: &str) -> Option<&CommandManifest> {
        self.by_id.get(id).map(|&i| &self.manifests[i])
    }

    pub fn get(&self, id: &str) -> Result<Arc<dyn Command>> {
        self.by_id
            .get(id)
            .map(|&i| self.commands[i].clone())
            .ok_or_else(|| Error::UnknownCommand(id.to_string()))
    }

    /// `/` queries: list commands by slash-name prefix, or — once a full name
    /// is followed by a space — resolve it and return the rest as arguments.
    pub fn search_slash(&self, query: &str) -> SlashResult {
        let Some(rest) = query.trim_start().strip_prefix('/') else { return SlashResult::default() };
        match rest.split_once(char::is_whitespace) {
            None => {
                let prefix = rest.to_lowercase();
                let mut hits: Vec<(&String, &usize)> =
                    self.by_slash.iter().filter(|(name, _)| name.starts_with(&prefix)).collect();
                hits.sort_by(|a, b| a.0.len().cmp(&b.0.len()).then_with(|| a.0.cmp(b.0)));
                SlashResult {
                    items: hits.into_iter().map(|(_, &i)| self.manifests[i].clone()).collect(),
                    args: String::new(),
                }
            }
            Some((name, args)) => match self.by_slash.get(&name.to_lowercase()) {
                Some(&i) => SlashResult { items: vec![self.manifests[i].clone()], args: args.trim().to_string() },
                None => SlashResult::default(),
            },
        }
    }

    /// Ranked search. An empty query lists everything by usage, then title.
    pub fn search(&self, query: &str, usage: &Frecency, limit: usize) -> Vec<CommandManifest> {
        let query = query.trim();
        let mut scored: Vec<(u32, &CommandManifest)> = if query.is_empty() {
            self.manifests.iter().map(|m| (usage.score(&m.id), m)).collect()
        } else {
            let mut matcher = search::new_matcher();
            let pattern = search::parse_pattern(query);
            let mut buf = Vec::new();
            self.manifests
                .iter()
                .filter_map(|m| {
                    search::match_score(&mut matcher, &pattern, m, &mut buf)
                        .map(|s| (s + usage.score(&m.id), m))
                })
                .collect()
        };
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.title.cmp(&b.1.title)));
        scored.into_iter().take(limit).map(|(_, m)| m.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct Dummy(CommandManifest);

    #[async_trait]
    impl Command for Dummy {
        fn manifest(&self) -> &CommandManifest {
            &self.0
        }
        async fn run(&self, _: &CommandContext, _: CommandInput) -> Result<CommandOutput> {
            Ok(CommandOutput::Done)
        }
    }

    struct Fixed(&'static str, Vec<CommandManifest>);

    impl CommandProvider for Fixed {
        fn id(&self) -> &'static str {
            self.0
        }
        fn commands(&self, _: &Config) -> Vec<Arc<dyn Command>> {
            self.1.iter().cloned().map(|m| Arc::new(Dummy(m)) as Arc<dyn Command>).collect()
        }
    }

    fn registry() -> Registry {
        let p: Vec<Box<dyn CommandProvider>> = vec![Box::new(Fixed(
            "t",
            vec![
                CommandManifest::new("ai.fix-grammar", "Fix Grammar", "AI").keywords(["spelling"]),
                CommandManifest::new("ai.summarize", "Summarize", "AI"),
                CommandManifest::new("app.settings", "Open Settings", "App"),
                CommandManifest::new("ai.fix-grammar", "Shadow", "AI"),
            ],
        ))];
        Registry::build(&p, &Config::default())
    }

    #[test]
    fn duplicate_ids_keep_first() {
        let r = registry();
        assert_eq!(r.len(), 3);
        assert_eq!(r.manifest("ai.fix-grammar").unwrap().title, "Fix Grammar");
    }

    #[test]
    fn fuzzy_title_and_keyword_matching() {
        let r = registry();
        let u = Frecency::default();
        assert_eq!(r.search("grmr", &u, 10)[0].id, "ai.fix-grammar");
        assert_eq!(r.search("spelling", &u, 10)[0].id, "ai.fix-grammar");
        assert!(r.search("zzzzqq", &u, 10).is_empty());
    }

    #[test]
    fn usage_breaks_ties_and_orders_empty_query() {
        let r = registry();
        let mut u = Frecency::default();
        assert_eq!(r.search("", &u, 10)[0].id, "ai.fix-grammar"); // alphabetical
        u.record("app.settings");
        assert_eq!(r.search("", &u, 10)[0].id, "app.settings");
    }

    #[test]
    fn user_shortcuts_are_applied() {
        let p: Vec<Box<dyn CommandProvider>> =
            vec![Box::new(Fixed("t", vec![CommandManifest::new("a.b", "B", "A")]))];
        let mut cfg = Config::default();
        cfg.command_shortcuts.insert("a.b".into(), "Ctrl+Alt+B".into());
        let r = Registry::build(&p, &cfg);
        assert_eq!(r.manifest("a.b").unwrap().shortcut.as_deref(), Some("Ctrl+Alt+B"));
    }

    fn slash_registry() -> Registry {
        let p: Vec<Box<dyn CommandProvider>> = vec![Box::new(Fixed(
            "t",
            vec![
                CommandManifest::new("ai.ste", "STE", "AI").slash("ste"),
                CommandManifest::new("ai.summarize", "Summarize", "AI").slash("Summarize"),
                CommandManifest::new("ai.ask", "Ask AI", "AI").slash("ask"),
                CommandManifest::new("x.other", "Other", "X").slash("ste"),
                CommandManifest::new("x.none", "No slash", "X"),
            ],
        ))];
        Registry::build(&p, &Config::default())
    }

    fn slugs(r: &SlashResult) -> Vec<&str> {
        r.items.iter().map(|m| m.id.as_str()).collect()
    }

    #[test]
    fn slash_lists_everything_and_filters_by_prefix() {
        let r = slash_registry();
        assert_eq!(slugs(&r.search_slash("/")), ["ai.ask", "ai.ste", "ai.summarize"]);
        assert_eq!(slugs(&r.search_slash("/s")), ["ai.ste", "ai.summarize"]);
        assert_eq!(slugs(&r.search_slash("/SU")), ["ai.summarize"], "case-insensitive");
        assert!(r.search_slash("/zzz").items.is_empty());
        assert!(r.search_slash("no slash").items.is_empty());
    }

    #[test]
    fn slash_with_a_space_resolves_exactly_and_returns_arguments() {
        let r = slash_registry();
        let hit = r.search_slash("/ste  make this friendly ");
        assert_eq!(slugs(&hit), ["ai.ste"]);
        assert_eq!(hit.args, "make this friendly");
        assert!(r.search_slash("/st text").items.is_empty(), "partial names do not resolve once args start");
        assert_eq!(r.search_slash("/ask ").args, "");
    }

    #[test]
    fn duplicate_slash_names_keep_the_first_command() {
        let r = slash_registry();
        assert_eq!(slugs(&r.search_slash("/ste x")), ["ai.ste"]);
        assert_eq!(r.manifest("x.other").unwrap().slash, None);
    }

    #[test]
    fn unknown_command_is_an_error() {
        assert!(matches!(registry().get("nope"), Err(Error::UnknownCommand(_))));
    }
}
