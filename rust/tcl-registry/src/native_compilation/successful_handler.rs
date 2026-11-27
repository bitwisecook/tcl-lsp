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

//! What a command's handler does once it has succeeded: the worker it reaches, variable outputs, user-procedure calls and possible body flow.

use super::implementation_lookup::NativeCompilerImplementationLookup;
use super::{NativeCompilationGrammar, NativeCompilationSpec};
use crate::SemanticOperationId;
use tcl_dialect::TclVersion;
use tcl_dialect::model::Family;
use tcl_native_model::InvocationDialect;

/// A vendor-owned value-handler contract independent of C compiler workers.
/// The caller must still prove the original converged public implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NormalValueLeafProvider {
    /// BIG-IP's documented string value operations on the selected TMM handler.
    F5String,
    /// BIG-IP's documented binary value operations on the selected TMM handler.
    F5Binary,
}

impl NormalValueLeafProvider {
    fn admits(self, dialect: InvocationDialect) -> bool {
        matches!(self, Self::F5String | Self::F5Binary)
            && dialect.family() == Some(Family::F5Irules)
    }
}

/// Audited successful-handler transfer, independently of compiler selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuccessfulHandlerSpec {
    /// Evaluate the selected expression arguments on the handler's normal
    /// path. Compiler preparation, operand callbacks and result headers are
    /// independent obligations of the expression owner.
    ExpressionArguments,
    /// A native leaf with no script or variable-name operands.
    Leaf,
    /// Sequence operands preserve the world only when the shared native
    /// argument decoder never enters its Tcl expression callback.
    ArithmeticSequenceArguments,
    /// Dictionary construction retains value objects without coercing them;
    /// only keys require materialised strings for closed normal world effects.
    DictionaryConstructor {
        /// Original mapping and independently mutable native worker.
        lookup: &'static NativeCompilerImplementationLookup,
        /// First C release installing this worker.
        implementation_from: TclVersion,
    },
    /// A native value leaf whose C ensemble worker became independently mutable.
    EnsembleLeaf {
        /// Original mapping and selected private worker on applicable C releases.
        lookup: &'static NativeCompilerImplementationLookup,
        /// First C release with this private worker protocol.
        implementation_from: TclVersion,
        /// Independent vendor handler contract, without a C worker assumption.
        direct_provider: Option<NormalValueLeafProvider>,
    },
    /// A native value leaf reached through multiple independently mutable ensembles.
    EnsemblePathLeaf {
        /// Frozen selector and all original public-to-private map dependencies.
        lookup: &'static crate::native_handler_path::NativeHandlerLookupPaths,
        /// First C release installing this nested worker protocol.
        implementation_from: TclVersion,
    },
    /// Audited binding installation uses only declared command-table transitions.
    /// This does not imply a pure value leaf or execute the installed target.
    CommandBindingTransition,
    /// Variable access roles describe all selected target names on success.
    VariableOperands,
    /// Original string-append handler, with the same variable-name/observer
    /// ordering as `VariableOperands`. Result header alternatives and COW retain
    /// their original recipe; no cache/type, Normal or opcode grant is implied.
    VariableStringAppend,
    /// Variable targets remain conditional until this native matcher proves
    /// which stores occur on the successful continuation.
    ConditionalVariableOperands(crate::variable_output::NativeVariableOutputSpec),
    /// Read the selected variable, creating an empty value only when it remains
    /// undefined after read observers. Existing values are not stored again.
    InitialiseEmptyVariable,
    /// Catch captures selected output names after its body completes.
    CatchOutputs,
    /// Entry and completion-dependent writes belong to the shared dictionary scope protocol.
    DictionaryScope,
    /// Audited caller-frame scripts can execute under an unknown compiler
    /// protocol. Their inventory is possible execution, never opcode proof.
    PossibleBodies,
    /// A converged handler invokes the named procedure with its remaining argv.
    /// This caller inventory grants no compiler operation or procedure effects.
    UserProcedureCall,
}

