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

//! WASM codegen backend (compiler-explorer `wasm` / `wasmOptimised` views).
//!
//! This is a second codegen backend, separate from the Tcl-bytecode emitter
//! in [`crate::codegen`]: it lowers the IR/CFG directly to WebAssembly,
//! targeting a runtime ABI (imported `tcl_*` host functions over a shared
//! linear memory).
//!
//! This module is the WASM IR: value types, the opcode set, per-function /
//! per-module containers, LEB128 + section **binary encoding**
//! ([`WasmModule::to_bytes`]) and **WAT** text rendering
//! ([`WasmModule::to_wat`]). It has no dependency on the emitter and can be
//! built and tested in isolation.

#![allow(dead_code)]

pub mod backend;
mod encoding;
mod ir;

pub use backend::{
    RESERVED_DATA_BASE, wasm_codegen_compilation_unit, wasm_codegen_compilation_unit_based,
    wasm_codegen_module, wasm_codegen_module_based, wasm_codegen_module_standalone,
    wasm_codegen_module_standalone_init,
};
pub use ir::{
    SectionId, ValType, WasmData, WasmFunction, WasmImport, WasmInstruction, WasmModule, WasmOp,
};
