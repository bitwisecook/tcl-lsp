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

//! Whole-file analysis: the direct and incremental paths agree, configuration reaches both and a body edit recomputes one item.

use super::*;

#[test]
fn body_cache_gate_is_per_body_and_whitespace_aware() {
    use tcl_compiler::lowering::body_cache_eligible;
    // A plain, context-free body is eligible.
    assert!(body_cache_eligible(" set x 1 "));
    assert!(body_cache_eligible(" puts hi "));
    // A body carrying a cross-item construct is not — including the tab-
    // separated forms: the isolated lowerer drops the effect.
    assert!(!body_cache_eligible(" interp\talias {} x {} y "));
    assert!(!body_cache_eligible(" namespace\timport ::ns::* "));
    assert!(!body_cache_eligible(" rename set myset "));
    assert!(!body_cache_eligible(" oo::class create C "));
    // A nested `proc` disqualifies the enclosing body.
    assert!(!body_cache_eligible(" proc inner {} {} "));
    // The gate is per-body: a context-carrying sibling does not disable the
    // clean body — the clean body stays eligible on its own.
    assert!(body_cache_eligible(" set y 2 "));
}

#[test]
fn file_analysis_matches_direct_analyse() {
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
    let got = file_analysis(&db, file, cfg(&db));

    let mut direct = Analyser::new();
    let expected = direct.analyse(SRC, "tcl");
    assert_eq!(*got, expected);
    assert!(got.all_procs.contains_key("::greet"));
}

/// An unchanged proc body must re-intern to the *same*
/// `ItemBodyKey` (so the memo hits) and must not cost a fresh copy of the
/// body text to get there — the key shares the analyser's `Arc<str>`.
#[test]
fn unchanged_body_reinterns_to_the_same_key_without_copying_its_text() {
    let db = TclDatabase::default();
    let body: Arc<str> = Arc::from("set s 1\nreturn $s\n");
    let make = || {
        ItemBodyKey::new(
            &db,
            Arc::clone(&body),
            "::".to_owned(),
            "alpha".to_owned(),
            Vec::new(),
            false,
            false,
            Vec::new(),
            None,
            ((Vec::new(), Vec::new()), None, None, None, None),
            "tcl8.6".to_owned(),
            Vec::new(),
            NonAsciiMode::Default,
        )
    };
    assert!(make() == make(), "unchanged text must re-intern to one key");
    assert!(
        Arc::ptr_eq(make().body_text(&db), &body),
        "the interned key must share the caller's body text, not copy it",
    );
    // A different body must not collide with it.
    let other: Arc<str> = Arc::from("set s 2\nreturn $s\n");
    let other_key = ItemBodyKey::new(
        &db,
        other,
        "::".to_owned(),
        "alpha".to_owned(),
        Vec::new(),
        false,
        false,
        Vec::new(),
        None,
        ((Vec::new(), Vec::new()), None, None, None, None),
        "tcl8.6".to_owned(),
        Vec::new(),
        NonAsciiMode::Default,
    );
    assert!(
        make() != other_key,
        "distinct bodies must intern distinctly"
    );
}

#[test]
fn editing_text_recomputes() {
    use salsa::Setter as _;
    let mut db = TclDatabase::default();
    let config = cfg(&db);
    let file = SourceFile::new(&db, "proc a {} {}\n".to_owned(), "tcl".to_owned(), None);
    assert!(
        file_analysis(&db, file, config)
            .all_procs
            .contains_key("::a")
    );

    file.set_text(&mut db).to("proc b {} {}\n".to_owned());
    let after = file_analysis(&db, file, config);
    assert!(after.all_procs.contains_key("::b"));
    assert!(!after.all_procs.contains_key("::a"));
}

#[test]
fn file_analysis_incremental_matches_full() {
    let db = TclDatabase::default();
    let cfg = cfg(&db);
    for src in [
        SRC,
        "proc a {x} { return $x }\nproc b {} { a 1 }\n",
        "namespace eval n { proc f {y} { set z $y } }\nset g 1\nputs $g\n",
        "oo::class create K {\n  method m {a} { set n $a }\n}\nproc p {} { set q 1 }\n",
    ] {
        let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        let inc = file_analysis_incremental(&db, file, cfg);
        let full = file_analysis(&db, file, cfg);
        assert_eq!(*inc, *full, "incremental != full for:\n{src}");
    }
}

