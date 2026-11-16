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
use crate::events::EventRegistry;
use crate::f5::evidence::{BigIpBuild, RuntimeFact, RuntimeFactKind, measured_fact};
use crate::f5::execution_context::BigIpExecutionContext;
use crate::model::ingress::{resolve_environment, static_context_for};
use crate::profiles::ProfileRegistry;
use tcl_dialect::model::family::{Family, Release, grammar};
use tcl_dialect::model::{SurfaceQuery, surface_admits};
use tcl_dialect::model::{VersionAxisId, VersionSet};

const F5_ENVIRONMENTS: [&str; 3] = ["f5-irules", "f5-tmsh", "f5-iapps"];

/// The corpus is a fixed, citeable size. A row appearing or vanishing
/// without a deliberate edit here is drift.
#[test]
fn the_corpus_has_its_measured_row_counts() {
    assert_eq!(PARSER_PARITY_VECTORS.len(), 21, "§4a parity cases");
    assert_eq!(CONTEXT_ENVIRONMENT_VECTORS.len(), 9, "§4a environment rows");
    assert_eq!(
        RELEASE_DISCRIMINATOR_VECTORS.len(),
        16,
        "§4's 8.4-vs-8.5 discriminators"
    );
    assert_eq!(COMMAND_CLASS_VECTORS.len(), 31, "§4b's 16 + 15 split");
    assert_eq!(
        EVENT_CONTEXT_VECTORS.len(),
        120,
        "§8's 15 commands × 8 events"
    );
    assert_eq!(PRIORITY_VECTORS.len(), 8, "§6/§8 priority facts");

    for row in PARSER_PARITY_VECTORS {
        assert!(row.section.contains('§'), "{}: no citation", row.case);
    }
    for row in CONTEXT_ENVIRONMENT_VECTORS {
        assert!(row.section.contains('§'), "{}: no citation", row.property);
    }
    for row in RELEASE_DISCRIMINATOR_VECTORS {
        assert!(row.section.contains('§'), "{}: no citation", row.feature);
    }
    for row in COMMAND_CLASS_VECTORS {
        assert!(row.section.contains('§'), "{}: no citation", row.command);
    }
    for row in PRIORITY_VECTORS {
        assert!(row.section.contains('§'));
    }
    for row in EVENT_CONTEXT_VECTORS {
        assert!(row.section().contains('§'), "{}", row.command);
    }
    assert_eq!(
        EventContextVector::PROBE_SET,
        ProbeSetId::EVENT_CONTEXT,
        "the matrix's provenance is the 120-cell sweep"
    );
}

/// The expr half of the parity check, extracted so the axis walk stays
/// readable: the word operator the row exercises must resolve
/// identically on the trunk and the offshoot, must be absent from
/// stock 8.4, and must match the row's recorded expectation.
fn assert_expr_word_operator_row(row: &ParserParityVector) {
    let f5_expr = tcl_dialect::model::expr_grammar::expr(Family::F5Tcl, Release::F5_TCL_TMOS);
    let irules_expr =
        tcl_dialect::model::expr_grammar::expr(Family::F5Irules, Release::F5_IRULES_TMM);
    let host84_expr = tcl_dialect::model::expr_grammar::expr(Family::Tcl, Release::TCL_8_4);
    let operator = match row.case {
        "e_starts_with" => "starts_with",
        "e_contains" => "contains",
        "e_matches" => "matches",
        "e_and_word" => "and",
        other => panic!("{other}: unmapped expr row"),
    };
    let present = f5_expr.has_word_operator(operator);
    assert_eq!(
        present,
        irules_expr.has_word_operator(operator),
        "{}: the offshoot answers from the trunk",
        row.case
    );
    assert!(
        !host84_expr.has_word_operator(operator),
        "{operator} is not stock Tcl"
    );
    match row.expectation {
        ModelExpectation::Agrees => assert!(
            present,
            "{}: measured {:?} on the appliance",
            row.case, row.f5
        ),
        ModelExpectation::Diverges(DivergenceReason::ModelMissingMeasuredSurface) => assert!(
            !present,
            "{}: the recorded gap is closed — reclassify the row as Agrees",
            row.case
        ),
        other => panic!("{}: unexpected expectation {other:?}", row.case),
    }
}

