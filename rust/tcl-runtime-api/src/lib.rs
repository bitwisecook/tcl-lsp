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

// The value-less vocabulary (the completion `Code`, the generic `Completion<V>`,
// and the opaque arena handles) lives in the dependency-free `tcl-core-types`
// leaf crate, so pure command logic (`tcl-cmd-core`) can name a completion code
// without transitively depending on `tcl-bytecode` (pulled in below for
// `CompileService`). Re-exported here so existing `tcl_runtime_api::{Code,
// Completion, NsId, …}` consumers are unaffected.
pub use tcl_core_types::{
    Code, CommandId, Completion, FrameId, GLOBAL_FRAME, NsId, ROOT_NS, VarId,
};

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

    /// Dispatch a command by name with its argv, resolving the name in the
    /// current context.
    fn dispatch(&mut self, name: &str, argv: &[Self::Value]) -> Completion<Self::Value>;

    /// Dispatch a command already resolved to a [`CommandId`] (by
    /// [`Namespaces::find_command`]) with its argv — the resolve-then-invoke
    /// pairing (mirrors Tcl's `Tcl_GetCommandFromObj` + `Tcl_NRCallObjProc`). A
    /// stale or fabricated id yields an error completion. This is what makes a
    /// `CommandId` *do* something: resolve once via `find_command`, invoke here.
    fn dispatch_id(&mut self, cmd: CommandId, argv: &[Self::Value]) -> Completion<Self::Value>;
}

/// The namespace tree and name resolution. Mirrors `runtime/zig`'s `tcl_ns.zig`
/// / `runtime/rust`'s `namespace.rs`. (Contract surface; impls land in M2+.)
pub trait Namespaces {
    /// Resolve `name` (qualified or unqualified) from context `cxt` to the
    /// command it names, following the `cxt → namespace path → root` order. The
    /// returned handle is invoked via [`Commands::dispatch_id`].
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
