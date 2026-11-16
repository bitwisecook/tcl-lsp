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

//! Callback argument-count diagnostics: original signatures, braced prefixes, execution traces, foreign library sources and the registry's callback descriptors.

use super::*;

#[test]
fn original_callback_missing_future_lookup_declines_reporting_candidates() {
    // Implementation contract naming.database.original-callback-future-lookup:
    // docs/design/analysis/name-resolution-proofs/database-original-callback-future-lookup.md
    let db = TclDatabase::default();
    let file = SourceFile::new(
        &db,
        "proc cb {} {}\ncb\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let analysis = file_analysis_incremental(&db, file, cfg(&db));
    let mut invocation = analysis
        .command_invocations
        .iter()
        .find(|invocation| invocation.name == "cb" && invocation.original_name_input.is_some())
        .expect("an actual original command producer")
        .clone();
    // This checks missing-purpose refusal, not callback context inference:
    // the independently retained name cannot manufacture a future lookup.
    invocation.original_lookup = None;
    invocation.resolved_command_reference = None;
    invocation.callback_arity = Some(tcl_registry::AppendedArity::Exactly(2));
    assert_eq!(invocation.logical_callback_candidate_keys().len(), 0);
    let mut missing = (*analysis).clone();
    missing.command_invocations = vec![invocation];
    let library = SourceFile::new(&db, "proc cb {} {}".to_owned(), "tcl8.6".to_owned(), None);
    let project = Project::new(&db, vec![library, file]);
    assert!(
        project_callback_diagnostics_for_analysis(&db, project, file.text(&db), &missing, |_| {
            false
        },)
        .iter()
        .all(|diagnostic| !matches!(
            diagnostic.code,
            DiagCode::E002 | DiagCode::E003 | DiagCode::E005
        ))
    );
}

/// Analyse a single-file project and return its diagnostic codes+messages.
fn callback_arity_diagnostics(src: &str) -> Vec<tcl_compiler::analyser::types::Diagnostic> {
    let db = TclDatabase::default();
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
    let f = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
    let proj = Project::new(&db, vec![f]);
    project_diagnostics(&db, f, cfg, proj).as_ref().clone()
}

fn callback_arity_codes(src: &str) -> Vec<(String, String)> {
    callback_arity_diagnostics(src)
        .iter()
        .map(|d| (d.code.as_str().to_owned(), d.message.clone()))
        .collect()
}

fn assert_callback_source_count(
    source: &str,
    input: &[u8],
    expected: &tcl_compiler::analyser::SourceCallbackArgumentCounts,
) {
    let diagnostics = callback_arity_diagnostics(source);
    let subject = diagnostics
        .iter()
        .find_map(|diagnostic| {
            let subject = diagnostic.callback_source_arity()?;
            (diagnostic.code == DiagCode::E003 && subject.prefix().name_input().bytes() == input)
                .then_some(subject)
        })
        .unwrap_or_else(|| panic!("{source}: no genuine E003 source subject in {diagnostics:?}"));
    assert_eq!(subject.argument_counts(), expected);
    assert!(subject.source_lookup().is_some());
}

#[test]
fn original_callback_diagnostics_keep_typed_subject_and_ignore_mutable_counts() {
    // naming.diagnostic.original-callback-signature-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
    // Current source headers only; no callback registration or execution.
    let db = TclDatabase::default();
    let file = SourceFile::new(
        &db,
        "proc cb {a b} {return 0}\nlsort -command {cb fixed} {3 1 2}".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![file]);
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(file.text(&db), "tcl9.0");
    let mut invocations = analysis.command_invocations;
    for invocation in &mut invocations {
        if invocation.original_callback_prefix.is_some() {
            invocation.name = "wrong presentation".to_owned();
            invocation.range = tcl_lexer::Span::new(0, 1);
            invocation.callback_arity = Some(tcl_registry::AppendedArity::Unknown);
            invocation.callback_baked_args = usize::MAX;
            invocation.original_name_input = None;
            invocation.original_lookup = None;
        }
    }
    let mut output = Vec::new();
    apply_original_callback_arity(&db, project, &mut output, &invocations, |_| false);
    let diagnostic = output
        .first()
        .expect("authentic prefix, not mutable scalar counts");
    assert_eq!(output.len(), 1);
    assert_eq!(diagnostic.code, DiagCode::E003);
    let subject = diagnostic.callback_source_arity().unwrap();
    assert_eq!(subject.prefix().baked_argument_count(), 1);
    assert_eq!(
        subject.argument_counts(),
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![3])
    );
    assert_eq!(&file.text(&db)[diagnostic.span.as_range()], "cb");
    assert_eq!(
        invocations
            .iter()
            .find(|inv| inv.original_callback_prefix.is_some())
            .unwrap()
            .logical_callback_candidate_keys()
            .len(),
        0
    );
    assert_eq!(
        tcl_lsp_core::diagnostic_subject::diagnostic_subject_data(diagnostic).unwrap()["subject"]["purpose"],
        "signature"
    );
}

