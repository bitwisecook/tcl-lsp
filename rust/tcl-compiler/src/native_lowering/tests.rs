// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lowering contract tests: each pins one decision the native tier takes on a
//! T0/T1 shape, through the real front end.

use crate::interprocedural::test_body_identity::fixture_id_if_present;
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
    lower_with_unit(source, config).map(|(function, report, _)| (function, report))
}

fn lower_with_unit(
    source: &str,
    config: SemanticOptimisationConfig,
) -> Result<(NativeFunction, FunctionReport, CompilationUnit), FunctionDecline> {
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
    let (function, report) = lower_function(&input)?;
    Ok((function, report, unit))
}

/// Retain the real interpreter and its entry independently of source metadata.
fn original_boolean_unit(
    source: &str,
    environment: &str,
) -> (
    tcl_vm::Vm,
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    CompilationUnit,
) {
    original_boolean_unit_with_compilation(
        source,
        environment,
        crate::environment_ingress::authoring_native_compilation(),
    )
}

/// Caller-selected original compilation purpose, retaining the same actual
/// interpreter entry and independently supplied source metadata.
fn original_boolean_unit_with_compilation(
    source: &str,
    environment: &str,
    compilation: tcl_registry::native_compilation::NativeCompilationContext,
) -> (
    tcl_vm::Vm,
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    CompilationUnit,
) {
    let environment = tcl_registry::model::ingress::resolve_known_environment(environment)
        .expect("original Boolean control requires a known native environment");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    assert_eq!(native.execution_point, environment.point());
    assert!(native.execution_point.is_some());
    let entry = crate::command_binding::SourceAnalysisEntry {
        metadata_context:
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                &input,
            )),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: compilation,
        native_entry: Some(std::sync::Arc::new(native)),
        ..crate::command_binding::SourceAnalysisEntry::default()
    };
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    (owner, context, unit)
}

fn lower_original_boolean_unit(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    module: &crate::ir::Module,
) -> Result<(NativeFunction, FunctionReport), FunctionDecline> {
    let facts = &unit.top_level.semantic_facts;
    if std::env::var_os("TCL_LSP_TRACE_NATIVE_BOOLEAN_SOURCE").is_some() {
        let mut pending = vec![(Vec::<usize>::new(), &module.top_level)];
        while let Some((path, script)) = pending.pop() {
            eprintln!(
                "NATIVE_BOOLEAN_SOURCE path={path:?} statements={} source={:?} namespace={:?} sites={}",
                script.statements.len(),
                script.executed_source.as_ref().map(|source| (
                    source.base(),
                    source.text.len(),
                    source.origin.kind(),
                )),
                script.namespace_context,
                script.command_binding_sites.iter().count(),
            );
            for (index, statement) in script.statements.iter().enumerate() {
                let inline = statement.tokens();
                let retained = script.retained_source_tokens_for_statement(statement);
                let (kind, reason) = match statement {
                    crate::ir::Statement::Call { .. } => ("Call", None),
                    crate::ir::Statement::Barrier { reason, .. } => {
                        ("Barrier", Some(reason.as_str()))
                    }
                    crate::ir::Statement::NativeCall { .. } => ("NativeCall", None),
                    crate::ir::Statement::If { .. } => ("If", None),
                    crate::ir::Statement::AssignConst { .. } => ("AssignConst", None),
                    crate::ir::Statement::AssignValue { .. } => ("AssignValue", None),
                    crate::ir::Statement::AssignExpr { .. } => ("AssignExpr", None),
                    _ => ("Other", None),
                };
                eprintln!(
                    "NATIVE_BOOLEAN_STATEMENT path={path:?} index={index} kind={kind} reason={reason:?} span={:?} inline_words={:?} retained_words={:?} synthetic={:?}",
                    statement.span(),
                    inline.map(|tokens| tokens.words().len()),
                    retained.map(|tokens| tokens.words().len()),
                    inline.and_then(|tokens| tokens.synthetic),
                );
                for (child, body) in statement.child_scripts().into_iter().enumerate().rev() {
                    let mut child_path = path.clone();
                    child_path.extend([index, child]);
                    pending.push((child_path, body));
                }
            }
        }
    }
    let function = facts.executable().function().unwrap_or_else(|| {
        panic!(
            "actual source builds executable IR; retained availability={:?}; module metadata current={}; function metadata current={}; module grammar={:?}; function grammar={:?}",
            facts.executable(),
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)
                .is_some(),
            unit.top_level
                .invocation_metadata_context_for_module(registry, module)
                .is_some(),
            module.lexer_config,
            unit.top_level.source_lexer_config(),
        )
    });
    let hints = BTreeMap::new();
    lower_function(&LoweringInput {
        registry,
        context: facts.context(),
        function,
        source: &unit.source,
        module,
        mutations: &unit.command_mutations,
        config: native_config(),
        escape: None,
        top_level: true,
        line_origin: 0,
        entry_assumption: facts.dispatch_entry_assumption(),
        type_hints: &hints,
    })
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
    let (_owner, context, unit) = original_boolean_unit(source, "tcl9.0");
    let (function, report) =
        lower_original_boolean_unit(&unit, context.commands(), &unit.ir_module)
            .expect("lowers under its genuine native entry");
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
    fn retains_runtime_dispatch(function: &NativeFunction) {
        let literals: BTreeMap<_, _> = all_ops(function)
            .into_iter()
            .filter_map(|op| match op {
                NativeOp::ConstStr { dst, text } => Some((*dst, text.as_str())),
                _ => None,
            })
            .collect();
        assert!(
            all_ops(function).into_iter().any(|op| {
                matches!(op, NativeOp::MathFunc { name, args, .. }
                    if name == "abs" && args.len() == 1)
                    || matches!(op, NativeOp::NestedInvoke { argv, .. }
                if argv.len() == 2
                && literals.get(&argv[0]) == Some(&"expr")
                && literals.get(&argv[1]) == Some(&"abs($a)"))
            }),
            "runtime math dispatch must be retained: {:?}",
            all_ops(function)
        );
    }
    // `expr` resolves `abs(…)` through the command table: after
    // `rename ::tcl::mathfunc::abs {}` tclsh raises `invalid command name`,
    // so the compiler must not keep folding it to a native absolute value.
    let renamed = "rename ::tcl::mathfunc::abs {}\nset a -2\nputs [expr {abs($a)}]\n";
    let (function, _) = lower(renamed, native_config()).expect("lowers");
    retains_runtime_dispatch(&function);
    // A `namespace import` into `::tcl::mathfunc` replaces the function just
    // as a rename does — C9.0 answers 999 for this source.
    let imported = "namespace eval ::evil {proc abs x {return 999}; namespace export abs}\n\
         namespace eval ::tcl::mathfunc { namespace import -force ::evil::abs }\n\
         set a -2\nputs [expr {abs($a)}]\n";
    let (function, _) = lower(imported, native_config()).expect("lowers");
    retains_runtime_dispatch(&function);
    // The namespace transition may itself be reached through an alias prefix.
    // The closed binding owner must carry that lookup effect into every
    // compiler consumer; a second syntax-only scan would miss `mutate` here.
    let alias_imported = "namespace eval ::evil { proc abs x { return 999 }; namespace export abs }\n\
         interp alias {} mutate {} namespace import -force\n\
         namespace eval ::tcl::mathfunc { ::mutate ::evil::abs }\n\
         set a -2\nputs [expr {abs($a)}]\n";
    let (function, _) = lower(alias_imported, native_config()).expect("lowers");
    retains_runtime_dispatch(&function);
    // Untouched, `abs` still folds to the inline compare/negate arm.
    let plain = "set a -2\nputs [expr {abs($a)}]\n";
    let (function, _) = lower(plain, native_config()).expect("lowers");
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::MathFunc { .. })),
        0,
        "{:?}",
        all_ops(&function)
    );
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::NestedInvoke { .. })),
        0,
        "the unchanged installed abs implementation retains its native path"
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
    let source = "proc watch args {}\nset name [read stdin]\ntrace add variable $name write watch\nset a 1\nincr a\n";
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

