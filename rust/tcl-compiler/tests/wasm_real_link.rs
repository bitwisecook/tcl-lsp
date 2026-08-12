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

//! **The real whole-program link** — an emitted WASM module runs against the
//! *actual* `tcl_runtime.wasm`, not a stub.
//!
//! `wasm_execute.rs` runs emitted modules against a hand-built host stub. This
//! goes all the way: it links the emitted module against the real Rust runtime
//! compiled to `wasm32-wasip1`, sharing one linear memory, and proves a
//! genuine side effect of the emitted program survives in the real interpreter.
//!
//! Three modules, composed by the wasmtime CLI (`--preload`), share the
//! runtime's exported memory:
//!
//! 1. **`tcl` = the real runtime** — exports `memory` + the codegen ABI
//!    (`tcl_obj_new_string`/`tcl_eval`/`tcl_eval_code`/`tcl_obj_release`/
//!    `tcl_expr_bool`, `tcl_runtime_*_interp`) and the C ABI
//!    (`Tcl_GetStringFromObj`). Built with `--global-base=0x200000` so its
//!    data/heap sit above a reserved gap `[0x100000, 0x200000)` (its 1 MiB shadow
//!    stack stays at `[0, 0x100000)`).
//! 2. **`user` = the emitted module** — its analysed statements use the direct
//!    Tcl-object ABI where binding and type lattices prove that path. Its
//!    constant pool is relocated to [`RESERVED_DATA_BASE`] (`0x100000`) so it
//!    lands in the gap, not under the runtime's stack.
//! 3. **bootstrap** (the WASI command) — creates an interp, makes it current,
//!    calls `user::top`, then `tcl_eval`s a `query` against the *same* interp,
//!    reads the result string with `Tcl_GetStringFromObj`, and writes it to
//!    stdout. The printed result proves the emitted program's side effect
//!    persisted in the real runtime.
//!
//! The cases cover a direct variable store, an interpreted fallback procedure,
//! and the direct numeric `add` procedure. That last one proves the linked
//! generated function binds Tcl-visible parameters, adds through the numeric
//! tower, and writes `6` through the real WASI stdout path.
//!
//! [`compiled_argv_invocations_run_against_the_real_runtime`] then covers the
//! general tier's normal leaf-command path — every word shape it compiles,
//! runtime dispatch following a `rename`, and an abrupt completion that stops
//! the script — and
//! [`compiled_argv_balances_allocations_in_the_real_runtime`] proves that path
//! balances its allocations in the real runtime, including when it leaves
//! part-way through argv construction.
//!
//! Heavy + gated: it builds `runtime/rust` to wasm (cached after the first run)
//! and shells out to the `wasmtime` CLI. It skips cleanly when either is
//! unavailable, mirroring the other wasm tests.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use tcl_compiler::codegen::wasm::{
    SemanticOptimisationPassId, WasmCodegenPlan, WasmCompileOptions, compile_wasm,
};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;
use tcl_runtime_api::codegen_abi::CodegenAbiImportId;

/// The bootstrap WASI command (see the module docs): create + select an interp,
/// run the emitted `::top`, then evaluate `query` against the same interp and
/// print its result string. Reserved-gap scratch: the `query` string at
/// `0x180000`, the `Tcl_GetStringFromObj` length-out at `0x190000`, and the
/// `fd_write` iovec/result at `0x190008`/`0x190010` — all above the emitted
/// module's data at `0x100000` and below the runtime's data at `0x200000`.
/// `query` must be WAT-literal-safe (no `"` or `\`).
fn bootstrap_wat(query: &str) -> String {
    let create = CodegenAbiImportId::RuntimeCreateInterp.descriptor().name;
    let set_current = CodegenAbiImportId::RuntimeSetCurrentInterp
        .descriptor()
        .name;
    let object_new_string = CodegenAbiImportId::ObjectNewString.descriptor().name;
    let object_release = CodegenAbiImportId::ObjectRelease.descriptor().name;
    format!(
        r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "{create}" (func $create (result i32)))
  (import "tcl" "{set_current}" (func $setcur (param i32)))
  (import "tcl" "{object_new_string}" (func $box (param i32 i32) (result i32)))
  (import "tcl" "tcl_eval" (func $eval (param i32) (result i32)))
  (import "tcl" "{object_release}" (func $rel (param i32)))
  (import "tcl" "Tcl_GetStringFromObj" (func $getstr (param i32 i32) (result i32)))
  (import "user" "::top" (func $top))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $interp i32) (local $result i32) (local $strptr i32) (local $len i32)
    (local.set $interp (call $create))
    (call $setcur (local.get $interp))
    (call $top)                                                                       ;; run the emitted program
    (local.set $result (call $eval (call $box (i32.const 0x180000) (i32.const {qlen})))) ;; eval the query
    (local.set $strptr (call $getstr (local.get $result) (i32.const 0x190000)))
    (local.set $len (i32.load (i32.const 0x190000)))
    (i32.store (i32.const 0x190008) (local.get $strptr))
    (i32.store (i32.const 0x19000C) (local.get $len))
    (drop (call $fd_write (i32.const 1) (i32.const 0x190008) (i32.const 1) (i32.const 0x190010)))
    (call $rel (local.get $result)))
  (data (i32.const 0x180000) "{query}"))
