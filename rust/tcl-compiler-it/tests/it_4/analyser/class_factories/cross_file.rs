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

//! Metaclasses in other files: the workspace factory index, shadowing, unreadable prologues and chained metaclass files.

use super::*;

/// A user metaclass whose own definition is in **another file** and whose
/// declaration the host never indexed cannot be recognised: the analyser
/// is per-file, and nothing in `X create Name Supers Body` distinguishes
/// a class factory from `interp create`, `image create`, or any ordinary
/// proc taking a script argument.  Guessing would invent classes out of
/// unrelated commands, so the LSP abstains — records no class, and emits
/// no diagnostic about the members it therefore cannot see.
///
/// This is the **floor** the workspace factory index raises, not
/// replaces: with no index (every single-file analysis, and every host
/// that has not enumerated the project) the behaviour is unchanged.  The
/// resolving half is `a_workspace_indexed_metaclass_creates_real_classes`
/// below; Tk's own `library/iconlist.tcl` is that shape.
#[test]
fn a_metaclass_defined_in_another_file_abstains_rather_than_guessing() {
    crate::install_surfaces();
    let src = concat!(
        "::Megawidget create IconList FocusableWidget {\n",
        "    method GetSpecs {} { return \"iconlist+[next]\" }\n",
        "}\n",
    );
    let r = analysis(src, "tcl9.0");
    assert!(
        !r.all_classes.contains_key("::IconList"),
        "no class may be invented from an unknown command: {:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
    assert!(
        r.diagnostics
            .iter()
            .all(|d| d.code.to_string() != "W308" && d.code.to_string() != "W001"),
        "abstention must be silent, not a wrong answer: {:?}",
        r.diagnostics
    );
}

// Cross-file class factories.
//
// Ground truth, byte-identical on tclsh 8.6.14 and 9.0.4, for the
// three-file shape below (`megawidget.tcl` / `base.tcl` /
// `iconlist.tcl`, each `source`d in turn):
//
//   info object isa class ::IconList          -> 1
//   info class superclasses ::IconList        -> ::tk::MegawidgetClass ::FocusableWidget
//   info class methods ::IconList -private    -> CreateHull GetSpecs
//   info class superclasses ::FocusableWidget -> ::tk::MegawidgetClass
//   [::IconList new] CreateHull               -> iconlist-hull/hull-from-FocusableWidget
//   [::IconList new] TraceOption a b          -> traced-from-MegawidgetClass
//
// i.e. the class, its members, its spliced superclass, and the `next`
// chain through the injected base are all real — the file that writes
// `::tk::Megawidget create IconList …` just cannot see any of it on its
// own.

/// The metaclass file of the three-file shape.
const META_FILE: &str = concat!(
    "oo::class create ::tk::MegawidgetClass {\n",
    "    method TraceOption {a b} { return traced-from-MegawidgetClass }\n",
    "}\n",
    "oo::class create ::tk::Megawidget {\n",
    "    superclass oo::class\n",
    "    self method create {name superclasses body} {\n",
    "        next $name [list superclass ::tk::MegawidgetClass {*}$superclasses]\\;$body\n",
    "    }\n",
    "}\n",
);

/// The consumer file — Tk's `library/iconlist.tcl` shape, reduced.
const CONSUMER_FILE: &str = concat!(
    "::tk::Megawidget create IconList FocusableWidget {\n",
    "    method GetSpecs {} { return iconlist }\n",
    "    method CreateHull {} { return hull }\n",
    "}\n",
);

/// The workspace factory index `src` publishes — what a host merges and
/// hands back to a consumer document's analysis.
fn factories_of(src: &str) -> std::sync::Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    std::sync::Arc::new(analysis(src, "tcl9.0").class_factories())
}

/// Analyse `src` with `index` supplied as the workspace factory oracle.
fn analysis_with_factories(
    src: &str,
    index: &std::sync::Arc<tcl_compiler::analyser::ClassFactoryIndex>,
) -> AnalysisResult {
    Analyser::new()
        .with_workspace_class_factories(Some(std::sync::Arc::clone(index)))
        .analyse(src, "tcl9.0")
}

