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

//! What the `source` graph and `package require` order: sourced exports, providers, imported path constants and source ancestors.

use super::*;

#[test]
fn original_requirement_graph_keeps_constraints_preference_and_unknowns() {
    // Implementation contract: naming.workspace.original-package-requirement-transport
    // docs/design/analysis/name-resolution-proofs/workspace-original-package-requirement-transport.md

    use crate::package_resolver::{PackagePrefer, PackageRequirementAdviceKey};
    let source = "package require -exact p\\uD800 1.2\npackage prefer latest\npackage require p\\uD800 1.0 2.0\npackage require $computed\n";
    let analysis = analyse(source);
    let mut index = WorkspaceIndex::from_documents([("file:///library.tcl", &analysis)]);
    let requirements =
        index.original_package_requirement_advice_for("file:///library.tcl", PackagePrefer::Stable);
    assert_eq!(requirements.len(), 3);
    assert_eq!(
        requirements[0].key().original().unwrap().bytes(),
        b"p\xed\xa0\x80"
    );
    assert_eq!(requirements[0].requirements(), &["1.2"]);
    assert!(requirements[0].exact());
    assert_eq!(requirements[0].prefer(), PackagePrefer::Stable);
    assert_eq!(requirements[1].key(), requirements[0].key());
    assert_eq!(requirements[1].requirements(), &["1.0", "2.0"]);
    assert!(!requirements[1].exact());
    assert_eq!(requirements[1].prefer(), PackagePrefer::Latest);
    assert_eq!(requirements[2].key(), &PackageRequirementAdviceKey::Unknown);
    let edges = vec![
        (
            "file:///library.tcl".to_owned(),
            "file:///consumer.tcl".to_owned(),
        ),
        (
            "file:///consumer.tcl".to_owned(),
            "file:///library.tcl".to_owned(),
        ),
    ];
    let records =
        std::collections::HashMap::from([("file:///library.tcl".to_owned(), requirements.clone())]);
    assert_eq!(
        crate::source_graph::ancestor_requirements("file:///consumer.tcl", &edges, &records),
        requirements
    );
    assert!(
        index
            .original_package_requirement_advice_for("file:///library.tcl", PackagePrefer::Latest)
            .iter()
            .all(|required| required.prefer() == PackagePrefer::Latest)
    );
    index.remove_document("file:///library.tcl");
    assert_eq!(
        index.original_package_requirement_advice_for("file:///library.tcl", PackagePrefer::Stable),
        [] as [crate::package_resolver::PackageRequirementAdvice; 0]
    );
}

