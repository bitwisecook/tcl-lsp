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

//! The published report: codes, policy, style and XC findings.

use super::*;

#[tokio::test]
async fn default_off_w242_hidden_by_default_enableable_via_config() {
    // End-to-end: a default-off W242 is not published by default, but
    // `tclLsp.diagnostics.W242: true` turns it on (the server has a
    // per-code enable path).
    let backend = test_backend();
    let uri = Uri::from_str("file:///w242.tcl").unwrap();
    let src = "while {$x < 10} {puts hi}\n"; // emits W242 from the analyser
    register(&backend, &uri, src).await;

    let off = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !off.iter().any(|d| matches!(
            &d.code,
            Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W242"
        )),
        "W242 hidden by default: {off:?}",
    );

    backend
        .apply_global_config(&serde_json::json!({ "diagnostics": { "W242": true } }))
        .await
        .unwrap();
    let on = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        on.iter().any(|d| matches!(
            &d.code,
            Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W242"
        )),
        "W242 should appear once enabled: {on:?}",
    );
}

/// A policy with every optimisation on and nothing configured — what the
/// old lifts took as `optimiser_enabled = true` and empty sets.
fn open_policy() -> core_policy::Policy {
    let mut policy = core_policy::PolicyBuilder::new().build();
    policy.optimiser = core_policy::OptimiserPolicy::all_on();
    policy
}

/// The wire set of `src`'s compiler checks and rewrites under `policy`,
/// through the one report path every publish path takes.
fn lifted_compiler_set(
    src: &str,
    registry: &CommandRegistry,
    dialect: &str,
    policy: &core_policy::Policy,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    let diags = tcl_lsp_db::compiler_check_diagnostics_uncached(src, registry, dialect, None, None);
    lift_report(src, &core_policy::apply(compiler_findings(&diags), policy))
}

#[test]
fn report_publication_retains_unavailable_generation_status() {
    // naming.diagnostic.typed-subject-reporting
    // docs/design/analysis/name-resolution-proofs/diagnostic-typed-subject-reporting.md
    // The publication adapter carries an explicit refused input, independently of message text.
    let mut analysis = AnalysisResult::default();
    analysis.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl9.0".to_owned(),
        overlay: u64::MAX,
    });
    let mut report = core_policy::apply(Vec::new(), &open_policy());
    report.retain_analysis_context(&analysis);
    let rows = lift_report("", &report);
    assert_eq!(rows.len(), 1);
    assert!(has_code(&rows, "ANALYSIS_CONTEXT_UNAVAILABLE"));
    let data = rows[0].data.as_ref().unwrap();
    assert_eq!(data["kind"], "analysisContextUnavailable");
    assert_eq!(data["environment"], "tcl9.0");
    assert_eq!(data["overlay"], u64::MAX);
    assert_eq!(
        lift_report("", &core_policy::apply(Vec::new(), &open_policy())),
        [] as [tower_lsp_server::ls_types::Diagnostic; 0]
    );
}

/// The constant-true `if` is folded by SCCP and surfaced
/// as an `O100` constant-branch diagnostic from `run_all_checks`.
/// The base analyser never emits it, so a non-empty result with an
/// O-code proves the compiler-check pipeline's output reaches the
/// published diagnostic set.
#[test]
fn the_report_surfaces_compiler_check_codes() {
    let registry = CommandRegistry::build_default();
    let src = "if {1} { set x 1 } else { set y 2 }\n";
    let diags = lifted_compiler_set(src, &registry, "", &open_policy());
    assert!(
        has_code(&diags, "O100"),
        "expected an O100 constant-branch diagnostic, got: {:?}",
        diags.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
    );
    // Every lifted diagnostic carries the shared source tag.
    assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));
}

/// An iRules taint flow (`HTTP::uri` → `HTTP::respond`) is an
/// IRULE3001 the base analyser does not emit; the dialect-aware
/// registry path must surface it through the report.
#[test]
fn the_report_surfaces_irules_taint_flow() {
    let registry = tcl_registry::model::ingress::static_context_for_profile(
        tcl_dialect::DialectProfile::irules(),
    )
    .commands();
    let src = "set u [HTTP::uri]\nHTTP::respond 200 content $u\n";
    let diags = lifted_compiler_set(src, registry, "f5-irules", &open_policy());
    assert!(
        has_code(&diags, "IRULE3001"),
        "expected IRULE3001, got: {:?}",
        diags.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
    );
}

