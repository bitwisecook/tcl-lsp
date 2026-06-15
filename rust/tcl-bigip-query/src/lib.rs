//! Faithful Rust port of the F5 BIG-IP query DSL (`dialects/f5/query`).
//!
//! The query DSL is a small jq-flavoured language for inspecting and
//! rewriting `bigip.conf` / SCF files; it powers the `f5 query` verb and is
//! the gating dependency for `f5 validate`.
//!
//! This crate is being ported in verifiable increments. **Landed so far:**
//! the front-end — the [`lexer`] (tokeniser), the [`ast`] node types, and
//! the recursive-descent [`parser`]. Still to come (tracked in
//! `docs/rust-cli-port.md`): the value model, the projection layer over the
//! typed `tcl-bigip` model, the evaluator, the builtin library, the output
//! renderers, and the mutation / edit-plan engine.
//!
//! The pure engine is I/O-free (typed in → typed out); the prompts and
//! stdout shaping live in the `f5-cli` binary.

pub mod ast;
pub mod errors;
pub mod jsonfmt;
pub mod lexer;
pub mod output;
pub mod parser;
pub mod value;

pub use ast::{Expr, LitValue, PathStep, Program};
pub use errors::QueryError;
pub use lexer::{Token, TokenKind, tokenise};
pub use parser::parse_query;
pub use value::{ObjectRef, PathRef, Value};
