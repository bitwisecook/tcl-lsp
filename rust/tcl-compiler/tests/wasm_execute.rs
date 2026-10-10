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

//! Execute emitted modules under wasmtime with a software ABI provider.
//!
//! The imported memory and descriptor signatures use the actual code-generation
//! ABI. The provider records evaluated source commands and writes controlled
//! completion codes/truth values; these controls establish structured dispatch
//! and ownership cleanup, not Native expression evaluation or object semantics.
//! Successful conditions exercise both branches. Guest failures and first Host
//! refusal must stop before any success truth value is used.
//!
//! `wasm_real_link.rs` independently links the actual Runtime and retains its
//! separate toolchain/admission requirements. This suite's stub outcome cannot
//! establish that Runtime or a Native provider executed.

use tcl_compiler::codegen::wasm::{
    GlobalInit, ValType, WasmCompileOptions, WasmFunction, WasmGlobal, WasmInstruction, WasmModule,
    WasmOp, compile_wasm,
};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

/// Is the `wasmtime` CLI present? (Mirrors the skip in `wasm_codegen.rs`.)
fn have_wasmtime() -> bool {
    std::process::Command::new("wasmtime")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// `i32.const 0` — the LEB128 of signed 0 is the single byte `0x00`.
fn i32_const_0() -> WasmInstruction {
    WasmInstruction::with_operands(WasmOp::I32Const, vec![0x00])
}

/// Build the **host stub**: a module that *defines and exports* `memory` plus
/// the `tcl_*` functions the emitted module imports, with trivial bodies.
/// `tcl_obj_new_string` returns a dummy `0` obj handle (the emitted module only
/// passes it to `tcl_eval_code`, never dereferences it); `tcl_eval_code` returns
/// `0` (the `ok` completion code, so nothing propagates). The combined Boolean
/// export writes an OK completion and false truth, so all loops terminate.
fn host_stub() -> WasmModule {
    let func =
        |name: &str, params: Vec<ValType>, results: Vec<ValType>, body: Vec<WasmInstruction>| {
            WasmFunction {
                name: name.to_string(),
                params,
                results,
                locals: Vec::new(),
                body,
                local_names: Vec::new(),
                exported: true,
                source_range: None,
                kind: "host".to_string(),
            }
        };
    let mut m = WasmModule::new();
    m.import_memory = false; // define + export our own memory (index 0)
    m.memory_pages = 1;
    m.functions = vec![
        func(
            "tcl_codegen_host_refusal_pending",
            Vec::new(),
            vec![ValType::I32],
            vec![i32_const_0()],
        ),
        func(
            "tcl_obj_new_string",
            vec![ValType::I32, ValType::I32],
            vec![ValType::I32],
            vec![i32_const_0()],
        ),
        func(
            "tcl_eval_code",
            vec![ValType::I32],
            vec![ValType::I32],
            vec![i32_const_0()],
        ),
        func(
            "tcl_obj_new_string_owned",
            vec![ValType::I32, ValType::I32],
            vec![ValType::I32],
            vec![i32_const_0()],
        ),
        func(
            "tcl_call_frame_alloc",
            vec![ValType::I32, ValType::I32],
            vec![ValType::I32],
            vec![WasmInstruction::with_operands(
                WasmOp::I32Const,
                vec![0x80, 0xc0, 0x03],
            )],
        ),
        func(
            "tcl_call_frame_free",
            vec![ValType::I32],
            vec![ValType::I32],
            vec![i32_const_0()],
        ),
        func(
            "tcl_obj_release",
            vec![ValType::I32],
            Vec::new(),
            Vec::new(),
        ),
        func(
            "tcl_completion_release",
            vec![ValType::I32],
            Vec::new(),
            Vec::new(),
        ),
        func(
            "tcl_codegen_expr_bool",
            vec![ValType::I32; 3],
            vec![ValType::I32],
            vec![
                WasmInstruction::with_operands(WasmOp::LocalGet, vec![1]),
                i32_const_0(),
                WasmInstruction::with_operands(WasmOp::I32Store, vec![2, 0]),
                WasmInstruction::with_operands(WasmOp::LocalGet, vec![2]),
                i32_const_0(),
                WasmInstruction::with_operands(WasmOp::I32Store, vec![2, 0]),
                i32_const_0(),
            ],
        ),
    ];
    m
}

/// Lower + emit the user module for `src`.
fn compile_user(src: &str) -> WasmModule {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(src, &registry, false);
    compile_wasm(
        &unit,
        &registry,
        WasmCompileOptions::hosted()
            .for_eval_only_test_host()
            .with_data_base(0),
    )
    .into_module()
}

/// Emit `src`'s `::top`, preload the host stub, and invoke it under wasmtime.
fn run(src: &str, tag: &str) -> std::process::Output {
    let tmp = std::env::temp_dir();
    let host = tmp.join(format!("tcl_e2e_host_{tag}.wasm"));
    let user = tmp.join(format!("tcl_e2e_user_{tag}.wasm"));
    std::fs::write(&host, host_stub().to_bytes()).expect("write host stub");
    std::fs::write(&user, compile_user(src).to_bytes()).expect("write user module");
    let out = std::process::Command::new("wasmtime")
        .arg("run")
        .arg("--preload")
        .arg(format!("tcl={}", host.display()))
        .arg("--invoke")
        .arg("::top")
        .arg(&user)
        .output()
        .expect("run wasmtime");
    let _ = std::fs::remove_file(&host);
    let _ = std::fs::remove_file(&user);
    out
}

/// Each emitted `::top` instantiates and runs to completion under wasmtime
/// (imports + memory wired via the preloaded host stub), without trapping.
#[test]
fn emitted_modules_run_under_wasmtime() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping end-to-end execution");
        return;
    }
    for (tag, src) in [
        ("linear", "set x 5\nputs $x\n"),
        ("if_else", "if {1} {puts a} else {puts b}\n"),
        ("if_noelse", "if {1} {puts a}\nputs after\n"),
        (
            "elseif",
            "if {$a} {puts a} elseif {$b} {puts b} else {puts c}\n",
        ),
        ("nested_if", "if {1} {if {2} {puts a}}\nputs done\n"),
        ("while_loop", "while {$i < 10} {puts $i}\n"),
        ("while_break", "while {1} {if {$done} {break}\nputs x}\n"),
        ("for_loop", "for {set i 0} {$i < 10} {incr i} {puts $i}\n"),
        (
            "nested_loop",
            "while {$a} {for {set i 0} {$i<3} {incr i} {if {$i} {break} else {continue}}}\n",
        ),
        ("return_mid", "if {$x} {return 1}\nputs after\n"),
        ("foreach_opaque", "foreach x {a b c} {puts $x}\n"),
        // A proc-defining module: `::top` runs (defining + calling `add` via
        // eval-fallback) and the separately-emitted `::add` function is valid.
        (
            "proc",
            "proc add {a b} {return [expr {$a + $b}]}\nadd 1 2\n",
        ),
    ] {
        let out = run(src, tag);
        assert!(
            out.status.success(),
            "`{tag}` did not run cleanly under wasmtime:\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }
}

/// A **WASI-writing host stub** (WAT): `tcl_obj_new_string` packs the
/// `(offset, len)` of the boxed string into one i32 (`ptr << 16 | len`), which
/// `tcl_eval_code` unpacks and writes the command text followed by a newline
/// to stdout via `fd_write`, then returns the fixed `eval_code` completion code
/// (`0` = `ok`, so the emitted dispatch falls through; a non-zero drives the
/// abrupt-completion paths — see [`emitted_completion_codes_propagate`]).
/// `tcl_codegen_expr_bool` writes an OK completion and the fixed `expr_result`
/// truth, so the test controls which branch the emitted control flow takes. (The scratch iovec at `0xF000` and the
/// newline iovec/byte at `0xF018` sit far above the emitted module's low-offset
/// data — no collision.)
fn wasi_recording_host(expr_result: u8, eval_code: u8) -> String {
    condition_recording_host(expr_result, eval_code, 0, false, false, false)
}

/// Record actual emitted control/cleanup calls, with a software completion
/// provider. The injected Guest codes exercise transport, not Native parsing.
fn condition_recording_host(
    expr_result: u8,
    eval_code: u8,
    condition_code: i32,
    host_refusal: bool,
    trace_cleanup: bool,
    only_inner: bool,
) -> String {
    let host_refusal = i32::from(host_refusal);
    let trace_cleanup = i32::from(trace_cleanup);
    let only_inner = i32::from(only_inner);
    format!(
        r#"(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (global $host (mut i32) (i32.const 0))
  (global $outer (mut i32) (i32.const 0))
  (func $line (param $ptr i32) (param $len i32)
    (i32.store (i32.const 0xF000) (local.get $ptr))
    (i32.store (i32.const 0xF004) (local.get $len))
    (drop (call $fd_write (i32.const 1) (i32.const 0xF000) (i32.const 1) (i32.const 0xF010)))
    (drop (call $fd_write (i32.const 1) (i32.const 0xF018) (i32.const 1) (i32.const 0xF010))))
  (func $box (param i32 i32) (result i32)
    local.get 0 i32.const 16 i32.shl local.get 1 i32.or)
  (export "tcl_obj_new_string" (func $box))
  (export "tcl_obj_new_string_owned" (func $box))
  (func (export "tcl_eval_code") (param i32) (result i32)
    (call $line (i32.shr_u (local.get 0) (i32.const 16))
      (i32.and (local.get 0) (i32.const 0xFFFF)))
    i32.const {eval_code})
  (func (export "tcl_codegen_host_refusal_pending") (result i32) global.get $host)
  (func (export "tcl_call_frame_alloc") (param i32 i32) (result i32) i32.const 0xE000)
  (func (export "tcl_call_frame_free") (param i32) (result i32)
    (if (i32.const {trace_cleanup}) (then (call $line (i32.const 0xF030) (i32.const 1))))
    i32.const 0)
  (func (export "tcl_obj_release") (param i32)
    (if (i32.const {trace_cleanup}) (then (call $line (i32.const 0xF031) (i32.const 1)))))
  (func (export "tcl_completion_release") (param i32)
    (if (i32.const {trace_cleanup}) (then (call $line (i32.const 0xF032) (i32.const 1)))))
  (func (export "tcl_codegen_expr_bool") (param $expr i32) (param $completion i32)
    (param $truth i32) (result i32) (local $head i32) (local $code i32) (local $value i32)
    (local.set $head (i32.load8_u offset=1 (i32.shr_u (local.get $expr) (i32.const 16))))
    (if (i32.const {host_refusal}) (then (global.set $host (i32.const 1)) (return (i32.const -6))))
    (if (i32.eq (i32.const {condition_code}) (i32.const -99)) (then (return (i32.const 1))))
    (local.set $code (i32.const {condition_code}))
    (local.set $value (i32.const {expr_result}))
    (if (i32.const {only_inner})
      (then
        (if (i32.eq (local.get $head) (i32.const 111))
          (then
            (local.set $code (i32.const 0))
            (local.set $value (i32.eqz (global.get $outer)))
            (global.set $outer (i32.add (global.get $outer) (i32.const 1)))))))
    (i32.store (local.get $completion) (local.get $code))
    (i32.store offset=4 (local.get $completion) (i32.const 0))
    (i32.store offset=8 (local.get $completion) (i32.const 0))
    (if (i32.eqz (local.get $code)) (then (i32.store (local.get $truth) (local.get $value))))
    i32.const 0)
  (data (i32.const 0xF018) "\20\f0\00\00\01\00\00\00\0a")
  (data (i32.const 0xF030) "FSC"))
"#
    )
}

