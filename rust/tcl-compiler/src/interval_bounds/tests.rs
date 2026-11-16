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

use crate::interprocedural::test_body_identity::fixture_id;

use crate::analyser::Analyser;

fn context_test_unit(
    source: &str,
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> crate::compilation_unit::CompilationUnit {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    crate::compilation_unit::CompilationUnit::build_with_context_registry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        std::sync::Arc::clone(context),
    )
}

fn context_test_candidates(
    function: &crate::compilation_unit::FunctionUnit,
    semantics: super::BoundsSemantics<'_>,
) -> Vec<super::Candidate> {
    function
        .ssa
        .blocks
        .iter()
        .flat_map(|(&block, body)| {
            body.statements
                .iter()
                .enumerate()
                .flat_map(move |(index, row)| {
                    super::statement_candidates(
                        &row.statement,
                        crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index),
                        semantics,
                    )
                })
        })
        .collect()
}

#[test]
fn retained_bounds_context_keeps_availability_source_operands_and_store_identity() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // A custom gated source descriptor; no stock release-availability or Native-entry claim.
    use super::*;
    use std::sync::Arc;
    let source = "proc f {i} {lset values $i NEW}";
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let mut registry = CommandRegistry::build_default();
    let mut gated = registry.get("lset").unwrap().clone();
    gated.surface = registry.get("dict").unwrap().surface;
    registry.insert(gated);
    let context = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl9.0")
            .with_command_store(Arc::new(registry)),
    );
    let unit = context_test_unit(source, &context);
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::f"))
        .expect("authored fixture body");
    let original = function
        .ssa
        .blocks
        .iter()
        .find_map(|(&block, body)| {
            body.statements.iter().enumerate().find_map(|(index, _)| {
                SsaSourceView::at_statement(&function.ssa, block, index).source_tokens()
            })
        })
        .unwrap();
    assert!(
        original
            .source_binding
            .as_ref()
            .unwrap()
            .execution_is_unknown()
    );
    let selected = BoundsSemantics {
        registry: context.commands(),
        context: Some(context.as_ref().into()),
        grammar: profile.grammar,
    };
    let positive = context_test_candidates(function, selected);
    assert_eq!(positive.len(), 1);
    assert!(
        positive[0].conditional_handler,
        "source layout cannot create a normal native handler"
    );
    assert_eq!(positive[0].index_arg, "$i");
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(Arc::clone(context.commands()));
    assert!(Arc::ptr_eq(older.commands(), context.commands()));
    let unavailable = BoundsSemantics {
        context: Some((&older).into()),
        ..selected
    };
    assert!(context_test_candidates(function, unavailable).is_empty());
    let mut foreign = CommandRegistry::build_default();
    foreign.insert(tcl_registry::CommandSpec {
        name: "foreign-marker",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let foreign_context = context.with_command_store(Arc::new(foreign));
    let mismatched = BoundsSemantics {
        context: Some((&foreign_context).into()),
        ..selected
    };
    assert!(context_test_candidates(function, mismatched).is_empty());
    let replaced = context_test_unit(
        "proc lset {args} {}; proc f {i} {lset values $i NEW}",
        &context,
    );
    assert!(
        context_test_candidates(
            replaced
                .procedures
                .get(&fixture_id(&replaced.procedures, "::f"))
                .expect("authored fixture body"),
            selected
        )
        .is_empty()
    );
    let mut missing = original.clone();
    missing.source_binding = None;
    assert!(
        crate::registry_invocation::conditional_index_access_advice_with_metadata_context(
            selected.registry,
            selected.context,
            &missing
        )
        .is_none()
    );
}

#[test]
fn bounds_follow_live_command_identity_and_literal_body_quoting() {
    assert_eq!(
        bounds(
            "proc lindex {args} {return custom}; proc f {} {set l {a b}; set i 9; return [lindex $l $i]}"
        ),
        [] as [(String, String); 0]
    );
    assert_eq!(
        bounds("proc f {} {set l {a b}; set i 9; puts {[lindex $l $i]}"),
        [] as [(String, String); 0]
    );
    let renamed =
        bounds("rename lindex pick; proc f {} {set l {a b}; set i 9; return [pick $l $i]}");
    assert_eq!(renamed.len(), 1, "{renamed:?}");
    let prefixed =
        bounds("interp alias {} pick {} lindex {a b}; proc f {} {set i 9; return [pick $i]}");
    assert_eq!(prefixed.len(), 1, "{prefixed:?}");
}

#[test]
fn lset_append_updates_following_length_without_an_old_length_warning() {
    assert_eq!(
        bounds("proc f {} {set l {a}; lset l 1 b; set i 1; return [lindex $l $i]}"),
        [] as [(String, String); 0]
    );
    let past = bounds("proc f {} {set l {a}; lset l 1 b; set i 2; return [lindex $l $i]}");
    assert_eq!(past.len(), 1, "{past:?}");
}

#[test]
fn append_slot_bounds_follow_each_c_runtime_release() {
    let source = "proc f {} {set l {a}; set i 1; lset l $i b}";
    for (dialect, expected) in [
        ("tcl8.4", 1),
        ("tcl8.5", 1),
        ("tcl8.6", 0),
        ("tcl9.0", 0),
        ("tcl9.1", 0),
    ] {
        let diagnostics = Analyser::new().analyse(source, dialect).diagnostics;
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code.as_str() == "W231")
                .count(),
            expected,
            "{dialect}: {diagnostics:?}"
        );
    }
}

