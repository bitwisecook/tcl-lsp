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

//! The debug-engine contract + the native record-and-replay VM backend.
//!
//! The [`VmBackend`] runs the script once on `tcl-vm` with a recording
//! debug hook installed (the [`tcl_vm::Vm::set_debug_hook`] seam), capturing a
//! [`tcl_vm::DebugSnapshot`] at every command boundary, then serves
//! stepping / inspection by navigating that trace with a
//! [`crate::controller::DebugController`].
//!
//! Record-and-replay avoids threading the debugger across a live, paused
//! interpreter: the full execution trace (line, stack, variables per command)
//! is captured up front, and `step_in`/`over`/`out`/`continue` move a cursor
//! over it. The trade-off is that `evaluate` only sees variables already
//! captured at the current step (no arbitrary re-execution).

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen as build_cfg;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect as lower_to_ir;
use tcl_compiler::lowering::lower_to_ir_traced_with_config;
use tcl_dialect::DialectProfile;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, DebugAction, DebugSnapshot, Vm};

use crate::controller::DebugController;
use crate::types::{StackFrame, StepMode, StopEvent, StopReason, Variable, VariableKind};

/// Stack budget for the dedicated worker thread [`VmBackend::record`] runs
/// compilation on. Matches `WORKER_STACK_SIZE` in `tcl-lsp-server`'s
/// `main.rs` — see that constant's doc comment for the full rationale
/// (issue #996).
const RECORD_STACK_SIZE: usize = 64 * 1024 * 1024;

/// Errors a backend can surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugError {
    /// The operation needs a capability the backend does not have.
    Unsupported(String),
    /// A launch / runtime failure.
    Failed(String),
    /// The script has run to completion — there is nothing more to step.
    Finished,
}

impl std::fmt::Display for DebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(m) => write!(f, "unsupported: {m}"),
            Self::Failed(m) => write!(f, "{m}"),
            Self::Finished => write!(f, "program has finished"),
        }
    }
}

impl std::error::Error for DebugError {}

/// Common interface for all debug backends.
pub trait DebugBackend {
    /// Load a script for debugging (from `source` if given, else `path`) and
    /// stop at the first statement.
    ///
    /// # Errors
    /// If the script cannot be prepared or compiled.
    fn launch(&mut self, path: &str, source: Option<&str>) -> Result<(), DebugError>;

    /// Set line breakpoints; return the accepted lines.
    ///
    /// # Errors
    /// If breakpoints cannot be applied.
    fn set_breakpoints(&mut self, lines: &[u32]) -> Result<Vec<u32>, DebugError>;

    /// Resume until the next breakpoint or the end.
    ///
    /// # Errors
    /// [`DebugError::Finished`] when the program has ended.
    fn continue_execution(&mut self) -> Result<(), DebugError>;

    /// Execute one statement, stepping into calls.
    ///
    /// # Errors
    /// [`DebugError::Finished`] when the program has ended.
    fn step_in(&mut self) -> Result<(), DebugError>;

    /// Execute one statement, stepping over calls.
    ///
    /// # Errors
    /// [`DebugError::Finished`] when the program has ended.
    fn step_over(&mut self) -> Result<(), DebugError>;

    /// Run until the current procedure returns.
    ///
    /// # Errors
    /// [`DebugError::Finished`] when the program has ended.
    fn step_out(&mut self) -> Result<(), DebugError>;

    /// The current call stack (top first).
    ///
    /// # Errors
    /// If the backend is not paused at a statement.
    fn stack_trace(&self) -> Result<Vec<StackFrame>, DebugError>;

    /// Variables visible in the given frame (`0` = top).
    ///
    /// # Errors
    /// If the backend is not paused at a statement.
    fn variables(&self, frame_id: u32) -> Result<Vec<Variable>, DebugError>;

    /// Evaluate a simple variable reference (`$name` / `name`) at the current
    /// step.
    ///
    /// # Errors
    /// [`DebugError::Unsupported`] for anything but a captured variable.
    fn evaluate(&mut self, expression: &str) -> Result<String, DebugError>;

    /// Terminate the session.
    fn terminate(&mut self);

    /// The most recent stop event, if paused.
    fn last_stop(&self) -> Option<&StopEvent>;
}

