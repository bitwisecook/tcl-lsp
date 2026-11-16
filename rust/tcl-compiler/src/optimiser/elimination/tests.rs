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

use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};

use super::*;

use tcl_registry::CommandRegistry;

use crate::interprocedural::InterproceduralAnalysis;

fn registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

fn run_pass(source: &str) -> Vec<Optimisation> {
    let cu = CompilationUnit::build_for(source, &registry(), false);
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    run(&mut ctx, &cu);
    ctx.optimisations
}

#[test]
fn original_global_alias_scan_retains_literal_target_and_command_identity() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source startup names are independent of Native cell identity.
    let registry = std::sync::Arc::new(
        CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap()),
    );
    let context = tcl_registry::model::ingress::static_context_for("tcl9.0")
        .with_command_store(std::sync::Arc::clone(&registry));
    for (source, expected) in [
        (
            "proc p {} {global argc {$argv} {scalar(open} ::other::argc}",
            vec!["argc", "$argv", "scalar(open"],
        ),
        ("proc global args {}; proc p {} {global argc}", vec![]),
        ("rename global moved; proc p {} {moved argc}", vec!["argc"]),
        (
            "interp alias {} link {} global argc; proc p {} {link}",
            vec!["argc"],
        ),
        (
            "rename global moved; proc global args {}; proc p {} {global argc}",
            vec![],
        ),
    ] {
        let compilation = CompilationUnit::build_for(source, &registry, false);
        let function = compilation.function("::p").unwrap();
        let names = scan_global_scope_aliases_with_metadata_context(
            &function.cfg,
            &registry,
            Some((&context).into()),
        );
        assert_eq!(
            names,
            expected.into_iter().map(str::to_owned).collect(),
            "{source}"
        );
    }
}

#[test]
fn original_alias_scans_refuse_unavailable_and_foreign_metadata() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let mut registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    registry.insert(tcl_registry::CommandSpec {
        name: "gated-global",
        surface: registry.get("dict").unwrap().surface,
        ..registry.get("global").unwrap().clone()
    });
    let registry = std::sync::Arc::new(registry);
    let compilation = CompilationUnit::build_for("proc p {} {gated-global argc}", &registry, false);
    let function = compilation.function("::p").unwrap();
    for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
        let context = tcl_registry::model::ingress::static_context_for(environment)
            .with_command_store(std::sync::Arc::clone(&registry));
        let metadata = Some((&context).into());
        assert_eq!(
            scan_scope_aliases_with_metadata_context(&function.cfg, &registry, metadata)
                .contains("argc"),
            available,
            "{environment}"
        );
        assert_eq!(
            scan_global_scope_aliases_with_metadata_context(&function.cfg, &registry, metadata)
                .contains("argc"),
            available,
            "{environment}"
        );
    }
    assert!(
        scan_global_scope_aliases_with_metadata_context(&function.cfg, &registry, None).is_empty()
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0");
    assert!(
        scan_scope_aliases_with_metadata_context(&function.cfg, &registry, Some(foreign.into()))
            .is_empty()
    );
}

/// The hidden-read scan keeps only what the SSA does not record:
/// an existence read, a `VarRead` role and a
/// nested cell update in a statement's words, or in a `return` word, are
/// uses of the version they read — on the statement itself or on its
/// word effects, the definition point the lowering pairs with it — so
/// they leave the scan; a braced `expr` body in an `incr` amount,
/// which nothing records, stays.
#[test]
fn hidden_reads_are_what_the_ssa_does_not_record() {
    let reg = registry();
    for (src, expected) in [
        (
            "proc p {} {set x 1; puts [info exists x]}",
            Vec::<&str>::new(),
        ),
        (
            "proc p {} {set x 1; set y [info exists x]; return $y}",
            vec![],
        ),
        (
            "proc p {j} {set i 0; lappend r [incr i $j]; return $r}",
            vec![],
        ),
        (
            "proc p {} {set x 1; set y [string length [set x]]; return $y}",
            vec![],
        ),
        ("proc p {} {set x 1; return [info exists x]}", vec![]),
        ("proc p {} {set n 1; set a [incr n]; return $a}", vec![]),
        ("proc p {} {set n 1; return [incr n]}", vec![]),
        (
            "proc p {w} {set i 0; incr i [expr {$w}]; return $i}",
            vec!["w"],
        ),
    ] {
        let cu = CompilationUnit::build_for(src, &reg, false);
        let fu = cu.function("::p").expect("procedure analysed");
        let mut got: Vec<String> = collect_rmw_hidden_reads(fu, &reg).into_iter().collect();
        got.sort();
        assert_eq!(got, expected, "{src}");
    }
}

// internal helper tests

/// Deep analytical expressions remain stack-safe; source substitutions
/// without original receipts remain uncertain even under a pure name label.
#[test]
fn deeply_nested_side_effect_walks_survive() {
    let reg = registry();
    let original = crate::lowering::lower_to_ir("proc a {} {}", &reg);
    let interproc_pure = [fixture_id(&original.procedures, "::a")]
        .into_iter()
        .collect();
    let purity = PurityCtx {
        registry: Some(&reg),
        interproc_pure: &interproc_pure,
        config: tcl_lexer::LexerConfig::default(),
        metadata: None,
        module: None,
    };
    let effect = EffectCtx {
        purity,
        source_tokens: None,
    };
    let mut deep = "x".to_owned();
    for _ in 0..3000 {
        deep = format!("[a {deep}]");
    }
    assert!(word_has_observable_side_effect(&deep, effect));
    let mut node = ExprNode::Literal {
        text: "1".into(),
        start: 0,
        end: 1,
    };
    for _ in 0..3000 {
        node = ExprNode::Unary {
            op: crate::expr_ast::UnaryOp::Not,
            operand: Box::new(node),
        };
    }
    assert!(expr_has_observable_side_effect(&node, effect, 0));
}

