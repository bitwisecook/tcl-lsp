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

//! IR, CFG, SSA, and codegen for the Tcl compiler pipeline.
//!
//! Exported modules:
//!
//! - [`expr_ast`] — structured AST for `[expr]` expressions, including
//!   operator enums, expression nodes, and rendering.
//! - [`ir`] — intermediate representation: [`Statement`], [`Script`],
//!   [`Procedure`], [`Module`], and supporting types. Every node
//!   carries a [`Span`](tcl_lexer::Span) for position tracking.
//! - [`expr_parser`] — Pratt parser that converts expression tokens
//!   into [`ExprNode`] trees.
//! - [`naming`] — variable and command name normalisation utilities.
//! - [`cfg`](mod@cfg) — control-flow graph types: [`Block`](cfg::Block),
//!   [`Function`](cfg::Function), [`CfgModule`](cfg::CfgModule),
//!   [`Terminator`](cfg::Terminator), plus graph traversal utilities
//!   (predecessors, reachability, reverse post-order).
//! - [`ssa`] — SSA data structures ([`Phi`](ssa::Phi),
//!   [`SsaBlock`](ssa::SsaBlock), [`SsaFunction`](ssa::SsaFunction)),
//!   dominator algorithms, dominance frontier, phi placement, and
//!   variable definition extraction.
//! - [`codegen`] — bytecode assembly types: [`Op`](codegen::Op) (150+
//!   opcodes), [`Instruction`](codegen::Instruction),
//!   [`LiteralTable`](codegen::LiteralTable),
//!   [`LocalVarTable`](codegen::LocalVarTable),
//!   [`FunctionAsm`](codegen::FunctionAsm),
//!   [`ModuleAsm`](codegen::ModuleAsm), plus operator mapping and
//!   index parsing.  The emission context
//!   [`CodegenCtx`](codegen::CodegenCtx) and submodules
//!   [`helpers`](codegen::helpers), [`values`](codegen::values),
//!   [`expressions`](codegen::expressions) provide the codegen emitter
//!   foundation.  Statement emission
//!   ([`statements`](codegen::statements)), peephole optimisations
//!   ([`peephole`](codegen::peephole)), layout resolution
//!   ([`layout`](codegen::layout)), and disassembly formatting
//!   ([`format`](codegen::format)) complete the codegen pipeline.
//! - [`types`] — Tcl intrep type lattice:
//!   [`TclType`](types::TclType), [`TypeLattice`](types::TypeLattice),
//!   [`type_join`](types::type_join).
//! - [`analyses`] — analysis result types:
//!   [`LatticeValue`](analyses::LatticeValue),
//!   [`FunctionAnalysis`](analyses::FunctionAnalysis),
//!   [`ModuleAnalysis`](analyses::ModuleAnalysis),
//!   plus diagnostic types.
//! - [`ir_helpers`] — recursive IR/expression helpers:
//!   [`defs_from_ir_script`](ir_helpers::defs_from_ir_script),
//!   [`defs_from_expr`](ir_helpers::defs_from_expr) for extracting
//!   variable definitions from structured IR trees and expression
//!   command substitutions.
//! - [`var_refs`] — variable-reference scanning:
//!   [`VarReferenceScanner`](var_refs::VarReferenceScanner) for
//!   extracting variable reads from Tcl words/scripts, with LRU
//!   caching.
//! - [`cfg_builder`] — CFG construction from structured IR:
//!   [`build_cfg`](cfg_builder::build_cfg),
//!   [`build_cfg_function`](cfg_builder::build_cfg_function) for
//!   flattening `if`/`for`/`while`/`switch`/`catch`/`try` into
//!   basic blocks.
//!
//! - [`parsing`] — the parsing frontend.  Houses the canonical
//!   red-green concrete syntax tree under
//!   [`parsing::syntax`]; the position-independent
//!   green layer ([`parsing::syntax::green`])
//!   is the lossless representation the segmenter / lowering / formatter
//!   / tooling are meant to share.

#![deny(missing_docs)]

pub mod alias;
pub mod analyser;
pub mod analyses;
pub mod auto_path_eval;
pub mod backend_registry;
pub mod bounded_set;
pub mod cfg;
pub mod cfg_builder;
pub mod cfg_layout;
pub mod codegen;
pub mod command_binding;
pub mod common_aot_plan;
pub mod compilation_unit;
pub mod compiler_checks;
pub mod completion;
pub mod connection_scope;
pub mod const_subst;
pub mod dataflow_graph;
pub mod dead_stores;
pub mod def_use;
mod depth_guard;
pub mod dispatch_proof;
pub mod dynamic_names;
pub mod effect_ssa;
pub mod executable_ir;
pub mod execution_intent;
// The `expr` AST + Pratt parser now live in the shared `tcl-syntax` crate
// (consumed by both the compiler and the runtime port). Re-exported under the
// original module paths so the ~45 in-crate consumers (and the LSP bindings)
// are unchanged.
pub use tcl_syntax::expr::ast as expr_ast;
pub use tcl_syntax::expr::parser as expr_parser;
pub mod gvn;
pub mod head_identity;
pub mod inline_uplevel;
pub mod inlining;
pub mod interprocedural;
pub mod interval_bounds;
pub mod intervals;
pub mod ir;
pub mod ir_helpers;
pub mod irules_checks;
pub mod lambda_literal;
mod lattice_rebase;
pub use lattice_rebase::rebase_script;
pub mod loops;
pub mod lowering;
pub mod lowering_hooks;
pub mod memory_ssa;
pub mod mixed_region_plan;
pub mod native_integer_proof;
pub mod object_types;
// Name normalisation moved to the shared `tcl-syntax` crate; re-export so
// `crate::naming::*` keeps resolving across the compiler.
pub use tcl_syntax::naming;
pub mod optimiser;
pub mod parsing;
pub mod path_concat;
pub mod place;
pub mod place_bridge;
pub mod regex_source;
pub mod registry_invocation;
pub mod rendered_properties;
pub mod representation_plan;
pub mod scan_predicate;
pub mod sccp;
pub mod script_arg;
pub mod segmenter;
pub mod semantic_analysis;
pub mod semantic_optimisation;
pub mod shimmer;
pub mod side_effects;
pub mod signature_scan;
pub mod slot_allocation;
pub mod specialise_factories;
pub mod ssa;
pub mod state_ssa;
pub mod static_loops;
pub mod subst_nocommands;
pub mod taint;
pub mod taint_interproc;
pub mod target_contract;
pub mod tcl_expr_eval;
pub mod text;
pub mod type_infer;
pub mod types;
pub mod unit_scope;
pub mod uri_split;
pub mod value_provenance;
pub mod value_shapes;
pub mod var_escape;
pub mod var_observability;
pub mod var_refs;
pub mod var_resolve;
pub mod var_scoping;
pub mod world_state_ssa;

// Re-export key types for convenience.
pub use completion::{CompletionCodeLattice, CompletionObligations, MAX_EXACT_COMPLETION_CODES};
pub use expr_ast::{BinOp, ExprNode, ExprOffset, UnaryOp};
pub use expr_parser::parse_expr;
pub use ir::{Module, Procedure, Script, Statement};
pub use tcl_expr_eval::{Env, EnvValue, TclValue, eval_tcl_expr, format_tcl_value};

/// Crate version string.
///
/// ```
/// assert!(!tcl_compiler::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
