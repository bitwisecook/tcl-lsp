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

//! Cross-version `info` surface vectors for the bytecode VM.
//!
//! The selected runtime release controls both its reported version and which
//! math-function builtins may appear in `info functions` / `info commands`.
//! Every vector is run against the VM and, when available, its matching C Tcl
//! release, so this command-table view cannot drift from the registry surface.

use std::cell::RefCell;
use std::io::Write as _;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode as lower_to_ir;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let registry = tcl_registry::registry_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let ir = tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect(
            src,
            registry,
            config,
            Some(profile),
        );
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry))
    }
}

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn vm_output(src: &str, version: TclVersion) -> String {
    let profile = DialectProfile::by_name(version.dialect_name());
    let service = CompilerSvc {
        registry: CommandRegistry::build_default(),
    };
    let asm = service
        .compile_for_profile(src, profile)
        .expect("test script compiles for its selected profile");
    let capture = Capture::default();
    let mut vm = Vm::with_output(Box::new(capture.clone()));
    vm.set_compiler(Box::new(service));
    vm.set_runtime_version(version);
    let _ = vm.run_module(&asm);
    String::from_utf8_lossy(&capture.0.borrow())
        .trim()
        .to_owned()
}

fn tclsh_output(bin_env: &str, names: &[&str], src: &str) -> Option<String> {
    let mut candidates = std::env::var(bin_env).ok().into_iter().collect::<Vec<_>>();
    candidates.extend(names.iter().map(ToString::to_string));
    for name in candidates {
        let Ok(mut child) = std::process::Command::new(name)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            continue;
        };
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(src.as_bytes())
            .expect("write Tcl script");
        let output = child.wait_with_output().expect("run Tcl script");
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_owned());
        }
    }
    None
}

const SCRIPT: &str = "puts [info tclversion]\n\
    puts [list [info functions sin] [info commands ::tcl::mathfunc::sin] \\
    [info functions isfinite] [info commands ::tcl::mathfunc::isfinite]]\n";

struct Vector {
    version: TclVersion,
    expected: &'static str,
    env: &'static str,
    tclsh: &'static [&'static str],
}

const VECTORS: &[Vector] = &[
    Vector {
        version: TclVersion::V8_4,
        expected: "8.4\nsin {} {} {}",
        env: "TCL_LSP_TCLSH84",
        tclsh: &["tclsh8.4"],
    },
    Vector {
        version: TclVersion::V8_5,
        expected: "8.5\nsin ::tcl::mathfunc::sin {} {}",
        env: "TCL_LSP_TCLSH85",
        tclsh: &["tclsh8.5"],
    },
    Vector {
        version: TclVersion::V8_6,
        expected: "8.6\nsin ::tcl::mathfunc::sin {} {}",
        env: "TCL_LSP_TCLSH86",
        tclsh: &["tclsh8.6"],
    },
    Vector {
        version: TclVersion::V9_0,
        expected: "9.0\nsin ::tcl::mathfunc::sin isfinite ::tcl::mathfunc::isfinite",
        env: "TCL_LSP_TCLSH90",
        tclsh: &["tclsh9.0"],
    },
    Vector {
        version: TclVersion::V9_1,
        expected: "9.1\nsin ::tcl::mathfunc::sin isfinite ::tcl::mathfunc::isfinite",
        env: "TCL_LSP_TCLSH91",
        tclsh: &["tclsh9.1"],
    },
];

#[test]
fn selected_runtime_surface_matches_the_cross_version_vectors() {
    for vector in VECTORS {
        assert_eq!(
            vm_output(SCRIPT, vector.version),
            vector.expected,
            "{:?}",
            vector.version
        );
    }
}

#[test]
fn vectors_match_real_tclsh_when_available() {
    let mut ran = 0;
    for vector in VECTORS {
        if let Some(actual) = tclsh_output(vector.env, vector.tclsh, SCRIPT) {
            assert_eq!(actual, vector.expected, "{:?}", vector.version);
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!("skipping: no versioned tclsh binaries found");
    }
}
