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

//! Recursive IR and expression tree helpers for the CFG builder.
//!
//! These extract variable definitions from structured IR scripts and
//! expression trees. Used at `catch`/`try` merge points to
//! conservatively invalidate variables that may have been partially
//! modified before an exception, and at condition sites to track
//! definitions produced by command substitutions.

mod materialized_footprint;
pub(crate) use materialized_footprint::{
    command_possible_footprint_with_metadata_context, command_possible_reads_with_metadata_context,
    expression_possible_writes_with_metadata_context, footprint_command_words,
    immediate_same_frame_script_values, script_value_name_ownership_with_metadata_context,
    script_value_possible_writes_with_metadata_context,
};
#[cfg(test)]
use std::collections::BTreeSet;

#[cfg(test)]
use tcl_lexer::{LexerConfig, SourceMap};
#[cfg(test)]
use tcl_registry::{CommandRegistry, InvocationWord};

#[cfg(test)]
use crate::expr_ast::ExprNode;
pub(crate) use crate::ir::ExecutionNamespace;
#[cfg(test)]
use crate::ir::{CommandTokens, Statement};

mod command_words;
#[cfg(test)]
mod condition_out_vars;
mod evaluated_substitutions;
mod nested_bodies;
mod substitution_surfaces;
mod variable_effects;

use command_words::command_words;
#[cfg(test)]
use command_words::command_written_spans;
pub(crate) use command_words::{CommandWord, tokenise_command_words};
#[cfg(test)]
pub(crate) use condition_out_vars::condition_command_out_vars;
#[cfg(test)]
use condition_out_vars::tokenise_to_words;
#[cfg(test)]
pub(crate) use evaluated_substitutions::command_substitutions_in_surfaces_with_replay;
pub(crate) use evaluated_substitutions::{
    EvaluatedCommandSubstitutions, ResolvedEmbeddedHead, evaluated_command_substitutions,
    evaluated_command_substitutions_with_heads,
    evaluated_command_substitutions_with_heads_and_metadata_context,
    evaluated_command_substitutions_with_metadata_context,
    evaluated_command_substitutions_with_replay,
    evaluated_command_substitutions_with_replay_and_metadata_context,
    expression_command_substitutions,
    expression_command_substitutions_with_replay_and_metadata_context,
    in_frame_expression_arg_indices,
};
pub(crate) use nested_bodies::{
    nested_bodies, nested_execution_bodies, requires_runtime_command_namespace,
};
use substitution_surfaces::collect_expr_command_surface_refs;
pub use substitution_surfaces::expr_has_command;
pub(crate) use substitution_surfaces::{
    collect_expr_commands, evaluated_command_substitution_surfaces,
};
#[cfg(test)]
pub(crate) use variable_effects::variable_write_effects_from_commands_with_metadata_context;
pub(crate) use variable_effects::{
    VariableWriteEffects, condition_substituted_reads, original_invocation_possible_name_writes,
    original_statement_possible_name_writes, standalone_typed_statement_writes_are_covered,
    statement_substituted_reads, structural_variable_write_projection,
    variable_read_effects_from_commands, variable_write_effects_from_commands,
};
pub use variable_effects::{defs_from_expr, defs_from_ir_script};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_recovered_source_purpose_tests;

#[cfg(test)]
mod original_written_source_tests;