/// Each parity row names the grammar axis that produced it, and that
/// axis must hold in **all three** F5 core profiles (the evidence that
/// the fork grammar is a trunk fact) while differing at the host build
/// wherever the row differs there.
#[test]
fn parser_parity_rows_are_explained_by_a_shared_trunk_axis() {
    let f5_grammars = [
        grammar(Family::F5Irules, Release::F5_IRULES_TMM),
        grammar(Family::F5Tcl, Release::F5_TCL_TMOS),
    ];
    let host84 = grammar(Family::Tcl, Release::TCL_8_4);
    let host85 = grammar(Family::Tcl, Release::TCL_8_5);

    for row in PARSER_PARITY_VECTORS {
        match row.axis {
            GrammarAxis::ImplicitWordBreak | GrammarAxis::WordBreakGating => {
                for g in f5_grammars {
                    assert!(g.irules_brace_separator, "{}", row.case);
                }
                assert!(!host84.irules_brace_separator);
                assert!(!host85.irules_brace_separator);
            }
            GrammarAxis::BraceLineContinuation | GrammarAxis::ContinuationBound => {
                for g in f5_grammars {
                    assert!(g.brace_line_continuation.continues(), "{}", row.case);
                }
                assert!(!host84.brace_line_continuation.continues());
                assert!(!host85.brace_line_continuation.continues());
            }
            GrammarAxis::InertExpansion => {
                for g in f5_grammars {
                    assert!(!g.expand_syntax, "{}: the separator wins", row.case);
                }
                assert!(!host84.expand_syntax);
                assert!(
                    host85.expand_syntax,
                    "the 8.5 control really does expand — which is why the \
                         F5 value is a false pass, not a feature"
                );
            }
            GrammarAxis::NumeralGrammar => {
                for g in f5_grammars {
                    assert_eq!(g.numbers, host84.numbers, "{}: 8.4 numerals", row.case);
                }
                assert_ne!(host85.numbers, host84.numbers);
            }
            GrammarAxis::ExprWordOperators => assert_expr_word_operator_row(row),
            GrammarAxis::Unmodified => {
                assert_eq!(row.f5, row.host84, "{}: control row", row.case);
            }
        }
        match row.axis {
            // A fork axis exists because the F5 answer differs from
            // the stock answer; a row that does not differ has no
            // business citing one.
            GrammarAxis::ImplicitWordBreak
            | GrammarAxis::BraceLineContinuation
            | GrammarAxis::InertExpansion
            | GrammarAxis::ExprWordOperators => assert!(
                row.f5 != row.host84 || row.f5 != row.host85,
                "{}: no divergence to record",
                row.case
            ),
            // The numeral rows are the opposite claim: the F5 tree
            // behaves as its 8.4 fork point throughout.
            GrammarAxis::NumeralGrammar => assert_eq!(
                row.f5, row.host84,
                "{}: 8.4 numerals throughout (§4a)",
                row.case
            ),
            // The gating rows say the opposite of a fork row: every
            // context agrees, which is what keeps R5 from
            // overclaiming.
            GrammarAxis::WordBreakGating | GrammarAxis::ContinuationBound => {
                assert_eq!(row.f5, row.host84, "{}: boundary row", row.case);
                assert_eq!(row.f5, row.host85, "{}: boundary row", row.case);
            }
            GrammarAxis::Unmodified => {}
        }
    }
}

/// None of the sixteen discriminators may be offered under any F5
/// environment, and the ones that are commands are checked through the
/// registry rather than asserted in prose.
#[test]
fn release_discriminators_are_absent_from_every_f5_environment() {
    let control = static_context_for("tcl8.5").commands();
    let control_point = Some(
        resolve_environment("tcl8.5")
            .analyser_profile()
            .surface_query(),
    );
    for row in RELEASE_DISCRIMINATOR_VECTORS {
        assert!(
            !row.tcl84 && row.tcl85,
            "{}: not a discriminator",
            row.feature
        );
        for behaviour in [row.tmsh, row.iapp] {
            assert_ne!(
                behaviour,
                DiscriminatorBehaviour::BehavesAs85,
                "{}: measured as 8.4 in both contexts",
                row.feature
            );
        }
        let ModelProbe::Command(command) = row.probe else {
            continue;
        };
        assert!(
            control.get_for_surface(command, control_point).is_some(),
            "{command}: the 8.5 control must have it"
        );
        for environment in F5_ENVIRONMENTS {
            let registry = static_context_for(environment).commands();
            let point = Some(
                resolve_environment(environment)
                    .analyser_profile()
                    .surface_query(),
            );
            assert!(
                registry.get_for_surface(command, point).is_none(),
                "{environment}: {command} is measured absent (§4)"
            );
        }
    }
}

