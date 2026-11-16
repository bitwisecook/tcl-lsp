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

use super::*;
use crate::source_style::{DEFAULT_LINE_ENDING, DEFAULT_LINE_LENGTH, style_diagnostics};
use serde_json::json;

fn finding(code: DiagCode, span: Span, producer: Producer) -> Finding {
    Finding {
        code,
        span,
        severity: Severity::Warning,
        message: format!("{code}"),
        fixes: Vec::new(),
        data: None,
        producer,
    }
}

fn analyser(code: DiagCode, start: u32, end: u32) -> Finding {
    finding(code, Span::new(start, end), Producer::Analyser)
}

fn outcomes(report: &Report) -> Vec<(DiagCode, Outcome)> {
    report.iter().map(|(f, o)| (f.code, *o)).collect()
}

fn shown_codes(report: &Report) -> Vec<DiagCode> {
    report.shown().map(|s| s.finding.code).collect()
}

fn tcl9() -> &'static DialectProfile {
    crate::profile_for_dialect("tcl9.0")
}

/// A group's edits apply all-or-nothing (#2149): once a directive hides
/// one member of an O127 pair, the other is neither offered nor applied —
/// the inline without its delete runs the assignment twice.
#[test]
fn a_group_that_lost_a_member_is_neither_offered_nor_applied() {
    let text = "proc p {y} {\n    set x [llength $y]\n    # noqa\n    puts $x\n}\n";
    let at = |needle: &str| {
        let start = u32::try_from(text.find(needle).expect(needle)).unwrap();
        Span::new(start, start + u32::try_from(needle.len()).unwrap())
    };
    let mut delete = Optimisation::new(
        DiagCode::O127,
        "Inline the single-use assignment",
        at("set x [llength $y]"),
        "",
    );
    delete.group = Some(7);
    let mut inline = Optimisation::new(
        DiagCode::O127,
        "Inline the single-use assignment",
        at("$x"),
        "[llength $y]",
    );
    inline.group = Some(7);
    let findings = || vec![Finding::from(delete.clone()), Finding::from(inline.clone())];

    let mut lines: HashMap<i32, HashSet<String>> = HashMap::new();
    lines.insert(3, std::iter::once("*".to_owned()).collect());
    let directed = Policy {
        optimiser: OptimiserPolicy::all_on(),
        directives: Directives::new(lines, text),
        ..Policy::default()
    };
    let report = apply(findings(), &directed);
    assert_eq!(
        shown_codes(&report),
        vec![DiagCode::O127],
        "the directive hides the member on line 3 alone"
    );
    assert!(report.applicable_rewrites().is_empty());
    assert_eq!(report.applicable_items(vec![0, 1]), [] as [i32; 0]);

    let plain = Policy {
        optimiser: OptimiserPolicy::all_on(),
        ..Policy::default()
    };
    let report = apply(findings(), &plain);
    let rewrites = report.applicable_rewrites();
    assert_eq!(rewrites.len(), 1, "{rewrites:?}");
    assert_eq!(rewrites[0].group, Some(7));
    assert_eq!(rewrites[0].members.len(), 2);
    assert_eq!(report.applicable_items(vec![0, 1]), vec![0, 1]);
}

#[test]
fn reporting_off_then_excluded_hide_the_whole_document_with_a_reason() {
    let findings = vec![
        analyser(DiagCode::W210, 0, 3),
        analyser(DiagCode::E002, 4, 8),
    ];
    let off = Policy {
        document: DocumentGates {
            reporting: false,
            excluded: true,
            abstain: false,
        },
        ..Policy::default()
    };
    let report = apply(findings.clone(), &off);
    assert_eq!(report.len(), 2, "nothing is deleted");
    assert!(
        report
            .iter()
            .all(|(_, o)| *o == Outcome::Suppressed(Reason::ReportingOff))
    );
    let excluded = Policy {
        document: DocumentGates {
            excluded: true,
            ..DocumentGates::default()
        },
        ..Policy::default()
    };
    let report = apply(findings, &excluded);
    assert!(
        report
            .iter()
            .all(|(_, o)| *o == Outcome::Suppressed(Reason::Excluded))
    );
}