"#,
        qlen = query.len(),
    )
}

/// Run one canonical generic-argv plan and print its full completion result.
fn generic_invoke_bootstrap_wat(expected_code: i32) -> String {
    semantic_invoke_bootstrap_wat(expected_code, None)
}

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

/// As [`generic_invoke_bootstrap_wat`], with an optional interpreter mutation
/// performed after the live interpreter exists and before generated code runs.
/// This lets the whole-program tests prove a guarded emission declines into its
/// retained generic argv fallback under a renamed or traced command.
/// Escape bytes for a WAT string literal.
///
/// A WAT string may not contain a raw newline, quote, or backslash — a
/// multi-line setup script interpolated verbatim makes `wasmtime` reject the
/// module with `invalid character in string`. Every escape here decodes back to
/// exactly one byte, so a caller may still use the *unescaped* length as the
/// data segment's byte count.
fn wat_escape(source: &str) -> String {
    let mut escaped = String::with_capacity(source.len());
    for byte in source.bytes() {
        match byte {
            b'\n' => escaped.push_str("\\n"),
            b'\r' => escaped.push_str("\\r"),
            b'\t' => escaped.push_str("\\t"),
            b'"' => escaped.push_str("\\\""),
            b'\\' => escaped.push_str("\\\\"),
            0x20..=0x7e => escaped.push(byte as char),
            other => {
                let _ = write!(escaped, "\\{other:02x}");
            }
        }
    }
    escaped
}

fn semantic_invoke_bootstrap_wat(expected_code: i32, setup: Option<&str>) -> String {
    let create = CodegenAbiImportId::RuntimeCreateInterp.descriptor().name;
    let set_current = CodegenAbiImportId::RuntimeSetCurrentInterp
        .descriptor()
        .name;
    let object_new_string = CodegenAbiImportId::ObjectNewString.descriptor().name;
    let object_release = CodegenAbiImportId::ObjectRelease.descriptor().name;
    let (setup_run, setup_data) = setup.map_or_else(
        || (String::new(), String::new()),
        |source| {
            (
                format!(
                    "    (local.set $setup_result (call $eval (call $box (i32.const 0x170000) (i32.const {}))))\n    (call $rel (local.get $setup_result))\n",
                    source.len()
                ),
                format!("  (data (i32.const 0x170000) \"{}\")\n", wat_escape(source)),
            )
        },
    );
    format!(
        r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "{create}" (func $create (result i32)))
  (import "tcl" "{set_current}" (func $setcur (param i32)))
  (import "tcl" "{object_new_string}" (func $box (param i32 i32) (result i32)))
  (import "tcl" "{object_release}" (func $rel (param i32)))
  (import "tcl" "tcl_eval" (func $eval (param i32) (result i32)))
  (import "tcl" "Tcl_GetStringFromObj" (func $getstr (param i32 i32) (result i32)))
  (import "tcl" "tcl_codegen_call_frame_outstanding" (func $frames (result i32)))
  (import "user" "::top" (func $top (result i32 i32 i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $interp i32) (local $code i32) (local $result i32) (local $options i32)
    (local $strptr i32) (local $len i32) (local $setup_result i32)
    (local.set $interp (call $create))
    (call $setcur (local.get $interp))
{setup_run}
    (call $top)
    (local.set $options)
    (local.set $result)
    (local.set $code)
    (if (i32.ne (local.get $code) (i32.const {expected_code})) (then unreachable))
    (local.set $strptr (call $getstr (local.get $result) (i32.const 0x190000)))
    (local.set $len (i32.load (i32.const 0x190000)))
    (i32.store (i32.const 0x190008) (local.get $strptr))
    (i32.store (i32.const 0x19000C) (local.get $len))
    (drop (call $fd_write (i32.const 1) (i32.const 0x190008) (i32.const 1) (i32.const 0x190010)))
    (call $rel (local.get $result))
    (call $rel (local.get $options))
    (if (i32.ne (call $frames) (i32.const 0)) (then unreachable)))
{setup_data})"#,
    )
}

