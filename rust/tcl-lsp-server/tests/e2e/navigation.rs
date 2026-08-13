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

//! Type-definition and declaration navigation, end-to-end.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};
use serde_json::Value;

// -- TestTypeDefinition --------------------------------------------------

#[test]
fn set_with_class_new() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Point {\n    variable x y\n    constructor {a b} { set x $a; set y $b }\n}\nset p [Point new 1 2]\nputs $p\n";
    lsp.open_ready(&uri, src);
    let locs = locations(&lsp.type_definition(&uri, 5, 6));
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range["start"]["line"].as_i64(), Some(0));
}

#[test]
fn plain_set_returns_empty() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\nputs $x\n");
    assert!(locations(&lsp.type_definition(&uri, 1, 6)).is_empty());
}

#[test]
fn my_call_returns_enclosing_class() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Animal {\n    method speak {} { return \"...\" }\n    method greet {} { my speak }\n}\n";
    lsp.open_ready(&uri, src);
    let locs = locations(&lsp.type_definition(&uri, 2, 26));
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range["start"]["line"].as_i64(), Some(0));
}

// -- TestDeclaration -----------------------------------------------------

#[test]
fn global_var_in_proc_returns_declaration_not_set() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    global foo\n    set foo 42\n    return $foo\n}\n";
    lsp.open_ready(&uri, src);
    let locs = locations(&lsp.declaration(&uri, 3, 12));
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range["start"]["line"].as_i64(), Some(1));
}

#[test]
fn falls_back_to_definition_for_proc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {name} { puts \"Hello $name\" }\ngreet World\n",
    );
    let locs = locations(&lsp.declaration(&uri, 1, 1));
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range["start"]["line"].as_i64(), Some(0));
}

#[test]
fn scope_isolation_between_procs() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc alpha {} {\n    global shared\n    set shared 1\n}\nproc beta {} {\n    global shared\n    return $shared\n}\n";
    lsp.open_ready(&uri, src);
    let lines = start_lines(&lsp.declaration(&uri, 6, 12));
    assert!(lines.contains(&5));
    assert!(!lines.contains(&1));
}

// M7: a proc-name literal held as a dispatch-table value (consumed by a
// `$table(...)` dispatch) is a real reference — go-to-definition from the
// literal jumps to the proc, and the `$cmd` const-dispatch head resolves too.
#[test]
fn dispatch_table_literal_resolves_to_the_proc_m7() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc do_add {a b} { return [expr {$a + $b}] }\narray set ops {add do_add}\nset k add\n$ops($k) 1 2\n";
    lsp.open_ready(&uri, src);
    // Cursor inside the `do_add` table value literal (line 1, col 20).
    let lines = start_lines(&lsp.definition(&uri, 1, 20));
    assert!(
        lines.contains(&0) && lines.len() == 1,
        "table literal must resolve to the proc: {lines:?}"
    );
    // References from the declaration include the table literal's line.
    let refs = start_lines(&lsp.references(&uri, 0, 6, true));
    assert!(
        refs.contains(&1),
        "table literal is a reference site: {refs:?}"
    );
}

// M9: `source` evaluates the file in the caller's namespace — a bare
// `proc helper` in a file sourced inside `namespace eval ::x` is really
// `::x::helper`, so a correctly-qualified cross-file call resolves to it and
// the sourced declaration's references reach the qualified callers.
#[test]
fn sourced_file_resolves_under_the_source_site_namespace_m9() {
    let mut lsp = Lsp::tcl();
    let b = unique_uri("tcl");
    let b_name = b.rsplit('/').next().unwrap().to_owned();
    lsp.open_ready(&b, "proc helper {} {}\n");
    let a = unique_uri("tcl");
    let a_src = format!("namespace eval ::x {{ source {b_name} }}\n::x::helper\n");
    lsp.open_ready(&a, &a_src);

    // Go-to-definition on the `::x::helper` call jumps into the sourced file.
    let defs = crate::common::helpers::locations(&lsp.definition(&a, 1, 4));
    assert!(
        defs.iter().any(|l| l.uri == b
            && l.range
                .get("start")
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
                == Some(0)),
        "::x::helper must resolve into the sourced file: {defs:?}"
    );

    // References from the sourced declaration reach the qualified caller.
    let refs = crate::common::helpers::locations(&lsp.references(&b, 0, 6, false));
    assert!(
        refs.iter().any(|l| l.uri == a
            && l.range
                .get("start")
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
                == Some(1)),
        "the qualified call is a reference of the sourced declaration: {refs:?}"
    );
}

