// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lowering contract tests: each pins one decision the native tier takes on a
//! T0/T1 shape, through the real front end.

use std::collections::BTreeMap;

use tcl_registry::CommandRegistry;

use super::elide::{BarrierDecision, BarrierKept, IncrGuard};
use super::ir::{CompareKind, EntryProtocol, NativeFunction, NativeOp};
use super::{
    FunctionDecline, FunctionReport, LoweringInput, NativeLoweringDecline, StatementOutcome,
    lower_function,
};
use crate::compilation_unit::CompilationUnit;
use crate::dispatch_proof::DispatchEntryAssumption;
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
        mutations: &unit.command_mutations,
        config,
        escape: None,
        top_level: true,
        line_origin: 0,
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
fn renaming_a_math_function_stops_the_native_arm() {
    // `expr` resolves `abs(…)` through the command table: after
    // `rename ::tcl::mathfunc::abs {}` tclsh raises `invalid command name`,
    // so the compiler must not keep folding it to a native absolute value.
    let renamed = "rename ::tcl::mathfunc::abs {}\nset a -2\nputs [expr {abs($a)}]\n";
    let (function, _) = lower(renamed, native_config()).expect("lowers");
    assert!(
        count(&function, |op| matches!(op, NativeOp::MathFunc { .. })) >= 1,
        "the call must go back through the command table: {:?}",
        all_ops(&function)
    );
    // A `namespace import` into `::tcl::mathfunc` replaces the function just
    // as a rename does — tclsh answers `EVIL` for the sheet below — and no
    // name is *rebound*, so only the resolution-changed signal catches it.
    let imported = "namespace eval ::tcl::mathfunc { namespace import -force ::evil::abs }\n\
         set a -2\nputs [expr {abs($a)}]\n";
    let (function, _) = lower(imported, native_config()).expect("lowers");
    assert!(
        count(&function, |op| matches!(op, NativeOp::MathFunc { .. })) >= 1,
        "{:?}",
        all_ops(&function)
    );
    // The namespace transition may itself be reached through an alias prefix.
    // The closed binding owner must carry that lookup effect into every
    // compiler consumer; a second syntax-only scan would miss `mutate` here.
    let alias_imported = "namespace eval ::evil { proc abs x { return 999 }; namespace export abs }\n\
         interp alias {} mutate {} namespace import -force\n\
         namespace eval ::tcl::mathfunc { ::mutate ::evil::abs }\n\
         set a -2\nputs [expr {abs($a)}]\n";
    let (function, _) = lower(alias_imported, native_config()).expect("lowers");
    assert!(
        count(&function, |op| matches!(op, NativeOp::MathFunc { .. })) >= 1,
        "an alias-resolved import must return math dispatch to the command table: {:?}",
        all_ops(&function)
    );
    // Untouched, `abs` still folds to the inline compare/negate arm.
    let plain = "set a -2\nputs [expr {abs($a)}]\n";
    let (function, _) = lower(plain, native_config()).expect("lowers");
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::MathFunc { .. })),
        0,
        "{:?}",
        all_ops(&function)
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

#[test]
fn a_proc_statement_lowers_to_the_definition_shape() {
    let (function, report) = lower(
        "proc greet {name} { return hi }\ngreet bob\n",
        native_config(),
    )
    .expect("lowers");
    assert_eq!(
        outcomes(&report, "invoke"),
        vec![
            StatementOutcome::NativeDefinition,
            StatementOutcome::GenericInvoke
        ],
        "the definition takes the definition shape; the call stays generic"
    );
    let defines: Vec<&NativeOp> = all_ops(&function)
        .into_iter()
        .filter(|op| matches!(op, NativeOp::DefineProc { .. }))
        .collect();
    assert_eq!(
        defines,
        vec![&NativeOp::DefineProc {
            qualified_name: "::greet".into(),
            params_raw: "name".into(),
            body_source: " return hi ".into(),
        }],
        "the definition carries the front end's own name, params and body text"
    );
}

/// Lowering keeps the *first* definition of a name, so only that statement can
/// name a compiled body; a later `proc` of the same name stays a generic
/// invocation and installs an ordinary source-only procedure at run time.
#[test]
fn a_second_definition_of_one_name_stays_a_generic_invocation() {
    let (function, report) = lower(
        "proc pick {} { return first }\nproc pick {} { return second }\n",
        native_config(),
    )
    .expect("lowers");
    assert_eq!(
        outcomes(&report, "invoke"),
        vec![
            StatementOutcome::NativeDefinition,
            StatementOutcome::GenericInvoke
        ]
    );
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::DefineProc { .. })),
        1
    );
}

