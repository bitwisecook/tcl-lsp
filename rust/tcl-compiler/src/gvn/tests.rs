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

use crate::interprocedural::test_body_identity::fixture_id_if_present;

use tcl_dialect::model::{Family, SurfaceQuery};

/// The default document's `${…}` close rule — these unit tests build no
/// dialect profile, so they exercise the same style the lexer would use.
const STYLE: tcl_dialect::BracedVarStyle = tcl_dialect::BracedVarStyle::Tcl9Nesting;

use super::*;
use crate::semantic_analysis::ExecutableAnalysisAvailability;

/// The catalogue a document under `environment` is analysed against: the
/// shipped registry projected for that release's analyser profile.
fn release_registry(environment: &str) -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment(environment).analyser_profile(),
    )
}

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

fn entry(key: &[&str], block: &str, idx: usize) -> ValueEntry {
    let key_owned: ExprKey = key.iter().map(|s| (*s).into()).collect();
    ValueEntry {
        key: key_owned.clone(),
        block: block.into(),
        statement_index: idx,
        span: Span::new(0, 0),
        expression_text: key_owned.join(" "),
    }
}

#[test]
fn scoped_table_root_scope_always_present() {
    let t = ScopedValueTable::new();
    assert_eq!(t.scope_depth(), 1);
    assert!(t.lookup(&vec!["call".into(), "foo".into()]).is_none());
}

#[test]
fn insert_and_lookup_in_root() {
    let mut t = ScopedValueTable::new();
    let e = entry(&["call", "llength", "$x@1"], "entry", 2);
    t.insert(e.clone());
    let key: ExprKey = vec!["call".into(), "llength".into(), "$x@1".into()];
    assert_eq!(t.lookup(&key), Some(&e));
}

#[test]
fn pop_scope_discards_inner_entries_only() {
    let mut t = ScopedValueTable::new();
    t.insert(entry(&["call", "root"], "b", 0));
    t.push_scope();
    t.insert(entry(&["call", "inner"], "b", 1));
    assert_eq!(t.total_entries(), 2);
    t.pop_scope();
    let root_key: ExprKey = vec!["call".into(), "root".into()];
    let inner_key: ExprKey = vec!["call".into(), "inner".into()];
    assert!(t.lookup(&root_key).is_some());
    assert!(t.lookup(&inner_key).is_none());
}

#[test]
fn pop_scope_preserves_root_scope() {
    let mut t = ScopedValueTable::new();
    t.pop_scope(); // Should be a no-op; root always survives.
    t.pop_scope();
    assert_eq!(t.scope_depth(), 1);
}

#[test]
fn lookup_shadows_outer_scope_first() {
    let mut t = ScopedValueTable::new();
    let key: ExprKey = vec!["call".into(), "f".into()];
    t.insert(entry(&["call", "f"], "outer", 0));
    t.push_scope();
    t.insert(entry(&["call", "f"], "inner", 5));
    assert_eq!(t.lookup(&key).unwrap().block, "inner");
    t.pop_scope();
    assert_eq!(t.lookup(&key).unwrap().block, "outer");
}

#[test]
fn kill_all_drops_every_scope() {
    let mut t = ScopedValueTable::new();
    t.insert(entry(&["call", "a"], "b", 0));
    t.push_scope();
    t.insert(entry(&["call", "b"], "b", 1));
    t.kill_all();
    // `kill_all` drops every entry but preserves the scope-stack depth, so
    // the push/pop discipline survives. Collapsing the
    // stack would strand later inserts in a never-popped root scope.
    assert_eq!(t.scope_depth(), 2);
    assert_eq!(t.total_entries(), 0);
}

#[test]
fn kill_all_mid_branch_does_not_leak_to_sibling() {
    // replicated at the table level exactly as the
    // dominator walk drives it: enter block, enter then-arm, a state write
    // kills, record an occurrence, leave the then-arm, enter the else-arm.
    // The then-arm's occurrence must not be visible in the sibling.
    let key: ExprKey = vec!["call".into(), "llength".into(), "$lst".into()];
    let mut t = ScopedValueTable::new(); // root, depth 1
    t.push_scope(); // enter condition block, depth 2
    t.push_scope(); // enter then-arm, depth 3
    t.kill_all(); // a state write invalidates everything
    t.insert(entry(&["call", "llength", "$lst"], "then", 0));
    t.pop_scope(); // leave then-arm → depth 2, entry dropped
    t.push_scope(); // enter else-arm, depth 3
    assert!(
        t.lookup(&key).is_none(),
        "then-arm occurrence leaked into the else-arm after kill_all",
    );
}

#[test]
fn redundant_computation_constructor() {
    let r = RedundantComputation::new(
        Span::new(10, 20),
        Span::new(0, 5),
        "llength $x",
        DiagCode::O105,
        "message",
    );
    assert_eq!(r.expression_text, "llength $x");
    assert_eq!(r.code, DiagCode::O105);
    assert_eq!(r.span.start(), 10);
    assert_eq!(r.first_span.end(), 5);
}

// Canonicalisation + messages.

/// A bare SSA shell (no blocks) for tests that only need the variable
/// interner to resolve canonicalisation names.
fn bare_ssa() -> SsaFunction {
    SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()])
}

