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
use crate::definition::LspRange;

#[test]
fn analyser_diagnostic_converts_one_to_one() {
    let d = analyser::Diagnostic::new(
        DiagCode::W210,
        Span::new(4, 9),
        "Variable read before set",
        Severity::Warning,
    )
    .with_fix(analyser::CodeFix::equivalent(
        Span::new(4, 9),
        "x",
        "Rename",
    ));
    let f = Finding::from(d);
    assert_eq!(f.code, DiagCode::W210);
    assert_eq!(f.span, Span::new(4, 9));
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.producer, Producer::Analyser);
    assert_eq!(f.fixes.len(), 1);
    assert_eq!(f.fixes[0].safety, FixSafety::SemanticsEquivalent);
    assert!(f.original_analyser_diagnostic().is_some());
}

#[test]
fn report_retains_typed_subjects_independently_of_diagnostic_presentation() {
    // naming.diagnostic.typed-subject-reporting
    // docs/design/analysis/name-resolution-proofs/diagnostic-typed-subject-reporting.md
    // Problem: Report conversion must retain counted subject units, even when prose changes.
    // This transport control asserts no Native publication or handler execution.
    let key = tcl_registry::native_package::NativePackageNameKey::from_native_units(
        b"Pkg'\0tail",
        tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6),
    );
    let diagnostic = analyser::Diagnostic::new(
        DiagCode::W120,
        Span::new(0, 1),
        "presentation",
        Severity::Hint,
    )
    .with_subject(analyser::DiagnosticSubject::RequiredPackage(key.clone()));
    let expected = crate::diagnostic_subject::diagnostic_subject_data(&diagnostic);
    let mut changed = diagnostic;
    changed.message = "unrelated text with no quoted package".to_owned();
    let report = apply(vec![Finding::from(changed)], &Policy::unrestricted());
    let finding = report.shown().next().unwrap().finding;
    assert_eq!(finding.structured_data(), expected);
    assert_eq!(
        finding
            .original_analyser_diagnostic()
            .unwrap()
            .required_package_key(),
        Some(&key)
    );
}

#[test]
fn compiler_check_replacement_becomes_a_same_span_fix() {
    let d = compiler_checks::Diagnostic {
        span: Span::new(0, 7),
        code: DiagCode::W201,
        source_context: None,
        category: "path".to_owned(),
        taint_subject: None,
        severity: Severity::Warning,
        message: "Use file join".to_owned(),
        replacement: Some("[file join a b]".to_owned()),
        fixes: Vec::new(),
    };
    let f = Finding::from(d);
    assert_eq!(f.producer, Producer::CompilerCheck);
    assert_eq!(f.fixes.len(), 1);
    assert_eq!(f.fixes[0].span, Span::new(0, 7));
    assert_eq!(f.fixes[0].new_text, "[file join a b]");
    assert_eq!(f.fixes[0].safety, FixSafety::RequiresReview);
    assert!(f.original_compiler_diagnostic().is_some());
}

#[test]
fn optimisation_carries_its_rewrite_as_data() {
    let mut o = Optimisation::new(DiagCode::O114, "incr", Span::new(2, 20), "incr x");
    o.group = Some(3);
    let f = Finding::from(o);
    assert_eq!(f.severity, Severity::Hint);
    assert_eq!(f.producer, Producer::Optimiser);
    assert_eq!(
        f.data,
        Some(FindingData::Rewrite {
            replacement: "incr x".to_owned(),
            group: Some(3),
            hint_only: false,
        })
    );
    assert_eq!(f.fixes, [] as [crate::diagnostic_policy::Fix; 0]);
}

#[test]
fn style_ranges_become_byte_spans_and_pick_the_producer_by_code() {
    // `set 😀  ` — the two trailing spaces start at UTF-16 column 6 and
    // byte offset 8.
    let text = "set 😀  \nputs ok\n";
    let line_index = LineIndex::new_lsp(text);
    let range = LspRange {
        start_line: 0,
        start_character: 6,
        end_line: 0,
        end_character: 8,
    };
    let style = StyleDiagnostic {
        range,
        message: "Trailing whitespace".to_owned(),
        severity: StyleSeverity::Hint,
        code: DiagCode::W112,
        fix: Some(StyleFix {
            range,
            new_text: String::new(),
            description: "Remove trailing whitespace".to_owned(),
        }),
    };
    let f = Finding::from_style(style.clone(), text, &line_index);
    assert_eq!(f.span, Span::new(8, 10));
    assert_eq!(f.producer, Producer::SourceStyle);
    assert_eq!(f.severity, Severity::Hint);
    assert_eq!(
        f.data,
        Some(FindingData::StyleFix {
            fix: style.fix.clone().unwrap()
        })
    );
    // The round trip back to the LSP range is exact.
    let start = line_index.position_at_utf16(f.span.start(), text);
    let end = line_index.position_at_utf16(f.span.end(), text);
    assert_eq!((start.line, start.character.get()), (0, 6));
    assert_eq!((end.line, end.character.get()), (0, 8));

    let integrity = StyleDiagnostic {
        range: LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 0,
        },
        message: "not UTF-8".to_owned(),
        severity: StyleSeverity::Warning,
        code: DiagCode::W109,
        fix: None,
    };
    let f = Finding::from_style(integrity, text, &line_index);
    assert_eq!(f.producer, Producer::SourceDecode);
    assert_eq!(f.span, Span::new(0, 0));
}

