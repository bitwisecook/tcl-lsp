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

//! **Scripts read, compiled and reported as each `tclsh` does** (#2474).
//!
//! Each `tests/data/tclsh_parity/<name>.tcl` runs as the script file
//! `tclsh` is given (`Vm::eval_file`, its uncaught error reported by its
//! `errorInfo`) and on its stdin (`Vm::eval_source_commands`, each failed
//! chunk reported by its result after the output, as `tclsh` writes it to
//! stderr). `<name>.expect` holds what tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4
//! and 9.1.0 print for it, section by section; the test compares the VM
//! with it, and with the release's `tclsh` again wherever one is installed.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;

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

/// How `tclsh` is handed the script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    File,
    Stdin,
}

impl Entry {
    const fn name(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Stdin => "stdin",
        }
    }
}

const RELEASES: [TclVersion; 5] = [
    TclVersion::V8_4,
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];

/// The name the script file is run under, which its `(file …)` frame quotes.
const FILE_NAME: &str = "parity.tcl";

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/tclsh_parity")
}

/// The vectors: every script in the data directory.
fn vectors() -> Vec<(String, String)> {
    let mut names: Vec<String> = std::fs::read_dir(data_dir())
        .expect("data directory")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".tcl").map(str::to_owned)
        })
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

/// The section of `<name>.expect` for `release` and `entry`.
fn expected(name: &str, release: TclVersion, entry: Entry) -> String {
    let text = std::fs::read_to_string(data_dir().join(format!("{name}.expect")))
        .expect("expectation file");
    let header = format!("==== {} {} ====\n", release.version_string(), entry.name());
    let start = text.find(&header).expect("section") + header.len();
    let end = text[start..]
        .find("\n==== ")
        .map_or(text.len(), |at| start + at + 1);
    text[start..end].to_owned()
}

/// Run `src` on a VM configured as `tclvm --tcl-version` configures one.
fn vm_output(src: &str, release: TclVersion, entry: Entry) -> String {
    let profile = tcl_registry::model::resolve_environment(release.dialect_name()).unit_profile();
    let capture = Capture::default();
    // Work's entered source runs on a native-core interpreter (the genuine
    // compiler provider admits each compile); the profile is pinned at
    // construction.
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(capture.clone()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let mut tail = String::new();
    match entry {
        Entry::File => {
            let completion = vm.eval_file(FILE_NAME, src, true);
            if !completion.code.is_ok() {
                let options = vm.return_options(&completion);
                let pairs = options.as_list().expect("options");
                let info = pairs
                    .chunks(2)
                    .find(|pair| &*pair[0].to_str() == "-errorinfo")
                    .map(|pair| pair[1].to_str().to_string())
                    .unwrap_or_default();
                tail = format!("{info}\n");
            }
        }
        Entry::Stdin => vm.eval_source_commands(src, |_, message| {
            tail.push_str(message);
            tail.push('\n');
        }),
    }
    let out = String::from_utf8(capture.0.borrow().clone()).expect("utf-8");
    match entry {
        Entry::Stdin if !tail.is_empty() => format!("{out}--- stderr\n{tail}"),
        _ => format!("{out}{tail}"),
    }
}

/// Run `src` under the release's `tclsh` as `entry` hands it over, `None`
/// where no such interpreter is installed.
fn tclsh_output(src: &str, release: TclVersion, entry: Entry) -> Option<String> {
    let tclsh = tcl_test_support::witness_tclsh(release)?;
    let dir = std::env::temp_dir().join(format!(
        "tclsh-parity-{}-{}",
        std::process::id(),
        release.version_string()
    ));
    std::fs::create_dir_all(&dir).expect("scratch directory");
    std::fs::write(dir.join(FILE_NAME), src).expect("write script");
    let mut command = std::process::Command::new(&tclsh.path);
    command
        .current_dir(&dir)
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8");
    let output = match entry {
        Entry::File => {
            let output = command.arg(FILE_NAME).output().expect("run tclsh");
            let mut out = output.stdout;
            if !output.status.success() {
                out.extend_from_slice(&output.stderr);
            }
            out
        }
        Entry::Stdin => {
            use std::io::Write as _;
            let mut child = command
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("run tclsh");
            child
                .stdin
                .take()
                .expect("stdin")
                .write_all(src.as_bytes())
                .expect("feed stdin");
            let output = child.wait_with_output().expect("tclsh output");
            let mut out = output.stdout;
            if !output.stderr.is_empty() {
                out.extend_from_slice(b"--- stderr\n");
                out.extend_from_slice(&output.stderr);
            }
            out
        }
    };
    let _ = std::fs::remove_dir_all(&dir);
    Some(String::from_utf8(output).expect("utf-8"))
}

/// Every vector, release and entry is compared; the failure lists each
/// mismatch. `TCLSH_PARITY_VECTOR=<name>` narrows the run to one vector.
#[test]
fn the_vm_reads_each_script_as_each_tclsh_does() {
    let only = std::env::var("TCLSH_PARITY_VECTOR").ok();
    let mut mismatches = Vec::new();
    for (name, script) in vectors() {
        if only.as_deref().is_some_and(|only| only != name) {
            continue;
        }
        for release in RELEASES {
            for entry in [Entry::File, Entry::Stdin] {
                let actual = vm_output(&script, release, entry);
                let expected = expected(&name, release, entry);
                if actual != expected {
                    eprintln!(
                        "MISMATCH {name} on {release:?} from {entry:?}\n--- expected\n{expected}--- actual\n{actual}--- end"
                    );
                    mismatches.push(format!("{name} {release:?} {entry:?}"));
                }
            }
        }
    }
    assert!(mismatches.is_empty(), "mismatches: {mismatches:#?}");
}

#[test]
fn the_expectations_match_each_tclsh() {
    for (name, script) in vectors() {
        for release in RELEASES {
            for entry in [Entry::File, Entry::Stdin] {
                if let Some(out) = tclsh_output(&script, release, entry) {
                    assert_eq!(
                        out,
                        expected(&name, release, entry),
                        "tclsh {release:?}: {name} from {entry:?}"
                    );
                }
            }
        }
    }
}
