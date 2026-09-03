// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lowering contract tests: each pins one decision the native tier takes on a
//! T0/T1 shape, through the real front end.

use std::collections::BTreeMap;

use tcl_registry::CommandRegistry;

use super::elide::{BarrierDecision, BarrierKept, IncrGuard};
use super::ir::{CompareKind, NativeFunction, NativeOp};
use super::{
    FunctionDecline, FunctionReport, LoweringInput, NativeLoweringDecline, StatementOutcome,
    lower_function,
};
use crate::compilation_unit::CompilationUnit;
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};

fn native_config() -> SemanticOptimisationConfig {
    SemanticOptimisationConfig::new()
        .with_enabled(SemanticOptimisationPassId::NativeLowering)
        .with_enabled(SemanticOptimisationPassId::RepresentationInference)
        .with_enabled(SemanticOptimisationPassId::TraceBarrierElision)
        .with_enabled(SemanticOptimisationPassId::CellDemotion)
}

fn lower(
    source: &str,
    config: SemanticOptimisationConfig,
) -> Result<(NativeFunction, FunctionReport), FunctionDecline> {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    let facts = &unit.top_level.semantic_facts;
    let function = facts
        .executable()
        .function()
        .expect("the sample builds executable IR");
    let hints = BTreeMap::new();
    let input = LoweringInput {
        registry: &registry,
        context: facts.context(),
        function,
        source: &unit.source,
        module: &unit.ir_module,
        config,
        escape: None,
        top_level: true,
        entry_assumption: facts.dispatch_entry_assumption(),
        type_hints: &hints,
    };
    lower_function(&input)
}

/// Every operation in the function, arms of `IfElse` included.
fn all_ops(function: &NativeFunction) -> Vec<&NativeOp> {
    fn walk<'a>(ops: &'a [NativeOp], out: &mut Vec<&'a NativeOp>) {
        for op in ops {
            out.push(op);
            if let NativeOp::IfElse {
                then_ops, else_ops, ..
            } = op
            {
                walk(then_ops, out);
                walk(else_ops, out);
            }
        }
    }
    let mut out = Vec::new();
    for block in &function.blocks {
        for statement in &block.statements {
            walk(&statement.ops, &mut out);
        }
    }
    out
}

fn count(function: &NativeFunction, predicate: impl Fn(&NativeOp) -> bool) -> usize {
    all_ops(function)
        .into_iter()
        .filter(|op| predicate(op))
        .count()
}

fn outcomes(report: &FunctionReport, instruction: &str) -> Vec<StatementOutcome> {
    report
        .statements
        .iter()
        .filter(|record| record.instruction == instruction)
        .map(|record| record.outcome)
        .collect()
}

#[test]
fn set_incr_puts_is_native_with_one_box_at_the_boundary() {
    let (function, report) = lower("set a 1\nincr a\nputs $a\n", native_config()).expect("lowers");
    assert!(
        !all_ops(&function)
            .iter()
            .any(|op| matches!(op, NativeOp::EvalSource { .. })),
        "no source rung remains"
    );
    assert_eq!(
        outcomes(&report, "execute-lowered"),
        vec![StatementOutcome::Native, StatementOutcome::Native]
    );
    assert_eq!(
        outcomes(&report, "invoke"),
        vec![StatementOutcome::NativeIntrinsic]
    );
    // `incr a` on the shadow of `set a 1` is a proven native add.
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::IntBinary { .. })),
        1
    );
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::CellIncr { .. })),
        0
    );
    // Every cell access elided its trace barrier.
    for record in &report.statements {
        for cell in &record.cells {
            assert!(cell.barrier.is_elided(), "{cell:?}");
        }
    }
    // The `puts` reads the shadow, so no cell read reaches the runtime.
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::CellRead { .. })),
        0
    );
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::Puts { .. })),
        1
    );
}

#[test]
fn the_arithmetic_chain_is_straight_line_proven_i64() {
    let source = "set x 10\nset y 3\nset z [expr {$x * $y + 7}]\nset z [expr {$z - $x / $y}]\nincr z -1\nputs $z\nputs [expr {$z % 5}]\n";
    let (function, report) = lower(source, native_config()).expect("lowers");
    assert!(
        !all_ops(&function)
            .iter()
            .any(|op| matches!(op, NativeOp::EvalSource { .. } | NativeOp::ExprEval { .. })),
        "the whole chain is native"
    );
    // mul, add, div, sub, and the incr are proven on the shadows. The
    // registry declares `puts` as a possibly re-entrant invocation (a
    // reflected channel may run Tcl), so the first `puts` is a world barrier:
    // the final `%` re-reads `z` and takes the dynamic fast path.
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::IntBinary { .. })),
        5
    );
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::DynamicBinary { .. })),
        1
    );
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::Puts { .. })),
        1,
        "only the first `puts` is proven; the second follows the barrier"
    );
    assert!(
        report
            .statements
            .iter()
            .all(|record| !matches!(record.outcome, StatementOutcome::EvalSource(_)))
    );
}

