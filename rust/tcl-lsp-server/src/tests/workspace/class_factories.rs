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

//! Workspace metaclasses: the factory oracle after scans and removals.

use super::*;

/// Publication admission is not a failed fixed-point computation. A
/// factory waiter must release all publisher locks until intent retires.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn factory_round_waits_for_snapshot_admission_without_holding_locks() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///factory-snapshot-admission.tcl").unwrap();
    backend
        .db_set_source(&uri, "set value 1\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    let handles = EvidenceHandles {
        db_config: Arc::clone(&backend.db_config),
        folder_db_configs: Arc::clone(&backend.folder_db_configs),
        db: Arc::clone(&backend.db),
        db_files: Arc::clone(&backend.db_files),
        db_project_members: Arc::clone(&backend.db_project_members),
        db_project: Arc::clone(&backend.db_project),
        workspace_index: Arc::clone(&backend.workspace_index),
        documents: Arc::clone(&backend.documents),
        rehoming_gate: Arc::clone(&backend.rehoming_gate),
        live_publication_gate: Arc::clone(&backend.live_publication_gate),
        class_factory_generation: Arc::clone(&backend.class_factory_generation),
    };
    let publication = backend.db.begin_snapshot_drain();
    let round = sync_workspace_class_factories_round(&handles);
    tokio::pin!(round);
    std::future::poll_fn(|context| {
        assert!(std::future::Future::poll(round.as_mut(), context).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    assert_eq!(backend.db.snapshot_report().outstanding, 0);
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        let _db = backend.db.lock().await;
        let _files = backend.db_files.lock().await;
        let project = backend.db_project.lock().await;
        assert!(project.is_some());
    })
    .await
    .expect("a pending factory reader must release publication locks");
    drop(publication);
    let outcome = crate::rt::timeout(std::time::Duration::from_secs(5), round)
        .await
        .expect("the original factory round must resume after publication");
    assert!(matches!(outcome, ClassFactoryRound::Settled));
}