#[test]
fn a_metaclass_file_publishes_exactly_one_factory() {
    crate::install_surfaces();
    // Non-vacuity floor for every test below: the index really carries
    // `::tk::Megawidget` (and only it — `::tk::MegawidgetClass` is an
    // ordinary class, not a factory), and the factory really describes
    // the `{name superclasses body}` layout rather than the builtin one.
    let index = factories_of(META_FILE);
    assert_eq!(
        index.keys().collect::<Vec<_>>(),
        ["::tk::Megawidget"],
        "{index:?}"
    );
    let spec = &index["::tk::Megawidget"].overrides["create"];
    assert_eq!((spec.name_arg, spec.body_arg), (1, 3), "{spec:?}");
    assert!(spec.injected.is_some(), "readable prologue: {spec:?}");
}

#[test]
fn a_workspace_indexed_metaclass_creates_real_classes() {
    crate::install_surfaces();
    // TP — the multi-file half.  The consumer file names
    // `::tk::Megawidget` and nothing else about it; the index proves it
    // is a factory, so the class, its members, and the superclasses the
    // manufacturer splices all come out matching the tclsh oracle above.
    let index = factories_of(META_FILE);
    let r = analysis_with_factories(CONSUMER_FILE, &index);
    let cd = r
        .all_classes
        .get("::IconList")
        .unwrap_or_else(|| panic!("::IconList recorded: {:?}", r.all_classes.keys()));
    let mut methods: Vec<&str> = cd.methods.keys().map(String::as_str).collect();
    methods.sort_unstable();
    assert_eq!(methods, ["CreateHull", "GetSpecs"], "{cd:?}");
    assert_eq!(
        cd.superclasses,
        ["::tk::MegawidgetClass", "FocusableWidget"],
        "{cd:?}"
    );
    assert!(!cd.inheritance_unknown, "{cd:?}");
}

#[test]
fn the_cross_file_result_matches_the_same_file_result() {
    crate::install_surfaces();
    // The two tiers must describe the same class: concatenating the
    // metaclass file with the consumer is the same program, so the
    // recorded members / superclasses / opacity flags must agree
    // field-for-field.  (Spans differ — the text is at a different
    // offset — so the comparison is over the modelled facts.)
    let index = factories_of(META_FILE);
    let cross = analysis_with_factories(CONSUMER_FILE, &index);
    let same = analysis(&format!("{META_FILE}{CONSUMER_FILE}"), "tcl9.0");
    let a = &cross.all_classes["::IconList"];
    let b = &same.all_classes["::IconList"];
    assert_eq!(a.superclasses, b.superclasses, "{a:?}\n{b:?}");
    assert_eq!(a.inheritance_unknown, b.inheritance_unknown);
    assert_eq!(a.member_set_incomplete, b.member_set_incomplete);
    let mut ma: Vec<&str> = a.methods.keys().map(String::as_str).collect();
    let mut mb: Vec<&str> = b.methods.keys().map(String::as_str).collect();
    ma.sort_unstable();
    mb.sort_unstable();
    assert_eq!(ma, mb);
}

