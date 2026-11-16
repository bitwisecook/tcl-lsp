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

//! Reuse of stable calls under world mutations: traces, renames, aliases, interpreters, namespaces and fail-closed operands.

use super::*;

/// Production function-level GVN over one Tcl 9.0 top-level script.
///
/// Every acceptance script below is straight-line top-level source, so the
/// executable semantic sidecar must exist; a decline there would make an
/// empty result meaningless.
fn stable_call_findings(registry: &CommandRegistry, source: &str) -> Vec<RedundantComputation> {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let registry = &registry.project_for_profile(profile);
    let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
        source, registry, false, profile,
    );
    assert!(
        cu.top_level
            .semantic_facts
            .executable()
            .function()
            .is_some(),
        "executable semantic source must be retained for {source}: {:?}",
        cu.top_level.semantic_facts.executable(),
    );
    let findings = find_redundancies_for_function(registry, &cu.top_level, Some(profile));
    if !findings.is_empty() {
        assert!(
            cu.top_level
                .semantic_facts
                .executable()
                .world_state_ssa()
                .is_some(),
            "reuse requires world-state SSA for {source}",
        );
    }
    findings
}

/// How many stable-call reuse findings one top-level script produces.
fn stable_call_reports(registry: &CommandRegistry, source: &str) -> usize {
    let findings = stable_call_findings(registry, source);
    assert!(
        findings
            .iter()
            .all(|finding| finding.code == DiagCode::O105),
        "stable-call reuse must only ever report O105: {source}"
    );
    findings.len()
}

/// Two repeated `llength` calls with no interposed statement.
const PLAIN_REPEATED_LLENGTH: &str = "llength {a b}\nllength {a b}";

#[test]
fn literal_execution_trace_removal_restores_stable_call_reuse() {
    let registry = CommandRegistry::build_default();
    assert_eq!(
        stable_call_reports(
            &registry,
            "trace add execution llength enter observe\n\
                 llength {a b}\n\
                 llength {a b}",
        ),
        0,
        "control: the live enter trace observes both invocations"
    );
    assert_eq!(
        stable_call_reports(
            &registry,
            "trace add execution llength enter observe\n\
                 trace remove execution llength enter observe\n\
                 llength {a b}\n\
                 llength {a b}",
        ),
        1,
        "an exactly matching literal removal retires the registration on the success edge"
    );
}

#[test]
fn dynamic_execution_trace_removal_abstains() {
    let registry = CommandRegistry::build_default();
    assert_eq!(
        stable_call_reports(
            &registry,
            "proc observe args {incr ::seen}\n\
                 set ::seen 0\n\
                 set h [info nameofexecutable]\n\
                 trace add execution llength enter observe\n\
                 trace remove execution llength enter $h\n\
                 llength {a b}\n\
                 llength {a b}",
        ),
        0,
        "a dynamic removal prefix cannot be proven to match the live registration"
    );
    assert_eq!(
        stable_call_reports(
            &registry,
            "proc observe args {incr ::seen}\n\
                 set ::seen 0\n\
                 set h observe\n\
                 trace add execution llength enter observe\n\
                 trace remove execution llength enter $h\n\
                 llength {a b}\n\
                 llength {a b}",
        ),
        1,
        "control: the frozen variable prefix exactly matches the live registration"
    );
}

#[test]
fn command_rename_trace_on_the_candidate_suppresses_reuse() {
    let registry = CommandRegistry::build_default();
    assert_eq!(
        stable_call_reports(
            &registry,
            "trace add command llength rename observe\n\
                 llength {a b}\n\
                 llength {a b}",
        ),
        0,
        "a rename/delete trace on the candidate is a live command-trace observer"
    );
    assert_eq!(
        stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
        1,
        "control: removing the command trace restores reuse"
    );
}