/// Run `::top` repeatedly and prove the compiled leaf-command path balances its
/// allocations: no outstanding transient call frame, and no residual growth
/// across identical runs.
///
/// The first `::top` is a warm-up — it creates the interpreter state a compiled
/// statement touches for the first time. Every later run is allocation-neutral
/// unless the generated cleanup path leaks, so the residual must not move.
/// `tcl_test_finalize` is also required to be positive, so a counter surface
/// that read back as zero would fail loudly instead of passing vacuously.
fn leak_bootstrap_wat() -> String {
    r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "tcl_runtime_create_interp" (func $create (result i32)))
  (import "tcl" "tcl_runtime_set_current_interp" (func $setcur (param i32)))
  (import "tcl" "tcl_codegen_call_frame_outstanding" (func $frames (result i32)))
  (import "tcl" "tcl_test_finalize" (func $residual (result i64)))
  (import "tcl" "tcl_test_double_free_count" (func $doubles (result i64)))
  (import "user" "::top" (func $top))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $interp i32) (local $before i64)
    (local.set $interp (call $create))
    (call $setcur (local.get $interp))
    (call $top)
    (if (i32.ne (call $frames) (i32.const 0)) (then unreachable))
    (local.set $before (call $residual))
    (if (i64.le_s (local.get $before) (i64.const 0)) (then unreachable))
    (call $top)
    (call $top)
    (if (i64.ne (call $residual) (local.get $before)) (then unreachable))
    (if (i64.ne (call $doubles) (i64.const 0)) (then unreachable))
    (if (i32.ne (call $frames) (i32.const 0)) (then unreachable)))
)"#
    .to_owned()
}

