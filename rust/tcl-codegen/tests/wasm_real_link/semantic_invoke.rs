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

//! The canonical generic-argv plan and the guarded boxed intrinsic with its fallback.

use super::*;

/// Run the semantic plan selected by the sole public codegen entry point.
fn run_real_generic_invoke(runtime: &Path, program: &str, expected_code: i32) -> String {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(program, &registry, false, "tcl9.0");
    let mut output = compile_wasm(&unit, &registry, WasmCompileOptions::runtime_linked());
    assert!(
        matches!(output.plan, WasmCodegenPlan::GenericInvoke { .. }),
        "literal program should select generic argv, got {:?}",
        output.plan
    );
    let user_bytes = output.to_bytes();

    let user = scratch("tcl_real_generic_user.wasm");
    let boot = scratch("tcl_real_generic_boot.wat");
    write_linked(runtime, &user, &user_bytes, program);
    std::fs::write(&boot, generic_invoke_bootstrap_wat(expected_code)).expect("write bootstrap");
    let out = Command::new("wasmtime")
        .arg("run")
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
        "canonical generic argv link trapped for {program:?}:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("stdout is utf-8")
}

/// Run the guarded semantic-invocation emission through the real runtime.
/// `setup` mutates the interpreter before `::top`, forcing its runtime guard
/// to take the exact generic argv fallback where appropriate.
fn run_real_guarded_intrinsic_invoke(
    runtime: &Path,
    program: &str,
    expected_code: i32,
    setup: Option<&str>,
) -> String {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(program, &registry, false, "tcl9.0");
    let options = WasmCompileOptions::runtime_linked()
        .with_semantic_optimisation(SemanticOptimisationPassId::GuardedIntrinsic);
    let mut output = compile_wasm(&unit, &registry, options);
    assert!(
        matches!(output.plan, WasmCodegenPlan::GenericInvoke { .. }),
        "literal program should retain semantic invocation evidence, got {:?}",
        output.plan
    );
    let wat = output.to_wat();
    assert!(wat.contains("tcl_intrinsic_invoke_argv"), "{wat}");
    let user_bytes = output.to_bytes();

    let user = scratch("tcl_real_guarded_user.wasm");
    let boot = scratch("tcl_real_guarded_boot.wat");
    write_linked(runtime, &user, &user_bytes, program);
    std::fs::write(&boot, semantic_invoke_bootstrap_wat(expected_code, setup))
        .expect("write guarded bootstrap");
    let out = Command::new("wasmtime")
        .arg("run")
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
        "guarded intrinsic link trapped for {program:?}, setup {setup:?}:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("stdout is utf-8")
}

/// The canonical semantic plan invokes prebuilt argv in the real runtime.
#[test]
fn canonical_generic_argv_runs_against_the_real_runtime() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    for (program, code, expected) in [("string length abc\n", 0, "3"), ("error boom\n", 1, "boom")]
    {
        assert_eq!(
            run_real_generic_invoke(&runtime, program, code),
            expected,
            "program {program:?}"
        );
    }
}

/// The selected boxed intrinsic both executes directly in the live runtime and
/// safely falls through the exact same argv path after command-environment or
/// execution-trace mutations invalidate its guard admission.
#[test]
fn guarded_boxed_intrinsic_runs_and_falls_back_against_the_real_runtime() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let cases = [
        ("fast hit", "string length 😀\n", None, "1"),
        (
            "renamed fallback",
            "string length abc\n",
            Some("rename string string_original\nproc string {subcommand value} {return 99}\n"),
            "99",
        ),
        (
            "traced fallback",
            "string length abc\n",
            Some("trace add execution string enter list\n"),
            "3",
        ),
        // The result is the same on either path, which is why the runtime
        // crate's `guarded_intrinsic_guards_survive_unrelated_command_mutation`
        // checks the guard itself; this row proves the emitted module links and
        // runs over an interpreter whose command table has moved.
        (
            "unrelated proc keeps the fast path",
            "string length 😀\n",
            Some(
                "proc unrelated {} {return 1}\nrename unrelated other\n\
                 interp alias {} alias_of_other {} other\n",
            ),
            "1",
        ),
    ];
    for (name, program, setup, expected) in cases {
        assert_eq!(
            run_real_guarded_intrinsic_invoke(&runtime, program, 0, setup),
            expected,
            "{name}"
        );
    }
}
