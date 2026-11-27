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

//! The interpreter model the engines and the compile recipes share: the
//! execution policies an invocation is selected under ([`InvocationDialect`])
//! and the native protocols, tables and recipes that model C Tcl and Jim
//! internals. It holds no command spec: `tcl-registry` depends on it, and it
//! never depends on `tcl-registry`.

#![deny(missing_docs)]

pub mod invocation_dialect;
pub use invocation_dialect::{
    CompletionOptionsPolicy, EnsembleImplementationFamily, ExpressionQuoteControl,
    InvocationDialect, LogicalExpressionParseProvider, LogicalExpressionQuoteProvider,
    LogicalSourceWordProvider, NativeArgumentUsageHeader, NativeEnsembleBootstrap,
    NativeErrorArguments, NativeScriptParseTiming, NativeTailcallProtocol, TemplateParseErrors,
};

pub mod logical_core_simulation;
pub mod native_alias_registration;
pub mod native_array;
pub mod native_binary_usage;
pub mod native_binary_value;
pub mod native_boolean_truth;
pub mod native_bytecode;
pub mod native_clock;
pub mod native_command_creation;
pub mod native_command_literal;
pub mod native_compiled_variables;
pub mod native_compiler_pass;
pub mod native_each_loop;
pub mod native_encoding;
pub mod native_ensemble;
pub mod native_ensemble_rewrite;
/// Actual native command-source error-info formatting.
pub mod native_error_log;
pub mod native_error_objects;
pub mod native_eval_object;
pub mod native_expression_error;
pub mod native_index_lookup;
/// Independent native info version-reporting sources.
pub mod native_info_version;
/// Actual interpreter option declarations and two-stage native lookup.
pub mod native_interpreter_options;
/// Jim Enum and immediate-string original cache recipes.
pub mod native_jim_enum;
pub mod native_jim_local;
/// Selected variable-table hash and entry-order policy.
pub mod native_jim_lookup;
pub mod native_jim_switch;
pub mod native_lambda;
pub mod native_list_index;
pub mod native_namespace_code;
/// Authenticated native C namespace-name primary semantics.
pub mod native_namespace_name;
pub mod native_namespace_upvar;
pub mod native_numeric_error;
pub mod native_object_append;
pub mod native_object_vector;
pub mod native_procedure_body;
pub mod native_property_lookup;
pub mod native_regex;
pub mod native_return_options;
pub mod native_string_length;
pub mod native_string_materialization;
pub mod native_tcloo_method_cache;
pub mod native_tcloo_method_definition;
pub mod native_unset_options;
pub mod native_usage;
pub mod native_variable_declaration;
pub mod native_variable_destruction;
pub mod native_variable_name;
pub mod native_variable_table;
/// Actual direct C variable trace registration policy.
pub mod native_variable_trace;
/// Selected native and explicitly authored logical argument-count error metadata.
pub mod native_wrong_arguments;
pub mod source_file;

#[cfg(test)]
mod test_profiles;
