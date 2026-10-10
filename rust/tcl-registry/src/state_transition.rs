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
use crate::model::binding::PackageTransition;
use crate::side_effects::SideEffectTarget;
use crate::world_effect::{
    TransitionEffectCoverage, TransitionEffectCoverages, WorldEffectWriteSource, WorldStateDomain,
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
    let mut position = name.len();
    while position > 0 {
        position -= 1;
        if name[position] == b':' && position > 0 && name[position - 1] == b':' {
            let mut qualifier_end = position - 1;
            while qualifier_end > 0 && name[qualifier_end - 1] == b':' {
                qualifier_end -= 1;
            }
            return &name[..qualifier_end];
        }
    }
    &name[..0]
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

/// Storage selected for a variable-alias destination name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableAliasDestination {
    /// A procedure local-table link. In C Tcl global and namespace evaluation
    /// frames this declaration has no effect.
    ProcedureLocal,
    /// An unqualified name binds locally in a procedure activation; namespace
    /// activations and qualified names create a namespace slot without the
    /// ordinary read lookup's global fallback.
    CurrentNamespaceOrLocal,
}

/// Actual activation class consumed by variable-alias declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableAliasFrame {
    /// Interpreter root activation with no local variable table.
    Global,
    /// A namespace-evaluation activation.
    Namespace,
    /// A procedure or method activation owning a local table.
    Procedure,
    /// A caller-selected activation whose class is not proved.
    Unknown,
}

impl VariableAliasDestination {
    /// Whether this destination grammar acts in the selected activation.
    /// Unknown frame or dialect policy never silently selects C Tcl rules.
    #[must_use]
    pub fn is_active_in_frame(
        self,
        frame: VariableAliasFrame,
        dialect: crate::InvocationDialect,
    ) -> Option<bool> {
        self.is_active_in_frame_with_policy(frame, Some(dialect))
    }

    /// The same activation query before an execution dialect is selected.
    /// Frame-independent facts remain usable; namespace policy abstains.
    #[must_use]
    pub fn is_active_in_frame_with_policy(
        self,
        frame: VariableAliasFrame,
        dialect: Option<crate::InvocationDialect>,
    ) -> Option<bool> {
        match (self, frame) {
            (_, VariableAliasFrame::Unknown) => None,
            (Self::CurrentNamespaceOrLocal, _)
            | (Self::ProcedureLocal, VariableAliasFrame::Procedure) => Some(true),
            (Self::ProcedureLocal, VariableAliasFrame::Global) => Some(false),
            (Self::ProcedureLocal, VariableAliasFrame::Namespace) => dialect
                .and_then(|dialect| dialect.variable_lookup_policy)
                .map(|policy| policy == tcl_dialect::VariableLookupPolicy::Jim),
        }
    }
}

/// A local variable cell bound to another Tcl cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableCellAliasTransition {
    /// The destination storage selected by this declaration grammar.
    pub destination: VariableAliasDestination,
    /// The current-frame local variable name.
    pub local: TransitionSubject,
    /// The cell reached through `local`.
    pub target: VariableAliasTarget,
    /// Whether this invocation also writes a value through the aliased cell.
    ///
    /// Alias establishment alone is declaration state, but some invocation
    /// forms establish the alias and initialise the target in the same
    /// operation.  Consumers must not infer that distinction from a command
    /// spelling or argument layout.
    pub writes_value: bool,
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
        /// Whether existing commands may be replaced. An unresolved leading
        /// option leaves this unknown rather than assuming a non-forcing import.
        force: Option<bool>,
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

impl NamespaceTransition {
    /// Split the sole exact export control word from the retained pattern
    /// operands. Other dash-prefixed values, including `--`, are patterns.
    /// An unknown leading value leaves both clearing and pattern layout unknown.
    #[must_use]
    pub fn export_pattern_operands(
        patterns: &[TransitionSubject],
    ) -> Option<(bool, &[TransitionSubject])> {
        let clear = patterns.first().map_or(Some(false), |first| {
            first.literal().map(|word| word == "-clear")
        })?;
        Some((clear, &patterns[usize::from(clear)..]))
    }

    /// Registry-authored global helper invoked before C namespace import.
    /// Absence or actual body effects require an independent current-table proof.
    #[must_use]
    pub const fn import_preload_command(
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<&'static str> {
        match policy.recipe() {
            tcl_syntax::naming::NativeNameProtocol::C(_) => Some("auto_import"),
            tcl_syntax::naming::NativeNameProtocol::Jim084 => None,
        }
    }

    /// Select the exact leading export control using the independently
    /// selected native pattern extent. Remaining operands retain whole values.
    #[must_use]
    pub fn export_pattern_byte_operands<'a>(
        patterns: &'a [&'a [u8]],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<(bool, &'a [&'a [u8]])> {
        if !matches!(
            policy.recipe(),
            tcl_syntax::naming::NativeNameProtocol::C(_)
        ) {
            return None;
        }
        let clear = patterns.first().map_or(Some(false), |first| {
            policy
                .recipe()
                .namespace_pattern_input(
                    first,
                    tcl_syntax::naming::NativeNamePurpose::NamespaceExportPattern,
                )
                .ok()
                .map(|input| input.selected() == b"-clear")
        })?;
        Some((clear, &patterns[usize::from(clear)..]))
    }

    /// Select the leading import option using the same independently selected
    /// pattern extent as actual import lookup; every remaining value is intact.
    #[must_use]
    pub fn import_pattern_byte_operands<'a>(
        patterns: &'a [&'a [u8]],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<(bool, &'a [&'a [u8]])> {
        let forced = patterns.first().map_or(Some(false), |first| {
            policy
                .recipe()
                .namespace_pattern_input(
                    first,
                    tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern,
                )
                .ok()
                .map(|input| input.selected() == b"-force")
        })?;
        Some((forced, &patterns[usize::from(forced)..]))
    }

    /// Whether this transition can change command lookup in the current
    /// interpreter.
    ///
    /// Creation alone installs no command, and changing an export pattern only
    /// affects a later import operation. Every other variant can add, remove,
    /// redirect, or provide fallback command resolution immediately.
    #[must_use]
    pub const fn affects_command_resolution(&self) -> bool {
        matches!(
            self,
            Self::Delete { .. }
                | Self::Import { .. }
                | Self::Forget { .. }
                | Self::SetPath { .. }
                | Self::SetUnknown { .. }
                | Self::Ensemble { .. }
        )
    }
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
            if word.literal().is_some()
                || (word.native_bytes().is_some()
                    && self
                        .facts
                        .iter()
                        .any(|fact| fact.transition.represents_native_identity_subject(&subject)))
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

