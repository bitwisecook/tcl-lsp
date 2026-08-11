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
//! - `compwasm` selects one of three explicit backends: the self-contained VM
//!   runner, the analysis-aware Tcl-object emitter with runtime fallback, or
//!   the bounded executable-IR generic-argv transport. The two per-script
//!   emitters can also write their WAT form.
//!
//! `--optimise` runs the source-to-source optimiser first, exactly as the
//! `compile_script(optimise=...)` path does (`apply_optimisations` over
//! the `optimise` rewrites), then compiles the rewritten source.

use tcl_cli_support::{
    OutputTarget, combine_sources, combined_effective_dialect, read_input_documents,
    registry_for_dialect, write_binary_output, write_text_output,
};
use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::codegen::format::format_module_asm;
use tcl_compiler::codegen::wasm::{
    LiteralSafeWasmOptions, compile_literal_safe_wasm, wasm_codegen_compilation_unit,
};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::executable_ir::ExecutableFunctionId;
use tcl_compiler::lowering::{lower_to_ir_for_bytecode_with_dialect, lower_to_ir_with_dialect};
use tcl_compiler::optimiser::{apply_optimisations, optimise_with_dialect};
use tcl_registry::CommandRegistry;

use crate::cli::InputArgs;

/// Apply the source-to-source optimiser when `optimise` is set, mirroring the
/// `compile_script(optimise=...)` entry: collect the rewrites, apply
/// them, and compile the rewritten text. Returns the (possibly unchanged)
/// source.
fn maybe_optimise(
    source: &str,
    registry: &CommandRegistry,
    dialect: &str,
    optimise_on: bool,
) -> String {
    if optimise_on {
        apply_optimisations(
            source,
            &optimise_with_dialect(source, registry, Some(dialect)),
        )
    } else {
        source.to_owned()
    }
}

/// `tcl dis` — compile source and emit human-readable bytecode disassembly.
pub fn run_dis(input: &InputArgs, optimise_on: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect.as_deref());
    let registry = registry_for_dialect(&dialect);
    let source = maybe_optimise(
        &combine_sources(&documents),
        registry,
        &dialect,
        optimise_on,
    );

    let ir = lower_to_ir_for_bytecode_with_dialect(
        &source,
        registry,
        tcl_lexer::LexerConfig::for_dialect(&dialect),
        &dialect,
    );
    let cfg = build_cfg_codegen(&ir, false);
    let module = codegen_module(&cfg, &ir, registry);
    let disassembly = format_module_asm(&module);

    let target = OutputTarget::from_arg(input.output.as_deref());
    write_text_output(&target, &disassembly)?;
    Ok(0)
}

/// `tcl compwasm` — compile source to a WebAssembly binary. The default `vm`
/// backend emits the self-contained bytecode-VM runner; `tree-walker` emits the
/// analysis-aware Tcl-object module with runtime fallback.
/// analysis-aware Tcl-object module with runtime fallback; and
/// `generic-invoke` emits the bounded executable-IR argv transport.
pub fn run_compwasm(
    input: &InputArgs,
    backend: crate::cli::WasmBackend,
    wat_output: Option<&std::path::Path>,
) -> anyhow::Result<u8> {
    match backend {
        crate::cli::WasmBackend::TreeWalker => run_compwasm_tree_walker(input, wat_output),
        crate::cli::WasmBackend::Vm => run_compwasm_vm(input, wat_output),
        crate::cli::WasmBackend::GenericInvoke => run_compwasm_generic_invoke(input, wat_output),
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
            "--wat-output is only supported with --backend tree-walker or generic-invoke (the vm runner is a \
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

/// The executable-IR generic argv transport.  This is intentionally an opt-in
/// diagnostic backend while its domain is one literal-safe flat command: it
/// must decline dynamic words and multi-command scripts, not reinterpret them
/// through the legacy eval fallback.
fn run_compwasm_generic_invoke(
    input: &InputArgs,
    wat_output: Option<&std::path::Path>,
) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect.as_deref());
    let dialect_set = tcl_registry::dialects::DialectSet::parse(&dialect).ok_or_else(|| {
        anyhow::anyhow!("generic-invoke requires a concrete registered dialect, got {dialect:?}")
    })?;
    let registry = registry_for_dialect(&dialect);
    let source = combine_sources(&documents);
    let ir = lower_to_ir_with_dialect(
        &source,
        registry,
        tcl_lexer::LexerConfig::for_dialect(&dialect),
        &dialect,
    );
    let mut output = compile_literal_safe_wasm(
        registry,
        dialect_set,
        &ir.top_level,
        LiteralSafeWasmOptions::new(ExecutableFunctionId::new(0), "::top"),
    )
    .map_err(|decline| anyhow::anyhow!("generic-invoke backend declined: {decline}"))?;
    let bytes = output.module.to_bytes();
    let target = match input.output.as_deref() {
        None => OutputTarget::File(std::path::PathBuf::from("out.wasm")),
        Some(path) => OutputTarget::from_arg(Some(path)),
    };
    write_binary_output(&target, &bytes)?;
    if let Some(wat_path) = wat_output {
        let wat_target = OutputTarget::from_arg(Some(wat_path));
        write_text_output(&wat_target, &output.module.to_wat())?;
    }
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

/// The `tree-walker` backend: a bare module importing the runtime C ABI, with
/// direct analysed operations, source fallback, and an optional WAT dump.
fn run_compwasm_tree_walker(
    input: &InputArgs,
    wat_output: Option<&std::path::Path>,
) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect.as_deref());
    let registry = registry_for_dialect(&dialect);
    let source = combine_sources(&documents);

    let unit = CompilationUnit::build_for_dialect(&source, registry, false, &dialect);
    let mut wasm = wasm_codegen_compilation_unit(&unit, registry);
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
