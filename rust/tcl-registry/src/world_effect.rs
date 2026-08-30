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

//! Registry-owned effects on Tcl's mutable execution world.
//!
//! The ordinary [`crate::side_effects`] model describes external resources
//! such as an HTTP header or a file.  This module describes the mutable Tcl
//! state that determines what later source means: command bindings, namespace
//! lookup, traces, object dispatch, interpreter policy, packages, and
//! observable variable storage.
//!
//! A command spec supplies a [`WorldEffectDescriptor`].  It may contain a
//! static footprint and, for argument-dependent forms, a resolver.  Consumers
//! receive only the resulting [`EffectFootprint`] through
//! [`crate::ResolvedInvocation::effect_footprint`]; they never infer an effect
//! from a command name.  The resolved footprint also bridges the established
//! [`crate::CommandBindingTransition`], [`crate::FrameEffectSpec`], and
//! [`crate::SideEffect`] declarations, so this vocabulary augments rather than
//! replaces the registry's existing facts.

use crate::frame_effect::{FrameArgLayout, FrameEffectSpec};
use crate::invocation_words::InvocationArguments;
use crate::side_effects::{SideEffect, SideEffectTarget};

/// A mutable Tcl or host state domain used by common effect analyses.
///
/// The first variants are deliberately Tcl-specific.  They model state that
/// affects dispatch, observability, and frame materialisation without making
/// those concerns backend-specific. [`Self::LegacyExternal`] preserves the
/// existing structured external-effect taxonomy while consumers migrate to
/// more precise domains where appropriate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldStateDomain {
    /// Child-interpreter existence and path-to-interpreter identity.
    InterpreterTopology,
    /// The name-to-command-object mapping in an interpreter.
    CommandBindings,
    /// Namespace creation, deletion, imports, exports, and search paths that
    /// affect ordinary command or variable lookup.
    NamespaceLookup,
    /// Namespace-specific `unknown` handlers used after a failed command
    /// lookup.
    NamespaceUnknown,
    /// Invocation enter, leave, and step traces attached to commands.
    ExecutionTraces,
    /// Read, write, unset, and array traces attached to Tcl variable cells.
    VariableTraces,
    /// Rename and delete traces attached to command names.
    CommandTraces,
    /// `TclOO` class, object, method, mixin, filter, and method-chain dispatch
    /// state.
    OoDispatch,
    /// Interpreter visibility, safety, hidden-command, and limit policy.
    InterpreterPolicy,
    /// Loaded packages and package-name/version resolution state.
    PackageState,
    /// Host services and capabilities exposed to an interpreter.
    HostCapabilities,
    /// Tcl variable cells when a descriptor cannot name a narrower cell.
    ///
    /// The subject is normally [`SubjectScope::Wildcard`].  Precise cells are
    /// a future memory-SSA concern; this domain deliberately establishes the
    /// conservative common baseline first.
    VariableStore,
    /// An established external side-effect target that does not yet have a
    /// dedicated world-state domain.
    LegacyExternal(SideEffectTarget),
}

/// How an operation accesses one [`WorldStateDomain`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectAccessMode {
    /// Reads the current state without changing it.
    Read,
    /// Changes the state without requiring its prior value as a declared
    /// dependency.
    Write,
    /// Reads and changes the state.
    ReadWrite,
    /// Invalidates all precise knowledge in the scoped state region.
    Clobber,
}

/// The registry declaration that supplied a mutable-world write.
///
/// A state transition may be a more precise, completion-qualified account of
/// one of these older effect declarations.  Keeping the source in the
/// coverage contract prevents a transition from accidentally suppressing an
/// unrelated explicit [`WorldEffectDescriptor`] access in the same domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldEffectWriteSource {
    /// The established command-table descriptor bridge.
    LegacyCommandTable,
    /// The established frame-effect descriptor bridge.
    LegacyFrame,
    /// One structured legacy side-effect target.
    LegacySideEffect(SideEffectTarget),
    /// An explicit `WorldEffectDescriptor` access.
    DeclaredWorldEffect,
}

/// One registry promise that completion-edge state transitions account for a
/// legacy or explicit world-effect write in a named domain.
///
/// This is deliberately both source- and domain-qualified.  A command may
/// have a transition for its binding identity while separately writing a
/// variable value, invoking a trace, or changing a host capability.  Only
/// the declared overlap is removed from the ordinary effect stream; reads,
/// callbacks, clobbers, and every other source remain observable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransitionEffectCoverage {
    /// The effect declaration whose write component is covered.
    pub source: WorldEffectWriteSource,
    /// Domains for which the transition is authoritative.
    pub domains: &'static [WorldStateDomain],
}

impl TransitionEffectCoverage {
    /// No transition covers an ordinary effect declaration.
    pub const NONE: &'static [Self] = &[];

    /// Whether this coverage declaration owns `source`'s write in `domain`.
    #[must_use]
    pub fn covers(self, source: WorldEffectWriteSource, domain: WorldStateDomain) -> bool {
        if self.source != source {
            return false;
        }
        let mut index = 0;
        while index < self.domains.len() {
            if self.domains[index] == domain {
                return true;
            }
            index += 1;
        }
        false
    }
}