#[test]
fn original_completion_deletion_consumer_uses_same_module_and_header() {
    // naming.interprocedural.original-procedure-completion-source-context
    // docs/design/analysis/name-resolution-proofs/interprocedural-original-procedure-completion-source-context.md
    // The call no-error consumer only. SCCP existence, observers, purity
    // and source edit permission are independently required for deletion.
    let source = "proc leaf {x} {return $x}; set unused [leaf VALUE]";
    let (context, unit) = crate::interprocedural::logical_completion_unit(source, "tcl8.6");
    let profile = unit.ir_module.dialect_profile;
    let unit = unit.with_interprocedural(context.commands(), profile);
    let interproc = unit.interproc.as_ref().unwrap();
    assert!(
        interproc
            .procedures
            .get(&fixture_id(&interproc.procedures, "::leaf"))
            .expect("authored fixture body")
            .pure
    );
    assert!(
        interproc
            .procedures
            .get(&fixture_id(&interproc.procedures, "::leaf"))
            .expect("authored fixture body")
            .completes
    );
    let statement = unit
        .top_level
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .find(|statement| {
            matches!(statement,
                Statement::AssignValue { name, .. } if name == "unused")
        })
        .unwrap();
    let mut ctx = PassContext::new(source, interproc.clone());
    ctx.registry = Some(context.commands());
    ctx.command_mutations = unit.command_mutations.clone();
    ctx.ir_module = Some(&unit.ir_module);
    assert!(RaiseProof::new(&ctx, &unit.top_level, None, true).calls_complete(statement));
    let mut changed = unit.ir_module.clone();
    {
        let id = fixture_id(&changed.procedures, "::leaf");
        changed.procedures.get_mut(&id)
    }
    .unwrap()
    .params_raw = "{x DEFAULT}".into();
    ctx.ir_module = Some(&changed);
    assert!(!RaiseProof::new(&ctx, &unit.top_level, None, true).calls_complete(statement));
    ctx.ir_module = None;
    assert!(!RaiseProof::new(&ctx, &unit.top_level, None, true).calls_complete(statement));
    ctx.ir_module = Some(&unit.ir_module);
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    ctx.registry = Some(foreign.commands());
    assert!(!RaiseProof::new(&ctx, &unit.top_level, None, true).calls_complete(statement));
}

#[test]
fn resolved_store_observer_advice_preserves_literal_sigil_and_scalar_names() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // This is the store-observer suppression gate, not a current Native
    // trace, reaching cell, SSA use or deletion proof.
    let mut context = PassContext::new("", InterproceduralAnalysis::default());
    let traced = ["counter".to_owned(), "café".to_owned()]
        .into_iter()
        .collect();
    assert!(dead_store_name_observed(
        &context,
        "counter",
        Some(&traced),
        true,
        &[]
    ));
    assert!(!dead_store_name_observed(
        &context,
        "$counter",
        Some(&traced),
        true,
        &[]
    ));
    assert!(!dead_store_name_observed(
        &context,
        "café(open",
        Some(&traced),
        true,
        &[]
    ));
    assert!(dead_store_name_observed(
        &context,
        "café(key)",
        Some(&traced),
        true,
        &[]
    ));
    context.cross_event_vars.insert("$counter".into());
    assert!(dead_store_name_observed(
        &context,
        "$counter(key)",
        None,
        true,
        &[]
    ));
    assert!(!dead_store_name_observed(
        &context,
        "counter",
        None,
        true,
        &[]
    ));
}

#[test]
fn unreachable_blocks_empty_when_all_executable() {
    let cu = CompilationUnit::build_for("set x 1", &registry(), false);
    let unreach = unreachable_blocks(&cu.top_level.cfg, &cu.top_level.sccp);
    assert_eq!(unreach.len(), 0);
}

// end-to-end tests

#[test]
fn opaque_native_reads_keep_named_stores_live_without_synthetic_uses() {
    let registry = registry();
    let mut unit = CompilationUnit::build_for("set retained VALUE", &registry, false);
    let function = &mut unit.top_level;
    let native = crate::ir::native_call_for_test(b"opaque \xff");
    function
        .cfg
        .blocks
        .get_mut(&function.cfg.entry)
        .unwrap()
        .statements
        .push(native);
    let mut context = PassContext::new(&unit.source, InterproceduralAnalysis::default());
    context.registry = Some(&registry);
    assert!(dead_store_observation_unbounded(&context, &unit.top_level));
    assert!(unit.top_level.dynamic_barrier_blocks_value_motion());
    run(&mut context, &unit);
    assert!(!context.optimisations.iter().any(|optimisation| {
        matches!(
            optimisation.code,
            DiagCode::O108 | DiagCode::O109 | DiagCode::O126
        )
    }));
}

#[test]
fn empty_source_produces_nothing() {
    let opts = run_pass("");
    assert_eq!(opts, [] as [crate::optimiser::Optimisation; 0]);
}