#[test]
fn canonicalise_empty_uses_returns_input() {
    let ssa = bare_ssa();
    let uses = HashMap::new();
    assert_eq!(canonicalise_word("foo", &uses, &ssa, STYLE), "foo");
}

#[test]
fn canonicalise_replaces_bare_and_braced() {
    let mut ssa = bare_ssa();
    let mut uses = HashMap::new();
    uses.insert(ssa.intern_var("x"), 3);
    assert_eq!(canonicalise_word("$x", &uses, &ssa, STYLE), "$x@3");
    assert_eq!(canonicalise_word("${x}", &uses, &ssa, STYLE), "$x@3");
}

/// The `${…}` closer follows the release rule, so a
/// nested-brace name is *versioned* rather than copied through verbatim.
///
/// The verbatim copy is the unsound half: two occurrences of `${a{b}c}` at
/// different SSA versions both render to the same literal text, so their
/// `ExprKey`s match and GVN reports the second as redundant — a CSE that
/// reuses a stale value: `set {a{b}c} 7; subst {${a{b}c}}` is `7`
/// on tclsh 9.0.4 and `can't read "a{b"` on 8.6.16.
#[test]
fn canonicalise_braced_var_follows_the_release_rule() {
    use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
    let mut ssa = bare_ssa();
    let mut uses = HashMap::new();
    let nested = ssa.intern_var("a{b}c");
    let first = ssa.intern_var("a{b");
    uses.insert(nested, 4);
    uses.insert(first, 9);

    // 9.x: the whole name is `a{b}c`, so the reference carries its version.
    assert_eq!(
        canonicalise_word("${a{b}c}", &uses, &ssa, Tcl9Nesting),
        "$a{b}c@4"
    );
    // 8.x: the name ends at the first `}` and `c}` is ordinary word text.
    assert_eq!(
        canonicalise_word("${a{b}c}", &uses, &ssa, FirstClose),
        "$a{b@9c}"
    );

    // Two versions of the same nested name must not canonicalise alike —
    // the property the unsound CSE turned on.
    let mut other = HashMap::new();
    other.insert(nested, 5);
    assert_ne!(
        canonicalise_word("${a{b}c}", &uses, &ssa, Tcl9Nesting),
        canonicalise_word("${a{b}c}", &other, &ssa, Tcl9Nesting),
    );
}

#[test]
fn canonicalise_sorts_by_name_length_desc() {
    // `$longname` must be replaced before `$long` so the
    // longer name is not partially matched.
    let mut ssa = bare_ssa();
    let mut uses = HashMap::new();
    uses.insert(ssa.intern_var("long"), 1);
    uses.insert(ssa.intern_var("longname"), 2);
    let out = canonicalise_word("$longname$long", &uses, &ssa, STYLE);
    assert_eq!(out, "$longname@2$long@1");
}

#[test]
fn canonicalise_ignores_unmentioned_variables() {
    let ssa = bare_ssa();
    let uses = HashMap::new();
    assert_eq!(canonicalise_word("$x", &uses, &ssa, STYLE), "$x");
}

#[test]
fn build_call_key_for_pure_command() {
    let mut ssa = bare_ssa();
    let mut uses = HashMap::new();
    uses.insert(ssa.intern_var("x"), 3);
    let args = vec!["$x".into(), "literal".into()];
    let key = build_call_key("llength", &args, &uses, &ssa, STYLE);
    assert_eq!(
        key,
        vec![
            "call".to_string(),
            "llength".into(),
            "$x@3".into(),
            "literal".into()
        ]
    );
}

#[test]
fn format_expression_text_no_args() {
    let args: Vec<String> = Vec::new();
    assert_eq!(format_expression_text("clock", &args), "clock");
}

#[test]
fn format_expression_text_with_args() {
    let args: Vec<String> = vec!["$x".into(), "literal".into()];
    assert_eq!(
        format_expression_text("llength", &args),
        "llength $x literal"
    );
}

#[test]
fn message_builders_include_expression_text() {
    assert!(full_redundancy_message("llength $x").contains("llength $x"));
    assert!(full_redundancy_message("llength $x").contains("local variable"));
    assert!(partial_redundancy_message("dict get $d k").contains("partially redundant"));
    assert!(loop_invariant_message("expr {$x + 1}").contains("loop-invariant"));
}

// Statement-level helpers.

