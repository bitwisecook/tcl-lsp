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

//! What a traced or failing command leaves behind: variable traces, `errorInfo` and `errorCode`, the pending host cause, and how a top-level completion settles.

use super::*;

/// `Traces::fire` runs a variable's registered traces, aborting the access with
/// the wrapped `can't read "var": <msg>` error when a read/write callback fails,
/// and swallowing `unset`-trace errors (matching C and the runtime). Traces are
/// registered via the `trace` command (callbacks evaluate through the compiler).
#[test]
fn traces_fire_ok_error_and_unset() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    // `;#` comments out the appended `name elem op` trace words.
    for add in [
        "trace add variable x read {list ok;#}",
        "trace add variable y read {error boom;#}",
        "trace add variable z unset {error nope;#}",
    ] {
        assert!(vm.eval_source(add).expect("compiles").code.is_ok());
    }

    // Read trace, callback succeeds → the access proceeds.
    assert!(
        Traces::fire(&mut vm, "x", "read")
            .expect("actual Guest trace has no Host refusal")
            .is_ok()
    );
    // Read trace, callback errors → the access aborts with the wrapped error.
    assert_eq!(
        &*Traces::fire(&mut vm, "y", "read")
            .expect("actual Guest trace has no Host refusal")
            .unwrap_err()
            .to_str(),
        "can't read \"y\": boom",
    );
    // An `unset`-trace error does not abort the access.
    assert!(
        Traces::fire(&mut vm, "z", "unset")
            .expect("actual Guest trace has no Host refusal")
            .is_ok()
    );
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

/// The actual Host command cause remains outside both facade Guest results.
#[test]
fn original_trace_facades_keep_the_full_pending_host_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual VM Host producer and transport; no native callback or parser observation.
    let mut vm = tcl_vm::test_support::native_vm();
    let _ = vm.refuse_host_command("original VM variable trace refusal".to_owned());
    let first = Traces::fire_bytes(&mut vm, b"x", "read")
        .expect_err("the full Host refusal is outside the Guest value");
    let tcl_runtime_api::NativeExecutionError::HostCommandRefusal(payload) = &first else {
        panic!("the actual Host command cause must remain complete")
    };
    assert_eq!(payload.reason, "original VM variable trace refusal");
    assert_eq!(
        Traces::fire(&mut vm, "x", "read").expect_err("full original Host cause"),
        first
    );
}

// Restore every original four-release top-level completion observation.
// Only the genuine constructor and checked result projection replace retired
// generic setup; original source, expected bytes, release set and assertions stay.
fn settle_original_top_level(vm: &mut Vm, source: &str) -> (Code, String) {
    let completion = vm
        .try_eval_source(source)
        .expect("original source provider");
    let settled = vm.settle_top_level(completion);
    let code = settled.code;
    let original = vm
        .capture_original_operand_projection()
        .expect("actual selected core projection");
    let text = vm
        .original_operand_unicode(&original, &settled.result)
        .expect("original top-level result Unicode");
    drop(settled);
    vm.check_original_operand_projection(&original)
        .expect("projection current through completion retirement");
    vm.finish_original_operand_projection(original)
        .expect("original projection settlement");
    (code, text)
}

#[test]
fn a_top_level_completion_settles_as_tclsh_reports_it() {
    use tcl_dialect::TclVersion;
    for (version, unexpected) in [
        (TclVersion::V8_4, "NONE"),
        (TclVersion::V8_5, "NONE"),
        (TclVersion::V8_6, "TCL UNEXPECTED_RESULT_CODE 3"),
        (TclVersion::V9_0, "TCL UNEXPECTED_RESULT_CODE 3"),
    ] {
        let profile = tcl_vm::test_support::profile(version.dialect_name());
        let mut vm =
            tcl_vm::test_support::native_vm_for_release(version, Box::new(std::io::sink()));
        vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
        let settle = settle_original_top_level;
        assert_eq!(
            settle(&mut vm, "break"),
            (
                Code::Error,
                "invoked \"break\" outside of a loop".to_owned()
            ),
        );
        assert_eq!(
            settle(&mut vm, "set errorCode"),
            (Code::Ok, unexpected.to_owned()),
            "{version:?}"
        );
        assert_eq!(
            settle(&mut vm, "continue"),
            (
                Code::Error,
                "invoked \"continue\" outside of a loop".to_owned()
            ),
        );
        assert_eq!(
            settle(&mut vm, "return -code 7 x"),
            (Code::Error, "command returned bad code: 7".to_owned()),
        );
        assert_eq!(settle(&mut vm, "return x"), (Code::Ok, "x".to_owned()));
        if version != TclVersion::V8_4 {
            assert_eq!(
                settle(&mut vm, "return -level 2 x"),
                (Code::Error, "command returned bad code: 2".to_owned()),
            );
        }
        assert_eq!(
            settle(&mut vm, "error boom {} {X Y}; set q 1"),
            (Code::Error, "boom".to_owned()),
        );
        assert_eq!(
            settle(&mut vm, "set errorCode"),
            (Code::Ok, "X Y".to_owned())
        );
    }
}