/// Resolved transition coverage selected for one invocation.
///
/// This owned set is carried with [`crate::InvocationFacts`] so common CFG
/// consumers can retain the provenance of the effect/transition hand-off.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransitionEffectCoverages {
    entries: Vec<TransitionEffectCoverage>,
}

impl TransitionEffectCoverages {
    /// Return the selected registry coverage declarations.
    #[must_use]
    pub fn entries(&self) -> &[TransitionEffectCoverage] {
        &self.entries
    }

    /// Append declarations without duplicating an identical contract.
    pub fn extend(&mut self, entries: &'static [TransitionEffectCoverage]) {
        for entry in entries {
            if !self.entries.contains(entry) {
                self.entries.push(*entry);
            }
        }
    }

    /// Discard all declarations, used by a replacing form descriptor.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn covers(&self, source: WorldEffectWriteSource, domain: WorldStateDomain) -> bool {
        self.entries
            .iter()
            .copied()
            .any(|entry| entry.covers(source, domain))
    }
}

impl EffectAccessMode {
    /// Combine two accesses to the same scoped state region conservatively.
    #[must_use]
    pub fn combine(self, other: Self) -> Self {
        use EffectAccessMode::{Clobber, Read, ReadWrite, Write};

        match (self, other) {
            (Clobber, _) | (_, Clobber) => Clobber,
            (ReadWrite, _) | (_, ReadWrite) | (Read, Write) | (Write, Read) => ReadWrite,
            (Read, Read) => Read,
            (Write, Write) => Write,
        }
    }
}

/// The interpreter a world-state access refers to.
///
/// This owned counterpart to [`StaticInterpreterScope`] is used by resolved
/// descriptors, whose argument-sensitive resolver may determine a name at
/// invocation resolution time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InterpreterScope {
    /// The interpreter executing the invocation.
    Current,
    /// One named interpreter path.
    Named(Box<str>),
    /// An interpreter not statically known, or every interpreter.
    Any,
}

impl InterpreterScope {
    /// Construct a named-interpreter scope.
    #[must_use]
    pub fn named(name: impl Into<Box<str>>) -> Self {
        Self::Named(name.into())
    }
}

/// Static [`InterpreterScope`] data suitable for a command-spec constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaticInterpreterScope {
    /// The interpreter executing the invocation.
    Current,
    /// One named interpreter path.
    Named(&'static str),
    /// An interpreter not statically known, or every interpreter.
    Any,
}

impl From<StaticInterpreterScope> for InterpreterScope {
    fn from(scope: StaticInterpreterScope) -> Self {
        match scope {
            StaticInterpreterScope::Current => Self::Current,
            StaticInterpreterScope::Named(name) => Self::named(name),
            StaticInterpreterScope::Any => Self::Any,
        }
    }
}

/// The namespace a world-state access refers to.
///
/// This owned counterpart to [`StaticNamespaceScope`] is used by resolved
/// descriptors, whose argument-sensitive resolver may determine a namespace
/// at invocation resolution time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NamespaceScope {
    /// The current namespace at the invocation site.
    Current,
    /// One named namespace.
    Named(Box<str>),
    /// A namespace not statically known, or every namespace.
    Any,
}

impl NamespaceScope {
    /// Construct a named-namespace scope.
    #[must_use]
    pub fn named(name: impl Into<Box<str>>) -> Self {
        Self::Named(name.into())
    }
}

/// Static [`NamespaceScope`] data suitable for a command-spec constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaticNamespaceScope {
    /// The current namespace at the invocation site.
    Current,
    /// One named namespace.
    Named(&'static str),
    /// A namespace not statically known, or every namespace.
    Any,
}

impl From<StaticNamespaceScope> for NamespaceScope {
    fn from(scope: StaticNamespaceScope) -> Self {
        match scope {
            StaticNamespaceScope::Current => Self::Current,
            StaticNamespaceScope::Named(name) => Self::named(name),
            StaticNamespaceScope::Any => Self::Any,
        }
    }
}

/// The command, variable, package, object, or other subject an access names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubjectScope {
    /// One resolved subject name.
    Named(Box<str>),
    /// Every subject in the surrounding interpreter and namespace scopes.
    Wildcard,
}

impl SubjectScope {
    /// Construct a named-subject scope.
    #[must_use]
    pub fn named(name: impl Into<Box<str>>) -> Self {
        Self::Named(name.into())
    }
}

/// Static [`SubjectScope`] data suitable for a command-spec constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaticSubjectScope {
    /// One registry-known subject name.
    Named(&'static str),
    /// Every subject in the surrounding interpreter and namespace scopes.
    Wildcard,
}

impl From<StaticSubjectScope> for SubjectScope {
    fn from(scope: StaticSubjectScope) -> Self {
        match scope {
            StaticSubjectScope::Named(name) => Self::named(name),
            StaticSubjectScope::Wildcard => Self::Wildcard,
        }
    }
}

