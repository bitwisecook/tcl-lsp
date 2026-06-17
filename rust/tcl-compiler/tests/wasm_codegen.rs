//! Stage-1 WASM backend tests: the greenfield eval-fallback emitter produces a
//! structurally valid module with the right shape (imports, data section,
//! eval-fallback calls, structured `if`). Execution against a runtime is a later
//! stage (the Rust runtime's wasm32 ABI is still the T1.1 stub), so these assert
//! the emitted structure; `make wasm-emitter-validate` runs `wasmtime validate`
//! on the bytes for full structural validity.

use tcl_compiler::cfg_builder::build_cfg;
use tcl_compiler::codegen::wasm::{wasm_codegen_module, WasmModule};
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

fn compile_wasm(source: &str) -> WasmModule {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(source, &registry);
    let cfg = build_cfg(&ir, false);
    wasm_codegen_module(&cfg, source)
}

/// A linear top-level script: each command is eval-fallback'd in order, results
/// discarded; the command texts live in the data section.
#[test]
fn linear_top_level_eval_fallback() {
    let mut m = compile_wasm("set x 5\nputs $x\n");
    let wat = m.to_wat();
    // The eval-fallback import boundary is declared.
    assert!(wat.contains(r#""tcl_obj_new_string""#), "{wat}");
    assert!(wat.contains(r#""tcl_eval""#), "{wat}");
    assert!(wat.contains(r#""memory""#), "{wat}");
    // Both commands' source text is interned in the data section.
    assert!(wat.contains(r#""set x 5""#), "{wat}");
    assert!(wat.contains(r#""puts $x""#), "{wat}");
    // Exported top-level entry.
    assert!(wat.contains(r#"(export "::top")"#), "{wat}");
    // Two eval-fallback sequences (box → eval → release) ⇒ two `call 1` (eval).
    assert_eq!(m.to_wat().matches("\n        call 1").count(), 2, "{wat}");
    // Valid module header.
    let bytes = m.to_bytes();
    assert_eq!(&bytes[0..4], b"\0asm", "wasm magic");
}

/// An `if`/`else` is recovered as **structured** WASM control flow.
#[test]
fn if_else_is_structured() {
    let mut m = compile_wasm("if {1} {puts a} else {puts b}\n");
    let wat = m.to_wat();
    assert!(wat.contains("\n        if"), "expected structured if:\n{wat}");
    assert!(wat.contains("else"), "expected else arm:\n{wat}");
    assert!(wat.contains("end"), "expected end:\n{wat}");
    // Both arms' bodies are present in the data section.
    assert!(wat.contains(r#""puts a""#), "{wat}");
    assert!(wat.contains(r#""puts b""#), "{wat}");
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// Every emitted module is **structurally valid WASM** — confirmed by
/// `wasmtime compile` (which fully validates before native compilation). Skips
/// gracefully where the `wasmtime` CLI isn't available, mirroring the Python
/// oracle skip in `differential_codegen.rs`.
#[test]
fn wasmtime_validates_emitted_modules() {
    let have_wasmtime = std::process::Command::new("wasmtime")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !have_wasmtime {
        eprintln!("wasmtime CLI unavailable; skipping structural validation");
        return;
    }
    let tmp = std::env::temp_dir();
    for (name, src) in [
        ("linear", "set x 5\nputs $x\n"),
        ("if-else", "if {1} {puts a} else {puts b}\n"),
        ("if-noelse", "if {1} {puts a}\nputs after\n"),
        ("nested-if", "if {1} {if {2} {puts a}}\nputs done\n"),
    ] {
        let mut m = compile_wasm(src);
        let wasm = tmp.join(format!("tcl_wasm_stage1_{name}.wasm"));
        let cwasm = tmp.join(format!("tcl_wasm_stage1_{name}.cwasm"));
        std::fs::write(&wasm, m.to_bytes()).expect("write wasm");
        let status = std::process::Command::new("wasmtime")
            .arg("compile")
            .arg(&wasm)
            .arg("-o")
            .arg(&cwasm)
            .output()
            .expect("run wasmtime");
        assert!(
            status.status.success(),
            "{name}.wasm failed wasmtime validation:\n{}",
            String::from_utf8_lossy(&status.stderr)
        );
        let _ = std::fs::remove_file(&wasm);
        let _ = std::fs::remove_file(&cwasm);
    }
}
