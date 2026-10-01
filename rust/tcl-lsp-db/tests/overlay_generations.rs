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

//! A workspace pack overlay nothing has installed is a miss the queries
//! answer for themselves, never the plain registry under the packs' name.
//!
//! The overlay is the key the packs' owner installed a registry generation
//! under, and only that owner can build one, so the database can look one up
//! and never make one. The queries that build a unit — and offer rewrites
//! from it — have no answer without the packs' declarations: they abstain,
//! and what they answered reads the overlay epoch, so an overlay installed a
//! moment later is picked up when the host moves the epoch instead of
//! standing for the life of the memo. A generation the process-wide cache has
//! since retired still serves the queries that resolved it, because a
//! per-procedure query that looked it up again would disagree with the unit
//! that keyed it.
//!
//! Every test here touches the process-wide overlay caches, and the
//! retirement test fills them, so they take one lock and run one at a time.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tcl_dialect::DialectProfile;
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, TclDb, compiler_check_diagnostics,
    document_compilation_unit_for, semantic_tokens, set_overlay_epoch, take_overlay_misses,
};
use tcl_registry::model::{KeyedVersions, OverlayMiss, resolve_environment};
use tcl_registry::registry_for_profile_with_overlay;

/// A document whose checks and optimiser have something to say under any
/// registry: a constant chain the optimiser folds.
const SRC: &str = "set x 1\nset y [expr {$x + 1}]\nputs $y\n";

static OVERLAYS: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    OVERLAYS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn profile() -> &'static DialectProfile {
    DialectProfile::find("tcl9.0").expect("catalogue profile")
}

/// A config whose only setting is the workspace's pack overlay.
fn overlay_config(db: &TclDatabase, overlay: u64) -> AnalyserConfig {
    AnalyserConfig::new(
        db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        overlay,
        Vec::new(),
        Vec::new(),
    )
}

/// Install the generation `overlay` names for `tcl9.0`, as the packs' owner
/// does. The pack contents are not the point here, only that a generation is
/// there to be found.
fn install(overlay: u64) -> Arc<tcl_registry::CommandRegistry> {
    registry_for_profile_with_overlay(profile(), overlay, |registry| {
        registry.insert_ambient_package("overlay-generations-test", "1.0");
    })
}

/// The misses recorded for `overlay` since the last look, whoever asked.
fn misses_for(overlay: u64) -> Vec<OverlayMiss> {
    take_overlay_misses()
        .into_iter()
        .filter(|miss| miss.overlay == overlay)
        .collect()
}

/// With no packs installed there is no unit and there are no findings or
/// rewrites — not the ones the plain registry would have given — and the miss
/// is recorded once for the host to report, however many queries hit it.
#[test]
fn an_uninstalled_overlay_yields_no_unit_and_no_findings() {
    const MISSING: u64 = 0x0DB0_0001;
    let _serial = serial();
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl9.0".to_owned(), None);

    // Control: with no overlay the same document has a unit and rewrites, so
    // the empty answer below is the abstention and not a quiet document.
    let plain = overlay_config(&db, 0);
    assert!(document_compilation_unit_for(&db, file, plain).is_some());
    assert!(
        !compiler_check_diagnostics(&db, file, plain)
            .optimisations
            .is_empty(),
        "the control document has a rewrite to offer"
    );

    let config = overlay_config(&db, MISSING);
    assert!(
        document_compilation_unit_for(&db, file, config).is_none(),
        "no unit is built without the packs"
    );
    let diagnostics = compiler_check_diagnostics(&db, file, config);
    assert!(
        diagnostics.checks.is_empty() && diagnostics.optimisations.is_empty(),
        "and nothing is offered in its place"
    );

    assert_eq!(
        misses_for(MISSING),
        vec![OverlayMiss {
            environment: "tcl9.0".to_owned(),
            overlay: MISSING,
        }],
        "one record for the two queries that missed"
    );
    let _ = document_compilation_unit_for(&db, file, config);
    assert!(
        misses_for(MISSING).is_empty(),
        "a miss already recorded is not recorded again"
    );
}

