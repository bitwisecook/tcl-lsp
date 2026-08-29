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

//! End-to-end: compile real Tcl to bytecode (via `tcl-compiler`, dev-dep only)
//! then run it through `tcl-vm`, asserting result + captured `puts` output.

use tcl_dialect::model::{Family};
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{Code, Commands, CompileError, CompileService, Traces, Value, Vm};

/// A `tcl-compiler`-backed compile service so the VM can resolve runtime
/// `eval` / `[command substitution]` (the injection seam — `tcl-vm` itself
/// never depends on the compiler).
struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        // Mirror the real VM compile services (`tcl-vm-cli`, `run_test`): a
        // hard parse error becomes a catchable runtime error with the exact
        // C Tcl message.
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

/// A `Write` sink backed by a shared buffer the test can read afterwards.
#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Compile and run `src`; return `(ok, result-string, captured-stdout)`.
fn run(src: &str) -> (bool, String, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    let completion = vm.run_module(&asm);

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

/// `Commands::dispatch` runs a compiled proc to completion — the nested
/// `is_proc` activation path (`invoke_command` pushes the call-frame, runs the
/// body, and absorbs `return` into an ok completion), and surfaces the proc's
/// arg-count usage error when binding fails.
#[test]
fn commands_dispatch_runs_proc() {
    let registry = CommandRegistry::build_default();
    let asm = {
        let src = "proc add {a b} { return [expr {$a + $b}] }";
        let ir = lower_to_ir(src, &registry);
        let cfg = build_cfg_codegen(&ir, false);
        codegen_module(&cfg, &ir, &registry)
    };

    let mut vm = Vm::new();
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    // Running the module registers `add`.
    assert!(vm.run_module(&asm).code.is_ok());

    // Dispatch the proc by name with name-stripped argv.
    let c = vm.dispatch("add", &[Value::string("3"), Value::string("4")]);
    assert_eq!(c.code, Code::Ok);
    assert_eq!(&*c.result.to_str(), "7");

    // Wrong arg count surfaces the proc usage error (the `enter_proc` Err path).
    let c = vm.dispatch("add", &[Value::string("3")]);
    assert_eq!(c.code, Code::Error);
    assert_eq!(&*c.result.to_str(), "wrong # args: should be \"add a b\"");
}

/// `Traces::fire` runs a variable's registered traces, aborting the access with
/// the wrapped `can't read "var": <msg>` error when a read/write callback fails,
/// and swallowing `unset`-trace errors (matching C and the runtime). Traces are
/// registered via the `trace` command (callbacks evaluate through the compiler).
#[test]
fn traces_fire_ok_error_and_unset() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    // `;#` comments out the appended `name elem op` trace words.
    for add in [
        "trace add variable x read {list ok;#}",
        "trace add variable y read {error boom;#}",
        "trace add variable z unset {error nope;#}",
    ] {
        assert!(vm.eval_source(add).expect("compiles").code.is_ok());
    }

    // Read trace, callback succeeds → the access proceeds.
    assert!(Traces::fire(&mut vm, "x", "read").is_ok());
    // Read trace, callback errors → the access aborts with the wrapped error.
    assert_eq!(
        &*Traces::fire(&mut vm, "y", "read").unwrap_err().to_str(),
        "can't read \"y\": boom",
    );
    // An `unset`-trace error does not abort the access.
    assert!(Traces::fire(&mut vm, "z", "unset").is_ok());
}

/// `trace add|remove|info <type>` accept an unambiguous prefix of the type
/// word — `var` resolves to `variable` — matching C's `Tcl_GetIndexFromObj`
/// over `traceTypeOptions`. Regression for set-2.4 / set-4.4, which add a
/// write trace via `trace add var x write …`.
#[test]
fn trace_type_accepts_unambiguous_abbreviation() {
    let (ok, result, _) = run(concat!(
        "set x 1\n",
        "proc ro args {error \"variable is read-only\"}\n",
        "trace add var x write ro\n",
        "catch {set x 9} m\n",
        "set m",
    ));
    assert!(ok, "script should complete (var → variable): {result}");
    // The trace fired (so `var` was accepted) and rewrapped the callback error.
    assert_eq!(result, "can't set \"x\": variable is read-only");
}

/// A write-trace rejection's `errorInfo` carries the full unwind down to the
/// command that triggered the trace: the callback frames, the
/// `(write trace on "x")` context frame, and finally the `set x 9` frame.
/// Regression for set-2.4 (the `-match glob` result requires the trace's
/// `errorInfo` to reach `"set x 9"`).
#[test]
fn write_trace_error_info_reaches_triggering_command() {
    let (ok, result, _) = run(concat!(
        "set x 1\n",
        "proc ro args {error \"variable is read-only\"}\n",
        "trace add variable x write ro\n",
        "catch {set x 9}\n",
        "string match {*variable is read-only*while executing*\"set x 9\"} $::errorInfo",
    ));
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result, "1",
        "errorInfo must unwind through the trace to the triggering `set x 9` frame",
    );
}

/// The compiled `linsert`/`lreplace` opcode rejects a non-integer index with
/// the same `bad index` error as the command, rather than silently treating it
/// as 0. Regression for linsert-2.2 / linsert-2.3.
#[test]
fn compiled_linsert_rejects_bad_index() {
    let (ok, result, _) = run("catch {linsert a b} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "bad index \"b\": must be integer?[+-]integer? or end?[+-]integer?"
    );

    // A valid index still inserts (no false positive).
    let (ok, result, _) = run("linsert {a b c} end X");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a b c X");
}

/// A `nan` arithmetic operand is reported as a non-numeric floating-point
/// *value*, not a string (mathop operator error tests).
#[test]
fn nan_operand_error_wording() {
    let (ok, result, _) = run("catch {tcl::mathop::+ nan 0} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "cannot use non-numeric floating-point value \"nan\" as left operand of \"+\""
    );
}

/// A non-numeric operand that is a well-formed multi-element **list** gets C's
/// distinct wording, `cannot use a list as <ord>operand of "<op>"` with
/// errorCode `ARITH DOMAIN list`. `IllegalExprOperandType`
/// (`tclExecute.c:9089-9107`) applies that branch to *binary* operators — where
/// `ord` is `"left "`/`"right "` — exactly as to unary ones, but only the unary
/// builder had the check.
#[test]
fn list_operand_error_wording() {
    for (e, want) in [
        ("{1 2} + 1", "cannot use a list as left operand of \"+\""),
        ("1 + {1 2}", "cannot use a list as right operand of \"+\""),
        (
            "{a 1 b 2} * 2",
            "cannot use a list as left operand of \"*\"",
        ),
        ("-{a 1 b 2}", "cannot use a list as operand of \"-\""),
    ] {
        let (ok, result, _) = run(&format!("catch {{expr {{{e}}}}} m\nset m"));
        assert!(ok, "script should complete: {result}");
        assert_eq!(result, want, "expr {{{e}}}");
    }
    // A single-element non-number is still a non-numeric *string* — the list
    // branch must not swallow it. Written through `expr` directly, with the
    // operand *quoted*: a bare `abc` never reaches the operand check in C either
    // (`ParseExpr` rejects it as `invalid bareword` first — `tclCompExpr.c:766`),
    // whereas a quoted string is a legal operand whose coercion is what fails
    // (expr-old-5.13/5.14). This used to be routed through `mathop` to dodge the
    // VM's old `exprStk` fallback, which handed a bare `abc + 1` back as text.
    let (ok, result, _) = run("catch {expr {\"abc\" + 1}} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "cannot use non-numeric string \"abc\" as left operand of \"+\""
    );
    // `mathop` reaches the same helper from its own entry point.
    let (ok, result, _) = run("catch {tcl::mathop::+ abc 1} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "cannot use non-numeric string \"abc\" as left operand of \"+\""
    );
    // And `mathop` takes the list branch too (same helper, both entry points).
    let (ok, result, _) = run("catch {tcl::mathop::+ {1 2} 1} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "cannot use a list as left operand of \"+\"");
}

/// Integer `expr` arithmetic promotes to i128 on i64 overflow instead of
/// wrapping (the VM's bounded stand-in for Tcl bignums), covering the common
/// large-value range that `expr`/`mathop` exercise.
#[test]
fn integer_arithmetic_promotes_on_overflow() {
    for (e, want) in [
        ("9223372036854775807 + 1", "9223372036854775808"),
        ("2 ** 70", "1180591620717411303424"),
        ("1000000000000000000000 + 5", "1000000000000000000005"),
        ("100000000000 * 100000000000", "10000000000000000000000"),
    ] {
        let (ok, result, _) = run(&format!("expr {{{e}}}"));
        assert!(ok, "`{e}` should evaluate: {result}");
        assert_eq!(result, want, "expr {{{e}}}");
    }
}

/// Unary minus of `9223372036854775808` (the magnitude 2^63, which overflows a
/// positive wide) yields the most-negative wide — C narrows `-2^63`. This
/// unblocks indexObj.test, whose `-result [expr {… ? -9223372036854775808 :
/// …}]` clauses are evaluated while the file is sourced.
#[test]
fn unary_minus_of_two_to_the_63() {
    let (ok, result, _) = run("expr {-9223372036854775808}");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "-9223372036854775808");

    let (ok, result, _) = run("expr {-9223372036854775808 < 0}");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "1");
}

