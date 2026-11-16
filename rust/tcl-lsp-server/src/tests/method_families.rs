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

//! Object methods across files: families, consumers, navigation and rename.

use super::*;

#[test]
fn workspace_class_cache_key_covers_bare_word_and_factory_oracles() {
    let classes = HashSet::from(["::Widget".to_owned()]);
    let no_bare_words = HashSet::new();
    let bare_words = HashSet::from(["::Widget".to_owned()]);
    let base = workspace_class_analysis_fingerprint(
        "Widget .w\n",
        tcl_lsp_core::profile_for_dialect("tcl9.0"),
        &classes,
        &no_bare_words,
        None,
    );
    let changed_bare_words = workspace_class_analysis_fingerprint(
        "Widget .w\n",
        tcl_lsp_core::profile_for_dialect("tcl9.0"),
        &classes,
        &bare_words,
        None,
    );
    assert_ne!(base, changed_bare_words);

    let analysis = Analyser::new()
        .analyse("oo::class create Meta { superclass oo::class }\n", "tcl9.0")
        .clone();
    let factories = analysis.class_factories();
    let changed_factories = workspace_class_analysis_fingerprint(
        "Widget .w\n",
        tcl_lsp_core::profile_for_dialect("tcl9.0"),
        &classes,
        &no_bare_words,
        Some(&factories),
    );
    assert_ne!(base, changed_factories);

    let changed_analysis = Analyser::new()
        .analyse(
            concat!(
                "oo::class create Meta {\n",
                "  superclass oo::class\n",
                "  method unknown {w args} {\n",
                "    [self] create ::$w\n",
                "    return $w\n",
                "  }\n",
                "}\n",
            ),
            "tcl9.0",
        )
        .clone();
    let changed_factory_content = changed_analysis.class_factories();
    assert_eq!(
        factories.keys().collect::<Vec<_>>(),
        changed_factory_content.keys().collect::<Vec<_>>()
    );
    assert_ne!(
        workspace_class_analysis_fingerprint(
            "Widget .w\n",
            tcl_lsp_core::profile_for_dialect("tcl9.0"),
            &classes,
            &no_bare_words,
            Some(&factories),
        ),
        workspace_class_analysis_fingerprint(
            "Widget .w\n",
            tcl_lsp_core::profile_for_dialect("tcl9.0"),
            &classes,
            &no_bare_words,
            Some(&changed_factory_content),
        )
    );
}

#[tokio::test]
async fn method_rename_spans_override_family_across_documents() {
    // Animal::speak in animal.tcl; Dog (subclass) overrides it in
    // dog.tcl and calls it via `$d speak`.  Renaming from the base decl
    // must rewrite the override *and* the external call site in the
    // sibling document.
    let backend = test_backend();
    let animal = Uri::from_str("file:///animal.tcl").unwrap();
    let dog = Uri::from_str("file:///dog.tcl").unwrap();
    register(
        &backend,
        &animal,
        "oo::class create Animal {\n    method speak {} {}\n}\n",
    )
    .await;
    register(
            &backend,
            &dog,
            "oo::class create Dog {\n    superclass Animal\n    method speak {} {}\n}\nset d [Dog new]\n$d speak\n",
        )
        .await;
    let params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: animal.clone(),
            },
            position: Position::new(1, 11), // on `speak` in Animal's decl
        },
        new_name: "vocalise".to_owned(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let edit = backend.rename(params).await.expect("ok").expect("some");
    let changes = edit.changes.expect("changes");
    assert!(
        changes.contains_key(&animal),
        "base doc edits missing: {changes:?}"
    );
    assert!(
        changes.contains_key(&dog),
        "override doc edits missing: {changes:?}"
    );
    // Every edit renames to `vocalise`.
    assert!(changes.values().flatten().all(|e| e.new_text == "vocalise"));
    // dog.tcl: the override declaration (line 2) + the `$d speak` call
    // site (line 5) are both rewritten.
    let dog_lines: Vec<u32> = changes[&dog].iter().map(|e| e.range.start.line).collect();
    assert!(
        dog_lines.contains(&2) && dog_lines.contains(&5),
        "expected override decl (l2) + call site (l5) in dog.tcl; got {:?}",
        changes[&dog],
    );
}