fn call_stmt(cmd: &str, args: &[&str]) -> Statement {
    Statement::Call {
        span: Span::new(0, 0),
        command: cmd.into(),
        canonical_command: None,
        args: args.iter().map(|s| (*s).into()).collect(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

#[test]
fn is_pure_command_picks_up_registry_pure_trait() {
    let registry = CommandRegistry::build_default();
    // `expr` is marked PURE / PURE_EVALUATION in the registry.
    assert!(is_pure_command(
        &registry,
        "expr",
        &["{1 + 2}".into()],
        None
    ));
    // `set` is variable-assigning — not pure.
    assert!(!is_pure_command(
        &registry,
        "set",
        &["x".into(), "1".into()],
        None
    ));
}

#[test]
fn is_pure_command_recurses_into_impure_substitution() {
    // O106: `expr` is pure, but an arg that embeds
    // an impure `[incr c]` substitution makes the whole invocation
    // impure — its value depends on the per-call side effect, so it
    // must not be hoisted as loop-invariant.
    let registry = CommandRegistry::build_default();
    assert!(
        !is_pure_command(&registry, "expr", &["[incr c]".into()], None),
        "outer pure `expr` wrapping impure `[incr c]` must be impure",
    );
    // A pure nested substitution keeps the outer call pure.
    assert!(
        is_pure_command(&registry, "expr", &["[expr {1 + 1}]".into()], None),
        "outer pure `expr` wrapping a pure subst stays pure",
    );
    // Deeper nesting: the impurity is two levels down.
    assert!(
        !is_pure_command(&registry, "expr", &["[expr [incr c]]".into()], None),
        "nested impure subst at any depth must propagate",
    );
}

/// A pure command (`expr`) gates to non-pure once its
/// canonical name appears in `traced_commands`.
#[test]
fn is_pure_command_with_traces_traced_command_not_pure() {
    let registry = CommandRegistry::build_default();
    let mut traced = std::collections::BTreeSet::new();
    traced.insert("foo".to_string());
    // `foo` is unknown to the registry but in the trace set —
    // never pure.
    assert!(!is_pure_command_with_traces(
        &registry,
        "foo",
        &["arg".into()],
        None,
        &traced,
        false,
    ));
}

/// `has_dynamic_trace=true` widens the gate to *every*
/// command (even ones the registry marks PURE).
#[test]
fn is_pure_command_with_traces_dynamic_trace_inhibits_all() {
    let registry = CommandRegistry::build_default();
    let traced = std::collections::BTreeSet::new();
    // `expr` is PURE per the registry, but `has_dynamic_trace=true`
    // means any call could have a trace registered at runtime.
    assert!(!is_pure_command_with_traces(
        &registry,
        "expr",
        &["{1 + 2}".into()],
        None,
        &traced,
        true,
    ));
}

/// Regression — untraced pure commands stay pure.
#[test]
fn is_pure_command_with_traces_untraced_command_still_pure() {
    let registry = CommandRegistry::build_default();
    let traced = std::collections::BTreeSet::new();
    assert!(is_pure_command_with_traces(
        &registry,
        "expr",
        &["{1 + 2}".into()],
        None,
        &traced,
        false,
    ));
}

/// `::ns::foo` and `ns::foo` look up identically in the
/// trace set (mirrors `populate_trace_facts`'s `::`-strip).
#[test]
fn is_pure_command_with_traces_qualified_name_canonicalised() {
    let registry = CommandRegistry::build_default();
    let mut traced = std::collections::BTreeSet::new();
    traced.insert("ns::foo".to_string());
    assert!(!is_pure_command_with_traces(
        &registry,
        "::ns::foo",
        &[],
        None,
        &traced,
        false,
    ));
    assert!(!is_pure_command_with_traces(
        &registry,
        "ns::foo",
        &[],
        None,
        &traced,
        false,
    ));
}

#[test]
fn opaque_native_call_kills_common_value_numbering_without_text_classification() {
    let registry = CommandRegistry::build_default();
    let native = crate::ir::native_call_for_test(b"opaque \xff");
    assert!(statement_writes_state_for_gvn(
        &registry,
        &native,
        registry.profile(),
        GvnLegalitySource::Common(None),
    ));
    let mut ssa = SsaFunction::trivial("::f", BlockId(0), vec!["entry".into()]);
    let statement = ssa_stmt_for(&mut ssa, native, None);
    assert_eq!(
        statement_occurrences(&registry, &statement, "entry", 0, registry.profile(), &ssa),
        [] as [crate::gvn::ExprOccurrence; 0]
    );
}

#[test]
fn statement_writes_state_true_for_barrier() {
    let registry = CommandRegistry::build_default();
    let barrier = Statement::Barrier {
        span: Span::new(0, 0),
        reason: "eval".into(),
        command: "eval".into(),
        canonical_command: None,
        args: vec!["script".into()],
        tokens: None,
    };
    assert!(statement_writes_state(&registry, &barrier, None));
}

#[test]
fn statement_writes_state_true_for_global_write() {
    let registry = CommandRegistry::build_default();
    // Global-scope `set` writes GLOBAL_STATE — must invalidate.
    let set_global = call_stmt("set", &["::foo::bar", "1"]);
    assert!(statement_writes_state(&registry, &set_global, None));
}

#[test]
fn statement_writes_state_false_for_proc_local_set() {
    let registry = CommandRegistry::build_default();
    // Proc-local `set x 1` only writes the local — no region.
    let set_local = call_stmt("set", &["x", "1"]);
    assert!(!statement_writes_state(&registry, &set_local, None));
}

#[test]
fn statement_writes_state_false_for_pure_expr() {
    let registry = CommandRegistry::build_default();
    let expr_stmt = call_stmt("expr", &["{1 + 2}"]);
    assert!(!statement_writes_state(&registry, &expr_stmt, None));
}

#[test]
fn statement_occurrences_skips_impure_commands() {
    let registry = CommandRegistry::build_default();
    // `set` is impure — statement_occurrences emits nothing.
    let stmt_ssa = SsaStatement {
        statement: call_stmt("set", &["x", "1"]),
        uses: HashMap::new(),
        defs: HashMap::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    let occurrences = statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa());
    assert_eq!(occurrences, [] as [crate::gvn::ExprOccurrence; 0]);
}

#[test]
fn statement_occurrences_emits_nothing_for_non_call_stmts() {
    let registry = CommandRegistry::build_default();
    let stmt_ssa = SsaStatement {
        statement: Statement::AssignConst {
            span: Span::new(0, 0),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        },
        uses: HashMap::new(),
        defs: HashMap::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    assert_eq!(
        statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa()).len(),
        0
    );
}

#[test]
fn is_worth_reporting_unknown_command_is_false() {
    let registry = CommandRegistry::build_default();
    assert!(!is_worth_reporting(&registry, "__nonexistent"));
}

// find_redundancies driver.

use crate::cfg::{Block, Function, Terminator};
use crate::ssa::{SsaBlock, SsaStatement};
use std::collections::HashMap as Map;

/// The [`BlockId`] a block name was interned to in `cfg`.
fn id_of(cfg: &Function, name: &str) -> BlockId {
    cfg.block_id(name).expect("interned")
}

fn llength_call() -> Statement {
    llength_call_at(0, 0)
}

fn llength_call_at(start: u32, end: u32) -> Statement {
    Statement::Call {
        span: Span::new(start, end),
        command: "llength".into(),
        canonical_command: None,
        args: vec!["$x".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

fn ssa_stmt_for(ssa: &mut SsaFunction, stmt: Statement, uses_x_ver: Option<u32>) -> SsaStatement {
    let mut uses = Map::new();
    if let Some(v) = uses_x_ver {
        uses.insert(ssa.intern_var("x"), v);
    }
    SsaStatement {
        statement: stmt,
        uses,
        defs: Map::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

fn empty_ssa_block(name: &str) -> SsaBlock {
    SsaBlock {
        name: name.into(),
        phis: Vec::new(),
        statements: Vec::new(),
        entry_versions: Map::new(),
        exit_versions: Map::new(),
    }
}

/// Every block of a hand-built CFG, as the executable set. These tests
/// exercise the loop / availability machinery on CFGs with no SCCP run,
/// so they hand it the whole CFG, stating the premise at the call site
/// instead of deriving it inside the pass.
fn all_blocks(cfg: &Function) -> std::collections::HashSet<BlockId> {
    cfg.blocks.keys().copied().collect()
}

#[test]
fn find_redundancies_detects_same_block_duplicate() {
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    let entry_blk = cfg.blocks.get_mut(&cfg.entry).unwrap();
    entry_blk.statements.push(llength_call());
    entry_blk.statements.push(llength_call());
    entry_blk.terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.dominator_tree.insert(cfg.entry, Vec::new());
    let mut ssa_entry = empty_ssa_block("entry");
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(cfg.entry, ssa_entry);

    let results = find_redundancies(&registry, &cfg, &ssa, None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].code, DiagCode::O105);
    assert!(results[0].expression_text.contains("llength"));
}

#[test]
fn find_redundancies_ignores_different_ssa_version() {
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    let entry_blk = cfg.blocks.get_mut(&cfg.entry).unwrap();
    entry_blk.statements.push(llength_call());
    entry_blk.statements.push(llength_call());
    entry_blk.terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.dominator_tree.insert(cfg.entry, Vec::new());
    let mut ssa_entry = empty_ssa_block("entry");
    // Same expression but different SSA versions of $x → no
    // redundancy.
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(2)));
    ssa.blocks.insert(cfg.entry, ssa_entry);

    let results = find_redundancies(&registry, &cfg, &ssa, None);
    assert_eq!(results, [] as [crate::gvn::RedundantComputation; 0]);
}

#[test]
fn find_redundancies_global_write_invalidates_cache() {
    let registry = CommandRegistry::build_default();
    // entry: llength $x; set ::g 1; llength $x
    let mut cfg = Function::new("::top", "entry");
    let entry_blk = cfg.blocks.get_mut(&cfg.entry).unwrap();
    entry_blk.statements.push(llength_call());
    entry_blk.statements.push(Statement::Call {
        span: Span::new(0, 0),
        command: "set".into(),
        canonical_command: None,
        args: vec!["::g".into(), "1".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    });
    entry_blk.statements.push(llength_call());
    entry_blk.terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.dominator_tree.insert(cfg.entry, Vec::new());
    let mut ssa_entry = empty_ssa_block("entry");
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa_entry.statements.push(ssa_stmt_for(
        &mut ssa,
        cfg.blocks[&cfg.entry].statements[1].clone(),
        None,
    ));
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(cfg.entry, ssa_entry);

    // The global write should invalidate — no redundancy reported.
    let results = find_redundancies(&registry, &cfg, &ssa, None);
    assert_eq!(results, [] as [crate::gvn::RedundantComputation; 0]);
}

#[test]
fn find_redundancies_descends_dominator_tree() {
    let registry = CommandRegistry::build_default();
    // entry: llength $x
    // dom_child: llength $x   (should trigger)
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let child = cfg.intern_block("child");
    cfg.blocks.insert(child, Block::new("child"));
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(llength_call());
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Goto {
        target: child,
        span: None,
    });
    cfg.blocks
        .get_mut(&child)
        .unwrap()
        .statements
        .push(llength_call());
    cfg.blocks.get_mut(&child).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.dominator_tree.insert(entry, vec![child]);
    ssa.dominator_tree.insert(child, Vec::new());
    let mut e = empty_ssa_block("entry");
    e.statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    let mut c = empty_ssa_block("child");
    c.statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(entry, e);
    ssa.blocks.insert(child, c);

    let results = find_redundancies(&registry, &cfg, &ssa, None);
    assert_eq!(results.len(), 1);
}

// Embedded command-substitution scanning.

#[test]
fn scan_bracketed_commands_simple() {
    let out = scan_bracketed_commands("[llength $x]");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].0, "llength");
    assert_eq!(out[0].1, vec!["$x".to_string()]);
}

#[test]
fn scan_bracketed_commands_nested() {
    let out = scan_bracketed_commands("[set v [lindex $x 0]]");
    // Outer pair + inner pair.
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].0, "set");
    assert_eq!(out[1].0, "lindex");
}

#[test]
fn scan_bracketed_commands_skips_braced_regions() {
    let out = scan_bracketed_commands("{[not a command]}");
    assert_eq!(
        out,
        [] as [(std::string::String, std::vec::Vec<std::string::String>); 0]
    );
}

#[test]
fn scan_bracketed_commands_handles_backslash_escape() {
    // `\[` shouldn't start a region.
    let out = scan_bracketed_commands("hello \\[world]");
    assert_eq!(
        out,
        [] as [(std::string::String, std::vec::Vec<std::string::String>); 0]
    );
}

#[test]
fn split_cmd_text_braced_and_quoted_words() {
    let (cmd, args) = split_cmd_text("list {a b c} \"d e\" f").unwrap();
    assert_eq!(cmd, "list");
    assert_eq!(
        args,
        vec!["{a b c}".to_string(), "\"d e\"".into(), "f".into()]
    );
}

#[test]
fn statement_occurrences_includes_embedded_pure_cmd() {
    let registry = CommandRegistry::build_default();
    // set x [llength $y] — an impure `set` with an embedded
    // pure `llength`. The top-level Call is skipped (impure),
    // but the embedded `llength` should be reported.
    let stmt_ssa = SsaStatement {
        statement: Statement::Call {
            span: Span::new(0, 0),
            command: "set".into(),
            canonical_command: None,
            args: vec!["x".into(), "[llength $y]".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        },
        uses: HashMap::new(),
        defs: HashMap::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    let occ = statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa());
    assert_eq!(occ.len(), 1);
    assert_eq!(occ[0].expression_text, "llength $y");
}

#[test]
fn statement_occurrences_includes_embedded_in_assign_value() {
    let registry = CommandRegistry::build_default();
    let stmt_ssa = SsaStatement {
        statement: Statement::AssignValue {
            span: Span::new(0, 0),
            name: "len".into(),
            name_braced: false,
            value: "[llength $y]".into(),
            value_needs_backsubst: false,
            tokens: None,
        },
        uses: HashMap::new(),
        defs: HashMap::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    let occ = statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa());
    assert_eq!(occ.len(), 1);
    assert_eq!(occ[0].expression_text, "llength $y");
}

#[test]
fn find_redundancies_empty_for_no_blocks() {
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    // Intern a name with no matching block so the SSA entry is absent
    // from `cfg.blocks` and the walk early-returns.
    let nonexistent = cfg.intern_block("nonexistent");
    let ssa = SsaFunction::trivial(cfg.name.clone(), nonexistent, cfg.block_names().to_vec());
    let results = find_redundancies(&registry, &cfg, &ssa, None);
    assert_eq!(results, [] as [crate::gvn::RedundantComputation; 0]);
}

// Loop-invariant detection.

#[test]
fn find_loop_invariants_detects_hoistable_llength() {
    let registry = CommandRegistry::build_default();
    // header: branch on $i < 10 → body → header (back edge)
    // body: llength $x (invariant w.r.t. loop-defined $i)
    let mut cfg = Function::new("::top", "header");
    let header = cfg.entry;
    let body = cfg.intern_block("body");
    let exit = cfg.intern_block("exit");
    cfg.blocks.insert(body, Block::new("body"));
    cfg.blocks.insert(exit, Block::new("exit"));
    cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
        condition: crate::expr_ast::ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        true_target: body,
        false_target: exit,
        span: None,
        condition_base: None,
    });
    cfg.blocks
        .get_mut(&body)
        .unwrap()
        .statements
        .push(llength_call());
    cfg.blocks.get_mut(&body).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });
    cfg.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    // header dominates body and exit.
    ssa.idom.insert(header, None);
    ssa.idom.insert(body, Some(header));
    ssa.idom.insert(exit, Some(header));
    ssa.dominator_tree.insert(header, vec![body, exit]);
    ssa.dominator_tree.insert(body, Vec::new());
    ssa.dominator_tree.insert(exit, Vec::new());

    let h = empty_ssa_block("header");
    let mut b = empty_ssa_block("body");
    // $x's SSA version comes from outside the loop (x@1).
    b.statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(header, h);
    ssa.blocks.insert(body, b);
    ssa.blocks.insert(exit, empty_ssa_block("exit"));

    let results = find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
    assert_eq!(results.len(), 1);
    // Canonical: loop-invariant code motion is O106 (not O107,
    // which is unreachable dead code).
    assert_eq!(results[0].code, DiagCode::O106);
    assert!(results[0].expression_text.contains("llength"));
}

