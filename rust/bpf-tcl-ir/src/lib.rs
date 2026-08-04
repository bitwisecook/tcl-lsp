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

//! Typed, backend-agnostic mid-IR ("BPF-IR") for the BPF-Tcl DSL, plus the
//! framework front-end that turns F5-inspired `when <EVENT> priority N { … }`
//! blocks into a priority-ordered bundle of typed eBPF programs.
//!
//! The Tcl front-end (lexer → IR → CFG) is *reused* from `tcl-compiler`; this
//! crate adds a strict typed lowering on top that rejects anything outside the
//! verifier-friendly DSL subset with span-anchored diagnostics. The resulting
//! [`ir::BpfProgram`] is the shared waist consumed by the codegen backends.
#![forbid(unsafe_code)]

pub mod alloc;
pub mod capability;
pub mod deploy;
pub mod diag;
pub mod event;
pub mod frontend;
pub mod ir;
pub mod lower;
pub mod profile;
pub mod template;
pub mod ty;
pub mod unroll;

pub use capability::CapabilityPolicy;
pub use deploy::collect_attach;
pub use diag::{BpfDiag, BpfError};
pub use frontend::compile_module;
pub use ir::{
    AttachSpec, BpfModule, BpfProgram, BpfProgramDecl, MapConcurrency, MapDef, MapKind, OobAction,
    ProgType,
};
pub use lower::lower_function;
pub use profile::{BpfProfileSpec, FieldDef};
pub use template::TemplateDef;
pub use ty::ByteOrder;
pub use unroll::unroll_loops;
