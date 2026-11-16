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

//! Function-level GVN over registry semantic facts: proved dispatch, declined dialects and volatile results.

use super::*;

#[test]
fn function_level_gvn_proves_llength_dispatch_and_reports_o105() {
    let registry = release_registry("tcl8.6");
    let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
        "llength {a b}\nllength {a b}",
        &registry,
        false,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let function = &cu.top_level;

    assert!(
        !find_redundancies(
            &registry,
            &function.cfg,
            &function.ssa,
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile())
        )
        .is_empty(),
        "the legacy classifier recognises the registry pure trait"
    );
    let semantic = find_redundancies_for_function(
        &registry,
        function,
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
    );
    assert_eq!(
        semantic.len(),
        1,
        "a complete site proof over a pristine entry world enables stable-call CSE"
    );
    assert_eq!(semantic[0].code, DiagCode::O105);
}

#[test]
fn function_level_gvn_declines_without_a_selected_semantic_dialect() {
    let registry = CommandRegistry::build_default();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "llength {a b}\nllength {a b}",
        &registry,
        false,
    );
    let function = &cu.top_level;
    let legacy = find_redundancies(&registry, &function.cfg, &function.ssa, None);
    let sidecar = find_redundancies_for_function(&registry, function, None);
    assert_ne!(legacy, [] as [crate::gvn::RedundantComputation; 0]);
    assert_eq!(
        sidecar.len(),
        0,
        "production GVN must not fall back to command strings when semantic facts are unavailable"
    );
}

#[test]
fn function_level_gvn_maps_calls_inside_a_structured_conditional_body() {
    let registry = CommandRegistry::build_default();
    let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
        "if {$flag} {\nllength {a b}\nllength {a b}\n}",
        &registry,
        false,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let function = &cu.top_level;
    assert!(
        !find_redundancies(
            &registry,
            &function.cfg,
            &function.ssa,
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile())
        )
        .is_empty(),
        "the explicit legacy API still sees the nested string-classified calls"
    );
    assert!(
        !find_redundancies_for_function(
            &registry,
            function,
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile())
        )
        .is_empty(),
        "an `if` body is now executable-IR blocks, so its two identical pure \
             calls have exact invocation mappings and the function-level API \
             agrees with the legacy CFG API"
    );
}

#[test]
fn function_level_gvn_proves_exact_mapped_closed_pure_call() {
    let mut registry = release_registry("tcl8.6");
    registry.insert(tcl_registry::CommandSpec {
        name: "test::pure_cse",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        ..tcl_registry::CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT
    });
    let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
        "test::pure_cse value\ntest::pure_cse value",
        &registry,
        false,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(matches!(
        cu.top_level.semantic_facts.executable(),
        ExecutableAnalysisAvailability::Available(_)
    ));
    assert_eq!(
        find_redundancies_for_function(
            &registry,
            &cu.top_level,
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile())
        )
        .len(),
        1,
        "a closed registry contract plus a complete site proof reports the replay"
    );
}

#[test]
fn function_level_gvn_proves_other_stable_commands() {
    let registry = release_registry("tcl9.0");
    for source in [
        "format %s x\nformat %s x",
        "lindex {a b} 0\nlindex {a b} 0",
        "lreverse {a b}\nlreverse {a b}",
    ] {
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            &registry,
            false,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        assert_eq!(
            find_redundancies_for_function(
                &registry,
                &cu.top_level,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
                )
            )
            .len(),
            1,
            "closed stable registry category with a live dispatch proof reuses: {source}"
        );
    }
}

