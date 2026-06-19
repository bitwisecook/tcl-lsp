//! IR, CFG, SSA, and codegen for the Tcl compiler pipeline.
//!
//! This crate is populated chunk-by-chunk as the Python-to-Rust
//! migration progresses. Currently exported:
//!
//! - [`expr_ast`] — structured AST for `[expr]` expressions, including
//!   operator enums, expression nodes, and rendering (chunk **C0**).
//! - [`ir`] — intermediate representation: [`Statement`], [`Script`],
//!   [`Procedure`], [`Module`], and supporting types. Every node
//!   carries a [`Span`](tcl_lexer::Span) for position tracking
//!   (chunk **C0**).
//! - [`expr_parser`] — Pratt parser that converts expression tokens
//!   into [`ExprNode`] trees (chunk **C1**).
//! - [`naming`] — variable and command name normalisation utilities
//!   (chunk **C1**).
//! - [`cfg`](mod@cfg) — control-flow graph types: [`Block`](cfg::Block),
//!   [`Function`](cfg::Function), [`CfgModule`](cfg::CfgModule),
//!   [`Terminator`](cfg::Terminator), plus graph traversal utilities
//!   (predecessors, reachability, reverse post-order) (chunk **C2**).
//! - [`ssa`] — SSA data structures ([`Phi`](ssa::Phi),
//!   [`SsaBlock`](ssa::SsaBlock), [`SsaFunction`](ssa::SsaFunction)),
//!   dominator algorithms, dominance frontier, phi placement, and
//!   variable definition extraction (chunk **C3**).
//! - [`codegen`] — bytecode assembly types: [`Op`](codegen::Op) (150+
//!   opcodes), [`Instruction`](codegen::Instruction),
//!   [`LiteralTable`](codegen::LiteralTable),
//!   [`LocalVarTable`](codegen::LocalVarTable),
//!   [`FunctionAsm`](codegen::FunctionAsm),
//!   [`ModuleAsm`](codegen::ModuleAsm), plus operator mapping and
//!   index parsing (chunk **C4**).  The emission context
//!   [`CodegenCtx`](codegen::CodegenCtx) and submodules
//!   [`helpers`](codegen::helpers), [`values`](codegen::values),
//!   [`expressions`](codegen::expressions) provide the codegen emitter
//!   foundation (chunk **C11**).  Statement emission
//!   ([`statements`](codegen::statements)), peephole optimisations
//!   ([`peephole`](codegen::peephole)), layout resolution
//!   ([`layout`](codegen::layout)), and disassembly formatting
//!   ([`format`](codegen::format)) complete the codegen pipeline
//!   (chunks **C12–C15**).
//! - [`types`] — Tcl intrep type lattice:
//!   [`TclType`](types::TclType), [`TypeLattice`](types::TypeLattice),
//!   [`type_join`](types::type_join) (chunk **C5**).
//! - [`analyses`] — analysis result types:
//!   [`LatticeValue`](analyses::LatticeValue),
//!   [`FunctionAnalysis`](analyses::FunctionAnalysis),
//!   [`ModuleAnalysis`](analyses::ModuleAnalysis),
//!   plus diagnostic types (chunk **C5**).
//! - [`ir_helpers`] — recursive IR/expression helpers:
//!   [`defs_from_ir_script`](ir_helpers::defs_from_ir_script),
//!   [`defs_from_expr`](ir_helpers::defs_from_expr) for extracting
//!   variable definitions from structured IR trees and expression
//!   command substitutions (chunk **C7**).
//! - [`var_refs`] — variable-reference scanning:
//!   [`VarReferenceScanner`](var_refs::VarReferenceScanner) for
//!   extracting variable reads from Tcl words/scripts, with LRU
//!   caching (chunk **C6**).
//! - [`cfg_builder`] — CFG construction from structured IR:
//!   [`build_cfg`](cfg_builder::build_cfg),
//!   [`build_cfg_function`](cfg_builder::build_cfg_function) for
//!   flattening `if`/`for`/`while`/`switch`/`catch`/`try` into
//!   basic blocks (chunk **C7**).
//!
//! - [`parsing`] — the parsing frontend.  Houses the canonical
//!   red-green concrete syntax tree under
//!   [`parsing::syntax`]; the position-independent
//!   green layer ([`parsing::syntax::green`])
//!   is the lossless representation the segmenter / lowering / formatter
//!   / tooling are meant to share (`CST-PORT` / `SYNC-JUN06`).
//!
//! The crate has no `pyo3` dependency and no Python-compat concerns —
//! those belong in the `tcl-lsp-rust` binding crate. See
//! `docs/rust-rewrite.md` in the main repository for the full
//! migration strategy.

#![deny(missing_docs)]

pub mod alias;
pub mod analyser;
pub mod analyses;
pub mod auto_path_eval;
pub mod cfg;
pub mod cfg_builder;
pub mod cfg_layout;
pub mod codegen;
pub mod command_binding;
pub mod compilation_unit;
pub mod compiler_checks;
pub mod connection_scope;
pub mod dataflow_graph;
pub mod dead_stores;
pub mod def_use;
pub mod execution_intent;
// The `expr` AST + Pratt parser now live in the shared `tcl-syntax` crate
// (consumed by both the compiler and the runtime port). Re-exported under the
// original module paths so the ~45 in-crate consumers (and the LSP bindings)
// are unchanged.
pub use tcl_syntax::expr::ast as expr_ast;
pub use tcl_syntax::expr::parser as expr_parser;
pub mod gvn;
pub mod inline_uplevel;
pub mod inlining;
pub mod interprocedural;
pub mod interval_bounds;
pub mod intervals;
pub mod ir;
pub mod ir_helpers;
pub mod irules_checks;
mod lattice_rebase;
pub mod loops;
pub mod lowering;
pub mod lowering_hooks;
pub mod memory_ssa;
// Name normalisation moved to the shared `tcl-syntax` crate; re-export so
// `crate::naming::*` keeps resolving across the compiler.
pub use tcl_syntax::naming;
pub mod optimiser;
pub mod parsing;
pub mod path_concat;
pub mod place;
pub mod place_bridge;
pub mod rendered_properties;
pub mod scan_predicate;
pub mod sccp;
pub mod segmenter;
pub mod shimmer;
pub mod side_effects;
pub mod signature_scan;
pub mod specialise_factories;
pub mod ssa;
pub mod static_loops;
pub mod subst_nocommands;
pub mod taint;
pub mod taint_interproc;
pub mod tcl_expr_eval;
pub mod text;
pub mod type_infer;
pub mod types;
pub mod uri_split;
pub mod value_shapes;
pub mod var_escape;
pub mod var_observability;
pub mod var_refs;
pub mod var_resolve;
pub mod var_scoping;

// Re-export key types for convenience.
pub use expr_ast::{BinOp, ExprNode, ExprOffset, UnaryOp};
pub use expr_parser::parse_expr;
pub use ir::{Module, Procedure, Script, Statement};
pub use tcl_expr_eval::{Env, EnvValue, TclValue, eval_tcl_expr, format_tcl_value};

/// Crate version string, useful for migration diagnostics.
///
/// ```
/// assert!(!tcl_compiler::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
