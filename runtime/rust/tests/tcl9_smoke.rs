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

//! Byte-for-byte runner for the small, primitive-oriented Tcl 9 smoke corpus.
//!
//! The corpus used to have only a Python driver. Keep this test in the
//! standalone runtime workspace so the same interpreter path as the
//! `run_script` example owns and executes the contract without another
//! language-specific harness.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use tcl_dialect::TclVersion;
use tcl_host_native::NativeHost;
use tcl_platform::{Capabilities, Clock, Env, Filesystem, Host, Process, StdIo};
use tcl_runtime::interp::{Code, Interp};

const BASELINE_SAMPLE_COUNT: usize = 10;

/// Delegate every real native capability except stdio, which the oracle needs
/// to capture without redirecting process-global file descriptors.
struct CaptureHost {
    native: NativeHost,
    stdout: RefCell<Vec<u8>>,
    stderr: RefCell<Vec<u8>>,
}

impl CaptureHost {
    fn new() -> Self {
        Self {
            native: NativeHost::new(),
            stdout: RefCell::new(Vec::new()),
            stderr: RefCell::new(Vec::new()),
        }
    }

    fn stdout(&self) -> Vec<u8> {
        self.stdout.borrow().clone()
    }

    fn stderr(&self) -> Vec<u8> {
        self.stderr.borrow().clone()
    }
}

impl StdIo for CaptureHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.stdout.borrow_mut().extend_from_slice(bytes);
    }

    fn write_stderr(&self, bytes: &[u8]) {
        self.stderr.borrow_mut().extend_from_slice(bytes);
    }
}

impl Host for CaptureHost {
    fn capabilities(&self) -> Capabilities {
        self.native.capabilities()
    }

    fn clock(&self) -> &dyn Clock {
        self.native.clock()
    }

    fn stdio(&self) -> &dyn StdIo {
        self
    }

    fn env(&self) -> &dyn Env {
        self.native.env()
    }

    fn filesystem(&self) -> Option<&dyn Filesystem> {
        self.native.filesystem()
    }

    fn process(&self) -> Option<&dyn Process> {
        self.native.process()
    }
}

fn collect_paths(root: &Path, extension: &str) -> Vec<PathBuf> {
    fn visit(directory: &Path, extension: &str, paths: &mut Vec<PathBuf>) {
        let mut entries = std::fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
            .map(|entry| entry.expect("read corpus directory entry").path())
            .collect::<Vec<_>>();
        entries.sort();

        for path in entries {
            if path.is_dir() {
                visit(&path, extension, paths);
            } else if path.extension().is_some_and(|ext| ext == extension) {
                paths.push(path);
            }
        }
    }

    let mut paths = Vec::new();
    visit(root, extension, &mut paths);
    paths
}

fn rendered(bytes: &[u8]) -> String {
    let escaped = bytes
        .iter()
        .flat_map(|byte| std::ascii::escape_default(*byte))
        .map(char::from)
        .collect::<String>();
    format!("b\"{escaped}\"")
}

#[test]
fn failure_rendering_preserves_every_byte() {
    assert_eq!(
        rendered(&[0x80, b'\n', b'"', b'\\', b'A']),
        "b\"\\x80\\n\\\"\\\\A\""
    );
}

#[test]
fn every_tcl9_smoke_sample_matches_its_oracle() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let corpus = repository.join("samples/tcl9_smoke");
    let samples = collect_paths(&corpus, "tcl");
    let expectations = collect_paths(&corpus, "expected");

    assert!(
        samples.len() >= BASELINE_SAMPLE_COUNT,
        "Tcl 9 smoke corpus shrank or became vacuous: found {}, expected at least {BASELINE_SAMPLE_COUNT}",
        samples.len()
    );

    for expected in &expectations {
        let sample = expected.with_extension("tcl");
        assert!(
            sample.is_file(),
            "orphaned expectation has no .tcl sample: {}",
            expected.display()
        );
    }

    let mut failures = Vec::new();
    for sample in samples {
        let expected_path = sample.with_extension("expected");
        assert!(
            expected_path.is_file(),
            "sample has no .expected oracle: {}",
            sample.display()
        );

        let script = std::fs::read(&sample)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", sample.display()));
        let expected = std::fs::read(&expected_path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", expected_path.display()));
        let source_path = sample
            .strip_prefix(&repository)
            .expect("corpus lives under repository root");
        let source_name = source_path.as_os_str().as_encoded_bytes();

        let host = Rc::new(CaptureHost::new());
        let mut interp = Interp::new();
        interp.set_runtime_version(TclVersion::V9_0);
        interp.set_host(host.clone());
        let code = interp.eval_sourced(&script, source_name);
        let actual = host.stdout();

        if code != Code::Ok || actual != expected {
            failures.push(format!(
                "{}: code={code:?}, result={}, stderr={}\n  expected {}\n    actual {}",
                source_path.display(),
                rendered(&interp.result_bytes()),
                rendered(&host.stderr()),
                rendered(&expected),
                rendered(&actual),
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "Tcl 9 smoke corpus failures:\n{}",
        failures.join("\n")
    );
}
