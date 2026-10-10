// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reporting compatibility cannot issue an original cross-document lookup.

use super::*;

fn native_analysis(source: &str, dialect: &str) -> AnalysisResult {
    let analysis = Analyser::new().analyse(source, dialect);
    assert!(!analysis.allows_lexical_declaration_advice(), "{dialect}");
    analysis
}

pub(super) fn lexical_analysis(source: &str) -> AnalysisResult {
    let point = tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79);
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "workspace-diagnostic-explicit-lexical",
        &[],
        "Logical workspace diagnostic source advice",
        point,
    )
    .intern();
    // Availability is independently selected; a Logical naming policy does
    // not reinterpret the C catalogue as Jim's earlier command vocabulary.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    assert!(analysis.allows_lexical_declaration_advice());
    analysis
}

fn report_index(source: &str) -> core_workspace_index::WorkspaceIndex {
    let analysis = lexical_analysis(source);
    core_workspace_index::WorkspaceIndex::from_documents([("file:///library.tcl", &analysis)])
}

fn report_calls(analysis: &AnalysisResult) -> CrossFileCalls {
    let invocation = analysis
        .command_invocations
        .iter()
        .find(|invocation| invocation.name == "missing")
        .expect("the actual original source contains this invocation");
    let key = (invocation.range.start(), invocation.range.end());
    CrossFileCalls {
        resolved: HashSet::from([key]),
        arity: HashMap::from([(key, vec![(3, Some(3))])]),
    }
}

fn original_diagnostic(analysis: &AnalysisResult) -> tcl_compiler::analyser::Diagnostic {
    analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == DiagCode::W123
                && diagnostic
                    .unresolved_command()
                    .is_some_and(|subject| subject.reporting_name() == "missing")
        })
        .cloned()
        .expect("the original emitter retains exact selected-slot absence advice")
}

#[test]
fn original_workspace_diagnostics_decline_reporting_lookup_and_callable_arity() {
    // naming.consumer.original-workspace-diagnostic-refinement
    // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
    let index = report_index("proc donated {a b c} {return $a}");
    let registry = tcl_lsp_core::registry_for_dialect("tcl8.6");
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut analysis = native_analysis("missing 1 2", dialect);
        let diagnostic = original_diagnostic(&analysis);
        let invocation = analysis
            .command_invocations
            .iter_mut()
            .find(|invocation| invocation.name == "missing")
            .unwrap();
        assert!(invocation.original_name_input.is_some(), "{dialect}");
        invocation.resolution_candidates = vec!["::donated".to_owned()];
        invocation.resolved_qualified_name = Some("::donated".to_owned());
        invocation.original_lookup = None;
        let reporting = report_calls(&analysis);

        assert!(
            settle_cross_file_calls(&index, &analysis, registry, "file:///caller.tcl")
                .resolved
                .is_empty(),
            "{dialect}: reporting candidates cannot repair the missing original lookup"
        );
        let invocation = analysis
            .command_invocations
            .iter()
            .find(|invocation| invocation.name == "missing")
            .unwrap();
        assert!(
            settle_call_against_workspace(
                &index,
                &analysis,
                registry,
                "file:///caller.tcl",
                invocation,
                index.export_snapshot().as_ref(),
                invocation.range.start(),
            )
            .is_none(),
            "{dialect}: the lowest reporting resolver must also refuse"
        );
        assert!(
            cross_file_arity_diagnostics(&analysis, &reporting, |_| false).is_empty(),
            "{dialect}: even supplied reporting arities cannot create a callable signature"
        );
        for project_tier in [false, true] {
            let names = HashSet::from(["missing".to_owned(), "::donated".to_owned()]);
            let retained = refine_workspace_index_w123(
                vec![diagnostic.clone()],
                &analysis,
                Some(&names),
                &reporting,
                project_tier,
            );
            assert_eq!(retained, vec![diagnostic.clone()], "{dialect}");
        }
    }
}

#[test]
fn original_workspace_diagnostics_keep_current_source_advice_after_foreign_reports() {
    // naming.consumer.original-workspace-diagnostic-refinement
    // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
    let source = "proc retained {x} {return $x}\nmissing 1 2";
    let mut analysis = native_analysis(source, "tcl8.6");
    let original = original_diagnostic(&analysis);
    let declaration = analysis.original_procedure_declarations().next().unwrap();
    let identity = tcl_lsp_core::original_declaration::OriginalDeclarationIdentity::for_procedure(
        "file:///caller.tcl",
        source,
        &analysis,
        declaration,
    )
    .unwrap();
    assert!(identity.is_current("file:///caller.tcl", source, &analysis));
    assert!(!identity.is_current("file:///copied.tcl", source, &analysis));

    let donor = lexical_analysis("proc donated {a b c} {return $a}");
    analysis.all_procs = donor.all_procs;
    analysis.global_scope.procs = donor.global_scope.procs;
    analysis.dialect = "copied report label".to_owned();
    let reporting = report_calls(&analysis);
    let names = HashSet::from(["missing".to_owned()]);
    assert_eq!(
        refine_workspace_index_w123(
            vec![original.clone()],
            &analysis,
            Some(&names),
            &reporting,
            true,
        ),
        vec![original.clone()]
    );
    assert!(identity.is_current("file:///caller.tcl", source, &analysis));

    let mut changed = analysis.clone();
    let config = changed.body_lexer_config.as_mut().unwrap();
    config.strict_quoting = !config.strict_quoting;
    assert!(!identity.is_current("file:///caller.tcl", source, &changed));
    assert!(!identity.is_current("file:///caller.tcl", "proc retained {} {}", &analysis));
    // Missing current source ownership cannot enable reporting compatibility.
    assert!(cross_file_arity_diagnostics(&changed, &reporting, |_| false).is_empty());
    assert_eq!(
        refine_workspace_index_w123(
            vec![original.clone()],
            &changed,
            Some(&names),
            &reporting,
            true,
        ),
        vec![original]
    );
}

#[test]
fn explicit_lexical_workspace_diagnostics_retain_their_compatibility_behaviour() {
    // naming.consumer.original-workspace-diagnostic-refinement
    // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
    let analysis = lexical_analysis("missing 1 2");
    let index = report_index("proc missing {a b c} {return $a}");
    let calls = report_calls(&analysis);
    let registry = analysis.resolved_registry().unwrap();
    let settled = settle_call_against_workspace(
        &index,
        &analysis,
        registry,
        "file:///caller.tcl",
        &analysis.command_invocations[0],
        index.export_snapshot().as_ref(),
        0,
    );
    assert_eq!(settled, Some("::missing"));
    let arity = cross_file_arity_diagnostics(&analysis, &calls, |_| false);
    assert_eq!(arity.len(), 1);
    assert_eq!(arity[0].code, DiagCode::E002);
    let diagnostic = tcl_compiler::analyser::Diagnostic::new(
        DiagCode::W123,
        analysis.command_invocations[0].range,
        "Logical source candidate is unavailable",
        tcl_compiler::analyser::Severity::Hint,
    );
    assert!(
        refine_workspace_index_w123(vec![diagnostic], &analysis, None, &calls, false).is_empty()
    );
}
