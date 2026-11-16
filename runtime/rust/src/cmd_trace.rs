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
//! keyed by the selected command generation and its current report.
//!
//! The deprecated 8.x `trace variable|vdelete|vinfo` forms are supported too.
//! C compiles them behind `#ifndef TCL_REMOVE_OBSOLETE_TRACES` and Tcl 9.0
//! dropped them, so the option word is resolved against the option set the
//! registry declares for the emulated release rather than a fixed list.

use tcl_cmd_core::trace as core_trace;
use tcl_runtime_api::Namespaces;
use tcl_syntax::value::ValueOps;

use crate::frame::VarError;
use crate::interp::native_arguments::OriginalArguments;
use crate::interp::{new_string, Code, Interp};
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
    /// Physical root generation owning this registration.
    pub binding_id: Option<tcl_runtime_api::VarId>,
    /// Physical member allocation, independent of a reused key in the same root.
    pub element_binding_id: Option<tcl_runtime_api::VarId>,
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
    /// Direct native callback, hidden from script trace queries and removal.
    pub native: Option<(
        tcl_runtime_api::native_variable_trace::NativeVariableTraceToken,
        std::rc::Rc<
            dyn tcl_runtime_api::native_variable_trace::NativeVariableObserver<
                Interp,
                Error = tcl_cmd_core::CmdError,
            >,
        >,
    )>,
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
    ///
    /// Distinct from [`Self::token`], and deliberately so: the token says which
    /// *list* an entry belongs to — which entries a deletion frees — while the
    /// id says which *registration* a walk is about to run, so an explicit
    /// untrace can cancel it. A delete walk needs both: it fires the dying
    /// token's list to the end even as a callback rebinds the name, and still
    /// skips whatever that callback untraced.
    ///
    /// It is also C's `TraceCommandInfo *` for the purpose its
    /// `TCL_TRACE_EXEC_IN_PROGRESS` flag hangs off: suppression while a
    /// callback runs is **per trace**, not per command, so a second trace on
    /// the same command still fires for a call the first one's callback makes
    /// (see [`TraceTable::firing_exec_traces`]).
    pub id: u64,
    /// The generation of the command **token** this trace hangs off, or `None`
    /// when an internal caller had no bound token to identify.
    ///
    /// C keeps the list on the `Command` itself and frees exactly that list
    /// when the token dies. Our registry is keyed by FQN, so a delete callback
    /// that binds a replacement at the same name registers on a *different*
    /// token under the same key; only the generation tells the two lists
    /// apart. Generations are minted in binding order across the whole
    /// interpreter, so a trace on a token installed after the dying one always
    /// compares greater — including when the two tokens live in a retained
    /// namespace table and a same-named recreation of it.
    pub token: Option<u64>,
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
    /// Exact command generation whose trace installed this step observer.
    pub token: Option<u64>,
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
/// different cell. Only a write to the *same* cell — or, for the
/// whole-array traces specifically, one reached while the array's own cell is
/// active — is suppressed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VarTraceScope {
    binding_id: Option<tcl_runtime_api::VarId>,
    element_binding_id: Option<tcl_runtime_api::VarId>,
    base: Vec<u8>,
    elem: Option<Vec<u8>>,
    frame_level: Option<usize>,
    ns: Option<NsId>,
}

impl VarTraceScope {
    /// The cell an access to `(base, elem)` at this home reaches.
    pub(crate) fn cell(
        home: &crate::vars::TraceHome,
        elem: Option<&[u8]>,
        element_binding_id: Option<tcl_runtime_api::VarId>,
    ) -> Self {
        Self {
            binding_id: home.binding_id,
            element_binding_id,
            base: home.base.clone(),
            elem: elem.map(<[u8]>::to_vec),
            frame_level: home.level,
            ns: home.ns,
        }
    }

    pub(crate) fn member_identity(&self) -> Option<tcl_runtime_api::VarId> {
        self.element_binding_id
    }

    fn owns_root(&self, trace: &VarTrace) -> bool {
        self.binding_id.is_some()
            && trace.binding_id == self.binding_id
            && same_variable(trace, &self.base, self.ns, self.frame_level)
    }

