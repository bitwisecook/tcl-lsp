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

use tcl_dialect::DialectProfile;
use tcl_registry::CommandRegistry;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::native_boolean_truth::{
    NativeBooleanExpressionResultProduction, NativeBooleanTruthPurpose,
};
use tcl_syntax::expr::ast::render_expr;
use tcl_syntax::number::Numbers;

use super::cells::{CellPlace, ShadowState};
use super::elide::{BarrierDecision, CellDemotion, TraceLedger};
use super::ir::{
    EntryProtocol, NativeBlock, NativeBlockId, NativeFunction, NativeOp, NativeStatement,
    NativeTerminator, NativeType, NativeValue, NativeValueId, StatementSite,
};
use super::representation::Representation;
use super::{
    CellAccessKind, CellAccessRecord, FunctionDecline, FunctionReport, FunctionStatus,
    NativeBinding, NativeLoweringDecline, StatementOutcome, StatementRecord,
};
use crate::codegen_helpers::command_text;
use crate::command_binding::ModuleCommandMutations;
use crate::dispatch_proof::{
    DispatchEntryAssumption, DispatchProofAnalysis, analyse_original_dispatch_stability,
};
use crate::executable_ir::{
    ExecutableArgvId, ExecutableBlock, ExecutableExpr, ExecutableFunction, ExecutableInstruction,
    ExecutableTerminator, ExecutableValueId,
};
use crate::intervals::Interval;
use crate::ir::Module;
use crate::registry_invocation::InvocationMetadataContext;
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::types::TypeShape;
use crate::var_escape::types::ProcEscapeSummary;
use tcl_source::words::NodeId;
use tcl_source::words::SourceSite;

mod expressions;
mod invocations;
mod lowered_statements;
mod words;

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
    /// The lowered module: trace targets and procedures.
    pub module: &'a Module,
    /// The compilation unit's prepared whole-module command-mutation summary.
    pub mutations: &'a ModuleCommandMutations,
    /// The enabled semantic optimisation passes.
    pub config: SemanticOptimisationConfig,
    /// The escape summary of a procedure body, when lowering one.
    pub escape: Option<&'a ProcEscapeSummary>,
    /// Whether this is the top-level script.
    pub top_level: bool,
    /// The offset in `source` the enclosing body opens at: `0` for the
    /// top-level script, the procedure body's first content byte for a
    /// procedure. A statement's absolute span less this offset is its
    /// body-relative position, which is the line `errorInfo` reports.
    pub line_origin: u32,
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
    OriginalBooleanObjectUnavailable,
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
    if input
        .function
        .native_compilation_admission
        .as_ref()
        .is_some_and(|admission| admission.requires_native_provider())
    {
        return Err(FunctionDecline::NativeCompilationAdmissionRequired);
    }
    if input.function.validate().is_err() {
        return Err(FunctionDecline::InvalidExecutableIr);
    }
    if let Some(kind) = unlowered_instruction(input.function) {
        return Err(FunctionDecline::UnloweredInstruction(kind));
    }
    let mut lowerer = Lowerer::new(input);
    let function = lowerer.lower();
    if lowerer.boolean_admission_missing {
        return Err(FunctionDecline::NativeCompilationAdmissionRequired);
    }
    let report = FunctionReport {
        status: FunctionStatus::Lowered,
        // The emitter decides binding, once it knows whether the module
        // installed an entry for the body.
        binding: NativeBinding::NotApplicable,
        statements: lowerer.records,
    };
    Ok((function, report))
}