/// Emit `src`'s `::top`, preload the WASI recording host, invoke it, and return
/// the captured stdout (the newline-terminated sequence of eval-fallback command
/// texts that actually executed).
fn run_capture(src: &str, expr_result: u8, tag: &str) -> String {
    run_capture_code(src, expr_result, 0, tag)
}

/// As [`run_capture`], but the recording host's `tcl_eval_code` returns the fixed
/// `eval_code` completion code for **every** leaf command, so the test can drive
/// the emitted abrupt-completion dispatch. Only codes that
/// terminate (`error`/`return`/`break`) are safe with a `true` guard — a fixed
/// `continue` (4) under a `true` condition would iterate forever.
fn run_capture_code(src: &str, expr_result: u8, eval_code: u8, tag: &str) -> String {
    run_capture_host(src, wasi_recording_host(expr_result, eval_code), tag)
}

fn run_capture_host(src: &str, host_source: String, tag: &str) -> String {
    let tmp = std::env::temp_dir();
    let host = tmp.join(format!("tcl_e2e_wasi_{tag}.wat"));
    let user = tmp.join(format!("tcl_e2e_wuser_{tag}.wasm"));
    std::fs::write(&host, host_source).expect("write host");
    std::fs::write(&user, compile_user(src).to_bytes()).expect("write user module");
    let out = std::process::Command::new("wasmtime")
        .arg("run")
        .arg("--preload")
        .arg(format!("tcl={}", host.display()))
        .arg("--invoke")
        .arg("::top")
        .arg(&user)
        .output()
        .expect("run wasmtime");
    let _ = std::fs::remove_file(&host);
    let _ = std::fs::remove_file(&user);
    assert!(
        out.status.success(),
        "`{tag}` trapped:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is utf-8")
}

