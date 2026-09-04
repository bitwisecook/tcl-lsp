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

//! Integration tests for the `dis` and `compwasm` verbs.
//!
//! These assert structural properties of the output rather than byte-for-byte
//! golden output: the bytecode codegen and the greenfield WASM emitter are
//! still maturing (notably `expr`/command-substitution inlining),
//! so the verbs are asserted to faithfully render whatever the current
//! pipeline produces — a valid disassembly and a structurally valid WASM module.
//!
//! The `compwasm` plumbing tests pin the one analysis-aware per-script WASM
//! pipeline, including output-path selection and WAT rendering.

use std::process::Command;

/// Run the built `tcl` binary with `args` and inline source, returning the
/// captured stdout bytes. Asserts the process succeeded.
fn run_tcl(args: &[&str]) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(args)
        .output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "tcl {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn dis_renders_bytecode_for_inline_source() {
    let out = run_tcl(&["dis", "--source", "set x 5\nputs $x\n"]);
    let text = String::from_utf8(out).expect("utf-8");
    // Module header, the literal pool the source populates, and the terminator.
    assert!(text.contains("ByteCode ::top"), "{text}");
    assert!(text.contains("Literals:"), "{text}");
    assert!(text.contains("\"puts\""), "{text}");
    assert!(text.contains("done"), "{text}");
}

#[test]
fn dis_optimise_folds_constants() {
    // Without --optimise the expr substitution is preserved; with it, the
    // optimiser const-folds `1 + 2` to `3` in the rewritten source.
    let out = run_tcl(&["dis", "--optimise", "--source", "set x [expr {1 + 2}]\n"]);
    let text = String::from_utf8(out).expect("utf-8");
    assert!(
        text.contains("\"3\""),
        "expected folded literal 3 in:\n{text}"
    );
}

/// Issue #1048 review follow-up: the compile verbs thread the resolved dialect
/// into lowering, not just registry selection. An auto-detected iRule's
/// word-operator condition must reach codegen as a real comparison node and
/// disassemble to its dedicated opcode — dialect-blind lowering left it an
/// opaque raw expression on the generic runtime-`expr` path.
#[test]
fn dis_lowers_word_operators_for_the_detected_dialect() {
    let input =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wordOperator.irule");
    let detected = String::from_utf8(run_tcl(&["dis", input.to_str().unwrap()])).expect("utf-8");
    assert!(
        detected.contains("iruleContains"),
        "expected the contains word-operator opcode in:\n{detected}"
    );
    let explicit = String::from_utf8(run_tcl(&[
        "dis",
        "--dialect",
        "f5-irules",
        input.to_str().unwrap(),
    ]))
    .expect("utf-8");
    assert_eq!(
        detected, explicit,
        "detection must disassemble exactly as --dialect f5-irules does"
    );
}

/// The control for [`dis_lowers_word_operators_for_the_detected_dialect`]: the
/// same condition in plain Tcl source has no word operators, so no iRules
/// opcode may appear.
#[test]
fn dis_keeps_plain_tcl_free_of_irules_opcodes() {
    let out = run_tcl(&[
        "dis",
        "--source",
        "set x \"abcdef\"\nif {$x eq \"cd\"} { puts hit }\n",
    ]);
    let text = String::from_utf8(out).expect("utf-8");
    assert!(
        !text.contains("irule"),
        "plain Tcl must not emit iRules opcodes:\n{text}"
    );
}