/// The `CompileService` the VM uses to compile the script and any runtime
/// `eval` / command substitution: the real Rust compiler pipeline, built from
/// the one resolved [`DialectProfile`] the debugger VM emulates (issue #1462)
/// so the compiler's grammar and registry match the runtime release.
struct Svc {
    registry: &'static CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: &'static str,
}

impl Svc {
    /// A compile service targeting `profile`'s grammar, registry, and dialect.
    fn for_profile(profile: &'static DialectProfile) -> Self {
        Self {
            registry: tcl_registry::registry_for_profile(profile),
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            dialect: profile.name,
        }
    }
}

impl CompileService for Svc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir(src, self.registry, self.config, self.dialect);
        let cfg = build_cfg(&ir, false);
        Ok(codegen_module(&cfg, &ir, self.registry))
    }
    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        Self::for_profile(profile).compile(src)
    }
    fn compile_traced(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir_traced_with_config(src, self.registry, self.config);
        let cfg = build_cfg(&ir, false);
        Ok(codegen_module(&cfg, &ir, self.registry))
    }
    fn compile_traced_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        Self::for_profile(profile).compile_traced(src)
    }
}

/// The native record-and-replay VM backend.
#[derive(Default)]
pub struct VmBackend {
    /// Every command-boundary snapshot, in execution order.
    trace: Vec<DebugSnapshot>,
    /// Index into `trace` of the current stop, once launched.
    cursor: Option<usize>,
    controller: DebugController,
    last_stop: Option<StopEvent>,
}

impl VmBackend {
    /// Construct a VM backend.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Compile and run `source`, capturing the execution trace.
    fn record(source: &str) -> Result<Vec<DebugSnapshot>, DebugError> {
        // `Svc::compile` runs the same `lower_to_ir`/`build_cfg_codegen`
        // recursive-descent chain that crashed `tcl-lsp-server` on deeply
        // nested input (issue #996) — its depth cap bounds the frame
        // *count* but not the stack the OS/ambient thread happens to
        // provide. Run it on a dedicated big-stack thread rather than
        // whatever `launch` was called on (this CLI's main thread, or a
        // DAP request-handling thread), matching the fix applied to
        // `tcl-lsp-server`/`tcl-mcp`/`tcl-cli`/`f5-cli`.
        let source = source.to_owned();
        std::thread::Builder::new()
            .stack_size(RECORD_STACK_SIZE)
            .spawn(move || Self::record_on_this_thread(&source))
            .expect("failed to spawn the tcl-debug record worker thread")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    }

    /// The actual compile-and-run work behind [`Self::record`] — split out
    /// so [`Self::record`] can run it on a dedicated big-stack thread; see
    /// that function's doc comment.
    fn record_on_this_thread(source: &str) -> Result<Vec<DebugSnapshot>, DebugError> {
        // The debugger VM runs the plain-Tcl 9.0 profile (dialect-profile
        // model §5.4); the profile is resolved once and drives both the
        // runtime release and the compiler's grammar/registry (issue #1462).
        let profile = DialectProfile::by_name("tcl9.0");
        let module = Svc::for_profile(profile)
            .compile(source)
            .map_err(|e| DebugError::Failed(e.0))?;

        let trace: Rc<RefCell<Vec<DebugSnapshot>>> = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&trace);

