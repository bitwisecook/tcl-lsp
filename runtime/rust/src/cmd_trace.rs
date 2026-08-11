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

use tcl_cmd_core::trace as core_trace;

use crate::frame::{VarError, split_array_ref};
use crate::interp::{Code, Interp, new_string, obj_bytes};
use crate::namespace::NsId;
use crate::obj::TclObj;

/// One registered variable trace.
pub struct VarTrace {
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

/// The resolved variable-cell identity used for the callback re-entrancy rule.
/// An array's element traces share their base cell, as Tcl does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VarTraceScope {
    base: Vec<u8>,
    frame_level: Option<usize>,
    ns: Option<NsId>,
}

impl VarTrace {
    pub(crate) fn scope(&self) -> VarTraceScope {
        VarTraceScope {
            base: self.base.clone(),
            frame_level: self.frame_level,
            ns: self.ns,
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

/// `bad option "X": must be ...` (the `Tcl_GetIndexFromObj` miss).
fn bad_option(interp: &mut Interp, bad: &[u8], must_be: &[u8]) -> Code {
    let mut m = b"bad option \"".to_vec();
    m.extend_from_slice(bad);
    m.extend_from_slice(b"\": must be ");
    m.extend_from_slice(must_be);
    interp.set_error(&m)
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
    match obj_bytes(argv[1]).as_slice() {
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
        other => bad_option(interp, other, b"add, info, or remove"),
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
        interp.traces.borrow_mut().cmd_traces.push(CmdTrace {
            name: fqn,
            ops: flags,
            command,
        });
    } else {
        // Remove the first trace matching exact ops + command string (C's
        // `FOREACH_COMMAND_TRACE` first-match).
        let pos = interp
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .position(|t| t.name == fqn && t.ops == flags && t.command == command);
        if let Some(i) = pos {
            interp.traces.borrow_mut().cmd_traces.remove(i);
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
    let name = obj_bytes(argv[3]);
    let ops = match parse_ops(interp, &obj_bytes(argv[4])) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let command = obj_bytes(argv[5]);
    if is_add {
        let (base, elem) = split_array_ref(&name);
        // Tracing an array element vivifies the array as an (undefined) array, so
        // a later read of a missing element reports "no such element in array"
        // rather than "no such variable", and whole-array semantics apply
        // (trace-1.4/1.8/5.x). Tracing an element of an existing *scalar* errors.
        let vivify = if elem.is_some() {
            interp.ensure_array(&base)
        } else {
            interp.ensure_trace_variable(&base)
        };
        if let Err(e) = vivify {
            return trace_var_error(interp, &name, e);
        }
        let frame_level = interp.local_trace_level(&base);
        // Key the trace by the variable it *resolves to*, not by the spelling
        // used to register it, so `trace add variable ::v write …` fires for a
        // later `set v X` in the same namespace — and, under the 8.x
        // namespace-scope fallback, for a write from inside `namespace eval`
        // that reaches that same global (issue #1328).  C hangs the trace off
        // the `Var` struct, so every spelling resolving to it fires.
        //
        // `name` keeps the original spelling: `trace info` / `trace remove`
        // match textually in C too.
        let (ns, base) = interp.trace_var_key(&base);
        interp.traces.borrow_mut().traces.push(VarTrace {
            name,
            base,
            elem,
            ops,
            command,
            frame_level,
            ns,
        });
    } else {
        let pos = interp
            .traces
            .borrow()
            .traces
            .iter()
            .position(|t| t.name == name && t.ops == ops && t.command == command);
        if let Some(i) = pos {
            interp.traces.borrow_mut().traces.remove(i);
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
    let mut entries: Vec<*mut TclObj> = Vec::new();
    for t in interp.traces.borrow().traces.iter().rev() {
        if t.name != name {
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
}