#[test]
fn retained_source_routes_share_original_plans_across_graph_consumers() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let app = analyse(
        "interp alias {} build {} file join /proj; package require Tk; package prefer latest; source [build lib.tcl]",
    );
    let child = analyse("proc leaf {} {}");
    let index = sourced_index([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/lib.tcl", &child),
    ]);
    let edges = index.resolved_source_edges();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].child, "file:///proj/lib.tcl");
    assert_eq!(
        index.source_ancestor_package_requires_from_resolved("file:///proj/lib.tcl"),
        vec!["Tk".to_owned()]
    );
    assert!(index.source_ancestor_prefers_latest_from_resolved("file:///proj/lib.tcl"));
    assert_eq!(
        index
            .source_seed_map(test_resolve)
            .get("file:///proj/lib.tcl"),
        Some(&std::collections::BTreeSet::from(["::".to_owned()]))
    );
    let shadow =
        analyse("proc file {args} {}; package require Tk; source [file join /proj lib.tcl]");
    let index = sourced_index([
        ("file:///proj/app.tcl", &shadow),
        ("file:///proj/lib.tcl", &child),
    ]);
    assert_ne!(
        index.sources().collect::<Vec<_>>(),
        [] as [&crate::workspace_index::WorkspaceSource; 0]
    );
    assert_eq!(
        index.resolved_source_edges(),
        [] as [crate::source_graph::RunEdge; 0]
    );
    assert!(index.source_seed_map(test_resolve).is_empty());
    assert_eq!(
        index.source_ancestor_package_requires_from_resolved("file:///proj/lib.tcl"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn an_export_sourced_before_the_import_resolves() {
    // TP: `app.tcl` sources the export before the import, so the import
    // really did see it — resolution here rests on a proved order rather
    // than an abstention.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse("source mod.tcl\nsource exp.tcl\nsource imp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn an_export_sourced_after_the_import_is_not_retroactive() {
    // FP guard (CRITICAL) — the whole point of the
    // order. The import runs first, `::mymod` has exported nothing yet,
    // so real Tcl installs no alias at all. Byte-identical documents to
    // the test above; only `app.tcl`'s two `source` lines swap.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse("source mod.tcl\nsource imp.tcl\nsource exp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///p/caller.tcl"),
    );
    assert!(
        resolved.is_none(),
        "an export sourced after the import cannot apply retroactively: {resolved:?}",
    );
}

#[test]
fn without_a_resolver_the_same_workspace_keeps_abstaining() {
    // TN for the deployment shape: an index with no `source` resolver
    // installed ranks no cross-document event — the foreign export counts
    // and the import resolves, whichever way the `source` statements are
    // written.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse("source mod.tcl\nsource imp.tcl\nsource exp.tcl\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn a_re_sourced_export_file_keeps_abstaining() {
    // TN (CRITICAL): `exp.tcl` is sourced twice, so it has no unique
    // position and the order must not invent one — Tcl tolerates
    // re-sourcing, and guessing would silently drop a real alias. Falls
    // back to the pre-graph abstention: the export counts.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse("source mod.tcl\nsource imp.tcl\nsource exp.tcl\nsource exp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

// The `package require` half of the load order.
//
// Every test below builds the index with `WorkspaceIndex::from_documents`
// — no `source` resolver at all — because the package half needs none: a
// `package require NAME` names its provider through the index's own
// `package provide` records.
//
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4, with `pkg/lib.tcl`
// holding the provider and `pkg/pkgIndex.tcl` the usual
// `[list source [file join $dir lib.tcl]]`:
//
//   package require mymod
//   namespace eval ::mymod { namespace export -clear }
//   namespace eval ::app { namespace import ::mymod::* }
//     -> NO ALIAS (invalid command name "::app::helper")
//
//   namespace eval ::mymod { namespace export -clear }
//   package require mymod
//   namespace eval ::app { namespace import ::mymod::* }
//     -> HELP
//
// …and the second script, byte-identical, answers NO ALIAS when some
// other file required `mymod` before it ran.  That flip is the whole
// reason the second shape abstains.

/// The provider document every `package require` order test shares: it
/// defines and exports `helper`, and provides the package.
fn package_provider() -> AnalysisResult {
    analyse(
        "namespace eval ::mymod { proc helper {} { return HELP }\n namespace export helper }\npackage provide mymod 1.0\n",
    )
}

/// Whether `helper` still resolves through `::app`'s wildcard import in
/// `index` — the question every test below asks.
fn helper_resolves(index: &WorkspaceIndex) -> Option<String> {
    index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///p/caller.tcl"),
    )
}

#[test]
fn a_clear_after_a_package_require_revokes_the_providers_export() {
    // TP (CRITICAL) — the movement the package half exists for. `app.tcl`
    // requires the package (so `lib.tcl` has run), clears `::mymod`'s
    // exports, and only then imports. Without the package order the
    // foreign export would count and the local `-clear` would revoke
    // nothing, so `helper` would resolve; real Tcl installs no alias at
    // all.
    let lib = package_provider();
    let app = analyse(
        "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///p/lib.tcl", &lib), ("file:///p/app.tcl", &app)]);
    let resolved = helper_resolves(&index);
    assert!(
        resolved.is_none(),
        "the `-clear` provably ran after the provider's export and before the import: {resolved:?}",
    );
    // Non-vacuity: with the `-clear` removed, the same workspace resolves,
    // so the assertion above is the ordering's doing.
    let app_no_clear =
        analyse("package require mymod\nnamespace eval ::app { namespace import ::mymod::* }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &lib),
        ("file:///p/app.tcl", &app_no_clear),
    ]);
    assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
}