/// The three-file Tk cross-file-metaclass shape, written to a scratch
/// workspace.
///
/// Ground truth (tclsh 8.6.14 and 9.0.4 agree, `source`ing the three in
/// order): `::IconList` is a real class, `info class superclasses
/// ::IconList` is `::tk::MegawidgetClass ::FocusableWidget`, and
/// `info class methods ::IconList -private` is `CreateHull GetSpecs`.
fn write_metaclass_workspace(root: &std::path::Path) {
    std::fs::write(
        root.join("megawidget.tcl"),
        concat!(
            "oo::class create ::tk::MegawidgetClass {\n",
            "    method TraceOption {a b} { return traced }\n",
            "}\n",
            "oo::class create ::tk::Megawidget {\n",
            "    superclass oo::class\n",
            "    self method create {name superclasses body} {\n",
            "        next $name [list superclass ::tk::MegawidgetClass",
            " {*}$superclasses]\\;$body\n",
            "    }\n",
            "}\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("base.tcl"),
        concat!(
            "::tk::Megawidget create FocusableWidget {} {\n",
            "    method CreateHull {} { return h }\n",
            "}\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("iconlist.tcl"),
        concat!(
            "::tk::Megawidget create IconList FocusableWidget {\n",
            "    method GetSpecs {} { return iconlist }\n",
            "    method CreateHull {} { return hull }\n",
            "}\n",
        ),
    )
    .unwrap();
}

/// The salsa analysis a scanned (unopened) workspace file resolves to.
async fn scanned_analysis(
    backend: &Backend,
    root: &std::path::Path,
    name: &str,
) -> Arc<AnalysisResult> {
    let uri = Uri::from_file_path(root.join(name)).unwrap();
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    let file = *files.get(&uri).expect("scanned file is a salsa input");
    let config = *backend.db_config.lock().await;
    tcl_lsp_db::file_analysis_incremental(&*db, file, config)
}

#[tokio::test]
async fn a_cross_file_metaclass_resolves_after_the_workspace_scan() {
    // TP — the whole point. `iconlist.tcl` names
    // `::tk::Megawidget` and nothing else about it; the scan publishes the
    // factory index and the file's own analysis then records the class the
    // interpreter really makes, superclasses and members included.
    let root = unique_scratch_dir("metaclass");
    write_metaclass_workspace(&root);
    let backend = test_backend();
    *backend.workspace_folders.lock().await = vec![Uri::from_file_path(&root).unwrap()];
    backend.scan_workspace_folders().await.unwrap();

    let analysis = scanned_analysis(&backend, &root, "iconlist.tcl").await;
    let class = analysis
        .all_classes
        .get("::IconList")
        .unwrap_or_else(|| panic!("::IconList: {:?}", analysis.all_classes.keys()));
    let mut methods: Vec<&str> = class.methods.keys().map(String::as_str).collect();
    methods.sort_unstable();
    assert_eq!(methods, ["CreateHull", "GetSpecs"], "{class:?}");
    assert_eq!(
        class.superclasses,
        ["::tk::MegawidgetClass", "FocusableWidget"],
        "{class:?}"
    );
    assert!(!class.inheritance_unknown, "{class:?}");

    // The outline must not come back empty for this file.
    let symbols = tcl_lsp_core::document_symbols::document_symbols_from_analysis(
        &std::fs::read_to_string(root.join("iconlist.tcl")).unwrap(),
        &analysis,
    );
    assert!(
        symbols.iter().any(|s| s.name.contains("IconList")),
        "outline names the class: {symbols:?}"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn a_three_level_factory_scan_withdraws_descendants_when_the_source_root_disappears() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Public source indexing and reporting hints, without Native allocation or execution.
    let root = unique_scratch_dir("factory-chain-withdrawal358");
    for (name, source) in [
        (
            "maker.tcl",
            "oo::class create ::MetaA {superclass oo::class}",
        ),
        (
            "derived.tcl",
            "::MetaA create ::MetaB {superclass oo::class}",
        ),
        ("consumer.tcl", "::MetaB create ::Made {method kept {} {}}"),
        (
            "independent.tcl",
            "oo::class create ::Other {superclass oo::class}",
        ),
        (
            "independent-consumer.tcl",
            "::Other create ::Retained {method ok {} {}}",
        ),
    ] {
        std::fs::write(
            root.join(name),
            format!("# tcl-dialect: tcl9.0\n{source}\n"),
        )
        .unwrap();
    }
    let backend = test_backend();
    *backend.workspace_folders.lock().await = vec![Uri::from_file_path(&root).unwrap()];
    backend.scan_workspace_folders().await.unwrap();
    {
        let index = backend.workspace_index.read().await;
        for name in ["::MetaA", "::MetaB", "::Made", "::Other", "::Retained"] {
            assert_eq!(
                index.classes_named(name).len(),
                1,
                "initial source card {name}"
            );
        }
    }
    let (_, initial) = backend.published_class_factories_for_fresh_analysis().await;
    let initial = initial.unwrap();
    assert!(initial.contains_key("::MetaA"));
    assert!(initial.contains_key("::MetaB"));
    assert!(initial.contains_key("::Other"));
    assert!(
        scanned_analysis(&backend, &root, "consumer.tcl")
            .await
            .all_classes
            .contains_key("::Made")
    );

    std::fs::write(root.join("maker.tcl"), "# source factory withdrawn\n").unwrap();
    backend.scan_workspace_folders().await.unwrap();
    {
        let index = backend.workspace_index.read().await;
        for name in ["::MetaA", "::MetaB", "::Made"] {
            assert!(
                index.classes_named(name).is_empty(),
                "withdrawn dependency {name}"
            );
        }
        assert_eq!(index.classes_named("::Other").len(), 1);
        assert_eq!(index.classes_named("::Retained").len(), 1);
    }
    let (_, current) = backend.published_class_factories_for_fresh_analysis().await;
    let current = current.unwrap();
    assert!(!current.contains_key("::MetaA"));
    assert!(!current.contains_key("::MetaB"));
    assert!(current.contains_key("::Other"));
    assert!(
        !scanned_analysis(&backend, &root, "consumer.tcl")
            .await
            .all_classes
            .contains_key("::Made")
    );
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn a_dynamic_metaclass_head_abstains_even_with_the_scan_index() {
    // TN, the non-negotiable half — the index is populated and correct,
    // but the creation call's head is a runtime value, so nothing about it
    // is provable and no class may be invented.
    let root = unique_scratch_dir("metaclass-dyn");
    write_metaclass_workspace(&root);
    std::fs::write(
        root.join("dynamic.tcl"),
        concat!(
            "set meta ::tk::Megawidget\n",
            "$meta create Invented FocusableWidget {\n",
            "    method GetSpecs {} { return nope }\n",
            "}\n",
        ),
    )
    .unwrap();
    let backend = test_backend();
    *backend.workspace_folders.lock().await = vec![Uri::from_file_path(&root).unwrap()];
    backend.scan_workspace_folders().await.unwrap();

    // Non-vacuity: the very same scan *does* resolve the provable file, so
    // this abstention is not just "the index never arrived".
    let resolved = scanned_analysis(&backend, &root, "iconlist.tcl").await;
    assert!(resolved.all_classes.contains_key("::IconList"));

    let analysis = scanned_analysis(&backend, &root, "dynamic.tcl").await;
    assert!(
        !analysis.all_classes.contains_key("::Invented"),
        "a dynamic head proves nothing: {:?}",
        analysis.all_classes.keys().collect::<Vec<_>>()
    );
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn a_workspace_with_no_metaclass_never_sets_the_factory_oracle() {
    // The no-op guarantee: an ordinary workspace must not acquire a new
    // cross-file input at all, or every file in every project would be
    // invalidated by a fact none of them use.
    let root = unique_scratch_dir("metaclass-none");
    std::fs::write(root.join("a.tcl"), "proc greet {n} { puts $n }\n").unwrap();
    std::fs::write(
        root.join("b.tcl"),
        "oo::class create Dog { method bark {} {} }\n",
    )
    .unwrap();
    let backend = test_backend();
    *backend.workspace_folders.lock().await = vec![Uri::from_file_path(&root).unwrap()];
    backend.scan_workspace_folders().await.unwrap();

    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    for (uri, file) in files.iter() {
        assert!(
            file.workspace_class_factories(&*db).is_none(),
            "{uri:?} acquired a factory oracle it has no use for"
        );
    }
    drop(files);
    drop(db);
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn removing_the_last_factory_reindexes_unopened_consumers_with_an_empty_oracle() {
    let root = unique_scratch_dir("metaclass-remove-last");
    write_metaclass_workspace(&root);
    let backend = test_backend();
    *backend.workspace_folders.lock().await = vec![Uri::from_file_path(&root).unwrap()];
    backend.scan_workspace_folders().await.unwrap();
    assert_eq!(
        backend
            .workspace_index
            .read()
            .await
            .classes_named("::IconList")
            .len(),
        1
    );

    std::fs::write(root.join("megawidget.tcl"), "# factory removed\n").unwrap();
    backend.scan_workspace_folders().await.unwrap();

    assert!(
        backend
            .workspace_index
            .read()
            .await
            .classes_named("::IconList")
            .is_empty(),
        "an unopened consumer must lose classes manufactured only by the old oracle"
    );
    let analysis = scanned_analysis(&backend, &root, "iconlist.tcl").await;
    assert!(!analysis.all_classes.contains_key("::IconList"));
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn dropping_one_factory_reindexes_its_consumers_while_retaining_others() {
    let root = unique_scratch_dir("metaclass-drop-one");
    write_metaclass_workspace(&root);
    std::fs::write(
        root.join("other-factory.tcl"),
        concat!(
            "namespace eval ::other {}\n",
            "oo::class create ::other::Factory {\n",
            "    superclass oo::class\n",
            "    self method create {name body} { next $name $body }\n",
            "}\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("other-consumer.tcl"),
        "::other::Factory create ::Kept { method ok {} {} }\n",
    )
    .unwrap();
    let backend = test_backend();
    *backend.workspace_folders.lock().await = vec![Uri::from_file_path(&root).unwrap()];
    backend.scan_workspace_folders().await.unwrap();
    {
        let index = backend.workspace_index.read().await;
        assert_eq!(index.classes_named("::IconList").len(), 1);
        assert_eq!(index.classes_named("::Kept").len(), 1);
    }

    std::fs::write(root.join("megawidget.tcl"), "# one factory removed\n").unwrap();
    backend.scan_workspace_folders().await.unwrap();

    let index = backend.workspace_index.read().await;
    assert_eq!(
        index.classes_named("::IconList"),
        [] as [&tcl_lsp_core::workspace_index::WorkspaceClass; 0]
    );
    assert_eq!(index.classes_named("::Kept").len(), 1);
    drop(index);
    std::fs::remove_dir_all(&root).ok();
}