#[test]
fn find_loop_invariants_skips_branch_that_does_not_dominate_latch() {
    // Loop:  header → cond → {then, latch}; then → latch; latch → header.
    // The invariant `llength $x` sits in `then`, which does NOT dominate
    // the latch (cond can reach the latch directly), so it runs only on
    // some iterations — the latch-dominance gate must suppress O106.
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "header");
    let header = cfg.entry;
    for b in ["cond", "then", "latch", "exit"] {
        let id = cfg.intern_block(b);
        cfg.blocks.insert(id, Block::new(b));
    }
    let cond = id_of(&cfg, "cond");
    let then = id_of(&cfg, "then");
    let latch = id_of(&cfg, "latch");
    let exit = id_of(&cfg, "exit");
    cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
        condition: crate::expr_ast::ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        true_target: cond,
        false_target: exit,
        span: None,
        condition_base: None,
    });
    cfg.blocks.get_mut(&cond).unwrap().terminator = Some(Terminator::Branch {
        condition: crate::expr_ast::ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        true_target: then,
        false_target: latch,
        span: None,
        condition_base: None,
    });
    cfg.blocks
        .get_mut(&then)
        .unwrap()
        .statements
        .push(llength_call());
    cfg.blocks.get_mut(&then).unwrap().terminator = Some(Terminator::Goto {
        target: latch,
        span: None,
    });
    cfg.blocks.get_mut(&latch).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });
    cfg.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(header, None);
    ssa.idom.insert(cond, Some(header));
    ssa.idom.insert(then, Some(cond));
    // The latch is reachable from `cond` both directly and via `then`,
    // so its immediate dominator is `cond`, not `then`.
    ssa.idom.insert(latch, Some(cond));
    ssa.idom.insert(exit, Some(header));
    ssa.dominator_tree.insert(header, vec![cond, exit]);
    ssa.dominator_tree.insert(cond, vec![then, latch]);
    for b in [then, latch, exit] {
        ssa.dominator_tree.insert(b, Vec::new());
    }

    ssa.blocks.insert(header, empty_ssa_block("header"));
    ssa.blocks.insert(cond, empty_ssa_block("cond"));
    let mut then_b = empty_ssa_block("then");
    then_b
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(then, then_b);
    ssa.blocks.insert(latch, empty_ssa_block("latch"));
    ssa.blocks.insert(exit, empty_ssa_block("exit"));

    let results = find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
    assert_eq!(
        results.len(),
        0,
        "invariant behind an in-loop branch must not be hoisted, got {results:?}",
    );
}