#[test]
fn abstention_2_a_statement_above_the_require_stays_unordered() {
    // TN (CRITICAL) — uncertainty 2, the package may already be loaded,
    // in which case the `package require` runs nothing at all and the
    // provider's statements are *older* than everything in this file.
    //
    // A `source` in this shape settles it — "an export sourced after the
    // import is not retroactive", the test above. A `package require`
    // must not, and the oracle is a file whose answer flips with nothing
    // in it changed. `imp_body.tcl`, byte for byte:
    //
    //   namespace eval ::mymod {}
    //   namespace eval ::app { namespace import ::mymod::* }
    //   package require mymod
    //
    //   sourced on its own            -> NO ALIAS
    //   sourced after `package require mymod` elsewhere -> HELP
    //
    // So the order says nothing and the pre-existing "abstain toward
    // answering" stands: the foreign export counts, `helper` resolves.
    let lib = package_provider();
    let app = analyse(
        "namespace eval ::mymod {}\nnamespace eval ::app { namespace import ::mymod::* }\npackage require mymod\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///p/lib.tcl", &lib), ("file:///p/app.tcl", &app)]);
    assert_eq!(
        helper_resolves(&index).as_deref(),
        Some("::mymod::helper"),
        "a require never proves the provider had *not* yet run",
    );
    // …and the same shape with a `-clear` above the require is equally
    // unordered: run alone the clear hits an empty `::mymod` and the
    // require then loads the exporting file (oracle: HELP), run after
    // any other file required `mymod` the very same text revokes
    // (oracle: NO ALIAS).
    let app = analyse(
        "namespace eval ::mymod { namespace export -clear }\npackage require mymod\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///p/lib.tcl", &lib), ("file:///p/app.tcl", &app)]);
    assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
}

