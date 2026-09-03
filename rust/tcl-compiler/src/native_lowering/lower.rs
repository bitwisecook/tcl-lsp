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

//! Lowering from the executable semantic IR to NLIR.
//!
//! Every executable block becomes one NLIR block and every instruction one
//! statement. Statements are selected by registry descriptor — the
//! [`LoweringHookId`] an already-lowered operation carries, or the
//! [`NativeLowering`] shape of a resolved invocation — never by command name.
//! A dispatch-stability proof at the site decides whether an intrinsic or a
//! fixed completion may be taken directly; otherwise the generic argv
//! invocation is kept. Anything the lowering cannot express structurally
//! becomes the source-text rung with a typed reason.

use std::collections::{BTreeMap, HashMap, HashSet};

use tcl_core_types::Code as CompletionCode;
use tcl_dialect::DialectProfile;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{CellUpdate, CommandRegistry, IntrinsicId, NativeLowering};
use tcl_syntax::expr::ast::render_expr;
use tcl_syntax::expr::{BinOp, ExprNode, UnaryOp};
use tcl_syntax::number::{Number, Numbers};

use super::cells::{CellPlace, ShadowState};
use super::elide::{BarrierDecision, CellDemotion, TraceLedger};
use super::ir::{
    CmpOp, CompareKind, IfElseResult, IntOp, NativeBlock, NativeBlockId, NativeFunction, NativeOp,
    NativeStatement, NativeTerminator, NativeType, NativeValue, NativeValueId,
};
use super::representation::{
    Representation, cmp_op, double_op, double_result_defined, exactly_representable_as_double,
    int_op, numeric_hint, proven_int_result, proven_neg_result,
};
use super::{
    CellAccessKind, CellAccessRecord, FunctionDecline, FunctionReport, FunctionStatus,
    NativeLoweringDecline, StatementOutcome, StatementRecord,
};
use crate::codegen::structured::command_text;
use crate::command_binding::{ModuleCommandMutations, scan_module_command_mutations};
use crate::dispatch_proof::{
    DispatchEntryAssumption, DispatchProofAnalysis, analyse_dispatch_stability,
};
use crate::executable_ir::{
    CellReference, ExecutableArgvId, ExecutableBlock, ExecutableExpr, ExecutableFunction,
    ExecutableInstruction, ExecutableTerminator, ExecutableValueId, GenericInvoke,
    InvocationResolution, LoweredOperation,
};
use crate::intervals::Interval;
use crate::ir::{
    CommandTokens, Module, NodeId, Provenance, SourceSite, Statement, WordExpr, WordPart,
};
use crate::registry_invocation::{RegistryInvocationResolution, resolve_word_exprs};
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::types::TypeShape;
use crate::var_escape::types::ProcEscapeSummary;
use tcl_lexer::Span;

/// Recursion cap for nested command substitution and compound-word parts.
const MAX_WORD_DEPTH: u32 = 32;

/// Everything the lowering of one function needs.
pub struct LoweringInput<'a> {
    /// The command registry.
    pub registry: &'a CommandRegistry,
    /// The semantic context invocations were resolved under.
    pub context: Option<SemanticContext>,
    /// The executable function to lower.
    pub function: &'a ExecutableFunction,
    /// The unit's source text (for the source-text rung only).
    pub source: &'a str,
    /// The lowered module: trace targets, procedures, and command mutations.
    pub module: &'a Module,
    /// The enabled semantic optimisation passes.
    pub config: SemanticOptimisationConfig,
    /// The escape summary of a procedure body, when lowering one.
    pub escape: Option<&'a ProcEscapeSummary>,
    /// Whether this is the top-level script.
    pub top_level: bool,
    /// The dispatch entry contract the world proofs are made under.
    pub entry_assumption: DispatchEntryAssumption,
    /// Front-end type shapes per variable name, joined over every SSA
    /// version, used only as fast-path hints.
    pub type_hints: &'a BTreeMap<String, TypeShape>,
}

/// Why an expression could not be lowered natively; the whole expression
/// then goes to the runtime expression intrinsic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExprDecline {
    CommandSubstitution,
    SubstitutedString,
    DynamicVariable,
    UnsupportedOperator,
    Raw,
}

/// Lower one executable function to NLIR.
pub fn lower_function(
    input: &LoweringInput<'_>,
) -> Result<(NativeFunction, FunctionReport), FunctionDecline> {
    if !input
        .config
        .is_enabled(SemanticOptimisationPassId::NativeLowering)
    {
        return Err(FunctionDecline::PassDisabled);
    }
    if input.function.validate().is_err() {
        return Err(FunctionDecline::InvalidExecutableIr);
    }
    if let Some(kind) = unlowered_instruction(input.function) {
        return Err(FunctionDecline::UnloweredInstruction(kind));
    }
    let mut lowerer = Lowerer::new(input);
    let function = lowerer.lower();
    let report = FunctionReport {
        status: FunctionStatus::Lowered,
        statements: lowerer.records,
    };
    Ok((function, report))
}

/// The first executable instruction kind this lowering does not project.
fn unlowered_instruction(function: &ExecutableFunction) -> Option<&'static str> {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match instruction {
            ExecutableInstruction::IterateLists { .. } => Some("iterate-lists"),
            ExecutableInstruction::MatchPattern { .. } => Some("match-pattern"),
            ExecutableInstruction::JoinCompletion { .. } => Some("join-completion"),
            ExecutableInstruction::WriteCompletionCell { .. } => Some("write-completion-cell"),
            ExecutableInstruction::EvaluateExpr {
                expr: ExecutableExpr::Operand { .. },
                ..
            } => Some("operand-expression"),
            ExecutableInstruction::EvaluateExpr {
                expr: ExecutableExpr::TrapPrefix { .. },
                ..
            } => Some("trap-prefix"),
            _ => None,
        })
}

struct Lowerer<'a> {
    input: &'a LoweringInput<'a>,
    values: Vec<NativeValue>,
    exec_values: HashMap<ExecutableValueId, NativeValueId>,
    argvs: HashMap<ExecutableArgvId, Vec<NativeValueId>>,
    ledger: TraceLedger<'a>,
    demotion: CellDemotion<'a>,
    mutations: ModuleCommandMutations,
    proofs: DispatchProofAnalysis,
    numbers: Numbers,
    dialect: Option<&'static DialectProfile>,
    representation: bool,
    mathfunc_native: bool,
    declined_nodes: BTreeMap<NodeId, NativeLoweringDecline>,
    /// Nodes whose operand words dispatched a runtime command (a nested
    /// invocation, a math function through the command table, a runtime
    /// expression) or read a cell whose trace barrier was kept, so the
    /// site's own operand-admissibility verdict must stand.
    observing_nodes: HashSet<NodeId>,
    current_node: Option<NodeId>,
    shadows: ShadowState,
    exit_shadows: Vec<Option<ShadowState>>,
    records: Vec<StatementRecord>,
    current: Option<StatementRecord>,
    ops: Vec<Vec<NativeOp>>,
    max_argc: usize,
}

impl<'a> Lowerer<'a> {
    fn new(input: &'a LoweringInput<'a>) -> Self {
        let module = input.module;
        let ledger = TraceLedger::new(
            &module.traced_variables,
            module.has_dynamic_variable_trace,
            input.config,
        );
        let demotion = if input.top_level {
            CellDemotion::top_level(input.config)
        } else {
            CellDemotion::procedure(input.escape, input.config)
        };
        let environment = input.context.map(SemanticContext::environment_id);
        // `expr` resolves `abs(…)` through the command table, so a native
        // math function is only sound while nothing in the module can have
        // replaced one: no dynamic trace, no proc declaring one, and no
        // `rename` / `interp alias` touching the namespace (a dynamic
        // mutation could touch anything).
        let mutations = scan_module_command_mutations(module, input.registry);
        let mathfunc_native = !module.has_dynamic_trace
            && !mutations.has_dynamic_mutation()
            && !mutations.changes_command_resolution()
            && !mutations.rebinds_under("::tcl::mathfunc::")
            && !module
                .procedures
                .keys()
                .any(|name| name.starts_with("::tcl::mathfunc::"));
        Self {
            input,
            values: Vec::new(),
            exec_values: HashMap::new(),
            argvs: HashMap::new(),
            ledger,
            demotion,
            mutations,
            proofs: analyse_dispatch_stability(input.function, input.entry_assumption),
            numbers: Numbers::of_dialect_name(environment),
            dialect: environment.and_then(DialectProfile::find),
            representation: input
                .config
                .is_enabled(SemanticOptimisationPassId::RepresentationInference),
            mathfunc_native,
            declined_nodes: BTreeMap::new(),
            observing_nodes: HashSet::new(),
            current_node: None,
            shadows: ShadowState::default(),
            exit_shadows: vec![None; input.function.blocks.len()],
            records: Vec::new(),
            current: None,
            ops: Vec::new(),
            max_argc: 0,
        }
    }