/// `wide()` truncates an out-of-range integer literal to a 64-bit wide via
/// two's-complement wrap, matching C — `wide(0x8000000000000000)` is the most
/// negative wide, `wide(0xFFFFFFFFFFFFFFFF)` is -1. This unblocks expr.test
/// (which probes `wide(0x8000000000000000) < 0` at the top level) and brings
/// obj.test into line with C Tcl without a full bignum rep.
#[test]
fn wide_truncates_out_of_range_literals() {
    let (ok, result, _) = run("expr {wide(0x8000000000000000)}");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "-9223372036854775808");

    let (ok, result, _) = run("expr {wide(0xFFFFFFFFFFFFFFFF)}");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "-1");

    let (ok, result, _) = run("expr {wide(0x8000000000000000) < 0}");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "1");
}

/// `namespace path` sets a per-namespace command resolution path consulted
/// after the current namespace and before the global one, so e.g. the math
/// operators resolve unqualified. Regression for mathop / cmdIL (which crashed
/// on the unimplemented subcommand). `namespace path` with no list reads it back.
#[test]
fn namespace_path_resolves_commands() {
    let (ok, result, _) = run(
        "namespace eval foo {\n  namespace path ::tcl::mathop\n  set r [+ 2 3]\n  lappend r [namespace path]\n  set r\n}",
    );
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "5 ::tcl::mathop");
}

/// A loop whose body redefines `break`/`continue` runs through the runtime
/// builtin (which dispatches them) instead of the inline JUMP fast-path, so the
/// redefinition is honoured rather than looping forever. Regression for
/// proc-7.3 (Bug 729692) — this previously hung.
#[test]
fn loop_body_redefining_continue_is_honoured() {
    let (ok, result, _) = run(concat!(
        "set i 0\n",
        "while {1} {\n",
        "  if {[incr i] > 3} { proc continue {} {return -code break} }\n",
        "  continue\n",
        "}\n",
        "set i",
    ));
    assert!(ok, "loop should terminate (no hang): {result}");
    assert_eq!(result, "4");
}

/// `init_auto_load` installs the `unknown`/`auto_load` procs so an unresolved
/// command is looked up on demand. With an empty `auto_path` (no library scan)
/// a genuinely unknown command still raises the standard `invalid command name`
/// error — confirming the proc is installed and the miss path matches C.
/// (word.test, driven through the real library, is the positive integration
/// test.)
#[test]
fn autoloader_installs_unknown_and_reports_miss() {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    vm.init_auto_load();
    // Decouple from any on-disk library so the test is environment-independent.
    let _ = vm.eval_source("set ::auto_path {}");

    let c = vm.eval_source("info commands unknown").expect("compiles");
    assert_eq!(&*c.result.to_str(), "unknown", "unknown proc is installed");

    let c = vm
        .eval_source("catch {nonexistent_xyz_cmd} m; set m")
        .expect("compiles");
    assert_eq!(
        &*c.result.to_str(),
        "invalid command name \"nonexistent_xyz_cmd\"",
    );
}

/// `string map` with three arguments whose first is not `-nocase` reports a
/// bad-option error, not a wrong-arg-count error (string-10.2).
#[test]
fn string_map_three_args_bad_option() {
    let (ok, result, _) = run("catch {string map {a b} abba oops} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "bad option \"a b\": must be -nocase");
}

/// An error unwinding out of a runtime `eval $script` adds the uncompiled
/// eval's `("eval" body line N)` frame to `errorInfo`, followed by the
/// enclosing invocation's `invoked from within "eval $s"` frame — matching C's
/// uncompiled-eval unwind. (The compiler inlines `eval {literal}`, so this only
/// fires for the dynamic form, which is the one that routes through `cmd_eval`.)
#[test]
fn dynamic_eval_error_info_carries_eval_body_frame() {
    let (ok, result, _) = run(concat!(
        "set s \"error foo\"\n",
        "catch {eval $s}\n",
        "string match {foo*while executing*\"error foo\"*(\"eval\" body line 1)*invoked from within*\"eval \\$s\"} $::errorInfo",
    ));
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result, "1",
        "dynamic eval errorInfo must carry the (\"eval\" body line 1) and invoked-from-within frames",
    );
}

/// A bare builtin error sets `$errorCode` to the code C attaches: a
/// "wrong # args" usage error → `TCL WRONGARGS` (`Tcl_WrongNumArgs`), and a
/// malformed-list parse error → `TCL VALUE LIST …` (`tclUtil.c`). An
/// unrecognised error stays `NONE`. Regression for join-2.1/2.3, split-2.1.
#[test]
fn builtin_errors_publish_their_error_code() {
    // wrong # args → TCL WRONGARGS
    let (ok, result, _) = run("catch join\nset ::errorCode");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "TCL WRONGARGS");

    // unmatched open brace in list → TCL VALUE LIST BRACE (the quoted `\{`
    // collapses to a bare `{`, so the list parse fails).
    let (ok, result, _) = run("set s \"a b c \\{\"\ncatch {llength $s}\nset ::errorCode");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "TCL VALUE LIST BRACE");

    // a user `error` with no -errorcode keeps NONE (not reclassified).
    let (ok, result, _) = run("catch {error {wrong # args: should be \"x\"}}\nset ::errorCode");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "NONE");
}

/// `ledit listVar first last ?element ...?` is the in-place `lreplace` (Tcl 9):
/// it edits the list held in the variable, stores the result back, and returns
/// it. A missing array element reports the array-specific read error. Regression
/// for lreplace.test's ledit battery.
#[test]
fn ledit_edits_in_place_and_reports_array_miss() {
    let (ok, result, _) = run("set l {1 2 3 4 5}\nledit l 1 1 a b\nset l");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "1 a b 3 4 5");

    let (ok, result, _) = run(concat!(
        "unset -nocomplain arr\nset arr(y) y\n",
        "catch {ledit arr(x) 0 0 x} m\nset m",
    ));
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "can't read \"arr(x)\": no such element in array");
}

/// `concat` trims surrounding whitespace per element, but a trailing space
/// escaped by an odd run of backslashes (`\ `) is part of the element and kept
/// — C's `Tcl_ConcatObj` (lreplace.test ledit-1.25, concat-4.2).
#[test]
fn concat_keeps_backslash_escaped_trailing_space() {
    // Braced `{a\ }` is the literal `a`, `\`, ` `; the trailing space is escaped
    // by the backslash, so concat keeps it and adds its own join space.
    let (ok, result, _) = run("concat {a\\ } b");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a\\  b");

    let (ok, result, _) = run("concat {  a b  } {  c  }");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a b c"); // plain whitespace is trimmed
}

