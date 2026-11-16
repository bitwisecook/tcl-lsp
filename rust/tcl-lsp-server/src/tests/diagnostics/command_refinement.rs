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

//! W123 against the workspace, and cross-file argument counts.

use super::*;

fn w123_command_name(diagnostic: &Diagnostic) -> Option<&str> {
    let data = diagnostic.data.as_ref()?;
    let subject =
        tcl_lsp_core::diagnostic_subject::DiagnosticSubjectData::from_value(data, "W123")?;
    subject.command_reporting_name()?;
    data.get("subject")?.get("reportingName")?.as_str()
}

fn w123_math_function_name(diagnostic: &Diagnostic) -> Option<&str> {
    let data = diagnostic.data.as_ref()?;
    let subject =
        tcl_lsp_core::diagnostic_subject::DiagnosticSubjectData::from_value(data, "W123")?;
    subject.math_function_reporting_name()?;
    data.get("subject")?.get("reportingName")?.as_str()
}

#[test]
fn original_package_procedures_use_shared_conditional_source_slots() {
    // naming.source.original-procedure-publications
    // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
    // naming.server.original-package-consumer-resolution
    // docs/design/analysis/name-resolution-proofs/server-original-package-consumer-resolution.md
    let profile = tcl_lsp_core::profile_for_dialect("tcl8.6");
    let source =
        "proc original {value} {}; proc removed {} {}; rename original moved; rename removed {}";
    let names = defined_original_commands(source, profile);
    assert_eq!(names.len(), 1);
    assert_eq!(names[0].slot().simple.as_bytes(), b"moved");
    assert_eq!(
        names[0].policy().authority(),
        tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    );
    let source = "proc original {first} {}; proc original {replacement} {}";
    let names = defined_original_commands(source, profile);
    assert_eq!(names.len(), 1);
    assert_eq!(names[0].slot().simple.as_bytes(), b"original");
}

#[test]
fn original_package_classes_use_shared_current_factory_slots() {
    // naming.source.original-class-publications
    // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
    // naming.server.original-package-consumer-resolution
    // docs/design/analysis/name-resolution-proofs/server-original-package-consumer-resolution.md
    let profile = tcl_lsp_core::profile_for_dialect("tcl8.6");
    for (source, expected) in [
        (
            "oo::class create Old {}; rename Old New",
            Some(b"New".as_slice()),
        ),
        ("oo::class create Old {}; rename Old {}", None),
        (
            "oo::class create Old {}; proc Old {} {}",
            Some(b"Old".as_slice()),
        ),
    ] {
        let names = defined_original_commands(source, profile);
        assert_eq!(names.len(), usize::from(expected.is_some()), "{source}");
        if let Some(expected) = expected {
            assert_eq!(names[0].slot().simple.as_bytes(), expected, "{source}");
        }
    }
    assert_eq!(
        defined_original_commands(
            "oo::class create Old {}",
            tcl_lsp_core::profile_for_dialect("tcl8.4"),
        ),
        [] as [tcl_compiler::signature_scan::scope::SignatureSourceCommand; 0]
    );
}

// Cross-file command resolution + arity.
//
// The two-file shape single-file coverage cannot catch: `deflib.tcl`
// defines `proc libtest {a b c}` and
// `plaincaller.tcl` calls `libtest 1 2`. Go-to-definition already
// resolved it; diagnostics called it unknown.
//
// Oracle, C Tcl 9.0.4:
//
// ```
// % cat deflib.tcl
// proc libtest {a b c} { return [expr {$a + $b + $c}] }
// % tclsh9.0
// % source deflib.tcl ; libtest 1 2
// wrong # args: should be "libtest a b c"
// ```

