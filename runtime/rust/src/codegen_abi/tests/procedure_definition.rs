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

//! Native procedure entries and the argv-based definition imports: dispatch, fallback, replacement, logging and input validation.

use super::*;

/// Arm the stub with the one command its body runs, and reset the counter.
fn stub_argv(words: &[&[u8]]) {
    STUB_ARGV.with(|argv| {
        *argv.borrow_mut() = words.iter().map(|word| word.to_vec()).collect();
    });
    STUB_LOG.with(|log| *log.borrow_mut() = None);
    STUB_CALLS.with(|calls| calls.set(0));
}

/// Give the armed stub the statement site it logs when its command fails.
fn stub_logs(line: i32, source: &[u8]) {
    STUB_LOG.with(|log| *log.borrow_mut() = Some((line, source.to_vec())));
}

fn stub_calls() -> u32 {
    STUB_CALLS.with(Cell::get)
}

/// A stub compiled proc body.
///
/// It dispatches one prebuilt argv through the same [`tcl_invoke_argv`] an
/// emitted body uses for a statement it could not lower, and forwards that
/// completion as its own. Nothing here is test-shaped: the completion
/// protocol, the borrowed argv, and the absent frame/activation prologue
/// are exactly what `native_emit` must emit for a proc entry.
pub(super) unsafe extern "C" fn stub_body(
    _argv: *const *mut TclObj,
    _argc: i32,
    out: *mut TclCompletionAbi,
) -> i32 {
    STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
    let words = STUB_ARGV.with(|argv| argv.borrow().clone());
    // SAFETY: each word is a valid readable slice; `owned_word` boxes it
    // and takes the caller-owned reference the borrowed-argv ABI needs.
    let boxed: Vec<*mut TclObj> = words
        .iter()
        .map(|word| unsafe { owned_word(word) })
        .collect();
    // SAFETY: `boxed` are live caller-owned words and `out` is the
    // runtime's own zeroed completion storage.
    let status = unsafe {
        tcl_invoke_argv(
            boxed.as_ptr(),
            i32::try_from(boxed.len()).expect("stub argv fits i32"),
            out,
        )
    };
    assert_eq!(status, TCL_INVOKE_ABI_OK);
    // SAFETY: balances `owned_word`'s reference on each word.
    unsafe { release_words(&boxed) };
    // The statement's error edge: an emitted body logs its own source site
    // here, because nothing else will.
    // SAFETY: `out` was written by the invoke above.
    if unsafe { (*out).code } == 1 {
        if let Some((line, source)) = STUB_LOG.with(|log| log.borrow().clone()) {
            // SAFETY: `source` is a live readable slice.
            unsafe {
                tcl_codegen_log_command(
                    line,
                    source.as_ptr(),
                    i32::try_from(source.len()).expect("source fits i32"),
                );
            }
        }
    }
    NATIVE_PROC_STATUS_RAN
}

/// A stub in the shape the emitter produces for a body whose last command
/// leaves no result of its own — a bare `return`, or a `puts`.
///
/// It reports success with `out` untouched, which the ABI defines as "the
/// runtime's own result is the body's answer".
unsafe extern "C" fn stub_no_result(
    _argv: *const *mut TclObj,
    _argc: i32,
    _out: *mut TclCompletionAbi,
) -> i32 {
    STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
    NATIVE_PROC_STATUS_RAN
}

/// A stub in the shape the emitter produces for a plain `return v`: it
/// records the pending return state the `return` command records, then
/// completes with the raw `Code::Return` the interpreted body reports.
unsafe extern "C" fn stub_plain_return(
    _argv: *const *mut TclObj,
    _argc: i32,
    out: *mut TclCompletionAbi,
) -> i32 {
    STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
    tcl_codegen_return_state(1, Code::Ok.as_int() as i32);
    // SAFETY: `out` is the runtime's own zeroed completion storage, and
    // `owned_word` hands over one owned reference on the result.
    unsafe {
        (*out).code = Code::Return.as_int() as i32;
        (*out).result = owned_word(b"v");
    }
    NATIVE_PROC_STATUS_RAN
}

/// The same stub without the state write — what an emitter that treated
/// `Code::Return` as nothing but a completion code would produce.
unsafe extern "C" fn stub_return_without_state(
    _argv: *const *mut TclObj,
    _argc: i32,
    out: *mut TclCompletionAbi,
) -> i32 {
    STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
    // SAFETY: as above.
    unsafe {
        (*out).code = Code::Return.as_int() as i32;
        (*out).result = owned_word(b"v");
    }
    NATIVE_PROC_STATUS_RAN
}

/// A stub that declines before doing anything at all — the only shape a
/// decline may take, since the runtime then runs the source body in the
/// frame it already prepared.
pub(in super::super) unsafe extern "C" fn stub_declines(
    _argv: *const *mut TclObj,
    _argc: i32,
    _out: *mut TclCompletionAbi,
) -> i32 {
    STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
    NATIVE_PROC_STATUS_DECLINED
}

