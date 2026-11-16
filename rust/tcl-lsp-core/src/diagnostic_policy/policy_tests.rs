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
use serde_json::json;

#[test]
fn w153_uses_file_and_editor_suppression_policy() {
    let source = "# tcl-lsp: stub db_eval {sql script:body}\n";
    let analysis = analyser::Analyser::new().analyse(source, "tcl8.6");
    let finding = Finding::from(
        analysis
            .diagnostics
            .into_iter()
            .find(|diagnostic| diagnostic.code == DiagCode::W153)
            .expect("unbounded stub emits W153"),
    );
    assert_eq!(
        apply(vec![finding.clone()], &Policy::default())
            .shown()
            .count(),
        1
    );

    let editor = PolicyBuilder::new()
        .layer(
            PolicyLayer::Editor,
            &json!({"diagnostics": {"W153": false}}),
        )
        .build();
    assert_eq!(
        apply(vec![finding.clone()], &editor)
            .suppressed()
            .next()
            .map(|(_, reason)| reason),
        Some(Reason::Disabled(PolicyLayer::Editor)),
    );

    let prefix = "# tcl-lsp: disable=W153\n";
    let directed_source = format!("{prefix}{source}");
    let mut directed_finding = finding;
    let offset = u32::try_from(prefix.len()).unwrap();
    directed_finding.span = Span::new(
        directed_finding.span.start() + offset,
        directed_finding.span.end() + offset,
    );
    let file = PolicyBuilder::new()
        .directives(Directives::scan(
            &directed_source,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        ))
        .build();
    assert_eq!(
        apply(vec![directed_finding], &file)
            .suppressed()
            .next()
            .map(|(_, reason)| reason),
        Some(Reason::FileDirective),
    );
    assert!(
        !analyser::Analyser::new()
            .analyse(&directed_source, "tcl8.6")
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W153),
        "the analyser's file-level skip agrees with the policy report",
    );
}

fn decision(policy: &Policy, code: DiagCode) -> Option<CodeDecision> {
    policy.codes.get(&code).copied()
}

#[test]
fn the_builder_names_the_layer_that_decided_a_code() {
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Global,
            &json!({"diagnostics": {"W111": false}}),
        )
        .layer(PolicyLayer::Editor, &json!({"diagnostics": {"W111": true}}))
        .build();
    assert_eq!(
        decision(&policy, DiagCode::W111),
        Some(CodeDecision {
            enabled: true,
            layer: PolicyLayer::Editor
        }),
        "the editor turns the global file's disable back on"
    );
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Global,
            &json!({"diagnostics": {"W111": false}}),
        )
        .layer(PolicyLayer::Editor, &json!({"diagnostics": {"W111": true}}))
        .layer(
            PolicyLayer::Project,
            &json!({"diagnostics": {"W111": false}}),
        )
        .build();
    assert_eq!(
        decision(&policy, DiagCode::W111),
        Some(CodeDecision {
            enabled: false,
            layer: PolicyLayer::Project
        })
    );
    // A layer silent on the code leaves the lower decision standing.
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Global,
            &json!({"diagnostics": {"W111": false}}),
        )
        .layer(
            PolicyLayer::Project,
            &json!({"diagnostics": {"W112": false}}),
        )
        .build();
    assert_eq!(
        decision(&policy, DiagCode::W111).map(|d| d.layer),
        Some(PolicyLayer::Global)
    );
    assert_eq!(
        decision(&policy, DiagCode::W112).map(|d| d.layer),
        Some(PolicyLayer::Project)
    );
}

