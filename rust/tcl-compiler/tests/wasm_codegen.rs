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

//! WASM backend tests: the greenfield eval-fallback emitter produces a
//! structurally valid module with the right shape (imports, data section,
//! eval-fallback calls, structured `if` / loops). Execution against a runtime is
//! currently (the Rust runtime's wasm32 ABI is still a stub), so these
//! assert the emitted structure; `wasmtime compile` (run below where the CLI is
//! present) validates the bytes for full structural validity.

use tcl_compiler::codegen::wasm::{
    RESERVED_DATA_BASE, WasmCompileOptions, WasmModule, compile_wasm as compile_wasm_unit,
};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

/// Lower Tcl source and run the greenfield WASM backend over its top level.
fn compile_wasm(source: &str) -> WasmModule {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(source, &registry, false);
    compile_wasm_unit(
        &unit,
        &registry,
        WasmCompileOptions::hosted()
            .for_eval_only_test_host()
            .with_data_base(0),
    )
    .into_module()
}

/// As [`compile_wasm`], but relocate the constant pool to `data_base`.
fn compile_wasm_based(source: &str, data_base: i64) -> WasmModule {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(source, &registry, false);
    compile_wasm_unit(
        &unit,
        &registry,
        WasmCompileOptions::hosted()
            .for_eval_only_test_host()
            .with_data_base(data_base),
    )
    .into_module()
}

fn compile_wasm_analysed(source: &str) -> WasmModule {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(source, &registry, false);
    compile_wasm_unit(&unit, &registry, WasmCompileOptions::hosted()).into_module()
}

#[test]
fn analysed_add_program_uses_direct_tcl_object_operations() {
    let source = r"proc add {b c} {
    return [expr {$b + $c}]
}

