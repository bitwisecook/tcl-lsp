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

//! Differential execution through a *real linked* WASM runtime.
//!
//! The in-process [`crate::wasm_diff`] arm intentionally substitutes a
//! `tcl-vm` host to isolate generated control flow. This module exercises the
//! independent deployment path: an emitted user module is linked to the Rust
//! `tcl_runtime.wasm`, then run as a WASI command by `wasmtime`. A bootstrap
//! creates the interpreter, invokes the emitted `::top`, and queries the same
//! interpreter. C Tcl 9 executes the identical source and query as the oracle.
//!
//! The generated surface is deliberately narrow: pure `proc`, parameter
//! binding, `return`, interpolation, and command substitution. These are the
//! documented capabilities of the browser runtime build and make an observed
//! mismatch attributable to the link/runtime path rather than an unsupported
//! host service. Broader generated Tcl remains covered by the native and
//! in-process WASM arms.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use tcl_compiler::codegen::wasm::{RESERVED_DATA_BASE, wasm_codegen_module_based};
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

use crate::harness::{Outcome, Verdict, compare_outcomes, run_backend, write_script};

/// One safe program generated for linked-runtime execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    name: String,
}

impl Case {
    /// Derive a Tcl identifier-safe input from a deterministic seed.
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        Self {
            name: format!("n{:016x}", seed),
        }
    }

    /// Tcl source evaluated by C Tcl and compiled into the user WASM module.
    #[must_use]
    pub fn program(&self) -> String {
        format!(
            "proc greet {{name}} {{return \"hi $name\"}}\nset r [greet {}]\n",
            self.name
        )
    }

    /// The final query used identically after each run.
    const QUERY: &'static str = "set r";

    /// The C Tcl 9 source adds only an output observation after the program.
    #[must_use]
    pub fn oracle_source(&self) -> String {
        format!("{}puts -nonewline [{}]\n", self.program(), Self::QUERY)
    }
}

/// Result from one linked-runtime differential execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkedVerdict {
    /// C Tcl 9 and the real linked runtime agree.
    Match,
    /// Both commands ran, but their normalised harness outcomes differ.
    Divergence {
        /// Captured output / status from C Tcl 9.
        oracle: Outcome,
        /// Captured output / status from the linked WASM command.
        wasm: Outcome,
    },
    /// Linking, compilation, or command invocation failed before a comparison.
    Unrunnable(String),
}

/// The workspace root (`CARGO_MANIFEST_DIR` is `…/rust/tcl-fuzz`).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// Verify the CLI is available before launching a costly runtime build.
pub fn have_wasmtime() -> bool {
    Command::new("wasmtime")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Build the actual Rust runtime into an isolated target directory.
///
/// The global base reserves `[0x100000, 0x200000)` for emitted user-module
/// data: the runtime's shadow stack stays below it and its own data/heap starts
/// above it. This is the same link contract the compiler's real-link test uses.
pub fn build_runtime(scratch: &Path) -> Result<PathBuf, String> {
    let target_dir = scratch.join("runtime-target");
    let output = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(workspace_root().join("runtime/rust/Cargo.toml"))
        .args(["--target", "wasm32-unknown-unknown"])
        .arg("--target-dir")
        .arg(&target_dir)
        .env("RUSTFLAGS", "-C link-arg=--global-base=2097152")
        .output()
        .map_err(|error| format!("could not start runtime WASM build: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "runtime WASM build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let artifact = target_dir.join("wasm32-unknown-unknown/debug/tcl_runtime.wasm");
    if artifact.is_file() {
        Ok(artifact)
    } else {
        Err(format!(
            "runtime WASM build succeeded but did not create {}",
            artifact.display()
        ))
    }
}

/// The small WASI command that establishes an interpreter, executes user
/// `::top`, then prints the value of [`Case::QUERY`] from that interpreter.
fn bootstrap_wat() -> String {
    let query = Case::QUERY;
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
    (call $top)
    (local.set $result (call $eval (call $box (i32.const 0x180000) (i32.const {query_len}))))
    (local.set $strptr (call $getstr (local.get $result) (i32.const 0x190000)))
    (local.set $len (i32.load (i32.const 0x190000)))
    (i32.store (i32.const 0x190008) (local.get $strptr))
    (i32.store (i32.const 0x19000C) (local.get $len))
    (drop (call $fd_write (i32.const 1) (i32.const 0x190008) (i32.const 1) (i32.const 0x190010)))
    (call $rel (local.get $result)))
  (data (i32.const 0x180000) "{query}"))
"#,
        query_len = query.len(),
    )
}

