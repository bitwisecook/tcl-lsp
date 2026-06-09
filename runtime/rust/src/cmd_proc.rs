//! `proc` (define a user procedure) + `puts` (output) — toward executing scripts.
//!
//! `proc name params body` parses the parameter spec (names, `{name default}`
//! pairs, and a trailing `args` catch-all) and registers a
//! [`Command::Proc`](crate::interp::Command); the call protocol
//! (`Interp::call_proc`) pushes a frame, binds the args, and runs the body —
//! see `proc-call-and-stack-traces.md` (PC-2). `puts` writes to stdout/stderr.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{obj_bytes, Code, Interp, Param};
use crate::obj::TclObj;
use crate::parse::split_list;

/// Register `proc`, `apply`, and `puts`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"proc", proc_cmd);
    interp.register_builtin(b"apply", apply_cmd);
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

// -- proc ------------------------------------------------------------------

/// `proc name params body` — define a procedure.
fn proc_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"proc name args body");
    }
    let name = obj_bytes(argv[1]);
    let params = match parse_params(&obj_bytes(argv[2])) {
        Ok(p) => p,
        Err(e) => return interp.set_error(&e),
    };
    let body = obj_bytes(argv[3]);
    interp.define_proc(&name, params, body);
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Parse a proc parameter spec (a Tcl list of names / `{name default}` pairs).
fn parse_params(spec: &[u8]) -> Result<Vec<Param>, Vec<u8>> {
    let elems = split_list(spec).map_err(|e| e.message().to_vec())?;
    let mut params = Vec::with_capacity(elems.len());
    for e in &elems {
        let parts = split_list(e).map_err(|er| er.message().to_vec())?;
        match parts.len() {
            0 => return Err(b"argument with no name".to_vec()),
            1 => params.push(Param {
                name: parts[0].clone(),
                default: None,
            }),
            2 => params.push(Param {
                name: parts[0].clone(),
                default: Some(parts[1].clone()),
            }),
            _ => {
                let mut m = b"too many fields in argument specifier \"".to_vec();
                m.extend_from_slice(e);
                m.push(b'"');
                return Err(m);
            }
        }
    }
    Ok(params)
}

// -- apply -----------------------------------------------------------------

/// `apply {params body ?namespace?} ?arg ...?` — invoke an anonymous procedure.
/// The lambda runs in `namespace` (default global), via the shared proc-call
/// protocol (`Interp::run_proc`).
fn apply_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"apply lambdaExpr ?arg ...?");
    }
    let lambda = obj_bytes(argv[1]);
    let parts = match split_list(&lambda) {
        Ok(p) => p,
        Err(e) => return interp.set_error(e.message()),
    };
    if parts.len() < 2 || parts.len() > 3 {
        let mut m = b"can't interpret \"".to_vec();
        m.extend_from_slice(&lambda);
        m.extend_from_slice(b"\" as a lambda expression");
        return interp.set_error(&m);
    }
    let params = match parse_params(&parts[0]) {
        Ok(p) => p,
        Err(e) => return interp.set_error(&e),
    };
    let ns = if parts.len() == 3 {
        interp.ensure_namespace(&parts[2])
    } else {
        crate::namespace::GLOBAL
    };
    interp.run_proc(&params, &parts[1], ns, &argv[2..], b"apply lambdaExpr")
}