/// A codegen hook (`concat`, `llength`, …) collapses a non-braced literal
/// argument's backslash escapes exactly like the generic per-word path — a
/// braced word stays verbatim. Regression for concat-1.4 / llength-2.3, where
/// the hook pushed the raw word and so saw `a\{` instead of `a{`.
#[test]
fn list_hooks_collapse_nonbraced_backslashes() {
    // Non-braced `a\{` collapses to `a{`; the braced `{b \{c d}` stays literal.
    let (ok, result, _) = run("concat a\\{ {b \\{c d} \\{d");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a{ b \\{c d {d");

    // llength of a quoted literal whose `\{` collapses to a bare `{` errors as
    // an unmatched open brace, rather than counting it as an escaped element.
    let (ok, result, _) = run("set rc [catch {llength \"a b c \\{\"} m]\nlist $rc $m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "1 {unmatched open brace in list}");
}

/// `while` / `for` with the wrong argument count raise `wrong # args` rather
/// than lowering a truncated loop and running an argument as a command.
/// Regression for while-old-4.3 / for-old-1.7.
#[test]
fn while_for_reject_wrong_arg_count() {
    let (ok, result, _) = run("catch {while 1 2 3} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "wrong # args: should be \"while test command\"");

    let (ok, result, _) = run("catch {for 1 2 3 4 5} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "wrong # args: should be \"for start test next command\""
    );
}

/// An inline-compiled command substitution interpolates a leading-`$` word that
/// is more than a bare variable (`$i.x`, `$a$b`): the word is decomposed into
/// its `$var`/literal parts, not pushed raw. Regression for for-2.5 — `set m
/// [concat a "$i.x"]` had yielded the literal `$i.x`.
#[test]
fn inline_cmd_subst_interpolates_dollar_prefixed_word() {
    let (ok, result, _) = run("set i 5\nset m [concat a \"$i.x\"]\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a 5.x");

    let (ok, result, _) = run("set a 1\nset b 2\nset m [concat [list $a$b]]\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "12");
}

/// `break` / `continue` with any argument raise `wrong # args` — even inside a
/// loop, where the bare forms compile to a jump. The compiler only takes the
/// jump fast path when there are no arguments, so `break foo` reaches the
/// builtin. Regression for for-2.1 / for-3.1. The bare forms still control the
/// loop.
#[test]
fn break_continue_reject_arguments() {
    let (ok, result, _) =
        run("set r {}\nforeach x {1 2 3} {catch {break foo} m; lappend r $m; break}\nset r");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "{wrong # args: should be \"break\"}");

    let (ok, result, _) = run("catch {continue foo} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "wrong # args: should be \"continue\"");

    // The bare forms still break/continue the loop.
    let (ok, result, _) =
        run("set r {}\nforeach x {1 2 3 4} {if {$x==3} break; lappend r $x}\nset r");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "1 2");
}

/// The runtime `lset` builtin (the fallback for the dynamic / wrong-arg paths
/// the compiler does not inline) reports the usage error, replaces the whole
/// value with no index, and descends a flat index path. Regression for
/// lsetComp-1.1 (`lset` was only a codegen hook, so the fallback hit "invalid
/// command name").
#[test]
fn lset_runtime_builtin() {
    // Dispatched indirectly so it cannot be inlined → exercises the builtin.
    let (ok, result, _) = run("set c lset\ncatch {$c} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "wrong # args: should be \"lset listVar ?index? ?index ...? value\""
    );

    let (ok, result, _) = run("set l {{a b} {c d}}\nset c lset\n$c l 0 1 X\nset l");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "{a X} {c d}");
}

/// `lrepeat` rejects a negative count with C's exact message (lrepeat-1.4).
#[test]
fn lrepeat_negative_count_message() {
    let (ok, result, _) = run("catch {lrepeat -3 1} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "bad count \"-3\": must be integer >= 0");
}

/// `lpop` removes and returns an element of the list in a variable (default the
/// last), descending into nested sublists with several indices, and stores the
/// trimmed list back. Out-of-range / non-integer indices error like C.
#[test]
fn lpop_removes_and_returns_element() {
    let (ok, result, _) = run("set l {a b c d}\nset got [lpop l]\nlist $got $l");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "d {a b c}");

    let (ok, result, _) = run("set l {{a b} {c d}}\nset got [lpop l 0 1]\nlist $got $l");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "b {a {c d}}");

    let (ok, result, _) = run("set l {a b c}\ncatch {lpop l 99} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "index \"99\" out of range");
}

/// The compiled `LIST_RANGE_IMM` path resolves `end+N` indices like the
/// uncompiled command: as the last index `end+N` clamps to the end (keeps
/// everything), as the first index it is past the end (empty). Regression for
/// lrange.test's lrange-5 "shared compiled" battery, where `end+N` encodes as
/// `INDEX_END + N` (above the old `<= INDEX_END` detection) and was misread as a
/// huge plain index.
#[test]
fn compiled_lrange_handles_end_plus_n() {
    // `end+1` as the last index includes through the end.
    let (ok, result, _) = run("join [lrange {a b c} 0 end+1] -");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a-b-c");

    // `end+1` as the first index is past the end → empty.
    let (ok, result, _) = run("list [lrange {a b c} end+1 end]");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "{}");
}

/// A brace-string array-element target keeps its key LITERAL: `set {a($x)} 5`
/// stores the element whose key is the literal string `$x` (Tcl braces suppress
/// substitution), so reading it back with the same braced key returns the value
/// — it does NOT substitute `$x`. Regression for set-1.25 (the unbraced
/// `set a($x) 5` still substitutes, covered by the array tests).
#[test]
fn braced_array_key_is_literal() {
    let (ok, result, _) = run(concat!(
        "catch {unset a}\n",
        "set {a($x)} 5\n",
        "set {a($x)}",
    ));
    assert!(
        ok,
        "braced key must not substitute $x (would error `can't read x`): {result}"
    );
    assert_eq!(result, "5");
}

/// A non-braced array-element key carrying backslash escapes is an ordinary
/// Tcl word: its escapes are decoded (`\w` → `w`, `\ ` → space) before the
/// element lookup, matching C Tcl. Regression for set-1.26.
#[test]
fn unbraced_array_key_decodes_backslashes() {
    let (ok, result, _) = run(concat!(
        "catch {unset be}\n",
        "set be(\\w\\w) 1\n",
        "set be(a\\ a) 1\n",
        "lsort [array names be]",
    ));
    assert!(ok, "should complete: {result}");
    assert_eq!(result, "{a a} ww");
}

/// An array-element read nested inside a command substitution that is itself
/// part of an array-key template substitutes its own key. Regression for
/// set-1.26's `set be([set be(a:$a)][set b\e($a)]) 1`.
#[test]
fn nested_array_read_in_key_template_substitutes() {
    let (ok, result, _) = run(concat!(
        "set a x\n",
        "set be(a:x) 5\n",
        "set be(x) 1\n",
        "set be([set be(a:$a)][set b\\e($a)]) 1\n",
        "lsort [array names be]",
    ));
    assert!(ok, "nested key read must substitute $a: {result}");
    assert_eq!(result, "51 a:x x");
}

/// A `\<newline>` line continuation inside an inline command substitution
/// assigned with `set` is a word separator: the inner command keeps all its
/// arguments. Regression for the inline-cmd-subst tokenizer dropping an arg
/// across a continuation (spurious `wrong # args`), which crashed tcltest's
/// `SubstArguments` and every test file using the `{-body … -result …}` dict
/// form (info / lrepeat / lseq).
#[test]
fn multiline_cmd_subst_in_assignment_keeps_all_args() {
    let (ok, result, _) = run("proc f {x} {set w [string range $x \\\n\t1 3]; return $w}\nf abcde");
    assert!(ok, "multi-line cmd subst must keep all args: {result}");
    assert_eq!(result, "bcd");
}

/// Ensemble subcommands resolve an unambiguous prefix, matching C's
/// `Tcl_GetIndexFromObj`: `info command` → `commands`, `file ext` →
/// `extension`, `file dir` → `dirname`. (cmdAH.test's constraint setup uses
/// `info command`; the absence of prefix matching aborted the whole file.)
#[test]
fn info_and_file_subcommand_prefix_abbreviation() {
    assert_eq!(run("info command set").1, "set");
    assert_eq!(run("file ext a.tar.gz").1, ".gz");
    assert_eq!(run("file dir /a/b/c").1, "/a/b");
}

/// `dict with` maps a dict's keys to local variables, runs the body, and
/// reflects the variables back: a modified key is updated, an unset key is
/// removed, and a variable the body merely created is not added. A nested
/// key path updates the sub-dict in place.
#[test]
fn dict_with_maps_and_reflects() {
    // Modified `a`, removed `b` (unset), new `c` not added back.
    assert_eq!(
        run("set d {a 1 b 2}\ndict with d {set a 9; unset b; set c 3}\nset d").1,
        "a 9",
    );
    // Nested key path: the sub-dict at `x` is updated in place.
    assert_eq!(
        run("set d {x {a 1 b 2}}\ndict with d x {incr a 5}\nset d").1,
        "x {a 6 b 2}",
    );
}

/// A hard parse error inside a compiled body (`set "i"xxx` — non-whitespace
/// after a close-quote) is deferred to a catchable runtime error carrying the
/// exact C Tcl message. Regression for set-1.3 / set-3.3.
#[test]
fn extra_chars_after_close_quote_is_catchable() {
    let (ok, result, _) = run(concat!(
        "set i 10\n",
        "catch {set \"i\"xxx} msg\n",
        "set msg",
    ));
    assert!(ok, "the catch must contain the parse error: {result}");
    assert_eq!(result, "extra characters after close-quote");
}

/// `subst` decodes one backslash escape at a time and handles the multi-byte
/// forms: a `\` before a multi-byte UTF-8 character (previously a fixed
/// two-byte slice split the char boundary and panicked), `\xHH` hex, and the
/// `\<newline><whitespace>` line continuation. Regression for the subst.rs
/// panic that aborted subst.test.
#[test]
fn subst_backslash_escapes_handle_multibyte_and_hex() {
    assert_eq!(run("subst {\\é}").1, "é");
    assert_eq!(run("subst {\\x41}").1, "A");
    // Tcl 9 caps `\x` at two hex digits: the rest is literal text.
    assert_eq!(run("subst {\\x41BC}").1, "ABC");
    // `\` then newline then leading whitespace collapses to a single space —
    // for a CRLF line ending too.
    assert_eq!(run("subst \"a\\\n   b\"").1, "a b");
    assert_eq!(run("subst \"a\\\r\n   b\"").1, "a b");
}

/// `info level` runs through the shared Family-B core
/// (`tcl_cmd_core::info::level`, over the `Introspect` role trait): the current
/// depth with no argument, and the correct coercion error for a non-integer
/// argument (the VM previously diverged from the runtime / real Tcl with a
/// "bad level" message — routing through the shared core unifies the behaviour).
#[test]
fn info_level_shared_core() {
    assert_eq!(run("info level").1, "0"); // global scope: depth 0
    let (ok, result, _out) = run("info level foo");
    assert!(!ok);
    assert_eq!(result, "expected integer but got \"foo\"");
}

/// `info exists` runs through the shared core (`VarStore::exists`). Routing it
/// surfaced and fixed a VM bug: the current-frame existence check was scalar-only
/// (`var_exists`), so arrays like `::env` / `a` reported as not existing.
#[test]
fn info_exists_shared_core() {
    assert_eq!(run("info exists nope").1, "0");
    assert_eq!(run("set x 1\ninfo exists x").1, "1");
    // Arrays and array elements (the cases the old scalar-only check missed).
    assert_eq!(run("set a(k) v\ninfo exists a").1, "1");
    assert_eq!(run("set a(k) v\ninfo exists a(k)").1, "1");
    assert_eq!(run("set a(k) v\ninfo exists a(nope)").1, "0");
}

/// `namespace tail`/`qualifiers` run through the shared pure core
/// (`tcl_cmd_core::namespace`). Routing fixed the VM's `::`-run handling: a run
/// of 3+ colons is one separator (C semantics), where the VM's old `rsplit("::")`
/// yielded a stray `:`.
#[test]
fn namespace_tail_qualifiers_colon_runs() {
    assert_eq!(run("namespace tail ::a::b::c").1, "c");
    assert_eq!(run("namespace qualifiers ::a::b::c").1, "::a::b");
    assert_eq!(run("namespace tail foo:::").1, ""); // was ":" before the fix
    assert_eq!(run("namespace qualifiers foo:::").1, "foo");
}

/// `info complete` runs through the shared core (`tcl_cmd_core::info::complete`,
/// C's `Tcl_CommandComplete`). Routing fixed the VM, whose old counter tracked
/// brackets even inside `{braces}` (where `[` is literal): `{[}` is complete.
#[test]
fn info_complete_shared_core() {
    assert_eq!(run("info complete {set x 1}").1, "1");
    assert_eq!(run("info complete {set x [}").1, "0"); // unclosed bracket
    assert_eq!(run("info complete {{[}}").1, "1"); // `{[}` — was "0" before the fix
}

/// `namespace current`/`which` route through the shared `Namespaces` cores
/// (`current`/`name` and `find_command`/`command_name`).
#[test]
fn namespace_current_which_shared_core() {
    assert_eq!(run("namespace current").1, "::");
    assert_eq!(run("namespace eval foo {namespace current}").1, "::foo");
    assert_eq!(run("namespace which -command set").1, "::set");
    assert_eq!(run("namespace which -command no_such_cmd_xyz").1, "");
}

/// `file dirname`/`tail`/`extension`/`rootname` run through the shared
/// `/`-based byte path core (platform-independent), replacing the VM's old
/// `std::path::Path` versions.
#[test]
fn file_path_ops_shared_core() {
    assert_eq!(run("file tail /a/b/c").1, "c");
    assert_eq!(run("file dirname /a/b/c").1, "/a/b");
    assert_eq!(run("file extension a/b.txt").1, ".txt");
    assert_eq!(run("file rootname a/b.txt").1, "a/b");
    assert_eq!(run("file tail /a/b/").1, "b"); // trailing slash ignored
    assert_eq!(run("file dirname foo").1, ".");
}

#[test]
fn set_expr_puts() {
    let (ok, _result, out) = run("set x 5\nputs [expr {$x * 2}]\n");
    assert!(ok);
    assert_eq!(out, "10\n");
}

#[test]
fn expr_precedence() {
    let (ok, result, _out) = run("expr {3 + 4 * 2}");
    assert!(ok);
    assert_eq!(result, "11");
}

#[test]
fn floored_integer_division() {
    let (ok, result, _out) = run("expr {-7 / 2}");
    assert!(ok);
    assert_eq!(result, "-4");
}

#[test]
fn incr_and_var_substitution() {
    let (ok, _result, out) = run("set n 0\nincr n\nincr n 5\nputs $n\n");
    assert!(ok);
    assert_eq!(out, "6\n");
}

/// `incr` routed through the shared `tcl_cmd_core::var::incr_value` core (the
/// `VarStore` read + `ValueOps::int_add` tower seam). Covers the unset → 0 start,
/// array elements (the name carries `base(key)`; the VM parses it), and the
/// canonical coercion errors. Crucially, an overflowing `incr` now **promotes**
/// through the integer tower (`i128`, then an arbitrary-precision bignum),
/// matching tclsh, rather than silently wrapping as the old hand-rolled
/// `wrapping_add` did.
#[test]
fn incr_shared_core() {
    // Unset variable starts at 0 (no prior `set`).
    assert_eq!(run("incr fresh 5").1, "5");
    assert_eq!(run("incr fresh").1, "1");
    // Array element: the VM's var_get/var_set parse `a(k)` from the name.
    assert_eq!(run("set a(k) 10\nincr a(k) 5\nset a(k)").1, "15");
    // Non-integer current value / increment → the canonical coercion error.
    let (ok, msg, _) = run("set x abc\nincr x");
    assert!(!ok);
    assert_eq!(msg, "expected integer but got \"abc\"");
    let (ok, msg, _) = run("set y 1\nincr y xyz");
    assert!(!ok);
    assert_eq!(msg, "expected integer but got \"xyz\"");
    // i64::MAX + 1 promotes to the wider integer tower (as `expr` does and as
    // tclsh's arbitrary-precision integers do), rather than erroring or wrapping.
    let (ok, val, _) = run("set big 9223372036854775807\nincr big");
    assert!(ok);
    assert_eq!(val, "9223372036854775808");
    // A sum past i128 now promotes to an arbitrary-precision bignum too, matching
    // tclsh, instead of overflowing. i128::MAX + 1 == 2**127.
    let (ok, val, _) = run("set huge 170141183460469231731687303715884105727\nincr huge");
    assert!(ok);
    assert_eq!(val, "170141183460469231731687303715884105728");
}

/// `append`/`lappend` routed through the shared cores
/// (`tcl_cmd_core::var::{append_bytes, lappend_value}`). Pins the user-visible
/// behaviour against tclsh: concatenation/list building, the no-values read
/// form, and — the fix — `append`/`lappend` of an unset variable with no values
/// errors (`can't read`) rather than the VM's old silent empty-variable create.
#[test]
fn append_lappend_shared_core() {
    assert_eq!(run("set x ab\nappend x cd ef").1, "abcdef");
    assert_eq!(run("lappend y a b\nlappend y c").1, "a b c");
    // no-values read returns the value (set) and errors (unset).
    assert_eq!(run("set s hi\nappend s").1, "hi");
    let (ok, msg, _) = run("append nope");
    assert!(!ok);
    assert_eq!(msg, "can't read \"nope\": no such variable");
    // lappend no-values validates the current value as a list.
    let (ok, msg, _) = run("set z \"{\"\nlappend z");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// `binary encode`/`decode` — newly added to the VM, sharing the byte codecs
/// with the WASM runtime (`tcl_cmd_core::binary`). Values cross the VM's value
/// boundary via the byte-array convention. Pinned against tclsh 9.0.
#[test]
fn binary_encode_decode_shared() {
    assert_eq!(run("binary encode hex abc").1, "616263");
    assert_eq!(run("binary decode hex 616263").1, "abc");
    assert_eq!(run("binary encode base64 hello").1, "aGVsbG8=");
    assert_eq!(run("binary decode base64 aGVsbG8=").1, "hello");
    // base64 line wrapping at -maxlen.
    assert_eq!(run("binary encode base64 -maxlen 4 Manny").1, "TWFu\nbnk=");
    // uuencode round-trips through a command substitution.
    assert_eq!(
        run("binary decode uuencode [binary encode uuencode Cat]").1,
        "Cat"
    );
    // bad codec and the bad-subcommand dispatch message.
    let (ok, msg, _) = run("binary encode zip x");
    assert!(!ok);
    assert_eq!(
        msg,
        "unknown subcommand \"zip\": must be base64, hex, or uuencode"
    );
    let (ok, msg, _) = run("binary frobnicate x");
    assert!(!ok);
    assert_eq!(
        msg,
        "unknown or ambiguous subcommand \"frobnicate\": must be decode, encode, format, or scan"
    );
    // decode error message + errorCode.
    let (ok, msg, _) = run("binary decode hex 6z");
    assert!(!ok);
    assert_eq!(
        msg,
        "invalid hexadecimal digit \"z\" (U+00007A) at position 1"
    );
    assert_eq!(
        run("catch {binary decode hex 6z}; set ::errorCode").1,
        "TCL BINARY DECODE INVALID"
    );
}

/// `binary format`/`scan` routed through the shared grammar
/// (`tcl_cmd_core::binary::{format, scan}`), lifting the VM from its old
/// integer-only subset to the runtime's full code set — floats, 64-bit and
/// big-endian ints, `*` counts, and the round-trip. Pinned against tclsh 9.0.
#[test]
fn binary_format_scan_shared() {
    // codes the VM lacked before sharing: f/d floats, W (64-bit big-endian).
    assert_eq!(
        run("binary scan [binary format f 1.5] H* h; set h").1,
        "0000c03f"
    );
    assert_eq!(
        run("binary scan [binary format d 2.5] H* h; set h").1,
        "0000000000000440"
    );
    assert_eq!(
        run("binary scan [binary format W 1] H* h; set h").1,
        "0000000000000001"
    );
    // `*` count over a list.
    assert_eq!(
        run("binary scan [binary format s* {1 2 3}] H* h; set h").1,
        "010002000300"
    );
    // float round-trips through scan.
    assert_eq!(run("binary scan [binary format f 1.5] f v; set v").1, "1.5");
    // scan returns the conversion count.
    assert_eq!(run("binary scan abcd a2a2 x y").1, "2");
}

/// `lsort` comparison modes routed through the shared `tcl_cmd_core::sort` core,
/// lifting the VM from its old crude subset: `-dictionary` (was a plain byte
/// compare), the `-integer`-vs-`-real` distinction (both were double), and
/// `-nocase`. Pinned against tclsh 9.0.
#[test]
fn lsort_shared_comparison() {
    assert_eq!(run("lsort -dictionary {x10 x9 x100}").1, "x9 x10 x100");
    assert_eq!(run("lsort -integer {10 9 100 2}").1, "2 9 10 100");
    assert_eq!(run("lsort -real {1.5 1.25 10.0}").1, "1.25 1.5 10.0");
    assert_eq!(run("lsort -nocase {B a C b}").1, "a B b C");
    // -unique removes elements equal under the mode (1 and 01 for -integer).
    assert_eq!(run("lsort -integer -unique {1 01 1 2}").1, "1 2");
    // plain ascii is unchanged (lexical, so x10 < x100 < x9).
    assert_eq!(run("lsort {x10 x9 x100}").1, "x10 x100 x9");
}

/// `::tcl::mathop::*` — newly added to the VM, sharing `tcl_cmd_core::mathop`'s
/// fold/chain logic over the VM's `ExprOps`. Pinned against tclsh 9.0.
#[test]
fn mathop_shared() {
    assert_eq!(run("::tcl::mathop::+ 1 2 3").1, "6");
    assert_eq!(run("::tcl::mathop::+").1, "0"); // identity
    assert_eq!(run("::tcl::mathop::- 10 3 2").1, "5");
    assert_eq!(run("::tcl::mathop::- 5").1, "-5"); // 1 arg negates
    assert_eq!(run("::tcl::mathop::* 2 3 4").1, "24");
    assert_eq!(run("::tcl::mathop::/ 100 5 2").1, "10");
    assert_eq!(run("::tcl::mathop::/ 4").1, "0.25"); // reciprocal
    assert_eq!(run("::tcl::mathop::** 2 3 2").1, "512"); // right-assoc: 2**(3**2)
    assert_eq!(run("::tcl::mathop::% 17 5").1, "2");
    assert_eq!(run("::tcl::mathop::<< 1 4").1, "16");
    assert_eq!(run("::tcl::mathop::& 12 10").1, "8");
    assert_eq!(run("::tcl::mathop::~ 5").1, "-6");
    assert_eq!(run("::tcl::mathop::! 0").1, "1");
    assert_eq!(run("::tcl::mathop::< 1 2 3").1, "1"); // chained
    assert_eq!(run("::tcl::mathop::< 1 3 2").1, "0");
    assert_eq!(run("::tcl::mathop::== 5 5 5").1, "1");
    assert_eq!(run("::tcl::mathop::eq abc abc").1, "1");
    assert_eq!(run("::tcl::mathop::in b {a b c}").1, "1");
    assert_eq!(run("::tcl::mathop::ni x {a b c}").1, "1");
    let (ok, msg, _) = run("::tcl::mathop::% 1");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"::tcl::mathop::% integer integer\""
    );
}

/// `lseq` — newly added to the VM, sharing `tcl_cmd_core::lseq`'s decode key +
/// generation over the VM's `ValueOps` and an `eval_expr`-backed expression edge.
/// Pinned against tclsh 9.0.
#[test]
fn lseq_shared() {
    // Integer forms (count-only, ranges, by-step, count keyword, n n).
    assert_eq!(run("lseq 5").1, "0 1 2 3 4");
    assert_eq!(run("lseq 0").1, "");
    assert_eq!(run("lseq -5").1, ""); // negative count → empty
    assert_eq!(run("lseq 1 .. 10").1, "1 2 3 4 5 6 7 8 9 10");
    assert_eq!(run("lseq 10 .. 1").1, "10 9 8 7 6 5 4 3 2 1");
    assert_eq!(run("lseq 1 to 10 by 2").1, "1 3 5 7 9");
    assert_eq!(run("lseq 1 to 10 by -2").1, ""); // wrong-sign step → empty
    assert_eq!(run("lseq 5 count 5 by -2").1, "5 3 1 -1 -3");
    assert_eq!(run("lseq 3 by 2").1, "0 2 4");
    assert_eq!(run("lseq 5 1").1, "5 4 3 2 1");
    // Double precision matches the inputs' fractional digits.
    assert_eq!(run("lseq 0 0.5 by 0.1").1, "0.0 0.1 0.2 0.3 0.4 0.5");
    assert_eq!(run("lseq 25. to 5. by -5").1, "25.0 20.0 15.0 10.0 5.0");
    // A double-valued count stays an integer sequence.
    assert_eq!(run("lseq 5 count 5.0").1, "5 6 7 8 9");
    // Expression-valued arguments (the eval edge).
    assert_eq!(run("lseq 1+2 to 10").1, "3 4 5 6 7 8 9 10");
    assert_eq!(run("set n 3; lseq $n*2").1, "0 1 2 3 4 5");
    // Errors.
    let (ok, msg, _) = run("lseq");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"lseq n ??op? n ??by? n??\"");
    let (ok, msg, _) = run("lseq 10 2147483647");
    assert!(!ok);
    assert_eq!(msg, "max length of a Tcl list exceeded");
}