/// `walk_eager` recurses once per
/// `ExprNode` level, so it needs a depth cap. A tree built
/// directly is unbounded (the Pratt parser caps its own output at 256)
/// and empirically overflowed the native stack (SIGABRT) in the low
/// thousands of levels on a 2 MiB thread. 3000 is past that crash range
/// and past `MAX_EXPR_NODE_DEPTH` (256); the assertion is that it returns
/// at all.
#[test]
fn deeply_nested_walk_eager_survives() {
    use crate::expr_ast::{ExprNode, UnaryOp};
    let mut node = ExprNode::Literal {
        text: "1".into(),
        start: 0,
        end: 1,
    };
    for _ in 0..3000 {
        node = ExprNode::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(node),
        };
    }
    let mut count = 0usize;
    super::walk_eager(&node, &mut |_| count += 1);
    // The visitor ran without overflowing the native stack; it stops
    // descending at the cap, so it visits at most ~257 nodes here.
    assert!(count >= 1);
}

/// The `op` of every W233 divide-by-zero finding for `src`'s top level.
fn divzero(src: &str) -> Vec<&'static str> {
    use crate::compilation_unit::CompilationUnit;
    let context = tcl_registry::model::ingress::static_context_for("tcl9.0");
    let profile = context.commands().profile().expect("actual C9 profile");
    let cu = CompilationUnit::build_for_profile(src, context.commands(), false, profile);
    let fu = &cu.top_level;
    super::find_divide_by_zero_with(
        &fu.cfg,
        &fu.ssa,
        (&fu.sccp, &fu.types),
        &fu.sccp.executable_blocks,
        tcl_dialect::NumberSyntax::of_profile(Some(profile)),
        profile.grammar,
    )
    .iter()
    .map(|d| d.op)
    .collect()
}

#[test]
fn entered_operand_divisions_survive_parent_failure_and_respect_reachability() {
    for (source, expected) in [
        ("return [expr {1 / 0}]", 1),
        ("set d 0; return [expr {1 / $d}]", 1),
        ("error STOP; return [expr {1 / 0}]", 0),
        ("return [error STOP] [expr {1 / 0}]", 0),
        ("return {[expr {1 / 0}]}", 0),
        (
            "rename expr saved; proc expr args {return CUSTOM}; return [expr {1 / 0}]",
            0,
        ),
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code.as_str() == "W233")
                .count(),
            expected,
            "{source}: {:?}",
            result.diagnostics,
        );
    }
    assert!(
        !Analyser::new()
            .analyse("return [expr {1 / 0o0}]", "tcl8.4")
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "W233")
    );
    assert!(
        Analyser::new()
            .analyse("return [expr {1 / 0o0}]", "tcl9.1")
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "W233")
    );
}

#[test]
fn provably_zero_divisor_fires_w233() {
    // `$d` is the SCCP/interval constant 0 → `1 / $d` is a runtime error.
    assert_eq!(divzero("set d 0\nset x [expr {1 / $d}]"), vec!["/"]);
    assert_eq!(divzero("set d 0\nexpr {1 / $d}"), vec!["/"]);
    assert_eq!(divzero("set d 0\nexpr {5 % $d}"), vec!["%"]);
}

#[test]
fn divisor_ranges_keep_literal_sigils_and_array_elements_separate() {
    assert_eq!(divzero("set {$d} 0; set d 3; expr {1 / ${$d}}"), vec!["/"]);
    assert_eq!(
        divzero("set {$d} 3; set d 0; expr {1 / ${$d}}"),
        [] as [&str; 0]
    );
    // A root and another element cannot donate a zero divisor range.
    assert_eq!(
        divzero("set a(k) 3; set a(j) 0; expr {1 / $a(k)}"),
        [] as [&str; 0]
    );
}

#[test]
fn nonzero_or_guarded_divisor_is_clean() {
    // A non-zero divisor: no finding.
    assert_eq!(divzero("set d 3\nset x [expr {1 / $d}]"), [] as [&str; 0]);
    // Guarded by `$d != 0`: SCCP marks the division block unreachable.
    assert_eq!(
        divzero("set d 0\nif {$d != 0} { expr {1 / $d} }"),
        [] as [&str; 0]
    );
    // No division at all.
    assert_eq!(divzero("set x 1\nset y 2"), [] as [&str; 0]);
}

