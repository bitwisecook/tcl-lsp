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
use crate::world_effect::{TransitionEffectCoverage, TransitionEffectCoverages};

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
    /// `TclOO` class, object, method, filter, and mixin dispatch state.
    ObjectDispatch,
}

/// A command, variable, namespace, interpreter, or frame-name subject.
///
/// Literal source words retain their exact Tcl value. Non-literal words never
/// expose source spelling as a value; the index is relative to the complete
/// post-head argument list, including a subcommand word where present.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransitionSubject {
    /// A known literal Tcl value.
    Literal(String),
    /// A runtime-computed, expanded, or opaque argument.
    Unknown {
        /// Post-head source-word index.
        argument_index: usize,
        /// Why the subject cannot be known statically.
        word_kind: InvocationWordKind,
    },
}

impl TransitionSubject {
    /// Return the subject at `argument_index`, retaining dynamic provenance.
    #[must_use]
    pub fn from_argument(
        arguments: InvocationArguments<'_>,
        argument_index: usize,
    ) -> Option<Self> {
        let word = arguments.get(argument_index)?;
        Some(match word.literal() {
            Some(value) => Self::Literal(value.to_owned()),
            None => Self::Unknown {
                argument_index,
                word_kind: word.kind(),
            },
        })
    }

    /// Return this subject's literal value, if known.
    #[must_use]
    pub fn literal(&self) -> Option<&str> {
        match self {
            Self::Literal(value) => Some(value),
            Self::Unknown { .. } => None,
        }
    }
}

/// A transition that changes a Tcl command binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandBindingTransition {
    /// Define a command in the current interpreter.
    Define {
        /// The command name being bound.
        name: TransitionSubject,
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
        /// Source interpreter path.
        source_interpreter: TransitionSubject,
        /// Name the alias receives in the source interpreter.
        alias: TransitionSubject,
        /// Target interpreter path.
        target_interpreter: TransitionSubject,
        /// Command reached by the alias.
        target: TransitionSubject,
    },
    /// One or more command binding operations whose precise kind depends on
    /// dynamic operands.
    Unknown {
        /// Operands that prevented a precise define/move/delete/alias fact.
        operands: Vec<TransitionSubject>,
    },
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

/// Which caller-frame cell an `upvar`-style transition addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallerFrameSelection {
    /// Tcl's implicit level: the immediate caller frame.
    DefaultCaller,
    /// An explicit level word selects the frame at runtime.
    Explicit(TransitionSubject),
}

/// The target cell of a local variable alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableAliasTarget {
    /// A variable in the global namespace.
    Global {
        /// Global variable name.
        variable: TransitionSubject,
    },
    /// A variable in the current namespace.
    CurrentNamespace {
        /// Namespace-relative or qualified variable name.
        variable: TransitionSubject,
    },
    /// A variable in a frame selected by an `upvar` level word.
    CallerSelectedFrame {
        /// The selected caller frame.
        frame: CallerFrameSelection,
        /// Variable name in that frame.
        variable: TransitionSubject,
    },
    /// A variable in an explicitly named namespace.
    Namespace {
        /// Namespace containing the target cell.
        namespace: TransitionSubject,
        /// Variable name within that namespace.
        variable: TransitionSubject,
    },
}

/// A local variable cell bound to another Tcl cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableCellAliasTransition {
    /// The current-frame local variable name.
    pub local: TransitionSubject,
    /// The cell reached through `local`.
    pub target: VariableAliasTarget,
}

/// The namespace selected by a namespace-state transition.
///
/// The current namespace is intentionally distinct from a literal namespace
/// word.  Keeping that distinction in the registry lets target-neutral
/// consumers retain the Tcl execution context without treating the spelling
/// of an unqualified word as its fully-qualified runtime value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceTransitionTarget {
    /// The namespace current at the invocation site.
    Current,
    /// A namespace selected by a command operand.
    Named(TransitionSubject),
}

