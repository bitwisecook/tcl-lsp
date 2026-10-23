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

//! Cross-version vectors for the deprecated `trace variable|vdelete|vinfo`
//! forms.
//!
//! C compiles them behind `#ifndef TCL_REMOVE_OBSOLETE_TRACES`
//! (`tclTrace.c` 8.6.16:198-206); Tcl 9.0 dropped them, so the very same
//! script is a working legacy trace at 8.x and a `bad option` at 9.x. The
//! registry states that boundary (`SpecSurface::TCL8X` on the three
//! subcommands), and the VM reads it rather than carrying its own list.
//!
//! The ops word is the `rwua` letter concatenation, expanded and validated by
//! `tcl_cmd_core::trace::parse_legacy_variable_ops` — so a non-`rwua` byte is
//! an error rather than a silently-installed never-firing trace, repeats
//! collapse, and the stored set is the same canonical set `trace add
//! variable` produces (`vdelete` therefore removes an `add`-installed trace).
//! `trace vinfo` renders that set back as letters in C's fixed `r`, `w`, `u`,
//! `a` order, unlike `trace info variable`'s word list.

mod common;

use tcl_dialect::TclVersion;
use tcl_registry::CommandRegistry;
use tcl_test_support::{JimCapability, require_jimsh, required_tclshs};

fn vm_output(source: &str, version: TclVersion) -> String {
    common::vm_output(source, version.dialect_name())
}

/// Runs to completion on every release: each line is `catch`-wrapped so the
/// 9.x `bad option` errors do not abort the script.
const SCRIPT: &str = "\
proc cb args { puts \"fired [lindex $args end]\" }\n\
proc cb2 args {}\n\
puts \"badopt: [catch {trace zzz} m]:$m\"\n\
puts \"wrongargs: [catch {trace variable x} m]:$m\"\n\
puts \"wrongargs2: [catch {trace vdelete x} m]:$m\"\n\
puts \"wrongargs3: [catch {trace vinfo x y} m]:$m\"\n\
puts \"badops: [catch {trace variable x q cb} m]:$m\"\n\
puts \"listops: [catch {trace vdelete x {read write} cb} m]:$m\"\n\
puts \"dedup: [catch {trace variable x rrw cb} m]:$m\"\n\
puts \"vinfo: [catch {trace vinfo x} m]:$m\"\n\
puts \"info: [catch {trace info variable x} m]:$m\"\n\
puts \"prefix: [catch {trace var x w cb2} m]:$m\"\n\
puts \"vinfo2: [catch {trace vinfo x} m]:$m\"\n\
puts \"crossremove: [catch {trace vdelete x wr cb} m]:$m\"\n\
puts \"vinfo3: [catch {trace vinfo x} m]:$m\"\n\
puts \"modernadd: [catch {trace add variable y {write read} cb} m]:$m\"\n\
puts \"legacyremove: [catch {trace vdelete y rw cb} m]:$m\"\n\
puts \"vinfo4: [catch {trace vinfo y} m]:$m\"\n\
puts \"all: [catch {trace variable z rwua cb} m]:$m\"\n\
puts \"vinfoall: [catch {trace vinfo z} m]:$m\"\n\
puts \"infoall: [catch {trace info variable z} m]:$m\"\n\
puts \"vivify: [info exists x][info exists z]\"\n\
puts \"legacyfire: [catch {set z 1} m]:$m\"\n\
trace add variable m2 write cb\n\
puts \"modernfire: [catch {set m2 1} m]:$m\"\n";

/// Tcl 8.4-8.6: the legacy forms work, and `trace var` abbreviates to
/// `trace variable` (C resolves the option word with `Tcl_GetIndexFromObj`
/// flags `0`).
const EXPECT_8X: &str = "\
badopt: 1:bad option \"zzz\": must be add, info, remove, variable, vdelete, or vinfo\n\
wrongargs: 1:wrong # args: should be \"trace variable name ops command\"\n\
wrongargs2: 1:wrong # args: should be \"trace vdelete name ops command\"\n\
wrongargs3: 1:wrong # args: should be \"trace vinfo name\"\n\
badops: 1:bad operations \"q\": should be one or more of rwua\n\
listops: 1:bad operations \"read write\": should be one or more of rwua\n\
dedup: 0:\n\
vinfo: 0:{rw cb}\n\
info: 0:{{read write} cb}\n\
prefix: 0:\n\
vinfo2: 0:{w cb2} {rw cb}\n\
crossremove: 0:\n\
vinfo3: 0:{w cb2}\n\
modernadd: 0:\n\
legacyremove: 0:\n\
vinfo4: 0:\n\
all: 0:\n\
vinfoall: 0:{rwua cb}\n\
infoall: 0:{{array read write unset} cb}\n\
fired r\n\
vivify: 00\n\
fired w\n\
legacyfire: 0:1\n\
fired write\n\
modernfire: 0:1";

