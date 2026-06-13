//! Control flow (toward executing scripts) — `if` / `while` / `for` / `foreach`
//! / `lmap` plus `break` / `continue`.
//!
//! Bodies evaluate through the eval loop ([`Interp::eval_str`]); a body that
//! completes with `break`/`continue` is caught by the enclosing loop, while
//! `return`/error propagate. Conditions (`if`/`while`/`for`) are Tcl
//! **expressions** evaluated via the shared `expr` walk, so those three are
//! gated on the numeric tower like `expr` itself; `foreach`/`break`/`continue`
//! need no tower.
//!
//! Semantics verified against tclsh 9.0.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{drop_fresh, new_string, obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register the control-flow commands.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"break", break_cmd);
    interp.register_builtin(b"continue", continue_cmd);
    interp.register_builtin(b"foreach", foreach);
    interp.register_builtin(b"lmap", lmap);
    // `if`/`while`/`for` test Tcl expressions → need the numeric tower.
    #[cfg(have_tommath)]
    {
        interp.register_builtin(b"if", if_cmd);
        interp.register_builtin(b"while", while_cmd);
        interp.register_builtin(b"for", for_cmd);
    }
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

// -- break / continue ------------------------------------------------------

fn break_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 1 {
        return wrong_args(interp, b"break");
    }
    Code::Break
}

fn continue_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 1 {
        return wrong_args(interp, b"continue");
    }
    Code::Continue
}

// -- if --------------------------------------------------------------------

/// `if expr1 ?then? body1 elseif expr2 ?then? body2 ... ?else? ?bodyN?`.
#[cfg(have_tommath)]
fn if_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut i = 1;
    loop {
        if i >= argv.len() {
            return interp.set_error(b"wrong # args: no expression after \"if\" argument");
        }
        let cond = obj_bytes(argv[i]);
        i += 1;
        // optional `then` keyword.
        if i < argv.len() && obj_bytes(argv[i]).as_slice() == b"then" {
            i += 1;
        }
        if i >= argv.len() {
            let mut m = b"wrong # args: no script following \"".to_vec();
            m.extend_from_slice(&cond);
            m.extend_from_slice(b"\" argument");
            return interp.set_error(&m);
        }
        let body = argv[i];
        i += 1;

        match crate::builtins::eval_bool_expr(interp, &cond) {
            Ok(true) => return interp.eval_str(&obj_bytes(body)),
            Ok(false) => {}
            Err(code) => return code,
        }

        // Condition false: dispatch on the next keyword.
        if i >= argv.len() {
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
        match obj_bytes(argv[i]).as_slice() {
            b"elseif" => {
                i += 1;
                continue;
            }
            b"else" => {
                i += 1;
                if i >= argv.len() {
                    return interp
                        .set_error(b"wrong # args: no script following \"else\" argument");
                }
                return interp.eval_str(&obj_bytes(argv[i]));
            }
            // A bare trailing body is the implicit else (`if {0} a b`).
            _ => return interp.eval_str(&obj_bytes(argv[i])),
        }
    }
}

// -- while -----------------------------------------------------------------