/// One resolved access to a scoped [`WorldStateDomain`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectAccess {
    /// The mutable state domain touched by the invocation.
    pub domain: WorldStateDomain,
    /// Whether the invocation reads, writes, or invalidates the domain.
    pub mode: EffectAccessMode,
    /// Interpreter scope for the access.
    pub interpreter: InterpreterScope,
    /// Namespace scope for the access.
    pub namespace: NamespaceScope,
    /// Subject scope within the domain.
    pub subject: SubjectScope,
}

impl EffectAccess {
    /// Construct a resolved world-state access.
    #[must_use]
    pub fn new(
        domain: WorldStateDomain,
        mode: EffectAccessMode,
        interpreter: InterpreterScope,
        namespace: NamespaceScope,
        subject: SubjectScope,
    ) -> Self {
        Self {
            domain,
            mode,
            interpreter,
            namespace,
            subject,
        }
    }

    /// A wildcard access to variable storage in the current Tcl context.
    #[must_use]
    pub fn variable_store_wildcard(mode: EffectAccessMode) -> Self {
        Self::new(
            WorldStateDomain::VariableStore,
            mode,
            InterpreterScope::Current,
            NamespaceScope::Current,
            SubjectScope::Wildcard,
        )
    }
}

/// One static world-state access declared in a command specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StaticEffectAccess {
    /// The mutable state domain touched by the invocation.
    pub domain: WorldStateDomain,
    /// Whether the invocation reads, writes, or invalidates the domain.
    pub mode: EffectAccessMode,
    /// Interpreter scope for the access.
    pub interpreter: StaticInterpreterScope,
    /// Namespace scope for the access.
    pub namespace: StaticNamespaceScope,
    /// Subject scope within the domain.
    pub subject: StaticSubjectScope,
}

impl StaticEffectAccess {
    /// A wildcard access to variable storage in the current Tcl context.
    pub const VARIABLE_STORE_WILDCARD: Self = Self {
        domain: WorldStateDomain::VariableStore,
        mode: EffectAccessMode::ReadWrite,
        interpreter: StaticInterpreterScope::Current,
        namespace: StaticNamespaceScope::Current,
        subject: StaticSubjectScope::Wildcard,
    };

    /// Construct a static world-state access.
    #[must_use]
    pub const fn new(
        domain: WorldStateDomain,
        mode: EffectAccessMode,
        interpreter: StaticInterpreterScope,
        namespace: StaticNamespaceScope,
        subject: StaticSubjectScope,
    ) -> Self {
        Self {
            domain,
            mode,
            interpreter,
            namespace,
            subject,
        }
    }

    fn resolve(self) -> EffectAccess {
        EffectAccess::new(
            self.domain,
            self.mode,
            self.interpreter.into(),
            self.namespace.into(),
            self.subject.into(),
        )
    }
}

/// A composable set of callbacks an invocation may synchronously perform.
///
/// A call may evaluate script, trigger a trace, and invoke a host callback in
/// one execution.  This is therefore a set rather than a single enum.  The
/// [`Self::UNKNOWN`] bit is a conservative "any callback" fact and dominates
/// unions with more precise callback kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallbackKinds(u8);

impl CallbackKinds {
    const COMMAND_PREFIX_BIT: u8 = 1 << 0;
    const SCRIPT_BIT: u8 = 1 << 1;
    const TRACE_BIT: u8 = 1 << 2;
    const HOST_BIT: u8 = 1 << 3;
    const UNKNOWN_BIT: u8 = 1 << 7;

    /// The operation does not call user- or host-provided code.
    pub const NONE: Self = Self(0);
    /// Invokes a Tcl command prefix supplied as data.
    pub const COMMAND_PREFIX: Self = Self(Self::COMMAND_PREFIX_BIT);
    /// Evaluates Tcl script text.
    pub const SCRIPT: Self = Self(Self::SCRIPT_BIT);
    /// Invokes a Tcl trace callback.
    pub const TRACE: Self = Self(Self::TRACE_BIT);
    /// Invokes a host callback outside Tcl command dispatch.
    pub const HOST: Self = Self(Self::HOST_BIT);
    /// A callback may occur, but its source is not known precisely.
    pub const UNKNOWN: Self = Self(Self::UNKNOWN_BIT);

    /// Whether this set contains every callback kind in `other`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.is_unknown() || (self.0 & other.0) == other.0
    }

    /// Whether this set is the conservative any-callback fact.
    #[must_use]
    pub const fn is_unknown(self) -> bool {
        (self.0 & Self::UNKNOWN_BIT) != 0
    }

    /// Conservatively union two callback-kind sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        if self.is_unknown() || other.is_unknown() {
            Self::UNKNOWN
        } else {
            Self(self.0 | other.0)
        }
    }
}

/// The extent to which a callback may re-enter interpreters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Reentrancy {
    /// The operation cannot re-enter Tcl execution.
    Never,
    /// The operation may re-enter the current interpreter.
    CurrentInterpreter,
    /// The operation may re-enter another or an unknown interpreter.
    AnyInterpreter,
}

