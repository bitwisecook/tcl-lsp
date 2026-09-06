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
//! - `compwasm` drives the canonical analysis-aware WASM pipeline and can also
//!   write the generated module's WAT form.
//!
//! `--optimise` runs the source-to-source optimiser first, exactly as the
//! `compile_script(optimise=...)` path does (`apply_optimisations` over
//! the `optimise` rewrites), then compiles the rewritten source.

use tcl_cli_support::{
    OutputTarget, combine_sources, combined_effective_dialect, read_input_documents,
    registry_for_dialect, write_binary_output, write_text_output,
};
use tcl_compiler::cfg_builder::build_cfg_codegen_with_registry;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::codegen::format::format_module_asm;
use tcl_compiler::codegen::wasm::{WasmCompileOptions, compile_wasm};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect;
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
        let profile = tcl_cli_support::environment::profile_for_dialect(dialect);
        apply_optimisations(
            source,
            &optimise_with_dialect(source, registry, Some(profile)),
        )
    } else {
        source.to_owned()
    }
}

/// `tcl dis` — compile source and emit human-readable bytecode disassembly.
pub fn run_dis(input: &InputArgs, optimise_on: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let registry = registry_for_dialect(dialect.name);
    let source = maybe_optimise(
        &combine_sources(&documents),
        &registry,
        dialect.name,
        optimise_on,
    );

    let ir = lower_to_ir_for_bytecode_with_dialect(
        &source,
        &registry,
        tcl_lexer::LexerConfig::for_profile(Some(dialect)),
        Some(dialect),
    );
    let cfg = build_cfg_codegen_with_registry(&ir, false, &registry);
    let module = codegen_module(&cfg, &ir, &registry);
    let disassembly = format_module_asm(&module);

    let target = OutputTarget::from_arg(input.output.as_deref());
    write_text_output(&target, &disassembly)?;
    Ok(0)
}

/// `tcl compwasm` — compile source through the canonical analysis-aware WASM
/// pipeline. Unsupported specialisations retain exact source-span runtime
/// fallback inside the generated module.
///
/// `optimisations` is the `--codegen-passes` selection. It defaults to empty:
/// an unflagged `tcl compwasm` emits the generic lowering, with every
/// semantic/AOT pass off.
pub fn run_compwasm(
    input: &InputArgs,
    wat_output: Option<&std::path::Path>,
    optimisations: tcl_explorer::SemanticOptimisationConfig,
) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let registry = registry_for_dialect(dialect.name);
    let source = combine_sources(&documents);

    let unit = CompilationUnit::build_for_dialect(&source, &registry, false, dialect.name);
    let options = WasmCompileOptions::hosted().with_semantic_optimisations(optimisations);
    let mut wasm = compile_wasm(&unit, &registry, options);
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
    if let Some(reason) = wasm.plan.semantic_decline() {
        eprintln!(
            "codegen plan: {} ({}: {})",
            wasm.plan.as_str(),
            reason.as_str(),
            reason.detail_kind()
        );
    } else {
        eprintln!("codegen plan: {}", wasm.plan.as_str());
    }
    Ok(0)
}
