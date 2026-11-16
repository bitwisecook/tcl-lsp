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

//! The recursive body scan: calls in condition position and deeply nested bodies.

use super::*;

/// The interprocedural call-graph scanners each need a depth cap —
/// `scan_expr_for_calls`, `note_params_in_expr` and
/// `walk_collect_param_refs` once per `ExprNode` level (Tier 1A);
/// `scan_source_for_calls` (+ its `scan_value_substitutions` helper) once
/// per nested `[cmd …]` substitution / `ArgRole::Body` arg inside a
/// single word's raw text (Tier 1B). Both axes were genuinely unbounded,
/// independent of the statement-tree `MAX_INTERPROCEDURAL_WALK_DEPTH`
/// cap, and empirically overflowed the native stack (SIGABRT) in the low
/// thousands of levels on a 2 MiB thread (`cargo test`'s default). 3000 is
/// comfortably past that crash range and past both caps (256); the
/// assertion is that each scanner returns at all.
#[test]
fn deeply_nested_interprocedural_scans_survive() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let known: HashSet<ProcedureId> = HashSet::new();
    let source_module = crate::ir::Module::default();
    let params: HashSet<String> = ["x".to_owned()].into_iter().collect();
    let ctx = ScanCtx {
        known: &known,
        registry: &registry,
        dialect: None,
        params: &params,
        object_types: ObjectTypeMap::none().candidates,
        identities: crate::realm::CommandBindingRealm::none(),
        declared: None,
        positioned_substitutions: false,
        source_input: None,
        source_procedures: &source_module.procedures,
        source_module: &source_module,
        current_tokens: None,
        parameter_body: None,
    };

    // A 3000-deep `ExprNode` tree (nested unary `!` over `$x`).
    let mut node = crate::expr_ast::ExprNode::Var {
        text: "$x".into(),
        name: "x".into(),
        start: 0,
        end: 2,
    };
    for _ in 0..3000 {
        node = crate::expr_ast::ExprNode::Unary {
            op: crate::expr_ast::UnaryOp::Not,
            operand: Box::new(node),
        };
    }

    // Tier 1A walkers over the deep expr tree.
    let mut facts = LocalFacts::default();
    scan_expr_for_calls(&node, ctx, &mut facts, 0);
    note_params_in_expr(
        &node,
        &params,
        &mut facts,
        tcl_lexer::LexerConfig::default(),
        0,
    );
    let mut refs = Vec::new();
    walk_collect_param_refs(
        &node,
        &params,
        &mut refs,
        Some(tcl_lexer::LexerConfig::default()),
        0,
    );

    // Tier 1B walker over deeply nested command substitutions
    // `a [a [a [ … [x] … ]]]`: each level is a literal-headed command
    // whose argument holds the next `[…]` substitution, so
    // `scan_source_for_calls` recurses through `scan_value_substitutions`
    // once per bracket level.
    let mut deep_brackets = "x".to_owned();
    for _ in 0..3000 {
        deep_brackets = format!("a [{deep_brackets}]");
    }
    scan_source_for_calls(&deep_brackets, ctx, &mut facts, 0);
}

#[test]
fn call_in_if_condition_is_recorded() {
    // Calls embedded in an `if {[q]} ...` predicate are call-graph edges:
    // ::q must appear in ::a's direct calls.
    let ia = build(
        "proc ::q {} { return 1 }\n\
             proc ::a {} { if {[::q]} { puts hi } }",
    );
    let a = fixture_proc(&ia, "::a").expect("::a recorded");
    assert!(
        edge_labels(&ia, &a.calls).contains(&"::q".to_string()),
        "expected ::q in ::a's call graph, got {:?}",
        a.calls
    );
}

#[test]
fn call_inside_catch_in_if_condition_is_recorded() {
    // `if {[catch {p}]} ...` — ::p is reached via BODY-role
    // recursion into the `catch` argument.
    let ia = build(
        "proc ::p {} { return 1 }\n\
             proc ::a {} { if {[catch {::p}]} { puts hi } }",
    );
    let a = fixture_proc(&ia, "::a").expect("::a recorded");
    assert!(
        edge_labels(&ia, &a.calls).contains(&"::p".to_string()),
        "expected ::p in ::a's call graph via BODY recursion, got {:?}",
        a.calls
    );
}

#[test]
fn call_in_while_condition_is_recorded() {
    let ia = build(
        "proc ::q {} { return 1 }\n\
             proc ::a {} { while {[::q]} { break } }",
    );
    let a = fixture_proc(&ia, "::a").expect("::a recorded");
    assert!(
        edge_labels(&ia, &a.calls).contains(&"::q".to_string()),
        "expected ::q via while-condition expr scan, got {:?}",
        a.calls
    );
}

/// `collect_instance_var_writes`
/// and the mutually-recursive `scan_script`/`scan_statement`/
/// `scan_control_flow_statement` trio recurse once per nested
/// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch` body, with no
/// depth cap of their own before this fix. Transitively bounded to
/// `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass today, so this is
/// defence-in-depth / consistency with every other full-tree walker in
/// this crate, not a currently-reproducible crash. 1000 levels of
/// *source* nesting is comfortably past this new cap; the assertion is
/// that `build_interprocedural_analysis` returns at all, not what it
/// returns. Spawns its own big-stack thread since the lexer/CST/
/// segmenter stages upstream of this walker's own new cap still walk
/// the full un-truncated source nesting before lowering's cap trims
/// it — same rationale as
/// `structured::tests::deeply_nested_if_survives_structured_walk`.
#[test]
pub(super) fn deeply_nested_if_survives_interprocedural_scan() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut src = "proc ::p {} {\n".to_owned();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("set done 1\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    src.push_str("}\n");
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let _ = build(&src);
        })
        .unwrap()
        .join()
        .unwrap();
}