/// Run a generated case through C Tcl 9 and the actual linked runtime.
pub fn check(
    case: &Case,
    runtime: &Path,
    oracle: &Path,
    oracle_args: &[String],
    scratch: &Path,
    seed: u64,
    timeout: Duration,
) -> LinkedVerdict {
    let oracle_path = match write_script(scratch, seed, &case.oracle_source()) {
        Ok(path) => path,
        Err(error) => return LinkedVerdict::Unrunnable(format!("oracle script: {error}")),
    };
    let oracle_outcome = run_backend(oracle, oracle_args, &oracle_path, timeout);
    let _ = std::fs::remove_file(oracle_path);

    let registry = CommandRegistry::build_default();
    let program = case.program();
    let module = lower_to_ir(&program, &registry);
    let user_path = scratch.join(format!("linked-user-{seed}.wasm"));
    let bootstrap_path = scratch.join(format!("linked-bootstrap-{seed}.wat"));
    let user = wasm_codegen_module_based(&module, &program, RESERVED_DATA_BASE).to_bytes();
    if let Err(error) = std::fs::write(&user_path, user) {
        return LinkedVerdict::Unrunnable(format!("user WASM: {error}"));
    }
    if let Err(error) = std::fs::write(&bootstrap_path, bootstrap_wat()) {
        let _ = std::fs::remove_file(&user_path);
        return LinkedVerdict::Unrunnable(format!("WASI bootstrap: {error}"));
    }
    let wasm_args = vec![
        "run".to_owned(),
        "--preload".to_owned(),
        format!("tcl={}", runtime.display()),
        "--preload".to_owned(),
        format!("user={}", user_path.display()),
    ];
    let wasm_outcome = run_backend(Path::new("wasmtime"), &wasm_args, &bootstrap_path, timeout);
    let _ = std::fs::remove_file(user_path);
    let _ = std::fs::remove_file(bootstrap_path);

    match (&oracle_outcome, &wasm_outcome) {
        (Outcome::Ran { .. }, Outcome::Ran { .. }) => {
            if matches!(
                compare_outcomes(&oracle_outcome, &wasm_outcome, true),
                Verdict::Match
            ) {
                LinkedVerdict::Match
            } else {
                LinkedVerdict::Divergence {
                    oracle: oracle_outcome,
                    wasm: wasm_outcome,
                }
            }
        }
        _ => LinkedVerdict::Unrunnable(format!(
            "oracle: {}; linked WASM: {}",
            outcome_summary(&oracle_outcome),
            outcome_summary(&wasm_outcome)
        )),
    }
}

fn outcome_summary(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Ran {
            stdout,
            stderr,
            errored,
        } => format!("ran (errored={errored}, stdout={stdout:?}, stderr={stderr:?})"),
        Outcome::Timeout => "timed out".to_owned(),
        Outcome::Unavailable(message) => format!("unavailable: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tn_generated_oracle_and_linked_program_share_the_same_query() {
        // TN: this is a normal, successful source/query shape; the observation
        // is an extra command after the program, not a different computation.
        let case = Case::from_seed(42);
        assert!(case.oracle_source().ends_with("puts -nonewline [set r]\n"));
        assert_eq!(Case::QUERY, "set r");
    }

    #[test]
    fn fn_seeded_names_are_tcl_identifier_safe_and_distinct() {
        // FN prevention: a source-generation failure must not be mistaken for
        // a runtime mismatch because an arbitrary seed formed invalid Tcl.
        let a = Case::from_seed(1);
        let b = Case::from_seed(2);
        assert!(a.name.starts_with('n'));
        assert_ne!(a, b);
    }

    #[test]
    fn bootstrap_uses_the_reserved_nonoverlapping_regions() {
        // TP for the link layout: user data, query data, and runtime data are
        // separated by the documented reserved gap contract.
        let wat = bootstrap_wat();
        assert!(wat.contains("0x180000"));
        assert!(wat.contains("0x190000"));
        assert!(wat.contains("import \"user\" \"::top\""));
    }
}
