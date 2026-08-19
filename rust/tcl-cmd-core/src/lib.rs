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

//! Portable Tcl command logic, written once and run by every Rust runtime.
//!
//! Because the two runtimes have **opposite command ABIs** — the bytecode VM's
//! builtins return `Completion<Value>` (argv name-stripped), the WASM runtime's
//! set the result on the interp and return a bare `Code` (argv name-included) —
//! this crate does **not** export "command bodies". It exports **pure logic
//! helpers** generic over [`tcl_syntax::value::ValueOps`] that take
//! already-sliced arguments and return [`Result<V, CmdError>`](CmdError). Each
//! runtime keeps a thin per-command adapter that slices argv to its base, calls
//! the helper, and maps the result onto its protocol.
//!
//! The helpers never name a runtime `Completion`/`Code` or touch an interp, so
//! the crate depends only on `tcl-syntax` (the value seam + parse grammars) and
//! `tcl-platform` (the host-capability seam) — not on `tcl-bytecode` or
//! `tcl-runtime-api`.
//!
//! Module layout is **by concern**: pure value→value families flat at the top
//! ([`string`], [`path`], …), platform-backed families under [`platform`].

pub mod array;
pub mod binary;
pub mod clock;
pub mod dict;
pub mod ensemble;
pub mod error;
pub mod format;
pub mod index;
pub mod info;
pub mod list;
pub mod lsearch;
pub mod lseq;
pub mod lsort;
pub mod mathop;
pub mod namespace;
pub mod path;
pub mod platform;
pub mod prefix;
pub mod regex;
pub mod scan;
pub mod sort;
pub mod string;
pub mod string_is;
pub mod switch;
pub mod trace;
pub mod var;

pub use error::CmdError;