/// Like [`run_pass`] but with `ctx.ir_module` wired the way the
/// production entry points wire it, so the whole-module variable-trace
/// facts reach the O109 / O126 gate.
fn run_pass_with_module(source: &str) -> Vec<Optimisation> {
    let cu = CompilationUnit::build_for(source, &registry(), false);
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    ctx.ir_module = Some(&cu.ir_module);
    run(&mut ctx, &cu);
    ctx.optimisations
}

/// The trace names `::g`, the store is spelled `g`, and
/// both name the same top-level global; the write trace observes
/// `set g 1`, so it is not a dead store.
#[test]
fn traced_global_unqualified_store_is_not_dead() {
    let src = "proc onw {a b c} { puts trace }\ntrace add variable ::g write ::onw\nset g 1\nset g 2\nputs $g";
    let opts = run_pass_with_module(src);
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O109),
        "traced global's store must not be a dead store, got {opts:?}",
    );
}

/// A dynamic trace target makes every name potentially
/// traced, so no store anywhere in the module is provably dead.
#[test]
fn dynamic_trace_target_blocks_dead_stores() {
    let src = "proc onw {a b c} { puts trace }\ntrace add variable $n write ::onw\nset g 1\nset g 2\nputs $g";
    let opts = run_pass_with_module(src);
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O109),
        "dynamic trace target must block dead-store elimination, got {opts:?}",
    );
}

/// Control for the two tests above: without any trace the overwritten
/// store still reports O109.
#[test]
fn untraced_overwritten_store_still_reports_o109() {
    let opts = run_pass_with_module("set g 1\nset g 2\nputs $g");
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O109),
        "control: untraced overwritten store must still fire, got {opts:?}",
    );
}

#[test]
fn straight_line_script_is_fully_reachable() {
    let opts = run_pass("set x 1\nset y 2\nputs $x");
    assert_eq!(opts, [] as [crate::optimiser::Optimisation; 0]);
}

#[test]
fn branch_folding_creates_unreachable_block() {
    // The else branch is unreachable under SCCP because the
    // condition folds to true.
    let opts = run_pass("if {1} { set x 1 } else { set y 2 }");
    // Expect at least one O107 — the else body's `set y 2`.
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O107 && o.message == "Eliminate unreachable dead code"),
        "expected at least one O107, got {opts:?}",
    );
}

#[test]
fn while_false_body_is_unreachable() {
    // The body of `while {0} { ... }` is unreachable.
    let opts = run_pass("while {0} { set x 1 }");
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O107),
        "expected an O107 for dead while body, got {opts:?}",
    );
}

#[test]
fn unreachable_for_clause_markers_are_not_source_commands() {
    for source in [
        "for {set previous NONEMPTY} {0} {} {}",
        "for {} {0} {} {puts unreachable}",
    ] {
        let opts = run_pass(source);
        for edit in opts.iter().filter(|edit| edit.code == DiagCode::O107) {
            let written = &source[edit.span.start() as usize..edit.span.end() as usize];
            assert_ne!(written, "{}", "synthetic clause must retain argv: {opts:?}");
        }
        if source.contains("puts unreachable") {
            assert!(
                opts.iter().any(|edit| edit.code == DiagCode::O107
                    && source[edit.span.start() as usize..edit.span.end() as usize]
                        .contains("puts unreachable")),
                "actual dead command remains editable: {opts:?}"
            );
        }
    }
}

#[test]
fn unreachable_statements_emitted_with_empty_replacement() {
    let opts = run_pass("if {0} { set x 1 }");
    let target = opts.iter().find(|o| o.code == DiagCode::O107);
    if let Some(o) = target {
        assert_eq!(o.replacement, "");
        assert!(!o.span.is_empty());
    }
}

#[test]
fn callee_read_preserves_only_the_contents_that_reach_it() {
    let source = "proc read_write {name} {upvar 1 $name value; puts $value; set value done}\nproc f {} {set tag init; set tag live; read_write tag}\nf";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let optimisations = crate::optimiser::optimise_raw(source, registry, None);
    let removes = |text: &str| {
        let start = u32::try_from(source.find(text).unwrap()).unwrap();
        optimisations.iter().any(|optimisation| {
            matches!(optimisation.code, DiagCode::O109 | DiagCode::O126)
                && optimisation.span.start() <= start
                && optimisation.span.end() > start
        })
    };
    assert!(
        removes("set tag init"),
        "overwritten contents: {optimisations:?}"
    );
    assert!(
        !removes("set tag live"),
        "callee observes live contents: {optimisations:?}"
    );
}

#[test]
fn native_getter_in_a_callee_argument_preserves_the_reaching_caller_store() {
    let source = "proc read_value {name} {upvar 1 $name value; set value}\nproc f {} {set tag init; set tag live; puts [read_value tag]}\nf";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let optimisations = crate::optimiser::optimise_raw(source, registry, None);
    let removes = |text: &str| {
        let start = u32::try_from(source.find(text).unwrap()).unwrap();
        optimisations.iter().any(|optimisation| {
            matches!(optimisation.code, DiagCode::O109 | DiagCode::O126)
                && optimisation.span.start() <= start
                && optimisation.span.end() > start
        })
    };
    assert!(removes("set tag init"), "overwritten: {optimisations:?}");
    assert!(
        !removes("set tag live"),
        "nested native getter: {optimisations:?}"
    );
}

