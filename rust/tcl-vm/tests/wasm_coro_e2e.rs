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

//! VM coroutines execute on **wasm32** (`RUST_ISSUE_008`, phase 4).
//!
//! The bytecode VM's coroutines are pure data (a frozen `Vec<Frame>` + saved
//! flow — no OS threads, no `unsafe`), so they are wasm-portable "for free": the
//! plan's preferred path to wasm coroutines is *the VM running on wasm32*, not
//! asyncify. This test proves it end to end.
//!
//! It generates a tiny `cdylib` that links this workspace's `tcl-vm` +
//! `tcl-compiler`, compiles a few coroutine scripts and returns their integer
//! results, builds it to `wasm32-unknown-unknown`, and executes it under
//! **Node**'s `WebAssembly` API (no imports, no WASI). The expectations are the
//! same tclsh 9.0.4 oracle values the native coroutine suite pins.
//!
//! Skips cleanly when the `wasm32-unknown-unknown` target or `node` is absent
//! (mirroring `tcl-compiler/tests/wasm_real_link.rs`), so CI without a wasm
//! toolchain is unaffected.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `…/rust/tcl-vm`.
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn have_node() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn have_wasm_target() -> bool {
    Command::new("rustc")
        .args(["--print", "target-list"])
        .output()
        .is_ok_and(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|t| t == "wasm32-unknown-unknown")
        })
}

/// The generated crate's `src/lib.rs`: run coroutine scripts through the VM and
/// return their integer results as raw wasm exports.
const LIB_RS: &str = r#"
use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct Svc(CommandRegistry);
impl CompileService for Svc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(m) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(m));
        }
        let ir = lower_to_ir(src, &self.0);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.0))
    }
}

fn eval_int(script: &str) -> i32 {
    let reg = CommandRegistry::build_default();
    let ir = lower_to_ir(script, &reg);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &reg);
    let mut vm = Vm::with_output(Box::new(std::io::sink()));
    vm.set_compiler(Box::new(Svc(CommandRegistry::build_default())));
    let comp = vm.run_module(&asm);
    comp.result.to_str().trim().parse::<i32>().unwrap_or(-1)
}

// A generator yielding across a `foreach`; `[c]` command substitutions are
// yieldable, so this packs 2,3,4 into 234.
#[unsafe(no_mangle)]
pub extern "C" fn run_generator() -> i32 {
    eval_int("proc gen {} { foreach x {1 2 3 4} { yield $x } }; \
              coroutine c gen; expr {[c]*100 + [c]*10 + [c]}")
}

// The `set n [yield $sum]` resume-value idiom (yieldable command substitution).
#[unsafe(no_mangle)]
pub extern "C" fn run_accumulator() -> i32 {
    eval_int("proc acc {} { set s 0; while 1 { set n [yield $s]; incr s $n } }; \
              coroutine a acc; set r1 [a 5]; set r2 [a 10]; set r3 [a 3]; \
              expr {$r1*10000 + $r2*100 + $r3}")
}

// `coroutine … apply {lambda}` — the anonymous generator on wasm.
#[unsafe(no_mangle)]
pub extern "C" fn run_apply() -> i32 {
    eval_int("coroutine c apply {{} { foreach x {7 8 9} { yield $x } }}; \
              expr {[c]*100 + [c]}")
}
"#;

/// The Node harness: instantiate the (import-free) module and print each export.
const HARNESS_MJS: &str = r"
import { readFileSync } from 'node:fs';
const bytes = readFileSync(process.argv[2]);
const { instance } = await WebAssembly.instantiate(bytes, {});
const out = {
  generator: instance.exports.run_generator(),
  accumulator: instance.exports.run_accumulator(),
  apply: instance.exports.run_apply(),
};
console.log(JSON.stringify(out));
";

fn cargo_toml() -> String {
    let base = crate_dir();
    let dep = |name: &str| {
        let path = base.join("..").join(name);
        format!("{name} = {{ path = \"{}\" }}", path.display())
    };
    format!(
        "[package]\nname = \"tcl-vm-wasm-coro\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\
         publish = false\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\n\
         tcl-vm = {{ path = \"{}\" }}\n{}\n{}\n{}\n\n[workspace]\n\n\
         [profile.release]\npanic = \"abort\"\nopt-level = \"s\"\n",
        base.display(),
        dep("tcl-compiler"),
        dep("tcl-registry"),
        dep("tcl-bytecode"),
    )
}

