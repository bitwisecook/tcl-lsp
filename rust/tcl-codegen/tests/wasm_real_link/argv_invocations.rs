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

//! The general tier's compiled prebuilt-argv path: side effects, intact compound words and balanced allocations.

use super::*;

/// Run `::top` three times in one owned interpreter, then prove complete
/// allocation conservation after that interpreter is destroyed. Persistent
/// error globals may legitimately appear between calls while it remains live.
///
/// The live residual must exceed its pre-creation value, so a counter surface
/// that always reads zero cannot pass. Every call must leave zero transient
/// frames. The second and third live object/buffer counts must match exactly;
/// teardown must restore each original raw count and residual and record
/// no double free. The current pointer is cleared before its owner is deleted.
fn leak_bootstrap_wat() -> String {
    r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "tcl_runtime_create_interp" (func $create (result i32)))
  (import "tcl" "tcl_runtime_set_current_interp" (func $setcur (param i32)))
  (import "tcl" "tcl_runtime_delete_interp" (func $delete (param i32)))
  (import "tcl" "tcl_codegen_call_frame_outstanding" (func $frames (result i32)))
  (import "tcl" "tcl_test_finalize" (func $residual (result i64)))
  (import "tcl" "tcl_test_live_obj_count" (func $objects (result i64)))
  (import "tcl" "tcl_test_live_buf_count" (func $buffers (result i64)))
  (import "tcl" "tcl_test_double_free_count" (func $doubles (result i64)))
  (import "tcl" "tcl_test_observe_heap_roots" (func $heap_roots))
  (import "user" "::top" (func $top))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $interp i32) (local $before i64)
    (local $before_objects i64) (local $before_buffers i64)
    (local $second_objects i64) (local $second_buffers i64)
    (local.set $before (call $residual))
    (local.set $before_objects (call $objects))
    (local.set $before_buffers (call $buffers))
    (local.set $interp (call $create))
    (call $setcur (local.get $interp))
    (call $top)
    (if (i32.ne (call $frames) (i32.const 0)) (then unreachable))
    (if (i64.le_s (call $residual) (local.get $before)) (then unreachable))
    (call $top)
    (if (i32.ne (call $frames) (i32.const 0)) (then unreachable))
    (local.set $second_objects (call $objects))
    (local.set $second_buffers (call $buffers))
    (call $top)
    (if (i32.ne (call $frames) (i32.const 0)) (then unreachable))
    (if (i64.ne (call $objects) (local.get $second_objects)) (then unreachable))
    (if (i64.ne (call $buffers) (local.get $second_buffers)) (then unreachable))
    (call $setcur (i32.const 0))
    (call $delete (local.get $interp))
    (if (i64.ne (call $residual) (local.get $before)) (then unreachable))
    (if (i64.ne (call $objects) (local.get $before_objects)) (then unreachable))
    (if (i64.ne (call $buffers) (local.get $before_buffers)) (then unreachable))
    (if (i64.ne (call $doubles) (i64.const 0)) (then unreachable))
    (if (i32.ne (call $frames) (i32.const 0)) (then unreachable)))
  ;; Diagnostic-only entry: a fresh instance reports original counter values.
  ;; This separate entry does not replace the owned-teardown _start gate.
  (func (export "diagnostic_counts") (result i64 i64 i64 i32 i64)
    (local $interp i32) (local $first i64) (local $second i64)
    (local.set $interp (call $create))
    (call $setcur (local.get $interp))
    (call $top)
    (call $heap_roots)
    (local.set $first (call $residual))
    (call $top)
    (call $heap_roots)
    (local.set $second (call $residual))
    (call $top)
    (call $heap_roots)
    (local.get $first) (local.get $second) (call $residual)
    (call $frames) (call $doubles))
)"#
    .to_owned()
}

/// The general tier's compiled prebuilt-argv path, running in the **real**
/// interpreter: each case's words are evaluated by the emitted module and
/// dispatched through the runtime's ordinary command resolution, and the side
/// effect is read back through the same interpreter afterwards.
#[test]
pub(super) fn compiled_argv_invocations_run_against_the_real_runtime() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };

    // (program run by `::top`, the query the bootstrap then evaluates, expected).
    let cases = [
        // Literal words only: a compiled argv reaches the real dispatcher.
        (
            "lappend out alpha\nlappend out beta\n",
            "set out",
            "alpha beta",
        ),
        // A braced word keeps its unsubstituted value; the runtime's `catch`
        // evaluates it, so a contained error stays contained.
        (
            "catch {error boom} msg\nlappend out $msg\n",
            "set out",
            "boom",
        ),
        // A `$var` scalar read feeding a `[nested command]` word.
        (
            "set greeting world\nlappend log [string toupper $greeting]\n",
            "set log",
            "WORLD",
        ),
        // A quoted compound word alongside a nested invocation.
        (
            "set name world\nlappend out \"hi $name\" [string length $name]\n",
            "set out",
            "{hi world} 5",
        ),
        // A `$arr(key)` element read, with the array built by a compiled argv.
        (
            "array set cfg {mode fast}\nlappend out $cfg(mode)\n",
            "set out",
            "fast",
        ),
        // A braced element *target*: `set {a(b)} 5` writes the element, as
        // tclsh does, not a scalar literally named `a(b)`. The direct
        // assignment gate reads that shape through `cell_place`, so this pins
        // the answer it must keep giving.
        (
            "set {cfg(mode)} fast\nlappend out $cfg(mode) [array exists cfg]\n",
            "set out",
            "fast 1",
        ),
        // Runtime dispatch is genuinely runtime dispatch: a renamed command
        // resolves to its new name, which no compile-time binding could know.
        (
            "proc original {x} {return got:$x}\nrename original renamed\nlappend out [renamed value]\n",
            "set out",
            "got:value",
        ),
        // Abrupt completion mid-script: the second statement errors, so the
        // third never runs — proving the compiled path propagates the code
        // after it has released its words and freed its frame.
        (
            "lappend out first\nstring index\nlappend out second\n",
            "set out",
            "first",
        ),
    ];
    for (index, (program, query, expected)) in cases.into_iter().enumerate() {
        assert_eq!(
            run_real_link_argv(&runtime, &format!("argv{index}"), program, query),
            expected,
            "program {program:?}, query {query:?}"
        );
    }
}