// -- Caller-frame variables (issue #923 audit idx 58) ---------------------

/// The ticklecharts shape, minimised: `gridlayoutHasDataSetObj dataset` names
/// a variable in the CALLER's frame, the callee's `upvar 1 $dts dataset`
/// creates it there, and a sibling accessor **method** happens to share the
/// name.  tclsh 9.0.4 / 8.6.14 both run the real thing to completion.
const CALLER_FRAME_SRC: &str = "\
proc gridlayoutHasDataSetObj {dts} {
    upvar 1 $dts dataset
    set dataset MY-SHARED-DATASET
}
oo::class create chart {
    constructor {} {
        gridlayoutHasDataSetObj dataset
        set _dataset $dataset
    }
    method dataset {} { return 1 }
}
";

#[test]
fn hover_on_a_caller_frame_read_names_the_creating_call_not_a_method() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, CALLER_FRAME_SRC);
    // Line 7 col 23 — inside the `dataset` of `set _dataset $dataset`.
    let text = hover_text(&lsp.hover(&uri, 7, 23));
    assert!(
        text.contains("Caller-frame variable") && text.contains("gridlayoutHasDataSetObj"),
        "expected the caller-frame card: {text:?}"
    );
    assert!(
        !text.contains("method"),
        "a `$`-led read must never render the same-named method's card: {text:?}"
    );
}

#[test]
fn definition_on_a_caller_frame_read_reaches_the_call_site_word() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, CALLER_FRAME_SRC);
    let locs = locations(&lsp.definition(&uri, 7, 23));
    assert_eq!(locs.len(), 1, "one definition: {locs:?}");
    assert_eq!(
        locs[0].range["start"]["line"].as_i64(),
        Some(6),
        "the creating write is the call-site word: {locs:?}"
    );
}

#[test]
fn references_link_the_call_site_word_and_the_caller_frame_read() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, CALLER_FRAME_SRC);
    let from_read = start_lines(&lsp.references(&uri, 7, 23, true));
    assert_eq!(
        from_read.iter().copied().collect::<Vec<i64>>(),
        vec![6, 7],
        "both halves of the idiom are one variable: {from_read:?}"
    );
    // The bare call-site word is the same anchor.
    let from_call = start_lines(&lsp.references(&uri, 6, 33, true));
    assert_eq!(from_call, from_read, "{from_call:?}");
    // …and never the unrelated same-named method's declaration (line 9).
    assert!(!from_read.contains(&9), "{from_read:?}");
}

// -- Literal caller-frame targets (issue #923 audit idx 22 / issue #1139) --

/// The `SpiceGenTcl` shape, minimised to its proc half: the callee spells the
/// caller-frame name **in its own body** (`upvar name name`), so no
/// call-site word carries it.  tclsh 9.0.4: `build` returns `W1`.
const LITERAL_TARGET_SRC: &str = "\
proc NameProcess {arguments object} {
    upvar name name
    set name W1
}
proc build {} {
    NameProcess x obj
    set out $name
}
";

#[test]
fn hover_on_a_literal_upvar_target_read_names_the_creating_call() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, LITERAL_TARGET_SRC);
    // Line 6 col 15 — inside the `name` of `set out $name`.
    let text = hover_text(&lsp.hover(&uri, 6, 15));
    assert!(
        text.contains("Caller-frame variable") && text.contains("NameProcess"),
        "expected the caller-frame card naming the callee: {text:?}"
    );
}

#[test]
fn definition_on_a_literal_upvar_target_read_reaches_the_creating_call() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, LITERAL_TARGET_SRC);
    let locs = locations(&lsp.definition(&uri, 6, 15));
    assert_eq!(locs.len(), 1, "one definition: {locs:?}");
    assert_eq!(
        locs[0].range["start"]["line"].as_i64(),
        Some(5),
        "the creating call is the nearest thing to a declaration: {locs:?}"
    );
}