/// A mutation of namespace-owned state.
///
/// These are deliberately semantic operations, rather than command names or
/// option spellings.  Registry resolvers select the applicable variant; the
/// compiler only projects the resulting typed state operation.  Pattern and
/// list operands remain whole Tcl values, never host-language token streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceTransition {
    /// Ensure a namespace exists, creating missing ancestors as Tcl does for
    /// `namespace eval`.
    Ensure {
        /// Namespace that must exist before the evaluated script begins.
        namespace: NamespaceTransitionTarget,
    },
    /// Delete a namespace and every namespace, command, variable, and
    /// namespace-local handler below it.
    Delete {
        /// Root of the recursively deleted namespace tree.
        namespace: NamespaceTransitionTarget,
    },
    /// Change the exported-command pattern list of a namespace.
    Export {
        /// Namespace whose export list changes.
        namespace: NamespaceTransitionTarget,
        /// Export patterns or the `-clear` control word, as Tcl values.
        patterns: Vec<TransitionSubject>,
    },
    /// Install command imports into a namespace.
    Import {
        /// Namespace receiving imported command bindings.
        namespace: NamespaceTransitionTarget,
        /// Import patterns, retained as whole Tcl values.
        patterns: Vec<TransitionSubject>,
    },
    /// Remove imported command bindings from a namespace.
    Forget {
        /// Namespace losing imported command bindings.
        namespace: NamespaceTransitionTarget,
        /// Import patterns, retained as whole Tcl values.
        patterns: Vec<TransitionSubject>,
    },
    /// Change the command-resolution path of a namespace.
    SetPath {
        /// Namespace whose path changes.
        namespace: NamespaceTransitionTarget,
        /// Tcl list value containing the path entries.
        path: TransitionSubject,
    },
    /// Change the namespace-local fallback command handler.
    SetUnknown {
        /// Namespace whose handler changes.
        namespace: NamespaceTransitionTarget,
        /// Command-prefix Tcl value installed as the handler.
        handler: TransitionSubject,
    },
    /// Create or reconfigure an ensemble dispatch surface in a namespace.
    Ensemble {
        /// Namespace owning the ensemble dispatch state.
        namespace: NamespaceTransitionTarget,
    },
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

impl StateTransitions {
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

    fn set_commit(&mut self, commit: StateTransitionCommit) {
        for fact in &mut self.facts {
            fact.commit = commit;
        }
    }

    fn widen(&mut self, subject: TransitionSubject, domains: &[StateTransitionDomain]) {
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
            let Some(subject) = TransitionSubject::from_argument(arguments, argument_index) else {
                continue;
            };
            let Some(word) = arguments.get(argument_index) else {
                continue;
            };
            if word.literal().is_some() {
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

/// Whether a registry invocation has a closed state-transition declaration.
///
/// An unstamped invocation is a wildcard over every [`StateTransitionDomain`]
/// and is not equivalent to an empty fact list. Consumers must widen their
/// tracked command, cell, namespace, interpreter, trace, and object state for
/// [`Self::UnknownInvocation`]. Only [`Self::Declared`] — including an
/// explicit [`StateTransitionDescriptor::EMPTY`] — permits a consumer to use
/// the contained fact list as a closed registry statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateTransitionKnowledge {
    /// No descriptor was declared; the generic Tcl invocation can affect any
    /// tracked identity domain.
    UnknownInvocation,
    /// A registry descriptor supplied the complete transition fact list.
    Declared(StateTransitions),
}

impl StateTransitionKnowledge {
    /// Return whether the registry declared a closed transition description.
    #[must_use]
    pub const fn is_declared(&self) -> bool {
        matches!(self, Self::Declared(_))
    }

    /// Return the closed transition facts, when the registry declared them.
    #[must_use]
    pub fn declared(&self) -> Option<&StateTransitions> {
        match self {
            Self::UnknownInvocation => None,
            Self::Declared(transitions) => Some(transitions),
        }
    }
}

/// Argument-sensitive resolver for a state transition descriptor.
///
/// The resolver sees only [`InvocationArguments`]' safe accessors. It can
/// retain literal subjects precisely with [`TransitionSubject::from_argument`]
/// and receives typed unknown subjects for dynamic, expanded, or opaque words.
pub type StateTransitionResolver = for<'w> fn(InvocationArguments<'w>) -> StateTransitions;

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

/// A state-domain widening declaration for identity-bearing argument
/// positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateTransitionWideningRule {
    /// Invocation positions whose dynamic values widen `domains`.
    pub operands: StateTransitionOperandLayout,
    /// State domains made unknown by those operands.
    pub domains: &'static [StateTransitionDomain],
}

/// Whether expansion or opacity can alter the positional grammar a resolver
/// uses to interpret operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateTransitionArgumentShape {
    /// Each argument can be interpreted independently; known arguments stay
    /// precise around an expanded sibling.
    Independent,
    /// Pairing or fixed positions matter. Expanded or opaque arguments
    /// suppress precise resolver facts and widen every declared domain.
    Positional,
}

/// How a narrower transition descriptor combines with a parent descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateTransitionComposition {
    /// Keep parent transitions and append the narrower descriptor's facts.
    Extend,
    /// Replace parent transition facts with the narrower descriptor's facts.
    Replace,
}