fn have_wasmtime() -> bool {
    Command::new("wasmtime")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The workspace root (`CARGO_MANIFEST_DIR` is `…/rust/tcl-compiler`).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// Build `runtime/rust` to `wasm32-wasip1` with the reserved-region
/// linker flag, into an isolated target dir. Returns the artifact path, or `None`
/// if the build can't run (e.g. the wasm32 target is missing) so the test skips.
fn build_reserved_runtime() -> Option<PathBuf> {
    let root = workspace_root();
    let target_dir = std::env::temp_dir().join("tcl_reserved_runtime");
    let mut command = Command::new("cargo");
    let wasi_sdk = std::env::var_os("WASI_SDK_PATH")
        .map(PathBuf::from)
        .or(Some(PathBuf::from("/opt/wasi-sdk")))
        .filter(|path| path.join("bin/clang").is_file())?;
    command.env("WASI_SDK_PATH", wasi_sdk);
    if let Some(tommath) = [
        root.join("tmp/tcl9.0.4/libtommath"),
        root.join("tmp/tcl9.0.3-src/libtommath"),
        root.join("tmp/tcl8.6.16/libtommath"),
    ]
    .into_iter()
    .find(|path| path.join("tommath.h").is_file())
    {
        command.env("TCL_TOMMATH_DIR", tommath);
    }
    let ok = command
        .arg("build")
        .arg("--manifest-path")
        .arg(root.join("runtime/rust/Cargo.toml"))
        .args(["--target", "wasm32-wasip1"])
        .arg("--target-dir")
        .arg(&target_dir)
        // Reserve [0x100000, 0x200000): the 1 MiB shadow stack stays at
        // [0, 0x100000), data/heap move to >= 0x200000, leaving the gap free for
        // the emitted constant pool.
        .env("RUSTFLAGS", "-C link-arg=--global-base=2097152")
        .status()
        .is_ok_and(|s| s.success());
    ok.then(|| target_dir.join("wasm32-wasip1/debug/tcl_runtime.wasm"))
}

/// Emit `program`, link it against the real `runtime`, run its `::top`, then
/// evaluate `query` against the same interp and return the printed result.
/// Compile `program` through the general structured tier, asserting it really
/// took that tier (a whole-function semantic plan would export a different
/// `::top` signature than these bootstraps import).
fn compile_general(program: &str) -> Vec<u8> {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(program, &registry, false, "tcl9.0");
    // Constant pool in the reserved gap so it does not hit the runtime's stack.
    let mut output = compile_wasm(&unit, &registry, WasmCompileOptions::runtime_linked());
    assert!(
        matches!(output.plan, WasmCodegenPlan::General { .. }),
        "expected the general structured tier for {program:?}, got {:?}",
        output.plan
    );
    output.to_bytes()
}

/// Link and run a module that must have taken the general structured tier.
///
/// `tag` names this case's scratch files. Cases in different tests run
/// concurrently, so a shared file name would let one run load another's module.
fn run_real_link(runtime: &Path, tag: &str, program: &str, query: &str) -> String {
    run_link_bytes(runtime, tag, &compile_general(program), program, query)
}

/// Compile under the analysis tier, proving the module really reaches the
/// runtime through each of `required_calls` before anyone runs it.
///
/// The proof is the point, and it is not ceremony. `runtime_linked()` leaves
/// every semantic pass disabled, so without an explicit
/// [`SemanticOptimisationPassId::LegacyAnalysisSpecialisation`] the whole script
/// lowers to `tcl_eval_code(<source span>)` — which produces exactly the same
/// observable results as the compiled path, because it *is* the same
/// interpreter. A behavioural assertion cannot tell the two apart, so these
/// cases silently exercised source evaluation until this helper existed.
///
/// Requiring the module to merely *import* a symbol would not fix it: the
/// analysis tier adds its whole import set whether or not any statement uses
/// it. So this resolves each import's own index and demands a real `call` to
/// it in the emitted body.
fn compile_argv_analysed(program: &str, required_calls: &[&str]) -> Vec<u8> {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(program, &registry, false, "tcl9.0");
    let mut output = compile_wasm(
        &unit,
        &registry,
        WasmCompileOptions::runtime_linked()
            .with_semantic_optimisation(SemanticOptimisationPassId::LegacyAnalysisSpecialisation),
    );
    assert!(
        matches!(output.plan, WasmCodegenPlan::General { .. }),
        "expected the general structured tier for {program:?}, got {:?}",
        output.plan
    );
    let indices: Vec<(&str, usize)> = required_calls
        .iter()
        .map(|name| {
            let index = output
                .module
                .imports
                .iter()
                .position(|import| import.name == *name)
                .unwrap_or_else(|| panic!("the analysis tier imports {name}"));
            (*name, index)
        })
        .collect();
    let wat = output.module.to_wat();
    for (name, index) in indices {
        let call = format!("call {index}");
        assert!(
            wat.lines().any(|line| line.trim() == call),
            "{program:?} must call {name}; without it this case proves nothing \
             about the compiled path:\n{wat}"
        );
    }
    output.to_bytes()
}

/// Link and run a module that must have reached the dispatcher through a
/// compiled argv rather than source evaluation. See [`compile_argv_analysed`].
fn run_real_link_argv(runtime: &Path, tag: &str, program: &str, query: &str) -> String {
    let bytes = compile_argv_analysed(program, &["tcl_invoke_argv"]);
    run_link_bytes(runtime, tag, &bytes, program, query)
}

/// Link and run a module compiled under caller-chosen options. Unlike
/// [`run_real_link`] this asserts nothing about the selected plan — the point
/// of passing options is usually to select a *different* one.
fn run_real_link_with_options(
    runtime: &Path,
    tag: &str,
    program: &str,
    query: &str,
    options: WasmCompileOptions,
) -> String {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(program, &registry, false, "tcl9.0");
    // Constant pool in the reserved gap so it does not hit the runtime's stack.
    let user_bytes = compile_wasm(&unit, &registry, options).to_bytes();
    run_link_bytes(runtime, tag, &user_bytes, program, query)
}

/// Compose the three modules, run them under wasmtime, and return stdout.
fn run_link_bytes(
    runtime: &Path,
    tag: &str,
    user_bytes: &[u8],
    program: &str,
    query: &str,
) -> String {
    let tmp = std::env::temp_dir();
    let user = tmp.join(format!("tcl_real_link_user_{tag}.wasm"));
    let boot = tmp.join(format!("tcl_real_link_boot_{tag}.wat"));
    std::fs::write(&user, user_bytes).expect("write user module");
    std::fs::write(&boot, bootstrap_wat(query)).expect("write bootstrap");

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
        "real link trapped for {program:?}:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("stdout is utf-8")
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

fn native_i64_add_bootstrap_wat() -> String {
    let create = CodegenAbiImportId::RuntimeCreateInterp.descriptor().name;
    let set_current = CodegenAbiImportId::RuntimeSetCurrentInterp
        .descriptor()
        .name;
    format!(
        r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "{create}" (func $create (result i32)))
  (import "tcl" "{set_current}" (func $setcur (param i32)))
  (import "user" "::top" (func $top))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $interp i32)
    (local.set $interp (call $create))
    (call $setcur (local.get $interp))
    (call $top)))"#,
    )
}

