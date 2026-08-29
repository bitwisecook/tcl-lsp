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

//! Family-B runtime contract — the state-mutation protocol shared across Tcl
//! runtimes.
//!
//! The emitter↔runtime contract is a *state-mutation protocol*, not a
//! value-passing interface: the runtime is a reified, mutable store (namespace
//! tree, frame stack, variable tables, traces, command table) that compiled or
//! interpreted code reaches into. This crate is the published contract for that
//! store — the completion type, opaque handles, the `CompileService` injection
//! point, and a set of small **role traits** generic over an associated
//! `Value`. It deliberately contains no implementations; a runtime such as the
//! bytecode VM (`tcl-vm`) satisfies it over its own value/storage model.
//!
//! See `docs/design/common-runtime-emitter-architecture.md` §4 (Family B).

use tcl_dialect::model::{Family};

// The value-less vocabulary (the completion `Code`, the generic `Completion<V>`,
// and the opaque arena handles) lives in the dependency-free `tcl-core-types`
// leaf crate. This crate's own only dependency is that leaf — the concrete
// bytecode artifact is kept out of [`CompileService`] (an associated `Module`
// type) precisely so a shared command-core crate (`tcl-cmd-core`) can depend on
// these role traits without pulling in `tcl-bytecode`. Re-exported here so
// existing `tcl_runtime_api::{Code, Completion, NsId, …}` consumers are unaffected.
pub use tcl_core_types::{
    Code, CommandId, Completion, FrameId, GLOBAL_FRAME, NsId, ROOT_NS, VarId,
};

/// Target-neutral compiler/runtime code-generation ABI descriptors and wasm32
/// transport layout constants.
pub mod codegen_abi;

/// Runtime-issued guards for speculative compiler fast paths.
pub mod guard;

// -- Compile service (the EVAL_STK / dynamic-code injection point) --

/// A compilation failure surfaced by [`CompileService`].
#[derive(Debug, Clone)]
pub struct CompileError(pub String);

/// Compiles a Tcl source string to a runtime-executable module at runtime.
///
/// `eval`/`uplevel`/dynamic command names compile a string while the program
/// runs, so a VM that supports them needs a compiler available during
/// execution. Injecting it as a trait keeps the VM crate lean and
/// compiler-optional: the embedder wires a real (`tcl-compiler`-backed)
/// implementation; a program that never hits `eval` can use a stub. Mirrors C
/// Tcl always carrying its bytecode compiler.
///
/// The produced module is an associated type (e.g. the VM sets it to
/// `tcl_bytecode::ModuleAsm`) so this contract crate stays free of any concrete
/// bytecode dependency — see the crate-level note.
pub trait CompileService {
    /// The runtime-executable artifact produced (the bytecode VM's `ModuleAsm`).
    type Module;

    /// Compile `src` to a [`Module`](Self::Module), or report why it could not.
    fn compile(&self, src: &str) -> Result<Self::Module, CompileError>;

    /// Compile `src` for the dialect profile currently selected by the
    /// interpreter. A VM may change profile after it has cached dynamic
    /// bodies, so reusing a compiler constructed for an older registry/grammar
    /// is not sound: an unavailable command could already have been lowered to
    /// bytecode and bypass normal command dispatch.
    ///
    /// Profile-aware services must override this and select the profile's
    /// registry, lexer grammar, and expression dialect for this invocation.
    /// A fixed-profile compiler is permitted only for the permissive fallback;
    /// named-profile dynamic compilation is rejected rather than silently
    /// stamping older bytecode as current.
    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        if profile.is_fallback() {
            self.compile(src)
        } else {
            Err(CompileError(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )))
        }
    }

    /// Compile `src` with every registry-driven inline/structured lowering
    /// hook suppressed: every command compiles to a plain dispatch, so
    /// execution traces — including `enterstep`/`leavestep` step traces —
    /// observe it (C Tcl's `DONT_COMPILE_CMDS_INLINE`, `tclTrace.c`; a step
    /// trace forces the traced proc "out of bytecode" so no inner command,
    /// including `set`/`incr`/`if`/`while`, is invisible to the trace).
    ///
    /// Used to recompile a proc's (or any dynamically-evaluated script's)
    /// body once a step-capable execution trace targets it, and reverted the
    /// same way once the last such trace is removed — the VM never leaves a
    /// proc permanently de-optimised. The default falls back to [`compile`]
    /// (an embedder that never threads trace-deopt information keeps its
    /// existing behaviour; the divergence stays but nothing breaks).
    fn compile_traced(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.compile(src)
    }

    /// Profile-aware counterpart of [`Self::compile_traced`]. See
    /// [`Self::compile_for_profile`] for why dynamic recompilation must select
    /// the VM's current profile rather than a compiler's construction-time
    /// profile. The default follows the same fixed-profile rejection rule.
    fn compile_traced_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        if profile.is_fallback() {
            self.compile_traced(src)
        } else {
            Err(CompileError(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )))
        }
    }
}

