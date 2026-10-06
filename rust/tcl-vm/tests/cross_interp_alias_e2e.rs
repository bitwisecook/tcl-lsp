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

//! `interp alias` loop prevention (C `TclPreventAliasLoop`, 8.6
//! `tclInterp.c`) and cross-interp aliases between a parent and its direct
//! children, in both directions.
//!
//! Every vector is a complete script whose stdout is compared against the
//! matching physical-core VM and every pinned C8.4–C9.1 interpreter through
//! the shared strict oracle runner. Jim's absent C-style surfaces have separate
//! native controls.  Key pinned facts:
//!
//! * a refused self-alias **destroys** the proc it clobbered (C creates the
//!   command first and does not restore it on rollback);
//! * `rename` of an alias onto a loop-forming name is refused with the
//!   command **restored** under its old name;
//! * a child→parent alias target runs at the parent's *current* frame
//!   (`info level` stacks on the pending frame, `uplevel 1` sees its
//!   locals), and its errors propagate catchably into the child;
//! * loops are detected across the interp boundary in both directions,
//!   with the error naming the *defining* command.

mod common;

use tcl_dialect::TclVersion;
use tcl_test_support::{JimCapability, require_jimsh, required_tclshs};

fn vm_output(source: &str) -> String {
    common::vm_output(source, "tcl9.0")
}

/// One behaviour vector: the script prints its observations; `want` is the
/// full expected stdout under C8.5–C9.1; the C8.4 rename diagnostic is separate.
struct Vector {
    name: &'static str,
    script: &'static str,
    want: &'static str,
}

const VECTORS: &[Vector] = &[
    // Loop prevention (`TclPreventAliasLoop`).
    Vector {
        name: "a self-alias is refused AND destroys the proc it clobbered",
        script: "proc x {} {return REAL}\n\
                 puts [catch {interp alias {} x {} x} m]:$m\n\
                 puts [catch {x} m2]:$m2\n",
        want: "1:cannot define or rename alias \"x\": would create a loop\n\
               1:invalid command name \"x\"",
    },
    Vector {
        name: "a mutual pair is refused at the closing alias, which is removed",
        script: "interp alias {} a {} b\n\
                 puts [catch {interp alias {} b {} a} m]:$m\n\
                 puts [info commands b]\n",
        want: "1:cannot define or rename alias \"b\": would create a loop",
    },
    Vector {
        name: "rename onto a loop-forming name is refused and rolled back",
        script: "interp alias {} a {} b\n\
                 puts [catch {rename a b} m]:$m\n\
                 puts a=[info commands a],b=[info commands b]\n",
        want: "1:cannot define or rename alias \"b\": would create a loop\n\
               a=a,b=",
    },
    Vector {
        name: "a multi-hop chain with baked args still closes into a loop",
        script: "interp alias {} c {} d\n\
                 interp alias {} d {} e\n\
                 puts [catch {interp alias {} e {} c extra} m]:$m\n",
        want: "1:cannot define or rename alias \"e\": would create a loop",
    },
    Vector {
        name: "a qualified self-spelling is caught by resolution, not string compare",
        script: "puts [catch {interp alias {} q {} ::q} m]:$m\n",
        want: "1:cannot define or rename alias \"q\": would create a loop",
    },
    Vector {
        name: "an alias to an undefined target is legal (aliases late-bind)",
        script: "interp alias {} lb {} no_such_command_yet\n\
                 proc no_such_command_yet {} {return LATE}\n\
                 puts [lb]\n",
        want: "LATE",
    },
    // Child-to-parent aliases.
    Vector {
        name: "a child→parent target stacks on the parent's pending frame",
        script: "proc probe {} { return [info level] }\n\
                 proc probe2 {} { uplevel 1 {set marker} }\n\
                 proc runner {} {\n\
                     set marker M\n\
                     interp create c\n\
                     interp alias c pcall {} probe\n\
                     interp alias c pcall2 {} probe2\n\
                     set r1 [interp eval c { pcall }]\n\
                     set r2 [interp eval c { pcall2 }]\n\
                     interp delete c\n\
                     return \"$r1 $r2\"\n\
                 }\n\
                 puts [runner]\n",
        want: "2 M",
    },
    Vector {
        name: "a parent-side error propagates catchably into the child",
        script: "interp create c2\n\
                 interp alias c2 boom {} error KABOOM\n\
                 puts [catch {interp eval c2 { boom }} m]:$m\n\
                 puts [interp eval c2 { catch { boom } im; set im }]\n\
                 interp delete c2\n",
        want: "1:KABOOM\nKABOOM",
    },
    Vector {
        name: "baked args and call args concatenate across the boundary",
        script: "interp create c3\n\
                 interp alias c3 lister {} list A B\n\
                 puts [interp eval c3 { lister C D }]\n\
                 interp delete c3\n",
        want: "A B C D",
    },
    // Parent-to-child aliases.
    Vector {
        name: "a parent-side alias dispatches into the child",
        script: "interp create c4\n\
                 interp eval c4 { proc inchild {} { return CHILD } }\n\
                 interp alias {} fwd c4 inchild\n\
                 puts [fwd]\n\
                 interp delete c4\n",
        want: "CHILD",
    },
    // Loops across the interp boundary.
    Vector {
        name: "a child→parent / parent→child pair is a loop (closed parent-side)",
        script: "interp create c5\n\
                 puts [interp alias c5 pcall {} fwd5]\n\
                 puts [catch {interp alias {} fwd5 c5 pcall} m]:$m\n\
                 puts [info commands fwd5]\n\
                 interp delete c5\n",
        want: "pcall\n\
               1:cannot define or rename alias \"fwd5\": would create a loop",
    },
    Vector {
        name: "the same loop closed child-side is caught too",
        script: "interp create c6\n\
                 interp alias {} fw2 c6 ptgt\n\
                 puts [catch {interp alias c6 ptgt {} fw2} m]:$m\n\
                 interp delete c6\n",
        want: "1:cannot define or rename alias \"ptgt\": would create a loop",
    },
];