    /// Registration queries and disposal select the same actual cell as firing.
    pub(crate) fn owns_registration(&self, trace: &VarTrace) -> bool {
        self.owns_root(trace)
            && trace.elem == self.elem
            && trace.element_binding_id == self.element_binding_id
    }

    pub(crate) fn matches(&self, trace: &VarTrace, operation: &[u8]) -> bool {
        self.owns_root(trace)
            && (trace.elem.is_none() || self.owns_registration(trace))
            && trace.ops.iter().any(|candidate| candidate == operation)
    }

    /// The containing array's own cell — C's `arrayPtr`, whose `VAR_TRACE_ACTIVE`
    /// gates the whole-array traces separately from the element's.
    pub(crate) fn array(&self) -> Self {
        Self {
            elem: None,
            element_binding_id: None,
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
    /// Ids `trace remove` has unlinked while a walk was in flight. The
    /// **delete** walk holds the dying token's list, so a callback that
    /// re-creates the command under the same name takes the name-keyed table
    /// entry over without cancelling the remaining callbacks; only an explicit
    /// untrace does. Recorded only while a walk is in flight
    /// ([`Self::trace_walk_in_flight`]) and cleared when the outermost one
    /// ends, so it never grows unbounded.
    pub untraced_cmd_trace_ids: Vec<u64>,
    /// The execution traces whose callbacks are currently running — C's
    /// per-trace `TCL_TRACE_EXEC_IN_PROGRESS` (`tclTrace.c` 9.0.4:1655,
    /// "Inside any kind of execution trace callback, we do not allow any
    /// further execution trace callbacks to be called for the same trace").
    /// This is what bounds a callback that invokes the command it traces;
    /// [`Self::exec_firing`] deliberately does not, because it gates only the
    /// step machinery. Distinct from [`Self::untraced_cmd_trace_ids`], which
    /// records registrations a callback *removed* rather than ones it is
    /// running.
    pub firing_exec_traces: Vec<u64>,
    /// Non-zero while an **execution** trace callback (`enter`/`leave`/
    /// `enterstep`/`leavestep`) is running — C's `INTERP_TRACE_IN_PROGRESS`,
    /// which `TraceExecutionProc` sets around exactly those callbacks
    /// (tclTrace.c 9.0.4:1765) and nothing else sets. A `rename`/`delete`
    /// callback does **not** raise it: C's `CallCommandTraces` sets no flag, so
    /// a command such a callback dispatches is traced like any other.
    ///
    /// It is read exactly where C reads its flag — `TclCheckInterpTraces`
    /// (:1426), the `enterstep`/`leavestep` machinery — and nowhere else:
    /// `TclCheckExecutionTraces` (:1301) never consults it, so a command
    /// dispatched from inside a callback still fires its own `enter`/`leave`
    /// traces. Re-entry into the *same* trace is bounded by
    /// [`Self::firing_exec_traces`], and into a command's own `rename`/`delete`
    /// traces by [`Self::firing_cmd_traces`] — which is also what marks such a
    /// walk in flight (see [`Self::trace_walk_in_flight`]).
    pub exec_firing: usize,
    /// The commands whose `rename`/`delete` traces are currently firing. C
    /// guards those per `Command` (`CMD_TRACE_ACTIVE`, and `CMD_DYING` for a
    /// deletion), not interpreter-wide: a callback that deletes a *different*
    /// command still fires that command's own delete traces, nested.
    pub firing_cmd_traces: Vec<(Vec<u8>, Option<u64>)>,
    /// The error message a read/write variable-trace callback left, captured so
    /// the variable access can fail with `can't read/set "name": <msg>` (C's
    /// `TclCallVarTraces` propagation). Taken by the access chokepoint.
    pub pending_err: Option<Vec<u8>>,
    /// The binding of the global `env` array that carries the native trace
    /// keeping it in step with the host environment (C's `EnvTraceProc`), or
    /// `None` where there is none (a safe or confined interpreter, or after
    /// `unset env`). Kept apart from [`Self::traces`] because scripts can
    /// neither see nor remove it, and so a variable access that cannot reach
    /// `env` still finds no traces at all (see
    /// [`crate::interp::Interp::traces_may_observe`]).
    pub env: Option<tcl_runtime_api::VarId>,
    /// Whether a link whose target is named `env` was ever made under a
    /// different local name (`upvar #0 env e`). Until then only a spelling
    /// whose tail is `env` can reach the global `env` array.
    pub env_aliased: bool,
}

impl TraceTable {
    /// Whether a command or execution trace walk is in flight — the window in
    /// which a `trace remove` has to be recorded so the walk skips the
    /// registration it unlinked (see [`Self::untraced_cmd_trace_ids`]).
    ///
    /// Two pieces of state, because the two walks are marked differently: an
    /// execution callback raises [`Self::exec_firing`], C's
    /// `INTERP_TRACE_IN_PROGRESS`, while a `rename`/`delete` walk deliberately
    /// raises nothing of the sort — C's `CallCommandTraces` sets no interpreter
    /// flag — and is marked only by its entry in [`Self::firing_cmd_traces`].
    pub fn trace_walk_in_flight(&self) -> bool {
        self.exec_firing > 0 || !self.firing_cmd_traces.is_empty()
    }
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

/// Register `trace`.
pub fn install(interp: &mut Interp) {
    interp.register_stock_builtin(b"trace", trace_cmd);
}

/// The variable resolver supplies the reason, while `trace` owns the command
/// verb in its diagnostic (`can't trace`, not the usual `can't set`).
pub(crate) fn trace_var_error(interp: &mut Interp, name: &[u8], error: VarError) -> Code {
    if error == VarError::NameProtocolUnavailable {
        return interp.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "variable naming",
            ),
        );
    }
    let reason = match error {
        VarError::NameProtocolUnavailable => unreachable!("host refusal handled above"),
        VarError::IsScalar => b"variable isn't array".as_slice(),
        VarError::IsArray => b"variable is array".as_slice(),
        VarError::NoSuchNamespace => b"parent namespace doesn't exist".as_slice(),
        VarError::DeletedNamespace => b"upvar refers to variable in deleted namespace".as_slice(),
        VarError::DeletedArray => b"upvar refers to element in deleted array".as_slice(),
        VarError::IsConstant => b"variable is a constant".as_slice(),
        VarError::TraceError => b"trace callback failed".as_slice(),
        VarError::Confined => b"stores are confined to the activation".as_slice(),
    };
    let mut message = b"can't trace \"".to_vec();
    message.extend_from_slice(name);
    message.extend_from_slice(b"\": ");
    message.extend_from_slice(reason);
    interp.set_error(&message)
}

fn trace_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = trace_cmd_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn trace_cmd_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"trace option ?arg ...?");
    }
    let protocol = interp.invocation_string_protocol();
    let selected = core_trace::resolve_option_original(interp, &argv[1], protocol);
    if let Err(error) = arguments.check() {
        return interp.report_cmd_error(error);
    }
    let option = match selected {
        Ok(option) => option,
        Err(e) => return interp.report_cmd_error(e),
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
            let selected = core_trace::resolve_type_original(interp, &argv[2]);
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
            match selected {
                Ok(core_trace::TraceKind::Variable) => trace_var_add_remove(interp, argv, is_add),
                Ok(core_trace::TraceKind::Command) => {
                    cmd_trace_add_remove(interp, argv, is_add, ops::CMD_ANY)
                }
                Ok(core_trace::TraceKind::Execution) => {
                    cmd_trace_add_remove(interp, argv, is_add, ops::EXEC_ANY)
                }
                Err(e) => interp.report_cmd_error(e),
            }
        }
        b"info" => {
            if argv.len() < 3 {
                return interp.wrong_args(b"trace info type name");
            }
            let selected = core_trace::resolve_type_original(interp, &argv[2]);
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
            match selected {
                Ok(core_trace::TraceKind::Variable) => trace_var_info(interp, argv),
                Ok(core_trace::TraceKind::Command) => cmd_trace_info(interp, argv, ops::CMD_ANY),
                Ok(core_trace::TraceKind::Execution) => cmd_trace_info(interp, argv, ops::EXEC_ANY),
                Err(e) => interp.report_cmd_error(e),
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
        _ => unreachable!("selected trace declaration is exhaustive"),
    }
}

