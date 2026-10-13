use std::sync::Arc;

use async_trait::async_trait;

use crate::config::{Config, UrlCommandDef};
use crate::error::{Error, Result};
use crate::registry::{
    Action, Command, CommandContext, CommandInput, CommandManifest, CommandOutput, CommandProvider,
    InputKind,
};

pub struct UrlProvider;

/// Only web links may be opened; this keeps a config edit from launching
/// `file:`, `javascript:` or custom-protocol handlers.
pub fn check_template(template: &str) -> Result<()> {
    let lower = template.trim().to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        Ok(())
    } else {
        Err(Error::Config(format!("URL \"{template}\" must start with http:// or https://")))
    }
}

pub fn expand(template: &str, query: &str) -> String {
    template.replace("{{query}}", &urlencoding::encode(query.trim()))
}

struct UrlCommand {
    manifest: CommandManifest,
    template: String,
}

#[async_trait]
impl Command for UrlCommand {
    fn manifest(&self) -> &CommandManifest {
        &self.manifest
    }
    async fn run(&self, _: &CommandContext, input: CommandInput) -> Result<CommandOutput> {
        let query = if self.template.contains("{{query}}") { input.require_text()? } else { "" };
        let url = expand(&self.template, query);
        check_template(&url)?;
        Ok(CommandOutput::Action { action: Action::OpenUrl { url } })
    }
}

impl CommandProvider for UrlProvider {
    fn id(&self) -> &'static str {
        "url"
    }

    fn commands(&self, cfg: &Config) -> Vec<Arc<dyn Command>> {
        cfg.url_commands
            .iter()
            .filter(|u| check_template(&u.url).is_ok())
            .map(|u: &UrlCommandDef| {
                let needs_text = u.url.contains("{{query}}");
                Arc::new(UrlCommand {
                    manifest: CommandManifest::new(format!("url.{}", u.id), &u.title, "Web")
                        .slash(&u.id)
                        .subtitle(&u.url)
                        .icon("globe")
                        .input(if needs_text { InputKind::Text } else { InputKind::None }),
                    template: u.url.clone(),
                }) as Arc<dyn Command>
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query() {
        assert_eq!(expand("https://x.test/?q={{query}}", " a b&c "), "https://x.test/?q=a%20b%26c");
    }

    #[test]
    fn action_output_serializes_with_distinct_keys() {
        let out = CommandOutput::Action { action: Action::OpenUrl { url: "https://a.b".into() } };
        assert_eq!(
            serde_json::to_value(out).unwrap(),
            serde_json::json!({"type":"action","action":{"type":"open-url","url":"https://a.b"}})
        );
    }

    #[test]
    fn only_web_schemes() {
        assert!(check_template("https://a.b").is_ok());
        assert!(check_template("HTTP://a.b").is_ok());
        for bad in ["file:///etc/passwd", "javascript:1", "ftp://x", "tel:1"] {
            assert!(check_template(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn query_cannot_change_scheme() {
        // Template is fixed; a query starting with "javascript:" is just encoded text.
        let url = expand("https://x.test/?q={{query}}", "javascript:alert(1)");
        assert!(check_template(&url).is_ok());
        assert!(!url.contains(':') || url.starts_with("https:"));
    }
}
