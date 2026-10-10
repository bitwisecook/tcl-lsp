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

//! Shared Tcl parse-tree + byte-exact-semantics layer.
//!
//! This is the convergence crate: the single home for the pure
//! Tcl parsing primitives that both the LSP/compiler suite and the WASM runtime
//! need, so neither side re-derives (and drifts) its own copy.
//!
//! The dependency-light owners remain `wasm32`-clean. Native resident strings
//! and names retain exact length-delimited bytes, including modified UTF-8,
//! surrogates and non-Unicode byte units. Analytical Unicode views are checked
//! projections and do not replace native identity or source provenance.
//!
//! What each module owns:
//! - [`backslash`] — the canonical `TclParseBackslash` decoder.
//! - [`boolean`] — the two C Tcl boolean acceptors (`ParseBoolean` and the
//!   boolean-context one).
//! - [`case_list`] — splitting a `{pattern body …}` clause list into clauses.
//! - [`event_handler`] — the `when EVENT ?priority N? { … }` boundary grammar.
//! - [`expr`] — the `expr` AST, Pratt parser, and shared evaluator walk.
//! - [`formal_params`] — strict `proc` / method / lambda formal-list parsing.
//! - [`mod@format`] — the `format` conversion-specifier grammar.
//! - [`glob`] — `Tcl_StringCaseMatch` (`string match`).
//! - [`list`] — `Tcl_SplitList` / `Tcl_Merge`.
//! - [`mro`] — `TclOO` method resolution order.
//! - [`naming`] — purpose-specific name inputs and constructed byte identities.
//! - [`native_string`] — native resident-string and pure-byte-array recipes.
//! - [`number`] — the `TclParseNumber` numeric-literal grammar.
//! - [`number_tower`] — the integer operator semantics of `tclExecute.c`,
//!   generic over the big-integer backend.
//! - [`scan`] — the `scan` conversion-specifier grammar.
//! - [`switch_body`] — tokenising a `switch` braced pattern/body list.
//! - [`value`] — the `ValueOps` value seam + `ValueError` (the construct/inspect
//!   parallel of [`expr::ExprOps`]).
//! - [`word_rules`] — what a written word means as a value in a dialect.
//!
//! The conformance vector tables live here too, so every consumer pins to the
//! same rows: [`release_expectations`] (the per-release expectation columns),
//! [`var_conformance`] (variable lookup/creation), [`ns_op_conformance`]
//! (namespace operations), and [`vector_ops`] (the shared row syntax).

pub mod backslash;
pub mod boolean;
pub mod case_list;
pub mod event_handler;
pub mod execution_conformance;
pub mod expr;
pub mod formal_params;
pub mod format;
pub mod glob;
pub mod jim_script_layout;
pub mod jim_script_objects;
pub mod list;
pub mod list_result;
pub mod logical_numeric_simulation;
pub mod mro;
pub mod naming;
/// Native array-search handle conversion and failure presentation.
pub mod native_array_search;
pub mod native_bytecode;
pub mod native_compiled_index;
pub mod native_end_offset;
pub mod native_equality;
pub mod native_frame_error;
pub mod native_glob;
pub mod native_instruction_name;
pub mod native_jim_index;
pub mod native_jim_local;
pub mod native_jim_lookup;
pub mod native_jim_substitution;
pub mod native_jim_switch;
/// Physical C namespace-name primary and lifecycle recipes.
pub mod native_namespace_name;
pub mod native_object;
pub mod native_object_append;
pub mod native_parse_context;
pub mod native_regex;
pub mod native_string;
pub mod native_string_trim;
pub mod native_tcl_case;
pub mod native_tcl_utf;
pub mod native_variable_name;
pub mod native_variable_words;
pub mod ns_op_conformance;
pub mod number;
pub mod number_tower;
pub mod raw_string;
pub mod release_expectations;
pub mod scalar_getter;
pub mod scan;
pub mod switch_body;
pub mod value;
pub mod var_conformance;
pub mod vector_ops;
pub mod word_rules;