/// The call an emitted module makes for a `proc` statement.
unsafe fn define_native(name: &[u8], params: &[u8], body: &[u8], entry: Option<NativeProcEntry>) {
    // SAFETY: all three ranges are live readable slices.
    let code = unsafe {
        tcl_codegen_proc_define_native(
            name.as_ptr(),
            i32::try_from(name.len()).expect("name fits i32"),
            params.as_ptr(),
            i32::try_from(params.len()).expect("params fit i32"),
            body.as_ptr(),
            i32::try_from(body.len()).expect("body fits i32"),
            entry,
        )
    };
    assert_eq!(code, 0, "defining {}", String::from_utf8_lossy(name));
}

/// Evaluate `script` and return its completion code and result text.
pub(super) unsafe fn eval(interp: *mut Interp, script: &str) -> (Code, Vec<u8>) {
    // SAFETY: the caller holds a live interpreter.
    unsafe {
        (
            (*interp).eval_str(script.as_bytes()),
            (*interp).result_bytes(),
        )
    }
}

/// A fresh interpreter, installed as the current one for the codegen ABI.
unsafe fn with_current_interp(body: impl FnOnce(*mut Interp)) {
    let interp = tcl_runtime_create_interp();
    // SAFETY: a live interpreter this helper owns for the call's duration.
    unsafe {
        tcl_runtime_set_current_interp(interp);
        body(interp);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    }
}

#[test]
fn a_native_entry_runs_instead_of_the_source_body_and_is_counted() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            assert_eq!(tcl_codegen_native_proc_dispatches(), 0);
            stub_argv(&[b"return", b"native"]);
            define_native(b"p", b"a", b"return source", Some(stub_body));

            assert_eq!(eval(interp, "p x"), (Code::Ok, b"native".to_vec()));
            assert_eq!(stub_calls(), 1);
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

            // The source body is still the proc's body: it is what a step
            // trace forces, what a decline falls back to, and what
            // introspection reports.
            assert_eq!(eval(interp, "info body p").1, b"return source");
            assert_eq!(eval(interp, "info args p").1, b"a");
        });
    });
}

#[test]
fn proc_register_is_proc_define_native_with_no_entry() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            let (name, params, body) = (b"only", b"a", b"return source");
            assert_eq!(
                tcl_codegen_proc_register(
                    name.as_ptr(),
                    i32::try_from(name.len()).unwrap(),
                    params.as_ptr(),
                    i32::try_from(params.len()).unwrap(),
                    body.as_ptr(),
                    i32::try_from(body.len()).unwrap(),
                ),
                0
            );
            assert_eq!(eval(interp, "only x"), (Code::Ok, b"source".to_vec()));
            assert_eq!(tcl_codegen_native_proc_dispatches(), 0);
        });
    });
}

#[test]
fn the_native_entry_runs_in_the_call_frame_run_proc_prepared() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            // `info level 0` is the double-frame detector: it reads the
            // *current* frame's recorded invocation words, and only the
            // frame `run_proc` pushed has any. An entry that pushed its
            // own would report the wrong level and have no words at all.
            stub_argv(&[b"info", b"level", b"0"]);
            define_native(b"lvl", b"a b", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "lvl 7 8").1, b"lvl 7 8");

            stub_argv(&[b"info", b"level"]);
            define_native(b"depth", b"", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "depth").1, b"1");

            // Formals are bound by name before the entry runs, so the body
            // reads them as ordinary cells.
            stub_argv(&[b"set", b"b"]);
            define_native(b"second", b"a b", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "second 7 8").1, b"8");

            // The body runs in the proc's defining namespace.
            assert_eq!(eval(interp, "namespace eval ns {}").0, Code::Ok);
            stub_argv(&[b"namespace", b"current"]);
            define_native(b"ns::inner", b"", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "ns::inner").1, b"::ns");
        });
    });
}

#[test]
fn wrong_number_of_arguments_never_reaches_the_native_entry() {
    // The message is `run_proc`'s, produced before any body runs, so the
    // native and source paths cannot differ. tclsh 9.0.4 and 8.6.16 both
    // print `wrong # args: should be "greet name"`.
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"return", b"native"]);
            define_native(b"greet", b"name", b"return source", Some(stub_body));

            assert_eq!(
                eval(interp, "greet"),
                (
                    Code::Error,
                    b"wrong # args: should be \"greet name\"".to_vec()
                )
            );
            assert_eq!(
                eval(interp, "greet a b"),
                (
                    Code::Error,
                    b"wrong # args: should be \"greet name\"".to_vec()
                )
            );
            assert_eq!(stub_calls(), 0, "the entry must never see a bad arity");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 0);
        });
    });
}