#[test]
fn find_loop_invariants_skips_loop_defined_var() {
    let registry = CommandRegistry::build_default();
    // As above, but `$i` is defined inside the loop — llength
    // uses `$i` → not loop-invariant.
    let mut cfg = Function::new("::top", "header");
    let header = cfg.entry;
    let body = cfg.intern_block("body");
    let exit = cfg.intern_block("exit");
    cfg.blocks.insert(body, Block::new("body"));
    cfg.blocks.insert(exit, Block::new("exit"));
    cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
        condition: crate::expr_ast::ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        true_target: body,
        false_target: exit,
        span: None,
        condition_base: None,
    });
    let llength_on_i = Statement::Call {
        span: Span::new(0, 0),
        command: "llength".into(),
        canonical_command: None,
        args: vec!["$i".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    cfg.blocks
        .get_mut(&body)
        .unwrap()
        .statements
        .push(llength_on_i.clone());
    cfg.blocks.get_mut(&body).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });
    cfg.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(header, None);
    ssa.idom.insert(body, Some(header));
    ssa.idom.insert(exit, Some(header));

    let h = empty_ssa_block("header");
    let mut b = empty_ssa_block("body");
    // `$i` defined in body.
    let mut defs = Map::new();
    defs.insert(ssa.intern_var("i"), 1);
    b.statements.push(SsaStatement {
        statement: Statement::AssignConst {
            span: Span::new(0, 0),
            name: "i".into(),
            name_braced: false,
            value: "0".into(),
            value_span: None,
        },
        uses: Map::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });
    // llength $i — uses map tracks `i`, not `x`.
    let mut uses_i = Map::new();
    uses_i.insert(ssa.intern_var("i"), 1);
    b.statements.push(SsaStatement {
        statement: llength_on_i,
        uses: uses_i,
        defs: Map::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });
    ssa.blocks.insert(header, h);
    ssa.blocks.insert(body, b);
    ssa.blocks.insert(exit, empty_ssa_block("exit"));

    let results = find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
    assert_eq!(results, [] as [crate::gvn::RedundantComputation; 0]);
}