/// The fresh version a call to code the module cannot see leaves is an
/// analysis value, not a store: a read of it still reads the store before
/// the call, which therefore stays.
#[test]
fn analysis_value_clobber_keeps_executable_store_live() {
    for body in [
        "set local 1; missing_command; return $local",
        "set local 1; missing_command; puts $local",
        "set local 1; missing_command; set copy $local; return $copy",
    ] {
        let source = format!("proc p {{v}} {{{body}}}");
        let opts = crate::optimiser::optimise(&source, &registry());
        assert!(
            opts.iter()
                .all(|opt| !matches!(opt.code, DiagCode::O108 | DiagCode::O109 | DiagCode::O126)),
            "{source}: {opts:?}"
        );
    }
}

#[test]
fn dead_store_fires_o109_when_overwritten_before_read() {
    // First set is dead — second version is the only live
    // value of x when read by puts.
    let opts = run_pass("set x 1\nset x 2\nputs $x");
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O109),
        "expected O109 for overwritten store, got {opts:?}",
    );
}

/// A cell update nested in a substitution reads its target before
/// writing it, so the store feeding it is never dead: `set n 1; set
/// result [incr n]; puts $n` prints 2 and keeps `set n 1` (#2050).
#[test]
fn store_read_by_a_nested_cell_update_is_not_dead() {
    for src in [
        "set n 1\nset result [incr n]\nputs $n",
        "set n 1\nset result [incr n]\nputs $result",
        "set l {a}\nset r [lappend l b]\nputs $r",
        "set s x\nputs [append s y]",
    ] {
        let opts = run_pass(src);
        assert!(
            !opts
                .iter()
                .any(|o| o.code == DiagCode::O109 || o.code == DiagCode::O126),
            "{src:?}: the nested cell update reads the store, got {opts:?}",
        );
    }
    // The control: a store nothing reads is still dead.
    let opts = run_pass("set n 1\nset n 2\nputs $n");
    assert!(opts.iter().any(|o| o.code == DiagCode::O109), "{opts:?}");
}

#[test]
fn qualified_global_write_is_not_a_dead_store() {
    // `set ::counter 42` inside a proc is a global write
    // visible to other scopes; eliminating it (O109/O126) leaves
    // `::counter` undefined. It must be kept even though the proc
    // never reads it.
    let src = "proc ::setit {} { set ::counter 42 }\n::setit\nputs $::counter";
    let opts = crate::optimiser::optimise(src, &registry());
    let removes_counter = opts.iter().any(|o| {
        (o.code == DiagCode::O109 || o.code == DiagCode::O126)
            && src
                .get(o.span.start() as usize..o.span.end() as usize)
                .is_some_and(|t| t.contains("::counter"))
    });
    assert!(
        !removes_counter,
        "must not remove the `set ::counter` global write, got {opts:?}",
    );
}

#[test]
fn o109_array_element_overwrite_not_dead() {
    // Per-element SSA: `a(k)` and `a(j)` are independent variables, so
    // `$a(k)` reads the value of `set a(k)` — never `set a(j)`. With a
    // constant value the pipeline may forward + inline it and then
    // delete the store as a *coupled* rewrite, but the forwarded value
    // must be the same element's `1` (conflation would forward `2`).
    let opts = crate::optimiser::optimise(
        "proc f {} { set a(k) 1; set a(j) 2; puts $a(k) }",
        &registry(),
    );
    // Scoped to the payload-bearing rewrites: a hint-only O102 spans the
    // whole consuming statement and deliberately carries no replacement
    // so it cannot conflate anything.
    assert!(
        opts.iter()
            .all(|o| o.code != DiagCode::O102 || o.hint_only || o.replacement == "1"),
        "a forwarded a(k) load must carry a(k)'s value, got {opts:?}",
    );
    // A non-constant element value cannot be forwarded, so the store
    // stays live through its read — no dead-store report of any kind.
    let live = crate::optimiser::optimise(
        "proc f {x} { set a(k) $x; set a(j) 2; puts $a(k) }",
        &registry(),
    );
    assert!(
        live.iter().all(|o| o.code != DiagCode::O109),
        "a(k) is read by `puts $a(k)`; got {live:?}",
    );
}

#[test]
fn o109_array_element_genuinely_dead_still_fires() {
    // Precision guard: here only a(j) is read, so a(k) IS overwritten
    // before any read — O109 must still fire on it.  The place model
    // suppresses only element writes that a read actually observes.
    let opts = crate::optimiser::optimise("set a(k) 1\nset a(j) 2\nputs $a(j)", &registry());
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O109),
        "O109 expected — a(k) is overwritten and never read; got {opts:?}",
    );
}

#[test]
fn unused_variable_fires_o126_in_proc_body() {
    let opts = run_pass("proc ::f {} { set y 42; return 1 }");
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O126),
        "expected O126 for unused var in proc, got {opts:?}",
    );
}

#[test]
fn top_level_unused_variable_not_flagged() {
    // At top level an unused variable is not O126 — the
    // script-return semantics and external consumers (upvar,
    // info exists) could read it.
    let opts = run_pass("set y 42");
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O126),
        "top-level unused var should not emit O126, got {opts:?}",
    );
}

#[test]
fn scope_alias_globals_never_flagged() {
    let opts = run_pass("proc ::f {} { global g; set g 42 }");
    assert!(
        opts.iter()
            .all(|o| o.code != DiagCode::O109 && o.code != DiagCode::O126),
        "writes through global should not be flagged, got {opts:?}",
    );
}

