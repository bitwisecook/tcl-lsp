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

//! Registry-owned transitions for Tcl state whose identity affects analysis.
//!
//! Command bindings and variable cells are not ordinary values: `rename`,
//! aliases, and frame/namespace variable links change what later source names
//! mean. This module records those transitions as target-neutral facts. A
//! descriptor sees structured [`InvocationArguments`] rather than raw source
//! spelling, so computed and expanded words become typed unknown facts and
//! widen the affected state domains conservatively.

use crate::invocation_words::{InvocationArguments, InvocationWordKind};
use crate::side_effects::SideEffectTarget;
use crate::world_effect::{TransitionEffectCoverage, WorldEffectWriteSource, WorldStateDomain};

mod descriptor;
mod namespace_transition;
mod subject_traversal;
mod variable_alias;

pub use descriptor::{
    LogicalSourceStateTransitionResolver, ResolvedStateTransitions, StateTransitionArgumentShape,
    StateTransitionComposition, StateTransitionDescriptor, StateTransitionKnowledge,
    StateTransitionResolver, StateTransitionWideningRule,
};
pub use namespace_transition::{
    EnsembleCommandName, EnsembleDispatchTable, EnsembleDispatchTarget,
    EnsembleManagementOperation, NamespaceTransition, NamespaceTransitionTarget,
};
use subject_traversal::{
    transition_commandbinding_identity, transition_commandbinding_visit,
    transition_interpreter_identity, transition_interpreter_visit, transition_namespace_identity,
    transition_namespace_visit, transition_objectdispatch_identity,
    transition_objectdispatch_visit, transition_package_identity, transition_package_visit,
};
pub use variable_alias::{
    AliasWords, CallerFrameSelection, VARIABLE_ALIAS_DOMAINS, VariableAliasDestination,
    VariableAliasFrame, VariableAliasNamePurpose, VariableAliasTarget, VariableCellAliasTransition,
    alias_pairs_resolver, local_alias_name, logical_source_alias_pairs_resolver,
};

/// A state-domain whose precise identity is invalidated by a dynamic
/// transition operand.
///
/// The vocabulary deliberately includes namespace, interpreter, trace, and
/// object-dispatch state even though the first stamped commands use only a
/// subset. Future registry descriptors can therefore add `TclOO`, namespace,
/// and trace transitions without changing common compiler consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateTransitionDomain {
    /// Command-name bindings in an interpreter.
    CommandBindings,
    /// Tcl variable cells and their aliases.
    VariableCells,
    /// Namespace membership and namespace-qualified lookup.
    Namespaces,
    /// The namespace search configuration used to resolve command names.
    /// Kept distinct from namespace-scoped variable-cell identity so dynamic
    /// `global`, `variable`, and `namespace upvar` operands do not invalidate
    /// an otherwise unchanged command table.
    CommandResolution,
    /// Child-interpreter existence and path-to-interpreter identity.
    InterpreterTopology,
    /// Safe, trusted, hidden-command, limit, and callback policy for one interpreter.
    InterpreterPolicy,
    /// Interpreter paths, visibility, and cross-interpreter bindings.
    Interpreters,
    /// Command rename/delete traces.
    CommandTraces,
    /// Command enter/leave/step execution traces.
    ExecutionTraces,
    /// Variable read/write/unset traces.
    VariableTraces,
    /// Interpreter-local package provisions, loaders and selection policy.
    Packages,
    /// `TclOO` class, object, method, filter, and mixin dispatch state.
    ObjectDispatch,
}

impl StateTransitionDomain {
    /// Every tracked identity domain, for an unlocated invocation whose
    /// callbacks or options have no narrower closed transition description.
    pub const ALL: &'static [Self] = &[
        Self::CommandBindings,
        Self::VariableCells,
        Self::Namespaces,
        Self::CommandResolution,
        Self::InterpreterTopology,
        Self::InterpreterPolicy,
        Self::Interpreters,
        Self::CommandTraces,
        Self::ExecutionTraces,
        Self::VariableTraces,
        Self::Packages,
        Self::ObjectDispatch,
    ];
}

/// A command, variable, namespace, interpreter, or frame-name subject.
///
/// Literal source words retain their exact Tcl value. Non-literal words never
/// expose source spelling as a value; the index is relative to the complete
/// post-head argument list, including a subcommand word where present.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransitionSubject {
    /// An explicitly authored value without invocation-operand provenance.
    Literal(String),
    /// A known value at an exact effective post-head argument ordinal.
    LocatedLiteral {
        /// The literal Tcl value facet, independently of source geometry.
        value: String,
        /// Effective post-head ordinal, including a selected subcommand.
        argument_index: usize,
    },
    /// An independently known native byte value at its effective operand.
    /// Bytes remain values, not source geometry or installed name identity.
    LocatedNativeBytes {
        /// Exact native units; no Unicode projection is required.
        value: Vec<u8>,
        /// Effective post-head ordinal, including a selected subcommand.
        argument_index: usize,
    },
    /// A runtime-computed, expanded, or opaque argument.
    Unknown {
        /// Post-head source-word index.
        argument_index: usize,
        /// Why the subject cannot be known statically.
        word_kind: InvocationWordKind,
    },
}

impl TransitionSubject {
    /// Return the subject at `argument_index`, retaining known or dynamic provenance.
    #[must_use]
    pub fn from_argument(
        arguments: InvocationArguments<'_>,
        argument_index: usize,
    ) -> Option<Self> {
        let word = arguments.get(argument_index)?;
        Some(match word.literal() {
            Some(value) => Self::LocatedLiteral {
                value: value.to_owned(),
                argument_index,
            },
            None => match word.native_bytes() {
                Some(value) => Self::LocatedNativeBytes {
                    value: value.to_vec(),
                    argument_index,
                },
                None => Self::Unknown {
                    argument_index,
                    word_kind: word.kind(),
                },
            },
        })
    }

