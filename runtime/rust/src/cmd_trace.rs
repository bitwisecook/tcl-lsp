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

//! `trace` — variable, command, and execution traces (`trace
//! add|remove|info variable|command|execution`).
//!
//! Mirrors `tclTrace.c`'s `Tcl_TraceObjCmd` dispatcher and the three type
//! helpers (`TraceVariableObjCmd`/`TraceCommandObjCmd`/`TraceExecutionObjCmd`):
//!
//! - **variable** (`read`/`write`/`unset`/`array`): when a traced variable is
//!   read, written, or unset, the registered command prefix is invoked as
//!   `command name element op`. Fired from the variable read/write/unset
//!   chokepoints (`Interp::fire_var_trace`).
//! - **command** (`rename`/`delete`): fired when the traced command is renamed
//!   or deleted, as `command oldName newName rename` / `command oldName {}
//!   delete` (`Interp::fire_cmd_trace`).
//! - **execution** (`enter`/`leave`/`enterstep`/`leavestep`): `enter`/`leave`
//!   fire around the traced command's own invocation; `enterstep`/`leavestep`
//!   fire around every command executed while a step-traced command is on the
//!   stack. Fired from the dispatch chokepoint (`Interp::dispatch`).
//!
//! Variable traces are keyed by the resolved variable identity (home namespace
//! or local call frame, plus simple name). Command and execution traces are
//! keyed by the resolved FQN (`Interp::resolve_cmd_fqn`).
//!
//! The deprecated 8.x `trace variable|vdelete|vinfo` forms are supported too.
//! C compiles them behind `#ifndef TCL_REMOVE_OBSOLETE_TRACES` and Tcl 9.0
//! dropped them, so the option word is resolved against the option set the
//! registry declares for the emulated release rather than a fixed list.

use tcl_cmd_core::trace as core_trace;
use tcl_dialect::model::surface_admits;

use crate::frame::{split_array_ref, VarError};
use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::namespace::NsId;
use crate::obj::TclObj;

/// One registered variable trace.
pub struct VarTrace {
    /// This registration's identity, unique for the life of the interpreter.
    ///
    /// C identifies a trace by its `VarTrace *`, and the firing loop follows
    /// `active.nextTracePtr`, which `Tcl_UntraceVar2` rewrites when it frees a
    /// trace mid-walk — so a callback that removes a *later* trace stops it
    /// firing in the same pass. A snapshot of the callbacks taken up front
    /// cannot express that; a snapshot of ids can, because the id says which
    /// registration to look for again. Ids are never reused, so a trace removed
    /// and re-added during one firing is a different trace, exactly as it is a
    /// different allocation in C.
    pub id: u64,
    /// The variable name as registered (for `trace info` matching).
    pub name: Vec<u8>,
    /// The array base / scalar name (for firing).
    pub base: Vec<u8>,
    /// The specific element, if the trace was on `arr(elem)`.
    pub elem: Option<Vec<u8>>,
    /// The operation names this trace fires on (`read`/`write`/`unset`/`array`).
    pub ops: Vec<Vec<u8>>,
    /// The command prefix invoked when the trace fires.
    pub command: Vec<u8>,
    /// For a trace on a **proc-local** variable, the call-frame level it lives
    /// at — the trace dies when that frame is popped (C frees the local var's
    /// trace list at frame teardown). `None` for global/namespace/qualified
    /// traces, which persist.
    pub frame_level: Option<usize>,
    /// The home namespace of the traced variable, for a trace registered on a
    /// namespace variable at namespace/global scope (or a qualified name). The
    /// trace fires only for accesses resolving to that same namespace variable,
    /// so a trace on `::a::x` doesn't fire for `::b::x` (and dies when its
    /// namespace is deleted). `None` for proc-local traces, which match by raw
    /// name as before (their frame disambiguates them).
    pub ns: Option<NsId>,
    /// Registered through the deprecated 8.x `trace variable` form (C's
    /// `TCL_TRACE_OLD_STYLE`), which calls the callback with the single
    /// `rwua` letter rather than the operation name. It is deliberately not
    /// part of the `trace remove` / `trace vdelete` match, exactly as C masks
    /// the flag out there.
    pub old_style: bool,
}

/// The operations a command/execution trace fires on, as a bitset (mirrors C's
/// `tcmdPtr->flags`). Execution and command ops are disjoint; `trace info`
/// filters by category and prints in C's fixed order.
pub mod ops {
    pub const ENTER: u8 = 1;
    pub const LEAVE: u8 = 2;
    pub const ENTERSTEP: u8 = 4;
    pub const LEAVESTEP: u8 = 8;
    pub const RENAME: u8 = 16;
    pub const DELETE: u8 = 32;
    /// Any execution op (the `trace info execution` category).
    pub const EXEC_ANY: u8 = ENTER | LEAVE | ENTERSTEP | LEAVESTEP;
    /// Any step op (a step trace installs an interp-wide trace while active).
    pub const STEP_ANY: u8 = ENTERSTEP | LEAVESTEP;
    /// Any command op (the `trace info command` category).
    pub const CMD_ANY: u8 = RENAME | DELETE;
}

/// One registered command or execution trace (C's `TraceCommandInfo`; both
/// kinds hang off the same command, distinguished by their op category).
pub struct CmdTrace {
    /// This registration's identity, unique for the life of the interpreter —
    /// the command/execution twin of [`VarTrace::id`], and for the same reason:
    /// C's `CallCommandTraces` and `TclCheckExecutionTraces` walk the live list
    /// through `nextPtr`, and `Tcl_UntraceCommand` unlinks a record as soon as
    /// a callback removes it, so a snapshot of callback strings would keep
    /// firing traces that are already gone. Ids are never reused.
    pub id: u64,
    /// The command's resolved FQN (the binding the trace is attached to).
    pub name: Vec<u8>,
    /// The user ops this trace fires on (a [`ops`] bitset).
    pub ops: u8,
    /// The command prefix invoked when the trace fires.
    pub command: Vec<u8>,
}

/// A live interp-wide step trace: installed when a command carrying
/// `enterstep`/`leavestep` is entered, it fires for every command executed
/// until that command returns (C's `tcmdPtr->stepTrace` + `startLevel`/
/// `startCmd`). Our call stack brackets the window: it is installed on entry to
/// the step-traced command's `dispatch_traced` and removed on its exit, so
/// recursion installs only the outermost (dedup by `owner`/`command`).
pub struct StepActive {
    /// The FQN of the command whose execution we are stepping (for dedup).
    pub owner: Vec<u8>,
    /// The step ops (`ENTERSTEP`/`LEAVESTEP`) and the callback prefix.
    pub ops: u8,
    pub command: Vec<u8>,
}

/// One variable **cell**, as the callback re-entrancy rule counts them: C sets
/// `VAR_TRACE_ACTIVE` on the `Var` an access reached, and an array element is a
/// `Var` of its own (`TclCallVarTraces`, tclVar.c).
///
/// So `elem` is part of the identity: a whole-array write trace whose callback
/// writes a *different* element fires again, because the second element is a
/// different cell (issue #1574). Only a write to the *same* cell — or, for the
/// whole-array traces specifically, one reached while the array's own cell is
/// active — is suppressed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VarTraceScope {
    base: Vec<u8>,
    elem: Option<Vec<u8>>,
    frame_level: Option<usize>,
    ns: Option<NsId>,
}

impl VarTraceScope {
    /// The cell an access to `(base, elem)` at this home reaches.
    pub(crate) fn cell(
        base: &[u8],
        elem: Option<&[u8]>,
        ns: Option<NsId>,
        frame_level: Option<usize>,
    ) -> Self {
        Self {
            base: base.to_vec(),
            elem: elem.map(<[u8]>::to_vec),
            frame_level,
            ns,
        }
    }

    /// The containing array's own cell — C's `arrayPtr`, whose `VAR_TRACE_ACTIVE`
    /// gates the whole-array traces separately from the element's.
    pub(crate) fn array(&self) -> Self {
        Self {
            elem: None,
            ..self.clone()
        }
    }
}

/// The interp's trace registries plus per-variable re-entrancy state.
#[derive(Default)]
pub struct TraceTable {
    pub traces: Vec<VarTrace>,
    /// Command + execution traces, keyed by resolved command FQN.
    pub cmd_traces: Vec<CmdTrace>,
    /// Live interp-wide step traces (see [`StepActive`]).
    pub step_active: Vec<StepActive>,
    /// The id the next variable-trace registration takes (see [`VarTrace::id`]).
    pub next_var_trace_id: u64,
    /// The id the next command/execution-trace registration takes (see
    /// [`CmdTrace::id`]).
    pub next_cmd_trace_id: u64,
    /// Variable cells whose trace callbacks are currently running. Other
    /// variables remain traceable from within a callback.
    pub active_var_scopes: Vec<VarTraceScope>,
    /// Non-zero while a command/execution trace callback is running — C's
    /// `INTERP_TRACE_IN_PROGRESS`. Suppresses re-entrant command/execution/step
    /// firing so a callback that renames/invokes the traced command doesn't
    /// recurse.
    pub exec_firing: usize,
    /// The error message a read/write variable-trace callback left, captured so
    /// the variable access can fail with `can't read/set "name": <msg>` (C's
    /// `TclCallVarTraces` propagation). Taken by the access chokepoint.
    pub pending_err: Option<Vec<u8>>,
}

