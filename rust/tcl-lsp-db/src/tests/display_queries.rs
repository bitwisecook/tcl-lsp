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

//! The outline, folding ranges and semantic tokens against their direct computations, and the project token indexes.

use super::*;

fn original_db_token_is_variable(
    tokens: &SemanticTokens,
    image: &str,
    needle: &str,
    variable: u32,
) -> bool {
    let offset = u32::try_from(image.rfind(needle).unwrap()).unwrap();
    let target = tcl_lexer::LineIndex::new(image).position_at_utf16(offset, image);
    let mut line = 0;
    let mut column = 0;
    tokens.data.as_chunks::<5>().0.iter().any(|entry| {
        if entry[0] == 0 {
            column += entry[1];
        } else {
            line += entry[0];
            column = entry[1];
        }
        line == target.line
            && column <= target.character.get()
            && target.character.get() < column + entry[2]
            && entry[3] == variable
    })
}

#[test]
fn original_db_token_roles_keep_opaque_declarations_and_full_caller_currency() {
    // Implementation contract: naming.database.original-token-role-consumer
    // docs/design/analysis/name-resolution-proofs/database-original-token-role-consumer.md
    use tcl_lsp_core::semantic_tokens::{
        WorkspaceTokenFacts, full_with_cu_and_facts, legend_token_types,
    };
    let source = r"proc p\uD800 {target} {upvar 1 $target cell; set cell 1}
proc p\uD801 {target} {return $target}
p\uD800 written
p\uD801 ordinary";
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, source.to_owned(), "tcl8.6".to_owned(), None);
    let project = Project::new(&db, vec![file]);
    let facts = file_token_facts(&db, file);
    let roles = project_proc_var_index(&db, project);
    assert!(
        !facts.proc_roles.is_empty(),
        "the retained writer declaration has a role"
    );
    assert_eq!(facts.proc_roles, *roles);
    let mut analysis = (*file_analysis_incremental(&db, file, cfg(&db))).clone();
    analysis.all_procs.clear();
    analysis.superseded_procs.clear();
    for invocation in &mut analysis.command_invocations {
        invocation.name = "REPORT-COLLISION".to_owned();
        invocation.resolution_candidates = vec!["REPORT-COLLISION".to_owned()];
    }
    let registry = db.registry(file.dialect(&db));
    let profile = tcl_lsp_core::profile_for_dialect(file.dialect(&db));
    let unit = document_compilation_unit(&db, file);
    let render = |image: &str, current: &AnalysisResult| {
        full_with_cu_and_facts(
            image,
            profile,
            registry,
            Some(&unit),
            WorkspaceTokenFacts {
                proc_roles: Some(&roles),
                analysis: Some(current),
                ..Default::default()
            },
        )
    };
    let variable = u32::try_from(
        legend_token_types()
            .iter()
            .position(|kind| *kind == "variable")
            .unwrap(),
    )
    .unwrap();
    let is_variable = |tokens: &SemanticTokens, image: &str, needle: &str| {
        original_db_token_is_variable(tokens, image, needle, variable)
    };
    let tokens = render(source, &analysis);
    assert!(
        is_variable(&tokens, source, "written"),
        "the exact opaque writer retains its caller argument role after reporting clear"
    );
    assert!(
        !is_variable(&tokens, source, "ordinary"),
        "the distinct opaque ordinary procedure does not borrow the writer role"
    );
    let changed = format!("{source} ");
    assert!(!is_variable(
        &render(&changed, &analysis),
        &changed,
        "written"
    ));
    let mut changed_config = analysis.clone();
    changed_config
        .body_lexer_config
        .as_mut()
        .unwrap()
        .strict_quoting ^= true;
    assert!(!is_variable(
        &render(source, &changed_config),
        source,
        "written"
    ));
    let mut missing_input = analysis;
    let call = u32::try_from(source.rfind(r"p\uD800 written").unwrap()).unwrap();
    missing_input
        .command_invocations
        .iter_mut()
        .find(|invocation| invocation.range.start() == call)
        .unwrap()
        .original_name_input = None;
    assert!(!is_variable(
        &render(source, &missing_input),
        source,
        "written"
    ));
}

