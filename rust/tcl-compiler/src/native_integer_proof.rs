// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target-neutral proof objects for native integer addition.
//!
//! This module does not select a backend, allocate native slots, or emit code.
//! It turns existing SCCP, SSA def-use, type, caller-scope, and variable
//! observability facts into explicit evidence that a later `TclVM` or WASM
//! consumer may choose to use. The analysis is opt-in through
//! [`SemanticOptimisationPassId::NativeInteger`], which is disabled by default.
//!
//! Tcl integers are arbitrary precision in supported modern Tcl dialects. A
//! proof therefore never authorises wrapping arithmetic. An addition either
//! fits the selected signed native width, or its evidence explicitly requires
//! a checked operation whose overflow edge resumes boxed Tcl arithmetic.

use tcl_registry::{CommandRegistry, DispatchDependencies};
use tcl_syntax::expr::ast::{BinOp, ExprNode};

use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::BlockId;
use crate::common_aot_plan::{CommonAotProofPlan, DirectCallSiteId, DirectProcDecision};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::intervals::{Interval, compute_intervals_with, numbers_for_dialect};
use crate::ir::Statement;
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::ssa::{SsaStatement, ValueKey};
use crate::tcl_expr_eval::{FoldPolicy, TclValue, parse_integer_operand_with_policy};
use crate::types::{TypeLattice, TypeShape};
use crate::var_observability::analyse_var_observability;

/// Signed machine width a consumer proposes for an integer fast path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeIntegerWidth {
    /// A signed 32-bit payload.
    I32,
    /// A signed 64-bit payload.
    I64,
}

impl NativeIntegerWidth {
    const fn bounds(self) -> (i128, i128) {
        match self {
            Self::I32 => (i32::MIN as i128, i32::MAX as i128),
            Self::I64 => (i64::MIN as i128, i64::MAX as i128),
        }
    }
}

/// What to do when integral operands fit the native width but their sum may not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverflowPolicy {
    /// Decline unless overflow is statically impossible.
    ProveImpossible,
    /// Permit evidence for a checked add with a boxed Tcl arithmetic fallback.
    CheckedBoxedFallback,
}

/// Width and overflow policy for an explicitly enabled native integer pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeIntegerPolicy {
    width: NativeIntegerWidth,
    overflow: NativeOverflowPolicy,
}

impl Default for NativeIntegerPolicy {
    fn default() -> Self {
        Self {
            width: NativeIntegerWidth::I64,
            overflow: NativeOverflowPolicy::ProveImpossible,
        }
    }
}

impl NativeIntegerPolicy {
    /// Construct a target-neutral width and overflow policy.
    #[must_use]
    pub const fn new(width: NativeIntegerWidth, overflow: NativeOverflowPolicy) -> Self {
        Self { width, overflow }
    }
}

/// Stable location of one SSA expression assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAddSite {
    /// Qualified function containing the operation.
    pub function: String,
    /// CFG/SSA block containing the operation.
    pub block: BlockId,
    /// Statement index within `block`, or `None` for the block terminator.
    pub statement_index: Option<usize>,
    /// Identity of the Tcl result produced by this operation.
    pub result: NativeAddResult,
}

/// Result identity for an add candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeAddResult {
    /// An expression assignment defines this SSA value.
    Ssa(ValueKey),
    /// A direct `return [expr {...}]` produces the function completion value.
    FunctionReturn,
}

/// Existing analysis that supplied a finite operand range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRangeSource {
    /// Binding-safe direct-call evidence joined from caller SSA values or
    /// caller-scope-proven literals.
    BindingSafeDirectCaller,
    /// An exact or small-set integral SCCP value.
    Sccp,
    /// The post-SCCP SSA interval fixpoint.
    SsaInterval,
}

/// Evidence for one operand SSA value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeValueEvidence {
    /// Operand SSA value.
    pub value: ValueKey,
    /// Proven finite range.
    pub range: Interval,
    /// Analysis that supplied `range`.
    pub source: NativeRangeSource,
    /// Existing type-lattice fact at this SSA value.
    pub type_lattice: TypeLattice,
}

/// Arithmetic contract a backend must implement for accepted evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeAddExecution {
    /// The complete result range fits; a native add cannot overflow.
    OverflowImpossible,
    /// Use a checked native add and resume boxed Tcl arithmetic on overflow.
    CheckedWithBoxedFallback,
}

/// Proof that one Tcl integer addition can start in a native representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAddEvidence {
    /// Operation site and result SSA value.
    pub site: NativeAddSite,
    /// Left operand evidence.
    pub left: NativeValueEvidence,
    /// Right operand evidence.
    pub right: NativeValueEvidence,
    /// Exact sum in the shared interval lattice, when it fits that lattice's
    /// `i64` endpoints. `None` is legal only with
    /// [`NativeAddExecution::CheckedWithBoxedFallback`]: private `i128` proof
    /// arithmetic established that the operand ranges fit the selected width,
    /// but their Tcl sum can require a bignum.
    pub result_interval: Option<Interval>,
    /// Required non-wrapping execution contract.
    pub execution: NativeAddExecution,
    /// Additional common proofs or runtime guards an emitter must compose with
    /// this numeric fact. Numeric evidence alone never authorises execution.
    pub composition: NativeAddComposition,
}

