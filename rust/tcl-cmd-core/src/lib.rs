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
pub mod dict;
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
pub mod regex;
pub mod scan;
pub mod sort;
pub mod string;
pub mod string_is;
pub mod switch;
pub mod var;

pub use error::CmdError;
