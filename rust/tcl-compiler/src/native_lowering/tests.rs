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

/// Retain the real interpreter and its entry independently of source metadata.
fn original_boolean_unit(
    source: &str,
    environment: &str,
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
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
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
    let function = facts
        .executable()
        .function()
        .expect("actual source builds executable IR");
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
    let body = unit.procedures.get("::p").expect("::p is a unit");
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
        let (_owner, context, unit) = original_boolean_unit(source, environment);
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