#[test]
fn config_diagnostic_normalises_its_inclusive_end() {
    let d = ConfigDiagnostic {
        code: "BIGIP6002".to_owned(),
        message: "pool missing".to_owned(),
        severity: DiagSeverity::Warning,
        subject: tcl_bigip::validator::ConfigDiagnosticSubject::IRule,
        range: tcl_bigip::Range::from_offsets(
            "pool my_pool\n",
            &LineIndex::new("pool my_pool\n"),
            5,
            11,
        ),
    };
    let f = Finding::try_from(&d).expect("catalogued code");
    assert_eq!(f.code, DiagCode::Bigip6002);
    assert_eq!(f.span, Span::new(5, 12));
    assert_eq!(f.producer, Producer::BigipModel);
    assert_eq!(f.severity, Severity::Warning);

    let unknown = ConfigDiagnostic {
        code: "BIGIP9999".to_owned(),
        ..d
    };
    assert_eq!(Finding::try_from(&unknown), Err(UnknownDiagCode));
}

fn finding(code: DiagCode, span: Span) -> Finding {
    Finding {
        code,
        span,
        severity: Severity::Warning,
        message: String::new(),
        fixes: Vec::new(),
        data: None,
        producer: Producer::Analyser,
    }
}

#[test]
fn report_views_split_shown_from_suppressed_and_keep_order() {
    let report = Report::new(vec![
        (
            finding(DiagCode::W210, Span::new(0, 1)),
            Outcome::Shown {
                severity: Severity::Error,
                tag: None,
            },
        ),
        (
            finding(DiagCode::W211, Span::new(2, 3)),
            Outcome::Suppressed(Reason::FileDirective),
        ),
        (
            finding(DiagCode::W220, Span::new(4, 5)),
            Outcome::Shown {
                severity: Severity::Warning,
                tag: Some(DiagTag::Unnecessary),
            },
        ),
    ]);
    let shown: Vec<(DiagCode, Severity, Option<DiagTag>)> = report
        .shown()
        .map(|s| (s.finding.code, s.severity, s.tag))
        .collect();
    assert_eq!(
        shown,
        vec![
            (DiagCode::W210, Severity::Error, None),
            (
                DiagCode::W220,
                Severity::Warning,
                Some(DiagTag::Unnecessary)
            ),
        ]
    );
    let suppressed: Vec<(DiagCode, Reason)> =
        report.suppressed().map(|(f, r)| (f.code, r)).collect();
    assert_eq!(suppressed, vec![(DiagCode::W211, Reason::FileDirective)]);
    assert_eq!(
        report.reason_for(DiagCode::W211, Span::new(2, 3)),
        Some(Reason::FileDirective)
    );
    assert_eq!(report.reason_for(DiagCode::W210, Span::new(0, 1)), None);
    assert_eq!(report.outcome_for(DiagCode::W210, Span::new(9, 9)), None);
    assert_eq!(report.len(), 3);
    assert!(!report.is_empty());
}

#[test]
fn every_reason_has_one_stable_spelling() {
    let cases: Vec<(Reason, &str)> = vec![
        (Reason::ReportingOff, "reporting-off"),
        (Reason::Excluded, "excluded"),
        (Reason::EncodingAbstention, "encoding-abstention"),
        (Reason::InlineDirective { line: 4 }, "inline-directive"),
        (Reason::FileDirective, "file-directive"),
        (Reason::Disabled(PolicyLayer::Editor), "disabled:editor"),
        (Reason::DefaultOff, "default-off"),
        (Reason::OptimiserOff, "optimiser-off"),
        (
            Reason::OptimiserProfile {
                profile: OptimisationProfile::Readability,
            },
            "optimiser-profile:readability",
        ),
        (Reason::ShimmerOff, "shimmer-off"),
        (
            Reason::Overlap {
                owner: OverlapOwner::Code(DiagCode::W110),
            },
            "overlap:W110",
        ),
        (
            Reason::Overlap {
                owner: OverlapOwner::Producer(Producer::SslicTcl),
            },
            "overlap:sslictcl",
        ),
    ];
    for (reason, spelling) in cases {
        assert_eq!(reason.to_string(), spelling, "{reason:?}");
    }

    // `Disabled` and `Overlap` render one example above; every layer and
    // producer spelling in full here.
    for (layer, spelling) in [
        (PolicyLayer::Global, "global"),
        (PolicyLayer::Editor, "editor"),
        (PolicyLayer::Invocation, "invocation"),
        (PolicyLayer::Project, "project"),
    ] {
        assert_eq!(layer.as_str(), spelling);
        assert_eq!(
            Reason::Disabled(layer).to_string(),
            format!("disabled:{spelling}")
        );
    }
    for (producer, spelling) in [
        (Producer::Analyser, "analyser"),
        (Producer::CompilerCheck, "compiler-check"),
        (Producer::Optimiser, "optimiser"),
        (Producer::SourceStyle, "source-style"),
        (Producer::SourceDecode, "source-decode"),
        (Producer::SslicTcl, "sslictcl"),
        (Producer::Xc, "xc"),
        (Producer::BigipModel, "bigip-model"),
    ] {
        assert_eq!(producer.as_str(), spelling);
    }
}