#[test]
fn sidecar_stubs_reach_incremental_proc_bodies_and_invalidate() {
    use salsa::Setter as _;

    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-sidecar-incremental-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let src_dir = root.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    let sidecar = root.join("tcl8.6.tcl.stubs");
    std::fs::write(&sidecar, "stub sidecar_cmd {value}\n").unwrap();
    let path = src_dir.join("main.tcl");
    let mut db = TclDatabase::default();
    let config = cfg(&db);
    let file = SourceFile::new(
        &db,
        "proc f {} { sidecar_cmd 1 }\n".to_owned(),
        "tcl8.6".to_owned(),
        Some(path.display().to_string()),
    );

    let initial = file_analysis_incremental(&db, file, config);
    assert!(
        !initial.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "sidecar command must resolve inside a proc body: {:?}",
        initial.diagnostics
    );
    assert_eq!(*initial, *file_analysis(&db, file, config));

    std::fs::write(&sidecar, "stub replacement_cmd {value}\n").unwrap();
    file.set_sidecar_stubs_epoch(&mut db).to(1);
    let changed = file_analysis_incremental(&db, file, config);
    assert!(
        changed.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "the unchanged source must be reanalysed after its sidecar changes"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn file_analysis_incremental_carries_bigip_version_override() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        Some("21.1.0".to_owned()),
        0,
        Vec::new(),
        Vec::new(),
    );
    let file = SourceFile::new(
        &db,
        "SSL::c3d cert_lifespan 24\n".to_owned(),
        "f5-irules".to_owned(),
        None,
    );

    let incremental = file_analysis_incremental(&db, file, cfg);
    let full = file_analysis(&db, file, cfg);

    assert_eq!(*incremental, *full);
    assert_eq!(
        incremental.library_versions.bigip_version.as_deref(),
        Some("21.1.0")
    );
}

/// `tclLsp.targets` (§5.4 range targeting) flows through the config
/// into both analysis queries: a declared `tcl 8.5-9.0` range flags
/// the 8.6-introduced `lmap` at the declared 8.5 target, and the
/// undeclared default stays byte-identically silent.
#[test]
fn file_analysis_carries_declared_targets() {
    let db = TclDatabase::default();
    let ranged = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        vec![("tcl".to_owned(), "8.5-9.0".to_owned())],
        Vec::new(),
    );
    let bare = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let file = SourceFile::new(
        &db,
        "lmap x {1 2} {set x}\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );

    let incremental = file_analysis_incremental(&db, file, ranged);
    let full = file_analysis(&db, file, ranged);
    assert_eq!(*incremental, *full);
    assert!(
        incremental
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "W150" && d.message.contains("8.5")),
        "the declared range flags lmap at the 8.5 target: {:?}",
        incremental.diagnostics
    );
    let undeclared = file_analysis_incremental(&db, file, bare);
    assert!(
        !undeclared
            .diagnostics
            .iter()
            .any(|d| matches!(d.code.as_str(), "W150" | "W151")),
        "no declaration, no range diagnostics"
    );
}

/// A body-only edit (same length, so other bodies keep their offset)
/// recomputes exactly one `item_body_analysis`.
#[test]
fn body_edit_recomputes_one_item() {
    use salsa::Setter as _;
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let file = SourceFile::new(
        &db,
        "proc a {} { set x 11111 }\nproc b {} { set y 22222 }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = file_analysis_incremental(&db, file, cfg);
    let init = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        init.iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        2,
        "initial: both bodies analysed: {init:?}"
    );

    // Edit proc a's body, *changing its length* — this shifts proc b's
    // byte offset.  Offset-invariance means b's key (its body text) is
    // unchanged, so it stays a cache hit and only a recomputes.
    file.set_text(&mut db)
        .to("proc a {} { set x 9999999999 }\nproc b {} { set y 22222 }\n".to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    let after = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        after
            .iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        1,
        "length-changing body edit -> exactly ONE item recomputes (offset-invariant): {after:?}"
    );
}

/// The method firewall: a body edit to one OO method recomputes exactly one
/// `item_body_analysis` — methods are isolated + memoised like procs, so an
/// unedited sibling method (shifted by the edit) stays a cache hit.
#[test]
fn method_body_edit_recomputes_one_item() {
    use salsa::Setter as _;
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let file = SourceFile::new(
        &db,
        "oo::class create K {\n  method a {} { set x 11111 }\n  method b {} { set y 22222 }\n}\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = file_analysis_incremental(&db, file, cfg);
    let init = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        init.iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        2,
        "initial: both method bodies analysed: {init:?}"
    );

    // Edit method a's body length — shifts method b; b's offset-0 body is
    // unchanged, so its key is a cache hit and only a recomputes.
    file.set_text(&mut db).to(
            "oo::class create K {\n  method a {} { set x 9999999999 }\n  method b {} { set y 22222 }\n}\n"
                .to_owned(),
        );
    let _ = file_analysis_incremental(&db, file, cfg);
    let after = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        after
            .iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        1,
        "method body edit -> exactly ONE item recomputes: {after:?}"
    );
}