#[test]
fn adce_removes_chain_of_dead_defs() {
    // `set a 1` → `set b $a` → `set c $b`; `c` is never
    // read. O126 flags `set c $b`, then ADCE should extend
    // to `set b $a` and `set a 1` since their only consumer
    // is the already-dead chain.
    let opts = run_pass("proc ::f {} { set a 1; set b $a; set c $b; return 7 }");
    let o108 = opts.iter().filter(|o| o.code == DiagCode::O108).count();
    assert!(
        o108 >= 1,
        "expected at least one O108 in transitive dead chain, got {opts:?}",
    );
}

#[test]
fn original_adce_keeps_actual_remote_stores_after_local_consumers_disappear() {
    // Implementation contract: naming.variable.transitive-store-observation
    // docs/design/analysis/name-resolution-proofs/transitive-store-observation.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
        let source = "proc f {} {global exported; set exported OLD; set first $exported; set unused $first; return OK}; f";
        let unit = CompilationUnit::build_for_dialect(source, registry, false, engine);
        let function = fixture_id_if_present(&unit.procedures, "::f")
            .and_then(|id| unit.procedures.get(&id))
            .unwrap();
        let offset = u32::try_from(source.find("set exported OLD").unwrap()).unwrap();
        let (&block, index, statement) = function
            .cfg
            .blocks
            .iter()
            .find_map(|(block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        (function.abs_span(statement.span()).start() == offset)
                            .then_some((block, index, statement))
                    })
            })
            .unwrap();
        let context = function
            .ssa
            .point_contexts
            .as_ref()
            .unwrap()
            .context_before(block, index)
            .unwrap();
        let writes = crate::place_bridge::def_places(statement, context, registry);
        assert!(writes.iter().any(|place| {
                matches!(crate::var_resolve::cell_key(place).root(), crate::var_resolve::VariableCellKey::Namespace { simple, .. }
                    if simple.as_bytes() == b"exported")
            }), "actual remote target retained for {engine}");
        let mut pass = PassContext::new(&unit.source, InterproceduralAnalysis::default());
        pass.registry = Some(registry);
        pass.ir_module = Some(&unit.ir_module);
        run(&mut pass, &unit);
        assert!(
            pass.optimisations.iter().any(|optimisation| {
                matches!(
                    optimisation.code,
                    DiagCode::O108 | DiagCode::O109 | DiagCode::O126
                ) && source
                    .get(optimisation.span.start() as usize..optimisation.span.end() as usize)
                    .is_some_and(|text| text.contains("set unused"))
            }),
            "the private consumer still qualifies for {engine}"
        );
        assert!(
            pass.optimisations.iter().all(|optimisation| {
                !matches!(
                    optimisation.code,
                    DiagCode::O108 | DiagCode::O109 | DiagCode::O126
                ) || source
                    .get(optimisation.span.start() as usize..optimisation.span.end() as usize)
                    .is_none_or(|text| !text.contains("set exported OLD"))
            }),
            "remote store survives its dead local chain for {engine}"
        );
    }
}

#[test]
fn adce_preserves_impure_link_in_dead_chain() {
    // `b` is read only by the dead `set c $b`, so the transitive
    // chain `set a [puts hi]` → `set b $a` → `set c $b` is dead by
    // def-use. But `set a [puts hi]` still prints — the O108 purity
    // gate must keep it, deleting at most the pure links.
    let src = "proc ::f {} { set a [puts hi]; set b $a; set c $b; return 7 }";
    let opts = crate::optimiser::optimise(src, &registry());
    let removes_puts_line = opts.iter().any(|o| {
        matches!(o.code.as_str(), "O108" | "O109" | "O126")
            && src
                .get(o.span.start() as usize..o.span.end() as usize)
                .is_some_and(|slice| slice.contains("puts"))
    });
    assert!(
        !removes_puts_line,
        "impure `set a [puts hi]` must survive ADCE, got {opts:?}",
    );
}

#[test]
fn used_variable_not_flagged() {
    let opts = run_pass("proc ::f {} { set x 1; return $x }");
    assert!(
        opts.iter()
            .all(|o| o.code != DiagCode::O109 && o.code != DiagCode::O126),
        "used var should not be flagged, got {opts:?}",
    );
}

fn original_fixture_set_store_span(source: &str) -> tcl_lexer::Span {
    let registry = registry();
    let unit = CompilationUnit::build_for(source, &registry, false);
    let procedure = unit.ir_module.procedures.values().next().unwrap();
    let statement = procedure.body.statements.first().unwrap();
    assert!(matches!(statement, Statement::AssignConst { name, .. } if name == "x"));
    let mut ctx = PassContext::new(source, InterproceduralAnalysis::default());
    ctx.registry = Some(&registry);
    ctx.ir_module = Some(&unit.ir_module);
    let span = original_statement_rewrite_span(&ctx, statement.span()).unwrap();
    assert_eq!(source.get(span.as_range()), Some("set x 1"));
    span
}

#[test]
fn collect_textual_var_references_detects_set_one_arg_read() {
    // ``[set varname]`` (1-arg form) is a variable read; without
    // it, DCE saw 0 reads on ``varname`` and incorrectly deleted
    // the write.
    let source = "proc ::f {} { set x 1; set y [set x]; return $y }";
    let subject = original_fixture_set_store_span(source);
    let opts = run_pass(source);
    // ``x`` is read via ``[set x]`` so neither O109 nor O126
    // should fire on the ``set x 1`` line.
    let bad: Vec<_> = opts
        .iter()
        .filter(|o| {
            (o.code == DiagCode::O109 || o.code == DiagCode::O126)
                && o.span.start() <= subject.start()
                && o.span.end() >= subject.end()
        })
        .collect();
    assert_eq!(
        bad.len(),
        0,
        "[set x] should count as a read for x; got {opts:?}",
    );
}