#[test]
fn step_execution_trace_on_an_unrelated_command_suppresses_every_site() {
    let registry = CommandRegistry::build_default();
    assert_eq!(
        stable_call_reports(
            &registry,
            "trace add execution observe enterstep observe\n\
                 llength {a b}\n\
                 llength {a b}",
        ),
        0,
        "a step trace observes commands nested inside its target, so it is a global hazard"
    );
    assert_eq!(
        stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
        1,
        "control: removing the step trace restores reuse"
    );
}

#[test]
fn non_step_execution_trace_on_an_unrelated_command_keeps_reuse() {
    let registry = CommandRegistry::build_default();
    assert_eq!(
        stable_call_reports(
            &registry,
            "proc observe {args} {}\n\
                 trace add execution observe enter observe\n\
                 llength {a b}\n\
                 llength {a b}",
        ),
        1,
        "an enter trace only observes its own target, so the candidate stays stable"
    );
}

#[test]
fn failed_execution_trace_installation_does_not_reach_later_calls() {
    let registry = CommandRegistry::build_default();
    assert_eq!(
        stable_call_reports(
            &registry,
            "trace add execution absent enter absent\nllength {a b}\nllength {a b}",
        ),
        0,
        "a missing traced command raises before either candidate executes"
    );
}

#[test]
fn renaming_the_candidate_command_suppresses_reuse() {
    let registry = CommandRegistry::build_default();
    // `rename` declares a trace-only current-interpreter callback barrier,
    // which the contents lattice discharges here because no trace is live.
    // What is left is the `CommandBinding(Move)` transition, which records
    // both subjects — so the candidate fails on CommandBinding alone.
    assert_eq!(
        stable_call_reports(
            &registry,
            "rename llength llen\nllength {a b}\nllength {a b}",
        ),
        0,
        "the candidate's binding is moved between the two invocations"
    );
    assert_eq!(
        stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
        1,
        "control: removing the rename restores reuse"
    );
}

#[test]
fn renaming_an_unrelated_command_keeps_reuse_per_subject() {
    let registry = CommandRegistry::build_default();
    // The per-domain, per-subject contract: the identity ledger records
    // only `lreverse` and `lrev`, so a differently-named candidate keeps a
    // complete CommandBinding proof.
    assert_eq!(
        stable_call_reports(
            &registry,
            "rename lreverse lrev\nllength {a b}\nllength {a b}",
        ),
        1,
        "renaming another command cannot disturb the candidate's binding"
    );
    assert_eq!(
        stable_call_reports(
            &registry,
            "rename llength llen\nllength {a b}\nllength {a b}",
        ),
        0,
        "contrast: renaming the candidate itself does suppress"
    );
}

#[test]
fn aliasing_over_the_candidate_command_suppresses_reuse() {
    let registry = CommandRegistry::build_default();
    // Two independent gates fail here: the `Alias` transition records the
    // candidate's own name, and the invocation's wildcard
    // interpreter-policy write marks current-interpreter policy unstable.
    assert_eq!(
        stable_call_reports(
            &registry,
            "interp alias {} llength {} list\nllength {a b}\nllength {a b}",
        ),
        0,
        "an alias installed over the candidate rebinds the name being dispatched"
    );
    assert_eq!(
        stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
        1,
        "control: removing the alias restores reuse"
    );
}

#[test]
fn aliasing_an_unrelated_name_keeps_reuse() {
    let registry = CommandRegistry::build_default();
    // tclsh 8.4-9.1: after `interp alias {} shortcut {} list`, `llength`
    // is still `::llength`, untraced, and answers 2 for `{a b}`. The
    // `Alias` transition names only `shortcut`, so the candidate keeps
    // its per-subject binding proof.
    assert_eq!(
        stable_call_reports(
            &registry,
            "interp alias {} shortcut {} list\nllength {a b}\nllength {a b}",
        ),
        1,
        "an alias installed under another name cannot rebind the candidate"
    );
    assert_eq!(
        stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
        1,
        "control: removing the alias restores reuse"
    );
}