#[test]
fn abstention_1_two_documents_providing_one_package_order_nothing() {
    // TN — uncertainty 1, `auto_path` is mutable. Two indexed files
    // provide `mymod`; which one a `package require` runs is decided by
    // the runtime search-path order, and only one of them exports.
    // Oracle (8.6.14 / 9.0.4), two directories each holding a `mylib2
    // 1.0`: `lappend auto_path pkgA pkgB` gives `from A` with `p`
    // exported, `pkgB pkgA` gives `from B` with nothing exported. So no
    // edge, and the `-clear` that would otherwise revoke does not.
    let lib = package_provider();
    let other = analyse(
        "namespace eval ::mymod { proc helper {} { return OTHER } }\npackage provide mymod 1.0\n",
    );
    let app = analyse(
        "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &lib),
        ("file:///p/other/lib.tcl", &other),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
}

#[test]
fn abstention_3_an_indexed_ifneeded_script_orders_nothing() {
    // TN — uncertainty 3, `ifneeded` bodies are arbitrary. Once the
    // workspace holds the package's own index script, the statements a
    // `package require mymod` runs are that script's business, and it may
    // load something else entirely. Oracle (8.6.14 / 9.0.4): one
    // `pkgIndex.tcl` whose body branches on the environment answers
    // `::c::p` -> `real` (with `namespace export p` run) or -> `stub`
    // (with nothing exported), for the same `package require`.
    let lib = package_provider();
    let pkg_index = analyse("package ifneeded mymod 1.0 [list source [file join $dir lib.tcl]]\n");
    let app = analyse(
        "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &lib),
        ("file:///p/pkgIndex.tcl", &pkg_index),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
}

#[test]
fn a_conditional_or_dynamic_require_orders_nothing() {
    // TN: the optional-dependency idiom `if {[catch {package require …}]}`
    // may never run, and `package require $pkg` names nothing statically.
    // Both mirror the gates `package prefer latest` already applies.
    let lib = package_provider();
    for app_src in [
        "catch {package require mymod}\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
        "set pkg mymod\npackage require $pkg\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    ] {
        let app = analyse(app_src);
        let index = WorkspaceIndex::from_documents([
            ("file:///p/lib.tcl", &lib),
            ("file:///p/app.tcl", &app),
        ]);
        assert_eq!(
            helper_resolves(&index).as_deref(),
            Some("::mymod::helper"),
            "{app_src}",
        );
    }
}

#[test]
fn a_conditional_provide_does_not_make_a_document_the_provider() {
    // TN: the shim idiom. tcllib's `doctools2idx/import_json.tcl` writes
    // `package provide dict 1` inside `if {[package vcompare …] < 0} { if
    // {[catch {package require dict}]} { … } }` — a fake `dict` package
    // supplied only on an old interpreter. Reading that as "this document
    // provides `dict`" would name the wrong file as the provider on every
    // other interpreter, so a guarded provide establishes nothing.
    let shim = analyse(
        "if {[package vcompare [package present Tcl] 8.5] < 0} {\nnamespace eval ::mymod { proc helper {} { return SHIM } }\npackage provide mymod 1.0\n}\n",
    );
    let app = analyse(
        "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &package_provider()),
        ("file:///p/shim.tcl", &shim),
        ("file:///p/app.tcl", &app),
    ]);
    // Only `lib.tcl` provides `mymod` unconditionally, so the edge stands
    // and the `-clear` revokes…
    assert!(helper_resolves(&index).is_none());
    // …but with `lib.tcl`'s provide made conditional too, nothing in the
    // workspace provably provides `mymod` and the order abstains.
    let guarded_lib = analyse(
        "namespace eval ::mymod { proc helper {} { return HELP }\n namespace export helper }\nif {$::tcl_platform(platform) eq \"unix\"} { package provide mymod 1.0 }\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &guarded_lib),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
}

#[test]
fn one_provider_required_from_many_documents_orders_each_of_them() {
    // TP: a package required from three files is loaded once, by whichever
    // require runs first — so it has three upper bounds and no unique
    // position. Each requiring file is still ordered against it, which a
    // tree position (one entry site per document) could not express.
    let lib = package_provider();
    let clearing = analyse(
        "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let bystander = analyse("package require mymod\nputs [::mymod::helper]\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &lib),
        ("file:///p/app.tcl", &clearing),
        ("file:///p/one.tcl", &bystander),
        ("file:///p/two.tcl", &bystander),
    ]);
    assert!(helper_resolves(&index).is_none());
}