/// A multi-line body whose error carries a `(procedure ... line N)` frame.
const BOOM_BODY: &[u8] = b"\n    set b 1\n    error \"bad $a\"\n";

#[test]
fn a_declining_entry_falls_back_to_the_source_body_observably_unchanged() {
    // naming.runtime.compiled-procedure-fallback-and-source-log
    // docs/design/analysis/name-resolution-proofs/compiled-procedure-fallback-and-source-log.md
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            // The same proc, same name, same body — once with a
            // declining entry and once with none. Comparing under one name
            // is what makes the comparison total: the proc name appears in
            // the `(procedure ...)` frame and in the caller's own.
            stub_argv(&[]);
            define_native(b"boom", b"a", BOOM_BODY, Some(stub_declines));
            assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
            let declined_msg = eval(interp, "set m").1;
            let declined_info = eval(interp, "dict get $o -errorinfo").1;
            let declined_stack = eval(interp, "dict get $o -errorstack").1;
            let declined_code = eval(interp, "dict get $o -errorcode").1;
            assert_eq!(stub_calls(), 1, "the entry was asked, and declined");
            assert_eq!(
                tcl_codegen_native_proc_dispatches(),
                0,
                "a decline is not a native run"
            );

            define_native(b"boom", b"a", BOOM_BODY, None);
            assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
            assert_eq!(eval(interp, "set m").1, declined_msg);
            assert_eq!(eval(interp, "dict get $o -errorinfo").1, declined_info);
            assert_eq!(eval(interp, "dict get $o -errorstack").1, declined_stack);
            assert_eq!(eval(interp, "dict get $o -errorcode").1, declined_code);

            // And that shared answer is the interpreted one: tclsh 9.0.4
            // and 8.6.16 print exactly these five lines.
            assert_eq!(declined_msg, b"bad Q");
            assert_eq!(
                String::from_utf8_lossy(&declined_info),
                "bad Q\n    while executing\n\"error \"bad $a\"\"\n    \
                     (procedure \"boom\" line 3)\n    invoked from within\n\"boom Q\""
            );
        });
    });
}

#[test]
fn redefining_the_proc_drops_the_native_entry() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"return", b"native"]);
            define_native(b"twice", b"x", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "twice 4").1, b"native");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

            // The `proc` command builds a definition with no entry, so the
            // new source body runs interpreted. Nothing invalidates
            // anything: the entry lived on the definition that just went.
            assert_eq!(
                eval(interp, "proc twice {x} {return redefined}").0,
                Code::Ok
            );
            assert_eq!(eval(interp, "twice 4").1, b"redefined");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);
            assert_eq!(stub_calls(), 1);
        });
    });
}

#[test]
fn rename_carries_the_native_entry_and_deleting_the_proc_drops_it() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"return", b"native"]);
            define_native(b"twice", b"x", b"return source", Some(stub_body));

            assert_eq!(eval(interp, "rename twice thrice").0, Code::Ok);
            assert_eq!(eval(interp, "thrice 4").1, b"native");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);
            assert_eq!(eval(interp, "catch {twice 4} m"), (Code::Ok, b"1".to_vec()));
            assert_eq!(eval(interp, "set m").1, b"invalid command name \"twice\"");

            assert_eq!(eval(interp, "rename thrice {}").0, Code::Ok);
            assert_eq!(eval(interp, "catch {thrice 4} m").1, b"1");
            assert_eq!(eval(interp, "set m").1, b"invalid command name \"thrice\"");

            // A whole namespace going away takes its definitions, and the
            // entries on them, with it.
            assert_eq!(eval(interp, "namespace eval ns {}").0, Code::Ok);
            define_native(b"ns::inner", b"", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "ns::inner").1, b"native");
            assert_eq!(eval(interp, "namespace delete ns").0, Code::Ok);
            assert_eq!(eval(interp, "catch {ns::inner} m").1, b"1");
            assert_eq!(
                eval(interp, "set m").1,
                b"invalid command name \"ns::inner\""
            );
            assert_eq!(tcl_codegen_native_proc_dispatches(), 2);
        });
    });
}