/// The same shape through the real entry point: an incomplete command in
/// a document must return conservative results, not crash the pass.
#[test]
fn textual_var_reference_scan_survives_an_incomplete_command_in_a_document() {
    for src in [
        "proc ::f {} { set x 1; return [set ",
        "proc ::f {} { set x 1; return [set {",
        "proc ::f {} { set x 1; return [set {$n",
    ] {
        let opts = run_pass(src);
        // No assertion on the verdicts — a half-typed document may
        // legitimately produce anything or nothing. The point is that it
        // returns at all.
        let _ = opts;
    }
}

#[test]
fn collect_textual_var_references_detects_qualified_set_read() {
    // ``[::set varname]`` form should also count.
    let source = "proc ::f {} { set x 1; set y [::set x]; return $y }";
    let subject = original_fixture_set_store_span(source);
    let opts = run_pass(source);
    let bad: Vec<_> = opts
        .iter()
        .filter(|o| {
            (o.code == DiagCode::O109 || o.code == DiagCode::O126)
                && o.span.start() <= subject.start()
                && o.span.end() >= subject.end()
        })
        .collect();
    assert_eq!(
        bad.len(),
        0,
        "[::set x] should count as a read for x; got {opts:?}",
    );
}

// O126/O109 RHS-purity gate

#[test]
fn o126_preserved_for_impure_command_sub_rhs() {
    // `set unused [puts hi]` discards the result but still prints —
    // the assignment must NOT be deleted (a latent FP: O126 would
    // otherwise fire unconditionally on dead chains).
    let opts = crate::optimiser::optimise(
        "proc ::f {} { set unused [puts hi]; return 1 }",
        &registry(),
    );
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O126),
        "impure cmd-sub RHS must be preserved, got {opts:?}",
    );
}

#[test]
fn o126_uses_original_callee_integer_completion_and_preserves_failures() {
    for (body, actual, erasable) in [
        ("expr {$a+$b}", "1 2", true),
        ("expr {$a+$b}", "abc 2", false),
        ("expr {$a/$b}", "1 0", false),
        ("expr {$a+$b}", "1", false),
        ("puts EFFECT; expr {$a+$b}", "1 2", false),
    ] {
        let source = format!(
            "proc add {{a b}} {{{body}}}; proc f {{}} {{set unused [add {actual}]; puts done}}"
        );
        let opts = crate::optimiser::optimise(&source, &registry());
        assert_eq!(
            opts.iter()
                .any(|opt| opt.code == DiagCode::O126 && opt.message.contains("unused")),
            erasable,
            "{source}: {opts:?}"
        );
    }
}

#[test]
fn o126_takes_the_store_of_a_pure_call_that_completes() {
    // A user proc proven pure by interproc analysis has no observable
    // side effect, and purity is no proof that the call completes
    // (`proc add {a b} {expr {$a + $b}}` raises for `add x 1`); but the
    // summary proves `return 1` completes whatever the call is given, so
    // the call cannot raise and `set unused [::pure]` is dead (tclsh
    // 8.4 to 9.1 run `::f` alike with and without it).
    let opts = crate::optimiser::optimise(
        "proc ::pure {} { return 1 }\nproc ::f {} { set unused [::pure]; return 1 }",
        &registry(),
    );
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O126),
        "a pure, completing proc's unused store should go, got {opts:?}",
    );
}

/// The store of a call goes only where the call cannot raise:
/// a callee whose body may raise, a word count its parameters reject, a
/// recursion, a read the existence rung does not hold set, a procedure
/// the caller's namespace shadows, a name a `rename` moves or a second
/// definition replaces, and a module that traces a command each keep it.
#[test]
fn o126_keeps_the_store_of_a_call_that_may_raise() {
    for (source, why) in [
        (
            "proc add {a b} {expr {$a + $b}}\nproc f {} {set unused [add x 1]; return 1}",
            "`expr` may raise",
        ),
        (
            "proc len {x} {return [string length $x]}\nproc f {} {set unused [len]; return 1}",
            "too few words",
        ),
        (
            "proc len {x} {return [string length $x]}\nproc f {} {set unused [len a b]; return 1}",
            "too many words",
        ),
        (
            "proc rec {n} {return [string length [rec $n]]}\nproc f {} {set unused [rec 1]; return 1}",
            "a recursion",
        ),
        (
            "proc first {x} {return [lindex $x 0]}\nproc f {} {set unused [first a]; return 1}",
            "`lindex` may raise",
        ),
        (
            "proc len {x} {return [string length $x]}\nproc f {} {set unused [len $y]; return 1}",
            "`y` is not set",
        ),
        (
            "proc ::len {x} {return [string length $x]}\nnamespace eval ns {\n    proc len {x} {puts $x; return 1}\n    proc f {} {set unused [len a]; return 1}\n}",
            "`len` in `::ns` is the printing `::ns::len`",
        ),
        (
            "proc len {x} {return [string length $x]}\nrename len other\nproc f {} {set unused [len a]; return 1}",
            "`len` was renamed away",
        ),
        (
            "proc len {x} {return [string length $x]}\nproc len {x} {error no}\nproc f {} {set unused [len a]; return 1}",
            "`len` is defined twice",
        ),
        (
            "proc len {x} {return [string length $x]}\nproc cb {args} {error no}\ntrace add execution len enter cb\nproc f {} {set unused [len a]; return 1}",
            "an execution trace may raise",
        ),
    ] {
        let opts = crate::optimiser::optimise(source, &registry());
        assert!(
            opts.iter()
                .all(|o| !matches!(o.code, DiagCode::O126 | DiagCode::O109 | DiagCode::O108)),
            "{why}: the store stays, got {opts:?}",
        );
    }
}

