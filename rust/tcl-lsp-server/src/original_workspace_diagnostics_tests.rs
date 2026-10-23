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
        // This deliberately fabricated reporting donation performs no index
        // query and cannot mint an original settlement or source dependency.
        command_queries: WorkspaceCommandQueries::NoRead,
        source_candidates: Vec::new(),
        resolved: HashSet::from([key]),
        arity: HashMap::from([(key, vec![(3, Some(3))])]),
    }
}

/// Preserve the old adversarial display-input fixture surface while calling
/// only the actual current refinement owner. Names/project-tier flags cannot
/// supply a source candidate or query receipt. Logical positives below supply
/// settlement issued by the genuine indexed caller/provider producer instead.
fn refine_workspace_index_w123(
    diagnostics: Vec<tcl_compiler::analyser::Diagnostic>,
    analysis: &AnalysisResult,
    _adversarial_reporting_names: Option<&HashSet<String>>,
    calls: &CrossFileCalls,
    _adversarial_project_tier: bool,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    super::refine_workspace_index_w123(diagnostics, analysis, calls)
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

    let source = "missing 1 2";
    let analysis = lexical_analysis(source);
    let provider = reporting_analysis_with_input(
        "proc missing {a b c} {return $a}",
        analysis.resolved_input.as_ref().unwrap(),
    );
    let index = core_workspace_index::WorkspaceIndex::from_documents([
        ("file:///caller.tcl", &analysis),
        ("file:///library.tcl", &provider),
    ]);
    let registry = analysis.resolved_registry().unwrap();
    let calls = settle_cross_file_calls(&index, &analysis, registry, "file:///caller.tcl");
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

fn reporting_analysis_with_input(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> AnalysisResult {
    let result = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, input.analyser_profile().name);
    assert!(result.allows_lexical_declaration_advice());
    result
}

#[test]
fn original_workspace_reporting_keeps_actual_builtin_availability_on_the_same_store() {
    // naming.consumer.original-workspace-diagnostic-refinement

    // Conditional nested source declarations are independent of entered Native
    // dispatch. Their exclusion depends on the actual builtin availability.
    let registry = CommandRegistry::build_default();
    let store = registry.snapshot().shared_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = |environment| {
        tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::new(
                tcl_registry::model::ingress::resolve_environment(environment)
                    .default_context_registry()
                    .with_command_store(Arc::clone(&store)),
            ),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        )
    };
    let older = input("tcl8.4");
    let current = input("tcl8.6");
    assert!(
        older
            .borrowed_context_registry()
            .resolve_command("try")
            .is_none()
    );
    assert!(
        current
            .borrowed_context_registry()
            .resolve_command("try")
            .is_some()
    );
    let provider = reporting_analysis_with_input(
        "proc factory {} {proc try {value} {return $value}}",
        &current,
    );
    let index =
        core_workspace_index::WorkspaceIndex::from_documents([("file:///provider.tcl", &provider)]);
    assert!(
        index
            .live_procs()
            .any(|proc| proc.qualified_name == "::try" && proc.nested)
    );
    let exports = index.export_snapshot();
    for (input, expected) in [(&older, Some("::try")), (&current, None)] {
        let analysis = reporting_analysis_with_input("try value", input);
        let invocation = analysis
            .command_invocations
            .iter()
            .find(|invocation| invocation.name == "try")
            .expect("the actual source scanner retains this call");
        assert_eq!(
            settle_call_against_workspace(
                &index,
                &analysis,
                &registry,
                "file:///caller.tcl",
                invocation,
                exports.as_ref(),
                invocation.range.start(),
            ),
            expected,
        );
        if expected.is_some() {
            // The diagnostic consumer shares the same selection owner.
            let calls = settle_cross_file_calls(&index, &analysis, &registry, "file:///caller.tcl");
            assert!(
                calls
                    .resolved
                    .contains(&(invocation.range.start(), invocation.range.end()))
            );
        }
    }
}