/// Non-numeric premises required before consuming native add evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAddComposition {
    /// Binding-safe direct call sites whose actual values produced the ranges.
    pub direct_calls: Vec<DirectCallSiteId>,
    /// Mutable dispatch domains covering both procedure entry and the
    /// statically lowered internal `expr`/`return` operation.
    pub dispatch_dependencies: DispatchDependencies,
    /// The consumer must pair this proof with a materialised Tcl frame or a
    /// separate frame-elision proof; this module makes no storage decision.
    pub requires_frame_plan: bool,
    /// A live interpreter must guard the internal lowered operation's dispatch
    /// and trace stability, unless the host proves an equivalent sealed policy.
    pub requires_internal_operation_guard_or_sealed_policy: bool,
}

/// Why a candidate add did not produce native evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeIntegerDeclineReason {
    /// A required SSA value has no matching def-use chain/use site.
    MissingDefUseEvidence,
    /// The variable is traced or aliases storage outside the private frame.
    ObservableVariable,
    /// Module trace registration uses a dynamic variable name.
    DynamicVariableTrace,
    /// A parameter did not receive a binding-safe, closed-world caller seed.
    DynamicCallerInput,
    /// The operand is not proved to be an integer.
    NonIntegerOperand,
    /// The operand is a Tcl bignum and cannot enter the selected native width.
    BignumOperand,
    /// Existing SCCP/interval facts do not give a finite range.
    UnboundedOperand,
    /// The type lattice admits a non-integral runtime value.
    NonIntegralType,
    /// An operand range does not fit the selected native width.
    OperandOutsideNativeWidth,
    /// The sum may overflow and checked boxed fallback was not enabled.
    OverflowNotExcluded,
    /// No binding/trace-safe direct-procedure proof covers every caller.
    MissingDirectProcEvidence,
}

/// Outcome for one SSA-keyed add site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeAddDecision {
    /// Accepted proof evidence.
    Proven(Box<NativeAddEvidence>),
    /// Conservative refusal at this site.
    Declined {
        /// Operation site and result SSA value.
        site: NativeAddSite,
        /// Reason no fast path is authorised.
        reason: NativeIntegerDeclineReason,
    },
}

impl NativeAddDecision {
    /// Operation site regardless of acceptance or decline.
    #[must_use]
    pub fn site(&self) -> &NativeAddSite {
        match self {
            Self::Proven(evidence) => &evidence.site,
            Self::Declined { site, .. } => site,
        }
    }
}

/// Result of one opt-in function proof request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeIntegerProof {
    /// The default-off control declined without inspecting candidates.
    Disabled,
    /// No function with this qualified name exists in the compilation unit.
    FunctionUnavailable,
    /// The compiler's shared complexity guard disabled deep analysis.
    ComplexityGuarded,
    /// Candidate decisions in deterministic CFG order.
    Analysed(Vec<NativeAddDecision>),
}

/// Prove direct variable-variable additions in expression assignments.
///
/// Candidates are semantic `ExprNode::Binary(Add, Var, Var)` nodes already
/// produced by registry-driven lowering; this module contains no command-name
/// dispatch. Unsupported expression shapes are simply outside this first
/// vertical slice.
#[must_use]
pub fn prove_native_integer_adds(
    unit: &CompilationUnit,
    function: &str,
    registry: &CommandRegistry,
    optimisations: SemanticOptimisationConfig,
    policy: NativeIntegerPolicy,
    common_plan: &CommonAotProofPlan,
) -> NativeIntegerProof {
    if !optimisations.is_enabled(SemanticOptimisationPassId::NativeInteger) {
        return NativeIntegerProof::Disabled;
    }
    let Some(function_unit) = unit.function(function) else {
        return NativeIntegerProof::FunctionUnavailable;
    };
    if function_unit.complexity_guarded {
        return NativeIntegerProof::ComplexityGuarded;
    }

    // The numeral grammar of the release this unit was lowered for: a literal
    // operand's value depends on it (`0755` is 493 up to 8.6, 755 from 9.0), and
    // this proof turns a range into a native-width decision, so it must be the
    // target's grammar rather than whatever is ambient.
    let numbers = numbers_for_dialect(
        unit.ir_module
            .dialect
            .as_deref()
            .map(tcl_dialect::DialectProfile::by_name),
    );
    let intervals = compute_intervals_with(
        &function_unit.cfg,
        &function_unit.ssa,
        &function_unit.sccp.values,
        numbers,
    );
    let observability = analyse_var_observability(&function_unit.cfg, registry);
    let params: &[String] = match unit.ir_module.procedures.get(function) {
        Some(procedure) => procedure.params.as_slice(),
        None => &[],
    };
    let caller = match collect_caller_ranges(unit, function, registry, common_plan, params) {
        Ok(caller) => caller,
        Err(reason) => {
            return NativeIntegerProof::Analysed(collect_candidate_declines(
                function,
                function_unit,
                reason,
            ));
        }
    };

    let mut context = ProofContext {
        unit,
        function_unit,
        intervals: &intervals,
        observability: &observability,
        params,
        caller: &caller,
        block: function_unit.cfg.entry,
        statement_index: None,
        policy,
    };
    let mut decisions = prove_statement_candidates(function, &mut context);
    decisions.extend(prove_return_candidates(function, &mut context));
    NativeIntegerProof::Analysed(decisions)
}