/// A per-check feature toggle (`tclLsp.diagnostics.<CODE> = false`)
/// must suppress a compiler-*check* code — not just the analyser families.
#[test]
fn the_report_honours_a_per_check_disable() {
    let registry = tcl_registry::model::ingress::static_context_for_profile(
        tcl_dialect::DialectProfile::irules(),
    )
    .commands();
    let src = "set u [HTTP::uri]\nHTTP::respond 200 content $u\n";
    let baseline = lifted_compiler_set(src, registry, "f5-irules", &open_policy());
    assert!(
        has_code(&baseline, "IRULE3001"),
        "expected IRULE3001 baseline"
    );
    let mut policy = core_policy::PolicyBuilder::new()
        .layer(
            core_policy::PolicyLayer::Editor,
            &serde_json::json!({ "diagnostics": { "IRULE3001": false } }),
        )
        .build();
    policy.optimiser = core_policy::OptimiserPolicy::all_on();
    let filtered = lifted_compiler_set(src, registry, "f5-irules", &policy);
    assert!(
        !has_code(&filtered, "IRULE3001"),
        "IRULE3001 must be suppressed when disabled per-check: {:?}",
        filtered.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
    );
}

/// W110 owns the string-comparison site it shares with O120: the report
/// keeps one squiggle where the two coincide and both where they do not.
#[test]
fn the_report_keeps_one_squiggle_where_w110_and_o120_coincide() {
    use tcl_lexer::Span;
    let finding =
        |code: DiagCode, span: Span, producer: core_policy::Producer| core_policy::Finding {
            code,
            span,
            severity: tcl_core_types::Severity::Warning,
            message: code.to_string(),
            fixes: Vec::new(),
            data: None,
            producer,
        };
    let same = Span::new(2, 4);
    let other = Span::new(10, 12);
    let policy = open_policy();
    let report = core_policy::apply(
        vec![
            finding(DiagCode::W110, same, core_policy::Producer::Analyser),
            finding(DiagCode::O120, same, core_policy::Producer::Optimiser),
            finding(DiagCode::O120, other, core_policy::Producer::Optimiser),
        ],
        &policy,
    );
    let codes: Vec<String> = report.shown().map(|s| s.finding.code.to_string()).collect();
    assert_eq!(codes, ["W110", "O120"]);
    let alone = core_policy::apply(
        vec![finding(
            DiagCode::O120,
            same,
            core_policy::Producer::Optimiser,
        )],
        &policy,
    );
    assert_eq!(alone.shown().count(), 1, "O120 survives without W110");
}

/// The wire set of the source-style pass over `src` under `policy`.
fn lifted_style_set(
    src: &str,
    policy: &core_policy::Policy,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    let doc = core_report::DocumentSource {
        text: src,
        analysis_text: src,
        decode: None,
        dialect: tcl_lsp_core::profile_for_dialect("tcl9.0"),
        pass: core_report::SourcePass::Tcl {
            line_length: tcl_lsp_core::source_style::DEFAULT_LINE_LENGTH,
        },
    };
    lift_report(src, &core_report::document_report(&doc, Vec::new(), policy))
}

/// The source-style pass must reach the
/// published set.  A long line + trailing whitespace + CRLF
/// endings exercise W111 / W112 / W118 — none of which the
/// analyser or compiler-check pipelines emit — so a non-empty
/// result with those codes proves the report runs the pass.
#[test]
fn the_report_surfaces_style_codes() {
    let long = "x".repeat(130);
    let src = format!("{long}  \r\nputs ok\r\n");
    let diags = lifted_style_set(&src, &core_policy::PolicyBuilder::new().build());
    let codes = diag_codes(&diags);
    for want in ["W111", "W112", "W118"] {
        assert!(
            codes.iter().any(|c| c == want),
            "expected {want}, got: {codes:?}",
        );
    }
    assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));
}