/// A braced operand's backslash-newline folds in Tcl and not in Jim, so
/// native lowering hands the expression to the runtime rather than push the
/// raw bytes as a constant (#2227, found in review).
#[test]
fn a_braced_operand_with_a_line_continuation_goes_to_the_runtime() {
    let (function, _) =
        lower("set q [expr {{a\\\n    b} eq {a b}}]\n", native_config()).expect("lowers");
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::ExprEval { .. })),
        1
    );
    let (function, _) = lower("set q [expr {{ab} eq {ab}}]\n", native_config()).expect("lowers");
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::ExprEval { .. })),
        0,
        "a braced operand without one is still a constant"
    );
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
    let (function, report, unit) = lower_with_unit(
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
    let [
        NativeOp::DefineProc {
            entry_key,
            argv,
            dispatch,
        },
    ] = defines.as_slice()
    else {
        panic!("the exact definition retains one original argv: {defines:?}");
    };
    assert_eq!(*dispatch, super::ir::DefinitionDispatch::Proved);
    assert_eq!(
        *entry_key,
        crate::interprocedural::test_body_identity::fixture_id(
            &unit.ir_module.procedures,
            "::greet"
        )
    );
    assert_eq!(argv.len(), 4);
    assert_eq!(original_constant(&function, argv[0]), "proc");
    assert_eq!(original_constant(&function, argv[1]), "greet");
    assert_eq!(original_constant(&function, argv[2]), "name");
    assert_eq!(original_constant(&function, argv[3]), " return hi ");
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
///
/// Both halves are asserted here. The name promised a contrast and the body
/// only ever lowered a script, so the `ProcEntry` side — the one with the
/// interesting contract, since `Interp::run_proc` has already pushed the
/// frame and emitting `Script`'s prologue there would push a second one —
/// went untested (#2072).
#[test]
fn the_top_level_script_and_a_procedure_body_take_different_entry_protocols() {
    let (top, _) = lower("set a 1\n", native_config()).expect("lowers");
    assert_eq!(top.protocol, EntryProtocol::Script);

    let source = "proc p {x} { return $x }\np 1\n";
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    let body = fixture_id_if_present(&unit.procedures, "::p")
        .and_then(|id| unit.procedures.get(&id))
        .expect("::p is a unit");
    let facts = &body.semantic_facts;
    let function = facts
        .executable()
        .function()
        .expect("::p builds executable IR");
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
    let (proc_body, _) = lower_function(&input).expect("::p lowers");
    assert_eq!(proc_body.protocol, EntryProtocol::ProcEntry);
    assert_ne!(
        top.protocol, proc_body.protocol,
        "the two entry points must not agree — that is the whole contract"
    );
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
/// under `UnknownWorld`, so in the current lowering the two never coincide.
/// This lowers the enclosing body under `PristineRegistryWorld` to remove that
/// coincidence, so the guard is exercised as a rule that holds by construction
/// rather than by luck.
#[test]
fn a_definition_declines_a_body_the_statement_does_not_write_out() {
    let source = "proc make {} {\n set body {return hello}\n proc p {x} $body\n}\nmake\n";
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");

    // The front end really does record a body it did not compile.
    let inner = fixture_id_if_present(&unit.ir_module.procedures, "::p")
        .and_then(|id| unit.ir_module.procedures.get(&id))
        .expect("the materialised body registers a procedure");
    assert_eq!(inner.body_source.as_deref(), Some("${body}"));
    assert_eq!(
        inner.body.statements.len(),
        1,
        "…while the compiled body came from the materialised `return hello`"
    );

    let outer = fixture_id_if_present(&unit.procedures, "::make")
        .and_then(|id| unit.procedures.get(&id))
        .expect("::make is a unit");
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
    let outer = fixture_id_if_present(&unit.procedures, "::make")
        .and_then(|id| unit.procedures.get(&id))
        .expect("::make is a unit");
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

#[test]
fn native_lowering_cannot_bypass_an_unresolved_chunk_entry() {
    use crate::command_binding::{CommandAllocationSite, ExecutedScriptSource, SourceOriginId};
    use crate::native_compilation_admission::NativeCompilationAdmission;
    use std::sync::Arc;
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect("set before 1", &registry, false, "tcl9.0");
    let facts = &unit.top_level.semantic_facts;
    let mut function = facts.executable().function().unwrap().clone();
    let source = ExecutedScriptSource::materialised(
        CommandAllocationSite {
            source: Arc::new(SourceOriginId::authored(&Arc::from(unit.source.as_str()))),
            offset: 0,
        },
        vec![0],
        &unit.source,
    );
    function.native_compilation_admission = Some(Arc::new(NativeCompilationAdmission {
        source: Some(source),
        failure: None,
        provider_required: true,
    }));
    let hints = BTreeMap::new();
    let input = LoweringInput {
        registry: &registry,
        context: facts.context(),
        function: &function,
        source: &unit.source,
        module: &unit.ir_module,
        mutations: &unit.command_mutations,
        config: native_config(),
        escape: None,
        top_level: true,
        line_origin: 0,
        entry_assumption: facts.dispatch_entry_assumption(),
        type_hints: &hints,
    };
    assert_eq!(
        lower_function(&input).unwrap_err(),
        FunctionDecline::NativeCompilationAdmissionRequired
    );
}

#[test]
fn original_boolean_lowering_keeps_reached_operand_purposes_and_c84_left_identity() {
    // naming.numeric.original-primitive-boolean-vs-expression-truth
    // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
    // Software source/IR observation only. The independently measured original
    // opcode rows own actual truth values and getter effects.
    use tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose as Purpose;
    let source = "if {!$lhs} {set result A}; if {$lhs ? $rhs : $other} {set result B}; if {$lhs && $rhs} {set result C}; if {$lhs || $rhs} {set result D}";
    for environment in ["tcl8.4", "tcl9.1"] {
        // The selected original script-object compiler visits all four
        // expressions before their possible runtime condition outcomes.
        let (_owner, context, unit) = original_boolean_unit_with_compilation(
            source,
            environment,
            tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                ..crate::environment_ingress::authoring_native_compilation()
            },
        );
        let (function, _) = lower_original_boolean_unit(&unit, context.commands(), &unit.ir_module)
            .expect("genuine original condition source lowers");
        let purposes: Vec<_> = all_ops(&function)
            .into_iter()
            .filter_map(|op| match op {
                NativeOp::UnboxBool { purpose, .. } => Some(*purpose),
                _ => None,
            })
            .collect();
        assert!(
            purposes.contains(&Purpose::LogicalNot),
            "{environment}: {purposes:?}"
        );
        assert!(
            purposes.contains(&Purpose::ConditionalJump),
            "{environment}: {purposes:?}"
        );
        assert!(
            purposes.contains(&Purpose::LogicalAnd),
            "{environment}: {purposes:?}"
        );
        assert!(
            purposes.contains(&Purpose::LogicalOr),
            "{environment}: {purposes:?}"
        );
        assert_eq!(
            count(&function, |op| matches!(
                op,
                NativeOp::BooleanExpressionResult { .. }
            )),
            4
        );
        assert_eq!(
            count(&function, |op| matches!(
                op,
                NativeOp::Unbox {
                    target: super::ir::NativeType::Bool,
                    ..
                } | NativeOp::Truth { .. }
            )),
            0
        );
        if environment == "tcl8.4" {
            for (jump, final_instruction) in [
                (Purpose::LogicalAnd, Purpose::LogicalAndInstruction),
                (Purpose::LogicalOr, Purpose::LogicalOrInstruction),
            ] {
                let conversions: Vec<_> = all_ops(&function)
                    .into_iter()
                    .filter_map(|op| match op {
                        NativeOp::UnboxBool { src, purpose, .. }
                            if *purpose == jump || *purpose == final_instruction =>
                        {
                            Some((*src, *purpose))
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(conversions.len(), 3);
                assert_eq!(conversions[0].1, jump);
                assert_eq!(
                    conversions[1],
                    (conversions[0].0, final_instruction),
                    "same original lhs is reconverted first"
                );
                assert_eq!(conversions[2].1, final_instruction);
                assert_ne!(
                    conversions[2].0, conversions[0].0,
                    "the genuine right operand remains separate"
                );
            }
        } else {
            assert!(!purposes.contains(&Purpose::LogicalAndInstruction));
            assert!(!purposes.contains(&Purpose::LogicalOrInstruction));
        }
    }
}

#[test]
fn original_boolean_lowering_separates_inline_and_public_expression_result_producers() {
    // naming.numeric.original-primitive-boolean-vs-expression-truth
    // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
    use tcl_registry::native_boolean_truth::NativeBooleanExpressionResultProduction as Production;
    let (_owner, context, unit) = original_boolean_unit(
        "if {$condition} {set result A}; if {[set condition]} {set result B}",
        "tcl8.6",
    );
    let (function, _) = lower_original_boolean_unit(&unit, context.commands(), &unit.ir_module)
        .expect("both actual expression source routes lower");
    let productions: Vec<_> = all_ops(&function)
        .into_iter()
        .filter_map(|op| match op {
            NativeOp::BooleanExpressionResult { production, .. } => Some(*production),
            _ => None,
        })
        .collect();
    assert_eq!(productions, [Production::InlineExpression]);
    let combined: Vec<_> = all_ops(&function)
        .into_iter()
        .filter_map(|op| match op {
            NativeOp::ExprBoolEval { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(combined, ["[set condition]"]);
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::ExprEval { .. })),
        0,
        "a public condition has one completion-bearing result producer"
    );
    for block in &function.blocks {
        if let super::ir::NativeTerminator::Branch { condition, .. } = &block.terminator {
            assert_eq!(
                function.values[condition.0 as usize].ty,
                super::ir::NativeType::Bool
            );
        }
    }
}

#[test]
fn original_boolean_lowering_refuses_missing_point_or_changed_actual_source_owner() {
    // naming.numeric.original-primitive-boolean-vs-expression-truth
    // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
    let (_owner, context, unit) =
        original_boolean_unit("if {$condition} {set result yes}", "tcl8.6");
    assert!(lower_original_boolean_unit(&unit, context.commands(), &unit.ir_module).is_ok());
    let mut missing_entry = unit.ir_module.clone();
    missing_entry.source_entry.native_entry = None;
    let mut missing_point = unit.ir_module.clone();
    std::sync::Arc::make_mut(missing_point.source_entry.native_entry.as_mut().unwrap())
        .execution_point = None;
    let mut missing_input = unit.ir_module.clone();
    missing_input.source_metadata_input = None;
    let mut missing_entry_input = unit.ir_module.clone();
    missing_entry_input.source_entry.metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    let mut changed_grammar = unit.ir_module.clone();
    changed_grammar.lexer_config.braced_var = tcl_lexer::BracedVarStyle::Tcl9Nesting;
    let mut foreign = unit.ir_module.clone();
    let foreign_context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let original = unit.ir_module.source_metadata_input.as_ref().unwrap();
    foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        foreign_context,
        original.lexer_config(),
    ));
    let mut changed_availability = unit.ir_module.clone();
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(context.commands())),
    );
    changed_availability.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        older,
        original.lexer_config(),
    ));
    for refused in [
        missing_entry,
        missing_point,
        missing_input,
        missing_entry_input,
        changed_grammar,
        foreign,
        changed_availability,
    ] {
        assert_eq!(
            lower_original_boolean_unit(&unit, context.commands(), &refused).unwrap_err(),
            FunctionDecline::NativeCompilationAdmissionRequired
        );
    }
}