/// The emitted control flow executes the **right** commands: stdout is the exact
/// sequence of eval-fallback texts for the branch/iteration the structure takes,
/// with `tcl_codegen_expr_bool` writing `0` (false) and `1` (true) to its truth
/// output to drive each side.
#[test]
fn emitted_control_flow_runs_the_right_commands() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping end-to-end execution");
        return;
    }

    // Conditions false (else arms taken, loops exit immediately).
    // Linear: both commands run, in order.
    assert_eq!(
        run_capture("set x 5\nputs $x\n", 0, "lin"),
        "set x 5\nputs $x\n"
    );
    // if/else → the else arm.
    assert_eq!(
        run_capture("if {1} {puts a} else {puts b}\n", 0, "ifF"),
        "puts b\n"
    );
    // if without else, condition false → only the trailing command.
    assert_eq!(
        run_capture("if {1} {puts a}\nputs after\n", 0, "ifNoF"),
        "puts after\n"
    );
    // while, condition false → body never runs.
    assert_eq!(run_capture("while {1} {puts body}\n", 0, "whF"), "");
    // for: the init command runs, then the (false) guard exits before the body.
    assert_eq!(
        run_capture("for {set i 0} {1} {incr i} {puts body}\n", 0, "forF"),
        "set i 0\n"
    );

    // Conditions true (then arms taken).
    // if/else → the then arm.
    assert_eq!(
        run_capture("if {1} {puts a} else {puts b}\n", 1, "ifT"),
        "puts a\n"
    );
    // if without else, condition true → the body then the trailing command.
    assert_eq!(
        run_capture("if {0} {puts first}\nputs after\n", 1, "ifNoT"),
        "puts first\nputs after\n"
    );
    // An opaque `foreach` is one whole-command eval regardless of the guard.
    assert_eq!(
        run_capture("foreach x {a b c} {puts $x}\n", 0, "feF"),
        "foreach x {a b c} {puts $x}\n"
    );
}

