//! The debug-engine contract. Port of `tooling/debugger/backends/base.py`.
//!
//! A backend owns a running (or launchable) script and exposes execution
//! control + state inspection. The VM backend ([`VmBackend`], below) runs the
//! script on `tcl-vm` and drives a [`crate::controller::DebugController`] from
//! the VM's per-statement hook.
//!
//! # The VM hook (RT-VM-gated)
//!
//! The live VM backend needs `tcl-vm` to call a hook at each source-line
//! boundary with `(source_line, frame_level, command_text, stack)` and to act
//! on the returned [`crate::types::DebugAction`] — i.e. an interpreter that can
//! *pause between statements*. The VM has no such hook yet (tracked under
//! **RT-VM**: an execution-control seam in the exec loop plus a bytecode-PC →
//! source-line map). Until it lands, [`VmBackend`] reports
//! [`DebugError::Unsupported`], while the controller, types, and this contract
//! are usable and tested.

use crate::types::{StackFrame, StopEvent, Variable};

/// Errors a backend can surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugError {
    /// The operation needs a capability the backend does not yet have (the VM
    /// per-statement hook). Carries a human-readable reason.
    Unsupported(String),
    /// A launch / runtime failure.
    Failed(String),
}

impl std::fmt::Display for DebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(m) => write!(f, "unsupported: {m}"),
            Self::Failed(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for DebugError {}

/// Common interface for all debug backends. Mirrors `DebugBackend` (base.py).
pub trait DebugBackend {
    /// Load a script for debugging (from `source` if given, else `path`).
    ///
    /// # Errors
    /// If the script cannot be prepared.
    fn launch(&mut self, path: &str, source: Option<&str>) -> Result<(), DebugError>;

    /// Set line breakpoints; return the accepted lines.
    ///
    /// # Errors
    /// If breakpoints cannot be applied.
    fn set_breakpoints(&mut self, lines: &[u32]) -> Result<Vec<u32>, DebugError>;

    /// Resume until the next breakpoint or the end.
    ///
    /// # Errors
    /// On a runtime failure.
    fn continue_execution(&mut self) -> Result<(), DebugError>;

    /// Execute one statement, stepping into calls.
    ///
    /// # Errors
    /// On a runtime failure.
    fn step_in(&mut self) -> Result<(), DebugError>;

    /// Execute one statement, stepping over calls.
    ///
    /// # Errors
    /// On a runtime failure.
    fn step_over(&mut self) -> Result<(), DebugError>;

    /// Run until the current procedure returns.
    ///
    /// # Errors
    /// On a runtime failure.
    fn step_out(&mut self) -> Result<(), DebugError>;

    /// The current call stack (top first).
    ///
    /// # Errors
    /// If the stack cannot be read.
    fn stack_trace(&self) -> Result<Vec<StackFrame>, DebugError>;

    /// Variables visible in the given frame.
    ///
    /// # Errors
    /// If the scope cannot be read.
    fn variables(&self, frame_id: u32) -> Result<Vec<Variable>, DebugError>;

    /// Evaluate a Tcl expression in the current context.
    ///
    /// # Errors
    /// If evaluation fails.
    fn evaluate(&mut self, expression: &str) -> Result<String, DebugError>;

    /// Terminate the session.
    fn terminate(&mut self);

    /// The most recent stop event, if the backend is paused.
    fn last_stop(&self) -> Option<&StopEvent>;
}

/// The native-VM backend. Drives the script on `tcl-vm` and reports stops via a
/// [`crate::controller::DebugController`].
///
/// Currently a placeholder: every control operation returns
/// [`DebugError::Unsupported`] because the VM has no per-statement debug hook
/// yet (see the module docs). Wiring it is the RT-VM-gated remainder of
/// **TOOL-DEBUGGER**; this type fixes the shape the wiring fills in.
#[derive(Debug, Default)]
pub struct VmBackend {
    script: Option<String>,
}

impl VmBackend {
    /// Construct an (unwired) VM backend.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn unsupported<T>() -> Result<T, DebugError> {
        Err(DebugError::Unsupported(
            "tcl-vm has no per-statement debug hook yet (RT-VM)".to_owned(),
        ))
    }
}

impl DebugBackend for VmBackend {
    fn launch(&mut self, path: &str, source: Option<&str>) -> Result<(), DebugError> {
        // Loading the script text is fine; only *stepping* it needs the hook.
        self.script = Some(match source {
            Some(s) => s.to_owned(),
            None => std::fs::read_to_string(path).map_err(|e| DebugError::Failed(e.to_string()))?,
        });
        Ok(())
    }

    fn set_breakpoints(&mut self, lines: &[u32]) -> Result<Vec<u32>, DebugError> {
        // Accept the lines; line→PC validation needs the bytecode/line map.
        Ok(lines.to_vec())
    }

    fn continue_execution(&mut self) -> Result<(), DebugError> {
        Self::unsupported()
    }
    fn step_in(&mut self) -> Result<(), DebugError> {
        Self::unsupported()
    }
    fn step_over(&mut self) -> Result<(), DebugError> {
        Self::unsupported()
    }
    fn step_out(&mut self) -> Result<(), DebugError> {
        Self::unsupported()
    }
    fn stack_trace(&self) -> Result<Vec<StackFrame>, DebugError> {
        Self::unsupported()
    }
    fn variables(&self, _frame_id: u32) -> Result<Vec<Variable>, DebugError> {
        Self::unsupported()
    }
    fn evaluate(&mut self, _expression: &str) -> Result<String, DebugError> {
        Self::unsupported()
    }
    fn terminate(&mut self) {
        self.script = None;
    }
    fn last_stop(&self) -> Option<&StopEvent> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_backend_launches_but_cannot_step_yet() {
        let mut b = VmBackend::new();
        b.launch("x.tcl", Some("set x 1\nputs $x\n")).expect("launch");
        assert_eq!(b.set_breakpoints(&[2]).unwrap(), vec![2]);
        assert!(matches!(b.step_in(), Err(DebugError::Unsupported(_))));
        assert!(matches!(b.stack_trace(), Err(DebugError::Unsupported(_))));
        assert!(b.last_stop().is_none());
        b.terminate();
    }
}