#[test]
fn original_boolean_lowering_keeps_cell_objects_and_withholds_unproved_literal_caches() {
    // naming.numeric.original-primitive-boolean-vs-expression-truth
    // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
    use tcl_registry::native_boolean_truth::NativeBooleanExpressionResultProduction as Production;
    let source = "set original 4294967296; if {$original} {set result cell}; if {4294967296} {set result literal}; if {1 + 2} {set result arithmetic}";
    let (_owner, context, unit) = original_boolean_unit(source, "tcl8.4");
    let (function, _) = lower_original_boolean_unit(&unit, context.commands(), &unit.ir_module)
        .expect("original read and generic expression paths lower");
    let ops = all_ops(&function);
    let cell_read = ops
        .iter()
        .find_map(|op| match op {
            NativeOp::CellRead { dst, place, .. } if place.base() == "original" => Some(*dst),
            _ => None,
        })
        .expect("truth retains an actual object read despite its known numeric value");
    let stages: Vec<_> = ops
        .iter()
        .filter_map(|op| match op {
            NativeOp::BooleanExpressionResult {
                src, production, ..
            } => Some((*src, *production)),
            _ => None,
        })
        .collect();
    assert_eq!(stages, [(cell_read, Production::InlineExpression)]);
    let combined: Vec<_> = ops
        .iter()
        .filter_map(|op| match op {
            NativeOp::ExprBoolEval { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(combined, ["4294967296", "1 + 2"]);
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::ExprEval { .. })),
        0
    );
    assert_eq!(
        count(&function, |op| matches!(op, NativeOp::Truth { .. })),
        0
    );
}