#[test]
fn compwasm_emits_valid_module_header() {
    let dir = std::env::temp_dir().join("tcl_compwasm_test");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let wasm_path = dir.join("out.wasm");
    let wat_path = dir.join("out.wat");

    let out = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args([
            "compwasm",
            "--source",
            "set x 5\nputs $x\n",
            "-o",
            wasm_path.to_str().unwrap(),
            "--wat-output",
            wat_path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn tcl");
    assert!(
        out.status.success(),
        "compwasm failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let bytes = std::fs::read(&wasm_path).expect("read wasm");
    // WASM magic + version (`\0asm` 0x01000000).
    assert_eq!(&bytes[0..4], b"\0asm", "missing wasm magic");
    assert_eq!(&bytes[4..8], &[1, 0, 0, 0], "unexpected wasm version");

    let wat = std::fs::read_to_string(&wat_path).expect("read wat");
    assert!(wat.starts_with("(module"), "{wat}");
    assert!(wat.contains("tcl_obj_new_string"), "{wat}");

    // Keep the test hermetic.
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--codegen-passes` is what turns the semantic/AOT passes on; without it
/// `compwasm` emits the generic lowering.
///
/// Asserting only that the two modules *differ* would pass on a flag that
/// merely perturbed the output, so this checks what actually changes. The
/// generic lowering for a hot integer loop is a source-eval fallback — it
/// pushes the command text and calls `tcl_eval_code`, with no arithmetic of
/// its own. The native tier replaces that with real instructions: the `<`
/// and the `+` become `i64` operations and not one `tcl_eval_code` call
/// survives.
///
/// The pass set itself is execution-verified elsewhere:
/// `tcl-compiler`'s `wasm_tiers::native_plan_samples_reproduce_the_tclsh_oracles`
/// links the same `native_tier()` options against the real runtime, runs
/// every `samples/wasm` script under wasmtime, and diffs stdout against the
/// committed `tclsh9.0` oracles.
#[test]
fn compwasm_codegen_passes_natively_lower_a_hot_loop() {
    let dir = std::env::temp_dir().join("tcl_compwasm_passes_test");
    std::fs::create_dir_all(&dir).expect("mkdir");
    // An integer loop: a `<` guard and a `+` the representation lattice can
    // keep native. Nothing here needs a string.
    let source = "proc sum {n} {\n\
                  set total 0\n\
                  for {set i 0} {$i < $n} {incr i} {\n\
                  set total [expr {$total + $i}]\n\
                  }\n\
                  return $total\n\
                  }\n\
                  puts [sum 10]\n";

    let emit = |name: &str, passes: Option<&str>| {
        let wasm_path = dir.join(format!("{name}.wasm"));
        let wat_path = dir.join(format!("{name}.wat"));
        let mut args = vec![
            "compwasm".to_owned(),
            "--source".to_owned(),
            source.to_owned(),
            "-o".to_owned(),
            wasm_path.to_str().unwrap().to_owned(),
            "--wat-output".to_owned(),
            wat_path.to_str().unwrap().to_owned(),
        ];
        if let Some(passes) = passes {
            args.push("--codegen-passes".to_owned());
            args.push(passes.to_owned());
        }
        let out = Command::new(env!("CARGO_BIN_EXE_tcl"))
            .args(&args)
            .output()
            .expect("spawn tcl");
        assert!(
            out.status.success(),
            "compwasm {passes:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let bytes = std::fs::read(&wasm_path).expect("read wasm");
        assert_eq!(&bytes[0..4], b"\0asm", "{name}: missing wasm magic");
        std::fs::read_to_string(&wat_path).expect("read wat")
    };

    let generic = emit("generic", None);
    let native = emit("native", Some("native-tier"));

    // The generic lowering hands the source back to the interpreter.
    assert!(
        generic.contains("tcl_eval_code"),
        "the unflagged lowering was expected to import the source-eval \
         fallback:\n{generic}"
    );
    assert!(
        !generic.contains("i64.add"),
        "the unflagged lowering must not emit native arithmetic:\n{generic}"
    );

    // The native tier compiles it instead.
    let native_ops = ["i64.add", "i64.lt_s"];
    for op in native_ops {
        assert!(
            native.contains(op),
            "--codegen-passes native-tier emitted no `{op}`:\n{native}"
        );
    }
    assert!(
        !native
            .lines()
            .any(|line| line.trim_start().starts_with("call ")
                && eval_code_index(&native).is_some_and(|i| line.trim() == format!("call {i}"))),
        "a source-eval call survived the native tier:\n{native}"
    );

    // A misspelled pass fails loudly and names the offender, rather than
    // quietly producing an unoptimised module.
    let bad = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args([
            "compwasm",
            "--source",
            source,
            "--codegen-passes",
            "no-such-pass",
            "-o",
            dir.join("bad.wasm").to_str().unwrap(),
        ])
        .output()
        .expect("spawn tcl");
    assert!(!bad.status.success());
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(stderr.contains("no-such-pass"), "{stderr}");
    assert!(stderr.contains("native-tier"), "{stderr}");

    let _ = std::fs::remove_dir_all(&dir);
}

/// The import index `tcl_eval_code` was given, so a `call N` can be read as a
/// source-eval fallback. Imports are numbered in declaration order.
fn eval_code_index(wat: &str) -> Option<usize> {
    wat.lines()
        .filter(|line| line.trim_start().starts_with("(import "))
        .position(|line| line.contains("\"tcl_eval_code\""))
}

/// `--codegen-passes` is rejected with `explore --serve`.
///
/// The serve path never reaches the compile the flag would configure, so
/// accepting it would start the server having silently ignored the
/// selection — including an unrecognised pass name, which would otherwise
/// have been an error. The served GUI carries its own per-pass toggles.
#[test]
fn explore_rejects_codegen_passes_with_serve() {
    for passes in ["native-tier", "no-such-pass"] {
        let out = Command::new(env!("CARGO_BIN_EXE_tcl"))
            .args([
                "explore",
                "--serve",
                "--codegen-passes",
                passes,
                "--source",
                "set x 1",
            ])
            .output()
            .expect("spawn tcl");
        assert!(!out.status.success(), "{passes}: --serve accepted the flag");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("--codegen-passes"), "{passes}: {stderr}");
        assert!(stderr.contains("--serve"), "{passes}: {stderr}");
    }
    // Without `--serve` the flag still works.
    let out = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args([
            "explore",
            "--json",
            "--codegen-passes",
            "native-tier",
            "--source",
            "set a 1\nincr a\n",
        ])
        .output()
        .expect("spawn tcl");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("explorer contract JSON");
    let enabled: Vec<&str> = value["semanticOptimisations"]["passes"]
        .as_array()
        .expect("pass rows")
        .iter()
        .filter(|row| row["enabled"] == true)
        .map(|row| row["id"].as_str().expect("a pass id"))
        .collect();
    assert_eq!(
        enabled,
        vec![
            "native-lowering",
            "representation-inference",
            "trace-barrier-elision",
            "cell-demotion"
        ]
    );
}

#[test]
fn compwasm_rejects_removed_backend_selection() {
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["compwasm", "--backend", "vm", "--source", "set x 1"])
        .output()
        .expect("spawn tcl");
    assert!(
        !output.status.success(),
        "the removed backend selector must not remain accepted"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unexpected argument '--backend'"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn compwasm_literal_uses_canonical_semantic_argv_plan() {
    let dir = std::env::temp_dir().join("tcl_compwasm_canonical_semantic");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let wasm_path = dir.join("out.wasm");
    let wat_path = dir.join("out.wat");

    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args([
            "compwasm",
            "--source",
            "string length hello",
            "--dialect",
            "tcl8.6",
            "-o",
            wasm_path.to_str().expect("utf-8 output path"),
            "--wat-output",
            wat_path.to_str().expect("utf-8 WAT path"),
        ])
        .output()
        .expect("spawn tcl");
    assert!(
        output.status.success(),
        "canonical compwasm failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(&wasm_path).expect("read wasm");
    assert_eq!(&bytes[..4], b"\0asm", "missing WASM header");
    let wat = std::fs::read_to_string(&wat_path).expect("read WAT");
    assert!(wat.contains("tcl_invoke_argv"), "{wat}");
    assert!(!wat.contains("tcl_eval"), "{wat}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn compwasm_defaults_to_out_wasm_file() {
    // With no `-o`, compwasm must write `out.wasm` in the cwd
    // rather than dumping raw bytes to the terminal. Run in a scratch
    // dir so the artifact lands somewhere hermetic.
    let dir = std::env::temp_dir().join("tcl_compwasm_default");
    std::fs::create_dir_all(&dir).expect("mkdir");

    let out = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .current_dir(&dir)
        .args(["compwasm", "--source", "set x 1\n"])
        .output()
        .expect("spawn tcl");
    assert!(
        out.status.success(),
        "compwasm failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The default does not write bytes to stdout.
    assert!(out.stdout.is_empty(), "default run wrote to stdout");

    let bytes = std::fs::read(dir.join("out.wasm")).expect("out.wasm not written");
    assert_eq!(&bytes[0..4], b"\0asm", "out.wasm is not a wasm module");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn compwasm_dash_o_writes_stdout() {
    // An explicit `-o -` still selects stdout.
    let out = run_tcl(&["compwasm", "--source", "set x 1\n", "-o", "-"]);
    assert_eq!(&out[0..4], b"\0asm", "stdout payload is not a wasm module");
}
