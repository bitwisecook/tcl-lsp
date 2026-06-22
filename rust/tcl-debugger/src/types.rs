//! Shared debugger types. Port of `tooling/debugger/types.py`.

/// What the debug hook tells the VM to do after a source-line boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugAction {
    /// Keep running.
    Continue,
    /// Stop the VM (terminate the debug session).
    Stop,
}

/// The controller's current stepping mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    /// Run until the next breakpoint.
    Continue,
    /// Stop at the next source line, descending into calls.
    StepIn,
    /// Stop at the next source line in the current frame or shallower.
    StepOver,
    /// Run until the current procedure returns.
    StepOut,
}

/// One frame in the debug call stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackFrame {
    /// Frame id (depth from the top).
    pub id: u32,
    /// Proc name, or `"global"`.
    pub name: String,
    /// 1-based source line of the active statement.
    pub line: u32,
    /// The frame's namespace.
    pub namespace: String,
}

/// A variable visible in a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    /// Variable name.
    pub name: String,
    /// Rendered value.
    pub value: String,
    /// `"scalar"`, `"array"`, or `"alias"`.
    pub kind: VariableKind,
    /// For `upvar`/alias visualisation: the target name.
    pub alias_target: Option<String>,
    /// Array elements, when `kind == Array`.
    pub children: Vec<Variable>,
}

/// The kind of a [`Variable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableKind {
    /// A plain scalar.
    Scalar,
    /// An array (its elements are `children`).
    Array,
    /// An `upvar`/alias to another variable.
    Alias,
}

/// Why the debugger stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// A breakpoint was hit.
    Breakpoint,
    /// A step completed.
    Step,
    /// Stopped at program entry.
    Entry,
}

/// Emitted when the debugger pauses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopEvent {
    /// The 1-based source line stopped at.
    pub line: u32,
    /// The text of the command about to run.
    pub command_text: String,
    /// Why it stopped.
    pub reason: StopReason,
    /// The current call stack (top first).
    pub frames: Vec<StackFrame>,
}
