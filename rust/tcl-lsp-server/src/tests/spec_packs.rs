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

//! Spec-pack reloads, overlay epochs and what a document reads from the packs.

use super::*;

/// A pack's dependency tier is read from the manifest and lockfile beside
/// it, so either moving reloads the packs although no `.tclspec` did. The
/// lockfile is no Tcl source and has a watcher of its own
/// ([`PACKAGE_LOCKFILE_GLOB`]); the manifest is reported by the source
/// watcher and stays an indexed Tcl source as well.
#[test]
fn a_manifest_or_lockfile_change_reloads_the_packs() {
    let changed = |uri: &Uri| {
        partition_watched_file_changes(vec![FileEvent {
            uri: uri.clone(),
            typ: FileChangeType::CHANGED,
        }])
    };
    let lockfile: Uri = "file:///w/tclpkg.lock".parse().unwrap();
    let manifest: Uri = "file:///w/lib/json-1.0.0/tclpkg.tcl".parse().unwrap();
    assert!(is_package_metadata_file(&lockfile));
    assert!(is_package_metadata_file(&manifest));

    let partition = changed(&lockfile);
    assert!(partition.spec_pack_changed);
    assert!(
        partition.last_kind.is_empty(),
        "a lockfile is no Tcl source"
    );
    let partition = changed(&manifest);
    assert!(partition.spec_pack_changed);
    assert!(partition.last_kind.contains_key(&manifest));

    // Negative: other lockfiles and other Tcl files move no pack.
    for other in ["file:///w/other.lock", "file:///w/lib/x.tcl"] {
        let other: Uri = other.parse().unwrap();
        assert!(!is_package_metadata_file(&other), "{other:?}");
        assert!(!changed(&other).spec_pack_changed, "{other:?}");
    }
    assert_eq!(PACKAGE_LOCKFILE_GLOB, "**/tclpkg.lock");
}

/// A unit the database declined to build for want of a workspace's packs
/// is built once the packs are installed and the server syncs the overlay
/// epoch, with the document and its config untouched: the epoch is what
/// tells the database the registry cache changed.
#[tokio::test]
async fn syncing_the_overlay_epoch_lets_an_abstained_unit_build() {
    const OVERLAY: u64 = 0x5E71_0001;
    let db = TrackedMutex::new("db", tcl_lsp_db::TclDatabase::default());
    let (file, config) = {
        let db = db.lock().await;
        let file = tcl_lsp_db::SourceFile::new(
            &*db,
            "set x 1\nputs $x\n".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        let config = tcl_lsp_db::AnalyserConfig::new(
            &*db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            OVERLAY,
            Vec::new(),
            Vec::new(),
        );
        (file, config)
    };
    let has_unit = || async {
        let db = db.lock().await;
        tcl_lsp_db::document_compilation_unit_for(&*db, file, config).is_some()
    };
    assert!(!has_unit().await, "nothing installed the overlay");

    let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("catalogue profile");
    let _installed = tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |_| {});
    sync_overlay_epoch(&db).await;
    assert!(has_unit().await, "installed and synced, the unit builds");
}

/// A wish script resolves to the `tk` environment, whose registry store is
/// the permissive sink's and no catalogue profile's. The overlay the server
/// installs has to reach that key too, or every compile query for a wish
/// document answers with a miss for the life of the session and the
/// analyser reads the plain registry for good: the document keeps its unit,
/// its compiler checks and its rewrites, and a workspace pack's command
/// stops being unknown in it.
#[tokio::test]
async fn a_wish_document_reads_the_workspace_packs_like_any_other() {
    const PACK: &str = "speclib wishpack 1 {\n    command wishcmd {\n        arity 1..\n        \
                            arg 0 -role Value\n        arg 1 -role VarWrite\n    }\n}\n";
    // A bare `wish` shebang names no dialect on its own; the directive (as
    // a `.tk` extension or `tcl-lsp.setDialect` would) selects `tk`.
    const SOURCE: &str = "#!/usr/bin/wish\n# tcl-dialect: tk\nwishcmd {1 2} a\nputs $a\n\
                              set x 1\nset y [expr {$x + 1}]\nputs $y\n\
                              if {$x == 1} {puts one}\n";
    assert_eq!(
        tcl_registry::detect_dialect(SOURCE, None, "tcl9.0"),
        "tk",
        "the document is in the `tk` environment"
    );
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: PathBuf::from("/workspace/.tcl-lsp/wishpack.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        PACK.to_owned(),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);

    let db = TrackedMutex::new("db", tcl_lsp_db::TclDatabase::default());
    let (file, config) = {
        let db = db.lock().await;
        let file = tcl_lsp_db::SourceFile::new(&*db, SOURCE.to_owned(), "tk".to_owned(), None);
        let config = tcl_lsp_db::AnalyserConfig::new(
            &*db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            packs.key,
            Vec::new(),
            Vec::new(),
        );
        (file, config)
    };
    let unknown = || async {
        let db = db.lock().await;
        tcl_lsp_db::file_analysis_incremental(&*db, file, config)
            .diagnostics
            .iter()
            .any(|d| {
                d.code == tcl_compiler::compiler_checks::DiagCode::W123
                    && d.message.contains("wishcmd")
            })
    };
    {
        // tclsh cannot decide this: a SpecTcl pack command does not exist
        // there. The design decision pinned is that an overlay whose packs
        // are not installed is a typed miss and the analysis abstains, so no
        // W123 flickers for a command the pack is about to define.
        let db = db.lock().await;
        let before = tcl_lsp_db::file_analysis_incremental(&*db, file, config);
        assert_eq!(
            before
                .analysis_context_unavailable
                .as_ref()
                .map(|miss| miss.overlay),
            Some(packs.key),
            "control: before the packs install, the analysis is unavailable for the overlay"
        );
        assert!(matches!(before.diagnostics.as_slice(), []));
    }
    let _ = tcl_lsp_db::take_overlay_misses();

    install_pack_overlays(&packs);
    sync_overlay_epoch(&db).await;

    assert!(!unknown().await, "the analyser reads the pack's command");
    {
        let db = db.lock().await;
        assert!(
            tcl_lsp_db::document_compilation_unit_for(&*db, file, config).is_some(),
            "the unit builds under the workspace key"
        );
        let diagnostics = tcl_lsp_db::compiler_check_diagnostics(&*db, file, config);
        assert!(
            !diagnostics.checks.is_empty(),
            "and the compiler checks that read it report"
        );
        assert!(
            !diagnostics.optimisations.is_empty(),
            "and the rewrites that read it are offered"
        );
    }
    let misses: Vec<_> = tcl_lsp_db::take_overlay_misses()
        .into_iter()
        .filter(|miss| miss.overlay == packs.key)
        .collect();
    assert!(misses.is_empty(), "no query missed: {misses:?}");
}