#[test]
fn document_symbols_match_direct() {
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
    let got = document_symbols(&db, file, cfg(&db));
    let expected = tcl_lsp_core::document_symbols::document_symbols(
        SRC,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
    );
    assert_eq!(got, expected);
}

#[test]
fn semantic_tokens_match_direct() {
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
    let got = semantic_tokens(&db, file, cfg(&db));
    let reg = db.registry("tcl");
    let expected = tcl_lsp_core::semantic_tokens::full(
        SRC,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        reg,
    );
    assert_eq!(got, expected);
    assert_ne!(got.data.len(), 0);
}

/// TP: the enriched `semantic_tokens` query (backed by a
/// [`CompilationUnit`] and SSA/SCCP-derived constant-string facts) retags
/// the `.*abc` literal at its originating `set` as regex source, because
/// `regexp $my_re` provably reads that constant — see
/// [`tcl_compiler::regex_source`]. The cheap coarse tier
/// (`tcl_lsp_core::semantic_tokens::full`, no `CompilationUnit`) has no
/// SSA facts to do this with, so the two streams differ — proving the
/// enrichment `semantic_tokens` performs over the coarse walk is real,
/// not a no-op: the fast-path fallback in
/// `Backend::semantic_tokens_core_data` trades this enrichment away,
/// so it must exist for the trade to mean anything.
#[test]
fn semantic_tokens_retags_constant_regex_source_true_positive() {
    let src = "set my_re \".*abc\"\nregexp $my_re $s\n";
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
    let enriched = semantic_tokens(&db, file, cfg(&db));
    let reg = db.registry("tcl9.0");
    let coarse = tcl_lsp_core::semantic_tokens::full(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        reg,
    );
    assert_ne!(
        enriched, coarse,
        "the CompilationUnit-informed regex-source retag must change the \
             token stream relative to the coarse (no-CU) tier"
    );
}

/// TN: a `regexp` pattern read from a variable that is *not* provably
/// constant (reassigned from a proc parameter) gets no retag from either
/// tier — the coarse and enriched streams agree, because there is no
/// constant fact for the enriched tier to add. Guards against the retag
/// firing indiscriminately on every `regexp $var` call.
#[test]
fn semantic_tokens_skips_retag_for_non_constant_pattern_true_negative() {
    let src = "proc match {re s} {\n    regexp $re $s\n}\n";
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
    let enriched = semantic_tokens(&db, file, cfg(&db));
    let reg = db.registry("tcl9.0");
    let coarse = tcl_lsp_core::semantic_tokens::full(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        reg,
    );
    assert_eq!(
        enriched, coarse,
        "a non-constant pattern source must not be retagged by either tier"
    );
}

/// `semantic_tokens` must depend on
/// the incremental, per-item-memoised `file_analysis_incremental` — not
/// the coarse `file_analysis` — so a token request that lands after the
/// diagnostics worker has already analysed this revision is a cache hit
/// (no second whole-file walk), and a token request that lands *first*
/// still primes the exact query diagnostics will reuse. Also asserts the
/// coarse, uncancellable `file_analysis` is never invoked by either order.
#[test]
fn semantic_tokens_shares_incremental_analysis_with_diagnostics() {
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
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl8.6".to_owned(), None);

    // Diagnostics-first order: the worker analyses, then a token request
    // arrives for the same revision.
    let _ = file_analysis_incremental(&db, file, cfg);
    let after_diagnostics = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        after_diagnostics
            .iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        1,
        "diagnostics analyses the one proc body: {after_diagnostics:?}"
    );

    let _ = semantic_tokens(&db, file, cfg);
    let after_tokens = std::mem::take(&mut *log.lock().unwrap());
    assert!(
        after_tokens
            .iter()
            .all(|s| !s.contains("item_body_analysis")),
        "a token request for the same revision must be a cache hit against \
             the diagnostics worker's analysis, not a second walk: {after_tokens:?}"
    );
    assert!(
        after_tokens.iter().all(|s| !s.contains("file_analysis(")),
        "semantic_tokens must never invoke the coarse, uncancellable \
             file_analysis query: {after_tokens:?}"
    );

    // Token-first order (a cold open before the debounced diagnostics
    // worker has run): editing then re-requesting tokens still only
    // recomputes the one changed body, proving the per-item firewall
    // covers the token path too, not just the diagnostics path.
    file.set_text(&mut db)
        .to("proc greet {name} {\n    puts \"hi $name!!\"\n}\n# c\nset x 1\n".to_owned());
    let _ = std::mem::take(&mut *log.lock().unwrap());
    let _ = semantic_tokens(&db, file, cfg);
    let after_edit = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        after_edit
            .iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        1,
        "a single-body edit must recompute exactly one item via the token \
             path: {after_edit:?}"
    );
}

