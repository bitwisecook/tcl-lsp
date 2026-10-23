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

//! Cross-version vectors for the array element a trace access does not name.
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
//! Runs with matching actual cores/source compilers and every pinned C oracle;
//! an absent or invalid interpreter fails the comparison.

mod common;

use tcl_dialect::TclVersion;
use tcl_test_support::required_tclshs;

fn vm_output(source: &str, version: TclVersion) -> String {
    common::vm_output(source, version.dialect_name())
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
}

const VECTORS: &[Vector] = &[
    Vector {
        version: TclVersion::V8_4,
        expected: EXPECT_8X,
    },
    Vector {
        version: TclVersion::V8_5,
        expected: EXPECT_8X,
    },
    Vector {
        version: TclVersion::V8_6,
        expected: EXPECT_8X,
    },
    Vector {
        version: TclVersion::V9_0,
        expected: EXPECT_9X,
    },
    Vector {
        version: TclVersion::V9_1,
        expected: EXPECT_9X,
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
    for oracle in required_tclshs(&TclVersion::ALL).expect("all five pinned C Tcl oracles") {
        let vector = VECTORS
            .iter()
            .find(|vector| vector.version == oracle.version)
            .expect("every pinned release has an original recovery vector");
        assert_eq!(
            common::oracle_output(&oracle.path, SCRIPT),
            vector.expected,
            "{}",
            oracle.patchlevel
        );
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