// -- Family-B role traits --
//
// Small, composable traits over an associated `Value`, each mirroring a
// `runtime/rust` storage module. A consumer depends only on the subset it
// needs; do not collapse them into one umbrella `Interp` trait. Impls grow
// over time; the trait surface is the contract.

/// Variable storage: scalars, arrays, and `upvar`/`global`/`variable` links,
/// addressed by call frame. Corresponds to `frame.rs`'s `Var`/`VarTable`.
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

    // Array-element access. The `name` is the array *base* and `key` the element,
    // already split from `base(key)` — runtimes differ on whether the by-name
    // accessors parse `a(k)`, so element ops are explicit. Mirror the scalar ops.

    /// Read array element `name(key)` in `frame`, following links.
    fn get_elem(&self, frame: FrameId, name: &str, key: &str) -> Option<Self::Value>;
    /// Write array element `name(key)` in `frame` (firing write traces).
    fn set_elem(&mut self, frame: FrameId, name: &str, key: &str, value: Self::Value);
    /// Remove array element `name(key)`; returns whether it existed.
    fn unset_elem(&mut self, frame: FrameId, name: &str, key: &str) -> bool;
    /// Whether array element `name(key)` exists.
    fn exists_elem(&self, frame: FrameId, name: &str, key: &str) -> bool;

    /// The element keys of array `name` in `frame`, in the runtime's storage
    /// order, or `None` if `name` is not an array (a scalar, or unset). An array
    /// with no elements yields `Some(vec![])` — the existence signal `array
    /// exists`/`info exists` need. This is the **enumeration** surface the
    /// otherwise-deliberately-listing-free state traits expose for the `array`
    /// family (`names`/`get`/`size`/`exists`/`unset`).
    fn array_keys(&self, frame: FrameId, name: &str) -> Option<Vec<String>>;
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

    // -- active-frame variable enumeration (backs the *frame-local* half of `info
    // vars`/`info locals`, the namespace-free counterpart to `Namespaces::vars_in`).

    /// Whether the active frame is a **procedure** activation (vs the global or a
    /// `namespace eval` frame) — `info vars` lists the frame's own variables in a
    /// proc, the current namespace's variables otherwise (C's `InfoVarsCmd`).
    fn in_proc(&self) -> bool;
    /// The variable names of the **active** frame. Genuine locals (scalars,
    /// arrays) are always included; `upvar`/`global`/`variable` **links** are
    /// included iff `include_links` — `info vars` lists links (by their local
    /// alias), `info locals` does not.
    fn var_names(&self, include_links: bool) -> Vec<String>;
}

/// The command table and dispatch: builtins, procs, aliases, imports,
/// ensembles, child interps.'s `interp.rs` `Command`.
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