fn prove_statement_candidates(
    function: &str,
    context: &mut ProofContext<'_, '_>,
) -> Vec<NativeAddDecision> {
    let mut decisions = Vec::new();
    let function_unit = context.function_unit;
    for block in function_unit.cfg.reverse_postorder() {
        let Some(ssa_block) = function_unit.ssa.blocks.get(&block) else {
            continue;
        };
        for (statement_index, statement) in ssa_block.statements.iter().enumerate() {
            let (expr, result) = match &statement.statement {
                Statement::AssignExpr { name, expr, .. } => {
                    let Some(result_symbol) = function_unit.ssa.var_symbol(name) else {
                        continue;
                    };
                    let Some(&result_version) = statement.defs.get(&result_symbol) else {
                        continue;
                    };
                    (expr, NativeAddResult::Ssa((result_symbol, result_version)))
                }
                Statement::Return {
                    expr: Some(expr), ..
                } => (expr, NativeAddResult::FunctionReturn),
                _ => continue,
            };
            let ExprNode::Binary {
                op: BinOp::Add,
                left,
                right,
            } = expr
            else {
                continue;
            };
            let (
                ExprNode::Var {
                    name: left_name, ..
                },
                ExprNode::Var {
                    name: right_name, ..
                },
            ) = (&**left, &**right)
            else {
                continue;
            };
            let site = NativeAddSite {
                function: function.to_owned(),
                block,
                statement_index: Some(statement_index),
                result,
            };
            context.block = block;
            context.statement_index = Some(statement_index);
            decisions.push(prove_add(
                context,
                site,
                Some(statement),
                left_name,
                right_name,
            ));
        }
    }
    decisions
}

fn prove_return_candidates(
    function: &str,
    context: &mut ProofContext<'_, '_>,
) -> Vec<NativeAddDecision> {
    let mut decisions = Vec::new();
    let function_unit = context.function_unit;
    for block in function_unit.cfg.reverse_postorder() {
        let Some(cfg_block) = function_unit.cfg.blocks.get(&block) else {
            continue;
        };
        let Some(crate::cfg::Terminator::Return {
            expr:
                Some(ExprNode::Binary {
                    op: BinOp::Add,
                    left,
                    right,
                }),
            ..
        }) = &cfg_block.terminator
        else {
            continue;
        };
        let (
            ExprNode::Var {
                name: left_name, ..
            },
            ExprNode::Var {
                name: right_name, ..
            },
        ) = (&**left, &**right)
        else {
            continue;
        };
        let site = NativeAddSite {
            function: function.to_owned(),
            block,
            statement_index: None,
            result: NativeAddResult::FunctionReturn,
        };
        context.block = block;
        context.statement_index = None;
        decisions.push(prove_add(context, site, None, left_name, right_name));
    }
    decisions
}

struct ProofContext<'a, 'o> {
    unit: &'a CompilationUnit,
    function_unit: &'a FunctionUnit,
    intervals: &'a std::collections::HashMap<ValueKey, Interval>,
    observability: &'a crate::var_observability::VarObservability<'o>,
    params: &'a [String],
    caller: &'a CallerRangeEvidence,
    block: BlockId,
    statement_index: Option<usize>,
    policy: NativeIntegerPolicy,
}

struct CallerRangeEvidence {
    ranges: std::collections::HashMap<String, Interval>,
    call_sites: Vec<DirectCallSiteId>,
    dispatch_dependencies: DispatchDependencies,
}

