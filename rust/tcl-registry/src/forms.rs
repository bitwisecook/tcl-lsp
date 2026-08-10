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

//! Command and subcommand form descriptors.
//!
//! A *form* is one concrete invocation shape of a command — for
//! example `dict create ...` and `dict for ...` are two forms of
//! `dict`, and `lset name index value` and `lset name idx1 idx2 value`
//! are two forms of `lset`. Forms carry their own arity, argument
//! roles, options, dialect applicability, and lowering / codegen hook
//! identifiers, so consumers can ask the registry "which form does
//! this call match?" rather than reimplementing the dispatch table.
//!
//! The thin [`crate::hover::FormSpec`] survives for hover and
//! completion text, where only the synopsis matters; the descriptors
//! defined here drive compiler routing in ARCH2.

use crate::arg_role::ArgRole;
use crate::arity::Arity;
use crate::completion::CompletionDescriptor;
use crate::dialects::DialectSet;
use crate::dispatch_stability::DispatchDependencyDescriptor;
use crate::hooks::{CodegenHookId, LoweringHookId, WasmCodegenHookId};
use crate::hover::OptionSpec;
use crate::semantic_operation::SemanticOperationId;
use crate::state_transition::StateTransitionDescriptor;
use crate::world_effect::WorldEffectDescriptor;

/// Self-contained metadata for one invocation form of a top-level
/// command.
///
/// `arity` and `arg_roles` are expressed against the argument list
/// *after* the command name, matching how
/// [`crate::CommandSpec::arg_roles`] is laid out.
#[derive(Debug, Clone, Copy)]
pub struct CommandForm {
    /// Form display name (`"default"`, `"with-amount"`, …) — used by
    /// completion and diagnostics.
    pub name: &'static str,

    /// Argument count constraint after the command name.
    pub arity: Arity,

    /// Static argument roles (no dynamic resolver — forms are
    /// already disambiguated by arity / option presence, so role
    /// resolution is positional).
    pub arg_roles: &'static [(u8, ArgRole)],

    /// Options recognised on this form.
    pub options: &'static [OptionSpec],

    /// Dialects in which this form applies. `None` = inherit from
    /// the parent [`crate::CommandSpec`] / [`crate::SubCommand`].
    pub dialects: Option<DialectSet>,

    /// Target-neutral semantic-operation override for this form.
    ///
    /// When present it wins over the resolved subcommand and command
    /// descriptors. `None` keeps the inherited semantic operation.
    pub semantic_operation: Option<SemanticOperationId>,

    /// Completion semantics specific to this form.
    ///
    /// When present, this takes precedence over a resolved subcommand's and
    /// parent command's completion descriptors. `None` inherits the more
    /// general descriptor.
    pub completion: Option<CompletionDescriptor>,

    /// Mutable Tcl-world effects specific to this concrete form.
    ///
    /// A matching form is applied after its subcommand and parent-command
    /// descriptors.  It extends them by default, or may explicitly replace
    /// them through [`crate::WorldEffectComposition`].  The result remains
    /// target-neutral and is resolved by the common registry projection before
    /// a compiler pass sees it.
    pub world_effects: Option<WorldEffectDescriptor>,

    /// Target-neutral command-binding and variable-cell transitions specific
    /// to this concrete invocation shape.
    pub state_transitions: Option<StateTransitionDescriptor>,

    /// Dispatch-stability dependency refinement for this invocation shape.
    /// Irreducible live-interpreter dependencies cannot be removed.
    pub dispatch_dependencies: Option<DispatchDependencyDescriptor>,

    /// Lowering hook identifier for this form.
    pub lowering_hook: Option<LoweringHookId>,

    /// `TclVM` bytecode codegen hook identifier for this form. See
    /// [`crate::CommandSpec::codegen_hook`].
    pub codegen_hook: Option<CodegenHookId>,

    /// WASM-runtime codegen hook identifier for this form. See
    /// [`crate::CommandSpec::wasm_codegen_hook`].
    pub wasm_codegen_hook: Option<WasmCodegenHookId>,
}

impl CommandForm {
    /// Default value for all fields — used with `..CommandForm::DEFAULT`.
    pub const DEFAULT: Self = Self {
        name: "",
        arity: Arity::any(),
        arg_roles: &[],
        options: &[],
        dialects: None,
        semantic_operation: None,
        completion: None,
        world_effects: None,
        state_transitions: None,
        dispatch_dependencies: None,
        lowering_hook: None,
        codegen_hook: None,
        wasm_codegen_hook: None,
    };
}

/// Self-contained metadata for one invocation form of a subcommand.
///
/// Identical layout to [`CommandForm`]; the type alias keeps the
/// call sites self-documenting at the point of use.
pub type SubCommandForm = CommandForm;
