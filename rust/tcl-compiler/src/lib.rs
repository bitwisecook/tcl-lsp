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
//!
//! The crate has no `pyo3` dependency and no Python-compat concerns —
//! those belong in the `tcl-lsp-rust` binding crate. See
//! `docs/rust-rewrite.md` in the main repository for the full
//! migration strategy.

#![deny(missing_docs)]

pub mod expr_ast;
pub mod ir;

// Re-export key types for convenience.
pub use expr_ast::{BinOp, ExprNode, ExprOffset, UnaryOp};
pub use ir::{Module, Procedure, Script, Statement};

/// Crate version string, useful for migration diagnostics.
///
/// ```
/// assert!(!tcl_compiler::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
