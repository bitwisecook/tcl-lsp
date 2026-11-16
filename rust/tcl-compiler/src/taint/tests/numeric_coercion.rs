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

//! T100 for coerced operands: branch conditions, `switch` subjects, expression operands and nested command substitutions.

use super::*;

#[test]
fn branch_nested_sink_uses_retained_expression_and_read_owners() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let source = "proc f {} { set x [gets stdin]; if {[exec $x] eq \"ok\"} { return 1 } }";
    let (_, unit) = retained_warning_unit(source, &context);
    let unit = unit.with_interprocedural(registry, Some(profile));
    let warnings = find_taint_warnings_for_cu(&unit, registry, Some(profile));
    assert!(
        warnings
            .iter()
            .any(|warning| warning.code == DiagCode::T100),
        "the retained branch expression and exact read occurrence select exec's tainted argument: {warnings:?}"
    );

    let mut detached = unit.clone();
    detached.ir_module.retained_source_bindings = None;
    let warnings = find_taint_warnings_for_cu(&detached, registry, Some(profile));
    assert!(
        warnings
            .iter()
            .all(|warning| warning.code != DiagCode::T100),
        "the same spelling without the retained evaluation/read owner cannot select the sink value: {warnings:?}"
    );

    let clean_source = "proc f {} { set x [gets stdin]; if {[string length $x] > 0} { return 1 } }";
    let (_, clean_unit) = retained_warning_unit(clean_source, &context);
    let clean_unit = clean_unit.with_interprocedural(registry, Some(profile));
    let warnings = find_taint_warnings_for_cu(&clean_unit, registry, Some(profile));
    assert!(
        warnings
            .iter()
            .all(|warning| warning.code != DiagCode::T100),
        "the actual selected sanitizer remains clean in a branch expression: {warnings:?}"
    );
}

/// `extract_guard_var` and `collect_coercion_operands` each recurse once
/// per `ExprNode` level, so both need a depth cap. 3000-deep trees are
/// past the crash range and past `MAX_EXPR_NODE_DEPTH` (256); the
/// assertion is that each returns at all.
#[test]
fn deeply_nested_expr_guard_and_coercion_walks_survive() {
    // A 3000-deep nested-unary tree drives both walkers' recursion.
    let mut node = ExprNode::Var {
        text: "$x".into(),
        name: "x".into(),
        start: 0,
        end: 2,
    };
    for _ in 0..3000 {
        node = ExprNode::Unary {
            op: crate::expr_ast::UnaryOp::Not,
            operand: Box::new(node),
        };
    }
    let _ = extract_guard_var(&node, false, 0, tcl_lexer::LexerConfig::default());
    let mut out = Vec::new();
    collect_coercion_operands(
        &node,
        true,
        &mut out,
        0,
        (
            tcl_lexer::LexerConfig::default(),
            tcl_dialect::LexerGrammar::default(),
        ),
    );
}

/// FN fix — TP: a tainted variable used directly in an `if` branch
/// condition is evaluated exactly like any other braced `expr`, but
/// conditions live on `Terminator::Branch`, not in `ssa_block.statements`,
/// so the per-statement scan alone never reaches them —
/// `emit_branch_condition_warnings` is what covers them.
#[test]
fn t100_fires_for_tainted_if_condition() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "proc f {} {\n  set x [gets stdin]\n  if {$x + 1 > 5} {\n    puts big\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    let hit = warnings
        .iter()
        .find(|w| w.code == DiagCode::T100)
        .unwrap_or_else(|| panic!("expected T100 for tainted if-condition, got {warnings:?}"));
    // Tight span: just the `{...}` condition text, not the whole
    // `if {...} { puts big }` statement (which also spans the body).
    let text = &source[hit.span.start() as usize..hit.span.end() as usize];
    assert!(
        text == "$x",
        "expected the exact coercing variable reference, got {text:?}",
    );
}

/// TP: the same hazard through `while` and `for` conditions.
#[test]
fn t100_fires_for_tainted_while_and_for_conditions() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    for source in [
        "proc f {} {\n  set x [gets stdin]\n  while {$x < 10} {\n    incr x\n  }\n}\n",
        "proc f {} {\n  set x [gets stdin]\n  for {set i 0} {$i < $x} {incr i} {\n    puts $i\n  }\n}\n",
    ] {
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 for tainted loop condition in {source:?}, got {warnings:?}",
        );
    }
}