#[test]
fn abstention_keeps_the_integrity_codes_and_beats_a_directive() {
    let text = "# noqa\nset x 1   \n";
    let findings = vec![
        finding(DiagCode::W109, Span::new(0, 0), Producer::SourceDecode),
        finding(DiagCode::W107, Span::new(0, 3), Producer::SourceDecode),
        analyser(DiagCode::W305, 0, 1),
        finding(DiagCode::W112, Span::new(14, 17), Producer::SourceStyle),
        analyser(DiagCode::W210, 7, 10),
    ];
    let policy = Policy {
        document: DocumentGates {
            abstain: true,
            ..DocumentGates::default()
        },
        directives: Directives::scan(text, tcl9()),
        ..Policy::default()
    };
    let report = apply(findings, &policy);
    assert_eq!(
        shown_codes(&report),
        vec![DiagCode::W109, DiagCode::W107, DiagCode::W305]
    );
    // The `# noqa` covers line 1 too, but abstention is the reason.
    assert_eq!(
        report.reason_for(DiagCode::W210, Span::new(7, 10)),
        Some(Reason::EncodingAbstention)
    );
    assert_eq!(
        report.reason_for(DiagCode::W112, Span::new(14, 17)),
        Some(Reason::EncodingAbstention)
    );
}

#[test]
fn directives_beat_the_layers_and_the_inline_one_is_reported_first() {
    // Line 0 carries the file directive for W112; line 2 (`puts $y`)
    // carries an inline `# noqa` for W210; the project layer enables
    // both codes and the global layer disables W111.
    let text = "# tcl-lsp: disable=W112\n# noqa: W210\nputs $y\nset z 1   \n";
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Global,
            &json!({"diagnostics": {"W111": false}}),
        )
        .layer(
            PolicyLayer::Project,
            &json!({"diagnostics": {"W210": true, "W112": true}}),
        )
        .directives(Directives::scan(text, tcl9()))
        .build();
    let findings = vec![
        analyser(DiagCode::W210, 37, 44),
        finding(DiagCode::W112, Span::new(52, 55), Producer::SourceStyle),
        finding(DiagCode::W111, Span::new(45, 55), Producer::SourceStyle),
    ];
    let report = apply(findings, &policy);
    assert_eq!(
        outcomes(&report),
        vec![
            (
                DiagCode::W210,
                Outcome::Suppressed(Reason::InlineDirective { line: 2 })
            ),
            (DiagCode::W112, Outcome::Suppressed(Reason::FileDirective)),
            (
                DiagCode::W111,
                Outcome::Suppressed(Reason::Disabled(PolicyLayer::Global))
            ),
        ]
    );
}

#[test]
fn a_layer_names_itself_and_the_seed_reports_default_off() {
    let findings = vec![
        analyser(DiagCode::W242, 0, 3),
        analyser(DiagCode::W211, 4, 8),
    ];
    let seeded = apply(findings.clone(), &Policy::default());
    assert_eq!(
        seeded.reason_for(DiagCode::W242, Span::new(0, 3)),
        Some(Reason::DefaultOff)
    );
    assert_eq!(seeded.reason_for(DiagCode::W211, Span::new(4, 8)), None);
    for layer in [
        PolicyLayer::Global,
        PolicyLayer::Editor,
        PolicyLayer::Project,
    ] {
        let policy = PolicyBuilder::new()
            .layer(
                layer,
                &json!({"diagnostics": {"W242": true, "W211": false}}),
            )
            .build();
        let report = apply(findings.clone(), &policy);
        assert_eq!(shown_codes(&report), vec![DiagCode::W242], "{layer:?}");
        assert_eq!(
            report.reason_for(DiagCode::W211, Span::new(4, 8)),
            Some(Reason::Disabled(layer))
        );
    }
    // The project turns back on what the global file disabled.
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Global,
            &json!({"diagnostics": {"W211": false}}),
        )
        .layer(
            PolicyLayer::Project,
            &json!({"diagnostics": {"W211": true}}),
        )
        .build();
    assert_eq!(shown_codes(&apply(findings, &policy)), vec![DiagCode::W211]);
}

