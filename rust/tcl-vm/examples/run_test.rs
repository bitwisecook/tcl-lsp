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

//! Drive a tcltest `.test` file through the VM (the bytecode executor) — the VM
//! analogue of `runtime/rust`'s `run_script --init`, for comparing tcltest
//! pass/fail/skip performance between the two runtimes.
//!
//! Usage: `TCL_LIBRARY=.../library cargo run -p tcl-vm --example run_test --
//! <file.test> [--match <test-glob-list>]`
//!
//! The VM sources the selected library's real `init.tcl`, then obtains tcltest
//! through normal `package require` discovery. Runs on a large-stack worker
//! thread so deep recursion is a catchable error rather than a native-stack
//! overflow (matching the `run_script` driver).

use std::io::Write;

use tcl_compiler::cfg_builder::build_cfg_codegen as build_cfg;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect as lower_to_ir_profile;
use tcl_compiler::lowering::lower_to_ir_traced_with_dialect;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

/// The `CompileService` the VM uses for runtime `eval` / command substitution:
/// the real Rust compiler pipeline (lower → CFG → bytecode).
struct Svc {
    registry: &'static CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static DialectProfile>,
}

impl Svc {
    fn for_profile(profile: &'static DialectProfile) -> Self {
        Self {
            registry: tcl_registry::model::static_context_for_profile(profile).commands(),
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            dialect: Some(profile),
        }
    }
}

impl CompileService for Svc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) =
            tcl_compiler::lowering::first_fatal_parse_error_with_config(src, self.config)
        {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir_profile(src, self.registry, self.config, self.dialect);
        let cfg = build_cfg(&ir, false);
        Ok(codegen_module(&cfg, &ir, self.registry))
    }
    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        Self::for_profile(profile).compile(src)
    }
    fn compile_traced(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) =
            tcl_compiler::lowering::first_fatal_parse_error_with_config(src, self.config)
        {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir_traced_with_dialect(src, self.registry, self.config, self.dialect);
        let cfg = build_cfg(&ir, false);
        Ok(codegen_module(&cfg, &ir, self.registry))
    }
    fn compile_traced_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        Self::for_profile(profile).compile_traced(src)
    }
}

/// Pass the VM's `puts` output straight through to the process stdout.
struct Stdout;
impl Write for Stdout {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        let n = std::io::stdout().write(b)?;
        std::io::stdout().flush()?;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

const STACK_BYTES: usize = 512 * 1024 * 1024;

fn main() {
    let code = std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(run)
        .expect("spawn")
        .join()
        .expect("worker panicked");
    std::process::exit(code);
}

fn run() -> i32 {
    let arguments = match Arguments::parse(std::env::args().skip(1)) {
        Ok(arguments) => arguments,
        Err(message) => {
            eprintln!("{message}\nusage: run_test <file.test> [--match <test-glob-list>]");
            return 2;
        }
    };
    // Optionally source the backend-constraint overlay after tcltest discovery
    // and before the test file so unsupported host/backend probes are skipped.
    let overlay = match std::env::var("TCL_BACKEND_CONSTRAINTS") {
        Ok(p) if !p.is_empty() => format!("source {p}\n"),
        _ => String::new(),
    };
    // Diagnostic: `TCL_TEST_VERBOSE=1` makes tcltest announce each test as it
    // starts, so a hanging test can be pinpointed.
    let verbose = match std::env::var("TCL_TEST_VERBOSE") {
        Ok(v) if !v.is_empty() => "::tcltest::configure -verbose {body start}\n",
        _ => "",
    };
    let match_config = arguments
        .match_filter
        .as_ref()
        .map_or_else(String::new, |pattern| {
            format!(
                "::tcltest::configure -match {}\n",
                tcl_syntax::list::list_element(pattern)
            )
        });
    let src = format!(
        "package require tcltest\nnamespace import -force ::tcltest::*\n{verbose}{match_config}{overlay}source {}\n",
        tcl_syntax::list::list_element(&arguments.testfile)
    );
    let profile = DialectProfile::find("tcl9.0").expect("Tcl 9.0 profile exists");
    let compiler = Svc::for_profile(profile);
    let asm = match compiler.compile(&src) {
        Ok(module) => module,
        Err(error) => {
            eprintln!("compile error: {}", error.0);
            return 1;
        }
    };

    let mut vm = Vm::with_output(Box::new(Stdout));
    vm.set_runtime_version(TclVersion::V9_0);
    vm.set_compiler(Box::new(Svc::for_profile(profile)));
    let init = vm.init_library();
    if !init.code.is_ok() {
        eprintln!("Tcl library initialisation error: {}", init.result.to_str());
        return 1;
    }
    let c = vm.run_module(&asm);
    if c.code.is_ok() {
        0
    } else {
        eprintln!(
            "VM error: {}\noptions: {}",
            c.result.to_str(),
            c.options.to_str()
        );
        1
    }
}

struct Arguments {
    testfile: String,
    match_filter: Option<String>,
}

impl Arguments {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let testfile = args.next().ok_or_else(|| "missing test file".to_owned())?;
        let mut match_filter = None;
        while let Some(option) = args.next() {
            match option.as_str() {
                "--match" if match_filter.is_none() => {
                    match_filter = Some(
                        args.next()
                            .ok_or_else(|| "--match requires a value".to_owned())?,
                    );
                }
                "--match" => return Err("--match may be supplied only once".to_owned()),
                _ => return Err(format!("unknown option {option:?}")),
            }
        }
        Ok(Self {
            testfile,
            match_filter,
        })
    }
}