        let mut vm = Vm::new();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(Svc::for_profile(profile)));
        vm.set_debug_hook(Some(Box::new(move |snap: &DebugSnapshot| {
            sink.borrow_mut().push(snap.clone());
            DebugAction::Continue
        })));
        let _ = vm.run_module(&module);
        vm.set_debug_hook(None);

        Ok(Rc::try_unwrap(trace)
            .map(RefCell::into_inner)
            .unwrap_or_default())
    }

    /// The snapshot at the current cursor, or `None` before launch / at end.
    fn current(&self) -> Option<&DebugSnapshot> {
        self.cursor.and_then(|i| self.trace.get(i))
    }

    /// Advance the cursor to the next trace index the controller stops at under
    /// `mode`, anchored at the current frame level. Updates `last_stop`.
    fn advance(&mut self, mode: StepMode) -> Result<(), DebugError> {
        let Some(cur) = self.cursor else {
            return Err(DebugError::Finished);
        };
        let anchor = self.trace.get(cur).map_or(0, |s| s.level);
        self.controller.resume(mode, anchor);
        let mut i = cur + 1;
        while i < self.trace.len() {
            let snap = &self.trace[i];
            if self.controller.should_stop(snap.line, snap.level).is_some() {
                self.cursor = Some(i);
                self.controller.anchor(snap.level);
                self.set_stop(i);
                return Ok(());
            }
            i += 1;
        }
        // Nothing left to stop at — the program has finished.
        self.cursor = None;
        self.last_stop = None;
        Err(DebugError::Finished)
    }

    /// Record the stop event for trace index `i`.
    fn set_stop(&mut self, i: usize) {
        let snap = &self.trace[i];
        let reason = if self.controller.breakpoints().contains(&snap.line) {
            StopReason::Breakpoint
        } else {
            StopReason::Step
        };
        self.last_stop = Some(StopEvent {
            line: snap.line,
            command_text: snap.command_text.clone(),
            reason,
            frames: frames_of(snap),
        });
    }
}

impl DebugBackend for VmBackend {
    fn launch(&mut self, path: &str, source: Option<&str>) -> Result<(), DebugError> {
        let src = match source {
            Some(s) => s.to_owned(),
            None => std::fs::read_to_string(path).map_err(|e| DebugError::Failed(e.to_string()))?,
        };
        self.trace = Self::record(&src)?;
        self.controller = DebugController::new();
        if self.trace.is_empty() {
            self.cursor = None;
            self.last_stop = None;
        } else {
            // Stop at entry (the first command).
            self.cursor = Some(0);
            self.controller.anchor(self.trace[0].level);
            self.last_stop = Some(StopEvent {
                line: self.trace[0].line,
                command_text: self.trace[0].command_text.clone(),
                reason: StopReason::Entry,
                frames: frames_of(&self.trace[0]),
            });
        }
        Ok(())
    }

    fn set_breakpoints(&mut self, lines: &[u32]) -> Result<Vec<u32>, DebugError> {
        Ok(self.controller.set_breakpoints(lines.iter().copied()))
    }

    fn continue_execution(&mut self) -> Result<(), DebugError> {
        self.advance(StepMode::Continue)
    }
    fn step_in(&mut self) -> Result<(), DebugError> {
        self.advance(StepMode::StepIn)
    }
    fn step_over(&mut self) -> Result<(), DebugError> {
        self.advance(StepMode::StepOver)
    }
    fn step_out(&mut self) -> Result<(), DebugError> {
        self.advance(StepMode::StepOut)
    }

    fn stack_trace(&self) -> Result<Vec<StackFrame>, DebugError> {
        self.current().map(frames_of).ok_or(DebugError::Finished)
    }

    fn variables(&self, frame_id: u32) -> Result<Vec<Variable>, DebugError> {
        // The trace captures the *current* frame's variables; frame 0 is the
        // only inspectable scope in replay mode.
        if frame_id != 0 {
            return Ok(Vec::new());
        }
        let snap = self.current().ok_or(DebugError::Finished)?;
        Ok(snap
            .variables
            .iter()
            .map(|v| Variable {
                name: v.name.clone(),
                value: v.value.clone(),
                kind: VariableKind::Scalar,
                alias_target: None,
                children: Vec::new(),
            })
            .collect())
    }

    fn evaluate(&mut self, expression: &str) -> Result<String, DebugError> {
        let name = expression.strip_prefix('$').unwrap_or(expression);
        let snap = self.current().ok_or(DebugError::Finished)?;
        snap.variables
            .iter()
            .find(|v| v.name == name)
            .map(|v| v.value.clone())
            .ok_or_else(|| {
                DebugError::Unsupported(format!(
                    "replay evaluate only resolves captured variables, not {expression:?}"
                ))
            })
    }

    fn terminate(&mut self) {
        self.trace.clear();
        self.cursor = None;
        self.last_stop = None;
    }

    fn last_stop(&self) -> Option<&StopEvent> {
        self.last_stop.as_ref()
    }
}

