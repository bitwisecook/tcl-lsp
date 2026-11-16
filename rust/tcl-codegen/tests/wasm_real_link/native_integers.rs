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

//! The i64 boundary: the wide-integer ABI link probe and sealed native i64 addition with its trace fallback.

use super::*;

/// A minimal WASI link probe for the i64-to-owned-Tcl-value ABI. It exercises
/// the shared descriptor's exact WASM signature without depending on an
/// emitter that has not yet selected native arithmetic.
fn native_wide_int_bootstrap_wat(value: i64) -> String {
    let value_new_wide_int = CodegenAbiImportId::ValueNewWideInt.descriptor().name;
    let object_release = CodegenAbiImportId::ObjectRelease.descriptor().name;
    format!(
        r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "{value_new_wide_int}" (func $new_wide_int (param i64) (result i32)))
  (import "tcl" "{object_release}" (func $release (param i32)))
  (import "tcl" "Tcl_GetStringFromObj" (func $getstr (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $value i32) (local $strptr i32) (local $len i32)
    (local.set $value (call $new_wide_int (i64.const {value})))
    (local.set $strptr (call $getstr (local.get $value) (i32.const 0x190000)))
    (local.set $len (i32.load (i32.const 0x190000)))
    (i32.store (i32.const 0x190008) (local.get $strptr))
    (i32.store (i32.const 0x19000C) (local.get $len))
    (drop (call $fd_write (i32.const 1) (i32.const 0x190008) (i32.const 1) (i32.const 0x190010)))
    (call $release (local.get $value))))
"#,
    )
}

#[test]
fn native_wide_int_link_probe_uses_the_shared_i64_descriptor_signature() {
    let wat = native_wide_int_bootstrap_wat(-42);
    assert!(wat.contains(r#""tcl_value_new_wide_int""#), "{wat}");
    assert!(wat.contains("(param i64) (result i32)"), "{wat}");
    assert!(wat.contains("i64.const -42"), "{wat}");
}

fn native_i64_add_options() -> WasmCompileOptions {
    [
        SemanticOptimisationPassId::DirectProc,
        SemanticOptimisationPassId::MaterialisableSlot,
        SemanticOptimisationPassId::FrameElision,
        SemanticOptimisationPassId::NativeInteger,
        SemanticOptimisationPassId::SemanticOperationSpecialisation,
    ]
    .into_iter()
    .fold(
        WasmCompileOptions::runtime_linked().for_sealed_program(),
        WasmCompileOptions::with_semantic_optimisation,
    )
}

fn native_i64_add_bootstrap_wat(expected_dispatches: Option<u32>) -> String {
    let create = CodegenAbiImportId::RuntimeCreateInterp.descriptor().name;
    let set_current = CodegenAbiImportId::RuntimeSetCurrentInterp
        .descriptor()
        .name;
    let dispatches = CodegenAbiImportId::NativeProcDispatches.descriptor().name;
    let checked = expected_dispatches.map_or_else(String::new, |count| {
        format!("(if (i32.ne (call $dispatches) (i32.const {count})) (then unreachable))")
    });
    format!(
        r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "{create}" (func $create (result i32)))
  (import "tcl" "{set_current}" (func $setcur (param i32)))
  (import "user" "::top" (func $top))
  (import "tcl" "{dispatches}" (func $dispatches (result i32)))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $interp i32)
    (local.set $interp (call $create))
    (call $setcur (local.get $interp))
    (call $top)
    {checked}))"#,
    )
}

fn run_real_native_i64_add(runtime: &Path, program: &str) -> String {
    let fixture = native_whole_program_fixture(program);
    let registry = fixture.context.commands();
    let unit = &fixture.unit;
    let mut output = compile_wasm(unit, registry, native_i64_add_options());
    report_actual_native_admission(program, unit, &output);
    assert!(
        matches!(output.plan, WasmCodegenPlan::NativeI64Add { .. }),
        "native proof did not select: {:?}",
        output.plan
    );
    let user = scratch("tcl_real_native_i64_user.wasm");
    let boot = scratch("tcl_real_native_i64_boot.wat");
    write_linked(runtime, &user, &output.to_bytes(), program);
    let expected_dispatches = match &output.plan {
        WasmCodegenPlan::NativeI64Add { native, .. } if !native.frame_elided => Some(1),
        _ => None,
    };
    std::fs::write(&boot, native_i64_add_bootstrap_wat(expected_dispatches))
        .expect("write native i64 bootstrap");
    let native_diagnostic =
        std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref() == Ok("first-decline-v1");
    let out = Command::new("wasmtime")
        .arg("run")
        .args(if native_diagnostic {
            &[
                "--env",
                "TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC=first-decline-v1",
            ][..]
        } else {
            &[][..]
        })
        .arg("--preload")
        .arg(format!("tcl={}", runtime.display()))
        .arg("--preload")
        .arg(format!("user={}", user.display()))
        .arg(&boot)
        .output()
        .expect("run wasmtime");
    let _ = std::fs::remove_file(&user);
    let _ = std::fs::remove_file(&boot);
    assert!(
        out.status.success(),
        "native i64 link trapped for {program:?}:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("stdout is utf-8")
}

fn run_real_native_wide_int_abi(runtime: &Path, value: i64) -> String {
    let boot = scratch("tcl_real_native_wide_int_boot.wat");
    std::fs::write(&boot, native_wide_int_bootstrap_wat(value))
        .expect("write native wide-int bootstrap");
    let out = Command::new("wasmtime")
        .arg("run")
        .arg("--preload")
        .arg(format!("tcl={}", runtime.display()))
        .arg(&boot)
        .output()
        .expect("run wasmtime");
    let _ = std::fs::remove_file(&boot);
    assert!(
        out.status.success(),
        "native wide-int ABI link trapped:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("stdout is utf-8")
}

/// The shared descriptor links an i64 WASM value to an owned Tcl wide integer,
/// whose normal Tcl string result remains exact at the signed range boundary.
#[test]
fn native_wide_int_codegen_abi_links_against_the_real_runtime() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    assert_eq!(
        run_real_native_wide_int_abi(&runtime, i64::MIN),
        i64::MIN.to_string()
    );
}

/// The sealed common proof path runs native i64 addition, materialises only
/// its output at the registry-proved channel-write boundary, and prints via
/// the live runtime. A registered execution trace makes the proof decline;
/// the unchanged general interpreter path produces the same Tcl output.
#[test]
fn sealed_native_i64_add_runs_and_trace_registration_falls_back() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let program = "proc add {b c} {return [expr {$b + $c}]}\nset d 2\nset e 4\nputs [add $d $e]\n";
    assert_eq!(run_real_native_i64_add(&runtime, program), "6\n");

    let traced = format!("trace add execution puts enter list\n{program}");
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(&traced, &registry, false, "tcl9.0");
    assert!(matches!(
        compile_wasm(&unit, &registry, native_i64_add_options()).plan,
        WasmCodegenPlan::General { .. }
    ));
    assert_eq!(
        run_real_link_with_options(
            &runtime,
            "native_i64_add",
            &traced,
            "set d",
            native_i64_add_options()
        ),
        "6\n2"
    );
    let published = format!("{program}puts [list [info args add] [info body add] $d $e]\n");
    assert_eq!(
        run_real_native_i64_add(&runtime, &published),
        "6\n{b c} {return [expr {$b + $c}]} 2 4\n",
        "retained Native selection must preserve actual procedure publication and both stores"
    );
}