/// Additional identity required before a reached normal handler can preserve world facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalHandlerImplementationLookup {
    /// This implementation invokes no independently selected worker.
    NoneRequired,
    /// The selected original worker and ensemble mapping must be proved.
    Required(NativeCompilerImplementationLookup),
    /// Each selected path edge requires its own actual ensemble and worker proof.
    RequiredPath(&'static crate::native_handler_path::NativeHandlerLookupPaths),
    /// The selected native protocol is not known well enough to choose a dependency.
    Unknown,
}

/// Exact operand layout of a converged handler's procedure invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserProcedureInvocation {
    /// Effective operand containing the literal procedure name.
    pub target_at: usize,
    /// First operand passed to the procedure, preserving every remaining slot.
    pub arguments_from: usize,
}

/// Candidate script execution of a converged native handler. Compiler entry
/// and execution certainty remain independent of this handler grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PossibleHandlerBodyFlow {
    /// Shared native argument grammar's execution ordering.
    pub flow: crate::script_body_flow::ScriptBodyFlow,
    /// Proved caller activation in which the handler evaluates these scripts.
    pub frame: crate::VariableAliasFrame,
}

/// When an operation selects the variable named by a retained argv operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableOperandBindingPhase {
    /// The selected arguments and frame identify the target before body execution.
    AfterArguments,
    /// A script body can retarget aliases before an output variable is selected.
    NormalContinuation,
    /// Entry and writeback select addresses through a body execution protocol.
    BodyProtocol,
}

/// Whether outputs are looked up once or separately around observable writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableOutputLookup {
    /// The operation selects one target before any value write callbacks.
    SingleTarget,
    /// Each named output is looked up after the preceding output write.
    Sequential,
    /// Catch captures follow a dialect- and compiler-selected order after its body.
    CatchSelected,
    /// A body protocol owns ordered entry and completion-dependent target selection.
    BodyProtocol,
}

impl SuccessfulHandlerSpec {
    /// Original native worker retained by a normal-handler contract. The
    /// caller independently proves applicable engine/release and live identity.
    #[must_use]
    pub const fn native_worker(
        self,
    ) -> Option<(&'static NativeCompilerImplementationLookup, TclVersion)> {
        match self {
            Self::EnsembleLeaf {
                lookup,
                implementation_from,
                ..
            }
            | Self::DictionaryConstructor {
                lookup,
                implementation_from,
            } => Some((lookup, implementation_from)),
            _ => None,
        }
    }

    /// Original private slots needed by an explicitly trusted stock bootstrap.
    /// Enumerating alternatives proves neither a selected path nor a live
    /// worker, ensemble mapping, native compiler hook, or execution effect.
    #[must_use]
    pub fn stock_native_workers(
        self,
        dialect: InvocationDialect,
    ) -> Vec<NativeCompilerImplementationLookup> {
        if dialect.family() != Some(Family::Tcl) {
            return Vec::new();
        }
        let Some(version) = dialect.tcl_version else {
            return Vec::new();
        };
        if let Self::EnsemblePathLeaf {
            lookup,
            implementation_from,
        } = self
        {
            return if version >= implementation_from {
                lookup
                    .alternatives
                    .iter()
                    .flat_map(|path| path.lookups.iter().copied())
                    .collect()
            } else {
                Vec::new()
            };
        }
        self.native_worker()
            .filter(|(_, first)| version >= *first)
            .map_or_else(Vec::new, |(lookup, _)| vec![*lookup])
    }
    /// Physical target lookup order; output callbacks are not replay-free.
    #[must_use]
    pub const fn variable_output_lookup(self) -> VariableOutputLookup {
        match self {
            Self::Leaf
            | Self::ExpressionArguments
            | Self::ArithmeticSequenceArguments
            | Self::DictionaryConstructor { .. }
            | Self::EnsembleLeaf { .. }
            | Self::EnsemblePathLeaf { .. }
            | Self::PossibleBodies
            | Self::UserProcedureCall
            | Self::InitialiseEmptyVariable
            | Self::CommandBindingTransition => VariableOutputLookup::SingleTarget,
            Self::VariableOperands
            | Self::VariableStringAppend
            | Self::ConditionalVariableOperands(_) => VariableOutputLookup::Sequential,
            Self::CatchOutputs => VariableOutputLookup::CatchSelected,
            Self::DictionaryScope => VariableOutputLookup::BodyProtocol,
        }
    }