fn visit_namespace_subject(
    target: &mut NamespaceTransitionTarget,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    if let NamespaceTransitionTarget::Named(subject) = target {
        visit(subject);
    }
}
fn visit_object_subject(
    target: &mut ObjectDispatchTarget,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    if let ObjectDispatchTarget::Named(subject) = target {
        visit(subject);
    }
}
fn visit_private_subject(
    target: &mut ObjectPrivateNamespace,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    if let ObjectPrivateNamespace::Named(subject) = target {
        visit(subject);
    }
}

fn transition_commandbinding_identity(
    transition: &CommandBindingTransition,
    subject: &TransitionSubject,
) -> bool {
    match transition {
        CommandBindingTransition::Define { name, .. } => name == subject,
        CommandBindingTransition::Move { from, to } => from == subject || to == subject,
        CommandBindingTransition::Delete { interpreter, name } => {
            name == subject || interpreter.as_ref() == Some(subject)
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            ..
        } => [source_interpreter, alias, target_interpreter, target].contains(&subject),
        CommandBindingTransition::Unknown { .. } => false,
    }
}

fn transition_interpreter_identity(
    transition: &InterpreterTransition,
    subject: &TransitionSubject,
) -> bool {
    match transition {
        InterpreterTransition::Create { interpreter, .. } => interpreter.as_ref() == Some(subject),
        InterpreterTransition::Delete { interpreter }
        | InterpreterTransition::MarkTrusted { interpreter }
        | InterpreterTransition::SetRecursionLimit { interpreter, .. }
        | InterpreterTransition::SetBackgroundError { interpreter, .. } => interpreter == subject,
        InterpreterTransition::Hide {
            interpreter,
            visible,
            hidden,
        }
        | InterpreterTransition::Expose {
            interpreter,
            hidden,
            visible,
        } => [interpreter, visible, hidden].contains(&subject),
    }
}

fn transition_namespace_identity(
    transition: &NamespaceTransition,
    subject: &TransitionSubject,
) -> bool {
    let namespace = |target: &NamespaceTransitionTarget| matches!(target, NamespaceTransitionTarget::Named(name) if name == subject);
    match transition {
        NamespaceTransition::Ensure { namespace: target }
        | NamespaceTransition::Delete { namespace: target }
        | NamespaceTransition::Ensemble { namespace: target }
        | NamespaceTransition::SetPath {
            namespace: target, ..
        }
        | NamespaceTransition::SetUnknown {
            namespace: target, ..
        } => namespace(target),
        NamespaceTransition::Export {
            namespace: target,
            patterns,
        }
        | NamespaceTransition::Import {
            namespace: target,
            patterns,
            ..
        }
        | NamespaceTransition::Forget {
            namespace: target,
            patterns,
        } => namespace(target) || patterns.contains(subject),
    }
}

fn transition_objectdispatch_identity(
    transition: &ObjectDispatchTransition,
    subject: &TransitionSubject,
) -> bool {
    let object = |target: &ObjectDispatchTarget| matches!(target, ObjectDispatchTarget::Named(name) if name == subject);
    let private = |target: &ObjectPrivateNamespace| matches!(target, ObjectPrivateNamespace::Named(name) if name == subject);
    match transition {
        ObjectDispatchTransition::Configure { target, .. }
        | ObjectDispatchTransition::Destroy { target } => target == subject,
        ObjectDispatchTransition::Create {
            target,
            private_namespace,
            ..
        } => object(target) || private(private_namespace),
        ObjectDispatchTransition::Copy {
            source,
            target,
            private_namespace,
        } => source == subject || object(target) || private(private_namespace),
    }
}

fn transition_package_identity(
    transition: &crate::model::binding::PackageTransition,
    subject: &TransitionSubject,
) -> bool {
    match transition {
        PackageTransition::Provide { package, .. }
        | PackageTransition::Require { package, .. }
        | PackageTransition::Ifneeded { package, .. } => package == subject,
        PackageTransition::Forget { packages } => packages.contains(subject),
        PackageTransition::DiscoveryDependencyChanged { .. }
        | PackageTransition::UnknownHandler { .. }
        | PackageTransition::Prefer { .. }
        | PackageTransition::SourceLoad { .. } => false,
    }
}