/// Callback and re-entrancy behaviour for one effect footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallbackEffect {
    /// The possible callback kinds.
    pub kinds: CallbackKinds,
    /// The possible interpreter re-entrancy.
    pub reentrancy: Reentrancy,
}

impl CallbackEffect {
    /// A callback-free footprint.
    pub const NONE: Self = Self {
        kinds: CallbackKinds::NONE,
        reentrancy: Reentrancy::Never,
    };

    /// Conservatively combine callback behaviour from two footprints.
    #[must_use]
    pub fn combine(self, other: Self) -> Self {
        let reentrancy = if self.reentrancy >= other.reentrancy {
            self.reentrancy
        } else {
            other.reentrancy
        };
        Self {
            kinds: self.kinds.union(other.kinds),
            reentrancy,
        }
    }
}

impl Default for CallbackEffect {
    fn default() -> Self {
        Self::NONE
    }
}

/// A static collection of state accesses and callback behaviour.
#[derive(Debug, Clone, Copy)]
pub struct StaticEffectFootprint {
    /// Unconditional accesses for this command form.
    pub accesses: &'static [StaticEffectAccess],
    /// Unconditional callback behaviour for this command form.
    pub callback: CallbackEffect,
}

impl StaticEffectFootprint {
    /// An empty static footprint.
    pub const EMPTY: Self = Self {
        accesses: &[],
        callback: CallbackEffect::NONE,
    };
}

/// Legacy effect data retained while callers migrate to [`EffectAccess`].
///
/// It is intentionally present in every resolved footprint generated through
/// [`EffectFootprint::from_legacy`].  This makes legacy facts visible to a
/// common consumer and prevents an incomplete state-domain mapping from
/// silently erasing information.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyEffectBridge {
    /// Whether a **command-binding transition** contributes to this
    /// footprint.
    ///
    /// This is a boolean, not a list of command-table effect words, because
    /// there is one transition vocabulary now (centralisation ledger C8):
    /// *what* the call did to the command table is the
    /// [`crate::CommandBindingTransition`] facts, read from
    /// [`crate::ResolvedInvocation::state_transitions`]. All the effect
    /// bridge needs is *whether* it did, so it can add the wildcard
    /// command-binding write and the synchronous command-trace read.
    pub command_table_mutation: bool,
    /// Frame-crossing descriptors contributing to this footprint.
    pub frame_effects: Vec<FrameEffectSpec>,
    /// Structured side-effect declarations contributing to this footprint.
    pub side_effects: Vec<SideEffect>,
}

impl LegacyEffectBridge {
    fn add_frame_effect(&mut self, effect: FrameEffectSpec) {
        if !self.frame_effects.contains(&effect) {
            self.frame_effects.push(effect);
        }
    }

    fn add_side_effect(&mut self, effect: SideEffect) {
        if !self.side_effects.contains(&effect) {
            self.side_effects.push(effect);
        }
    }

    fn extend(&mut self, other: &Self) {
        self.command_table_mutation |= other.command_table_mutation;
        for effect in other.frame_effects.iter().copied() {
            self.add_frame_effect(effect);
        }
        for effect in other.side_effects.iter().copied() {
            self.add_side_effect(effect);
        }
    }
}

/// A fully resolved, owned effect summary for one invocation.
///
/// The static part of a descriptor and its optional argument-dependent
/// resolver are combined before this type reaches a compiler pass.  A pass can
/// therefore use state domains and callback facts without inspecting a command
/// name or raw command-spec field.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectFootprint {
    accesses: Vec<EffectAccess>,
    callback: CallbackEffect,
    legacy: LegacyEffectBridge,
}

impl EffectFootprint {
    /// The conservative effect of an invocation without a registry-owned
    /// closed-world declaration.
    ///
    /// Missing metadata never means a command is pure: a generic invocation
    /// can call an alias, `unknown`, a trace, or host code that reads and
    /// mutates arbitrary interpreter state. Common compiler consumers project
    /// this re-entrant callback as an all-world use and clobber until a
    /// resolved command descriptor proves a narrower footprint.
    #[must_use]
    pub fn conservative_unknown_invocation() -> Self {
        let mut footprint = Self::default();
        footprint.add_callback(CallbackEffect {
            kinds: CallbackKinds::UNKNOWN,
            reentrancy: Reentrancy::AnyInterpreter,
        });
        footprint
    }

    /// Return the coalesced state accesses in this footprint.
    #[must_use]
    pub fn accesses(&self) -> &[EffectAccess] {
        &self.accesses
    }

    /// Return the footprint's callback and re-entrancy behaviour.
    #[must_use]
    pub const fn callback(&self) -> CallbackEffect {
        self.callback
    }

    /// Whether this invocation is a common-world barrier until a later
    /// interprocedural callback summary proves a narrower footprint.
    ///
    /// A re-entrant callback can mutate command bindings, traces, namespaces,
    /// `TclOO` dispatch, variable cells, or host state before the invoking
    /// operation resumes.  A common pass must therefore invalidate or guard
    /// every relevant world-state assumption at this boundary.  It may narrow
    /// the barrier only with a summary of the callback itself, never merely
    /// because this footprint lists a subset of directly accessed domains.
    #[must_use]
    pub const fn requires_world_barrier(&self) -> bool {
        !matches!(self.callback.reentrancy, Reentrancy::Never)
    }

