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

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_test_support::{locate_source_tree, upstream_test_definition};
use tcl_vm::{Value, Vm};

fn configure_vm(mut vm: Vm, library: &Path) -> Vm {
    vm.set_runtime_version(TclVersion::V9_0);
    let profile = DialectProfile::find("tcl9.0").expect("Tcl 9.0 profile exists");
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
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

/// Execute a bounded slice of one pinned upstream test file through the real
/// Tcl 9.0.4 `init.tcl` and `tcltest` package.
fn run_upstream_definitions(
    test_file: &'static str,
    start_marker: &'static str,
    next_marker: &'static str,
    thread_name: &'static str,
) -> Option<(bool, String, String)> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source_tree = locate_source_tree(&repo_root, TclVersion::V9_0, None)
        .expect("Tcl 9 source tree discovery")?;
    assert_eq!(source_tree.patchlevel, "9.0.4", "real-library oracle pin");

    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = std::thread::Builder::new()
        .name(thread_name.to_owned())
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
            let test_path = source_tree.tests_dir().join(test_file);
            let test_source = fs::read_to_string(&test_path).expect("read pinned upstream test");
            let definitions = upstream_test_definition(&test_source, start_marker, next_marker)
                .expect("extract pinned upstream definition");
            let script = format!(
                "package require tcltest\n\
                 namespace import -force ::tcltest::*\n\
                 {definitions}\n\
                 ::tcltest::cleanupTests\n",
            );
            let run = vm
                .eval_source(&script)
                .expect("compile focused upstream definitions");
            let output = String::from_utf8_lossy(&bytes.borrow()).into_owned();
            sender
                .send((run.code.is_ok(), run.result.to_str().to_string(), output))
                .expect("report focused upstream definitions");
        })
        .expect("spawn focused upstream definitions");

    // Loading real init.tcl/tcltest dominates these small slices and can take
    // several minutes under a contended test runner.
    let result = match receiver.recv_timeout(Duration::from_secs(300)) {
        Ok(result) => result,
        Err(RecvTimeoutError::Disconnected) => {
            worker.join().expect("focused upstream worker panicked");
            unreachable!("worker exited without reporting")
        }
        Err(RecvTimeoutError::Timeout) => {
            panic!("focused upstream {test_file} did not finish within 300 seconds")
        }
    };
    worker.join().expect("focused upstream worker panicked");
    Some(result)
}

/// Issue #1598: run the upstream binary/UTF-8 channel cases through the real
/// Tcl 9.0.4 `init.tcl` and `tcltest`, then pin strict-profile prefix output
/// with the same setup as upstream io-75.9.  The upstream io-75.9 body uses a
/// bidirectional streaming channel, which this focused output change does not
/// add, so its output half is retained here and the file prefix is inspected
/// from Rust after tcltest closes it.
#[test]
fn upstream_channel_output_cases_use_real_tcltest() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Some(source_tree) = locate_source_tree(&repo_root, TclVersion::V9_0, None)
        .expect("Tcl 9 source tree discovery")
    else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert_eq!(source_tree.patchlevel, "9.0.4", "real-library oracle pin");

    let io_source =
        fs::read_to_string(source_tree.tests_dir().join("io.test")).expect("read pinned io.test");
    let io_39_14 = upstream_test_definition(&io_source, "test io-39.14 {", "test io-39.15 {")
        .expect("extract io-39.14");
    let io_39_15 = upstream_test_definition(&io_source, "test io-39.15 {", "test io-39.16 {")
        .expect("extract io-39.15");
    let fixture = FixtureDir::new();
    let path1 = fixture.0.join("io-39.bin");
    let path2 = fixture.0.join("io-75.bin");
    let bytes = Rc::new(RefCell::new(Vec::new()));
    let mut vm = configure_vm(
        Vm::with_output(Box::new(Capture(Rc::clone(&bytes)))),
        &source_tree.library_dir(),
    );
    let init = vm.init_library();
    assert!(
        init.code.is_ok(),
        "real Tcl 9.0.4 init.tcl failed: {}",
        init.result.to_str()
    );
    let script = format!(
        "package require tcltest\n\
         namespace import -force ::tcltest::*\n\
         set path(test1) {}\n\
         set path(test2) {}\n\
         {io_39_14}\n\
         {io_39_15}\n\
         test io-75.9-native {{strict output conversion keeps its valid prefix}} -setup {{\n\
             set f [open $path(test2) w]\n\
             fconfigure $f -encoding iso8859-1 -profile strict\n\
         }} -body {{\n\
             set code [catch {{puts -nonewline $f \"A\\u2022\"}} msg opts]\n\
             close $f\n\
             list $code [string match {{error writing \"*\": invalid or incomplete multibyte or wide character}} $msg] [dict get $opts -errorcode]\n\
         }} -result [list 1 1 {{POSIX EILSEQ {{invalid or incomplete multibyte or wide character}}}}]\n\
         ::tcltest::cleanupTests\n",
        tcl_syntax::list::list_element(&path1.to_string_lossy()),
        tcl_syntax::list::list_element(&path2.to_string_lossy()),
    );
    let completion = vm
        .eval_source(&script)
        .expect("compile upstream channel cases");
    assert!(
        completion.code.is_ok(),
        "channel tcltest execution failed: {}",
        completion.result.to_str()
    );
    assert_eq!(
        fs::read(path2).expect("read strict output prefix"),
        b"A",
        "io-75.9 strict conversion must retain the valid byte prefix"
    );
    let summary = String::from_utf8(bytes.borrow().clone()).expect("tcltest output is UTF-8");
    assert!(
        summary.contains("Total\t3\tPassed\t3\tSkipped\t0\tFailed\t0"),
        "{summary}"
    );
}