/// A leaf command's **completion code** is honoured, not swallowed:
/// an `error`/`return` unwinds the compiled function and a
/// `break` re-enters the enclosing loop's exit — so an abrupt code inside a
/// compiled `while` must not loop forever or run dead code. The recording
/// host makes `tcl_codegen_expr_bool` write `1` (guard true) and `tcl_eval_code` to the
/// code under test, so a *swallowed* code would iterate the `while {1}` forever;
/// the tests terminate precisely because the code is honoured.
#[test]
fn emitted_completion_codes_propagate() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping completion-code propagation");
        return;
    }

    // `error` (1) in a `while {1}` body: the body runs once, then the error
    // unwinds `::top` — the trailing `puts after` never runs (dead after the
    // abrupt completion). Without the fix this loops forever.
    assert_eq!(
        run_capture_code("while {1} {puts body}\nputs after\n", 1, 1, "errLoop"),
        "puts body\n"
    );

    // `return` (2) unwinds a linear script just the same: the first command
    // completes `return`, so the rest of the script is dead.
    assert_eq!(
        run_capture_code("puts one\nputs two\n", 0, 2, "retLin"),
        "puts one\n"
    );

    // `break` (3) in a `while {1}` body exits *the loop* (not the function): the
    // body runs once, then control falls through to `puts after` (which itself
    // then completes `break` outside any loop, unwinding). The trailing command
    // running is the observable difference from the `error`/`return` cases.
    assert_eq!(
        run_capture_code("while {1} {puts body}\nputs after\n", 1, 3, "brkLoop"),
        "puts body\nputs after\n"
    );
}