#[test]
fn string_and_numeric_compare() {
    let (ok, result, _out) = run("expr {9 < 10}");
    assert!(ok);
    assert_eq!(result, "1");
}

/// The `string` subcommands served by the shared `tcl-cmd-core` (driven over
/// the VM's `ValueOps`) behave identically end-to-end.
#[test]
fn string_core_helpers() {
    assert_eq!(run("puts [string length hello]").2, "5\n");
    assert_eq!(run("puts [string index hello 1]").2, "e\n");
    assert_eq!(run("puts [string range hello 1 3]").2, "ell\n");
    assert_eq!(run("puts [string reverse hello]").2, "olleh\n");
    assert_eq!(run("puts [string repeat ab 3]").2, "ababab\n");
    // end-relative and arithmetic index forms resolve in the shared core.
    assert_eq!(run("puts [string index hello end]").2, "o\n");
    assert_eq!(run("puts [string index hello 1+1]").2, "l\n");
    assert_eq!(run("puts [string range hello 1 end-1]").2, "ell\n");
    // count <= 0 yields empty; an out-of-range index yields empty.
    assert_eq!(run("puts \"[string repeat ab 0]<\"").2, "<\n");
    assert_eq!(run("puts \"[string index hello 99]<\"").2, "<\n");
    // `wordstart`/`wordend` — the VM gained these via the shared core (it errored
    // "not yet implemented" before). Pinned against tclsh 9.0.
    assert_eq!(run("string wordstart {hello world} 7").1, "6");
    assert_eq!(run("string wordend {hello world} 0").1, "5");
    assert_eq!(run("string wordstart {a_b.c} 2").1, "0"); // underscore is a word char
    assert_eq!(run("string wordend {ab cd} -5").1, "2"); // negative clamps to 0
    assert_eq!(run("string wordstart {ab cd} 10").1, "3"); // past-end clamps
}

