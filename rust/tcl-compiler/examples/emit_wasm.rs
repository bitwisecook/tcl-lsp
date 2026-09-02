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

use tcl_compiler::codegen::wasm::{SemanticOptimisationPassId, WasmCompileOptions, compile_wasm};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (standalone, init, analysis, wat, rest): (bool, bool, bool, bool, Vec<&String>) = {
        let mut standalone = false;
        let mut init = false;
        let mut analysis = false;
        let mut wat = false;
        let mut rest = Vec::new();
        for a in &args {
            match a.as_str() {
                "--standalone" => standalone = true,
                // `--init` bootstraps the stdlib in `_start`; it implies standalone.
                "--init" => {
                    init = true;
                    standalone = true;
                }
                // Opt into the analysis-derived specialisation tier (off by default).
                "--analysis" => analysis = true,
                // Also write `<out>.wat` next to the binary.
                "--wat" => wat = true,
                _ => rest.push(a),
            }
        }
        (standalone, init, analysis, wat, rest)
    };
    let [script_path, out_path] = rest.as_slice() else {
        eprintln!(
            "usage: emit_wasm [--standalone] [--init] [--analysis] [--wat] <script-file> <out.wasm>"
        );
        std::process::exit(2);
    };
    let src = fs::read_to_string(script_path).expect("read script");
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(&src, &registry, false, "tcl9.0");
    let options = if standalone {
        WasmCompileOptions::standalone(init)
    } else {
        WasmCompileOptions::runtime_linked()
    };
    let options = if analysis {
        options.with_semantic_optimisation(SemanticOptimisationPassId::LegacyAnalysisSpecialisation)
    } else {
        options
    };
    let mut wasm = compile_wasm(&unit, &registry, options);
    fs::write(out_path, wasm.to_bytes()).expect("write wasm");
    if wat {
        fs::write(format!("{out_path}.wat"), wasm.to_wat()).expect("write wat");
    }
    eprintln!("codegen plan: {}", wasm.plan.as_str());
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