#[test]
fn original_inline_control_structure_retains_compiler_only_conditions_and_direct_refusal() {
    // Actual retained C compiler structure; no entered handler or callback chronology.
    let source = "if {$a} {set one 1}; if {$b} {set two 2}; if {$c} {set three 3}";
    for environment in ["tcl8.4", "tcl9.1"] {
        let (_owner, context, unit) = original_boolean_unit_with_compilation(
            source,
            environment,
            tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                ..crate::environment_ingress::authoring_native_compilation()
            },
        );
        assert_eq!(unit.ir_module.top_level.statements.len(), 3);
        assert!(
            unit.ir_module
                .top_level
                .statements
                .iter()
                .all(|statement| matches!(statement, crate::ir::Statement::If { .. }))
        );
        let script = &unit.ir_module.top_level;
        let last = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_module(
            context.commands(),
            &unit.ir_module,
        )
        .unwrap();
        assert!(
            last.source_binding
                .as_ref()
                .unwrap()
                .original_inline_control_metadata_for_input(
                    last,
                    unit.ir_module.source_metadata_input.as_ref().unwrap(),
                    context.commands()
                )
                .is_some()
        );
        assert!(
            crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                context.commands(),
                Some(metadata),
                last
            )
            .is_none()
        );
        assert!(
            crate::registry_invocation::original_structured_invocation_with_metadata_context(
                context.commands(),
                metadata,
                last
            )
            .is_some()
        );
        assert!(
            unit.top_level
                .semantic_facts
                .executable()
                .function()
                .is_some()
        );
        let (_direct_owner, direct_context, direct) = original_boolean_unit(source, environment);
        let direct_script = &direct.ir_module.top_level;
        let direct_tokens = direct_script
            .retained_source_tokens_for_statement(direct_script.statements.first().unwrap())
            .unwrap();
        assert!(
            direct_tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_inline_control_compilation(direct_tokens)
                .is_none()
        );
        assert!(
            crate::registry_invocation::InvocationMetadataContext::for_module(
                direct_context.commands(),
                &direct.ir_module
            )
            .is_some()
        );
    }
}

