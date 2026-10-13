//! Keymint core. Everything that is not windowing or OS integration lives here
//! so it can be tested headlessly and reused by other frontends (CLI, plugins).

pub mod ai;
pub mod commands;
pub mod config;
pub mod error;
pub mod history;
pub mod registry;
pub mod secrets;

pub use error::{Error, Result};
