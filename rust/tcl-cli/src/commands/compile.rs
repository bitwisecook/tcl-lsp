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

//! Low-level compilation verbs: `dis` (bytecode disassembly) and `compwasm`
//! (WebAssembly emit).
//!
//! Both verbs resolve their inputs the same way the rest of the CLI does, then drive the
//! compiler pipeline directly:
//!
//! - `dis` lowers to the bytecode IR, builds the codegen CFG (the
//!   `faithful_exceptions`-off shape — see `tcl-explorer`'s `asm` view for why),
//!   emits the `ModuleAsm`, and renders it with `format_module_asm`.
//! - `compwasm` lowers to the analysis IR and runs the greenfield WASM
//!   eval-fallback emitter, writing the binary module (and, optionally, its WAT
//!   text form).
//!
//! `--optimise` runs the source-to-source optimiser first, exactly as the
//! `compile_script(optimise=...)` path does (`apply_optimisations` over
//! the `optimise` rewrites), then compiles the rewritten source.

use tcl_cli_support::{
    OutputTarget, combine_sources, read_input_documents, registry_for_dialect, write_binary_output,
    write_text_output,
};
use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::codegen::format::format_module_asm;
use tcl_compiler::codegen::wasm::wasm_codegen_module;
use tcl_compiler::lowering::{lower_to_ir, lower_to_ir_for_bytecode};
use tcl_compiler::optimiser::{apply_optimisations, optimise};
use tcl_registry::CommandRegistry;

use crate::cli::InputArgs;

/// Apply the source-to-source optimiser when `optimise` is set, mirroring the
/// `compile_script(optimise=...)` entry: collect the rewrites, apply
/// them, and compile the rewritten text. Returns the (possibly unchanged)
/// source.
fn maybe_optimise(source: &str, registry: &CommandRegistry, optimise_on: bool) -> String {
    if optimise_on {
        apply_optimisations(source, &optimise(source, registry))
    } else {
        source.to_owned()
    }
}

/// `tcl dis` — compile source and emit human-readable bytecode disassembly.
pub fn run_dis(input: &InputArgs, optimise_on: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let registry = registry_for_dialect(input.dialect_or_default());
    let source = maybe_optimise(&combine_sources(&documents), registry, optimise_on);

    let ir = lower_to_ir_for_bytecode(&source, registry);
    let cfg = build_cfg_codegen(&ir, false);
    let module = codegen_module(&cfg, &ir, registry);
    let disassembly = format_module_asm(&module);

    let target = OutputTarget::from_arg(input.output.as_deref());
    write_text_output(&target, &disassembly)?;
    Ok(0)
}

/// `tcl compwasm` — compile source to a WebAssembly binary. The default `vm`
/// backend emits the self-contained bytecode-VM runner; `tree-walker` emits the
/// legacy eval-fallback module (`RUST_ISSUE_008`).
pub fn run_compwasm(
    input: &InputArgs,
    backend: crate::cli::WasmBackend,
    wat_output: Option<&std::path::Path>,
) -> anyhow::Result<u8> {
    match backend {
        crate::cli::WasmBackend::TreeWalker => run_compwasm_tree_walker(input, wat_output),
        crate::cli::WasmBackend::Vm => run_compwasm_vm(input, wat_output),
    }
}

/// The default `vm` backend: emit the self-contained bytecode-VM runner
/// (`vm.wasm`). The runner is a cargo-built cdylib (bytecode is not serialisable,
/// so there is no per-script module), so the user's script is compile-checked
/// here and then fed to the shipped runner at run time via its
/// `tcl_alloc`/`tcl_eval`/`tcl_dealloc` ABI.
fn run_compwasm_vm(input: &InputArgs, wat_output: Option<&std::path::Path>) -> anyhow::Result<u8> {
    if wat_output.is_some() {
        anyhow::bail!(
            "--wat-output is only supported with --backend tree-walker (the vm runner is a \
             cargo-built cdylib, not a WAT-introspectable module)"
        );
    }
    // Compile-check the script so a syntax error is reported now, even though the
    // generic runner executes it at run time rather than embedding it.
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let source = combine_sources(&documents);
    if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(&source) {
        anyhow::bail!("{msg}");
    }

    let runner = locate_vm_wasm().ok_or_else(|| {
        anyhow::anyhow!(
            "VM wasm runner not found — build it with `make tcl-vm-wasm` (or point TCL_VM_WASM \
             at a vm.wasm)"
        )
    })?;
    let bytes =
        std::fs::read(&runner).map_err(|e| anyhow::anyhow!("reading {}: {e}", runner.display()))?;

    let target = match input.output.as_deref() {
        None => OutputTarget::File(std::path::PathBuf::from("out.wasm")),
        Some(path) => OutputTarget::from_arg(Some(path)),
    };
    write_binary_output(&target, &bytes)?;
    let where_to = match &target {
        OutputTarget::Stdout => "stdout".to_owned(),
        OutputTarget::File(path) => path.display().to_string(),
    };
    eprintln!("wrote VM wasm runner ({} bytes) to {where_to}", bytes.len());
    eprintln!(
        "  feed a Tcl script (coroutines included) through it via the tcl_alloc/tcl_eval/tcl_dealloc \
         ABI — see rust/tcl-vm-wasm/verify.mjs for a host example"
    );
    Ok(0)
}

/// Find the shipped VM wasm runner: an explicit `TCL_VM_WASM`, then the
/// `make tcl-vm-wasm` output, then the crate's own build artifact.
fn locate_vm_wasm() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("TCL_VM_WASM") {
        let path = std::path::PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    // Relative to this crate's source (a repo checkout): the `make` output dir
    // and the crate's own `cargo build` artifact.
    let cli = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        cli.join("../../build/tcl-vm-wasm/vm.wasm"),
        cli.join("../tcl-vm-wasm/target/wasm32-unknown-unknown/release/tcl_vm_wasm.wasm"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// The legacy `tree-walker` backend: the eval-fallback WASM emitter — a bare
/// module importing the runtime C-ABI (`tcl_*`), with an optional WAT dump.
fn run_compwasm_tree_walker(
    input: &InputArgs,
    wat_output: Option<&std::path::Path>,
) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let registry = registry_for_dialect(input.dialect_or_default());
    let source = combine_sources(&documents);

    let ir = lower_to_ir(&source, registry);
    let mut wasm = wasm_codegen_module(&ir, &source);
    let bytes = wasm.to_bytes();

    // Unlike the other verbs, `compwasm` defaults to a file, not stdout: a bare
    // `tcl compwasm script.tcl` must not dump raw WASM bytes to the terminal, so
    // it writes `out.wasm`; an explicit `-o -` still selects stdout.
    let target = match input.output.as_deref() {
        None => OutputTarget::File(std::path::PathBuf::from("out.wasm")),
        Some(path) => OutputTarget::from_arg(Some(path)),
    };
    write_binary_output(&target, &bytes)?;
    if let Some(wat_path) = wat_output {
        let wat_target = OutputTarget::from_arg(Some(wat_path));
        write_text_output(&wat_target, &wasm.to_wat())?;
    }

    let where_to = match &target {
        OutputTarget::Stdout => "stdout".to_owned(),
        OutputTarget::File(path) => path.display().to_string(),
    };
    eprintln!("wrote wasm binary ({} bytes) to {where_to}", bytes.len());
    Ok(0)
}
