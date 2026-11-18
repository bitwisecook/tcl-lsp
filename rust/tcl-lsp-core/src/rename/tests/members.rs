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

//! Method and property renames: declarations, `my` dispatch, cursor disambiguation and external `$obj method` sites.

use super::*;

#[test]
fn original_primary_member_rename_does_not_enter_string_override_families() {
    // Implementation contract: naming.editor.original-command-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-command-rename-plans.md
    // The family is the resolved relation component, never equal names:
    // `D` inherits `ping` without declaring it and unrelated `E` declares
    // its own. tclsh 8.6/9.0/9.1 print `destroy ping/e` before and
    // `changed destroy/e` after renaming only `C`'s declaration.
    let source = "oo::class create C {method ping {} {return}}\noo::class create D {superclass C}\noo::class create E {method ping {} {return e}}\n";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let row = analysis.original_class_declarations().next().unwrap();
    assert_eq!(
        row.metadata()
            .original_members
            .methods(tcl_compiler::analyser::types::MemberSide::Instance)
            .unwrap()
            .len(),
        1
    );
    let offset = u32::try_from(source.find("ping").unwrap()).unwrap();
    let position = LineIndex::new(source).position_at_utf16(offset, source);
    assert!(prepare_rename(source, position.line, position.character.get(), &analysis).is_some());
    let edits = rename_with_diagnosis(
        source,
        crate::profile_for_dialect("tcl8.6"),
        position.line,
        position.character.get(),
        "changed",
        &analysis,
        None,
    )
    .unwrap();
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(
        (edits[0].range.start_line, edits[0].range.start_character),
        (position.line, position.character.get())
    );
}

// method rename

#[test]
fn rename_method_at_decl_rewrites_decl_and_calls() {
    // Method declared on line 1, dispatched via `my greet` from line 2's
    // body twice — should rewrite the declaration plus both call sites.
    // A bare `greet` call is never valid `TclOO` dispatch (only `my
    // greet` reaches the method), so the fixture must use `my`.
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1 col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "salute",
        &analysis,
        None,
    );
    assert!(!edits.is_empty(), "{edits:?}");
    // All three sites should be present.
    assert!(edits.len() >= 3, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "salute");
    }
}

#[test]
fn rename_method_rewrites_a_my_dispatch_call_inside_a_switch_arm() {
    // The `ticklecharts::chart::Add` dispatcher shape — `switch
    // ... { barSeries { my AddBarSeries {*}$args } ... }`. `rename`
    // reaches `my`-dispatch call sites via `references::
    // method_references_for_class` (`scan_my_method_region`), so this
    // exercises the same resolver as
    // `references_for_method_reach_a_my_dispatch_call_inside_a_switch_arm`
    // end-to-end through rename.
    let src = "oo::class create widget {\n    method bar {} { return \"bar-value\" }\n    method dispatch {args} {\n        switch -exact -- [lindex $args 0] {\n            bar { my bar {*}[lrange $args 1 end] }\n        }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `bar` declaration (line 1, col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "baz",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {edits:?}");
    assert!(
        lines.contains(&4),
        "the my bar call site inside the switch arm must be rewritten too: {edits:?}"
    );
    assert!(edits.iter().all(|e| e.new_text == "baz"));
}

#[test]
fn rename_method_rewrites_my_dispatch_when_class_extended_via_separate_oo_define() {
    // `Gadget` is created via `oo::class create` with no body; every
    // method (including the `my Helper` call site) is added via a
    // *separate*, later `oo::define Gadget { ... }` block — the
    // `ticklecharts::chart` shape. Renaming from the `Helper` declaration
    // must rewrite the `my Helper` call site living in that separate
    // block too, not silently skip it (which would leave the program
    // calling a now-nonexistent method after the rename).
    let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `Helper` declaration (line 4, col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        11,
        "Assist",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&4), "decl missing: {edits:?}");
    assert!(
        lines.contains(&5),
        "the my Helper call site inside the separate oo::define block must be rewritten too: {edits:?}"
    );
    assert!(edits.iter().all(|e| e.new_text == "Assist"));
}