    /// Return this subject's literal value, if known.
    #[must_use]
    pub fn literal(&self) -> Option<&str> {
        match self {
            Self::Literal(value) | Self::LocatedLiteral { value, .. } => Some(value),
            Self::LocatedNativeBytes { .. } | Self::Unknown { .. } => None,
        }
    }

    /// Borrow the independently retained native byte facet. An authored
    /// logical string is not encoded by this accessor.
    #[must_use]
    pub fn native_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::LocatedNativeBytes { value, .. } => Some(value),
            _ => None,
        }
    }

    /// Exact effective operand ordinal. Authored values have no such receipt.
    #[must_use]
    pub const fn argument_index(&self) -> Option<usize> {
        match self {
            Self::Literal(_) => None,
            Self::LocatedLiteral { argument_index, .. }
            | Self::LocatedNativeBytes { argument_index, .. }
            | Self::Unknown { argument_index, .. } => Some(*argument_index),
        }
    }

    /// Retain an independently selected byte value at the same operand.
    /// Authored text and unknown arguments do not acquire native byte facts.
    #[must_use]
    pub fn with_native_bytes_value(&self, value: Vec<u8>) -> Option<Self> {
        match self {
            Self::LocatedNativeBytes { argument_index, .. } => Some(Self::LocatedNativeBytes {
                value,
                argument_index: *argument_index,
            }),
            _ => None,
        }
    }

    /// Replace a known value facet without changing its operand provenance.
    #[must_use]
    pub fn with_literal_value(&self, value: String) -> Option<Self> {
        match self {
            Self::Literal(_) => Some(Self::Literal(value)),
            Self::LocatedLiteral { argument_index, .. } => Some(Self::LocatedLiteral {
                value,
                argument_index: *argument_index,
            }),
            Self::LocatedNativeBytes { .. } | Self::Unknown { .. } => None,
        }
    }
}

/// Return the namespace qualifier Tcl derives from a command or variable name.
///
/// This is the same byte-oriented operation exposed by `namespace qualifiers`:
/// it splits at the final `::` run without consulting the interpreter's
/// namespace table.  Keeping it here makes registry transition resolvers use
/// one Tcl-compatible spelling rule when a command implicitly creates a
/// namespace for a qualified target.
#[must_use]
pub fn namespace_qualifiers(name: &str) -> &str {
    &name[..namespace_qualifiers_bytes(name.as_bytes()).len()]
}

/// Borrow the namespace qualifier from counted name units. This is spelling
/// geometry only; callers select their own operation's input extent first.
#[must_use]
pub fn namespace_qualifiers_bytes(name: &[u8]) -> &[u8] {
    tcl_syntax::naming::written_separator_extent(name)
        .map_or(&name[..0], |separator| &name[..separator.start])
}

/// Namespace used when a command-prefix alias resolves its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AliasTargetLookup {
    /// Tcl interpreter aliases select the target interpreter's root namespace.
    Global,
    /// Jim command-prefix aliases preserve the caller's active namespace.
    CallerNamespace,
}

/// A transition that changes a Tcl command binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandBindingTransition {
    /// Define a command in the current interpreter.
    Define {
        /// The command name being bound.
        name: TransitionSubject,
        /// The semantic kind of binding installed at that name.
        kind: CommandBindingDefinitionKind,
    },
    /// Move a command binding in the current interpreter.
    Move {
        /// Existing command name.
        from: TransitionSubject,
        /// New command name.
        to: TransitionSubject,
    },
    /// Delete a command binding, optionally in a named interpreter.
    Delete {
        /// Interpreter containing the binding, or the current interpreter
        /// when omitted.
        interpreter: Option<TransitionSubject>,
        /// Command name being removed.
        name: TransitionSubject,
    },
    /// Establish a command alias, potentially across interpreters.
    Alias {
        /// Namespace used for target lookup at invocation.
        target_lookup: AliasTargetLookup,
        /// Source interpreter path.
        source_interpreter: TransitionSubject,
        /// Name the alias receives in the source interpreter.
        alias: TransitionSubject,
        /// Target interpreter path.
        target_interpreter: TransitionSubject,
        /// Command reached by the alias.
        target: TransitionSubject,
        /// Arguments Tcl bakes in ahead of the caller's own.
        ///
        /// `interp alias {} Cat {} Dog extra` makes `Cat x` the call `Dog
        /// extra x`, so an alias with baked arguments is **not** a plain
        /// second name for its target — a consumer resolving a call through
        /// the alias has to know. Empty for the ordinary form.
        arguments: Vec<TransitionSubject>,
    },
    /// One or more command binding operations whose precise kind depends on
    /// dynamic operands.
    Unknown {
        /// Operands that prevented a precise define/move/delete/alias fact.
        operands: Vec<TransitionSubject>,
    },
}

/// The semantic identity installed by a command-binding definition.
///
/// Consumers use this to preserve useful binding information without
/// rediscovering which command performed the definition.  It deliberately
/// describes the resulting binding, rather than the defining command's
/// spelling or syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandBindingDefinitionKind {
    /// An ordinary Tcl command whose more-specific identity is not modelled.
    Command,
    /// A Tcl procedure.
    Procedure,
    /// A `TclOO`, snit, or other registry-described object/class command.
    Object,
}