// command / execution traces

/// Parse an execution-trace op list into a [`ops`] bitset, via the shared core
/// (split + validation + the catalogue) then folding the canonical names to bits.
fn parse_exec_ops(interp: &mut Interp, spec: *mut TclObj) -> Result<u8, Code> {
    let names = core_trace::parse_ops_original(interp, &spec, core_trace::TraceKind::Execution)
        .map_err(|e| interp.report_cmd_error(e))?;
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
fn parse_cmd_ops(interp: &mut Interp, spec: *mut TclObj) -> Result<u8, Code> {
    let names = core_trace::parse_ops_original(interp, &spec, core_trace::TraceKind::Command)
        .map_err(|e| interp.report_cmd_error(e))?;
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
    let kind = if category == ops::EXEC_ANY {
        core_trace::TraceKind::Execution
    } else {
        core_trace::TraceKind::Command
    };
    if argv.len() != 6 {
        let mut usage = if is_add {
            b"trace add ".to_vec()
        } else {
            b"trace remove ".to_vec()
        };
        usage.extend_from_slice(kind.canonical_name().as_bytes());
        usage.extend_from_slice(b" name opList command");
        return interp.wrong_args(&usage);
    }
    let spec = argv[4];
    let flags = match if category == ops::EXEC_ANY {
        parse_exec_ops(interp, spec)
    } else {
        parse_cmd_ops(interp, spec)
    } {
        Ok(f) => f,
        Err(c) => return c,
    };
    let Some(protocol) = interp
        .native_invocation_dialect()
        .native_variable_trace_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("command trace protocol")
                .into(),
        );
    };
    // Native command/execution trace registration reaches the prefix getter first.
    let command = match interp.native_string_bytes(&argv[5]) {
        Ok(command) => command,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let name = match interp.native_string_bytes(&argv[3]) {
        Ok(name) => name,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    // Both add and remove require the command to exist (C's `Tcl_TraceCommand`
    // / `Tcl_FindCommand` with `TCL_LEAVE_ERR_MSG`).
    let (fqn, generation) = match checked_command_trace_binding(interp, &name) {
        Ok(Some(binding)) => binding,
        Ok(None) => return interp.unknown_command(&name),
        Err(code) => return code,
    };
    let token = Some(generation);
    if is_add {
        // The trace belongs to the token standing at `fqn` now, not to the
        // name (C hangs it off `cmdPtr->tracePtr`).
        let mut traces = interp.traces.borrow_mut();
        let first_execution = flags & ops::EXEC_ANY != 0
            && !traces
                .cmd_traces
                .iter()
                .any(|trace| trace.token == token && trace.ops & ops::EXEC_ANY != 0);
        traces.next_cmd_trace_id += 1;
        let id = traces.next_cmd_trace_id;
        traces.cmd_traces.push(CmdTrace {
            id,
            token,
            name: fqn,
            ops: flags,
            command: command.to_vec(),
        });
        drop(traces);
        if first_execution {
            note_execution_trace_boundary(interp, generation);
        }
        interp.invalidate_guard_domain(tcl_runtime_api::guard::GuardDomain::CommandTrace);
    } else {
        // Remove the first trace matching exact ops + command string, where
        // "first" is C's `FOREACH_COMMAND_TRACE` head→tail order and its head
        // is the newest registration — so among duplicates the newest goes.
        // Our Vec is oldest-first, hence `rposition`.
        let pos = interp.traces.borrow().cmd_traces.iter().rposition(|t| {
            t.name == fqn
                && t.token == token
                && t.ops == flags
                && protocol.command_prefix_matches(&t.command, &command)
        });
        if let Some(i) = pos {
            let mut traces = interp.traces.borrow_mut();
            if traces.trace_walk_in_flight() {
                let id = traces.cmd_traces[i].id;
                traces.untraced_cmd_trace_ids.push(id);
            }
            traces.cmd_traces.remove(i);
            let last_execution = flags & ops::EXEC_ANY != 0
                && !traces
                    .cmd_traces
                    .iter()
                    .any(|trace| trace.token == token && trace.ops & ops::EXEC_ANY != 0);
            drop(traces);
            if last_execution {
                note_execution_trace_boundary(interp, generation);
            }
            interp.invalidate_guard_domain(tcl_runtime_api::guard::GuardDomain::CommandTrace);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn note_execution_trace_boundary(interp: &Interp, generation: u64) {
    use tcl_registry::native_procedure::NativeCompilerCacheMutation;
    use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
    let mut namespaces = interp.namespaces_mut();
    let hook = namespaces
        .native_compiler_hook(generation)
        .unwrap_or(NativeCompilerHookPresence::Unknown);
    namespaces
        .note_native_compiler_mutation(None, NativeCompilerCacheMutation::ExecutionTrace { hook });
}

fn checked_command_trace_binding(
    interp: &mut Interp,
    name: &[u8],
) -> Result<Option<(Vec<u8>, u64)>, Code> {
    let command = interp
        .find_command_bytes_checked(Namespaces::current(interp), name)
        .map_err(|error| interp.report_cmd_error(error.into()))?;
    command
        .map(|command| {
            let generation = interp
                .command_identity(command.0)
                .map(|(_, generation)| generation);
            generation
                .and_then(|generation| {
                    interp
                        .namespaces()
                        .native_command_slot_at_node(generation)?;
                    let report = interp.command_name_bytes(command)?;
                    Some((report, generation))
                })
                .ok_or_else(|| {
                    interp.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "command trace selected binding",
                        )
                        .into(),
                    )
                })
        })
        .transpose()
}

/// `trace info command|execution name` — the matching traces, most-recent
/// first, each a `{opList command}` pair. Ops printed in C's fixed order.
fn cmd_trace_info(interp: &mut Interp, argv: &[*mut TclObj], category: u8) -> Code {
    let kind = if category == ops::EXEC_ANY {
        core_trace::TraceKind::Execution
    } else {
        core_trace::TraceKind::Command
    };
    if argv.len() != 4 {
        let mut usage = b"trace info ".to_vec();
        usage.extend_from_slice(kind.canonical_name().as_bytes());
        usage.extend_from_slice(b" name");
        return interp.wrong_args(&usage);
    }
    let Some(protocol) = interp
        .native_invocation_dialect()
        .native_variable_trace_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("command trace protocol")
                .into(),
        );
    };
    let name = match interp.native_string_bytes(&argv[3]) {
        Ok(name) => name,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let (fqn, generation) = match checked_command_trace_binding(interp, &name) {
        Ok(Some(binding)) => binding,
        Ok(None) => return interp.unknown_command(&name),
        Err(code) => return code,
    };
    let token = Some(generation);
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
        if t.name != fqn || t.token != token || (t.ops & category) == 0 {
            continue;
        }
        let op_objs: Vec<*mut TclObj> = order
            .iter()
            .filter(|(bit, _)| (t.ops & bit) != 0)
            .map(|(_, label)| new_string(label))
            .collect();
        let ops_list = interp.new_list_object(&op_objs);
        let cmd = new_string(protocol.command_prefix_report(&t.command));
        entries.push(interp.new_list_object(&[ops_list, cmd]));
    }
    interp.set_result(interp.new_list_object(&entries));
    Code::Ok
}