#[tokio::test]
async fn method_rename_reaches_subclass_only_document() {
    // Animal::speak in animal.tcl; Dog *inherits* speak (no override) in
    // dog.tcl and calls it via `my speak` and `$d speak`.  dog.tcl holds
    // no definer, so the family-only pass never opened it; the inheritor
    // pass must, or those sites silently keep the old name.
    let backend = test_backend();
    let animal = Uri::from_str("file:///animal.tcl").unwrap();
    let dog = Uri::from_str("file:///dog.tcl").unwrap();
    register(
        &backend,
        &animal,
        "oo::class create Animal {\n    method speak {} {}\n}\n",
    )
    .await;
    register(
            &backend,
            &dog,
            "oo::class create Dog {\n    superclass Animal\n    method describe {} { my speak }\n}\nset d [Dog new]\n$d speak\n",
        )
        .await;
    let params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: animal.clone(),
            },
            position: Position::new(1, 11), // on `speak` in Animal's decl
        },
        new_name: "vocalise".to_owned(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let edit = backend.rename(params).await.expect("ok").expect("some");
    let changes = edit.changes.expect("changes");
    assert!(
        changes.contains_key(&animal),
        "base doc edits missing: {changes:?}"
    );
    assert!(
        changes.contains_key(&dog),
        "subclass-only doc edits missing: {changes:?}",
    );
    assert!(changes.values().flatten().all(|e| e.new_text == "vocalise"));
    // dog.tcl: `my speak` (line 2) + `$d speak` (line 5) rewritten; there
    // is no declaration to rewrite in this file.
    let dog_lines: Vec<u32> = changes[&dog].iter().map(|e| e.range.start.line).collect();
    assert!(
        dog_lines.contains(&2) && dog_lines.contains(&5),
        "expected `my speak` (l2) + `$d speak` (l5) in dog.tcl; got {:?}",
        changes[&dog],
    );
}

#[tokio::test]
async fn classmethod_rename_reaches_subclass_only_document() {
    // The classmethod analogue, end-to-end through the production
    // `rename` handler: `SubFactory` inherits `make` (no override) in
    // its own file and dispatches it via `SubFactory make` — a bare
    // classmethod dispatch on the *subclass's* own command, which
    // `subfactory.tcl`'s own `all_classes` cannot classify as a
    // classmethod call by itself (it never declares `make`).
    let backend = test_backend();
    let factory = Uri::from_str("file:///factory5.tcl").unwrap();
    let subfactory = Uri::from_str("file:///subfactory5.tcl").unwrap();
    register(
            &backend,
            &factory,
            "oo::class create Factory {\n    classmethod make {} {\n        return [Factory new]\n    }\n}\n",
        )
        .await;
    register(
        &backend,
        &subfactory,
        "oo::class create SubFactory {\n    superclass Factory\n}\nSubFactory make\n",
    )
    .await;
    let params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: factory.clone(),
            },
            position: Position::new(1, 16), // on `make` in Factory's decl
        },
        new_name: "build".to_owned(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let edit = backend.rename(params).await.expect("ok").expect("some");
    let changes = edit.changes.expect("changes");
    assert!(
        changes.contains_key(&subfactory),
        "subclass-only doc edits missing: {changes:?}"
    );
    let sub_edits = &changes[&subfactory];
    assert!(
        sub_edits
            .iter()
            .any(|e| e.range.start.line == 3 && e.new_text == "build"),
        "expected `SubFactory make` (l3) rewritten to `build`: {sub_edits:?}"
    );
}

