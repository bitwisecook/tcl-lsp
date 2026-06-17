//! WASM backend tests: the greenfield eval-fallback emitter produces a
//! structurally valid module with the right shape (imports, data section,
//! eval-fallback calls, structured `if` / loops). Execution against a runtime is
//! a later stage (the Rust runtime's wasm32 ABI is still the T1.1 stub), so these
//! assert the emitted structure; `wasmtime compile` (run below where the CLI is
//! present) validates the bytes for full structural validity.

use tcl_compiler::codegen::wasm::{wasm_codegen_module, WasmModule};
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

/// Lower Tcl source and run the greenfield WASM backend over its top level.
fn compile_wasm(source: &str) -> WasmModule {
    let registry = CommandRegistry::build_default();
    let module = lower_to_ir(source, &registry);
    wasm_codegen_module(&module, source)
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

/// An `if`/`else` is recovered as **structured** WASM control flow, with the
/// clean (brace-stripped) condition text interned for `tcl_expr_bool`.
#[test]
fn if_else_is_structured() {
    let mut m = compile_wasm("if {1} {puts a} else {puts b}\n");
    let wat = m.to_wat();
    assert!(wat.contains("\n        if"), "expected structured if:\n{wat}");
    assert!(wat.contains("else"), "expected else arm:\n{wat}");
    assert!(wat.contains("end"), "expected end:\n{wat}");
    // The condition is interned brace-stripped (`1`, not `{1`).
    assert!(wat.contains(r#"(data (i32.const 0) "1")"#), "{wat}");
    // Both arms' bodies are present in the data section.
    assert!(wat.contains(r#""puts a""#), "{wat}");
    assert!(wat.contains(r#""puts b""#), "{wat}");
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// A `while` loop becomes a structured `block`/`loop` with a `br_if` guard.
#[test]
fn while_loop_is_structured() {
    let mut m = compile_wasm("while {$i < 10} {puts $i}\n");
    let wat = m.to_wat();
    assert!(wat.contains("\n        block"), "expected break block:\n{wat}");
    assert!(wat.contains("loop"), "expected loop:\n{wat}");
    assert!(wat.contains("br_if"), "expected guard br_if:\n{wat}");
    // Condition interned brace-stripped; body interned.
    assert!(wat.contains(r#""$i < 10""#), "{wat}");
    assert!(wat.contains(r#""puts $i""#), "{wat}");
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// `break` / `continue` inside a loop are realised as structured branches, not
/// eval-fallback'd commands.
#[test]
fn break_continue_are_structured() {
    let mut m = compile_wasm("while {1} {if {$x} {break} else {continue}}\n");
    let wat = m.to_wat();
    // Two unconditional branches (break + continue); the guard uses br_if.
    assert!(wat.contains("\n            br ") || wat.contains("\n                br "), "{wat}");
    // `break`/`continue` must NOT be interned as eval-fallback command text.
    assert!(!wat.contains(r#""break""#), "break should be structural, not eval:\n{wat}");
    assert!(!wat.contains(r#""continue""#), "continue should be structural:\n{wat}");
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// `foreach` (whose iteration eval-fallback can't realise structurally) degrades
/// to a single whole-command eval-fallback.
#[test]
fn foreach_is_opaque_eval_fallback() {
    let mut m = compile_wasm("foreach x {a b c} {puts $x}\n");
    let wat = m.to_wat();
    // The whole `foreach …` command is interned and eval'd as one unit; there is
    // no structured `loop` recovered for it.
    assert!(wat.contains(r#""foreach x {a b c} {puts $x}""#), "{wat}");
    assert!(!wat.contains("\n            loop"), "foreach must not structure:\n{wat}");
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
        ("elseif", "if {$a} {puts a} elseif {$b} {puts b} else {puts c}\n"),
        ("nested-if", "if {1} {if {2} {puts a}}\nputs done\n"),
        ("while", "while {$i < 10} {puts $i}\n"),
        ("while-break", "while {1} {if {$done} {break}\nputs x}\n"),
        ("while-continue", "while {$i} {if {$skip} {continue}\nputs $i}\n"),
        ("for", "for {set i 0} {$i < 10} {incr i} {puts $i}\n"),
        ("for-break", "for {set i 0} {1} {incr i} {if {$i} {break}}\n"),
        (
            "nested-loop",
            "while {$a} {for {set i 0} {$i<3} {incr i} {if {$i} {break} else {continue}}}\n",
        ),
        ("return-mid", "if {$x} {return 1}\nputs after\n"),
        ("foreach-opaque", "foreach x {a b c} {puts $x}\n"),
    ] {
        let mut m = compile_wasm(src);
        let wasm = tmp.join(format!("tcl_wasm_stage2_{name}.wasm"));
        let cwasm = tmp.join(format!("tcl_wasm_stage2_{name}.cwasm"));
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
            "{name}.wasm failed wasmtime validation:\n{}\n--- wat ---\n{}",
            String::from_utf8_lossy(&status.stderr),
            m.to_wat(),
        );
        let _ = std::fs::remove_file(&wasm);
        let _ = std::fs::remove_file(&cwasm);
    }
}