#[test]
fn find_loop_invariants_no_loops_empty() {
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    cfg.blocks.get_mut(&cfg.entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(cfg.entry, None);
    ssa.blocks.insert(cfg.entry, empty_ssa_block("entry"));
    assert_eq!(
        find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None),
        [] as [crate::gvn::RedundantComputation; 0]
    );
}

// Partial-redundancy detection.

#[test]
fn find_partial_redundancies_if_diamond() {
    // Diamond:
    //   entry → tt (computes llength $x) → join
    //         → ff → join (computes llength $x again)
    // `llength $x` is may-available at join (came via tt), but
    // not must-available (ff didn't compute it). On the join
    // occurrence, report O106 — consider hoisting before the
    // branch.
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let tt = cfg.intern_block("tt");
    let ff = cfg.intern_block("ff");
    let join = cfg.intern_block("join");
    cfg.blocks.insert(tt, Block::new("tt"));
    cfg.blocks.insert(ff, Block::new("ff"));
    cfg.blocks.insert(join, Block::new("join"));
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
        condition: crate::expr_ast::ExprNode::Var {
            text: "$c".into(),
            name: "c".into(),
            start: 0,
            end: 2,
        },
        true_target: tt,
        false_target: ff,
        span: None,
        condition_base: None,
    });
    cfg.blocks
        .get_mut(&tt)
        .unwrap()
        .statements
        .push(llength_call_at(100, 110));
    cfg.blocks.get_mut(&tt).unwrap().terminator = Some(Terminator::Goto {
        target: join,
        span: None,
    });
    cfg.blocks.get_mut(&ff).unwrap().terminator = Some(Terminator::Goto {
        target: join,
        span: None,
    });
    cfg.blocks
        .get_mut(&join)
        .unwrap()
        .statements
        .push(llength_call_at(200, 210));
    cfg.blocks.get_mut(&join).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(entry, None);
    ssa.idom.insert(tt, Some(entry));
    ssa.idom.insert(ff, Some(entry));
    ssa.idom.insert(join, Some(entry));

    let entry_b = empty_ssa_block("entry");
    let mut tt_b = empty_ssa_block("tt");
    tt_b.statements
        .push(ssa_stmt_for(&mut ssa, llength_call_at(100, 110), Some(1)));
    let ff_b = empty_ssa_block("ff");
    let mut join_b = empty_ssa_block("join");
    join_b
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call_at(200, 210), Some(1)));
    ssa.blocks.insert(entry, entry_b);
    ssa.blocks.insert(tt, tt_b);
    ssa.blocks.insert(ff, ff_b);
    ssa.blocks.insert(join, join_b);

    let results = find_partial_redundancies(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
    assert_eq!(results.len(), 1);
    // Canonical: partial redundancy (GVN-PRE) shares O105 with
    // full redundancy — the loop-invariant code O106 was wrong
    // here.
    assert_eq!(results[0].code, DiagCode::O105);
    assert_eq!(results[0].span.start(), 200);
    assert_eq!(results[0].first_span.start(), 100);
}