/// What the queries answered without the packs reads the overlay epoch: the
/// host installs the generation, moves the epoch, and the unit — and the
/// checks and rewrites that read it — are answered again, with the document
/// untouched. Until the epoch moves the abstention stands, which is the
/// contract: a database cannot see the cache change, only its inputs.
#[test]
fn an_abstention_runs_again_once_the_overlay_is_installed() {
    const LATER: u64 = 0x0DB0_0002;
    let _serial = serial();
    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl9.0".to_owned(), None);
    let config = overlay_config(&db, LATER);
    assert!(document_compilation_unit_for(&db, file, config).is_none());
    assert!(
        compiler_check_diagnostics(&db, file, config)
            .optimisations
            .is_empty()
    );

    let _installed = install(LATER);
    assert!(
        document_compilation_unit_for(&db, file, config).is_none(),
        "installed, but the database has not been told: nothing it reads moved"
    );

    assert!(
        set_overlay_epoch(&mut db, tcl_registry::overlay_epoch()),
        "installing an overlay moved the epoch"
    );
    assert!(
        document_compilation_unit_for(&db, file, config).is_some(),
        "told, the query is asked again and builds the unit"
    );
    assert!(
        !compiler_check_diagnostics(&db, file, config)
            .optimisations
            .is_empty(),
        "and the checks and rewrites that read it are asked again too"
    );
    assert!(
        !set_overlay_epoch(&mut db, tcl_registry::overlay_epoch()),
        "a sync that finds nothing moved writes nothing"
    );
}

/// The highlighting queries only advise, and a pack reload has the client
/// ask again, so they read the plain registry for a miss: the tokens are the
/// plain ones, not an empty set.
#[test]
fn the_token_query_reads_the_plain_registry_for_a_miss() {
    const MISSING: u64 = 0x0DB0_0003;
    let _serial = serial();
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl9.0".to_owned(), None);
    let plain = semantic_tokens(&db, file, overlay_config(&db, 0));
    assert!(!plain.data.is_empty());
    assert_eq!(
        semantic_tokens(&db, file, overlay_config(&db, MISSING)).data,
        plain.data,
        "the plain registry's tokens"
    );
    assert_eq!(misses_for(MISSING).len(), 1, "and the miss is on record");
}

/// A generation the process-wide cache has retired still serves the queries
/// that resolved it: a per-procedure query that looked it up again would find
/// it gone and disagree with the unit that keyed it. A key nothing resolved
/// stays a miss once retired.
#[test]
fn a_retired_generation_still_serves_the_queries_that_resolved_it() {
    const RESOLVED: u64 = 0x0DB0_0004;
    const NEVER_RESOLVED: u64 = 0x0DB0_0005;
    const FILL: u64 = 0x0DB1_0000;
    let _serial = serial();
    let db = TclDatabase::default();
    let environment = resolve_environment("tcl9.0");
    let keyed = KeyedVersions::default();

    let installed = install(RESOLVED);
    let _also = install(NEVER_RESOLVED);
    let first = db
        .registry_with_overlay("tcl9.0", RESOLVED)
        .expect("installed");
    assert!(Arc::ptr_eq(&first, &installed));

    // Retire both the way a long pack-editing session does: distinct keys,
    // until the caches let go.
    let mut built = 0;
    while environment.context_registry(&keyed, RESOLVED).is_ok()
        || environment.context_registry(&keyed, NEVER_RESOLVED).is_ok()
    {
        built += 1;
        assert!(built < 400, "the caches never retired the overlays");
        let _ = registry_for_profile_with_overlay(profile(), FILL + built, |_| {});
        let _ = environment.context_registry(&keyed, FILL + built);
    }

    let again = db
        .registry_with_overlay("tcl9.0", RESOLVED)
        .expect("held, though the cache has let go");
    assert!(
        Arc::ptr_eq(&again, &first),
        "the same generation the unit started with"
    );
    assert_eq!(
        db.registry_with_overlay("tcl9.0", NEVER_RESOLVED)
            .expect_err("nothing resolved it before it was retired")
            .overlay,
        NEVER_RESOLVED
    );
}