    /// Return the existing registry descriptors bridged into this footprint.
    #[must_use]
    pub const fn legacy(&self) -> &LegacyEffectBridge {
        &self.legacy
    }

    /// Add one access, coalescing an equivalent state scope conservatively.
    pub fn add_access(&mut self, access: EffectAccess) {
        if let Some(existing) = self.accesses.iter_mut().find(|existing| {
            existing.domain == access.domain
                && existing.interpreter == access.interpreter
                && existing.namespace == access.namespace
                && existing.subject == access.subject
        }) {
            existing.mode = existing.mode.combine(access.mode);
        } else {
            self.accesses.push(access);
        }
    }

    /// Add callback behaviour, preserving the strongest re-entrancy fact.
    pub fn add_callback(&mut self, callback: CallbackEffect) {
        self.callback = self.callback.combine(callback);
    }

    /// Combine another resolved footprint into this one.
    pub fn extend(&mut self, other: Self) {
        for access in other.accesses {
            self.add_access(access);
        }
        self.add_callback(other.callback);
        self.legacy.extend(&other.legacy);
    }

    fn remove_declared_writes_covered_by(&mut self, coverage: &TransitionEffectCoverages) {
        self.accesses.retain_mut(|access| {
            if !coverage.covers(WorldEffectWriteSource::DeclaredWorldEffect, access.domain) {
                return true;
            }
            access.mode = match access.mode {
                EffectAccessMode::Write => return false,
                EffectAccessMode::ReadWrite => EffectAccessMode::Read,
                EffectAccessMode::Read | EffectAccessMode::Clobber => access.mode,
            };
            true
        });
    }

    fn from_static(
        static_footprint: StaticEffectFootprint,
        coverage: &TransitionEffectCoverages,
    ) -> Self {
        let mut footprint = Self::default();
        for access in static_footprint.accesses.iter().copied() {
            footprint.add_access_if_uncovered(
                access.resolve(),
                WorldEffectWriteSource::DeclaredWorldEffect,
                coverage,
            );
        }
        footprint.add_callback(static_footprint.callback);
        footprint
    }

    /// Resolve and bridge the established registry effect descriptors.
    #[must_use]
    pub fn from_legacy(
        command_table_mutation: bool,
        frame_effect: Option<FrameEffectSpec>,
        side_effects: &[SideEffect],
    ) -> Self {
        Self::from_legacy_with_transition_coverage(
            command_table_mutation,
            frame_effect,
            side_effects,
            &TransitionEffectCoverages::default(),
        )
    }

    /// Resolve legacy descriptors while removing only write components that
    /// a selected completion-edge transition explicitly covers.
    #[must_use]
    pub fn from_legacy_with_transition_coverage(
        command_table_mutation: bool,
        frame_effect: Option<FrameEffectSpec>,
        side_effects: &[SideEffect],
        coverage: &TransitionEffectCoverages,
    ) -> Self {
        let mut footprint = Self::default();
        if command_table_mutation {
            footprint.bridge_command_table_mutation(coverage);
        }
        if let Some(effect) = frame_effect {
            footprint.bridge_frame_effect(effect, coverage);
        }
        for effect in side_effects.iter().copied() {
            footprint.bridge_side_effect(effect, coverage);
        }
        footprint
    }

    fn bridge_command_table_mutation(&mut self, coverage: &TransitionEffectCoverages) {
        self.legacy.command_table_mutation = true;
        self.add_access_if_uncovered(
            current_wildcard(WorldStateDomain::CommandBindings, EffectAccessMode::Write),
            WorldEffectWriteSource::LegacyCommandTable,
            coverage,
        );
        // Defining, replacing, renaming, deleting, or aliasing a command can
        // run a matching command trace synchronously. The bridge is keyed by
        // the resolved transition facts, never by a command spelling.
        self.add_access(current_wildcard(
            WorldStateDomain::CommandTraces,
            EffectAccessMode::Read,
        ));
        self.add_callback(CallbackEffect {
            kinds: CallbackKinds::TRACE,
            reentrancy: Reentrancy::CurrentInterpreter,
        });
    }

    fn bridge_frame_effect(
        &mut self,
        effect: FrameEffectSpec,
        coverage: &TransitionEffectCoverages,
    ) {
        self.legacy.add_frame_effect(effect);
        match effect.layout {
            FrameArgLayout::AliasPairs => self.add_access_if_uncovered(
                EffectAccess::variable_store_wildcard(EffectAccessMode::ReadWrite),
                WorldEffectWriteSource::LegacyFrame,
                coverage,
            ),
            FrameArgLayout::ScriptInSelectedFrame | FrameArgLayout::ScriptInCurrentFrame => {
                self.add_access(EffectAccess::variable_store_wildcard(
                    EffectAccessMode::Clobber,
                ));
                self.clobber_reentrant_tcl_world();
                self.add_callback(CallbackEffect {
                    kinds: CallbackKinds::SCRIPT,
                    reentrancy: Reentrancy::CurrentInterpreter,
                });
            }
            FrameArgLayout::OpaqueCallerVars => self.add_access(
                EffectAccess::variable_store_wildcard(EffectAccessMode::Clobber),
            ),
        }
    }