fn run_real_native_i64_add(runtime: &Path, program: &str) -> String {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(program, &registry, false, "tcl9.0");
    let mut output = compile_wasm(&unit, &registry, native_i64_add_options());
    assert!(
        matches!(output.plan, WasmCodegenPlan::NativeI64Add { .. }),
        "native proof did not select: {:?}",
        output.plan
    );
    let tmp = std::env::temp_dir();
    let user = tmp.join("tcl_real_native_i64_user.wasm");
    let boot = tmp.join("tcl_real_native_i64_boot.wat");
    std::fs::write(&user, output.to_bytes()).expect("write native i64 user module");
    std::fs::write(&boot, native_i64_add_bootstrap_wat()).expect("write native i64 bootstrap");
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
        "native i64 link trapped for {program:?}:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("stdout is utf-8")
}

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

    let tmp = std::env::temp_dir();
    let user = tmp.join("tcl_real_generic_user.wasm");
    let boot = tmp.join("tcl_real_generic_boot.wat");
    std::fs::write(&user, user_bytes).expect("write generic user module");
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

    let tmp = std::env::temp_dir();
    let user = tmp.join("tcl_real_guarded_user.wasm");
    let boot = tmp.join("tcl_real_guarded_boot.wat");
    std::fs::write(&user, user_bytes).expect("write guarded user module");
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

fn run_real_native_wide_int_abi(runtime: &Path, value: i64) -> String {
    let tmp = std::env::temp_dir();
    let boot = tmp.join("tcl_real_native_wide_int_boot.wat");
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

/// Emitted modules linked against the real runtime produce real side effects in
/// the live interpreter — observed by reading state back through the same interp.
#[test]
fn emitted_modules_run_against_the_real_runtime() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping the real link");
        return;
    }
    let Some(runtime) = build_reserved_runtime() else {
        eprintln!("could not build the wasm32 runtime; skipping the real link");
        return;
    };

    // (program run by `::top`, the query the bootstrap then evaluates, expected).
    let cases = [
        // A bare variable set, read back.
        ("set x 42\n", "set x", "42"),
        // A proc defined and called: frame push + parameter binding + `return` +
        // string interpolation + command substitution — all pure, so it runs on
        // the wasm build (no `expr`/numeric tower, no host capabilities).
        (
            "proc greet {name} {return \"hi $name\"}\nset r [greet world]\n",
            "set r",
            "hi world",
        ),
        (
            "proc add {b c} {\n    return [expr {$b + $c}]\n}\n\nset e 2\nset f 4\nputs [add $e $f]\n",
            "set e",
            "6\n2",
        ),
    ];
    for (index, (program, query, expected)) in cases.into_iter().enumerate() {
        assert_eq!(
            run_real_link(&runtime, &format!("mixed{index}"), program, query),
            expected,
            "program {program:?}, query {query:?}"
        );
    }
}

/// The general tier's compiled prebuilt-argv path, running in the **real**
/// interpreter: each case's words are evaluated by the emitted module and
/// dispatched through the runtime's ordinary command resolution, and the side
/// effect is read back through the same interpreter afterwards.
#[test]
fn compiled_argv_invocations_run_against_the_real_runtime() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping the real link");
        return;
    }
    let Some(runtime) = build_reserved_runtime() else {
        eprintln!("could not build the wasm32 runtime; skipping the real link");
        return;
    };

    // (program run by `::top`, the query the bootstrap then evaluates, expected).
    let cases = [
        // Literal words only: a compiled argv reaches the real dispatcher.
        (
            "lappend out alpha\nlappend out beta\n",
            "set out",
            "alpha beta",
        ),
        // A braced word keeps its unsubstituted value; the runtime's `catch`
        // evaluates it, so a contained error stays contained.
        (
            "catch {error boom} msg\nlappend out $msg\n",
            "set out",
            "boom",
        ),
        // A `$var` scalar read feeding a `[nested command]` word.
        (
            "set greeting world\nlappend log [string toupper $greeting]\n",
            "set log",
            "WORLD",
        ),
        // A quoted compound word alongside a nested invocation.
        (
            "set name world\nlappend out \"hi $name\" [string length $name]\n",
            "set out",
            "{hi world} 5",
        ),
        // A `$arr(key)` element read, with the array built by a compiled argv.
        (
            "array set cfg {mode fast}\nlappend out $cfg(mode)\n",
            "set out",
            "fast",
        ),
        // Runtime dispatch is genuinely runtime dispatch: a renamed command
        // resolves to its new name, which no compile-time binding could know.
        (
            "proc original {x} {return got:$x}\nrename original renamed\nlappend out [renamed value]\n",
            "set out",
            "got:value",
        ),
        // Abrupt completion mid-script: the second statement errors, so the
        // third never runs — proving the compiled path propagates the code
        // after it has released its words and freed its frame.
        (
            "lappend out first\nstring index\nlappend out second\n",
            "set out",
            "first",
        ),
    ];
    for (index, (program, query, expected)) in cases.into_iter().enumerate() {
        assert_eq!(
            run_real_link_argv(&runtime, &format!("argv{index}"), program, query),
            expected,
            "program {program:?}, query {query:?}"
        );
    }
}