/// Build a database whose executed-query keys land in the returned log.
fn logging_db() -> (TclDatabase, Arc<Mutex<Vec<String>>>) {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    (db, log)
}

/// [`project_class_index`] / [`project_proc_var_index`] read
/// *every* file in the project, so the tier they read decides what an
/// interactive `semanticTokens` request costs on a large workspace. They
/// must stay on the light [`file_token_facts`] tier and never reach the deep
/// one — reading `file_analysis_incremental` there would mean a
/// whole-workspace deep analysis (CFG/SSA units, per-body lattices,
/// diagnostic emitters) behind a token request for one file, which on an
/// 883-file checkout would never complete before the next `set_text`
/// cancelled it.
#[test]
fn project_indexes_never_touch_the_deep_tier() {
    let (db, log) = logging_db();
    let lib = SourceFile::new(
        &db,
        "oo::configurable create ::Pin { property node }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let main = SourceFile::new(
        &db,
        "set p [::Pin new]\n$p configure -node n1\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![lib, main]);

    let _ = project_class_index(&db, project);
    let _ = project_proc_var_index(&db, project);
    let log_snapshot = log.lock().unwrap().clone();
    assert!(
        log_snapshot.iter().any(|s| s.contains("file_token_facts")),
        "project indexes must read the light per-file tier: {log_snapshot:?}"
    );
    assert!(
        log_snapshot.iter().all(|s| !s.contains("file_analysis")),
        "project indexes must never analyse a project file at the deep tier \
             (#1163): {log_snapshot:?}"
    );
}

/// The per-file [`file_token_facts`] firewall: a body edit that changes no
/// class and no parameter role backdates, so neither project index
/// re-executes — the property that keeps the cross-file token aggregates off
/// the per-keystroke path once they are warm.
#[test]
fn project_indexes_backdate_on_a_role_neutral_body_edit() {
    use salsa::Setter as _;
    let (mut db, log) = logging_db();
    let lib = SourceFile::new(
        &db,
        "oo::configurable create ::Pin { property node }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let main = SourceFile::new(
        &db,
        "proc ::helper {a b} { return [expr {$a + $b}] }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![lib, main]);
    let _ = project_class_index(&db, project);
    let _ = project_proc_var_index(&db, project);

    log.lock().unwrap().clear();
    main.set_text(&mut db)
        .to("proc ::helper {a b} { return [expr {$a * $b}] }\n".to_owned());
    let _ = project_class_index(&db, project);
    let _ = project_proc_var_index(&db, project);
    let after = log.lock().unwrap().clone();
    assert!(
        after.iter().any(|s| s.contains("file_token_facts")),
        "the edited file's own facts must be recomputed: {after:?}"
    );
    assert!(
        after
            .iter()
            .all(|s| !s.contains("project_class_index") && !s.contains("project_proc_var_index")),
        "a role-neutral body edit must backdate at the per-file firewall, \
             leaving the project aggregates memoised: {after:?}"
    );
}

/// The merged project role index must equal one built from every file's
/// procs at once — including the abstentions each file made on its own, so
/// a name one file already dropped as ambiguous is not re-adopted from
/// another file's unambiguous entry.
#[test]
fn merged_role_index_equals_the_all_at_once_build() {
    let sources = [
        // `::a::grow` and `::b::grow` disagree, so this file abstains on the
        // bare `grow` key all by itself.
        "namespace eval a { proc grow {v} { upvar 1 $v x; set x 1 } }\n\
             namespace eval b { proc grow {n v} { upvar 1 $v x; set x 1 } }\n",
        // A third `grow`, unambiguous within its own file.
        "namespace eval c { proc grow {v} { upvar 1 $v x; set x 2 } }\n",
    ];
    let db = TclDatabase::default();
    let files: Vec<SourceFile> = sources
        .iter()
        .map(|s| SourceFile::new(&db, (*s).to_owned(), "tcl8.6".to_owned(), None))
        .collect();
    let project = Project::new(&db, files.clone());

    let all_procs: Vec<Arc<AnalysisResult>> = sources
        .iter()
        .map(|s| Arc::new(Analyser::new().structure_only().analyse(s, "tcl8.6")))
        .collect();
    let expected = VarNameArgRoles::from_procs(all_procs.iter().flat_map(|a| a.all_procs.values()));
    assert_eq!(*project_proc_var_index(&db, project), expected);
    // Order-independent: reversing the project's files changes nothing.
    let reversed = Project::new(&db, files.into_iter().rev().collect());
    assert_eq!(*project_proc_var_index(&db, reversed), expected);
}

#[test]
fn cross_file_object_dispatch_resolves_via_project_index() {
    // A class defined in one file, dispatched on via a direct constructor in
    // another: `semantic_tokens_project` resolves the method through the
    // workspace-merged `project_class_index`, so its output differs from the
    // local-only `semantic_tokens` (which leaves the method a plain string).
    let db = TclDatabase::default();
    let lib = SourceFile::new(
        &db,
        "oo::configurable create ::Pin { property node }\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let user = SourceFile::new(
        &db,
        "[::Pin new] configure -node 5\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![lib, user]);
    let cross = semantic_tokens_project(&db, user, project, cfg(&db));
    let local = semantic_tokens(&db, user, cfg(&db));
    assert_ne!(
        cross.data, local.data,
        "cross-file index must resolve the method the local pass leaves unresolved"
    );
    // The merged index sees the class from the other file.
    assert!(
        project_class_index(&db, project)
            .classes
            .contains_key("::Pin"),
        "project index should contain ::Pin from the library file"
    );
}

#[test]
fn project_index_abstains_on_duplicate_class_name() {
    // The same qualified class name defined in two files is genuinely
    // ambiguous — which `::Pin` does a cross-file dispatch mean?  The merged
    // index drops it (sound-by-abstention), deterministically regardless of
    // file order, rather than letting an order-dependent "winner" leak into
    // resolution.
    let db = TclDatabase::default();
    let a = SourceFile::new(
        &db,
        "oo::class create ::Pin { method a {} {} }\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let b = SourceFile::new(
        &db,
        "oo::class create ::Pin { method b {} {} }\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    // Both orderings must agree (and both must drop the ambiguous class).
    for files in [vec![a, b], vec![b, a]] {
        let project = Project::new(&db, files);
        assert!(
            !project_class_index(&db, project)
                .classes
                .contains_key("::Pin"),
            "an ambiguous cross-file class name must be dropped, not resolved"
        );
    }
}

#[test]
fn folding_matches_direct() {
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
    let got = folding_ranges(&db, file, cfg(&db));
    let reg = db.registry("tcl");
    let expected = tcl_lsp_core::folding::folding_ranges(
        SRC,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        reg,
    );
    assert_eq!(got, expected);
}

/// Folding served through salsa is memoised: a repeat request for the
/// same revision must not re-execute the tracked query — the property
/// `Backend::db_folding_ranges` relies on to avoid a fresh
/// segmenter/registry walk on every `textDocument/foldingRange`.
#[test]
fn folding_ranges_memoised_across_repeat_queries() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let db = {
        let sink = Arc::clone(&log);
        TclDatabase::with_event_logger(move |key| sink.lock().unwrap().push(key))
    };
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
    // `AnalyserConfig` is an input: one handle, so the repeat request
    // names the same query key.
    let config = cfg(&db);
    let drain = || std::mem::take(&mut *log.lock().unwrap());
    let executions = |events: &[String]| {
        events
            .iter()
            .filter(|key| key.contains("folding_ranges"))
            .count()
    };

    let _ = folding_ranges(&db, file, config);
    assert_eq!(
        executions(&drain()),
        1,
        "cold: one execution for the first request"
    );
    let _ = folding_ranges(&db, file, config);
    assert_eq!(
        executions(&drain()),
        0,
        "warm: memoised, no re-execution for a repeat request at the same revision"
    );
}
