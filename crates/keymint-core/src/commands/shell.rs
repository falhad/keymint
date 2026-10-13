use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::AsyncReadExt;
use tokio::process::Command as Process;

use crate::config::{Config, ShellCommandDef};
use crate::error::{Error, Result};
use crate::registry::{
    Command, CommandContext, CommandInput, CommandManifest, CommandOutput, CommandProvider,
    InputKind,
};

pub struct ShellProvider;

/// Cap on captured output so a runaway program cannot exhaust memory.
const MAX_OUTPUT_BYTES: u64 = 256 * 1024;

pub fn build_args(args: &[String], text: &str) -> Vec<String> {
    args.iter().map(|a| a.replace("{{text}}", text)).collect()
}

struct ShellCommand {
    manifest: CommandManifest,
    def: ShellCommandDef,
}

#[async_trait]
impl Command for ShellCommand {
    fn manifest(&self) -> &CommandManifest {
        &self.manifest
    }

    async fn run(&self, _: &CommandContext, input: CommandInput) -> Result<CommandOutput> {
        let text = if self.def.uses_input { input.require_text()?.to_string() } else { String::new() };
        let args = build_args(&self.def.args, &text);

        // argv only, never `sh -c`: the selection cannot inject extra commands.
        let mut child = Process::new(&self.def.program)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| Error::Process(format!("cannot start \"{}\": {e}", self.def.program)))?;

        let mut stdout = child.stdout.take().expect("piped").take(MAX_OUTPUT_BYTES);
        let mut stderr = child.stderr.take().expect("piped").take(MAX_OUTPUT_BYTES);
        let timeout = Duration::from_secs(self.def.timeout_secs.max(1));

        let run = async {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let (r1, r2) = tokio::join!(stdout.read_to_end(&mut out), stderr.read_to_end(&mut err));
            r1?;
            r2?;
            let status = child.wait().await?;
            Ok::<_, std::io::Error>((status, out, err))
        };

        let (status, out, err) = tokio::time::timeout(timeout, run)
            .await
            .map_err(|_| Error::Process(format!("\"{}\" timed out after {}s", self.def.title, timeout.as_secs())))??;

        let out = String::from_utf8_lossy(&out).trim().to_string();
        let err = String::from_utf8_lossy(&err).trim().to_string();
        if !status.success() {
            let detail = if err.is_empty() { out } else { err };
            return Err(Error::Process(format!("exited with {status}: {detail}")));
        }
        Ok(if out.is_empty() { CommandOutput::Done } else { CommandOutput::Text { text: out } })
    }
}

impl CommandProvider for ShellProvider {
    fn id(&self) -> &'static str {
        "shell"
    }

    fn commands(&self, cfg: &Config) -> Vec<Arc<dyn Command>> {
        cfg.shell_commands
            .iter()
            .map(|d| {
                Arc::new(ShellCommand {
                    manifest: CommandManifest::new(format!("shell.{}", d.id), &d.title, "Shell")
                        .slash(&d.id)
                        .subtitle(format!("{} {}", d.program, d.args.join(" ")).trim().to_string())
                        .icon("terminal")
                        .input(if d.uses_input { InputKind::Selection } else { InputKind::None }),
                    def: d.clone(),
                }) as Arc<dyn Command>
            })
            .collect()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::ai::AiService;
    use crate::secrets::MemorySecretStore;

    fn ctx() -> CommandContext {
        CommandContext {
            config: Config::default(),
            ai: Arc::new(AiService::new(Arc::new(MemorySecretStore::default())).unwrap()),
            progress: crate::ai::no_progress(),
        }
    }

    fn cmd(program: &str, args: &[&str], uses_input: bool, timeout: u64) -> Arc<dyn Command> {
        let mut cfg = Config::default();
        cfg.shell_commands.push(ShellCommandDef {
            id: "t".into(),
            title: "T".into(),
            program: program.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            uses_input,
            timeout_secs: timeout,
        });
        ShellProvider.commands(&cfg).remove(0)
    }

    #[test]
    fn substitutes_per_argument() {
        assert_eq!(build_args(&["-a".into(), "x{{text}}y".into()], "Q"), vec!["-a", "xQy"]);
    }

    #[tokio::test]
    async fn selection_is_not_interpreted_by_a_shell() {
        let c = cmd("echo", &["{{text}}"], true, 5);
        let out = c.run(&ctx(), CommandInput { text: Some("a; echo pwned $(id)".into()) }).await.unwrap();
        assert_eq!(out, CommandOutput::Text { text: "a; echo pwned $(id)".into() });
    }

    #[tokio::test]
    async fn failure_reports_stderr() {
        let c = cmd("sh", &["-c", "echo boom >&2; exit 3"], false, 5);
        let err = c.run(&ctx(), CommandInput::default()).await.unwrap_err().to_string();
        assert!(err.contains("boom"), "{err}");
    }

    #[tokio::test]
    async fn times_out() {
        let c = cmd("sleep", &["5"], false, 1);
        let err = c.run(&ctx(), CommandInput::default()).await.unwrap_err().to_string();
        assert!(err.contains("timed out"), "{err}");
    }

    #[tokio::test]
    async fn missing_program_is_an_error() {
        let c = cmd("definitely-not-a-program-xyz", &[], false, 5);
        assert!(c.run(&ctx(), CommandInput::default()).await.is_err());
    }
}