    // -- values and operations ------------------------------------------------

    fn new_value(&mut self, ty: NativeType, rep: Representation) -> NativeValueId {
        let id = NativeValueId(u32::try_from(self.values.len()).expect("value count fits u32"));
        self.values.push(NativeValue { ty, rep });
        if let Some(record) = &mut self.current {
            record
                .representations
                .push(self.values[id.0 as usize].rep.kind_str());
        }
        id
    }

    fn ty(&self, id: NativeValueId) -> NativeType {
        self.values[id.0 as usize].ty
    }

    fn rep(&self, id: NativeValueId) -> &Representation {
        &self.values[id.0 as usize].rep
    }

    fn emit(&mut self, op: NativeOp) {
        self.ops
            .last_mut()
            .expect("an open operation buffer")
            .push(op);
    }

    fn push_buffer(&mut self) {
        self.ops.push(Vec::new());
    }

    fn pop_buffer(&mut self) -> Vec<NativeOp> {
        self.ops.pop().expect("a buffer to pop")
    }

    /// Run `f` in a fresh buffer; keep its operations only when it succeeds.
    fn attempt<T, E>(&mut self, f: impl FnOnce(&mut Self) -> Result<T, E>) -> Result<T, E> {
        self.push_buffer();
        let outcome = f(self);
        let ops = self.pop_buffer();
        if outcome.is_ok() {
            self.ops
                .last_mut()
                .expect("an enclosing buffer")
                .extend(ops);
        }
        outcome
    }

    fn const_int(&mut self, value: i64) -> NativeValueId {
        let dst = self.new_value(NativeType::I64, Representation::exact_int(value));
        self.emit(NativeOp::ConstInt { dst, value });
        dst
    }

    fn const_str(&mut self, text: &str) -> NativeValueId {
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        self.emit(NativeOp::ConstStr {
            dst,
            text: text.to_owned(),
        });
        dst
    }