#[allow(
    clippy::too_many_lines,
    reason = "the caller-evidence walk shares one conservative decline path across every supported call shape"
)]
fn collect_caller_ranges(
    unit: &CompilationUnit,
    function: &str,
    registry: &CommandRegistry,
    common_plan: &CommonAotProofPlan,
    params: &[String],
) -> Result<CallerRangeEvidence, NativeIntegerDeclineReason> {
    if let Some(call_facts) = unit.caller_scope.call_sites.get(function)
        && (call_facts.arg_counts.contains(&0)
            || call_facts
                .arg_counts
                .iter()
                .any(|count| *count != params.len()))
    {
        // `record_unenumerable_caller` contributes arity zero for this fixed-
        // arity procedure. It poisons a mixed direct+dynamic caller set even
        // when the common plan can separately select one direct site.
        return Err(NativeIntegerDeclineReason::DynamicCallerInput);
    }

    // Joined operand ranges bound this procedure's parameters only if the
    // selected call sites are *all* of its callers.  A declined candidate
    // naming it — a same-arity call reached through an alias, a rebound name,
    // a site inside exceptional control flow — still invokes it with values no
    // selected site carries, so the evidence is refused rather than narrowed
    // against an incomplete caller set.
    if common_plan.has_declined_direct_call_to(function) {
        return Err(NativeIntegerDeclineReason::MissingDirectProcEvidence);
    }

    let mut ranges = std::collections::HashMap::<String, Interval>::new();
    let mut call_sites = Vec::new();
    let mut dependencies = DispatchDependencies::BASE;
    for (id, decision) in common_plan.direct_calls() {
        let DirectProcDecision::Selected(direct) = decision else {
            continue;
        };
        if direct.callee.qualified_name != function {
            continue;
        }
        let (caller, statement, args) = direct_actuals(unit, id)
            .ok_or(NativeIntegerDeclineReason::MissingDirectProcEvidence)?;
        if args.len() != params.len() {
            return Err(NativeIntegerDeclineReason::MissingDirectProcEvidence);
        }
        let caller_intervals = compute_intervals_with(
            &caller.cfg,
            &caller.ssa,
            &caller.sccp.values,
            numbers_for_dialect(
                unit.ir_module
                    .dialect
                    .as_deref()
                    .map(tcl_dialect::DialectProfile::by_name),
            ),
        );
        let caller_observability = analyse_var_observability(&caller.cfg, registry);
        for (param, argument) in params.iter().zip(args.iter()) {
            let range =
                if let Some(name) = crate::value_shapes::whole_word_scalar_var_name(argument) {
                    if unit.ir_module.has_dynamic_variable_trace
                        || unit.ir_module.traced_variables.contains(name)
                        || caller_observability.is_escaping_at(
                            id.block,
                            id.statement_index as usize,
                            name,
                        )
                    {
                        return Err(NativeIntegerDeclineReason::ObservableVariable);
                    }
                    let symbol = caller
                        .ssa
                        .var_symbol(name)
                        .ok_or(NativeIntegerDeclineReason::MissingDefUseEvidence)?;
                    let version = statement
                        .uses
                        .get(&symbol)
                        .copied()
                        .ok_or(NativeIntegerDeclineReason::MissingDefUseEvidence)?;
                    let key = (symbol, version);
                    if let Some(range) = range_from_sccp(caller.sccp.values.get(&key)) {
                        range?
                    } else {
                        caller_intervals
                            .get(&key)
                            .copied()
                            .filter(|range| bounded(*range))
                            .ok_or(NativeIntegerDeclineReason::UnboundedOperand)?
                    }
                } else {
                    // Flattened call argument text is accepted as a literal only
                    // when the existing caller-scope seed retained the same value;
                    // that seed already checked word provenance, binding, and every
                    // known caller. Templates/substitutions never reach this path.
                    let seeded = unit
                        .caller_scope
                        .param_constants_by_proc
                        .get(function)
                        .and_then(|seeds| {
                            seeds.iter().find(|(name, version, value)| {
                                name == param && *version == 0 && value == argument
                            })
                        })
                        .ok_or(NativeIntegerDeclineReason::DynamicCallerInput)?;
                    crate::intervals::constant(parse_caller_integer(&seeded.2, registry)?)
                };
            ranges
                .entry(param.clone())
                .and_modify(|current| *current = join_intervals(*current, range))
                .or_insert(range);
        }
        dependencies = dependencies.union(direct.dispatch_dependencies);
        call_sites.push(id.clone());
    }
    if call_sites.is_empty() {
        return Err(NativeIntegerDeclineReason::DynamicCallerInput);
    }
    if ranges.len() != params.len() {
        return Err(NativeIntegerDeclineReason::MissingDirectProcEvidence);
    }
    call_sites.sort();
    Ok(CallerRangeEvidence {
        ranges,
        call_sites,
        dispatch_dependencies: dependencies,
    })
}

fn direct_actuals<'a>(
    unit: &'a CompilationUnit,
    id: &DirectCallSiteId,
) -> Option<(&'a FunctionUnit, &'a SsaStatement, Vec<String>)> {
    let caller = unit.function(&id.function)?;
    let block = caller.ssa.blocks.get(&id.block)?;
    let statement = block.statements.get(id.statement_index as usize)?;
    let Statement::Call { args, .. } = &statement.statement else {
        return None;
    };
    let actuals = if let Some(argument_index) = id.nested_argument {
        let argument = args.get(argument_index as usize)?;
        crate::value_shapes::parse_command_substitution(argument)?.1
    } else {
        args.clone()
    };
    Some((caller, statement, actuals))
}

fn join_intervals(left: Interval, right: Interval) -> Interval {
    let lo = match (left.lo, right.lo) {
        (Some(left), Some(right)) => Some(left.min(right)),
        _ => None,
    };
    let hi = match (left.hi, right.hi) {
        (Some(left), Some(right)) => Some(left.max(right)),
        _ => None,
    };
    Interval { lo, hi }
}

