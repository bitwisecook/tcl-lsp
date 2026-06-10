//! `trace` — variable traces (`trace add|remove|info variable`).
//!
//! A focused subset of `tclTrace.c`: read / write / unset / array variable
//! traces. When a traced variable is read, written, or unset, the registered
//! command prefix is invoked as `command name element op` (the standard Tcl
//! trace callback protocol). The interpreter fires these from the variable
//! read/write/unset chokepoints (`Interp::fire_var_trace`).
//!
//! Simplifications (tracked): traces match by variable *name* (the namespace
//! the trace was registered in is not distinguished), trace-callback errors are
//! swallowed rather than propagated, and command/execution traces (`trace add
//! execution`) are not modelled. These cover the `tcltest` bring-up usage
//! (option/constraint read+write traces).
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::frame::split_array_ref;
use crate::interp::{obj_bytes, Code, Interp};
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

/// The interp's variable-trace registry plus a re-entrancy guard.
#[derive(Default)]
pub struct TraceTable {
    pub traces: Vec<VarTrace>,
    /// Non-zero while traces are firing — suppresses re-entrant firing so a
    /// trace that touches its own variable doesn't recurse (Tcl marks the
    /// active trace; a global guard is a safe coarsening).
    pub firing: usize,
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

fn trace_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"trace option ?arg ...?");
    }
    match obj_bytes(argv[1]).as_slice() {
        b"add" => trace_add(interp, argv),
        b"remove" => trace_remove(interp, argv),
        b"info" => trace_info(interp, argv),
        other => {
            let mut m = b"bad option \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be add, info, or remove");
            interp.set_error(&m)
        }
    }
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

/// `trace add variable name ops command`.
fn trace_add(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 6 || obj_bytes(argv[2]) != b"variable" {
        return wrong_args(interp, b"trace add variable name opList command");
    }
    let name = obj_bytes(argv[3]);
    let ops = match parse_ops(interp, &obj_bytes(argv[4])) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let command = obj_bytes(argv[5]);
    let (base, elem) = split_array_ref(&name);
    interp.traces.borrow_mut().traces.push(VarTrace {
        name,
        base,
        elem,
        ops,
        command,
    });
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `trace remove variable name ops command` — drop the matching registration.
fn trace_remove(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 6 || obj_bytes(argv[2]) != b"variable" {
        return wrong_args(interp, b"trace remove variable name opList command");
    }
    let name = obj_bytes(argv[3]);
    let ops = match parse_ops(interp, &obj_bytes(argv[4])) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let command = obj_bytes(argv[5]);
    let pos = interp
        .traces
        .borrow()
        .traces
        .iter()
        .position(|t| t.name == name && t.ops == ops && t.command == command);
    if let Some(i) = pos {
        interp.traces.borrow_mut().traces.remove(i);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `trace info variable name` — the registered traces, most-recent first, each
/// as a `{opList command}` pair.
fn trace_info(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 || obj_bytes(argv[2]) != b"variable" {
        return wrong_args(interp, b"trace info variable name");
    }
    let name = obj_bytes(argv[3]);
    let mut entries: Vec<*mut TclObj> = Vec::new();
    for t in interp.traces.borrow().traces.iter().rev() {
        if t.name != name {
            continue;
        }
        let op_objs: Vec<*mut TclObj> =
            t.ops.iter().map(|o| crate::interp::new_string(o)).collect();
        let ops_list = crate::list::new_list_obj(&op_objs);
        let cmd = crate::interp::new_string(&t.command);
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
}