/// Settle `caller_src`'s call sites against an index built from `others`.
pub(in super::super) fn settle(
    caller_uri: &Uri,
    caller_src: &str,
    others: &[(&Uri, &str)],
) -> (tcl_compiler::analyser::AnalysisResult, CrossFileCalls) {
    // These reporting-index controls explicitly select Logical source
    // compatibility; they provide no Native lookup or callable entry.
    let analysis = original_workspace_diagnostics_tests::lexical_analysis(caller_src);
    let providers: Vec<_> = others
        .iter()
        .map(|(uri, source)| {
            (
                uri.as_str().to_owned(),
                original_workspace_diagnostics_tests::lexical_analysis(source),
            )
        })
        .collect();
    let index = core_workspace_index::WorkspaceIndex::from_documents(
        providers
            .iter()
            .map(|(uri, analysis)| (uri.as_str(), analysis)),
    );
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    if std::env::var_os("TCL_LSP_TRACE_LOGICAL_SETTLEMENT130").is_some() {
        eprintln!(
            "logical130 caller={caller_src:?} lexical={} unresolved={:?}",
            analysis.allows_lexical_declaration_advice(),
            analysis.unresolved_command_sites
        );
        eprintln!(
            "logical130 providers={:?} live={:?}",
            providers
                .iter()
                .map(|(uri, analysis)| (
                    uri,
                    analysis.allows_lexical_declaration_advice(),
                    analysis.all_procs.keys().collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>(),
            index
                .live_procs()
                .map(|proc| (&proc.qualified_name, proc.arity))
                .collect::<Vec<_>>()
        );
        for invocation in &analysis.command_invocations {
            let binding = analysis
                .retained_command_realm()
                .unwrap()
                .invocation_at_source("", invocation.range.start());
            eprintln!(
                "logical130 invocation={:?} lookup={:?} candidates={:?} logical={} presence={:?} diagnostic={:?} original={} index={:?}",
                invocation.name,
                invocation.lookup,
                invocation.resolution_candidates,
                binding.logical_source_name_advice_input().is_some(),
                binding.selected_slot_presence(),
                binding.selected_slot_diagnostic_presence(),
                binding.original_recorded_head_name_input().is_some(),
                invocation
                    .resolution_candidates
                    .iter()
                    .map(|candidate| (
                        candidate,
                        index.workspace_command_exists_for_call(candidate, false)
                    ))
                    .collect::<Vec<_>>()
            );
        }
    }
    let settled = settle_cross_file_calls(&index, &analysis, registry, caller_uri.as_str());
    (analysis, settled)
}

/// The E002/E003 codes `cross_file_arity_diagnostics` reports, in order.
fn arity_codes(
    analysis: &tcl_compiler::analyser::AnalysisResult,
    settled: &CrossFileCalls,
) -> Vec<String> {
    cross_file_arity_diagnostics(analysis, settled, |_| false)
        .iter()
        .map(|d| d.code.as_str().to_owned())
        .collect()
}

pub(in super::super) fn caller_uri() -> Uri {
    Uri::from_file_path("/proj/plaincaller.tcl").unwrap()
}

fn deflib() -> Uri {
    Uri::from_file_path("/proj/deflib.tcl").unwrap()
}

/// **TP.** The cross-file call resolves (so its W123 is
/// suppressed) *and* its wrong argument count is reported.
#[test]
fn cross_file_call_resolves_and_arity_checks() {
    let (analysis, settled) = settle(
        &caller_uri(),
        "libtest 1 2\n",
        &[(&deflib(), "proc libtest {a b c} { return 1 }\n")],
    );
    assert_eq!(
        settled.resolved.len(),
        1,
        "a workspace proc must settle the call: {settled:?}",
    );
    assert_eq!(
        arity_codes(&analysis, &settled),
        ["E002"],
        "tclsh: `wrong # args: should be \"libtest a b c\"`",
    );
}

/// **TP.** Too many arguments is the other half of the same check.
#[test]
fn cross_file_call_reports_too_many_arguments() {
    let (analysis, settled) = settle(
        &caller_uri(),
        "libtest 1 2 3 4\n",
        &[(&deflib(), "proc libtest {a b c} { return 1 }\n")],
    );
    assert_eq!(arity_codes(&analysis, &settled), ["E003"]);
}

/// **TP.** A correct call draws nothing — the check is not simply
/// "anything cross-file is an error".
#[test]
fn cross_file_call_with_a_fitting_count_is_silent() {
    let (analysis, settled) = settle(
        &caller_uri(),
        "libtest 1 2 3\n",
        &[(&deflib(), "proc libtest {a b c} { return 1 }\n")],
    );
    assert_eq!(settled.resolved.len(), 1, "still resolves: {settled:?}");
    assert_eq!(
        arity_codes(&analysis, &settled),
        [] as [std::string::String; 0]
    );
}

#[test]
fn cross_file_source_candidates_do_not_match_foreign_namespace_tails() {
    let caller = Uri::from_file_path("/proj/caller.tcl").unwrap();
    let foreign = Uri::from_file_path("/proj/foreign.tcl").unwrap();
    let (analysis, settled) = settle(
        &caller,
        "namespace eval caller { helper x }\n",
        &[(&foreign, "namespace eval foreign { proc helper {x} {} }\n")],
    );
    assert!(analysis.allows_lexical_declaration_advice());
    assert!(settled.resolved.is_empty());
}

#[test]
fn cross_file_math_function_uses_the_retained_expression_coordinate() {
    let caller = Uri::from_file_path("/proj/vector.tcl").unwrap();
    let mathfunc = Uri::from_file_path("/proj/mathfunc.tcl").unwrap();
    let source = "expr {Pi()}\n";
    let (analysis, settled) = settle(
        &caller,
        source,
        &[(
            &mathfunc,
            "namespace eval ::tcl::mathfunc { proc Pi {} { return 3.14 } }\n",
        )],
    );
    let start = u32::try_from(source.find("Pi").unwrap()).unwrap();
    assert!(settled.resolved.contains(&(start, start + 2)));
    assert!(analysis.allows_lexical_declaration_advice());
}

/// **TP — disjoint union.** Two indexed runtime identities accepting
/// exactly one and exactly three arguments do not accept the gap at two.
/// C Tcl 9.0.4 rejects `p x y` under either definition: exact-one says
/// `p a`, and exact-three says `p a b c`. The workspace may not know which
/// identity runs, but it does know the call fits neither.
#[test]
fn disjoint_cross_file_arity_ranges_reject_the_gap() {
    let second = Uri::from_file_path("/proj/deflib2.tcl").unwrap();
    let (analysis, settled) = settle(
        &caller_uri(),
        "p 1 2\n",
        &[
            (&deflib(), "proc p {a} {}\n"),
            (&second, "proc p {a b c} {}\n"),
        ],
    );
    assert_eq!(arity_codes(&analysis, &settled), ["E005"]);
    let diagnostics = cross_file_arity_diagnostics(&analysis, &settled, |_| false);
    assert_eq!(
        diagnostics[0].message,
        "Wrong argument-count shape for 'p': expected 1 or 3, got 2",
    );
}

/// **TN/FP guard.** Every member of a disjoint arity union remains valid;
/// preserving the gap must not turn the union into an intersection.
#[test]
fn disjoint_cross_file_arity_ranges_accept_each_member() {
    let second = Uri::from_file_path("/proj/deflib2.tcl").unwrap();
    for call in ["p 1\n", "p 1 2 3\n"] {
        let (analysis, settled) = settle(
            &caller_uri(),
            call,
            &[
                (&deflib(), "proc p {a} {}\n"),
                (&second, "proc p {a b c} {}\n"),
            ],
        );
        assert!(
            arity_codes(&analysis, &settled).is_empty(),
            "one indexed proc accepts `{call}`: {settled:?}",
        );
    }
}

/// **FP guard — the precision that lets this tier be on by default.** A
/// bare `buried` called at global scope has candidates `::buried` only; a
/// `proc ::deep::buried` matches none of them, so the call must NOT
/// resolve. This is exactly the bare-tail match that made the opt-in tier
/// silence 197 justified W123s over tcllib.
#[test]
fn a_namespaced_proc_does_not_settle_a_bare_global_call() {
    let (analysis, settled) = settle(
        &caller_uri(),
        "buried 1\n",
        &[(
            &deflib(),
            "namespace eval ::deep { proc buried {x} { return $x } }\n",
        )],
    );
    assert!(
        settled.resolved.is_empty(),
        "`buried` at global scope reaches `::buried`, which nothing \
             defines — tclsh: `invalid command name \"buried\"`: {settled:?}",
    );
    assert_eq!(
        arity_codes(&analysis, &settled),
        [] as [std::string::String; 0]
    );
}

/// **FN guard.** A command nothing in the workspace defines still does not
/// resolve, so its W123 survives every tier.
#[test]
fn a_genuinely_unknown_command_never_resolves() {
    let (_, settled) = settle(
        &caller_uri(),
        "totallyUnknownCommand 1 2\n",
        &[(&deflib(), "proc libtest {a b c} { return 1 }\n")],
    );
    assert!(settled.resolved.is_empty(), "{settled:?}");
}

/// **TN — `args` tail.** An unbounded signature accepts any count above
/// its minimum, so no argument count draws an error.
#[test]
fn arity_abstains_for_an_args_tailed_signature() {
    let (analysis, settled) = settle(
        &caller_uri(),
        "argstail 1 2 3 4 5 6 7\n",
        &[(&deflib(), "proc argstail {a args} { return $a }\n")],
    );
    assert_eq!(settled.resolved.len(), 1, "still resolves: {settled:?}");
    assert!(
        arity_codes(&analysis, &settled).is_empty(),
        "a trailing `args` is unbounded (tclsh accepts every count >= 1)",
    );
}

/// **TN — `args` tail, below the minimum.** The minimum is still enforced:
/// abstention is about the *upper* bound, not a blanket exemption.
#[test]
fn arity_still_enforces_the_minimum_of_an_args_tailed_signature() {
    let (analysis, settled) = settle(
        &caller_uri(),
        "argstail\n",
        &[(&deflib(), "proc argstail {a args} { return $a }\n")],
    );
    assert_eq!(arity_codes(&analysis, &settled), ["E002"]);
}

/// **TN — defaulted parameter.** A default lowers the minimum, so both the
/// short and the full call are accepted.
#[test]
fn arity_abstains_across_a_defaulted_parameter() {
    for call in ["defaulted 1\n", "defaulted 1 2\n"] {
        let (analysis, settled) = settle(
            &caller_uri(),
            call,
            &[(&deflib(), "proc defaulted {a {b 2}} { return $a }\n")],
        );
        assert!(
            arity_codes(&analysis, &settled).is_empty(),
            "`{call}` is accepted by tclsh",
        );
    }
}

/// **TN — computed parameter list.** `proc p $params {…}` declares an
/// unknown number of formals; reading the empty recorded list as "takes no
/// arguments" draws a false E003, so the arity must be fully open.
#[test]
fn arity_abstains_for_a_computed_parameter_list() {
    let (analysis, settled) = settle(
        &caller_uri(),
        "computed 1 2 3\n",
        &[(
            &deflib(),
            "set params {a b}\nproc computed $params { return 1 }\n",
        )],
    );
    assert!(arity_codes(&analysis, &settled).is_empty(), "{settled:?}");
}

/// **TN — a class shares the name.** The call may dispatch to the
/// arity-less class command, so the name resolves (no W123) but draws no
/// arity error.
#[test]
fn arity_abstains_when_a_class_shares_the_name() {
    let (analysis, settled) = settle(
        &caller_uri(),
        "Widget 1 2 3 4 5\n",
        &[(&deflib(), "oo::class create Widget {}\n")],
    );
    assert_eq!(
        settled.resolved.len(),
        1,
        "the class command resolves the name: {settled:?}",
    );
    assert!(
        arity_codes(&analysis, &settled).is_empty(),
        "a class command has no fixed arity",
    );
}

/// **TN — dynamic providers.** `namespace unknown` makes resolution
/// unknowable, and the analyser records no unresolved sites at all, so
/// both the suppression and the arity check abstain together.
#[test]
fn nothing_settles_when_the_document_has_dynamic_providers() {
    let (analysis, settled) = settle(
        &caller_uri(),
        "namespace unknown myhandler\nlibtest 1 2\n",
        &[(&deflib(), "proc libtest {a b c} { return 1 }\n")],
    );
    assert!(
        analysis.has_dynamic_providers,
        "`namespace unknown` must widen to dynamic providers",
    );
    assert!(settled.resolved.is_empty(), "{settled:?}");
    assert_eq!(
        arity_codes(&analysis, &settled),
        [] as [std::string::String; 0]
    );
}

/// **TN — a `{*}`-expanded call.** The true argument count is a runtime
/// fact, so no arity claim can be made.
#[test]
fn arity_abstains_for_an_expanded_call() {
    let (analysis, settled) = settle(
        &caller_uri(),
        "set a {1 2}\nlibtest {*}$a\n",
        &[(&deflib(), "proc libtest {a b c} { return 1 }\n")],
    );
    assert!(arity_codes(&analysis, &settled).is_empty(), "{settled:?}");
}

/// A disabled code is honoured — the arity diagnostics are synthesised
/// after the analyser applied its own filter, so this path must replicate
/// it or a user who turned E002 off still sees it.
#[test]
fn cross_file_arity_honours_a_disabled_code() {
    let (analysis, settled) = settle(
        &caller_uri(),
        "libtest 1 2\n",
        &[(&deflib(), "proc libtest {a b c} { return 1 }\n")],
    );
    let out = cross_file_arity_diagnostics(&analysis, &settled, |code| code == "E002");
    assert!(out.is_empty(), "{out:?}");
}

/// Native selected-slot absence survives both exact report-name and
/// opt-in report-tail matches. Indexed source does not prove loading.
#[tokio::test]
async fn original_workspace_w123_preserves_native_absence_with_project_matching() {
    // naming.consumer.original-workspace-diagnostic-refinement
    // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
    let backend = test_backend();
    let a = Uri::from_str("file:///a.tcl").unwrap();
    let decoy = Uri::from_str("file:///decoy.tcl").unwrap();
    let exact = Uri::from_str("file:///exact.tcl").unwrap();
    register(
        &backend,
        &decoy,
        "namespace eval decoy { proc helper {x y} { return $x } }\n",
    )
    .await;
    register(&backend, &exact, "proc exact_helper {x y} { return $x }\n").await;
    for project_tier in [false, true] {
        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "crossFileResolution": project_tier })
                .as_object()
                .unwrap(),
        );
        // Each root script has its own authentic first lookup. An unknown
        // first command cannot establish later command completion/lookup.
        for (source, expected) in [
            ("helper foo bar\n", "helper"),
            ("exact_helper foo bar\n", "exact_helper"),
            ("neverDefinedAnywhereAtAll x\n", "neverDefinedAnywhereAtAll"),
        ] {
            let diagnostics = backend
                .full_diagnostics_for(&a, Arc::from(source), "tcl8.6".to_owned(), "tcl")
                .await
                .unwrap();
            let names: Vec<_> = diagnostics.iter().filter_map(w123_command_name).collect();
            assert!(
                names.contains(&expected),
                "project={project_tier}: a report-name/tail supplies no loaded Native lookup: {names:?}"
            );
        }
    }
}