/// `while test body`.
#[cfg(have_tommath)]
fn while_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"while test command");
    }
    let cond = obj_bytes(argv[1]);
    let body = obj_bytes(argv[2]);
    loop {
        match crate::builtins::eval_bool_expr(interp, &cond) {
            Ok(true) => {}
            Ok(false) => break,
            Err(code) => return code,
        }
        match interp.eval_str(&body) {
            Code::Ok | Code::Continue => {}
            Code::Break => break,
            other => return other, // return / error propagate
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- for -------------------------------------------------------------------

/// `for start test next body`.
#[cfg(have_tommath)]
fn for_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return wrong_args(interp, b"for start test next command");
    }
    let (init, cond, next, body) = (
        obj_bytes(argv[1]),
        obj_bytes(argv[2]),
        obj_bytes(argv[3]),
        obj_bytes(argv[4]),
    );
    match interp.eval_str(&init) {
        Code::Ok => {}
        other => return other,
    }
    loop {
        match crate::builtins::eval_bool_expr(interp, &cond) {
            Ok(true) => {}
            Ok(false) => break,
            Err(code) => return code,
        }
        match interp.eval_str(&body) {
            Code::Ok | Code::Continue => {} // `continue` still runs `next`
            Code::Break => break,
            other => return other,
        }
        match interp.eval_str(&next) {
            Code::Ok => {}
            other => return other,
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- foreach / lmap --------------------------------------------------------

/// `foreach varList list ?varList list ...? body` — iterate one or more
/// (var-list, value-list) groups in parallel, padding exhausted lists with `""`.
fn foreach(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    each_loop(interp, argv, false)
}

/// `lmap varList list ?varList list ...? body` — `foreach` that collects each
/// (non-`continue`) body result into a list and returns it. `break` ends the
/// loop and returns the list accumulated so far. Mirrors C's `EachloopCmd`
/// (`tclCmdAH.c`) with `TCL_EACH_COLLECT`.
fn lmap(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    each_loop(interp, argv, true)
}

/// Shared `foreach`/`lmap` engine. With `collect`, each successful body result
/// is appended to a result list (the `lmap` return value); without, the result
/// is the empty string (`foreach`). The two differ only in result collection
/// and the body-frame command name — exactly as C factors them through one
/// `EachloopCmd`.
fn each_loop(interp: &mut Interp, argv: &[*mut TclObj], collect: bool) -> Code {
    let name: &[u8] = if collect { b"lmap" } else { b"foreach" };
    // [cmd] + N×(varlist, list) pairs + body ⇒ an even arg count ≥ 4.
    if argv.len() < 4 || argv.len() % 2 != 0 {
        let usage: &[u8] = if collect {
            b"lmap varList list ?varList list ...? command"
        } else {
            b"foreach varList list ?varList list ...? command"
        };
        return wrong_args(interp, usage);
    }
    let body = obj_bytes(argv[argv.len() - 1]);

    // Parse each group's variable names + values; track the iteration count.
    // A group is `(variable names, values)`.
    type Group = (Vec<Vec<u8>>, Vec<Vec<u8>>);
    let mut groups: Vec<Group> = Vec::new();
    let mut iterations = 0usize;
    let pairs = &argv[1..argv.len() - 1];
    for pair in pairs.chunks_exact(2) {
        let vars = match crate::parse::split_list(&obj_bytes(pair[0])) {
            Ok(v) => v,
            Err(e) => return interp.set_error(e.message()),
        };
        if vars.is_empty() {
            let mut m = name.to_vec();
            m.extend_from_slice(b" varlist is empty");
            return interp.set_error(&m);
        }
        let vals = match crate::parse::split_list(&obj_bytes(pair[1])) {
            Ok(v) => v,
            Err(e) => return interp.set_error(e.message()),
        };
        iterations = iterations.max(vals.len().div_ceil(vars.len()));
        groups.push((vars, vals));
    }

    // Collected body results (bytes; rematerialised into the result list at the
    // end). Only populated for `lmap`.
    let mut collected: Vec<Vec<u8>> = Vec::new();
    for it in 0..iterations {
        for (vars, vals) in &groups {
            for (k, var) in vars.iter().enumerate() {
                let val = vals.get(it * vars.len() + k).cloned().unwrap_or_default();
                let o = new_string(&val);
                if let Err(e) = interp.var_set(var, o) {
                    drop_fresh(o);
                    return crate::builtins::var_error(interp, var, e);
                }
            }
        }
        match interp.eval_str(&body) {
            Code::Ok => {
                if collect {
                    collected.push(obj_bytes(interp.get_obj_result()));
                }
            }
            Code::Continue => {} // skip collection, keep looping
            Code::Break => break,
            Code::Error => {
                // `("<cmd>" body line N)` — only at top level. Tcl inlines
                // `foreach`/`lmap` when it compiles the enclosing script (a proc
                // body), so no body-frame appears there; an uncompiled top-level
                // form runs its command form, which does add the frame. A
                // tree-walker has no bytecode, so `!in_proc()` approximates the
                // compilation boundary — matches tclsh 9.0 for both cases.
                if !interp.in_proc() {
                    interp.append_body_frame(name);
                }
                return Code::Error;
            }
            other => return other,
        }
    }
    if collect {
        // Rematerialise into a list; `new_list_obj` takes a +1 on each fresh
        // (rc-0) element, so the list owns them — no `drop_fresh` here.
        let objs: Vec<*mut TclObj> = collected.iter().map(|b| new_string(b)).collect();
        interp.set_result(crate::list::new_list_obj(&objs));
    } else {
        interp.set_result_bytes(b"");
    }
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

    fn run(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?} → {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    #[cfg(have_tommath)]
    #[test]
    fn if_elseif_else() {
        leak_free(|i| {
            assert_eq!(run(i, b"if {1 < 2} {set x yes}"), b"yes");
            assert_eq!(run(i, b"if {0} {set x a} else {set x b}"), b"b");
            assert_eq!(
                run(i, b"if {0} {set x a} elseif {1} {set x c} else {set x d}"),
                b"c"
            );
            // false with no else → empty result.
            assert_eq!(run(i, b"if {0} {set x a}"), b"");
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn while_and_for_accumulate() {
        leak_free(|i| {
            assert_eq!(run(i, b"set n 0; while {$n < 3} {incr n}; set n"), b"3");
            assert_eq!(
                run(
                    i,
                    b"set s 0; for {set i 1} {$i <= 5} {incr i} {incr s $i}; set s"
                ),
                b"15"
            );
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn break_and_continue() {
        leak_free(|i| {
            assert_eq!(
                run(
                    i,
                    b"set out {}; foreach x {1 2 3 4} {if {$x==3} break; append out $x}; set out"
                ),
                b"12"
            );
            assert_eq!(
                run(
                    i,
                    b"set out {}; foreach x {1 2 3 4} {if {$x==2} continue; append out $x}; set out"
                ),
                b"134"
            );
        });
    }

    #[test]
    fn foreach_single_multi_var_and_parallel() {
        leak_free(|i| {
            assert_eq!(
                run(i, b"set out {}; foreach x {a b c} {append out $x}; set out"),
                b"abc"
            );
            assert_eq!(
                run(
                    i,
                    b"set out {}; foreach {k v} {1 2 3 4} {append out \"$k=$v \"}; set out"
                ),
                b"1=2 3=4 "
            );
            // parallel lists pad the exhausted one with "".
            assert_eq!(
                run(
                    i,
                    b"set out {}; foreach a {1 2} b {x y z} {append out \"$a$b \"}; set out"
                ),
                b"1x 2y z "
            );
        });
    }

    #[test]
    fn lmap_collects_results() {
        leak_free(|i| {
            // Single-var collect.
            assert_eq!(run(i, b"lmap x {a b c} {string toupper $x}"), b"A B C");
            // Multi-var per iteration → one element per iteration.
            assert_eq!(run(i, b"lmap {a b} {1 2 3 4} {list $a $b}"), b"{1 2} {3 4}");
            // Parallel lists pad with "".
            assert_eq!(
                run(i, b"lmap a {1 2} b {x y z} {list $a $b}"),
                b"{1 x} {2 y} {{} z}"
            );
            // `continue` skips collection; `break` ends with the list so far.
            assert_eq!(
                run(i, b"lmap x {1 2 3 4} {if {$x % 2 == 0} continue; set x}"),
                b"1 3"
            );
            assert_eq!(
                run(i, b"lmap x {1 2 3 4} {if {$x == 3} break; set x}"),
                b"1 2"
            );
            // Empty body → one empty element per iteration.
            assert_eq!(run(i, b"lmap x {1 2 3} {}"), b"{} {} {}");
        });
    }
}