/// Rule 5's order lives on [`PolicyLayer`]: a surface that adds the
/// project file first resolves exactly as one that adds the layers
/// lowest first, every section alike.
#[test]
fn the_layers_resolve_in_scope_order_whatever_order_they_are_added() {
    let global = json!({
        "diagnostics": {"W111": false, "W112": false},
        "diagnosticSeverity": {"W210": "warning"},
        "optimiser": {"profile": "full", "O114": false},
        "shimmer": {"enabled": false},
    });
    let editor = json!({
        "diagnostics": {"W111": true, "W242": true},
        "diagnosticSeverity": {"W210": "error"},
        "optimiser": {"enabled": false},
    });
    let project = json!({
        "diagnostics": {"W111": false, "W112": true},
        "optimiser": {"profile": "readability", "O114": true, "enabled": true},
        "shimmer": {"enabled": true},
    });
    let lowest_first = PolicyBuilder::new()
        .layer(PolicyLayer::Global, &global)
        .layer(PolicyLayer::Editor, &editor)
        .layer(PolicyLayer::Project, &project)
        .build();
    let project_first = PolicyBuilder::new()
        .layer(PolicyLayer::Project, &project)
        .layer(PolicyLayer::Global, &global)
        .layer(PolicyLayer::Editor, &editor)
        .build();
    assert_eq!(project_first.codes, lowest_first.codes);
    assert_eq!(
        project_first.severity_overrides,
        lowest_first.severity_overrides
    );
    assert_eq!(project_first.optimiser, lowest_first.optimiser);
    assert_eq!(project_first.shimmer, lowest_first.shimmer);
    assert_eq!(
        decision(&project_first, DiagCode::W111),
        Some(CodeDecision {
            enabled: false,
            layer: PolicyLayer::Project
        }),
        "the project file decides, though it was added first"
    );
    assert_eq!(
        project_first.severity_overrides.get(&DiagCode::W210),
        Some(&Severity::Error)
    );
    assert_eq!(
        project_first.optimiser.profile,
        OptimisationProfile::Readability
    );
    assert!(project_first.optimiser.enabled);
    assert!(project_first.shimmer);
}

#[test]
fn the_default_off_seed_is_the_lowest_layer() {
    let policy = PolicyBuilder::new().build();
    assert!(policy.default_off.contains(&DiagCode::W242));
    assert_eq!(decision(&policy, DiagCode::W242), None);
    let policy = PolicyBuilder::new()
        .layer(PolicyLayer::Global, &json!({"diagnostics": {"W242": true}}))
        .build();
    assert_eq!(
        decision(&policy, DiagCode::W242),
        Some(CodeDecision {
            enabled: true,
            layer: PolicyLayer::Global
        })
    );
}

#[test]
fn an_unusable_higher_value_resets_the_lower_decision() {
    // Merge-then-parse semantics: the higher layer's value replaced the
    // lower one in the merged JSON, and the parse then skipped it.
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Global,
            &json!({"diagnostics": {"W111": false}}),
        )
        .layer(
            PolicyLayer::Project,
            &json!({"diagnostics": {"W111": "no"}}),
        )
        .build();
    assert_eq!(decision(&policy, DiagCode::W111), None);
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Global,
            &json!({"diagnosticSeverity": {"W211": "warning"}}),
        )
        .layer(
            PolicyLayer::Editor,
            &json!({"diagnosticSeverity": {"W211": "default"}}),
        )
        .build();
    assert!(policy.severity_overrides.is_empty());
}

#[test]
fn wrapped_and_flat_dotted_shapes_are_read() {
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Editor,
            &json!({"tclLsp": {"diagnostics": {"W108": false}}}),
        )
        .layer(
            PolicyLayer::Project,
            &json!({"tclLsp.diagnostics.W210": false, "diagnosticSeverity.W211": "error"}),
        )
        .build();
    assert_eq!(
        decision(&policy, DiagCode::W108).map(|d| d.layer),
        Some(PolicyLayer::Editor)
    );
    assert_eq!(
        decision(&policy, DiagCode::W210).map(|d| d.layer),
        Some(PolicyLayer::Project)
    );
    assert_eq!(
        policy.severity_overrides.get(&DiagCode::W211),
        Some(&Severity::Error)
    );
    // Keys that are not codes (`exclude`, `genericVariablePatterns`)
    // and codes the catalogue lacks are ignored.
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Project,
            &json!({"diagnostics": {"exclude": ["a/**"], "W122": false, "W999": false}}),
        )
        .build();
    assert!(policy.codes.is_empty());
}