#[test]
fn the_per_item_walk_agrees_with_the_whole_file_walk_cross_file() {
    crate::install_surfaces();
    // The per-item (incremental) strategy defers proc / method bodies to
    // an isolated second pass, and this campaign has already found two
    // bugs where that path silently lost a fact the whole-file walk kept.
    // A creation call at load level *and* one inside a proc body are both
    // exercised, and the two strategies must produce identical results —
    // asserted directly rather than assumed.
    let index = factories_of(META_FILE);
    let src = concat!(
        "::tk::Megawidget create IconList FocusableWidget {\n",
        "    method GetSpecs {} { return iconlist }\n",
        "}\n",
        "proc late {} {\n",
        "    ::tk::Megawidget create Deferred FocusableWidget {\n",
        "        method Inner {} { return inner }\n",
        "    }\n",
        "}\n",
    );
    let whole = analysis_with_factories(src, &index);
    let per_item = Analyser::new()
        .with_workspace_class_factories(Some(std::sync::Arc::clone(&index)))
        .analyse_per_item(src, "tcl9.0");
    for name in ["::IconList", "::Deferred"] {
        assert!(
            whole.all_classes.contains_key(name),
            "{name} on the whole-file walk: {:?}",
            whole.all_classes.keys().collect::<Vec<_>>()
        );
    }
    assert_eq!(
        whole.all_classes, per_item.all_classes,
        "per-item and whole-file class tables must be identical"
    );
    assert_eq!(whole.diagnostics, per_item.diagnostics);
}

