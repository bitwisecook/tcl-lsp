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

//! **End-to-end execution** — a real emitted module *runs* under wasmtime.
//!
//! `wasm_codegen.rs` proves the emitted bytes are structurally valid
//! (`wasmtime compile`). This goes one step further: it **instantiates and
//! invokes** the emitted `::top` on the wasmtime engine, proving the module
//! actually runs — imports resolve against a real provider, the imported memory
//! and data segments wire up, and the structured control flow + eval-fallback
//! `call`s execute to completion without trapping.
//!
//! The emitted module imports three `tcl_*` host functions and its linear memory
//! from module `"tcl"` (the runtime's codegen ABI — `runtime/rust/codegen_abi.rs`
//! now exports exactly this surface). Rather than link the whole runtime (the
//! shared-memory dynamic-linking + `__memory_base` relocation that the
//! whole-program artifact needs — a later increment), we satisfy those imports
//! with a tiny **host stub** generated from the compiler's own WASM IR and
//! `--preload`ed into wasmtime. The stub's `tcl_eval_code` returns `0` (`ok`, so
//! the emitted completion-code dispatch always falls through) and `tcl_expr_bool`
//! returns `0`, so every condition is false: each `if` takes its else, each loop
//! exits immediately, and the invoked `::top` terminates without a real interp.
//!
//! Two tiers, both over the wasmtime CLI (no embedder crate):
//!
//! - **Tier 0** ([`emitted_modules_run_under_wasmtime`]) — *runnability*: the
//!   module instantiates and `::top` runs to completion without trapping, across
//!   the full snippet set (incl. nested loops, break/continue, mid-return). The
//!   host stub is generated from the compiler's own WASM IR.
//! - **Tier 1** ([`emitted_control_flow_runs_the_right_commands`]) —
//!   *correctness*: a WASI-writing host stub prints each eval-fallback command's
//!   text to stdout, so the test asserts the **exact executed command sequence**
//!   — proving the emitted control flow takes the right branch / iterates as
//!   structured. Driving `tcl_expr_bool` to `0` vs `1` exercises both the false
//!   (else / loop-exit) and true (then) wiring.
//!
//! Backing the host with a real `Interp` to observe actual Tcl *side effects*
//! (not just the command texts) is the next tier and needs the wasmtime embedder
//! crate; running against the real runtime wasm needs the shared-memory dynamic
//! link (`__memory_base` relocation) — both later increments.

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
/// the three `tcl_*` functions the emitted module imports, with trivial bodies.
/// `tcl_obj_new_string` returns a dummy `0` obj handle (the emitted module only
/// passes it to `tcl_eval_code`, never dereferences it); `tcl_eval_code` returns
/// `0` (the `ok` completion code, so nothing propagates) and `tcl_expr_bool`
/// returns `0` (false) so all control flow terminates.
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
            "tcl_expr_bool",
            vec![ValType::I32],
            vec![ValType::I32],
            vec![i32_const_0()],
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
/// `tcl_eval_code` unpacks and writes — the command text followed by a newline —
/// to stdout via `fd_write`, then returns the fixed `eval_code` completion code
/// (`0` = `ok`, so the emitted dispatch falls through; a non-zero drives the
/// abrupt-completion paths — see [`emitted_completion_codes_propagate`]).
/// `tcl_expr_bool` returns the fixed `expr_result`, so the test controls which
/// branch the emitted control flow takes. (The scratch iovec at `0xF000` and the
/// newline iovec/byte at `0xF018` sit far above the emitted module's low-offset
/// data — no collision.)
fn wasi_recording_host(expr_result: u8, eval_code: u8) -> String {
    format!(
        r#"(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "tcl_obj_new_string") (param i32 i32) (result i32)
    local.get 0 i32.const 16 i32.shl local.get 1 i32.or)
  (func (export "tcl_eval_code") (param i32) (result i32)
    (i32.store (i32.const 0xF000) (i32.shr_u (local.get 0) (i32.const 16)))
    (i32.store (i32.const 0xF004) (i32.and (local.get 0) (i32.const 0xFFFF)))
    (drop (call $fd_write (i32.const 1) (i32.const 0xF000) (i32.const 1) (i32.const 0xF010)))
    (drop (call $fd_write (i32.const 1) (i32.const 0xF018) (i32.const 1) (i32.const 0xF010)))
    i32.const {eval_code})
  (func (export "tcl_expr_bool") (param i32) (result i32) i32.const {expr_result})
  (data (i32.const 0xF018) "\20\f0\00\00\01\00\00\00\0a"))
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
    let tmp = std::env::temp_dir();
    let host = tmp.join(format!("tcl_e2e_wasi_{tag}.wat"));
    let user = tmp.join(format!("tcl_e2e_wuser_{tag}.wasm"));
    std::fs::write(&host, wasi_recording_host(expr_result, eval_code)).expect("write host");
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
/// with `tcl_expr_bool` forced to `0` (false) and `1` (true) to drive each side.
#[test]
fn emitted_control_flow_runs_the_right_commands() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping end-to-end execution");
        return;
    }

    // ----- conditions false (else arms taken, loops exit immediately) -----
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

    // ----- conditions true (then arms taken) -----
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
/// compiled `while` no longer loops forever or runs dead code. The recording
/// host forces `tcl_expr_bool` to `1` (guard true) and `tcl_eval_code` to the
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

/// A host stub standing in for the linked runtime's table half: it exports a
/// **growable** function table and one function that calls back through it.
const TABLE_HOST_WAT: &str = r#"(module
  (memory (export "memory") 1)
  (table (export "__indirect_function_table") 1 funcref)
  (type $slot_t (func (result i32)))
  (func (export "tcl_call_slot") (param $slot i32) (result i32)
    (call_indirect (type $slot_t) (local.get $slot))))
"#;

/// A module in the shape issue #1774 emits: import the runtime's table, grow
/// it, keep the base in a global, install a function of its own, and have the
/// host call it back through the table.
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