fn defined_original_argv(
    function: &NativeFunction,
) -> Vec<(crate::ir::ProcedureId, &[super::ir::NativeValueId])> {
    all_ops(function)
        .into_iter()
        .filter_map(|op| match op {
            NativeOp::DefineProc {
                entry_key, argv, ..
            } => Some((*entry_key, argv.as_slice())),
            _ => None,
        })
        .collect()
}

fn original_constant(function: &NativeFunction, id: super::ir::NativeValueId) -> &str {
    all_ops(function)
        .into_iter()
        .find_map(|op| match op {
            NativeOp::ConstStr { dst, text } if *dst == id => Some(text.as_str()),
            _ => None,
        })
        .expect("the admitted literal is transported by its already evaluated value")
}

#[test]
fn original_publication_retains_body_without_native_dispatch_permission() {
    let source = "proc :audit {} {return ORIGINAL}\n:audit\n";
    for environment in ["tcl8.6", "tcl9.0"] {
        let (_actual_owner, context, unit) = original_boolean_unit(source, environment);
        let declaration = unit
            .ir_module
            .procedures
            .values()
            .find(|procedure| procedure.span.start() == 0)
            .expect("the original declaration body is retained independently of compiler policy");
        let publication = declaration
            .publication
            .as_ref()
            .expect("the original word and caller scope issue source publication geometry");
        assert_eq!(
            publication.slot(),
            &tcl_core_types::ByteCommandSlot::new(
                tcl_core_types::ByteNamespacePath::root(),
                tcl_core_types::NameBytes::from(":audit"),
            )
        );
        assert_eq!(declaration.body_source.as_deref(), Some("return ORIGINAL"));
        assert_eq!(declaration.params_raw, "");
        let facts = &unit.top_level.semantic_facts;
        let function = facts
            .executable()
            .function()
            .expect("original retained call");
        let proof = crate::dispatch_proof::analyse_original_dispatch_stability(
            function,
            facts.dispatch_entry_assumption(),
            context.commands(),
            crate::registry_invocation::InvocationMetadataContext::for_module(
                context.commands(),
                &unit.ir_module,
            ),
        );
        let (block, instruction) =
            function
                .blocks
                .iter()
                .enumerate()
                .find_map(|(block, body)| {
                    body.instructions.iter().enumerate().find_map(|(instruction, value)| {
                matches!(value, crate::executable_ir::ExecutableInstruction::Invoke(invoke)
                    if invoke.source.span.start() == 0).then_some((block, instruction))
            })
                })
                .expect("the actual original declaration invocation");
        assert!(
            proof
                .site(
                    u32::try_from(block).unwrap(),
                    u32::try_from(instruction).unwrap()
                )
                .is_none(),
            "source publication cannot broaden Native execution-ledger permissions"
        );
    }
}

#[test]
fn original_root_colon_definition_preserves_evaluated_publication_operand() {
    let source = "proc :audit {} {return ORIGINAL}\n:audit\n";
    for environment in ["tcl8.6", "tcl9.0"] {
        let (_actual_owner, context, unit) = original_boolean_unit(source, environment);
        let (function, report) =
            lower_original_boolean_unit(&unit, context.commands(), &unit.ir_module)
                .expect("the fixed root declaration actually reaches native lowering");
        assert_eq!(
            outcomes(&report, "invoke"),
            vec![
                StatementOutcome::NativeDefinition,
                StatementOutcome::GenericInvoke
            ],
            "a QName-only or wholly declined control cannot establish the transport",
        );
        let definitions = defined_original_argv(&function);
        let [(entry_key, argv)] = definitions.as_slice() else {
            panic!("one original definition must reach DefineProc: {definitions:?}");
        };
        assert_eq!(argv.len(), 4);
        assert_eq!(original_constant(&function, argv[0]), "proc");
        assert_eq!(original_constant(&function, argv[1]), ":audit");
        assert_eq!(original_constant(&function, argv[2]), "");
        assert_eq!(original_constant(&function, argv[3]), "return ORIGINAL");
        let point = unit
            .ir_module
            .source_entry
            .native_entry
            .as_ref()
            .and_then(|entry| entry.execution_point)
            .expect("the independently captured actual entry supplies its point");
        let protocol = tcl_registry::InvocationDialect::of_point(point)
            .native_name_protocol()
            .expect("the actual C86/C9 point selects its original protocol");
        let root = tcl_syntax::naming::NativeNameContext::root();
        let actual_publication = protocol
            .command_publication_slot(root, original_constant(&function, argv[1]).as_bytes())
            .expect("original literal publication recipe");
        let wrong_projection = protocol
            .command_publication_slot(
                root,
                unit.ir_module
                    .procedures
                    .get(entry_key)
                    .expect("original DefineProc body")
                    .qualified_name
                    .as_bytes(),
            )
            .expect("internal key used as new text is a separate publication input");
        assert_eq!(
            actual_publication,
            tcl_core_types::ByteCommandSlot::new(
                tcl_core_types::ByteNamespacePath::root(),
                tcl_core_types::NameBytes::from(":audit"),
            ),
        );
        assert_ne!(actual_publication, wrong_projection);
    }
}

