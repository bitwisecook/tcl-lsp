//! Command registry — single source of truth for Tcl command metadata.
//!
//! This crate defines [`CommandSpec`], [`SubCommand`], and the
//! [`CommandRegistry`] lookup facade. Every consumer (compiler,
//! analyser, codegen, LSP, formatter) reads command metadata from
//! here. No command-specific knowledge is hardcoded elsewhere.
//!
//! ## Architecture
//!
//! - [`arg_role`] — what role each argument plays (`Body`, `Expr`, `VarWrite`, ...).
//! - [`arity`] — argument count constraints.
//! - [`traits`] — behavioural trait bitflags replacing ~35 boolean fields.
//! - [`dialects`] — compact dialect membership sets.
//! - [`types`] — Tcl internal representation types (`TclType`).
//! - [`spec`] — [`CommandSpec`] and [`SubCommand`] definitions.
//! - [`registry`] — [`CommandRegistry`] lookup facade.
//! - [`commands`] — one file per command, one directory per dialect.
//! - [`events`] — iRules event metadata (247 events, firing order, flow chains).
//! - [`profiles`] — F5 profile types (57 profiles), protocol namespaces (87),
//!   and stack modification commands.
//!
//! ## One file per command
//!
//! Each command lives in its own `.rs` file under `commands/<dialect>/`.
//! Command files return a [`CommandSpec`] with all metadata declared
//! inline. Use `..CommandSpec::DEFAULT` to fill unset fields.
//!
//! The crate has no `pyo3` dependency — Python bindings live in
//! `tcl-lsp-rust`.

#![deny(missing_docs)]

pub mod arg_role;
pub mod arity;
pub mod body_kind;
pub mod commands;
pub mod dialects;
pub mod events;
pub mod forms;
pub mod hooks;
pub mod hover;
pub mod profiles;
pub mod registry;
pub mod side_effects;
pub mod spec;
pub mod taint;
pub mod traits;
pub mod types;

/// Convenience prelude for command spec files.
///
/// `use crate::prelude::*;` in each command file brings in all the
/// types needed to construct a `CommandSpec`.
pub mod prelude {
    pub use crate::arg_role::ArgRole;
    pub use crate::arity::Arity;
    pub use crate::body_kind::BodyKind;
    pub use crate::dialects::DialectSet;
    pub use crate::forms::{CommandForm, SubCommandForm};
    pub use crate::hooks::{ArgTypeHint, CodegenHookId, LoweringHookId, WasmCodegenHookId};
    pub use crate::hover::{FormKind, FormSpec, HoverSnippet, OptionSpec};
    pub use crate::side_effects::{ConnectionSide, SideEffect, SideEffectTarget, StorageType};
    pub use crate::spec::{CommandSpec, SubCommand};
    pub use crate::traits::Traits;
    pub use crate::types::TclType;
}

// Re-export key types at crate root.
pub use arg_role::ArgRole;
pub use arity::Arity;
pub use body_kind::BodyKind;
pub use registry::{CommandRegistry, ResolvedTerminator};
pub use spec::{CommandSpec, SubCommand};
pub use traits::Traits;
pub use types::TclType;

/// Crate version string.
///
/// ```
/// assert!(!tcl_registry::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