/// A separately owned source function header retains no reached function
/// dispatch. The exact slot label cannot supply that missing obligation.
#[tokio::test]
async fn original_workspace_math_source_candidate_keeps_dispatch_unresolved() {
    // Implementation contract: naming.diagnostic.original-math-function-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
    // naming.consumer.original-workspace-diagnostic-refinement
    // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
    let backend = test_backend();
    let helper = Uri::from_str("file:///helper.tcl").unwrap();
    let vector = Uri::from_str("file:///vector.tcl").unwrap();
    register(
        &backend,
        &helper,
        "namespace eval tcl::mathfunc {\n    proc Pi {} { return 3.14159 }\n}\n",
    )
    .await;
    for project_tier in [false, true] {
        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "crossFileResolution": project_tier })
                .as_object()
                .unwrap(),
        );
        // A source-owned sibling header is not a loaded function in this
        // actual root expression's independently selected lookup purpose.
        for (source, expected) in [
            ("expr {Pi()}\n", "Pi"),
            ("NeverDefinedAnywhere\n", "NeverDefinedAnywhere"),
        ] {
            let diagnostics = backend
                .full_diagnostics_for(&vector, Arc::from(source), "tcl8.6".to_owned(), "tcl")
                .await
                .unwrap();
            let names: Vec<_> = diagnostics
                .iter()
                .filter_map(|diagnostic| {
                    if source.starts_with("expr") {
                        w123_math_function_name(diagnostic)
                    } else {
                        w123_command_name(diagnostic)
                    }
                })
                .collect();
            assert!(
                names.contains(&expected),
                "project={project_tier}: source advice supplies no dispatch: {names:?}; diagnostics={diagnostics:#?}"
            );
        }
    }
}