#[test]
fn the_optimiser_gates_fire_in_order_and_after_a_directive() {
    let text = "set x 1\n# noqa: O109\nset y 2\n";
    let rewrite = |code: DiagCode, start: u32, end: u32| {
        Finding::from(Optimisation::new(
            code,
            "rewrite",
            Span::new(start, end),
            "x",
        ))
    };
    let findings = vec![
        rewrite(DiagCode::O114, 0, 7),
        rewrite(DiagCode::O109, 21, 28),
        analyser(DiagCode::W210, 0, 7),
    ];
    let off = Policy {
        optimiser: OptimiserPolicy {
            enabled: false,
            ..OptimiserPolicy::for_profile(OptimisationProfile::Full)
        },
        ..Policy::default()
    };
    let report = apply(findings.clone(), &off);
    assert_eq!(
        report.reason_for(DiagCode::O114, Span::new(0, 7)),
        Some(Reason::OptimiserOff)
    );
    assert_eq!(shown_codes(&report), vec![DiagCode::W210]);

    let readability = Policy {
        optimiser: OptimiserPolicy::for_profile(OptimisationProfile::Readability),
        directives: Directives::scan(text, tcl9()),
        ..Policy::default()
    };
    let report = apply(findings, &readability);
    assert_eq!(
        report.reason_for(DiagCode::O114, Span::new(0, 7)),
        None,
        "a readability rewrite shows under the readability profile"
    );
    assert_eq!(
        report.reason_for(DiagCode::O109, Span::new(21, 28)),
        Some(Reason::InlineDirective { line: 2 }),
        "the directive is reported ahead of the profile"
    );
    let no_directive = Policy {
        directives: Directives::none(),
        ..readability
    };
    let report = apply(
        vec![Finding::from(Optimisation::new(
            DiagCode::O109,
            "dead store",
            Span::new(21, 28),
            "",
        ))],
        &no_directive,
    );
    assert_eq!(
        report.reason_for(DiagCode::O109, Span::new(21, 28)),
        Some(Reason::OptimiserProfile {
            profile: OptimisationProfile::Readability
        })
    );
}

#[test]
fn the_shimmer_switch_hides_the_family_only() {
    let findings = vec![
        finding(DiagCode::S100, Span::new(0, 3), Producer::CompilerCheck),
        finding(DiagCode::S110, Span::new(4, 8), Producer::CompilerCheck),
        finding(DiagCode::T100, Span::new(9, 12), Producer::CompilerCheck),
    ];
    let policy = Policy {
        shimmer: false,
        ..Policy::default()
    };
    let report = apply(findings, &policy);
    assert_eq!(shown_codes(&report), vec![DiagCode::T100]);
    assert_eq!(
        report.reason_for(DiagCode::S100, Span::new(0, 3)),
        Some(Reason::ShimmerOff)
    );
}

#[test]
fn w110_owns_an_o120_whose_span_holds_it() {
    let w110 = analyser(DiagCode::W110, 10, 12);
    let o120_at = |start: u32, end: u32| {
        Finding::from(Optimisation::new(
            DiagCode::O120,
            "eq",
            Span::new(start, end),
            "eq",
        ))
    };
    // The analyser marks the operator, the optimiser the condition.
    let o120 = o120_at(4, 20);
    let same = o120_at(10, 12);
    let across = o120_at(11, 20);
    let elsewhere = o120_at(30, 32);
    let policy = Policy {
        optimiser: OptimiserPolicy::all_on(),
        ..Policy::default()
    };
    let owned = Some(Reason::Overlap {
        owner: OverlapOwner::Code(DiagCode::W110),
    });
    let report = apply(
        vec![w110.clone(), o120.clone(), same, across, elsewhere],
        &policy,
    );
    assert_eq!(report.reason_for(DiagCode::O120, Span::new(4, 20)), owned);
    assert_eq!(report.reason_for(DiagCode::O120, Span::new(10, 12)), owned);
    assert_eq!(
        report.reason_for(DiagCode::O120, Span::new(11, 20)),
        None,
        "W110 does not lie inside a span that starts after it"
    );
    assert_eq!(report.reason_for(DiagCode::O120, Span::new(30, 32)), None);
    // An owner that policy suppressed cannot supersede anything.
    let mut disabled = Policy {
        optimiser: OptimiserPolicy::all_on(),
        ..Policy::default()
    };
    disabled.codes.insert(
        DiagCode::W110,
        CodeDecision {
            enabled: false,
            layer: PolicyLayer::Editor,
        },
    );
    let report = apply(vec![w110, o120], &disabled);
    assert_eq!(shown_codes(&report), vec![DiagCode::O120]);
}