#[tokio::test]
async fn rename_disambiguates_method_and_classmethod_of_the_same_name() {
    // Same fixture as `code_lens_resolve_disambiguates_method_and_classmethod_of_the_same_name`,
    // but through the production `rename` handler: renaming from the
    // cursor on `method make`'s declaration must rewrite only the
    // method's own declaration and its `$f make` dispatch — never the
    // unrelated `classmethod make`'s declaration or its `Factory make`
    // dispatch.
    let backend = test_backend();
    let uri = Uri::from_str("file:///dual_make_rename.tcl").unwrap();
    let src = "oo::class create Factory {\n    method make {} { return 1 }\n    classmethod make {} { return [Factory new] }\n}\nset f [Factory new]\n$f make\nFactory make\n";
    register(&backend, &uri, src).await;
    let params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position::new(1, 11), // on `make` in `method make`'s decl
        },
        new_name: "build".to_owned(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let edit = backend.rename(params).await.expect("ok").expect("some");
    let changes = edit.changes.expect("changes");
    let edits = &changes[&uri];
    let renamed_lines: Vec<u32> = edits
        .iter()
        .filter(|e| e.new_text == "build")
        .map(|e| e.range.start.line)
        .collect();
    assert!(
        renamed_lines.contains(&1),
        "method decl (l1) must be renamed: {edits:?}"
    );
    assert!(
        renamed_lines.contains(&5),
        "`$f make` (l5) must be renamed: {edits:?}"
    );
    assert!(
        !renamed_lines.contains(&2),
        "classmethod decl (l2) must NOT be renamed: {edits:?}"
    );
    assert!(
        !renamed_lines.contains(&6),
        "`Factory make` (l6, classmethod dispatch) must NOT be renamed: {edits:?}"
    );
}

/// Register `animal.tcl` (`Animal::speak`) and `dog.tcl` (a `Dog` subclass
/// that overrides `speak` and calls `$d speak`).  Returns the backend and
/// the two URIs.
async fn register_method_family_workspace() -> (Backend, Uri, Uri) {
    let backend = test_backend();
    let animal = Uri::from_str("file:///animal.tcl").unwrap();
    let dog = Uri::from_str("file:///dog.tcl").unwrap();
    register(
        &backend,
        &animal,
        "oo::class create Animal {\n    method speak {} {}\n}\n",
    )
    .await;
    register(
            &backend,
            &dog,
            "oo::class create Dog {\n    superclass Animal\n    method speak {} {}\n}\nset d [Dog new]\n$d speak\n",
        )
        .await;
    (backend, animal, dog)
}

#[tokio::test]
async fn cross_file_method_references_span_override_family() {
    // References on `Animal::speak` reach the override declaration and the
    // `$d speak` call site in the sibling dog.tcl.
    let (backend, animal, dog) = register_method_family_workspace().await;
    let refs = backend
        .cross_file_method_references(
            &animal,
            "::Animal",
            "speak",
            true,
            false,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
        )
        .await
        .unwrap();
    let dog_lines: Vec<u32> = refs
        .iter()
        .filter(|l| l.uri == dog)
        .map(|l| l.range.start.line)
        .collect();
    assert!(
        dog_lines.contains(&2),
        "override decl (l2) missing: {refs:?}"
    );
    assert!(dog_lines.contains(&5), "`$d speak` (l5) missing: {refs:?}");
    // The current document is excluded — the single-document provider
    // already covers its own method sites.
    assert!(
        refs.iter().all(|l| l.uri != animal),
        "current document must be excluded: {refs:?}",
    );
}

#[tokio::test]
async fn cross_file_method_references_exclude_declaration() {
    // With `include_declaration` false, the sibling's override *declaration*
    // (l2) is dropped but the `$d speak` call site (l5) stays.
    let (backend, animal, dog) = register_method_family_workspace().await;
    let refs = backend
        .cross_file_method_references(
            &animal,
            "::Animal",
            "speak",
            false,
            false,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
        )
        .await
        .unwrap();
    let dog_lines: Vec<u32> = refs
        .iter()
        .filter(|l| l.uri == dog)
        .map(|l| l.range.start.line)
        .collect();
    assert!(!dog_lines.contains(&2), "decl should be excluded: {refs:?}");
    assert!(dog_lines.contains(&5), "call site (l5) missing: {refs:?}");
}

