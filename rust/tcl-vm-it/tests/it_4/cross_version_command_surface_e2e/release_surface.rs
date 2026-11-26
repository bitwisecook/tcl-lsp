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

//! The command surface follows the emulated release: the per-release vectors, checked against real `tclsh` when it is available.

use super::*;

/// One script and what each release must produce (`puts` output, or the
/// compile/runtime error message).
struct Vector {
    pub(super) name: &'static str,
    script: &'static str,
    want_84: &'static str,
    want_85: &'static str,
    want_86: &'static str,
    want_90: &'static str,
}

const VECTORS: &[Vector] = &[
    // The {*} expansion grammar is a compile fact.
    Vector {
        name: "{*} expansion is a hard parse error before 8.5",
        script: "puts [llength [list {*}{a b}]]\n",
        want_84: "extra characters after close-brace",
        want_85: "2",
        want_86: "2",
        want_90: "2",
    },
    // The builtin surface follows the release.
    Vector {
        name: "lassign arrives in 8.5",
        script: "puts [lassign {a b} x]\n",
        want_84: "invalid command name \"lassign\"",
        want_85: "b",
        want_86: "b",
        want_90: "b",
    },
    Vector {
        name: "dict arrives in 8.5",
        script: "puts [dict get {a 1} a]\n",
        want_84: "invalid command name \"dict\"",
        want_85: "1",
        want_86: "1",
        want_90: "1",
    },
    Vector {
        name: "dict info follows the registry's 8.5 surface",
        script: "puts [string match {*entries in table*} [dict info {}]]\n",
        want_84: "invalid command name \"dict\"",
        want_85: "1",
        want_86: "1",
        want_90: "1",
    },
    Vector {
        name: "qualified dict info follows its registry-owned 8.5 surface",
        script: "puts [string match {*entries in table*} [::tcl::dict::info {}]]\n",
        want_84: "invalid command name \"::tcl::dict::info\"",
        want_85: "1",
        want_86: "1",
        want_90: "1",
    },
    // The vector above uses `dict get`, whose argument is a runtime value, so
    // it never reaches the constant folder. An **all-literal** `dict create` in
    // a value position *is* folded in codegen — and a fold is a rewrite that
    // bypasses the runtime's availability gate entirely, so a naive check
    // would let the folded form succeed under 8.4 while every unfoldable
    // spelling correctly raises `invalid command name "dict"`.
    Vector {
        name: "a *foldable* dict create is still absent before 8.5",
        script: "puts [dict create a 1 a 2]\n",
        want_84: "invalid command name \"dict\"",
        want_85: "a 2",
        want_86: "a 2",
        want_90: "a 2",
    },
    Vector {
        name: "try arrives in 8.6",
        script: "puts [catch {try { } } m]$m\n",
        want_84: "1invalid command name \"try\"",
        want_85: "1invalid command name \"try\"",
        want_86: "0",
        want_90: "0",
    },
    Vector {
        name: "lmap arrives in 8.6",
        script: "puts [catch {lmap v {1 2} {expr {$v * 2}}} m]$m\n",
        want_84: "1invalid command name \"lmap\"",
        want_85: "1invalid command name \"lmap\"",
        want_86: "02 4",
        want_90: "02 4",
    },
    Vector {
        name: "lpop arrives in 9.0",
        script: "set l {a b c}\nputs [catch {lpop l} m]$m\n",
        want_84: "1invalid command name \"lpop\"",
        want_85: "1invalid command name \"lpop\"",
        want_86: "1invalid command name \"lpop\"",
        want_90: "0c",
    },
    Vector {
        name: "an unavailable builtin is absent from info commands",
        script: "puts [llength [info commands lassign]]\n",
        want_84: "0",
        want_85: "1",
        want_86: "1",
        want_90: "1",
    },
    // `package`'s option table follows the release.
    // `prefer` is TIP 268 (8.5); `files` is 9.0's. Byte-checked against
    // tclsh8.4.20 / 8.5.19 / 8.6.16 / 9.0.4, which this suite also re-runs
    // against any installed tclsh.
    Vector {
        name: "package's option list follows the release",
        script: "puts [catch {package zz} m]$m\n",
        want_84: "1bad option \"zz\": must be forget, ifneeded, names, present, \
                  provide, require, unknown, vcompare, versions, or vsatisfies",
        want_85: "1bad option \"zz\": must be forget, ifneeded, names, prefer, present, \
                  provide, require, unknown, vcompare, versions, or vsatisfies",
        want_86: "1bad option \"zz\": must be forget, ifneeded, names, prefer, present, \
                  provide, require, unknown, vcompare, versions, or vsatisfies",
        want_90: "1bad option \"zz\": must be files, forget, ifneeded, names, prefer, \
                  present, provide, require, unknown, vcompare, versions, or vsatisfies",
    },
    // The polyfill pattern: a user proc always wins over a hidden builtin.
    Vector {
        name: "a user-defined proc is callable at every release",
        script: "proc lassign {l args} { return polyfill }\nputs [lassign {a b} x]\n",
        want_84: "polyfill",
        want_85: "polyfill",
        want_86: "polyfill",
        want_90: "polyfill",
    },
];

#[test]
fn command_surface_follows_the_emulated_release() {
    for v in VECTORS {
        assert_eq!(
            vm_output(v.script, TclVersion::V8_4),
            v.want_84,
            "[8.4] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V8_5),
            v.want_85,
            "[8.5] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V8_6),
            v.want_86,
            "[8.6] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V9_0),
            v.want_90,
            "[9.0] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V9_1),
            v.want_90,
            "[9.1] {}",
            v.name
        );
    }
}

/// A `catch` body is compiled at run time through the VM's compile service,
/// so an 8.5+-only spelling inside it is a *catchable* error under an 8.4
/// pin — matching C Tcl, which defers a compile-time parse error to a
/// bytecode instruction that raises it at runtime.
#[test]
fn expansion_parse_error_inside_catch_is_catchable_at_8_4() {
    let src = "puts [catch {llength [list {*}{a b}]} m]\nputs $m\n";
    assert_eq!(
        vm_output(src, TclVersion::V8_4),
        "1\nextra characters after close-brace"
    );
    assert_eq!(vm_output(src, TclVersion::V9_0), "0\n2");
}

/// Byte-compare the value-producing vectors against a real interpreter when
/// one is installed; skips silently otherwise. Error-message vectors are
/// excluded — tclsh reports those on stderr with a traceback, so only the
/// `catch`-shaped vectors (whose divergence is on stdout) are comparable.
#[test]
fn vectors_match_real_tclsh_when_available() {
    let mut checked = 0usize;
    for v in VECTORS {
        if !v.script.contains("catch") && !v.script.contains("info commands") {
            continue;
        }
        for (env, names, want) in [
            ("TCLSH84", &["tclsh8.4"][..], v.want_84),
            ("TCLSH86", &["tclsh8.6"][..], v.want_86),
            ("TCLSH90", &["tclsh9.0"][..], v.want_90),
        ] {
            if let Some(out) = tclsh_output(env, names, v.script) {
                assert_eq!(out, want, "[{env}] {}", v.name);
                checked += 1;
            }
        }
    }
    if checked == 0 {
        eprintln!("no system tclsh found — pinned expectations still verified");
    }
}