#[test]
fn references_on_a_literal_upvar_target_link_the_call_and_the_read() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, LITERAL_TARGET_SRC);
    let from_read = start_lines(&lsp.references(&uri, 6, 15, true));
    assert_eq!(
        from_read.iter().copied().collect::<Vec<i64>>(),
        vec![5, 6],
        "the creating call and the read are one variable: {from_read:?}"
    );
}

// -- `my <method>` dispatch through a mixin (issue #923 audit idx 22) --

/// The `SpiceGenTcl` shape the audit reported: the callee is a **mixin**'s
/// method reached by `my NameProcess …`, and the constructor that reads
/// `$name` never assigns it.  tclsh 9.0.4 / 8.6.16, identical: `Widget new
/// {-base 1}` prints `name=::oo::Obj24 params=-base 1`.
const MIXIN_DISPATCH_SRC: &str = "\
oo::class create Utility {
    method NameProcess {arguments object} {
        upvar name name
        set name $object
    }
}
oo::class create Widget {
    mixin Utility
    constructor {arguments} {
        my NameProcess $arguments [self object]
        set params $arguments
        puts \"name=$name params=$params\"
    }
}
";

/// Character of the `name` inside `puts "name=$name params=$params"` on
/// line 11 of [`MIXIN_DISPATCH_SRC`].
fn mixin_read_column() -> u32 {
    let at = MIXIN_DISPATCH_SRC
        .lines()
        .nth(11)
        .and_then(|l| l.find("$name"))
        .expect("the read");
    u32::try_from(at).expect("column fits u32") + 2
}

#[test]
fn hover_on_a_mixin_dispatch_caller_frame_read_names_the_creating_call() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, MIXIN_DISPATCH_SRC);
    // Line 11 — inside the `name` of `puts "name=$name params=$params"`.
    let col = mixin_read_column();
    let text = hover_text(&lsp.hover(&uri, 11, col));
    assert!(
        text.contains("Caller-frame variable") && text.contains("NameProcess"),
        "the card must name the method the MRO resolves: {text:?}"
    );
}

#[test]
fn definition_on_a_mixin_dispatch_caller_frame_read_reaches_the_dispatch() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, MIXIN_DISPATCH_SRC);
    let col = mixin_read_column();
    let locs = locations(&lsp.definition(&uri, 11, col));
    assert_eq!(locs.len(), 1, "one definition: {locs:?}");
    assert_eq!(
        locs[0].range["start"]["line"].as_i64(),
        Some(9),
        "the `my NameProcess …` dispatch is where the variable comes to \
         exist in this frame: {locs:?}"
    );
}

#[test]
fn references_on_a_mixin_dispatch_link_the_dispatch_and_the_read() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, MIXIN_DISPATCH_SRC);
    let col = mixin_read_column();
    let refs = start_lines(&lsp.references(&uri, 11, col, true));
    assert_eq!(
        refs.iter().copied().collect::<Vec<i64>>(),
        vec![9, 11],
        "the dispatch and the read are one variable: {refs:?}"
    );
}

/// TN — a mixin method with no caller-frame `upvar` creates nothing, so the
/// read keeps abstaining rather than gaining a wrong answer.  tclsh 9.0.4:
/// the constructor's `$name` raises `can't read "name"`.
#[test]
fn a_mixin_dispatch_without_an_upvar_still_abstains() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Utility {\n    method NameProcess {arguments object} { set name $object }\n}\noo::class create Widget {\n    mixin Utility\n    constructor {arguments} {\n        my NameProcess $arguments [self object]\n        puts \"name=$name\"\n    }\n}\n",
    );
    // Line 7 col 21 — inside the `name` of `puts "name=$name"`.
    assert!(
        hover_text(&lsp.hover(&uri, 7, 21)).is_empty(),
        "an unbound `$`-led read must draw no hover"
    );
    assert!(
        locations(&lsp.definition(&uri, 7, 21)).is_empty(),
        "…and no definition"
    );
}