#[test]
fn a_dynamic_metaclass_name_still_abstains() {
    crate::install_surfaces();
    // TN, the non-negotiable half — the command word is a runtime value,
    // so *no* index can prove what it names.  The abstention is narrowed
    // to provable names, never removed.
    let index = factories_of(META_FILE);
    let src = concat!(
        "set meta ::tk::Megawidget\n",
        "$meta create IconList FocusableWidget {\n",
        "    method GetSpecs {} { return iconlist }\n",
        "}\n",
    );
    let r = analysis_with_factories(src, &index);
    assert!(
        !r.all_classes.contains_key("::IconList"),
        "a dynamic head proves nothing: {:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn an_unindexed_metaclass_still_abstains() {
    crate::install_surfaces();
    // TN — the index is populated, but with a different metaclass.  A
    // name it does not carry stays exactly as unrecognised as it was
    // before the index existed.
    let index = factories_of(META_FILE);
    let src = concat!(
        "::some::OtherFactory create IconList FocusableWidget {\n",
        "    method GetSpecs {} { return iconlist }\n",
        "}\n",
    );
    let r = analysis_with_factories(src, &index);
    assert!(
        !r.all_classes.contains_key("::IconList"),
        "{:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_workspace_metaclass_is_not_reached_by_a_same_tailed_bare_name() {
    crate::install_surfaces();
    // TN (cross-link guard) —
    // the index holds `::tk::Megawidget`; a *global* `Megawidget create …`
    // names `::Megawidget`, which real Tcl does not resolve to it.
    // Matching on the tail would manufacture a class the interpreter
    // never makes.
    let index = factories_of(META_FILE);
    let src = concat!(
        "Megawidget create IconList FocusableWidget {\n",
        "    method GetSpecs {} { return iconlist }\n",
        "}\n",
    );
    let r = analysis_with_factories(src, &index);
    assert!(
        !r.all_classes.contains_key("::IconList"),
        "{:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_bare_name_inside_the_metaclass_namespace_does_resolve() {
    crate::install_surfaces();
    // TP — the mirror of the guard above: inside `namespace eval ::tk`,
    // a bare `Megawidget` *is* `::tk::Megawidget` by Tcl's own
    // current-namespace-then-global rule, so it resolves.  Without this
    // the guard could be satisfied by never resolving anything.
    let index = factories_of(META_FILE);
    let src = concat!(
        "namespace eval ::tk {\n",
        "    Megawidget create IconList FocusableWidget {\n",
        "        method GetSpecs {} { return iconlist }\n",
        "    }\n",
        "}\n",
    );
    let r = analysis_with_factories(src, &index);
    assert!(
        r.all_classes.contains_key("::tk::IconList"),
        "{:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_local_metaclass_shadows_the_workspace_one() {
    crate::install_surfaces();
    // TN — a class of the same qualified name written *here* is what the
    // interpreter would have; the index must not override it.  The local
    // `::tk::Megawidget` is an ordinary class, so its `create` makes an
    // instance and no `::IconList` class may appear.
    let index = factories_of(META_FILE);
    let src = concat!(
        "oo::class create ::tk::Megawidget {\n",
        "    method notafactory {} { return 1 }\n",
        "}\n",
        "::tk::Megawidget create IconList FocusableWidget {\n",
        "    method GetSpecs {} { return iconlist }\n",
        "}\n",
    );
    let r = analysis_with_factories(src, &index);
    assert!(
        !r.all_classes.contains_key("::IconList"),
        "the local definition wins: {:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn an_unreadable_cross_file_prologue_marks_inheritance_unknown() {
    crate::install_surfaces();
    // Abstention preserved across the index: the factory's own prologue
    // is built from a runtime value, so it publishes `injected: None` and
    // the consumer records the class with its inheritance opaque —
    // exactly what the same-file path does.
    let meta = concat!(
        "oo::class create ::ns::Meta {\n",
        "    superclass oo::class\n",
        "    self method create {name extra body} {\n",
        "        next $name [list superclass [pickBase $extra]]\\;$body\n",
        "    }\n",
        "}\n",
    );
    let index = factories_of(meta);
    assert!(
        index["::ns::Meta"].overrides["create"].injected.is_none(),
        "{index:?}"
    );
    let src = concat!(
        "::ns::Meta create Widget somewhere {\n",
        "    method m {} { return 1 }\n",
        "}\n",
    );
    let cd = analysis_with_factories(src, &index)
        .all_classes
        .get("::Widget")
        .cloned()
        .expect("::Widget recorded");
    assert!(cd.inheritance_unknown, "{cd:?}");
    assert!(cd.superclasses.is_empty(), "{cd:?}");
    assert!(cd.methods.contains_key("m"), "{cd:?}");
}

#[test]
fn a_cross_file_factory_keeps_method_checks_live_when_it_can() {
    crate::install_surfaces();
    // FN guard — resolving cross-file must not blanket-suppress W308.
    // `PlainMeta` injects nothing provably, so a method that exists
    // nowhere on the manufactured class still warns.
    let meta = concat!(
        "oo::class create ::ns::PlainMeta {\n",
        "    superclass oo::class\n",
        "    self method create {name body} { next $name $body }\n",
        "}\n",
    );
    let index = factories_of(meta);
    let src = concat!(
        "::ns::PlainMeta create PlainWidget {\n",
        "    method own {} { return plain-own }\n",
        "}\n",
        "set w [PlainWidget new]\n",
        "$w nosuchmethod\n",
    );
    let r = analysis_with_factories(src, &index);
    assert!(
        r.diagnostics.iter().any(|d| d.code.to_string() == "W308"),
        "{:?}",
        r.diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect::<Vec<_>>()
    );
}

// The chained factory index.
//
// The workspace index is published by the host as a *fixpoint*, because a
// metaclass manufactured by another file's metaclass is only provable once
// the manufacturing one is already in the index.  The salsa-layer loop is
// covered by `tcl-lsp-db`'s `class_factory_fixpoint` suite; what belongs
// here is the analyser half of the same fact — that a round handed the
// deeper index really does prove the next link, and that the surfaces the
// language allows around it behave.
//
// Ground truth, byte-identical on tclsh 8.6.16 and 9.0.4, for the chain
// below sourced as one script:
//
//   info object isa class ::MC::MetaB      -> 1
//   info class superclasses ::MC::MetaB    -> ::oo::class
//   info object isa class ::MC::Widget     -> 1
//   info class methods ::MC::Widget        -> go
//   [::MC::Widget new] go                  -> went

const CHAIN_META_A: &str =
    "namespace eval ::MC {}\noo::class create ::MC::MetaA { superclass oo::class }\n";

const CHAIN_META_B: &str = "::MC::MetaA create ::MC::MetaB { superclass oo::class }\n";

const CHAIN_WIDGET: &str = "::MC::MetaB create ::MC::Widget { method go {} { return went } }\n";

#[test]
fn a_second_link_metaclass_publishes_a_factory_once_the_first_is_indexed() {
    crate::install_surfaces();
    // TP: Round 1's index (`MetaA` alone,
    // provable with no index at all) is exactly what lets the file holding
    // `MetaA create MetaB` publish `MetaB` — so the host's next round has
    // something new to merge. Without the index the same file publishes
    // nothing, which is the false negative this fixpoint closes.
    let round1 = factories_of(CHAIN_META_A);
    assert_eq!(round1.keys().collect::<Vec<_>>(), ["::MC::MetaA"]);

    let bare = Analyser::new()
        .analyse(CHAIN_META_B, "tcl9.0")
        .class_factories();
    assert!(
        bare.is_empty(),
        "with no oracle the middle link proves nothing: {:?}",
        bare.keys().collect::<Vec<_>>()
    );

    let round2 = analysis_with_factories(CHAIN_META_B, &round1).class_factories();
    assert_eq!(
        round2.keys().collect::<Vec<_>>(),
        ["::MC::MetaB"],
        "handed the first link, the file proves the second"
    );
    assert_eq!(round2["::MC::MetaB"].root_metaclass, "oo::class");
}

#[test]
fn the_third_link_records_the_class_and_its_members() {
    crate::install_surfaces();
    // TP. With both metaclasses in the index the consuming file records a
    // real class with a real method — the fact go-to-definition on `$w go`
    // needs, and the one that came back empty before the fixpoint.
    let mut index = (*factories_of(CHAIN_META_A)).clone();
    for (name, factory) in
        analysis_with_factories(CHAIN_META_B, &factories_of(CHAIN_META_A)).class_factories()
    {
        index.insert(name, factory);
    }
    let index = std::sync::Arc::new(index);
    let r = analysis_with_factories(CHAIN_WIDGET, &index);
    let cd = r
        .all_classes
        .get("::MC::Widget")
        .unwrap_or_else(|| panic!("::MC::Widget: {:?}", r.all_classes.keys()));
    assert_eq!(cd.methods.keys().collect::<Vec<_>>(), ["go"], "{cd:?}");
    assert!(!cd.inheritance_unknown, "{cd:?}");
}

#[test]
fn oo_define_after_the_fact_extends_a_chained_factory_made_class() {
    crate::install_surfaces();
    // The `oo::define` surface: a class manufactured by a second-link
    // metaclass takes later `oo::define` members like any other class.
    //
    // Oracle (tclsh 9.0.4 / 8.6.16): `info class methods ::MC::Widget`
    // -> `go later` after the `oo::define`.
    let index = chained_index();
    let src = concat!(
        "::MC::MetaB create ::MC::Widget { method go {} { return went } }\n",
        "oo::define ::MC::Widget { method later {} { return l } }\n",
    );
    let r = analysis_with_factories(src, &index);
    let cd = &r.all_classes["::MC::Widget"];
    let mut methods: Vec<&str> = cd.methods.keys().map(String::as_str).collect();
    methods.sort_unstable();
    assert_eq!(methods, ["go", "later"], "{cd:?}");
}

#[test]
fn an_unproved_second_link_still_abstains() {
    crate::install_surfaces();
    // TN. Handed an index that names only `MetaA`, a file writing
    // `::MC::MetaB create …` has no proof `::MC::MetaB` manufactures
    // anything — it could be `interp create` for all the walk knows — so it
    // must record nothing rather than guessing the deeper link.
    let r = analysis_with_factories(CHAIN_WIDGET, &factories_of(CHAIN_META_A));
    assert!(
        r.all_classes.is_empty(),
        "no guess from a partially-published index: {:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
}

/// The index a two-round publish produces for this chain:
/// `MetaA` proved with no oracle, `MetaB` proved with `MetaA` published.
fn chained_index() -> std::sync::Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    let first = factories_of(CHAIN_META_A);
    let mut index = (*first).clone();
    for (name, factory) in analysis_with_factories(CHAIN_META_B, &first).class_factories() {
        index.insert(name, factory);
    }
    std::sync::Arc::new(index)
}