#[test]
fn original_callback_signature_query_updates_headers_without_tail_substitution() {
    // naming.diagnostic.original-callback-signature-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
    use salsa::Setter as _;
    let mut db = TclDatabase::default();
    let definitions = SourceFile::new(&db,"namespace eval a {proc cb {one} {return FIRST}}\nnamespace eval b {proc cb {one two} {return 0}}".to_owned(),"tcl9.0".to_owned(),None);
    let call_source = "lsort -command ::a::cb {3 1 2}";
    let calls = SourceFile::new(&db, call_source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::new(&db, vec![definitions, calls]);
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(call_source, "tcl9.0");
    let prefix = analysis
        .command_invocations
        .iter()
        .find_map(|inv| inv.original_callback_prefix.as_ref())
        .unwrap();
    let lookup = prefix.lookup().unwrap();
    let first = original_lookup_command_signatures(&db, project, lookup).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(
        first[0].formal_count_projection().arity(),
        tcl_registry::Arity::exact(1)
    );
    definitions.set_text(&mut db).to("namespace eval a {proc cb {one} {return LATER}}\nnamespace eval b {proc cb {one two} {return 0}}".to_owned());
    assert_eq!(
        original_lookup_command_signatures(&db, project, lookup).unwrap(),
        first,
        "body-free header equality"
    );
    definitions.set_text(&mut db).to("namespace eval a {proc cb {one two} {return LATER}}\nnamespace eval b {proc cb {one} {return 0}}".to_owned());
    let updated = original_lookup_command_signatures(&db, project, lookup).unwrap();
    assert_eq!(updated.len(), 1);
    assert_eq!(
        updated[0].formal_count_projection().arity(),
        tcl_registry::Arity::exact(2)
    );
    assert!(
        tcl_compiler::analyser::SourceCallbackAritySubject::from_source_signatures(
            Arc::clone(prefix),
            &updated
        )
        .is_none()
    );
}

#[test]
fn original_callback_signature_gap_retains_disjoint_source_headers() {
    // naming.diagnostic.original-callback-signature-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
    // Two current source candidates, not two installed implementations.
    let db = TclDatabase::default();
    let one = SourceFile::new(&db, "proc cb {a} {}".to_owned(), "tcl9.0".to_owned(), None);
    let three = SourceFile::new(
        &db,
        "proc cb {a b c} {}".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let source = "lsort -command cb {1 2}";
    let call = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::new(&db, vec![one, three, call]);
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
    let mut diagnostics = Vec::new();
    apply_original_callback_arity(
        &db,
        project,
        &mut diagnostics,
        &analysis.command_invocations,
        |_| false,
    );
    let diagnostic = diagnostics
        .first()
        .expect("finite count falls in a signature gap");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostic.code, DiagCode::E005);
    let subject = diagnostic.callback_source_arity().unwrap();
    assert_eq!(subject.declarations().len(), 2);
    assert_eq!(
        subject.issue(),
        tcl_compiler::analyser::SourceCallbackArityIssue::NoCompatibleSignature { supplied: 2 }
    );
}

#[test]
fn qualified_callback_arity_keeps_same_tailed_namespaces_separate() {
    let diagnostics = callback_arity_codes(
        "namespace eval a {proc cb {one} {return 0}}\n\
             namespace eval b {proc cb {one two} {return 0}}\n\
             lsort -command ::a::cb {3 1 2}\n",
    );
    assert!(
        diagnostics.iter().any(|(code, _)| code == "E003"),
        "{diagnostics:?}"
    );
    let absent = callback_arity_codes(
        "namespace eval b {proc cb {one} {return 0}}\n\
             lsort -command ::absent::cb {3 1 2}\n",
    );
    assert!(
        absent
            .iter()
            .all(|(code, _)| !matches!(code.as_str(), "E002" | "E003" | "E005")),
        "{absent:?}"
    );
}

#[test]
fn callback_arity_mismatch_draws_e002() {
    // `lsort -command` appends 2 (Exactly(2)); `badCb` needs 3 → E002 too few.
    let d = callback_arity_codes("proc badCb {a b c} { return 0 }\nlsort -command badCb {3 1 2}\n");
    assert!(
        d.iter().any(|(c, m)| c == "E002" && m.contains("badCb")),
        "a callback proc needing 3 args used where 2 are appended must draw E002; got {d:?}"
    );
}

#[test]
fn callback_arity_correct_is_silent() {
    // A 2-arg callback matches lsort's Exactly(2) → no arity error (TN).
    let d = callback_arity_codes("proc goodCb {a b} { return 0 }\nlsort -command goodCb {3 1 2}\n");
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a correctly-sized callback must draw no arity error; got {d:?}"
    );
}

