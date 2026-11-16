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

//! Safe interpreters: inherited safety, the measured hidden command sets and what stays callable.

use super::*;

/// A safe parent's children are safe with or without `-safe`, at every
/// level, including a child addressed by path; a `marktrusted` parent's are
/// not (tclsh 8.6.18 / 9.0.4 agree).
#[test]
fn children_of_a_safe_interp_inherit_safety() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                b"interp create -safe s; interp create n; list \
                      [s eval {interp issafe [interp create]}] \
                      [s eval {interp issafe [interp create -safe]}] \
                      [s eval {interp create t; t eval {interp issafe [interp create]}}] \
                      [n eval {interp issafe [interp create]}] \
                      [s eval {interp issafe t}] \
                      [interp issafe [interp create {s p}]]"
            ),
            b"1 1 1 0 1 1"
        );
        assert_eq!(
            ok(
                i,
                b"interp marktrusted s; s eval {interp issafe [interp create]}"
            ),
            b"0"
        );
    });
}

/// Every command `interp create -safe` hides, per release, measured on
/// the reference interpreters with
/// `interp create -safe s; lsort [interp hidden s]` — top-level command
/// names only. The `tcl:file:*` / `tcl:zipfs:*` / `tcl:clock:*` entries a
/// real 8.6+ interpreter also lists are C's internal rewrite names for
/// the *unsafe subcommands* of an ensemble, not commands a script can
/// name; this runtime does not model ensembles that way.
///
/// Patch levels measured: 8.4.20, 8.5.19, 8.6.14, 9.0.4, 9.1b0.
#[cfg(all(test, have_tommath))]
const MEASURED_SAFE_HIDDEN: &[(tcl_dialect::TclVersion, &[&str])] = &[
    (
        tcl_dialect::TclVersion::V8_4,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
        ],
    ),
    // 8.5 adds `unload` (TIP 100); 8.6 is the same set.
    (
        tcl_dialect::TclVersion::V8_5,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
        ],
    ),
    (
        tcl_dialect::TclVersion::V8_6,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
        ],
    ),
    // 9.0 adds `zipfs`.
    (
        tcl_dialect::TclVersion::V9_0,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
            "zipfs",
        ],
    ),
    // 9.1 additionally lists `clock` — an artefact of the safe base
    // hiding the C `clock` and immediately re-providing a safe one, not
    // an unsafety fact, so it is deliberately absent from the registry
    // trait (`clock format 0 -gmt 1` works inside a 9.1 safe child).
    (
        tcl_dialect::TclVersion::V9_1,
        &[
            "cd",
            "clock",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
            "zipfs",
        ],
    ),
];

/// The hidden set under each pinned release is exactly the measured
/// tclsh set, narrowed to the commands this runtime carries — `make_safe`
/// reads that set from the registry's `Traits::SAFE_INTERP_HIDDEN` query
/// rather than a hand-kept name list.
///
/// The narrowing *is* the per-release mechanism, not a fudge: `unload`
/// (8.5+) and `zipfs` (9.0+) are release-gated commands, so "hide what
/// the trait names, if this interpreter carries it" reproduces the
/// differences between the rows with no second availability rule. Any
/// residue is asserted to be genuinely absent from `info commands`, so
/// implementing one of them forces this test to be revisited.
#[cfg(have_tommath)]
#[test]
fn safe_interp_hidden_set_matches_the_measured_tclsh_sets() {
    for &(version, measured) in MEASURED_SAFE_HIDDEN {
        leak_free(|i| {
            i.set_runtime_version(version);
            // `clock` is implemented and stays *visible*, which is what a
            // real 9.1 safe child does behaviourally; only its appearance
            // in `interp hidden` differs.
            let carried: Vec<&str> = measured
                .iter()
                .copied()
                .filter(|name| {
                    *name != "clock" && {
                        let script = format!("llength [info commands {name}]");
                        ok(i, script.as_bytes()) == b"1"
                    }
                })
                .collect();
            assert_eq!(i.eval_str(b"set s [interp create -safe]"), Code::Ok);
            let hidden = ok(i, b"lsort [$s hidden]");
            let hidden = String::from_utf8_lossy(&hidden);
            let actual: Vec<&str> = hidden.split_whitespace().collect();
            assert_eq!(actual, carried, "{version:?} hidden set");
            i.eval_str(b"interp delete $s");
        });
    }
}

/// TP: `clock` stays callable inside a safe child on every release —
/// measured on tclsh 8.6.14, 9.0.4 and 9.1b0, where
/// `s eval {clock format 0 -gmt 1}` succeeds even though 9.1 lists
/// `clock` in `interp hidden`.
#[cfg(have_tommath)]
#[test]
fn clock_remains_callable_in_a_safe_child() {
    for &(version, _) in MEASURED_SAFE_HIDDEN {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(i.eval_str(b"set s [interp create -safe]"), Code::Ok);
            assert_eq!(
                ok(i, b"$s eval {clock format 0 -gmt 1 -format %Y}"),
                b"1970",
                "{version:?}"
            );
            i.eval_str(b"interp delete $s");
        });
    }
}