#[test]
fn an_unfoldable_computed_source_path_orders_nothing() {
    // TN: `source $dir/exp.tcl` where `dir` comes from `[lindex $argv 0]`
    // names no document statically — `lindex` is outside the evaluator's
    // subset, so the constant never folds — the edge is dropped and the
    // export goes back to being unrankable: the deliberate abstention,
    // not an accident of path resolution.
    //
    // Spelling `dir` as `[file dirname [info script]]` instead would fold
    // through the chained single-assignment evaluator; that foldable
    // spelling has its own TP pin directly below.
    let (mod_doc, exp, imp) = import_order_modules();
    let app =
        analyse("set dir [lindex $argv 0]\nsource mod.tcl\nsource imp.tcl\nsource $dir/exp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn inherited_path_constants_cross_twelve_source_hops_and_retire_with_the_edge() {
    let root_source = "set dir /s\nsource /s/hop1.tcl\n";
    let root = tcl_compiler::analyser::Analyser::new().analyse(root_source, "tcl8.6");
    let mut documents = vec![("file:///s/root.tcl".to_owned(), root.clone())];
    for hop in 1..=12 {
        let source = if hop == 12 {
            "source $dir/leaf.tcl\n".to_owned()
        } else {
            format!("source /s/hop{}.tcl\n", hop + 1)
        };
        documents.push((
            format!("file:///s/hop{hop}.tcl"),
            tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl8.6"),
        ));
    }
    documents.push((
        "file:///s/leaf.tcl".to_owned(),
        tcl_compiler::analyser::Analyser::new().analyse("proc leaf {} {}\n", "tcl8.6"),
    ));
    let mut index = sourced_index(
        documents
            .iter()
            .map(|(uri, analysis)| (uri.as_str(), analysis)),
    );
    assert_eq!(
        index
            .imported_path_constants_for("file:///s/hop12.tcl")
            .get("dir")
            .map(String::as_str),
        Some("/s")
    );
    assert!(
        index
            .source_seed_map(test_resolve)
            .contains_key("file:///s/leaf.tcl")
    );
    let removed = tcl_compiler::analyser::Analyser::new().analyse("set dir /s\n", "tcl8.6");
    index.replace_document("file:///s/root.tcl", &removed);
    assert!(
        !index
            .imported_path_constants_for("file:///s/hop12.tcl")
            .contains_key("dir")
    );
    assert!(
        !index
            .source_seed_map(test_resolve)
            .contains_key("file:///s/leaf.tcl")
    );
    index.replace_document("file:///s/root.tcl", &root);
    assert_eq!(
        index
            .imported_path_constants_for("file:///s/hop12.tcl")
            .get("dir")
            .map(String::as_str),
        Some("/s")
    );
}

#[test]
fn typed_source_imports_export_global_homes_and_keep_original_site_scope() {
    let parent = tcl_compiler::analyser::Analyser::new().analyse(
            "namespace eval N {set dir /EPHEMERAL; source /s/shared.tcl}; set globalDir /GLOBAL; source /s/shared.tcl", "jim");
    let reader = tcl_compiler::analyser::Analyser::new()
        .analyse("source $dir/missing.tcl; source $globalDir/core.tcl", "jim");
    let core = tcl_compiler::analyser::Analyser::new().analyse("", "jim");
    let index = sourced_index([
        ("file:///s/start.tcl", &parent),
        ("file:///s/shared.tcl", &reader),
        ("file:///GLOBAL/core.tcl", &core),
    ]);
    let imports = index.imported_path_constants_for("file:///s/shared.tcl");
    assert!(!imports.contains_key("dir"));
    // The earlier namespace route supplies no globalDir; agreement must
    // not use the later route's value for all executions.
    assert!(!imports.contains_key("globalDir"));
    assert_eq!(
        imports.naming_policy(),
        parent.path_constant_assignments.naming_policy()
    );
    let seeds = index.source_seed_map(test_resolve);
    assert!(!seeds.contains_key("file:///s/missing.tcl"));
}

#[test]
fn path_inventory_reset_and_snapshot_preserve_empty_policy_changes() {
    let c = tcl_compiler::analyser::Analyser::new().analyse("", "tcl8.6");
    let jim = tcl_compiler::analyser::Analyser::new().analyse("", "jim");
    let mut index = sourced_index([("file:///empty.tcl", &c)]);
    let before = index.path_constant_assignments("file:///empty.tcl").clone();
    assert!(before.is_empty());
    index.add_document("file:///empty.tcl", &jim);
    assert_ne!(
        index.path_constant_assignments("file:///empty.tcl"),
        &before
    );
    assert_eq!(
        index
            .path_constant_assignments("file:///missing.tcl")
            .naming_policy(),
        None
    );
}

#[test]
fn a_cross_file_constant_resolves_a_readers_source_rows_1368() {
    // OSVVM's shape in miniature: two simulator entry scripts each
    // assign the same namespace constant and source the shared reader;
    // the reader sources a grandchild *through the imported value*.
    // Both parents live in one directory, so the agreement rule keeps
    // the name and the reader's own row resolves.
    let parent = analyse(
        "namespace eval ::osvvm {\n    variable dir [file dirname [file normalize [info script]]]\n}\nsource ${::osvvm::dir}/shared.tcl\n",
    );
    let reader = analyse("source ${::osvvm::dir}/core.tcl\n");
    let core = analyse("proc core_helper {} {}\n");
    let index = sourced_index([
        ("file:///s/start1.tcl", &parent),
        ("file:///s/start2.tcl", &parent),
        ("file:///s/shared.tcl", &reader),
        ("file:///s/core.tcl", &core),
    ]);
    assert_eq!(
        index
            .imported_path_constants_for("file:///s/shared.tcl")
            .get("::osvvm::dir")
            .map(String::as_str),
        Some("/s"),
    );
    let seeds = index.source_seed_map(test_resolve);
    assert!(
        seeds.contains_key("file:///s/core.tcl"),
        "the reader's own source row must resolve through the import: {seeds:?}",
    );
}

#[test]
fn disagreeing_parents_import_nothing_1368() {
    // TN: the same reader sourced from two directories — the constant
    // folds to a different value per parent, so no route-independent
    // value exists and the name must not import.  Whichever parent runs
    // decides, and this tier never guesses.
    let parent = analyse(
        "namespace eval ::osvvm {\n    variable dir [file dirname [file normalize [info script]]]\n}\nsource /s/shared.tcl\n",
    );
    let reader = analyse("source ${::osvvm::dir}/core.tcl\n");
    let core = analyse("proc core_helper {} {}\n");
    let index = sourced_index([
        ("file:///a/start1.tcl", &parent),
        ("file:///b/start2.tcl", &parent),
        ("file:///s/shared.tcl", &reader),
        ("file:///s/core.tcl", &core),
    ]);
    assert!(
        !index
            .imported_path_constants_for("file:///s/shared.tcl")
            .contains_key("::osvvm::dir"),
    );
    let seeds = index.source_seed_map(test_resolve);
    assert!(
        !seeds.contains_key("file:///s/core.tcl"),
        "no agreed value, no edge: {seeds:?}",
    );
}

#[test]
fn a_body_local_source_route_imports_nothing_1368() {
    // A `source` inside a proc runs when the proc is called, not at its
    // lexical position: `set dir /a; proc load {} {source shared.tcl};
    // set dir /b; load` reaches the child with `/b`.  The lexical
    // position gate would say `/a`, so a body-local route must supply
    // nothing at all.
    let parent = analyse(
        "namespace eval ::cfg {\n    variable dir [file dirname [file normalize [info script]]]\n}\nproc load {} {\n    source /s/shared.tcl\n}\n",
    );
    let reader = analyse("source ${::cfg::dir}/core.tcl\n");
    let core = analyse("proc core_helper {} {}\n");
    let index = sourced_index([
        ("file:///s/start.tcl", &parent),
        ("file:///s/shared.tcl", &reader),
        ("file:///s/core.tcl", &core),
    ]);
    assert!(
        !index
            .imported_path_constants_for("file:///s/shared.tcl")
            .contains_key("::cfg::dir"),
        "a body-local source route must not provide load-time constants",
    );
}

#[test]
fn an_assignment_after_the_source_does_not_import_1368() {
    // TN: the parent assigns the constant *after* sourcing the reader —
    // when the reader runs, the variable does not exist yet, and the
    // position gate must say so.
    let parent = analyse(
        "source /s/shared.tcl\nnamespace eval ::osvvm {\n    variable dir [file dirname [file normalize [info script]]]\n}\n",
    );
    let reader = analyse("source ${::osvvm::dir}/core.tcl\n");
    let core = analyse("proc core_helper {} {}\n");
    let index = sourced_index([
        ("file:///s/start1.tcl", &parent),
        ("file:///s/shared.tcl", &reader),
        ("file:///s/core.tcl", &core),
    ]);
    assert!(
        !index
            .imported_path_constants_for("file:///s/shared.tcl")
            .contains_key("::osvvm::dir"),
        "a write after the source has not happened when the child runs",
    );
}

#[test]
fn a_foldable_computed_source_path_orders_its_export() {
    // TP: the same shape with `dir` from `[file dirname
    // [info script]]` *does* fold through the chained constant, so
    // `$dir/exp.tcl` names its document and the load order ranks it.
    // With the order known the answer sharpens: app.tcl runs the
    // wildcard import *before* sourcing exp.tcl's `namespace export`,
    // and `namespace import` copies only currently-exported commands
    // (a later export does not retro-import), so `helper` is genuinely
    // not imported and resolution answers nothing — more precise than
    // the abstention the unfoldable spelling above falls back to.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse(
        "set dir [file dirname [info script]]\nsource mod.tcl\nsource imp.tcl\nsource $dir/exp.tcl\n",
    );
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        None,
    );
}