#[test]
fn callback_arity_too_many_draws_e003() {
    // A 1-param callback where lsort appends 2 → E003 too many.
    let d = callback_arity_codes("proc oneArg {a} { return 0 }\nlsort -command oneArg {3 1 2}\n");
    assert!(
        d.iter().any(|(c, m)| c == "E003" && m.contains("oneArg")),
        "a 1-param callback fed 2 args must draw E003; got {d:?}"
    );
}

#[test]
fn callback_arity_args_catchall_is_silent() {
    // FP guard: an `args` catch-all accepts any count → no arity error.
    let d = callback_arity_codes("proc anyN {args} { return 0 }\nlsort -command anyN {3 1 2}\n");
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "an `args`-catchall callback must draw no arity error; got {d:?}"
    );
}

#[test]
fn callback_arity_braced_prefix_bakes_extra_args_too_few() {
    // `-command {cb 99}` bakes 1 extra arg ahead of `lsort`'s own
    // appended 2, for 3 total; `cb` needs 4 → E002.  A braced
    // multi-word prefix must be recorded as an invocation, or this
    // would draw nothing at all.
    let d =
        callback_arity_codes("proc cb {a b c d} { return 0 }\nlsort -command {cb 99} {3 1 2}\n");
    assert!(
        d.iter().any(|(c, m)| c == "E002" && m.contains("cb")),
        "a braced prefix's baked arg must count toward the total; got {d:?}"
    );
}

#[test]
fn callback_arity_braced_prefix_bakes_extra_args_exact_match_is_silent() {
    // Same shape, but `cb` needs exactly 3 (1 baked + 2 appended) — TN.
    let d = callback_arity_codes("proc cb {a b c} { return 0 }\nlsort -command {cb 99} {3 1 2}\n");
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "1 baked + 2 appended matching cb's 3 params must be silent; got {d:?}"
    );
}

#[test]
fn callback_arity_braced_prefix_bakes_extra_args_too_many() {
    // `-command {cb 99 88}` bakes 2 + appended 2 = 4, but `cb` takes
    // only 3 → E003.
    let d =
        callback_arity_codes("proc cb {a b c} { return 0 }\nlsort -command {cb 99 88} {3 1 2}\n");
    assert!(
        d.iter().any(|(c, m)| c == "E003" && m.contains("cb")),
        "2 baked + 2 appended against a 3-param cb must draw E003; got {d:?}"
    );
}

#[test]
fn callback_arity_braced_prefix_dynamic_head_is_never_checked() {
    // FP guard: `{$cb 99}` — a dynamic head inside the braces — can't be
    // resolved to a proc; must never be flagged (and must not panic on
    // the list-parse).
    let d = callback_arity_codes(
        "proc cb {a b c d} { return 0 }\nset cb cb\nlsort -command {$cb 99} {3 1 2}\n",
    );
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a dynamic braced-prefix head must never be arity-checked; got {d:?}"
    );
}

#[test]
fn execution_trace_source_counts_keep_unavailable_future_frame() {
    // naming.callback.lookup-scope-owner
    // docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md
    // Registration syntax retains suffix metadata; it does not observe
    // the command table or namespace of a future triggering frame.
    for (operations, appended) in [
        ("enter", 2),
        ("enterstep", 2),
        ("leave", 4),
        ("leavestep", 4),
    ] {
        let source =
            format!("proc h {{a b c}} {{return 0}}\ntrace add execution somecmd {operations} h");
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl9.0");
        let prefix = analysis
            .command_invocations
            .iter()
            .find_map(|inv| inv.original_callback_prefix.as_ref())
            .expect(operations);
        assert_eq!(
            prefix.appended_arity(),
            Some(tcl_registry::AppendedArity::Exactly(appended))
        );
        assert_eq!(
            prefix.scope(),
            Some(tcl_registry::ScriptLookupScope::TriggerFrame)
        );
        assert!(prefix.lookup().is_none());
        assert!(
            callback_arity_codes(&source)
                .iter()
                .all(|(code, _)| !matches!(code.as_str(), "E002" | "E003" | "E005"))
        );
    }
}