/// The retired analysis-tier `puts` fast path's exact failure mode, against
/// the real runtime.
///
/// The retired analysis-tier `puts` fast path took a single-argument `puts`
/// and re-read the argument's *compatibility text* to find a variable name.
/// For `puts "$p $q"` that text is `${p} ${q}`, whose outer-brace strip yields
/// the name `p} ${q` — a variable that does not exist, so the runtime wrote
/// nothing and the following `puts end` was swallowed with it. Nothing else in
/// the suite shows the symptom the issue reported, because it is a *missing*
/// line rather than a wrong value: the first `puts` proves the compound word
/// survives, and `end` proves the statement after it still runs.
#[test]
fn analysed_compound_puts_word_reaches_the_runtime_intact() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    assert_eq!(
        run_real_link_argv(
            &runtime,
            "compound-puts",
            "set p 1\nset q 2\nputs \"$p $q\"\nputs end\n",
            "set p",
        ),
        "1 2\nend\n1",
    );
}

/// The compiled path releases its allocations when the owned interpreter is
/// destroyed after three calls. Each call leaves zero transient frames, and
/// teardown restores the raw residual without a double free — including for
/// a statement that leaves through the abrupt-completion branch.
#[test]
pub(super) fn compiled_argv_balances_allocations_in_the_real_runtime() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };

    for (tag, program) in [
        // Every word shape, all side-effect free so repeated runs are neutral.
        (
            "words",
            "set greeting hello\nset cfg(mode) fast\nstring toupper $greeting\n\
             string length [string trim {  padded  }]\nstring equal \"hi $greeting\" hi\n\
             lindex {a b c} 1\n",
        ),
        // A missing variable aborts part-way through argv construction: the
        // words already built must still be released.
        (
            "abort-mid-word",
            "string toupper [string tolower $definitely_missing_variable]\nstring length abc\n",
        ),
        // The invocation itself completes abruptly inside a loop, so the
        // completion dispatch branches out after the cleanup runs.
        (
            "abort-in-loop",
            "set go 1\nwhile {$go} {string index\nlappend never x}\nstring length abc\n",
        ),
    ] {
        let user = scratch(&format!("tcl_real_leak_user_{tag}.wasm"));
        let boot = scratch(&format!("tcl_real_leak_boot_{tag}.wat"));
        // The frame-alloc requirement is what keeps this honest: under source
        // evaluation no transient frame is ever allocated, so the outstanding
        // count the bootstrap checks would balance at zero and the case would
        // pass while proving nothing about the compiled path's ownership.
        write_linked(
            &runtime,
            &user,
            &compile_argv_analysed(
                program,
                &["tcl_invoke_argv", "tcl_codegen_call_frame_alloc"],
            ),
            program,
        );
        std::fs::write(&boot, leak_bootstrap_wat()).expect("write bootstrap");
        let out = Command::new("wasmtime")
            .arg("run")
            .arg("--preload")
            .arg(format!("tcl={}", runtime.display()))
            .arg("--preload")
            .arg(format!("user={}", user.display()))
            .arg(&boot)
            .output()
            .expect("run wasmtime");
        if !out.status.success() {
            // Preserve the failed original process and its assertion. This separate
            // fresh instance only reports first/second/third residual, frames and doubles.
            let diagnostic = Command::new("wasmtime")
                .arg("run")
                .arg("--invoke")
                .arg("diagnostic_counts")
                .arg("--preload")
                .arg(format!("tcl={}", runtime.display()))
                .arg("--preload")
                .arg(format!("user={}", user.display()))
                .arg(&boot)
                .output();
            eprintln!("WASM_LEAK_COUNTER_WITNESS tag={tag} result={diagnostic:?}");
        }
        let _ = std::fs::remove_file(&user);
        let _ = std::fs::remove_file(&boot);
        assert!(
            out.status.success(),
            "`{tag}` did not balance its allocations:\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }
}
