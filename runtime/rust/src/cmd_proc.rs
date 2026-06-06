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

use std::io::Write;

use crate::interp::{obj_bytes, Code, Interp, Param};
use crate::obj::TclObj;
use crate::parse::split_list;

/// Register `proc` and `puts`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"proc", proc_cmd);
    interp.register_builtin(b"puts", puts);
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

// -- puts ------------------------------------------------------------------

/// `puts ?-nonewline? ?channelId? string` — write to stdout (default) or stderr.
fn puts(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let usage = b"puts ?-nonewline? ?channelId? string";
    let mut rest = &argv[1..];
    let mut newline = true;
    if let Some(&first) = rest.first() {
        if obj_bytes(first).as_slice() == b"-nonewline" {
            newline = false;
            rest = &rest[1..];
        }
    }
    // Remaining is `string` or `channelId string`.
    let (channel, string) = match rest {
        [s] => (b"stdout".to_vec(), obj_bytes(*s)),
        [ch, s] => (obj_bytes(*ch), obj_bytes(*s)),
        _ => return wrong_args(interp, usage),
    };

    let result = match channel.as_slice() {
        b"stdout" => write_out(&mut std::io::stdout(), &string, newline),
        b"stderr" => write_out(&mut std::io::stderr(), &string, newline),
        _ => {
            let mut m = b"can not find channel named \"".to_vec();
            m.extend_from_slice(&channel);
            m.push(b'"');
            return interp.set_error(&m);
        }
    };
    if result.is_err() {
        return interp.set_error(b"error writing to channel");
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn write_out(w: &mut impl Write, bytes: &[u8], newline: bool) -> std::io::Result<()> {
    w.write_all(bytes)?;
    if newline {
        w.write_all(b"\n")?;
    }
    w.flush()
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
