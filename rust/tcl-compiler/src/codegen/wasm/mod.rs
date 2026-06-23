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
    RESERVED_DATA_BASE, wasm_codegen_module, wasm_codegen_module_based,
    wasm_codegen_module_standalone, wasm_codegen_module_standalone_init,
};
pub use ir::{
    SectionId, ValType, WasmData, WasmFunction, WasmImport, WasmInstruction, WasmModule, WasmOp,
};