/// These are executed emitted-module controls with a software ABI provider.
/// They prove completion dispatch/cleanup, not Native expression equivalence.
#[test]
fn original_general_conditions_dispatch_guest_codes_before_truth() {
    // naming.numeric.original-primitive-boolean-vs-expression-truth
    // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; condition completion controls unexecuted");
        return;
    }
    for code in [1, 2, 3, 4, 7] {
        for (kind, source, prefix) in [
            (
                "if",
                "if {[probe]} {puts WRONG} else {puts WRONG_ELSE}\nputs AFTER\n",
                "",
            ),
            ("while", "while {[probe]} {puts WRONG}\nputs AFTER\n", ""),
            (
                "for",
                "for {puts INIT} {[probe]} {puts STEP} {puts WRONG}\nputs AFTER\n",
                "puts INIT\n",
            ),
        ] {
            let out = run_capture_host(
                source,
                condition_recording_host(1, 0, code, false, true, false),
                &format!("condition_{kind}_code{code}"),
            );
            assert_eq!(out, format!("{prefix}S\nC\nF\n"), "{kind} code{code}");
        }
    }
    for (truth, branch) in [(0, "puts ELSE"), (1, "puts THEN")] {
        let out = run_capture_host(
            "if {[probe]} {puts THEN} else {puts ELSE}\nputs AFTER\n",
            condition_recording_host(truth, 0, 0, false, true, false),
            &format!("condition_success{truth}"),
        );
        assert_eq!(out, format!("S\nC\nF\n{branch}\nputs AFTER\n"));
    }
}

#[test]
fn original_general_nested_condition_completions_reach_the_owning_loop() {
    // naming.numeric.original-primitive-boolean-vs-expression-truth
    // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; nested condition controls unexecuted");
        return;
    }
    for inner in [
        "if {[inner]} {puts WRONG} else {puts WRONG_ELSE}",
        "while {[inner]} {puts WRONG}",
        "for {} {[inner]} {puts WRONG_STEP} {puts WRONG}",
    ] {
        for code in [1, 2, 3, 4, 7] {
            let source =
                format!("while {{[outer]}} {{{inner}\nputs WRONG_AFTER_INNER}}\nputs AFTER\n");
            let out = run_capture_host(
                &source,
                condition_recording_host(1, 0, code, false, true, true),
                &format!(
                    "nested_condition_{}_code{code}",
                    if inner.starts_with("if") {
                        "if"
                    } else if inner.starts_with("while") {
                        "while"
                    } else {
                        "for"
                    }
                ),
            );
            let cleanup = "S\nC\nF\n";
            let expected = match code {
                3 => format!("{cleanup}{cleanup}puts AFTER\n"),
                4 => format!("{cleanup}{cleanup}{cleanup}puts AFTER\n"),
                _ => format!("{cleanup}{cleanup}"),
            };
            assert_eq!(out, expected, "{inner} code{code}");
        }
    }
}

#[test]
fn original_general_condition_host_refusal_cleans_before_any_guest_branch() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; condition first-Host control unexecuted");
        return;
    }
    for (kind, source) in [
        (
            "if",
            "if {[probe]} {puts WRONG} else {puts WRONG_ELSE}\nputs AFTER\n",
        ),
        ("while", "while {[probe]} {puts WRONG}\nputs AFTER\n"),
    ] {
        for (status, host_refusal) in [(3, true), (-99, false)] {
            let out = run_capture_host(
                source,
                condition_recording_host(1, 0, status, host_refusal, true, false),
                &format!("condition_refusal_{kind}_{status}"),
            );
            assert_eq!(
                out, "S\nC\nF\n",
                "Host or invalid transport bypasses Guest break/false dispatch"
            );
        }
    }
}