    /// The boxed form of `value`.
    fn boxed(&mut self, value: NativeValueId) -> NativeValueId {
        if self.ty(value) == NativeType::Obj {
            return value;
        }
        let shape = match self.rep(value) {
            Representation::NativeInt(_) => Some(TypeShape::Int),
            Representation::NativeDouble { .. } => Some(TypeShape::Double),
            Representation::NativeBool => Some(TypeShape::Boolean),
            _ => None,
        };
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(shape));
        self.emit(NativeOp::Box { dst, src: value });
        dst
    }

    /// The truth of `value`, through the erroring boolean conversion for
    /// anything a native truth test could get wrong.
    fn truth(&mut self, value: NativeValueId) -> NativeValueId {
        match self.ty(value) {
            NativeType::Bool => value,
            NativeType::I64 => {
                let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                self.emit(NativeOp::Truth { dst, src: value });
                dst
            }
            NativeType::F64
                if matches!(
                    self.rep(value),
                    Representation::NativeDouble { finite: true }
                ) =>
            {
                let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                self.emit(NativeOp::Truth { dst, src: value });
                dst
            }
            NativeType::F64 | NativeType::Obj => {
                let src = self.boxed(value);
                let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                self.emit(NativeOp::Unbox {
                    dst,
                    src,
                    target: NativeType::Bool,
                });
                dst
            }
        }
    }

    fn int_to_double(&mut self, value: NativeValueId) -> NativeValueId {
        let dst = self.new_value(
            NativeType::F64,
            Representation::NativeDouble { finite: true },
        );
        self.emit(NativeOp::IntToDouble { dst, src: value });
        dst
    }

    fn bool_to_int(&mut self, value: NativeValueId) -> NativeValueId {
        let dst = self.new_value(
            NativeType::I64,
            Representation::NativeInt(Interval {
                lo: Some(0),
                hi: Some(1),
            }),
        );
        self.emit(NativeOp::BoolToInt { dst, src: value });
        dst
    }

    // -- cells ----------------------------------------------------------------

    fn record_cell(
        &mut self,
        place: &CellPlace,
        access: CellAccessKind,
        barrier: BarrierDecision,
        shadowed: bool,
    ) {
        let storage = self.demotion.decide(place.base());
        if let Some(record) = &mut self.current {
            record.cells.push(CellAccessRecord {
                place: place.spelling(),
                access,
                storage,
                barrier,
                shadowed,
            });
        }
    }

    /// Read `place`, reusing its native shadow when one is live.
    fn read_cell(&mut self, place: &CellPlace) -> NativeValueId {
        if let Some(value) = self.shadows.read(place) {
            let barrier = self.ledger.decide(place);
            self.record_cell(place, CellAccessKind::Read, barrier, true);
            return value;
        }
        let hint = self.input.type_hints.get(place.base()).cloned();
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(hint));
        let barrier = self.ledger.decide(place);
        self.emit(NativeOp::CellRead {
            dst,
            place: place.clone(),
            barrier,
        });
        if barrier.is_elided() {
            self.shadows.write(place.clone(), dst);
        } else {
            self.note_observation();
        }
        self.record_cell(place, CellAccessKind::Read, barrier, false);
        dst
    }

    /// Write `boxed` into `place`; `shadow` is the value later reads may
    /// reuse (the native form of `boxed`, when there is one).
    fn write_cell(&mut self, place: &CellPlace, boxed: NativeValueId, shadow: NativeValueId) {
        let barrier = self.ledger.decide(place);
        self.emit(NativeOp::CellWrite {
            place: place.clone(),
            src: boxed,
            barrier,
        });
        if barrier.is_elided() {
            self.shadows.write(place.clone(), shadow);
        } else {
            self.shadows.forget_base(place.base());
        }
        self.record_cell(place, CellAccessKind::Write, barrier, false);
    }

    fn clobber_shadows(&mut self) {
        self.shadows.clobber();
    }

    /// Record that the statement's operand words observed the world through
    /// a runtime dispatch or an unproven cell read.
    fn note_observation(&mut self) {
        if let Some(node) = &self.current_node {
            self.observing_nodes.insert(node.clone());
        }
    }

    // -- driver ---------------------------------------------------------------

    fn lower(&mut self) -> NativeFunction {
        let function = self.input.function;
        self.prescan();
        let (order, headers, predecessors) = block_order(function);
        let mut blocks: Vec<Option<NativeBlock>> = vec![None; function.blocks.len()];
        for index in order {
            let block = &function.blocks[index];
            self.shadows = match predecessors[index].as_slice() {
                [only] if !headers.contains(&index) => {
                    self.exit_shadows[*only].clone().unwrap_or_default()
                }
                _ => ShadowState::default(),
            };
            let mut statements = Vec::with_capacity(block.instructions.len());
            for (position, instruction) in block.instructions.iter().enumerate() {
                statements.push(self.lower_instruction(block, index, position, instruction));
            }
            let terminator = self.lower_terminator(block);
            self.exit_shadows[index] = Some(std::mem::take(&mut self.shadows));
            blocks[index] = Some(NativeBlock {
                id: NativeBlockId(u32::try_from(index).unwrap_or(u32::MAX)),
                statements,
                terminator,
            });
        }
        let blocks = blocks
            .into_iter()
            .enumerate()
            .map(|(index, block)| {
                block.unwrap_or_else(|| NativeBlock {
                    id: NativeBlockId(u32::try_from(index).unwrap_or(u32::MAX)),
                    statements: Vec::new(),
                    terminator: NativeTerminator::Return(unreachable_completion(function)),
                })
            })
            .collect();
        NativeFunction {
            values: std::mem::take(&mut self.values),
            blocks,
            entry: NativeBlockId(u32::try_from(function.entry.index()).unwrap_or(u32::MAX)),
            completion_count: completion_count(function),
            max_argc: self.max_argc,
            pushes_frame: !self.input.top_level,
        }
    }

    /// Decide, before any operation is emitted, which invocation nodes must
    /// keep the source-text rung because one of their words cannot be
    /// evaluated structurally.
    fn prescan(&mut self) {
        for block in &self.input.function.blocks {
            for instruction in &block.instructions {
                match instruction {
                    ExecutableInstruction::Invoke(invoke) => {
                        if let Err(reason) = self.words_lowerable(&invoke.original_words, 0) {
                            self.declined_nodes.insert(invoke.node.clone(), reason);
                        }
                    }
                    ExecutableInstruction::ExpandWord { node, .. } => {
                        self.declined_nodes
                            .insert(node.clone(), NativeLoweringDecline::ArgumentExpansion);
                    }
                    _ => {}
                }
            }
        }
    }

    fn lower_terminator(&mut self, block: &ExecutableBlock) -> NativeTerminator {
        let target = |id: crate::executable_ir::ExecutableBlockId| {
            NativeBlockId(u32::try_from(id.index()).unwrap_or(u32::MAX))
        };
        match block
            .terminator
            .as_ref()
            .expect("a validated executable block is terminated")
        {
            ExecutableTerminator::Goto(next) => NativeTerminator::Goto(target(*next)),
            ExecutableTerminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let value = self.exec_values[condition];
                let condition = self.truth(value);
                NativeTerminator::Branch {
                    condition,
                    then_target: target(*then_target),
                    else_target: target(*else_target),
                }
            }
            ExecutableTerminator::CompletionSwitch {
                completion,
                cases,
                default,
            } => NativeTerminator::CompletionSwitch {
                completion: *completion,
                cases: cases
                    .iter()
                    .map(|case| {
                        (
                            i32::try_from(case.code.as_int()).unwrap_or(i32::MAX),
                            target(case.target),
                        )
                    })
                    .collect(),
                default: target(*default),
            },
            ExecutableTerminator::ReturnCompletion(completion) => {
                NativeTerminator::Return(*completion)
            }
        }
    }

    fn begin(&mut self, node: Option<&NodeId>, instruction: &'static str) {
        self.current_node = node.cloned();
        self.current = Some(StatementRecord {
            node: node.cloned(),
            instruction,
            outcome: StatementOutcome::Empty,
            cells: Vec::new(),
            representations: Vec::new(),
        });
        self.push_buffer();
    }

    fn finish(
        &mut self,
        completion: crate::executable_ir::CompletionId,
        node: Option<&NodeId>,
        outcome: StatementOutcome,
    ) -> NativeStatement {
        let ops = self.pop_buffer();
        let mut record = self.current.take().expect("an open statement record");
        record.outcome = outcome;
        self.records.push(record);
        NativeStatement {
            completion,
            node: node.cloned(),
            ops,
        }
    }

    fn eval_source(&mut self, source: &SourceSite, reason: NativeLoweringDecline) {
        let text = command_text(self.input.source, source.span).to_owned();
        self.emit(NativeOp::EvalSource { text, reason });
        self.clobber_shadows();
    }

    #[allow(clippy::too_many_lines)]
    fn lower_instruction(
        &mut self,
        block: &ExecutableBlock,
        block_index: usize,
        position: usize,
        instruction: &ExecutableInstruction,
    ) -> NativeStatement {
        match instruction {
            ExecutableInstruction::EvaluateWord {
                value,
                completion,
                word,
                node,
                ..
            } => {
                self.begin(Some(node), "evaluate-word");
                if self.declined_nodes.contains_key(node) {
                    return self.finish(*completion, Some(node), StatementOutcome::Empty);
                }
                match self.attempt(|this| this.lower_word(word, 0)) {
                    Ok(id) => {
                        self.exec_values.insert(*value, id);
                        self.finish(*completion, Some(node), StatementOutcome::Native)
                    }
                    Err(reason) => {
                        // The pre-scan admitted this word, so this cannot
                        // happen; fail closed onto the source rung anyway.
                        self.declined_nodes.insert(node.clone(), reason);
                        self.finish(*completion, Some(node), StatementOutcome::Empty)
                    }
                }
            }
            ExecutableInstruction::ExpandWord {
                completion, node, ..
            } => {
                self.begin(Some(node), "expand-word");
                self.finish(*completion, Some(node), StatementOutcome::Empty)
            }
            ExecutableInstruction::BuildArgv {
                argv,
                completion,
                entries,
            } => {
                self.begin(None, "build-argv");
                let words: Option<Vec<NativeValueId>> = entries
                    .iter()
                    .map(|entry| match entry {
                        crate::executable_ir::ArgvEntry::Value(value) => {
                            self.exec_values.get(value).copied()
                        }
                        crate::executable_ir::ArgvEntry::Expanded(_) => None,
                    })
                    .collect();
                if let Some(words) = words {
                    self.argvs.insert(*argv, words);
                }
                self.finish(*completion, None, StatementOutcome::Empty)
            }
            ExecutableInstruction::Invoke(invoke) => {
                self.begin(Some(&invoke.node), "invoke");
                let outcome = self.lower_invoke(invoke, block_index, position);
                self.finish(invoke.completion, Some(&invoke.node), outcome)
            }
            ExecutableInstruction::ExecuteLowered(operation) => {
                self.begin(Some(&operation.node), "execute-lowered");
                let outcome = self.lower_lowered(operation);
                self.finish(operation.completion, Some(&operation.node), outcome)
            }
            ExecutableInstruction::ExecuteOpaqueRegion(region) => {
                self.begin(Some(&region.node), "execute-opaque-region");
                let reason = NativeLoweringDecline::OpaqueRegion(region.descriptor);
                self.eval_source(&region.source, reason);
                self.finish(
                    region.completion,
                    Some(&region.node),
                    StatementOutcome::EvalSource(reason),
                )
            }
            ExecutableInstruction::EvaluateExpr {
                value,
                completion,
                expr,
                node,
                ..
            } => {
                self.begin(Some(node), "evaluate-expr");
                let outcome = match expr {
                    ExecutableExpr::Condition { expr, .. } => {
                        let result = self.lower_expression(expr);
                        let truth = self.truth(result);
                        self.exec_values.insert(*value, truth);
                        StatementOutcome::Native
                    }
                    ExecutableExpr::Operand { .. } | ExecutableExpr::TrapPrefix { .. } => {
                        unreachable!("the function-level pre-check declines structured operands")
                    }
                };
                self.finish(*completion, Some(node), outcome)
            }
            ExecutableInstruction::CompleteStructuredRegion(region) => {
                self.begin(Some(&region.node), "complete-structured-region");
                self.finish(
                    region.completion,
                    Some(&region.node),
                    StatementOutcome::Empty,
                )
            }
            ExecutableInstruction::MatchPattern { .. }
            | ExecutableInstruction::IterateLists { .. }
            | ExecutableInstruction::JoinCompletion { .. }
            | ExecutableInstruction::WriteCompletionCell { .. } => {
                unreachable!("the function-level pre-check declines these instructions")
            }
        }
        .with_block(block)
    }

    // -- invocations ----------------------------------------------------------

    fn lower_invoke(
        &mut self,
        invoke: &GenericInvoke,
        block_index: usize,
        position: usize,
    ) -> StatementOutcome {
        if let Some(reason) = self.declined_nodes.get(&invoke.node).copied() {
            self.eval_source(&invoke.source, reason);
            return StatementOutcome::EvalSource(reason);
        }
        let Some(argv) = self.argvs.get(&invoke.argv).cloned() else {
            let reason = NativeLoweringDecline::MissingCommandTokens;
            self.eval_source(&invoke.source, reason);
            return StatementOutcome::EvalSource(reason);
        };
        if let InvocationResolution::Resolved(facts) = &invoke.resolution
            && let Some(spec) = self.input.registry.get(&facts.canonical_command)
        {
            // The site proof's operand-admissibility verdict is conservative
            // about command substitutions and variable reads in the operand
            // words; the lowering knows exactly which of those words
            // dispatched a command or read an unproven cell, so a node whose
            // words did neither may take the proof on its domain coverage.
            let words_observed = self.observing_nodes.contains(&invoke.node);
            let proven = self
                .proofs
                .site(
                    u32::try_from(block_index).unwrap_or(u32::MAX),
                    u32::try_from(position).unwrap_or(u32::MAX),
                )
                .is_some_and(|site| {
                    (site.operand_words_admissible || !words_observed)
                        && facts
                            .dispatch_dependencies
                            .iter()
                            .all(|domain| site.covers.contains(domain))
                });
            match spec.native_lowering() {
                NativeLowering::Intrinsic {
                    id: IntrinsicId::ChannelWrite,
                    arity,
                } if proven && argv.len() == 2 && arity.accepts(1) => {
                    self.emit(NativeOp::Puts { src: argv[1] });
                    if !cells_unobserved(&facts.effects) {
                        self.clobber_shadows();
                    }
                    return StatementOutcome::NativeIntrinsic;
                }
                NativeLowering::Completion(code) if proven && argv.len() == 1 => {
                    self.emit(NativeOp::Complete { code, result: None });
                    return StatementOutcome::NativeCompletion;
                }
                NativeLowering::CellReadModifyWrite(update) if proven && argv.len() >= 2 => {
                    if let Some(outcome) = self.lower_cell_update(update, invoke, &argv) {
                        return outcome;
                    }
                }
                _ => {}
            }
        }
        self.max_argc = self.max_argc.max(argv.len());
        self.emit(NativeOp::Invoke { argv });
        self.clobber_shadows();
        StatementOutcome::GenericInvoke
    }

    /// `incr`/`append`/`lappend` reached as an invocation: the place is the
    /// first argument word, which must be a literal name.
    fn lower_cell_update(
        &mut self,
        update: CellUpdate,
        invoke: &GenericInvoke,
        argv: &[NativeValueId],
    ) -> Option<StatementOutcome> {
        let name_word = invoke.original_words.get(1)?;
        let (text, braced) = match name_word {
            WordExpr::Literal { text, .. } => (text.as_str(), false),
            WordExpr::BracedLiteral { text, .. } => (text.as_str(), true),
            _ => return None,
        };
        let place = cell_place(text, braced)?;
        match update {
            CellUpdate::Increment => {
                if argv.len() > 3 {
                    return None;
                }
                let delta = match argv.get(2) {
                    Some(value) => *value,
                    None => self.const_int(1),
                };
                self.lower_incr(&place, delta);
                Some(StatementOutcome::Native)
            }
            CellUpdate::Append | CellUpdate::ListAppend => {
                let barrier = self.ledger.decide(&place);
                self.emit(NativeOp::CellAppend {
                    place: place.clone(),
                    values: argv[2..].to_vec(),
                    list: update == CellUpdate::ListAppend,
                    barrier,
                });
                self.shadows.forget_base(place.base());
                self.record_cell(&place, CellAccessKind::Update, barrier, false);
                Some(StatementOutcome::Native)
            }
        }
    }

    /// `incr place delta`: native when the cell's shadow and the delta are
    /// proven-in-range integers, else the guarded runtime read-modify-write.
    fn lower_incr(&mut self, place: &CellPlace, delta: NativeValueId) {
        let shadow = self.shadows.read(place);
        if let (Some(current), NativeType::I64) = (shadow, self.ty(delta))
            && self.ty(current) == NativeType::I64
            && let (Some(lhs), Some(rhs)) =
                (self.rep(current).interval(), self.rep(delta).interval())
            && let Some(result) = proven_int_result(IntOp::Add, lhs, rhs)
        {
            let dst = self.new_value(NativeType::I64, Representation::NativeInt(result));
            self.emit(NativeOp::IntBinary {
                dst,
                op: IntOp::Add,
                lhs: current,
                rhs: delta,
            });
            let boxed = self.boxed(dst);
            self.write_cell(place, boxed, dst);
            return;
        }
        let barrier = self.ledger.decide(place);
        let guard = self.ledger.incr_guard(place);
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(Some(TypeShape::Int)));
        self.emit(NativeOp::CellIncr {
            dst,
            place: place.clone(),
            delta,
            guard,
            barrier,
        });
        if barrier.is_elided() {
            self.shadows.write(place.clone(), dst);
        } else {
            self.shadows.forget_base(place.base());
        }
        self.record_cell(place, CellAccessKind::Update, barrier, shadow.is_some());
    }

    // -- already-lowered operations -------------------------------------------

    #[allow(clippy::too_many_lines)]
    fn lower_lowered(&mut self, operation: &LoweredOperation) -> StatementOutcome {
        match &operation.statement {
            Statement::AssignConst {
                name,
                name_braced,
                value,
                ..
            } => {
                let Some(place) = cell_place(name, *name_braced) else {
                    return self
                        .decline(&operation.source, NativeLoweringDecline::ComputedCellName);
                };
                let boxed = self.const_str(value);
                let shadow = self.native_constant(value).unwrap_or(boxed);
                self.write_cell(&place, boxed, shadow);
                StatementOutcome::Native
            }
            Statement::AssignValue {
                name,
                name_braced,
                tokens,
                ..
            } => {
                let Some(place) = cell_place(name, *name_braced) else {
                    return self
                        .decline(&operation.source, NativeLoweringDecline::ComputedCellName);
                };
                let Some(word) = value_word(tokens.as_ref()) else {
                    return self.decline(
                        &operation.source,
                        NativeLoweringDecline::MissingCommandTokens,
                    );
                };
                let word = word.clone();
                if let WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } = &word
                    && !text.contains('\\')
                {
                    let boxed = self.const_str(text);
                    let shadow = self.native_constant(text).unwrap_or(boxed);
                    self.write_cell(&place, boxed, shadow);
                    return StatementOutcome::Native;
                }
                match self.attempt(|this| this.lower_word(&word, 0)) {
                    Ok(value) => {
                        self.write_cell(&place, value, value);
                        StatementOutcome::Native
                    }
                    Err(reason) => self.decline(&operation.source, reason),
                }
            }
            Statement::AssignExpr {
                name,
                name_braced,
                expr,
                ..
            } => {
                let Some(place) = cell_place(name, *name_braced) else {
                    return self
                        .decline(&operation.source, NativeLoweringDecline::ComputedCellName);
                };
                let value = self.lower_expression(expr);
                let boxed = self.boxed(value);
                self.write_cell(&place, boxed, value);
                StatementOutcome::Native
            }
            Statement::Incr {
                name,
                name_braced,
                amount,
                ..
            } => {
                let Some(place) = cell_place(name, *name_braced) else {
                    return self
                        .decline(&operation.source, NativeLoweringDecline::ComputedCellName);
                };
                let delta = match amount.as_deref() {
                    None => self.const_int(1),
                    Some(text) => match self.lower_operand_word(text) {
                        Some(value) => value,
                        None => {
                            return self.decline(
                                &operation.source,
                                NativeLoweringDecline::SubstitutedOperand,
                            );
                        }
                    },
                };
                self.lower_incr(&place, delta);
                StatementOutcome::Native
            }
            Statement::ExprEval { expr, .. } => {
                let value = self.lower_expression(expr);
                let boxed = self.boxed(value);
                self.emit(NativeOp::Complete {
                    code: CompletionCode::Ok,
                    result: Some(boxed),
                });
                StatementOutcome::Native
            }
            Statement::Return {
                value,
                expr,
                braced,
                ..
            } => {
                let result = match (expr, value) {
                    (Some(expr), _) => {
                        let value = self.lower_expression(expr);
                        Some(self.boxed(value))
                    }
                    (None, Some(text)) if *braced || !has_substitution(text) => {
                        Some(self.const_str(text))
                    }
                    (None, Some(_)) => {
                        return self
                            .decline(&operation.source, NativeLoweringDecline::SubstitutedOperand);
                    }
                    (None, None) => None,
                };
                self.emit(NativeOp::Complete {
                    code: CompletionCode::Return,
                    result,
                });
                StatementOutcome::Native
            }
            _ => self.decline(
                &operation.source,
                NativeLoweringDecline::OpaqueRegion(Some(operation.descriptor)),
            ),
        }
    }

    fn decline(&mut self, source: &SourceSite, reason: NativeLoweringDecline) -> StatementOutcome {
        // Drop anything the statement emitted before declining.
        if let Some(buffer) = self.ops.last_mut() {
            buffer.clear();
        }
        self.eval_source(source, reason);
        StatementOutcome::EvalSource(reason)
    }

    /// The native constant a literal value text denotes, when boxing that
    /// native value would print exactly the text again.
    fn native_constant(&mut self, text: &str) -> Option<NativeValueId> {
        if !self.representation {
            return None;
        }
        match self.numbers.parse_whole(text)? {
            Number::Int(value) if value.to_string() == text => Some(self.const_int(value)),
            Number::Double(value)
                if value.is_finite() && tcl_syntax::number::format_double(value) == text =>
            {
                let dst = self.new_value(
                    NativeType::F64,
                    Representation::NativeDouble { finite: true },
                );
                self.emit(NativeOp::ConstDouble { dst, value });
                Some(dst)
            }
            _ => None,
        }
    }

    /// A single retained operand word (an `incr` amount): a literal, or one
    /// bare scalar variable reference.
    fn lower_operand_word(&mut self, text: &str) -> Option<NativeValueId> {
        if text.starts_with('$') {
            let name = variable_name(text)?;
            if name.contains('(') {
                return None;
            }
            let place = CellPlace::Named {
                name: name.to_owned(),
            };
            let value = self.read_cell(&place);
            return Some(self.as_int_operand(value));
        }
        if has_substitution(text) {
            return None;
        }
        if let Some(Number::Int(value)) = self.numbers.parse_whole(text)
            && self.representation
        {
            return Some(self.const_int(value));
        }
        Some(self.const_str(text))
    }

    /// The `i64` form of an integer operand when it is native, else its
    /// boxed form (the runtime then owns the conversion and its error).
    fn as_int_operand(&mut self, value: NativeValueId) -> NativeValueId {
        match self.ty(value) {
            NativeType::I64 | NativeType::Obj => value,
            NativeType::Bool => self.bool_to_int(value),
            NativeType::F64 => self.boxed(value),
        }
    }

    // -- words ----------------------------------------------------------------

    fn words_lowerable(&self, words: &[WordExpr], depth: u32) -> Result<(), NativeLoweringDecline> {
        if depth > MAX_WORD_DEPTH {
            return Err(NativeLoweringDecline::WordNestingTooDeep);
        }
        if words.is_empty() {
            return Err(NativeLoweringDecline::MissingCommandTokens);
        }
        for word in words {
            self.word_lowerable(word, depth)?;
        }
        Ok(())
    }

    fn word_lowerable(&self, word: &WordExpr, depth: u32) -> Result<(), NativeLoweringDecline> {
        match word {
            WordExpr::Literal { .. } => Ok(()),
            WordExpr::BracedLiteral { text, .. } => {
                if text.contains('\\') {
                    Err(NativeLoweringDecline::BackslashSubstitution)
                } else {
                    Ok(())
                }
            }
            WordExpr::Variable { spelling, source } => variable_place(spelling, source).map(|_| ()),
            WordExpr::CommandSubstitution { spelling, source } => {
                let inner = nested_words(spelling, source)?;
                self.words_lowerable(&inner, depth + 1)
            }
            WordExpr::Template { parts, .. } => {
                if parts.is_empty() {
                    return Err(NativeLoweringDecline::OpaqueWord);
                }
                for part in parts {
                    match part {
                        WordPart::Text { text, .. } => {
                            if text.contains('\\') {
                                return Err(NativeLoweringDecline::BackslashSubstitution);
                            }
                        }
                        WordPart::Variable { spelling, source } => {
                            variable_place(spelling, source)?;
                        }
                        WordPart::CommandSubstitution { spelling, source } => {
                            let inner = nested_words(spelling, source)?;
                            self.words_lowerable(&inner, depth + 1)?;
                        }
                        WordPart::Opaque { .. } => return Err(NativeLoweringDecline::OpaqueWord),
                    }
                }
                Ok(())
            }
            WordExpr::Expand { .. } => Err(NativeLoweringDecline::ArgumentExpansion),
            WordExpr::Opaque { .. } => Err(NativeLoweringDecline::OpaqueWord),
        }
    }

    /// Lower one word to a boxed value.
    fn lower_word(
        &mut self,
        word: &WordExpr,
        depth: u32,
    ) -> Result<NativeValueId, NativeLoweringDecline> {
        if depth > MAX_WORD_DEPTH {
            return Err(NativeLoweringDecline::WordNestingTooDeep);
        }
        match word {
            WordExpr::Literal { text, .. } => Ok(self.const_str(text)),
            WordExpr::BracedLiteral { text, .. } => {
                if text.contains('\\') {
                    return Err(NativeLoweringDecline::BackslashSubstitution);
                }
                Ok(self.const_str(text))
            }
            WordExpr::Variable { spelling, source } => {
                let place = variable_place(spelling, source)?;
                let value = self.read_cell(&place);
                Ok(self.boxed(value))
            }
            WordExpr::CommandSubstitution { spelling, source } => {
                self.lower_command_substitution(spelling, source, depth)
            }
            WordExpr::Template { parts, .. } => {
                if parts.is_empty() {
                    return Err(NativeLoweringDecline::OpaqueWord);
                }
                let mut values = Vec::with_capacity(parts.len());
                for part in parts {
                    values.push(match part {
                        WordPart::Text { text, .. } => {
                            if text.contains('\\') {
                                return Err(NativeLoweringDecline::BackslashSubstitution);
                            }
                            self.const_str(text)
                        }
                        WordPart::Variable { spelling, source } => {
                            let place = variable_place(spelling, source)?;
                            let value = self.read_cell(&place);
                            self.boxed(value)
                        }
                        WordPart::CommandSubstitution { spelling, source } => {
                            self.lower_command_substitution(spelling, source, depth)?
                        }
                        WordPart::Opaque { .. } => return Err(NativeLoweringDecline::OpaqueWord),
                    });
                }
                if let [only] = values.as_slice() {
                    return Ok(*only);
                }
                let dst = self.new_value(
                    NativeType::Obj,
                    Representation::Boxed(Some(TypeShape::String)),
                );
                self.emit(NativeOp::Concat { dst, parts: values });
                Ok(dst)
            }
            WordExpr::Expand { .. } => Err(NativeLoweringDecline::ArgumentExpansion),
            WordExpr::Opaque { .. } => Err(NativeLoweringDecline::OpaqueWord),
        }
    }

    /// A `[…]` word: a native expression when the registry resolves it to the
    /// expression hook over one braced literal and the module keeps `expr`
    /// bound to its builtin, else a nested generic invocation.
    fn lower_command_substitution(
        &mut self,
        spelling: &str,
        source: &SourceSite,
        depth: u32,
    ) -> Result<NativeValueId, NativeLoweringDecline> {
        let inner = nested_words(spelling, source)?;
        if let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_word_exprs(self.input.registry, self.input.context, &inner)
            && let Some(spec) = self.input.registry.get(&facts.canonical_command)
            && spec.native_lowering() == NativeLowering::Structured(LoweringHookId::Expr)
            && inner.len() == 2
            && let WordExpr::BracedLiteral { text, .. } = &inner[1]
            && self.command_trusted(&facts.canonical_command)
        {
            let expr = tcl_syntax::expr::parser::parse_expr_for_profile(text, self.dialect);
            let value = self.lower_expression_text(&expr, text);
            return Ok(self.boxed(value));
        }
        let mut argv = Vec::with_capacity(inner.len());
        for word in &inner {
            argv.push(self.lower_word(word, depth + 1)?);
        }
        self.max_argc = self.max_argc.max(argv.len());
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        self.emit(NativeOp::NestedInvoke { dst, argv });
        self.clobber_shadows();
        self.note_observation();
        Ok(dst)
    }

    /// Whether the module keeps `name` bound to its registry builtin at every
    /// point, so a nested command word may take the builtin's native shape.
    fn command_trusted(&self, name: &str) -> bool {
        let bare = name.strip_prefix("::").unwrap_or(name);
        self.mutations.trusts(bare)
            && !self.input.module.has_dynamic_trace
            && !self.input.module.traced_commands.contains(bare)
            && !self.input.module.traced_commands.contains(name)
    }

    // -- expressions ----------------------------------------------------------

    /// Lower an expression, falling back to the runtime expression intrinsic
    /// over the rendered expression when it has no native shape.
    fn lower_expression(&mut self, expr: &ExprNode) -> NativeValueId {
        let text = render_expr(expr);
        self.lower_expression_text(expr, &text)
    }

    fn lower_expression_text(&mut self, expr: &ExprNode, text: &str) -> NativeValueId {
        if let Ok(value) = self.attempt(|this| this.lower_expr(expr)) {
            return value;
        }
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        self.emit(NativeOp::ExprEval {
            dst,
            text: text.to_owned(),
        });
        self.clobber_shadows();
        self.note_observation();
        dst
    }

    #[allow(clippy::too_many_lines)]
    fn lower_expr(&mut self, node: &ExprNode) -> Result<NativeValueId, ExprDecline> {
        match node {
            ExprNode::Literal { text, .. } => Ok(self.lower_literal(text)),
            ExprNode::String { text, .. } => {
                let inner = text
                    .strip_prefix('{')
                    .and_then(|rest| rest.strip_suffix('}'))
                    .or_else(|| {
                        text.strip_prefix('"')
                            .and_then(|rest| rest.strip_suffix('"'))
                            .filter(|rest| !has_substitution(rest))
                    });
                match inner {
                    Some(inner) => Ok(self.const_str(inner)),
                    None => Err(ExprDecline::SubstitutedString),
                }
            }
            ExprNode::Var { name, .. } => {
                let place = expr_variable_place(name).ok_or(ExprDecline::DynamicVariable)?;
                Ok(self.read_cell(&place))
            }
            ExprNode::Command { .. } => Err(ExprDecline::CommandSubstitution),
            ExprNode::Raw { .. } => Err(ExprDecline::Raw),
            ExprNode::Binary { op, left, right } => self.lower_binary(*op, left, right),
            ExprNode::Unary { op, operand } => self.lower_unary(*op, operand),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                let condition = self.lower_expr(condition)?;
                let condition = self.truth(condition);
                // Exactly one arm runs: each lowers from the state the
                // condition left, and only what both agree on survives.
                let entry_shadows = self.shadows.clone();
                self.push_buffer();
                let then_value = self.lower_expr(true_branch);
                let mut then_ops = self.pop_buffer();
                let then_shadows = std::mem::replace(&mut self.shadows, entry_shadows);
                let then_value = then_value?;
                self.push_buffer();
                let else_value = self.lower_expr(false_branch);
                let mut else_ops = self.pop_buffer();
                self.shadows.intersect(&then_shadows);
                let else_value = else_value?;
                let result = self.merge_arms(&mut then_ops, then_value, &mut else_ops, else_value);
                self.emit(NativeOp::IfElse {
                    condition,
                    then_ops,
                    else_ops,
                    result: Some(result.clone()),
                });
                Ok(result.dst)
            }
            ExprNode::Call { function, args, .. } => self.lower_call(function, args),
        }
    }

    fn lower_literal(&mut self, text: &str) -> NativeValueId {
        if self.representation {
            match self.numbers.parse_whole(text) {
                Some(Number::Int(value)) => return self.const_int(value),
                Some(Number::Double(value)) if value.is_finite() => {
                    let dst = self.new_value(
                        NativeType::F64,
                        Representation::NativeDouble { finite: true },
                    );
                    self.emit(NativeOp::ConstDouble { dst, value });
                    return dst;
                }
                _ => {}
            }
        }
        self.const_str(text)
    }

    fn lower_binary(
        &mut self,
        op: BinOp,
        left: &ExprNode,
        right: &ExprNode,
    ) -> Result<NativeValueId, ExprDecline> {
        match op {
            BinOp::And | BinOp::Or => {
                let lhs = self.lower_expr(left)?;
                let lhs = self.truth(lhs);
                // The right operand is only evaluated when the left does not
                // short-circuit, so its shadows hold on one path only.
                let entry_shadows = self.shadows.clone();
                self.push_buffer();
                let rhs = self.lower_expr(right).map(|value| self.truth(value));
                let rhs_ops = self.pop_buffer();
                self.shadows.intersect(&entry_shadows);
                let rhs = rhs?;
                let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                let short = self.new_value(NativeType::Bool, Representation::NativeBool);
                let short_op = NativeOp::ConstBool {
                    dst: short,
                    value: op == BinOp::Or,
                };
                let (then_ops, else_ops, then_src, else_src) = if op == BinOp::And {
                    (rhs_ops, vec![short_op], rhs, short)
                } else {
                    (vec![short_op], rhs_ops, short, rhs)
                };
                self.emit(NativeOp::IfElse {
                    condition: lhs,
                    then_ops,
                    else_ops,
                    result: Some(IfElseResult {
                        dst,
                        then_src,
                        else_src,
                    }),
                });
                Ok(dst)
            }
            _ => {
                let lhs = self.lower_expr(left)?;
                let rhs = self.lower_expr(right)?;
                self.lower_binary_values(op, lhs, rhs)
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn lower_binary_values(
        &mut self,
        op: BinOp,
        lhs: NativeValueId,
        rhs: NativeValueId,
    ) -> Result<NativeValueId, ExprDecline> {
        let lhs = self.numeric_operand(lhs);
        let rhs = self.numeric_operand(rhs);
        let lrep = self.rep(lhs).clone();
        let rrep = self.rep(rhs).clone();
        if let Some(cmp) = cmp_op(op) {
            match (&lrep, &rrep) {
                (Representation::NativeInt(_), Representation::NativeInt(_)) => {
                    let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                    self.emit(NativeOp::Compare {
                        dst,
                        op: cmp,
                        kind: CompareKind::I64,
                        lhs,
                        rhs,
                    });
                    return Ok(dst);
                }
                // A mixed int/double comparison is only exact while the
                // integer side converts to `f64` without rounding; outside
                // that range the dynamic edge's exact comparator is the only
                // right answer.
                (l, r)
                    if l.is_native_numeric()
                        && r.is_native_numeric()
                        && mixed_compare_is_exact(l, r) =>
                {
                    let lhs = self.promote_double(lhs);
                    let rhs = self.promote_double(rhs);
                    let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                    self.emit(NativeOp::Compare {
                        dst,
                        op: cmp,
                        kind: CompareKind::F64,
                        lhs,
                        rhs,
                    });
                    return Ok(dst);
                }
                _ => {
                    let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                    let hint = numeric_hint(&lrep, &rrep);
                    self.emit(NativeOp::DynamicCompare {
                        dst,
                        op,
                        lhs,
                        rhs,
                        hint,
                    });
                    return Ok(dst);
                }
            }
        }
        if let Some(iop) = int_op(op) {
            if let (Representation::NativeInt(l), Representation::NativeInt(r)) = (&lrep, &rrep)
                && let Some(result) = proven_int_result(iop, *l, *r)
            {
                let dst = self.new_value(NativeType::I64, Representation::NativeInt(result));
                self.emit(NativeOp::IntBinary {
                    dst,
                    op: iop,
                    lhs,
                    rhs,
                });
                return Ok(dst);
            }
            if let Some(dop) = double_op(op)
                && double_result_defined(dop)
                && lrep.is_native_numeric()
                && rrep.is_native_numeric()
                && (matches!(lrep, Representation::NativeDouble { .. })
                    || matches!(rrep, Representation::NativeDouble { .. }))
                && self.finite(lhs)
                && self.finite(rhs)
            {
                let lhs = self.promote_double(lhs);
                let rhs = self.promote_double(rhs);
                let dst = self.new_value(
                    NativeType::F64,
                    Representation::NativeDouble { finite: false },
                );
                self.emit(NativeOp::DoubleBinary {
                    dst,
                    op: dop,
                    lhs,
                    rhs,
                });
                return Ok(dst);
            }
            let hint = numeric_hint(&lrep, &rrep);
            let dst = self.new_value(
                NativeType::Obj,
                Representation::Boxed(Some(TypeShape::Numeric)),
            );
            self.emit(NativeOp::DynamicBinary {
                dst,
                op,
                lhs,
                rhs,
                hint,
            });
            return Ok(dst);
        }
        if op.spec().mathop_shape.is_none() {
            return Err(ExprDecline::UnsupportedOperator);
        }
        let lhs = self.boxed(lhs);
        let rhs = self.boxed(rhs);
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        self.emit(NativeOp::MathOp {
            dst,
            op: op.spec().spelling,
            args: vec![lhs, rhs],
        });
        Ok(dst)
    }

    fn lower_unary(
        &mut self,
        op: UnaryOp,
        operand: &ExprNode,
    ) -> Result<NativeValueId, ExprDecline> {
        let value = self.lower_expr(operand)?;
        match op {
            UnaryOp::Not => {
                let truth = self.truth(value);
                let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                self.emit(NativeOp::NotBool { dst, src: truth });
                Ok(dst)
            }
            UnaryOp::WordNot => Err(ExprDecline::UnsupportedOperator),
            UnaryOp::Neg | UnaryOp::Pos | UnaryOp::BitNot => {
                let value = self.numeric_operand(value);
                let rep = self.rep(value).clone();
                match (op, &rep) {
                    (
                        UnaryOp::Pos,
                        Representation::NativeInt(_) | Representation::NativeDouble { .. },
                    ) => Ok(value),
                    (UnaryOp::Neg, Representation::NativeInt(interval)) => {
                        if let Some(result) = proven_neg_result(*interval) {
                            let dst =
                                self.new_value(NativeType::I64, Representation::NativeInt(result));
                            self.emit(NativeOp::IntNeg { dst, src: value });
                            return Ok(dst);
                        }
                        Ok(self.dynamic_unary(op, value))
                    }
                    (UnaryOp::Neg, Representation::NativeDouble { finite }) => {
                        let dst = self.new_value(
                            NativeType::F64,
                            Representation::NativeDouble { finite: *finite },
                        );
                        self.emit(NativeOp::DoubleNeg { dst, src: value });
                        Ok(dst)
                    }
                    (UnaryOp::BitNot, Representation::NativeInt(_)) => {
                        let dst = self.new_value(NativeType::I64, Representation::any_int());
                        self.emit(NativeOp::IntBitNot { dst, src: value });
                        Ok(dst)
                    }
                    _ => Ok(self.dynamic_unary(op, value)),
                }
            }
        }
    }

    fn dynamic_unary(&mut self, op: UnaryOp, value: NativeValueId) -> NativeValueId {
        let dst = self.new_value(
            NativeType::Obj,
            Representation::Boxed(Some(TypeShape::Numeric)),
        );
        self.emit(NativeOp::DynamicUnary {
            dst,
            op,
            src: value,
        });
        dst
    }

    #[allow(clippy::too_many_lines)]
    fn lower_call(
        &mut self,
        function: &str,
        args: &[ExprNode],
    ) -> Result<NativeValueId, ExprDecline> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            let value = self.lower_expr(arg)?;
            values.push(self.numeric_operand(value));
        }
        if self.mathfunc_native {
            match (function, values.as_slice()) {
                ("double", [value]) => match self.rep(*value) {
                    Representation::NativeInt(_) => return Ok(self.int_to_double(*value)),
                    Representation::NativeDouble { .. } => return Ok(*value),
                    _ => {}
                },
                ("int" | "wide", [value]) => {
                    if matches!(self.rep(*value), Representation::NativeInt(_)) {
                        return Ok(*value);
                    }
                }
                ("abs", [value]) => {
                    if let Representation::NativeInt(interval) = self.rep(*value).clone()
                        && let Some(negated) = proven_neg_result(interval)
                    {
                        let zero = self.const_int(0);
                        let negative = self.new_value(NativeType::Bool, Representation::NativeBool);
                        self.emit(NativeOp::Compare {
                            dst: negative,
                            op: CmpOp::Lt,
                            kind: CompareKind::I64,
                            lhs: *value,
                            rhs: zero,
                        });
                        let neg =
                            self.new_value(NativeType::I64, Representation::NativeInt(negated));
                        let hull = hull(interval, negated);
                        let dst = self.new_value(NativeType::I64, Representation::NativeInt(hull));
                        self.emit(NativeOp::IfElse {
                            condition: negative,
                            then_ops: vec![NativeOp::IntNeg {
                                dst: neg,
                                src: *value,
                            }],
                            else_ops: Vec::new(),
                            result: Some(IfElseResult {
                                dst,
                                then_src: neg,
                                else_src: *value,
                            }),
                        });
                        return Ok(dst);
                    }
                }
                ("max" | "min", [first, second]) => {
                    let both_int = matches!(
                        (self.rep(*first), self.rep(*second)),
                        (Representation::NativeInt(_), Representation::NativeInt(_))
                    );
                    if both_int {
                        let (l, r) = (
                            self.rep(*first)
                                .interval()
                                .unwrap_or(Interval { lo: None, hi: None }),
                            self.rep(*second)
                                .interval()
                                .unwrap_or(Interval { lo: None, hi: None }),
                        );
                        let pick_first =
                            self.new_value(NativeType::Bool, Representation::NativeBool);
                        self.emit(NativeOp::Compare {
                            dst: pick_first,
                            op: if function == "max" {
                                CmpOp::Ge
                            } else {
                                CmpOp::Le
                            },
                            kind: CompareKind::I64,
                            lhs: *first,
                            rhs: *second,
                        });
                        let dst =
                            self.new_value(NativeType::I64, Representation::NativeInt(hull(l, r)));
                        self.emit(NativeOp::IfElse {
                            condition: pick_first,
                            then_ops: Vec::new(),
                            else_ops: Vec::new(),
                            result: Some(IfElseResult {
                                dst,
                                then_src: *first,
                                else_src: *second,
                            }),
                        });
                        return Ok(dst);
                    }
                }
                _ => {}
            }
        }
        let args = values
            .into_iter()
            .map(|value| self.boxed(value))
            .collect::<Vec<_>>();
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        self.emit(NativeOp::MathFunc {
            dst,
            name: function.to_owned(),
            args,
        });
        // The math function resolves through the command table, so it may be
        // user code that observes or writes any cell.
        self.clobber_shadows();
        self.note_observation();
        Ok(dst)
    }

    /// A boolean operand of arithmetic reads as the integer `0`/`1`.
    fn numeric_operand(&mut self, value: NativeValueId) -> NativeValueId {
        if self.ty(value) == NativeType::Bool {
            self.bool_to_int(value)
        } else {
            value
        }
    }

    fn promote_double(&mut self, value: NativeValueId) -> NativeValueId {
        if self.ty(value) == NativeType::I64 {
            self.int_to_double(value)
        } else {
            value
        }
    }

    fn finite(&self, value: NativeValueId) -> bool {
        match self.rep(value) {
            Representation::NativeInt(_) => true,
            Representation::NativeDouble { finite } => *finite,
            _ => false,
        }
    }

    /// Merge the values two arms produce into one, boxing both when their
    /// native types differ.
    fn merge_arms(
        &mut self,
        then_ops: &mut Vec<NativeOp>,
        then_value: NativeValueId,
        else_ops: &mut Vec<NativeOp>,
        else_value: NativeValueId,
    ) -> IfElseResult {
        let then_ty = self.ty(then_value);
        let else_ty = self.ty(else_value);
        if then_ty == else_ty {
            let rep = match (self.rep(then_value).clone(), self.rep(else_value).clone()) {
                (Representation::NativeInt(l), Representation::NativeInt(r)) => {
                    Representation::NativeInt(hull(l, r))
                }
                (
                    Representation::NativeDouble { finite: l },
                    Representation::NativeDouble { finite: r },
                ) => Representation::NativeDouble { finite: l && r },
                (Representation::NativeBool, Representation::NativeBool) => {
                    Representation::NativeBool
                }
                (Representation::Boxed(l), Representation::Boxed(r)) if l == r => {
                    Representation::Boxed(l)
                }
                _ => Representation::Boxed(None),
            };
            let dst = self.new_value(then_ty, rep);
            return IfElseResult {
                dst,
                then_src: then_value,
                else_src: else_value,
            };
        }
        let then_boxed = self.box_into(then_ops, then_value);
        let else_boxed = self.box_into(else_ops, else_value);
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        IfElseResult {
            dst,
            then_src: then_boxed,
            else_src: else_boxed,
        }
    }

    fn box_into(&mut self, ops: &mut Vec<NativeOp>, value: NativeValueId) -> NativeValueId {
        if self.ty(value) == NativeType::Obj {
            return value;
        }
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        ops.push(NativeOp::Box { dst, src: value });
        dst
    }
}