#[test]
fn the_optimiser_profile_and_overrides_resolve_in_layer_order() {
    let policy = PolicyBuilder::new().build();
    assert!(policy.optimiser.enabled);
    assert_eq!(policy.optimiser.profile, DEFAULT_EDITOR_PROFILE);
    assert!(policy.optimiser.disabled.contains(&DiagCode::O109));
    assert!(!policy.optimiser.disabled.contains(&DiagCode::O114));

    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Global,
            &json!({"optimiser": {"profile": "full", "O114": false}}),
        )
        .layer(
            PolicyLayer::Project,
            &json!({"optimiser": {"enabled": false, "O109": false, "O114": true}}),
        )
        .build();
    assert!(!policy.optimiser.enabled);
    assert_eq!(policy.optimiser.profile, OptimisationProfile::Full);
    assert!(policy.optimiser.disabled.contains(&DiagCode::O109));
    assert!(
        !policy.optimiser.disabled.contains(&DiagCode::O114),
        "the project's `true` wins over the global `false`"
    );
    // An unknown profile name falls back to the editor default, as the
    // server's parse does.
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Editor,
            &json!({"optimiser": {"profile": "nope"}}),
        )
        .build();
    assert_eq!(policy.optimiser.profile, DEFAULT_EDITOR_PROFILE);
}

/// The owner's ruling: a profile the request names is the profile in
/// force over every layer, the files' `optimiser.profile` is the default
/// when it names none, and the switch and the per-code overrides keep
/// the layer order (`docs/design/compiler/diagnostic-policy.md`
/// § Configuration).
#[test]
fn a_requested_profile_is_in_force_over_every_layer() {
    let global = json!({"optimiser": {"profile": "aggressive"}});
    let invocation = json!({"optimiser": {"O114": false}});
    let project = json!({"optimiser": {"profile": "readability", "O109": true, "O114": true}});
    let layered = |requested: Option<OptimisationProfile>| {
        PolicyBuilder::new()
            .layer(PolicyLayer::Global, &global)
            .layer(PolicyLayer::Invocation, &invocation)
            .layer(PolicyLayer::Project, &project)
            .requested_profile(requested)
            .default_profile(OptimisationProfile::Full)
            .build()
    };
    let named = layered(Some(OptimisationProfile::Standard));
    assert_eq!(named.optimiser.profile, OptimisationProfile::Standard);
    assert!(
        !named.optimiser.disabled.contains(&DiagCode::O109),
        "the project's per-code `true` still applies over the named profile"
    );
    assert!(
        !named.optimiser.disabled.contains(&DiagCode::O114),
        "the project's per-code `true` still beats the invocation's `false`"
    );
    assert_eq!(
        layered(None).optimiser.profile,
        OptimisationProfile::Readability,
        "with none named, the project file supplies the profile"
    );

    let switched_off = PolicyBuilder::new()
        .layer(
            PolicyLayer::Project,
            &json!({"optimiser": {"enabled": false}}),
        )
        .requested_profile(Some(OptimisationProfile::Full))
        .build();
    assert!(
        !switched_off.optimiser.enabled,
        "the master switch stays on the layers"
    );

    let global_only = PolicyBuilder::new()
        .layer(
            PolicyLayer::Global,
            &json!({"optimiser": {"profile": "standard"}}),
        )
        .default_profile(OptimisationProfile::Full)
        .build();
    assert_eq!(
        global_only.optimiser.profile,
        OptimisationProfile::Standard,
        "the global file supplies the default too"
    );
    let unnamed = PolicyBuilder::new()
        .default_profile(OptimisationProfile::Full)
        .build();
    assert_eq!(
        unnamed.optimiser.profile,
        OptimisationProfile::Full,
        "the surface's own default when nothing names one"
    );
}

