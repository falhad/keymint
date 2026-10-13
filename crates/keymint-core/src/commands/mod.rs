//! Built-in command providers. A future plugin system adds more providers here
//! (or registers external ones) without touching the registry.

pub mod ai;
pub mod builtin;
pub mod shell;
pub mod url;

use crate::registry::CommandProvider;

pub fn default_providers() -> Vec<Box<dyn CommandProvider>> {
    vec![
        Box::new(builtin::BuiltinProvider),
        Box::new(ai::AiProvider),
        Box::new(shell::ShellProvider),
        Box::new(url::UrlProvider),
    ]
}