#[test]
fn static_llength_candidate_requires_coverage_of_registry_dispatch_dependencies() {
    use crate::dispatch_proof::{SiteDispatchProof, TrackVersion, WorldTrack};

    let registry = CommandRegistry::build_default();
    let resolved = registry
        .resolve_invocation(
            "llength",
            &["a b"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .expect("llength resolves")
        .facts();

    assert!(resolved_invocation_is_gvn_candidate(&resolved));
    assert!(!resolved.dispatch_dependencies.is_empty());

    // No site proof at all (world graph unavailable or unreached).
    assert_eq!(
        gvn_site_eligibility(&resolved, None),
        GvnSiteEligibility::Declined
    );

    let complete = SiteDispatchProof {
        covers: resolved.dispatch_dependencies,
        versions: [TrackVersion::Entry; WorldTrack::ALL.len()],
        operand_words_admissible: true,
    };
    assert!(matches!(
        gvn_site_eligibility(&resolved, Some(&complete)),
        GvnSiteEligibility::Eligible { .. }
    ));

    // Any uncovered dependency domain fails the site closed.
    let partial = SiteDispatchProof {
        covers: tcl_registry::DispatchDependencies::NONE,
        versions: [TrackVersion::Entry; WorldTrack::ALL.len()],
        operand_words_admissible: true,
    };
    assert_eq!(
        gvn_site_eligibility(&resolved, Some(&partial)),
        GvnSiteEligibility::Declined
    );

    // Inadmissible operand words fail the site closed even with covered
    // dispatch domains.
    let inadmissible = SiteDispatchProof {
        operand_words_admissible: false,
        ..complete
    };
    assert_eq!(
        gvn_site_eligibility(&resolved, Some(&inadmissible)),
        GvnSiteEligibility::Declined
    );
}

#[test]
fn live_execution_trace_keeps_o105_suppressed() {
    let registry = release_registry("tcl9.0");
    // The handler is deliberately never defined here. `proc` declares a
    // script-kind callback barrier that clobbers every domain on its own,
    // so defining `observe` would suppress the report for a reason that
    // has nothing to do with the trace, and this test would pass while
    // proving nothing. A trace prefix need not name a defined command.
    let source = "trace add execution llength enter observe\n\
                      llength {a b}\n\
                      llength {a b}";
    let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
        source,
        &registry,
        false,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );

    // Traced calls need no reusable-value graph, but retain executable IR.
    assert!(
        cu.top_level
            .semantic_facts
            .executable()
            .function()
            .is_some()
    );
    assert_eq!(
        find_redundancies_for_function(
            &registry,
            &cu.top_level,
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile())
        )
        .len(),
        0,
        "an execution-traced command is observably invoked twice"
    );

    // Control: with the registration removed, the same two calls report.
    let untraced = crate::compilation_unit::CompilationUnit::build_for_profile(
        "llength {a b}\nllength {a b}",
        &registry,
        false,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    assert_eq!(
        find_redundancies_for_function(
            &registry,
            &untraced.top_level,
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile())
        )
        .len(),
        1,
        "the trace registration must be the only reason the report is withheld"
    );
}

#[test]
fn function_level_gvn_declines_volatile_result_despite_closed_effects() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "test::volatile_cse",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        result_stability: Some(ResultStability::Volatile),
        world_effects: Some(tcl_registry::WorldEffectDescriptor::EMPTY),
        state_transitions: Some(tcl_registry::StateTransitionDescriptor::EMPTY),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
        "test::volatile_cse\ntest::volatile_cse",
        &registry,
        false,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );

    assert_eq!(
        find_redundancies_for_function(
            &registry,
            &cu.top_level,
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile())
        )
        .len(),
        0,
        "read-only volatile results must not become common subexpressions"
    );
}

#[test]
fn function_level_gvn_declines_clock_seconds_and_versioned_clock_context() {
    let registry = CommandRegistry::build_default();
    for source in [
        "clock seconds\nclock seconds",
        "clock add 0 1 day\nclock add 0 1 day",
    ] {
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            &registry,
            false,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        assert_eq!(
            find_redundancies_for_function(
                &registry,
                &cu.top_level,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
                )
            )
            .len(),
            0,
            "clock result dependencies must make production GVN abstain: {source}"
        );
    }
}