fn transition_commandbinding_visit(
    transition: &mut CommandBindingTransition,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    match transition {
        CommandBindingTransition::Define { name, .. } => visit(name),
        CommandBindingTransition::Move { from, to } => {
            visit(from);
            visit(to);
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            if let Some(interpreter) = interpreter {
                visit(interpreter);
            }
            visit(name);
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            ..
        } => {
            visit(source_interpreter);
            visit(alias);
            visit(target_interpreter);
            visit(target);
            for argument in arguments {
                visit(argument);
            }
        }
        CommandBindingTransition::Unknown { operands } => {
            for operand in operands {
                visit(operand);
            }
        }
    }
}

fn transition_interpreter_visit(
    transition: &mut InterpreterTransition,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    match transition {
        InterpreterTransition::Create { interpreter, .. } => {
            if let Some(interpreter) = interpreter {
                visit(interpreter);
            }
        }
        InterpreterTransition::Delete { interpreter }
        | InterpreterTransition::MarkTrusted { interpreter } => visit(interpreter),
        InterpreterTransition::SetRecursionLimit { interpreter, limit } => {
            visit(interpreter);
            visit(limit);
        }
        InterpreterTransition::SetBackgroundError {
            interpreter,
            handler,
        } => {
            visit(interpreter);
            visit(handler);
        }
        InterpreterTransition::Hide {
            interpreter,
            visible,
            hidden,
        }
        | InterpreterTransition::Expose {
            interpreter,
            hidden,
            visible,
        } => {
            visit(interpreter);
            visit(visible);
            visit(hidden);
        }
    }
}

fn transition_namespace_visit(
    transition: &mut NamespaceTransition,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    match transition {
        NamespaceTransition::Ensure { namespace: target }
        | NamespaceTransition::Delete { namespace: target }
        | NamespaceTransition::Ensemble { namespace: target } => {
            visit_namespace_subject(target, visit);
        }
        NamespaceTransition::Export {
            namespace: target,
            patterns,
        }
        | NamespaceTransition::Import {
            namespace: target,
            patterns,
            ..
        }
        | NamespaceTransition::Forget {
            namespace: target,
            patterns,
        } => {
            visit_namespace_subject(target, visit);
            for pattern in patterns {
                visit(pattern);
            }
        }
        NamespaceTransition::SetPath {
            namespace: target,
            path,
        } => {
            visit_namespace_subject(target, visit);
            visit(path);
        }
        NamespaceTransition::SetUnknown {
            namespace: target,
            handler,
        } => {
            visit_namespace_subject(target, visit);
            visit(handler);
        }
    }
}

fn transition_objectdispatch_visit(
    transition: &mut ObjectDispatchTransition,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    match transition {
        ObjectDispatchTransition::Configure { target, .. }
        | ObjectDispatchTransition::Destroy { target } => visit(target),
        ObjectDispatchTransition::Create {
            target,
            private_namespace,
            ..
        } => {
            visit_object_subject(target, visit);
            visit_private_subject(private_namespace, visit);
        }
        ObjectDispatchTransition::Copy {
            source,
            target,
            private_namespace,
        } => {
            visit(source);
            visit_object_subject(target, visit);
            visit_private_subject(private_namespace, visit);
        }
    }
}

