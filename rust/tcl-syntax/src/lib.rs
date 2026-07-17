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
//! It depends only on [`tcl_lexer`] (the canonical scanner +
//! [`tcl_lexer::backslash_subst`]) and so stays `wasm32`-clean. Everything is
//! `&str`-based — Tcl strings are UTF-8 internally, and byte consumers convert
//! at the call (the UTF-8-internal-rep invariant).
//!
//! Modules land per the phased extraction plan:
//! - [`backslash`] — the canonical `TclParseBackslash` decoder (done).
//! - [`list`] — `Tcl_SplitList` / `Tcl_Merge` (done).
//! - [`naming`] — variable/command name normalisation (done).
//! - [`expr`] — the `expr` AST + Pratt parser (done).
//! - [`mod@format`] — the `format` conversion-specifier grammar (done).
//! - [`number`] — the `TclParseNumber` numeric-literal grammar (done).
//! - [`glob`] — `Tcl_StringCaseMatch` (`string match`) (done).
//! - [`value`] — the `ValueOps` value seam + `ValueError` (the construct/inspect
//!   parallel of [`expr::ExprOps`]).
//! - `subst` — to follow.

pub mod backslash;
pub mod boolean;
pub mod case_list;
pub mod expr;
pub mod format;
pub mod glob;
pub mod list;
pub mod mro;
pub mod naming;
pub mod number;
pub mod number_tower;
pub mod scan;
pub mod switch_body;
pub mod value;