#[test]
fn loop_counters_read_from_the_cell_take_the_dynamic_fast_path() {
    let source = "set i 0\nset sum 0\nwhile {$i < 20} {\n    incr i\n    if {$i % 3 == 0} continue\n    if {$i > 15} break\n    incr sum $i\n}\nputs \"$i $sum\"\n";
    let (function, report) = lower(source, native_config()).expect("lowers");
    let rungs: Vec<String> = all_ops(&function)
        .iter()
        .filter_map(|op| match op {
            NativeOp::EvalSource { text, reason } => Some(format!("eval {reason:?}: {text}")),
            NativeOp::ExprEval { text, .. } => Some(format!("expr: {text}")),
            _ => None,
        })
        .collect();
    assert!(
        rungs.is_empty(),
        "no source rung and no runtime expression: {rungs:?}"
    );
    assert!(
        count(&function, |op| matches!(
            op,
            NativeOp::DynamicCompare { .. }
        )) >= 2
    );
    assert!(count(&function, |op| matches!(op, NativeOp::CellIncr { .. })) >= 1);
    assert!(
        outcomes(&report, "invoke").contains(&StatementOutcome::NativeCompletion),
        "`break`/`continue` lower to their completion codes: {:?}",
        outcomes(&report, "invoke")
    );
}

#[test]
fn a_mixed_comparison_past_the_exact_double_range_takes_the_runtime_edge() {
    // tclsh 8.6.16 / 9.0.4: `9007199254740993 == 9007199254740992.0` is 0 and
    // the integer compares *greater*. Both sides through `f64` would answer 1,
    // so the native compare is only taken while the integer is exact in f64.
    let big = "set a 9007199254740993\nset b 9007199254740992.0\nputs [expr {$a == $b}]\n";
    let (function, _) = lower(big, native_config()).expect("lowers");
    assert_eq!(
        count(&function, |op| matches!(
            op,
            NativeOp::Compare {
                kind: CompareKind::F64,
                ..
            }
        )),
        0,
        "{:?}",
        all_ops(&function)
    );
    // A small integer against a double still compares natively.
    let small = "set a 3\nset b 2.5\nputs [expr {$a > $b}]\n";
    let (function, _) = lower(small, native_config()).expect("lowers");
    assert!(
        count(&function, |op| matches!(
            op,
            NativeOp::Compare {
                kind: CompareKind::F64,
                ..
            }
        )) >= 1
    );
}

#[test]
fn double_division_takes_the_runtime_operator() {
    // C Tcl raises ARITH DOMAIN for `0.0/0.0` but yields Inf for `1.0/0.0`,
    // and the double lattice cannot prove a divisor non-zero, so division
    // must not be emitted as a raw `f64.div` that stores NaN and continues.
    let source = "set a 1.0\nset b 0.0\nputs [expr {$a / $b}]\n";
    let (function, _) = lower(source, native_config()).expect("lowers");
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::DoubleBinary { .. })),
        0,
        "{:?}",
        all_ops(&function)
    );
    assert!(count(&function, |op| matches!(op, NativeOp::DynamicBinary { .. })) >= 1);
}

#[test]
fn doubles_lower_to_native_f64_arithmetic() {
    let source =
        "set r 2.5\nset area [expr {3.14159 * $r * $r}]\nputs $area\nputs [expr {$area > 19.0}]\n";
    let (function, _) = lower(source, native_config()).expect("lowers");
    assert!(
        count(&function, |op| matches!(op, NativeOp::DoubleBinary { .. })) >= 1,
        "{:?}",
        all_ops(&function)
    );
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::EvalSource { .. })),
        0
    );
}

#[test]
fn argument_expansion_keeps_the_source_rung_with_its_reason() {
    let (function, report) = lower("puts {*}$args\n", native_config()).expect("lowers");
    assert_eq!(
        outcomes(&report, "invoke"),
        vec![StatementOutcome::EvalSource(
            NativeLoweringDecline::ArgumentExpansion
        )]
    );
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::EvalSource { .. })),
        1
    );
}