/// A transition of a child interpreter's lifecycle or policy.
///
/// Interpreter paths are Tcl values rather than source names.  Consequently,
/// every operand is retained as a [`TransitionSubject`], and an unnamed
/// `interp create` has no source-addressable child identity at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterpreterTransition {
    /// Create a child interpreter, optionally under a callable path.
    Create {
        /// The child path when the invocation supplied one.
        interpreter: Option<TransitionSubject>,
        /// Whether the child starts with Tcl's safe-interpreter policy.
        safety: ChildInterpreterSafety,
    },
    /// Delete one child interpreter.
    Delete {
        /// The child path being deleted.
        interpreter: TransitionSubject,
    },
    /// Mark a child interpreter as trusted.
    MarkTrusted {
        /// The child path whose trust policy changes.
        interpreter: TransitionSubject,
    },
    /// Change a child interpreter's recursion limit.
    SetRecursionLimit {
        /// The child path whose limit changes.
        interpreter: TransitionSubject,
        /// The new limit Tcl value.
        limit: TransitionSubject,
    },
    /// Install a child interpreter's background-error command prefix.
    SetBackgroundError {
        /// The child path whose handler changes.
        interpreter: TransitionSubject,
        /// The opaque Tcl command-prefix value stored as the handler.
        handler: TransitionSubject,
    },
    /// Move a visible command into an interpreter's hidden-command table.
    Hide {
        /// The interpreter whose command table changes.
        interpreter: TransitionSubject,
        /// The ordinary command name removed from visible lookup.
        visible: TransitionSubject,
        /// The hidden-table name assigned to the command.
        hidden: TransitionSubject,
    },
    /// Move a hidden command back into ordinary command lookup.
    Expose {
        /// The interpreter whose command table changes.
        interpreter: TransitionSubject,
        /// The hidden-table name removed from hidden lookup.
        hidden: TransitionSubject,
        /// The ordinary command name assigned to the command.
        visible: TransitionSubject,
    },
}

impl InterpreterTransition {
    /// The one original script operand of the canonical created-child command
    /// `HANDLE eval SCRIPT`. A selected Create and successful parent-command
    /// installation remain independent obligations. Abbreviations, expansion,
    /// concatenated scripts and other child operations are not inferred here.
    #[must_use]
    pub fn created_handle_eval_body(
        &self,
        dialect: crate::InvocationDialect,
        arguments: InvocationArguments<'_>,
    ) -> Option<TransitionSubject> {
        self.created_parent_command(dialect)?;
        if arguments.exact_argv_len() != Some(2) || arguments.literal_at(0) != Some("eval") {
            return None;
        }
        TransitionSubject::from_argument(arguments, 1)
    }

    /// Parent-interpreter command installed on a normal native creation edge.
    /// Tcl child paths are lists: a nested path installs no command in the
    /// current parent. The selected handler and successful edge remain caller
    /// obligations; this supplies no child dispatch or compiler-hook proof.
    #[must_use]
    pub fn created_parent_command(&self, dialect: crate::InvocationDialect) -> Option<String> {
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version.is_none()
        {
            return None;
        }
        let Self::Create {
            interpreter: Some(interpreter),
            ..
        } = self
        else {
            return None;
        };
        let elements = tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar)
            .split_list(interpreter.literal()?)
            .ok()?;
        let [name] = elements.as_slice() else {
            return None;
        };
        (!name.is_empty()).then(|| name.to_string())
    }
}

/// The safety policy selected while creating a child interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildInterpreterSafety {
    /// `interp create -safe` creates a safe child.
    Safe,
    /// The child inherits the safety policy of its parent interpreter.
    Inherited,
    /// A computed option word prevents a static policy decision.
    Unknown,
}

/// Whether a `TclOO` definition mutates class-wide or per-object dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectDispatchLayer {
    /// Class configuration inherited by instances and subclasses.
    Class,
    /// Configuration attached only to one object.
    Object,
}

/// The externally visible command identity of a newly-created `TclOO` object.
///
/// Tcl generates names for `new` and for omitted or empty `oo::copy` targets.
/// Those names are real runtime identities, but source analysis cannot invent
/// their spelling.  Keeping that case distinct from a dynamic source operand
/// lets consumers preserve the lifecycle fact without claiming a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectDispatchTarget {
    /// A target supplied as a Tcl invocation operand.
    Named(TransitionSubject),
    /// Tcl chooses a fresh command name at runtime.
    Fresh,
}

/// Identity of the namespace holding one `TclOO` object's private state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectPrivateNamespace {
    /// Tcl generates the private namespace independently of the object's
    /// public command name.
    Fresh,
    /// `createWithNamespace` or `oo::copy` supplied the namespace explicitly.
    Named(TransitionSubject),
}

/// What kind of `TclOO` dispatch entity a factory creates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectDispatchKind {
    /// An ordinary object instance.
    Object,
    /// A class object with both object-side and class-side dispatch state.
    Class,
}

/// A target-neutral `TclOO` dispatch and lifecycle transition.
///
/// Public command bindings are deliberately separate
/// [`CommandBindingTransition`] facts.  `TclOO` object identity survives a
/// command rename, while the command-table name does not, so common analyses
/// must not collapse the two domains into one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectDispatchTransition {
    /// Reconfigure an existing class or object.
    Configure {
        /// Class or object whose method chain changes.
        target: TransitionSubject,
        /// Whether the mutation is inherited class state or per-object state.
        layer: ObjectDispatchLayer,
    },
    /// Create a new object or class.
    Create {
        /// Public object-command identity, named or runtime-generated.
        target: ObjectDispatchTarget,
        /// Namespace that owns the object's private state.
        private_namespace: ObjectPrivateNamespace,
        /// Whether the new entity is an object or a class.
        kind: ObjectDispatchKind,
    },
    /// Copy an existing object or class into a new identity.
    Copy {
        /// Existing object whose dispatch and private state are read.
        source: TransitionSubject,
        /// Public command identity of the copy.
        target: ObjectDispatchTarget,
        /// Namespace that owns the copy's private state.
        private_namespace: ObjectPrivateNamespace,
    },
    /// Destroy an object identity and its private namespace.
    ///
    /// This vocabulary is ready for a registry resolver that can prove a
    /// command head is an object receiver.  Ordinary unknown command heads are
    /// not stamped as destroys merely because their first argument is the
    /// word `destroy`.
    Destroy {
        /// Object being destroyed.
        target: TransitionSubject,
    },
}

/// The target class selected by a trace registration.
///
/// This records Tcl's target semantics without exposing the `trace` command's
/// option spelling to common compiler consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceTarget {
    /// A variable cell and its access-trace registrations.
    Variable(TransitionSubject),
    /// A command binding and its rename/delete-trace registrations.
    Command(TransitionSubject),
    /// A command binding and its enter/leave execution-trace registrations.
    Execution(TransitionSubject),
}