#[test]
fn vm_coroutines_run_on_wasm32() {
    if !have_wasm_target() {
        eprintln!("wasm32-unknown-unknown target unavailable; skipping VM-on-wasm coroutine test");
        return;
    }
    if !have_node() {
        eprintln!("node unavailable; skipping VM-on-wasm coroutine test");
        return;
    }

    // A stable scratch dir keeps the (large) wasm dependency build cached between
    // runs; only this crate's source changing forces a rebuild.
    let dir = std::env::temp_dir().join("tcl_vm_wasm_coro");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).expect("create crate dir");
    std::fs::write(dir.join("Cargo.toml"), cargo_toml()).expect("write Cargo.toml");
    std::fs::write(src.join("lib.rs"), LIB_RS).expect("write lib.rs");
    let harness = dir.join("harness.mjs");
    std::fs::write(&harness, HARNESS_MJS).expect("write harness");

    let build = Command::new(env!("CARGO"))
        .current_dir(&dir)
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .output()
        .expect("run cargo build");
    assert!(
        build.status.success(),
        "wasm build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let wasm = dir.join("target/wasm32-unknown-unknown/release/tcl_vm_wasm_coro.wasm");
    assert!(wasm.exists(), "wasm artifact missing at {}", wasm.display());

    let out = run_node(&harness, &wasm);
    // Same values the native coroutine suite pins against tclsh 9.0.4.
    assert!(
        out.contains("\"generator\":234"),
        "generator wrong on wasm: {out}"
    );
    assert!(
        out.contains("\"accumulator\":51518"),
        "accumulator (resume-value idiom) wrong on wasm: {out}"
    );
    assert!(
        out.contains("\"apply\":809"),
        "coroutine-apply wrong on wasm: {out}"
    );
}

/// The **shipped** VM-on-wasm cdylib (`rust/tcl-vm-wasm`) — the primary wasm
/// compile target (`RUST_ISSUE_008` piece 1) — builds to wasm32 and runs coroutine
/// scripts through its `tcl_alloc`/`tcl_eval`/`tcl_dealloc` ABI under Node,
/// returning the tclsh-9.0.4 oracle values. The generated-crate test above proves
/// the raw capability; this pins the product crate, its ABI, and `verify.mjs`
/// (including a `yield`-across-`catch` case, exercised on wasm).
#[test]
fn vm_wasm_crate_runs_coroutines_via_abi() {
    if !have_wasm_target() {
        eprintln!("wasm32-unknown-unknown target unavailable; skipping tcl-vm-wasm ABI test");
        return;
    }
    if !have_node() {
        eprintln!("node unavailable; skipping tcl-vm-wasm ABI test");
        return;
    }
    let crate_path = crate_dir().join("..").join("tcl-vm-wasm");
    let manifest = crate_path.join("Cargo.toml");
    let build = Command::new(env!("CARGO"))
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "--manifest-path",
            manifest.to_str().expect("manifest path utf-8"),
        ])
        .output()
        .expect("run cargo build");
    assert!(
        build.status.success(),
        "tcl-vm-wasm build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let wasm = crate_path.join("target/wasm32-unknown-unknown/release/tcl_vm_wasm.wasm");
    assert!(wasm.exists(), "wasm artifact missing at {}", wasm.display());

    // `verify.mjs` runs a generator, the resume-value idiom, a yield-across-catch
    // case, and a yield-across-`lmap` case through the ABI, exiting non-zero on any
    // mismatch (so `run_node`'s success assertion is the real gate).
    let out = run_node(&crate_path.join("verify.mjs"), &wasm);
    assert!(out.contains("ok   generator: 234"), "verify output: {out}");
    assert!(
        out.contains("ok   catch: done:0: foo"),
        "verify output: {out}"
    );
    assert!(
        out.contains("ok   lmap: 2 3 {A B C}"),
        "verify output: {out}"
    );
    assert!(out.contains("ok   subst: 2 a=Pb=Q"), "verify output: {out}");
}

fn run_node(harness: &Path, wasm: &Path) -> String {
    let out = Command::new("node")
        .arg(harness)
        .arg(wasm)
        .output()
        .expect("run node");
    assert!(
        out.status.success(),
        "node failed:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("node stdout utf-8")
}
