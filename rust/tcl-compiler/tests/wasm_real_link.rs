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
//! and the direct numeric `add` procedure. The final case proves the linked
//! generated function binds Tcl-visible parameters, adds through the numeric
//! tower, and writes `6` through the real WASI stdout path.
//!
//! Heavy + gated: it builds `runtime/rust` to wasm (cached after the first run)
//! and shells out to the `wasmtime` CLI. It skips cleanly when either is
//! unavailable, mirroring the other wasm tests.

use std::path::{Path, PathBuf};
use std::process::Command;

use tcl_compiler::codegen::wasm::{WasmCodegenPlan, WasmCompileOptions, compile_wasm};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

/// The bootstrap WASI command (see the module docs): create + select an interp,
/// run the emitted `::top`, then evaluate `query` against the same interp and
/// print its result string. Reserved-gap scratch: the `query` string at
/// `0x180000`, the `Tcl_GetStringFromObj` length-out at `0x190000`, and the
/// `fd_write` iovec/result at `0x190008`/`0x190010` — all above the emitted
/// module's data at `0x100000` and below the runtime's data at `0x200000`.
/// `query` must be WAT-literal-safe (no `"` or `\`).
fn bootstrap_wat(query: &str) -> String {
    format!(
        r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "tcl_runtime_create_interp" (func $create (result i32)))
  (import "tcl" "tcl_runtime_set_current_interp" (func $setcur (param i32)))
  (import "tcl" "tcl_obj_new_string" (func $box (param i32 i32) (result i32)))
  (import "tcl" "tcl_eval" (func $eval (param i32) (result i32)))
  (import "tcl" "tcl_obj_release" (func $rel (param i32)))
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
    format!(
        r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "tcl_runtime_create_interp" (func $create (result i32)))
  (import "tcl" "tcl_runtime_set_current_interp" (func $setcur (param i32)))
  (import "tcl" "tcl_obj_release" (func $rel (param i32)))
  (import "tcl" "Tcl_GetStringFromObj" (func $getstr (param i32 i32) (result i32)))
  (import "tcl" "tcl_codegen_call_frame_outstanding" (func $frames (result i32)))
  (import "user" "::top" (func $top (result i32 i32 i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $interp i32) (local $code i32) (local $result i32) (local $options i32)
    (local $strptr i32) (local $len i32)
    (local.set $interp (call $create))
    (call $setcur (local.get $interp))
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
)"#,
    )
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
fn run_real_link(runtime: &Path, program: &str, query: &str) -> String {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(program, &registry, false, "tcl9.0");
    // Constant pool in the reserved gap so it does not hit the runtime's stack.
    let user_bytes =
        compile_wasm(&unit, &registry, WasmCompileOptions::runtime_linked()).to_bytes();

    let tmp = std::env::temp_dir();
    let user = tmp.join("tcl_real_link_user.wasm");
    let boot = tmp.join("tcl_real_link_boot.wat");
    std::fs::write(&user, &user_bytes).expect("write user module");
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
    for (program, query, expected) in cases {
        assert_eq!(
            run_real_link(&runtime, program, query),
            expected,
            "program {program:?}, query {query:?}"
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