#[test]
fn mixed_execution_trace_source_counts_do_not_issue_a_future_lookup() {
    // naming.callback.lookup-scope-owner
    // docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md
    for params in [
        "a b",
        "a b c",
        "a b c d",
        "a b {c default} {d default}",
        "a b args",
    ] {
        let source = format!(
            "proc h {{{params}}} {{return 0}}\ntrace add execution somecmd {{enter leave}} h"
        );
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl9.0");
        let prefix = analysis
            .command_invocations
            .iter()
            .find_map(|inv| inv.original_callback_prefix.as_ref())
            .expect(params);
        assert_eq!(
            prefix
                .appended_arity()
                .unwrap()
                .exact_counts()
                .unwrap()
                .collect::<Vec<_>>(),
            vec![2, 4]
        );
        assert!(prefix.lookup().is_none());
        assert!(
            callback_arity_codes(&source)
                .iter()
                .all(|(code, _)| !matches!(code.as_str(), "E002" | "E003" | "E005"))
        );
    }
}

#[test]
fn execution_trace_dynamic_and_malformed_operation_lists_abstain() {
    for source in [
        "proc h {a b c} { return 0 }\nset operations {enter leave}\ntrace add execution somecmd $operations h\n",
        "proc h {a b c} { return 0 }\ntrace add execution somecmd \"{enter\" h\n",
    ] {
        let diagnostics = callback_arity_codes(source);
        assert!(
            !diagnostics
                .iter()
                .any(|(code, _)| matches!(code.as_str(), "E002" | "E003" | "E005")),
            "an unproved operation list must not drive callback arity: {diagnostics:?}"
        );
    }
}

#[test]
fn callback_arity_namespace_unknown_zero_param_draws_e003() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    // The absolute head selects a conditional source declaration without
    // borrowing the unentered trigger's namespace. AtLeast(1) cannot fit
    // the zero-parameter source header; registration itself calls nothing.
    assert_callback_source_count(
        "proc h {} { return 0 }\nnamespace unknown ::h\n",
        b"::h",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::AtLeast(1),
    );
}

#[test]
fn callback_arity_namespace_unknown_relative_trigger_does_not_borrow_installer_frame() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    let diagnostics = callback_arity_codes("proc h {} {}; namespace unknown h");
    assert!(
        !diagnostics
            .iter()
            .any(|(code, _)| code == "E002" || code == "E003")
    );
}

fn assert_closed_tk_descriptor_contexts(
    available: &Arc<tcl_registry::model::assembly::ContextRegistry>,
    unavailable: &Arc<tcl_registry::model::assembly::ContextRegistry>,
    commands: &Arc<tcl_registry::CommandRegistry>,
) {
    assert!(Arc::ptr_eq(available.commands(), unavailable.commands()));
    assert!(
        available
            .context()
            .resolve_spec(commands, "scale")
            .is_some()
    );
    assert!(
        unavailable
            .context()
            .resolve_spec(commands, "scale")
            .is_none()
    );
    assert!(!unavailable.context().required_package_available(Some("Tk")));
    assert_eq!(unavailable.context().environment.id.as_str(), "bpf");
    assert_eq!(
        unavailable
            .context()
            .environment
            .policy_defaults
            .closed_world,
        tcl_dialect::model::WorldPolicy::Closed
    );
}