/// W107 in a file whose lines end with a lone `\r` is published at the
/// U+FFFD — line 2, character 6 — where the same bytes with `\n`
/// endings put it, not clamped to the end of line 0.
#[test]
fn the_report_places_w107_on_the_lsp_line_model() {
    let (text, decode) =
        tcl_lsp_core::source_decode::decode_source(b"set a 1\rset b 2\rputs \"\xff bad\"\n");
    let analysis_text = tcl_lexer::normalise_lone_cr(&text);
    let doc = core_report::DocumentSource {
        text: &text,
        analysis_text: &analysis_text,
        decode: Some(&decode),
        dialect: tcl_lsp_core::profile_for_dialect("tcl9.0"),
        pass: core_report::SourcePass::Tcl {
            line_length: tcl_lsp_core::source_style::DEFAULT_LINE_LENGTH,
        },
    };
    let report =
        core_report::document_report(&doc, Vec::new(), &core_policy::PolicyBuilder::new().build());
    let w107 = lift_report(&text, &report)
        .into_iter()
        .find(|d| {
            d.code
                == Some(tower_lsp_server::ls_types::NumberOrString::String(
                    "W107".to_owned(),
                ))
        })
        .expect("W107 is published");
    assert_eq!((w107.range.start.line, w107.range.start.character), (2, 6));
    assert_eq!((w107.range.end.line, w107.range.end.character), (2, 7));
}

#[test]
fn report_driven_abstention_survives_a_disabled_w109() {
    let bytes: Vec<u8> = "ltm pool /Common/p { members { /Common/a:80 {} } }\n"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let (text, report) = tcl_lsp_core::source_decode::decode_source(&bytes);
    assert!(report.requires_abstention());
    let derived = core_policy::Finding {
        code: DiagCode::Bigip6002,
        span: tcl_lexer::Span::new(0, 1),
        severity: tcl_core_types::Severity::Warning,
        message: "derived from mis-decoded bytes".to_owned(),
        fixes: Vec::new(),
        data: None,
        producer: core_policy::Producer::BigipModel,
    };
    let layers = PolicyLayers {
        editor: serde_json::json!({ "diagnostics": { "W109": false } }),
        ..PolicyLayers::default()
    };
    let diagnostics = f5_model_report(
        &F5ModelDocument {
            text: &text,
            analysis_text: &text,
            decode_report: Some(&report),
            dialect: tcl_lsp_core::profile_for_dialect("f5-tmsh"),
        },
        &layers,
        vec![derived],
    );
    assert!(
        diagnostics.is_empty(),
        "hiding W109 must still remove derived findings: {diagnostics:?}"
    );
}

/// The document's own analysis, through the server's ingress, for the
/// XC findings that walk it.
async fn irules_document_analysis(src: &str) -> Arc<AnalysisResult> {
    let backend = test_backend();
    let uri = Uri::from_str("file:///xc-findings.irul").unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(src.to_owned(), "f5-irules".to_owned()),
    );
    backend
        .db_set_source(&uri, src, "f5-irules".to_owned())
        .await
        .expect("the fixture retains its actual checked SourceFile owner");
    backend
        .analysis_for(&uri, Arc::from(src), "f5-irules".to_owned())
        .await
        .expect("the fixture analyses its actual current source and full input")
}

/// `xc_findings` surfaces the XC translatability codes for an iRule, and
/// the report hides those the editor disabled.
#[tokio::test]
async fn xc_findings_surface_and_the_report_filters_codes() {
    let src = "when HTTP_REQUEST {\n    pool my_pool\n}";
    let analysis = irules_document_analysis(src).await;
    let diags = lift_report(
        src,
        &core_policy::apply(
            xc_findings(src, &analysis),
            &core_policy::PolicyBuilder::new().build(),
        ),
    );
    let codes = diag_codes(&diags);
    assert!(
        codes.iter().any(|c| c == "XC100"),
        "expected XC100, got: {codes:?}",
    );
    assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));

    // Disabling XC100 drops it from the lifted set.
    let policy = core_policy::PolicyBuilder::new()
        .layer(
            core_policy::PolicyLayer::Editor,
            &serde_json::json!({ "diagnostics": { "XC100": false } }),
        )
        .build();
    let filtered = lift_report(
        src,
        &core_policy::apply(xc_findings(src, &analysis), &policy),
    );
    assert!(
        !diag_codes(&filtered).iter().any(|c| c == "XC100"),
        "XC100 should be filtered when disabled",
    );
}