#[test]
fn a_same_span_overlap_claims_only_the_span_it_shares() {
    let owner = analyser(DiagCode::W110, 10, 12);
    let superseded = |start: u32, end: u32| {
        Finding::from(Optimisation::new(
            DiagCode::O120,
            "eq",
            Span::new(start, end),
            "eq",
        ))
    };
    let policy = Policy {
        optimiser: OptimiserPolicy::all_on(),
        overlaps: vec![Overlap {
            owner: OverlapOwner::Code(DiagCode::W110),
            superseded: DiagCode::O120,
            scope: OverlapScope::SameSpan,
        }],
        ..Policy::default()
    };
    let report = apply(vec![owner, superseded(10, 12), superseded(4, 20)], &policy);
    assert_eq!(
        report.reason_for(DiagCode::O120, Span::new(10, 12)),
        Some(Reason::Overlap {
            owner: OverlapOwner::Code(DiagCode::W110)
        })
    );
    assert_eq!(
        report.reason_for(DiagCode::O120, Span::new(4, 20)),
        None,
        "a span that only holds the owner's is not the same span"
    );
}

#[test]
fn a_producer_owns_the_document_without_a_finding_of_its_own() {
    let policy = Policy {
        overlaps: dialect_overlaps(crate::profile_for_dialect("sslictcl")),
        ..Policy::default()
    };
    let findings = vec![
        analyser(DiagCode::W123, 0, 4),
        analyser(DiagCode::E003, 5, 9),
    ];
    let report = apply(findings, &policy);
    assert_eq!(
        report.reason_for(DiagCode::W123, Span::new(0, 4)),
        Some(Reason::Overlap {
            owner: OverlapOwner::Producer(Producer::SslicTcl)
        })
    );
    assert_eq!(shown_codes(&report), vec![DiagCode::E003]);
}

#[test]
fn a_shown_finding_carries_the_override_and_the_tag() {
    let findings = vec![
        analyser(DiagCode::W214, 0, 1),
        analyser(DiagCode::W210, 2, 3),
    ];
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Editor,
            &json!({"diagnosticSeverity": {"W214": "error"}}),
        )
        .build();
    let report = apply(findings, &policy);
    let shown: Vec<(DiagCode, Severity, Option<DiagTag>)> = report
        .shown()
        .map(|s| (s.finding.code, s.severity, s.tag))
        .collect();
    assert_eq!(
        shown,
        vec![
            (DiagCode::W214, Severity::Error, Some(DiagTag::Unnecessary)),
            (DiagCode::W210, Severity::Warning, None),
        ]
    );
}

#[test]
fn the_wildcard_reaches_both_directive_spellings() {
    let inline = "# noqa\nset x 1   \n";
    let report = apply(
        vec![
            analyser(DiagCode::W210, 7, 10),
            finding(DiagCode::W112, Span::new(14, 17), Producer::SourceStyle),
        ],
        &Policy {
            directives: Directives::scan(inline, tcl9()),
            ..Policy::default()
        },
    );
    assert!(report.shown().next().is_none(), "{:?}", outcomes(&report));
    let file = "# tcl-lsp: disable=*\nset x 1   \n";
    let report = apply(
        vec![
            analyser(DiagCode::W210, 21, 24),
            finding(DiagCode::W118, Span::new(0, 0), Producer::SourceStyle),
        ],
        &Policy {
            directives: Directives::scan(file, tcl9()),
            ..Policy::default()
        },
    );
    assert!(
        report
            .iter()
            .all(|(_, o)| *o == Outcome::Suppressed(Reason::FileDirective))
    );
}

