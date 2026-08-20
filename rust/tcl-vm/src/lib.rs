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

//! `tcl-vm` — the native, idiomatic Rust bytecode VM ("TCLVM").
//!
//! Executes the bytecode artifact (`tcl-bytecode`) the compiler emits, over an
//! `Rc`-based dual-rep [`Value`] (Tcl shimmering without the wasm32 ABI layout)
//! and a non-recursive (NRE / trampoline) engine. It reuses `tcl-syntax` for the
//! expr tower / number / list semantics and satisfies the `tcl-runtime-api`
//! Family-B contract. `forbid(unsafe)` and compiler-optional: runtime `eval`
//! comes through an injected [`CompileService`].
//!
//! The foundation — `set`/`puts`/`incr`/`expr`, arithmetic,
//! comparisons, and jumps over a flat global scope. Control flow / procs / catch
//! and the rest of the Family-B impls build on this skeleton. See
//! `docs/design/common-runtime-emitter-architecture.md`.

pub mod debug;
pub mod embed;
pub mod error;
pub mod host_native;
// The `wasm32-unknown-unknown` half of the capability seam. Gated rather than
// always-compiled because its clock is a JavaScript import (`js-sys`), which is
// a dependency no native build should carry.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod host_wasm;
pub mod value;
mod value_ops;

mod cmd_array;
mod cmd_binary;
mod cmd_chan;
mod cmd_clock;
mod cmd_control;
mod cmd_coro;
mod cmd_dict;
mod cmd_event;
mod cmd_file;
mod cmd_format;
mod cmd_info;
mod cmd_list;
mod cmd_lseq;
mod cmd_math;
mod cmd_mathop;
mod cmd_namespace;
mod cmd_oo;
mod cmd_package;
mod cmd_prefix;
mod cmd_regexp;
mod cmd_string;
mod cmd_string_is;
mod cmd_switch;
mod cmd_thread;
mod cmd_trace;
mod cmd_try;
mod command;
mod exec;
mod expr;
mod frame;
mod interp;
mod subst;

pub use cmd_thread::{CompileFactory, ThreadedOutput};
pub use command::NativeCommand;
pub use debug::{DebugAction, DebugFrame, DebugHook, DebugSnapshot, DebugVar};
pub use embed::FunctionHandle;
pub use error::TclError;
pub use interp::Vm;
pub use value::Value;

pub use tcl_runtime_api::{Code, CompileError, CompileService, Completion};
// The Family-B role traits the VM satisfies, re-exported so a consumer can call
// the impls (a trait must be in scope to use its methods). More land as the VM
// grows.
pub use tcl_runtime_api::{Commands, Frames, Introspect, Namespaces, Traces, VarStore};