#[test]
fn find_partial_redundancies_none_on_linear_chain() {
    let registry = CommandRegistry::build_default();
    // Single-block function with one llength — no partial
    // redundancy possible.
    let mut cfg = Function::new("::top", "entry");
    cfg.blocks
        .get_mut(&cfg.entry)
        .unwrap()
        .statements
        .push(llength_call());
    cfg.blocks.get_mut(&cfg.entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(cfg.entry, None);
    let mut entry_b = empty_ssa_block("entry");
    entry_b
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(cfg.entry, entry_b);

    assert_eq!(
        find_partial_redundancies(&registry, &cfg, &ssa, &all_blocks(&cfg), None).len(),
        0
    );
}

#[test]
fn transfer_occurrence_keys_kill_clears_state() {
    let key: ExprKey = vec!["call".into(), "llength".into(), "$x@1".into()];
    let occ = ExprOccurrence {
        key: key.clone(),
        span: Span::new(0, 0),
        expression_text: String::new(),
        block: String::new(),
        statement_index: 0,
        variable_uses: Vec::new(),
    };
    let events = vec![OccurrenceEvent::Occur(occ), OccurrenceEvent::Kill];
    let out = transfer_occurrence_keys(&events, FxHashSet::default());
    assert!(!out.contains(&key));
}

#[test]
fn dominates_trivial_and_chain() {
    // Intern the chain through a CFG so the SSA shares its BlockIds.
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let a = cfg.intern_block("a");
    let b = cfg.intern_block("b");
    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(entry, None);
    ssa.idom.insert(a, Some(entry));
    ssa.idom.insert(b, Some(a));
    let dom = ssa.dominator_intervals();
    assert!(dom.dominates(entry, b));
    assert!(dom.dominates(a, b));
    assert!(!dom.dominates(b, a));
    assert!(dom.dominates(b, b));
}

#[test]
fn dominator_intervals_answer_siblings_and_unreachable() {
    // The interval index must give the same answers an idom chain walk
    // does — including "no", which is the case an interval scheme can get
    // wrong if the numbering is off by one.
    //
    //   entry → a → { b, c },  d unreachable (no idom entry)
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let a = cfg.intern_block("a");
    let b = cfg.intern_block("b");
    let c = cfg.intern_block("c");
    let d = cfg.intern_block("d");
    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(entry, None);
    ssa.idom.insert(a, Some(entry));
    ssa.idom.insert(b, Some(a));
    ssa.idom.insert(c, Some(a));
    let dom = ssa.dominator_intervals();
    for node in [a, b, c] {
        assert!(dom.dominates(entry, node), "entry must dominate {node:?}");
        assert!(dom.dominates(node, node), "{node:?} dominates itself");
    }
    assert!(dom.dominates(a, b) && dom.dominates(a, c));
    assert!(!dom.dominates(b, c) && !dom.dominates(c, b));
    assert!(!dom.dominates(b, a) && !dom.dominates(c, entry));
    // A block outside the dominator forest dominates only itself.
    assert!(dom.dominates(d, d));
    assert!(!dom.dominates(entry, d) && !dom.dominates(d, entry));
}

#[test]
fn expr_occurrence_carries_variable_uses() {
    let occ = ExprOccurrence {
        key: vec!["call".into(), "llength".into(), "$x@1".into()],
        span: Span::new(0, 5),
        expression_text: "llength $x".into(),
        block: "entry".into(),
        statement_index: 0,
        variable_uses: vec!["x".into()],
    };
    assert_eq!(occ.variable_uses, vec!["x".to_string()]);
}

/// Exercise the explicitly legacy CFG-only classifier. Production
/// [`FunctionUnit`](crate::compilation_unit::FunctionUnit) GVN deliberately
/// does not scan embedded source strings when no exact semantic node exists.
fn legacy_redundancies(src: &str) -> Vec<String> {
    use crate::compilation_unit::CompilationUnit;
    let registry = release_registry("tcl8.6");
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut out = Vec::new();
    for function in cu.analysable_functions() {
        out.extend(
            find_redundancies(&registry, &function.cfg, &function.ssa, None)
                .into_iter()
                .map(|redundancy| redundancy.expression_text),
        );
    }
    out.sort();
    out
}

#[test]
fn detects_redundant_ensemble_subcommand_cse() {
    // `string length $a` is pure via its *subcommand* purity — the
    // second computation is redundant. Before subcommand-aware
    // side-effect classification this was missed (the ensemble call
    // looked like an unknown-write).
    assert_eq!(
        legacy_redundancies(
            "proc f {a} { set x [string length $a]\n set y [string length $a]\n return [expr {$x + $y}] }"
        ),
        vec!["string length $a".to_string()],
    );
    // Likewise `dict get`.
    assert_eq!(
        legacy_redundancies(
            "proc f {d} { set a [dict get $d k]\n set b [dict get $d k]\n return [list $a $b] }"
        ),
        vec!["dict get $d k".to_string()],
    );
}

#[test]
fn mutating_ensemble_subcommand_is_not_cse() {
    // `dict set` mutates — never a redundant-computation candidate.
    assert_eq!(
        legacy_redundancies("proc f {d} { dict set d k 1\n dict set d k 1\n return $d }").len(),
        0
    );
}

#[test]
fn kill_all_does_not_leak_across_sibling_branches() {
    // A state-writing statement (a global write here) in
    // the then-arm calls `kill_all`. That must NOT collapse the scope
    // stack, or the then-arm's `[llength $lst]` occurrence strands in the
    // never-popped root scope and the else-arm's identical computation is
    // falsely reported redundant. The two arms are mutually exclusive
    // paths — nothing is redundant.
    assert_eq!(
            legacy_redundancies(
                "proc f {c lst} { if {$c} { set ::g 1\n set a [llength $lst] } else { set b [llength $lst] }\n return 1 }"
            ).len(), 0,
            "llength on mutually-exclusive branches must not be flagged redundant",
        );
}
#[test]
fn kill_all_preserves_within_path_detection() {
    // FP-guard for: the in-place clear must not disturb
    // ordinary same-path redundancy detection — two identical pure
    // computations with nothing between them are still redundant.
    assert_eq!(
        legacy_redundancies(
            "proc f {lst} { set a [llength $lst]\n set b [llength $lst]\n return [list $a $b] }"
        ),
        vec!["llength $lst".to_string()],
    );
}

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
#[test]
fn original_token_purity_requires_actual_availability_and_complete_children() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = current.commands();
    let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "dict create key value",
        registry,
        false,
        "tcl8.6",
    );
    let tokens = cu
        .ir_module
        .top_level
        .statements
        .iter()
        .find_map(Statement::tokens)
        .unwrap();
    let traced = std::collections::BTreeSet::new();
    assert!(super::is_pure_tokens_with_metadata_context(
        registry,
        Some(current.into()),
        tokens,
        &traced,
        false
    ));
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(registry));
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    for metadata in [Some((&older).into()), Some(foreign.into()), None] {
        assert!(!super::is_pure_tokens_with_metadata_context(
            registry, metadata, tokens, &traced, false
        ));
    }
    let replaced = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "proc dict args {return MUTATED}; list [dict create key value]",
        registry,
        false,
        "tcl8.6",
    );
    let tokens = replaced
        .ir_module
        .top_level
        .statements
        .last()
        .and_then(Statement::tokens)
        .unwrap();
    assert!(!super::is_pure_tokens_with_metadata_context(
        registry,
        Some(current.into()),
        tokens,
        &traced,
        false
    ));
}