#[test]
fn the_style_pass_through_apply_keeps_the_orchestrator_rules() {
    // What the style pass used to decide for itself: a line-0 `*`
    // hides the line-0 W112 but never the file-level W118; a file
    // directive hides a named code; a disabled set hides a code.
    let src = "set x 1   \r\n";
    let style = style_diagnostics(src, DEFAULT_LINE_LENGTH, DEFAULT_LINE_ENDING, None, tcl9());
    let codes: Vec<DiagCode> = style.iter().map(|d| d.code).collect();
    assert_eq!(codes, vec![DiagCode::W112, DiagCode::W118]);
    let line_index = LineIndex::new_lsp(src);
    let findings = |style: &[StyleDiagnostic]| -> Vec<Finding> {
        style
            .iter()
            .cloned()
            .map(|d| Finding::from_style(d, src, &line_index))
            .collect()
    };

    let mut inline: HashMap<i32, HashSet<String>> = HashMap::new();
    inline.insert(0, std::iter::once("*".to_owned()).collect());
    let policy = Policy {
        directives: Directives::new(inline, src),
        ..Policy::default()
    };
    let report = apply(findings(&style), &policy);
    assert_eq!(shown_codes(&report), vec![DiagCode::W118]);

    let mut file: HashMap<i32, HashSet<String>> = HashMap::new();
    file.insert(
        FILE_SUPPRESS_KEY,
        std::iter::once("W112".to_owned()).collect(),
    );
    let policy = Policy {
        directives: Directives::new(file, src),
        ..Policy::default()
    };
    assert_eq!(
        shown_codes(&apply(findings(&style), &policy)),
        vec![DiagCode::W118]
    );

    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Editor,
            &json!({"diagnostics": {"W118": false}}),
        )
        .build();
    let report = apply(findings(&style), &policy);
    assert_eq!(shown_codes(&report), vec![DiagCode::W112]);
    assert_eq!(
        report.reason_for(DiagCode::W118, Span::new(0, 0)),
        Some(Reason::Disabled(PolicyLayer::Editor))
    );
}

#[test]
fn the_report_keeps_the_producers_order() {
    let findings = vec![
        analyser(DiagCode::W211, 9, 10),
        analyser(DiagCode::W210, 0, 1),
        analyser(DiagCode::E002, 5, 6),
    ];
    let expected: Vec<DiagCode> = findings.iter().map(|f| f.code).collect();
    let report = apply(findings, &Policy::default());
    let got: Vec<DiagCode> = report.iter().map(|(f, _)| f.code).collect();
    assert_eq!(got, expected);
}

#[test]
fn code_reason_is_the_per_code_decision_and_the_family_gates() {
    let mut policy = Policy::default();
    assert_eq!(policy.code_reason(DiagCode::W210), None);
    assert_eq!(policy.code_reason(DiagCode::W242), Some(Reason::DefaultOff));
    policy.codes.insert(
        DiagCode::W210,
        CodeDecision {
            enabled: false,
            layer: PolicyLayer::Project,
        },
    );
    assert_eq!(
        policy.code_reason(DiagCode::W210),
        Some(Reason::Disabled(PolicyLayer::Project))
    );
    policy.optimiser.enabled = false;
    assert_eq!(
        policy.code_reason(DiagCode::O100),
        Some(Reason::OptimiserOff)
    );
    policy.shimmer = false;
    assert_eq!(policy.code_reason(DiagCode::S100), Some(Reason::ShimmerOff));
}

#[test]
fn production_skip_is_the_per_code_decision_and_the_seed() {
    let mut policy = Policy::default();
    policy.codes.insert(
        DiagCode::W210,
        CodeDecision {
            enabled: false,
            layer: PolicyLayer::Global,
        },
    );
    policy.shimmer = false;
    let skip = policy.production_skip();
    assert!(skip.contains(&DiagCode::W210));
    assert!(
        skip.contains(&DiagCode::W242),
        "the default-off seed is skipped"
    );
    assert!(!skip.contains(&DiagCode::W100));
    // The family gates hide these codes, but no producer that honours a
    // skip emits them, so nothing is skipped on their account.
    assert_eq!(
        policy.code_reason(DiagCode::O107),
        Some(Reason::OptimiserProfile {
            profile: DEFAULT_EDITOR_PROFILE
        }),
        "the default readability profile hides O107"
    );
    assert!(!skip.contains(&DiagCode::O107));
    assert_eq!(policy.code_reason(DiagCode::S100), Some(Reason::ShimmerOff));
    assert!(!skip.contains(&DiagCode::S100));
}