#[test]
fn the_family_switches_and_the_document_gates_resolve() {
    let policy = PolicyBuilder::new()
        .layer(PolicyLayer::Global, &json!({"shimmer": {"enabled": false}}))
        .layer(
            PolicyLayer::Project,
            &json!({"features": {"diagnostics": false}}),
        )
        .excluded(true)
        .build();
    assert!(!policy.shimmer);
    assert!(!policy.document.reporting);
    assert!(policy.document.excluded);
    assert!(!policy.document.abstain);
    // The caller's own answer for `features.diagnostics` wins.
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Project,
            &json!({"features": {"diagnostics": false}}),
        )
        .reporting(true)
        .build();
    assert!(policy.document.reporting);
}

#[test]
fn the_decode_report_drives_abstention() {
    let (_, report) = crate::source_decode::decode_source(&[0xFF, 0xFE, b'p', 0, b'u', 0]);
    assert!(
        PolicyBuilder::new()
            .decode(Some(&report))
            .build()
            .document
            .abstain
    );
    let (_, clean) = crate::source_decode::decode_source(b"puts ok\n");
    assert!(
        !PolicyBuilder::new()
            .decode(Some(&clean))
            .build()
            .document
            .abstain
    );
    assert!(!PolicyBuilder::new().decode(None).build().document.abstain);
}

#[test]
fn the_overlap_table_follows_the_dialect() {
    let plain = dialect_overlaps(crate::profile_for_dialect("tcl9.0"));
    assert_eq!(plain, base_overlaps());
    assert_eq!(
        plain[0],
        Overlap {
            owner: OverlapOwner::Code(DiagCode::W110),
            superseded: DiagCode::O120,
            scope: OverlapScope::WithinSpan,
        }
    );
    let sslic = dialect_overlaps(crate::profile_for_dialect("sslictcl"));
    assert!(sslic.contains(&Overlap {
        owner: OverlapOwner::Producer(Producer::SslicTcl),
        superseded: DiagCode::W123,
        scope: OverlapScope::Document,
    }));
    let built = PolicyBuilder::new()
        .dialect(crate::profile_for_dialect("sslictcl"))
        .build();
    assert_eq!(built.overlaps, sslic);
}

#[test]
fn directives_scan_reads_both_directive_spellings() {
    let text = "# tcl-lsp: disable=W111, O114\nset a 1\n# noqa: W210\nputs $b\n";
    let directives = Directives::scan(text, crate::profile_for_dialect("tcl9.0"));
    let file = &directives.lines()[&FILE_SUPPRESS_KEY];
    assert!(file.contains("W111") && file.contains("O114"));
    assert!(directives.lines()[&3].contains("W210"));
    assert!(directives.hit(FILE_SUPPRESS_KEY, DiagCode::W111));
    assert!(!directives.hit(FILE_SUPPRESS_KEY, DiagCode::W112));
    assert!(directives.hit(3, DiagCode::W210));
    // `puts $b` starts at byte 51 — line 3.
    assert_eq!(directives.line_of(Span::new(51, 58)), 3);
    // The analyser's own map is the same shape.
    let analysis = analyser::Analyser::new().analyse(text, "tcl9.0");
    let from_analysis = Directives::from_analysis(&analysis, text);
    assert!(from_analysis.hit(3, DiagCode::W210));
    assert!(from_analysis.hit(FILE_SUPPRESS_KEY, DiagCode::W111));
    // No text at all: nothing hits, and every span is on line 0.
    let none = Directives::none();
    assert!(!none.hit(0, DiagCode::W210));
    assert_eq!(none.line_of(Span::new(51, 58)), 0);
}

