//! Family-B runtime contract — the state-mutation protocol shared across Tcl
//! runtimes.
//!
//! The emitter↔runtime contract is a *state-mutation protocol*, not a
//! value-passing interface: the runtime is a reified, mutable store (namespace
//! tree, frame stack, variable tables, traces, command table) that compiled or
//! interpreted code reaches into. This crate is the published contract for that
//! store — the completion type, opaque handles, the `CompileService` injection
//! point, and a set of small **role traits** generic over an associated
//! `Value`. It deliberately contains no implementations; the bytecode VM
//! (`tcl-vm`) and, later, the Rust WASM runtime (`runtime/rust`) each satisfy
//! it over their own value/storage models.
//!
//! See `docs/design/common-runtime-emitter-architecture.md` §4 (Family B).

use tcl_bytecode::ModuleAsm;

/// A Tcl completion code (`TCL_OK` … `TCL_CONTINUE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// `TCL_OK` — normal completion.
    Ok,
    /// `TCL_ERROR` — an error; `result` is the message, `options` the dict.
    Error,
    /// `TCL_RETURN` — a `return` propagating to the enclosing proc boundary.
    Return,
    /// `TCL_BREAK` — a loop `break`.
    Break,
    /// `TCL_CONTINUE` — a loop `continue`.
    Continue,
}

impl Code {
    /// The integer completion code (`TCL_OK` = 0 … `TCL_CONTINUE` = 4) — what
    /// `catch` returns and the `-code` options-dict entry uses.
    #[must_use]
    pub fn as_int(self) -> i64 {
        match self {
            Code::Ok => 0,
            Code::Error => 1,
            Code::Return => 2,
            Code::Break => 3,
            Code::Continue => 4,
        }
    }

    /// Whether this is `TCL_OK`.
    #[must_use]
    pub fn is_ok(self) -> bool {
        matches!(self, Code::Ok)
    }
}

/// A command/script completion: a code, the result value, and the return
/// options dict. The "result is not a string" contract — every dispatch yields
/// this, never a bare string. Generic over the runtime's value type `V`.
#[derive(Debug, Clone)]
pub struct Completion<V> {
    /// The completion code.
    pub code: Code,
    /// The result value (the message when `code == Error`).
    pub result: V,
    /// The return-options dict (carries `-code`/`-level`/`-errorinfo`/…).
    pub options: V,
}

impl<V> Completion<V> {
    /// Construct a completion from its parts.
    pub fn new(code: Code, result: V, options: V) -> Self {
        Self {
            code,
            result,
            options,
        }
    }

    /// Whether the completion is `TCL_OK`.
    pub fn is_ok(&self) -> bool {
        self.code.is_ok()
    }
}

// -- Opaque handles --
//
// Arena indices into the runtime's storage; they cross the trait boundary, the
// concrete storage behind them does not.

/// A namespace handle (arena id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NsId(pub u32);

/// A call-frame handle (absolute level / arena id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameId(pub usize);

/// A command handle (arena id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandId(pub u32);

/// A variable-cell handle (arena id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(pub u32);

/// The global call frame (level 0) and root namespace handles.
pub const GLOBAL_FRAME: FrameId = FrameId(0);
/// The root (`::`) namespace handle.
pub const ROOT_NS: NsId = NsId(0);

// -- Compile service (the EVAL_STK / dynamic-code injection point) --

/// A compilation failure surfaced by [`CompileService`].
#[derive(Debug, Clone)]
pub struct CompileError(pub String);

/// Compiles a Tcl source string to bytecode at runtime.
///
/// `eval`/`uplevel`/dynamic command names compile a string while the program
/// runs, so a VM that supports them needs a compiler available during
/// execution. Injecting it as a trait keeps the VM crate lean and
/// compiler-optional: the embedder wires a real (`tcl-compiler`-backed)
/// implementation; a program that never hits `eval` can use a stub. Mirrors C
/// Tcl always carrying its bytecode compiler.
pub trait CompileService {
    /// Compile `src` to a [`ModuleAsm`], or report why it could not.
    fn compile(&self, src: &str) -> Result<ModuleAsm, CompileError>;
}