/// Whether `t` belongs to the resolved variable identity. Namespace variables
/// are distinguished by home namespace, and proc locals by their call frame.
pub fn same_variable(
    t: &VarTrace,
    base: &[u8],
    access_ns: Option<NsId>,
    access_frame_level: Option<usize>,
) -> bool {
    t.base == base && t.ns == access_ns && t.frame_level == access_frame_level
}

/// Whether `t` fires for a `(base, elem)` access doing operation `op`.
pub fn matches(
    t: &VarTrace,
    base: &[u8],
    elem: Option<&[u8]>,
    op: &[u8],
    access_ns: Option<NsId>,
    access_frame_level: Option<usize>,
) -> bool {
    if !same_variable(t, base, access_ns, access_frame_level) {
        return false;
    }
    if let Some(te) = &t.elem {
        // Element-specific trace: only that element.
        if elem != Some(te.as_slice()) {
            return false;
        }
    }
    t.ops.iter().any(|o| o == op)
}

/// Register `trace`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"trace", trace_cmd);
}

/// The `trace` option words the emulated release carries, in the registry's
/// declaration order — which is C's `traceOptions[]` order, so the `bad
/// option` / `ambiguous option` enumeration matches byte for byte. The three
/// legacy forms are gated to `the retired availability mask::TCL8X`, so 9.0+ sees only
/// `add`/`info`/`remove` (C drops them behind `TCL_REMOVE_OBSOLETE_TRACES`).
fn visible_options(interp: &Interp) -> Vec<&'static str> {
    // The emulated release's name is a dialect *name*: one resolution
    // through the ingress seam yields both the generation whose store the
    // spec is read from and the document authoring mask the option table
    // is gated on (ledger row B1).
    let profile =
        crate::environment::profile_for_dialect(interp.runtime_version().dialect_profile_name());
    let dialect = Some(crate::environment::surface_point(profile));
    let Some(spec) = crate::environment::store_for_profile(profile).get("trace") else {
        return Vec::new();
    };
    spec.subcommands
        .iter()
        .filter(|sub| {
            sub.surface
                .or(spec.surface)
                .is_none_or(|gate| surface_admits(gate, dialect.as_ref()))
        })
        .map(|sub| sub.name)
        .collect()
}

/// The variable resolver supplies the reason, while `trace` owns the command
/// verb in its diagnostic (`can't trace`, not the usual `can't set`).
fn trace_var_error(interp: &mut Interp, name: &[u8], error: VarError) -> Code {
    let reason = match error {
        VarError::IsScalar => b"variable isn't array".as_slice(),
        VarError::IsArray => b"variable is array".as_slice(),
        VarError::NoSuchNamespace => b"parent namespace doesn't exist".as_slice(),
        VarError::IsConstant => b"variable is a constant".as_slice(),
        VarError::TraceError => b"trace callback failed".as_slice(),
    };
    let mut message = b"can't trace \"".to_vec();
    message.extend_from_slice(name);
    message.extend_from_slice(b"\": ");
    message.extend_from_slice(reason);
    interp.set_error(&message)
}

fn trace_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"trace option ?arg ...?");
    }
    let options = visible_options(interp);
    let word = obj_bytes(argv[1]);
    let option = match core_trace::resolve_option(&String::from_utf8_lossy(&word), &options) {
        Ok(option) => option,
        Err(e) => return interp.set_error(e.message().as_bytes()),
    };
    match option.as_bytes() {
        // `trace add`/`remove` then dispatch on the type word (objv[2]).
        op @ (b"add" | b"remove") => {
            let is_add = op == b"add";
            if argv.len() < 3 {
                return interp.wrong_args(if is_add {
                    b"trace add type ?arg ...?"
                } else {
                    b"trace remove type ?arg ...?"
                });
            }
            let ty = obj_bytes(argv[2]);
            match core_trace::resolve_type(&String::from_utf8_lossy(&ty)) {
                Ok(core_trace::TraceKind::Variable) => trace_var_add_remove(interp, argv, is_add),
                Ok(core_trace::TraceKind::Command) => {
                    cmd_trace_add_remove(interp, argv, is_add, ops::CMD_ANY)
                }
                Ok(core_trace::TraceKind::Execution) => {
                    cmd_trace_add_remove(interp, argv, is_add, ops::EXEC_ANY)
                }
                Err(e) => interp.set_error(e.message().as_bytes()),
            }
        }
        b"info" => {
            if argv.len() < 3 {
                return interp.wrong_args(b"trace info type name");
            }
            let ty = obj_bytes(argv[2]);
            match core_trace::resolve_type(&String::from_utf8_lossy(&ty)) {
                Ok(core_trace::TraceKind::Variable) => trace_var_info(interp, argv),
                Ok(core_trace::TraceKind::Command) => cmd_trace_info(interp, argv, ops::CMD_ANY),
                Ok(core_trace::TraceKind::Execution) => cmd_trace_info(interp, argv, ops::EXEC_ANY),
                Err(e) => interp.set_error(e.message().as_bytes()),
            }
        }
        // The deprecated 8.x forms; C rewrites them into `trace add|remove
        // variable` with the `rwua` letters expanded to a word list.
        b"variable" => legacy_var_add_remove(interp, argv, true),
        b"vdelete" => legacy_var_add_remove(interp, argv, false),
        b"vinfo" => legacy_var_info(interp, argv),
        // A registry-declared option this engine has no arm for. Reporting it
        // as unknown keeps a data-only spec edit (a new subcommand or alias)
        // from turning into a panic in a shipped interpreter.
        _ => {
            let mut message = b"bad option \"".to_vec();
            message.extend_from_slice(&word);
            message.extend_from_slice(b"\": must be ");
            message.extend_from_slice(tcl_cmd_core::prefix::choice_list(&options).as_bytes());
            interp.set_error(&message)
        }
    }
}

// -- command / execution traces -------------------------------------------

/// Parse an execution-trace op list into a [`ops`] bitset, via the shared core
/// (split + validation + the catalogue) then folding the canonical names to bits.
fn parse_exec_ops(interp: &mut Interp, spec: &[u8]) -> Result<u8, Code> {
    let names = core_trace::parse_ops(spec, core_trace::TraceKind::Execution)
        .map_err(|e| interp.set_error(e.message().as_bytes()))?;
    Ok(names.iter().fold(0u8, |acc, o| {
        acc | match *o {
            "enter" => ops::ENTER,
            "leave" => ops::LEAVE,
            "enterstep" => ops::ENTERSTEP,
            "leavestep" => ops::LEAVESTEP,
            _ => 0,
        }
    }))
}

/// Parse a command-trace op list (`rename`/`delete`) into a [`ops`] bitset.
fn parse_cmd_ops(interp: &mut Interp, spec: &[u8]) -> Result<u8, Code> {
    let names = core_trace::parse_ops(spec, core_trace::TraceKind::Command)
        .map_err(|e| interp.set_error(e.message().as_bytes()))?;
    Ok(names.iter().fold(0u8, |acc, o| {
        acc | match *o {
            "rename" => ops::RENAME,
            "delete" => ops::DELETE,
            _ => 0,
        }
    }))
}