/// The first executable instruction kind this lowering does not project.
fn unlowered_instruction(function: &ExecutableFunction) -> Option<&'static str> {
    // No partial lowering: the original wrapper would replay phases already
    // represented by executable edges. Reject before constructing the lowerer.
    if function.blocks.iter().any(|block| {
        matches!(
            block.terminator,
            Some(ExecutableTerminator::RegionChoice { .. })
        )
    }) {
        return Some("evaluated-body-region");
    }
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match instruction {
            ExecutableInstruction::CompleteEvaluatedRegion(_) => Some("evaluated-body-region"),
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

/// Native truth stages require a current complete source input and its
/// independently retained interpreter entry. Availability does not select a
/// physical point, and stale entry metadata cannot donate its point.
fn native_boolean_dialect(input: &LoweringInput<'_>) -> Option<tcl_registry::InvocationDialect> {
    let metadata = InvocationMetadataContext::for_module(input.registry, input.module)?;
    if input
        .module
        .source_entry
        .metadata_context
        .source_analysis_input()
        != metadata.source_analysis_input()
    {
        return None;
    }
    let entry = input.module.source_entry.native_entry.as_deref()?;
    entry
        .execution_point
        .map(tcl_registry::InvocationDialect::of_point)
}

struct Lowerer<'a> {
    input: &'a LoweringInput<'a>,
    original_invocations: Option<super::original_invocations::OriginalInvocations>,
    values: Vec<NativeValue>,
    exec_values: HashMap<ExecutableValueId, NativeValueId>,
    argvs: HashMap<ExecutableArgvId, Vec<NativeValueId>>,
    ledger: TraceLedger<'a>,
    demotion: CellDemotion<'a>,
    mutations: &'a ModuleCommandMutations,
    proofs: DispatchProofAnalysis,
    numbers: Numbers,
    dialect: Option<&'static DialectProfile>,
    /// A current source owner and independently retained physical entry select
    /// the truth recipe; an authored profile alone supplies no opcode protocol.
    boolean_dialect: Option<tcl_registry::InvocationDialect>,
    boolean_admission_missing: bool,
    /// The document's lexer grammar, resolved once with `dialect` and
    /// threaded into every re-read of source text this pass makes
    /// (`docs/design/registry/dialect-profile-model.md` §2.5).
    lexer_config: tcl_lexer::LexerConfig,
    representation: bool,
    mathfunc_native: bool,
    declined_nodes: BTreeMap<NodeId, NativeLoweringDecline>,
    /// The enclosing command's source site per node, so a statement that
    /// evaluates one word of a command still names the whole command in an
    /// `errorInfo` frame — which is what the eval loop logs.
    command_sites: BTreeMap<NodeId, SourceSite>,
    /// Nodes whose operand words dispatched a runtime command (a nested
    /// invocation, a math function through the command table, a runtime
    /// expression) or read a cell whose trace barrier was kept, so the
    /// site's own operand-admissibility verdict must stand.
    observing_nodes: HashSet<NodeId>,
    current_node: Option<NodeId>,
    current_site: Option<StatementSite>,
    shadows: ShadowState,
    boxed_integer_sources: BTreeMap<NativeValueId, NativeValueId>,
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
            // A source trace ledger cannot enumerate observers already installed
            // in an independently entered native frame. Retain the runtime bit.
            module.has_dynamic_variable_trace || module.source_entry.native_entry.is_some(),
            input.config,
        );
        let demotion = if input.top_level {
            CellDemotion::top_level(input.config)
        } else {
            // Named escape summaries are advisory. An actual native entry
            // requires a separately retained byte/ordinal/layout projection;
            // this lowering does not yet consume that capability.
            CellDemotion::procedure(
                input
                    .escape
                    .filter(|_| input.module.source_entry.native_entry.is_none()),
                input.config,
            )
        };
        let environment = input.context.map(SemanticContext::environment_id);
        // The document's profile, taken from the ingress rather than found
        // on a catalogue row by name. `DialectProfile::find` answers `None`
        // for an environment with a Tcl ladder and no row — `jim` — so a
        // name lookup here would hand this pass the permissive fallback and
        // lower a Jim unit as Tcl 9.0. The
        // context's id is already the ingress's canonical one, so this is a
        // keyed lookup, not a second resolution of a user string, and
        // `unit_profile` is the same door `CompilationUnit::build_for_dialect`
        // uses (so `tk` keeps its typed profile).
        let profile = environment
            .map(|id| crate::environment_ingress::resolve_environment(id).unit_profile());
        // `expr` resolves `abs(…)` through the command table, so a native
        // math function is only sound while nothing in the module can have
        // replaced one: no dynamic trace, no proc declaring one, and no
        // `rename` / `interp alias` touching the namespace (a dynamic
        // mutation could touch anything).
        let mutations = input.mutations;
        let mathfunc_native = !module.has_dynamic_trace
            && !mutations.has_dynamic_mutation()
            && !mutations.changes_command_resolution()
            && !mutations.rebinds_under("::tcl::mathfunc::")
            && !module.procedures.values().any(|procedure| {
                procedure.publication.as_ref().is_none_or(|publication| {
                    publication.slot().namespace.as_segments()
                        == [
                            tcl_core_types::NameBytes::from("tcl"),
                            tcl_core_types::NameBytes::from("mathfunc"),
                        ]
                })
            });
        Self {
            input,
            original_invocations: super::original_invocations::OriginalInvocations::for_module(
                module,
                input.registry,
                input.source,
            ),
            values: Vec::new(),
            exec_values: HashMap::new(),
            argvs: HashMap::new(),
            ledger,
            demotion,
            mutations,
            proofs: analyse_original_dispatch_stability(
                input.function,
                input.entry_assumption,
                input.registry,
                crate::registry_invocation::InvocationMetadataContext::for_module(
                    input.registry,
                    module,
                ),
            ),
            numbers: Numbers::Target(module.number_syntax()),
            dialect: profile,
            boolean_dialect: native_boolean_dialect(input),
            boolean_admission_missing: false,
            lexer_config: module.native_lexer_config(),
            representation: input
                .config
                .is_enabled(SemanticOptimisationPassId::RepresentationInference),
            mathfunc_native,
            declined_nodes: BTreeMap::new(),
            command_sites: BTreeMap::new(),
            observing_nodes: HashSet::new(),
            current_node: None,
            current_site: None,
            shadows: ShadowState::default(),
            boxed_integer_sources: BTreeMap::new(),
            exit_shadows: vec![None; input.function.blocks.len()],
            records: Vec::new(),
            current: None,
            ops: Vec::new(),
            max_argc: 0,
        }
    }

    // Values and operations.

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
        if let NativeOp::Box { dst, src } = &op
            && self.ty(*src) == NativeType::I64
        {
            self.boxed_integer_sources.insert(*dst, *src);
        }
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

    /// Convert the original operand at its actual reached instruction.
    /// Canonical Boolean values need no object conversion; all other values
    /// remain boxed so the selected getter observes their original cache.
    fn truth(&mut self, value: NativeValueId, purpose: NativeBooleanTruthPurpose) -> NativeValueId {
        if self.ty(value) == NativeType::Bool {
            return value;
        }
        if self
            .boolean_dialect
            .and_then(|dialect| dialect.native_boolean_truth_protocol(purpose))
            .is_none()
        {
            self.boolean_admission_missing = true;
        }
        debug_assert_eq!(self.ty(value), NativeType::Obj);
        let src = value;
        let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
        self.emit(NativeOp::UnboxBool { dst, src, purpose });
        dst
    }

    fn expression_result_truth(
        &mut self,
        value: NativeValueId,
        production: NativeBooleanExpressionResultProduction,
    ) -> NativeValueId {
        if self
            .boolean_dialect
            .and_then(|dialect| dialect.native_boolean_expression_result_protocol(production))
            .is_none()
        {
            self.boolean_admission_missing = true;
        }
        let src = self.boxed(value);
        let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
        self.emit(NativeOp::BooleanExpressionResult {
            dst,
            src,
            production,
        });
        dst
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

    // Cells.

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
        self.read_original_cell(place)
    }

    /// A reached Boolean operand retains the actual cell's object and cache,
    /// independently of an equal numeric value in the shadow lattice.
    fn read_original_cell(&mut self, place: &CellPlace) -> NativeValueId {
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
        self.remember_cached_increment(place, boxed, shadow, barrier);
        self.record_cell(place, CellAccessKind::Write, barrier, false);
    }

    fn clobber_shadows(&mut self) {
        self.shadows.clobber();
    }

    /// Record that the statement's operand words observed the world through
    /// a runtime dispatch or an unproven cell read.
    fn note_observation(&mut self) {
        self.shadows.forget_cached_increments();
        if let Some(node) = &self.current_node {
            self.observing_nodes.insert(node.clone());
        }
    }

    // Driver.

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
            native_compilation_admission: function.native_compilation_admission.clone(),
            values: std::mem::take(&mut self.values),
            blocks,
            entry: NativeBlockId(u32::try_from(function.entry.index()).unwrap_or(u32::MAX)),
            completion_count: completion_count(function),
            max_argc: self.max_argc,
            protocol: if self.input.top_level {
                EntryProtocol::Script
            } else {
                EntryProtocol::ProcEntry
            },
        }
    }

    /// Decide, before any operation is emitted, which invocation nodes must
    /// keep the source-text rung because one of their words cannot be
    /// evaluated structurally.
    fn prescan(&mut self) {
        for block in &self.input.function.blocks {
            for instruction in &block.instructions {
                // The command site every statement of this command reports.
                // A word evaluation carries only its *word's* site, but the
                // eval loop logs the whole command, so the command-carrying
                // instructions publish theirs for the whole node.
                match instruction {
                    ExecutableInstruction::Invoke(invoke) => {
                        self.command_sites
                            .insert(invoke.node.clone(), invoke.source.clone());
                    }
                    ExecutableInstruction::ExecuteLowered(operation) => {
                        self.command_sites
                            .insert(operation.node.clone(), operation.source.clone());
                    }
                    ExecutableInstruction::ExecuteOpaqueRegion(region) => {
                        self.command_sites
                            .insert(region.node.clone(), region.source.clone());
                    }
                    ExecutableInstruction::CompleteStructuredRegion(region) => {
                        self.command_sites
                            .insert(region.node.clone(), region.source.clone());
                    }
                    _ => {}
                }
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
            ExecutableTerminator::RegionChoice { .. } => {
                unreachable!("the function-level pre-check declines evaluated regions")
            }
            ExecutableTerminator::Goto(next) => NativeTerminator::Goto(target(*next)),
            ExecutableTerminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let condition = self.exec_values[condition];
                debug_assert_eq!(self.ty(condition), NativeType::Bool);
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
        self.current_site = node.and_then(|node| self.statement_site(node));
        self.current = Some(StatementRecord {
            node: node.cloned(),
            instruction,
            outcome: StatementOutcome::Empty,
            cells: Vec::new(),
            representations: Vec::new(),
        });
        self.push_buffer();
    }

    /// The command site an `errorInfo` frame for `node` names: its exact
    /// source text and its 1-based line within the body being compiled.
    fn statement_site(&self, node: &NodeId) -> Option<StatementSite> {
        let source = self.command_sites.get(node)?;
        let text = command_text(
            self.input.source,
            source.span,
            &self.input.module.source,
            self.input.module.lexer_config,
        )
        .to_owned();
        Some(StatementSite {
            line: body_line(
                self.input.source,
                self.input.line_origin,
                source.span.start(),
            ),
            text,
        })
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
            site: self.current_site.take(),
            ops,
        }
    }

    fn eval_source(&mut self, source: &SourceSite, reason: NativeLoweringDecline) {
        let text = command_text(
            self.input.source,
            source.span,
            &self.input.module.source,
            self.input.module.lexer_config,
        )
        .to_owned();
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
                        let text = render_expr(expr);
                        let truth = self.lower_condition_expression(expr, &text);
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
            ExecutableInstruction::CompleteEvaluatedRegion(_)
            | ExecutableInstruction::MatchPattern { .. }
            | ExecutableInstruction::IterateLists { .. }
            | ExecutableInstruction::JoinCompletion { .. }
            | ExecutableInstruction::WriteCompletionCell { .. } => {
                unreachable!("the function-level pre-check declines these instructions")
            }
        }
        .with_block(block)
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

/// The 1-based line of `offset` within the body that opens at `origin`.
///
/// `errorInfo` counts a procedure's lines from the first byte of its body
/// text, not from the start of the file, which is exactly what
/// `Interp::log_command_bytes` expects from a compiled statement.
fn body_line(source: &str, origin: u32, offset: u32) -> u32 {
    let Some(preceding) = source.get(origin as usize..offset as usize) else {
        return 1;
    };
    u32::try_from(preceding.matches('\n').count())
        .unwrap_or(u32::MAX)
        .saturating_add(1)
}

/// Reverse post-order over the executable CFG, the loop headers (targets of
/// back edges), and every block's predecessor list.
fn block_order(function: &ExecutableFunction) -> (Vec<usize>, HashSet<usize>, Vec<Vec<usize>>) {
    let count = function.blocks.len();
    let successors: Vec<Vec<usize>> = function
        .blocks
        .iter()
        .map(|block| match &block.terminator {
            Some(ExecutableTerminator::RegionChoice { enter, skip }) => {
                vec![enter.index(), skip.index()]
            }
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
        ExecutableInstruction::CompleteEvaluatedRegion(region) => region.invocation.completion,
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

#[cfg(test)]
mod evaluated_region_tests;

#[cfg(test)]
mod original_entry_storage_tests;
