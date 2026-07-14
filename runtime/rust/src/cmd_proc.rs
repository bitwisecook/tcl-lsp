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

//! `proc` (define a user procedure) + `puts` (output) — toward executing scripts.
//!
//! `proc name params body` parses the parameter spec (names, `{name default}`
//! pairs, and a trailing `args` catch-all) and registers a
//! [`Command::Proc`](crate::interp::Command); the call protocol
//! (`Interp::call_proc`) pushes a frame, binds the args, and runs the body —
//! see `proc-call-and-stack-traces.md` (PC-2). `puts` writes to stdout/stderr.

use crate::interp::{obj_bytes, CallMeta, Code, Interp, Param, ProcFrame};
use crate::obj::TclObj;
use crate::parse::split_list;

/// Register `proc`, `apply`, and `puts`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"proc", proc_cmd);
    interp.register_builtin(b"apply", apply_cmd);
}

// -- proc ------------------------------------------------------------------

/// `proc name params body` — define a procedure.
fn proc_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"proc name args body");
    }
    let name = obj_bytes(argv[1]);
    // A namespace-qualified proc name requires that namespace to already exist
    // (C's `Tcl_ProcObjCmd` via `TclGetNamespaceForQualName`).
    if let Some(i) = name.windows(2).rposition(|w| w == b"::") {
        let qualifier = &name[..i];
        if !qualifier.is_empty() && interp.find_namespace_id(qualifier).is_none() {
            let mut m = b"can't create procedure \"".to_vec();
            m.extend_from_slice(&name);
            m.extend_from_slice(b"\": unknown namespace");
            return interp.set_error(&m);
        }
    }
    let params = match parse_params(&obj_bytes(argv[2])) {
        Ok(p) => p,
        Err(e) => return interp.set_error(&e),
    };
    interp.define_proc(&name, params, argv[3]);
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Parse a proc parameter spec (a Tcl list of names / `{name default}` pairs).
pub(crate) fn parse_params(spec: &[u8]) -> Result<Vec<Param>, Vec<u8>> {
    let elems = split_list(spec).map_err(|e| e.message().to_vec())?;
    let mut params = Vec::with_capacity(elems.len());
    for e in &elems {
        let parts = split_list(e).map_err(|er| er.message().to_vec())?;
        match parts.len() {
            0 => return Err(b"argument with no name".to_vec()),
            1 => {
                check_param_name(&parts[0])?;
                params.push(Param {
                    name: parts[0].clone(),
                    default: None,
                });
            }
            2 => {
                check_param_name(&parts[0])?;
                params.push(Param {
                    name: parts[0].clone(),
                    default: Some(parts[1].clone()),
                });
            }
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

/// A formal parameter name must be a scalar simple name: not an array element
/// (`a(1)`) and not namespace-qualified (`a::b`). Mirrors the scan in C's
/// `TclCreateProc` (`tclProc.c`): an `(` anywhere before the final char with a
/// trailing `)` is an array element; a `::` anywhere before the final char makes
/// the name non-simple.
fn check_param_name(name: &[u8]) -> Result<(), Vec<u8>> {
    if name.is_empty() {
        return Ok(());
    }
    let last = name.len() - 1;
    let mut i = 0;
    while i < last {
        if name[i] == b'(' {
            if name[last] == b')' {
                let mut m = b"formal parameter \"".to_vec();
                m.extend_from_slice(name);
                m.extend_from_slice(b"\" is an array element");
                return Err(m);
            }
        } else if name[i] == b':' && name[i + 1] == b':' {
            let mut m = b"formal parameter \"".to_vec();
            m.extend_from_slice(name);
            m.extend_from_slice(b"\" is not a simple name");
            return Err(m);
        }
        i += 1;
    }
    Ok(())
}

// -- apply -----------------------------------------------------------------

/// `apply {params body ?namespace?} ?arg ...?` — invoke an anonymous procedure.
/// The lambda runs in `namespace` (default global), via the shared proc-call
/// protocol (`Interp::run_proc`).
fn apply_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"apply lambdaExpr ?arg ...?");
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
        Err(e) => {
            // A malformed parameter list: report the parse error, then add the
            // `(parsing lambda expression "<lambda>")` errorInfo frame (C's
            // `Tcl_ApplyObjCmd` after a failed `TclCreateProc`).
            let code = interp.set_error(&e);
            interp.append_lambda_parse_frame(&lambda);
            return code;
        }
    };
    let ns = if parts.len() == 3 {
        // C prepends `::` to a non-global-qualified namespace name, then resolves
        // it (`TclGetNamespaceFromObj`); a missing namespace is an error — apply
        // does **not** create it.
        let full: Vec<u8> = if parts[2].starts_with(b"::") {
            parts[2].clone()
        } else {
            let mut f = b"::".to_vec();
            f.extend_from_slice(&parts[2]);
            f
        };
        match interp.find_namespace_id(&full) {
            Some(id) => id,
            None => {
                let mut m = b"namespace \"".to_vec();
                m.extend_from_slice(&full);
                m.extend_from_slice(b"\" not found");
                let mut ecode = b"TCL LOOKUP NAMESPACE ".to_vec();
                ecode.extend_from_slice(&full);
                return interp.error_with_code(&m, &ecode);
            }
        }
    } else {
        crate::namespace::GLOBAL
    };
    // A literal lambda in a sourced file reports `type source` at its body's line
    // (element 1 of the lambda list — TIP 280); a dynamic lambda is body-relative.
    let (source, body_line_base) = match interp.list_element_location(argv[1], 1) {
        Some((file, line)) => (Some(file), line.saturating_sub(1)),
        None => (None, 0),
    };
    // `info level N` of a lambda reports the actual invocation words
    // (`apply <lambdaExpr> ?arg ...?`), not the `apply lambdaExpr` usage prefix
    // used for `wrong # args` (C records `objv` verbatim for the lambda frame).
    let level_words: Vec<Vec<u8>> = argv.iter().map(|&a| obj_bytes(a)).collect();
    interp.run_proc(
        &params,
        &parts[1],
        ns,
        &argv[2..],
        b"apply lambdaExpr",
        CallMeta {
            err: ProcFrame::Lambda(&lambda),
            fqn: None,
            source,
            body_line_base,
            link_vars: &[],
            keep_loop_codes: false,
            same_level: false,
            usage_prefix: None,
            level_words: Some(level_words),
            quote_name: false,
        },
    )
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
    fn proc_default_arity_edges_dont_panic() {
        // Regression: defaulted positionals must not panic when fewer args than
        // positionals are supplied (the `args` split) or when a *required*
        // parameter follows a defaulted one (non-trailing default). Matches
        // tclsh 9.0.
        leak_free(|i| {
            // All-defaulted positionals + args, called with none.
            run(i, b"proc q {{a 1} {b 2} args} {list $a $b $args}");
            assert_eq!(run(i, b"q"), b"1 2 {}");
            assert_eq!(run(i, b"q 5"), b"5 2 {}");
            assert_eq!(run(i, b"q 5 6 7 8"), b"5 6 {7 8}");
            // Non-trailing default: a required param after a defaulted one.
            run(i, b"proc p {a {b 2} c} {list $a $b $c}");
            assert_eq!(run(i, b"p 1 2 3"), b"1 2 3");
            assert_eq!(i.eval_str(b"p 1 2"), Code::Error);
            assert_eq!(i.result_bytes(), b"wrong # args: should be \"p a ?b? c\"");
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
