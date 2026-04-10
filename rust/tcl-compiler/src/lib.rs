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
//!   index parsing (chunk **C4**).
//! - [`types`] — Tcl intrep type lattice:
//!   [`TclType`](types::TclType), [`TypeLattice`](types::TypeLattice),
//!   [`type_join`](types::type_join) (chunk **C5**).
//! - [`analyses`] — analysis result types:
//!   [`LatticeValue`](analyses::LatticeValue),
//!   [`FunctionAnalysis`](analyses::FunctionAnalysis),
//!   [`ModuleAnalysis`](analyses::ModuleAnalysis),
//!   plus diagnostic types (chunk **C5**).
//!
//! The crate has no `pyo3` dependency and no Python-compat concerns —
//! those belong in the `tcl-lsp-rust` binding crate. See
//! `docs/rust-rewrite.md` in the main repository for the full
//! migration strategy.

#![deny(missing_docs)]

pub mod analyses;
pub mod cfg;
pub mod codegen;
pub mod expr_ast;
pub mod expr_parser;
pub mod ir;
pub mod naming;
pub mod ssa;
pub mod types;

// Re-export key types for convenience.
pub use expr_ast::{BinOp, ExprNode, ExprOffset, UnaryOp};
pub use expr_parser::parse_expr;
pub use ir::{Module, Procedure, Script, Statement};

/// Crate version string, useful for migration diagnostics.
///
/// ```
/// assert!(!tcl_compiler::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