/// A non-integer `string repeat` count produces the canonical coercion error,
/// surfaced through `tcl-cmd-core`'s `CmdError` from `ValueError`.
#[test]
fn string_repeat_bad_count_errors() {
    let (ok, result, _out) = run("string repeat ab x");
    assert!(!ok);
    assert_eq!(result, "expected integer but got \"x\"");
}

/// The `list`-family commands served by the shared `tcl-cmd-core` (driven over
/// the VM's `ValueOps` list operations) behave identically end-to-end.
#[test]
fn list_core_helpers() {
    assert_eq!(run("puts [llength {a b c}]").2, "3\n");
    assert_eq!(run("puts [lindex {a b c} 1]").2, "b\n");
    assert_eq!(run("puts [lindex {a b c} end]").2, "c\n");
    assert_eq!(run("puts [lindex {{a b} {c d}} 1 0]").2, "c\n");
    assert_eq!(run("puts [lrange {a b c d} 1 2]").2, "b c\n");
    assert_eq!(run("puts [lreverse {a b c}]").2, "c b a\n");
    assert_eq!(run("puts [lrepeat 3 x]").2, "x x x\n");
    assert_eq!(run("puts [concat a {b c} d]").2, "a b c d\n");
    assert_eq!(run("puts [join {a b c} -]").2, "a-b-c\n");
    assert_eq!(run("puts [llength [split a,b,c ,]]").2, "3\n");
    assert_eq!(run("puts [list x {a b} y]").2, "x {a b} y\n");
}

/// A malformed list index errors faithfully (the shared core errors where the
/// VM's old lenient path silently returned empty).
#[test]
fn lindex_bad_index_errors() {
    let (ok, result, _out) = run("lindex {a b c} foo");
    assert!(!ok);
    assert!(result.contains("bad index"), "got: {result}");
}

/// Case conversions (with the `?first? ?last?` range form), `replace`, `insert`.
#[test]
fn string_case_replace_insert() {
    assert_eq!(run("puts [string toupper hello]").2, "HELLO\n");
    assert_eq!(run("puts [string toupper hello 0 2]").2, "HELlo\n");
    assert_eq!(run("puts [string tolower HELLO 1]").2, "HeLLO\n");
    assert_eq!(run("puts [string totitle hello]").2, "Hello\n");
    assert_eq!(run("puts [string replace abcde 1 3]").2, "ae\n");
    assert_eq!(run("puts [string replace abcde 1 3 XY]").2, "aXYe\n");
    assert_eq!(run("puts [string insert abc 1 XY]").2, "aXYbc\n");
    assert_eq!(run("puts [string insert abc end Z]").2, "abcZ\n");
}

/// `string cat` and the `string trim` family, served by the shared core.
#[test]
fn string_cat_and_trim() {
    assert_eq!(run("puts [string cat foo bar baz]").2, "foobarbaz\n");
    assert_eq!(run("puts \"[string trim {  hi  }]<\"").2, "hi<\n");
    assert_eq!(run("puts \"[string trimleft xxabc x]<\"").2, "abc<\n");
    assert_eq!(run("puts \"[string trimright abcxx x]<\"").2, "abc<\n");
}