fn bounds(src: &str) -> Vec<(String, String)> {
    bounds_for(src, "tcl8.6")
}

fn bounds_for(src: &str, dialect: &str) -> Vec<(String, String)> {
    let mut a = Analyser::new();
    a.analyse(src, dialect)
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W230" | "W231" | "W232"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

/// The index `$j` is the list's length: an append from 8.6, a raise on
/// tclsh 8.4.20 and 8.5.19; a document naming no release is not told.
#[test]
fn lset_index_at_the_length_follows_the_release() {
    let src = "proc f {v} { set l {a b c}\n set j 3\n lset l $j $v }";
    for dialect in ["tcl8.4", "tcl8.5"] {
        let v = bounds_for(src, dialect);
        assert_eq!(v.len(), 1, "{dialect}: {v:?}");
        assert_eq!(v[0].0, "W231");
        assert!(v[0].1.contains("past the end (3)"), "{v:?}");
    }
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1", "tcl"] {
        assert!(bounds_for(src, dialect).is_empty(), "{dialect}");
    }
    let past = "proc f {v} { set l {a b c}\n set j 4\n lset l $j $v }";
    for dialect in ["tcl8.4", "tcl8.6", "tcl"] {
        assert_eq!(bounds_for(past, dialect).len(), 1, "{dialect}");
    }
}

#[test]
fn lset_loop_counter_past_append_fires_w231() {
    // `$j ∈ [4, 8]` against a length-3 list — the interval domain proves
    // every iteration is past the append slot.  The list length is
    // recovered through the loop-header phi `lset` induces.
    let v = bounds(
        "proc f {v} {\n    set l {a b c}\n    for {set j 4} {$j < 9} {incr j} { lset l $j $v }\n}\n",
    );
    assert_eq!(v.len(), 1, "{v:?}");
    assert_eq!(v[0].0, "W231");
    assert!(v[0].1.contains("$j"), "{v:?}");
}

#[test]
fn string_index_const_var_past_end_fires_w232() {
    // `$i` is the SCCP constant 10 against a 5-char string — past end.
    let source =
        "proc f {} {\n    set s \"hello\"\n    set i 10\n    return [string index $s $i]\n}\n";
    let result = bounds(source);
    assert_eq!(
        result.iter().map(|(c, _)| c.clone()).collect::<Vec<_>>(),
        vec!["W232"]
    );
    // `$i == length` is also out of range for `string index`.
    assert_eq!(
        bounds("proc f {} { set s \"hello\"\n set i 5\n return [string index $s $i] }").len(),
        1
    );
}

#[test]
fn dynamic_bounds_silent_when_not_provable() {
    // In-range index — no diagnostic.
    assert_eq!(
        bounds("proc f {} { set s \"hello\"\n set i 2\n return [string index $s $i] }").len(),
        0
    );
    // Unknown string + unknown index (both params) — not provable.
    assert_eq!(
        bounds("proc f {s i} { return [string index $s $i] }"),
        [] as [(std::string::String, std::string::String); 0]
    );
    // The legal append slot (`index == length`) for `lset` is silent.
    assert_eq!(
        bounds("proc f {v} { set l {a b c}\n set j 3\n lset l $j $v }"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn lset_preserves_length_through_linear_chain_fires_w231() {
    // (precision gain): a length-preserving `lset` is now
    // seen through to its input, so the length survives a linear `lset`
    // chain. `lset l 99` on the (still length-3) list is a real Tcl error
    // ("list index out of range") — W231 correctly fires.
    let v = bounds("proc f {v w} { set l {a b c}\n lset l 0 $v\n set j 99\n lset l $j $w }");
    assert_eq!(v.len(), 1, "{v:?}");
    assert_eq!(v[0].0, "W231");
}

#[test]
fn lset_after_length_growing_op_is_silent() {
    // (false-positive fix): a length-*growing* def
    // (`lappend`) in the loop body makes the list length unknown at the
    // `lset`, so no bound can be proven. The pre-loop length of 3 must NOT
    // be trusted — `lset l 5` is the legal append slot after two lappends.
    assert_eq!(
        bounds("proc f {v} { set l {a b c}\n foreach i {1} { lappend l x y\n lset l 5 $v }\n}",)
            .len(),
        0,
        "a length-growing op in the loop must poison the length, not trust the pre-loop value",
    );
    // `concat`/reassignment in the loop is equally opaque — no false W231.
    assert_eq!(
            bounds(
                "proc f {v} { set l {a b c}\n foreach i {1} { set l [concat $l x y z]\n lset l 5 $v }\n}",
            ).len(), 0,
        );
}