#[test]
fn rename_method_does_not_rewrite_next_dispatch() {
    // `next` is a keyword, not the method name.  Find-references counts the
    // super-dispatch as a reference, but a rename must never rewrite it —
    // the reference paths add `next` sites, the rename path must not.
    let src = "oo::class create Base {\n    method greet {} {}\n}\noo::class create Sub {\n    superclass Base\n    method greet {} { next }\n}\n";
    let analysis = analyse(src);
    // Cursor on `Sub::greet`'s declaration (line 5, col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        11,
        "salute",
        &analysis,
        None,
    );
    assert!(!edits.is_empty(), "{edits:?}");
    // The declaration (line 5, col 11) is rewritten; nothing in the `{ next
    // }` body region (col >= 20 on line 5) is.
    assert!(
        edits
            .iter()
            .all(|e| !(e.range.start_line == 5 && e.range.start_character >= 20)),
        "a rename edit landed on the `next` keyword: {edits:?}",
    );
}

#[test]
fn rename_method_at_call_site_also_works() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the first `greet` call site (line 2 col 25, after `my `).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        25,
        "salute",
        &analysis,
        None,
    );
    assert!(edits.len() >= 3, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "salute");
    }
}

/// A method rename must not run a bare-head scan over every command
/// invocation whose head text matches the renamed method's name —
/// including an unrelated builtin call that merely happens to share it.
/// A `TclOO` method is never a bare-callable command (only `my <method>`
/// dispatches it), so a bare call is always a call to something else.
#[test]
fn rename_method_does_not_rewrite_unrelated_bare_command_with_same_name() {
    let src =
        "oo::class create C {\n    method format {} {}\n    method show {} { format %d 1 }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `format` method's declaration (line 1, col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "render",
        &analysis,
        None,
    );
    // Only the declaration is renamed — the bare `format %d 1` call
    // inside `show` invokes the builtin, not this method.
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].range.start_line, 1);
}

/// Regression: methods, classmethods, and properties are independent
/// tables, so a name shared by more than one (rare, but `TclOO` never
/// merges them) must resolve to whichever declaration the cursor
/// actually sits on — not always the same one by blind priority, which
/// would silently rename the wrong declaration.
#[test]
fn rename_disambiguates_property_and_method_sharing_a_name_by_cursor() {
    // tclsh 9.0.4/9.1.0: property is an oo::configurable definition word.
    let src =
        "oo::configurable create C {\n    property color\n    method color {} { return c }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    // Cursor on the *property* declaration (line 1, col 13).
    let property_edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        1,
        13,
        "shade",
        &analysis,
        None,
    );
    assert_eq!(
        property_edits.len(),
        1,
        "expected only the property decl: {property_edits:?}"
    );
    assert_eq!(property_edits[0].range.start_line, 1, "{property_edits:?}");
    // Cursor on the *method* declaration (line 2) picks the method
    // instead, leaving the same-named property untouched.
    let method_edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        2,
        11,
        "shade",
        &analysis,
        None,
    );
    assert!(
        method_edits.iter().any(|e| e.range.start_line == 2),
        "{method_edits:?}"
    );
    assert!(
        method_edits.iter().all(|e| e.range.start_line != 1),
        "must not touch the same-named property: {method_edits:?}"
    );
}