#[test]
fn installing_an_alias_into_a_child_interpreter_abstains() {
    let registry = CommandRegistry::build_default();
    // `interp create child` alone keeps the site eligible (see
    // `creating_a_child_interpreter_only_invalidates_the_child_binding`),
    // so the topology change is not what suppresses.  The alias names
    // `child` as its source interpreter, which puts the transition outside
    // the current interpreter; the trace-only callback barrier therefore
    // cannot be discharged (foreign trace registrations are not tracked)
    // and clobbers every domain.  A conservative abstention, not a bug.
    assert_eq!(
        stable_call_reports(
            &registry,
            "interp create child\n\
                 interp alias child foo {} list\n\
                 llength {a b}\n\
                 llength {a b}",
        ),
        0,
        "an alias reaching into a named child interpreter keeps its world barrier"
    );
    assert_eq!(
        stable_call_reports(
            &registry,
            "interp create child\nllength {a b}\nllength {a b}",
        ),
        1,
        "control: dropping only the alias statement restores reuse"
    );
}

#[test]
fn hiding_the_candidate_in_the_current_interpreter_suppresses_reuse() {
    let registry = CommandRegistry::build_default();
    // The empty interpreter path is Tcl's spelling of "this interpreter",
    // so the transition records the candidate's visible binding and marks
    // interpreter policy unstable — exactly the two domains llength needs.
    assert_eq!(
        stable_call_reports(
            &registry,
            "interp hide {} llength\nllength {a b}\nllength {a b}",
        ),
        0,
        "hiding the candidate in the current interpreter unbinds the dispatched name"
    );
    assert_eq!(
        stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
        1,
        "control: removing the hide restores reuse"
    );
}

#[test]
fn creating_a_safe_child_interpreter_keeps_reuse() {
    let registry = CommandRegistry::build_default();
    assert_eq!(
        stable_call_reports(
            &registry,
            "interp create -safe jail\nllength {a b}\nllength {a b}",
        ),
        1,
        "a child interpreter's safety policy is not the current interpreter's policy"
    );
}

#[test]
fn creating_a_child_interpreter_only_invalidates_the_child_binding() {
    let registry = CommandRegistry::build_default();
    // `interp create child` binds the command `child` in the current
    // interpreter and bumps interpreter topology.  The identity ledger
    // records only that subject, so an unrelated candidate stays provably
    // stable — the per-domain, per-subject invalidation contract.
    assert_eq!(
        stable_call_reports(
            &registry,
            "interp create child\nllength {a b}\nllength {a b}",
        ),
        1,
        "a fresh child-path binding cannot collide with the candidate's resolution"
    );
}

#[test]
fn namespace_import_suppresses_reuse() {
    let registry = CommandRegistry::build_default();
    assert_eq!(
        stable_call_reports(
            &registry,
            "namespace import ::tcl::mathfunc::*\nllength {a b}\nllength {a b}",
        ),
        0,
        "an import can shadow the candidate's resolution in the current namespace"
    );
    assert_eq!(
        stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
        1,
        "control: removing the import restores reuse"
    );
}

#[test]
fn namespace_unknown_abstains_on_its_declared_lookup_and_policy_writes() {
    let registry = CommandRegistry::build_default();
    // llength's dispatch dependencies are the irreducible BASE set, which
    // excludes UnknownHandling, so the `SetUnknown` transition alone would
    // leave the site eligible.  The invocation additionally declares
    // ordinary wildcard footprint writes to NamespaceLookup and
    // InterpreterPolicy, and those two domains *are* in BASE — hence a
    // conservative abstention rather than the predicted report.
    assert_eq!(
        stable_call_reports(
            &registry,
            "namespace unknown handler\nllength {a b}\nllength {a b}",
        ),
        0,
        "the invocation's wildcard lookup and policy writes widen two BASE domains"
    );
    assert_eq!(
        stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
        1,
        "control: removing the unknown-handler statement restores reuse"
    );
}