/// A completing call whose words read variables the existence rung holds
/// set leaves a dead store: `y` is a parameter.
#[test]
fn o126_takes_the_store_of_a_completing_call_whose_reads_are_set() {
    let opts = crate::optimiser::optimise(
        "proc len {x} {return [string length $x]}\nproc f {y} {set unused [len $y]; return 1}",
        &registry(),
    );
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O126),
        "a completing call over a set parameter leaves a dead store, got {opts:?}",
    );
}

#[test]
fn o126_still_fires_for_literal_rhs() {
    // The gate must not regress the literal case — `set y 42` has
    // no RHS side effect and stays foldable.
    let opts = run_pass("proc ::f {} { set y 42; return 1 }");
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O126),
        "literal RHS should still fold, got {opts:?}",
    );
}

// method-body O126

#[test]
fn runtime_selected_my_dispatch_is_not_assumed_pure() {
    // Tcl 9.0.4 resolves bare commands in the receiving object's runtime
    // namespace. Installing `${object_namespace}::my` shadows TclOO's
    // normal self-dispatch command, so even a same-named lexical method
    // proven pure is not an exact binding proof for `[my pure]`.
    let src = "oo::class create C {\n\
                   \x20   method pure {} { ::return 1 }\n\
                   \x20   method uses {} { ::set unused [my pure]; ::return 2 }\n\
                   }";
    let opts = crate::optimiser::optimise(src, &registry());
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O126),
        "runtime-selected `my` binding must preserve the RHS, got {opts:?}",
    );
}

#[test]
fn sf2_o126_preserves_impure_my_dispatch_in_method_body() {
    // An impure method (`puts`) must keep its self-dispatch — the
    // assignment is preserved so the side effect still runs.
    let src = "oo::class create C {\n\
                   \x20   method noisy {} { puts hi }\n\
                   \x20   method uses {} { set unused [my noisy]; return 2 }\n\
                   }";
    let opts = crate::optimiser::optimise(src, &registry());
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O126),
        "impure `my` self-dispatch RHS must be preserved, got {opts:?}",
    );
}

#[test]
fn sf2_instance_var_write_in_method_not_deleted() {
    // An instance-var write inside a method body is object state
    // that escapes the frame — it must not be flagged O109/O126
    // even when the method never reads it back.
    let src = "oo::class create C {\n\
                   \x20   variable n\n\
                   \x20   method bump {} { set n 5 }\n\
                   }";
    let opts = crate::optimiser::optimise(src, &registry());
    assert!(
        opts.iter()
            .all(|o| o.code != DiagCode::O109 && o.code != DiagCode::O126),
        "instance-var write must be preserved, got {opts:?}",
    );
}

#[test]
fn run_passes_dispatches_elimination() {
    let cu = CompilationUnit::build_for("if {0} { set x 1 }", &registry(), false);
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::Elimination]);
    // At minimum the dispatch must not panic; O107 may or
    // may not fire depending on how SCCP models this exact
    // shape, but running the pass must be side-effect free
    // otherwise.
    let only_o107 = ctx.optimisations.iter().all(|o| o.code == DiagCode::O107);
    assert!(only_o107, "unexpected codes: {:?}", ctx.optimisations);
}

/// A write the profile's registry does not have is not a write: under
/// `tcl8.4` there is no `lassign`, so the store ahead of one stays (#2144).
///
/// Measured on the real interpreters. For
///
/// ```tcl
/// proc unknown {args} {return -code error absent}
/// set a old
/// catch {lassign {new second} a b} m
/// puts $a
/// ```
///
/// tclsh 8.4.20 prints `old` — the body raises
/// the explicitly closed error-only fallback before any write and `catch` swallows
/// it — while 8.6.18 prints `new`. O109 used to delete `set a old` under
/// both, and the 8.4 program then failed with
/// `can't read "a": no such variable`.
#[test]
fn a_write_by_a_command_the_profile_lacks_does_not_kill_the_store() {
    let source = "proc unknown {args} {return -code error absent}\nset a old\ncatch {lassign {new second} a b} m\nputs $a\n";

    // The registry has to be the dialect's own, as production builds it
    // (`static_context_for_profile`): availability is a property of the
    // loaded surface, so a default registry would still carry `lassign`.
    let codes = |dialect: &str| {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        crate::optimiser::optimise_raw_for_profile(
            source,
            tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
            Some(profile),
        )
        .into_iter()
        .map(|o| o.code)
        .collect::<Vec<_>>()
    };

    // Under 8.4 the actual error-only fallback preserves `a`; catalogue
    // absence alone cannot exclude autoload or a mutating unknown handler.
    // Its value is forwarded and the store
    // that fed it is then genuinely dead. The rewritten program prints
    // `old`, which is what tclsh 8.4.20 prints.
    let early = codes("tcl8.4");
    assert!(
        early.contains(&DiagCode::O102),
        "8.4 must forward `old` — no lassign there to overwrite it: {early:?}",
    );

    // From 8.5 `lassign` really does write `a`, so its value at the `puts`
    // is unknown and nothing may be forwarded. The body stops at its first
    // error, so what it writes is a may-definition and the store before it
    // stays: the program still prints `new`.
    let late = codes("tcl8.6");
    assert!(
        !late.contains(&DiagCode::O102),
        "8.6 must not forward a value lassign overwrites: {late:?}",
    );
    assert!(
        !late.contains(&DiagCode::O109),
        "and a write the body may not reach leaves the store before it live: {late:?}",
    );
}