/// The namespace tree and name resolution. (Contract surface; not yet
/// implemented.)
pub trait Namespaces {
    /// Resolve `name` (qualified or unqualified) from context `cxt` to the
    /// command it names, following the `cxt → namespace path → root` order. The
    /// returned handle is invoked via [`Commands::dispatch_id`].
    fn find_command(&self, cxt: NsId, name: &str) -> Option<CommandId>;
    /// The current namespace.
    fn current(&self) -> NsId;
    /// The fully-qualified name of namespace `ns` (`"::"` for the global root) —
    /// what `namespace current` reports.
    fn name(&self, ns: NsId) -> String;
    /// The fully-qualified name a [`CommandId`] names (the inverse of
    /// [`find_command`](Self::find_command)), or `None` for a stale/unknown id —
    /// backs `namespace which`.
    fn command_name(&self, cmd: CommandId) -> Option<String>;

    // -- namespace-tree navigation (mirrors C's `Namespace` struct: `nsId`
    // identity, `parentPtr`, `childTable`). A namespace *is* a handle; its
    // FQN/parent/children are queried from it. `name(ns)` (above) is its
    // `fullName`. These back `namespace exists`/`parent`/`children`.

    /// Resolve namespace `name` (qualified or unqualified) from context `cxt` to
    /// its handle (`Tcl_FindNamespace`), or `None` if no such namespace exists.
    fn find_namespace(&self, cxt: NsId, name: &str) -> Option<NsId>;
    /// The handle of `ns`'s parent (`parentPtr`), or `None` for the global root.
    fn parent(&self, ns: NsId) -> Option<NsId>;
    /// The handles of `ns`'s direct child namespaces (`childTable`).
    fn children(&self, ns: NsId) -> Vec<NsId>;

    // -- command enumeration (a namespace's `cmdTable`; backs `info commands`/
    // `info procs`). Direct members only — one level, not descendants — returned
    // as **unqualified** tail names. This is the command-listing **enumeration**
    // surface, the namespace analogue of `VarStore::array_keys`.

    /// The unqualified names of commands defined **directly** in `ns`. Mirrors a
    /// walk of `Namespace.cmdTable`; backs `info commands`.
    fn commands_in(&self, ns: NsId) -> Vec<String>;
    /// The unqualified names of **user procedures** defined directly in `ns` (the
    /// `TclIsProc` subset of [`commands_in`](Self::commands_in)); backs `info procs`.
    fn procs_in(&self, ns: NsId) -> Vec<String>;
    /// The unqualified names of **variables** defined directly in `ns` (a walk of
    /// `Namespace.varTable`); backs `info vars ::ns::*` and `info globals` (the
    /// global namespace's variables). The variable analogue of
    /// [`commands_in`](Self::commands_in).
    fn vars_in(&self, ns: NsId) -> Vec<String>;

    // -- resolution accessors the shared `namespace which -variable` / `origin`
    // cores need beyond navigation (`tcl_cmd_core::namespace`).

    /// Does namespace `ns`'s **own** variable table hold an entry named
    /// `simple` (an unqualified name)? This is `Tcl_FindNamespaceVar`'s single
    /// probe: the namespace's `varTable` only — never the call frame, so a
    /// proc local of the same name is invisible here. Backs
    /// [`which_variable`](../tcl_cmd_core/namespace/fn.which_variable.html).
    fn namespace_var_exists(&self, ns: NsId, simple: &str) -> bool;

    /// The command `cmd` was ultimately imported from — C's
    /// `TclGetOriginalCommand`, which is itself the whole walk (`while
    /// (cmdPtr->deleteProc == DeleteImportedCmd) cmdPtr = realCmdPtr`), not a
    /// single hop. `None` when `cmd` is not an imported command.
    ///
    /// The walk stays with the runtime because an import link is a command
    /// *token*, and a runtime whose tokens are name-keyed needs its own
    /// disambiguation (the VM's hidden/visible domains: `interp hide {} a b`
    /// leaves a hidden token `b` whose provenance must not be confused with an
    /// unrelated visible command also called `b`). Backs
    /// [`origin`](../tcl_cmd_core/namespace/fn.origin.html).
    fn command_origin(&self, cmd: CommandId) -> Option<CommandId>;

