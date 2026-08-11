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

//! WASM-runnability arm: compile a generated program to the eval-fallback WASM
//! module and run it under `wasmtime`, flagging codegen panics / errors and
//! modules that fail to instantiate or trap.
//!
//! This is the **tractable slice** of the WASM third differential arm. The
//! general lowering boxes every unsupported leaf command as a `call` to an imported
//! `tcl_eval_code`; without an interpreter-backed host the module cannot
//! *evaluate* Tcl, so a *value* differential against `tclsh` is not possible
//! here. What
//! **is** possible — and valuable — is exercising the WASM **codegen** over the
//! fuzzer's randomised programs: a generated program that makes the canonical
//! `compile_wasm` pipeline panic, emits a module that `wasmtime` rejects, or traps
//! at run time, is a real codegen bug. We satisfy the module's `tcl_*` imports
//! with the same tiny host stub the `tcl-compiler` end-to-end execution test
//! uses (`tcl_expr_bool` returns `0`, so control flow terminates), invoke
//! `::top`, and treat a clean exit as a pass.
//!
//! Without an interpreter-backed host this arm performs only a structural
//! codegen check; swapping the stub for a real host turns it into a true value
//! differential.

use std::path::Path;
use std::process::Command;

use tcl_compiler::codegen::wasm::{
    ValType, WasmCompileOptions, WasmFunction, WasmInstruction, WasmModule, WasmOp, compile_wasm,
};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

/// The outcome of running one generated program through the WASM arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmVerdict {
    /// The module compiled, instantiated, and `::top` ran to completion.
    Ran,
    /// Lowering or WASM codegen panicked / errored — a codegen bug. Carries a
    /// short reason.
    CodegenFailed(String),
    /// The module compiled but `wasmtime` rejected or trapped it — a malformed
    /// or unsound emission. Carries the captured stderr (truncated).
    Trapped(String),
    /// `wasmtime` is not installed — the arm is skipped, not a finding.
    Unavailable,
}

/// Is the `wasmtime` CLI present?
#[must_use]
pub fn have_wasmtime() -> bool {
    Command::new("wasmtime")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Compile `src` to the eval-fallback WASM module bytes, catching a codegen
/// panic as an `Err` so a single bad program can't wedge the campaign.
fn compile_to_wasm(src: &str) -> Result<Vec<u8>, String> {
    let src = src.to_owned();
    let result = std::panic::catch_unwind(move || {
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for(&src, &registry, false);
        compile_wasm(
            &unit,
            &registry,
            WasmCompileOptions::hosted()
                .for_eval_only_test_host()
                .with_data_base(0),
        )
        .to_bytes()
    });
    result.map_err(|e| {
        e.downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| e.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "codegen panicked".to_owned())
    })
}

/// Build the host stub: a module defining + exporting `memory` and the three
/// `tcl_*` functions the emitted module imports, with trivial bodies. Mirrors
/// `tcl-compiler/tests/wasm_execute.rs::host_stub`. `tcl_eval_code` returns `0`
/// (`ok`, so the emitted completion-code dispatch falls through) and
/// `tcl_expr_bool` returns `0` (false), so every condition is false and all
/// control flow terminates.
fn host_stub_bytes() -> Vec<u8> {
    let i32_const_0 = || WasmInstruction::with_operands(WasmOp::I32Const, vec![0x00]);
    let func =
        |name: &str, params: Vec<ValType>, results: Vec<ValType>, body: Vec<WasmInstruction>| {
            WasmFunction {
                name: name.to_owned(),
                params,
                results,
                locals: Vec::new(),
                body,
                local_names: Vec::new(),
                exported: true,
                source_range: None,
                kind: "host".to_owned(),
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
            "tcl_eval_code",
            vec![ValType::I32],
            vec![ValType::I32],
            vec![i32_const_0()],
        ),
        func(
            "tcl_expr_bool",
            vec![ValType::I32],
            vec![ValType::I32],
            vec![i32_const_0()],
        ),
    ];
    m.to_bytes()
}

/// Compile `src` to WASM and run it under `wasmtime` with the host stub
/// preloaded, returning the verdict. `scratch` is a directory for the temp
/// module files; `tag` disambiguates concurrent writes.
#[must_use]
pub fn check(src: &str, scratch: &Path, tag: u64) -> WasmVerdict {
    if !have_wasmtime() {
        return WasmVerdict::Unavailable;
    }
    let user_bytes = match compile_to_wasm(src) {
        Ok(b) => b,
        Err(reason) => return WasmVerdict::CodegenFailed(reason),
    };
    let host = scratch.join(format!("wasm-host-{tag}.wasm"));
    let user = scratch.join(format!("wasm-user-{tag}.wasm"));
    if std::fs::write(&host, host_stub_bytes()).is_err()
        || std::fs::write(&user, &user_bytes).is_err()
    {
        return WasmVerdict::Unavailable;
    }
    let out = Command::new("wasmtime")
        .arg("run")
        .arg("--preload")
        .arg(format!("tcl={}", host.display()))
        .arg("--invoke")
        .arg("::top")
        .arg(&user)
        .output();
    let _ = std::fs::remove_file(&host);
    let _ = std::fs::remove_file(&user);
    match out {
        Ok(o) if o.status.success() => WasmVerdict::Ran,
        Ok(o) => {
            let mut stderr = String::from_utf8_lossy(&o.stderr).into_owned();
            stderr.truncate(400);
            WasmVerdict::Trapped(stderr)
        }
        Err(_) => WasmVerdict::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_stub_is_valid_wasm() {
        // The stub must be a structurally valid module (it starts the magic).
        let bytes = host_stub_bytes();
        assert_eq!(&bytes[0..4], b"\0asm", "WASM magic");
    }

    #[test]
    fn simple_program_compiles_and_runs() {
        // A plain program must reach `Ran` (or skip if wasmtime is absent) —
        // never a codegen failure.
        let scratch = std::env::temp_dir();
        let v = check("set x 1\nputs $x\n", &scratch, 9_000_001);
        assert!(
            matches!(v, WasmVerdict::Ran | WasmVerdict::Unavailable),
            "unexpected verdict for a trivial program: {v:?}"
        );
    }

    #[test]
    fn control_flow_program_runs() {
        let scratch = std::env::temp_dir();
        let src =
            "set a 0\nif {$a < 3} { puts low } else { puts high }\nforeach i {1 2} { puts $i }\n";
        let v = check(src, &scratch, 9_000_002);
        assert!(
            matches!(v, WasmVerdict::Ran | WasmVerdict::Unavailable),
            "control-flow program should run cleanly: {v:?}"
        );
    }
}