/// Parse and validate a variable-trace ops list (`{read write unset array}`),
/// via the shared core (the op catalogue lives once in `tcl-cmd-core::trace`).
fn parse_ops(interp: &mut Interp, spec: *mut TclObj) -> Result<Vec<Vec<u8>>, Code> {
    match core_trace::parse_ops_original(interp, &spec, core_trace::TraceKind::Variable) {
        Ok(ops) => Ok(ops.iter().map(|o| o.as_bytes().to_vec()).collect()),
        Err(e) => Err(interp.report_cmd_error(e)),
    }
}

// variable traces

/// `trace add|remove variable name ops command`.
fn trace_var_add_remove(interp: &mut Interp, argv: &[*mut TclObj], is_add: bool) -> Code {
    if argv.len() != 6 {
        return interp.wrong_args(if is_add {
            b"trace add variable name opList command"
        } else {
            b"trace remove variable name opList command"
        });
    }
    let ops = match parse_ops(interp, argv[4]) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let command = match interp.native_string_bytes(&argv[5]) {
        Ok(command) => command,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let name = match interp.native_string_bytes(&argv[3]) {
        Ok(name) => name,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    var_trace_apply(interp, name.to_vec(), ops, command.to_vec(), is_add, false)
}

/// The resolved `(home namespace, home frame level, simple base, element)` a
/// `trace info|remove variable name` question is about.
///
/// C looks the variable up and walks *that* `Var`'s trace list, so every
/// spelling of the one cell — `v` and `::v`, an `upvar` alias and its target —
/// sees the same traces. Matching the registration spelling textually instead
/// makes `trace info variable alias` report nothing for a trace the very next
/// `set alias` fires.
fn var_trace_query(interp: &Interp, name: &[u8]) -> Result<crate::vars::TraceHome, VarError> {
    let protocol = interp.require_variable_name_protocol()?;
    let input = protocol
        .trace_query_input(name)
        .map_err(|_| VarError::NameProtocolUnavailable)?;
    let (base, elem) = interp.variable_name_parts(input.selected())?;
    let mut home = interp.trace_identity(&base);
    // An element alias selects the same physical registration list as the
    // element spelling, even while that array generation is detached.
    home.link_elem = elem.or(home.link_elem);
    Ok(home)
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
    let operation =
        match crate::interp::native_operation_currency::NativeOperationScope::enter(interp) {
            Ok(operation) => operation,
            Err(first) => return interp.refuse_native_execution(first),
        };
    let Some(prefix_protocol) = interp
        .native_invocation_dialect()
        .native_variable_trace_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable trace prefix protocol",
            )
            .into(),
        );
    };
    if is_add {
        let protocol = match interp.require_variable_name_protocol() {
            Ok(protocol) => protocol,
            Err(error) => return trace_var_error(interp, &name, error),
        };
        let input = match protocol.trace_registration_input(&name) {
            Ok(input) => input,
            Err(_) => return trace_var_error(interp, &name, VarError::NameProtocolUnavailable),
        };
        let (base, spelled_elem) = match interp.variable_name_parts(input.selected()) {
            Ok(parts) => parts,
            Err(error) => return trace_var_error(interp, &name, error),
        };
        // An alias for an array *element* (`upvar #0 a(k) e`) is a trace on that
        // element: C hangs it off the element's `Var`, which the alias and the
        // spelling `a(k)` share, so `trace add variable e …` and
        // `trace add variable a(k) …` install one and the same trace and each
        // spelling's `trace info` reports it. The element is invisible in the
        // spelling, so it comes from the resolution.
        let home = match interp.prepare_trace_registration(&base, spelled_elem.as_deref(), None) {
            Ok(home) => home,
            Err(error) => return trace_var_error(interp, &name, error),
        };
        let elem = home.link_elem.clone();
        let cell = interp.variable_trace_scope(&home, elem.as_deref());
        let mut table = interp.traces.borrow_mut();
        let id = table.next_var_trace_id;
        table.next_var_trace_id += 1;
        table.traces.push(VarTrace {
            id,
            binding_id: home.binding_id,
            element_binding_id: cell.member_identity(),
            name,
            base: home.base,
            elem,
            ops,
            command: prefix_protocol.variable_prefix_storage(&command).to_vec(),
            native: None,
            frame_level: home.level,
            ns: home.ns,
            old_style,
        });
        drop(table);
        interp.invalidate_guard_domain(tcl_runtime_api::guard::GuardDomain::VariableTrace);
    } else {
        let query = match var_trace_query(interp, &name) {
            Ok(query) => query,
            Err(error) => return trace_var_error(interp, &name, error),
        };
        let cell = interp.variable_trace_scope(&query, query.link_elem.as_deref());
        let pos = interp
            .traces
            .borrow()
            .traces
            .iter()
            // Newest-first first match: C breaks at the first hit walking its
            // list head→tail, and the head is the newest registration. Our Vec
            // is oldest-first, hence `rposition`. `old_style` is deliberately
            // absent from the match, as C masks `TCL_TRACE_OLD_STYLE` out
            // here.
            .rposition(|t| {
                cell.owns_registration(t)
                    && t.ops == ops
                    && t.native.is_none()
                    && prefix_protocol.variable_prefix_matches(&t.command, &command)
            });
        if let Some(i) = pos {
            let removed = interp.traces.borrow_mut().traces.remove(i);
            interp.invalidate_guard_domain(tcl_runtime_api::guard::GuardDomain::VariableTrace);
            let traced = interp
                .traces
                .borrow()
                .traces
                .iter()
                .any(|trace| cell.owns_registration(trace));
            let reached = if !traced {
                interp.cleanup_trace_shell(&query, query.link_elem.as_deref(), || {
                    operation
                        .currency()
                        .ensure_current_or_refuse()
                        .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
                })
            } else {
                operation
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
            };
            drop(removed);
            let checked = operation
                .currency()
                .ensure_current_or_refuse()
                .map_err(tcl_cmd_core::CmdError::from_execution_refusal);
            if let Err(error) = checked.and(reached) {
                return interp.report_cmd_error(error);
            }
        }
    }
    if let Err(first) = operation.currency().ensure_current_or_refuse() {
        return interp.refuse_native_execution(first);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `trace info variable name` — the registered traces, most-recent first, each
/// as a `{opList command}` pair.
fn trace_var_info(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = trace_var_info_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn trace_var_info_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"trace info variable name");
    }
    let name = match arguments.bytes(3) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    let query = match var_trace_query(interp, &name) {
        Ok(query) => query,
        Err(error) => return trace_var_error(interp, &name, error),
    };
    let Some(prefix_protocol) = interp
        .native_invocation_dialect()
        .native_variable_trace_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable trace prefix report",
            )
            .into(),
        );
    };
    let cell = interp.variable_trace_scope(&query, query.link_elem.as_deref());
    let mut entries: Vec<*mut TclObj> = Vec::new();
    for t in interp.traces.borrow().traces.iter().rev() {
        if t.native.is_some() || !cell.owns_registration(t) {
            continue;
        }
        let op_objs: Vec<*mut TclObj> = t.ops.iter().map(|o| new_string(o)).collect();
        let ops_list = interp.new_list_object(&op_objs);
        let cmd = new_string(prefix_protocol.variable_prefix_report(&t.command));
        entries.push(interp.new_list_object(&[ops_list, cmd]));
    }
    interp.set_result(interp.new_list_object(&entries));
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
    let ops = match core_trace::parse_legacy_variable_ops_original(interp, &argv[3]) {
        Ok(ops) => ops.iter().map(|o| o.as_bytes().to_vec()).collect(),
        Err(e) => return interp.report_cmd_error(e),
    };
    let command = match interp.native_string_bytes(&argv[4]) {
        Ok(command) => command,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let name = match interp.native_string_bytes(&argv[2]) {
        Ok(name) => name,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    var_trace_apply(interp, name.to_vec(), ops, command.to_vec(), is_add, true)
}

/// `trace vinfo name` — the same live trace list `trace info variable` reports,
/// with each op set rendered as the `rwua` letter string C's `TRACE_OLD_VINFO`
/// arm builds instead of a word list.
fn legacy_var_info(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = legacy_var_info_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn legacy_var_info_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"trace vinfo name");
    }
    let name = match arguments.bytes(2) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    let query = match var_trace_query(interp, &name) {
        Ok(query) => query,
        Err(error) => return trace_var_error(interp, &name, error),
    };
    let Some(prefix_protocol) = interp
        .native_invocation_dialect()
        .native_variable_trace_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable trace prefix report",
            )
            .into(),
        );
    };
    let cell = interp.variable_trace_scope(&query, query.link_elem.as_deref());
    let mut entries: Vec<*mut TclObj> = Vec::new();
    for t in interp.traces.borrow().traces.iter().rev() {
        if t.native.is_some() || !cell.owns_registration(t) {
            continue;
        }
        let letters = new_string(core_trace::legacy_ops_letters(&t.ops).as_bytes());
        let cmd = new_string(prefix_protocol.variable_prefix_report(&t.command));
        entries.push(interp.new_list_object(&[letters, cmd]));
    }
    interp.set_result(interp.new_list_object(&entries));
    Code::Ok
}

#[cfg(test)]
mod native_command_tests;

#[cfg(test)]
mod selected_member_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_operand_tests;