/// A statement carries no block reference; the helper exists so the block
/// parameter of `lower_instruction` stays available to future per-block
/// decisions without a warning today.
trait WithBlock {
    fn with_block(self, block: &ExecutableBlock) -> Self;
}

impl WithBlock for NativeStatement {
    fn with_block(self, _block: &ExecutableBlock) -> Self {
        self
    }
}

/// Whether an invocation's registry effect footprint proves it neither
/// writes a variable cell nor runs a callback that could.
fn cells_unobserved(effects: &tcl_registry::world_effect::EffectFootprint) -> bool {
    use tcl_registry::world_effect::{EffectAccessMode, WorldStateDomain};
    !effects.requires_world_barrier()
        && effects.accesses().iter().all(|access| {
            !matches!(
                access.domain,
                WorldStateDomain::VariableStore | WorldStateDomain::VariableTraces
            ) || access.mode == EffectAccessMode::Read
        })
}

fn hull(a: Interval, b: Interval) -> Interval {
    let lo = match (a.lo, b.lo) {
        (Some(x), Some(y)) => Some(x.min(y)),
        _ => None,
    };
    let hi = match (a.hi, b.hi) {
        (Some(x), Some(y)) => Some(x.max(y)),
        _ => None,
    };
    Interval { lo, hi }
}