#[tokio::test]
async fn cross_file_method_references_reach_inheritor_document() {
    // Dog *inherits* speak (no override) and calls it via `my speak` and
    // `$d speak` in a document holding no definer.  The inheritor pass must
    // still reach those sites.
    let backend = test_backend();
    let animal = Uri::from_str("file:///animal.tcl").unwrap();
    let dog = Uri::from_str("file:///dog.tcl").unwrap();
    register(
        &backend,
        &animal,
        "oo::class create Animal {\n    method speak {} {}\n}\n",
    )
    .await;
    register(
            &backend,
            &dog,
            "oo::class create Dog {\n    superclass Animal\n    method run {} { my speak }\n}\nset d [Dog new]\n$d speak\n",
        )
        .await;
    let refs = backend
        .cross_file_method_references(
            &animal,
            "::Animal",
            "speak",
            false,
            false,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
        )
        .await
        .unwrap();
    let dog_lines: Vec<u32> = refs
        .iter()
        .filter(|l| l.uri == dog)
        .map(|l| l.range.start.line)
        .collect();
    // `my speak` (l2) and `$d speak` (l5).
    assert!(dog_lines.contains(&2), "`my speak` (l2) missing: {refs:?}");
    assert!(dog_lines.contains(&5), "`$d speak` (l5) missing: {refs:?}");
}

#[tokio::test]
async fn cross_file_method_references_reach_inheritor_document_for_classmethod() {
    // The classmethod analogue: SubFactory *inherits* `make` (no
    // override) and dispatches it via its own command, in a document
    // holding no definer of `Factory` at all — unlike the pure-consumer
    // case, this document *does* declare a class (SubFactory), so it
    // goes through the inheritor pass (`inherited_method_call_sites`),
    // not `cross_file_consumer_method_references`.
    let backend = test_backend();
    let factory = Uri::from_str("file:///factory4.tcl").unwrap();
    let subfactory = Uri::from_str("file:///subfactory4.tcl").unwrap();
    register(
            &backend,
            &factory,
            "oo::class create Factory {\n    classmethod make {} {\n        return [Factory new]\n    }\n}\n",
        )
        .await;
    register(
        &backend,
        &subfactory,
        "oo::class create SubFactory {\n    superclass Factory\n}\nSubFactory make\n",
    )
    .await;
    let refs = backend
        .cross_file_method_references(
            &factory,
            "::Factory",
            "make",
            false,
            true,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
        )
        .await
        .unwrap();
    let sub_lines: Vec<u32> = refs
        .iter()
        .filter(|l| l.uri == subfactory)
        .map(|l| l.range.start.line)
        .collect();
    assert!(
        sub_lines.contains(&3),
        "`SubFactory make` (l3) in the inheritor-only document missing: {refs:?}"
    );
}

#[tokio::test]
async fn cross_file_method_references_empty_for_unrelated_method() {
    // A method name no family class defines yields nothing — no spurious
    // cross-file sites.
    let (backend, animal, _dog) = register_method_family_workspace().await;
    let refs = backend
        .cross_file_method_references(
            &animal,
            "::Animal",
            "nonexistent",
            true,
            false,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
        )
        .await
        .unwrap();
    assert!(refs.is_empty(), "{refs:?}");
}

