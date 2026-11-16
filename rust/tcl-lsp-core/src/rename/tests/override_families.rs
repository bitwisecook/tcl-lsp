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

//! Method renames across overrides, inheritors and export lists.

use super::*;

// method rename across overrides

#[test]
fn rename_method_renames_subclass_override() {
    // `Animal::speak` overridden by `Dog::speak`.  Renaming from the
    // base declaration must rewrite *both* declarations — leaving the
    // override behind would silently break polymorphic dispatch.
    let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                       method speak {} { return woof }\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `speak` in Animal's declaration (line 1 col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "vocalise",
        &analysis,
        None,
    );
    assert!(edits.iter().all(|e| e.new_text == "vocalise"));
    // Both declarations rewritten: line 1 (Animal) and line 5 (Dog).
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1) && lines.contains(&5),
        "expected both Animal (l1) and Dog override (l5) renamed; got {edits:?}",
    );
}

#[test]
fn rename_method_renames_from_subclass_up_to_base() {
    // Symmetric to the above: renaming from the *subclass* override
    // must also rewrite the base declaration.
    let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                       method speak {} { return woof }\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `speak` in Dog's declaration (line 5 col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        7,
        "vocalise",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1) && lines.contains(&5),
        "expected base (l1) renamed from subclass override; got {edits:?}",
    );
}

#[test]
fn rename_method_renames_sibling_overrides_of_common_base() {
    // Two siblings override a common base method.  Renaming any one
    // must rewrite the base and *both* siblings — they are one
    // polymorphic name reachable via the base's static type.
    let src = "oo::class create Shape {\n\
                       method area {} { return 0 }\n\
                   }\n\
                   oo::class create Circle {\n\
                       superclass Shape\n\
                       method area {} { return 3 }\n\
                   }\n\
                   oo::class create Square {\n\
                       superclass Shape\n\
                       method area {} { return 4 }\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `area` in Circle (line 5 col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        7,
        "measure",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    // Shape (l1), Circle (l5), Square (l9) all rewritten.
    assert!(
        lines.contains(&1) && lines.contains(&5) && lines.contains(&9),
        "expected Shape + both siblings renamed; got {edits:?}",
    );
}

#[test]
fn rename_method_leaves_unrelated_same_name_method_untouched() {
    // Two classes in *disjoint* hierarchies both define `run`.
    // Renaming one must not touch the other.
    let src = "oo::class create Engine {\n\
                       method run {} { return e }\n\
                   }\n\
                   oo::class create Task {\n\
                       method run {} { return t }\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `run` in Engine (line 1 col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "start",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1),
        "Engine::run should be renamed: {edits:?}"
    );
    assert!(
        !lines.contains(&4),
        "unrelated Task::run must NOT be renamed; got {edits:?}",
    );
}

#[test]
fn rename_inherited_method_from_external_obj_site() {
    // `$d speak` where `d` is a `Dog` that *inherits* `speak` from
    // `Animal` (no override).  Renaming from the external call site
    // must rewrite the base declaration.
    let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                   }\n\
                   set d [Dog new]\n\
                   $d speak\n";
    let analysis = analyse(src);
    // Cursor on `speak` in `$d speak` (line 7 col 3).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        7,
        3,
        "vocalise",
        &analysis,
        None,
    );
    assert!(
        !edits.is_empty(),
        "inherited-method rename produced nothing"
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1),
        "expected base Animal::speak decl (l1) renamed; got {edits:?}",
    );
}

#[test]
fn rename_inherited_method_rewrites_subclass_instance_obj_sites() {
    // Renaming an inherited method from the *base declaration* must also
    // rewrite `$d method` sites where `$d` is an instance of a subclass
    // that inherits (does not override) it — an exact-class-equality
    // filter would drop these, leaving them pointing at the old name.
    let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                   }\n\
                   set d [Dog new]\n\
                   $d speak\n";
    let analysis = analyse(src);
    // Cursor on `speak` in Animal's declaration (line 1 col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "vocalise",
        &analysis,
        None,
    );
    assert!(edits.iter().all(|e| e.new_text == "vocalise"));
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1) && lines.contains(&7),
        "expected decl (l1) + inheriting-subclass instance site (l7); got {edits:?}",
    );
}

#[test]
fn rename_inherited_method_rewrites_pure_inheritor_my_sites() {
    // A subclass that inherits `speak` (no override) calls `my speak`
    // from one of its own method bodies.  That call dispatches to the
    // base definition, so renaming the base must rewrite it — even
    // though the subclass is not itself a rename family member.
    let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                       method describe {} { my speak }\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `speak` in Animal's declaration (line 1 col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "vocalise",
        &analysis,
        None,
    );
    assert!(edits.iter().all(|e| e.new_text == "vocalise"));
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1) && lines.contains(&5),
        "expected decl (l1) + `my speak` in inheriting subclass (l5); got {edits:?}",
    );
}

/// TP — the `export` list is part of the rename.
///
/// Oracle (tclsh 9.0.4 and 8.6.16, identical): with `method Foo` +
/// `export Foo`, `[$a Foo]` prints `foo`.  Rename `Foo` → `Bar` touching
/// only the declaration and the call site, leaving `export Foo` behind,
/// and both interpreters answer
/// `unknown method "Bar": must be destroy` — the renamed method is no
/// longer exported.  A `method` whose name begins with an upper-case
/// letter is unexported by default (probed on 9.0.4: `$a Foo` errors,
/// `$a bar` works), which is exactly why the corpus writes the list.
#[test]
fn tp_rename_method_rewrites_its_export_list() {
    let src = "oo::class create A {\n    method Foo {} { return 1 }\n    export Foo\n}\nset a [A new]\nputs [$a Foo]\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        12,
        "Bar",
        &analysis,
        None,
    );
    let applied = apply_edits(src, &edits);
    assert!(
        applied.contains("export Bar"),
        "the export list must be renamed with the method: {applied}"
    );
    assert!(
        !applied.contains("Foo"),
        "no stale name may survive: {applied}"
    );
}

/// TP — an `export` written in a separate `oo::define` block is found
/// through the definer grammar, not the class's own body span.
#[test]
fn tp_rename_method_rewrites_an_export_in_a_separate_oo_define_block() {
    let src = "oo::class create A {\n    method Foo {} { return 1 }\n}\noo::define A {\n    export Foo\n}\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        12,
        "Bar",
        &analysis,
        None,
    );
    let applied = apply_edits(src, &edits);
    assert!(
        applied.contains("export Bar"),
        "an `oo::define` export list must rename too: {applied}"
    );
}
