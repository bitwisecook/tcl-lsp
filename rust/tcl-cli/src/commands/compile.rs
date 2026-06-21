//! Low-level compilation verbs: `dis` (bytecode disassembly) and `compwasm`
//! (WebAssembly emit).
//!
//! Port of `tooling/tcl/verbs/compile.py` (`_run_dis` / `_run_compwasm`). Both
//! resolve their inputs the same way the rest of the CLI does, then drive the
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
//! Python `compile_script(optimise=...)` path does (`apply_optimisations` over
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
/// Python `compile_script(optimise=...)` entry: collect the rewrites, apply
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
    let registry = registry_for_dialect(&input.dialect);
    let source = maybe_optimise(&combine_sources(&documents), registry, optimise_on);

    let ir = lower_to_ir_for_bytecode(&source, registry);
    let cfg = build_cfg_codegen(&ir, false);
    let module = codegen_module(&cfg, &ir, registry);
    let disassembly = format_module_asm(&module);

    let target = OutputTarget::from_arg(input.output.as_deref());
    write_text_output(&target, &disassembly)?;
    Ok(0)
}

/// `tcl compwasm` — compile source to a WebAssembly binary (eval-fallback tier).
pub fn run_compwasm(input: &InputArgs, wat_output: Option<&std::path::Path>) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let registry = registry_for_dialect(&input.dialect);
    let source = combine_sources(&documents);

    let ir = lower_to_ir(&source, registry);
    let mut wasm = wasm_codegen_module(&ir, &source);
    let bytes = wasm.to_bytes();

    // Unlike the other verbs, `compwasm` defaults to a file, not stdout: a bare
    // `tcl compwasm script.tcl` must not dump raw WASM bytes to the terminal.
    // Mirrors the Python verb's `output="out.wasm"` default; an explicit `-o -`
    // still selects stdout.
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