    fn clobber_reentrant_tcl_world(&mut self) {
        for domain in [
            WorldStateDomain::InterpreterTopology,
            WorldStateDomain::CommandBindings,
            WorldStateDomain::NamespaceLookup,
            WorldStateDomain::NamespaceUnknown,
            WorldStateDomain::ExecutionTraces,
            WorldStateDomain::VariableTraces,
            WorldStateDomain::CommandTraces,
            WorldStateDomain::OoDispatch,
            WorldStateDomain::InterpreterPolicy,
            WorldStateDomain::PackageState,
            WorldStateDomain::HostCapabilities,
        ] {
            self.add_access(current_wildcard(domain, EffectAccessMode::Clobber));
        }
    }

    fn bridge_side_effect(&mut self, effect: SideEffect, coverage: &TransitionEffectCoverages) {
        self.legacy.add_side_effect(effect);
        let Some(mode) = mode_from_side_effect(effect) else {
            return;
        };
        let domain = match effect.target {
            SideEffectTarget::Variable => WorldStateDomain::VariableStore,
            SideEffectTarget::ProcDefinition => WorldStateDomain::CommandBindings,
            SideEffectTarget::NamespaceState => WorldStateDomain::NamespaceLookup,
            SideEffectTarget::InterpState => WorldStateDomain::InterpreterPolicy,
            SideEffectTarget::Unknown => WorldStateDomain::HostCapabilities,
            target => WorldStateDomain::LegacyExternal(target),
        };
        let mode = if effect.target == SideEffectTarget::Unknown {
            EffectAccessMode::Clobber
        } else {
            mode
        };
        self.add_access_if_uncovered(
            current_wildcard(domain, mode),
            WorldEffectWriteSource::LegacySideEffect(effect.target),
            coverage,
        );
    }

    fn add_access_if_uncovered(
        &mut self,
        mut access: EffectAccess,
        source: WorldEffectWriteSource,
        coverage: &TransitionEffectCoverages,
    ) {
        if coverage.covers(source, access.domain) {
            access.mode = match access.mode {
                EffectAccessMode::Write => return,
                EffectAccessMode::ReadWrite => EffectAccessMode::Read,
                EffectAccessMode::Read | EffectAccessMode::Clobber => access.mode,
            };
        }
        self.add_access(access);
    }
}

fn current_wildcard(domain: WorldStateDomain, mode: EffectAccessMode) -> EffectAccess {
    EffectAccess::new(
        domain,
        mode,
        InterpreterScope::Current,
        NamespaceScope::Current,
        SubjectScope::Wildcard,
    )
}

fn mode_from_side_effect(effect: SideEffect) -> Option<EffectAccessMode> {
    match (effect.reads, effect.writes) {
        (false, false) => None,
        (true, false) => Some(EffectAccessMode::Read),
        (false, true) => Some(EffectAccessMode::Write),
        (true, true) => Some(EffectAccessMode::ReadWrite),
    }
}

/// Resolver for a descriptor whose effect depends on supplied word facts.
///
/// `args` is the complete post-command word view, including a resolved
/// subcommand word when there is one.  A resolver can inspect an argument's
/// runtime value only through [`InvocationArguments::literal_at`]. Dynamic,
/// expanded, and opaque words have no string accessor, so callbacks cannot
/// accidentally use source spelling as an evaluated Tcl value.
///
/// [`WorldEffectDescriptor::resolve`] retains static effects and adds the
/// conservative unknown-invocation barrier whenever such a callback receives
/// a non-literal argument. A resolver may still contribute additional safe
/// facts, but it cannot understate effects because a dynamic value selected a
/// different branch.
pub type WorldEffectResolver = for<'w> fn(InvocationArguments<'w>) -> EffectFootprint;

/// The conservative effect added when an argument-sensitive resolver receives
/// a non-literal argument.
///
/// Most descriptors cannot know which runtime branch a dynamic word selects,
/// so they use [`Self::ConservativeUnknownInvocation`].  A descriptor may
/// instead declare an explicit wildcard footprint when dynamic operands still
/// cannot invoke callbacks, but do have a known direct state envelope.
#[derive(Debug, Clone, Copy)]
pub enum WorldEffectDynamicFallback {
    /// Preserve the established generic-invocation barrier.
    ConservativeUnknownInvocation,
    /// Add a registry-declared direct-effect envelope without inventing a
    /// callback that the invocation does not perform.
    Declared(StaticEffectFootprint),
}