/// Tcl 9.0+: `Tcl_TraceObjCmd`'s option table no longer carries them.
const EXPECT_9X: &str = "\
badopt: 1:bad option \"zzz\": must be add, info, or remove\n\
wrongargs: 1:bad option \"variable\": must be add, info, or remove\n\
wrongargs2: 1:bad option \"vdelete\": must be add, info, or remove\n\
wrongargs3: 1:bad option \"vinfo\": must be add, info, or remove\n\
badops: 1:bad option \"variable\": must be add, info, or remove\n\
listops: 1:bad option \"vdelete\": must be add, info, or remove\n\
dedup: 1:bad option \"variable\": must be add, info, or remove\n\
vinfo: 1:bad option \"vinfo\": must be add, info, or remove\n\
info: 0:\n\
prefix: 1:bad option \"var\": must be add, info, or remove\n\
vinfo2: 1:bad option \"vinfo\": must be add, info, or remove\n\
crossremove: 1:bad option \"vdelete\": must be add, info, or remove\n\
vinfo3: 1:bad option \"vinfo\": must be add, info, or remove\n\
modernadd: 0:\n\
legacyremove: 1:bad option \"vdelete\": must be add, info, or remove\n\
vinfo4: 1:bad option \"vinfo\": must be add, info, or remove\n\
all: 1:bad option \"variable\": must be add, info, or remove\n\
vinfoall: 1:bad option \"vinfo\": must be add, info, or remove\n\
infoall: 0:\n\
vivify: 00\n\
legacyfire: 0:1\n\
fired write\n\
modernfire: 0:1";

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
fn legacy_variable_trace_forms_follow_the_selected_release() {
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
fn vectors_match_all_five_pinned_tclshs() {
    for oracle in required_tclshs(&TclVersion::ALL).expect("all five pinned C trace engines") {
        let vector = VECTORS
            .iter()
            .find(|vector| vector.version == oracle.version)
            .expect("exact release vector");
        assert_eq!(
            common::oracle_output(&oracle.path, SCRIPT),
            vector.expected,
            "{}",
            oracle.patchlevel
        );
    }
}

#[test]
fn jim_variable_trace_surfaces_are_explicitly_unsupported() {
    let oracle = require_jimsh().expect("pinned Jim trace surface");
    assert!(!oracle.supports(JimCapability::VariableTrace));
    let script = "puts [catch {trace variable x w list} m]\nputs $m\nputs [catch {trace add variable x write list} m]\nputs $m\n";
    let expected = "1\ninvalid command name \"trace\"\n1\ninvalid command name \"trace\"";
    assert_eq!(common::oracle_output(&oracle.path, script), expected);
    assert_eq!(common::vm_output(script, "jim"), expected);
}

/// The registry, not the VM, states the 9.0 boundary — so a spec edit moves
/// the runtime with it.
#[test]
fn the_registry_owns_the_release_boundary() {
    let registry = CommandRegistry::build_default();
    let spec = registry.get("trace").expect("trace is registered");
    for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        let mask = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
            .analyser_profile()
            .surface_query();
        for name in ["variable", "vdelete", "vinfo"] {
            assert!(
                spec.resolve_subcommand_for_dialect(name, Some(mask))
                    .is_some(),
                "{version:?} should carry trace {name}"
            );
        }
    }
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let mask = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
            .analyser_profile()
            .surface_query();
        for name in ["variable", "vdelete", "vinfo"] {
            assert!(
                spec.resolve_subcommand_for_dialect(name, Some(mask))
                    .is_none(),
                "{version:?} should not carry trace {name}"
            );
        }
    }
}