#[test]
fn eval_source_and_package_barriers_suppress_reuse() {
    let registry = CommandRegistry::build_default();
    for barrier in ["eval {set x 1}", "source lib.tcl", "package require json"] {
        assert_eq!(
            stable_call_reports(
                &registry,
                &format!("{barrier}\nllength {{a b}}\nllength {{a b}}"),
            ),
            0,
            "an arbitrary-script barrier widens every dispatch domain: {barrier}"
        );
    }
    assert_eq!(
        stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
        1,
        "control: without a barrier statement the same pair reuses"
    );
}

#[test]
fn variable_read_trace_on_an_argument_variable_suppresses_reuse() {
    let registry = CommandRegistry::build_default();
    assert_eq!(
        stable_call_reports(
            &registry,
            "set items {a b}\n\
                 trace add variable items read observe\n\
                 llength $items\n\
                 llength $items",
        ),
        0,
        "reading the traced variable runs arbitrary Tcl, so the operand word is inadmissible"
    );
    assert_eq!(
        stable_call_reports(&registry, "set items {a b}\nllength $items\nllength $items",),
        1,
        "control: with no variable trace the same argument reads are admissible"
    );
}

#[test]
fn variable_trace_removal_restores_argument_variable_reuse() {
    let registry = CommandRegistry::build_default();
    assert_eq!(
        stable_call_reports(
            &registry,
            "set items {a b}\n\
                 trace add variable items read observe\n\
                 trace remove variable items read observe\n\
                 llength $items\n\
                 llength $items",
        ),
        1,
        "an exactly matching removal retires the read trace and re-admits the operand word"
    );
}

#[test]
fn variable_write_trace_on_another_variable_keeps_reuse() {
    let registry = CommandRegistry::build_default();
    // The traced variable is written before the trace is installed and
    // never touched afterwards; the candidate's operands are literals, so
    // nothing can fire the trace between the two invocations.
    assert_eq!(
        stable_call_reports(
            &registry,
            "set other 1\n\
                 trace add variable other write observe\n\
                 llength {a b}\n\
                 llength {a b}",
        ),
        1,
        "a write trace on an untouched variable is not a hazard for literal operands"
    );
}

#[test]
fn procedure_units_abstain_under_the_unknown_world_entry_assumption() {
    let registry = CommandRegistry::build_default();
    let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
        "proc p {items} {\nllength $items\nllength $items\n}",
        &registry,
        false,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let procedure = fixture_id_if_present(&cu.procedures, "::p")
        .and_then(|id| cu.procedures.get(&id))
        .expect("the proc unit is built");
    assert!(
        !find_redundancies(
            &registry,
            &procedure.cfg,
            &procedure.ssa,
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile())
        )
        .is_empty(),
        "the legacy classifier still recognises the repeated pure call"
    );
    assert_eq!(
        find_redundancies_for_function(
            &registry,
            procedure,
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile())
        )
        .len(),
        0,
        "a proc body runs after arbitrary interposed history, so every site proof fails closed"
    );
}

#[test]
fn repeated_stable_calls_in_a_while_body_are_mapped() {
    let registry = CommandRegistry::build_default();
    let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
        "while {$go} {\nllength {a b}\nllength {a b}\n}",
        &registry,
        false,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    assert!(
        !find_redundancies(
            &registry,
            &cu.top_level.cfg,
            &cu.top_level.ssa,
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile())
        )
        .is_empty(),
        "the explicit legacy API still sees the nested string-classified calls"
    );
    assert!(
        !find_redundancies_for_function(
            &registry,
            &cu.top_level,
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile())
        )
        .is_empty(),
        "a loop body is now executable-IR blocks with a real back edge, so \
             the two identical pure calls in one iteration are mapped"
    );
}

