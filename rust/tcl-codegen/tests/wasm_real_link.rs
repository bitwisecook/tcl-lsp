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
//! # Running it — and why the skip is loud
//!
//! Heavy + gated: it builds `runtime/rust` to wasm (cached after the first
//! run) and shells out to the `wasmtime` CLI. Four things must be present:
//! the `wasmtime` CLI, the `wasm32-wasip1` Rust target, wasi-sdk, and the
//! libtommath source. [`common::wasm_link::missing_requirements`] enumerates **all** of them that
//! are absent, each with its install command.
//!
//! Set **`TCL_REQUIRE_WASM_LINK=1`** in any environment that is supposed to
//! have the whole toolchain — above all CI — and a missing dimension becomes a
//! failure instead of a skip. This is not optional ceremony: without it, a CI
//! job that installs none of the four toolchain pieces gets all eight tests
//! here reporting green while linking nothing at all. A test that cannot fail
//! is worse than no test, because it is counted.
//!
//! The gate, the reserved-runtime build and the scratch paths live in
//! `tests/common/wasm_link.rs`, shared with `wasm_tiers.rs` — one copy, so the
//! two suites cannot drift into linking different runtimes.
//!
//! The libtommath dimension is the subtle one. `runtime/rust`'s `build.rs`
//! does not fail without it; it prints a `cargo:warning` and builds with the
//! bignum backend disabled. `expr {$b + $c}` then fails *inside* the linked
//! module, `puts` prints nothing, and the assertion reads `"2"` against
//! `"6\n2"` — indistinguishable from a codegen regression. So it is checked
//! up front and passed explicitly, never left to the build script's fallback.
//!
//! # Hermetic across checkouts and processes
//!
//! The reserved runtime builds into [`common::wasm_link::reserved_runtime_target_dir`] — inside
//! *this* checkout's `target/`, never a machine-global `/tmp` name — and every
//! transient module/bootstrap goes through [`common::wasm_link::scratch`], which is keyed by both
//! checkout and pid. Sharing either was not a tidiness problem: a suite could
//! link a `tcl_runtime.wasm` built from a different checkout's source, and
//! when that stale runtime lacks the current tree's host wiring `puts` is
//! silently dropped into a hostless `StdIo` (`runtime/rust`'s
//! `cmd_chan.rs:474`). The module still runs and still exits 0, so the failure
//! surfaces only as `"2"` against `"6\n2"`, indistinguishable from a `puts`
//! leak in the runtime or a capture bug in the harness. See AGENTS.md on
//! never sharing a target dir across worktrees.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

use tcl_codegen::codegen::wasm::{
    SemanticOptimisationPassId, WasmCodegenPlan, WasmCompileOptions, compile_wasm,
};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;
use tcl_runtime_api::codegen_abi::CodegenAbiImportId;

#[path = "../../tcl-compiler/tests/common/mod.rs"]
mod common;
#[path = "common/native_whole_program.rs"]
mod native_whole_program;
use native_whole_program::native_whole_program_fixture;

use common::wasm_link::{LinkRefusal, check_manifest, real_link_runtime, scratch};
use tcl_runtime_api::codegen_abi::WASM32_FUNCTION_TABLE_IMPORT;

#[path = "wasm_real_link/argv_invocations.rs"]
mod argv_invocations;
#[path = "wasm_real_link/extension_commands.rs"]
mod extension_commands;
#[path = "wasm_real_link/linked_modules.rs"]
mod linked_modules;
#[path = "wasm_real_link/native_integers.rs"]
mod native_integers;
#[path = "wasm_real_link/native_procedures.rs"]
mod native_procedures;
#[path = "wasm_real_link/original_definitions.rs"]
mod original_definitions;
#[path = "wasm_real_link/semantic_invoke.rs"]
mod semantic_invoke;

/// Write an emitted module for linking, refusing first one whose manifest
/// disagrees with the runtime's ABI or intrinsic table
/// ([`check_manifest`]): every module this suite links is one the runtime
/// agrees it was emitted for.
fn write_linked(runtime: &Path, path: &Path, module: &[u8], what: &str) {
    check_manifest(runtime, module)
        .unwrap_or_else(|refusal| panic!("{what} was not linked: {refusal:?}"));
    std::fs::write(path, module).expect("write user module");
}