/// FP fix / TN: `if {$tainted eq "admin"}` raises nothing — `eq` is a
/// pure string comparison with no numeric coercion, so neither the
/// "numeric coercion" message nor any other T100 variant applies. This
/// also proves the branch-condition pass reuses the same
/// `collect_coercion_operands` operator-context filter as the
/// `AssignExpr`/`ExprEval` path, not a re-implementation that flags
/// every variable indiscriminately.
#[test]
fn t100_silent_for_string_only_if_condition() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source =
        "proc f {} {\n  set x [gets stdin]\n  if {$x eq \"admin\"} {\n    puts ok\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "eq is a pure string compare, expected no T100, got {warnings:?}",
    );
}

/// FP fix / TN: a `switch` on a tainted value raises no T100 in any of
/// its matching modes.
///
/// An exact `switch` is flattened into a chain of `StrEq` branch
/// terminators whose subject operand is an `ExprNode::Raw` holding the
/// normalised variable reference
/// (`cfg_builder::cfg_lower::switch_subject_operand`), so an arm's
/// coercion context is the one that `StrEq` gives its operands: none.
///
/// Every string mode is covered because none of them is an exception:
/// `-exact`, `-glob` and `-regexp` all match text, so there is no arm
/// spelling the message would fit. Tcl 9.1's `-integer` is the one numeric
/// mode; see `t100_for_a_tainted_switch_integer_subject`.
///
/// tclsh 8.6.18 and 9.0.4 both print `other` for `set c 0x10; switch -- $c
/// {16 {puts hex} default {puts other}}` — the subject is compared as the
/// string `0x10`, never coerced to 16.
#[test]
fn t100_silent_for_switch_arms_on_a_tainted_subject() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    for mode in ["--", "-exact --", "-glob --", "-regexp --"] {
        let source = format!(
            "proc f {{}} {{\n  set cmd [gets stdin]\n  switch {mode} $cmd {{\n    \
                 status {{ puts ok }}\n    version {{ puts 1 }}\n    \
                 default {{ puts none }}\n  }}\n}}\n"
        );
        let cu = CompilationUnit::build_for(&source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T100),
            "`switch {mode}` matches as a string, expected no T100, got {warnings:?}",
        );
    }
}

fn switch_integer_t100s(release: &str, body: &str) -> Vec<TaintWarning> {
    use crate::compilation_unit::CompilationUnit;
    let registry = tcl_registry::model::ingress::static_context_for(release).commands();
    let profile = tcl_dialect::DialectProfile::find(release).expect("catalogue profile");
    let source = format!("proc f {{}} {{\n  set cmd [gets stdin]\n  set ok 5\n  {body}\n}}\n");
    let cu = CompilationUnit::build_for_profile(&source, registry, false, profile)
        .with_interprocedural(registry, Some(profile));
    find_taint_warnings_for_cu(&cu, registry, Some(profile))
        .into_iter()
        .filter(|w| w.code == DiagCode::T100)
        .collect()
}

/// TP: Tcl 9.1's `switch -integer` (TIP 730) reads its subject with
/// `Tcl_GetWideIntFromObj`, so a tainted subject is the same coercion
/// hazard as a braced `expr` operand. tclsh 9.1.0 runs the `16` arm for
/// `switch -integer -- 0x10 {16 {puts hex} default {puts other}}` and
/// raises `expected integer but got "abc"` for a subject of `abc`.
#[test]
fn t100_for_a_tainted_switch_integer_subject() {
    for call in [
        "switch -integer -- $cmd {1 { puts one } default { puts none }}",
        "switch -int $cmd {1 { puts one }}",
        "switch -integer -- $cmd 1 { puts one } default { puts none }",
        "if {$ok} { switch -integer -- $cmd {1 { puts one }} }",
    ] {
        let hits = switch_integer_t100s("tcl9.1", call);
        assert_eq!(hits.len(), 1, "{call}: {hits:?}");
        assert_eq!(hits[0].variable, "cmd", "{call}");
        assert_eq!(hits[0].sink_command, "switch -integer operand", "{call}");
        assert!(
            hits[0].message.contains("numeric coercion"),
            "{call}: {}",
            hits[0].message
        );
    }
    // The squiggle is the coerced word, not the whole `switch`.
    let call = "switch -integer -- $cmd {1 { puts one }}";
    let hits = switch_integer_t100s("tcl9.1", call);
    let source = format!("proc f {{}} {{\n  set cmd [gets stdin]\n  set ok 5\n  {call}\n}}\n");
    let span = hits[0].span;
    assert_eq!(&source[span.start() as usize..span.end() as usize], "$cmd");
    // A tainted inline pattern is read as an integer too.
    let hits = switch_integer_t100s("tcl9.1", "switch -integer -- $ok $cmd { puts one }");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].variable, "cmd");
}

