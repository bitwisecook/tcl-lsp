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

//! Fresh analysis seeds, the configured analyser and the dialect registry.

use super::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn fresh_analysis_driver_preserves_native_send_boundary() {
    fn require_send<T: Send>() {}

    require_send::<Analyser>();
    require_send::<Arc<tcl_compiler::compilation_unit::CompilationUnit>>();
    require_send::<Box<FreshAnalysisDriver>>();
}

#[test]
fn configured_analyser_threads_mode_and_disabled() {
    // `Off` mode suppresses W108 entirely.
    let mut a = Backend::configured_analyser(
        HashSet::new(),
        NonAsciiMode::Off,
        HashSet::new(),
        0,
        ResourceAnalyserInputs::default(),
    );
    let r = a.analyse("set x \u{201c}hi\u{201d}\n", "tcl8.6");
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W108));
    // A disabled code is filtered from the analyser's output.
    let mut disabled = HashSet::new();
    disabled.insert("W108".to_string());
    let mut b = Backend::configured_analyser(
        disabled,
        NonAsciiMode::Strict,
        HashSet::new(),
        0,
        ResourceAnalyserInputs::default(),
    );
    let r = b.analyse("set x \u{201c}hi\u{201d}\n", "tcl8.6");
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W108));
}

/// The recovery path's widened known-command set must be
/// rebuilt only when the workspace index (or the package database, or the
/// configured extra commands) actually changes — not on every keystroke
/// inside an unterminated block.
#[tokio::test]
async fn recovery_widening_is_reused_across_edits_and_invalidated_by_the_index() {
    let backend = test_backend();
    let registry = tcl_registry::model::ingress::static_context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .commands();
    let dialects = provider_source::ProviderDialectSources::of(&backend);
    let providers = provider_source::ProviderCaptureInputs {
        db: &backend.db,
        files: &backend.db_files,
        global: &backend.db_config,
        folders: &backend.folder_db_configs,
        store: &backend.store,
        source_path_admissions: &backend.source_path_admissions,
        dialects: &dialects,
        proposal: None,
    };
    let ctx = RecoveryWidenCtx {
        cache: &backend.recovery_names,
        registry,
        workspace_index: &backend.workspace_index,
        package_resolver: &backend.package_resolver,
        provider_capture: &providers,
        prefer: tcl_lsp_core::package_resolver::PackagePrefer::Stable,
    };
    let base: HashSet<String> = ["mycmd".to_owned()].into_iter().collect();

    // Seed the index with a workspace proc, so the widened set has
    // something index-derived in it.
    {
        let mut analysis = tcl_compiler::analyser::Analyser::new();
        let result = analysis.analyse("proc ::ws::helper {} {}\n", "tcl8.6");
        backend
            .workspace_index
            .write()
            .await
            .add_document("file:///ws.tcl", &result);
    }

    let first = widen_recovery_extra_commands(
        &ctx,
        &base,
        "proc foo {",
        &Analyser::new()
            .structure_only()
            .analyse("proc foo {", "tcl8.6"),
    )
    .await;
    assert!(
        first.names.contains("ws::helper")
            && first.names.contains("helper")
            && first.names.contains("mycmd"),
        "the widened set must carry the workspace proc (qualified + tail) and the base",
    );

    // A keystroke inside the same unterminated block: different text, same
    // index / package database / config — the identical allocation is
    // handed back.
    let second = widen_recovery_extra_commands(
        &ctx,
        &base,
        "proc foo {x",
        &Analyser::new()
            .structure_only()
            .analyse("proc foo {x", "tcl8.6"),
    )
    .await;
    assert!(
        Arc::ptr_eq(&first.names, &second.names),
        "an edit that does not change the index must reuse the cached set",
    );

    // Indexing another document bumps the index generation, which must
    // invalidate the memo.
    {
        let mut analysis = tcl_compiler::analyser::Analyser::new();
        let result = analysis.analyse("proc ::ws::second {} {}\n", "tcl8.6");
        backend
            .workspace_index
            .write()
            .await
            .add_document("file:///ws2.tcl", &result);
    }
    let third = widen_recovery_extra_commands(
        &ctx,
        &base,
        "proc foo {x",
        &Analyser::new()
            .structure_only()
            .analyse("proc foo {x", "tcl8.6"),
    )
    .await;
    assert!(
        !Arc::ptr_eq(&second.names, &third.names),
        "an index change must rebuild the widened set",
    );
    assert!(
        third.names.contains("ws::second"),
        "the rebuilt set must carry the newly-indexed proc",
    );
}