#[test]
fn callback_arity_closed_tk_descriptors_do_not_select_local_headers() {
    let available = tcl_registry::model::ingress::static_context_for("tcl9.0");
    let commands = Arc::clone(available.commands());
    let available = Arc::new(available.with_command_store(Arc::clone(&commands)));
    let unavailable = Arc::new(
        tcl_registry::model::ingress::static_context_for("bpf")
            .with_command_store(Arc::clone(&commands)),
    );
    assert_closed_tk_descriptor_contexts(&available, &unavailable, &commands);
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    let input = |context| {
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config)
    };
    let analyse = |source: &str, context| {
        tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input(context))
            .analyse(source, "tcl9.0")
    };
    let db = TclDatabase::default();
    let project = Project::new(&db, Vec::new());
    for source in [
        "proc cb {} {}; scale .s -command cb",
        "proc cb {} {}; scale .s -command ::cb",
        "package require Tk; proc cb {} {}; scale .s -command ::cb",
    ] {
        let analysis = analyse(source, Arc::clone(&unavailable));
        let supplied = input(Arc::clone(&unavailable));
        assert_eq!(analysis.resolved_input.as_ref(), Some(&supplied));
        assert!(
            analysis
                .retained_command_realm()
                .unwrap()
                .matches_resolved_analysis_input(&supplied)
        );
        assert!(
            analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        );
        assert!(
            analysis
                .resolved_input
                .as_ref()
                .unwrap()
                .borrowed_context_registry()
                .context()
                .resolve_spec(&commands, "scale")
                .is_none()
        );
        let diagnostics =
            project_callback_diagnostics_for_analysis(&db, project, source, &analysis, |_| false);
        assert!(
            !diagnostics
                .iter()
                .any(|diagnostic| matches!(diagnostic.code, DiagCode::E002 | DiagCode::E003))
        );
    }
    let source = "proc cb {} {}; scale .s -command ::cb";
    let analysis = analyse(source, Arc::clone(&available));
    assert!(
        analysis
            .resolved_input
            .as_ref()
            .unwrap()
            .borrowed_context_registry()
            .context()
            .resolve_spec(&commands, "scale")
            .is_some()
    );
    let diagnostics =
        project_callback_diagnostics_for_analysis(&db, project, source, &analysis, |_| false);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::E003)
    );
}

type ForeignCallbackContexts<'a> = (
    &'a Arc<tcl_registry::model::assembly::ContextRegistry>,
    &'a Arc<tcl_registry::model::assembly::ContextRegistry>,
    &'a Arc<tcl_registry::CommandRegistry>,
);

fn callback_input(
    context: Arc<tcl_registry::model::assembly::ContextRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) -> tcl_compiler::analyser::ResolvedAnalysisInput {
    tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config)
}

fn callback_analysis(
    source: &str,
    context: Arc<tcl_registry::model::assembly::ContextRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) -> AnalysisResult {
    tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(callback_input(context, profile, config))
        .analyse(source, "tcl9.0")
}

fn callback_codes_for_project(
    db: &TclDatabase,
    project: Project,
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<(String, String)> {
    project_callback_diagnostics_for_analysis(db, project, source, analysis, |_| false)
        .into_iter()
        .map(|diagnostic| (diagnostic.code.as_str().to_owned(), diagnostic.message))
        .collect()
}

fn assert_foreign_callback_control_source(
    db: &TclDatabase,
    project: Project,
    source: &str,
    available: &Arc<tcl_registry::model::assembly::ContextRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) -> (
    Vec<tcl_compiler::analyser::Diagnostic>,
    AnalysisResult,
    String,
) {
    let control_source = format!(
        "proc availability_control {{x}} {{}}; lsort -command ::availability_control {{2 1}}; {source}"
    );
    let control = callback_analysis(&control_source, Arc::clone(available), profile, config);
    assert!(
        control.matches_original_source_image(
            &tcl_lexer::SourceImage::document(&control_source),
            config
        )
    );
    let control_diagnostics =
        project_callback_diagnostics_for_analysis(db, project, &control_source, &control, |_| {
            false
        });
    assert!(
        control_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagCode::E003
                && diagnostic.callback_source_arity().is_some_and(|subject| {
                    subject.prefix().name_input().bytes() == b"::availability_control"
                        && subject.source_lookup().is_some()
                })
        }),
        "{control_source}"
    );
    (control_diagnostics, control, control_source)
}