/// An analysis of other text states no XC findings: the translation never
/// falls back to reparsing the buffer under the standalone defaults.
#[tokio::test]
async fn xc_findings_refuse_an_analysis_of_other_source() {
    let src = "when HTTP_REQUEST {\n    pool my_pool\n}";
    let analysis = irules_document_analysis(src).await;
    assert_ne!(
        xc_findings(src, &analysis),
        [] as [tcl_lsp_core::diagnostic_policy::Finding; 0]
    );
    let edited = "when HTTP_REQUEST {\n    pool other_pool\n}";
    assert!(
        xc_findings(edited, &analysis).is_empty(),
        "a stale analysis must not describe the edited buffer",
    );
}

/// Grouped optimisations retain every member, including empty deletions,
/// and cannot be independently applied.
#[test]
fn a_grouped_optimisation_is_never_independently_applicable_issue_2149() {
    let registry = CommandRegistry::build_default();
    // O127 fires on a *computed* single-use assignment.
    let src = "proc p {y} {\n    set x [llength $y]\n    puts $x\n}\n";
    // O127 abstains without a resolved dialect profile (it cannot prove
    // its alias-safety gate under "every dialect at once"), so the
    // fixture must state one.
    let diags =
        tcl_lsp_db::compiler_check_diagnostics_uncached(src, &registry, "tcl8.6", None, None);
    let grouped: Vec<_> = diags
        .optimisations
        .iter()
        .filter(|o| o.group.is_some())
        .collect();
    assert_eq!(
        grouped.len(),
        2,
        "fixture must produce the O127 pair to assert about: {:?}",
        diags.optimisations,
    );
    assert!(
        grouped.iter().any(|o| o.replacement.is_empty()),
        "one member is the delete, with an empty replacement: {grouped:?}",
    );

    // Nothing disabled, no directives: the pair is intact and publishable.
    let mut policy = core_policy::PolicyBuilder::new()
        .dialect(tcl_lsp_core::profile_for_dialect("tcl8.6"))
        .build();
    policy.optimiser = core_policy::OptimiserPolicy::all_on();
    let o127_of = |published: &[tower_lsp_server::ls_types::Diagnostic]| {
        published
                .iter()
                .filter(|d| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "O127"))
                .cloned()
                .collect::<Vec<_>>()
    };
    let published = lift_report(src, &core_policy::apply(compiler_findings(&diags), &policy));
    let o127 = o127_of(&published);
    assert_eq!(o127.len(), 2, "both members are published: {o127:?}");
    for d in &o127 {
        let data = d
            .data
            .as_ref()
            .expect("every member of an intact group carries the group's payload");
        assert!(
            data.get("replacement").is_none(),
            "a grouped member must not advertise its own edit alone — that \
                 is the corrupting half-fix of #2149: {data}",
        );
        assert!(
            data.get("group").is_some(),
            "a grouped member's payload carries its group id (#2123): {data}",
        );
        let edits = data
            .get("edits")
            .and_then(serde_json::Value::as_array)
            .expect("grouped payload carries the group's edits");
        assert_eq!(
            edits.len(),
            2,
            "the payload applies the whole pair, not one half: {data}",
        );
    }

    // A directive over the use site hides that member, so the survivor
    // may not be applied alone: it publishes with no payload at all.
    let mut lines: std::collections::HashMap<i32, HashSet<String>> =
        std::collections::HashMap::new();
    lines.insert(2, std::iter::once("*".to_owned()).collect());
    policy.directives = core_policy::Directives::new(lines, src);
    let published = lift_report(src, &core_policy::apply(compiler_findings(&diags), &policy));
    let o127 = o127_of(&published);
    assert_eq!(o127.len(), 1, "one member survives the directive: {o127:?}");
    assert!(
        o127[0].data.is_none(),
        "a group that lost a member carries no payload: {:?}",
        o127[0].data
    );
}

