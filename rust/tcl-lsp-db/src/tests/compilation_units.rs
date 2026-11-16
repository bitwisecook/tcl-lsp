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

//! The shared compilation unit: its document policy key, the body-lowering memo it is assembled from and one build for every consumer.

use super::*;

#[test]
fn original_document_policy_key_admits_actual_input_and_root_bom_geometry() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // The actual memoised source build supplies no Native entry or execution.
    let source = "\u{feff}probe root\n\u{feff}probe later\n";
    for name in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
    ] {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, source.to_owned(), name.to_owned(), None);
        let policy = cfg(&db);
        let input =
            document_analysis_input(&db, file, policy).expect("actual document source input");
        let key = lexer_cfg_key(&db, name);
        assert_eq!(key.to_config(&db), input.lexer_config(), "{name}");
        let unit = compilation_unit_for_config(&db, file, key, policy)
            .expect("complete document policy must join its actual input");
        assert_eq!(
            unit.ir_module.source,
            tcl_lexer::SourceImage::document(source),
            "{name}"
        );
        assert_eq!(
            unit.ir_module.source_metadata_input.as_ref(),
            Some(input.as_ref()),
            "{name}"
        );
        assert_eq!(unit.ir_module.lexer_config, input.lexer_config(), "{name}");
        assert_eq!(unit.ir_module.top_level.statements.len(), 2, "{name}");
        let first = unit.ir_module.top_level.statements[0]
            .tokens()
            .expect("original root command");
        let expected = if input.unit_profile().grammar.script_skips_leading_bom {
            "probe"
        } else {
            "\u{feff}probe"
        };
        assert_eq!(first.argv_texts[0], expected, "{name}");
        assert_eq!(&source[first.argv[0].as_range()], expected, "{name}");
        let later = unit.ir_module.top_level.statements[1]
            .tokens()
            .expect("original later command");
        assert_eq!(later.argv_texts[0], "\u{feff}probe", "{name}");
        assert_eq!(&source[later.argv[0].as_range()], "\u{feff}probe", "{name}");
    }
}

#[test]
fn original_document_policy_key_refuses_foreign_or_changed_full_input() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    let db = TclDatabase::default();
    let file = SourceFile::new(
        &db,
        "\u{feff}probe root".to_owned(),
        "tcl9.1".to_owned(),
        None,
    );
    let policy = cfg(&db);
    let input = document_analysis_input(&db, file, policy).unwrap();
    let key = lexer_cfg_key(&db, "tcl9.1");
    let registry = input.borrowed_context_registry().commands();
    let declared = declared_command_surface(&db, file, registry);
    assert!(
        supplied_unit_build_options(&db, file, key, registry, None, &declared, &input).is_some()
    );
    let foreign_key = lexer_cfg_key(&db, "tcl8.6");
    assert!(compilation_unit_for_config(&db, file, foreign_key, policy).is_none());
    for config in [
        tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Content,
            ..input.lexer_config()
        },
        tcl_lexer::LexerConfig {
            strict_quoting: !input.lexer_config().strict_quoting,
            ..input.lexer_config()
        },
    ] {
        let changed = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        assert!(
            supplied_unit_build_options(&db, file, key, registry, None, &declared, &changed)
                .is_none()
        );
    }
    let mut foreign_registry = registry.project_for_profile(input.unit_profile());
    let mut changed_spec = foreign_registry.get("puts").unwrap().clone();
    if changed_spec
        .traits
        .contains(tcl_registry::Traits::EVALUATES_CODE)
    {
        changed_spec
            .traits
            .remove(tcl_registry::Traits::EVALUATES_CODE);
    } else {
        changed_spec
            .traits
            .insert(tcl_registry::Traits::EVALUATES_CODE);
    }
    foreign_registry.insert(changed_spec);
    assert!(
        supplied_unit_build_options(&db, file, key, &foreign_registry, None, &declared, &input)
            .is_none()
    );
}

