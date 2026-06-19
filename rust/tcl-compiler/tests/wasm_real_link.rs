//! **The real whole-program link** — an emitted WASM module runs against the
//! *actual* `tcl_runtime.wasm`, not a stub.
//!
//! `wasm_execute.rs` runs emitted modules against a hand-built host stub. This
//! goes all the way: it links the emitted module against the real Rust runtime
//! compiled to `wasm32-unknown-unknown`, sharing one linear memory, and proves a
//! genuine side effect of the emitted program survives in the real interpreter.
//!
//! Three modules, composed by the wasmtime CLI (`--preload`), sharing the
//! runtime's exported memory:
//!
//! 1. **`tcl` = the real runtime** — exports `memory` + the codegen ABI
//!    (`tcl_obj_new_string`/`tcl_eval`/`tcl_obj_release`/`tcl_expr_bool`,
//!    `tcl_runtime_*_interp`) and the C ABI (`Tcl_GetStringFromObj`). Built with
//!    `--global-base=0x200000` so its data/heap sit above a reserved gap
//!    `[0x100000, 0x200000)` (its 1 MiB shadow stack stays at `[0, 0x100000)`).
//! 2. **`user` = the emitted module** — its `::top` boxes `set x 42` and
//!    `tcl_eval`s it. Its constant pool is relocated to [`RESERVED_DATA_BASE`]
//!    (`0x100000`) so it lands in the gap, not under the runtime's stack.
//! 3. **bootstrap** (the WASI command) — creates an interp, makes it current,
//!    calls `user::top` (running `set x 42`), then `tcl_eval`s `set x` against the
//!    *same* interp, reads the result string with `Tcl_GetStringFromObj`, and
//!    writes it to stdout. Stdout `42` proves the emitted program's side effect
//!    persisted in the real runtime.
//!
//! Heavy + gated: it builds `runtime/rust` to wasm (cached after the first run)
//! and shells out to the `wasmtime` CLI. It skips cleanly when either is
//! unavailable, mirroring the other wasm tests. `set x` needs no `expr` (the
//! numeric tower is off in the wasm build) and no host capabilities, so it runs
//! faithfully on the placeholder `BrowserHost`.

use std::path::PathBuf;
use std::process::Command;

use tcl_compiler::codegen::wasm::{RESERVED_DATA_BASE, wasm_codegen_module_based};
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

/// The bootstrap WASI command (see the module docs). Reserved-gap scratch: the
/// query string `set x` at `0x180000`, the `Tcl_GetStringFromObj` length-out at
/// `0x190000`, and the `fd_write` iovec/result at `0x190008`/`0x190010` — all
/// above the emitted module's data at `0x100000` and below the runtime's data at
/// `0x200000`.
const BOOTSTRAP_WAT: &str = r#"(module
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
    (call $top)                                                                   ;; run "set x 42"
    (local.set $result (call $eval (call $box (i32.const 0x180000) (i32.const 5)))) ;; eval "set x"
    (local.set $strptr (call $getstr (local.get $result) (i32.const 0x190000)))
    (local.set $len (i32.load (i32.const 0x190000)))
    (i32.store (i32.const 0x190008) (local.get $strptr))
    (i32.store (i32.const 0x19000C) (local.get $len))
    (drop (call $fd_write (i32.const 1) (i32.const 0x190008) (i32.const 1) (i32.const 0x190010)))
    (call $rel (local.get $result)))
  (data (i32.const 0x180000) "set x"))
"#;

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

/// Build `runtime/rust` to `wasm32-unknown-unknown` with the reserved-region
/// linker flag, into an isolated target dir. Returns the artifact path, or `None`
/// if the build can't run (e.g. the wasm32 target is missing) so the test skips.
fn build_reserved_runtime() -> Option<PathBuf> {
    let root = workspace_root();
    let target_dir = std::env::temp_dir().join("tcl_reserved_runtime");
    let ok = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(root.join("runtime/rust/Cargo.toml"))
        .args(["--target", "wasm32-unknown-unknown"])
        .arg("--target-dir")
        .arg(&target_dir)
        // Reserve [0x100000, 0x200000): the 1 MiB shadow stack stays at
        // [0, 0x100000), data/heap move to >= 0x200000, leaving the gap free for
        // the emitted constant pool.
        .env("RUSTFLAGS", "-C link-arg=--global-base=2097152")
        .status()
        .is_ok_and(|s| s.success());
    ok.then(|| target_dir.join("wasm32-unknown-unknown/debug/tcl_runtime.wasm"))
}

/// The emitted `set x 42` module, linked against the real runtime, sets `x` in
/// the live interpreter — observed by reading it back through the same interp.
#[test]
fn emitted_module_runs_against_the_real_runtime() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping the real link");
        return;
    }
    let Some(runtime) = build_reserved_runtime() else {
        eprintln!("could not build the wasm32 runtime; skipping the real link");
        return;
    };

    // Emit the user module with its constant pool in the reserved gap.
    let src = "set x 42\n";
    let registry = CommandRegistry::build_default();
    let module = lower_to_ir(src, &registry);
    let user_bytes = wasm_codegen_module_based(&module, src, RESERVED_DATA_BASE).to_bytes();

    let tmp = std::env::temp_dir();
    let user = tmp.join("tcl_real_link_user.wasm");
    let boot = tmp.join("tcl_real_link_boot.wat");
    std::fs::write(&user, &user_bytes).expect("write user module");
    std::fs::write(&boot, BOOTSTRAP_WAT).expect("write bootstrap");

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
        "real link trapped:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "42",
        "the emitted `set x 42` did not take effect in the real runtime"
    );
}