#[test]
fn a_live_step_trace_forces_the_source_body() {
    // A natively lowered `set`/`list`/`return` never reaches `dispatch`,
    // so it would fire no `enterstep` and the transcript would silently
    // lose lines. `dispatch_traced` installs the proc's own step traces
    // before dispatching, so one read of `step_active` in `run_proc`
    // covers both this proc's traces and an outer one's.
    //
    // Transcript pinned against tclsh 9.0.4 and 8.6.16: step traces fire
    // on the *substituted* command, after its `[...]` words have run.
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"return", b"NATIVE"]);
            define_native(
                b"stepped",
                b"x",
                b"set y $x; return [list $y $y]",
                Some(stub_body),
            );
            assert_eq!(eval(interp, "stepped ab").1, b"NATIVE");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

            assert_eq!(
                eval(
                    interp,
                    "trace add execution stepped enterstep \
                         {apply {{cmd op} {lappend ::steps $cmd}}}"
                )
                .0,
                Code::Ok
            );
            assert_eq!(
                eval(interp, "stepped ab").1,
                b"ab ab",
                "the traced call must run the source body"
            );
            assert_eq!(
                eval(interp, "set ::steps").1,
                b"{set y ab} {list ab ab} {return {ab ab}}"
            );
            assert_eq!(
                tcl_codegen_native_proc_dispatches(),
                1,
                "the traced call must have run the source body"
            );
            assert_eq!(stub_calls(), 1);

            // Removing the trace restores the compiled body.
            assert_eq!(
                eval(
                    interp,
                    "trace remove execution stepped enterstep \
                         {apply {{cmd op} {lappend ::steps $cmd}}}"
                )
                .0,
                Code::Ok
            );
            assert_eq!(eval(interp, "stepped ab").1, b"NATIVE");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 2);
        });
    });
}

/// An error out of a compiled body takes `run_proc`'s ordinary error tail
/// — with the two gaps a compiled statement's missing `log_command_info`
/// leaves, both pinned here as a gate that fails in either direction.
#[test]
fn an_error_from_the_native_body_unwinds_through_the_procedure_frame() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"error", b"bad Q"]);
            define_native(b"boom", b"a", BOOM_BODY, Some(stub_body));

            assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
            assert_eq!(eval(interp, "set m").1, b"bad Q");
            // `run_proc`'s error tail is shared by both bodies: the
            // `(procedure ...)` frame and the TIP 348 `CALL` entry are
            // appended exactly as they are for the source body.
            let info = eval(interp, "dict get $o -errorinfo").1;
            assert!(
                info.starts_with(b"bad Q"),
                "errorInfo seeds from the message: {}",
                String::from_utf8_lossy(&info)
            );
            assert!(
                info.windows(26)
                    .any(|w| w == b"(procedure \"boom\" line 1)\n"),
                "errorInfo must carry the procedure frame: {}",
                String::from_utf8_lossy(&info)
            );
            // A compiled statement calls no `log_command_info`, and that
            // one absence has two visible consequences here, both closed
            // by `tcl_codegen_log_command` once the emitter calls it on a
            // statement's error edge:
            //
            //  - no `while executing "<source text>"` frame, and
            //    `error_line` never advances, so the procedure frame reads
            //    `line 1` where the interpreted body reads `line 3`;
            //  - no TIP 348 `CALL` entry either. `error_stack_push_call`
            //    still runs, but it deliberately refuses to chain a `CALL`
            //    onto an error stack with no inner context yet
            //    (`reset_error_stack`), so the errorstack holds only the
            //    caller's own `INNER`.
            assert!(
                !info.windows(16).any(|w| w == b"while executing "),
                "unexpected `while executing` frame: {}",
                String::from_utf8_lossy(&info)
            );
            assert_eq!(eval(interp, "dict get $o -errorstack").1, b"INNER {boom Q}");
        });
    });
}

/// Source logging reproduces the public source trace. A generic argv
/// invocation retains its own inner error-stack context, independently from
/// the original body's selected native instruction.
#[test]
fn a_logged_statement_site_preserves_the_source_trace_without_instruction_identity() {
    // naming.runtime.compiled-procedure-fallback-and-source-log
    // docs/design/analysis/name-resolution-proofs/compiled-procedure-fallback-and-source-log.md
    // This ABI stub exercises source logging, not native returnImm admission.
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            define_native(b"boom", b"a", BOOM_BODY, None);
            assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
            let source_info = eval(interp, "dict get $o -errorinfo").1;
            let source_stack = eval(interp, "dict get $o -errorstack").1;

            // `error "bad $a"` is the third line of the body and the exact
            // text of the failing statement — what the emitter carries.
            stub_argv(&[b"error", b"bad Q"]);
            stub_logs(3, b"error \"bad $a\"");
            define_native(b"boom", b"a", BOOM_BODY, Some(stub_body));
            assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

            assert_eq!(eval(interp, "dict get $o -errorinfo").1, source_info);
            assert_eq!(
                source_stack, b"INNER {returnImm {bad Q} {}} CALL {boom Q}",
                "the source body owns its selected native instruction context"
            );
            assert_eq!(
                eval(interp, "dict get $o -errorstack").1,
                b"INNER {error \"bad $a\"} CALL {boom Q}",
                "logging source text does not grant an instruction context"
            );
            assert_eq!(
                String::from_utf8_lossy(&source_info),
                "bad Q\n    while executing\n\"error \"bad $a\"\"\n    \
                     (procedure \"boom\" line 3)\n    invoked from within\n\"boom Q\"",
                "and that shared answer is tclsh 9.0.4's and 8.6.16's"
            );
        });
    });
}