/// Issue #2121: every XC code a published diagnostic carries must be a
/// known [`DiagCode`], because the report reads every code-table fact
/// from it — the per-code switch, the severity override, the
/// `DiagnosticTag` — and a code outside the table could answer none of
/// them. Before the fix `DiagCode::from_str("XC100")` was `Err`, so this
/// assertion failed on the very first lifted diagnostic.
#[tokio::test]
async fn published_xc_codes_are_known_diag_codes_issue_2121() {
    use core::str::FromStr as _;
    use tcl_core_types::{DiagCode, DiagSection};

    // An iRule exercising a translated, a partial and an untranslatable
    // construct, so the lifted set spans XC1xx / XC2xx / XC3xx.
    let src = concat!(
        "when HTTP_REQUEST {\n",
        "    pool my_pool\n",
        "    if { [HTTP::uri] contains [expr {rand()}] } { pool other }\n",
        "    while { 1 } { pool third }\n",
        "}\n",
        "when CLIENT_ACCEPTED {\n",
        "    TCP::collect\n",
        "}\n",
    );
    let analysis = irules_document_analysis(src).await;
    let diagnostics = lift_report(
        src,
        &core_policy::apply(
            xc_findings(src, &analysis),
            &core_policy::PolicyBuilder::new().build(),
        ),
    );
    assert!(
        !diagnostics.is_empty(),
        "the fixture must produce XC diagnostics to assert about",
    );
    // The report resolves each code's tag from the code table; it must be
    // able to resolve every code it sees here.
    for code in diag_codes(&diagnostics) {
        let parsed = DiagCode::from_str(&code)
            .unwrap_or_else(|_| panic!("published XC code {code} is outside the DiagCode table"));
        assert_eq!(
            parsed.diag_section(),
            Some(DiagSection::Xc),
            "{code} must sit in the xc section",
        );
    }
}

/// The opt-in `xcDiagnostics` toggle gates whether XC100-301 reach the
/// `f5-irules` diagnostics pull path: off by default, present once
/// enabled.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn xc_diagnostics_are_opt_in_on_irules_documents() {
    let uri = Uri::from_str("file:///rule.irul").unwrap();
    let src = "when HTTP_REQUEST {\n    pool my_pool\n}";

    // Default-off: no XC codes surface.
    let backend = test_backend();
    let off = backend
        .full_diagnostics_for(&uri, Arc::from(src), "f5-irules".to_owned(), "tcl-irule")
        .await
        .unwrap();
    assert!(
        !diag_codes(&off).iter().any(|c| c.starts_with("XC")),
        "XC diagnostics must be off by default, got: {:?}",
        diag_codes(&off),
    );

    // Opt in via the `xcDiagnostics` feature toggle.
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "xcDiagnostics": true })
            .as_object()
            .unwrap(),
    );
    for dialect in ["f5-irules", "irules", "tcl-irule"] {
        let on = backend
            .full_diagnostics_for(&uri, Arc::from(src), dialect.to_owned(), "tcl-irule")
            .await
            .unwrap();
        assert!(
            diag_codes(&on).iter().any(|c| c == "XC100"),
            "{dialect}: expected XC100 once xcDiagnostics is enabled, got: {:?}",
            diag_codes(&on),
        );
    }
    let tcl = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl9.0".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !diag_codes(&tcl).iter().any(|c| c.starts_with("XC")),
        "plain Tcl must not receive XC diagnostics: {:?}",
        diag_codes(&tcl),
    );
}

