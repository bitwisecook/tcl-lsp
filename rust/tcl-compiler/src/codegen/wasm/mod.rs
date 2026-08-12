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

//! Canonical Tcl-to-WebAssembly compilation and its target IR.
//!
//! [`compile_wasm`] is the sole production entry point. It consumes a complete
//! compiler unit and targets the shared `runtime/rust` ABI. Binary encoding,
//! WAT rendering, Explorer views, runtime linking, and standalone packaging
//! all consume the same [`WasmModule`].

#![allow(dead_code)]

mod backend;
mod encoding;
mod ir;
mod pipeline;
mod semantic_plan;

pub use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
pub use backend::RESERVED_DATA_BASE;
pub use ir::{
    SectionId, ValType, WasmData, WasmFunction, WasmImport, WasmInstruction, WasmModule, WasmOp,
};
pub use pipeline::WasmSemanticDecline;
pub use pipeline::{
    WasmCodegenPlan, WasmCompilation, WasmCompileOptions, WasmExecutableAvailabilityDecline,
    WasmNativeI64AddSelection, WasmPackagingConstraint, compile_wasm,
};
pub use semantic_plan::WasmExecutableInvokeDecline;