/// Registry data for a command, subcommand, or concrete invocation form.
///
/// The static footprint supplies unconditional effects.  `resolver`, when
/// present, contributes further facts selected from the invocation words.  A
/// resolver cannot weaken static or legacy bridge effects, which keeps the
/// composition monotonic and safe for common analyses.
#[derive(Debug, Clone, Copy)]
pub struct WorldEffectDescriptor {
    /// How this descriptor interacts with less-specific command descriptors.
    pub composition: WorldEffectComposition,
    /// Unconditional state effects.
    pub static_footprint: StaticEffectFootprint,
    /// Optional argument-dependent effect resolver.
    pub resolver: Option<WorldEffectResolver>,
    /// Conservative policy for non-literal arguments supplied to `resolver`.
    pub dynamic_fallback: WorldEffectDynamicFallback,
}

impl WorldEffectDescriptor {
    /// An explicitly effect-free, closed-world descriptor with no resolver.
    ///
    /// This is distinct from absent metadata, which resolves to
    /// [`EffectFootprint::conservative_unknown_invocation`] at the resolved
    /// invocation boundary.
    pub const EMPTY: Self = Self {
        composition: WorldEffectComposition::Extend,
        static_footprint: StaticEffectFootprint::EMPTY,
        resolver: None,
        dynamic_fallback: WorldEffectDynamicFallback::ConservativeUnknownInvocation,
    };

    /// Resolve the descriptor for one structured post-command argument view.
    #[must_use]
    pub fn resolve(self, args: InvocationArguments<'_>) -> EffectFootprint {
        self.resolve_with_transition_coverage(args, &TransitionEffectCoverages::default())
    }

    /// Resolve this declaration while respecting selected state-transition
    /// coverage. Only explicitly covered write components are omitted.
    #[must_use]
    pub fn resolve_with_transition_coverage(
        self,
        args: InvocationArguments<'_>,
        coverage: &TransitionEffectCoverages,
    ) -> EffectFootprint {
        let mut footprint = EffectFootprint::from_static(self.static_footprint, coverage);
        if let Some(resolver) = self.resolver {
            let mut resolved = resolver(args);
            resolved.remove_declared_writes_covered_by(coverage);
            footprint.extend(resolved);
            if args.has_non_literal() {
                let fallback = match self.dynamic_fallback {
                    WorldEffectDynamicFallback::ConservativeUnknownInvocation => {
                        EffectFootprint::conservative_unknown_invocation()
                    }
                    WorldEffectDynamicFallback::Declared(declared) => {
                        EffectFootprint::from_static(declared, coverage)
                    }
                };
                footprint.extend(fallback);
            }
        }
        footprint
    }
}

/// How a form or subcommand effect descriptor composes with its parent.
///
/// [`Self::Extend`] is the default: parent effects remain true for every child
/// form, while a child contributes additional facts.  [`Self::Replace`] is an
/// explicit assertion by the registry author that the descriptor completely
/// and more precisely describes this shape; it is the only way to discard a
/// broader parent footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldEffectComposition {
    /// Preserve parent effects and add this descriptor's effects.
    Extend,
    /// Discard parent declared effects before applying this descriptor.
    Replace,
}

/// The cheap command/subcommand/form descriptor chain on a resolved call.
///
/// It retains all registry descriptors without allocating a resolved
/// [`EffectFootprint`] until a common pass asks for one.  Resolution applies
/// command, then subcommand, then form descriptors; each descriptor's
/// [`WorldEffectComposition`] controls whether it extends or deliberately
/// replaces the less-specific declaration.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResolvedWorldEffects {
    /// Parent command descriptor, when declared.
    pub command: Option<WorldEffectDescriptor>,
    /// Resolved subcommand descriptor, when declared.
    pub subcommand: Option<WorldEffectDescriptor>,
    /// Matched invocation-form descriptor, when declared.
    pub form: Option<WorldEffectDescriptor>,
}

impl ResolvedWorldEffects {
    /// Whether this resolved invocation has an explicit registry-owned
    /// world-effect declaration at any specificity.
    ///
    /// A declaration may be [`WorldEffectDescriptor::EMPTY`], which is an
    /// explicit closed proof of effect-freedom. By contrast, an entirely
    /// absent chain must remain a conservative unknown invocation.
    #[must_use]
    pub const fn is_declared(self) -> bool {
        self.command.is_some() || self.subcommand.is_some() || self.form.is_some()
    }

    /// Resolve this descriptor chain for one structured post-command argument
    /// view.
    #[must_use]
    pub fn resolve(self, args: InvocationArguments<'_>) -> EffectFootprint {
        self.resolve_with_transition_coverage(args, &TransitionEffectCoverages::default())
    }