fn assert_foreign_callback_sources_refuse_local_headers(
    db: &TclDatabase,
    project: Project,
    contexts: ForeignCallbackContexts<'_>,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) {
    let (available, unavailable, commands) = contexts;
    for source in [
        "proc cb {} {}; scale .s -command cb",
        "proc cb {x y} {}; math::calculus::integral 0 1 100 cb",
        "proc cb {x} {}; smtp::sendmessage $t -tlspolicy cb",
        "proc cb {x} {}; tcl::chan::halfpipe -write-command cb",
        "proc cb {} {}; mime::getbody $t -command cb",
        "proc cb {x y} {}; struct::graph g; g walk root -command cb",
        "proc cb {x y} {}; struct::tree t; t walkproc root cb",
        "package require smtp; proc cb {x} {}; smtp::sendmessage $t -tlspolicy ::cb",
        "package require mime; proc cb {} {}; mime::getbody $t -command ::cb",
        "package require struct::graph; proc cb {x y} {}; struct::graph g; g walk root -command ::cb",
        "package require struct::tree; proc cb {x y} {}; struct::tree t; t walkproc root ::cb",
    ] {
        // These library descriptors remain available in the closed model.
        // Its supplied input cannot borrow the other source producer's receipts.
        let mut analysis = callback_analysis(source, Arc::clone(available), profile, config);
        assert!(
            analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config,)
        );
        let supplied = callback_input(Arc::clone(unavailable), profile, config);
        analysis.resolved_input = Some(supplied.clone());
        assert!(
            analysis.resolved_input.as_ref() == Some(&supplied),
            "{source}: exact retained editing input"
        );
        let resolved = analysis.resolved_input.as_ref().unwrap();
        let current = resolved.context_registry();
        assert!(Arc::ptr_eq(current.commands(), commands));
        assert_eq!(resolved.lexer_config(), config);
        assert_eq!(resolved.vendor_source_policy(), None);
        assert!(!resolved.has_hosted_source_name_context());
        for command in [
            "smtp::sendmessage",
            "mime::getbody",
            "struct::graph",
            "struct::tree",
        ] {
            assert!(
                current.context().resolve_spec(commands, command).is_some(),
                "{source}: ordinary library metadata remains available"
            );
        }
        assert!(
            !analysis
                .retained_command_realm()
                .unwrap()
                .matches_resolved_analysis_input(&supplied)
        );
        assert!(
            !analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        );
        assert!(
            analysis
                .original_procedure_declarations()
                .any(|declaration| declaration.metadata().qualified_name == "::cb")
        );
        let _retained_control =
            assert_foreign_callback_control_source(db, project, source, available, profile, config);
        let diagnostics = callback_codes_for_project(db, project, source, &analysis);
        assert!(
            !diagnostics
                .iter()
                .any(|(code, _)| code == "E002" || code == "E003"),
            "{source}: {diagnostics:?}"
        );
    }
}

fn assert_foreign_callback_source_currency_refusals(
    db: &TclDatabase,
    project: Project,
    contexts: ForeignCallbackContexts<'_>,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) {
    let (available, unavailable, commands) = contexts;
    for source in [
        "package require smtp; proc cb {x} {}; smtp::sendmessage $t -tlspolicy ::cb",
        "package require mime; proc cb {} {}; mime::getbody $t -command ::cb",
        "package require struct::graph; proc cb {x y} {}; struct::graph g; g walk root -command ::cb",
        "package require struct::tree; proc cb {x y} {}; struct::tree t; t walkproc root ::cb",
    ] {
        let analysis = callback_analysis(source, Arc::clone(available), profile, config);
        let diagnostics = callback_codes_for_project(db, project, source, &analysis);
        assert!(
            diagnostics.iter().any(|(code, _)| code == "E003"),
            "{source}: {diagnostics:?}"
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            !callback_codes_for_project(db, project, source, &missing)
                .iter()
                .any(|(code, _)| code == "E002" || code == "E003")
        );
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(callback_input(Arc::clone(unavailable), profile, config));
        assert!(
            !callback_codes_for_project(db, project, source, &foreign)
                .iter()
                .any(|(code, _)| code == "E002" || code == "E003")
        );
        let foreign_store = Arc::new(available.with_command_store(Arc::new(
            tcl_registry::CommandRegistry::build_default().project_for_profile(profile),
        )));
        assert_ne!(
            foreign_store.commands().snapshot().semantic_key(),
            commands.snapshot().semantic_key()
        );
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(callback_input(foreign_store, profile, config));
        assert!(
            !callback_codes_for_project(db, project, source, &foreign)
                .iter()
                .any(|(code, _)| code == "E002" || code == "E003")
        );
        assert!(
            !callback_codes_for_project(db, project, &format!("{source}\nputs OTHER"), &analysis)
                .iter()
                .any(|(code, _)| code == "E002" || code == "E003")
        );
    }
}

