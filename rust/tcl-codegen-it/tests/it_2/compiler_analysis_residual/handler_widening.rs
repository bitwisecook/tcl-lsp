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

//! What a code-running handler widens: seeded parameters, array elements, expression bindings and collection classes.

use super::*;

fn seeded_parameter_result(source: &str) -> tcl_compiler::sccp::SccpResult {
    use std::collections::{BTreeSet, HashMap};
    use tcl_compiler::analyses::{ConstValue, LatticeValue};
    let registry = reg();
    let cu = CompilationUnit::build_for(source, &registry, false);
    let fu = cu.fixture_function("::p").expect("procedure");
    let seeds = HashMap::from([(("x".to_owned(), 0), LatticeValue::Const(ConstValue::int(5)))]);
    tcl_compiler::sccp::sccp(
        &fu.cfg,
        &fu.ssa,
        Some(&seeds),
        tcl_compiler::tcl_expr_eval::FoldPolicy::default(),
        tcl_compiler::sccp::TraceInputs {
            registry: &registry,
            source_metadata_input: fu.source_metadata_input(),
            traced_variables: &BTreeSet::new(),
            has_dynamic_variable_trace: false,
            deferred_writes: &tcl_compiler::ir::NO_DEFERRED_WRITES,
            analysis_context: None,
            existence: None,
        },
    )
}

#[test]
fn registry_handler_widens_seeded_parameter() {
    crate::install_surfaces();
    // Tcl 9.0.4 with its unmodified init.tcl: auto_index(missing_command)
    // loads a proc using `upvar 1 x x; set x 6`; p 5 returns changed.
    let result = seeded_parameter_result(
        "proc p {x} { missing_command; if {$x == 5} { return stale } else { return changed } }",
    );
    assert_eq!(
        result.constant_branches,
        [] as [tcl_compiler::sccp::ConstantBranch; 0]
    );
}

#[test]
fn conditional_registry_handler_widens_seeded_parameter() {
    crate::install_surfaces();
    let result = seeded_parameter_result(
        "proc p {x flag} { if {$flag} { missing_command }; if {$x == 5} { return stale } else { return changed } }",
    );
    assert_eq!(
        result.constant_branches,
        [] as [tcl_compiler::sccp::ConstantBranch; 0]
    );
}

#[test]
fn safe_registry_call_keeps_seeded_parameter_constant() {
    crate::install_surfaces();
    let result = seeded_parameter_result(
        "proc p {x} { string length safe; if {$x == 5} { return kept } else { return changed } }",
    );
    assert_eq!(result.constant_branches.len(), 1);
}

