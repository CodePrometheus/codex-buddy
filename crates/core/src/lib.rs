//! Core logic for codex-buddy: a macOS multi-account switcher / parallel runner for the
//! Codex CLI. No CLI or interactive IO here; reused by the `codex-buddy` binary.

pub mod auth;
pub mod config_check;
pub mod doctor;
pub mod error;
pub mod init;
pub mod layout;
pub mod ops;
pub mod paths;
pub mod registry;
pub mod usage;

#[cfg(test)]
pub(crate) mod testutil;
