// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Current source projection parity and explicit compatibility admission.

use super::*;
use salsa::Setter as _;
use tcl_compiler::signature_scan::types::SignatureCommandInvocation;

fn configuration(db: &TclDatabase) -> AnalyserConfig {
    AnalyserConfig::new(
        db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    )
}

fn callback_codes(diagnostics: &[tcl_compiler::analyser::Diagnostic]) -> Vec<DiagCode> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.callback_source_arity().is_some())
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn supplied_project_callback_projection_matches_tracked_headers_and_source_barriers() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // This compares conditional source diagnostics, not callback entry.
    for (source, expected) in [
        ("lsort -command cb {2 1}", Some(DiagCode::E003)),
        (
            "interp alias {} cb {} external FIXED\nlsort -command cb {2 1}",
            Some(DiagCode::E002),
        ),
        (
            "proc cb {a} {}\nrename cb {}\nlsort -command cb {2 1}",
            None,
        ),
        (
            "proc cb {a} {}\ninterp alias {} cb {} puts\nlsort -command cb {2 1}",
            None,
        ),
    ] {
        let db = TclDatabase::default();
        let config = configuration(&db);
        let library = SourceFile::new(
            &db,
            "proc cb {a} {}\nproc external {a b c d} {}".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        let caller = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
        let project = Project::new(&db, vec![library, caller]);
        let analysis = file_analysis_incremental(&db, caller, config);
        let supplied =
            project_callback_diagnostics_for_analysis(&db, project, source, &analysis, |_| false);
        assert_eq!(
            project_callback_diagnostics(&db, caller, config, project).as_ref(),
            &supplied,
            "{source}"
        );
        assert_eq!(
            callback_codes(&supplied),
            expected.into_iter().collect::<Vec<_>>()
        );
        assert!(
            callback_codes(&project_callback_diagnostics_for_analysis(
                &db,
                project,
                source,
                &analysis,
                |code| matches!(code, "E002" | "E003" | "E005"),
            ))
            .is_empty()
        );
    }
}

#[test]
fn supplied_project_callback_projection_reads_current_exact_project_headers() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    let mut db = TclDatabase::default();
    let config = configuration(&db);
    let library = SourceFile::new(
        &db,
        "namespace eval a {proc cb {one} {return FIRST}}\nnamespace eval b {proc cb {one two} {}}"
            .to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let source = "lsort -command ::a::cb {2 1}";
    let caller = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::new(&db, vec![library, caller]);
    let analysis = file_analysis_incremental(&db, caller, config);
    let first =
        project_callback_diagnostics_for_analysis(&db, project, source, &analysis, |_| false);
    assert_eq!(callback_codes(&first), vec![DiagCode::E003]);
    library.set_text(&mut db).to(
        "namespace eval a {proc cb {one} {return LATER}}\nnamespace eval b {proc cb {one two} {}}"
            .to_owned(),
    );
    assert_eq!(
        project_callback_diagnostics_for_analysis(&db, project, source, &analysis, |_| false),
        first,
        "a body edit preserves the body-free source header"
    );
    library.set_text(&mut db).to(
        "namespace eval a {proc cb {one two} {return LATER}}\nnamespace eval b {proc cb {one} {}}"
            .to_owned(),
    );
    let updated =
        project_callback_diagnostics_for_analysis(&db, project, source, &analysis, |_| false);
    assert!(callback_codes(&updated).is_empty());
    assert_eq!(
        project_callback_diagnostics(&db, caller, config, project).as_ref(),
        &updated
    );
}

#[test]
fn supplied_project_callback_projection_refuses_foreign_images_inputs_and_horizons() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    let db = TclDatabase::default();
    let source = "proc cb {a} {}\nlsort -command cb {2 1}";
    let file = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::new(&db, vec![file]);
    let analysis = Analyser::new().analyse(source, "tcl9.0");
    assert_eq!(
        callback_codes(&project_callback_diagnostics_for_analysis(
            &db,
            project,
            source,
            &analysis,
            |_| false,
        )),
        vec![DiagCode::E003]
    );
    let changed_source = format!("{source}\n");
    assert_eq!(
        project_callback_diagnostics_for_analysis(&db, project, &changed_source, &analysis, |_| {
            false
        },),
        analysis.diagnostics
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert_eq!(
        project_callback_diagnostics_for_analysis(&db, project, source, &missing, |_| false),
        missing.diagnostics
    );
    let mut foreign = Analyser::new().analyse(source, "tcl8.6");
    foreign
        .command_invocations
        .clone_from(&analysis.command_invocations);
    assert!(
        callback_codes(&project_callback_diagnostics_for_analysis(
            &db,
            project,
            source,
            &foreign,
            |_| false,
        ))
        .is_empty(),
        "a same-text foreign lookup cannot donate its selected context"
    );
    let mut unowned = analysis;
    for invocation in &mut unowned.command_invocations {
        invocation.original_callback_signature_lookup = None;
        invocation.name = "cb".to_owned();
        invocation.callback_arity = Some(tcl_registry::AppendedArity::Exactly(2));
        invocation.callback_baked_args = 0;
        invocation.resolution_candidates = vec!["::cb".to_owned(), "cb".to_owned()];
    }
    assert!(
        callback_codes(&project_callback_diagnostics_for_analysis(
            &db,
            project,
            source,
            &unowned,
            |_| false,
        ))
        .is_empty(),
        "prefix geometry and scalar candidates cannot replace a source horizon"
    );
}