fn collect_candidate_declines(
    function: &str,
    function_unit: &FunctionUnit,
    reason: NativeIntegerDeclineReason,
) -> Vec<NativeAddDecision> {
    let mut decisions = Vec::new();
    for block in function_unit.cfg.reverse_postorder() {
        let Some(ssa_block) = function_unit.ssa.blocks.get(&block) else {
            continue;
        };
        for (statement_index, statement) in ssa_block.statements.iter().enumerate() {
            let (expr, result) = match &statement.statement {
                Statement::AssignExpr { name, expr, .. } => {
                    let Some(symbol) = function_unit.ssa.var_symbol(name) else {
                        continue;
                    };
                    let Some(version) = statement.defs.get(&symbol) else {
                        continue;
                    };
                    (expr, NativeAddResult::Ssa((symbol, *version)))
                }
                Statement::Return {
                    expr: Some(expr), ..
                } => (expr, NativeAddResult::FunctionReturn),
                _ => continue,
            };
            if !matches!(
                expr,
                ExprNode::Binary {
                    op: BinOp::Add,
                    left,
                    right,
                } if matches!(&**left, ExprNode::Var { .. })
                    && matches!(&**right, ExprNode::Var { .. })
            ) {
                continue;
            }
            decisions.push(NativeAddDecision::Declined {
                site: NativeAddSite {
                    function: function.to_owned(),
                    block,
                    statement_index: Some(statement_index),
                    result,
                },
                reason,
            });
        }
        let Some(cfg_block) = function_unit.cfg.blocks.get(&block) else {
            continue;
        };
        if matches!(
            &cfg_block.terminator,
            Some(crate::cfg::Terminator::Return {
                expr: Some(ExprNode::Binary {
                    op: BinOp::Add,
                    left,
                    right,
                }),
                ..
            }) if matches!(&**left, ExprNode::Var { .. })
                && matches!(&**right, ExprNode::Var { .. })
        ) {
            decisions.push(NativeAddDecision::Declined {
                site: NativeAddSite {
                    function: function.to_owned(),
                    block,
                    statement_index: None,
                    result: NativeAddResult::FunctionReturn,
                },
                reason,
            });
        }
    }
    decisions
}

fn prove_add(
    context: &ProofContext<'_, '_>,
    site: NativeAddSite,
    statement: Option<&SsaStatement>,
    left_name: &str,
    right_name: &str,
) -> NativeAddDecision {
    let result = (|| {
        require_result_def_use(context, &site)?;
        let left = prove_operand(context, statement, left_name)?;
        let right = prove_operand(context, statement, right_name)?;
        let (sum_min, sum_max) = add_bounds(left.range, right.range)
            .ok_or(NativeIntegerDeclineReason::UnboundedOperand)?;
        let execution = if bounds_fit(sum_min, sum_max, context.policy.width) {
            NativeAddExecution::OverflowImpossible
        } else if context.policy.overflow == NativeOverflowPolicy::CheckedBoxedFallback {
            NativeAddExecution::CheckedWithBoxedFallback
        } else {
            return Err(NativeIntegerDeclineReason::OverflowNotExcluded);
        };
        Ok(NativeAddEvidence {
            site: site.clone(),
            left,
            right,
            result_interval: interval_from_i128(sum_min, sum_max),
            execution,
            composition: NativeAddComposition {
                direct_calls: context.caller.call_sites.clone(),
                dispatch_dependencies: context.caller.dispatch_dependencies,
                requires_frame_plan: true,
                requires_internal_operation_guard_or_sealed_policy: true,
            },
        })
    })();

    match result {
        Ok(evidence) => NativeAddDecision::Proven(Box::new(evidence)),
        Err(reason) => NativeAddDecision::Declined { site, reason },
    }
}

fn require_result_def_use(
    context: &ProofContext<'_, '_>,
    site: &NativeAddSite,
) -> Result<(), NativeIntegerDeclineReason> {
    let NativeAddResult::Ssa(result) = site.result else {
        return Ok(());
    };
    let name = context.function_unit.ssa.var_name(result.0);
    context
        .function_unit
        .def_use
        .chain_for(name, result.1)
        .map(|_| ())
        .ok_or(NativeIntegerDeclineReason::MissingDefUseEvidence)
}