/// A host stub standing in for the linked runtime's table half: it exports a
/// **growable** function table and one function that calls back through it.
const TABLE_HOST_WAT: &str = r#"(module
  (memory (export "memory") 1)
  (table (export "__indirect_function_table") 1 funcref)
  (type $slot_t (func (result i32)))
  (func (export "tcl_call_slot") (param $slot i32) (result i32)
    (call_indirect (type $slot_t) (local.get $slot))))
"#;

/// A module in the shape the table-install codegen emits: import the
/// runtime's table, grow it, keep the base in a global, install a function of
/// its own, and have the host call it back through the table.
///
/// The installed function is deliberately **not** exported, so the declarative
/// element segment is the only thing making its `ref.func` legal — which is
/// what the segment is there to guarantee once a later change stops exporting
/// generated functions.
fn table_install_module() -> WasmModule {
    let mut m = WasmModule::new();
    let call_slot = m.add_import("tcl", "tcl_call_slot", &[ValType::I32], &[ValType::I32]);
    m.import_table = true;
    m.globals.push(WasmGlobal {
        name: "table_base".into(),
        mutable: true,
        init: GlobalInit::I32(-1),
    });
    let answer_index = u32::try_from(m.imports.len()).expect("import count fits u32");
    m.elem_declared.push(answer_index);
    m.functions.push(WasmFunction {
        name: "answer".into(),
        params: vec![],
        results: vec![ValType::I32],
        locals: vec![],
        body: vec![WasmInstruction::with_operands(WasmOp::I32Const, vec![42])],
        local_names: vec![],
        exported: false,
        source_range: None,
        kind: "proc".into(),
    });
    m.functions.push(WasmFunction {
        name: "::top".into(),
        params: vec![],
        results: vec![ValType::I32],
        locals: vec![],
        body: vec![
            // base = table.grow(null, 1); traps below if the host's table is
            // not growable, because table.set at -1 is out of bounds.
            WasmInstruction::ref_null_func(),
            WasmInstruction::with_operands(WasmOp::I32Const, vec![0x01]),
            WasmInstruction::table_grow(0),
            WasmInstruction::with_operands(WasmOp::GlobalSet, vec![0x00]),
            // table[base] = ref.func $answer
            WasmInstruction::with_operands(WasmOp::GlobalGet, vec![0x00]),
            WasmInstruction::ref_func(answer_index),
            WasmInstruction::table_set(0),
            // return host(base)
            WasmInstruction::with_operands(WasmOp::GlobalGet, vec![0x00]),
            WasmInstruction::with_operands(
                WasmOp::Call,
                vec![u8::try_from(call_slot).expect("import index fits a byte")],
            ),
        ],
        local_names: vec![],
        exported: true,
        source_range: None,
        kind: "top".into(),
    });
    m
}

/// The IR's table, global and element encodings are accepted by a real engine,
/// and the installed function is reachable through the shared table.
///
/// The unit tests in `ir.rs` pin the bytes; this proves the bytes are a module
/// wasmtime will validate, instantiate and run — the half a byte assertion
/// cannot reach.
#[test]
fn an_emitted_module_installs_a_function_into_the_hosts_table() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping the table-install execution");
        return;
    }
    let tmp = std::env::temp_dir();
    let host = tmp.join("tcl_e2e_host_table.wat");
    let user = tmp.join("tcl_e2e_user_table.wasm");
    std::fs::write(&host, TABLE_HOST_WAT).expect("write table host");
    std::fs::write(&user, table_install_module().to_bytes()).expect("write user module");
    let out = std::process::Command::new("wasmtime")
        .arg("run")
        .arg("--preload")
        .arg(format!("tcl={}", host.display()))
        .arg("--invoke")
        .arg("::top")
        .arg(&user)
        .output()
        .expect("run wasmtime");
    let _ = std::fs::remove_file(&host);
    let _ = std::fs::remove_file(&user);
    assert!(
        out.status.success(),
        "the table-installing module did not run:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("42"),
        "the host must reach the installed function through the table, got {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}