/// Issue #923 audit idx 98: `upvar ::tk::FocusGrab($index) data` names one
/// fixed global cell (level-independent), so every occurrence of the cell —
/// the `upvar` `otherVar` word, the sibling proc's `info exists` argument,
/// its `$::tk::FocusGrab($index)` read, and its `unset` argument — is one
/// variable.
///
/// Pinned through the **live server**, i.e. the incremental per-item
/// analysis a real editor drives, and on all three navigation verbs.  The
/// compiler library answered all four anchors from the whole-file walk
/// while the server answered nothing for hover and go-to-definition,
/// because the cell is defined *inside a deferred body* and the per-item
/// graft merged a fragment's proc scope only — so the cell never reached
/// the scope tree the providers read.  A `references`-only assertion could
/// not see that, which is exactly why this asserts all three.
///
/// Known residual, deliberately left visible rather than asserted away:
/// the *bareword* `info exists` / `unset` arguments (lines 7 and 9) are
/// not in the cell's reference set.  A bareword in a variable-role
/// argument slot is a separate anchor kind on both sides — the cursor
/// does not resolve there either — and the compiler-library twin
/// (`tcl-lsp-core`'s `a_fully_qualified_upvar_target_navigates_from_word_and_read`)
/// now asserts the same set by position, so the two tiers agree about
/// exactly what is and is not covered.
const FOCUS_GRAB_SRC: &str = "proc SetFocusGrab {grab focus} {\n    set index \"$grab,$focus\"\n    upvar ::tk::FocusGrab($index) data\n    lappend data one\n}\nproc RestoreFocusGrab {grab focus} {\n    set index \"$grab,$focus\"\n    if {[info exists ::tk::FocusGrab($index)]} {\n        set data2 $::tk::FocusGrab($index)\n        unset ::tk::FocusGrab($index)\n    }\n}\n";

/// Line/character of each anchor: the `upvar` `otherVar` word (line 2) and
/// the `$::tk::FocusGrab($index)` read (line 8), both inside `FocusGrab`.
const FOCUS_GRAB_ANCHORS: [(u32, u32); 2] = [(2, 20), (8, 26)];

#[test]
fn a_fully_qualified_upvar_target_hovers_from_word_and_read() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, FOCUS_GRAB_SRC);
    for (line, character) in FOCUS_GRAB_ANCHORS {
        let text = hover_text(&lsp.hover(&uri, line, character));
        assert!(
            text.contains("::tk::FocusGrab"),
            "hover at {line}:{character} must name the cell: {text:?}"
        );
    }
}

#[test]
fn a_fully_qualified_upvar_target_defines_from_word_and_read() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, FOCUS_GRAB_SRC);
    for (line, character) in FOCUS_GRAB_ANCHORS {
        let locs = locations(&lsp.definition(&uri, line, character));
        assert_eq!(
            locs.len(),
            1,
            "one definition at {line}:{character}: {locs:?}"
        );
        assert_eq!(
            locs[0].range["start"]["line"].as_i64(),
            Some(2),
            "the `upvar` otherVar word is where the cell comes to exist: {locs:?}"
        );
    }
}

#[test]
fn a_fully_qualified_upvar_target_cross_references_between_procs() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, FOCUS_GRAB_SRC);
    for (line, character) in FOCUS_GRAB_ANCHORS {
        let refs = start_lines(&lsp.references(&uri, line, character, true));
        assert_eq!(
            refs.iter().copied().collect::<Vec<i64>>(),
            vec![2, 3, 8],
            "the `upvar` otherVar word and its `data` alias (line 2), the alias \
             use (line 3), and the sibling proc's read (line 8), from \
             {line}:{character}: {refs:?}"
        );
        // The cross-proc link is the point of the finding, so it is called
        // out separately from the exact set above.
        assert!(
            refs.contains(&2) && refs.contains(&8),
            "the read must link the upvar otherVar word across procs: {refs:?}"
        );
    }
}

/// TN — an unbound `$`-led read abstains rather than resolving to a
/// coincidentally same-named method.  This is the wrong-kind conflation the
/// audit confirmed: Tcl's variable and command namespaces are disjoint.
#[test]
fn an_unbound_dollar_read_never_resolves_to_a_same_named_method() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create widget {\n    constructor {} { puts $thing }\n    method thing {} { return 1 }\n}\n",
    );
    assert!(
        hover_text(&lsp.hover(&uri, 1, 28)).is_empty(),
        "an unbound `$`-led read must draw no hover"
    );
    assert!(
        locations(&lsp.definition(&uri, 1, 28)).is_empty(),
        "…and no definition"
    );
    assert!(
        start_lines(&lsp.references(&uri, 1, 28, true)).is_empty(),
        "…and no references"
    );
}
