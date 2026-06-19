//! **End-to-end execution** — a real emitted module *runs* under wasmtime.
//!
//! `wasm_codegen.rs` proves the emitted bytes are structurally valid
//! (`wasmtime compile`). This goes one step further: it **instantiates and
//! invokes** the emitted `::top` on the wasmtime engine, proving the module
//! actually runs — imports resolve against a real provider, the imported memory
//! and data segments wire up, and the structured control flow + eval-fallback
//! `call`s execute to completion without trapping.
//!
//! The emitted module imports four `tcl_*` host functions and its linear memory
//! from module `"tcl"` (the runtime's codegen ABI — `runtime/rust/codegen_abi.rs`
//! now exports exactly this surface). Rather than link the whole runtime (the
//! shared-memory dynamic-linking + `__memory_base` relocation that the
//! whole-program artifact needs — a later increment), we satisfy those imports
//! with a tiny **host stub** generated from the compiler's own WASM IR and
//! `--preload`ed into wasmtime. The stub's `tcl_expr_bool` returns `0`, so every
//! condition is false: each `if` takes its else, each loop exits immediately, and
//! the invoked `::top` terminates without needing a real interpreter.
//!
//! This is **Tier 0**: it proves *runnability*. Observing the eval-fallback
//! command sequence (a host backed by a real `Interp`, asserting which branch
//! ran) is the next tier and needs in-process host functions (the wasmtime
//! embedder) or a WASI-writing stub.

use tcl_compiler::codegen::wasm::{
    ValType, WasmFunction, WasmInstruction, WasmModule, WasmOp, wasm_codegen_module,
};
use tcl_compiler::lowering::lower_to_ir;
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
/// the four `tcl_*` functions the emitted module imports, with trivial bodies.
/// The eval fallbacks return a dummy `0` obj handle (the emitted module only
/// passes these handles back, never dereferences them); `tcl_expr_bool` returns
/// `0` so all control flow terminates.
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
            "tcl_eval",
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
    let module = lower_to_ir(src, &registry);
    wasm_codegen_module(&module, src)
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