#[test]
fn a_forget_written_after_the_source_revokes_the_sourced_import() {
    // TP (CRITICAL), the `source lib.tcl ; namespace forget ::lib::p`
    // idiom: the install from the sourced file counts *and* the forget
    // beside the `source` revokes it. Oracle: `::app::helper` is an
    // `invalid command name` after the forget.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse(
        "source mod.tcl\nsource exp.tcl\nsource imp.tcl\nnamespace eval ::app { namespace forget ::mymod::helper }\n",
    );
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    // A call written after the forget no longer resolves…
    let after = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///p/app.tcl", u32::MAX),
    );
    assert!(
        after.is_none(),
        "a forget written after the `source` that installed the alias must revoke it: {after:?}",
    );
    // …and a call written *above* every `source` statement has no alias
    // yet either — the install has not run at that point.
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_at("file:///p/app.tcl", 0),
            )
            .as_deref(),
        None,
    );
    // TP: with the forget removed, the same call site after the sources
    // does resolve — so the assertion above is the forget's doing and not
    // an accident of the graph.
    let app_no_forget = analyse("source mod.tcl\nsource exp.tcl\nsource imp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app_no_forget),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_at("file:///p/app.tcl", u32::MAX),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn a_cross_file_import_conflict_is_decided_by_the_source_order() {
    // TP (CRITICAL) — two imports of one name from
    // different sources, in different files. Without an order neither
    // conflicts and the later one silently installs; with one, the file
    // sourced first owns the name and the second import raises `can't
    // import command "helper": already exists` and installs nothing.
    //
    // Oracle (8.6.14 / 9.0.4), with the two importers in separate files
    // sourced in this order:
    //   namespace origin ::app::helper  ->  ::first::helper
    let first = analyse(
        "namespace eval ::first { proc helper {} { return F }\n namespace export helper }\n",
    );
    let second = analyse(
        "namespace eval ::second { proc helper {} { return S }\n namespace export helper }\n",
    );
    let imp_a = analyse("namespace eval ::app { namespace import ::first::* }\n");
    let imp_b = analyse("namespace eval ::app { namespace import ::second::* }\n");
    let app = analyse("source first.tcl\nsource second.tcl\nsource impa.tcl\nsource impb.tcl\n");
    let docs = [
        ("file:///p/first.tcl", &first),
        ("file:///p/second.tcl", &second),
        ("file:///p/impa.tcl", &imp_a),
        ("file:///p/impb.tcl", &imp_b),
        ("file:///p/app.tcl", &app),
    ];
    assert_eq!(
        sourced_index(docs)
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::first::helper"),
        "the import sourced first owns the name; the second conflicts and installs nothing",
    );
    // Swap the two `source` lines and the winner swaps with them.
    let app = analyse("source first.tcl\nsource second.tcl\nsource impb.tcl\nsource impa.tcl\n");
    let docs = [
        ("file:///p/first.tcl", &first),
        ("file:///p/second.tcl", &second),
        ("file:///p/impa.tcl", &imp_a),
        ("file:///p/impb.tcl", &imp_b),
        ("file:///p/app.tcl", &app),
    ];
    assert_eq!(
        sourced_index(docs)
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::second::helper"),
    );
}