#[test]
fn an_unseen_call_preserves_dead_prior_and_future_definitions() {
    crate::install_surfaces();
    let registry = reg();
    let cu = CompilationUnit::build_for(
        "proc p {} { set first 5; set before [expr {$first + 1}]; missing_command; set after 7; return $after }",
        &registry,
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    for (name, expected) in [("first", 5), ("after", 7)] {
        let symbol = fu.ssa.var_symbol(name).expect("variable");
        assert!(
            fu.sccp.values.iter().any(|((var, _), value)| {
                *var == symbol
                    && *value
                        == tcl_compiler::analyses::LatticeValue::Const(
                            tcl_compiler::analyses::ConstValue::int(expected),
                        )
            }),
            "{name} must retain its constant: {:?}",
            fu.sccp.values
        );
    }
}

#[test]
fn registry_handler_widens_array_element_read_by_terminator() {
    crate::install_surfaces();
    let registry = reg();
    for tail in [
        "return $a(k)",
        "if {$a(k) == 5} {return stale} else {return changed}",
        "if {$flag} {set a(other) 9}; return $a(k)",
    ] {
        let source = format!("proc p {{flag}} {{set a(k) 5; missing_command; {tail}}}");
        let cu = CompilationUnit::build_for(&source, &registry, false);
        let fu = cu.fixture_function("::p").expect("procedure");
        let symbol = fu.ssa.var_symbol("a(k)").expect("element");
        let transitions: Vec<_> = fu
            .ssa
            .value_clobbers
            .values()
            .flat_map(|markers| markers.values())
            .filter_map(|versions| versions.get(&symbol))
            .collect();
        assert!(!transitions.is_empty(), "a(k) needs a fresh value boundary");
        for &&(prior, fresh) in &transitions {
            assert_eq!(
                fu.sccp.values.get(&(symbol, prior)),
                Some(&tcl_compiler::analyses::LatticeValue::Const(
                    tcl_compiler::analyses::ConstValue::int(5)
                ))
            );
            assert_eq!(
                fu.sccp.values.get(&(symbol, fresh)),
                Some(&tcl_compiler::analyses::LatticeValue::Overdefined)
            );
        }
        for (id, block) in &fu.cfg.blocks {
            if matches!(&block.terminator,
                Some(tcl_compiler::cfg::Terminator::Return {value: Some(value), ..})
                    if value == "$a(k)")
            {
                let version = fu.ssa.blocks[id].exit_versions[&symbol];
                assert_eq!(
                    fu.sccp.values.get(&(symbol, version)),
                    Some(&tcl_compiler::analyses::LatticeValue::Overdefined)
                );
            }
        }
        assert_eq!(
            fu.sccp.constant_branches,
            [] as [tcl_compiler::sccp::ConstantBranch; 0]
        );
    }
    let control =
        CompilationUnit::build_for("proc p {} {set a(k) 5; return $a(k)}", &registry, false);
    let fu = control.fixture_function("::p").expect("procedure");
    let symbol = fu.ssa.var_symbol("a(k)").expect("element");
    assert!(
        fu.sccp.values.iter().any(|((var, _), value)| {
            *var == symbol
                && *value
                    == tcl_compiler::analyses::LatticeValue::Const(
                        tcl_compiler::analyses::ConstValue::int(5),
                    )
        }),
        "the safe control must retain element precision"
    );
}

#[test]
fn brace_expression_handler_barrier_blocks_following_scalar_fold() {
    crate::install_surfaces();
    for expression in [
        "if {[expr {[missing_command]}]} {}",
        "set ignored [expr {[missing_command]}]",
    ] {
        let source = format!(
            "proc p {{}} {{set x 5; {expression}; if {{$x == 5}} {{return stale}} else {{return changed}}}}"
        );
        let cu = CompilationUnit::build_for(&source, &reg(), false);
        assert_eq!(
            cu.fixture_function("::p").unwrap().sccp.constant_branches,
            [] as [tcl_compiler::sccp::ConstantBranch; 0]
        );
    }
    let safe = CompilationUnit::build_for(
        "proc p {} {set x 5; if {[expr {[string length safe]}]} {}; if {$x == 5} {return kept} else {return changed}}",
        &reg(),
        false,
    );
    assert!(
        safe.fixture_function("::p")
            .unwrap()
            .sccp
            .constant_branches
            .iter()
            .any(|branch| branch.condition == "$x == 5")
    );
}

#[test]
fn registry_value_clobber_preserves_branch_before_and_widens_branch_after() {
    crate::install_surfaces();
    let source =
        "set x 5; if {$x == 5} {set y before}; missing_command; if {$x == 5} {set y after}";
    let cu = CompilationUnit::build_for(source, &reg(), false);
    let branches = &cu.top_level.sccp.constant_branches;
    assert_eq!(
        branches.len(),
        1,
        "only the pre-handler condition may fold: {branches:?}"
    );
    assert!(branches[0].value);
    assert!(
        branches[0].span.unwrap().end()
            < u32::try_from(source.find("missing_command").unwrap()).unwrap()
    );
    let x = cu.top_level.ssa.var_symbol("x").unwrap();
    for versions in cu
        .top_level
        .ssa
        .value_clobbers
        .values()
        .flat_map(|markers| markers.values())
    {
        let &(prior, fresh) = versions.get(&x).unwrap();
        assert_eq!(
            cu.top_level.sccp.values.get(&(x, prior)),
            Some(&tcl_compiler::analyses::LatticeValue::Const(
                tcl_compiler::analyses::ConstValue::int(5)
            ))
        );
        assert_eq!(
            cu.top_level.sccp.values.get(&(x, fresh)),
            Some(&tcl_compiler::analyses::LatticeValue::Overdefined)
        );
    }
}

#[test]
fn registry_value_clobber_preserves_seeded_parameter_before_handler() {
    crate::install_surfaces();
    let result = seeded_parameter_result(
        "proc p {x} {if {$x == 5} {set before yes}; missing_command; if {$x == 5} {return stale} else {return changed}}",
    );
    assert_eq!(result.constant_branches.len(), 1);
    assert!(result.constant_branches[0].value);
}

#[test]
fn braced_expression_rebinding_precedes_later_sibling_and_statement() {
    crate::install_surfaces();
    // tclsh 9.0.4: all forms print `changed`; the former pure procedure now
    // resolves to eval after the earlier expression runs the rename script.
    for (prefix, middle) in [
        (
            "",
            "list [expr {[rename aaa {}; rename eval aaa; set _ 0]}] [aaa {set x 6}]",
        ),
        (
            "",
            "list [expr {[rename aaa {}; rename eval aaa; set _ 0]}]; aaa {set x 6}",
        ),
        (
            "interp alias {} e {} expr; ",
            "list [e {[rename aaa {}; rename eval aaa; set _ 0]}]; aaa {set x 6}",
        ),
        (
            "interp alias {} e {} expr 1 +; ",
            "list [e {[rename aaa {}; rename eval aaa; set _ 0]}]; aaa {set x 6}",
        ),
    ] {
        let source = format!(
            "proc aaa args {{return 0}}; {prefix}set x 5; {middle}; if {{$x == 5}} {{puts stale}} else {{puts changed}}"
        );
        let cu = CompilationUnit::build_for(&source, &reg(), false);
        assert!(
            cu.top_level.sccp.constant_branches.is_empty(),
            "earlier expression transitions must reach later invocations: {middle}"
        );
    }
    for (prefix, head) in [
        ("", "expr"),
        ("interp alias {} e {} expr; ", "e"),
        ("interp alias {} e {} expr 1 +; ", "e"),
    ] {
        let source = format!(
            "proc aaa args {{return 0}}; {prefix}set x 5; list [{head} {{[string length safe]}}] [aaa {{set x 6}}]; if {{$x == 5}} {{puts kept}} else {{puts changed}}"
        );
        let safe = CompilationUnit::build_for(&source, &reg(), false);
        assert_eq!(safe.top_level.sccp.constant_branches.len(), 1);
        assert!(safe.top_level.sccp.constant_branches[0].value);
    }
}

#[test]
fn expression_alias_roles_use_bindings_before_substitution() {
    crate::install_surfaces();
    // The later redefinition must not hide the expression role of this alias
    // at its earlier invocation. Tcl 9.0.4 prints changed and kept respectively.
    for (expression, should_fold) in [
        ("[rename aaa {}; rename eval aaa; set _ 0]", false),
        ("[string length safe]", true),
    ] {
        let source = format!(
            "proc aaa args {{return 0}}; interp alias {{}} e {{}} expr; set x 5; set ignored \"[e {{{expression}}}][aaa {{set x 6}}]\"; if {{$x == 5}} {{puts kept}} else {{puts changed}}; rename e {{}}; proc e args {{return 0}}"
        );
        let cu = CompilationUnit::build_for(&source, &reg(), false);
        assert_eq!(
            cu.top_level.sccp.constant_branches.len(),
            usize::from(should_fold),
            "expression roles must reflect the invocation's source-order state"
        );
        if should_fold {
            assert!(cu.top_level.sccp.constant_branches[0].value);
        }
    }
}

#[test]
fn expression_alias_created_by_earlier_substitution_replays_nested_effects() {
    crate::install_surfaces();
    // Tcl 9.0.4: each mutating form prints changed; the pure control prints kept.
    for middle in [
        "list [interp alias {} e {} expr] [e {[rename aaa {}; rename eval aaa; set _ 0]}] [aaa {set x 6}]",
        "list [interp alias {} e {} expr 1 +] [e {[rename aaa {}; rename eval aaa; set _ 0]}] [aaa {set x 6}]",
        "set ignored \"[interp alias {} e {} expr][e {[rename aaa {}; rename eval aaa; set _ 0]}][aaa {set x 6}]\"",
        "list [expr {[interp alias {} e {} expr] ne \"\"}] [e {[rename aaa {}; rename eval aaa; set _ 0]}] [aaa {set x 6}]",
    ] {
        let source = format!(
            "proc aaa args {{return 0}}; set x 5; {middle}; if {{$x == 5}} {{puts stale}} else {{puts changed}}"
        );
        let cu = CompilationUnit::build_for(&source, &reg(), false);
        assert!(cu.top_level.sccp.constant_branches.is_empty(), "{middle}");
    }
    let safe = CompilationUnit::build_for(
        "proc aaa args {return 0}; set x 5; list [interp alias {} e {} expr] [e {[string length safe]}] [aaa {set x 6}]; if {$x == 5} {puts kept} else {puts changed}",
        &reg(),
        false,
    );
    assert_eq!(safe.top_level.sccp.constant_branches.len(), 1);
    assert!(safe.top_level.sccp.constant_branches[0].value);
}

#[test]
fn conditional_expression_binding_replay_keeps_skipped_paths() {
    crate::install_surfaces();
    // Tcl 9.0.4 prints changed: each skipped replacement leaves e as eval.
    for expression in [
        "0 && [interp alias {} e {} list]",
        "1 || [interp alias {} e {} list]",
        "1 ? 0 : [interp alias {} e {} list]",
        "0 ? [interp alias {} e {} list] : 0",
        "$flag && [interp alias {} e {} list]",
    ] {
        for middle in [
            format!("list [expr {{{expression}}}] [e {{set x 6}}]"),
            format!("set ignored [expr {{{expression}}}]; e {{set x 6}}"),
            format!("if {{{expression}}} {{puts ignored}}; e {{set x 6}}"),
        ] {
            let source = format!(
                "interp alias {{}} e {{}} eval; set flag 0; set x 5; {middle}; if {{$x == 5}} {{puts stale}} else {{puts changed}}"
            );
            let cu = CompilationUnit::build_for(&source, &reg(), false);
            assert!(
                !cu.top_level
                    .sccp
                    .constant_branches
                    .iter()
                    .any(|branch| branch.value && branch.condition.contains('x')),
                "{middle}"
            );
        }
    }
    let safe = CompilationUnit::build_for(
        "set x 5; list [expr {0 && [string length safe]}]; if {$x == 5} {puts kept} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        safe.top_level
            .sccp
            .constant_branches
            .iter()
            .any(|branch| branch.value)
    );
}

#[test]
fn global_script_and_unseen_call_both_widen_seeded_parameter() {
    crate::install_surfaces();
    for middle in [
        "set y [setter $::script][missing_command]",
        "list [setter $::script] [missing_command]",
        "if {[setter $::script] + [missing_command]} {puts ignored}",
        "foreach item [setter $::script][missing_command] {puts ignored}",
        "switch -- [setter $::script][missing_command] {default {puts ignored}}",
    ] {
        let source = format!(
            "proc setter {{body}} {{uplevel #0 $body}}; proc p {{x}} {{{middle}; if {{$x == 5}} {{return stale}} else {{return changed}}}}"
        );
        let result = seeded_parameter_result(&source);
        assert!(
            !result
                .constant_branches
                .iter()
                .any(|branch| branch.condition.contains('x')),
            "{middle}"
        );
        let cu = CompilationUnit::build_for(&source, &reg(), false);
        let fu = cu.fixture_function("::p").unwrap();
        for marker in [
            tcl_compiler::ir::SyntheticMarker::GlobalFrameScript,
            tcl_compiler::ir::SyntheticMarker::UnseenCall,
        ] {
            assert!(
                fu.cfg
                    .blocks
                    .values()
                    .flat_map(|block| &block.statements)
                    .any(|stmt| stmt.synthetic_marker() == Some(marker)),
                "{middle}: {marker:?}"
            );
        }
    }
    let safe = seeded_parameter_result(
        "proc setter {body} {uplevel #0 {set ::g 1}}; proc p {x} {set y [setter unused]literal; if {$x == 5} {return kept} else {return changed}}",
    );
    assert_eq!(safe.constant_branches.len(), 1);
    assert!(safe.constant_branches[0].value);
}

#[test]
fn catch_header_handler_widens_seeded_parameter_on_both_dispatch_paths() {
    crate::install_surfaces();
    use tcl_compiler::cfg_builder::{build_cfg_codegen_with_registry, build_cfg_with_registry};
    let registry = reg();
    for body in ["", "set body_value ok"] {
        let source = format!(
            "proc p {{x}} {{catch {{{body}}} [missing_command]; if {{$x == 5}} {{return stale}} else {{return changed}}}}"
        );
        assert_eq!(
            seeded_parameter_result(&source).constant_branches,
            [] as [tcl_compiler::sccp::ConstantBranch; 0]
        );
        let cu = CompilationUnit::build_for(&source, &registry, false);
        for cfg in [
            build_cfg_with_registry(&cu.ir_module, false, &registry),
            build_cfg_codegen_with_registry(&cu.ir_module, false, &registry),
        ] {
            assert!(
                cfg.procedures
                    .get(&fixture_id(&cfg.procedures, "::p"))
                    .expect("authored fixture body")
                    .blocks
                    .values()
                    .flat_map(|block| &block.statements)
                    .any(|stmt| stmt.synthetic_marker()
                        == Some(tcl_compiler::ir::SyntheticMarker::UnseenCall))
            );
        }
    }
    assert_eq!(
        seeded_parameter_result(
            "proc p {x} {catch {} [list result]; if {$x == 5} {return kept} else {return changed}}"
        )
        .constant_branches
        .len(),
        1
    );
}

#[test]
fn registry_handler_widens_collection_classes_before_later_updates() {
    crate::install_surfaces();
    for middle in ["missing_command", "set ignored [missing_command]"] {
        let source = format!(
            "oo::class create A {{}}; oo::class create B {{}}; set d [dict create k [A new]]; {middle}; dict set d k2 [A new]; set x [dict get $d k]"
        );
        let cu = CompilationUnit::build_for(&source, &reg(), false);
        let fu = cu.fixture_function("::top").unwrap();
        let x = fu.ssa.var_symbol("x").unwrap();
        assert!(
            fu.types
                .iter()
                .filter(|((symbol, _), _)| *symbol == x)
                .all(|(_, ty)| ty.class_name().is_none()),
            "{middle}: {:?}",
            fu.types
        );
    }
}

#[test]
fn constructor_effects_and_redefinitions_withdraw_collection_type_precision() {
    crate::install_surfaces();
    for source in [
        "oo::class create A {constructor {} {upvar 1 acc cell; set cell {changed}}}; set acc [list]; lappend acc [A new]; set x [lindex $acc 0]",
        "oo::class create A {}; set acc [list]; oo::define A constructor {} {upvar 1 acc cell; set cell {changed}}; lappend acc [A new]; set x [lindex $acc 0]",
        "oo::class create A {}; set acc [list]; missing_command; lappend acc [A new]; set x [lindex $acc 0]",
    ] {
        let cu = CompilationUnit::build_for(source, &reg(), false);
        let fu = cu.fixture_function("::top").unwrap();
        let x = fu.ssa.var_symbol("x").unwrap();
        let inferred: Vec<_> = fu
            .types
            .iter()
            .filter(|((symbol, _), _)| *symbol == x)
            .map(|(_, ty)| ty)
            .collect();
        assert_ne!(inferred, [] as [&tcl_compiler::types::TypeLattice; 0]);
        assert!(
            inferred.iter().all(|ty| ty.class_name().is_none()),
            "{source}: {inferred:?}"
        );
    }
}