    // -- byte-valued spellings ------------------------------------------------
    //
    // A Tcl name is a byte string, not text: `set [binary format c 255] 1`
    // names a variable no `&str` can hold without a lossy round trip. The
    // `&str` methods above stay the ergonomic form for the UTF-8-keyed VM,
    // whose own tables cannot hold anything else, while a byte-native runtime
    // overrides these so a name reaches its table verbatim. The defaults
    // preserve today's behaviour exactly (`from_utf8_lossy`), so an
    // implementation that is already UTF-8-keyed need not do anything.

    /// [`find_command`](Self::find_command) over a byte-valued name.
    fn find_command_bytes(&self, cxt: NsId, name: &[u8]) -> Option<CommandId> {
        self.find_command(cxt, &String::from_utf8_lossy(name))
    }

    /// [`find_namespace`](Self::find_namespace) over a byte-valued name.
    fn find_namespace_bytes(&self, cxt: NsId, name: &[u8]) -> Option<NsId> {
        self.find_namespace(cxt, &String::from_utf8_lossy(name))
    }

    /// [`namespace_var_exists`](Self::namespace_var_exists) over a
    /// byte-valued simple name.
    fn namespace_var_exists_bytes(&self, ns: NsId, simple: &[u8]) -> bool {
        self.namespace_var_exists(ns, &String::from_utf8_lossy(simple))
    }

    /// [`name`](Self::name) as the bytes the namespace is actually keyed by.
    fn name_bytes(&self, ns: NsId) -> Vec<u8> {
        self.name(ns).into_bytes()
    }

    /// [`command_name`](Self::command_name) as the bytes the command is
    /// actually keyed by.
    fn command_name_bytes(&self, cmd: CommandId) -> Option<Vec<u8>> {
        self.command_name(cmd).map(String::into_bytes)
    }
}

/// Variable traces (read/write/unset). (Contract surface; not yet
/// implemented.)
pub trait Traces {
    /// The runtime's value type.
    type Value;

    /// Fire any traces registered for `var` on operation `op`
    /// (`"read"`/`"write"`/`"unset"`); a trace error aborts the access.
    fn fire(&mut self, var: &str, op: &str) -> Result<(), Self::Value>;
}

/// Runtime introspection backing the `info` family: retained proc bodies, the
/// per-frame argv, and the command-frame stack (`errorInfo`/`info frame`).
/// (Contract surface; not yet implemented.)
pub trait Introspect {
    /// The runtime's value type.
    type Value;

    /// The current call-stack depth (`info level`).
    fn level(&self) -> usize;
    /// The argv of the call at `level` (`info level N`), if retained.
    fn level_argv(&self, level: usize) -> Option<Self::Value>;
}

/// Procedure introspection backing `info body`/`args`/`default`: the retained
/// formal parameters (name + optional default) and source body of a user
/// procedure. Mirrors the `Proc`/`CompiledLocal` chain C's `InfoBodyCmd`/
/// `InfoArgsCmd`/`InfoDefaultCmd` walk.
///
/// The answer is plain owned bytes, **not** the runtime's `Value`: the contract
/// stays value-agnostic, and the byte-oriented runtime avoids minting fresh
/// refcounted result objects inside a `&self` query — the shared `info` core
/// builds the result value from these bytes through `ValueOps`.
pub trait Procs {
    /// The formals + body of the user procedure `name` resolves to (following
    /// `namespace import` redirects, exactly as a call would), or `None` if
    /// `name` is not a user procedure (a builtin, alias, or unknown command).
    fn proc_info(&self, name: &str) -> Option<ProcInfo>;
}

/// A user procedure's introspectable definition — the [`Procs::proc_info`] answer.
#[derive(Debug, Clone)]
pub struct ProcInfo {
    /// The procedure body source (`info body`), byte-exact.
    pub body: Vec<u8>,
    /// The formal parameters, in declaration order (`info args`/`default`).
    pub params: Vec<ProcParam>,
}

/// One formal parameter of a [`ProcInfo`]: a name with an optional default value.
#[derive(Debug, Clone)]
pub struct ProcParam {
    /// The parameter name.
    pub name: Vec<u8>,
    /// The declared default value, if the parameter has one.
    pub default: Option<Vec<u8>>,
}