// `puts` lives in `cmd_chan` (it is a channel write — stdout/stderr/file).

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

    #[test]
    fn proc_basic_args_defaults_and_locals() {
        leak_free(|i| {
            run(i, b"proc pick {a b} {return $b}");
            assert_eq!(run(i, b"pick 1 2"), b"2");
            // default parameter
            run(
                i,
                b"proc greet {name {greeting hello}} {return $name-$greeting}",
            );
            assert_eq!(run(i, b"greet bob"), b"bob-hello");
            assert_eq!(run(i, b"greet bob hi"), b"bob-hi");
            // proc locals don't leak to the caller
            run(i, b"proc uselocal {} {set v 99; return $v}");
            assert_eq!(run(i, b"set v outer"), b"outer");
            assert_eq!(run(i, b"uselocal"), b"99");
            assert_eq!(run(i, b"set v"), b"outer");
            i.eval_str(b"unset v");
        });
    }

    #[test]
    fn proc_args_catch_all() {
        leak_free(|i| {
            run(i, b"proc va {first args} {return $first|$args}");
            assert_eq!(run(i, b"va 1"), b"1|");
            assert_eq!(run(i, b"va 1 2 3"), b"1|2 3");
        });
    }

    #[test]
    fn proc_wrong_args() {
        leak_free(|i| {
            run(i, b"proc add {a b} {return x}");
            assert_eq!(i.eval_str(b"add 1"), Code::Error);
            assert_eq!(i.result_bytes(), b"wrong # args: should be \"add a b\"");
            assert_eq!(i.eval_str(b"add 1 2 3"), Code::Error);
            assert_eq!(i.result_bytes(), b"wrong # args: should be \"add a b\"");
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn proc_recursion_fib() {
        leak_free(|i| {
            run(
                i,
                b"proc fib {n} {if {$n < 2} {return $n}; return [expr {[fib [expr {$n-1}]] + [fib [expr {$n-2}]]}]}",
            );
            assert_eq!(run(i, b"fib 10"), b"55");
            assert_eq!(run(i, b"fib 0"), b"0");
            assert_eq!(run(i, b"fib 1"), b"1");
        });
    }

    #[test]
    fn unqualified_proc_in_namespace_eval_binds_in_that_ns() {
        // `proc p` inside `namespace eval X` binds ::X::p (not a global) — the
        // command-home-ns fix.
        leak_free(|i| {
            run(i, b"namespace eval X { proc p {} {return inX} }");
            assert_eq!(run(i, b"X::p"), b"inX");
            assert_eq!(run(i, b"namespace eval X { p }"), b"inX");
            // it is NOT a global command.
            assert_eq!(i.eval_str(b"p"), Code::Error);
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn proc_in_namespace_uses_its_ns() {
        leak_free(|i| {
            // a proc defined in ::ctr sees ::ctr's variables via `variable`.
            run(i, b"namespace eval ctr { variable n 0 }");
            run(i, b"proc ctr::bump {} { variable n; incr n; return $n }");
            assert_eq!(run(i, b"ctr::bump"), b"1");
            assert_eq!(run(i, b"ctr::bump"), b"2");
            assert_eq!(run(i, b"set ::ctr::n"), b"2");
            i.eval_str(b"unset ::ctr::n");
        });
    }

    #[test]
    fn apply_lambda() {
        leak_free(|i| {
            assert_eq!(run(i, b"apply {{a b} {return $b}} 1 2"), b"2");
            assert_eq!(run(i, b"apply {{a {b 9}} {return $b}} 1"), b"9");
            assert_eq!(run(i, b"apply {{args} {return $args}} a b c"), b"a b c");
            assert_eq!(i.eval_str(b"apply {{a b} {return x}} 1"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"apply lambdaExpr a b\""
            );
            // a 3-element lambda runs in the named namespace.
            run(i, b"namespace eval foo { variable v 42 }");
            assert_eq!(run(i, b"apply {{} {variable v; return $v} foo}"), b"42");
            i.eval_str(b"unset ::foo::v");
        });
    }

    #[test]
    fn break_continue_escaping_a_proc_is_an_error() {
        leak_free(|i| {
            run(i, b"proc f {} {break}");
            assert_eq!(i.eval_str(b"f"), Code::Error);
            assert_eq!(i.result_bytes(), b"invoked \"break\" outside of a loop");
            run(i, b"proc g {} {continue}");
            assert_eq!(i.eval_str(b"g"), Code::Error);
            assert_eq!(i.result_bytes(), b"invoked \"continue\" outside of a loop");
        });
    }

    #[test]
    fn infinite_recursion_is_caught() {
        // An unbounded proc loop raises a catchable error, not a stack overflow.
        // The tree-walking interpreter recurses on the native stack (~one deep
        // chain per Tcl level), so the 1000-level bound needs a production-sized
        // stack to be *reached* — the default 2 MiB test-thread stack is too
        // small. Run it on a large-stack thread (the main thread / a configured
        // wasm stack have ample room); the leak counters are thread-local, so the
        // check runs inside the spawned thread.
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                leak_free(|i| {
                    run(i, b"proc loop {} {loop}");
                    assert_eq!(i.eval_str(b"loop"), Code::Error);
                    assert_eq!(
                        i.result_bytes(),
                        b"too many nested evaluations (infinite loop?)"
                    );
                });
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn puts_runs() {
        leak_free(|i| {
            // writes to stdout (captured/discarded by the test harness); returns "".
            assert_eq!(i.eval_str(b"puts -nonewline {}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            assert_eq!(i.eval_str(b"puts stdout hello"), Code::Ok);
            assert_eq!(i.eval_str(b"puts nosuchchan x"), Code::Error);
        });
    }
}
