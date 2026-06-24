//! Typed, backend-agnostic mid-IR ("BPF-IR") for the BPF-Tcl DSL, plus the
//! framework front-end that turns F5-inspired `when <EVENT> priority N { … }`
//! blocks into a priority-ordered bundle of typed eBPF programs.
//!
//! The Tcl front-end (lexer → IR → CFG) is *reused* from `tcl-compiler`; this
//! crate adds a strict typed lowering on top that rejects anything outside the
//! verifier-friendly DSL subset with span-anchored diagnostics. The resulting
//! [`ir::BpfProgram`] is the shared waist consumed by the codegen backends.
#![forbid(unsafe_code)]

pub mod diag;
pub mod event;
pub mod frontend;
pub mod ir;
pub mod lower;
pub mod profile;
pub mod template;
pub mod ty;
pub mod unroll;

pub use diag::{BpfDiag, BpfError};
pub use frontend::compile_module;
pub use ir::{BpfModule, BpfProgram, BpfProgramDecl, ProgType};
pub use lower::lower_function;
pub use profile::{BpfProfileSpec, FieldDef};
pub use template::TemplateDef;
pub use unroll::unroll_loops;