/// Convert a VM snapshot's stack into debugger [`StackFrame`]s.
fn frames_of(snap: &DebugSnapshot) -> Vec<StackFrame> {
    snap.stack
        .iter()
        .enumerate()
        .map(|(id, fr)| StackFrame {
            id: u32::try_from(id).unwrap_or(0),
            name: fr.name.clone(),
            // Each frame's active line is the snapshot line for the top frame;
            // deeper frames keep the call-site line the VM recorded.
            line: snap.line,
            namespace: fr.namespace.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launched(src: &str) -> VmBackend {
        let mut b = VmBackend::new();
        b.launch("x.tcl", Some(src)).expect("launch");
        b
    }

    #[test]
    fn launch_stops_at_entry_with_first_line() {
        let b = launched("set x 1\nset y 2\nputs $x\n");
        let stop = b.last_stop().expect("entry stop");
        assert_eq!(stop.reason, StopReason::Entry);
        assert_eq!(stop.line, 1);
    }

    /// Regression test for issue #996 in this binary specifically: `launch`
    /// compiles caller-supplied source through the same
    /// `lower_to_ir`/`build_cfg_codegen` recursive-descent chain that
    /// crashed `tcl-lsp-server`. Before `record` ran this on its own
    /// [`RECORD_STACK_SIZE`] thread, this reliably overflowed the stack
    /// `cargo test` gives each `#[test]` (~2 MiB, same default that made
    /// the original crash reproducible) — 400 levels is comfortably past
    /// the 130-140 level range that crashed the unfixed binary.
    ///
    /// Asserts only that `launch` returns rather than aborting the process:
    /// 400 levels is also past `MAX_LOWER_DEPTH` (256), so lowering's own
    /// depth-cap barrier legitimately empties the debug trace past that
    /// point (a separate, already-tested concern in `tcl_compiler::lowering`)
    /// — this test is specifically about surviving, not about what gets
    /// traced past the cap.
    #[test]
    fn launch_survives_deeply_nested_control_flow() {
        const DEPTH: usize = 400;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("set done 1\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        let mut b = VmBackend::new();
        b.launch("x.tcl", Some(&src))
            .expect("launch must not crash or error on deeply nested input");
    }

    #[test]
    fn step_in_walks_lines_and_sees_variables() {
        let mut b = launched("set x 1\nset y 2\nputs $x\n");
        b.step_in().expect("step to line 2");
        assert_eq!(b.last_stop().unwrap().line, 2);
        // After `set x 1` ran, x is visible.
        let vars = b.variables(0).expect("vars");
        assert!(
            vars.iter().any(|v| v.name == "x" && v.value == "1"),
            "{vars:?}"
        );
        assert_eq!(b.evaluate("$x").unwrap(), "1");
    }

    #[test]
    fn continue_stops_at_breakpoint() {
        let mut b = launched("set a 1\nset b 2\nset c 3\nset d 4\n");
        b.set_breakpoints(&[3]).expect("bp");
        b.continue_execution().expect("run to bp");
        assert_eq!(b.last_stop().unwrap().line, 3);
        assert_eq!(b.last_stop().unwrap().reason, StopReason::Breakpoint);
    }

    #[test]
    fn step_over_does_not_descend_into_a_proc() {
        // The proc body runs at a deeper level; step-over from the call should
        // land on the next top-level line, not inside the proc.
        let src = "proc f {} {\n  set inner 9\n}\nf\nset after 1\n";
        let mut b = launched(src);
        // Walk to the `f` call line.
        let mut guard = 0;
        while b.last_stop().is_some_and(|s| s.command_text.trim() != "f") && guard < 50 {
            if b.step_in().is_err() {
                break;
            }
            guard += 1;
        }
        // From the call, stepping over should not stop inside f's body.
        if b.last_stop().is_some_and(|s| s.command_text.trim() == "f") {
            let _ = b.step_over();
            if let Some(stop) = b.last_stop() {
                assert_ne!(stop.command_text.trim(), "set inner 9");
            }
        }
    }

    #[test]
    fn run_to_completion_reports_finished() {
        let mut b = launched("set x 1\n");
        // Only one command — continuing past it finishes.
        assert_eq!(b.continue_execution(), Err(DebugError::Finished));
        assert!(b.last_stop().is_none());
    }
}