/// The first upstream `set` definition reaches `cleanupTests` through the
/// real init/require path and emits the summary that xtask parses.
#[test]
fn upstream_set_stem_emits_a_parseable_summary_after_real_startup() {
    let Some((ok, error, output)) = run_upstream_definitions(
        "set.test",
        "test set-1.1 {",
        "test set-1.2 {",
        "tcltest-set-stem",
    ) else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert!(ok, "focused upstream set.test failed: {error}\n{output}");
    assert!(
        output.contains("Total\t1\tPassed\t1\tSkipped\t0\tFailed\t0"),
        "missing parseable upstream set-1.1 summary: {output:?}"
    );
}

/// Tcl 9.0.4's canonical array-read trace mutation cases run verbatim through
/// the real startup and `tcltest` package. They exercise key snapshotting,
/// element removal, and whole-array destruction during `array get`.
#[test]
fn upstream_array_get_read_trace_definitions_pass_after_real_startup() {
    let Some((ok, error, output)) = run_upstream_definitions(
        "trace.test",
        "test trace-1.11 {",
        "# Basic write-tracing",
        "tcltest-array-get-read-traces",
    ) else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert!(ok, "focused upstream trace.test failed: {error}\n{output}");
    assert!(
        output.contains("Total\t4\tPassed\t4\tSkipped\t0\tFailed\t0"),
        "missing parseable upstream trace-1.11/1.14 summary: {output:?}"
    );
}

/// The four upstream `dict info` definitions are executed verbatim through
/// Tcl 9.0.4's real `tcltest` package. They cover a successful result, both
/// arity errors, and malformed-dictionary validation without restating the
/// upstream bodies in this harness.
#[test]
fn upstream_dict_info_definitions_pass_after_real_startup() {
    let Some((ok, error, output)) = run_upstream_definitions(
        "dict.test",
        "test dict-10.1 {",
        "test dict-11.1 {",
        "tcltest-dict-info",
    ) else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert!(ok, "focused upstream dict.test failed: {error}\n{output}");
    assert!(
        output.contains("Total\t4\tPassed\t4\tSkipped\t0\tFailed\t0"),
        "missing parseable upstream dict-10 summary: {output:?}"
    );
}

/// Tcl 9.0.4's array-search mutation tests require `try` to bind the body's
/// complete error options, including `-errorinfo`.
#[test]
fn upstream_var_search_mutation_handlers_receive_errorinfo() {
    let Some((ok, error, output)) = run_upstream_definitions(
        "var.test",
        "test var-23.10 {",
        "test var-23.12 {",
        "tcltest-var-search-mutation",
    ) else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert!(ok, "focused upstream var.test failed: {error}\n{output}");
    assert!(
        output.contains("Total\t2\tPassed\t2\tSkipped\t0\tFailed\t0"),
        "missing parseable upstream var-23.10/23.11 summary: {output:?}"
    );
}

/// Tcl 9.0.4's canonical shifted-context cases prove that `UP 1` is stamped
/// where the inner command errors, even when that error is caught before the
/// surrounding `uplevel` invocation returns.
#[test]
fn upstream_error_shifted_context_definitions_pass_after_real_startup() {
    let Some((ok, error, output)) = run_upstream_definitions(
        "error.test",
        "test error-4.6 {",
        "test error-5.1 {",
        "tcltest-error-shifted-context",
    ) else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert!(ok, "focused upstream error.test failed: {error}\n{output}");
    assert!(
        output.contains("Total\t3\tPassed\t3\tSkipped\t0\tFailed\t0"),
        "missing parseable upstream error-4.6/4.7/4.8 summary: {output:?}"
    );
}

#[test]
fn upstream_error_stack_reset_preserves_shifted_context() {
    let Some((ok, error, output)) = run_upstream_definitions(
        "error.test",
        "test error-6.10 {",
        "test error-7.1 {",
        "tcltest-error-stack-reset",
    ) else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert!(ok, "focused upstream error.test failed: {error}\n{output}");
    assert!(
        output.contains("Total\t1\tPassed\t1\tSkipped\t0\tFailed\t0"),
        "missing parseable upstream error-6.10 summary: {output:?}"
    );
}

#[test]
fn upstream_invalid_error_stack_preserves_shifted_context() {
    let Some((ok, error, output)) = run_upstream_definitions(
        "result.test",
        "test result-6.4 {",
        "# cleanup",
        "tcltest-result-error-stack",
    ) else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert!(ok, "focused upstream result.test failed: {error}\n{output}");
    assert!(
        output.contains("Total\t2\tPassed\t2\tSkipped\t0\tFailed\t0"),
        "missing parseable upstream result-6.4/6.5 summary: {output:?}"
    );
}

/// Tcl's own active-frame namespace deletion block exercises immediate and
/// deferred command/variable teardown, including re-entrant delete traces.
#[test]
fn upstream_namespace_active_deletion_definitions_pass_after_real_startup() {
    let Some((ok, error, output)) = run_upstream_definitions(
        "namespace.test",
        "test namespace-7.1 {",
        "test namespace-7.7 {",
        "tcltest-namespace-active-delete",
    ) else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert!(
        ok,
        "focused upstream namespace.test failed: {error}\n{output}"
    );
    assert!(
        output.contains("Total\t6\tPassed\t6\tSkipped\t0\tFailed\t0"),
        "missing parseable upstream namespace-7.1..7.6 summary: {output:?}"
    );
}