#[test]
fn original_substituted_name_retains_generic_evaluated_argv() {
    // The first declaration is literal. The second must not acquire an entry
    // merely because it eventually produces a name also found in a source map.
    let source = "set calls 0\nproc next_name {} {incr ::calls; return :audit}\n\
                  proc [next_name] {} {return COUNTED}\n:audit\n";
    for environment in ["tcl8.6", "tcl9.0"] {
        let (_actual_owner, context, unit) = original_boolean_unit(source, environment);
        let (function, _report) =
            lower_original_boolean_unit(&unit, context.commands(), &unit.ir_module)
                .expect("the structural control builds without replacing admission");
        assert!(
            defined_original_argv(&function)
                .iter()
                .all(|(_, argv)| { original_constant(&function, argv[1]) != ":audit" })
        );
        // This checks IR selection only. Runtime/emitted-WASM tests below must
        // separately demonstrate that [next_name] runs exactly once.
    }
}

#[test]
fn original_unknown_name_and_replaced_definer_keep_generic_invocation() {
    for source in [
        "proc $unknown_name {} {return NEVER}\n",
        "rename proc stock_proc\nstock_proc proc args {return SHADOW}\nproc :audit {} {return ORIGINAL}\n",
    ] {
        for environment in ["tcl8.6", "tcl9.0"] {
            let (_owner, context, unit) = original_boolean_unit(source, environment);
            let (function, report) =
                lower_original_boolean_unit(&unit, context.commands(), &unit.ir_module)
                    .expect("the original negative control retains its evaluated invocation");
            assert!(defined_original_argv(&function).is_empty(), "{source}");
            assert!(
                outcomes(&report, "invoke").contains(&StatementOutcome::GenericInvoke),
                "{source}: {report:?}",
            );
        }
    }
}

#[test]
fn original_definition_without_argv_is_rejected_as_invalid_executable_ir() {
    use crate::executable_ir::ExecutableInstruction;

    let (_owner, context, unit) =
        original_boolean_unit("proc :audit {} {return ORIGINAL}\n", "tcl9.0");
    let facts = &unit.top_level.semantic_facts;
    let mut function = facts.executable().function().unwrap().clone();
    let before = function
        .blocks
        .iter()
        .map(|block| block.instructions.len())
        .sum::<usize>();
    for block in &mut function.blocks {
        block
            .instructions
            .retain(|instruction| !matches!(instruction, ExecutableInstruction::BuildArgv { .. }));
    }
    assert!(
        function
            .blocks
            .iter()
            .map(|block| block.instructions.len())
            .sum::<usize>()
            < before
    );
    assert!(function.validate().is_err());
    let hints = BTreeMap::new();
    assert_eq!(
        lower_function(&LoweringInput {
            registry: context.commands(),
            context: facts.context(),
            function: &function,
            source: &unit.source,
            module: &unit.ir_module,
            mutations: &unit.command_mutations,
            config: native_config(),
            escape: None,
            top_level: true,
            line_origin: 0,
            entry_assumption: facts.dispatch_entry_assumption(),
            type_hints: &hints,
        })
        .unwrap_err(),
        FunctionDecline::InvalidExecutableIr,
    );
}

#[test]
fn original_increment_amount_keeps_braced_data_and_actual_scalar_reads_separate() {
    use super::cells::CellPlace;
    for environment in ["tcl8.6", "tcl9.0"] {
        for (amount, literal, reads_delta) in [
            ("{$delta}", Some("$delta"), false),
            ("1.5", Some("1.5"), false),
            ("$delta", None, true),
            ("\"$delta\"", None, true),
            (r"0\x33", Some("03"), false),
            (r"{0\x33}", Some(r"0\x33"), false),
        ] {
            let source = format!("set n 5; incr n {amount}");
            let (_owner, context, unit) = original_boolean_unit(&source, environment);
            assert!(
                unit.top_level
                    .semantic_facts
                    .executable()
                    .function()
                    .unwrap()
                    .blocks
                    .iter()
                    .flat_map(|block| &block.instructions)
                    .any(|instruction| matches!(instruction,
                        crate::executable_ir::ExecutableInstruction::ExecuteLowered(operation)
                            if matches!(operation.statement, crate::ir::Statement::Incr { .. })
                                && operation.original_tokens.is_some()
                    )),
                "{environment}: actual original lowered increment"
            );
            let (function, _) =
                lower_original_boolean_unit(&unit, context.commands(), &unit.ir_module)
                    .expect("actual original scalar update");
            assert_eq!(
                all_ops(&function).iter().any(|op| matches!(op,
                    NativeOp::CellRead { place: CellPlace::Named { name }, .. } if name == "delta"
                )),
                reads_delta,
                "{environment}: {source}"
            );
            if let Some(expected) = literal {
                let delta = all_ops(&function)
                    .into_iter()
                    .find_map(|op| match op {
                        NativeOp::CellIncr { delta, .. } => Some(*delta),
                        _ => None,
                    })
                    .expect("original literal delta reaches runtime integer conversion");
                assert_eq!(
                    original_constant(&function, delta),
                    expected,
                    "{environment}: {source}"
                );
            }
        }
    }
}

