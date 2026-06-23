//! Emit a Tcl script's AOT WASM module (`::top` + procs) to a file.
//!
//! Usage: `emit_wasm [--standalone] <script-file> <out.wasm>`
//!
//! The emitted module imports the codegen ABI (`tcl_eval`, `tcl_obj_new_string`,
//! `tcl_expr_bool`, `tcl_obj_release`) and its linear `memory` from module
//! `"tcl"` — the surface `runtime/rust` exports — so a host can link it against
//! the real runtime and run `::top`. (Eval-fallback tier: each leaf command is
//! boxed and handed to `tcl_eval`.)
//!
//! With `--standalone`, the module also imports the interp bootstrap
//! (`tcl_runtime_create_interp`/`set_current_interp`) and exports a WASI
//! `_start` that bootstraps then runs `::top` — so after merging with the
//! runtime (`wasm-merge runtime.wasm tcl out.wasm user -all -o merged.wasm`) the
//! single `merged.wasm` runs self-contained under `wasmtime merged.wasm`.

use std::fs;

use tcl_compiler::codegen::wasm::{
    RESERVED_DATA_BASE, wasm_codegen_module_based, wasm_codegen_module_standalone,
    wasm_codegen_module_standalone_init,
};
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (standalone, init, rest): (bool, bool, Vec<&String>) = {
        let mut standalone = false;
        let mut init = false;
        let mut rest = Vec::new();
        for a in &args {
            match a.as_str() {
                "--standalone" => standalone = true,
                // `--init` bootstraps the stdlib in `_start`; it implies standalone.
                "--init" => {
                    init = true;
                    standalone = true;
                }
                _ => rest.push(a),
            }
        }
        (standalone, init, rest)
    };
    let [script_path, out_path] = rest.as_slice() else {
        eprintln!("usage: emit_wasm [--standalone] [--init] <script-file> <out.wasm>");
        std::process::exit(2);
    };
    let src = fs::read_to_string(script_path).expect("read script");
    let registry = CommandRegistry::build_default();
    let module = lower_to_ir(&src, &registry);
    // Relocate the constant pool into the runtime's reserved gap so boxed
    // command strings sit at non-null offsets — at base 0 the first string lands
    // at offset 0 and `tcl_obj_new_string(ptr=0, …)` is read as a null/empty
    // pointer, silently dropping that command.
    let mut wasm = if init {
        wasm_codegen_module_standalone_init(&module, &src, RESERVED_DATA_BASE)
    } else if standalone {
        wasm_codegen_module_standalone(&module, &src, RESERVED_DATA_BASE)
    } else {
        wasm_codegen_module_based(&module, &src, RESERVED_DATA_BASE)
    };
    fs::write(out_path, wasm.to_bytes()).expect("write wasm");
    let tag = match (standalone, init) {
        (_, true) => " [standalone +_start +init_library]",
        (true, false) => " [standalone +_start]",
        _ => "",
    };
    eprintln!(
        "wrote {out_path} ({} bytes){tag}",
        fs::metadata(out_path).map_or(0, |m| m.len()),
    );
}