set e 2
set f 4
puts [add $e $f]
";
    let mut module = compile_wasm_analysed(source);
    let wat = module.to_wat();

    assert!(
        wat.contains(r#"(func $::add (export "::add") (param $b i32) (param $c i32) (result i32)"#),
        "{wat}"
    );
    assert!(wat.contains(r#""tcl_codegen_local_bind""#), "{wat}");
    assert!(wat.contains(r#""tcl_codegen_expr_add""#), "{wat}");
    assert!(wat.contains(r#""tcl_codegen_puts""#), "{wat}");
    assert!(wat.contains(r#""tcl_codegen_proc_register""#), "{wat}");
    assert!(!wat.lines().any(|line| line.trim() == "call 1"), "{wat}");
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

#[test]
fn analysed_direct_call_declines_when_binding_lattice_is_opaque() {
    let source = r"proc add {b c} {return [expr {$b + $c}]}
rename add old_add
puts [add 2 4]
";
    let mut module = compile_wasm_analysed(source);
    let wat = module.to_wat();

    assert!(
        wat.lines().any(|line| line.trim() == "call 1"),
        "the rebound call must retain source evaluation:\n{wat}"
    );
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

#[test]
fn analysed_assignment_preserves_dynamic_array_index_substitution() {
    let mut module = compile_wasm_analysed("set x foo\nset a($x) 1\n");
    let wat = module.to_wat();

    assert!(
        wat.contains(r#""set a($x) 1""#),
        "the substituted target must retain source evaluation:\n{wat}"
    );
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

#[test]
fn analysed_assignment_preserves_a_rebound_set_command() {
    let source = r"proc replacement args {return replacement}
proc rebind {} {
    rename set original_set
    rename replacement set
}
rebind
set x 1
";
    let mut module = compile_wasm_analysed(source);
    let wat = module.to_wat();

    assert!(
        wat.contains(r#""set x 1""#),
        "a potentially rebound set must retain source evaluation:\n{wat}"
    );
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

#[test]
fn analysed_direct_proc_requires_original_expr_and_return_bindings() {
    for (name, rebind) in [
        ("expr", "rename expr original_expr\nrename replacement expr"),
        (
            "return",
            "rename return original_return\nrename replacement return",
        ),
    ] {
        let source = format!(
            "proc replacement args {{list replacement}}\n\
             proc rebind {{}} {{{rebind}}}\n\
             proc add {{a b}} {{return [expr {{$a + $b}}]}}\n\
             rebind\n\
             puts [add 2 4]\n"
        );
        let mut module = compile_wasm_analysed(&source);
        let wat = module.to_wat();

        assert!(
            !wat.contains(
                r#"(func $::add (export "::add") (param $a i32) (param $b i32) (result i32)"#
            ),
            "a potentially rebound {name} must disable the direct procedure:\n{wat}"
        );
        assert!(
            wat.contains(r#""puts [add 2 4]""#),
            "the call must retain source evaluation when {name} can change:\n{wat}"
        );
        assert_eq!(&module.to_bytes()[0..4], b"\0asm");
    }
}

#[test]
fn analysed_direct_call_preserves_static_and_dynamic_execution_traces() {
    for (name, trace_setup) in [
        ("static", "trace add execution add enter callback"),
        (
            "dynamic",
            "set traced_command add\ntrace add execution $traced_command enter callback",
        ),
    ] {
        let source = format!(
            "proc callback args {{}}\n\
             proc add {{a b}} {{return [expr {{$a + $b}}]}}\n\
             {trace_setup}\n\
             puts [add 1 2]\n"
        );
        let mut module = compile_wasm_analysed(&source);
        let wat = module.to_wat();

        assert!(
            wat.contains(r#""puts [add 1 2]""#),
            "a {name} execution trace must retain runtime dispatch:\n{wat}"
        );
        assert_eq!(&module.to_bytes()[0..4], b"\0asm");
    }
}

#[test]
fn analysed_puts_preserves_braced_command_text_as_literal() {
    let mut module = compile_wasm_analysed("puts {[add 2 4]}\n");
    let wat = module.to_wat();

    assert!(
        wat.lines().any(|line| line.trim() == "call 1"),
        "a braced command-shaped word must remain literal:\n{wat}"
    );
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// With a non-zero data base, both the data segments and the `i32.const` offsets
/// the module hands to `tcl_obj_new_string` are relocated into the runtime's
/// reserved region — so the emitted constant pool does not collide with the
/// runtime's shadow stack in the shared-memory whole-program link.
#[test]
fn data_pool_relocates_to_reserved_base() {
    let mut m = compile_wasm_based("set x 5\nputs $x\n", RESERVED_DATA_BASE);
    let wat = m.to_wat();
    // First string sits at the base, the second right after it (base + 7).
    assert!(
        wat.contains(&format!(
            "(data (i32.const {RESERVED_DATA_BASE}) \"set x 5\")"
        )),
        "{wat}"
    );
    let second = RESERVED_DATA_BASE + i64::try_from("set x 5".len()).unwrap();
    assert!(
        wat.contains(&format!("(data (i32.const {second}) \"puts $x\")")),
        "{wat}"
    );
    // The body pushes the *based* offset (not 0) before calling tcl_obj_new_string.
    assert!(
        wat.contains(&format!("i32.const {RESERVED_DATA_BASE}")),
        "{wat}"
    );
    // Still a valid module, and offset 0 no longer carries data.
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
    assert!(!wat.contains("(data (i32.const 0)"), "{wat}");
}

/// A user-defined `proc` is emitted as its own exported WASM function, its body
/// driven through the same structured walk as `::top`. (The `proc` definition
/// itself still eval-fallbacks in `::top` to register it at run time, until
/// call-dispatch is wired.)
#[test]
fn procs_emit_their_own_functions() {
    let mut m = compile_wasm("proc add {a b} {return [expr {$a + $b}]}\nadd 1 2\n");
    let wat = m.to_wat();
    // Both the top-level entry and the proc body are exported functions.
    assert!(wat.contains(r#"(export "::top")"#), "{wat}");
    assert!(wat.contains(r#"(export "::add")"#), "{wat}");
    // The proc body's command is interned (from the `::add` walk).
    assert!(wat.contains(r#""return [expr {$a + $b}]""#), "{wat}");
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// A proc body's control flow is **structured inside its own function**, not an
/// opaque eval-fallback of the whole body.
#[test]
fn proc_body_control_flow_is_structured() {
    let mut m = compile_wasm("proc choose {x} {if {$x} {puts a} else {puts b}}\n");
    let wat = m.to_wat();
    assert!(wat.contains(r#"(export "::choose")"#), "{wat}");
    assert!(
        wat.contains("\n        if"),
        "expected structured if in the proc body:\n{wat}"
    );
    // The condition + both arms are interned (driven through the walk).
    assert!(wat.contains(r#""$x""#), "{wat}");
    assert!(
        wat.contains(r#""puts a""#) && wat.contains(r#""puts b""#),
        "{wat}"
    );
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// Namespace-scoped procs (created at run time inside `namespace eval`, not at
/// load) are not emitted as load-time functions — mirroring the bytecode backend.
#[test]
fn namespace_scoped_procs_are_not_emitted() {
    let mut m = compile_wasm("namespace eval ns {\n  proc p {} {puts hi}\n}\n");
    let wat = m.to_wat();
    assert!(
        !wat.contains(r#"(export "::ns::p")"#),
        "a namespace-scoped proc must not become a load-time function:\n{wat}"
    );
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// A linear top-level script: each command is eval-fallback'd in order, results
/// discarded; the command texts live in the data section.
#[test]
fn linear_top_level_eval_fallback() {
    let mut m = compile_wasm("set x 5\nputs $x\n");
    let wat = m.to_wat();
    // The eval-fallback import boundary is declared.
    assert!(wat.contains(r#""tcl_obj_new_string""#), "{wat}");
    assert!(wat.contains(r#""tcl_eval_code""#), "{wat}");
    assert!(wat.contains(r#""memory""#), "{wat}");
    // Both commands' source text is interned in the data section.
    assert!(wat.contains(r#""set x 5""#), "{wat}");
    assert!(wat.contains(r#""puts $x""#), "{wat}");
    // Exported top-level entry.
    assert!(wat.contains(r#"(export "::top")"#), "{wat}");
    // Two eval-fallback sequences (box → eval_code → dispatch) ⇒ two `call 1`
    // (`tcl_eval_code`, import index 1).
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
    assert!(
        wat.contains("\n        if"),
        "expected structured if:\n{wat}"
    );
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
    assert!(
        wat.contains("\n        block"),
        "expected break block:\n{wat}"
    );
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
    assert!(
        wat.contains("\n            br ") || wat.contains("\n                br "),
        "{wat}"
    );
    // `break`/`continue` must NOT be interned as eval-fallback command text.
    assert!(
        !wat.contains(r#""break""#),
        "break should be structural, not eval:\n{wat}"
    );
    assert!(
        !wat.contains(r#""continue""#),
        "continue should be structural:\n{wat}"
    );
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
    assert!(
        !wat.contains("\n            loop"),
        "foreach must not structure:\n{wat}"
    );
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// Every emitted module is **structurally valid WASM** — confirmed by
/// `wasmtime compile` (which fully validates before native compilation). Skips
/// gracefully where the `wasmtime` CLI isn't available.
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
        (
            "elseif",
            "if {$a} {puts a} elseif {$b} {puts b} else {puts c}\n",
        ),
        ("nested-if", "if {1} {if {2} {puts a}}\nputs done\n"),
        ("while", "while {$i < 10} {puts $i}\n"),
        ("while-break", "while {1} {if {$done} {break}\nputs x}\n"),
        (
            "while-continue",
            "while {$i} {if {$skip} {continue}\nputs $i}\n",
        ),
        ("for", "for {set i 0} {$i < 10} {incr i} {puts $i}\n"),
        (
            "for-break",
            "for {set i 0} {1} {incr i} {if {$i} {break}}\n",
        ),
        (
            "nested-loop",
            "while {$a} {for {set i 0} {$i<3} {incr i} {if {$i} {break} else {continue}}}\n",
        ),
        ("return-mid", "if {$x} {return 1}\nputs after\n"),
        ("foreach-opaque", "foreach x {a b c} {puts $x}\n"),
        // Multi-function modules: a proc body becomes its own function alongside
        // `::top`; both must validate.
        (
            "proc",
            "proc add {a b} {return [expr {$a + $b}]}\nadd 1 2\n",
        ),
        (
            "proc-cf",
            "proc choose {x} {if {$x} {puts a} else {puts b}}\nchoose 1\n",
        ),
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
