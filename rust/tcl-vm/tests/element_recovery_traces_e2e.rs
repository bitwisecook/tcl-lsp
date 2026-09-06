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

//! Cross-version vectors for the array element a trace access does not name
//! (issue #1633 rows 6 and 7).
//!
//! Tcl 9.0 added the recovery in three places at once —
//! `TclVarFindHiddenArray` (`tclInt.h` 9.0.4:866), the `part2` refill in
//! `TclCallVarTraces` (`tclTrace.c` 9.0.4:2560-2565) and the same refill in
//! `UnsetVarStruct` (`tclVar.c` 9.0.4:2638-2642). 8.4/8.5/8.6 carry none of
//! them, so the identical script reports different callback arguments per
//! release:
//!
//! * `upvar #0 a(k) e; set e 5` fires the *array's* traces as well as the
//!   element's and reports `name2 = k` at 9.x; at 8.x only the element's own
//!   fire, with an empty `name2`.
//! * An element unset named by the **one-part** `a(k)` spelling, so `part2`
//!   starts empty, reports `name1 = a(k)` at 9.x and `name1 = a` at 8.x. A
//!   *two-part* element unset (`array unset a k`, the `INST_UNSET_ARRAY*`
//!   opcodes) supplies `part2` itself and reports `name1 = a` at every release.
//!
//! The sheet spells the one-part unset dynamically (`set nm a; unset
//! ${nm}(k)`) because C's answer for the *literal* `unset a(k)` depends on
//! whether the enclosing script was byte-compiled: `TclCompileUnsetCmd` emits
//! the two-part `INST_UNSET_ARRAY_STK` (so `name1 = a`), while an uncompiled
//! evaluation reaches `Tcl_UnsetObjCmd` and the one-part form. Measured both
//! ways on 9.0.4 — a script file given to `tclsh` reports `a(k)`, the same
//! text piped to its stdin reports `a`. A dynamic name is never compiled to
//! the two-part opcode, so it pins the recovery itself rather than the
//! compiler's choice.
//!
//! The axis is a `tcl-dialect` fact, `TclVersion::
//! traces_recover_linked_array_element`, so both engines read one truth table.
//! Measured on tclsh 8.4.20, 8.5.19, 8.6.16, 9.0.4 and 9.1b0.

use std::cell::RefCell;
use std::io::Write as _;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::{CompileService, Vm};

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
    let service = BytecodeCompileService::for_profile(profile);
    let asm = service
        .compile_for_profile(src, profile)
        .expect("test script compiles for its selected profile");
    let capture = Capture::default();
    let mut vm = Vm::with_output(Box::new(capture.clone()));
    vm.set_compiler(Box::new(service));
    vm.set_runtime_version(version);
    let completion = vm.run_module(&asm);
    assert!(
        completion.code.is_ok(),
        "VM run failed: {}",
        completion.result.to_str()
    );
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

/// Proc callbacks, not `apply`: 8.4 has neither `apply` nor `lassign`.
const SCRIPT: &str = "\
proc A {n1 n2 op} { puts \"A n1=<$n1> n2=<$n2> op=$op\" }\n\
proc E {n1 n2 op} { puts \"E n1=<$n1> n2=<$n2> op=$op\" }\n\
array set a {k v}\n\
trace add variable a write A\n\
trace add variable a(k) write E\n\
upvar #0 a(k) e\n\
set e 5\n\
trace add variable a unset A\n\
trace add variable a(k) unset E\n\
set nm a\n\
unset ${nm}(k)\n\
array set b {k v}\n\
trace add variable b(k) unset E\n\
upvar #0 b(k) f\n\
unset f\n\
puts \"b(k) exists after alias unset: [info exists b(k)]\"\n\
array set g {k v}\n\
trace add variable g unset A\n\
trace add variable g(k) unset E\n\
array unset g k\n\
array set d {k v}\n\
trace add variable d write A\n\
trace add variable d(k) write E\n\
set d(k) 2\n";

const EXPECT_8X: &str = "\
E n1=<e> n2=<> op=write\n\
A n1=<a> n2=<k> op=unset\n\
E n1=<a> n2=<k> op=unset\n\
E n1=<f> n2=<> op=unset\n\
b(k) exists after alias unset: 0\n\
A n1=<g> n2=<k> op=unset\n\
E n1=<g> n2=<k> op=unset\n\
A n1=<d> n2=<k> op=write\n\
E n1=<d> n2=<k> op=write";

const EXPECT_9X: &str = "\
A n1=<e> n2=<k> op=write\n\
E n1=<e> n2=<k> op=write\n\
A n1=<a(k)> n2=<k> op=unset\n\
E n1=<a(k)> n2=<k> op=unset\n\
E n1=<f> n2=<k> op=unset\n\
b(k) exists after alias unset: 0\n\
A n1=<g> n2=<k> op=unset\n\
E n1=<g> n2=<k> op=unset\n\
A n1=<d> n2=<k> op=write\n\
E n1=<d> n2=<k> op=write";

struct Vector {
    version: TclVersion,
    expected: &'static str,
    env: &'static str,
    tclsh: &'static [&'static str],
}

const VECTORS: &[Vector] = &[
    Vector {
        version: TclVersion::V8_4,
        expected: EXPECT_8X,
        env: "TCL_LSP_TCLSH84",
        tclsh: &["tclsh8.4"],
    },
    Vector {
        version: TclVersion::V8_5,
        expected: EXPECT_8X,
        env: "TCL_LSP_TCLSH85",
        tclsh: &["tclsh8.5"],
    },
    Vector {
        version: TclVersion::V8_6,
        expected: EXPECT_8X,
        env: "TCL_LSP_TCLSH86",
        tclsh: &["tclsh8.6"],
    },
    Vector {
        version: TclVersion::V9_0,
        expected: EXPECT_9X,
        env: "TCL_LSP_TCLSH90",
        tclsh: &["tclsh9.0"],
    },
    Vector {
        version: TclVersion::V9_1,
        expected: EXPECT_9X,
        env: "TCL_LSP_TCLSH91",
        tclsh: &["tclsh9.1"],
    },
];

#[test]
fn element_recovery_follows_the_selected_release() {
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

/// The release axis is a dialect fact, so a profile edit moves both engines.
#[test]
fn the_dialect_owns_the_release_boundary() {
    for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        assert!(
            !version.traces_recover_linked_array_element(),
            "{version:?}"
        );
    }
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        assert!(version.traces_recover_linked_array_element(), "{version:?}");
    }
}