/// One canonical operation accepted in a modern Tcl trace operation list.
///
/// The registry resolver validates the operation against its [`TraceTarget`]
/// before constructing a transition.  The common compiler projection only
/// consumes the target and transition kind, while retaining this information
/// lets registry clients distinguish the installed trace precisely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraceOperation {
    /// Variable array access.
    Array,
    /// Variable read.
    Read,
    /// Variable write.
    Write,
    /// Variable unset.
    Unset,
    /// Command rename.
    Rename,
    /// Command deletion.
    Delete,
    /// Command execution entry.
    Enter,
    /// Command execution exit.
    Leave,
    /// Entry to a command nested in the traced command.
    EnterStep,
    /// Exit from a command nested in the traced command.
    LeaveStep,
}

/// The operation set selected for a trace registration.
///
/// A literal operation list is parsed and canonicalised by the shared trace
/// decoder. A dynamic but non-expanded list still occupies one argv position,
/// so the registration can retain a typed unknown rather than pretending the
/// transition is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceOperationSet {
    /// Canonical, duplicate-free operations in Tcl's fixed reporting order.
    Known(Vec<TraceOperation>),
    /// A one-word operation-list value that cannot be parsed statically.
    Unknown(TransitionSubject),
}

/// A trace registration or removal.
///
/// `prefix` remains a complete Tcl value.  It is neither split as a list nor
/// interpreted as a command here: `trace add` and `trace remove` only install
/// or match it, and do not invoke it themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceTransition {
    /// Register a new trace.  Variable registration also vivifies its target
    /// cell when it was absent.
    Add {
        /// The traced variable or command.
        target: TraceTarget,
        /// Canonical or dynamic operations selected by the Tcl operation list.
        operations: TraceOperationSet,
        /// Opaque command-prefix value stored with the registration.
        prefix: TransitionSubject,
    },
    /// Remove a matching trace registration.  Removing from a missing
    /// variable is a successful no-op.
    Remove {
        /// The traced variable or command.
        target: TraceTarget,
        /// Canonical or dynamic operations selected by the Tcl operation list.
        operations: TraceOperationSet,
        /// Opaque command-prefix value matched against the registration.
        prefix: TransitionSubject,
    },
}

/// An explicit conservative widening caused by a non-literal transition
/// operand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransitionWidening {
    /// State domains whose identity must be treated as unknown.
    pub domains: Vec<StateTransitionDomain>,
    /// The argument that forced widening.
    pub subject: TransitionSubject,
}

/// One target-neutral transition established by an invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateTransition {
    /// A command binding is defined, moved, deleted, or aliased.
    CommandBinding(CommandBindingTransition),
    /// A child interpreter's lifecycle, policy, or hidden-command state.
    Interpreter(InterpreterTransition),
    /// A current-frame local aliases another variable cell.
    VariableCellAlias(VariableCellAliasTransition),
    /// Namespace lookup, imports, handlers, or membership changes.
    Namespace(NamespaceTransition),
    /// Variable, command, or execution trace registration state changes.
    Trace(TraceTransition),
    /// `TclOO` object lifecycle or dispatch configuration changes.
    ObjectDispatch(ObjectDispatchTransition),
    /// Interpreter-local package bookkeeping or loader execution.
    Package(crate::model::binding::PackageTransition),
    /// A dynamic transition operand widens named state domains.
    Widen(StateTransitionWidening),
}

/// How a transition fact crosses a Tcl completion edge.
///
/// A future executable control-flow graph applies every fact on the ordinary
/// `OK` edge. On an abrupt edge, [`Self::OnOkOnly`] leaves the incoming state
/// unchanged, while [`Self::MayCommitBeforeAbruptCompletion`] joins it with
/// the state after the transition. The latter covers pair-wise mutation and
/// re-entrant callbacks, where Tcl can expose an intermediate state before an
/// error, return, break, or custom completion leaves the invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateTransitionCommit {
    /// Tcl establishes this transition only if the invocation completes with
    /// `OK`.
    OnOkOnly,
    /// Tcl may establish this transition before an abrupt completion.
    MayCommitBeforeAbruptCompletion,
}

/// The conservative transfer required on an abrupt completion edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AbruptTransitionTransfer {
    /// Preserve the incoming state without this transition.
    Unchanged,
    /// Join the incoming state with the state after this transition.
    JoinWithTransition,
}

impl StateTransitionCommit {
    /// Return the transfer a future CFG must use for an abrupt edge.
    #[must_use]
    pub const fn abrupt_transfer(self) -> AbruptTransitionTransfer {
        match self {
            Self::OnOkOnly => AbruptTransitionTransfer::Unchanged,
            Self::MayCommitBeforeAbruptCompletion => AbruptTransitionTransfer::JoinWithTransition,
        }
    }
}

/// One transition fact with its completion-edge applicability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransitionFact {
    /// The state change described by this fact.
    pub transition: StateTransition,
    /// When the state change may be observable.
    pub commit: StateTransitionCommit,
}

/// Owned transition facts resolved from one invocation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateTransitions {
    facts: Vec<StateTransitionFact>,
}

/// Aggregate effect of resolved state transitions on command lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResolutionImpact {
    /// No transition changes which command an invocation can reach.
    None,
    /// Exact command-binding transitions enumerate the affected identities.
    ExplicitBindings,
    /// Namespace lookup or an unbounded binding transition prevents a finite
    /// command-resolution projection.
    Unbounded,
}

impl StateTransitions {
    /// Project every effective operand ordinal through one argv mapping.
    ///
    /// Nested alias/frame/namespace/trace/package subjects use the same map.
    /// Values, kinds, execution order and completion commits remain exact.
    /// An unmappable operand withdraws the whole projection; no partial fact
    /// list or authored value is substituted for its missing provenance.
    #[must_use]
    pub fn project_argument_indices(
        &self,
        mut project: impl FnMut(usize) -> Option<usize>,
    ) -> Option<Self> {
        let mut projected = self.clone();
        let mut complete = true;
        for fact in &mut projected.facts {
            fact.transition.for_each_subject_mut(&mut |subject| {
                let index = match subject {
                    TransitionSubject::Literal(_) => return,
                    TransitionSubject::LocatedLiteral { argument_index, .. }
                    | TransitionSubject::LocatedNativeBytes { argument_index, .. }
                    | TransitionSubject::Unknown { argument_index, .. } => argument_index,
                };
                if let Some(mapped) = project(*index) {
                    *index = mapped;
                } else {
                    complete = false;
                }
            });
        }
        complete.then_some(projected)
    }

