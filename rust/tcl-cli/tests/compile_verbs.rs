//! Integration tests for the `dis` and `compwasm` verbs.
//!
//! These assert structural properties of the output rather than byte-for-byte
//! Python parity: the Rust bytecode codegen and the greenfield WASM emitter are
//! still reaching feature parity (notably `expr`/command-substitution inlining),
//! so the verbs are asserted to faithfully render whatever the current Rust
//! pipeline produces — a valid disassembly and a structurally valid WASM module.

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

#[test]
fn compwasm_writes_to_stdout_by_default() {
    let out = run_tcl(&["compwasm", "--source", "set x 1\n"]);
    assert_eq!(&out[0..4], b"\0asm", "stdout payload is not a wasm module");
}
