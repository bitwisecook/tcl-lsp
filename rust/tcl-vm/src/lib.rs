//! `tcl-vm` — the native, idiomatic Rust bytecode VM ("TCLVM").
//!
//! Executes the bytecode artifact (`tcl-bytecode`) the compiler emits, over an
//! `Rc`-based dual-rep [`Value`] (Tcl shimmering without the wasm32 ABI layout)
//! and a non-recursive (NRE / trampoline) engine. It reuses `tcl-syntax` for the
//! expr tower / number / list semantics and satisfies the `tcl-runtime-api`
//! Family-B contract. `forbid(unsafe)` and compiler-optional: runtime `eval`
//! comes through an injected [`CompileService`].
//!
//! This is the M1 foundation — `set`/`puts`/`incr`/`expr`, arithmetic,
//! comparisons, and jumps over a flat global scope. Control flow / procs / catch
//! (M2) and the rest of the Family-B impls (M3+) build on this skeleton. See
//! `docs/design/common-runtime-emitter-architecture.md`.

pub mod error;
pub mod value;

mod cmd_array;
mod cmd_dict;
mod cmd_file;
mod cmd_format;
mod cmd_info;
mod cmd_list;
mod cmd_math;
mod cmd_namespace;
mod cmd_package;
mod cmd_string;
mod cmd_switch;
mod cmd_trace;
mod command;
mod exec;
mod expr;
mod frame;
mod interp;
mod subst;

pub use error::TclError;
pub use interp::Vm;
pub use value::Value;

pub use tcl_runtime_api::{Code, CompileError, CompileService, Completion};