fn transition_package_visit(
    transition: &mut crate::model::binding::PackageTransition,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    match transition {
        PackageTransition::DiscoveryDependencyChanged { .. } => {}
        PackageTransition::Provide { package, version } => {
            visit(package);
            if let Some(version) = version {
                visit(version);
            }
        }
        PackageTransition::Require {
            package,
            requirements,
            ..
        } => {
            visit(package);
            for requirement in requirements {
                visit(requirement);
            }
        }
        PackageTransition::Ifneeded {
            package,
            version,
            script,
            ..
        } => {
            visit(package);
            visit(version);
            if let Some(script) = script {
                visit(script);
            }
        }
        PackageTransition::Forget { packages } => {
            for package in packages {
                visit(package);
            }
        }
        PackageTransition::UnknownHandler { handler } => {
            if let Some(handler) = handler {
                visit(handler);
            }
        }
        PackageTransition::Prefer { mode } => {
            if let Some(mode) = mode {
                visit(mode);
            }
        }
        PackageTransition::SourceLoad { path } => visit(path),
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
    /// Optional transition projection valid only on successful completion.
    /// Ordinary/error facts retain `resolver`; consumers must select the edge.
    pub success_resolver: Option<StateTransitionResolver>,
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
        success_resolver: None,
        argument_shape: StateTransitionArgumentShape::Independent,
        dynamic_widening: &[],
        effect_coverage: TransitionEffectCoverage::NONE,
        commit: StateTransitionCommit::OnOkOnly,
    };

    /// Resolve one descriptor against structured invocation arguments.
    #[must_use]
    pub fn resolve(self, arguments: InvocationArguments<'_>) -> StateTransitions {
        self.resolve_on_edge(arguments, false)
    }

    /// Resolve facts on the normal completion edge, preserving ordinary/error
    /// facts in [`Self::resolve`].
    #[must_use]
    pub fn resolve_after_success(self, arguments: InvocationArguments<'_>) -> StateTransitions {
        self.resolve_on_edge(arguments, true)
    }

    fn resolve_on_edge(
        self,
        arguments: InvocationArguments<'_>,
        successful: bool,
    ) -> StateTransitions {
        let positional_shape = self.argument_shape == StateTransitionArgumentShape::Positional;
        let mut transitions = if positional_shape && !arguments.has_exact_argv_len() {
            StateTransitions::default()
        } else {
            let resolver = if successful {
                self.success_resolver.or(self.resolver)
            } else {
                self.resolver
            };
            resolver.map_or_else(StateTransitions::default, |resolver| resolver(arguments))
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
        self.resolve_coverage_on_edge(arguments, false)
    }

    /// Resolve normal-edge transitions and their matching write coverage.
    #[must_use]
    pub fn resolve_after_success_with_effect_coverage(
        self,
        arguments: InvocationArguments<'_>,
    ) -> (StateTransitions, TransitionEffectCoverages) {
        self.resolve_coverage_on_edge(arguments, true)
    }

    fn resolve_coverage_on_edge(
        self,
        arguments: InvocationArguments<'_>,
        successful: bool,
    ) -> (StateTransitions, TransitionEffectCoverages) {
        let mut transitions = StateTransitions::default();
        let mut coverage = TransitionEffectCoverages::default();
        for descriptor in [self.command, self.subcommand, self.form]
            .into_iter()
            .flatten()
        {
            let resolved = descriptor.resolve_on_edge(arguments, successful);
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

/// The operation selecting a local variable alias name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableAliasNamePurpose {
    /// A global-variable link; Jim leaves rooted names without a local link.
    Global,
    /// A namespace-variable link, with the engine's namespace-tail rules.
    NamespaceVariable,
}

/// Project an original alias operand through the selected native naming owner.
/// Missing policies and non-Unicode projections remain opaque; no native
/// lookup, successful link, physical cell or compiler permission follows.
/// `None` means the selected operation creates no local alias.
#[must_use]
pub fn local_alias_name(
    subject: &TransitionSubject,
    argument_index: usize,
    purpose: VariableAliasNamePurpose,
    dialect: Option<crate::InvocationDialect>,
) -> Option<TransitionSubject> {
    if let TransitionSubject::LocatedNativeBytes {
        value: name,
        argument_index,
    } = subject
    {
        let argument_index = *argument_index;
        let selected = match dialect.and_then(crate::InvocationDialect::native_name_protocol) {
            Some(protocol) => match purpose {
                VariableAliasNamePurpose::Global => {
                    tcl_syntax::naming::global_local_name_bytes(protocol, name)?
                }
                VariableAliasNamePurpose::NamespaceVariable => {
                    tcl_syntax::naming::variable_local_name_bytes(protocol, name)
                }
            },
            None if dialect.is_some_and(|dialect| {
                dialect.family() == Some(tcl_dialect::model::Family::Tcl)
            }) =>
            {
                tcl_syntax::naming::c_family_local_alias_name_bytes(name)?
            }
            None => {
                return Some(TransitionSubject::Unknown {
                    argument_index,
                    word_kind: InvocationWordKind::KnownBytes,
                });
            }
        };
        return Some(TransitionSubject::LocatedNativeBytes {
            value: selected,
            argument_index,
        });
    }
    let Some(name) = subject.literal() else {
        return Some(subject.clone());
    };
    let opaque = || TransitionSubject::Unknown {
        argument_index,
        word_kind: InvocationWordKind::Opaque,
    };
    let Some(protocol) = dialect.and_then(crate::InvocationDialect::native_name_protocol) else {
        if dialect.is_some_and(|dialect| dialect.family() == Some(tcl_dialect::model::Family::Tcl))
        {
            return Some(
                tcl_syntax::naming::c_family_local_alias_name_bytes(name.as_bytes())
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .map_or_else(opaque, |value| {
                        subject
                            .with_literal_value(value)
                            .expect("known alias value")
                    }),
            );
        }
        return Some(opaque());
    };
    let selected = match purpose {
        VariableAliasNamePurpose::Global => {
            tcl_syntax::naming::global_local_name_bytes(protocol, name.as_bytes())?
        }
        VariableAliasNamePurpose::NamespaceVariable => {
            tcl_syntax::naming::variable_local_name_bytes(protocol, name.as_bytes())
        }
    };
    Some(String::from_utf8(selected).map_or_else(
        |_| opaque(),
        |value| {
            subject
                .with_literal_value(value)
                .expect("known alias value")
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_namespace_export_control_uses_the_selected_cstring_extent() {
        // Implementation contract: naming.namespace.original-byte-pattern-transfers
        // docs/design/analysis/name-resolution-proofs/namespace-original-byte-pattern-transfers.md
        for version in tcl_dialect::TclVersion::ALL {
            let policy = tcl_syntax::naming::NamePolicyProtocol::authored_tcl(version);
            let patterns = [b"-clear\0suffix".as_slice(), b"p\xed\xa0\x80"];
            let (clear, retained) =
                NamespaceTransition::export_pattern_byte_operands(&patterns, policy).unwrap();
            assert!(clear);
            assert_eq!(retained, &patterns[1..]);
            let modified = [b"-clear\xc0\x80suffix".as_slice()];
            assert_eq!(
                NamespaceTransition::export_pattern_byte_operands(&modified, policy),
                Some((false, modified.as_slice()))
            );
            assert_eq!(
                NamespaceTransition::import_preload_command(policy),
                Some("auto_import")
            );
        }
        let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some("jimtcl")).unwrap();
        let policy = tcl_syntax::naming::NamePolicyProtocol::for_native_point(point).unwrap();
        assert!(NamespaceTransition::export_pattern_byte_operands(&[], policy).is_none());
        assert_eq!(NamespaceTransition::import_preload_command(policy), None);
    }

    #[test]
    fn original_namespace_patterns_close_only_their_represented_byte_ordinals() {
        // Implementation contract: naming.namespace.original-byte-pattern-transfers
        // docs/design/analysis/name-resolution-proofs/namespace-original-byte-pattern-transfers.md
        for version in tcl_dialect::TclVersion::ALL {
            let registry =
                crate::model::ingress::static_context_for(version.dialect_name()).commands();
            let namespace = registry.get("namespace").unwrap();
            let dialect = crate::InvocationDialect::for_version(version);
            for operation in ["export", "import", "forget"] {
                let descriptor = namespace
                    .subcommands
                    .iter()
                    .find(|entry| entry.name == operation)
                    .and_then(|entry| entry.state_transitions)
                    .unwrap();
                let words = [
                    crate::InvocationWord::Literal(operation),
                    crate::InvocationWord::KnownBytes(b"p\xed\xa0\x80"),
                ];
                let transitions = descriptor
                    .resolve(InvocationArguments::structured(&words).with_dialect(dialect));
                assert!(
                    transitions
                        .facts()
                        .iter()
                        .all(|fact| !matches!(fact.transition, StateTransition::Widen(_))),
                    "{version:?}: {operation}"
                );
                let pattern = match &transitions.facts()[0].transition {
                    StateTransition::Namespace(
                        NamespaceTransition::Export { patterns, .. }
                        | NamespaceTransition::Forget { patterns, .. },
                    ) => &patterns[0],
                    StateTransition::Namespace(NamespaceTransition::Import {
                        patterns,
                        force,
                        ..
                    }) => {
                        assert_eq!(*force, Some(false));
                        &patterns[0]
                    }
                    _ => panic!("selected namespace pattern operation"),
                };
                assert_eq!(pattern.native_bytes(), Some(b"p\xed\xa0\x80".as_slice()));
                assert_eq!(pattern.argument_index(), Some(1));
                let words = [
                    crate::InvocationWord::Literal(operation),
                    crate::InvocationWord::Dynamic,
                ];
                let unknown = descriptor
                    .resolve(InvocationArguments::structured(&words).with_dialect(dialect));
                assert!(
                    unknown
                        .facts()
                        .iter()
                        .any(|fact| matches!(fact.transition, StateTransition::Widen(_)))
                );
            }
        }
    }

    #[test]
    fn native_byte_subjects_preserve_values_and_project_each_ordinal() {
        // Implementation contract: naming.invocation.known-native-byte-values
        // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
        let payload = [b'n', 0, 0xff];
        let words = [
            crate::InvocationWord::KnownBytes(&payload),
            crate::InvocationWord::KnownBytes(&payload),
        ];
        let arguments = InvocationArguments::structured(&words);
        let first = TransitionSubject::from_argument(arguments, 0).unwrap();
        let second = TransitionSubject::from_argument(arguments, 1).unwrap();
        assert_eq!(first.native_bytes(), Some(payload.as_slice()));
        assert_eq!(first.literal(), None);
        assert_eq!(first.argument_index(), Some(0));
        assert_ne!(first, second);
        assert!(first.with_literal_value("replacement".to_owned()).is_none());
        let mut transitions = StateTransitions::default();
        transitions.push(StateTransition::CommandBinding(
            CommandBindingTransition::Move {
                from: first.clone(),
                to: second.clone(),
            },
        ));
        let projected = transitions
            .project_argument_indices(|index| index.checked_add(2))
            .unwrap();
        let StateTransition::CommandBinding(CommandBindingTransition::Move { from, to }) =
            &projected.facts()[0].transition
        else {
            panic!("move");
        };
        assert_eq!(from.native_bytes(), first.native_bytes());
        assert_eq!(to.native_bytes(), second.native_bytes());
        assert_eq!(from.argument_index(), Some(2));
        assert_eq!(to.argument_index(), Some(3));
        assert!(
            transitions
                .project_argument_indices(|index| index.checked_sub(1))
                .is_none()
        );
    }

    #[test]
    fn original_byte_rename_transitions_select_destination_extent_and_keep_ordinals() {
        // Implementation contract: naming.command.original-byte-rename-transition
        // docs/design/analysis/name-resolution-proofs/command-original-byte-rename-transition.md
        use crate::InvocationWord;
        let from = b"P\xed\xa0\x80";
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            for destination in [b"Q\xed\xa0\x81".as_slice(), b"Q\xc0\x80tail"] {
                let words = [
                    InvocationWord::KnownBytes(from),
                    InvocationWord::KnownBytes(destination),
                ];
                let transitions = command_binding::RENAMES_COMMANDS
                    .resolve(InvocationArguments::structured(&words).with_dialect(dialect));
                let [fact] = transitions.facts() else {
                    panic!("one rename transition")
                };
                let StateTransition::CommandBinding(CommandBindingTransition::Move {
                    from: original,
                    to,
                }) = &fact.transition
                else {
                    panic!("byte move")
                };
                assert_eq!(original.native_bytes(), Some(from.as_slice()));
                assert_eq!(original.argument_index(), Some(0));
                assert_eq!(to.native_bytes(), Some(destination));
                assert_eq!(to.argument_index(), Some(1));
                assert_eq!(
                    fact.commit,
                    StateTransitionCommit::MayCommitBeforeAbruptCompletion
                );
                assert!(!transitions.widens(StateTransitionDomain::CommandBindings));
            }
            let destination = b"N\xed\xa0\x81::::Q\0ignored::later";
            let words = [
                InvocationWord::KnownBytes(from),
                InvocationWord::KnownBytes(destination),
            ];
            let transitions = command_binding::RENAMES_COMMANDS
                .resolve(InvocationArguments::structured(&words).with_dialect(dialect));
            let StateTransition::Namespace(NamespaceTransition::Ensure {
                namespace: NamespaceTransitionTarget::Named(namespace),
            }) = &transitions.facts()[0].transition
            else {
                panic!("selected qualifier")
            };
            assert_eq!(namespace.native_bytes(), Some(b"N\xed\xa0\x81".as_slice()));
            assert_eq!(namespace.argument_index(), Some(1));
            let StateTransition::CommandBinding(CommandBindingTransition::Move { to, .. }) =
                &transitions.facts()[1].transition
            else {
                panic!("move retains full input")
            };
            assert_eq!(to.native_bytes(), Some(destination.as_slice()));
            let words = [
                InvocationWord::KnownBytes(from),
                InvocationWord::KnownBytes(b"\0tail"),
            ];
            let transitions = command_binding::RENAMES_COMMANDS
                .resolve(InvocationArguments::structured(&words).with_dialect(dialect));
            let StateTransition::CommandBinding(CommandBindingTransition::Delete { name, .. }) =
                &transitions.facts()[0].transition
            else {
                panic!("empty selected CString deletes")
            };
            assert_eq!(name.native_bytes(), Some(from.as_slice()));
        }
        let words = [
            InvocationWord::KnownBytes(from),
            InvocationWord::KnownBytes(b"Q"),
        ];
        for dialect in [
            None,
            Some(crate::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
            )),
        ] {
            let arguments = InvocationArguments::structured(&words);
            let arguments = dialect.map_or(arguments, |dialect| arguments.with_dialect(dialect));
            let transitions = command_binding::RENAMES_COMMANDS.resolve(arguments);
            assert!(transitions.facts().iter().any(|fact| matches!(
                fact.transition,
                StateTransition::CommandBinding(CommandBindingTransition::Unknown { .. })
            )));
            assert!(transitions.widens(StateTransitionDomain::CommandBindings));
        }
        assert!(
            TransitionSubject::Literal("Q".to_owned())
                .with_native_bytes_value(vec![b'Q'])
                .is_none()
        );
    }

    #[test]
    fn native_identity_values_do_not_close_unknown_control_operands() {
        // Implementation contract: naming.invocation.known-native-byte-values
        // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
        let subject = TransitionSubject::LocatedNativeBytes {
            value: vec![0xff],
            argument_index: 0,
        };
        let different_ordinal = TransitionSubject::LocatedNativeBytes {
            value: vec![0xff],
            argument_index: 1,
        };
        let defined = StateTransition::CommandBinding(CommandBindingTransition::Define {
            name: subject.clone(),
            kind: CommandBindingDefinitionKind::Command,
        });
        assert!(defined.represents_native_identity_subject(&subject));
        assert!(!defined.represents_native_identity_subject(&different_ordinal));
        let unresolved = StateTransition::CommandBinding(CommandBindingTransition::Unknown {
            operands: vec![subject.clone()],
        });
        assert!(!unresolved.represents_native_identity_subject(&subject));
        let interpreter = TransitionSubject::Literal(String::new());
        let limit = StateTransition::Interpreter(InterpreterTransition::SetRecursionLimit {
            interpreter,
            limit: subject.clone(),
        });
        assert!(!limit.represents_native_identity_subject(&subject));
        let package = StateTransition::Package(crate::model::binding::PackageTransition::Provide {
            package: TransitionSubject::Literal("P".to_owned()),
            version: Some(subject.clone()),
        });
        assert!(!package.represents_native_identity_subject(&subject));
        let words = [crate::InvocationWord::KnownBytes(&[0xff])];
        let arguments = InvocationArguments::structured(&words);
        let rules = [StateTransitionWideningRule {
            operands: StateTransitionOperandLayout::Indices(&[0]),
            domains: &[StateTransitionDomain::CommandBindings],
        }];
        let mut unknown = StateTransitions::default();
        unknown.push(unresolved);
        unknown.widen_dynamic_arguments(arguments, &rules, true);
        assert!(unknown.widens(StateTransitionDomain::CommandBindings));
        let mut represented = StateTransitions::default();
        represented.push(defined);
        represented.widen_dynamic_arguments(arguments, &rules, true);
        assert!(!represented.widens(StateTransitionDomain::CommandBindings));
    }

    #[test]
    fn native_byte_alias_names_use_the_selected_name_purpose() {
        // Implementation contract: naming.invocation.known-native-byte-values
        // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
        let input = TransitionSubject::LocatedNativeBytes {
            value: b"::scope::n\xed\xa0\x80".to_vec(),
            argument_index: 3,
        };
        for version in tcl_dialect::TclVersion::ALL {
            let local = local_alias_name(
                &input,
                3,
                VariableAliasNamePurpose::Global,
                Some(crate::InvocationDialect::for_version(version)),
            )
            .unwrap();
            assert_eq!(local.native_bytes(), Some(b"n\xed\xa0\x80".as_slice()));
            assert_eq!(local.literal(), None);
            assert_eq!(local.argument_index(), Some(3));
        }
        let jim = crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
        );
        assert!(local_alias_name(&input, 3, VariableAliasNamePurpose::Global, Some(jim)).is_none());
        let local = local_alias_name(
            &input,
            3,
            VariableAliasNamePurpose::NamespaceVariable,
            Some(jim),
        )
        .unwrap();
        assert_eq!(local.native_bytes(), Some(b"n\xed\xa0\x80".as_slice()));
        assert_eq!(local.argument_index(), Some(3));
        assert!(matches!(
            local_alias_name(&input, 3, VariableAliasNamePurpose::Global, None),
            Some(TransitionSubject::Unknown {
                argument_index: 3,
                word_kind: InvocationWordKind::KnownBytes
            })
        ));
    }

    #[test]
    fn known_equal_operands_retain_distinct_effective_ordinals() {
        // Implementation contract: naming.invocation.effective-transition-operands
        // docs/design/analysis/name-resolution-proofs/effective-transition-operands.md

        let arguments = InvocationArguments::literals(&["same", "same"]);
        let first = TransitionSubject::from_argument(arguments, 0).unwrap();
        let second = TransitionSubject::from_argument(arguments, 1).unwrap();
        assert_eq!(first.literal(), second.literal());
        assert_ne!(first, second);
        assert_eq!(first.argument_index(), Some(0));
        assert_eq!(second.argument_index(), Some(1));
        assert_eq!(
            TransitionSubject::Literal("same".to_owned()).argument_index(),
            None
        );
        assert_eq!(
            second
                .with_literal_value("selected".to_owned())
                .unwrap()
                .argument_index(),
            Some(1)
        );
    }

    #[test]
    fn argv_projection_retains_nested_operands_order_and_completion_commits() {
        // Implementation contract: naming.invocation.effective-transition-operands
        // docs/design/analysis/name-resolution-proofs/effective-transition-operands.md

        let subject = |index| TransitionSubject::LocatedLiteral {
            value: "same".to_owned(),
            argument_index: index,
        };
        let mut transitions = StateTransitions::default();
        transitions.push(StateTransition::CommandBinding(
            CommandBindingTransition::Alias {
                target_lookup: AliasTargetLookup::Global,
                source_interpreter: TransitionSubject::Literal(String::new()),
                alias: subject(1),
                target_interpreter: subject(2),
                target: subject(3),
                arguments: vec![
                    subject(4),
                    TransitionSubject::Unknown {
                        argument_index: 5,
                        word_kind: InvocationWordKind::Dynamic,
                    },
                ],
            },
        ));
        transitions.push(StateTransition::VariableCellAlias(
            VariableCellAliasTransition {
                destination: VariableAliasDestination::CurrentNamespaceOrLocal,
                local: subject(4),
                target: VariableAliasTarget::CallerSelectedFrame {
                    frame: CallerFrameSelection::Explicit(subject(1)),
                    variable: subject(2),
                },
                writes_value: false,
            },
        ));
        transitions.set_commit(StateTransitionCommit::MayCommitBeforeAbruptCompletion);
        let projected = transitions
            .project_argument_indices(|index| index.checked_sub(1))
            .unwrap();
        assert!(
            projected
                .facts()
                .iter()
                .all(|fact| fact.commit == StateTransitionCommit::MayCommitBeforeAbruptCompletion)
        );
        let mut indices = Vec::new();
        let mut values = Vec::new();
        for mut fact in projected.facts().iter().cloned() {
            fact.transition.for_each_subject_mut(&mut |subject| {
                indices.push(subject.argument_index());
                values.push(subject.literal().map(str::to_owned));
            });
        }
        assert_eq!(
            indices,
            [
                None,
                Some(0),
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(3),
                Some(0),
                Some(1)
            ]
        );
        assert_eq!(values[0].as_deref(), Some(""));
        assert_eq!(values[5], None);
        assert!(
            transitions
                .project_argument_indices(|index| index.checked_sub(2))
                .is_none()
        );
        assert_eq!(
            transitions.facts()[0].commit,
            StateTransitionCommit::MayCommitBeforeAbruptCompletion
        );
    }
    use crate::{InvocationWord, InvocationWords};

    const VARIABLE_DOMAINS: &[StateTransitionDomain] = &[
        StateTransitionDomain::VariableCells,
        StateTransitionDomain::VariableTraces,
    ];

    #[test]
    fn dynamic_operands_become_typed_widenings() {
        const DESCRIPTOR: StateTransitionDescriptor = StateTransitionDescriptor {
            composition: StateTransitionComposition::Extend,
            success_resolver: None,
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

    #[test]
    fn command_resolution_impact_distinguishes_namespace_cells_from_lookup() {
        let widening = |domain| {
            StateTransition::Widen(StateTransitionWidening {
                domains: vec![StateTransitionDomain::Namespaces, domain],
                subject: TransitionSubject::Unknown {
                    argument_index: 0,
                    word_kind: InvocationWordKind::Dynamic,
                },
            })
        };

        let mut variable_cell = StateTransitions::default();
        variable_cell.push(widening(StateTransitionDomain::VariableCells));
        assert_eq!(
            variable_cell.command_resolution_impact(),
            CommandResolutionImpact::None
        );

        let mut lookup = StateTransitions::default();
        lookup.push(widening(StateTransitionDomain::CommandResolution));
        assert_eq!(
            lookup.command_resolution_impact(),
            CommandResolutionImpact::Unbounded
        );
    }

    fn named_definition(name: &str) -> StateTransitions {
        let mut transitions = StateTransitions::default();
        transitions.push(StateTransition::CommandBinding(
            CommandBindingTransition::Define {
                name: TransitionSubject::Literal(name.to_owned()),
                kind: CommandBindingDefinitionKind::Command,
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
            success_resolver: None,
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
            success_resolver: None,
            resolver: Some(command_definition),
            argument_shape: StateTransitionArgumentShape::Independent,
            dynamic_widening: &[],
            effect_coverage: TransitionEffectCoverage::NONE,
            commit: StateTransitionCommit::OnOkOnly,
        };
        let subcommand = StateTransitionDescriptor {
            composition: StateTransitionComposition::Extend,
            success_resolver: None,
            resolver: Some(subcommand_definition),
            argument_shape: StateTransitionArgumentShape::Independent,
            dynamic_widening: &[],
            effect_coverage: TransitionEffectCoverage::NONE,
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
        };
        let form = StateTransitionDescriptor {
            composition: StateTransitionComposition::Replace,
            success_resolver: None,
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
                kind: CommandBindingDefinitionKind::Command,
            })
        );
    }

    #[test]
    fn qualified_aliases_use_the_current_frame_tail_name() {
        assert_eq!(
            local_alias_name(
                &TransitionSubject::Literal("::pkg::counter".to_owned()),
                0,
                VariableAliasNamePurpose::Global,
                Some(crate::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6
                )),
            ),
            Some(TransitionSubject::Literal("counter".to_owned()))
        );
    }

    #[test]
    fn unversioned_c_alias_assistance_keeps_native_release_unknown() {
        let dialect =
            crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::plain_tcl());
        assert!(dialect.native_name_protocol().is_none());
        let subject = TransitionSubject::Literal("ns:::v".to_owned());
        assert_eq!(
            local_alias_name(&subject, 0, VariableAliasNamePurpose::Global, Some(dialect)),
            Some(TransitionSubject::Literal("v".to_owned())),
        );
        assert!(matches!(
            local_alias_name(
                &TransitionSubject::Literal("v\0tail".to_owned()),
                0,
                VariableAliasNamePurpose::Global,
                Some(dialect),
            ),
            Some(TransitionSubject::Unknown {
                argument_index: 0,
                ..
            }),
        ));
        assert!(dialect.native_name_protocol().is_none());
    }

    #[test]
    fn alias_name_projection_keeps_operation_and_engine_separate() {
        let rooted = TransitionSubject::Literal("::pkg:::counter".to_owned());
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert_eq!(
            local_alias_name(&rooted, 2, VariableAliasNamePurpose::Global, Some(jim)),
            None
        );
        assert_eq!(
            local_alias_name(
                &rooted,
                2,
                VariableAliasNamePurpose::NamespaceVariable,
                Some(jim)
            ),
            Some(TransitionSubject::Literal("counter".to_owned()))
        );
        for version in tcl_dialect::TclVersion::ALL {
            for purpose in [
                VariableAliasNamePurpose::Global,
                VariableAliasNamePurpose::NamespaceVariable,
            ] {
                assert_eq!(
                    local_alias_name(
                        &rooted,
                        2,
                        purpose,
                        Some(crate::InvocationDialect::for_version(version))
                    ),
                    Some(TransitionSubject::Literal("counter".to_owned()))
                );
            }
        }
        for dialect in [
            None,
            Some(crate::InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::irules(),
            )),
        ] {
            assert_eq!(
                local_alias_name(&rooted, 2, VariableAliasNamePurpose::Global, dialect),
                Some(TransitionSubject::Unknown {
                    argument_index: 2,
                    word_kind: InvocationWordKind::Opaque
                })
            );
        }
    }
}

#[cfg(test)]
mod original_namespace_option_advice_tests {
    use super::NamespaceTransition;
    use tcl_syntax::naming::NamePolicyProtocol;

    #[test]
    fn original_namespace_option_advice_selects_only_first_native_control_extent() {
        // Implementation contract: naming.namespace.original-export-source-advice
        // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
        for version in tcl_dialect::TclVersion::ALL {
            let policy = NamePolicyProtocol::authored_tcl(version);
            let options = [b"-force\0suffix".as_slice(), b"-force".as_slice()];
            let (forced, patterns) =
                NamespaceTransition::import_pattern_byte_operands(&options, policy).unwrap();
            assert!(forced);
            assert_eq!(patterns, &options[1..]);
            let escaped_zero = [b"-force\xc0\x80suffix".as_slice()];
            assert!(
                !NamespaceTransition::import_pattern_byte_operands(&escaped_zero, policy)
                    .unwrap()
                    .0
            );
            let clear = [b"-clear".as_slice(), b"-clear".as_slice()];
            let (clears, patterns) =
                NamespaceTransition::export_pattern_byte_operands(&clear, policy).unwrap();
            assert!(clears);
            assert_eq!(patterns, &clear[1..]);
        }
    }
}

#[cfg(test)]
mod created_handle_source_tests {
    use super::*;
    use crate::{InvocationArguments, InvocationDialect, InvocationWord};

    #[test]
    fn created_handle_eval_body_owns_only_the_canonical_single_script_ordinal() {
        // naming.interpreter.original-created-handle-source-body
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-handle-source-body.md
        let create = InterpreterTransition::Create {
            interpreter: Some(TransitionSubject::LocatedLiteral {
                value: "s".to_owned(),
                argument_index: 1,
            }),
            safety: ChildInterpreterSafety::Safe,
        };
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let argv = [InvocationWord::Literal("eval"), InvocationWord::Dynamic];
            let body = create
                .created_handle_eval_body(
                    dialect,
                    InvocationArguments::Structured(&argv).with_dialect(dialect),
                )
                .unwrap();
            assert_eq!(body.argument_index(), Some(1));
            assert!(body.literal().is_none());
            for argv in [
                vec![
                    InvocationWord::Literal("e"),
                    InvocationWord::Literal("script"),
                ],
                vec![InvocationWord::Dynamic, InvocationWord::Literal("script")],
                vec![InvocationWord::Literal("eval"), InvocationWord::Expanded],
                vec![
                    InvocationWord::Literal("eval"),
                    InvocationWord::Literal("script"),
                    InvocationWord::Literal("more"),
                ],
            ] {
                assert!(
                    create
                        .created_handle_eval_body(
                            dialect,
                            InvocationArguments::Structured(&argv).with_dialect(dialect)
                        )
                        .is_none()
                );
            }
        }
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert!(
            create
                .created_handle_eval_body(
                    jim,
                    InvocationArguments::Literals(&["eval", "script"]).with_dialect(jim)
                )
                .is_none()
        );
    }
}