/// A lone `\r` is a command terminator in a file on disk — `tclsh` opens
/// scripts with `-translation auto`, which rewrites every `\r` to `\n`
/// before the parser runs.  Treating it as horizontal whitespace made
/// `set zz 1` part of the preceding comment, so `puts $zz` drew a bogus
/// W210 "read before it is set".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn diagnostics_treat_a_lone_cr_as_a_command_terminator() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///oldmac.tcl").unwrap();
    let src = "# note \rset zz 1\nputs $zz\n";
    register(&backend, &uri, src).await;
    backend
        .db_set_source(&uri, src, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .publish_analyser_diagnostics(uri.clone(), src.to_owned(), "tcl8.6".to_owned(), 0, Some(1))
        .await;
    let cache = backend.pull_diag_cache.lock().await;
    let entry = cache.get(&uri).expect("pull cache entry");
    let codes: Vec<&str> = entry
        .diagnostics
        .iter()
        .filter_map(|d| match &d.code {
            Some(tower_lsp_server::ls_types::NumberOrString::String(c)) => Some(c.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        !codes.contains(&"W210"),
        "`set zz 1` is a command, so `zz` is set before it is read: {codes:?}",
    );
    // W118 still sees the document's real terminators.
    assert!(codes.contains(&"W118"), "{codes:?}");
}

/// The deep push and the pull publish one set: both assemble it through
/// `published_findings`, whose XC walk reads the analysis form, so the
/// lone `\r` that ends the comment ends it for both, and the `pool`
/// command after it draws XC100 on either delivery.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_deep_push_and_the_pull_publish_one_set_on_a_lone_cr_irule() {
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({ "xcDiagnostics": { "enabled": true } }))
        .await
        .unwrap();
    let uri = Uri::from_str("file:///oldmac-xc.irul").unwrap();
    let src = "when HTTP_REQUEST {\n    # note \r    pool my_pool\n}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(src.to_owned(), "f5-irules".to_owned())
            .with_language_id("tcl-irule".to_owned()),
    );
    backend
        .db_set_source(&uri, src, "f5-irules".to_owned())
        .await
        .unwrap();
    backend
        .publish_analyser_diagnostics(
            uri.clone(),
            src.to_owned(),
            "f5-irules".to_owned(),
            0,
            Some(1),
        )
        .await;
    let pushed = backend
        .pull_diag_cache
        .lock()
        .await
        .get(&uri)
        .expect("the deep push primes the pull cache")
        .diagnostics
        .clone();
    assert!(
        diag_codes(&pushed).iter().any(|code| code == "XC100"),
        "`pool` after the lone `\\r` is a command on the push: {:?}",
        diag_codes(&pushed),
    );
    let pulled = backend
        .full_diagnostics_for(&uri, Arc::from(src), "f5-irules".to_owned(), "tcl-irule")
        .await
        .unwrap();
    assert_eq!(pushed, pulled, "the deep push and the pull disagree");
}

/// The style lints are line-oriented, so on an old-Mac document their line
/// numbers must follow the client's (and the analyser's) line model, not a
/// `\n`-only split that would collapse the file to one line.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn style_diagnostics_line_numbers_follow_the_client_model_on_a_cr_document() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///oldmac-style.tcl").unwrap();
    let src = "set a 1\rset b 2   \rputs \"$a$b\"\r";
    register(&backend, &uri, src).await;
    backend
        .publish_analyser_diagnostics(uri.clone(), src.to_owned(), "tcl8.6".to_owned(), 0, Some(1))
        .await;
    let cache = backend.pull_diag_cache.lock().await;
    let entry = cache.get(&uri).expect("pull cache entry");
    let w112 = entry
            .diagnostics
            .iter()
            .find(|d| {
                matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W112")
            })
            .expect("trailing whitespace on the second line");
    assert_eq!(w112.range.start.line, 1, "{:?}", w112.range);
}

/// A disabled fact code is computed and suppressed, never skipped:
/// the session's analyser skip leaves W100 out, while `getEffectiveConfig`
/// and the INI export still list it as disabled — they report what the
/// configuration turns off, not what the analyser skips.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_disabled_fact_code_is_reported_disabled_but_computed() {
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({ "diagnostics": { "W100": false } }))
        .await
        .unwrap();
    assert!(
        !backend.disabled_diagnostics.lock().await.contains("W100"),
        "the analyser computes W100 for O111",
    );
    let effective = backend
        .get_effective_config_command(&[serde_json::json!("file:///fact.tcl")])
        .await
        .expect("effective config")
        .expect("config payload");
    assert_eq!(
        effective["disabled_diagnostics"],
        serde_json::json!(["W100", "W242"]),
        "{effective}"
    );
    assert!(backend.render_config_ini().await.contains("W100 = false"));
}