/// `string is` classification, served by the shared core (`string_is`).
#[test]
fn string_is_classes() {
    assert_eq!(run("puts [string is alpha abc]").2, "1\n");
    assert_eq!(run("puts [string is alpha abc1]").2, "0\n");
    assert_eq!(run("puts [string is integer 123]").2, "1\n");
    assert_eq!(run("puts [string is integer 12x]").2, "0\n");
    assert_eq!(run("puts [string is double 1.5]").2, "1\n");
    assert_eq!(run("puts [string is boolean yes]").2, "1\n");
    assert_eq!(run("puts [string is list {a b c}]").2, "1\n");
    // -failindex reports the first failing character.
    assert_eq!(run("string is alpha -failindex fi abc5\nset fi").1, "3");
}

/// `format` rendering, served by the shared core over `ValueOps`.
#[test]
fn format_core() {
    assert_eq!(run("puts [format %d-%s 5 hi]").2, "5-hi\n");
    assert_eq!(run("puts [format %05d 42]").2, "00042\n");
    assert_eq!(run("puts [format %05d -42]").2, "-0042\n");
    assert_eq!(run("puts [format %+d 42]").2, "+42\n");
    assert_eq!(run("puts \"[format %-5d 42]|\"").2, "42   |\n");
    assert_eq!(run("puts [format %#x 255]").2, "0xff\n");
    assert_eq!(run("puts [format %c 65]").2, "A\n");
    assert_eq!(run("puts [format %.2f 1.23456]").2, "1.23\n");
    assert_eq!(run("puts [format %.3s hello]").2, "hel\n");
    assert_eq!(run("puts [format 100%%]").2, "100%\n");
}

/// The pure `dict` family, served by the shared core (over the default
/// list-backed `dict_pairs`/`new_dict` seam).
#[test]
fn dict_core_helpers() {
    assert_eq!(run("puts [dict get {a 1 b 2} b]").2, "2\n");
    assert_eq!(run("puts [dict size {a 1 b 2}]").2, "2\n");
    assert_eq!(run("puts [dict keys {a 1 b 2}]").2, "a b\n");
    assert_eq!(run("puts [dict exists {a 1} a]").2, "1\n");
    assert_eq!(run("puts [dict exists {a 1} z]").2, "0\n");
    assert_eq!(
        run("puts [dict merge {a 1 b 2} {b 3 c 4}]").2,
        "a 1 b 3 c 4\n"
    );
    // canonicalisation (last value wins) — the VM's old non-deduping path was
    // wrong here; the shared core corrects it.
    assert_eq!(run("puts [dict get [dict create x 1 x 2] x]").2, "2\n");
}

/// A missing key errors faithfully.
#[test]
fn dict_get_missing_key_errors() {
    let (ok, result, _out) = run("dict get {a 1} z");
    assert!(!ok);
    assert!(result.contains("not known in dictionary"), "got: {result}");
}

/// `string first` / `string last` (character-indexed substring search).
#[test]
fn string_first_last() {
    assert_eq!(run("puts [string first bc abcbc]").2, "1\n");
    assert_eq!(run("puts [string first bc abcbc 2]").2, "3\n");
    assert_eq!(run("puts [string last bc abcbc]").2, "3\n");
    assert_eq!(run("puts [string first zz abc]").2, "-1\n");
}

/// A `PUSH1 idx` that pushes literal `idx` verbatim (no word substitution) — the
/// form the codegen uses for braced/constant words. Needed so a regexp pattern
/// like `^[0-9]+$` is pushed as data, not run through `subst_word`.
fn pushv(idx: i32) -> tcl_bytecode::Instruction {
    use tcl_bytecode::{Instruction, Op, Operand};
    let mut i = Instruction::new(Op::PUSH1, vec![Operand::Imm(idx)]);
    i.push_verbatim = true;
    i
}

/// Run a hand-assembled top-level instruction stream (with its literal pool),
/// returning the result string. `DONE` yields the value on top of the stack, so
/// the caller ends the stream with the opcode under test. This drives the VM's
/// opcode dispatch directly — unlike [`run`], it does not go through the codegen,
/// so it exercises an opcode even when no codegen path emits it yet.
fn run_asm(literals: &[&str], instrs: Vec<tcl_bytecode::Instruction>) -> String {
    use tcl_bytecode::{FunctionAsm, LiteralTable, LocalVarTable, ModuleAsm};
    let mut lits = LiteralTable::new();
    for l in literals {
        lits.intern(l);
    }
    let mut instrs = instrs;
    let labels =
        tcl_bytecode::layout::resolve_layout(&mut instrs, &std::collections::HashMap::new());
    let top = FunctionAsm {
        name: "::top".to_string(),
        literals: lits,
        lvt: LocalVarTable::new(&[]),
        instructions: instrs,
        labels,
        loop_targets: std::collections::HashMap::new(),
        body_base_line: 0,
        error_regions: Vec::new(),
    };
    let module = ModuleAsm {
        profile: tcl_dialect::DialectProfile::plain_tcl(),
        top_level: top,
        procedures: std::collections::HashMap::new(),
    };
    let mut vm = Vm::new();
    vm.run_module(&module).result.to_str().to_string()
}

/// `NUMERIC_TYPE` pushes C Tcl's numeric-tower code: 0 = not a number,
/// INT = 2, BIG = 3, DOUBLE = 4, NAN = 5.
#[test]
fn opcode_numeric_type() {
    use tcl_bytecode::{Instruction, Op};
    let asm = |lit: &str| {
        run_asm(
            &[lit],
            vec![
                pushv(0),
                Instruction::new(Op::NUMERIC_TYPE, vec![]),
                Instruction::new(Op::DONE, vec![]),
            ],
        )
    };
    assert_eq!(asm("42"), "2"); // wide integer
    assert_eq!(asm("-17"), "2");
    assert_eq!(asm("3.5"), "4"); // double
    assert_eq!(asm("1e10"), "4");
    assert_eq!(asm("abc"), "0"); // not a number
    assert_eq!(asm(""), "0");
    assert_eq!(asm("99999999999999999999999999"), "3"); // bignum
    assert_eq!(asm("NaN"), "5");
}

/// `STR_CLASS` (operand = class id) reports whether every character of the
/// popped string belongs to the class; the empty string matches.
#[test]
fn opcode_str_class() {
    use tcl_bytecode::{Instruction, Op, Operand};
    let asm = |lit: &str, class: &str| {
        let id = i32::from(tcl_bytecode::str_class_id(class).unwrap());
        run_asm(
            &[lit],
            vec![
                pushv(0),
                Instruction::new(Op::STR_CLASS, vec![Operand::Imm(id)]),
                Instruction::new(Op::DONE, vec![]),
            ],
        )
    };
    assert_eq!(asm("abc", "alpha"), "1");
    assert_eq!(asm("ab2", "alpha"), "0");
    assert_eq!(asm("12345", "digit"), "1");
    assert_eq!(asm("12x45", "digit"), "0");
    assert_eq!(asm("ABC", "upper"), "1");
    assert_eq!(asm("DeadBEEFG", "xdigit"), "0"); // G is not a hex digit
    assert_eq!(asm("dEadBEEF", "xdigit"), "1");
    assert_eq!(asm("", "alpha"), "1"); // empty string matches
}

/// `REGEXP` (operand = cflags) reports whether the pattern (under top) matches
/// the subject (top); the `TCL_REG_NOCASE` (8) bit makes it case-insensitive.
#[test]
fn opcode_regexp() {
    use tcl_bytecode::{Instruction, Op, Operand};
    let asm = |pat: &str, subj: &str, cflags: i32| {
        run_asm(
            &[pat, subj],
            vec![
                pushv(0), // pattern (under top)
                pushv(1), // subject (top)
                Instruction::new(Op::REGEXP, vec![Operand::Imm(cflags)]),
                Instruction::new(Op::DONE, vec![]),
            ],
        )
    };
    // TCL_REG_ADVANCED = 3 (the codegen's plain-form cflags).
    assert_eq!(asm("^[0-9]+$", "12345", 3), "1");
    assert_eq!(asm("^[0-9]+$", "12a45", 3), "0");
    assert_eq!(asm("ab+c", "xxabbbcyy", 3), "1");
    assert_eq!(asm("ABC", "abc", 3), "0");
    assert_eq!(asm("ABC", "abc", 3 | 0o10), "1"); // TCL_REG_NOCASE
}

/// `lset` (the `LSET_LIST` single-index / index-path form and the `LSET_FLAT`
/// multi-index form) end-to-end. Both the proc-local opcode form and the
/// top-level stack form route through these opcodes; verified against tclsh 9.0.
#[test]
fn lset_list_and_flat() {
    // Single index (LSET_LIST).
    assert_eq!(run("set l {a b c}; lset l 1 X; puts $l").2, "a X c\n");
    assert_eq!(run("set l {a b c}; lset l end 9; puts $l").2, "a b 9\n");
    // Appending one past the end.
    assert_eq!(run("set l {a b c}; lset l 3 D; puts $l").2, "a b c D\n");
    // Empty index path replaces the whole value.
    assert_eq!(run("set l {a b c}; lset l {} Z; puts $l").2, "Z\n");
    // An index *path* given as one list argument (LSET_LIST descends it).
    assert_eq!(
        run("set l {{a b} {c d}}; lset l {1 0} X; puts $l").2,
        "{a b} {X d}\n"
    );
    // Multiple flat index arguments (LSET_FLAT), top-level and in a proc.
    assert_eq!(
        run("set l {{a b} {c d}}; lset l 1 0 X; puts $l").2,
        "{a b} {X d}\n"
    );
    assert_eq!(
        run("proc f {} { set l {{a b} {c d}}; lset l 1 0 X; return $l }; puts [f]").2,
        "{a b} {X d}\n"
    );
    // Errors match tclsh 9.0.
    let (ok, msg, _) = run("set l {a b c}; lset l 5 X");
    assert!(!ok);
    assert_eq!(msg, "index \"5\" out of range");
    let (ok, msg, _) = run("set l {a b c}; lset l foo X");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad index \"foo\": must be integer?[+-]integer? or end?[+-]integer?"
    );
}