/// A call to a command the module cannot see may read a top-level name as
/// it may read `::x`, and a procedure's local through `upvar 1`, so the
/// store it can observe is no dead store; a store overwritten before any
/// such call is still dead.
#[test]
fn o109_keeps_a_store_a_call_the_module_cannot_see_may_read() {
    let dead = |src: &str| run_pass(src).iter().any(|o| o.code == DiagCode::O109);
    assert!(!dead("set x 1\nfoo\nset x 2\nputs $x\n"));
    assert!(!dead("set ::x 1\nfoo\nset ::x 2\nputs $::x\n"));
    assert!(!dead("set x 1\nsource other.tcl\nset x 2\nputs $x\n"));
    assert!(!dead(
        "proc p {} {\n set x 1\n source other.tcl\n set x 2\n puts $x\n}\n"
    ));
    assert!(!dead(
        "proc p {} {\n set x 1\n foo\n set x 2\n puts $x\n}\n"
    ));
    assert!(dead("set x 1\nset x 2\nfoo\nputs $x\n"));
    assert!(dead("proc p {} {\n set x 1\n set x 2\n foo\n puts $x\n}\n"));
    assert!(dead(
        "proc foo {} { puts hi }\nset x 1\nfoo\nset x 2\nputs $x\n"
    ));
}
#[test]
fn actual_jim_raise_proof_cannot_use_raw_logical_assistance() {
    // naming.optimisation.original-source-purpose
    // docs/design/analysis/name-resolution-proofs/optimisation-original-source-purpose.md
    let source = "set value VALUE; set copy $value";
    // Actual software Vm entry; no external Jim/provider execution.
    let environment = tcl_registry::model::ingress::resolve_environment("jimtcl");
    let profile = environment.unit_profile();
    let context = environment.default_context_registry();
    let registry = context.commands();
    let (_owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    assert!(captured.command_name_policy().is_none());
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let native = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(environment.grammar()),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&context),
    );
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_module(
        registry,
        &native.ir_module,
    )
    .unwrap();
    assert!(metadata.permits_logical_source_names());
    assert_eq!(
        native
            .ir_module
            .original_invocation_source_purpose(registry, metadata),
        Some(crate::command_binding::OriginalInvocationSourcePurpose::Execution)
    );
    let mut context_native = PassContext::new(&native.source, InterproceduralAnalysis::default());
    context_native.registry = Some(registry);
    context_native.ir_module = Some(&native.ir_module);
    let proof = RaiseProof::new(&context_native, &native.top_level, None, true);
    assert!(proof.metadata_available);
    assert!(!proof.logical_source_assistance);
    // Same assistance input, independently retained Logical source driver.
    let input = metadata.source_analysis_input().unwrap();
    let logical_entry =
        crate::command_binding::SourceAnalysisEntry::for_logical_source(input).unwrap();
    let logical = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&logical_entry),
        std::sync::Arc::clone(&context),
    );
    let mut context_logical = PassContext::new(&logical.source, InterproceduralAnalysis::default());
    context_logical.registry = Some(registry);
    context_logical.ir_module = Some(&logical.ir_module);
    let proof = RaiseProof::new(&context_logical, &logical.top_level, None, true);
    assert!(proof.metadata_available);
    assert!(proof.logical_source_assistance);
    context_logical.ir_module = None;
    let proof = RaiseProof::new(&context_logical, &logical.top_level, None, true);
    assert!(!proof.logical_source_assistance);
}
#[test]
fn literal_increment_target_uses_source_definition_not_reference_spelling() {
    let source = "set {$x} 1; incr {$x}; set x 2; incr x";
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input).unwrap();
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            config,
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    let mut ctx = PassContext::new(source, InterproceduralAnalysis::default());
    ctx.registry = Some(context.commands());
    ctx.ir_module = Some(&unit.ir_module);
    let mut targets = Vec::new();
    {
        let proof = RaiseProof::new(&ctx, &unit.top_level, None, true).with_source_unit(&unit);
        for (block, body) in &unit.top_level.cfg.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                if matches!(statement, Statement::Incr { .. }) {
                    targets.push(proof.original_incr_target(*block, index).unwrap());
                }
            }
        }
    }
    assert_eq!(targets.len(), 2);
    assert_ne!(targets[0].0, targets[1].0);
    assert!(targets.iter().any(|(_, name)| name == b"$x"));
    assert!(targets.iter().any(|(_, name)| name == b"x"));
    ctx.source = "foreign source";
    assert!(
        RaiseProof::new(&ctx, &unit.top_level, None, true)
            .with_source_unit(&unit)
            .source_unit
            .is_none()
    );
    ctx.source = source;
    let mut wrong = config;
    wrong.strict_quoting = !wrong.strict_quoting;
    ctx.source_lexer_config = Some(wrong);
    assert!(
        RaiseProof::new(&ctx, &unit.top_level, None, true)
            .with_source_unit(&unit)
            .source_unit
            .is_none()
    );
}