#[test]
fn directives_answer_inline_before_file_and_skip_inline_for_whole_file_codes() {
    let text = "# tcl-lsp: disable=W112\n# noqa\nset x 1   \nputs $y\n";
    let directives = Directives::scan(text, crate::profile_for_dialect("tcl9.0"));
    // `set x 1   ` is line 2, bytes 31..41; `puts $y` is line 3, 42..49.
    assert_eq!(
        directives.reason_for(DiagCode::W210, Span::new(31, 41)),
        Some(Reason::InlineDirective { line: 2 })
    );
    // Both directives cover W112 on line 2; the inline one is reported.
    assert_eq!(
        directives.reason_for(DiagCode::W112, Span::new(38, 41)),
        Some(Reason::InlineDirective { line: 2 })
    );
    assert_eq!(
        directives.reason_for(DiagCode::W112, Span::new(42, 49)),
        Some(Reason::FileDirective)
    );
    assert_eq!(
        directives.reason_for(DiagCode::W210, Span::new(42, 49)),
        None
    );
    // A whole-file code ignores the inline bucket and honours the file one.
    let mut lines: HashMap<i32, HashSet<String>> = HashMap::new();
    lines.insert(0, std::iter::once("*".to_owned()).collect());
    let inline_only = Directives::new(lines.clone(), "a\r\nb\r\n");
    assert_eq!(
        inline_only.reason_for(DiagCode::W118, Span::new(0, 0)),
        None
    );
    assert_eq!(
        inline_only.reason_for(DiagCode::W111, Span::new(0, 0)),
        Some(Reason::InlineDirective { line: 0 })
    );
    lines.insert(
        FILE_SUPPRESS_KEY,
        std::iter::once("W118".to_owned()).collect(),
    );
    let with_file = Directives::new(lines, "a\r\nb\r\n");
    assert_eq!(
        with_file.reason_for(DiagCode::W118, Span::new(0, 0)),
        Some(Reason::FileDirective)
    );
    for code in WHOLE_FILE_CODES {
        assert!(!code.is_optimisation(), "{code}");
    }
}

#[test]
fn a_wildcard_bucket_silences_every_code() {
    let mut lines: HashMap<i32, HashSet<String>> = HashMap::new();
    lines.insert(0, std::iter::once("*".to_owned()).collect());
    let directives = Directives::new(lines, "set x 1\n");
    assert!(directives.hit(0, DiagCode::W210));
    assert!(directives.hit(0, DiagCode::O114));
    assert!(!directives.hit(1, DiagCode::W210));
}

/// `Directives::hit` restates the bucket rule of the owner's
/// `line_suppressed` rather than calling it, which needs a single-bucket
/// predicate `tcl-compiler` does not offer.
/// This keeps the two equal: an inline bucket or the file bucket, holding
/// `*` or a code, silences a line code where `line_suppressed` says so,
/// and a whole-file code only where the file bucket does.
#[test]
fn directives_agree_with_line_suppressed() {
    let text = "a\nb\nc\nd\n";
    let span_at = |line: u32| Span::new(line * 2, line * 2 + 1);
    let mut maps: Vec<HashMap<i32, HashSet<String>>> = Vec::new();
    let mut every: HashMap<i32, HashSet<String>> = HashMap::new();
    for key in [1, FILE_SUPPRESS_KEY] {
        for entry in ["*", "W210", "W112"] {
            maps.push(HashMap::from([(key, HashSet::from([entry.to_owned()]))]));
            every.entry(key).or_default().insert(entry.to_owned());
        }
    }
    maps.push(every);
    for map in maps {
        let directives = Directives::new(map.clone(), text);
        for code in [DiagCode::W210, DiagCode::W112, DiagCode::W118] {
            for line in 0..=3_u32 {
                let key = if WHOLE_FILE_CODES.contains(&code) {
                    FILE_SUPPRESS_KEY
                } else {
                    i32::try_from(line).expect("a small line")
                };
                assert_eq!(
                    directives.reason_for(code, span_at(line)).is_some(),
                    analyser::line_suppressed(code.as_str(), key, &map),
                    "{code} at line {line} under {map:?}"
                );
            }
        }
    }
}