fn has_substitution(text: &str) -> bool {
    text.bytes().any(|byte| matches!(byte, b'$' | b'[' | b'\\'))
}

/// The value word of a `set name value` statement's retained tokens.
fn value_word(tokens: Option<&CommandTokens>) -> Option<&WordExpr> {
    let tokens = tokens?;
    if !tokens.words_align_with_argv_text() || tokens.words().len() != 3 {
        return None;
    }
    tokens.words().get(2)
}

/// The cell a statically spelled variable name denotes.
fn cell_place(name: &str, braced: bool) -> Option<CellPlace> {
    match CellReference::from_name(name, braced) {
        CellReference::Named {
            name: base,
            element,
        } => {
            if !element {
                return Some(CellPlace::Named { name: base });
            }
            let (_, key) = tcl_syntax::naming::split_array_name_braced(name, braced);
            let key = key?;
            if !braced && has_substitution(key) {
                return None;
            }
            Some(CellPlace::Element {
                name: base,
                key: key.to_owned(),
            })
        }
        CellReference::Computed => None,
    }
}

/// The cell an expression `$name` / `$arr(key)` reads.
fn expr_variable_place(name: &str) -> Option<CellPlace> {
    let (base, key) = tcl_syntax::naming::split_array_name_braced(name, false);
    if base.is_empty() || has_substitution(base) {
        return None;
    }
    match key {
        None => Some(CellPlace::Named {
            name: base.to_owned(),
        }),
        Some(key) if !has_substitution(key) => Some(CellPlace::Element {
            name: base.to_owned(),
            key: key.to_owned(),
        }),
        Some(_) => None,
    }
}