#[test]
fn original_assignment_values_decode_actual_words_without_name_reinterpretation() {
    use super::cells::CellPlace;
    for environment in ["tcl8.6", "tcl9.0"] {
        for (source, expected) in [
            (r"set payload a\x62", "ab"),
            (r"set payload {a\x62}", r"a\x62"),
            ("set payload {cash$name}", "cash$name"),
        ] {
            let (_owner, context, unit) = original_boolean_unit(source, environment);
            let (function, _) =
                lower_original_boolean_unit(&unit, context.commands(), &unit.ir_module)
                    .expect("actual original value assignment");
            let value = all_ops(&function)
                .into_iter()
                .find_map(|op| match op {
                    NativeOp::CellWrite {
                        place: CellPlace::Named { name },
                        src,
                        ..
                    } if name == "payload" => Some(*src),
                    _ => None,
                })
                .expect("actual original destination");
            assert_eq!(
                original_constant(&function, value),
                expected,
                "{environment}: {source}"
            );
            assert!(
                !all_ops(&function)
                    .iter()
                    .any(|op| matches!(op, NativeOp::CellRead { .. })),
                "{environment}: {source}"
            );
        }
    }
}

#[test]
fn original_native_descriptor_queries_retain_child_bindings_and_current_module_input() {
    use super::original_invocations::OriginalInvocations;
    use crate::registry_invocation::{
        InvocationMetadataContext, resolved_tokens_invocation_with_metadata_context,
    };
    let source = "puts [expr {1 + 2}]\n";
    for environment in ["tcl8.6", "tcl9.0"] {
        let (_owner, context, unit) = original_boolean_unit(source, environment);
        let registry = context.commands();
        let module = &unit.ir_module;
        let inventory = OriginalInvocations::for_module(module, registry, source)
            .expect("actual retained Module source inventory");
        let statement = &module.top_level.statements[0];
        let parent = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .expect("actual original parent carrier");
        let invoke = unit
            .top_level
            .semantic_facts
            .executable()
            .function()
            .unwrap()
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match instruction {
                crate::executable_ir::ExecutableInstruction::Invoke(invoke)
                    if invoke.source.span == statement.span() =>
                {
                    Some(invoke)
                }
                _ => None,
            })
            .expect("actual original generic invocation");
        assert_eq!(inventory.command(invoke), Some(parent));
        let metadata = InvocationMetadataContext::for_module(registry, module).unwrap();
        let calls =
            crate::word_subst::checked_original_argument_lifted_calls(parent, registry, metadata)
                .expect("complete original argument substitution inventory");
        assert_eq!(calls.len(), 1);
        let child = calls[0].tokens.as_ref().unwrap();
        let retained = inventory
            .substitution(child.words())
            .expect("exact original child and binding");
        assert_eq!(retained, child);
        let child_metadata = retained
            .source_binding
            .as_ref()
            .unwrap()
            .original_invocation_metadata_for_input(
                retained,
                module.source_metadata_input.as_ref().unwrap(),
                registry,
            )
            .expect("actual child source point and full input");
        let selected = resolved_tokens_invocation_with_metadata_context(
            registry,
            Some(child_metadata),
            retained,
        )
        .expect("independently selected original child binding");
        assert_eq!(
            registry.native_lowering_for_descriptor(
                selected.facts.descriptor_key(),
                Some(&child_metadata.context().authoring_query()),
            ),
            Some(tcl_registry::NativeLowering::Structured(
                tcl_registry::hooks::LoweringHookId::Expr
            ))
        );
        let (function, _) = lower_original_boolean_unit(&unit, registry, module).unwrap();
        assert_eq!(
            count(&function, |op| matches!(op, NativeOp::NestedInvoke { .. })),
            0
        );
        assert!(
            all_ops(&function).iter().any(|op| matches!(
                op,
                NativeOp::IntBinary { .. } | NativeOp::ConstInt { value: 3, .. }
            )),
            "{environment}: original expression recipe"
        );
        let mut changed_words = child.words().to_vec();
        let crate::ir::WordExpr::Literal { text, .. } = &mut changed_words[0] else {
            panic!("actual literal child head");
        };
        *text = "different".to_owned();
        assert!(inventory.substitution(&changed_words).is_none());
    }
}