#[test]
fn production_skip_never_skips_a_fact_code() {
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Editor,
            &json!({ "diagnostics": { "W100": false, "W210": false } }),
        )
        .build();
    let skip = policy.production_skip();
    assert!(!skip.contains(&DiagCode::W100), "W100 is read as a fact");
    assert!(skip.contains(&DiagCode::W210));
    assert_eq!(
        policy.code_reason(DiagCode::W100),
        Some(Reason::Disabled(PolicyLayer::Editor)),
        "the policy step still hides it"
    );
    assert!(policy.disabled_codes().contains(&DiagCode::W100));
    // A top-of-file directive the analyser folds is the analyser's own
    // skip, fact code or not.
    let text = "# tcl-lsp: disable=W100\nputs ok\n";
    let directed = Policy {
        directives: Directives::scan(text, tcl9()),
        ..Policy::default()
    };
    assert!(directed.analyser_skip().contains(&DiagCode::W100));
}

#[test]
fn a_gap_reason_follows_the_step_order() {
    let mut abstaining = Policy::default();
    abstaining.document.abstain = true;
    assert_eq!(
        abstaining.gap_reason(DiagCode::W210),
        Some(Reason::EncodingAbstention)
    );
    assert_eq!(
        abstaining.gap_reason(DiagCode::W109),
        None,
        "an integrity code survives abstention"
    );

    let text = "# tcl-lsp: disable=W210\nputs $y\n";
    let mut directed = Policy {
        directives: Directives::scan(text, tcl9()),
        ..Policy::default()
    };
    directed.codes.insert(
        DiagCode::W210,
        CodeDecision {
            enabled: false,
            layer: PolicyLayer::Global,
        },
    );
    assert_eq!(
        directed.gap_reason(DiagCode::W210),
        Some(Reason::FileDirective),
        "the file directive is step 3, ahead of the per-code decision"
    );
    assert_eq!(
        directed.gap_reason(DiagCode::W242),
        Some(Reason::DefaultOff)
    );

    directed.document.reporting = false;
    directed.document.abstain = true;
    assert_eq!(
        directed.gap_reason(DiagCode::W210),
        Some(Reason::ReportingOff),
        "reporting off answers before everything"
    );
}

#[test]
fn the_analyser_skip_adds_the_codes_the_file_directive_names() {
    let text = "# tcl-lsp: disable=W210, *\nputs $y\n";
    let policy = Policy {
        directives: Directives::scan(text, tcl9()),
        ..Policy::default()
    };
    let production = policy.production_skip();
    let analyser = policy.analyser_skip();
    assert!(!production.contains(&DiagCode::W210));
    assert!(analyser.contains(&DiagCode::W210));
    let mut expected = production;
    expected.insert(DiagCode::W210);
    assert_eq!(
        analyser, expected,
        "`*` names no code, so it adds nothing to the skip"
    );
}

#[test]
fn a_declared_skip_explains_a_code_no_finding_carries() {
    let mut policy = Policy::default();
    policy.codes.insert(
        DiagCode::W210,
        CodeDecision {
            enabled: false,
            layer: PolicyLayer::Invocation,
        },
    );
    let mut report = apply(Vec::new(), &policy);
    report.declare_skipped([DiagCode::W210, DiagCode::W100], &policy);
    assert_eq!(
        report.reason_for(DiagCode::W210, Span::new(0, 1)),
        Some(Reason::Disabled(PolicyLayer::Invocation))
    );
    assert_eq!(
        report.reason_for(DiagCode::W100, Span::new(0, 1)),
        None,
        "a code the policy would show has no skip to declare"
    );
    assert_eq!(report.skipped().count(), 1);
}

/// A checks-emitted code can carry both a `Disabled` finding and a
/// declared skip (`Policy::production_skip` declares every catalogued
/// code the decision turns off, whichever producer emits it) —
/// `gaps()` renders only the code no finding explains.
#[test]
fn a_gap_is_a_declared_skip_no_finding_explains() {
    let mut policy = Policy::default();
    for code in [DiagCode::W210, DiagCode::T100] {
        policy.codes.insert(
            code,
            CodeDecision {
                enabled: false,
                layer: PolicyLayer::Project,
            },
        );
    }
    let mut report = apply(
        vec![finding(
            DiagCode::T100,
            Span::new(0, 1),
            Producer::CompilerCheck,
        )],
        &policy,
    );
    report.declare_skipped([DiagCode::W210, DiagCode::T100], &policy);
    let gaps: Vec<DiagCode> = report.gaps().map(|(code, _)| code).collect();
    assert_eq!(gaps, vec![DiagCode::W210]);
}