    /// Address selection phase for outputs of this normal transfer contract.
    #[must_use]
    pub const fn variable_binding_phase(self) -> VariableOperandBindingPhase {
        match self {
            Self::CatchOutputs => VariableOperandBindingPhase::NormalContinuation,
            Self::DictionaryScope => VariableOperandBindingPhase::BodyProtocol,
            Self::Leaf
            | Self::ExpressionArguments
            | Self::ArithmeticSequenceArguments
            | Self::DictionaryConstructor { .. }
            | Self::EnsembleLeaf { .. }
            | Self::EnsemblePathLeaf { .. }
            | Self::VariableOperands
            | Self::VariableStringAppend
            | Self::ConditionalVariableOperands(_)
            | Self::InitialiseEmptyVariable
            | Self::PossibleBodies
            | Self::UserProcedureCall
            | Self::CommandBindingTransition => VariableOperandBindingPhase::AfterArguments,
        }
    }
}

impl crate::InvocationFacts {
    /// Project an authored normal-handler worker dependency independently of
    /// compiler selection. Unknown C release/protocol cannot mean no dependency.
    #[must_use]
    pub fn normal_handler_implementation_lookup(
        &self,
        dialect: Option<InvocationDialect>,
    ) -> NormalHandlerImplementationLookup {
        use NormalHandlerImplementationLookup as Lookup;
        if self.body_execution == Some(crate::body_execution::BodyExecutionSpec::ArrayIteration) {
            return match dialect {
                Some(dialect)
                    if dialect.family() == Some(Family::Tcl)
                        && dialect
                            .tcl_version
                            .is_some_and(|version| version >= TclVersion::V9_0) =>
                {
                    Lookup::Required(crate::array_iteration::IMPLEMENTATION_LOOKUP)
                }
                _ => Lookup::Unknown,
            };
        }
        let Some(contract) = self.successful_handler else {
            return Lookup::NoneRequired;
        };
        if let SuccessfulHandlerSpec::EnsemblePathLeaf {
            lookup,
            implementation_from,
        } = contract
        {
            return match dialect {
                Some(dialect)
                    if dialect.family() == Some(Family::Tcl)
                        && dialect
                            .tcl_version
                            .is_some_and(|version| version >= implementation_from) =>
                {
                    Lookup::RequiredPath(lookup)
                }
                _ => Lookup::Unknown,
            };
        }
        let Some((lookup, implementation_from)) = contract.native_worker() else {
            return Lookup::NoneRequired;
        };
        let direct_provider = match contract {
            SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider, ..
            } => direct_provider,
            _ => None,
        };
        let Some(dialect) = dialect else {
            return Lookup::Unknown;
        };
        match dialect.family() {
            Some(Family::Tcl) => match dialect.tcl_version {
                Some(version) if version >= implementation_from => Lookup::Required(*lookup),
                Some(_) => Lookup::NoneRequired,
                None => Lookup::Unknown,
            },
            Some(Family::Jim) => Lookup::NoneRequired,
            _ if direct_provider.is_some_and(|provider| provider.admits(dialect)) => {
                Lookup::NoneRequired
            }
            // No F5 trunk host has namespace ensembles: `namespace ensemble`
            // is the 8.4 `bad option` in TMM, iApp, iCall and tmsh on
            // 21.1.0.1 (bigip-name-checks.md, section 3), so a value leaf has
            // no private worker to rebind before its first C release.
            Some(Family::F5Tcl)
                if dialect
                    .tcl_version
                    .is_some_and(|version| version < implementation_from) =>
            {
                Lookup::NoneRequired
            }
            _ => Lookup::Unknown,
        }
    }
    /// Caller inventory of an independently proved normal handler. Neither
    /// compiler uncertainty nor possible pre-entry failure invents callers;
    /// actual handler or operand uncertainty still prevents this projection.
    #[must_use]
    pub fn successful_handler_user_procedure_call(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<UserProcedureInvocation> {
        if self.successful_handler != Some(SuccessfulHandlerSpec::UserProcedureCall)
            || !self.traits.contains(crate::Traits::INVOKES_USER_PROC)
            || !self.arg_roles_complete
        {
            return None;
        }
        arguments.exact_argv_len()?;
        if self.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        let mut names = self.arg_roles.iter().filter_map(|&(index, role)| {
            (role == crate::ArgRole::Name).then_some(self.argument_offset + usize::from(index))
        });
        let target_at = names.next()?;
        if names.next().is_some() {
            return None;
        }
        arguments.literal_at(target_at)?;
        Some(UserProcedureInvocation {
            target_at,
            arguments_from: target_at + 1,
        })
    }

    /// Candidate body entry after independently proved handler convergence.
    /// Uses the existing shared script grammar and preserves unknown native
    /// compilation protocol; consumers must not turn candidates into Must.
    #[must_use]
    pub fn possible_handler_body_flow(
        &self,
        registry: &crate::CommandRegistry,
        arguments: crate::InvocationArguments<'_>,
        frame: crate::VariableAliasFrame,
    ) -> Option<PossibleHandlerBodyFlow> {
        if !matches!(
            self.successful_handler,
            Some(SuccessfulHandlerSpec::PossibleBodies | SuccessfulHandlerSpec::CatchOutputs)
        ) || frame == crate::VariableAliasFrame::Unknown
        {
            return None;
        }
        arguments.exact_argv_len()?;
        let flow = crate::case_bodies::script_body_flow_in_registry(registry, self, arguments);
        if self.arity_accepts_frozen_arguments() != Some(true)
            && !(self.arity_accepts_frozen_arguments().is_none()
                && matches!(flow, crate::script_body_flow::ScriptBodyFlow::CaseBodies(_)))
        {
            return None;
        }
        if !self.arg_roles_complete
            && !matches!(flow, crate::script_body_flow::ScriptBodyFlow::CaseBodies(_))
        {
            return None;
        }
        if matches!(flow, crate::script_body_flow::ScriptBodyFlow::Unknown(_)) {
            return None;
        }
        Some(PossibleHandlerBodyFlow { flow, frame })
    }

    /// Closed normal effects after every coercing operand has materialised.
    /// Dictionary construction retains its values without coercion and only
    /// requires known keys. Other dynamic objects may invoke coercion observers
    /// and retain the ordinary envelope.
    /// This never describes an invocation's error edge or proves worker identity.
    #[must_use]
    pub fn successful_value_leaf_world(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::EffectFootprint> {
        if !matches!(
            self.successful_handler,
            Some(
                SuccessfulHandlerSpec::Leaf
                    | SuccessfulHandlerSpec::ArithmeticSequenceArguments
                    | SuccessfulHandlerSpec::EnsembleLeaf { .. }
                    | SuccessfulHandlerSpec::EnsemblePathLeaf { .. }
                    | SuccessfulHandlerSpec::DictionaryConstructor { .. }
            )
        ) || self
            .successful_handler_effects(arguments, crate::VariableAliasFrame::Unknown)
            .is_none()
        {
            return None;
        }
        for index in 0..arguments.exact_argv_len()? {
            if !matches!(
                self.successful_handler,
                Some(SuccessfulHandlerSpec::DictionaryConstructor { .. })
            ) || index < self.argument_offset
                || (index - self.argument_offset).is_multiple_of(2)
            {
                arguments.literal_at(index)?;
            }
        }
        let mut effects = crate::EffectFootprint::default();
        for access in self.effects.accesses() {
            if access.mode == crate::EffectAccessMode::Read
                && !matches!(
                    self.successful_handler,
                    Some(SuccessfulHandlerSpec::DictionaryConstructor { .. })
                )
            {
                effects.add_access(access.clone());
            }
        }
        effects.add_access(crate::EffectAccess::new(
            crate::WorldStateDomain::InterpreterResult,
            crate::EffectAccessMode::Write,
            crate::InterpreterScope::Current,
            crate::NamespaceScope::Any,
            crate::SubjectScope::Wildcard,
        ));
        Some(effects)
    }

    /// Command-binding contracts admit only their declared table transitions;
    /// callback, body and variable effects are separate semantic purposes.
    fn command_binding_handler_admitted(&self) -> Option<()> {
        let transitions = self.state_transitions.declared()?;
        if transitions
            .facts()
            .iter()
            .any(|fact| match &fact.transition {
                crate::StateTransition::CommandBinding(_) => false,
                crate::StateTransition::Widen(widening) => widening
                    .domains
                    .iter()
                    .any(|domain| *domain != crate::StateTransitionDomain::CommandBindings),
                _ => true,
            })
            || self.arg_roles.iter().any(|(_, role)| {
                role.carries_script()
                    || matches!(role, crate::ArgRole::VarRead | crate::ArgRole::VarWrite)
            })
        {
            return None;
        }
        Some(())
    }

    /// Normal effects require an independently proved converged handler identity.
    /// This projection cannot license compiler entry, body traversal or opcodes.
    #[must_use]
    pub fn successful_handler_effects(
        &self,
        arguments: crate::InvocationArguments<'_>,
        frame: crate::VariableAliasFrame,
    ) -> Option<SuccessfulHandlerEffects> {
        let Some(contract) = self.successful_handler else {
            return self
                .native_compilation?
                .successful_handler_effects(arguments, frame);
        };
        let count = arguments.exact_argv_len()?;
        count.checked_sub(self.argument_offset)?;
        if self.arity_accepts_frozen_arguments() != Some(true) || !self.arg_roles_complete {
            return None;
        }
        match contract {
            SuccessfulHandlerSpec::ArithmeticSequenceArguments => {
                self.closed_arithmetic_sequence_arguments(arguments)?;
            }
            SuccessfulHandlerSpec::Leaf
            | SuccessfulHandlerSpec::EnsembleLeaf { .. }
            | SuccessfulHandlerSpec::EnsemblePathLeaf { .. }
            | SuccessfulHandlerSpec::DictionaryConstructor { .. }
                if self.arg_roles.iter().any(|(_, role)| {
                    role.carries_script()
                        || matches!(
                            role,
                            crate::ArgRole::VarRead
                                | crate::ArgRole::VarWrite
                                | crate::ArgRole::CommandPrefix
                        )
                }) =>
            {
                return None;
            }
            SuccessfulHandlerSpec::CommandBindingTransition => {
                self.command_binding_handler_admitted()?;
            }
            SuccessfulHandlerSpec::VariableOperands
            | SuccessfulHandlerSpec::VariableStringAppend
            | SuccessfulHandlerSpec::ConditionalVariableOperands(_)
            | SuccessfulHandlerSpec::InitialiseEmptyVariable => {
                if frame == crate::VariableAliasFrame::Unknown
                    || self.arg_roles.iter().any(|(_, role)| {
                        matches!(role, crate::ArgRole::Body | crate::ArgRole::Expr)
                    })
                {
                    return None;
                }
                for &(index, role) in &self.arg_roles {
                    if matches!(role, crate::ArgRole::VarRead | crate::ArgRole::VarWrite) {
                        match arguments.get(self.argument_offset + usize::from(index))? {
                            crate::InvocationWord::Literal(_)
                            | crate::InvocationWord::ArrayElementName { .. } => {}
                            _ => return None,
                        }
                    }
                }
            }
            SuccessfulHandlerSpec::CatchOutputs => {
                if frame == crate::VariableAliasFrame::Unknown {
                    return None;
                }
                let dialect = arguments.dialect()?;
                let crate::catch_invocation::CatchInvocationSelection::Valid(selected) =
                    crate::catch_invocation::select_catch_invocation(arguments, dialect)
                else {
                    return None;
                };
                if selected.ignores(0) {
                    return None;
                }
                for index in [selected.result_var_at, selected.options_var_at]
                    .into_iter()
                    .flatten()
                {
                    arguments.literal_at(index)?;
                }
            }
            SuccessfulHandlerSpec::Leaf
            | SuccessfulHandlerSpec::EnsembleLeaf { .. }
            | SuccessfulHandlerSpec::EnsemblePathLeaf { .. }
            | SuccessfulHandlerSpec::DictionaryConstructor { .. } => {
                match self.normal_handler_implementation_lookup(arguments.dialect()) {
                    NormalHandlerImplementationLookup::Unknown => return None,
                    NormalHandlerImplementationLookup::RequiredPath(paths)
                        if paths.select(arguments, self.argument_offset).is_none() =>
                    {
                        return None;
                    }
                    _ => {}
                }
            }
            SuccessfulHandlerSpec::PossibleBodies
            | SuccessfulHandlerSpec::ExpressionArguments
            | SuccessfulHandlerSpec::DictionaryScope
            | SuccessfulHandlerSpec::UserProcedureCall => {
                return None;
            }
        }
        Some(SuccessfulHandlerEffects {
            operation: self.operation,
        })
    }

    fn closed_arithmetic_sequence_arguments(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<()> {
        if self.native_result
            != Some(crate::native_result::NativeResultContract::ArithmeticSequence)
        {
            return None;
        }
        self.normal_list_method_provider(arguments)?;
        let values = (self.argument_offset..arguments.exact_argv_len()?)
            .map(|index| arguments.literal_at(index).map(str::as_bytes))
            .collect::<Option<Vec<_>>>()?;
        tcl_cmd_core::lseq::decode(&values).ok()?;
        Some(())
    }
}

/// Normal-edge effects shared by a stable native handler's generic and
/// compiled implementations. This is not a native operation selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuccessfulHandlerEffects {
    /// Authored normative effect operation. Result object identity and internal
    /// representation remain independent; no compiler entry or error timing
    /// is established by this projection.
    pub operation: SemanticOperationId,
}

impl NativeCompilationSpec {
    /// Prove only successful handler effects when every possible compiler
    /// protocol reaches the same original handler and frozen argv. The caller
    /// must prove that identity convergence independently before using this.
    ///
    /// Body/control compilers may change traversal and exception boundaries;
    /// they are deliberately absent. Variable operations additionally require
    /// the actual physical frame and literal selected name. This never grants
    /// executable lowering, opcode eligibility or successful compilation.
    #[must_use]
    pub fn successful_handler_effects(
        self,
        arguments: crate::InvocationArguments<'_>,
        frame: crate::VariableAliasFrame,
    ) -> Option<SuccessfulHandlerEffects> {
        let count = arguments.exact_argv_len()?;
        match self.grammar {
            NativeCompilationGrammar::VariableLoadStore | NativeCompilationGrammar::Increment
                if matches!(count, 1 | 2)
                    && frame != crate::VariableAliasFrame::Unknown
                    && arguments.literal_at(0).is_some() => {}
            NativeCompilationGrammar::GlobalBindings
                if frame != crate::VariableAliasFrame::Unknown && arguments.are_all_literals() => {}
            NativeCompilationGrammar::Upvar
                if frame != crate::VariableAliasFrame::Unknown
                    && arguments.are_all_literals()
                    && matches!(
                        crate::FrameEffectSpec::UPVAR
                            .successful_layout(arguments)
                            .layout,
                        crate::frame_effect::FrameArgumentResolution::Valid { .. }
                    ) => {}
            NativeCompilationGrammar::ArgumentList => {}
            _ => return None,
        }
        Some(SuccessfulHandlerEffects {
            operation: self.operation,
        })
    }
}

/// Analysis topology of possible caller-frame body entries. It does not prove
/// that the native compiler selected an opcode or entered any body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PossibleBodyTopology {
    /// Reached bodies execute in sequence.
    Sequence(Vec<usize>),
    /// The selected capture consumes ordinary script completions. Native
    /// process termination remains outside the capture boundary.
    Captured(crate::catch_invocation::CatchInvocation),
    /// At most one body is selected.
    Alternatives(Vec<usize>),
    /// Native conditional tests retain their operand and associated body.
    /// A missing test operand is the final unconditional alternative.
    Conditional(Vec<(Option<usize>, usize)>),
    /// Case bodies retain their true list element/argument locations.
    CaseAlternatives(Vec<crate::body_execution::BodyOperand>),
    /// Initial bodies execute once; repeated bodies follow a possible loop edge.
    Loop {
        /// Once-only entry operands.
        initial: Vec<usize>,
        /// Repeated body and update operands.
        repeated: Vec<usize>,
        /// Operands reached after a continue completion.
        continued: Vec<usize>,
    },
}