fn prove_operand(
    context: &ProofContext<'_, '_>,
    statement: Option<&SsaStatement>,
    name: &str,
) -> Result<NativeValueEvidence, NativeIntegerDeclineReason> {
    if context.unit.ir_module.has_dynamic_variable_trace {
        return Err(NativeIntegerDeclineReason::DynamicVariableTrace);
    }
    if context.unit.ir_module.traced_variables.contains(name)
        || context.observability.is_escaping_at(
            context.block,
            context.statement_index.unwrap_or_else(|| {
                context
                    .function_unit
                    .ssa
                    .blocks
                    .get(&context.block)
                    .map_or(0, |block| block.statements.len())
            }),
            name,
        )
    {
        return Err(NativeIntegerDeclineReason::ObservableVariable);
    }

    let symbol = context
        .function_unit
        .ssa
        .var_symbol(name)
        .ok_or(NativeIntegerDeclineReason::MissingDefUseEvidence)?;
    let version = match statement {
        Some(statement) => statement.uses.get(&symbol).copied(),
        None => context
            .function_unit
            .ssa
            .blocks
            .get(&context.block)
            .map(|block| block.exit_versions.get(&symbol).copied().unwrap_or(0)),
    }
    .ok_or(NativeIntegerDeclineReason::MissingDefUseEvidence)?;
    let value = (symbol, version);
    require_operand_use(context, name, version)?;

    let type_lattice = context
        .function_unit
        .types
        .get(&value)
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    let is_parameter = version == 0 && context.params.iter().any(|param| param == name);
    let (range, source) = if is_parameter {
        let range = context
            .caller
            .ranges
            .get(name)
            .copied()
            .ok_or(NativeIntegerDeclineReason::DynamicCallerInput)?;
        (range, NativeRangeSource::BindingSafeDirectCaller)
    } else if let Some(range) = range_from_sccp(context.function_unit.sccp.values.get(&value)) {
        (range?, NativeRangeSource::Sccp)
    } else {
        ensure_integral_type(&type_lattice)?;
        let range = context
            .intervals
            .get(&value)
            .copied()
            .filter(|range| bounded(*range))
            .ok_or(NativeIntegerDeclineReason::UnboundedOperand)?;
        (range, NativeRangeSource::SsaInterval)
    };

    let (Some(min), Some(max)) = (range.lo, range.hi) else {
        return Err(NativeIntegerDeclineReason::UnboundedOperand);
    };
    if !bounds_fit(i128::from(min), i128::from(max), context.policy.width) {
        return Err(NativeIntegerDeclineReason::OperandOutsideNativeWidth);
    }
    Ok(NativeValueEvidence {
        value,
        range,
        source,
        type_lattice,
    })
}

fn require_operand_use(
    context: &ProofContext<'_, '_>,
    name: &str,
    version: u32,
) -> Result<(), NativeIntegerDeclineReason> {
    let block_name = context.function_unit.ssa.block_name(context.block);
    let statement_index = match context.statement_index {
        Some(index) => {
            i32::try_from(index).map_err(|_| NativeIntegerDeclineReason::MissingDefUseEvidence)?
        }
        None => -1,
    };
    let chain = context
        .function_unit
        .def_use
        .chain_for(name, version)
        .ok_or(NativeIntegerDeclineReason::MissingDefUseEvidence)?;
    chain
        .uses
        .iter()
        .any(|use_site| use_site.block == block_name && use_site.statement_index == statement_index)
        .then_some(())
        .ok_or(NativeIntegerDeclineReason::MissingDefUseEvidence)
}

fn range_from_sccp(
    value: Option<&LatticeValue>,
) -> Option<Result<Interval, NativeIntegerDeclineReason>> {
    match value? {
        LatticeValue::Const(value) => Some(const_range(value)),
        LatticeValue::ConstSet(values) if !values.is_empty() => {
            let mut min = i64::MAX;
            let mut max = i64::MIN;
            for value in values {
                let range = match const_range(value) {
                    Ok(range) => range,
                    Err(reason) => return Some(Err(reason)),
                };
                min = min.min(range.lo.expect("constant SCCP range is bounded"));
                max = max.max(range.hi.expect("constant SCCP range is bounded"));
            }
            Some(Ok(Interval {
                lo: Some(min),
                hi: Some(max),
            }))
        }
        LatticeValue::Unknown | LatticeValue::ConstSet(_) | LatticeValue::Overdefined => None,
    }
}

fn const_range(value: &ConstValue) -> Result<Interval, NativeIntegerDeclineReason> {
    match value {
        ConstValue::Int(value) => Ok(crate::intervals::constant(*value)),
        ConstValue::Bool(value) => Ok(crate::intervals::constant(i64::from(*value))),
        ConstValue::Float(_) | ConstValue::String(_) => {
            Err(NativeIntegerDeclineReason::NonIntegerOperand)
        }
    }
}

fn bounded(range: Interval) -> bool {
    matches!((range.lo, range.hi), (Some(lo), Some(hi)) if lo <= hi)
}

fn add_bounds(left: Interval, right: Interval) -> Option<(i128, i128)> {
    let (Some(left_min), Some(left_max), Some(right_min), Some(right_max)) =
        (left.lo, left.hi, right.lo, right.hi)
    else {
        return None;
    };
    Some((
        i128::from(left_min) + i128::from(right_min),
        i128::from(left_max) + i128::from(right_max),
    ))
}

fn bounds_fit(min: i128, max: i128, width: NativeIntegerWidth) -> bool {
    let (width_min, width_max) = width.bounds();
    min >= width_min && max <= width_max
}

fn interval_from_i128(min: i128, max: i128) -> Option<Interval> {
    Some(Interval {
        lo: Some(i64::try_from(min).ok()?),
        hi: Some(i64::try_from(max).ok()?),
    })
}