fn legacy_callback() -> SignatureCommandInvocation {
    let analysis = Analyser::new().analyse("lsort -command cb {2 1}", "tcl9.0");
    let mut invocation = analysis
        .command_invocations
        .iter()
        .find(|invocation| invocation.original_callback_prefix.is_some())
        .unwrap()
        .clone();
    invocation.original_callback_prefix = None;
    invocation.original_callback_signature_lookup = None;
    invocation.original_lookup = None;
    invocation.original_name_input = None;
    invocation.resolved_command_reference = None;
    invocation.name = "cb".to_owned();
    invocation.resolution_candidates = vec!["cb".to_owned()];
    invocation.callback_arity = Some(tcl_registry::AppendedArity::Exactly(2));
    invocation.callback_baked_args = 0;
    invocation
}

fn legacy_advice(analysis: &AnalysisResult) -> (Vec<DiagCode>, Vec<DiagCode>) {
    let invocation = legacy_callback();
    let arities = HashMap::from([("cb".to_owned(), vec![(0, 0)])]);
    let callbacks = apply_project_callback_arity(
        analysis,
        &[],
        std::slice::from_ref(&invocation),
        &arities,
        |_| false,
    );
    let mut direct = invocation;
    direct.callback_arity = None;
    direct.argc = Some(1);
    let unresolved = vec![(direct.range, "cb".to_owned())];
    let direct_calls =
        apply_cross_file_resolution(analysis, &[], &unresolved, &[direct], &arities, |_| false);
    (
        callbacks.iter().map(|diagnostic| diagnostic.code).collect(),
        direct_calls
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect(),
    )
}

#[test]
fn legacy_project_advice_requires_positive_retained_logical_input() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    let logical = Analyser::new().analyse("cb 1", "tcl");
    assert!(logical.allows_retained_logical_declaration_advice());
    assert_eq!(
        legacy_advice(&logical),
        (vec![DiagCode::E003], vec![DiagCode::E003])
    );
    let mut missing = logical.clone();
    missing.resolved_input = None;
    assert!(!missing.allows_lexical_declaration_advice());
    assert_eq!(legacy_advice(&missing), (Vec::new(), Vec::new()));
    let mut unavailable = logical.clone();
    unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl".to_owned(),
        overlay: 0x348,
    });
    assert_eq!(unavailable.resolved_input, logical.resolved_input);
    assert!(!unavailable.allows_lexical_declaration_advice());
    assert!(!unavailable.allows_retained_logical_declaration_advice());
    assert_eq!(legacy_advice(&unavailable), (Vec::new(), Vec::new()));
    assert_eq!(
        legacy_advice(&AnalysisResult::default()),
        (Vec::new(), Vec::new())
    );
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
        "f5-tmsh",
        "f5-iapps",
    ] {
        let mut analysis = Analyser::new().analyse("proc cb {} {}", dialect);
        analysis.command_invocations = vec![legacy_callback()];
        analysis.dialect = "tcl".to_owned();
        assert!(
            !analysis.allows_retained_logical_declaration_advice(),
            "{dialect}"
        );
        assert_eq!(
            legacy_advice(&analysis),
            (Vec::new(), Vec::new()),
            "{dialect}"
        );
    }
}

#[test]
fn supplied_project_callbacks_refuse_unavailable_generation_with_original_receipts() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Original source signature advice never establishes callback execution.
    let db = TclDatabase::default();
    let source = "lsort -command cb {2 1}";
    let library = SourceFile::new(
        &db,
        "proc cb {one} {}".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let caller = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::new(&db, vec![library, caller]);
    let analysis = Analyser::new().analyse(source, "tcl9.0");
    let selection = analysis
        .command_invocations
        .iter()
        .find_map(|invocation| invocation.original_callback_signature_lookup.as_ref())
        .expect("the genuine original callback has a source registration and target horizon");
    let original_join = |result: &AnalysisResult| {
        tcl_compiler::analyser::SourceCallbackSignatureLookup::from_original_lookup(
            result,
            Arc::new(selection.prefix().clone()),
            Arc::new(selection.original().clone()),
        )
    };
    assert_eq!(original_join(&analysis).as_ref(), Some(selection.as_ref()));
    let input = analysis.resolved_input.as_ref().unwrap();
    let image = tcl_lexer::SourceImage::document(source);
    assert!(analysis.matches_original_source_image(&image, input.lexer_config()));
    assert!(callback_codes(&analysis.diagnostics).is_empty());
    assert_eq!(
        callback_codes(&project_callback_diagnostics_for_analysis(
            &db,
            project,
            source,
            &analysis,
            |_| false,
        )),
        vec![DiagCode::E003],
    );
    let mut unavailable = analysis.clone();
    unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl9.0".to_owned(),
        overlay: 0x348,
    });
    assert_eq!(unavailable.resolved_input, analysis.resolved_input);
    assert_eq!(
        unavailable.command_invocations,
        analysis.command_invocations
    );
    assert!(!unavailable.matches_original_source_image(&image, input.lexer_config()));
    assert!(original_join(&unavailable).is_none());
    assert_eq!(
        project_callback_diagnostics_for_analysis(&db, project, source, &unavailable, |_| false),
        unavailable.diagnostics,
    );
    assert!(unavailable.analysis_context_unavailable.is_some());
}
