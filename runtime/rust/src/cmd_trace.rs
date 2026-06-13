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
//! Simplifications (tracked): variable traces match by variable *name* (the
//! namespace the trace was registered in is not distinguished). Command and
//! execution traces are keyed by the resolved FQN (`Interp::resolve_cmd_fqn`).
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::frame::split_array_ref;
use crate::interp::{new_string, obj_bytes, Code, Interp};
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

/// The interp's trace registries plus a re-entrancy guard.
#[derive(Default)]
pub struct TraceTable {
    pub traces: Vec<VarTrace>,
    /// Command + execution traces, keyed by resolved command FQN.
    pub cmd_traces: Vec<CmdTrace>,
    /// Non-zero while traces are firing — suppresses re-entrant firing so a
    /// trace that touches its own variable doesn't recurse (Tcl marks the
    /// active trace; a global guard is a safe coarsening).
    pub firing: usize,
    /// Non-zero while a command/execution trace callback is running — C's
    /// `INTERP_TRACE_IN_PROGRESS`. Suppresses re-entrant command/execution/step
    /// firing so a callback that renames/invokes the traced command doesn't
    /// recurse.
    pub exec_firing: usize,
}

/// Whether `t` fires for a `(base, elem)` access doing operation `op`.
pub fn matches(t: &VarTrace, base: &[u8], elem: Option<&[u8]>, op: &[u8]) -> bool {
    if t.base != base {
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

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

/// `bad option "X": must be ...` (the `Tcl_GetIndexFromObj` miss).
fn bad_option(interp: &mut Interp, bad: &[u8], must_be: &[u8]) -> Code {
    let mut m = b"bad option \"".to_vec();
    m.extend_from_slice(bad);
    m.extend_from_slice(b"\": must be ");
    m.extend_from_slice(must_be);
    interp.set_error(&m)
}

fn trace_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"trace option ?arg ...?");
    }
    match obj_bytes(argv[1]).as_slice() {
        // `trace add`/`remove` then dispatch on the type word (objv[2]).
        op @ (b"add" | b"remove") => {
            let is_add = op == b"add";
            if argv.len() < 3 {
                return wrong_args(
                    interp,
                    if is_add {
                        b"trace add type ?arg ...?"
                    } else {
                        b"trace remove type ?arg ...?"
                    },
                );
            }
            match obj_bytes(argv[2]).as_slice() {
                b"variable" => trace_var_add_remove(interp, argv, is_add),
                b"command" => cmd_trace_add_remove(interp, argv, is_add, ops::CMD_ANY),
                b"execution" => cmd_trace_add_remove(interp, argv, is_add, ops::EXEC_ANY),
                other => bad_option(interp, other, b"execution, command, or variable"),
            }
        }
        b"info" => {
            if argv.len() < 3 {
                return wrong_args(interp, b"trace info type name");
            }
            match obj_bytes(argv[2]).as_slice() {
                b"variable" => trace_var_info(interp, argv),
                b"command" => cmd_trace_info(interp, argv, ops::CMD_ANY),
                b"execution" => cmd_trace_info(interp, argv, ops::EXEC_ANY),
                other => bad_option(interp, other, b"execution, command, or variable"),
            }
        }
        other => bad_option(interp, other, b"add, info, or remove"),
    }
}

// -- command / execution traces -------------------------------------------

/// Parse an execution-trace op list into a [`ops`] bitset, with C's verbatim
/// `bad operation list ""` / `bad operation "X"` messages.
fn parse_exec_ops(interp: &mut Interp, spec: &[u8]) -> Result<u8, Code> {
    parse_cmd_or_exec_ops(
        interp,
        spec,
        b"enter, leave, enterstep, or leavestep",
        |o| match o {
            b"enter" => Some(ops::ENTER),
            b"leave" => Some(ops::LEAVE),
            b"enterstep" => Some(ops::ENTERSTEP),
            b"leavestep" => Some(ops::LEAVESTEP),
            _ => None,
        },
    )
}

/// Parse a command-trace op list (`rename`/`delete`) into a [`ops`] bitset.
fn parse_cmd_ops(interp: &mut Interp, spec: &[u8]) -> Result<u8, Code> {
    parse_cmd_or_exec_ops(interp, spec, b"delete or rename", |o| match o {
        b"rename" => Some(ops::RENAME),
        b"delete" => Some(ops::DELETE),
        _ => None,
    })
}

fn parse_cmd_or_exec_ops(
    interp: &mut Interp,
    spec: &[u8],
    must_be: &[u8],
    classify: impl Fn(&[u8]) -> Option<u8>,
) -> Result<u8, Code> {
    let list = match crate::parse::split_list(spec) {
        Ok(l) => l,
        Err(e) => return Err(interp.set_error(e.message())),
    };
    if list.is_empty() {
        let mut m = b"bad operation list \"\": must be one or more of ".to_vec();
        m.extend_from_slice(must_be);
        return Err(interp.set_error(&m));
    }
    let mut flags = 0u8;
    for o in &list {
        match classify(o) {
            Some(bit) => flags |= bit,
            None => {
                let mut m = b"bad operation \"".to_vec();
                m.extend_from_slice(o);
                m.extend_from_slice(b"\": must be ");
                m.extend_from_slice(must_be);
                return Err(interp.set_error(&m));
            }
        }
    }
    Ok(flags)
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
        return wrong_args(interp, &usage);
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
        return wrong_args(interp, &usage);
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

/// Parse and validate an ops list (`{read write unset array}`).
fn parse_ops(interp: &mut Interp, spec: &[u8]) -> Result<Vec<Vec<u8>>, Code> {
    let ops = match crate::parse::split_list(spec) {
        Ok(o) => o,
        Err(e) => return Err(interp.set_error(e.message())),
    };
    if ops.is_empty() {
        return Err(interp.set_error(
            b"bad operation list \"\": must be one or more of array, read, unset, or write",
        ));
    }
    for o in &ops {
        if !matches!(o.as_slice(), b"read" | b"write" | b"unset" | b"array") {
            let mut m = b"bad operation \"".to_vec();
            m.extend_from_slice(o);
            m.extend_from_slice(b"\": must be array, read, unset, or write");
            return Err(interp.set_error(&m));
        }
    }
    Ok(ops)
}

// -- variable traces -------------------------------------------------------

/// `trace add|remove variable name ops command`.
fn trace_var_add_remove(interp: &mut Interp, argv: &[*mut TclObj], is_add: bool) -> Code {
    if argv.len() != 6 {
        return wrong_args(
            interp,
            if is_add {
                b"trace add variable name opList command"
            } else {
                b"trace remove variable name opList command"
            },
        );
    }
    let name = obj_bytes(argv[3]);
    let ops = match parse_ops(interp, &obj_bytes(argv[4])) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let command = obj_bytes(argv[5]);
    if is_add {
        let (base, elem) = split_array_ref(&name);
        interp.traces.borrow_mut().traces.push(VarTrace {
            name,
            base,
            elem,
            ops,
            command,
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
        return wrong_args(interp, b"trace info variable name");
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
}