    /// Resolve the descriptor chain while applying registry-declared
    /// completion-edge transition coverage to matching write components.
    #[must_use]
    pub fn resolve_with_transition_coverage(
        self,
        args: InvocationArguments<'_>,
        coverage: &TransitionEffectCoverages,
    ) -> EffectFootprint {
        let mut footprint = EffectFootprint::default();
        for descriptor in [self.command, self.subcommand, self.form]
            .into_iter()
            .flatten()
        {
            let resolved = descriptor.resolve_with_transition_coverage(args, coverage);
            if descriptor.composition == WorldEffectComposition::Replace {
                footprint = resolved;
            } else {
                footprint.extend(resolved);
            }
        }
        footprint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_modes_coalesce_without_losing_a_write() {
        let mut footprint = EffectFootprint::default();
        footprint.add_access(EffectAccess::variable_store_wildcard(
            EffectAccessMode::Read,
        ));
        footprint.add_access(EffectAccess::variable_store_wildcard(
            EffectAccessMode::Write,
        ));

        assert_eq!(footprint.accesses().len(), 1);
        assert_eq!(footprint.accesses()[0].mode, EffectAccessMode::ReadWrite);
    }

    #[test]
    fn a_command_table_mutation_bridges_the_write_and_the_trace_callback() {
        let footprint = EffectFootprint::from_legacy(true, None, &[]);

        assert!(footprint.legacy().command_table_mutation);
        assert!(footprint.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::CommandBindings
                && access.mode == EffectAccessMode::Write
        }));
        assert!(footprint.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::CommandTraces
                && access.mode == EffectAccessMode::Read
        }));
        assert!(footprint.callback().kinds.contains(CallbackKinds::TRACE));
        assert_eq!(
            footprint.callback().reentrancy,
            Reentrancy::CurrentInterpreter
        );
    }

    #[test]
    fn no_command_table_mutation_bridges_no_command_trace_read() {
        let footprint = EffectFootprint::from_legacy(false, None, &[]);
        assert!(!footprint.legacy().command_table_mutation);
        assert!(!footprint.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::CommandTraces
                || access.domain == WorldStateDomain::CommandBindings
        }));
        assert!(!footprint.callback().kinds.contains(CallbackKinds::TRACE));
    }

    #[test]
    fn frame_script_bridge_clobbers_tcl_world_and_preserves_frame_data() {
        let frame = FrameEffectSpec {
            level_word: crate::frame_effect::FrameLevelWord::LeadingProbe,
            layout: FrameArgLayout::ScriptInSelectedFrame,
        };
        let footprint = EffectFootprint::from_legacy(false, Some(frame), &[]);

        assert_eq!(footprint.legacy().frame_effects, vec![frame]);
        assert!(footprint.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::CommandBindings
                && access.mode == EffectAccessMode::Clobber
        }));
        assert!(footprint.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableStore
                && access.mode == EffectAccessMode::Clobber
        }));
        assert!(footprint.callback().kinds.contains(CallbackKinds::SCRIPT));
        assert!(footprint.requires_world_barrier());
    }

    #[test]
    fn coverage_is_source_qualified_and_preserves_unrelated_declared_writes() {
        const VARIABLE_DOMAIN: &[WorldStateDomain] = &[WorldStateDomain::VariableStore];
        const LEGACY_VARIABLE_WRITE: &[TransitionEffectCoverage] = &[TransitionEffectCoverage {
            source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::Variable),
            domains: VARIABLE_DOMAIN,
        }];
        const DECLARED_VARIABLE_WRITE: StaticEffectFootprint = StaticEffectFootprint {
            accesses: &[StaticEffectAccess::VARIABLE_STORE_WILDCARD],
            callback: CallbackEffect::NONE,
        };

        let mut coverage = TransitionEffectCoverages::default();
        coverage.extend(LEGACY_VARIABLE_WRITE);
        let legacy = EffectFootprint::from_legacy_with_transition_coverage(
            false,
            None,
            &[SideEffect {
                target: SideEffectTarget::Variable,
                writes: true,
                connection_side: crate::side_effects::ConnectionSide::None,
                ..SideEffect::DEFAULT
            }],
            &coverage,
        );
        assert!(legacy.accesses().is_empty());

        let declared = WorldEffectDescriptor {
            composition: WorldEffectComposition::Extend,
            static_footprint: DECLARED_VARIABLE_WRITE,
            resolver: None,
            dynamic_fallback: WorldEffectDynamicFallback::ConservativeUnknownInvocation,
        }
        .resolve_with_transition_coverage(InvocationArguments::literals(&[]), &coverage);
        assert!(declared.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableStore
                && access.mode == EffectAccessMode::ReadWrite
        }));
    }

    #[test]
    fn callback_kinds_union_without_losing_simultaneous_observers() {
        let mut footprint = EffectFootprint::default();
        footprint.add_callback(CallbackEffect {
            kinds: CallbackKinds::SCRIPT,
            reentrancy: Reentrancy::CurrentInterpreter,
        });
        footprint.add_callback(CallbackEffect {
            kinds: CallbackKinds::TRACE,
            reentrancy: Reentrancy::Never,
        });
        footprint.add_callback(CallbackEffect {
            kinds: CallbackKinds::HOST,
            reentrancy: Reentrancy::AnyInterpreter,
        });

        assert!(footprint.callback().kinds.contains(CallbackKinds::SCRIPT));
        assert!(footprint.callback().kinds.contains(CallbackKinds::TRACE));
        assert!(footprint.callback().kinds.contains(CallbackKinds::HOST));
        assert_eq!(footprint.callback().reentrancy, Reentrancy::AnyInterpreter);
        assert!(footprint.requires_world_barrier());
        assert!(
            CallbackKinds::UNKNOWN.contains(CallbackKinds::SCRIPT),
            "an unknown callback is an any-callback fact"
        );
    }
}