/// The name a `$…` / `${…}` word spelling refers to.
fn variable_name(spelling: &str) -> Option<&str> {
    if let Some(name) = spelling
        .strip_prefix("${")
        .and_then(|rest| rest.strip_suffix('}'))
    {
        return (!name.is_empty()).then_some(name);
    }
    let name = spelling.strip_prefix('$')?;
    (!name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b':'))
    .then_some(name)
}

/// The cell a variable word reads, distinguishing `$a(b)` from `${a(b)}` by
/// the recorded lexical extent exactly as the leaf-invocation planner does.
fn variable_place(spelling: &str, source: &SourceSite) -> Result<CellPlace, NativeLoweringDecline> {
    let name = variable_name(spelling).ok_or(NativeLoweringDecline::DynamicVariableName)?;
    if !name.contains('(') {
        return Ok(CellPlace::Named {
            name: name.to_owned(),
        });
    }
    if source.provenance != Provenance::Source {
        return Err(NativeLoweringDecline::AmbiguousVariableSpelling);
    }
    match extent(source.span).checked_sub(name.len()) {
        Some(2) => Ok(CellPlace::Named {
            name: name.to_owned(),
        }),
        Some(1) => {
            let inner = name
                .strip_suffix(')')
                .ok_or(NativeLoweringDecline::AmbiguousVariableSpelling)?;
            let open = inner
                .find('(')
                .ok_or(NativeLoweringDecline::AmbiguousVariableSpelling)?;
            let (base, key) = inner.split_at(open);
            let key = &key[1..];
            if base.is_empty() {
                return Err(NativeLoweringDecline::AmbiguousVariableSpelling);
            }
            if has_substitution(key) {
                return Err(NativeLoweringDecline::DynamicVariableName);
            }
            Ok(CellPlace::Element {
                name: base.to_owned(),
                key: key.to_owned(),
            })
        }
        _ => Err(NativeLoweringDecline::AmbiguousVariableSpelling),
    }
}

