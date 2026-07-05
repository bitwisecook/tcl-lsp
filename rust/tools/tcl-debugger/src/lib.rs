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