#[test]
fn a_logged_command_is_truncated_and_deduplicated_like_an_interpreted_one() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            // One `log_command_bytes` protocol for both callers: the first
            // frame seeds errorInfo from the message and truncates the
            // command at 150 bytes with `...`; a second call for the same
            // error is a no-op (`already_logged`).
            let long = format!("error nope ;# {}", "x".repeat(200));
            stub_argv(&[b"error", b"nope"]);
            stub_logs(1, long.as_bytes());
            define_native(b"long", b"", b"error nope", Some(stub_body));
            assert_eq!(eval(interp, "catch {long} m o").1, b"1");
            let info = eval(interp, "dict get $o -errorinfo").1;
            let expected = format!(
                "nope\n    while executing\n\"{}...\"\n    (procedure \"long\" line 1)",
                &long[..150]
            );
            assert!(
                info.starts_with(expected.as_bytes()),
                "{}",
                String::from_utf8_lossy(&info)
            );
        });
    });
}

#[test]
fn the_native_path_holds_one_activation_per_level_and_refuses_at_the_bound() {
    // `run_native_body` holds the activation the eval loop would hold, and
    // the body's own `tcl_invoke_argv` holds one more: two eval-depth
    // units per Tcl level, the same as the interpreted plain-recursion
    // shape. A level that cost more would fail sooner and one that cost
    // less would fail later, so the reached depth pins the accounting.
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"recur"]);
            define_native(b"recur", b"", b"return source", Some(stub_body));

            assert_eq!(eval(interp, "catch {recur} m").1, b"1");
            assert_eq!(
                eval(interp, "set m").1,
                b"too many nested evaluations (infinite loop?)",
                "the refusal is the eval loop's own message"
            );
            assert_eq!(
                tcl_codegen_native_proc_dispatches(),
                63,
                "the outermost script and the `catch` body cost one \
                     eval-depth unit each, leaving 126 of the native bound's \
                     128 for 63 levels at two apiece"
            );
        });
    });
}

/// A completion that comes back with no result answers with the empty
/// string, never the caller's.
///
/// `Tcl_EvalEx` resets the result at entry and the eval loop can skip that
/// because its first command always sets one. A compiled body cannot make
/// that promise — `tcl_codegen_var_set` and `tcl_codegen_puts` set no
/// interpreter result — so `run_native_body` resets it instead. Without
/// that a `puts`-ending body would answer with whatever the caller's last
/// command left, which is the kind of wrong answer no result assertion on
/// the body itself would catch.
#[test]
fn a_native_entry_that_writes_no_result_answers_with_the_empty_string() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            define_native(b"quiet", b"", b"return source", Some(stub_no_result));
            assert_eq!(eval(interp, "set marker abc"), (Code::Ok, b"abc".to_vec()));
            assert_eq!(eval(interp, "quiet"), (Code::Ok, Vec::new()));
            assert_eq!(stub_calls(), 1);
            // The caller's own live result is the interesting case: the
            // list's first element sets it, and the second must not report
            // it back.
            assert_eq!(
                eval(interp, "list [set marker] [quiet]").1,
                b"abc {}".to_vec()
            );
        });
    });
}

/// A compiled `return` has to record the pending return state, because
/// the return boundary consumes it whether or not anything set it.
///
/// `catch {return -level 2 …}` leaves `(level 2, code error)` behind:
/// `catch` reports the code without crossing a boundary, so nothing
/// decrements it. An entry that then completes with `Code::Return` and no
/// state write has its level counted down from 2 instead of 1, and the
/// call propagates `Code::Return` to its caller instead of returning `v`.
/// tclsh 9.0.4 and 8.6.16 both answer `0 v` here.
#[test]
fn a_compiled_return_records_the_state_the_return_command_records() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            define_native(b"p", b"", b"return source", Some(stub_plain_return));
            define_native(
                b"bad",
                b"",
                b"return source",
                Some(stub_return_without_state),
            );

            // Poison the pending state exactly as a caught deferred
            // return does, then ask each entry for its answer.
            let poison = "catch {return -level 2 -code error boom}";
            assert_eq!(eval(interp, poison).1, b"2");
            assert_eq!(eval(interp, "list [catch {p} m] $m").1, b"0 v");

            assert_eq!(eval(interp, poison).1, b"2");
            assert_eq!(
                eval(interp, "list [catch {bad} m] $m").1,
                b"2 v",
                "without the state write the boundary consumes the stale \
                     level and the call propagates a return"
            );
        });
    });
}

#[test]
fn the_dispatch_counter_reports_a_missing_interpreter_distinctly() {
    tcl_runtime_set_current_interp(ptr::null_mut());
    assert_eq!(tcl_codegen_native_proc_dispatches(), -1);
}

