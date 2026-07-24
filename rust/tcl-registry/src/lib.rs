// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
//! - [`events`] — iRules event metadata (176 events, firing order, flow chains).
//! - [`profiles`] — F5 profile types (65 profiles), protocol namespaces (113),
//!   and stack modification commands.
//! - [`special_vars`] — dialect-versioned interpreter-provided variables
//!   (`auto_path`, `env`, `tcl_platform`, the iRules `static::` namespace).
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
pub mod base_objects;
pub mod bigip;
pub mod body_kind;
pub mod byte_array_effect;
pub mod cache;
pub mod clause_shape;
pub mod command_snapshot;
pub mod command_table;
pub mod commands;
pub mod const_fold;
pub mod definer;
pub mod dialects;
mod event_descriptions;
pub mod event_facts;
pub mod events;
pub mod forms;
pub mod hooks;
pub mod hover;
pub mod patterns;
pub mod profile_defaults;
pub mod profile_queries;
pub mod profiles;
pub mod registry;
pub mod scoped;
pub mod side_effects;
pub mod snapshot;
pub mod spec;
pub mod special_vars;
pub mod stub_overlay;
pub mod symbol_def;
pub mod taint;
pub mod traits;
pub mod types;
pub mod version;

/// Convenience prelude for command spec files.
///
/// `use crate::prelude::*;` in each command file brings in all the
/// types needed to construct a `CommandSpec`.
pub mod prelude {
    pub use crate::arg_role::{AppendedArity, ArgRole};
    pub use crate::arity::Arity;
    pub use crate::body_kind::BodyKind;
    pub use crate::byte_array_effect::ByteArrayEffect;
    pub use crate::clause_shape::{ClauseShapeChecker, ClauseShapeError};
    pub use crate::command_table::CommandTableEffect;
    pub use crate::definer::{
        DefinerFamily, DefinitionBodyGrammar, MemberKind, MemberRefKind, MemberSpec,
    };
    pub use crate::dialects::DialectSet;
    pub use crate::events::EventRequires;
    pub use crate::forms::{CommandForm, SubCommandForm};
    pub use crate::hooks::{
        AnalyserHookId, ArgTypeHint, CodegenHookId, LoweringHookId, TclVersion,
        VersionedConstFoldFn, WasmCodegenHookId,
    };
    pub use crate::hover::{
        ArgValue, FormKind, FormSpec, HoverSnippet, IntegerDomain, OptionArg, OptionArity,
        OptionSpec, OptionValue, OptionValueHook, OptionValueOutcome,
    };
    pub use crate::patterns::{FormatType, PatternType};
    pub use crate::scoped::{ScopedCommand, ScopedCommandEnv};
    pub use crate::side_effects::{ConnectionSide, SideEffect, SideEffectTarget, StorageType};
    pub use crate::spec::{
        BytePayloadSpec, CaseListSpec, CommandSpec, ContextGate, DefaultFormFirstWord,
        ObjectClassSpec, SubCommand, SubSubCommand,
    };
    pub use crate::symbol_def::{DefinedSymbolKind, SymbolDef};
    pub use crate::taint::{SetterConstraint, TaintColour};
    pub use crate::traits::Traits;
    pub use crate::types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
}

// Re-export key types at crate root.
pub use arg_role::{AppendedArity, ArgRole};
pub use arity::Arity;
pub use bigip::{BigipObjectSpec, BigipPropertySpec, BigipRegistry, ValueKind};
pub use body_kind::BodyKind;
pub use byte_array_effect::ByteArrayEffect;
pub use cache::{registry_for_dialect, registry_for_profile};
pub use clause_shape::{ClauseShapeChecker, ClauseShapeError};
pub use command_table::CommandTableEffect;
pub use dialects::{
    DETECT_SCAN_BYTES, KNOWN_DIALECTS, available_dialects, detect_dialect,
    detect_dialect_directive, detect_dialect_from_source, dialect_from_extension,
};
pub use hover::ArgValue;
pub use patterns::{FormatType, PatternType};
pub use profile_queries::{ProfileQueries, VendorSurface};
pub use registry::{CommandRegistry, ResolvedCall, ResolvedTerminator};
pub use spec::{
    BytePayloadSpec, CaseListSpec, CommandSpec, ContextGate, DefaultFormFirstWord, ObjectClassSpec,
    SubCommand, SubSubCommand,
};
pub use special_vars::{
    SPECIAL_VARS, SpecialVarKey, SpecialVarKind, SpecialVarSpec, VarAccess, VarOrigin,
    is_externally_read, is_special_var, special_var, special_var_in_dialect,
    special_var_read_taint, special_var_write_effect, special_vars_for_dialect,
};
pub use symbol_def::{DefinedSymbolKind, SymbolDef};
pub use taint::{SetterConstraint, TaintColour};
pub use traits::Traits;
pub use types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};

/// Crate version string.
///
/// ```
/// assert!(!tcl_registry::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