/// TN: no coercion reaches the tainted value — the option does not exist
/// before 9.1 (tclsh 9.0.4 rejects `-integer` as a bad option), the value
/// is brace-quoted text, a sanitiser hands over an integer it produced, the
/// subject is clean, the mode compares text (including when `-integer` is
/// only another option's value), or C rejects the option combination.
#[test]
fn t100_silent_for_switch_integer_without_a_coerced_tainted_word() {
    for (release, call) in [
        ("tcl9.0", "switch -integer -- $cmd {1 { puts one }}"),
        ("tcl8.6", "switch -integer -- $cmd {1 { puts one }}"),
        ("tcl9.1", "switch -integer -- {$cmd} {1 { puts one }}"),
        (
            "tcl9.1",
            "switch -integer -- [string length $cmd] {1 { puts one }}",
        ),
        ("tcl9.1", "switch -integer -- $ok {1 { puts one }}"),
        ("tcl9.1", "switch -glob -- $cmd {1* { puts one }}"),
        // `-integer` is `-matchvar`'s value here: a regexp switch.
        (
            "tcl9.1",
            "switch -matchvar -integer -regexp -- $cmd {1 { puts one }}",
        ),
        // C rejects the call before reading the subject.
        ("tcl9.1", "switch -integer -nocase -- $cmd {1 { puts one }}"),
        (
            "tcl9.1",
            "if {[scan $cmd %d n] == 1} { switch -integer -- $n {1 { puts one }} }",
        ),
    ] {
        let hits = switch_integer_t100s(release, call);
        assert_eq!(hits.len(), 0, "{release} `{call}`: {hits:?}");
    }
}

/// The same silence for an array element as a `switch` subject.
///
/// `$a(k)` normalises to a `${a(k)}` subject, whose reconstruction has to
/// match the array name the expr scan reports plus the index it does not.
/// It is one reference to one element, so the arm's `StrEq` context is the
/// whole answer here too.
#[test]
fn t100_silent_for_a_switch_on_a_tainted_array_element() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "proc f {} {\n  array set a [list k [gets stdin]]\n  \
                      switch -- $a(k) {\n    status { puts ok }\n    \
                      default { puts none }\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "an array-element switch subject matches as a string too, got {warnings:?}",
    );
}

/// TP guard for the test above: the `Raw` arm's whole-variable narrowing
/// must not reach a `Raw` that really is unparsed expression source with a
/// coercing operator inside it.
#[test]
fn t100_still_fires_for_a_tainted_operand_in_an_unparsed_expr() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "proc f {} {\n  set x [gets stdin]\n  expr {$x + 1}\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for a numeric-coercing operand, got {warnings:?}",
    );
}

/// FP fix — TN: a tainted variable that only appears inside a nested
/// `[cmd $x]` command substitution within a braced expr must not be
/// flagged as an "expr operand: numeric coercion" hit — it flows into
/// that nested call's own argument, a completely different risk
/// category (code execution, if `cmd` is itself dangerous) which that
/// call's own sink classification already covers independently.
#[test]
fn t100_expr_operand_message_not_raised_for_var_only_in_nested_command_sub() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    // `string length` is a sanitiser (fixed numeric return) and not a
    // sink, so the nested call itself raises nothing either — isolating
    // the "expr operand" false positive this test targets.
    let source = "proc f {} {\n  set x [gets stdin]\n  if {[string length $x] > 5} {\n    puts big\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "$x only reaches a nested command-sub argument, not a direct \
             expr operand; expected no T100, got {warnings:?}",
    );
}

/// TP: a tainted variable nested through a nested command sub still
/// raises T100 when that nested command is *itself* a sink.
#[test]
fn t100_fires_when_nested_command_sub_is_itself_a_sink() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source =
        "proc f {} {\n  set x [gets stdin]\n  if {[exec $x] eq \"ok\"} {\n    puts ok\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 from the nested `exec $x`, got {warnings:?}",
    );
}

/// TP: `$tainted` inside a ternary branch still flags when the ternary
/// itself sits in a numeric-coercion position — the branch's value
/// stands in for the ternary as a whole in the parent operator.
#[test]
fn t100_fires_for_tainted_var_through_ternary_branch_in_coercing_context() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "proc f {c} {\n  set x [gets stdin]\n  if {($c ? $x : 0) + 1 > 5} {\n    puts big\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for $x flowing through a coerced ternary branch, got {warnings:?}",
    );
}

/// TP: the ternary *condition* is always boolean-coerced, regardless of
/// the ternary's own surrounding context.
#[test]
fn t100_fires_for_tainted_var_in_ternary_condition() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source =
        "proc f {} {\n  set x [gets stdin]\n  if {$x ? \"a\" : \"b\"} {\n    puts ok\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for tainted ternary condition, got {warnings:?}",
    );
}