/// The bootstrap WASI command (see the module docs): create + select an interp,
/// run the emitted `::top`, then evaluate `query` against the same interp and
/// print its result string. Reserved-gap scratch: the `query` string at
/// `0x180000`, the `Tcl_GetStringFromObj` length-out at `0x190000`, and the
/// `fd_write` iovec/result at `0x190008`/`0x190010` — all above the emitted
/// module's data at `0x100000` and below the runtime's data at `0x200000`.
/// `query` must be WAT-literal-safe (no `"` or `\`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TopAbi {
    Void,
    Completion,
}

fn top_abi(output: &tcl_codegen::codegen::wasm::WasmCompilation) -> TopAbi {
    let expected = match &output.plan {
        WasmCodegenPlan::General { .. } => TopAbi::Void,
        WasmCodegenPlan::GenericInvoke { .. } => TopAbi::Completion,
        other @ WasmCodegenPlan::NativeI64Add { .. } => {
            panic!("unexpected hosted entry plan: {other:?}")
        }
    };
    let tops = output
        .module
        .functions
        .iter()
        .filter(|function| function.name == "::top")
        .collect::<Vec<_>>();
    assert_eq!(tops.len(), 1, "hosted plan must define exactly one ::top");
    let top = tops[0];
    assert!(top.exported, "hosted ::top must be exported");
    assert!(top.params.is_empty(), "::top takes no arguments");
    match expected {
        TopAbi::Void => assert!(top.results.is_empty(), "general ::top must return void"),
        TopAbi::Completion => assert!(
            top.results.len() == 3 && top.results.iter().all(|result| result.wat_name() == "i32"),
            "semantic ::top must return code, result and options"
        ),
    }
    expected
}

fn bootstrap_wat(query: &str, top_abi: TopAbi) -> String {
    let (top_results, run_top) = match top_abi {
        TopAbi::Void => ("", "    (call $top)"),
        TopAbi::Completion => (
            " (result i32 i32 i32)",
            "    (call $top)\n    (local.set $top_options)\n    (local.set $top_result)\n    (local.set $top_code)\n    (call $rel (local.get $top_result))\n    (call $rel (local.get $top_options))\n    (if (i32.ne (local.get $top_code) (i32.const 0)) (then unreachable))",
        ),
    };
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
  (import "user" "::top" (func $top{top_results}))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $interp i32) (local $result i32) (local $strptr i32) (local $len i32)
    (local $top_code i32) (local $top_result i32) (local $top_options i32)
    (local.set $interp (call $create))
    (call $setcur (local.get $interp))
{run_top}
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
    run_link_bytes_with_top_abi(runtime, tag, user_bytes, program, query, TopAbi::Void)
}