#[test]
fn callback_arity_foreign_library_source_contexts_do_not_select_local_headers() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    let available = tcl_registry::model::ingress::static_context_for("tcl9.0");
    let commands = Arc::clone(available.commands());
    let available = Arc::new(available.with_command_store(Arc::clone(&commands)));
    let unavailable = Arc::new(
        tcl_registry::model::ingress::static_context_for("bpf")
            .with_command_store(Arc::clone(&commands)),
    );
    assert!(Arc::ptr_eq(available.commands(), unavailable.commands()));
    for command in [
        "smtp::sendmessage",
        "mime::getbody",
        "struct::graph",
        "struct::tree",
    ] {
        assert!(
            available
                .context()
                .resolve_spec(&commands, command)
                .is_some()
        );
        assert!(
            unavailable
                .context()
                .resolve_spec(&commands, command)
                .is_some()
        );
    }
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    assert_eq!(unavailable.context().environment.id.as_str(), "bpf");
    assert_eq!(
        unavailable
            .context()
            .environment
            .policy_defaults
            .closed_world,
        tcl_dialect::model::WorldPolicy::Closed,
    );
    assert_eq!(unavailable.context().environment.expected_packages.len(), 0);
    let excluded_input = callback_input(Arc::clone(&unavailable), profile, config);
    assert!(!excluded_input.has_hosted_source_name_context());
    assert_eq!(excluded_input.vendor_source_policy(), None);
    let db = TclDatabase::default();
    let project = Project::new(&db, Vec::new());
    assert_foreign_callback_sources_refuse_local_headers(
        &db,
        project,
        (&available, &unavailable, &commands),
        profile,
        config,
    );
    assert_foreign_callback_source_currency_refusals(
        &db,
        project,
        (&available, &unavailable, &commands),
        profile,
        config,
    );
}

#[test]
fn callback_arity_package_unknown_variadic_handler_is_silent() {
    // FP guard: `package unknown` appends AtLeast(1); an `args` handler
    // absorbs any count → no arity error (the canonical handler shape).
    let d = callback_arity_codes("proc h {args} { return 0 }\npackage unknown h\n");
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a variadic `package unknown` handler must draw no arity error; got {d:?}"
    );
}

#[test]
fn callback_arity_tk_scale_command_arity_checked() {
    // Conditional source descriptor: `scale -command` appends Exactly(1).
    // A 0-param callback can't accept it → E003; a bareword 1-param callback
    // is silent (TN).
    let bad = callback_arity_codes(
        "# tcl-lsp: requires Tk\npackage require Tk\nproc onChange {} { }\nscale .s -command ::onChange\n",
    );
    assert!(
        bad.iter()
            .any(|(c, m)| c == "E003" && m.contains("onChange")),
        "a 0-param `scale -command` callback (1 appended) must draw E003; got {bad:?}"
    );
    let ok = callback_arity_codes(
        "# tcl-lsp: requires Tk\npackage require Tk\nproc onChange {v} { }\nscale .s -command ::onChange\n",
    );
    assert!(
        !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a correct 1-param scale callback must be silent; got {ok:?}"
    );
    // A braced widget-path scroll callback is never arity-checked (not a
    // literal bareword head) — no false arity error.
    let widget = callback_arity_codes(
        "# tcl-lsp: requires Tk\npackage require Tk\nlistbox .lb -yscrollcommand {.sb set}\n",
    );
    assert!(
        !widget.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a braced widget-path scroll callback must not draw an arity error; got {widget:?}"
    );
}

#[test]
fn callback_arity_tcllib_calculus_func_arity_checked() {
    // `math::calculus::integral begin end nosteps func` calls `func x`
    // (Exactly(1), man-page-pinned).  A 2-param func is under-fed → E002.
    let d = callback_arity_codes(
        "# tcl-lsp: requires math::calculus\npackage require math::calculus\nproc f {x y} { expr {$x + $y} }\nmath::calculus::integral 0 1 100 ::f\n",
    );
    assert!(
        d.iter().any(|(c, m)| c == "E002" && m.contains("'::f'")),
        "a 2-param func where calculus::integral appends 1 must draw E002; got {d:?}"
    );
    // The correct 1-param shape is silent (TN).
    let ok = callback_arity_codes(
        "# tcl-lsp: requires math::calculus\npackage require math::calculus\nproc f {x} { expr {$x * 2} }\nmath::calculus::integral 0 1 100 ::f\n",
    );
    assert!(
        !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a correct 1-param calculus func must be silent; got {ok:?}"
    );
}