/// A foreign namespace's same-tail function header cannot satisfy the
/// expression's independently selected function-lookup purpose.
#[tokio::test]
async fn original_workspace_mathfunc_rejects_foreign_namespace_report_tails() {
    // Implementation contract: naming.diagnostic.original-math-function-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
    // naming.consumer.original-workspace-diagnostic-refinement
    // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
    let backend = test_backend();
    let helper = Uri::from_str("file:///decoy.tcl").unwrap();
    let vector = Uri::from_str("file:///consumer.tcl").unwrap();
    register(
        &backend,
        &helper,
        "namespace eval helpers {\n    proc Pi {} { return 3.14159 }\n}\n",
    )
    .await;
    let vector_src = "expr {Pi()}\n";

    let off = backend
        .full_diagnostics_for(&vector, Arc::from(vector_src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    let off_names: Vec<&str> = off
            .iter()
            .filter(|d| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W123"))
            .filter_map(|d| w123_math_function_name(d))
            .collect();
    assert!(
        off_names.contains(&"Pi"),
        "`::helpers::Pi` is not `::tcl::mathfunc::Pi`; `expr {{Pi()}}` cannot \
             reach it, so the W123 must stand, got: {off_names:?}; diagnostics={off:#?}",
    );

    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "crossFileResolution": true })
            .as_object()
            .unwrap(),
    );
    let on = backend
        .full_diagnostics_for(&vector, Arc::from(vector_src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    let on_names: Vec<&str> = on
            .iter()
            .filter(|d| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W123"))
            .filter_map(|d| w123_math_function_name(d))
            .collect();
    assert!(
        on_names.contains(&"Pi"),
        "Native function lookup cannot be replaced by report tails, got: {on_names:?}; diagnostics={on:#?}",
    );
}

/// A command defined in a library on the
/// `auto_path` — a `tclIndex` auto-loads it by bare name, the BLT/Rbc idiom —
/// must NOT be flagged "Unknown command" (W123), *with `xcDiagnostics` and
/// `crossFileResolution` both left off* (their default), because the
/// package database resolves it exactly as go-to-definition does.
///
/// * TN (must-stay-silent): the caller uses `Rbc_ActiveLegend`, which the
///   scanned `tclIndex` declares → no W123.
/// * TP (must-fire control): a typo the index does not declare → W123 stands.
/// * FN-guard: an empty database makes the real command genuinely unknowable
///   → W123 fires — proving suppression is data-driven, not a name allowlist.
#[tokio::test]
async fn autoload_library_command_suppresses_w123_issue_832() {
    let backend = test_backend();
    // A library dir on the auto_path: global procs registered by a `tclIndex`,
    // auto-loadable by bare name with no `package require`.
    let lib = TmpWs::new("rbc-lib");
    lib.write(
        "rbc/graph.tcl",
        "proc Rbc_ActiveLegend {graph} {}\nproc Rbc_ZoomStack {graph args} {}\n",
    );
    lib.write(
        "rbc/tclIndex",
        "# Tcl autoload index file, version 2.0\n\
             set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n\
             set auto_index(Rbc_ZoomStack) [list source [file join $dir graph.tcl]]\n",
    );
    {
        let mut resolver = PackageResolver::new();
        resolver.scan_tree(&lib.0, 100);
        *backend.package_resolver.write().await = resolver;
    }
    let uri = Uri::from_str("file:///app.tcl").unwrap();

    // TN: the real library commands — no `package require` in the caller —
    // must not be flagged, with xcDiagnostics/crossFileResolution at their
    // default (off).
    let ok_src = "Rbc_ActiveLegend .g\nRbc_ZoomStack .g\n";
    let diags = backend
        .full_diagnostics_for(&uri, Arc::from(ok_src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !diag_codes(&diags).iter().any(|c| c == "W123"),
        "a command the auto_path tclIndex provides must not be W123 (#832), got: {:?}",
        diag_codes(&diags),
    );

    // TP control: a typo the index does not declare stays flagged.
    let typo_src = "Rbc_ActveLegend .g\n";
    let typo = backend
        .full_diagnostics_for(&uri, Arc::from(typo_src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        diag_codes(&typo).iter().any(|c| c == "W123"),
        "a command no index provides must still be W123, got: {:?}",
        diag_codes(&typo),
    );

    // FN-guard: with an EMPTY database the real command is unknowable ⇒ W123
    // fires — suppression is driven by the database, not a name allowlist.
    *backend.package_resolver.write().await = PackageResolver::new();
    let empty = backend
        .full_diagnostics_for(&uri, Arc::from(ok_src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        diag_codes(&empty).iter().any(|c| c == "W123"),
        "with no package database the command is genuinely unknown ⇒ W123, got: {:?}",
        diag_codes(&empty),
    );
}

/// Secondary path: a `pkgIndex`-only package (no `tclIndex`) whose
/// implementation defines the command, made available to a sourced module by
/// an entry file's `package require`, suppresses the module's W123. The
/// package's source files are consulted through the analyser's
/// registry-driven definer walk (`proc` / `oo::class` / … from the command
/// registry's `SymbolDef`s), not a `proc`-name scan.
#[tokio::test]
async fn pkgindex_package_source_command_suppresses_w123() {
    let backend = test_backend();
    // A pkgIndex-only package whose source defines a global proc.
    let ws = TmpWs::new("pkgsrc-w123");
    ws.write(
        "mylib/mylib.tcl",
        "package provide mylib 1.0\nproc draw_widget {w} {}\n",
    );
    ws.write(
        "mylib/pkgIndex.tcl",
        "package ifneeded mylib 1.0 [list source [file join $dir mylib.tcl]]\n",
    );
    {
        let mut resolver = PackageResolver::new();
        resolver.scan_tree(&ws.0, 100);
        *backend.package_resolver.write().await = resolver;
    }
    let util = Uri::from_file_path("/proj/lib/util.tcl").unwrap();
    // The module calls `draw_widget` with no local `package require`.
    let util_src = "draw_widget .w\n";

    // Control (FN would be a bug here): without an entry file requiring mylib,
    // the module inherits nothing, so `draw_widget` is genuinely unknown ⇒ W123.
    let unrefined = backend
        .full_diagnostics_for(&util, Arc::from(util_src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        diag_codes(&unrefined).iter().any(|c| c == "W123"),
        "without an inherited require the command is unknown ⇒ W123, got: {:?}",
        diag_codes(&unrefined),
    );

    // Index an entry file that requires mylib and sources lib/util.tcl, so the
    // module inherits the `mylib` require via the workspace `source` graph.
    {
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a
            .analyse("package require mylib\nsource lib/util.tcl\n", "tcl8.6")
            .clone();
        let app = Uri::from_file_path("/proj/app.tcl").unwrap();
        backend
            .workspace_index
            .write()
            .await
            .add_document(app.as_str(), &analysis);
    }

    // TN: the inherited `mylib` require makes `draw_widget` resolvable via the
    // package's implementation source ⇒ the W123 is refined away.
    let refined = backend
        .full_diagnostics_for(&util, Arc::from(util_src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !diag_codes(&refined).iter().any(|c| c == "W123"),
        "an available package's source-defined command must not be W123, got: {:?}",
        diag_codes(&refined),
    );
}

/// Whole-workspace scope: a proc defined in a file
/// that is on disk but **not open** in the editor (driven here through
/// `reindex_index_from_disk`, the `did_close` / scan / watched-file path) must
/// still suppress a sibling's W123 and drive its arity error — cross-file diagnostics
/// tracks the same on-disk population as the workspace index, not only open
/// documents.
#[tokio::test]
async fn cross_file_resolves_against_disk_backed_file() {
    let root = unique_scratch_dir("xc-disk");
    let on_disk = root.join("lib.tcl");
    std::fs::write(&on_disk, "proc helper {x y} { return $x }\n").unwrap();

    let backend = test_backend();
    let b = Uri::from_file_path(&on_disk).unwrap();
    let a = Uri::from_str("file:///caller.tcl").unwrap();

    // B is on disk but never opened — the did_close / scan path indexes it
    // into both the workspace index and the salsa `Project`.
    backend.reindex_index_from_disk(&b).await.unwrap();
    assert!(
        backend.db_project.lock().await.is_some(),
        "a disk-backed file must seed the salsa Project"
    );
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "crossFileResolution": true })
            .as_object()
            .unwrap(),
    );

    // Correct arity (2 args) → resolves cross-file, no W123, proving the
    // never-opened disk file is in the resolution domain.
    let ok = backend
        .full_diagnostics_for(
            &a,
            Arc::from("helper foo bar\n"),
            "tcl8.6".to_owned(),
            "tcl",
        )
        .await
        .unwrap();
    assert!(
        !diag_codes(&ok).iter().any(|c| c == "W123"),
        "W123 must be suppressed against a disk-backed (unopened) file, got: {:?}",
        diag_codes(&ok),
    );

    // Wrong arity (3 args to the 2-param proc) → E003 (too many) from the
    // disk-backed proc's arity signature.
    let bad = backend
        .full_diagnostics_for(&a, Arc::from("helper a b c\n"), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        diag_codes(&bad).iter().any(|c| c == "E003"),
        "E003 must fire against a disk-backed proc's arity, got: {:?}",
        diag_codes(&bad),
    );

    std::fs::remove_dir_all(&root).ok();
}

/// When a disk-backed workspace file disappears (deletion / no longer readable)
/// and is reindexed, it must drop out of the salsa `Project` too, so its procs
/// stop resolving cross-file (a sibling's W123 returns).  Guards the removal
/// half of the whole-workspace scope.
#[tokio::test]
async fn cross_file_drops_disk_backed_file_when_gone() {
    let root = unique_scratch_dir("xc-disk-gone");
    let on_disk = root.join("lib.tcl");
    std::fs::write(&on_disk, "proc helper {x y} { return $x }\n").unwrap();

    let backend = test_backend();
    let b = Uri::from_file_path(&on_disk).unwrap();
    let a = Uri::from_str("file:///caller.tcl").unwrap();
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "crossFileResolution": true })
            .as_object()
            .unwrap(),
    );

    // Present on disk → resolves cross-file (no W123).
    backend.reindex_index_from_disk(&b).await.unwrap();
    let present = backend
        .full_diagnostics_for(
            &a,
            Arc::from("helper foo bar\n"),
            "tcl8.6".to_owned(),
            "tcl",
        )
        .await
        .unwrap();
    assert!(
        !diag_codes(&present).iter().any(|c| c == "W123"),
        "precondition: resolves while the file is present, got: {:?}",
        diag_codes(&present),
    );

    // Delete the file and reindex — it must leave the Project, so `helper` is
    // unresolved again (W123 returns).
    std::fs::remove_file(&on_disk).unwrap();
    backend.reindex_index_from_disk(&b).await.unwrap();
    let gone = backend
        .full_diagnostics_for(
            &a,
            Arc::from("helper foo bar\n"),
            "tcl8.6".to_owned(),
            "tcl",
        )
        .await
        .unwrap();
    assert!(
        diag_codes(&gone).iter().any(|c| c == "W123"),
        "W123 must return once the defining file is gone, got: {:?}",
        diag_codes(&gone),
    );

    std::fs::remove_dir_all(&root).ok();
}

/// Pull path, W123 disabled: cross-file arity must
/// stay independent of the W123 toggle on the pull path too — disabling W123
/// while keeping E002/E003 must still report the cross-file `E003` (matching
/// the push path / local arity).
#[tokio::test]
async fn cross_file_arity_survives_w123_disabled_on_pull_path() {
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({ "diagnostics": { "W123": false } }))
        .await
        .unwrap();
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "crossFileResolution": true })
            .as_object()
            .unwrap(),
    );
    let a = Uri::from_str("file:///caller.tcl").unwrap();
    let b = Uri::from_str("file:///lib.tcl").unwrap();
    register(&backend, &b, "proc helper {x y} { return $x }\n").await;
    // 3 args to a 2-param cross-file proc → E003, even with W123 disabled.
    let diags = backend
        .full_diagnostics_for(&a, Arc::from("helper a b c\n"), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    let codes = diag_codes(&diags);
    assert!(
        codes.iter().any(|c| c == "E003"),
        "cross-file E003 must survive W123 disabled on the pull path, got: {codes:?}",
    );
    assert!(
        !codes.iter().any(|c| c == "W123"),
        "W123 stays disabled, got: {codes:?}",
    );
}