#[test]
fn a_variable_store_write_between_candidates_keeps_reuse() {
    let registry = CommandRegistry::build_default();
    // The variable store is versioned but is not a dispatch domain, so an
    // untraced `set` between the two calls neither widens a ledger nor
    // changes a key fragment llength depends on.
    assert_eq!(
        stable_call_reports(&registry, "llength {a b}\nset x 1\nllength {a b}"),
        1,
        "an untraced variable write is not a dispatch-stability hazard"
    );
}

#[test]
fn command_substitution_in_an_operand_word_fails_closed() {
    let registry = CommandRegistry::build_default();
    assert_eq!(
        stable_call_reports(&registry, "llength [list a b]\nllength [list a b]"),
        0,
        "a nested command substitution is an inadmissible operand word and a world barrier"
    );
    assert_eq!(
        stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
        1,
        "control: the same calls with literal operands reuse"
    );
}

#[test]
fn argument_expansion_fails_closed() {
    let registry = CommandRegistry::build_default();
    // Every dispatch domain is still proven stable here; expansion is
    // declined purely on operand-word admissibility, because it changes
    // the argv shape the registry facts were resolved for.
    assert_eq!(
        stable_call_reports(&registry, "set l {a b}\nllength {*}$l\nllength {*}$l"),
        0,
        "an expanded word is never an admissible operand"
    );
}

#[test]
fn versioned_world_results_key_on_the_read_domain_version() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "test::versioned",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        result_stability: Some(ResultStability::ReadsVersionedWorld(&[
            tcl_registry::WorldStateDomain::VariableStore,
        ])),
        world_effects: Some(tcl_registry::WorldEffectDescriptor::EMPTY),
        state_transitions: Some(tcl_registry::StateTransitionDescriptor::EMPTY),
        ..tcl_registry::CommandSpec::DEFAULT
    });

    let versioned = registry
        .resolve_invocation(
            "test::versioned",
            &["x"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .expect("the synthetic spec resolves")
        .facts();
    assert!(resolved_invocation_is_versioned_world_gvn_candidate(
        &versioned
    ));
    assert!(!resolved_invocation_is_gvn_candidate(&versioned));

    let llength = registry
        .resolve_invocation(
            "llength",
            &["a b"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .expect("llength resolves")
        .facts();
    assert!(!resolved_invocation_is_versioned_world_gvn_candidate(
        &llength
    ));
    assert!(resolved_invocation_is_gvn_candidate(&llength));

    assert_eq!(
        stable_call_reports(&registry, "test::versioned x\ntest::versioned x"),
        1,
        "the declared VariableStore version is equal at both sites"
    );
    assert_eq!(
        stable_call_reports(&registry, "test::versioned x\nset y 1\ntest::versioned x",),
        0,
        "an interposed variable write bumps the read domain, so the two keys differ"
    );
}

#[test]
fn oo_class_creation_suppresses_reuse_through_its_constructor_barrier() {
    let registry = CommandRegistry::build_default();
    // llength's dispatch dependencies are BASE and exclude ObjectDispatch,
    // so the `oo::define` object-dispatch transition is not what suppresses
    // here.  Both `oo::class create` and `oo::define` declare a callback
    // world barrier — a constructor and a definition script run arbitrary
    // Tcl — and that barrier clobbers every domain.  `oo::class create`
    // alone is already enough.
    assert_eq!(
        stable_call_reports(
            &registry,
            "oo::class create C\n\
                 oo::define C method m {} {}\n\
                 llength {a b}\n\
                 llength {a b}",
        ),
        0,
        "the oo definition statements declare callback world barriers"
    );
    assert_eq!(
        stable_call_reports(
            &registry,
            "oo::class create C\nllength {a b}\nllength {a b}",
        ),
        0,
        "the class-creation barrier alone already widens every dispatch domain"
    );
    assert_eq!(
        stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
        1,
        "control: without the oo statements the same pair reuses"
    );
}
