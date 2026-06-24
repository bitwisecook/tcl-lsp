//! Code generation for the BPF-Tcl DSL.
//!
//! Consumes the typed [`bpf_tcl_ir::BpfProgram`] and emits eBPF bytecode
//! directly (its own 64-bit instruction encoder — no LLVM in the compile path).
//! A WASM backend off the same IR follows as a later milestone.
#![forbid(unsafe_code)]

pub mod ebpf;
