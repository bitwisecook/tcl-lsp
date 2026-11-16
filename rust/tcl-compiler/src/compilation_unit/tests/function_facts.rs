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

//! Facts of one built function: callee global writes, the existence tier, parameter uses and return types.

use super::*;

/// A reached callee write withdraws the old value at later reads without
/// poisoning the value that existed before the call.
fn assert_callee_global_write_is_positioned(name: &str) {
    use crate::analyses::{ConstValue, LatticeValue};
    let src = format!(
        "set {name} 4\nset before ${name}\nproc helper {{}} {{global {name}; set {name} 17}}\nhelper\nset after ${name}\nif {{${name} == 4}} {{set stale BAD}}\n"
    );
    let cu = CompilationUnit::build_for(&src, &registry(), false);
    let before = cu
        .top_level
        .sccp
        .values
        .iter()
        .filter(|((symbol, _), _)| cu.top_level.ssa.var_name(*symbol) == "before")
        .collect::<Vec<_>>();
    assert!(
        before.iter().any(
            |(_, value)| matches!(value, LatticeValue::Const(ConstValue::Int(4)))
                || matches!(value, LatticeValue::Const(ConstValue::String(value)) if value == "4")
        ),
        "prior value must remain represented: {before:?}"
    );
    let after = cu
        .top_level
        .sccp
        .values
        .iter()
        .filter(|((symbol, _), _)| cu.top_level.ssa.var_name(*symbol) == "after")
        .collect::<Vec<_>>();
    assert!(!after.is_empty(), "post-call read must remain represented");
    assert!(
        after.iter().all(
            |(_, value)| !matches!(value, LatticeValue::Const(ConstValue::Int(4)))
                && !matches!(value, LatticeValue::Const(ConstValue::String(value)) if value == "4")
        ),
        "post-call read must not reuse the stale value: {after:?}"
    );
    assert!(
        cu.top_level
            .sccp
            .constant_branches
            .iter()
            .all(|selected| !selected.value),
        "stale condition cannot select its true body"
    );
}

#[test]
fn top_level_var_touched_by_callee_global_is_overdefined() {
    assert_callee_global_write_is_positioned("g");
}

#[test]
fn top_level_dual_ported_var_touched_by_callee_global_is_overdefined() {
    assert_callee_global_write_is_positioned("tcl_precision");
}

/// Control: a top-level name *no* procedure ever `global`-declares must
/// still fold to a genuine `Const` — the whole-module scan must not
/// over-widen every top-level variable (that would be a precision
/// regression, not a soundness fix).
#[test]
fn top_level_var_untouched_by_any_callee_still_folds() {
    let reg = registry();
    let src = "set safe_const 42\nproc other {} { puts unrelated }\nother\n";
    let cu = CompilationUnit::build_for(src, &reg, false);
    let sym = cu
        .top_level
        .ssa
        .var_symbol("safe_const")
        .expect("top-level `safe_const` should be interned");
    let has_const = cu
        .top_level
        .sccp
        .values
        .iter()
        .any(|((s, _), lv)| *s == sym && matches!(lv, crate::analyses::LatticeValue::Const(_)));
    assert!(has_const, "expected `safe_const` to still fold to a Const");
}

/// Existence is a deep-tier fact: a procedure lattice requested
/// at the fast tier runs no rung, so every read of it answers
/// `Unavailable(Fast)` — neither bound nor unbound — and W210 and W213
/// stay silent where the deep build reports both; a unit over the
/// complexity ceiling answers `Unavailable(ComplexityGuarded)`.
#[test]
fn existence_is_unavailable_at_the_fast_tier() {
    use tcl_core_types::DiagCode;
    let reg = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let src = "proc p {} {set x 1; unset x; unset x; puts $x}\n";
    let lifecycle = |cu: CompilationUnit| -> Vec<DiagCode> {
        let mut analyser = crate::analyser::Analyser::new();
        analyser.set_cu_override(Arc::new(cu));
        analyser
            .analyse(src, "tcl8.6")
            .diagnostics
            .into_iter()
            .map(|d| d.code)
            .filter(|code| matches!(code, DiagCode::W210 | DiagCode::W213))
            .collect()
    };
    let deep = CompilationUnit::build_for_dialect(src, reg, false, "tcl8.6");
    let deep_p = deep.function("::p").expect("::p built");
    assert_eq!(deep_p.tier, AnalysisTier::Deep);
    let x = deep_p.ssa.var_symbol("x").expect("x interned");
    let entry = deep_p.cfg.entry;
    let cfg = deep_p.cfg.clone();
    assert!(matches!(
        deep_p.existence(x, ExistencePoint::Exit(entry)),
        FactView::Domain(DomainFact::Existence(_))
    ));

    let key = crate::value_transfer::AnalysisContextKey::detached().at_tier(AnalysisTier::Fast);
    let fast_p = FunctionUnit::build_with_param_constants_and_classes_under(
        "::p",
        cfg.clone(),
        &[],
        UnitDialect {
            registry: reg,
            config: tcl_lexer::LexerConfig::default(),
            source_metadata_input: None,
        },
        None,
        &HashSet::new(),
        ModuleAnalysisFacts {
            trace: ModuleTraceFacts::none(),
            analysis_context: &key,
            command_trust: &crate::command_binding::ModuleCommandMutations::default(),
        },
    );
    assert_eq!(fast_p.tier, AnalysisTier::Fast);
    assert!(fast_p.sccp.existence.is_empty());
    for point in [
        ExistencePoint::Exit(entry),
        ExistencePoint::Before(entry, 0),
        ExistencePoint::Version(1),
    ] {
        assert_eq!(
            fast_p.existence(x, point),
            FactView::Top(DeclineReason::Unavailable(AnalysisTier::Fast))
        );
    }
    let mut fast = deep.clone();
    let p_id = fixture_id(&deep.ir_module.procedures, "::p");
    fast.procedures.insert(p_id, fast_p);

    let found = lifecycle(deep);
    assert!(
        found.contains(&DiagCode::W213) && found.contains(&DiagCode::W210),
        "{found:?}"
    );
    assert_eq!(lifecycle(fast), Vec::<DiagCode>::new());

    let guarded = FunctionUnit::trivial_guarded("::p", cfg);
    assert_eq!(
        guarded.existence(x, ExistencePoint::Exit(entry)),
        FactView::Top(DeclineReason::Unavailable(AnalysisTier::ComplexityGuarded))
    );
}

