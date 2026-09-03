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

//! Integration: startup uses a selected Tcl library and discovers `tcltest`
//! through ordinary package machinery.

use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::{
    first_fatal_parse_error_with_config, lower_to_ir_for_bytecode_with_dialect,
};
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_registry::CommandRegistry;
use tcl_test_support::locate_source_tree;
use tcl_vm::{CompileError, CompileService, Value, Vm};

struct CompilerSvc {
    registry: &'static CommandRegistry,
    config: tcl_lexer::LexerConfig,
    profile: &'static DialectProfile,
}

impl CompilerSvc {
    fn tcl_9_0() -> Self {
        let profile = DialectProfile::find("tcl9.0").expect("Tcl 9.0 profile exists");
        Self {
            registry: tcl_registry::model::static_context_for_profile(profile).commands(),
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            profile,
        }
    }
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(message) = first_fatal_parse_error_with_config(src, self.config) {
            return Err(CompileError(message));
        }
        let ir = lower_to_ir_for_bytecode_with_dialect(
            src,
            self.registry,
            self.config,
            Some(self.profile),
        );
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, self.registry))
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        assert_eq!(profile.name, self.profile.name, "test VM changed profile");
        self.compile(src)
    }
}

fn configure_vm(mut vm: Vm, library: &Path) -> Vm {
    vm.set_runtime_version(TclVersion::V9_0);
    vm.set_compiler(Box::new(CompilerSvc::tcl_9_0()));
    vm.set_var(
        "::tcl_library",
        Value::string(library.to_string_lossy().into_owned()),
    )
    .expect("test library path is settable");
    vm
}

fn vm_for_library(library: &Path) -> Vm {
    configure_vm(Vm::new(), library)
}

#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct FixtureDir(PathBuf);

impl FixtureDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let suffix = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tcl-vm-init-library-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated startup fixture");
        Self(path)
    }
}

impl Drop for FixtureDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// This intentionally uses a library which process startup cannot know about.
/// It catches an implementation that merely reports success, sources a fixed
/// built-in script, or bypasses `package unknown` / `package ifneeded`.
#[test]
fn init_library_sources_the_selected_init_and_require_discovers_its_package() {
    let library = FixtureDir::new();
    fs::write(
        library.0.join("init.tcl"),
        "set ::startup_marker [file tail [info script]]\n\
         namespace eval ::fixture {\n\
             proc discover {name args} {\n\
                 if {$name eq {fixture_package}} {\n\
                     package ifneeded fixture_package 1.0 {\n\
                         set ::package_marker loaded\n\
                         package provide fixture_package 1.0\n\
                     }\n\
                 }\n\
             }\n\
         }\n\
         package unknown ::fixture::discover\n",
    )
    .expect("write init.tcl");

    let mut vm = vm_for_library(&library.0);
    let init = vm.init_library();
    assert!(
        init.code.is_ok(),
        "selected init.tcl failed: {}",
        init.result.to_str()
    );
    assert_eq!(
        &*vm.get_var("::startup_marker")
            .expect("init.tcl marker")
            .to_str(),
        "init.tcl"
    );

    let required = vm
        .eval_source("package require -exact fixture_package 1.0")
        .expect("compile fixture package require");
    assert!(
        required.code.is_ok(),
        "fixture package was not discovered: {}",
        required.result.to_str()
    );
    assert_eq!(&*required.result.to_str(), "1.0");
    assert_eq!(
        &*vm.get_var("::package_marker")
            .expect("ifneeded loader marker")
            .to_str(),
        "loaded"
    );
}

/// The selected upstream Tcl 9.0.4 library is the production proof: init must
/// install its package-unknown callback, and `tcltest` must be found by normal
/// `package require`, never by a direct source in the driver.
#[test]
fn real_tcl_9_0_4_init_discovers_tcltest_via_package_require() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Some(source_tree) = locate_source_tree(&repo_root, TclVersion::V9_0, None)
        .expect("Tcl 9 source tree discovery")
    else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert_eq!(source_tree.patchlevel, "9.0.4", "real-library oracle pin");

    let mut vm = vm_for_library(&source_tree.library_dir());
    let init = vm.init_library();
    assert!(
        init.code.is_ok(),
        "real Tcl 9.0.4 init.tcl failed: {}",
        init.result.to_str()
    );
    let required = vm
        .eval_source("package require -exact tcltest 2.5.11")
        .expect("compile tcltest package require");
    assert!(
        required.code.is_ok(),
        "tcltest package discovery failed: {}",
        required.result.to_str()
    );
    assert_eq!(&*required.result.to_str(), "2.5.11");
    assert!(
        vm.get_var("::tcltest::numTests(Total)").is_some(),
        "package require did not load the tcltest namespace"
    );
}

/// A real upstream stem reaches `cleanupTests` through the same init/require
/// path as the sweep driver and emits the summary that xtask parses.
#[test]
fn upstream_set_stem_emits_a_parseable_summary_after_real_startup() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Some(source_tree) = locate_source_tree(&repo_root, TclVersion::V9_0, None)
        .expect("Tcl 9 source tree discovery")
    else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert_eq!(source_tree.patchlevel, "9.0.4", "real-library oracle pin");

    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = std::thread::Builder::new()
        .name("tcltest-set-stem".to_owned())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let bytes = Rc::new(RefCell::new(Vec::new()));
            let vm = Vm::with_output(Box::new(Capture(Rc::clone(&bytes))));
            let mut vm = configure_vm(vm, &source_tree.library_dir());
            let init = vm.init_library();
            if !init.code.is_ok() {
                sender
                    .send((false, init.result.to_str().to_string(), String::new()))
                    .expect("report init failure");
                return;
            }
            let testfile = source_tree.tests_dir().join("set.test");
            let script = format!(
                "package require tcltest\n\
                 namespace import -force ::tcltest::*\n\
                 ::tcltest::configure -match set-1.1\n\
                 source {}\n",
                tcl_syntax::list::list_element(&testfile.to_string_lossy())
            );
            let run = vm
                .eval_source(&script)
                .expect("compile focused upstream stem");
            let output = String::from_utf8_lossy(&bytes.borrow()).into_owned();
            sender
                .send((run.code.is_ok(), run.result.to_str().to_string(), output))
                .expect("report focused upstream stem");
        })
        .expect("spawn focused upstream stem");

    let result = match receiver.recv_timeout(Duration::from_secs(30)) {
        Ok(result) => result,
        Err(RecvTimeoutError::Disconnected) => {
            worker.join().expect("focused upstream worker panicked");
            unreachable!("worker exited without reporting")
        }
        Err(RecvTimeoutError::Timeout) => {
            panic!("focused upstream set.test did not finish within 30 seconds")
        }
    };
    worker.join().expect("focused upstream worker panicked");
    let (ok, error, output) = result;
    assert!(ok, "focused upstream set.test failed: {error}\n{output}");
    assert!(
        output.contains("Total\t64\tPassed\t1\tSkipped\t63\tFailed\t0"),
        "missing parseable focused Tcltest summary: {output:?}"
    );
}