/// `trace add|remove command|execution name opList command`. `category` is
/// `ops::CMD_ANY` or `ops::EXEC_ANY`, selecting the op vocabulary.
fn cmd_trace_add_remove(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    is_add: bool,
    category: u8,
) -> Code {
    let kind: &[u8] = if category == ops::EXEC_ANY {
        b"execution"
    } else {
        b"command"
    };
    if argv.len() != 6 {
        let mut usage = if is_add {
            b"trace add ".to_vec()
        } else {
            b"trace remove ".to_vec()
        };
        usage.extend_from_slice(kind);
        usage.extend_from_slice(b" name opList command");
        return interp.wrong_args(&usage);
    }
    let spec = obj_bytes(argv[4]);
    let flags = match if category == ops::EXEC_ANY {
        parse_exec_ops(interp, &spec)
    } else {
        parse_cmd_ops(interp, &spec)
    } {
        Ok(f) => f,
        Err(c) => return c,
    };
    let name = obj_bytes(argv[3]);
    // Both add and remove require the command to exist (C's `Tcl_TraceCommand`
    // / `Tcl_FindCommand` with `TCL_LEAVE_ERR_MSG`).
    let Some(fqn) = interp.resolve_cmd_fqn(&name) else {
        return interp.unknown_command(&name);
    };
    let command = obj_bytes(argv[5]);
    if is_add {
        let mut traces = interp.traces.borrow_mut();
        traces.next_cmd_trace_id += 1;
        let id = traces.next_cmd_trace_id;
        traces.cmd_traces.push(CmdTrace {
            id,
            name: fqn,
            ops: flags,
            command,
        });
        drop(traces);
        interp.invalidate_guard_domain(tcl_runtime_api::guard::GuardDomain::CommandTrace);
    } else {
        // Remove the first trace matching exact ops + command string, where
        // "first" is C's `FOREACH_COMMAND_TRACE` head→tail order and its head
        // is the newest registration — so among duplicates the newest goes.
        // Our Vec is oldest-first, hence `rposition`. Issue #1440.
        let pos = interp
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .rposition(|t| t.name == fqn && t.ops == flags && t.command == command);
        if let Some(i) = pos {
            interp.traces.borrow_mut().cmd_traces.remove(i);
            interp.invalidate_guard_domain(tcl_runtime_api::guard::GuardDomain::CommandTrace);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `trace info command|execution name` — the matching traces, most-recent
/// first, each a `{opList command}` pair. Ops printed in C's fixed order.
fn cmd_trace_info(interp: &mut Interp, argv: &[*mut TclObj], category: u8) -> Code {
    let kind: &[u8] = if category == ops::EXEC_ANY {
        b"execution"
    } else {
        b"command"
    };
    if argv.len() != 4 {
        let mut usage = b"trace info ".to_vec();
        usage.extend_from_slice(kind);
        usage.extend_from_slice(b" name");
        return interp.wrong_args(&usage);
    }
    let name = obj_bytes(argv[3]);
    let Some(fqn) = interp.resolve_cmd_fqn(&name) else {
        return interp.unknown_command(&name);
    };
    // (bit, label) pairs in C's print order for each category.
    let order: &[(u8, &[u8])] = if category == ops::EXEC_ANY {
        &[
            (ops::ENTER, b"enter"),
            (ops::LEAVE, b"leave"),
            (ops::ENTERSTEP, b"enterstep"),
            (ops::LEAVESTEP, b"leavestep"),
        ]
    } else {
        &[(ops::RENAME, b"rename"), (ops::DELETE, b"delete")]
    };
    let mut entries: Vec<*mut TclObj> = Vec::new();
    for t in interp.traces.borrow().cmd_traces.iter().rev() {
        if t.name != fqn || (t.ops & category) == 0 {
            continue;
        }
        let op_objs: Vec<*mut TclObj> = order
            .iter()
            .filter(|(bit, _)| (t.ops & bit) != 0)
            .map(|(_, label)| new_string(label))
            .collect();
        let ops_list = crate::list::new_list_obj(&op_objs);
        let cmd = new_string(&t.command);
        entries.push(crate::list::new_list_obj(&[ops_list, cmd]));
    }
    interp.set_result(crate::list::new_list_obj(&entries));
    Code::Ok
}

/// Parse and validate a variable-trace ops list (`{read write unset array}`),
/// via the shared core (the op catalogue lives once in `tcl-cmd-core::trace`).
fn parse_ops(interp: &mut Interp, spec: &[u8]) -> Result<Vec<Vec<u8>>, Code> {
    match core_trace::parse_ops(spec, core_trace::TraceKind::Variable) {
        Ok(ops) => Ok(ops.iter().map(|o| o.as_bytes().to_vec()).collect()),
        Err(e) => Err(interp.set_error(e.message().as_bytes())),
    }
}

// -- variable traces -------------------------------------------------------

/// `trace add|remove variable name ops command`.
fn trace_var_add_remove(interp: &mut Interp, argv: &[*mut TclObj], is_add: bool) -> Code {
    if argv.len() != 6 {
        return interp.wrong_args(if is_add {
            b"trace add variable name opList command"
        } else {
            b"trace remove variable name opList command"
        });
    }
    let ops = match parse_ops(interp, &obj_bytes(argv[4])) {
        Ok(o) => o,
        Err(c) => return c,
    };
    var_trace_apply(
        interp,
        obj_bytes(argv[3]),
        ops,
        obj_bytes(argv[5]),
        is_add,
        false,
    )
}

/// The resolved `(home namespace, home frame level, simple base, element)` a
/// `trace info|remove variable name` question is about.
///
/// C looks the variable up and walks *that* `Var`'s trace list, so every
/// spelling of the one cell — `v` and `::v`, an `upvar` alias and its target —
/// sees the same traces. Matching the registration spelling textually instead
/// makes `trace info variable alias` report nothing for a trace the very next
/// `set alias` fires.
fn var_trace_query(
    interp: &Interp,
    name: &[u8],
) -> (Option<NsId>, Option<usize>, Vec<u8>, Option<Vec<u8>>) {
    let (base, elem) = split_array_ref(name);
    let home = interp.trace_identity(&base);
    // An alias for an array *element* (`upvar #0 a(k) e`) shows no parentheses,
    // so the element it names comes from the resolution: in C the alias and
    // `a(k)` are the same `Var` and therefore carry the same trace list.
    (home.ns, home.level, home.base, elem.or(home.link_elem))
}

/// Install or remove one variable trace, shared by `trace add|remove variable`
/// and the deprecated `trace variable`/`trace vdelete` forms — which C
/// implements by rewriting them into the modern ones. `old_style` records the
/// legacy spelling for the callback's op word only; it never takes part in the
/// removal match, exactly as C masks `TCL_TRACE_OLD_STYLE` out there.
fn var_trace_apply(
    interp: &mut Interp,
    name: Vec<u8>,
    ops: Vec<Vec<u8>>,
    command: Vec<u8>,
    is_add: bool,
    old_style: bool,
) -> Code {
    if is_add {
        let (base, spelled_elem) = split_array_ref(&name);
        // An alias for an array *element* (`upvar #0 a(k) e`) is a trace on that
        // element: C hangs it off the element's `Var`, which the alias and the
        // spelling `a(k)` share, so `trace add variable e …` and
        // `trace add variable a(k) …` install one and the same trace and each
        // spelling's `trace info` reports it. The element is invisible in the
        // spelling, so it comes from the resolution.
        let linked_elem = interp.trace_identity(&base).link_elem;
        let elem = spelled_elem.or_else(|| linked_elem.clone());
        // Tracing an array element vivifies the array as an (undefined) array, so
        // a later read of a missing element reports "no such element in array"
        // rather than "no such variable", and whole-array semantics apply
        // (trace-1.4/1.8/5.x). Tracing an element of an existing *scalar* errors.
        // An alias already names a live element cell, so there is nothing to
        // create — and vivifying `e` as a scalar or an array would both be
        // wrong.
        let vivify = if linked_elem.is_some() {
            Ok(())
        } else if elem.is_some() {
            interp.ensure_array(&base)
        } else {
            interp.ensure_trace_variable(&base)
        };
        if let Err(e) = vivify {
            return trace_var_error(interp, &name, e);
        }
        // Key the trace by the variable it *resolves to*, not by the spelling
        // used to register it, so `trace add variable ::v write …` fires for a
        // later `set v X` in the same namespace — and, under the 8.x
        // namespace-scope fallback, for a write from inside `namespace eval`
        // that reaches that same global (issue #1328).  C hangs the trace off
        // the `Var` struct, so every spelling resolving to it fires — including
        // an `upvar` alias, whose home frame is only visible on the resolved
        // place (issue #1633's `upvar` row).
        //
        // `name` keeps the original spelling for diagnostics only; the
        // `trace info` / `trace remove` match is the same resolved identity,
        // because C looks the variable up and walks *that* `Var`'s list.
        let home = interp.trace_identity(&base);
        let mut table = interp.traces.borrow_mut();
        let id = table.next_var_trace_id;
        table.next_var_trace_id += 1;
        table.traces.push(VarTrace {
            id,
            name,
            base: home.base,
            elem,
            ops,
            command,
            frame_level: home.level,
            ns: home.ns,
            old_style,
        });
        drop(table);
        interp.invalidate_guard_domain(tcl_runtime_api::guard::GuardDomain::VariableTrace);
    } else {
        let (q_ns, q_level, q_base, q_elem) = var_trace_query(interp, &name);
        let pos = interp
            .traces
            .borrow()
            .traces
            .iter()
            // Newest-first first match: C breaks at the first hit walking its
            // list head→tail, and the head is the newest registration. Our Vec
            // is oldest-first, hence `rposition`. `old_style` is deliberately
            // absent from the match, as C masks `TCL_TRACE_OLD_STYLE` out
            // here. Issue #1440.
            .rposition(|t| {
                same_variable(t, &q_base, q_ns, q_level)
                    && t.elem == q_elem
                    && t.ops == ops
                    && t.command == command
            });
        if let Some(i) = pos {
            interp.traces.borrow_mut().traces.remove(i);
            interp.invalidate_guard_domain(tcl_runtime_api::guard::GuardDomain::VariableTrace);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `trace info variable name` — the registered traces, most-recent first, each
/// as a `{opList command}` pair.
fn trace_var_info(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"trace info variable name");
    }
    let name = obj_bytes(argv[3]);
    let (q_ns, q_level, q_base, q_elem) = var_trace_query(interp, &name);
    let mut entries: Vec<*mut TclObj> = Vec::new();
    for t in interp.traces.borrow().traces.iter().rev() {
        if !same_variable(t, &q_base, q_ns, q_level) || t.elem != q_elem {
            continue;
        }
        let op_objs: Vec<*mut TclObj> = t.ops.iter().map(|o| new_string(o)).collect();
        let ops_list = crate::list::new_list_obj(&op_objs);
        let cmd = new_string(&t.command);
        entries.push(crate::list::new_list_obj(&[ops_list, cmd]));
    }
    interp.set_result(crate::list::new_list_obj(&entries));
    Code::Ok
}

/// `trace variable name ops command` / `trace vdelete name ops command` — the
/// deprecated 8.x forms. The op word is a concatenation of the letters
/// `r`/`w`/`u`/`a`; the shared parser expands and validates it, so a
/// non-`rwua` byte is C's `bad operations "…": should be one or more of rwua`
/// and the stored set is the same canonical set `trace add variable`
/// produces — `vdelete` therefore removes an `add`-installed trace and vice
/// versa, as C's `~TCL_TRACE_OLD_STYLE` match does.
fn legacy_var_add_remove(interp: &mut Interp, argv: &[*mut TclObj], is_add: bool) -> Code {
    if argv.len() != 5 {
        return interp.wrong_args(if is_add {
            b"trace variable name ops command"
        } else {
            b"trace vdelete name ops command"
        });
    }
    let ops = match core_trace::parse_legacy_variable_ops(&obj_bytes(argv[3])) {
        Ok(ops) => ops.iter().map(|o| o.as_bytes().to_vec()).collect(),
        Err(e) => return interp.set_error(e.message().as_bytes()),
    };
    var_trace_apply(
        interp,
        obj_bytes(argv[2]),
        ops,
        obj_bytes(argv[4]),
        is_add,
        true,
    )
}

/// `trace vinfo name` — the same live trace list `trace info variable` reports,
/// with each op set rendered as the `rwua` letter string C's `TRACE_OLD_VINFO`
/// arm builds instead of a word list.
fn legacy_var_info(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"trace vinfo name");
    }
    let name = obj_bytes(argv[2]);
    let (q_ns, q_level, q_base, q_elem) = var_trace_query(interp, &name);
    let mut entries: Vec<*mut TclObj> = Vec::new();
    for t in interp.traces.borrow().traces.iter().rev() {
        if !same_variable(t, &q_base, q_ns, q_level) || t.elem != q_elem {
            continue;
        }
        let letters = new_string(core_trace::legacy_ops_letters(&t.ops).as_bytes());
        let cmd = new_string(&t.command);
        entries.push(crate::list::new_list_obj(&[letters, cmd]));
    }
    interp.set_result(crate::list::new_list_obj(&entries));
    Code::Ok
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?} → {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    #[test]
    fn write_trace_fires_and_records() {
        leak_free(|i| {
            ok(i, b"set log {}");
            ok(i, b"proc rec {name elem op} {global log; lappend log $op}");
            ok(i, b"trace add variable v write rec");
            ok(i, b"set v 1");
            ok(i, b"set v 2");
            assert_eq!(ok(i, b"set log"), b"write write");
            // info reports the registration.
            assert_eq!(ok(i, b"trace info variable v"), b"{write rec}");
            // remove stops further firing.
            ok(i, b"trace remove variable v write rec");
            ok(i, b"set v 3");
            assert_eq!(ok(i, b"set log"), b"write write");
            i.eval_str(b"unset -nocomplain v log");
        });
    }

    /// Adding a trace to an array element vivifies the array (undefined), so a
    /// later read reports the element-aware miss and array/element existence
    /// match C (trace-1.4); tracing an element of an existing scalar errors.
    #[test]
    fn trace_array_element_vivifies_array() {
        leak_free(|i| {
            ok(i, b"proc foo args {}");
            ok(i, b"trace add variable x(2) read foo");
            assert_eq!(ok(i, b"array exists x"), b"1");
            assert_eq!(ok(i, b"info exists x"), b"1");
            assert_eq!(ok(i, b"info exists x(2)"), b"0");
            assert_eq!(i.eval_str(b"set x(2)"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't read \"x(2)\": no such element in array"
            );
            i.eval_str(b"unset -nocomplain x");
        });
        leak_free(|i| {
            ok(i, b"set y foo");
            assert_eq!(i.eval_str(b"trace add variable y(2) read foo"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't trace \"y(2)\": variable isn't array"
            );
            i.eval_str(b"unset -nocomplain y");
        });
    }

    #[test]
    fn trace_registration_vivifies_only_on_add_and_uses_canonical_sets() {
        leak_free(|i| {
            ok(i, b"proc cb args {}");
            // A scalar registration creates Tcl's unset Var, not an empty
            // value. Its callback is not invoked during registration.
            ok(i, b"trace add variable fresh {write read write} cb");
            assert_eq!(ok(i, b"info exists fresh"), b"0");
            assert_eq!(ok(i, b"trace info variable fresh"), b"{{read write} cb}");

            // Identical registrations are distinct; removal consumes exactly
            // one first match and does not materialise a missing variable.
            ok(i, b"trace add variable fresh {read write} cb");
            ok(i, b"trace remove variable fresh {write read} cb");
            assert_eq!(ok(i, b"trace info variable fresh"), b"{{read write} cb}");
            ok(i, b"trace remove variable absent read cb");
            assert_eq!(ok(i, b"info exists absent"), b"0");

            assert_eq!(
                err(i, b"trace add {} fresh read cb"),
                b"ambiguous option \"\": must be execution, command, or variable"
            );
            assert_eq!(
                err(i, b"trace add variable ::missing::x read cb"),
                b"can't trace \"::missing::x\": parent namespace doesn't exist"
            );
        });
    }

    #[test]
    fn active_trace_does_not_suppress_an_unrelated_variable_trace() {
        leak_free(|i| {
            ok(i, b"set x 0");
            ok(i, b"set y 0");
            ok(i, b"set hits 0");
            ok(i, b"proc x_cb args {global y; incr y}");
            ok(i, b"proc y_cb args {global hits; incr hits}");
            ok(i, b"trace add variable x write x_cb");
            ok(i, b"trace add variable y write y_cb");
            ok(i, b"set x 1");
            assert_eq!(ok(i, b"set hits"), b"1");
        });
    }

    #[test]
    fn active_trace_suppresses_every_trace_on_its_variable_cell() {
        leak_free(|i| {
            ok(i, b"set x 0");
            ok(i, b"set hits_one 0");
            ok(i, b"set hits_two 0");
            ok(
                i,
                b"proc one args {global hits_one x; incr hits_one; incr x}",
            );
            ok(i, b"proc two args {global hits_two; incr hits_two}");
            ok(i, b"trace add variable x write one");
            ok(i, b"trace add variable x write two");
            ok(i, b"set x 1");
            // The outer write invokes both registrations. The nested `incr x`
            // sees the same active cell, so it invokes neither.
            assert_eq!(ok(i, b"set hits_one"), b"1");
            assert_eq!(ok(i, b"set hits_two"), b"1");
        });
    }

    #[test]
    fn read_trace_fires() {
        leak_free(|i| {
            ok(i, b"set hits 0");
            ok(i, b"proc bump {args} {global hits; incr hits}");
            ok(i, b"set x 5");
            ok(i, b"trace add variable x read bump");
            ok(i, b"set y $x");
            ok(i, b"set y $x");
            assert_eq!(ok(i, b"set hits"), b"2");
            i.eval_str(b"unset -nocomplain x y hits");
        });
    }

    fn err(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(i.eval_str(src), Code::Error, "expected error for {src:?}");
        i.result_bytes()
    }

    #[test]
    fn exec_trace_register_info_remove_roundtrip() {
        leak_free(|i| {
            ok(i, b"proc foo {a} {return $a}");
            ok(i, b"trace add execution foo {enter leave} cb1");
            ok(i, b"trace add execution foo enterstep cb2");
            // Most-recent first; ops in C's fixed print order within each entry.
            assert_eq!(
                ok(i, b"trace info execution foo"),
                b"{enterstep cb2} {{enter leave} cb1}"
            );
            // A command trace does not show under `execution` (category filter).
            ok(i, b"trace add command foo delete cbd");
            assert_eq!(
                ok(i, b"trace info execution foo"),
                b"{enterstep cb2} {{enter leave} cb1}"
            );
            assert_eq!(ok(i, b"trace info command foo"), b"{delete cbd}");
            // Remove the first exact match.
            ok(i, b"trace remove execution foo {enter leave} cb1");
            assert_eq!(ok(i, b"trace info execution foo"), b"{enterstep cb2}");
        });
    }

    #[test]
    fn cmd_trace_info_op_order_is_rename_then_delete() {
        leak_free(|i| {
            ok(i, b"proc foo {} {}");
            ok(i, b"trace add command foo {delete rename} cb");
            assert_eq!(ok(i, b"trace info command foo"), b"{{rename delete} cb}");
        });
    }

    #[test]
    fn trace_errors_match_c() {
        leak_free(|i| {
            ok(i, b"proc foo {} {}");
            assert_eq!(
                err(i, b"trace add bogus foo enter cb"),
                b"bad option \"bogus\": must be execution, command, or variable"
            );
            assert_eq!(
                err(i, b"trace add execution nosuch enter cb"),
                b"unknown command \"nosuch\""
            );
            assert_eq!(
                err(i, b"trace add execution foo {} cb"),
                b"bad operation list \"\": must be one or more of enter, leave, enterstep, or leavestep"
            );
            assert_eq!(
                err(i, b"trace add execution foo bogus cb"),
                b"bad operation \"bogus\": must be enter, leave, enterstep, or leavestep"
            );
            assert_eq!(
                err(i, b"trace add command foo bogus cb"),
                b"bad operation \"bogus\": must be delete or rename"
            );
            assert_eq!(
                err(i, b"trace info command nosuch"),
                b"unknown command \"nosuch\""
            );
            assert_eq!(
                err(i, b"trace remove command nosuch delete cb"),
                b"unknown command \"nosuch\""
            );
        });
    }

    #[test]
    fn command_trace_fires_on_rename_and_delete() {
        leak_free(|i| {
            ok(i, b"set log {}");
            ok(i, b"proc cb {args} {global log; lappend log $args}");
            ok(i, b"proc foo {} {return hi}");
            ok(i, b"trace add command foo {rename delete} cb");
            ok(i, b"rename foo bar");
            // FQN old/new + op; the trace follows the command to ::bar.
            assert_eq!(ok(i, b"set log"), b"{::foo ::bar rename}");
            assert_eq!(ok(i, b"trace info command bar"), b"{{rename delete} cb}");
            ok(i, b"rename bar {}");
            assert_eq!(ok(i, b"set log"), b"{::foo ::bar rename} {::bar {} delete}");
            // The trace went away with the command.
            assert_eq!(ok(i, b"set log2 [info commands bar]"), b"");
            i.eval_str(b"unset -nocomplain log log2");
        });
    }

    #[test]
    fn command_trace_callback_error_is_ignored() {
        leak_free(|i| {
            ok(i, b"proc boom {args} {error kaboom}");
            ok(i, b"proc q {} {}");
            ok(i, b"trace add command q delete boom");
            // Delete still succeeds; the callback error is swallowed.
            assert_eq!(ok(i, b"rename q {}"), b"");
        });
    }

    #[test]
    fn exec_trace_enter_leave_fire() {
        leak_free(|i| {
            ok(i, b"set log {}");
            ok(i, b"proc cb {args} {global log; lappend log $args}");
            ok(i, b"proc foo {a b} {return $a-$b}");
            ok(i, b"trace add execution foo {enter leave} cb");
            assert_eq!(ok(i, b"foo x y"), b"x-y");
            // enter: {cmd args} enter ; leave: {cmd args} code result leave.
            assert_eq!(
                ok(i, b"set log"),
                &b"{{foo x y} enter} {{foo x y} 0 x-y leave}"[..]
            );
            i.eval_str(b"unset -nocomplain log");
        });
    }

    #[test]
    fn exec_trace_order_and_live_result() {
        leak_free(|i| {
            ok(i, b"set log {}");
            ok(i, b"proc cb {args} {global log; lappend log $args}");
            ok(i, b"proc baz {} {return R}");
            ok(i, b"trace add execution baz {enter leave} {cb 1}");
            ok(i, b"trace add execution baz {enter leave} {cb 2}");
            ok(i, b"baz");
            // enter newest-first (2,1); leave oldest-first (1,2); the 2nd leave
            // sees the 1st leave callback's result (live, not preserved).
            assert_eq!(
                ok(i, b"set log"),
                &b"{2 baz enter} {1 baz enter} {1 baz 0 R leave} {2 baz 0 {{2 baz enter} {1 baz enter} {1 baz 0 R leave}} leave}"[..]
            );
            i.eval_str(b"unset -nocomplain log");
        });
    }

    #[test]
    fn exec_trace_enter_error_aborts_leave_error_overrides() {
        leak_free(|i| {
            ok(i, b"proc bar {} {return ok}");
            ok(i, b"proc deny {args} {error NOPE}");
            ok(i, b"trace add execution bar enter deny");
            assert_eq!(err(i, b"bar"), b"NOPE");
            ok(i, b"trace remove execution bar enter deny");
            // Command runs again cleanly once the enter trace is gone.
            assert_eq!(ok(i, b"bar"), b"ok");
            // A leave-trace error overrides the command's result/code.
            ok(i, b"proc boom {args} {error OVERRIDE}");
            ok(i, b"trace add execution bar leave boom");
            assert_eq!(err(i, b"bar"), b"OVERRIDE");
        });
    }

    #[test]
    fn step_trace_fires_for_body_commands() {
        leak_free(|i| {
            ok(i, b"set log {}");
            ok(i, b"proc tr {args} {global log; lappend log $args}");
            ok(i, b"proc foo {} {set a 1; set b 2}");
            ok(i, b"trace add execution foo {enterstep leavestep} tr");
            ok(i, b"foo");
            // Each body command gets enterstep before and leavestep after, with
            // <code> <result>; foo itself is not stepped by its own trace.
            assert_eq!(
                ok(i, b"set log"),
                &b"{{set a 1} enterstep} {{set a 1} 0 1 leavestep} {{set b 2} enterstep} {{set b 2} 0 2 leavestep}"[..]
            );
            i.eval_str(b"unset -nocomplain log");
        });
    }

    // Needs the numeric tower: the traced proc body is `if`+`expr` (quoted in the pinned log).
    #[cfg(have_tommath)]
    #[test]
    fn step_trace_recursion_installs_once() {
        leak_free(|i| {
            ok(i, b"set log {}");
            ok(
                i,
                b"proc tr {args} {global log; lappend log [lindex $args 0]}",
            );
            ok(i, b"proc rec {n} {if {$n>0} {rec [expr {$n-1}]}}");
            ok(i, b"trace add execution rec enterstep tr");
            ok(i, b"rec 1");
            // Only the outermost rec installs the step trace; the inner call is
            // stepped by that single active trace (no double-firing).
            assert_eq!(
                ok(i, b"set log"),
                &b"{if {$n>0} {rec [expr {$n-1}]}} {expr {$n-1}} {rec 0} {if {$n>0} {rec [expr {$n-1}]}}"[..]
            );
            i.eval_str(b"unset -nocomplain log");
        });
    }

    #[test]
    fn redefining_a_proc_clears_and_fires_delete_traces() {
        leak_free(|i| {
            ok(i, b"set log {}");
            ok(i, b"proc cb {args} {global log; lappend log $args}");
            ok(i, b"proc foo {} {}");
            ok(i, b"trace add command foo delete cb");
            ok(i, b"trace add execution foo enter cb");
            // Redefining foo deletes the old command: fire its delete trace,
            // drop all its traces (C's Tcl_CreateObjCommand replace).
            ok(i, b"proc foo {} {}");
            assert_eq!(ok(i, b"set log"), b"{::foo {} delete}");
            assert_eq!(ok(i, b"trace info command foo"), b"");
            assert_eq!(ok(i, b"trace info execution foo"), b"");
            i.eval_str(b"unset -nocomplain log");
        });
    }

    #[test]
    fn write_trace_error_propagates() {
        leak_free(|i| {
            ok(i, b"unset -nocomplain z");
            ok(
                i,
                b"trace add variable z write {unset z; error {memory corruption};#}",
            );
            // The set fails with the trace's message; C's TclObjCallVarTraces.
            assert_eq!(err(i, b"set z 1"), b"can't set \"z\": memory corruption");
            i.eval_str(b"unset -nocomplain z");
        });
    }

    #[test]
    fn read_trace_error_propagates() {
        leak_free(|i| {
            ok(i, b"set w 5");
            ok(i, b"trace add variable w read {error boom;#}");
            // Both the `$w` and `set w` read forms fail with `can't read`.
            assert_eq!(err(i, b"set w"), b"can't read \"w\": boom");
            assert_eq!(err(i, b"set q $w"), b"can't read \"w\": boom");
            i.eval_str(b"unset -nocomplain w q");
        });
    }

    #[test]
    fn unset_trace_error_is_ignored() {
        leak_free(|i| {
            ok(i, b"set v 1");
            ok(i, b"trace add variable v unset {error boom;#}");
            // Unset-trace errors are swallowed (C ignores them).
            assert_eq!(ok(i, b"unset v"), b"");
        });
    }

    #[test]
    fn unsetting_a_variable_clears_its_traces() {
        leak_free(|i| {
            ok(i, b"set x 1");
            ok(i, b"proc cb {args} {}");
            ok(i, b"trace add variable x read cb");
            ok(i, b"unset x");
            ok(i, b"set x 2");
            // The trace died with the variable (C frees the Var's trace list).
            assert_eq!(ok(i, b"trace info variable x"), b"");
            i.eval_str(b"unset -nocomplain x");
        });
    }

    #[test]
    fn unset_keeps_traces_on_same_named_variables_in_other_namespaces() {
        leak_free(|i| {
            ok(
                i,
                b"proc record {label args} {lappend ::events $label}\n\
                  set ::events {}\n\
                  namespace eval ::a {variable x 1; trace add variable x write {record scalar-a}}\n\
                  namespace eval ::b {variable x 1; trace add variable x write {record scalar-b}}\n\
                  unset ::a::x\n\
                  set ::b::x 2",
            );
            // TP: deleting `::a::x` removes only its own trace, not the
            // same-bare-name trace attached to `::b::x`.
            assert_eq!(ok(i, b"set ::events"), b"scalar-b");

            ok(
                i,
                b"set ::events {}\n\
                  namespace eval ::c {variable x; set x(k) 1; trace add variable x(k) write {record element-c}}\n\
                  namespace eval ::d {variable x; set x(k) 1; trace add variable x(k) write {record element-d}}\n\
                  unset ::c::x(k)\n\
                  set ::d::x(k) 2",
            );
            // TP: element removal has the same home-namespace identity check.
            assert_eq!(ok(i, b"set ::events"), b"element-d");
        });
    }

    #[test]
    fn unset_keeps_a_callers_local_trace_when_a_callee_uses_the_same_name() {
        leak_free(|i| {
            ok(
                i,
                b"proc record {label args} {lappend ::events $label}\n\
                  proc inner {} {set x inner; trace add variable x write {record inner}; unset x}\n\
                  proc outer {} {set x outer; trace add variable x write {record outer}; inner; set x after}\n\
                  set ::events {}\n\
                  outer",
            );
            // TN: a proc-local trace is scoped by its call frame, so the inner
            // unset cannot delete the caller's trace for its own `x`.
            assert_eq!(ok(i, b"set ::events"), b"outer");
        });
    }

    /// Every trace list is prepended in C (`TraceVarEx`, tclTrace.c
    /// 9.0.4:3090-3092) and walked head→tail, so the newest registration fires
    /// first for `read`/`write`/`unset` alike. Issue #1440; pinned against
    /// tclsh 8.6.16 and 9.0.4.
    #[test]
    fn variable_traces_fire_newest_first() {
        for op in [&b"write"[..], b"read", b"unset"] {
            leak_free(|i| {
                ok(i, b"set ::log {}");
                ok(i, b"proc rec {label args} {lappend ::log $label}");
                let mut add = b"trace add variable v ".to_vec();
                add.extend_from_slice(op);
                for label in [&b" {rec one}"[..], b" {rec two}", b" {rec three}"] {
                    let mut line = add.clone();
                    line.extend_from_slice(label);
                    ok(i, &line);
                }
                ok(i, b"set v x");
                ok(i, b"set ignore $v");
                ok(i, b"unset v");
                assert_eq!(ok(i, b"set ::log"), b"three two one");
                i.eval_str(b"unset -nocomplain ignore ::log");
            });
        }
    }

    /// C walks the containing array's trace list before the element's own
    /// (`TclCallVarTraces`' `arrayPtr` loop precedes its `varPtr` loop), so
    /// registration order does not decide which fires first. Issue #1440.
    #[test]
    fn whole_array_traces_fire_before_element_traces() {
        for script in [
            &b"trace add variable a write {rec W}\ntrace add variable a(k) write {rec E}"[..],
            b"trace add variable a(k) write {rec E}\ntrace add variable a write {rec W}",
        ] {
            leak_free(|i| {
                ok(i, b"set ::log {}");
                ok(i, b"proc rec {label args} {lappend ::log $label}");
                ok(i, b"array set a {}");
                ok(i, script);
                ok(i, b"set a(k) 1");
                assert_eq!(ok(i, b"set ::log"), b"W E");
                i.eval_str(b"unset -nocomplain a ::log");
            });
        }
    }

    /// `Tcl_TraceCommand` prepends and `CallCommandTraces` walks head→tail, so
    /// `rename`/`delete` callbacks also run newest-first. Issue #1440.
    #[test]
    fn command_traces_fire_newest_first() {
        leak_free(|i| {
            ok(i, b"set ::log {}");
            ok(
                i,
                b"proc rec {label args} {lappend ::log $label-[lindex $args end]}",
            );
            ok(i, b"proc victim {} {}");
            ok(i, b"trace add command victim {rename delete} {rec one}");
            ok(i, b"trace add command victim {rename delete} {rec two}");
            ok(i, b"rename victim victim2");
            ok(i, b"rename victim2 {}");
            assert_eq!(
                ok(i, b"set ::log"),
                b"two-rename one-rename two-delete one-delete"
            );
            i.eval_str(b"unset -nocomplain ::log");
        });
    }

    /// `trace remove` breaks at the first match walking C's list head→tail, and
    /// that head is the newest registration — so among identical duplicates the
    /// **newest** goes. Observable in the surviving firing order and in
    /// `trace info`. Issue #1440; pinned against tclsh 8.6.16 and 9.0.4.
    #[test]
    fn trace_remove_drops_the_newest_duplicate() {
        leak_free(|i| {
            ok(i, b"set ::log {}");
            ok(i, b"proc rec {label args} {lappend ::log $label}");
            ok(i, b"trace add variable v write {rec one}");
            ok(i, b"trace add variable v write {rec two}");
            ok(i, b"trace add variable v write {rec one}");
            ok(i, b"trace remove variable v write {rec one}");
            assert_eq!(
                ok(i, b"trace info variable v"),
                b"{write {rec two}} {write {rec one}}"
            );
            ok(i, b"set v 1");
            assert_eq!(ok(i, b"set ::log"), b"two one");
            i.eval_str(b"unset -nocomplain v ::log");
        });
        leak_free(|i| {
            ok(i, b"proc cb1 args {}");
            ok(i, b"proc cb2 args {}");
            ok(i, b"proc p {} {}");
            ok(i, b"trace add command p delete cb1");
            ok(i, b"trace add command p delete cb2");
            ok(i, b"trace add command p delete cb1");
            ok(i, b"trace remove command p delete cb1");
            assert_eq!(ok(i, b"trace info command p"), b"{delete cb2} {delete cb1}");
        });
    }

    /// Namespace teardown fires each variable's unset traces newest-first, like
    /// every other trace list (the order *across* the namespace's variables is
    /// C's hash walk and is not pinned). Issue #1440.
    #[test]
    fn namespace_teardown_fires_unset_traces_newest_first() {
        leak_free(|i| {
            ok(i, b"set ::log {}");
            ok(i, b"proc rec {label args} {lappend ::log $label}");
            ok(
                i,
                b"namespace eval ::gone {variable v 1\n\
                  trace add variable v unset {rec first}\n\
                  trace add variable v unset {rec second}}",
            );
            ok(i, b"namespace delete ::gone");
            assert_eq!(ok(i, b"set ::log"), b"second first");
            i.eval_str(b"unset -nocomplain ::log");
        });
    }

    /// The commands the same teardown deletes fire their `delete` traces
    /// newest-first too (`CallCommandTraces` walks head→tail). Issue #1440;
    /// tclsh 8.6.16 and 9.0.4 both report `second first`.
    #[test]
    fn namespace_teardown_fires_command_delete_traces_newest_first() {
        leak_free(|i| {
            ok(i, b"set ::log {}");
            ok(i, b"proc rec {label args} {lappend ::log $label}");
            ok(i, b"namespace eval ::doomed {proc victim {} {}}");
            ok(i, b"trace add command ::doomed::victim delete {rec first}");
            ok(i, b"trace add command ::doomed::victim delete {rec second}");
            ok(i, b"namespace delete ::doomed");
            assert_eq!(ok(i, b"set ::log"), b"second first");
            i.eval_str(b"unset -nocomplain ::log");
        });
    }

    #[test]
    fn parent_teardown_reaches_each_child_after_parent_commands() {
        // Tcl drains a parent's owned ensembles while both namespace tokens are
        // live, marks only that parent dying for its ordinary command traces,
        // and reaches each child ensemble through a separate recursive delete.
        // Exact Tcl 9.0.4 oracle result.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc ensdeleted {tag old new op} {
                          lappend ::seen [list $tag [namespace exists ::P] \
                                               [namespace exists ::P::C]]
                      }
                      proc cmddeleted {old new op} {
                          lappend ::seen [list PC [namespace exists ::P] \
                                               [namespace exists ::P::C] \
                                               [info commands ::CE]]
                      }
                      namespace eval ::P {
                          namespace ensemble create -command ::PE
                          proc p {} {}
                          namespace eval C {
                              namespace ensemble create -command ::CE
                          }
                      }
                      trace add command ::PE delete [list ensdeleted P]
                      trace add command ::P::p delete cmddeleted
                      trace add command ::CE delete [list ensdeleted C]
                      namespace delete ::P
                      set seen"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{P 1 1} {PC 0 1 ::CE} {C 0 1}");
        });
    }

    #[test]
    fn namespace_teardown_does_not_resurrect_a_dying_namespace() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc cb {old new op} {
                          lappend ::seen [namespace exists ::N]
                          set c [catch {
                              namespace eval ::N {proc q {} {return Q}}
                          } m]
                          lappend ::seen $c $m [namespace exists ::N] \
                              [info commands ::N::q]
                      }
                      namespace eval N {proc p {} {return P}}
                      trace add command ::N::p delete cb
                      namespace delete ::N
                      list $seen [namespace exists ::N] [info commands ::N::q]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{0 1 {can't create namespace \"::N\": already exists} 0 {}} 0 {}"
            );
        });
    }

    #[test]
    fn retained_dying_namespace_handle_is_not_publicly_live() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc cb {old new op} {
                          proc ::N::q {} {
                              set c [catch {namespace parent {}} m o]
                              list [namespace current] [namespace exists {}] \
                                  $c $m [dict get $o -errorcode]
                          }
                          set ::seen [::N::q]
                      }
                      namespace eval N {proc p {} {return P}}
                      trace add command ::N::p delete cb
                      namespace delete ::N
                      list $seen [namespace exists ::N] \
                          [info commands ::N::q]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{::N 0 1 {namespace \"\" not found in \"::N\"} \
                  {TCL LOOKUP NAMESPACE {}}} 0 {}"
            );
        });
    }

    #[test]
    fn retained_namespace_handle_remains_dead_after_deletion_returns() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval N {
                          proc p {} {
                              namespace delete ::N
                              set code [catch {namespace parent {}} message options]
                              list [namespace current] [namespace exists {}] \
                                  $code $message [dict get $options -errorcode]
                          }
                          p
                      }"
                ),
                Code::Ok
            );
            // Exact Tcl 9.0.4 oracle: the retained token still names `::N`,
            // but relative public lookup cannot treat it as a live namespace.
            assert_eq!(
                i.result_bytes(),
                b"::N 0 1 {namespace \"\" not found in \"::N\"} \
                  {TCL LOOKUP NAMESPACE {}}"
            );
        });
    }

    #[test]
    fn recreated_namespace_does_not_revive_a_retained_dead_token() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval N {
                          proc p {} {
                              namespace delete ::N
                              namespace eval ::N {}
                              set old [list [namespace current] \
                                  [namespace exists {}] [namespace exists ::N]]
                              set old_code [catch {namespace parent {}} \
                                  old_message old_options]
                              set new_code [catch {namespace parent ::N} new_message]
                              list $old $old_code $old_message \
                                  [dict get $old_options -errorcode] \
                                  $new_code $new_message
                          }
                          p
                      }"
                ),
                Code::Ok
            );
            // Exact Tcl 9.0.4 oracle: the recreated spelling is live through
            // absolute lookup, but the old activation retains its dead token.
            assert_eq!(
                i.result_bytes(),
                b"{::N 0 1} 1 {namespace \"\" not found in \"::N\"} \
                  {TCL LOOKUP NAMESPACE {}} 0 ::"
            );
        });
    }

    #[test]
    fn namespace_teardown_sweeps_callback_created_commands_and_imports() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc qdel {old new op} {
                          set c [catch {$old} result]
                          lappend ::seen [list qdel $old $c $result]
                      }
                      proc cb {old new op} {
                          proc ::N::q {} {return Q}
                          trace add command ::N::q delete qdel
                          namespace eval ::I {namespace import ::N::q}
                          lappend ::seen [list callback [namespace exists ::N] \
                              [::N::q] [::I::q] [namespace origin ::I::q]]
                      }
                      namespace eval I {}
                      namespace eval N {
                          namespace export q
                          proc p {} {return P}
                      }
                      trace add command ::N::p delete cb
                      namespace delete ::N
                      set first $seen
                      set absent [list [namespace exists ::N] \
                          [info commands ::N::q] [info commands ::I::q] \
                          [namespace eval I {namespace import}]]
                      namespace eval N {proc q {} {return NEW}}
                      rename ::N::q {}
                      list $first $absent $seen [namespace exists ::N] \
                          [info commands ::I::q]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{{callback 0 Q Q ::N::q} {qdel ::N::q 0 Q}} \
                  {0 {} {} {}} \
                  {{callback 0 Q Q ::N::q} {qdel ::N::q 0 Q}} 1 {}"
            );
        });
    }

    #[test]
    fn descendant_delete_callback_creates_a_fresh_visible_namespace_tree() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc cb {old new op} {
                          namespace eval ::N::C::X {
                              proc p {} {return P}
                          }
                      }
                      namespace eval ::N::C {
                          namespace export q
                          proc q {} {return Q}
                      }
                      namespace eval ::I {namespace import ::N::C::q}
                      trace add command ::N::C::q delete cb
                      namespace delete ::N
                      list [namespace exists ::N] \
                          [info commands ::N::C::X::p] [::N::C::X::p] \
                          [info commands ::I::q] \
                          [namespace eval I {namespace import}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"1 ::N::C::X::p P {} {}");
        });
    }

    /// C tears a namespace down one entity at a time, completing each
    /// variable's whole trace list before the next, so **interleaved**
    /// registrations still fire as contiguous per-variable groups. This is the
    /// shape a flat reverse gets wrong: `A1 B1 A2 B2` must fire `A2 A1 B2 B1`,
    /// not `B2 A2 B1 A1`. tclsh 8.6.16 and 9.0.4 agree. Issue #1440.
    #[test]
    fn namespace_teardown_groups_interleaved_variable_traces() {
        leak_free(|i| {
            ok(i, b"set ::log {}");
            ok(i, b"proc rec {label args} {lappend ::log $label}");
            ok(i, b"namespace eval ::nsv {variable a 1\nvariable b 2}");
            ok(i, b"trace add variable ::nsv::a unset {rec A1}");
            ok(i, b"trace add variable ::nsv::b unset {rec B1}");
            ok(i, b"trace add variable ::nsv::a unset {rec A2}");
            ok(i, b"trace add variable ::nsv::b unset {rec B2}");
            ok(i, b"namespace delete ::nsv");
            assert_eq!(ok(i, b"set ::log"), b"A2 A1 B2 B1");
            i.eval_str(b"unset -nocomplain ::log");
        });
    }

    /// The same grouping for the command `delete` traces the teardown collects
    /// alongside them: `X1 Y1 X2 Y2` fires `X2 X1 Y2 Y1`. Issue #1440.
    #[test]
    fn namespace_teardown_groups_interleaved_command_traces() {
        leak_free(|i| {
            ok(i, b"set ::log {}");
            ok(i, b"proc rec {label args} {lappend ::log $label}");
            ok(i, b"namespace eval ::nsc {proc x {} {}\nproc y {} {}}");
            ok(i, b"trace add command ::nsc::x delete {rec X1}");
            ok(i, b"trace add command ::nsc::y delete {rec Y1}");
            ok(i, b"trace add command ::nsc::x delete {rec X2}");
            ok(i, b"trace add command ::nsc::y delete {rec Y2}");
            ok(i, b"namespace delete ::nsc");
            assert_eq!(ok(i, b"set ::log"), b"X2 X1 Y2 Y1");
            i.eval_str(b"unset -nocomplain ::log");
        });
    }

    /// Grouping holds when a teardown callback re-enters the interpreter — the
    /// callbacks run after the namespace is already gone, so a callback that
    /// reads or writes global state must not disturb the remaining groups.
    #[test]
    fn namespace_teardown_grouping_survives_a_re_entrant_callback() {
        leak_free(|i| {
            ok(i, b"set ::log {}");
            ok(i, b"set ::side 0");
            ok(
                i,
                b"proc rec {label args} {lappend ::log $label; incr ::side; \
                  catch {namespace exists ::nsr}}",
            );
            ok(i, b"namespace eval ::nsr {variable a 1\nvariable b 2}");
            ok(i, b"trace add variable ::nsr::a unset {rec A1}");
            ok(i, b"trace add variable ::nsr::b unset {rec B1}");
            ok(i, b"trace add variable ::nsr::a unset {rec A2}");
            ok(i, b"trace add variable ::nsr::b unset {rec B2}");
            ok(i, b"namespace delete ::nsr");
            assert_eq!(ok(i, b"set ::log"), b"A2 A1 B2 B1");
            assert_eq!(ok(i, b"set ::side"), b"4");
            i.eval_str(b"unset -nocomplain ::log ::side");
        });
    }

    /// `trace info` renders the stored op set in the order each C `TRACE_INFO`
    /// arm tests the flag bits — `array read write unset` and `rename delete`,
    /// neither of which is the `opStrings[]` table order the bad-operation
    /// error enumerates.
    #[test]
    fn trace_info_renders_ops_in_cs_fixed_order() {
        leak_free(|i| {
            ok(i, b"proc cb args {}");
            ok(i, b"proc p {} {}");
            ok(i, b"trace add command p {delete rename} cb");
            ok(
                i,
                b"trace add execution p {leavestep leave enterstep enter} cb",
            );
            ok(i, b"trace add variable q {unset write read array} cb");
            assert_eq!(ok(i, b"trace info command p"), b"{{rename delete} cb}");
            assert_eq!(
                ok(i, b"trace info execution p"),
                b"{{enter leave enterstep leavestep} cb}"
            );
            assert_eq!(
                ok(i, b"trace info variable q"),
                b"{{array read write unset} cb}"
            );
            i.eval_str(b"unset -nocomplain q");
        });
    }

    /// The deprecated 8.x forms (issue #1444): `rwua`-only validation with C's
    /// error text, duplicate letters collapsed, a set the modern `trace
    /// remove variable` matches (and vice versa), `trace vinfo` rendering the
    /// letters in C's fixed `r`,`w`,`u`,`a` order, and the callback receiving
    /// the single letter rather than the operation name
    /// (`TCL_TRACE_OLD_STYLE`). Every expectation is tclsh 8.6.16-pinned; the
    /// 9.0 side lives in the `dialect_gate` test below.
    #[test]
    fn legacy_variable_trace_forms_match_c() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            ok(i, b"set ::log {}");
            ok(i, b"proc cb args {lappend ::log [lindex $args end]}");
            ok(i, b"proc cb2 args {}");
            assert_eq!(
                err(i, b"trace variable x q cb"),
                b"bad operations \"q\": should be one or more of rwua"
            );
            assert_eq!(
                err(i, b"trace vdelete x {read write} cb"),
                b"bad operations \"read write\": should be one or more of rwua"
            );
            assert_eq!(
                err(i, b"trace variable x"),
                b"wrong # args: should be \"trace variable name ops command\""
            );
            assert_eq!(
                err(i, b"trace vdelete x"),
                b"wrong # args: should be \"trace vdelete name ops command\""
            );
            assert_eq!(
                err(i, b"trace vinfo x y"),
                b"wrong # args: should be \"trace vinfo name\""
            );

            // Repeated letters collapse, and `trace var` abbreviates.
            ok(i, b"trace variable x rrw cb");
            assert_eq!(ok(i, b"trace vinfo x"), b"{rw cb}");
            assert_eq!(ok(i, b"trace info variable x"), b"{{read write} cb}");
            ok(i, b"trace var x w cb2");
            assert_eq!(ok(i, b"trace vinfo x"), b"{w cb2} {rw cb}");

            // Letter order does not matter to the removal match, and the two
            // spellings remove each other's registrations.
            ok(i, b"trace vdelete x wr cb");
            assert_eq!(ok(i, b"trace vinfo x"), b"{w cb2}");
            ok(i, b"trace add variable y {write read} cb");
            ok(i, b"trace vdelete y rw cb");
            assert_eq!(ok(i, b"trace vinfo y"), b"");

            // An old-style callback is invoked with the `rwua` letter.
            ok(i, b"trace variable z rwua cb");
            assert_eq!(ok(i, b"trace vinfo z"), b"{rwua cb}");
            ok(i, b"set z 1");
            ok(i, b"set ignore $z");
            assert_eq!(ok(i, b"set ::log"), b"w r");
            // …while a modern registration still gets the full word.
            ok(i, b"set ::log {}");
            ok(i, b"trace add variable m write cb");
            ok(i, b"set m 1");
            assert_eq!(ok(i, b"set ::log"), b"write");
            i.eval_str(b"unset -nocomplain x y z m ignore ::log");
        });
    }

    /// Where #1444's letter convention meets the teardown path: a trace
    /// installed the deprecated way still receives the `rwua` **letter** when
    /// it is fired by `namespace delete` rather than an explicit `unset`.
    /// Teardown collects callbacks into a reduced list, so the flag has to be
    /// carried through that reduction. tclsh 8.6.16 gives `L:u` for the legacy
    /// registration, `M:unset` for a modern one, and — with both on one
    /// variable, newest-first — `Mz:unset Lz:u`.
    #[test]
    fn legacy_letter_survives_namespace_teardown() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            ok(i, b"set ::log {}");
            ok(i, b"proc rec {label n1 n2 op} {lappend ::log $label:$op}");

            ok(
                i,
                b"namespace eval ::legacy {variable x 1\ntrace variable x u {rec L}}",
            );
            ok(i, b"namespace delete ::legacy");
            assert_eq!(ok(i, b"set ::log"), b"L:u");

            ok(i, b"set ::log {}");
            ok(
                i,
                b"namespace eval ::modern {variable y 1\ntrace add variable y unset {rec M}}",
            );
            ok(i, b"namespace delete ::modern");
            assert_eq!(ok(i, b"set ::log"), b"M:unset");

            // Both conventions on one variable: each callback keeps its own.
            ok(i, b"set ::log {}");
            ok(
                i,
                b"namespace eval ::both {variable z 1\ntrace variable z u {rec Lz}\ntrace add variable z unset {rec Mz}}",
            );
            ok(i, b"namespace delete ::both");
            assert_eq!(ok(i, b"set ::log"), b"Mz:unset Lz:u");

            // An array element registered the legacy way behaves the same.
            ok(i, b"set ::log {}");
            ok(
                i,
                b"namespace eval ::arr {variable a\nset a(k) 1\ntrace variable a(k) u {rec A}}",
            );
            ok(i, b"namespace delete ::arr");
            assert_eq!(ok(i, b"set ::log"), b"A:u");
            i.eval_str(b"unset -nocomplain ::log");
        });
    }

    /// The 9.x side of the same seam: the legacy form does not exist there, so
    /// a modern registration fired by teardown must still say `unset`.
    #[test]
    fn teardown_op_word_is_the_full_word_at_9x() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            ok(i, b"set ::log {}");
            ok(i, b"proc rec {label n1 n2 op} {lappend ::log $label:$op}");
            ok(
                i,
                b"namespace eval ::modern {variable y 1\ntrace add variable y unset {rec M}}",
            );
            ok(i, b"namespace delete ::modern");
            assert_eq!(ok(i, b"set ::log"), b"M:unset");
            assert_eq!(
                err(i, b"trace variable q u {rec L}"),
                b"bad option \"variable\": must be add, info, or remove"
            );
            i.eval_str(b"unset -nocomplain ::log");
        });
    }

    /// The registry retires the three legacy forms at 9.0, and the runtime
    /// reads that rather than carrying its own list — so the same script is a
    /// working trace at 8.x and `bad option` at 9.x, with the option
    /// enumeration following too. Issue #1444.
    #[test]
    fn legacy_variable_trace_forms_follow_the_release() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(
                err(i, b"trace zzz"),
                b"bad option \"zzz\": must be add, info, remove, variable, vdelete, or vinfo"
            );
            assert_eq!(
                err(i, b"trace v x"),
                b"ambiguous option \"v\": must be add, info, remove, variable, vdelete, or vinfo"
            );
        });
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            for word in [&b"variable"[..], b"vdelete", b"vinfo", b"var"] {
                let mut line = b"trace ".to_vec();
                line.extend_from_slice(word);
                line.extend_from_slice(b" x w cb");
                let mut want = b"bad option \"".to_vec();
                want.extend_from_slice(word);
                want.extend_from_slice(b"\": must be add, info, or remove");
                assert_eq!(err(i, &line), want);
            }
            assert_eq!(
                err(i, b"trace zzz"),
                b"bad option \"zzz\": must be add, info, or remove"
            );
        });
    }
}