/// The per-procedure body-lowering memo
/// (`lower_proc_body`) must skip an unchanged proc's body lowering across a
/// body-only edit. For a context-free file (no `namespace`/`oo::`/nested
/// `proc`), `build_unit_with_keys` lowers each top-level proc's static body
/// through `lower_proc_body`, keyed on the offset-0 body text — so editing one
/// proc's body re-lowers only that body; the others are cache hits. A shift
/// (prepended blank line) re-lowers nothing (offset-invariant key).
#[test]
fn lower_proc_body_reused_on_unrelated_edit() {
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
    let runs = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("lower_proc_body"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let file = SourceFile::new(
        &db,
        "proc a {} { puts 11111 }\n\
             proc b {} { puts 22222 }\n\
             proc c {} { puts 33333 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        runs(&log),
        3,
        "cold build: every top-level proc body lowers once through the memo"
    );

    // Edit only `b`'s body — `a`/`c` bodies are cache hits.
    file.set_text(&mut db).to("proc a {} { puts 11111 }\n\
             proc b {} { puts 99999999 }\n\
             proc c {} { puts 33333 }\n"
        .to_owned());
    let _ = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        runs(&log),
        1,
        "unrelated body edit -> exactly ONE lower_proc_body recomputes"
    );

    // Prepend a blank line: every body shifts but none changes — the offset-0
    // body key is identical, so no body re-lowers.
    file.set_text(&mut db).to("\nproc a {} { puts 11111 }\n\
             proc b {} { puts 99999999 }\n\
             proc c {} { puts 33333 }\n"
        .to_owned());
    let _ = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        runs(&log),
        0,
        "pure offset shift -> no body re-lowers (offset-invariant key)"
    );
}

/// Both diagnostics consumers must **share one `compilation_unit` build per
/// edit** when their lexer configs coincide (every dialect but `tcl8.4` /
/// `f5-irules`): demanding `file_analysis_incremental` then
/// `compiler_check_diagnostics` in the same revision executes
/// `compilation_unit` exactly once.  For `tcl8.4` the configs differ
/// (`expand_syntax`), so each consumer builds its own — executed twice.
#[test]
fn compilation_unit_shared_across_consumers() {
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
    let src = "proc a {x} { return $x }\nproc b {} { a 1 }\n";
    let count_cu = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .iter()
            .filter(|s| s.contains("compilation_unit"))
            .count()
    };

    // tcl8.6: both consumers intern the document's environment id, so
    // they share one build.
    let file86 = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
    let _ = file_analysis_incremental(&db, file86, cfg);
    let _ = compiler_check_diagnostics(&db, file86, cfg);
    assert_eq!(
        count_cu(&log),
        1,
        "tcl8.6: both consumers share exactly one compilation_unit build"
    );

    // `tcl8.4` is the interesting case: its `expand_syntax` differs from
    // `LexerConfig::default()`, so keying on the truncated three-field
    // config (rather than the environment id) would intern two separate
    // entries — one for the analyser tail's default config and one for
    // the checks pass's `for_dialect("tcl8.4")`. Keying on the
    // environment id instead means both consumers lex under the
    // document's own environment, so `tcl8.4` shares one build like
    // every other environment.
    let file84 = SourceFile::new(&db, src.to_owned(), "tcl8.4".to_owned(), None);
    let _ = file_analysis_incremental(&db, file84, cfg);
    let _ = compiler_check_diagnostics(&db, file84, cfg);
    assert_eq!(
        count_cu(&log),
        1,
        "tcl8.4: one environment id -> one shared build, as for every dialect"
    );

    // A fresh edit re-shares for tcl8.6 (one build for the new revision).
    file86
        .set_text(&mut db)
        .to("proc a {x} { return $x }\nproc b {} { a 2 }\n".to_owned());
    let _ = file_analysis_incremental(&db, file86, cfg);
    let _ = compiler_check_diagnostics(&db, file86, cfg);
    assert_eq!(
        count_cu(&log),
        1,
        "after an edit, tcl8.6 again shares one build across both consumers"
    );
}
