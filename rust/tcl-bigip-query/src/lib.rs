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

//! F5 BIG-IP query DSL.
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

pub mod architecture;
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

pub use architecture::build_architecture;
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