/// `tailcall` — the proc finishes and the named command runs in its place
/// (in the caller's activation). A tail-recursive loop must not grow the
/// activation stack or hit the recursion limit (the whole point of `tailcall`).
#[test]
fn tailcall_basic_and_deep() {
    // Tail call replaces the proc's continuation.
    assert_eq!(run("proc f {} { tailcall set g 9 }; f; puts $g").2, "9\n");
    assert_eq!(
        run("proc a {} { return A }; proc b {} { tailcall a }; puts [b]").2,
        "A\n"
    );
    // Tail-recursive factorial.
    assert_eq!(
        run("proc fact {n acc} { if {$n<=1} { return $acc }; tailcall fact [expr {$n-1}] [expr {$n*$acc}] }; puts [fact 5 1]").2,
        "120\n"
    );
    // Deep tail recursion: far past the recursion limit — must not overflow.
    assert_eq!(
        run("proc cd {n} { if {$n<=0} { return done }; tailcall cd [expr {$n-1}] }; puts [cd 200000]").2,
        "done\n"
    );
    // No-args tailcall terminates the proc with an empty result.
    assert_eq!(
        run("proc p {} { tailcall; puts X }; p; puts done").2,
        "done\n"
    );
    assert_eq!(run("proc p {} { tailcall }; puts [p]Y").2, "Y\n");
    // Dynamic command word (generic-invoke fallback).
    assert_eq!(
        run("proc p {} { set c set; tailcall $c z 5 }; p; puts $z").2,
        "5\n"
    );
    // Outside a proc it errors (matching tclsh 9.0).
    let (ok, msg, _) = run("tailcall foo");
    assert!(!ok);
    assert_eq!(
        msg,
        "tailcall can only be called from a proc, lambda or method"
    );
}

/// `time command ?count?` — runs the body in the current frame `count` times and
/// reports `N microseconds per iteration` as a 4-element list (C Tcl
/// `Tcl_TimeObjCmd`). Timing is non-deterministic, so the assertions cover the
/// shape, the current-frame side effects, error propagation, and the count<=0
/// edge (which is the one fixed value, 0).
#[test]
fn time_command() {
    // Body runs in the caller's frame.
    assert_eq!(run("time {set x 99}; puts $x").2, "99\n");
    assert_eq!(run("set n 0; time {incr n} 4; puts $n").2, "4\n");
    // count <= 0 runs nothing and reports 0 (an integer).
    assert_eq!(
        run("puts [time {set x 1} 0]").2,
        "0 microseconds per iteration\n"
    );
    // Result is the 4-element list shape.
    assert_eq!(
        run("puts [lrange [time {set x 1}] 1 3]").2,
        "microseconds per iteration\n"
    );
    // A body error propagates.
    let (ok, msg, _) = run("time {error boom}");
    assert!(!ok);
    assert_eq!(msg, "boom");
    // Arity error.
    let (ok, msg, _) = run("time");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"time command ?count?\"");
}

/// `REVERSE` (reverse top N stack items), `LINDEX_MULTI` (nested `lindex`), and
/// `STR_REPLACE` (`string replace`) — driven directly through the VM's opcode
/// dispatch (these are emitted by the inline command-substitution emitter).
#[test]
fn opcode_reverse_lindex_multi_str_replace() {
    use tcl_bytecode::{Instruction, Op, Operand};
    // REVERSE 2 then concat: push x,y → reverse → y,x → "yx".
    let r = run_asm(
        &["x", "y"],
        vec![
            pushv(0),
            pushv(1),
            Instruction::new(Op::REVERSE, vec![Operand::Imm(2)]),
            Instruction::new(Op::STR_CONCAT1, vec![Operand::Imm(2)]),
            Instruction::new(Op::DONE, vec![]),
        ],
    );
    assert_eq!(r, "yx");
    // LINDEX_MULTI 3: lindex {{a b} {c d}} 1 0 → "c".
    let r = run_asm(
        &["{a b} {c d}", "1", "0"],
        vec![
            pushv(0),
            pushv(1),
            pushv(2),
            Instruction::new(Op::LINDEX_MULTI, vec![Operand::Imm(3)]),
            Instruction::new(Op::DONE, vec![]),
        ],
    );
    assert_eq!(r, "c");
    // STR_REPLACE: string replace "abcde" 1 3 "XY" → "aXYe".
    let r = run_asm(
        &["abcde", "1", "3", "XY"],
        vec![
            pushv(0),
            pushv(1),
            pushv(2),
            pushv(3),
            Instruction::new(Op::STR_REPLACE, vec![]),
            Instruction::new(Op::DONE, vec![]),
        ],
    );
    assert_eq!(r, "aXYe");
    // STR_REPLACE whole-string replace and end indices.
    let r = run_asm(
        &["abcde", "0", "end", "Z"],
        vec![
            pushv(0),
            pushv(1),
            pushv(2),
            pushv(3),
            Instruction::new(Op::STR_REPLACE, vec![]),
            Instruction::new(Op::DONE, vec![]),
        ],
    );
    assert_eq!(r, "Z");
}

/// `string is boolean` in value position compiles to C's `INST_TRY_CVT_TO_BOOLEAN`
/// sequence (`tryCvtToBoolean; jumpTrue L; push ""; streq; jump end; L: pop;
/// push "1"`), which only balances because the opcode leaves the value in place
/// and pushes the 0/1 flag on top (`tclExecute.c:6404-6414`, table
/// `tclCompile.c:616`: stack effect **+1**, "No errors"). While the arm
/// pop/re-pushed the *value* and errored on a non-boolean, the first two cases
/// raised `expected boolean value but got "foo"` and the third silently lost the
/// `set` (the codegen's `pop` ate `storeStk`'s operand instead of the value).
#[test]
fn string_is_boolean_inline_try_cvt_to_boolean() {
    assert_eq!(run("set x [string is boolean foo]").1, "0");
    assert_eq!(run("set x [string is boolean {}]").1, "1");
    // The stack-corruption case: `$x` has to be readable afterwards.
    assert_eq!(run("set x [string is boolean yes]; set x").1, "1");
    assert_eq!(run("puts [list a [string is boolean yes] b]").2, "a 1 b\n");
    // The flag is C's *strict* acceptor (`ParseBoolean`, `tclObj.c:2100-2280`),
    // so a truthy non-boolean number is not a boolean — matching the
    // uncompiled `string is boolean` command.
    assert_eq!(run("set x [string is boolean 2]").1, "0");
    assert_eq!(run("set x [string is boolean 1]").1, "1");
    assert_eq!(
        run("string is boolean 2").1,
        run("set x [string is boolean 2]").1
    );
}

/// `lindex` with a non-constant index compiles to `LIST_INDEX`, which is C's
/// full `TclLindexList` (`tclExecute.c:4696-4768` falling through to line 4754):
/// a list-valued index drills nested sublists, an empty index list is the whole
/// list, and a garbage spec errors. The old `unwrap_or(-1)` funnel made all
/// three the empty string.
#[test]
fn compiled_list_index_implements_tcl_lindex_list() {
    assert_eq!(run("set i {1 0}; set r [lindex {{a b} {c d}} $i]").1, "c");
    assert_eq!(
        run("set i {}; set r [lindex {{a b} {c d}} $i]").1,
        "{a b} {c d}"
    );
    assert_eq!(run("set i 9; set r [lindex {a b c} $i]").1, "");
    let (ok, msg, _) = run("set i foo; set r [lindex {a b c} $i]");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad index \"foo\": must be integer?[+-]integer? or end?[+-]integer?"
    );
    // `LINDEX_MULTI` (≥ 2 indices) is C's `TclLindexFlat` and validates each.
    assert_eq!(run("set i 0; set r [lindex {{a b} {c d}} 1 $i]").1, "c");
    let (ok, msg, _) = run("set i foo; set r [lindex {{a b} {c d}} 0 $i]");
    assert!(!ok);
    assert!(msg.starts_with("bad index \"foo\""), "got: {msg}");
    // A malformed *list value* still propagates its own conversion error, as
    // C's `TclListObjGetElements` failure does — not `bad index`.
    let (ok, msg, _) = run("set l \"a \\{b\"; set i 0; set r [lindex $l $i]");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
    // The compiled path agrees with the uncompiled command.
    assert_eq!(
        run("set i {1 0}; set r [lindex {{a b} {c d}} $i]").1,
        run("puts -nonewline [lindex {{a b} {c d}} {1 0}]").2
    );
}