fn run_link_bytes_with_top_abi(
    runtime: &Path,
    tag: &str,
    user_bytes: &[u8],
    program: &str,
    query: &str,
    top_abi: TopAbi,
) -> String {
    let user = scratch(&format!("tcl_real_link_user_{tag}.wasm"));
    let boot = scratch(&format!("tcl_real_link_boot_{tag}.wat"));
    write_linked(runtime, &user, user_bytes, program);
    std::fs::write(&boot, bootstrap_wat(query, top_abi)).expect("write bootstrap");

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

/// Run a bootstrap against the runtime alone — no emitted `user` module.
fn run_boot_only(runtime: &Path, tag: &str, wat: &str) -> Result<String, String> {
    let boot = scratch(&format!("tcl_real_link_boot_{tag}.wat"));
    std::fs::write(&boot, wat).expect("write bootstrap");
    let depth_diagnostic =
        std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref() == Ok("first-decline-v1");
    let out = Command::new("wasmtime")
        .arg("run")
        .args(if depth_diagnostic {
            &[
                "--env",
                "TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC=first-decline-v1",
            ][..]
        } else {
            &[][..]
        })
        .arg("--preload")
        .arg(format!("tcl={}", runtime.display()))
        .arg(&boot)
        .output()
        .expect("run wasmtime");
    let _ = std::fs::remove_file(&boot);
    if out.status.success() {
        if depth_diagnostic {
            report_native_depth_ledger(tag, &out.stderr);
        }
        Ok(String::from_utf8(out.stdout).expect("stdout is utf-8"))
    } else {
        Err(format!(
            "--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        ))
    }
}

/// Like [`bootstrap_wat`], plus the one thing a behavioural assertion cannot
/// see: how many procedure bodies actually ran through a native entry.
///
/// The compiled body and the source body of a procedure produce the same Tcl
/// answer by construction, so without this counter a binding that silently
/// stopped taking effect would keep every result assertion green. Below
/// `minimum` the module traps instead of printing, so the case fails loudly
/// rather than quietly proving nothing.
fn native_dispatch_bootstrap_wat(query: &str, minimum: i32) -> String {
    let create = CodegenAbiImportId::RuntimeCreateInterp.descriptor().name;
    let set_current = CodegenAbiImportId::RuntimeSetCurrentInterp
        .descriptor()
        .name;
    let object_new_string = CodegenAbiImportId::ObjectNewString.descriptor().name;
    let object_release = CodegenAbiImportId::ObjectRelease.descriptor().name;
    let dispatches = CodegenAbiImportId::NativeProcDispatches.descriptor().name;
    format!(
        r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "{create}" (func $create (result i32)))
  (import "tcl" "{set_current}" (func $setcur (param i32)))
  (import "tcl" "{object_new_string}" (func $box (param i32 i32) (result i32)))
  (import "tcl" "tcl_eval" (func $eval (param i32) (result i32)))
  (import "tcl" "{object_release}" (func $rel (param i32)))
  (import "tcl" "{dispatches}" (func $dispatches (result i32)))
  (import "tcl" "Tcl_GetStringFromObj" (func $getstr (param i32 i32) (result i32)))
  (import "user" "::top" (func $top))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $interp i32) (local $result i32) (local $strptr i32) (local $len i32)
    (local.set $interp (call $create))
    (call $setcur (local.get $interp))
    (call $top)
    (if (i32.lt_s (call $dispatches) (i32.const {minimum})) (then unreachable))
    (local.set $result (call $eval (call $box (i32.const 0x180000) (i32.const {qlen}))))
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

/// Observational admission report before the unchanged compiled-path assertions.
fn report_actual_native_admission(
    program: &str,
    unit: &CompilationUnit,
    output: &tcl_codegen::codegen::wasm::WasmCompilation,
) {
    use tcl_compiler::native_compilation_admission::{
        NativeCompilationAdmissionScope, script_admission_plan,
    };
    eprintln!(
        "CCG_WASM_SUMMARY program={program:?} ir_procedures={} unit_procedures={} top_executable={} native_functions={} native_enabled={} native_entry={} compiled_variable_provider={:?} compilation={:?} native_body_owners={} retained_source={} source_metadata={}",
        unit.ir_module.procedures.len(),
        unit.procedures.len(),
        unit.top_level
            .semantic_facts
            .executable()
            .function()
            .is_some(),
        output.native.functions.len(),
        output.native.enabled,
        unit.ir_module.source_entry.native_entry.is_some(),
        unit.ir_module.source_entry.compiled_variable_provider,
        unit.ir_module.source_entry.native_compilation,
        unit.ir_module.original_native_declaration_body_units.len(),
        unit.ir_module.retained_source_bindings.is_some(),
        unit.ir_module.source_metadata_input.is_some(),
    );
    eprintln!("CCG_WASM_PLAN {:?}", output.plan);
    eprintln!(
        "CCG_WASM_TOP_ADMISSION {:?}",
        script_admission_plan(
            &unit.ir_module.top_level,
            NativeCompilationAdmissionScope::Script
        )
    );
    for (id, procedure) in &unit.ir_module.procedures {
        eprintln!(
            "CCG_WASM_PROCEDURE id={id:?} name={:?} declaration={:?} body_admission={:?}",
            procedure.qualified_name,
            procedure.declaration_site,
            script_admission_plan(
                &procedure.body,
                NativeCompilationAdmissionScope::ProcedureBody
            ),
        );
    }
    for (id, report) in &output.native.functions {
        eprintln!("CCG_WASM_NATIVE id={id:?} report={report:?}");
    }
}

/// Compile under the native tier and prove, before anything runs, that the
/// module really binds `entries` compiled bodies: it imports the runtime's
/// shared table, declares exactly that many functions for `ref.func`, and
/// calls the definition ABI at least once.
fn compile_native_bound(program: &str, entries: usize) -> Vec<u8> {
    // This fixture exercises native table installation under the same genuine
    // whole-program owner as the native-i64 driver; it never executes a preload.
    let fixture = native_whole_program_fixture(program);
    let registry = fixture.context.commands();
    let unit = &fixture.unit;
    let mut output = compile_wasm(
        unit,
        registry,
        WasmCompileOptions::runtime_linked().native_tier(),
    );
    report_actual_native_admission(program, unit, &output);
    assert!(
        output.module.import_table,
        "{program:?} must import the runtime's shared function table"
    );
    assert_eq!(
        output.module.elem_declared.len(),
        entries,
        "{program:?} must declare {entries} installable bodies"
    );
    assert_eq!(
        output.module.globals.len(),
        1,
        "{program:?} must keep exactly one table-base global"
    );
    // This original Native entry requires dispatch at the live stock handler.
    // Legacy proved-dispatch modes retain the original bypass ABI.
    let define = CodegenAbiImportId::ProcDefineGuardedArgv.descriptor().name;
    let index = output
        .module
        .imports
        .iter()
        .position(|import| import.name == define)
        .expect("the native tier imports the definition ABI");
    let wat = output.module.to_wat();
    let call = format!("call {index}");
    assert!(
        wat.lines().any(|line| line.trim() == call),
        "{program:?} must call {define}; without it nothing binds an entry"
    );
    output.to_bytes()
}

/// Link a native-tier module, run it, and require `minimum` native proc
/// dispatches before the query is even evaluated.
fn run_real_native_proc(
    runtime: &Path,
    tag: &str,
    program: &str,
    entries: usize,
    query: &str,
    minimum: i32,
) -> String {
    let user = scratch(&format!("tcl_real_link_user_{tag}.wasm"));
    let boot = scratch(&format!("tcl_real_link_boot_{tag}.wat"));
    write_linked(
        runtime,
        &user,
        &compile_native_bound(program, entries),
        program,
    );
    std::fs::write(&boot, native_dispatch_bootstrap_wat(query, minimum)).expect("write bootstrap");
    let depth_diagnostic =
        std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref() == Ok("first-decline-v1");
    let out = Command::new("wasmtime")
        .arg("run")
        .args(if depth_diagnostic {
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
    if out.status.success() && depth_diagnostic {
        report_native_depth_refusals(tag, &out.stderr);
        report_native_depth_ledger(tag, &out.stderr);
    }
    assert!(
        out.status.success(),
        "the native proc link trapped for {program:?} (fewer than {minimum} \
         native dispatches, or a table install that did not take):\n--- stdout ---\n{}\n\
         --- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("stdout is utf-8")
}

fn report_native_depth_refusals(tag: &str, stderr: &[u8]) {
    let mut retained = 0;
    for line in stderr.split(|byte| *byte == b'\n') {
        if !line.starts_with(b"CCG_NATIVE_DEPTH_REFUSAL ") {
            continue;
        }
        if line.len() > 512 {
            eprintln!("CCG_NATIVE_DEPTH_EVIDENCE_REFUSED tag={tag} reason=marker-line-cap");
            return;
        }
        let record = format!(
            "CCG_NATIVE_DEPTH_GUEST tag={tag} {}",
            String::from_utf8_lossy(line)
        );
        if retained + record.len() + 1 > 4096 {
            eprintln!("CCG_NATIVE_DEPTH_EVIDENCE_REFUSED tag={tag} reason=marker-aggregate-cap");
            return;
        }
        retained += record.len() + 1;
        eprintln!("{record}");
    }
}

// Separate bounded ledger transport; the original refusal channel/caps stay intact.
fn report_native_depth_ledger(tag: &str, stderr: &[u8]) {
    if tag.len() > 96 {
        eprintln!("CCG_WASM_DEPTH_EVIDENCE_REFUSED reason=tag-cap");
        return;
    }
    let mut records = 0;
    let mut bytes = 0;
    for line in stderr.split(|byte| *byte == b'\n') {
        if !line.starts_with(b"CCG_WASM_DEPTH ") {
            continue;
        }
        if line.len() > 512 || records == 512 {
            eprintln!("CCG_WASM_DEPTH_EVIDENCE_REFUSED tag={tag} reason=record-cap");
            return;
        }
        let record = format!(
            "CCG_WASM_DEPTH_GUEST tag={tag} {}",
            String::from_utf8_lossy(line)
        );
        if bytes + record.len() + 1 > 512 * 640 {
            eprintln!("CCG_WASM_DEPTH_EVIDENCE_REFUSED tag={tag} reason=aggregate-cap");
            return;
        }
        records += 1;
        bytes += record.len() + 1;
        eprintln!("{record}");
    }
}