// -- Family-B role traits --
//
// Small, composable traits over an associated `Value`, each mirroring a
// `runtime/rust` storage module. A consumer depends only on the subset it
// needs; do not collapse them into one umbrella `Interp` trait. Impls grow as
// the VM advances through the milestones; the trait surface is the contract.

/// Variable storage: scalars, arrays, and `upvar`/`global`/`variable` links,
/// addressed by call frame. Mirrors `runtime/rust`'s `frame.rs` `Var`/`VarTable`.
pub trait VarStore {
    /// The runtime's value type.
    type Value;

    /// Read a scalar variable in `frame`, following links.
    fn get(&self, frame: FrameId, name: &str) -> Option<Self::Value>;
    /// Write a scalar variable in `frame` (firing write traces).
    fn set(&mut self, frame: FrameId, name: &str, value: Self::Value);
    /// Remove a variable in `frame`; returns whether it existed.
    fn unset(&mut self, frame: FrameId, name: &str) -> bool;
    /// Whether a variable exists in `frame`.
    fn exists(&self, frame: FrameId, name: &str) -> bool;
}

/// The call-frame stack: proc-call frames and the `uplevel` active-level dance.
/// Mirrors `runtime/rust`'s `frame.rs` `FrameStack` (`framePtr`/`varFramePtr`).
pub trait Frames {
    /// Push a new call frame whose namespace context is `ns`; returns its id.
    fn push(&mut self, ns: NsId) -> FrameId;
    /// Pop the current call frame.
    fn pop(&mut self);
    /// The current (top) frame.
    fn current(&self) -> FrameId;
    /// Install a link (`upvar`/`global`/`variable`) in `here` to `target`'s
    /// variable `target_name`.
    fn link(&mut self, here: FrameId, target: FrameId, local: &str, target_name: &str);
}

/// The command table and dispatch: builtins, procs, aliases, imports,
/// ensembles, child interps. Mirrors `runtime/rust`'s `interp.rs` `Command`.
pub trait Commands {
    /// The runtime's value type.
    type Value;

    /// Dispatch a command by (already-resolved) name with its argv.
    fn dispatch(&mut self, name: &str, argv: &[Self::Value]) -> Completion<Self::Value>;
}

/// The namespace tree and name resolution. Mirrors `runtime/zig`'s `tcl_ns.zig`
/// / `runtime/rust`'s `namespace.rs`. (Contract surface; impls land in M2+.)
pub trait Namespaces {
    /// Resolve `name` (qualified or unqualified) from context `cxt` to the
    /// command it names, following the `cxt → namespace path → root` order.
    fn find_command(&self, cxt: NsId, name: &str) -> Option<CommandId>;
    /// The current namespace.
    fn current(&self) -> NsId;
}

/// Variable traces (read/write/unset). Mirrors `runtime/zig`'s
/// `tcl_var_trace.zig`. (Contract surface; impls land in M2+.)
pub trait Traces {
    /// The runtime's value type.
    type Value;

    /// Fire any traces registered for `var` on operation `op`
    /// (`"read"`/`"write"`/`"unset"`); a trace error aborts the access.
    fn fire(&mut self, var: &str, op: &str) -> Result<(), Self::Value>;
}

/// Runtime introspection backing the `info` family: retained proc bodies, the
/// per-frame argv, and the command-frame stack (`errorInfo`/`info frame`).
/// (Contract surface; impls land in M2+.)
pub trait Introspect {
    /// The runtime's value type.
    type Value;

    /// The current call-stack depth (`info level`).
    fn level(&self) -> usize;
    /// The argv of the call at `level` (`info level N`), if retained.
    fn level_argv(&self, level: usize) -> Option<Self::Value>;
}