    /// Retain an unresolved whole-invocation identity effect. The authored
    /// wildcard subject has no operand ordinal or original name authority.
    #[must_use]
    pub fn unknown_invocation() -> Self {
        let mut transitions = Self::default();
        transitions.push(StateTransition::Widen(StateTransitionWidening {
            domains: StateTransitionDomain::ALL.to_vec(),
            subject: TransitionSubject::Literal(String::new()),
        }));
        transitions
    }

    /// Return transition facts in Tcl execution order.
    #[must_use]
    pub fn facts(&self) -> &[StateTransitionFact] {
        &self.facts
    }

    /// Add one resolved transition.
    pub fn push(&mut self, transition: StateTransition) {
        self.facts.push(StateTransitionFact {
            transition,
            commit: StateTransitionCommit::OnOkOnly,
        });
    }

    /// Append transitions from a less-specific or sibling descriptor.
    pub fn extend(&mut self, other: Self) {
        self.facts.extend(other.facts);
    }

    /// The command-binding facts, in Tcl execution order — the one
    /// vocabulary for "this call changed the command table" (ledger C8).
    pub fn command_bindings(&self) -> impl Iterator<Item = &CommandBindingTransition> {
        self.facts.iter().filter_map(|fact| match &fact.transition {
            StateTransition::CommandBinding(binding) => Some(binding),
            _ => None,
        })
    }

    /// Whether a dynamic operand widened `domain` — the precise identity in
    /// that domain is unknown after this invocation.
    #[must_use]
    pub fn widens(&self, domain: StateTransitionDomain) -> bool {
        self.facts.iter().any(|fact| {
            matches!(&fact.transition, StateTransition::Widen(widening)
                if widening.domains.contains(&domain))
        })
    }

    /// Whether this invocation changes command-name bindings at all, precisely
    /// or conservatively.
    ///
    /// A consumer that only gates ("may this call have moved a binding?")
    /// asks this rather than re-deriving it from the fact list, so a
    /// dynamic operand that produced only a widening still counts.
    #[must_use]
    pub fn touches_command_bindings(&self) -> bool {
        self.command_bindings().next().is_some()
            || self.widens(StateTransitionDomain::CommandBindings)
    }

    /// Project every registry transition onto command-resolution stability.
    ///
    /// This is the common owner for flow-sensitive and whole-module binding
    /// consumers; neither needs its own namespace-variant allow-list.
    #[must_use]
    pub fn command_resolution_impact(&self) -> CommandResolutionImpact {
        let mut explicit = false;
        for fact in &self.facts {
            match &fact.transition {
                StateTransition::CommandBinding(CommandBindingTransition::Unknown { .. })
                | StateTransition::Package(
                    crate::model::binding::PackageTransition::Require { .. }
                    | crate::model::binding::PackageTransition::IndexedAutoload { .. }
                    | crate::model::binding::PackageTransition::SourceLoad { .. },
                ) => {
                    return CommandResolutionImpact::Unbounded;
                }
                StateTransition::CommandBinding(_) => explicit = true,
                StateTransition::Namespace(transition)
                    if transition.affects_command_resolution() =>
                {
                    return CommandResolutionImpact::Unbounded;
                }
                StateTransition::Widen(widening)
                    if widening.domains.iter().any(|domain| {
                        matches!(
                            domain,
                            StateTransitionDomain::CommandBindings
                                | StateTransitionDomain::CommandResolution
                        )
                    }) =>
                {
                    return CommandResolutionImpact::Unbounded;
                }
                StateTransition::Package(_)
                | StateTransition::Interpreter(_)
                | StateTransition::VariableCellAlias(_)
                | StateTransition::Namespace(_)
                | StateTransition::Trace(_)
                | StateTransition::ObjectDispatch(_)
                | StateTransition::Widen(_) => {}
            }
        }
        if explicit {
            CommandResolutionImpact::ExplicitBindings
        } else {
            CommandResolutionImpact::None
        }
    }

    fn set_commit(&mut self, commit: StateTransitionCommit) {
        for fact in &mut self.facts {
            fact.commit = commit;
        }
    }

    /// Widen `domains` for `subject`, merged into the widening this list
    /// already holds for that subject — the one way an abstention is
    /// recorded, so a resolver's own and a descriptor's rules never repeat
    /// one subject.
    pub(crate) fn widen(&mut self, subject: TransitionSubject, domains: &[StateTransitionDomain]) {
        if domains.is_empty() {
            return;
        }
        if let Some(StateTransitionFact {
            transition: StateTransition::Widen(existing),
            ..
        }) = self.facts.iter_mut().find(
            |fact| matches!(&fact.transition, StateTransition::Widen(widening) if widening.subject == subject),
        ) {
            existing.domains.extend_from_slice(domains);
            existing.domains.sort_unstable();
            existing.domains.dedup();
            return;
        }
        let mut domains = domains.to_vec();
        domains.sort_unstable();
        domains.dedup();
        self.push(StateTransition::Widen(StateTransitionWidening {
            domains,
            subject,
        }));
    }