/// `string index`/`string range` with a non-constant index compile to
/// `STR_INDEX`/`STR_RANGE`, whose indices go through `TclGetIntForIndexM` with a
/// parse failure as `goto gotError` (`tclExecute.c:5348-5351`, `5388-5397`).
/// Clamping a bad spec to 0/-1 returned a plausible substring instead.
#[test]
fn compiled_str_index_and_range_reject_bad_indices() {
    let bad = |spec: &str| {
        format!("bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?")
    };
    for (src, spec) in [
        ("set i foo; set r [string index abc $i]", "foo"),
        ("set i {1 0}; set r [string index abc $i]", "1 0"),
        ("set i foo; set r [string range abcde $i 2]", "foo"),
        ("set i foo; set r [string range abcde 1 $i]", "foo"),
    ] {
        let (ok, msg, _) = run(src);
        assert!(!ok, "{src} should error");
        assert_eq!(msg, bad(spec), "{src}");
    }
    // Valid indices — including out-of-range ones — keep C's clamp/empty result.
    assert_eq!(run("set i 1; set r [string index abc $i]").1, "b");
    assert_eq!(run("set i end; set r [string index abc $i]").1, "c");
    assert_eq!(run("set i 9; set r [string index abc $i]").1, "");
    assert_eq!(run("set i -1; set r [string index abc $i]").1, "");
    assert_eq!(run("set i 1; set r [string range abcde $i 3]").1, "bcd");
    assert_eq!(run("set i -3; set r [string range abcde $i 2]").1, "abc");
    assert_eq!(run("set i 3; set r [string range abcde $i 1]").1, "");
}

/// `set x [cmd …]` inline command-substitution assign:
/// the value is compiled inline (specialised opcode where available, else a
/// generic invoke) rather than pushed as raw `[…]` text for the runtime
/// `subst_word` fallback. Results verified against tclsh 9.0. The
/// `array names … $pat*` case pins the composite-arg fix (`$pat*` must be
/// substituted, not pushed literally) that unblocked real `tcltest.tcl`.
#[test]
fn set_inline_cmd_subst() {
    assert_eq!(run("set x [string length hello]; puts $x").2, "5\n");
    assert_eq!(
        run("proc f {s} { set n [string toupper $s]; return $n }; puts [f abc]").2,
        "ABC\n"
    );
    assert_eq!(run("set x [lindex {a b c} 1]; puts $x").2, "b\n");
    assert_eq!(run("set x [llength {a b c d}]; puts $x").2, "4\n");
    assert_eq!(run("set x [expr {2+3}]; puts $x").2, "5\n");
    assert_eq!(run("set d [dict get {a 1 b 2} b]; puts $d").2, "2\n");
    assert_eq!(
        run("proc h {} { set c [catch {error boom} m]; return $c:$m }; puts [h]").2,
        "1:boom\n"
    );
    // Composite arg (`$pat*`) must substitute then append the glob, not push
    // the literal `$pat*` — the bug that broke tcltest's MatchingOption.
    assert_eq!(
        run("array set A {xa 1 xb 2 yc 3}\nset m [lsort [array names A x*]]; puts $m").2,
        "xa xb\n"
    );
    assert_eq!(
        run("array set A {xa 1 xb 2 yc 3}\nset p x\nset m [lsort [array names A $p*]]; puts $m").2,
        "xa xb\n"
    );
    // Excluded forms still fold / behave as before.
    assert_eq!(run("set l [list a b c]; puts $l").2, "a b c\n");
    assert_eq!(
        run("set d [dict create k1 v1 k2 v2]; puts [dict get $d k2]").2,
        "v2\n"
    );
}

/// `INVOKE_REPLACE` (ensemble-rewrite invoke): `[string equal -nocase …]` /
/// `[string compare -length …]` in value position inline to the resolved
/// implementation (`::tcl::string::equal …`) via the opcode. This is sensitive to
/// inline `set x [cmd]` routing the flagged forms through the
/// (otherwise unimplemented) opcode; verified against tclsh 9.0.
#[test]
fn invoke_replace_string_ensemble() {
    assert_eq!(
        run("set x [string equal -nocase ABC abc]; puts $x").2,
        "1\n"
    );
    assert_eq!(run("set x [string equal abc abd]; puts $x").2, "0\n");
    assert_eq!(
        run("proc f {} { set x [string compare -nocase A a]; return $x }; puts [f]").2,
        "0\n"
    );
    assert_eq!(
        run("set x [string equal -length 2 abcd abXX]; puts $x").2,
        "1\n"
    );
}

/// Top-level (non-proc) mutating `dict` subcommands compile to the ensemble
/// rewrite `INVOKE_REPLACE` form (`push dict <sub> … ::tcl::dict::<sub>;
/// invokeReplace`), matching tclsh 9.0, rather than a plain generic invoke.
#[test]
fn dict_toplevel_ensemble() {
    assert_eq!(run("dict set d k v; puts $d").2, "k v\n");
    assert_eq!(
        run("dict set d a 1; dict set d b 2; puts $d").2,
        "a 1 b 2\n"
    );
    assert_eq!(run("dict incr c n 3; puts $c").2, "n 3\n");
    assert_eq!(
        run("dict append e k ab; dict append e k cd; puts $e").2,
        "k abcd\n"
    );
    assert_eq!(run("dict lappend f k 1 2; puts $f").2, "k {1 2}\n");
    assert_eq!(run("set d {a 1 b 2}; dict unset d a; puts $d").2, "b 2\n");
}

/// `{*}` expansion inside a command substitution in value position
/// (`set x [cmd a {*}$args b]`) compiles to `expandStart … expandStkTop N;
/// invokeExpanded` (result on the stack), matching tclsh 9.0.
/// Statement-position `{*}` already lowered correctly.
#[test]
fn set_inline_cmd_subst_expand() {
    assert_eq!(
        run("proc f {args} {set x [concat a {*}$args b]; return $x}; puts [f 1 2 3]").2,
        "a 1 2 3 b\n"
    );
    assert_eq!(
        run("set a {1 2}; set x [concat {*}$a 3]; puts $x").2,
        "1 2 3\n"
    );
    assert_eq!(
        run("proc h {args} {set x [string cat {*}$args]; return $x}; puts [h a b c]").2,
        "abc\n"
    );
    // `{*}{a b c}` expands a braced list verbatim.
    assert_eq!(
        run("set x [concat p {*}{a b c} q]; puts $x").2,
        "p a b c q\n"
    );
}

/// `encoding` — matches the tree-walking runtime (`runtime/rust`): UTF-8
/// internal model, so `convertto`/`convertfrom` pass data
/// through, `system` is `utf-8`, `names` lists the supported set, `dirs` is
/// ignored. (Real codepage conversion is unimplemented on both sides.)
#[test]
fn encoding_command() {
    assert_eq!(run("encoding system").1, "utf-8");
    assert_eq!(run("encoding names").1, "utf-8 unicode ascii iso8859-1");
    assert_eq!(run("encoding convertto utf-8 abc").1, "abc");
    assert_eq!(run("encoding convertfrom utf-8 hello").1, "hello");
    assert_eq!(run("encoding convertto abc").1, "abc"); // no encoding arg
    assert_eq!(run("encoding dirs").1, "");
    let (ok, msg, _) = run("encoding bogus");
    assert!(!ok);
    assert_eq!(
        msg,
        "unknown or ambiguous subcommand \"bogus\": must be convertfrom, convertto, dirs, names, or system"
    );
    let (ok, msg, _) = run("encoding");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"encoding subcommand ?arg ...?\""
    );
}

/// `interp eval {} script …` — only the current interpreter (empty path) exists;
/// its scripts evaluate in the current frame, like `eval` (verified vs tclsh 9.0).
/// A non-empty path is an unknown child interpreter.
#[test]
fn interp_eval_current() {
    assert_eq!(run("interp eval {} {expr 1+1}").1, "2");
    assert_eq!(run("set y 0; interp eval {} {set y 5}; puts $y").2, "5\n");
    // Runs in the current frame: a proc local is visible.
    assert_eq!(
        run("proc p {} { set loc 1; interp eval {} {info exists loc} }; puts [p]").2,
        "1\n"
    );
    // Multiple script args are concatenated.
    let (ok, msg, _) = run("interp eval {} {string length hello} {append}");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"string length string\"");
    // A named (non-empty) path is an unknown interpreter.
    let (ok, msg, _) = run("interp eval foo {set x 1}");
    assert!(!ok);
    assert_eq!(msg, "could not find interpreter \"foo\"");
}

/// Regression tests for inline `set x [cmd]` command-substitution edge cases.
#[test]
fn inline_cmd_subst_review_fixes() {
    // `string is` generic fallback must keep the `is` subcommand (it used to be
    // dropped, yielding `string list …`).
    assert_eq!(run("set x [string is list {a b c}]; puts $x").2, "1\n");
    assert_eq!(run("set x [string is wideinteger 99]; puts $x").2, "1\n");
    // `-strict` char-class: STR_CLASS can't honour it (empty is a member), so it
    // defers to the command — empty string is a non-member under -strict.
    assert_eq!(
        run("proc p {} { string is alpha -strict \"\" }; puts [p]").2,
        "0\n"
    );
    assert_eq!(
        run("proc p {} { string is alpha -strict abc }; puts [p]").2,
        "1\n"
    );
    // `regexp` option words (`--`) are consumed; no stale stack value survives.
    assert_eq!(
        run("proc p {} { set r [regexp -- a a]; return $r }; puts [p]X").2,
        "1X\n"
    );
    // A multi-command substitution that also contains `{*}` runs as two commands
    // (the separator fallback precedes the expand path).
    assert_eq!(
        run("set x [set y 1; concat a {*}{b c}]; puts $x").2,
        "a b c\n"
    );
}
