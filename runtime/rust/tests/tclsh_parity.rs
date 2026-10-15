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

//! Scripts read, compiled and reported as each `tclsh` does (#2474).
//!
//! The vectors are tclvm's (`rust/tcl-vm/tests/data/tclsh_parity`): each
//! script runs as the script file `tclsh` is given
//! (`Interp::eval_sourced_completion`, its uncaught error reported by its
//! `errorInfo`) and on its stdin (`Interp::eval_compiled_completion`, a failed one
//! reported by its result after the output, as `tclsh` writes it to
//! stderr), and the output is compared with what tclsh 8.4.20, 8.5.19,
//! 8.6.18, 9.0.4 and 9.1.0 print for it (`<name>.expect`, which tclvm's
//! test compares with each `tclsh` live).

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use tcl_dialect::TclVersion;
use tcl_host_native::NativeHost;
use tcl_platform::{Capabilities, Clock, Env, Filesystem, Host, Process, StdIo};
use tcl_runtime::interp::Interp;
use tcl_runtime::CompletionCode;

/// The native host, its stdout captured.
struct CaptureHost {
    native: NativeHost,
    stdout: RefCell<Vec<u8>>,
}

impl StdIo for CaptureHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.stdout.borrow_mut().extend_from_slice(bytes);
    }

    fn write_stderr(&self, _bytes: &[u8]) {}
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

    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.native.system_encoding()
    }

    fn filesystem(&self) -> Option<&dyn Filesystem> {
        self.native.filesystem()
    }

    fn process(&self) -> Option<&dyn Process> {
        self.native.process()
    }
}

const RELEASES: [TclVersion; 5] = [
    TclVersion::V8_4,
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];

#[derive(Clone, Copy, Debug)]
enum Entry {
    File,
    Stdin,
}

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../rust/tcl-vm/tests/data/tclsh_parity")
}

/// The section of `<name>.expect` for `release` and `entry`.
fn expected(name: &str, release: TclVersion, entry: Entry) -> String {
    let text = std::fs::read_to_string(data_dir().join(format!("{name}.expect")))
        .expect("expectation file");
    let entry = match entry {
        Entry::File => "file",
        Entry::Stdin => "stdin",
    };
    let header = format!("==== {} {entry} ====\n", release.version_string());
    let start = text.find(&header).expect("section") + header.len();
    let end = text[start..]
        .find("\n==== ")
        .map_or(text.len(), |at| start + at + 1);
    text[start..end].to_owned()
}

/// The vectors the runtime runs: every script in the data directory but
/// those named in `runtime.skip` there, one per line.
fn vectors() -> Vec<(String, String)> {
    let skip = std::fs::read_to_string(data_dir().join("runtime.skip")).unwrap_or_default();
    let mut names: Vec<String> = std::fs::read_dir(data_dir())
        .expect("data directory")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".tcl").map(str::to_owned)
        })
        .filter(|name| !skip.lines().any(|line| line.trim() == name))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let script =
                std::fs::read_to_string(data_dir().join(format!("{name}.tcl"))).expect("script");
            (name, script)
        })
        .collect()
}

/// The `-errorinfo` of a completion's options.
fn error_info(options: &[u8]) -> String {
    let options = String::from_utf8_lossy(options);
    let pairs = tcl_syntax::list::split_list(&options).expect("options list");
    pairs
        .chunks(2)
        .find(|pair| pair[0] == "-errorinfo")
        .map(|pair| pair[1].to_string())
        .unwrap_or_default()
}

/// What the runtime prints for `script` handed over as `entry`.
fn runtime_output(script: &str, release: TclVersion, entry: Entry) -> String {
    let host = Rc::new(CaptureHost {
        native: NativeHost::new(),
        stdout: RefCell::new(Vec::new()),
    });
    let mut interp = Interp::with_host(host.clone());
    interp.set_runtime_version(release);
    let tail = match entry {
        Entry::File => {
            let completion = interp.eval_sourced_completion(script.as_bytes(), b"parity.tcl");
            if completion.code == CompletionCode::Ok {
                String::new()
            } else {
                format!("{}\n", error_info(&completion.options))
            }
        }
        Entry::Stdin => {
            let completion = interp.eval_compiled_completion(script.as_bytes());
            if completion.code == CompletionCode::Ok {
                String::new()
            } else {
                format!(
                    "--- stderr\n{}\n",
                    String::from_utf8_lossy(&completion.result)
                )
            }
        }
    };
    drop(interp);
    let stdout = host.stdout.borrow().clone();
    format!("{}{tail}", String::from_utf8(stdout).expect("utf-8"))
}

#[test]
fn the_runtime_reads_each_script_as_each_tclsh_does() {
    for (name, script) in vectors() {
        for release in RELEASES {
            for entry in [Entry::File, Entry::Stdin] {
                assert_eq!(
                    runtime_output(&script, release, entry),
                    expected(&name, release, entry),
                    "{name} on {release:?} from {entry:?}"
                );
            }
        }
    }
}