    fn widen_dynamic_arguments(
        &mut self,
        arguments: InvocationArguments<'_>,
        rules: &[StateTransitionWideningRule],
        positional_shape: bool,
    ) {
        for argument_index in 0..arguments.len() {
            // A literal word widens nothing: skip it before building (and
            // copying) its subject (#2422).
            let Some(word) = arguments.get(argument_index) else {
                continue;
            };
            if word.literal().is_some() {
                continue;
            }
            let Some(subject) = TransitionSubject::from_argument(arguments, argument_index) else {
                continue;
            };
            if word.native_bytes().is_some()
                && self
                    .facts
                    .iter()
                    .any(|fact| fact.transition.represents_native_identity_subject(&subject))
            {
                continue;
            }
            let mut domains: Vec<StateTransitionDomain> = rules
                .iter()
                .filter(|rule| rule.operands.includes(argument_index))
                .flat_map(|rule| rule.domains.iter().copied())
                .collect();
            if positional_shape && !word.has_exactly_one_argv_entry() {
                domains.extend(rules.iter().flat_map(|rule| rule.domains.iter().copied()));
            }
            self.widen(subject, &domains);
        }
    }
}

impl StateTransition {
    // Known native units replace a dynamic identity obligation only where a
    // selected direct operation represents that same positioned identity.
    // Flags, frame levels, versions and unresolved operation operands do not.
    fn represents_native_identity_subject(&self, subject: &TransitionSubject) -> bool {
        match self {
            Self::CommandBinding(transition) => {
                transition_commandbinding_identity(transition, subject)
            }
            Self::Interpreter(transition) => transition_interpreter_identity(transition, subject),
            Self::VariableCellAlias(alias) => {
                alias.local == *subject
                    || match &alias.target {
                        VariableAliasTarget::Global { variable }
                        | VariableAliasTarget::CurrentNamespace { variable }
                        | VariableAliasTarget::CallerSelectedFrame { variable, .. } => {
                            variable == subject
                        }
                        VariableAliasTarget::Namespace {
                            namespace,
                            variable,
                        } => namespace == subject || variable == subject,
                    }
            }
            Self::Namespace(transition) => transition_namespace_identity(transition, subject),
            Self::Trace(
                TraceTransition::Add { target, prefix, .. }
                | TraceTransition::Remove { target, prefix, .. },
            ) => {
                prefix == subject
                    || match target {
                        TraceTarget::Variable(name)
                        | TraceTarget::Command(name)
                        | TraceTarget::Execution(name) => name == subject,
                    }
            }
            Self::ObjectDispatch(transition) => {
                transition_objectdispatch_identity(transition, subject)
            }
            Self::Package(transition) => transition_package_identity(transition, subject),
            Self::Widen(_) => false,
        }
    }

    fn for_each_subject_mut(&mut self, visit: &mut impl FnMut(&mut TransitionSubject)) {
        match self {
            Self::CommandBinding(transition) => transition_commandbinding_visit(transition, visit),
            Self::Interpreter(transition) => transition_interpreter_visit(transition, visit),
            Self::VariableCellAlias(alias) => {
                visit(&mut alias.local);
                match &mut alias.target {
                    VariableAliasTarget::Global { variable }
                    | VariableAliasTarget::CurrentNamespace { variable } => visit(variable),
                    VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
                        if let CallerFrameSelection::Explicit(subject) = frame {
                            visit(subject);
                        }
                        visit(variable);
                    }
                    VariableAliasTarget::Namespace {
                        namespace,
                        variable,
                    } => {
                        visit(namespace);
                        visit(variable);
                    }
                }
            }
            Self::Namespace(transition) => transition_namespace_visit(transition, visit),
            Self::Trace(
                TraceTransition::Add {
                    target,
                    operations,
                    prefix,
                }
                | TraceTransition::Remove {
                    target,
                    operations,
                    prefix,
                },
            ) => {
                match target {
                    TraceTarget::Variable(subject)
                    | TraceTarget::Command(subject)
                    | TraceTarget::Execution(subject) => visit(subject),
                }
                if let TraceOperationSet::Unknown(subject) = operations {
                    visit(subject);
                }
                visit(prefix);
            }
            Self::ObjectDispatch(transition) => transition_objectdispatch_visit(transition, visit),
            Self::Package(transition) => transition_package_visit(transition, visit),
            Self::Widen(widening) => visit(&mut widening.subject),
        }
    }
}

/// Which post-head source words carry transition identities for one
/// descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateTransitionOperandLayout {
    /// A fixed set of post-head positions, including a subcommand word when
    /// the descriptor is attached to a subcommand.
    Indices(&'static [u8]),
    /// Every post-head word is an identity-bearing operand.
    EveryArgument,
    /// Every `stride`th word starting at `first` is an identity-bearing
    /// operand.
    Strided {
        /// First identity-bearing post-head position.
        first: u8,
        /// Distance between identity-bearing operands.
        stride: u8,
    },
}

impl StateTransitionOperandLayout {
    const fn includes(self, index: usize) -> bool {
        match self {
            Self::Indices(indices) => {
                let mut offset = 0;
                while offset < indices.len() {
                    if indices[offset] as usize == index {
                        return true;
                    }
                    offset += 1;
                }
                false
            }
            Self::EveryArgument => true,
            Self::Strided { first, stride } => {
                stride != 0
                    && index >= first as usize
                    && (index - first as usize).is_multiple_of(stride as usize)
            }
        }
    }
}

/// The **stock command-binding transition descriptors** — one per
/// [`crate::CommandTableEffect`] shape (centralisation ledger C8, redesign
/// §11.2 D9).
///
/// Before this module there were three vocabularies for "this command
/// changed the command table": these transitions, the coarse
/// `CommandTableEffect` a consumer dispatched on, and the argument
/// destructuring each consumer then repeated for itself. The destructuring
/// is here now, once, and it is the **only** producer: a shipped spec
/// names one of these descriptors, and a `SpecTcl` pack that can only
/// write the `command_table_effect` shorthand (a pack cannot supply a Rust
/// resolver) gets the very same descriptor through
/// [`crate::CommandTableEffect::transitions`]. One declaration, one
/// resolver, one fact vocabulary for every consumer.
pub mod command_binding {
    use super::{
        AliasTargetLookup, CommandBindingDefinitionKind, CommandBindingTransition,
        InvocationArguments, NamespaceTransition, NamespaceTransitionTarget, SideEffectTarget,
        StateTransition, StateTransitionArgumentShape, StateTransitionCommit,
        StateTransitionComposition, StateTransitionDescriptor, StateTransitionDomain,
        StateTransitionOperandLayout, StateTransitionWideningRule, StateTransitions,
        TransitionEffectCoverage, TransitionSubject, WorldEffectWriteSource, WorldStateDomain,
    };