fn expected(vector: &Vector, version: TclVersion) -> String {
    if version == TclVersion::V8_4
        && vector.name == "rename onto a loop-forming name is refused and rolled back"
    {
        // Actual C8.4 reports the source alias; later C reports the destination.
        return vector.want.replacen("alias \"b\"", "alias \"a\"", 1);
    }
    vector.want.to_owned()
}

#[test]
fn vm_matches_the_pinned_cross_interp_alias_vectors() {
    for version in TclVersion::ALL {
        for vector in VECTORS {
            assert_eq!(
                common::vm_output(vector.script, version.dialect_name()),
                expected(vector, version),
                "{version:?}: {}",
                vector.name
            );
        }
    }
}

#[test]
fn deleting_an_alias_target_retires_a_retained_source_token() {
    // The active ::N frame retains its alias after namespace deletion. Target
    // interpreter teardown must still find and delete that exact source token.
    // Exact Tcl 9.0.4 oracle.
    assert_eq!(
        vm_output(
            "interp create i
             namespace eval N {
                 interp alias {} ::N::a i set x
                 proc p {} {
                     namespace delete ::N
                     interp delete i
                     puts [list [info commands a] [catch {a} m] $m]
                 }
             }
             ::N::p",
        ),
        r#"{} 1 {invalid command name "a"}"#
    );
}

#[test]
fn child_interpreter_deletion_uses_command_token_lifecycle_once() {
    // The child naming command's delete trace runs while the interpreter still
    // exists. A later child at the same spelling is a new generation and does
    // not inherit the old trace. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        vm_output(
            "set log {}
             proc cb {old new op} {
                 lappend ::log [list $old $new $op [interp exists i]]
             }
             interp create i
             trace add command i delete cb
             interp delete i
             set first $log
             interp create i
             rename i {}
             puts [list $first $log]",
        ),
        "{{::i {} delete 1}} {{::i {} delete 1}}"
    );
}

/// All five validated C engines retain the exact native behavior vectors.
#[test]
fn vectors_match_real_tclsh() {
    for oracle in required_tclshs(&TclVersion::ALL).expect("all five pinned C alias engines") {
        for vector in VECTORS {
            assert_eq!(
                common::oracle_output(&oracle.path, vector.script),
                expected(vector, oracle.version),
                "{}: {}",
                oracle.patchlevel,
                vector.name
            );
        }
    }
}

#[test]
fn jim_c_style_interpreter_alias_surfaces_are_explicitly_unsupported() {
    let oracle = require_jimsh().expect("pinned Jim interpreter surface");
    assert!(!oracle.supports(JimCapability::InterpCreate));
    assert!(!oracle.supports(JimCapability::InterpAlias));
    let script = "puts [catch {interp create c} m]\nputs $m\nputs [catch {interp alias {} a {} list} m]\nputs $m\n";
    let expected = "1\nwrong # args: should be \"interp\"\n1\nwrong # args: should be \"interp\"";
    assert_eq!(common::oracle_output(&oracle.path, script), expected);
    assert_eq!(common::vm_output(script, "jim"), expected);
}

#[test]
fn jim_child_handle_aliases_keep_original_parent_lookup_and_retirement() {
    let oracle = require_jimsh().expect("pinned Jim child aliases");
    assert!(oracle.supports(JimCapability::InterpHandleCreate));
    assert!(oracle.supports(JimCapability::InterpHandleAlias));
    for (script, expected) in [
        (
            "set child [interp]; $child alias collect list PREFIX; puts [$child eval {collect ARG}]; $child delete; puts [llength [info commands $child]]\n",
            "PREFIX ARG\n0",
        ),
        (
            "set child [interp]; $child alias collect list PREFIX; rename list originalList; proc list args {originalList OVERRIDE {*}$args}; puts [$child eval {collect ARG}]; $child delete\n",
            "OVERRIDE PREFIX ARG",
        ),
    ] {
        assert_eq!(common::oracle_output(&oracle.path, script), expected);
        assert_eq!(common::vm_output(script, "jim"), expected);
    }
}
