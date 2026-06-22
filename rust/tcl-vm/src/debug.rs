//! Debug-hook seam: a callback fired at each source-command boundary.
//!
//! When a [`DebugHook`] is installed (via [`Vm::set_debug_hook`]), the VM calls
//! it at every `startCommand` boundary with a full [`DebugSnapshot`] of the
//! interpreter state — the source line and command, the call stack, and the
//! current frame's variables — and honours the returned [`DebugAction`]. This
//! is the execution-control seam a step debugger (`tcl-debugger`) drives; it is
//! inert (zero per-instruction cost beyond an `Option` check) when no hook is
//! installed.
//!
//! [`Vm::set_debug_hook`]: crate::Vm::set_debug_hook

/// What the hook tells the VM to do after a command boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugAction {
    /// Continue execution.
    Continue,
    /// Stop the VM (the run unwinds with a debug-terminate result).
    Stop,
}

/// One call-stack frame in a [`DebugSnapshot`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugFrame {
    /// Absolute frame level (0 = global).
    pub level: u32,
    /// The proc name, or `"global"` at top level.
    pub name: String,
    /// The frame's namespace (no leading `::`; empty = global).
    pub namespace: String,
}

/// A variable visible in the stopped frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugVar {
    /// Variable name.
    pub name: String,
    /// Rendered value.
    pub value: String,
}

/// A full snapshot of interpreter state at a command boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugSnapshot {
    /// 1-based source line of the command about to run.
    pub line: u32,
    /// The command's source text.
    pub command_text: String,
    /// The current (top) frame level.
    pub level: u32,
    /// The call stack, top frame first.
    pub stack: Vec<DebugFrame>,
    /// Variables in the current frame.
    pub variables: Vec<DebugVar>,
}

/// The installed debug callback.
pub type DebugHook = Box<dyn FnMut(&DebugSnapshot) -> DebugAction>;
