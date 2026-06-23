//! WASM codegen backend (compiler-explorer `wasm` / `wasmOptimised` views).
//!
//! This is a second codegen backend, separate from the Tcl-bytecode emitter
//! in [`crate::codegen`]: it lowers the IR/CFG directly to WebAssembly,
//! targeting a runtime ABI (imported `tcl_*` host functions over a shared
//! linear memory). Ported from `compiler/codegen/wasm/` in phases:
//!
//! - **Phase 1 (this module)** — the WASM IR: value types, the opcode set,
//!   per-function / per-module containers, LEB128 + section **binary
//!   encoding** ([`WasmModule::to_bytes`]) and **WAT** text rendering
//!   ([`WasmModule::to_wat`]). No dependency on the emitter; safe to build
//!   and test in isolation. Mirrors `compiler/codegen/wasm/_ir.py` +
//!   `_encoding.py`.
//! - Later phases add the emitter (`wasm_codegen_module`) and the explorer
//!   JSON serialiser; until then the explorer's `wasm` view stays `null`.

// The emitter that consumes these types lands in a follow-on phase; until
// then the IR surface is exercised only by unit tests.
#![allow(dead_code)]

pub mod backend;
mod encoding;
mod ir;

pub use backend::{
    RESERVED_DATA_BASE, wasm_codegen_module, wasm_codegen_module_based,
    wasm_codegen_module_standalone,
};
pub use ir::{
    SectionId, ValType, WasmData, WasmFunction, WasmImport, WasmInstruction, WasmModule, WasmOp,
};