/// Regression: an instance `method` and a `classmethod` sharing a name
/// (rare, but `TclOO` keeps them in independent tables, so it's legal)
/// must never cross-link during rename — `my <word>` dispatch scope
/// depends on which table the *caller's own body* belongs to (`self` is
/// the class object inside a `classmethod`, the instance everywhere
/// else), so renaming the classmethod must not touch a `my greet` site
/// that actually dispatches to the unrelated instance method — doing so
/// would corrupt otherwise-working code (the call would then reference
/// a name that no longer exists on the instance).
#[test]
fn rename_classmethod_does_not_corrupt_unrelated_instance_method_call_site() {
    let src = "oo::class create C {\n    method greet {} {}\n    classmethod greet {} {}\n    method twice {} { my greet }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    // Cursor on the *classmethod* declaration (line 2, col 16).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        2,
        16,
        "hail",
        &analysis,
        None,
    );
    assert_eq!(
        edits.len(),
        1,
        "only the classmethod's own decl — `twice`'s `my greet` targets \
             the unrelated instance method and must be untouched: {edits:?}"
    );
    assert_eq!(edits[0].range.start_line, 2, "{edits:?}");
}

// property rename

/// Regression: renaming a property must rewrite every `my <property>`
/// call site alongside the declaration.  Properties have no `$obj
/// prop` dispatch and no override family — but they *are* read via
/// `my <property>` inside the class's own methods, and a bare
/// `<property>` occurrence is never valid `TclOO` dispatch (only `my
/// <property>` reads it), so the bare-head scan `rename_method` also
/// runs never matches this shape — without the dedicated `my`-aware
/// scan, only the declaration was renamed, leaving call sites pointing
/// at the old name.
#[test]
fn rename_property_rewrites_decl_and_my_dispatch_call_sites() {
    let src = "oo::class create C {\n    property color\n    method describe {} {\n        return [my color]\n    }\n}\n";
    // `property` is Tcl 9.0+ — the shared `analyse()` helper fixes the
    // dialect at 8.6, so this test analyses at 9.0 directly.
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    // Cursor on the `color` property declaration (line 1, col 13).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        1,
        13,
        "shade",
        &analysis,
        None,
    );
    assert_eq!(edits.len(), 2, "decl + `my color` call site: {edits:?}");
    assert!(edits.iter().all(|e| e.new_text == "shade"));
    assert_eq!(edits[0].range.start_line, 1);
    assert_eq!(edits[1].range.start_line, 3);
}

/// FN→TP: a `my <property>` read nested
/// inside `if` control flow is renamed too — the property rename path
/// shares `scan_my_method_sites`'s control-flow recursion with methods.
#[test]
fn rename_property_rewrites_control_flow_nested_call_site() {
    let src = "oo::class create C {\n    property color\n    method describe {} {\n        if {1} {\n            return [my color]\n        }\n    }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        1,
        13,
        "shade",
        &analysis,
        None,
    );
    assert_eq!(
        edits.len(),
        2,
        "decl + nested `my color` call site: {edits:?}"
    );
    assert!(edits.iter().all(|e| e.new_text == "shade"));
    assert_eq!(edits[1].range.start_line, 4);
}

#[test]
fn rename_method_skipped_outside_class_body() {
    // Cursor on bare `greet` outside the class — no class
    // method named `greet` is visible, so the rename
    // returns empty.
    let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        2,
        "salute",
        &analysis,
        None,
    );
    assert!(edits.is_empty(), "{edits:?}");
}

/// Renaming a method must
/// rewrite a `my method` call site nested inside `if` / `switch`
/// control flow, not just a top-level or `[...]`-nested call — `rename`
/// delegates to the same `method_references_for_class` resolver
/// `references`/the code lens use, so the fix there covers rename too.
#[test]
fn rename_method_rewrites_control_flow_nested_my_dispatch() {
    let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        if {1} {\n            switch -- $k {\n                default {\n                    my getOptions $k\n                }\n            }\n        }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `getOptions` declaration (line 1, col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "fetchOptions",
        &analysis,
        None,
    );
    assert_eq!(edits.len(), 2, "decl + nested `my` site: {edits:?}");
    assert!(edits.iter().all(|e| e.new_text == "fetchOptions"));
    assert_eq!(edits[0].range.start_line, 1);
    // The nested call site, six lines further down.
    assert_eq!(edits[1].range.start_line, 6);
}