fn extent(span: Span) -> usize {
    span.end().saturating_sub(span.start()) as usize
}

/// The structured words of a `[…]` command substitution, recovered by the
/// canonical segmenter over the recorded lexical extent.
fn nested_words(
    spelling: &str,
    source: &SourceSite,
) -> Result<Vec<WordExpr>, NativeLoweringDecline> {
    let inner = spelling
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
        .ok_or(NativeLoweringDecline::UnmodelledCommandSubstitution)?;
    if inner.trim().is_empty() {
        return Err(NativeLoweringDecline::UnmodelledCommandSubstitution);
    }
    let base = if source.provenance == Provenance::Source {
        source.span.start().saturating_add(1)
    } else {
        0
    };
    let segments = crate::segmenter::segment_commands_with_offset(inner, base);
    let [segment] = segments.as_slice() else {
        return Err(NativeLoweringDecline::UnmodelledCommandSubstitution);
    };
    if segment.is_partial {
        return Err(NativeLoweringDecline::UnmodelledCommandSubstitution);
    }
    let tokens = CommandTokens::from_segmented(segment);
    if tokens.word_exprs.is_empty() {
        return Err(NativeLoweringDecline::MissingCommandTokens);
    }
    Ok(tokens.word_exprs)
}

/// Reverse post-order over the executable CFG, the loop headers (targets of
/// back edges), and every block's predecessor list.
fn block_order(function: &ExecutableFunction) -> (Vec<usize>, HashSet<usize>, Vec<Vec<usize>>) {
    let count = function.blocks.len();
    let successors: Vec<Vec<usize>> = function
        .blocks
        .iter()
        .map(|block| match &block.terminator {
            Some(ExecutableTerminator::Goto(target)) => vec![target.index()],
            Some(ExecutableTerminator::Branch {
                then_target,
                else_target,
                ..
            }) => vec![then_target.index(), else_target.index()],
            Some(ExecutableTerminator::CompletionSwitch { cases, default, .. }) => {
                let mut targets: Vec<usize> =
                    cases.iter().map(|case| case.target.index()).collect();
                targets.push(default.index());
                targets
            }
            Some(ExecutableTerminator::ReturnCompletion(_)) | None => Vec::new(),
        })
        .collect();
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (from, targets) in successors.iter().enumerate() {
        let mut seen = HashSet::new();
        for target in targets {
            if *target < count && seen.insert(*target) {
                predecessors[*target].push(from);
            }
        }
    }
    // Iterative DFS for post-order and back-edge detection.
    let mut state = vec![0u8; count]; // 0 unvisited, 1 on stack, 2 done
    let mut post = Vec::with_capacity(count);
    let mut headers = HashSet::new();
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let entry = function.entry.index();
    if entry < count {
        state[entry] = 1;
        stack.push((entry, 0));
    }
    while let Some((block, next)) = stack.last_mut() {
        let block = *block;
        if *next < successors[block].len() {
            let target = successors[block][*next];
            *next += 1;
            if target >= count {
                continue;
            }
            match state[target] {
                0 => {
                    state[target] = 1;
                    stack.push((target, 0));
                }
                1 => {
                    headers.insert(target);
                }
                _ => {}
            }
        } else {
            state[block] = 2;
            post.push(block);
            stack.pop();
        }
    }
    post.reverse();
    (post, headers, predecessors)
}