impl PossibleHandlerBodyFlow {
    /// Project only the authored body topology; unsupported execution protocols
    /// remain absent rather than inheriting a sequence from role order.
    ///
    /// ```
    /// use tcl_registry::native_compilation::{PossibleBodyTopology, PossibleHandlerBodyFlow};
    /// use tcl_registry::{VariableAliasFrame, script_body_flow::ScriptBodyFlow};
    /// let selected = PossibleHandlerBodyFlow {
    ///     flow: ScriptBodyFlow::Conditional(vec![(Some(0), 1), (None, 3)]),
    ///     frame: VariableAliasFrame::Procedure,
    /// };
    /// assert_eq!(selected.topology(), Some(PossibleBodyTopology::Conditional(vec![(Some(0), 1), (None, 3)])));
    /// // This topology supplies possible entries; it grants no compiler opcode.
    /// ```
    #[must_use]
    pub fn topology(&self) -> Option<PossibleBodyTopology> {
        use crate::script_body_flow::ScriptBodyFlow;
        match &self.flow {
            ScriptBodyFlow::Capture(crate::catch_invocation::CatchInvocationSelection::Valid(
                selected,
            )) if selected.ignored_codes == 0 => Some(PossibleBodyTopology::Captured(*selected)),
            ScriptBodyFlow::Sequence(indices) => {
                Some(PossibleBodyTopology::Sequence(indices.clone()))
            }
            ScriptBodyFlow::ConcatenatedScript { argument_offset } => {
                // The source inventory attaches the complete evaluated concat
                // to each contributing operand. Select its first operand once;
                // the topology must not evaluate the constituent scripts again.
                Some(PossibleBodyTopology::Sequence(vec![*argument_offset]))
            }
            ScriptBodyFlow::Alternatives(indices) => {
                Some(PossibleBodyTopology::Alternatives(indices.clone()))
            }
            ScriptBodyFlow::CaseBodies(selection) => Some(PossibleBodyTopology::CaseAlternatives(
                selection.bodies.clone(),
            )),
            ScriptBodyFlow::Conditional(branches) => {
                Some(PossibleBodyTopology::Conditional(branches.clone()))
            }
            ScriptBodyFlow::Loop {
                initial,
                repeated,
                continued,
                ..
            } => Some(PossibleBodyTopology::Loop {
                initial: initial.clone(),
                repeated: repeated.clone(),
                continued: continued.clone(),
            }),
            _ => None,
        }
    }
}