/// The diagnostics surfaces' policy: the optimiser switched off.
fn optimiser_off() -> Policy {
    let mut policy = Policy::default();
    policy.optimiser.enabled = false;
    policy
}

/// The optimiser's declared skip names only the codes the optimiser
/// alone emits: the compiler checks and the O111 producer ran, so O100,
/// O105, O106 and O111 are neither a gap nor in the optimiser's row.
#[test]
fn the_optimiser_skip_leaves_out_the_codes_other_producers_emit() {
    let policy = optimiser_off();
    let mut report = apply(Vec::new(), &policy);
    report.declare_optimiser_skip(&policy);
    let rows = report.not_run();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].producer, Producer::Optimiser);
    assert_eq!(rows[0].reason, Reason::OptimiserOff);
    assert_eq!(rows[0].message(), "optimiser not run on this surface");
    let only_the_optimisers: Vec<DiagCode> = DiagCode::ALL
        .iter()
        .copied()
        .filter(|code| code.is_optimisation() && !PRODUCED_WITHOUT_THE_OPTIMISER.contains(code))
        .collect();
    assert_eq!(rows[0].codes, only_the_optimisers);
    for code in PRODUCED_WITHOUT_THE_OPTIMISER {
        assert!(code.is_optimisation(), "{code}");
        assert_eq!(
            report.reason_for(*code, Span::new(0, 1)),
            None,
            "{code} is computed without the optimiser"
        );
    }
    assert_eq!(report.gaps().count(), 0);
}

/// A producer the surface did not run is one row per reason: a code an
/// earlier step decides — the file directive here — leaves the switch's
/// row for a row of its own, and a code some finding carries is in
/// neither.
#[test]
fn a_producer_not_run_is_one_row_per_reason() {
    let text = "# tcl-lsp: disable=O120\nputs ok\n";
    let policy = Policy {
        directives: Directives::scan(text, tcl9()),
        ..optimiser_off()
    };
    let mut report = apply(
        vec![finding(
            DiagCode::O101,
            Span::new(0, 1),
            Producer::Optimiser,
        )],
        &policy,
    );
    report.declare_optimiser_skip(&policy);
    let rows = report.not_run();
    let row = |reason: Reason| {
        rows.iter()
            .find(|row| row.reason == reason)
            .unwrap_or_else(|| panic!("no {reason} row: {rows:?}"))
    };
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(row(Reason::FileDirective).codes, vec![DiagCode::O120]);
    let switched = &row(Reason::OptimiserOff).codes;
    assert!(!switched.contains(&DiagCode::O120), "{switched:?}");
    assert!(
        !switched.contains(&DiagCode::O101),
        "a code a finding carries is no row's: {switched:?}"
    );
    assert_eq!(
        report.reason_for(DiagCode::O120, Span::new(0, 1)),
        Some(Reason::FileDirective)
    );
}

/// A code declared both by name (the analyser's skip holds every code
/// the file directive names) and as a code of a producer the surface did
/// not run is the producer's, whichever is declared first.
#[test]
fn a_producer_not_run_explains_its_codes_in_either_order() {
    let text = "# tcl-lsp: disable=O120\nputs ok\n";
    let policy = Policy {
        directives: Directives::scan(text, tcl9()),
        ..optimiser_off()
    };
    assert!(policy.analyser_skip().contains(&DiagCode::O120));
    let mut analyser_first = apply(Vec::new(), &policy);
    analyser_first.declare_analyser_skip(&policy);
    analyser_first.declare_optimiser_skip(&policy);
    let mut optimiser_first = apply(Vec::new(), &policy);
    optimiser_first.declare_optimiser_skip(&policy);
    optimiser_first.declare_analyser_skip(&policy);
    assert_eq!(analyser_first, optimiser_first);
    assert!(
        !analyser_first
            .gaps()
            .any(|(code, _)| code == DiagCode::O120),
        "O120 is the optimiser's row's, not a gap of its own"
    );
    assert!(
        analyser_first
            .not_run()
            .iter()
            .any(|row| row.reason == Reason::FileDirective && row.codes == [DiagCode::O120])
    );
}