unsafe fn define_guarded_words(words: &[*mut TclObj], expected: GuardIdentity) -> TclCompletionAbi {
    let mut out = TclCompletionAbi {
        code: 777,
        result: ptr::null_mut(),
        options: ptr::null_mut(),
    };
    let status = unsafe {
        tcl_codegen_proc_define_guarded_argv(
            words.as_ptr(),
            i32::try_from(words.len()).unwrap(),
            Some(stub_body),
            expected.value(),
            &mut out,
        )
    };
    assert_eq!(status, TCL_INVOKE_ABI_OK);
    out
}

#[test]
fn guarded_original_definition_uses_stock_selection_and_preserves_generic_handlers() {
    leak_free(|| unsafe {
        for (setup, head, compiled, answer) in [
            ("", "proc", true, ""),
            ("rename proc original", "original", true, ""),
            ("interp alias {} forwarded {} proc", "forwarded", false, ""),
            (
                "rename proc original; original proc args {return REPLACED}",
                "proc",
                false,
                "REPLACED",
            ),
            (
                "trace add execution proc enter {apply {{args} {lappend ::observed ENTER}}}",
                "proc",
                false,
                "",
            ),
        ] {
            let mut interp = scalar_interpreter("tcl9.0");
            tcl_runtime_set_current_interp(&mut interp);
            assert_eq!(interp.eval_str(setup.as_bytes()), Code::Ok);
            stub_argv(&[b"set", b"::compiled_seen", b"yes"]);
            let expected = tcl_registry::NativeLowering::Definition
                .definition_guard_identity(interp.native_invocation_dialect())
                .unwrap();
            let words = [
                owned_word(head.as_bytes()),
                owned_word(b"p"),
                owned_word(b""),
                owned_word(b"set ::compiled_seen yes"),
            ];
            let mut out = define_guarded_words(&words, expected);
            assert_eq!(out.code, i32::try_from(Code::Ok.as_int()).unwrap());
            assert_eq!(obj_bytes(out.result), answer.as_bytes());
            tcl_completion_release(&mut out);
            release_words(&words);
            let actual = match interp.resolve_dispatchable(crate::namespace::GLOBAL, b"p") {
                Some(crate::interp::Command::Proc(proc)) => proc.declaration().native.is_some(),
                _ => false,
            };
            assert_eq!(actual, compiled, "{setup}");
            if answer.is_empty() {
                assert_eq!(interp.eval_str(b"p"), Code::Ok);
                assert_eq!(interp.result_bytes(), b"yes");
                assert_eq!(stub_calls() > 0, compiled, "{setup}");
            }
            if setup.starts_with("trace") {
                assert_eq!(interp.eval_str(b"set ::observed"), Code::Ok);
                assert_eq!(interp.result_bytes(), b"ENTER");
            }
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn guarded_original_definition_refuses_foreign_namespace_policy_and_unknown_head() {
    leak_free(|| unsafe {
        for kind in 0..3 {
            let mut interp = scalar_interpreter("tcl9.0");
            tcl_runtime_set_current_interp(&mut interp);
            let expected = tcl_registry::NativeLowering::Definition
                .definition_guard_identity(interp.native_invocation_dialect())
                .unwrap();
            let mut head = b"proc".as_slice();
            if kind == 0 {
                let holder = interp
                    .namespaces_mut()
                    .ensure_namespace(crate::namespace::GLOBAL, b"holder");
                interp.set_current_ns(holder);
            } else if kind == 1 {
                interp.set_runtime_version(TclVersion::V8_6);
            } else {
                head = b"missing_original_definer";
            }
            let words = [
                owned_word(head),
                owned_word(b"p"),
                owned_word(b""),
                owned_word(b"set ::compiled_seen yes"),
            ];
            let mut out = define_guarded_words(&words, expected);
            if kind == 2 {
                assert_eq!(out.code, i32::try_from(Code::Error.as_int()).unwrap());
            } else {
                assert_eq!(out.code, i32::try_from(Code::Ok.as_int()).unwrap());
                let command = interp
                    .resolve_dispatchable(interp.current_ns(), b"p")
                    .unwrap();
                let crate::interp::Command::Proc(proc) = command else {
                    panic!("ordinary procedure definition");
                };
                assert!(proc.declaration().native.is_none());
            }
            tcl_completion_release(&mut out);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

unsafe fn define_original_words(
    words: &[*mut TclObj],
    entry: Option<NativeProcEntry>,
) -> TclCompletionAbi {
    let mut out = TclCompletionAbi {
        code: 777,
        result: ptr::null_mut(),
        options: ptr::null_mut(),
    };
    // SAFETY: caller-owned live objects and writable local completion storage.
    let status = unsafe {
        tcl_codegen_proc_define_original_argv(
            words.as_ptr(),
            i32::try_from(words.len()).unwrap(),
            entry,
            &mut out,
        )
    };
    assert_eq!(status, TCL_INVOKE_ABI_OK);
    out
}

#[test]
fn original_definition_argv_uses_actual_name_and_current_holder() {
    leak_free(|| unsafe {
        for environment in ["tcl8.6", "tcl9.0"] {
            let mut interp = scalar_interpreter(environment);
            tcl_runtime_set_current_interp(&mut interp);
            for holder_name in [b"holder".as_slice(), b":".as_slice(), b":ns".as_slice()] {
                let holder = interp
                    .namespaces_mut()
                    .ensure_namespace(crate::namespace::GLOBAL, holder_name);
                interp.set_current_ns(holder);
                let name = if holder_name == b"holder" {
                    b":audit".as_slice()
                } else {
                    b"audit".as_slice()
                };
                let words = [
                    owned_word(b"proc"),
                    owned_word(name),
                    owned_word(b""),
                    owned_word(b"return ORIGINAL"),
                ];
                let mut completion = define_original_words(&words, None);
                assert_eq!(completion.code, i32::try_from(Code::Ok.as_int()).unwrap());
                tcl_completion_release(&mut completion);
                // The ABI borrows these objects; they stay live until cleanup.
                release_words(&words);
                assert_eq!(interp.eval_str(name), Code::Ok);
                assert_eq!(interp.result_bytes(), b"ORIGINAL");
                assert_eq!(interp.current_ns(), holder);
            }
            interp.set_current_ns(crate::namespace::GLOBAL);
            assert_eq!(interp.eval_str(b"info commands :audit"), Code::Ok);
            assert!(interp.result_bytes().is_empty());
            assert_eq!(interp.eval_str(b"info commands audit"), Code::Ok);
            assert!(interp.result_bytes().is_empty());
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn original_definition_argv_does_not_create_a_missing_holder() {
    leak_free(|| unsafe {
        for environment in ["tcl8.6", "tcl9.0"] {
            let mut interp = scalar_interpreter(environment);
            tcl_runtime_set_current_interp(&mut interp);
            let words = [
                owned_word(b"proc"),
                owned_word(b"missing::audit"),
                owned_word(b""),
                owned_word(b"return NEVER"),
            ];
            let mut completion = define_original_words(&words, None);
            assert_eq!(
                completion.code,
                i32::try_from(Code::Error.as_int()).unwrap()
            );
            tcl_completion_release(&mut completion);
            release_words(&words);
            assert_eq!(interp.eval_str(b"namespace exists missing"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"0");
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn original_root_name_object_retains_the_installed_software_entry() {
    // This callable is an existing Rust software ABI stub, not Native evidence.
    leak_free(|| unsafe {
        for environment in ["tcl8.6", "tcl9.0"] {
            let mut interp = scalar_interpreter(environment);
            tcl_runtime_set_current_interp(&mut interp);
            stub_argv(&[b"return", b"compiled"]);
            let words = [
                owned_word(b"proc"),
                owned_word(b":audit"),
                owned_word(b""),
                owned_word(b"return source"),
            ];
            let before = tcl_codegen_native_proc_dispatches();
            let mut completion = define_original_words(&words, Some(stub_body));
            assert_eq!(completion.code, i32::try_from(Code::Ok.as_int()).unwrap());
            tcl_completion_release(&mut completion);
            release_words(&words);
            assert_eq!(interp.eval_str(b":audit"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"compiled");
            assert_eq!(tcl_codegen_native_proc_dispatches(), before + 1);
            assert_eq!(interp.eval_str(b"info body :audit"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"return source");
            assert_eq!(interp.eval_str(b"info commands audit"), Code::Ok);
            assert!(interp.result_bytes().is_empty());
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn original_definition_argv_replacement_retires_its_compiled_entry() {
    // The existing callable is a Rust software ABI stub, not Native provenance.
    leak_free(|| unsafe {
        for environment in ["tcl8.6", "tcl9.0"] {
            let mut interp = scalar_interpreter(environment);
            tcl_runtime_set_current_interp(&mut interp);
            stub_argv(&[b"info", b"level", b"0"]);
            let original = [
                owned_word(b"proc"),
                owned_word(b":audit"),
                owned_word(b"name"),
                owned_word(b"return OLD_SOURCE"),
            ];
            let before = tcl_codegen_native_proc_dispatches();
            let mut completion = define_original_words(&original, Some(stub_body));
            assert_eq!(completion.code, i32::try_from(Code::Ok.as_int()).unwrap());
            tcl_completion_release(&mut completion);
            assert_eq!(interp.eval_str(b":audit first"), Code::Ok);
            assert_eq!(interp.result_bytes(), b":audit first");
            assert_eq!(stub_calls(), 1);
            assert_eq!(tcl_codegen_native_proc_dispatches(), before + 1);
            assert_eq!(interp.eval_str(b"info body :audit"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"return OLD_SOURCE");

            // Reuse the live original head/name/formal argument objects. Their
            // caller-owned references span both borrowed ABI calls; only the
            // new source body has an additional caller-owned reference.
            let body = owned_word(b"return [list REPLACED $name [info level 0]]");
            let replacement = [original[0], original[1], original[2], body];
            let mut completion = define_original_words(&replacement, None);
            assert_eq!(completion.code, i32::try_from(Code::Ok.as_int()).unwrap());
            tcl_completion_release(&mut completion);
            assert_eq!(interp.eval_str(b":audit second"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"REPLACED second {:audit second}");
            assert_eq!(
                stub_calls(),
                1,
                "replacement must not enter the retired callable"
            );
            assert_eq!(tcl_codegen_native_proc_dispatches(), before + 1);
            assert_eq!(interp.eval_str(b"info body :audit"), Code::Ok);
            assert_eq!(
                interp.result_bytes(),
                b"return [list REPLACED $name [info level 0]]"
            );
            assert_eq!(interp.eval_str(b"info args :audit"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"name");
            assert_eq!(interp.eval_str(b"info commands audit"), Code::Ok);
            assert!(interp.result_bytes().is_empty());
            // Replacement retires the procedure's old references, but cannot
            // consume or mutate either caller's original argument objects.
            assert_eq!(obj_bytes(original[1]), b":audit");
            assert_eq!(obj_bytes(original[3]), b"return OLD_SOURCE");
            assert_eq!(
                obj_bytes(body),
                b"return [list REPLACED $name [info level 0]]"
            );
            release_words(&[body]);
            release_words(&original);
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn original_definition_argv_validates_inputs_without_consuming_arguments() {
    leak_free(|| unsafe {
        tcl_runtime_set_current_interp(ptr::null_mut());
        let mut out = empty_completion();
        assert_eq!(
            tcl_codegen_proc_define_original_argv(ptr::null(), 1, None, &mut out),
            TCL_INVOKE_ABI_NO_CURRENT_INTERP,
        );
        assert_eq!(out.code, 1);
        tcl_completion_release(&mut out);

        let mut interp = scalar_interpreter("tcl9.0");
        tcl_runtime_set_current_interp(&mut interp);
        for argc in [-1, 0] {
            assert_eq!(
                tcl_codegen_proc_define_original_argv(ptr::null(), argc, None, &mut out),
                TCL_INVOKE_ABI_INVALID_ARGC,
            );
            assert_eq!(out.code, 1);
            tcl_completion_release(&mut out);
        }
        assert_eq!(
            tcl_codegen_proc_define_original_argv(ptr::null(), 1, None, &mut out),
            TCL_INVOKE_ABI_NULL_ARGV,
        );
        tcl_completion_release(&mut out);
        let invalid = [ptr::null_mut::<TclObj>()];
        assert_eq!(
            tcl_codegen_proc_define_original_argv(invalid.as_ptr(), 1, None, &mut out),
            TCL_INVOKE_ABI_NULL_WORD,
        );
        tcl_completion_release(&mut out);

        let words = [
            owned_word(b"proc"),
            owned_word(b":audit"),
            owned_word(b""),
            owned_word(b"return ORIGINAL"),
        ];
        assert_eq!(
            tcl_codegen_proc_define_original_argv(words.as_ptr(), 4, None, ptr::null_mut()),
            TCL_INVOKE_ABI_NULL_OUT,
        );
        assert_eq!(obj_bytes(words[1]), b":audit");
        assert_eq!(interp.eval_str(b"info commands :audit"), Code::Ok);
        assert!(interp.result_bytes().is_empty());
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
    });
}

#[test]
fn original_definition_argv_preserves_pending_host_refusal_and_untouched_output() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let failing = [
            owned_word(b"string"),
            owned_word(b"toupper"),
            owned_word(b"A\xff"),
        ];
        let words = [
            owned_word(b"proc"),
            owned_word(b":audit"),
            owned_word(b""),
            owned_word(b"return NEVER"),
        ];
        let mut out = TclCompletionAbi {
            code: 77,
            result: ptr::null_mut(),
            options: ptr::null_mut(),
        };
        assert_eq!(
            tcl_invoke_argv(failing.as_ptr(), 3, &mut out),
            TCL_INVOKE_ABI_HOST_REFUSED,
        );
        assert_eq!(
            tcl_codegen_proc_define_original_argv(words.as_ptr(), 4, None, &mut out),
            TCL_INVOKE_ABI_HOST_REFUSED,
        );
        assert_eq!(out.code, 77);
        assert!(out.result.is_null() && out.options.is_null());
        assert_eq!(tcl_codegen_host_refusal_pending(), 1);
        assert_eq!((*interp).unicode_access_refusal().unwrap().valid_up_to, 1);
        assert_eq!(obj_bytes(words[1]), b":audit");
        assert_eq!(obj_bytes(words[3]), b"return NEVER");
        release_words(&failing);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}