fn completion_count(function: &ExecutableFunction) -> usize {
    let mut count = 0;
    for block in &function.blocks {
        for instruction in &block.instructions {
            count = count.max(instruction_completion(instruction).index() + 1);
        }
        if let Some(
            ExecutableTerminator::CompletionSwitch { completion, .. }
            | ExecutableTerminator::ReturnCompletion(completion),
        ) = &block.terminator
        {
            count = count.max(completion.index() + 1);
        }
    }
    count
}

fn instruction_completion(
    instruction: &ExecutableInstruction,
) -> crate::executable_ir::CompletionId {
    match instruction {
        ExecutableInstruction::EvaluateWord { completion, .. }
        | ExecutableInstruction::ExpandWord { completion, .. }
        | ExecutableInstruction::BuildArgv { completion, .. }
        | ExecutableInstruction::EvaluateExpr { completion, .. }
        | ExecutableInstruction::MatchPattern { completion, .. }
        | ExecutableInstruction::JoinCompletion { completion, .. }
        | ExecutableInstruction::WriteCompletionCell { completion, .. }
        | ExecutableInstruction::IterateLists { completion, .. } => *completion,
        ExecutableInstruction::Invoke(invoke) => invoke.completion,
        ExecutableInstruction::ExecuteLowered(operation) => operation.completion,
        ExecutableInstruction::ExecuteOpaqueRegion(region) => region.completion,
        ExecutableInstruction::CompleteStructuredRegion(region) => region.completion,
    }
}

/// A completion for a block the executable CFG never reaches.
fn unreachable_completion(function: &ExecutableFunction) -> crate::executable_ir::CompletionId {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .map(instruction_completion)
        .next()
        .unwrap_or_else(|| crate::executable_ir::CompletionId::new(function.id, 0))
}

/// Whether comparing these two native numeric representations through `f64`
/// gives Tcl's answer. Two doubles always do; a mixed pair only while the
/// integer side is exactly representable (see
/// [`exactly_representable_as_double`]). Two integers never reach here.
fn mixed_compare_is_exact(l: &Representation, r: &Representation) -> bool {
    match (l, r) {
        (Representation::NativeInt(i), Representation::NativeDouble { .. })
        | (Representation::NativeDouble { .. }, Representation::NativeInt(i)) => {
            exactly_representable_as_double(*i)
        }
        _ => true,
    }
}