#[test]
fn switch_subject_counts_as_param_use() {
    // The `switch -- $col` subject lowers to an
    // `ExprNode::Raw` branch condition.  The expr var-scan now
    // recovers `$col`, so the parameter's def-use chain has a
    // live (terminator) use instead of looking dead — the precise
    // path behind W214 (unused parameter).  `col` is referenced
    // *only* as the switch subject here, so a live chain proves
    // the Raw scan reached the def-use builder.
    let cu = CompilationUnit::build_for(
        "proc p {col} { switch -- $col { a {set y 1} } }",
        &registry(),
        false,
    );
    let fu = cu.function("::p").expect("proc ::p should exist");
    let col_live = fu.def_use.chains.iter().any(|(key, chain)| {
        fu.def_use
            .symbol(&fu.ssa, key.0)
            .is_some_and(|symbol| fu.ssa.var_name(symbol) == "col")
            && !chain.is_dead()
    });
    assert!(
        col_live,
        "switch subject `$col` should register a live use; chains: {:?}",
        fu.def_use.chains.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn switch_glob_arm_body_read_counts_as_param_use() {
    // A `switch -glob`/`-regexp` arm body is a real analysed CFG region,
    // so a parameter referenced *only* inside a glob-arm body has a live
    // def-use chain — the precise path behind W214.
    // Multi-arg arm form (the single-braced-body form is a separate,
    // pre-existing lowering gap affecting every mode equally).
    for (mode, pat) in [("-glob", "a*"), ("-regexp", "a.*")] {
        let src = format!("proc p {{val}} {{ switch {mode} -- $col {pat} {{puts $val}} }}");
        let cu = CompilationUnit::build_for(&src, &registry(), false);
        let fu = cu.function("::p").expect("proc ::p should exist");
        let val_live = fu.def_use.chains.iter().any(|(key, chain)| {
            fu.def_use
                .symbol(&fu.ssa, key.0)
                .is_some_and(|symbol| fu.ssa.var_name(symbol) == "val")
                && !chain.is_dead()
        });
        assert!(
            val_live,
            "{mode} arm-body read `$val` should register a live use; chains: {:?}",
            fu.def_use.chains.keys().collect::<Vec<_>>(),
        );
    }
}

#[test]
fn return_type_infers_int_literal() {
    let cu = CompilationUnit::build_for("proc f {} { return 1 }", &registry(), false);
    let fu = cu.function("::f").expect("proc ::f");
    assert_eq!(fu.return_type, TypeLattice::of(tcl_registry::TclType::Int));
}

#[test]
fn return_type_infers_string_literal() {
    let cu = CompilationUnit::build_for("proc f {} { return \"hi\" }", &registry(), false);
    let fu = cu.function("::f").expect("proc ::f");
    assert_eq!(
        fu.return_type,
        TypeLattice::of(tcl_registry::TclType::String)
    );
}

#[test]
fn return_type_follows_local_var() {
    let cu = CompilationUnit::build_for("proc f {} { set x 1; return $x }", &registry(), false);
    let fu = cu.function("::f").expect("proc ::f");
    assert_eq!(fu.return_type, TypeLattice::of(tcl_registry::TclType::Int));
}

#[test]
fn return_type_joins_branches_to_common_int() {
    let cu = CompilationUnit::build_for(
        "proc f {a} { if {$a} { return 1 } else { return 2 } }",
        &registry(),
        false,
    );
    let fu = cu.function("::f").expect("proc ::f");
    assert_eq!(fu.return_type, TypeLattice::of(tcl_registry::TclType::Int));
}

#[test]
fn return_type_partial_return_widens_via_fallthrough() {
    // `if {$a} { return 1 }` with no else: the false path falls off
    // the end of the body (Tcl returns the last command's result),
    // so the joined return type must not be a confident `Int` — the
    // fall-through exit widens it to Overdefined.
    let cu = CompilationUnit::build_for("proc f {a} { if {$a} { return 1 } }", &registry(), false);
    let fu = cu.function("::f").expect("proc ::f");
    assert_eq!(
        fu.return_type,
        TypeLattice::overdefined(),
        "partial-return proc must widen, got {:?}",
        fu.return_type,
    );
}