/// The compiled path leaks nothing in the real runtime: after repeated runs the
/// residual allocation count is unchanged, no double free was recorded, and no
/// transient call frame is outstanding — including for a statement that leaves
/// through the abrupt-completion branch.
#[test]
fn compiled_argv_balances_allocations_in_the_real_runtime() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping the real link");
        return;
    }
    let Some(runtime) = build_reserved_runtime() else {
        eprintln!("could not build the wasm32 runtime; skipping the real link");
        return;
    };

    for (tag, program) in [
        // Every word shape, all side-effect free so repeated runs are neutral.
        (
            "words",
            "set greeting hello\nset cfg(mode) fast\nstring toupper $greeting\n\
             string length [string trim {  padded  }]\nstring equal \"hi $greeting\" hi\n\
             lindex {a b c} 1\n",
        ),
        // A missing variable aborts part-way through argv construction: the
        // words already built must still be released.
        (
            "abort-mid-word",
            "string toupper [string tolower $definitely_missing_variable]\nstring length abc\n",
        ),
        // The invocation itself completes abruptly inside a loop, so the
        // completion dispatch branches out after the cleanup runs.
        (
            "abort-in-loop",
            "set go 1\nwhile {$go} {string index\nlappend never x}\nstring length abc\n",
        ),
    ] {
        let tmp = std::env::temp_dir();
        let user = tmp.join(format!("tcl_real_leak_user_{tag}.wasm"));
        let boot = tmp.join(format!("tcl_real_leak_boot_{tag}.wat"));
        // The frame-alloc requirement is what keeps this honest: under source
        // evaluation no transient frame is ever allocated, so the outstanding
        // count the bootstrap checks would balance at zero and the case would
        // pass while proving nothing about the compiled path's ownership.
        std::fs::write(
            &user,
            compile_argv_analysed(
                program,
                &["tcl_invoke_argv", "tcl_codegen_call_frame_alloc"],
            ),
        )
        .expect("write user module");
        std::fs::write(&boot, leak_bootstrap_wat()).expect("write bootstrap");
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
            "`{tag}` did not balance its allocations:\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }
}

/// The canonical semantic plan invokes prebuilt argv in the real runtime.
#[test]
fn canonical_generic_argv_runs_against_the_real_runtime() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping the real link");
        return;
    }
    let Some(runtime) = build_reserved_runtime() else {
        eprintln!("could not build the wasm32 runtime; skipping the real link");
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
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping the real link");
        return;
    }
    let Some(runtime) = build_reserved_runtime() else {
        eprintln!("could not build the wasm32 runtime; skipping the real link");
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
    ];
    for (name, program, setup, expected) in cases {
        assert_eq!(
            run_real_guarded_intrinsic_invoke(&runtime, program, 0, setup),
            expected,
            "{name}"
        );
    }
}

/// The shared descriptor links an i64 WASM value to an owned Tcl wide integer,
/// whose normal Tcl string result remains exact at the signed range boundary.
#[test]
fn native_wide_int_codegen_abi_links_against_the_real_runtime() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping the real link");
        return;
    }
    let Some(runtime) = build_reserved_runtime() else {
        eprintln!("could not build the wasm32 runtime; skipping the real link");
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
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping the real link");
        return;
    }
    let Some(runtime) = build_reserved_runtime() else {
        eprintln!("could not build the wasm32 runtime; skipping the real link");
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
}