fn ensure_integral_type(lattice: &TypeLattice) -> Result<(), NativeIntegerDeclineReason> {
    if lattice.shapes().is_empty() {
        return Err(NativeIntegerDeclineReason::NonIntegralType);
    }
    if lattice
        .shapes()
        .iter()
        .any(|shape| matches!(shape, TypeShape::Bignum))
    {
        return Err(NativeIntegerDeclineReason::BignumOperand);
    }
    lattice
        .shapes()
        .iter()
        .all(|shape| matches!(shape, TypeShape::Int | TypeShape::Boolean))
        .then_some(())
        .ok_or(NativeIntegerDeclineReason::NonIntegralType)
}

fn parse_caller_integer(
    literal: &str,
    registry: &CommandRegistry,
) -> Result<i64, NativeIntegerDeclineReason> {
    match parse_integer_operand_with_policy(literal, FoldPolicy::from_registry(registry)) {
        Some(TclValue::Int(value)) => Ok(value),
        Some(TclValue::Big(_)) => Err(NativeIntegerDeclineReason::BignumOperand),
        Some(TclValue::Float(_)) | None => Err(NativeIntegerDeclineReason::NonIntegerOperand),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_aot_plan::CommonAotEnvironment;
    use tcl_registry::dialects::DialectSet;

    fn prove(
        source: &str,
        dialect: &str,
        optimisations: SemanticOptimisationConfig,
        policy: NativeIntegerPolicy,
    ) -> NativeIntegerProof {
        let registry = tcl_registry::registry_for_dialect(dialect);
        let unit = CompilationUnit::build_for_dialect(source, registry, false, dialect);
        let plan = CommonAotProofPlan::build(
            &unit,
            registry,
            DialectSet::parse(dialect).expect("test dialect"),
            optimisations,
            CommonAotEnvironment::Hosted,
        );
        prove_native_integer_adds(&unit, "::add", registry, optimisations, policy, &plan)
    }

    fn checked_i64() -> NativeIntegerPolicy {
        NativeIntegerPolicy::new(
            NativeIntegerWidth::I64,
            NativeOverflowPolicy::CheckedBoxedFallback,
        )
    }

    fn enabled() -> SemanticOptimisationConfig {
        SemanticOptimisationConfig::new()
            .with_enabled(SemanticOptimisationPassId::DirectProc)
            .with_enabled(SemanticOptimisationPassId::NativeInteger)
    }

    fn one_decision(proof: NativeIntegerProof) -> NativeAddDecision {
        let NativeIntegerProof::Analysed(mut decisions) = proof else {
            panic!("expected analysed proof, got {proof:?}");
        };
        assert_eq!(decisions.len(), 1);
        decisions.remove(0)
    }

    const ADD_BODY: &str = "proc add {b c} { return [expr {$b + $c}] }\n";

    #[test]
    fn proof_is_default_off() {
        let proof = prove(
            &format!("{ADD_BODY}set d 20\nset e 22\nadd $d $e\n"),
            "tcl9.0",
            SemanticOptimisationConfig::new().with_enabled(SemanticOptimisationPassId::DirectProc),
            NativeIntegerPolicy::default(),
        );
        assert_eq!(proof, NativeIntegerProof::Disabled);
    }

    #[test]
    fn closed_world_add_proves_ssa_ranges_and_no_overflow() {
        let decision = one_decision(prove(
            &format!("{ADD_BODY}set d 2\nset e 4\nputs [add $d $e]\n"),
            "tcl9.0",
            enabled(),
            checked_i64(),
        ));
        let NativeAddDecision::Proven(evidence) = decision else {
            panic!("expected proof, got {decision:?}");
        };
        assert_eq!(evidence.left.range, crate::intervals::constant(2));
        assert_eq!(evidence.right.range, crate::intervals::constant(4));
        assert_eq!(
            evidence.result_interval,
            Some(crate::intervals::constant(6))
        );
        assert_eq!(evidence.execution, NativeAddExecution::OverflowImpossible);
        assert_eq!(
            evidence.left.source,
            NativeRangeSource::BindingSafeDirectCaller
        );
        assert_eq!(evidence.site.result, NativeAddResult::FunctionReturn);
        assert!(!evidence.composition.direct_calls.is_empty());
        assert!(evidence.composition.requires_frame_plan);
        assert!(
            evidence
                .composition
                .requires_internal_operation_guard_or_sealed_policy
        );
    }

    #[test]
    fn native_integer_does_not_bypass_disabled_direct_proc() {
        let optimisations = SemanticOptimisationConfig::new()
            .with_enabled(SemanticOptimisationPassId::NativeInteger);
        let decision = one_decision(prove(
            &format!("{ADD_BODY}set d 2\nset e 4\nputs [add $d $e]\n"),
            "tcl9.0",
            optimisations,
            checked_i64(),
        ));
        assert!(matches!(
            decision,
            NativeAddDecision::Declined {
                reason: NativeIntegerDeclineReason::MissingDirectProcEvidence,
                ..
            }
        ));
    }

    #[test]
    fn multiple_direct_callers_join_ranges_conservatively() {
        let source =
            format!("{ADD_BODY}set d 1\nset e 2\nadd $d $e\nset d 3\nset e 4\nadd $d $e\n");
        let decision = one_decision(prove(&source, "tcl9.0", enabled(), checked_i64()));
        let NativeAddDecision::Proven(evidence) = decision else {
            panic!("expected joined proof, got {decision:?}");
        };
        assert_eq!(
            evidence.left.range,
            Interval {
                lo: Some(1),
                hi: Some(3)
            }
        );
        assert_eq!(
            evidence.right.range,
            Interval {
                lo: Some(2),
                hi: Some(4)
            }
        );
        assert_eq!(
            evidence.result_interval,
            Some(Interval {
                lo: Some(3),
                hi: Some(7)
            })
        );
    }

    #[test]
    fn a_declined_caller_refuses_the_selected_sites_as_the_whole_caller_set() {
        // The call inside `::risky` reaches `::add` with operands no selected
        // site carries, but its candidate declines (`ExceptionalControlFlow`)
        // and the arity gate cannot see it: it supplies the same two arguments
        // the selected site does.  Joining only `add $d $e` would publish a
        // [2,2]/[4,4] operand range the procedure does not actually have.
        let source = format!(
            "{ADD_BODY}proc risky {{}} {{ try {{ add 100 200 }} on error {{}} {{}} }}\n\
             set d 2\nset e 4\nputs [add $d $e]\n"
        );
        let decision = one_decision(prove(&source, "tcl9.0", enabled(), checked_i64()));
        assert!(
            matches!(
                decision,
                NativeAddDecision::Declined {
                    reason: NativeIntegerDeclineReason::MissingDirectProcEvidence,
                    ..
                }
            ),
            "an unselected caller must refuse the proof, got {decision:?}"
        );
    }

    #[test]
    fn wide_overflow_requires_explicit_checked_boxed_fallback() {
        let source = format!("{ADD_BODY}add 9223372036854775807 1\n");
        let strict = one_decision(prove(
            &source,
            "tcl9.0",
            enabled(),
            NativeIntegerPolicy::new(
                NativeIntegerWidth::I64,
                NativeOverflowPolicy::ProveImpossible,
            ),
        ));
        assert!(matches!(
            strict,
            NativeAddDecision::Declined {
                reason: NativeIntegerDeclineReason::OverflowNotExcluded,
                ..
            }
        ));

        let checked = one_decision(prove(&source, "tcl9.0", enabled(), checked_i64()));
        let NativeAddDecision::Proven(evidence) = checked else {
            panic!("expected checked proof, got {checked:?}");
        };
        assert_eq!(evidence.result_interval, None);
        assert_eq!(
            evidence.execution,
            NativeAddExecution::CheckedWithBoxedFallback
        );
    }

    #[test]
    fn bignum_operand_never_enters_native_path() {
        let decision = one_decision(prove(
            &format!("{ADD_BODY}add 9223372036854775808 1\n"),
            "tcl9.0",
            enabled(),
            checked_i64(),
        ));
        assert!(matches!(
            decision,
            NativeAddDecision::Declined {
                reason: NativeIntegerDeclineReason::BignumOperand,
                ..
            }
        ));
    }

    #[test]
    fn dynamic_caller_declines_parameter_range() {
        let decision = one_decision(prove(
            &format!("{ADD_BODY}set target add\n$target 20 22\n"),
            "tcl9.0",
            enabled(),
            checked_i64(),
        ));
        assert!(matches!(
            decision,
            NativeAddDecision::Declined {
                reason: NativeIntegerDeclineReason::DynamicCallerInput,
                ..
            }
        ));
    }

    #[test]
    fn variable_trace_declines_even_with_literal_callers() {
        let source = "proc cb args {}\n\
                      proc add {a b} { trace add variable a read cb; \
                      set result [expr {$a + $b}]; return $result }\n\
                      add 20 22\n";
        let decision = one_decision(prove(source, "tcl9.0", enabled(), checked_i64()));
        assert!(matches!(
            decision,
            NativeAddDecision::Declined {
                reason: NativeIntegerDeclineReason::ObservableVariable,
                ..
            }
        ));
    }

    #[test]
    fn caller_literals_follow_the_registry_dialect() {
        let source = format!("{ADD_BODY}add 010 1\n");
        let tcl8 = one_decision(prove(&source, "tcl8.6", enabled(), checked_i64()));
        let tcl9 = one_decision(prove(&source, "tcl9.0", enabled(), checked_i64()));
        let NativeAddDecision::Proven(tcl8) = tcl8 else {
            panic!("expected Tcl 8 proof, got {tcl8:?}");
        };
        let NativeAddDecision::Proven(tcl9) = tcl9 else {
            panic!("expected Tcl 9 proof, got {tcl9:?}");
        };
        assert_eq!(tcl8.result_interval, Some(crate::intervals::constant(9)));
        assert_eq!(tcl9.result_interval, Some(crate::intervals::constant(11)));
    }
}