#[test]
fn original_native_descriptor_queries_refuse_changed_source_input_or_foreign_carriers() {
    use super::original_invocations::OriginalInvocations;
    let source = "puts [expr {1 + 2}]\n";
    for environment in ["tcl8.6", "tcl9.0"] {
        let (_owner, context, unit) = original_boolean_unit(source, environment);
        let registry = context.commands();
        let module = &unit.ir_module;
        let inventory = OriginalInvocations::for_module(module, registry, source).unwrap();
        let parent = module
            .top_level
            .retained_source_tokens_for_statement(&module.top_level.statements[0])
            .unwrap();
        let invoke = unit
            .top_level
            .semantic_facts
            .executable()
            .function()
            .unwrap()
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| {
                if let crate::executable_ir::ExecutableInstruction::Invoke(invoke) = instruction {
                    Some(invoke)
                } else {
                    None
                }
            })
            .expect("actual original invocation");
        let mut missing = module.clone();
        missing.retained_source_bindings = None;
        assert!(OriginalInvocations::for_module(&missing, registry, source).is_none());
        let (function, _) = lower_original_boolean_unit(&unit, registry, &missing).unwrap();
        assert_eq!(
            count(&function, |op| matches!(op, NativeOp::NestedInvoke { .. })),
            1
        );
        let mut changed = module.clone();
        changed.source_metadata_input = None;
        assert!(OriginalInvocations::for_module(&changed, registry, source).is_none());
        let mut changed = module.clone();
        changed.lexer_config.braced_var =
            if changed.lexer_config.braced_var == tcl_lexer::BracedVarStyle::FirstClose {
                tcl_lexer::BracedVarStyle::Tcl9Nesting
            } else {
                tcl_lexer::BracedVarStyle::FirstClose
            };
        assert!(OriginalInvocations::for_module(&changed, registry, source).is_none());
        assert!(
            OriginalInvocations::for_module(module, registry, "puts [expr {4 + 5}]\n").is_none()
        );
        let original = module.source_metadata_input.as_ref().unwrap();
        let older = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(std::sync::Arc::clone(context.commands())),
        );
        let mut availability = module.clone();
        availability.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            original.analyser_profile(),
            original.unit_profile(),
            older,
            original.lexer_config(),
        ));
        assert_ne!(
            availability.source_metadata_input,
            module.source_metadata_input
        );
        assert!(OriginalInvocations::for_module(&availability, registry, source).is_none());
        let (_foreign_owner, _foreign_context, foreign) =
            original_boolean_unit(source, environment);
        let foreign_tokens = foreign
            .ir_module
            .top_level
            .retained_source_tokens_for_statement(&foreign.ir_module.top_level.statements[0])
            .unwrap();
        assert_ne!(parent.source_binding, foreign_tokens.source_binding);
        let mut foreign_invoke = invoke.clone();
        foreign_invoke
            .source_binding
            .clone_from(&foreign_tokens.source_binding);
        assert!(inventory.command(&foreign_invoke).is_none());
    }
}

#[test]
fn empty_source_with_withdrawn_metadata_or_unknown_history_cannot_use_standalone_descriptors() {
    let source = "puts [expr {1 + 2}]\n";
    let (_owner, context, unit) = original_boolean_unit(source, "tcl8.6");
    for unknown_entry in [false, true] {
        let mut module = unit.ir_module.clone();
        module.source = tcl_lexer::SourceImage::document("");
        module.retained_source_bindings = None;
        module.source_metadata_input = None;
        module.source_entry.native_entry = None;
        module.source_entry.metadata_context = if unknown_entry {
            crate::registry_invocation::OwnedInvocationMetadataContext::Standalone
        } else {
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable
        };
        module.source_entry.unknown_entry = unknown_entry;
        let (function, _) = lower_original_boolean_unit(&unit, context.commands(), &module)
            .expect("withdrawn source keeps the existing generic argv path");
        assert_eq!(
            count(&function, |op| matches!(op, NativeOp::NestedInvoke { .. })),
            1,
            "unknown_entry={unknown_entry}",
        );
    }
}

#[test]
fn original_native_definition_keeps_runtime_guard_separate_from_dispatch_proof() {
    use super::ir::DefinitionDispatch;
    for environment in ["tcl8.6", "tcl9.0"] {
        let (_owner, context, unit) =
            original_boolean_unit("proc :audit {} {return ORIGINAL}\n", environment);
        let (function, report) =
            lower_original_boolean_unit(&unit, context.commands(), &unit.ir_module).unwrap();
        assert!(outcomes(&report, "invoke").contains(&StatementOutcome::NativeDefinition));
        let definitions: Vec<_> = all_ops(&function)
            .into_iter()
            .filter_map(|op| match op {
                NativeOp::DefineProc { dispatch, argv, .. } => Some((dispatch, argv)),
                _ => None,
            })
            .collect();
        let [(DefinitionDispatch::RuntimeGuarded(identity), argv)] = definitions.as_slice() else {
            panic!("one genuinely guarded original definition: {definitions:?}");
        };
        let point = unit
            .ir_module
            .source_entry
            .native_entry
            .as_ref()
            .unwrap()
            .execution_point
            .unwrap();
        assert_eq!(
            *identity,
            tcl_registry::NativeLowering::Definition
                .definition_guard_identity(tcl_registry::InvocationDialect::of_point(point))
                .unwrap()
        );
        assert_eq!(original_constant(&function, argv[0]), "proc");
        assert_eq!(original_constant(&function, argv[1]), ":audit");
        assert_eq!(original_constant(&function, argv[3]), "return ORIGINAL");
        let absent = unit.ir_module.clone();
        let mut foreign = absent.clone();
        foreign.source_metadata_input = None;
        assert!(lower_original_boolean_unit(&unit, context.commands(), &foreign).is_err());
    }
}

#[test]
fn definition_policy_comparison_retains_canonical_release_and_all_other_axes() {
    use tcl_registry::InvocationDialect;
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let declared = InvocationDialect::of_profile(profile);
        let retained =
            InvocationDialect::of_point(tcl_dialect::model::DialectPoint::for_tcl_version(version));
        assert_ne!(
            declared, retained,
            "static profile stores its explicit release without a core_point"
        );
        assert!(retained.has_same_execution_policy(declared));
        assert!(
            !retained.has_same_execution_policy(InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::irules()
            ))
        );
        for other in tcl_dialect::TclVersion::ALL {
            if other != version {
                assert!(!retained.has_same_execution_policy(InvocationDialect::for_version(other)));
            }
        }
    }
}