#[test]
fn indexes_and_removes_package_requires() {
    let a = analyse("package require Tk\npackage require http\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    assert_eq!(
        index.package_requires_for("file:///a.tcl"),
        vec!["Tk".to_owned(), "http".to_owned()]
    );
    index.remove_document("file:///a.tcl");
    assert!(index.package_requires().next().is_none());
}

#[test]
fn source_ancestor_requires_walks_the_graph() {
    // app.tcl requires Tk and sources lib/util.tcl; util inherits Tk.
    let app = analyse("package require Tk\nsource lib/util.tcl\n");
    let util = analyse("proc u {} {}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/lib/util.tcl", &util),
    ]);
    // Resolver mirrors the server's: join the raw path onto the parent's
    // directory (path portion of the file URI).
    let resolve = |parent: &str, raw: &str| -> Option<String> {
        let dir = parent.rsplit_once('/').map(|(d, _)| d)?;
        Some(format!("{dir}/{raw}"))
    };
    let got = index.source_ancestor_package_requires("file:///proj/lib/util.tcl", resolve);
    assert_eq!(got, vec!["Tk".to_owned()]);
    // The entry file itself inherits nothing.
    assert_eq!(
        index.source_ancestor_package_requires("file:///proj/app.tcl", resolve),
        [] as [std::string::String; 0]
    );
}