#[tokio::test]
async fn cross_file_consumer_finds_classmethod_bare_dispatch() {
    // A `classmethod` dispatches on the class's own command, never an
    // instance — so a *pure-consumer* document that never
    // declares/extends the class (only calls `Factory make`) has no
    // entry for `::Factory` in its own `all_classes`, unlike the
    // instance path (`$obj method`, resolved via a plain
    // `instance_classes` string match that needs no `all_classes` at
    // all). `cross_file_consumer_method_references` closes this gap by
    // asking the *workspace index* whether `method` is a classmethod
    // (`WorkspaceMethod::kind`) and, if so, passing the family's class
    // names down as valid bare-dispatch heads — the same centralised
    // `kind` field the same-document path already reads locally.
    let backend = test_backend();
    let factory = Uri::from_str("file:///factory.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
    let factory_src = "oo::class create Factory {\n    classmethod make {} {\n        return [Factory new]\n    }\n}\n";
    register(&backend, &factory, factory_src).await;
    register(&backend, &consumer, "Factory make\n").await;
    let refs = backend
        .cross_file_consumer_method_references(
            &factory,
            factory_src,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
            "::Factory",
            "make",
            true,
        )
        .await
        .unwrap();
    assert!(
        refs.iter()
            .any(|l| l.uri == consumer && l.range.start.line == 0),
        "`Factory make` in the pure-consumer document missing: {refs:?}"
    );
}

#[tokio::test]
async fn cross_file_consumer_bare_dispatch_is_namespace_scoped() {
    // The consumer-document variant. Two classes named `Factory`, in
    // `::a` and `::b`, each declaring `make`; a consumer document
    // dispatches `Factory make` inside `namespace eval ::b`. Real Tcl
    // resolves that bare word to `::b::Factory` (tclsh 8.6.14 and 9.0.4
    // both answer `b-made`), so it must count for `::b`'s classmethod and
    // *not* `::a`'s — a name-set match counts it for both, and rename then
    // rewrites an unrelated class's call site.
    let backend = test_backend();
    let a = Uri::from_str("file:///a.tcl").unwrap();
    let b = Uri::from_str("file:///b.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
    let a_src = "namespace eval ::a {\n    oo::class create Factory {\n        classmethod make {} { return 1 }\n    }\n}\n";
    let b_src = "namespace eval ::b {\n    oo::class create Factory {\n        classmethod make {} { return 2 }\n    }\n}\n";
    register(&backend, &a, a_src).await;
    register(&backend, &b, b_src).await;
    register(
        &backend,
        &consumer,
        "namespace eval ::b {\n    Factory make\n}\n",
    )
    .await;

    let from_a = backend
        .cross_file_consumer_method_references(
            &a,
            a_src,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
            "::a::Factory",
            "make",
            true,
        )
        .await
        .unwrap();
    assert!(
        !from_a.iter().any(|l| l.uri == consumer),
        "`::b`'s bare dispatch must not count for `::a::Factory`: {from_a:?}"
    );

    let from_b = backend
        .cross_file_consumer_method_references(
            &b,
            b_src,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
            "::b::Factory",
            "make",
            true,
        )
        .await
        .unwrap();
    assert!(
        from_b
            .iter()
            .any(|l| l.uri == consumer && l.range.start.line == 1),
        "and it must count for `::b::Factory`: {from_b:?}"
    );
}

#[tokio::test]
async fn cross_file_consumer_finds_inheriting_subclass_classmethod_dispatch() {
    // The same gap, one level of inheritance deeper: `SubFactory` inherits
    // (does not override) `make`, and the pure-consumer document
    // dispatches via `SubFactory`'s own command.
    let backend = test_backend();
    let factory = Uri::from_str("file:///factory2.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer2.tcl").unwrap();
    let factory_src = "oo::class create Factory {\n    classmethod make {} { return [Factory new] }\n}\noo::class create SubFactory {\n    superclass Factory\n}\n";
    register(&backend, &factory, factory_src).await;
    register(&backend, &consumer, "SubFactory make\n").await;
    let refs = backend
        .cross_file_consumer_method_references(
            &factory,
            factory_src,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
            "::Factory",
            "make",
            true,
        )
        .await
        .unwrap();
    assert!(
        refs.iter()
            .any(|l| l.uri == consumer && l.range.start.line == 0),
        "`SubFactory make` (inherited classmethod) in the pure-consumer document missing: {refs:?}"
    );
}

/// TN: a stock-`TclOO` `self method` is not
/// inherited, so a subclass's own class command never reaches it —
/// tclsh 8.6 and 9.0.4 both answer `Gadget make` with `unknown method
/// "make": must be create, destroy or new`.  A consumer document's
/// `Gadget make` therefore calls something else entirely and must not
/// appear as a reference (nor be rewritten by a rename) when the
/// parent's `self method make` is renamed.  The TP half — the parent's
/// own `Parent make` — must still be found.
#[tokio::test]
async fn cross_file_consumer_excludes_subclass_dispatch_of_a_self_method() {
    let backend = test_backend();
    let parent = Uri::from_str("file:///selfparent.tcl").unwrap();
    let consumer = Uri::from_str("file:///selfconsumer.tcl").unwrap();
    let parent_src = "oo::class create Parent {\n    self method make {} { return 1 }\n}\noo::class create Gadget {\n    superclass Parent\n}\n";
    register(&backend, &parent, parent_src).await;
    register(&backend, &consumer, "Parent make\nGadget make\n").await;
    let refs = backend
        .cross_file_consumer_method_references(
            &parent,
            parent_src,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
            "::Parent",
            "make",
            true,
        )
        .await
        .unwrap();
    assert!(
        refs.iter()
            .any(|l| l.uri == consumer && l.range.start.line == 0),
        "`Parent make` (the declaring class's own dispatch) missing: {refs:?}"
    );
    assert!(
        !refs
            .iter()
            .any(|l| l.uri == consumer && l.range.start.line == 1),
        "`Gadget make` is not a call of a non-inherited `self method`: {refs:?}"
    );
}

/// The rename half of the same rule: the consumer edit list rewrites
/// `Parent make` and leaves `Gadget make` alone.  References and rename
/// resolve consumer sites through the one resolver, so this pins that
/// they stay in step.
#[tokio::test]
async fn cross_file_rename_of_a_self_method_leaves_subclass_dispatch_alone() {
    let backend = test_backend();
    let parent = Uri::from_str("file:///selfparent2.tcl").unwrap();
    let consumer = Uri::from_str("file:///selfconsumer2.tcl").unwrap();
    let parent_src = "oo::class create Parent {\n    self method make {} { return 1 }\n}\noo::class create Gadget {\n    superclass Parent\n}\n";
    register(&backend, &parent, parent_src).await;
    register(&backend, &consumer, "Parent make\nGadget make\n").await;
    let ranges = backend
        .consumer_method_site_ranges(
            &parent,
            parent_src,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
            ConsumerTarget {
                seed_class: "::Parent",
                method: "make",
                is_classmethod: true,
                coverage: ConsumerCoverage::WholeWorkspace,
            },
        )
        .await
        .unwrap();
    let consumer_ranges: Vec<u32> = ranges
        .iter()
        .filter(|(uri, _)| *uri == consumer)
        .flat_map(|(_, rs)| rs.iter().map(|r| r.start.line))
        .collect();
    assert_eq!(consumer_ranges, vec![0], "{ranges:?}");
}

#[tokio::test]
async fn cross_file_consumer_does_not_bare_dispatch_a_plain_instance_method() {
    // FP guard: a plain (non-classmethod) `method` must not gain bare
    // class-command dispatch cross-file, any more than it does
    // same-document — `Factory get` (no `$obj`) is not a valid TclOO
    // call for an instance method.
    let backend = test_backend();
    let factory = Uri::from_str("file:///factory3.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer3.tcl").unwrap();
    let factory_src = "oo::class create Factory {\n    method get {} { return 1 }\n}\n";
    register(&backend, &factory, factory_src).await;
    register(&backend, &consumer, "Factory get\n").await;
    let refs = backend
        .cross_file_consumer_method_references(
            &factory,
            factory_src,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
            "::Factory",
            "get",
            false,
        )
        .await
        .unwrap();
    assert!(
        !refs.iter().any(|l| l.uri == consumer),
        "a plain method must not gain phantom class-command dispatch: {refs:?}"
    );
}

#[tokio::test]
async fn cross_file_consumer_does_not_bare_dispatch_an_itcl_class_proc() {
    // FP guard, cross-file: [incr Tcl]'s class-scoped `proc` lands in the
    // same `class_methods` bucket as a `classmethod`/`typemethod` (so the
    // workspace index also sees it as `WorkspaceMethod::kind ==
    // "classmethod"`), but itcl dispatches it as a single
    // `::`-qualified identifier (`Factory::make`), never the two-word
    // `Factory make` shape — which is itcl's own instance-creation
    // syntax (`ClassName instanceName`) for an unrelated object named
    // "make".  `classmethod_dispatch_names` must exclude itcl definers
    // the same way the same-document scanner does.
    let backend = test_backend();
    let factory = Uri::from_str("file:///factory5.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer5.tcl").unwrap();
    let factory_src = "itcl::class Factory {\n    proc make {} {\n        return 1\n    }\n}\n";
    register(&backend, &factory, factory_src).await;
    register(&backend, &consumer, "Factory make\n").await;
    let refs = backend
        .cross_file_consumer_method_references(
            &factory,
            factory_src,
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
            "::Factory",
            "make",
            true,
        )
        .await
        .unwrap();
    assert!(
        !refs.iter().any(|l| l.uri == consumer),
        "itcl class-proc must not gain phantom two-word dispatch: {refs:?}"
    );
}

#[tokio::test]
async fn references_handler_includes_cross_file_method_sites() {
    // End-to-end through the `references` handler: the cursor on
    // `Animal::speak`'s declaration surfaces the override + `$d speak` in
    // dog.tcl alongside the current document's own sites.
    let (backend, animal, dog) = register_method_family_workspace().await;
    let params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: animal.clone(),
            },
            position: Position::new(1, 11), // on `speak` in Animal's decl
        },
        context: ReferenceContext {
            include_declaration: true,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let refs = Box::pin(backend.references(params))
        .await
        .expect("ok")
        .expect("some references");
    assert!(
        refs.iter().any(|l| l.uri == dog && l.range.start.line == 5),
        "cross-file `$d speak` (dog.tcl l5) missing: {refs:?}",
    );
    assert!(
        refs.iter().any(|l| l.uri == animal),
        "current-document declaration missing: {refs:?}",
    );
}

#[tokio::test]
async fn cross_file_method_definition_jumps_to_inherited_declaration() {
    // `Dog` inherits `speak` from `Animal` in another file.  Go-to-def on a
    // `Dog` instance's `speak` call resolves to `Animal::speak`'s
    // declaration in animal.tcl.
    let backend = test_backend();
    let animal = Uri::from_str("file:///animal.tcl").unwrap();
    let dog = Uri::from_str("file:///dog.tcl").unwrap();
    register(
        &backend,
        &animal,
        "oo::class create Animal {\n    method speak {} {}\n}\n",
    )
    .await;
    register(
        &backend,
        &dog,
        "oo::class create Dog {\n    superclass Animal\n}\nset d [Dog new]\n$d speak\n",
    )
    .await;
    let defs = Box::pin(backend.compute_definition(&dog, Position::new(4, 4)))
        .await
        .unwrap();
    assert_eq!(defs.len(), 1, "{defs:?}");
    assert_eq!(defs[0].uri, animal);
    assert_eq!(defs[0].range.start.line, 1, "{defs:?}");
}

#[tokio::test]
async fn goto_definition_follows_inherited_method_cross_file() {
    // End-to-end through `compute_definition`: the cursor on `$d speak`
    // (dog.tcl) jumps to the inherited `Animal::speak` declaration in
    // animal.tcl — a method-call cursor is not a command head, so the
    // dedicated method path (not the command-head cross-doc fallback)
    // resolves it.
    let backend = test_backend();
    let animal = Uri::from_str("file:///animal.tcl").unwrap();
    let dog = Uri::from_str("file:///dog.tcl").unwrap();
    register(
        &backend,
        &animal,
        "oo::class create Animal {\n    method speak {} {}\n}\n",
    )
    .await;
    register(
        &backend,
        &dog,
        "oo::class create Dog {\n    superclass Animal\n}\nset d [Dog new]\n$d speak\n",
    )
    .await;
    // `$d speak` is on line 4; `speak` starts at column 3.
    let defs = Box::pin(backend.compute_definition(&dog, Position::new(4, 4)))
        .await
        .expect("ok");
    assert_eq!(defs.len(), 1, "{defs:?}");
    assert_eq!(defs[0].uri, animal);
    assert_eq!(defs[0].range.start.line, 1, "{defs:?}");
}

#[tokio::test]
async fn cross_file_definition_selects_the_dispatch_entry_945() {
    // With `Animal::speak` overridden by
    // `Dog::speak` in another file, a `Dog` receiver's definition
    // request identifies the runtime entry (`Dog::speak`) only —
    // never the whole override family (tclsh 9.0.4: `info object
    // call` = Dog then Animal).
    let backend = test_backend();
    let animal = Uri::from_str("file:///animal.tcl").unwrap();
    let dog = Uri::from_str("file:///dog.tcl").unwrap();
    let main = Uri::from_str("file:///main.tcl").unwrap();
    register(
        &backend,
        &animal,
        "oo::class create Animal {\n    method speak {} { return animal }\n}\n",
    )
    .await;
    register(
        &backend,
        &dog,
        "oo::class create Dog {\n    superclass Animal\n    method speak {} { return dog }\n}\n",
    )
    .await;
    register(&backend, &main, "set d [Dog new]\n$d speak\n").await;
    let defs = Box::pin(backend.compute_definition(&main, Position::new(1, 4)))
        .await
        .expect("ok");
    assert_eq!(
        defs.len(),
        1,
        "go-to-definition must select Dog::speak only: {defs:?}"
    );
    assert_eq!(defs[0].uri, dog, "{defs:?}");
    assert_eq!(defs[0].range.start.line, 2, "{defs:?}");
}

#[tokio::test]
async fn cross_file_definition_refuses_an_unexported_method_945() {
    // `Vault::_secret` is default-unexported
    // (tclsh 9.0.4: `unknown method "_secret"`), so a consumer file's
    // external `$v _secret` resolves to nothing — cross-file
    // navigation must not resolve what C rejects.
    let backend = test_backend();
    let vault = Uri::from_str("file:///vault.tcl").unwrap();
    let main = Uri::from_str("file:///main.tcl").unwrap();
    register(
        &backend,
        &vault,
        "oo::class create Vault {\n    method _secret {} { return hidden }\n}\n",
    )
    .await;
    register(&backend, &main, "set v [Vault new]\n$v _secret\n").await;
    let defs = Box::pin(backend.compute_definition(&main, Position::new(1, 5)))
        .await
        .expect("ok");
    assert!(
        defs.is_empty(),
        "an externally unexported TclOO method is not callable: {defs:?}"
    );
}

/// Register `animal.tcl` (`Animal::speak`) and a pure-consumer `main.tcl`
/// that only creates and uses an `Animal` instance — it defines no part of
/// the class.
async fn register_consumer_workspace() -> (Backend, Uri, Uri) {
    let backend = test_backend();
    let animal = Uri::from_str("file:///animal.tcl").unwrap();
    let main = Uri::from_str("file:///main.tcl").unwrap();
    register(
        &backend,
        &animal,
        "oo::class create Animal {\n    method speak {} {}\n}\n",
    )
    .await;
    register(&backend, &main, "set a [Animal new]\n$a speak\n").await;
    (backend, animal, main)
}

#[tokio::test]
async fn references_reach_pure_consumer_document() {
    // References on `Animal::speak` include the `$a speak` call in the
    // pure-consumer main.tcl, which defines no part of `Animal` and so is
    // invisible to the family/inheritor pass — only the workspace class
    // oracle resolves `a` to `::Animal` there.
    let (backend, animal, main) = register_consumer_workspace().await;
    let refs = backend
        .cross_file_consumer_method_references(
            &animal,
            "oo::class create Animal {\n    method speak {} {}\n}\n",
            tcl_lsp_core::profile_for_dialect("tcl8.6"),
            "::Animal",
            "speak",
            false,
        )
        .await
        .unwrap();
    assert!(
        refs.iter()
            .any(|l| l.uri == main && l.range.start.line == 1),
        "`$a speak` in the consumer document missing: {refs:?}",
    );
}

#[tokio::test]
async fn references_handler_from_consumer_cursor_finds_family() {
    // End-to-end: the cursor on `$a speak` in the pure-consumer main.tcl
    // resolves the method oracle-aware and finds `Animal::speak`'s
    // declaration in animal.tcl plus its own call site.
    let (backend, animal, main) = register_consumer_workspace().await;
    let params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: main.clone() },
            position: Position::new(1, 4), // on `speak` in `$a speak`
        },
        context: ReferenceContext {
            include_declaration: true,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let refs = Box::pin(backend.references(params))
        .await
        .expect("ok")
        .expect("some references");
    assert!(
        refs.iter()
            .any(|l| l.uri == animal && l.range.start.line == 1),
        "cross-file declaration `Animal::speak` missing: {refs:?}",
    );
    assert!(
        refs.iter()
            .any(|l| l.uri == main && l.range.start.line == 1),
        "consumer call `$a speak` missing: {refs:?}",
    );
}

#[tokio::test]
async fn goto_definition_from_consumer_cursor_finds_cross_file_method() {
    // Go-to-definition on `$a speak` in the pure-consumer main.tcl resolves
    // `a`'s class cross-file (oracle-aware) and jumps to `Animal::speak`.
    let (backend, animal, main) = register_consumer_workspace().await;
    let defs = Box::pin(backend.compute_definition(&main, Position::new(1, 4)))
        .await
        .expect("ok");
    assert_eq!(defs.len(), 1, "{defs:?}");
    assert_eq!(defs[0].uri, animal);
    assert_eq!(defs[0].range.start.line, 1, "{defs:?}");
}