#[test]
fn rename_method_rejects_unsafe_new_name() {
    let src = "oo::class create C {\n    method greet {} {}\n}\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "1bad",
        &analysis,
        None,
    );
    assert!(edits.is_empty(), "{edits:?}");
}

#[test]
fn prepare_rename_returns_range_for_method() {
    let src = "oo::class create C {\n    method greet {} {}\n}\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 1, 11, &analysis).expect("expected prepare_rename on method");
    assert_eq!(p.placeholder, "greet");
}

// external $obj method rename

#[test]
fn rename_method_from_decl_rewrites_external_obj_sites() {
    // Renaming `bark` from its declaration also rewrites the
    // external `$d bark` / `[$d bark]` call sites.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "yip",
        &analysis,
        None,
    );
    // Declaration + 2 external sites = 3 edits, all "yip".
    assert!(edits.len() >= 3, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "yip");
    }
    // One edit on line 4 (`$d bark`) and one on line 5
    // (`[$d bark]`).
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&4) && lines.contains(&5), "{edits:?}");
}

#[test]
fn rename_method_rewrites_bare_created_instance_command_site() {
    // `Dog create rex` binds `rex` as an object command, so renaming
    // `bark` must also rewrite the bare `rex bark` dispatch — the same
    // shared resolver drives the peek, the lens, and the rename.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "yip",
        &analysis,
        None,
    );
    for e in &edits {
        assert_eq!(e.new_text, "yip");
    }
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "decl not renamed: {edits:?}");
    assert!(
        lines.contains(&4),
        "bare `rex bark` site not renamed: {edits:?}"
    );
}

#[test]
fn rename_classmethod_from_call_site_rewrites_decl_and_inheriting_subclass_call() {
    // TP — renaming `find` with the cursor on the
    // `ActiveRecord find foo bar` call site must rewrite the
    // declaration, that same call, AND the inherited `Table find`
    // call — three edits, none missed (rename.rs has no separate
    // scan of its own; it delegates fully to
    // `references::method_references_for_class`, so this exercises
    // Part 2 + Part 3 together end-to-end).
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\nActiveRecord find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    // Cursor on `find` in `ActiveRecord find foo bar` (line 7, col 13).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        7,
        13,
        "lookup",
        &analysis,
        None,
    );
    assert_eq!(edits.len(), 3, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "lookup");
    }
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "decl not renamed: {edits:?}");
    assert!(lines.contains(&6), "Table find call not renamed: {edits:?}");
    assert!(
        lines.contains(&7),
        "ActiveRecord find call not renamed: {edits:?}"
    );
}

#[test]
fn rename_method_from_cursor_on_bare_obj_command_call_site() {
    // Symmetry: triggering rename with the cursor ON the
    // `bark` token of a bare `rex bark` dispatch rewrites the declaration
    // and the call site — not only the declaration-triggered rename.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    // Line 4 `rex bark` — cursor on `bark` (col 4).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        4,
        "yip",
        &analysis,
        None,
    );
    assert!(edits.len() >= 2, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "yip");
    }
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "decl not renamed: {edits:?}");
    assert!(lines.contains(&4), "call site not renamed: {edits:?}");
}

#[test]
fn rename_method_from_external_call_site() {
    // Triggering rename from the external `$d bark` site
    // rewrites the declaration + all sites.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    // Cursor on `bark` in `$d bark` (line 4, col 3).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        3,
        "yip",
        &analysis,
        None,
    );
    assert!(edits.len() >= 2, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "yip");
    }
    // Declaration (line 1) is rewritten too.
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "decl not renamed: {edits:?}");
}

#[test]
fn rename_method_external_rewrites_proc_body_sites() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nproc f {} { global d; $d bark }\nf\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "yip",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    // Declaration (1) + proc-body call (4).
    assert!(lines.contains(&1) && lines.contains(&4), "{edits:?}");
}