#[test]
fn callback_arity_unknown_appended_never_fires() {
    // FP guard: `coroprobe` carries `Unknown` appended arity (depends on
    // the yield point), so the injected command is a reference only —
    // never arity-checked, whatever its param count. `coroinject` is not
    // an example of this guard: its own implementation always appends a
    // verified `Exactly(2)` (see `coroinject.rs`), so a 0-param `h` there
    // correctly draws E003 instead.
    let d = callback_arity_codes("proc h {} { return 0 }\ncoroprobe myCoro h\n");
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "an Unknown-arity callback must never draw an arity error; got {d:?}"
    );
}

#[test]
fn callback_arity_option_value_exactly_checked() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // Required package metadata supplies source grammar, never loader or
    // future callback execution. The subject retains the actual prefix.
    // `smtp -tlspolicy` and `halfpipe -write-command` both append exactly 2;
    // a 1-param callback is over-fed → E003, a 2-param callback is silent.
    assert_callback_source_count(
        "# tcl-lsp: requires smtp\npackage require smtp\nproc pol {code} { }\nsmtp::sendmessage $t -tlspolicy ::pol\n",
        b"::pol",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![2]),
    );
    let smtp_ok = callback_arity_codes(
        "# tcl-lsp: requires smtp\npackage require smtp\nproc pol {code diag} { }\nsmtp::sendmessage $t -tlspolicy ::pol\n",
    );
    assert!(
        !smtp_ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a correct 2-param -tlspolicy callback must be silent; got {smtp_ok:?}"
    );
    assert_callback_source_count(
        "# tcl-lsp: requires tcl::chan::halfpipe\npackage require tcl::chan::halfpipe\nproc w {chan} { }\ntcl::chan::halfpipe -write-command ::w\n",
        b"::w",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![2]),
    );
}

#[test]
fn callback_arity_mime_getbody_command_atleast_one() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // Required package metadata supplies source grammar, never loader or
    // future callback execution. The subject retains the actual prefix.
    // `mime::getbody -command` appends AtLeast(1) (reason keyword + optional
    // payload).  A 0-param callback can't accept the reason word → E003; the
    // canonical `{reason args}` shape is silent (open-ended max ⇒ no
    // false "too many").
    assert_callback_source_count(
        "# tcl-lsp: requires mime\npackage require mime\nproc cb {} { }\nmime::getbody $t -command ::cb\n",
        b"::cb",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::AtLeast(1),
    );
    let ok = callback_arity_codes(
        "# tcl-lsp: requires mime\npackage require mime\nproc cb {reason args} { }\nmime::getbody $t -command ::cb\n",
    );
    assert!(
        !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a `{{reason args}}` mime -command callback must be silent; got {ok:?}"
    );
    // FP guard: comm's 14-arg reply callback is virtually always `{args}` —
    // the catch-all absorbs all 14 → no arity error.
    let comm = callback_arity_codes(
        "# tcl-lsp: requires comm\npackage require comm\nproc reply {args} { }\ncomm::comm send -command ::reply $id {list x}\n",
    );
    assert!(
        !comm.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a variadic comm -command ::reply handler must be silent; got {comm:?}"
    );
}

#[test]
fn callback_arity_struct_graph_walk_command_checked() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // Required package metadata supplies source grammar, never loader or
    // future callback execution. The subject retains the actual prefix.
    // `$g walk … -command ::cb` (object instance method) appends 3 (action
    // graphName node).  A 2-param callback is over-fed → E003; a 3-param one
    // is silent.  Exercises both the named (`struct::graph name`) and handle
    // (`set g [struct::graph]`) instance forms.
    assert_callback_source_count(
        "# tcl-lsp: requires struct::graph\npackage require struct::graph\nproc twoP {a b} { }\nstruct::graph myG\nmyG walk root -command ::twoP\n",
        b"::twoP",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![3]),
    );
    let ok = callback_arity_codes(
        "# tcl-lsp: requires struct::graph\npackage require struct::graph\nproc threeP {a b c} { }\nset g [struct::graph]\n$g walk root -command ::threeP\n",
    );
    assert!(
        !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a correct 3-param graph walk callback must be silent; got {ok:?}"
    );
}

#[test]
fn callback_arity_struct_tree_walkproc_checked() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // Required package metadata supplies source grammar, never loader or
    // future callback execution. The subject retains the actual prefix.
    // `$t walkproc … cmdprefix` (trailing positional prefix) appends 3 (tree
    // node action).  A 2-param callback → E003.
    assert_callback_source_count(
        "# tcl-lsp: requires struct::tree\npackage require struct::tree\nproc twoP {a b} { }\nstruct::tree myT\nmyT walkproc root ::twoP\n",
        b"::twoP",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![3]),
    );
}