    /// The identity domains a command-table mutation can invalidate. A
    /// binding move can create a namespace and fires command traces, so a
    /// dynamic operand widens all three.
    const BINDING_DOMAINS: &[StateTransitionDomain] = &[
        StateTransitionDomain::CommandBindings,
        StateTransitionDomain::Namespaces,
        StateTransitionDomain::CommandTraces,
    ];

    /// [`BINDING_DOMAINS`] plus the interpreter axis an alias crosses.
    const ALIAS_DOMAINS: &[StateTransitionDomain] = &[
        StateTransitionDomain::CommandBindings,
        StateTransitionDomain::Namespaces,
        StateTransitionDomain::Interpreters,
        StateTransitionDomain::CommandTraces,
    ];

    /// The transition is authoritative for the command-table write the
    /// legacy bridges report — both the command-table descriptor's and a
    /// `ProcDefinition` side effect's, for a definer that declares one.
    const DEFINING_COVERAGE: &[TransitionEffectCoverage] = &[
        TransitionEffectCoverage {
            source: WorldEffectWriteSource::LegacyCommandTable,
            domains: &[WorldStateDomain::CommandBindings],
        },
        TransitionEffectCoverage {
            source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::ProcDefinition),
            domains: &[WorldStateDomain::CommandBindings],
        },
    ];

    /// An alias declares no `ProcDefinition` side effect, so only the
    /// command-table bridge is covered.
    const ALIAS_COVERAGE: &[TransitionEffectCoverage] = &[TransitionEffectCoverage {
        source: WorldEffectWriteSource::LegacyCommandTable,
        domains: &[WorldStateDomain::CommandBindings],
    }];

    /// `proc name params body` — bind argument 0 as a procedure.
    pub const DEFINES_PROCEDURE: StateTransitionDescriptor = StateTransitionDescriptor {
        logical_source_resolver: None,
        composition: StateTransitionComposition::Extend,
        success_resolver: None,
        resolver: Some(defines_procedure),
        argument_shape: StateTransitionArgumentShape::Positional,
        dynamic_widening: &[StateTransitionWideningRule {
            operands: StateTransitionOperandLayout::Indices(&[0]),
            domains: BINDING_DOMAINS,
        }],
        effect_coverage: DEFINING_COVERAGE,
        commit: StateTransitionCommit::OnOkOnly,
    };

    /// `rename oldName newName` — move argument 0 to argument 1, or delete
    /// it when the target is empty.
    pub const RENAMES_COMMANDS: StateTransitionDescriptor = StateTransitionDescriptor {
        logical_source_resolver: None,
        composition: StateTransitionComposition::Extend,
        success_resolver: None,
        resolver: Some(renames_commands),
        argument_shape: StateTransitionArgumentShape::Positional,
        dynamic_widening: &[StateTransitionWideningRule {
            operands: StateTransitionOperandLayout::Indices(&[0, 1]),
            domains: BINDING_DOMAINS,
        }],
        effect_coverage: DEFINING_COVERAGE,
        // Keep the transition visible on an abrupt edge.  It is deliberately
        // conservative for command traces and re-entrant callbacks, which can
        // observe the command-table change while Tcl is completing the
        // operation.
        commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
    };

    /// `interp alias srcPath srcCmd ?targetPath targetCmd ?args…??` —
    /// create, delete, or query a command alias.
    pub const CREATES_ALIASES: StateTransitionDescriptor = StateTransitionDescriptor {
        logical_source_resolver: None,
        composition: StateTransitionComposition::Extend,
        success_resolver: None,
        resolver: Some(creates_aliases),
        argument_shape: StateTransitionArgumentShape::Positional,
        dynamic_widening: &[StateTransitionWideningRule {
            // The subcommand at index 0 is already known to have selected
            // this descriptor. The source/target interpreter and command
            // positions carry the transition's identity; baked alias
            // arguments do not.
            operands: StateTransitionOperandLayout::Indices(&[1, 2, 3, 4]),
            domains: ALIAS_DOMAINS,
        }],
        effect_coverage: ALIAS_COVERAGE,
        // Alias setup can cross interpreter boundaries and invoke observable
        // lifecycle hooks before reporting an error.
        commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
    };

    /// Jim's `alias newname command ?args?` preserves the caller namespace.
    pub const CREATES_CALLER_ALIASES: StateTransitionDescriptor = StateTransitionDescriptor {
        logical_source_resolver: None,
        composition: StateTransitionComposition::Extend,
        success_resolver: None,
        resolver: Some(creates_caller_aliases),
        argument_shape: StateTransitionArgumentShape::Positional,
        dynamic_widening: &[StateTransitionWideningRule {
            operands: StateTransitionOperandLayout::Indices(&[0, 1]),
            domains: ALIAS_DOMAINS,
        }],
        effect_coverage: ALIAS_COVERAGE,
        commit: StateTransitionCommit::OnOkOnly,
    };

    fn creates_caller_aliases(arguments: InvocationArguments<'_>) -> StateTransitions {
        let mut transitions = StateTransitions::default();
        if arguments.len() < 2 {
            return transitions;
        }
        let (Some(alias), Some(target)) = (
            TransitionSubject::from_argument(arguments, 0),
            TransitionSubject::from_argument(arguments, 1),
        ) else {
            return transitions;
        };
        transitions.push(StateTransition::CommandBinding(
            CommandBindingTransition::Alias {
                target_lookup: AliasTargetLookup::CallerNamespace,
                source_interpreter: TransitionSubject::Literal(String::new()),
                alias,
                target_interpreter: TransitionSubject::Literal(String::new()),
                target,
                arguments: (2..arguments.len())
                    .filter_map(|index| TransitionSubject::from_argument(arguments, index))
                    .collect(),
            },
        ));
        transitions
    }

    fn defines_procedure(arguments: InvocationArguments<'_>) -> StateTransitions {
        let mut transitions = StateTransitions::default();
        if let Some(name) = TransitionSubject::from_argument(arguments, 0) {
            transitions.push(StateTransition::CommandBinding(
                CommandBindingTransition::Define {
                    name,
                    kind: CommandBindingDefinitionKind::Procedure,
                },
            ));
        }
        transitions
    }

    fn renames_commands(arguments: InvocationArguments<'_>) -> StateTransitions {
        let mut transitions = StateTransitions::default();
        // `rename` takes exactly two arguments in every release and dialect
        // that has it; any other arity is a `wrong # args` error that moves
        // nothing, so a malformed call states nothing rather than half a
        // fact.
        if arguments.len() != 2 {
            return transitions;
        }
        let (Some(from), Some(to)) = (
            TransitionSubject::from_argument(arguments, 0),
            TransitionSubject::from_argument(arguments, 1),
        ) else {
            return transitions;
        };
        let destination = if let Some(target) = to.literal() {
            let namespace = super::namespace_qualifiers(target);
            Some((
                target.is_empty(),
                (!namespace.is_empty()).then(|| {
                    to.with_literal_value(namespace.to_owned())
                        .expect("known rename destination")
                }),
            ))
        } else {
            to.native_bytes()
                .zip(
                    arguments
                        .dialect()
                        .and_then(crate::InvocationDialect::native_name_protocol)
                        .filter(|protocol| {
                            matches!(protocol, tcl_syntax::naming::NativeNameProtocol::C(_))
                        }),
                )
                .and_then(|(target, protocol)| {
                    let selected = protocol
                        .rename_destination_input(
                            tcl_syntax::naming::NativeNameContext::root(),
                            target,
                        )
                        .ok()?;
                    let namespace = super::namespace_qualifiers_bytes(selected.selected());
                    Some((
                        selected.selected().is_empty(),
                        (!namespace.is_empty()).then(|| {
                            to.with_native_bytes_value(namespace.to_vec())
                                .expect("known native rename destination")
                        }),
                    ))
                })
        };
        match destination {
            Some((true, _)) => transitions.push(StateTransition::CommandBinding(
                CommandBindingTransition::Delete {
                    interpreter: None,
                    name: from,
                },
            )),
            Some((false, namespace)) => {
                // Namespace geometry is independent of whether this operation
                // can create that lineage or complete normally.
                if let Some(namespace) = namespace {
                    transitions.push(StateTransition::Namespace(NamespaceTransition::Ensure {
                        namespace: NamespaceTransitionTarget::Named(namespace),
                    }));
                }
                transitions.push(StateTransition::CommandBinding(
                    CommandBindingTransition::Move { from, to },
                ));
            }
            None => transitions.push(StateTransition::CommandBinding(
                CommandBindingTransition::Unknown {
                    operands: vec![from, to],
                },
            )),
        }
        transitions
    }

    /// Whether this invocation is `interp alias`-shaped — a literal
    /// `alias` subcommand word at position 0.
    ///
    /// The shipped registry stamps the alias effect on `interp`'s `alias`
    /// subcommand, so the word is guaranteed there. A `SpecTcl` pack may
    /// stamp the same effect at *command* level on something that only
    /// builds an alias internally (`struct::tree` and `struct::graph` in the
    /// tcllib draft under `docs/design/spec-dsl-examples/external/` do
    /// exactly that), and such a call carries none of `interp alias`'s
    /// words. Reading them as if it did would invent an alias out of the
    /// command's own arguments. Registry data must not be able to do that,
    /// so the shape check is a **fact**: a call that is not `interp
    /// alias`-shaped states no alias.
    fn is_alias_subcommand_shape(arguments: InvocationArguments<'_>) -> bool {
        arguments.literal_at(0) == Some("alias")
    }

    fn creates_aliases(arguments: InvocationArguments<'_>) -> StateTransitions {
        let mut transitions = StateTransitions::default();
        if !is_alias_subcommand_shape(arguments) {
            return transitions;
        }
        let (Some(source_interpreter), Some(alias)) = (
            TransitionSubject::from_argument(arguments, 1),
            TransitionSubject::from_argument(arguments, 2),
        ) else {
            return transitions;
        };

        match arguments.len() {
            // `interp alias sourcePath sourceCmd` queries an existing alias.
            0..=3 => {}
            // `interp alias sourcePath sourceCmd {}` deletes it. A computed
            // target-path position could be empty at runtime, so retain a
            // typed wildcard transition rather than pretending this is alias
            // creation.
            // A brace-quoted empty word's Tcl **value** is the empty
            // string, but not every source-word reconstruction strips the
            // braces before handing the registry a literal, so both
            // spellings mean "delete" here.
            4 => match arguments.literal_at(3) {
                Some("" | "{}") => transitions.push(StateTransition::CommandBinding(
                    CommandBindingTransition::Delete {
                        interpreter: Some(source_interpreter),
                        name: alias,
                    },
                )),
                Some(_) => {}
                None => {
                    let Some(target_path) = TransitionSubject::from_argument(arguments, 3) else {
                        return transitions;
                    };
                    transitions.push(StateTransition::CommandBinding(
                        CommandBindingTransition::Unknown {
                            operands: vec![source_interpreter, alias, target_path],
                        },
                    ));
                }
            },
            // The create form requires both target interpreter path and
            // command.
            _ => {
                let (Some(target_interpreter), Some(target)) = (
                    TransitionSubject::from_argument(arguments, 3),
                    TransitionSubject::from_argument(arguments, 4),
                ) else {
                    return transitions;
                };
                let arguments = (5..arguments.len())
                    .filter_map(|index| TransitionSubject::from_argument(arguments, index))
                    .collect();
                transitions.push(StateTransition::CommandBinding(
                    CommandBindingTransition::Alias {
                        target_lookup: AliasTargetLookup::Global,
                        source_interpreter,
                        alias,
                        target_interpreter,
                        target,
                        arguments,
                    },
                ));
            }
        }
        transitions
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_namespace_option_advice_tests;

#[cfg(test)]
mod created_handle_source_tests;