#[test]
fn a_traced_variable_keeps_its_barrier_and_its_runtime_incr() {
    let source = "proc watch args {}\ntrace add variable a write watch\nset a 1\nincr a\n";
    let (function, report) = lower(source, native_config()).expect("lowers");
    let barriers: Vec<BarrierDecision> = report
        .statements
        .iter()
        .flat_map(|record| record.cells.iter().map(|cell| cell.barrier))
        .collect();
    assert!(
        barriers
            .iter()
            .all(|barrier| *barrier == BarrierDecision::Kept(BarrierKept::VariableTraced)),
        "{barriers:?}"
    );
    assert!(all_ops(&function).iter().any(|op| matches!(
        op,
        NativeOp::CellIncr {
            guard: IncrGuard::RuntimeOnly,
            ..
        }
    )));
}

#[test]
fn a_dynamic_trace_target_guards_incr_with_the_runtime_trace_bit() {
    let source = "set name a\ntrace add variable $name write puts\nset a 1\nincr a\n";
    let (function, _) = lower(source, native_config()).expect("lowers");
    assert!(all_ops(&function).iter().any(|op| matches!(
        op,
        NativeOp::CellIncr {
            guard: IncrGuard::RuntimeTraceBit,
            ..
        }
    )));
}

#[test]
fn the_pass_gates_decline_with_typed_reasons() {
    assert_eq!(
        lower("set a 1\n", SemanticOptimisationConfig::new()).err(),
        Some(FunctionDecline::PassDisabled)
    );
    assert_eq!(
        lower("foreach x {a b} {puts $x}\n", native_config()).err(),
        Some(FunctionDecline::UnloweredInstruction("operand-expression"))
    );
    assert_eq!(
        lower("catch {puts x} msg\n", native_config()).err(),
        Some(FunctionDecline::UnloweredInstruction("join-completion"))
    );
}

#[test]
fn representation_inference_off_boxes_every_value() {
    let config = SemanticOptimisationConfig::new()
        .with_enabled(SemanticOptimisationPassId::NativeLowering)
        .with_enabled(SemanticOptimisationPassId::TraceBarrierElision);
    let (function, _) = lower("set x 10\nset y [expr {$x * 3}]\n", config).expect("lowers");
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::IntBinary { .. })),
        0
    );
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::DynamicBinary { .. })),
        1
    );
}

#[test]
fn trace_barrier_elision_off_re_reads_every_cell() {
    let config = SemanticOptimisationConfig::new()
        .with_enabled(SemanticOptimisationPassId::NativeLowering)
        .with_enabled(SemanticOptimisationPassId::RepresentationInference);
    let (function, report) = lower("set a 1\nputs $a\n", config).expect("lowers");
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::CellRead { .. })),
        1
    );
    assert!(report.statements.iter().all(|record| {
        record
            .cells
            .iter()
            .all(|cell| cell.barrier == BarrierDecision::Kept(BarrierKept::PassDisabled))
    }));
}

#[test]
fn expression_operators_without_a_native_shape_use_the_runtime_operator() {
    let source = "set a 12\nset b 5\nset p [expr {$b ** 3}]\nset q [expr {\"abc\" eq \"abc\"}]\nset r [expr {$a in {1 12 3}}]\nset s [expr {max($a, $b) + min($a, $b)}]\nset t [expr {abs(-$a)}]\nset u [expr {double($a) / $b}]\n";
    let (function, _) = lower(source, native_config()).expect("lowers");
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::MathOp { .. })),
        3
    );
    assert_eq!(
        count(&function, |op| matches!(
            op,
            NativeOp::ExprEval { .. } | NativeOp::EvalSource { .. }
        )),
        0
    );
    assert!(count(&function, |op| matches!(op, NativeOp::IfElse { .. })) >= 3);
    // `double($a) / $b` is a division: no divisor can be proven non-zero, so
    // it takes the runtime operator rather than a raw `f64.div`.
    assert!(count(&function, |op| matches!(op, NativeOp::DynamicBinary { .. })) >= 1);
}

#[test]
fn a_command_inside_an_expression_goes_to_the_runtime_expression_intrinsic() {
    let (function, _) =
        lower("set x [expr {[string length abc] + 1}]\n", native_config()).expect("lowers");
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::ExprEval { .. })),
        1
    );
}

#[test]
fn a_nested_generic_command_word_is_a_nested_invocation() {
    let (function, report) =
        lower("set out { a }\nputs [string trim $out]\n", native_config()).expect("lowers");
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::NestedInvoke { .. })),
        1
    );
    // The nested command word makes the `puts` site unprovable, so it stays
    // a generic invocation rather than the intrinsic.
    assert_eq!(
        outcomes(&report, "invoke"),
        vec![StatementOutcome::GenericInvoke]
    );
}
