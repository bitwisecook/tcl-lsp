//! F5 BIG-IP query DSL (`dialects/f5/query`).
//!
//! The query DSL is a small jq-flavoured language for inspecting and
//! rewriting `bigip.conf` / SCF files; it powers the `f5 query` verb and is
//! the gating dependency for `f5 validate`.
//!
//! The crate provides the front-end — the [`lexer`] (tokeniser), the [`ast`]
//! node types, and the recursive-descent [`parser`]; the [`value`] model; the
//! [`projection`] layer over the typed `tcl-bigip` model; the [`eval`]uator +
//! builtin library; the output renderers; the field-value [`edit_plan`] engine
//! (`=` / `|=` / `+=` / `-=` assignments → in-place source rewrite); plus the
//! token-bounded [`rewrite`] rename engine (identity-field writes and the
//! `rename*` builtins).
//!
//! The pure engine is I/O-free (typed in → typed out); the prompts and
//! stdout shaping live in the `f5-cli` binary.

// The DSL models an unbounded integer; index / length / numeric casts
// between `usize`, `i64`, and `f64` are intentional and pervasive, as is
// type-dispatch with arms that happen to share a body.
#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::unnecessary_wraps
)]

pub mod ast;
pub mod builtins;
pub mod edit_plan;
pub mod errors;
pub mod eval;
pub mod examples;
pub mod grammar;
pub mod inputs;
pub mod jsonfmt;
pub mod lexer;
pub mod manual;
pub mod output;
pub mod parser;
// The module hosts both the pure `x509` surface and the network probes; the
// network parts inside are individually gated on `probes`.
#[cfg(any(feature = "x509", feature = "probes"))]
pub mod probes;
pub mod projection;
pub mod renderers;
pub mod rewrite;
pub mod runner;
pub mod special;
pub mod value;

pub use ast::{Expr, LitValue, PathStep, Program};
pub use edit_plan::{AppliedSource, EditOp, EditPlan, apply};
pub use errors::QueryError;
pub use eval::{EvalContext, Root, evaluate, evaluate_statement};
pub use inputs::InputSpec;
pub use lexer::{Token, TokenKind, tokenise};
pub use parser::parse_query;
pub use rewrite::{RenameReport, rename_object};
pub use runner::{QueryOptions, QueryResult, SideInput, run_query};
pub use value::{ObjectRef, PathRef, Value};