/// Registry data for command, subcommand, or form state transitions.
///
/// `dynamic_widening` is enforced centrally only for declared
/// identity-bearing positions, even if a resolver accidentally omits a
/// matching unknown branch. This preserves known `proc` names when unrelated
/// parameter/body words are dynamic, while still widening positional layouts
/// around expansion and opacity.
#[derive(Debug, Clone, Copy)]
pub struct StateTransitionDescriptor {
    /// How this descriptor composes with a less-specific descriptor.
    pub composition: StateTransitionComposition,
    /// Optional resolver for precise transition operands.
    pub resolver: Option<StateTransitionResolver>,
    /// Whether expansion changes the resolver's positional grammar.
    pub argument_shape: StateTransitionArgumentShape,
    /// Domains widened for dynamic identity-bearing operands.
    pub dynamic_widening: &'static [StateTransitionWideningRule],
    /// Legacy or explicit world-effect writes for which the resolved
    /// completion-edge transitions are the authoritative representation.
    ///
    /// The contract is source-qualified, so declaring a command-binding
    /// transition cannot erase an independent variable or host write.
    pub effect_coverage: &'static [TransitionEffectCoverage],
    /// Completion edge on which this descriptor's transitions may commit.
    pub commit: StateTransitionCommit,
}

impl StateTransitionDescriptor {
    /// Explicitly declares that this invocation shape establishes no tracked
    /// state transitions.
    pub const EMPTY: Self = Self {
        composition: StateTransitionComposition::Extend,
        resolver: None,
        argument_shape: StateTransitionArgumentShape::Independent,
        dynamic_widening: &[],
        effect_coverage: TransitionEffectCoverage::NONE,
        commit: StateTransitionCommit::OnOkOnly,
    };

    /// Resolve one descriptor against structured invocation arguments.
    #[must_use]
    pub fn resolve(self, arguments: InvocationArguments<'_>) -> StateTransitions {
        let positional_shape = self.argument_shape == StateTransitionArgumentShape::Positional;
        let mut transitions = if positional_shape && !arguments.has_exact_argv_len() {
            StateTransitions::default()
        } else {
            self.resolver
                .map_or_else(StateTransitions::default, |resolver| resolver(arguments))
        };
        transitions.widen_dynamic_arguments(arguments, self.dynamic_widening, positional_shape);
        transitions.set_commit(self.commit);
        transitions
    }
}

/// The command/subcommand/form transition descriptor chain selected for an
/// invocation. It stays borrowed and allocation-free until a common consumer
/// requests [`Self::resolve`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ResolvedStateTransitions {
    /// Parent command transition descriptor.
    pub command: Option<StateTransitionDescriptor>,
    /// Resolved subcommand transition descriptor.
    pub subcommand: Option<StateTransitionDescriptor>,
    /// Matched form transition descriptor.
    pub form: Option<StateTransitionDescriptor>,
}

impl ResolvedStateTransitions {
    /// Whether this invocation has any registry-owned transition descriptor.
    #[must_use]
    pub const fn is_declared(self) -> bool {
        self.command.is_some() || self.subcommand.is_some() || self.form.is_some()
    }

    /// Resolve command, subcommand, then form facts, applying the narrower
    /// descriptor last so a form can explicitly replace a broader shape.
    #[must_use]
    pub fn resolve(self, arguments: InvocationArguments<'_>) -> StateTransitions {
        self.resolve_with_effect_coverage(arguments).0
    }

    /// Resolve transitions and the corresponding write-coverage contract as
    /// one registry operation. Coverage is selected only when its descriptor
    /// produced at least one transition (including a dynamic widening), so a
    /// no-op shape never suppresses an ordinary effect by itself.
    #[must_use]
    pub fn resolve_with_effect_coverage(
        self,
        arguments: InvocationArguments<'_>,
    ) -> (StateTransitions, TransitionEffectCoverages) {
        let mut transitions = StateTransitions::default();
        let mut coverage = TransitionEffectCoverages::default();
        for descriptor in [self.command, self.subcommand, self.form]
            .into_iter()
            .flatten()
        {
            let resolved = descriptor.resolve(arguments);
            let produced_transition = !resolved.facts().is_empty();
            if descriptor.composition == StateTransitionComposition::Replace {
                transitions = resolved;
                coverage.clear();
            } else {
                transitions.extend(resolved);
            }
            if produced_transition {
                coverage.extend(descriptor.effect_coverage);
            }
        }
        (transitions, coverage)
    }
}

