//! Step debugger for the native Tcl VM.
//!
//! A [`controller::DebugController`] owns the breakpoints and the step-mode
//! state machine; a [`backend::DebugBackend`] is the debug engine that runs the
//! script and calls the controller at each source-line boundary. The native
//! [`backend::VmBackend`] runs the script on `tcl-vm`.
//!
//! This module is the portable, tested core — the controller's stop decision
//! logic, the shared [`types`], and the backend contract. The live VM backend
//! is gated on `tcl-vm` exposing a per-statement debug hook (source-line +
//! frame-level) and a bytecode-PC → source-line map; see [`backend`].

#![forbid(unsafe_code)]

pub mod backend;
pub mod controller;
pub mod dap;
pub mod types;

pub use backend::{DebugBackend, DebugError, VmBackend};
pub use controller::DebugController;
pub use types::{DebugAction, StackFrame, StepMode, StopEvent, StopReason, Variable, VariableKind};