#[test]
fn original_workspace_reporting_refuses_pack_generations_and_withdrawn_source_owners() {
    // naming.consumer.original-workspace-diagnostic-refinement

    let source = "donated VALUE";
    let analysis = lexical_analysis(source);
    let index = report_index("proc donated {value} {return $value}");
    let registry = analysis.resolved_registry().unwrap();
    let select = |analysis: &AnalysisResult, registry: &CommandRegistry| {
        let invocation = analysis.command_invocations.first().unwrap();
        settle_call_against_workspace(
            &index,
            analysis,
            registry,
            "file:///caller.tcl",
            invocation,
            index.export_snapshot().as_ref(),
            invocation.range.start(),
        )
        .is_some()
    };
    assert!(select(&analysis, registry));
    assert!(
        !settle_cross_file_calls(&index, &analysis, registry, "file:///caller.tcl")
            .resolved
            .is_empty()
    );
    let mut changed = CommandRegistry::build_default();
    let mut descriptor = changed.get("set").unwrap().clone();
    descriptor.name = "workspace_reporting_pack_change";
    changed.insert(descriptor);
    assert_ne!(
        changed.snapshot().semantic_key(),
        registry.snapshot().semantic_key()
    );
    assert!(!select(&analysis, &changed));
    assert!(
        settle_cross_file_calls(&index, &analysis, &changed, "file:///caller.tcl")
            .resolved
            .is_empty()
    );

    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(!select(&missing, registry));
    let mut stale = analysis.clone();
    let config = stale.body_lexer_config.as_mut().unwrap();
    config.strict_quoting = !config.strict_quoting;
    assert!(!select(&stale, registry));
    let mut unavailable = analysis.clone();
    unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: analysis.dialect.clone(),
        overlay: u64::MAX - 403,
    });
    assert_ne!(unavailable.command_invocations.len(), 0);
    assert!(!select(&unavailable, registry));
    assert!(
        settle_cross_file_calls(&index, &unavailable, registry, "file:///caller.tcl")
            .resolved
            .is_empty()
    );
    assert!(
        select(&analysis, registry),
        "the restored genuine owner remains nonvacuous"
    );
}

#[test]
fn original_workspace_reporting_currency_keeps_full_store_and_independent_context() {
    // naming.consumer.original-workspace-diagnostic-refinement

    let source = "donated VALUE";
    let baseline = lexical_analysis(source);
    let original_input = baseline.resolved_input.as_ref().unwrap();
    let store = baseline
        .resolved_registry()
        .unwrap()
        .snapshot()
        .shared_registry();
    let current_input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        original_input.analyser_profile(),
        original_input.unit_profile(),
        Arc::new(
            original_input
                .borrowed_context_registry()
                .with_command_store(Arc::clone(&store)),
        ),
        original_input.lexer_config(),
    );
    let analysis = reporting_analysis_with_input(source, &current_input);
    let registry = analysis.resolved_registry().unwrap();
    let current = workspace_reporting_context(&analysis, &store).unwrap();
    assert!(current.resolve_command("try").is_some());

    let input = analysis.resolved_input.as_ref().unwrap();
    let older = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        Arc::new(
            tcl_registry::model::ingress::resolve_environment("tcl8.4")
                .default_context_registry()
                .with_command_store(Arc::clone(&store)),
        ),
        input.lexer_config(),
    );
    let older_analysis = reporting_analysis_with_input(source, &older);
    let older_context = workspace_reporting_context(&older_analysis, &store).unwrap();
    assert!(older_context.resolve_command("try").is_none());
    assert!(Arc::ptr_eq(current.commands(), older_context.commands()));

    let mut foreign = store.project_for_profile(registry.profile().unwrap());
    let mut descriptor = foreign.get("set").unwrap().clone();
    descriptor.name = "workspace_reporting_currency_foreign";
    foreign.insert(descriptor);
    assert_eq!(
        foreign
            .profile()
            .map(tcl_dialect::DialectProfile::cache_key),
        registry
            .profile()
            .map(tcl_dialect::DialectProfile::cache_key),
    );
    assert_ne!(
        foreign.snapshot().semantic_key(),
        registry.snapshot().semantic_key()
    );
    assert!(workspace_reporting_context(&analysis, &foreign).is_none());

    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(workspace_reporting_context(&missing, &store).is_none());
    let mut stale = analysis.clone();
    let config = stale.body_lexer_config.as_mut().unwrap();
    config.strict_quoting = !config.strict_quoting;
    assert!(workspace_reporting_context(&stale, &store).is_none());
    let native = native_analysis(source, "tcl8.6");
    assert!(workspace_reporting_context(&native, native.resolved_registry().unwrap()).is_none());
    assert!(workspace_reporting_context(&analysis, &store).is_some());
}