/// A statement carries the enclosing command's exact text and its line within
/// the body being compiled, which is everything the runtime needs to write the
/// `errorInfo` frame the eval loop would have written.
#[test]
fn a_statement_carries_the_site_its_error_frame_names() {
    let source = "set a 1\nputs [foo]\n";
    let (function, _) = lower(source, native_config()).expect("lowers");
    let sites: Vec<(u32, String)> = function
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter_map(|statement| {
            statement
                .site
                .as_ref()
                .map(|site| (site.line, site.text.clone()))
        })
        .collect();
    assert!(sites.contains(&(1, "set a 1".to_owned())), "{sites:?}");
    assert!(
        sites.contains(&(2, "puts [foo]".to_owned())),
        "a word evaluation names the whole command the eval loop would log: {sites:?}"
    );
}

/// The top-level script and a procedure body are entered differently, and the
/// lowering is the one place that decides which.
#[test]
fn the_top_level_script_and_a_procedure_body_take_different_entry_protocols() {
    let (top, _) = lower("set a 1\n", native_config()).expect("lowers");
    assert_eq!(top.protocol, EntryProtocol::Script);
}

/// A definition may only register words the statement writes out literally.
///
/// `Procedure` records the *written* body text, but lowering may have compiled
/// the body from a value it materialised instead — a const-mapped `$body`, or
/// a `[subst -nocommands …]` template — and it keeps the original word beside
/// that compiled body. Registering the word would report the wrong `info body`
/// and, worse, make any later run of the source body evaluate the substitution
/// in the *procedure's own frame*, where its operands do not exist.
///
/// The materialising paths only fire inside a procedure body (both consult the
/// const map, which is empty at depth 0), and no procedure-body site is proven
/// under `UnknownWorld` — so today the two never coincide. This lowers the
/// enclosing body under `PristineRegistryWorld` to remove that coincidence,
/// because it is exactly the assumption P5 proper introduces, and the point of
/// the guard is that the invariant holds by construction rather than by luck.
#[test]
fn a_definition_declines_a_body_the_statement_does_not_write_out() {
    let source = "proc make {} {\n set body {return hello}\n proc p {x} $body\n}\nmake\n";
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");

    // The front end really does record a body it did not compile.
    let inner = unit
        .ir_module
        .procedures
        .get("::p")
        .expect("the materialised body registers a procedure");
    assert_eq!(inner.body_source.as_deref(), Some("${body}"));
    assert_eq!(
        inner.body.statements.len(),
        1,
        "…while the compiled body came from the materialised `return hello`"
    );

    let outer = unit.procedures.get("::make").expect("::make is a unit");
    let facts = &outer.semantic_facts;
    let function = facts
        .executable()
        .function()
        .expect("::make builds executable IR");
    let hints = BTreeMap::new();
    let input = LoweringInput {
        registry: &registry,
        context: facts.context(),
        function,
        source: &unit.source,
        module: &unit.ir_module,
        mutations: &unit.command_mutations,
        config: native_config(),
        escape: None,
        top_level: false,
        line_origin: 0,
        entry_assumption: DispatchEntryAssumption::PristineRegistryWorld,
        type_hints: &hints,
    };
    let (lowered, report) = lower_function(&input).expect("::make lowers");
    assert_eq!(
        outcomes(&report, "invoke"),
        vec![StatementOutcome::GenericInvoke],
        "the definition keeps the runtime's own `proc`, which evaluates the \
         body word at the call site as Tcl does"
    );
    assert_eq!(
        count(&lowered, |op| matches!(op, NativeOp::DefineProc { .. })),
        0
    );
}

/// The same statement with a written-out body still binds, so the guard is a
/// rule about substitution rather than a blanket refusal.
#[test]
fn a_definition_with_a_written_body_still_binds_under_the_same_proof() {
    let source = "proc make {} {\n proc p {x} {return hello}\n}\nmake\n";
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    let outer = unit.procedures.get("::make").expect("::make is a unit");
    let facts = &outer.semantic_facts;
    let function = facts
        .executable()
        .function()
        .expect("::make builds executable IR");
    let hints = BTreeMap::new();
    let input = LoweringInput {
        registry: &registry,
        context: facts.context(),
        function,
        source: &unit.source,
        module: &unit.ir_module,
        mutations: &unit.command_mutations,
        config: native_config(),
        escape: None,
        top_level: false,
        line_origin: 0,
        entry_assumption: DispatchEntryAssumption::PristineRegistryWorld,
        type_hints: &hints,
    };
    let (lowered, _) = lower_function(&input).expect("::make lowers");
    assert_eq!(
        count(&lowered, |op| matches!(op, NativeOp::DefineProc { .. })),
        1
    );
}
