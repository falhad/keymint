use std::sync::Arc;

use async_trait::async_trait;

use crate::ai::prompts::{self, BUILTIN_PROMPTS};
use crate::ai::ChatMessage;
use crate::config::Config;
use crate::error::Result;
use crate::registry::{
    Action, Command, CommandContext, CommandInput, CommandManifest, CommandOutput, CommandProvider,
    InputKind,
};

pub struct AiProvider;

/// A prompt-driven command: system prompt + the user's text.
struct PromptCommand {
    manifest: CommandManifest,
    system: String,
}

#[async_trait]
impl Command for PromptCommand {
    fn manifest(&self) -> &CommandManifest {
        &self.manifest
    }

    async fn run(&self, ctx: &CommandContext, input: CommandInput) -> Result<CommandOutput> {
        let text = input.require_text()?.to_string();
        let system = prompts::render(&self.system, &[("language", &ctx.config.target_language)]);
        let out = ctx.ai.complete(&ctx.config, system, vec![ChatMessage::user(text)], &ctx.progress).await?;
        Ok(CommandOutput::Text { text: out })
    }
}

/// Opens the chat view. The conversation itself runs through the `chat` IPC call,
/// so this command only decides whether a first question was typed.
struct AskCommand {
    manifest: CommandManifest,
}

#[async_trait]
impl Command for AskCommand {
    fn manifest(&self) -> &CommandManifest {
        &self.manifest
    }

    async fn run(&self, _: &CommandContext, input: CommandInput) -> Result<CommandOutput> {
        let question = Some(input.text().trim().to_string()).filter(|q| !q.is_empty());
        Ok(CommandOutput::Action { action: Action::OpenChat { question } })
    }
}

impl CommandProvider for AiProvider {
    fn id(&self) -> &'static str {
        "ai"
    }

    fn commands(&self, cfg: &Config) -> Vec<Arc<dyn Command>> {
        let builtin = BUILTIN_PROMPTS.iter().map(|p| {
            Arc::new(PromptCommand {
                manifest: CommandManifest::new(format!("ai.{}", p.id), p.title, "AI")
                    .slash(p.slash)
                    .subtitle(p.subtitle)
                    .icon(p.icon)
                    .keywords(p.keywords.iter().copied())
                    .input(InputKind::Selection)
                    .replaces_selection(true),
                system: p.system.to_string(),
            }) as Arc<dyn Command>
        });
        let custom = cfg.custom_prompts.iter().map(|p| {
            Arc::new(PromptCommand {
                manifest: CommandManifest::new(format!("prompt.{}", p.id), &p.title, "Custom Prompt")
                    .slash(&p.id)
                    .subtitle("Custom AI prompt")
                    .icon("sparkles")
                    .input(InputKind::Selection)
                    .replaces_selection(true),
                system: prompts::wrap_custom(&p.prompt),
            }) as Arc<dyn Command>
        });
        let ask = std::iter::once(Arc::new(AskCommand {
            manifest: CommandManifest::new("ai.ask", "Ask AI", "AI")
                .slash("ask")
                .subtitle("Chat with your AI provider")
                .icon("message-circle")
                .keywords(["chat", "question", "assistant", "gpt", "talk"])
                .input(InputKind::OptionalText),
        }) as Arc<dyn Command>);
        ask.chain(builtin).chain(custom).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CustomPrompt;

    #[tokio::test]
    async fn ask_opens_chat_with_or_without_a_first_question() {
        use crate::ai::AiService;
        use crate::secrets::MemorySecretStore;
        let ctx = CommandContext {
            config: Config::default(),
            ai: Arc::new(AiService::new(Arc::new(MemorySecretStore::default())).unwrap()),
            progress: crate::ai::no_progress(),
        };
        let ask = AiProvider.commands(&Config::default()).into_iter().find(|c| c.manifest().id == "ai.ask").unwrap();
        let open = |q: Option<&str>| CommandOutput::Action { action: Action::OpenChat { question: q.map(str::to_string) } };
        assert_eq!(ask.run(&ctx, CommandInput::default()).await.unwrap(), open(None));
        assert_eq!(ask.run(&ctx, CommandInput { text: Some("  ".into()) }).await.unwrap(), open(None));
        assert_eq!(ask.run(&ctx, CommandInput { text: Some(" why? ".into()) }).await.unwrap(), open(Some("why?")));
    }

    #[test]
    fn exposes_builtin_and_custom_prompts() {
        let mut cfg = Config::default();
        cfg.custom_prompts.push(CustomPrompt { id: "pirate".into(), title: "Pirate".into(), prompt: "Arr".into() });
        let ids: Vec<_> = AiProvider.commands(&cfg).iter().map(|c| c.manifest().id.clone()).collect();
        for expected in ["ai.ask", "ai.ste", "ai.friendly", "ai.fix-grammar", "ai.summarize", "ai.translate", "prompt.pirate"] {
            assert!(ids.contains(&expected.to_string()), "missing {expected}");
        }
    }
}