/// Return the local name Tcl gives a namespace-qualified variable reference.
///
/// `global ::pkg::counter` and `variable ::pkg::counter` bind a current-frame
/// local named `counter`, while dynamic source words remain typed unknown.
#[must_use]
pub fn local_alias_name(subject: &TransitionSubject) -> TransitionSubject {
    match subject {
        TransitionSubject::Literal(name) => {
            TransitionSubject::Literal(name.rsplit("::").next().unwrap_or(name).to_owned())
        }
        TransitionSubject::Unknown {
            argument_index,
            word_kind,
        } => TransitionSubject::Unknown {
            argument_index: *argument_index,
            word_kind: *word_kind,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InvocationWord, InvocationWords};

    const VARIABLE_DOMAINS: &[StateTransitionDomain] = &[
        StateTransitionDomain::VariableCells,
        StateTransitionDomain::VariableTraces,
    ];

    #[test]
    fn dynamic_operands_become_typed_widenings() {
        const DESCRIPTOR: StateTransitionDescriptor = StateTransitionDescriptor {
            composition: StateTransitionComposition::Extend,
            resolver: None,
            argument_shape: StateTransitionArgumentShape::Independent,
            dynamic_widening: &[StateTransitionWideningRule {
                operands: StateTransitionOperandLayout::Indices(&[1]),
                domains: VARIABLE_DOMAINS,
            }],
            effect_coverage: TransitionEffectCoverage::NONE,
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
        };
        let arguments = [InvocationWord::Literal("fixed"), InvocationWord::Dynamic];
        let transitions = DESCRIPTOR.resolve(
            InvocationWords::structured(InvocationWord::Literal("fixture"), &arguments).arguments(),
        );

        assert_eq!(
            transitions.facts(),
            &[StateTransitionFact {
                transition: StateTransition::Widen(StateTransitionWidening {
                    domains: vec![
                        StateTransitionDomain::VariableCells,
                        StateTransitionDomain::VariableTraces,
                    ],
                    subject: TransitionSubject::Unknown {
                        argument_index: 1,
                        word_kind: InvocationWordKind::Dynamic,
                    },
                }),
                commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            }]
        );
    }

    fn named_definition(name: &str) -> StateTransitions {
        let mut transitions = StateTransitions::default();
        transitions.push(StateTransition::CommandBinding(
            CommandBindingTransition::Define {
                name: TransitionSubject::Literal(name.to_owned()),
            },
        ));
        transitions
    }

    fn command_definition(_: InvocationArguments<'_>) -> StateTransitions {
        named_definition("command")
    }

    fn subcommand_definition(_: InvocationArguments<'_>) -> StateTransitions {
        named_definition("subcommand")
    }

    fn form_definition(_: InvocationArguments<'_>) -> StateTransitions {
        named_definition("form")
    }

    #[test]
    fn on_ok_transition_is_unchanged_on_an_abrupt_edge() {
        // A `proc`-like definition becomes precise only on the OK edge. An
        // exact CFG must not treat this as an unconditional binding fact.
        let descriptor = StateTransitionDescriptor {
            composition: StateTransitionComposition::Extend,
            resolver: Some(command_definition),
            argument_shape: StateTransitionArgumentShape::Independent,
            dynamic_widening: &[],
            effect_coverage: TransitionEffectCoverage::NONE,
            commit: StateTransitionCommit::OnOkOnly,
        };
        let transitions = descriptor.resolve(InvocationArguments::literals(&[]));
        let [fact] = transitions.facts() else {
            panic!("definition resolver must emit one transition");
        };

        assert_eq!(fact.commit, StateTransitionCommit::OnOkOnly);
        assert_eq!(
            fact.commit.abrupt_transfer(),
            AbruptTransitionTransfer::Unchanged
        );
    }

    #[test]
    fn descriptors_resolve_command_subcommand_then_form() {
        let command = StateTransitionDescriptor {
            composition: StateTransitionComposition::Extend,
            resolver: Some(command_definition),
            argument_shape: StateTransitionArgumentShape::Independent,
            dynamic_widening: &[],
            effect_coverage: TransitionEffectCoverage::NONE,
            commit: StateTransitionCommit::OnOkOnly,
        };
        let subcommand = StateTransitionDescriptor {
            composition: StateTransitionComposition::Extend,
            resolver: Some(subcommand_definition),
            argument_shape: StateTransitionArgumentShape::Independent,
            dynamic_widening: &[],
            effect_coverage: TransitionEffectCoverage::NONE,
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
        };
        let form = StateTransitionDescriptor {
            composition: StateTransitionComposition::Replace,
            resolver: Some(form_definition),
            argument_shape: StateTransitionArgumentShape::Independent,
            dynamic_widening: &[],
            effect_coverage: TransitionEffectCoverage::NONE,
            commit: StateTransitionCommit::OnOkOnly,
        };
        let transitions = ResolvedStateTransitions {
            command: Some(command),
            subcommand: Some(subcommand),
            form: Some(form),
        }
        .resolve(InvocationArguments::literals(&[]));

        assert_eq!(transitions.facts().len(), 1);
        assert_eq!(
            transitions.facts()[0].transition,
            StateTransition::CommandBinding(CommandBindingTransition::Define {
                name: TransitionSubject::Literal("form".to_owned()),
            })
        );
    }

    #[test]
    fn qualified_aliases_use_the_current_frame_tail_name() {
        assert_eq!(
            local_alias_name(&TransitionSubject::Literal("::pkg::counter".to_owned())),
            TransitionSubject::Literal("counter".to_owned())
        );
    }
}