#[tokio::test]
async fn registry_for_dialect_reuses_cached_registry() {
    let backend = test_backend();
    let first = backend.registry_for_dialect("f5-irules").await;
    let second = backend.registry_for_dialect("f5-irules").await;

    assert!(
        Arc::ptr_eq(&first, &second),
        "repeated dialect lookups should return the cached registry",
    );
    assert!(
        first
            .profile()
            .is_some_and(tcl_dialect::DialectProfile::is_irules),
        "the shared cache stamps the dialect profile the fold policy reads",
    );
}

/// The independent index seed must consume the
/// same analysis text as Salsa. A raw old-Mac buffer leaves the proc inside
/// the preceding comment; normalising lone CRs makes it a real command.
#[tokio::test]
async fn fresh_open_index_seed_normalises_lone_cr_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///fresh-old-mac-1854.tcl").unwrap();
    let analysis = backend
        .fresh_analysis_for(
            &uri,
            Arc::from("# note\rproc current {} {}\r"),
            "tcl8.6".to_owned(),
        )
        .await
        .unwrap();

    assert!(
        analysis.all_procs.contains_key("::current"),
        "the fresh index seed must parse the same normalised source as Salsa",
    );
}

/// The standalone seed must receive the source path just like the Salsa
/// analyser. `pkgIndex.tcl` gives `$dir` an implicit initial value, so a
/// pathless seed incorrectly reports its first assignment as dead.
#[tokio::test]
async fn fresh_open_index_seed_carries_file_path_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///packages/demo/pkgIndex.tcl").unwrap();
    let analysis = backend
        .fresh_analysis_for(
            &uri,
            Arc::from("set dir first\nset dir second\nputs $dir\n"),
            "tcl8.6".to_owned(),
        )
        .await
        .unwrap();

    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != DiagCode::W220),
        "the fresh pkgIndex seed must honour its loader-provided $dir: {:?}",
        analysis.diagnostics,
    );
}

/// An index seed is independent of Salsa's
/// snapshot lifetime, not independent of the source's cross-file inputs.
/// Carry the current factory oracle so a class made by another file's
/// metaclass exists in the first published live index.
#[tokio::test]
async fn fresh_open_index_seed_carries_workspace_class_factories_1854() {
    use salsa::Setter as _;

    let backend = test_backend();
    let uri = Uri::from_str("file:///fresh-metaclass-consumer-1854.tcl").unwrap();
    let consumer = concat!(
        "::tk::Megawidget create IconList FocusableWidget {\n",
        "    method GetSpecs {} { return iconlist }\n",
        "}\n",
    );
    let oracle = {
        let mut analyser = Analyser::new();
        Arc::new(
            analyser
                .analyse(
                    concat!(
                        "oo::class create ::tk::MegawidgetClass {}\n",
                        "oo::class create ::tk::Megawidget {\n",
                        "    superclass oo::class\n",
                        "    self method create {name superclasses body} {\n",
                        "        next $name [list superclass ::tk::MegawidgetClass",
                        " {*}$superclasses]\\;$body\n",
                        "    }\n",
                        "}\n",
                    ),
                    "tcl9.0",
                )
                .class_factories(),
        )
    };
    assert!(
        oracle.contains_key("::tk::Megawidget"),
        "the test oracle must contain the cross-file metaclass",
    );
    backend
        .db_set_source(&uri, consumer, "tcl9.0".to_owned())
        .await
        .unwrap();
    {
        let mut db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        let file = *files.get(&uri).expect("the consumer is a Salsa input");
        file.set_workspace_class_factories(&mut *db)
            .to(Some(Arc::clone(&oracle)));
    }

    let analysis = backend
        .fresh_analysis_for(&uri, Arc::from(consumer), "tcl9.0".to_owned())
        .await
        .unwrap();
    let class = analysis
        .all_classes
        .get("::IconList")
        .unwrap_or_else(|| panic!("the factory-made class must be indexed: {analysis:?}"));
    assert!(class.methods.contains_key("GetSpecs"), "{class:?}");
}
