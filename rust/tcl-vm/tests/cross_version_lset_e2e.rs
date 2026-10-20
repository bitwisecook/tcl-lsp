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

//! `lset` and the index equal to a list's length: an append from 8.6, a
//! `list index out of range` raise on 8.4 and 8.5 (`TclLsetFlat`,
//! `tclListObj.c`: `index >= elemCount` through 8.5, `index > elemCount` from
//! 8.6).
//!
//! Each vector runs through the bytecode VM at every release — both as a
//! compiled `lset` and through the runtime builtin the compiler does not
//! inline — and, when the matching real tclsh is on PATH, under it too, so the
//! table cannot drift from C Tcl.

use std::cell::RefCell;
use std::io::Write as _;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::Vm;

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
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let cap = Capture::default();
    let mut vm = Vm::with_native_core(
        Box::new(cap.clone()),
        std::rc::Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("native core for the selected profile");
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let _ = vm.try_eval_source(src).expect("original source provider");
    String::from_utf8_lossy(&cap.0.borrow()).trim().to_string()
}

fn tclsh_output(version: TclVersion, src: &str) -> Option<String> {
    let mut child = std::process::Command::new(format!("tclsh{}", version.version_string()))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(src.as_bytes())
        .expect("write");
    let out = child.wait_with_output().expect("run");
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The expected stdout of one vector under each release, in `TclVersion::ALL`
/// order.
struct Vector {
    name: &'static str,
    script: &'static str,
    want: [&'static str; 5],
}

const RAISES: &str = "1:list index out of range";

const VECTORS: &[Vector] = &[
    Vector {
        name: "the length appends from 8.6",
        script: "set x {a b c}\nputs [catch {lset x 3 Z} m]:$m\n",
        want: [RAISES, RAISES, "0:a b c Z", "0:a b c Z", "0:a b c Z"],
    },
    Vector {
        name: "a braced one-element index list names the same slot",
        script: "set x {a b c}\nputs [catch {lset x {3} Z} m]:$m\n",
        want: [RAISES, RAISES, "0:a b c Z", "0:a b c Z", "0:a b c Z"],
    },
    Vector {
        name: "the empty list's first slot is its length",
        script: "set x {}\nputs [catch {lset x 0 Z} m]:$m\n",
        want: [RAISES, RAISES, "0:Z", "0:Z", "0:Z"],
    },
    Vector {
        name: "a nested level appends at its own length",
        script: "set x {{a b c}}\nputs [catch {lset x 0 3 Z} m]:$m\n",
        want: [RAISES, RAISES, "0:{a b c Z}", "0:{a b c Z}", "0:{a b c Z}"],
    },
    Vector {
        name: "end+1 is the length where the release reads it",
        script: "set x {a b c}\nputs [catch {lset x end+1 Z} m]:$m\n",
        want: [
            "1:bad index \"end+1\": must be integer or end?-integer?",
            RAISES,
            "0:a b c Z",
            "0:a b c Z",
            "0:a b c Z",
        ],
    },
    Vector {
        name: "the runtime builtin follows the release too",
        script: "set x {a b c}\nset c lset\nputs [catch {$c x 3 Z} m]:$m\n",
        want: [RAISES, RAISES, "0:a b c Z", "0:a b c Z", "0:a b c Z"],
    },
    Vector {
        name: "past the length raises under every release",
        script: "set x {a b c}\nputs [catch {lset x 4 Z} m]:$m\n",
        want: [
            RAISES,
            RAISES,
            RAISES,
            "1:index \"4\" out of range",
            "1:index \"4\" out of range",
        ],
    },
];

#[test]
fn lset_at_the_length_follows_the_release() {
    for vector in VECTORS {
        for (version, want) in TclVersion::ALL.into_iter().zip(vector.want) {
            assert_eq!(
                vm_output(vector.script, version),
                want,
                "{} (VM, {})",
                vector.name,
                version.version_string()
            );
        }
    }
}

#[test]
fn lset_at_the_length_agrees_with_the_real_tclsh() {
    for vector in VECTORS {
        for (version, want) in TclVersion::ALL.into_iter().zip(vector.want) {
            if let Some(real) = tclsh_output(version, vector.script) {
                assert_eq!(
                    real,
                    want,
                    "{} (tclsh{})",
                    vector.name,
                    version.version_string()
                );
            }
        }
    }
}