/// The §4b split, from the transcript side: the corpus, the policy
/// module, and the evidence table must all say the same thing.
#[test]
fn command_classes_match_the_measured_split() {
    let build = BigIpBuild::MEASURED_21_1_0_1;
    let mut absent = 0;
    let mut refused = 0;
    for row in COMMAND_CLASS_VECTORS {
        assert_eq!(
            crate::irules_policy::irules_disabled_class(row.command),
            Some(row.class),
            "{}",
            row.command
        );
        assert_eq!(
            row.runtime_reachable,
            row.class == IrulesDisabledClass::CompilerRefused,
            "{}: reachability follows the mechanism",
            row.command
        );
        assert_eq!(
            row.class.is_language_fact(),
            !row.runtime_reachable,
            "{}: severity follows the mechanism",
            row.command
        );
        let evidence = measured_fact(
            BigIpExecutionContext::TmmIRule,
            build,
            RuntimeFactKind::CommandSurface(row.command),
        )
        .unwrap_or_else(|| panic!("{}: no evidence row", row.command));
        assert_eq!(
            evidence.provenance.probe_set,
            ProbeSetId::PROC_SEMANTICS,
            "{}",
            row.command
        );
        match row.class {
            IrulesDisabledClass::InterpreterAbsent => absent += 1,
            IrulesDisabledClass::CompilerRefused => refused += 1,
        }
    }
    assert_eq!((absent, refused), (16, 15));
}

/// The 120-cell matrix, asserted against the model's own
/// event-validity oracle.
///
/// Rows marked [`ModelExpectation::Agrees`] must keep agreeing; rows
/// marked [`ModelExpectation::Diverges`] must keep diverging *exactly
/// where they are recorded*, so that closing a gap is a deliberate
/// edit of this table and never a silent change of behaviour.
#[test]
fn event_context_vectors_pin_the_model() {
    let registry = static_context_for("f5-irules").commands();
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let mut agreeing = 0;
    let mut diverging = 0;
    for row in EVENT_CONTEXT_VECTORS {
        if let ModelExpectation::NotComparable(reason) = row.expectation {
            assert_eq!(
                row.command, "static::",
                "only the variable-namespace row is incomparable"
            );
            assert!(!reason.is_empty(), "an incomparable row must say why");
            continue;
        }
        let model =
            registry.is_irules_call_legal_in_event(row.command, &[], row.event, &events, &profiles);
        match row.expectation {
            ModelExpectation::Agrees => {
                assert_eq!(
                    model,
                    row.measured.accepted(),
                    "{} in {}: the model no longer matches the appliance (§8)",
                    row.command,
                    row.event
                );
                agreeing += 1;
            }
            ModelExpectation::Diverges(_) => {
                assert_ne!(
                    model,
                    row.measured.accepted(),
                    "{} in {}: the recorded divergence is closed — \
                         reclassify the row as Agrees",
                    row.command,
                    row.event
                );
                diverging += 1;
            }
            ModelExpectation::NotComparable(_) => unreachable!("handled above"),
        }
    }
    assert_eq!(
        (agreeing, diverging),
        (106, 6),
        "the divergence budget is fixed: change it deliberately"
    );

    // Every remaining divergence is the deliberate `RULE_INIT`
    // compile-acceptance one.
    let deliberate = EVENT_CONTEXT_VECTORS
        .iter()
        .filter(|row| {
            matches!(
                row.expectation,
                ModelExpectation::Diverges(DivergenceReason::RuleInitCompileAcceptance)
            )
        })
        .count();
    assert_eq!(deliberate, 6, "§8's compile-acceptance caveat");
    assert_eq!(deliberate, diverging, "no open event-context gap is left");

    // …and the two open-gap classes are empty. They are kept as
    // vocabulary rather than deleted: the corpus has to be able to
    // say *which way* a future regression hurts — an over-permissive
    // cell costs the user a missed load-time error, an over-strict
    // one a false positive.
    let count = |reason: DivergenceReason| {
        EVENT_CONTEXT_VECTORS
            .iter()
            .filter(|row| row.expectation == ModelExpectation::Diverges(reason))
            .count()
    };
    assert_eq!(
        (
            count(DivergenceReason::ModelBroaderThanMeasured),
            count(DivergenceReason::ModelNarrowerThanMeasured),
        ),
        (0, 0),
        "open event-context gaps: over-permissive vs over-strict"
    );
}

/// The priority policy the traffic lab measured is the policy the
/// registry ships.
#[test]
fn priority_vectors_match_the_shipping_policy() {
    let policy = crate::events::BIGIP_EVENT_HANDLER_PRIORITY;
    let mut ordering: Vec<(u8, u16)> = Vec::new();
    for row in PRIORITY_VECTORS {
        match row.case {
            PriorityCase::Bound { value, accepted } => {
                let modelled = u16::try_from(value).is_ok_and(|v| policy.accepts(v));
                assert_eq!(modelled, accepted, "priority {value}");
            }
            PriorityCase::Ordering {
                declared,
                effective,
                rank,
                ..
            } => {
                match declared {
                    Some(value) => assert_eq!(value, effective),
                    None => assert_eq!(
                        effective, policy.default_priority,
                        "an omitted clause takes the default"
                    ),
                }
                ordering.push((rank, effective));
            }
        }
    }
    ordering.sort_unstable();
    assert!(
        ordering.windows(2).all(|pair| pair[0].1 < pair[1].1),
        "lower numbers ran first: {ordering:?}"
    );
    assert!(policy.lower_runs_first);
}

