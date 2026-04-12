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
//! - [`cfg`] — control-flow graph types: [`Block`](cfg::Block),
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
//! The crate has no `pyo3` dependency and no Python-compat concerns —
//! those belong in the `tcl-lsp-rust` binding crate. See
//! `docs/rust-rewrite.md` in the main repository for the full
//! migration strategy.

#![deny(missing_docs)]

pub mod alias;
pub mod analyses;
pub mod cfg;
pub mod cfg_builder;
pub mod codegen;
pub mod dataflow_graph;
pub mod def_use;
pub mod execution_intent;
pub mod expr_ast;
pub mod expr_parser;
pub mod gvn;
pub mod ir;
pub mod ir_helpers;
pub mod lowering;
pub mod lowering_hooks;
pub mod memory_ssa;
pub mod naming;
pub mod rendered_properties;
pub mod sccp;
pub mod segmenter;
pub mod shimmer;
pub mod side_effects;
pub mod ssa;
pub mod static_loops;
pub mod tcl_expr_eval;
pub mod types;
pub mod value_shapes;
pub mod var_refs;
pub mod var_scoping;

// Re-export key types for convenience.
pub use expr_ast::{BinOp, ExprNode, ExprOffset, UnaryOp};
pub use expr_parser::parse_expr;
pub use ir::{Module, Procedure, Script, Statement};
pub use tcl_expr_eval::{eval_tcl_expr, format_tcl_value, Env, EnvValue, TclValue};

/// Crate version string, useful for migration diagnostics.
///
/// ```
/// assert!(!tcl_compiler::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