#[test]
fn source_ancestor_requires_ignores_nonliteral_sources() {
    // A computed `source $path` produces no resolvable edge.
    let app = analyse("package require Tk\nsource $dir/util.tcl\n");
    let util = analyse("proc u {} {}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/util.tcl", &util),
    ]);
    let resolve = |_p: &str, _r: &str| -> Option<String> { panic!("non-literal must not resolve") };
    assert_eq!(
        index.source_ancestor_package_requires("file:///proj/util.tcl", resolve),
        [] as [std::string::String; 0]
    );
}

/// `package prefer latest` is interpreter-global, and
/// along the `source` graph "ran first" is a static fact.
///
/// tclsh-proof (8.6.14), with `lib.tcl` holding `puts [package prefer]`:
/// `app.tcl` written as `package prefer latest; source lib.tcl` prints
/// `latest`, and as `source lib.tcl; package prefer latest` prints
/// `stable`.
#[test]
fn source_ancestor_prefer_latest_crosses_documents() {
    let resolve = |parent: &str, raw: &str| -> Option<String> {
        let dir = parent.rsplit_once('/').map(|(d, _)| d)?;
        Some(format!("{dir}/{raw}"))
    };
    let lib = analyse("package require w\n");

    // TP — the raise runs before the `source`.
    let app = analyse("package prefer latest\nsource lib.tcl\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/lib.tcl", &lib),
    ]);
    assert!(index.source_ancestor_prefers_latest("file:///proj/lib.tcl", resolve));
    // …and the entry file itself inherits nothing (its own raise is the
    // position-sensitive single-document question).
    assert!(!index.source_ancestor_prefers_latest("file:///proj/app.tcl", resolve));

    // FP guard — the raise runs after the `source`.
    let app = analyse("source lib.tcl\npackage prefer latest\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/lib.tcl", &lib),
    ]);
    assert!(!index.source_ancestor_prefers_latest("file:///proj/lib.tcl", resolve));

    // FP guard — a conditional raise is never recorded at all.
    let app = analyse("if {$::c} { package prefer latest }\nsource lib.tcl\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/lib.tcl", &lib),
    ]);
    assert!(index.package_prefers().next().is_none());
    assert!(!index.source_ancestor_prefers_latest("file:///proj/lib.tcl", resolve));
}

/// The prefer rows drop with their document, like every other table.
#[test]
fn package_prefers_are_dropped_with_their_document() {
    let a = analyse("package prefer latest\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    assert_eq!(index.package_prefers().count(), 1);
    index.remove_document("file:///a.tcl");
    assert!(index.package_prefers().next().is_none());
}