#[tokio::test]
async fn o111_brace_expr_hint_pairs_with_w100() {
    // An O111 "brace your expression" Information hint is paired with
    // every W100; the pairing is gated on the optimiser being enabled and
    // O111 not being disabled.
    let backend = test_backend();
    let uri = Uri::from_str("file:///o111.tcl").unwrap();
    let src = "set y [expr $a + $b]\n"; // unbraced expr → W100
    register(&backend, &uri, src).await;

    let on = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    let codes = |ds: &[tower_lsp_server::ls_types::Diagnostic]| -> Vec<String> {
        ds.iter()
            .filter_map(|d| match &d.code {
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) => Some(c.clone()),
                _ => None,
            })
            .collect()
    };
    let on_codes = codes(&on);
    assert!(
        on_codes.iter().any(|c| c == "W100"),
        "expected W100: {on_codes:?}"
    );
    assert!(
        on_codes.iter().any(|c| c == "O111"),
        "expected the paired O111 hint: {on_codes:?}",
    );

    // Disabling the optimiser drops the O111 hint but keeps W100.
    backend
        .apply_global_config(&serde_json::json!({ "optimiser": { "enabled": false } }))
        .await
        .unwrap();
    let off = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    let off_codes = codes(&off);
    assert!(
        off_codes.iter().any(|c| c == "W100"),
        "W100 still expected: {off_codes:?}"
    );
    assert!(
        !off_codes.iter().any(|c| c == "O111"),
        "O111 must be gated off when the optimiser is disabled: {off_codes:?}",
    );
}

/// Disabling W100 does not silence O111: the analyser computes
/// W100, a fact code, and the policy step decides the two on their own —
/// a layer's `W100 = false` or a `# noqa: W100` hides W100 and leaves
/// the O111 over the same expression.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn o111_survives_a_disabled_w100() {
    let codes = |ds: &[tower_lsp_server::ls_types::Diagnostic]| -> Vec<String> {
        ds.iter()
            .filter_map(|d| match &d.code {
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) => Some(c.clone()),
                _ => None,
            })
            .collect()
    };
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({ "diagnostics": { "W100": false } }))
        .await
        .unwrap();
    let uri = Uri::from_str("file:///o111-disabled.tcl").unwrap();
    let src = "set y [expr $a + $b]\n";
    register(&backend, &uri, src).await;
    let disabled = codes(
        &backend
            .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
            .await
            .unwrap(),
    );
    assert!(
        disabled.iter().any(|c| c == "O111"),
        "O111 stands: {disabled:?}"
    );
    assert!(
        !disabled.iter().any(|c| c == "W100"),
        "W100 is off: {disabled:?}"
    );

    let backend = test_backend();
    let uri = Uri::from_str("file:///o111-noqa.tcl").unwrap();
    let src = "# noqa: W100\nset y [expr $a + $b]\n";
    register(&backend, &uri, src).await;
    let marked = codes(
        &backend
            .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
            .await
            .unwrap(),
    );
    assert!(
        marked.iter().any(|c| c == "O111"),
        "O111 stands: {marked:?}"
    );
    assert!(
        !marked.iter().any(|c| c == "W100"),
        "W100 is silenced: {marked:?}"
    );
}

#[tokio::test]
async fn style_line_length_setting_drives_w111() {
    // `tclLsp.style.lineLength` is the W111 threshold, distinct from the
    // formatter width. A short threshold makes an otherwise-fine line long.
    let backend = test_backend();
    let uri = Uri::from_str("file:///w111.tcl").unwrap();
    let src = "set x 1234567890\n"; // 16 chars on line 0
    register(&backend, &uri, src).await;

    // Default (120): no W111.
    let none = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
            !none
                .iter()
                .any(|d| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W111")),
            "no W111 at the default threshold: {none:?}",
        );

    // Lower the style threshold to 8 → the 16-char line now trips W111.
    backend
        .apply_global_config(&serde_json::json!({ "style": { "lineLength": 8 } }))
        .await
        .unwrap();
    assert_eq!(*backend.style_line_length.lock().await, 8);
    let some = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
            some.iter()
                .any(|d| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W111")),
            "W111 should fire once the style line length is lowered: {some:?}",
        );
}