/// The §4a environment table and the evidence records are two views of
/// one transcript; they cannot be allowed to drift apart.
#[test]
fn context_environment_vectors_agree_with_the_evidence_table() {
    let build = BigIpBuild::MEASURED_21_1_0_1;
    let value_of = |row: &ContextEnvironmentVector, context: BigIpExecutionContext| match context {
        BigIpExecutionContext::TmmIRule => row.tmm,
        BigIpExecutionContext::TmshCliScript => row.tmsh,
        BigIpExecutionContext::IAppImplementation => row.iapp,
        BigIpExecutionContext::HostShellTcl => row.host84,
        other => panic!("{other} has no column"),
    };
    let contexts = [
        BigIpExecutionContext::TmmIRule,
        BigIpExecutionContext::TmshCliScript,
        BigIpExecutionContext::IAppImplementation,
        BigIpExecutionContext::HostShellTcl,
    ];

    for row in CONTEXT_ENVIRONMENT_VECTORS {
        for context in contexts {
            let cell = value_of(row, context);
            match row.property {
                "info patchlevel" => {
                    let Some(RuntimeFact::ReportedPatchlevel {
                        info_patchlevel, ..
                    }) = measured_fact(context, build, RuntimeFactKind::ReportedPatchlevel)
                        .map(|e| e.fact)
                    else {
                        panic!("{context}: no patchlevel row");
                    };
                    assert_eq!(info_patchlevel, cell, "{context}");
                }
                "llength [info commands]" => {
                    let Some(RuntimeFact::CommandCount(count)) =
                        measured_fact(context, build, RuntimeFactKind::CommandCount)
                            .map(|e| e.fact)
                    else {
                        panic!("{context}: no command-count row");
                    };
                    assert_eq!(count.to_string(), cell, "{context}");
                }
                "tcl_platform keys" => {
                    let Some(RuntimeFact::TclPlatform { keys, .. }) =
                        measured_fact(context, build, RuntimeFactKind::TclPlatform).map(|e| e.fact)
                    else {
                        panic!("{context}: no platform row");
                    };
                    assert_eq!(keys.to_string(), cell, "{context}");
                }
                "tcl_platform(wordSize)" => {
                    let Some(RuntimeFact::TclPlatform { word_size, .. }) =
                        measured_fact(context, build, RuntimeFactKind::TclPlatform).map(|e| e.fact)
                    else {
                        panic!("{context}: no platform row");
                    };
                    assert_eq!(
                        word_size.map_or_default(|w| w.to_string()),
                        cell,
                        "{context}"
                    );
                }
                _ => {}
            }
        }
    }

    // TMM's seven fabricated `tcl_platform` keys are exactly the seven
    // the shipping variable model claims for iRules.
    let irules_keys = crate::special_vars::special_var("tcl_platform")
        .expect("tcl_platform is modelled")
        .keys
        .iter()
        .filter(|key| {
            surface_admits(
                key.surface,
                Some(&SurfaceQuery::any_release(Family::F5Irules)),
            )
        })
        .count();
    assert_eq!(irules_keys, 7, "§4: TMM reports 7 fabricated keys");
}

/// The two `Unknown` contexts have no corpus coverage at all, and the
/// acceptance matrix's remaining columns are still open — the corpus
/// says so rather than implying completeness.
#[test]
fn the_corpus_covers_one_build_and_four_contexts() {
    assert_eq!(crate::f5::evidence::MEASURED_BUILDS.len(), 1);
    for context in [
        BigIpExecutionContext::IAppPresentationApl,
        BigIpExecutionContext::IAppPresentationTclCallback,
    ] {
        assert!(!context.measurement().is_measured(), "{context}");
    }
    // The BIG-IP axis the acceptance matrix's other two columns will
    // land on is typed and ready; nothing on it is claimed yet.
    let measured = VersionSet::from_requirements(VersionAxisId::big_ip(), &["21.1.0.1-21.1.0.1"])
        .expect("measured build");
    let seventeen = VersionSet::from_requirements(VersionAxisId::big_ip(), &["17.1.0-18.0"])
        .expect("17.x column");
    assert!(
        measured
            .intersect(&seventeen)
            .expect("same axis")
            .is_empty(),
        "the 17.x column is not covered by this build"
    );
}
