//! Emit a Tcl script's AOT WASM module (`::top` + procs) to a file.
//!
//! Usage: `emit_wasm <script-file> <out.wasm>`
//!
//! The emitted module imports the codegen ABI (`tcl_eval`, `tcl_obj_new_string`,
//! `tcl_expr_bool`, `tcl_obj_release`) and its linear `memory` from module
//! `"tcl"` — the surface `runtime/rust` exports — so a host can link it against
//! the real runtime and run `::top`. (Eval-fallback tier: each leaf command is
//! boxed and handed to `tcl_eval`.)

use std::fs;

use tcl_compiler::codegen::wasm::{RESERVED_DATA_BASE, wasm_codegen_module_based};
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, script_path, out_path] = args.as_slice() else {
        eprintln!("usage: emit_wasm <script-file> <out.wasm>");
        std::process::exit(2);
    };
    let src = fs::read_to_string(script_path).expect("read script");
    let registry = CommandRegistry::build_default();
    let module = lower_to_ir(&src, &registry);
    // Relocate the constant pool into the runtime's reserved gap so boxed
    // command strings sit at non-null offsets — at base 0 the first string lands
    // at offset 0 and `tcl_obj_new_string(ptr=0, …)` is read as a null/empty
    // pointer, silently dropping that command.
    let mut wasm = wasm_codegen_module_based(&module, &src, RESERVED_DATA_BASE);
    fs::write(out_path, wasm.to_bytes()).expect("write wasm");
    eprintln!(
        "wrote {out_path} ({} bytes)",
        fs::metadata(out_path).map(|m| m.len()).unwrap_or(0)
    );
}
