// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target-independent plans for Tcl values at representation boundaries.
//!
//! These types describe the obligations that analyses and legalisation passes
//! must establish before a backend chooses an encoding.  They deliberately do
//! not mention a command, instruction set, object layout, or calling convention.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Whether a dual-representation value can be mutated without copying.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharingState {
    /// This compilation-visible reference is the sole owner.
    Unique,
    /// Another reference may observe the value.
    Shared,
}

/// Which cached representation a mutation invalidated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidatedRepresentation {
    /// The string representation was invalidated by an internal mutation.
    String,
    /// The typed internal representation was invalidated by a string mutation.
    Internal,
}

/// The ownership transition required before mutating a value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutationTransition {
    /// The uniquely owned value can be changed in place.
    InPlace,
    /// The shared value must first be copied into a unique value.
    ///
    /// This is an obligation on the eventual lowering.  Recording the
    /// transition here does not itself clone an external runtime object.
    CopyOnWrite,
}

/// A change to the set of currently valid representations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepresentationTransition {
    /// A string representation was derived without changing the current intrep.
    MaterialisedString,
    /// An internal representation was derived without invalidating the string.
    ShimmeredToInternal,
    /// A mutation invalidated the other representation.
    Mutated {
        /// Whether the mutation was in-place or required copy-on-write.
        ownership: MutationTransition,
        /// The cached representation invalidated by the mutation.
        invalidated: InvalidatedRepresentation,
    },
}

/// Failure to construct or transition a dual-representation value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepresentationError {
    /// A value cannot have neither a string nor an internal representation.
    MissingRepresentation,
}

impl fmt::Display for RepresentationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRepresentation => f.write_str("a value must retain one representation"),
        }
    }
}

impl Error for RepresentationError {}

/// A Tcl value with independently cached string and typed internal forms.
///
/// `K` identifies the internal-representation kind, while `V` carries the
/// representation itself.  Backends may choose concrete types for both.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DualRepresentation<K, V> {
    string: Option<String>,
    internal: Option<(K, V)>,
    sharing: SharingState,
}

impl<K, V> DualRepresentation<K, V> {
    /// Constructs a value and rejects the impossible representation-less state.
    pub fn new(
        string: Option<String>,
        internal: Option<(K, V)>,
        sharing: SharingState,
    ) -> Result<Self, RepresentationError> {
        if string.is_none() && internal.is_none() {
            return Err(RepresentationError::MissingRepresentation);
        }
        Ok(Self {
            string,
            internal,
            sharing,
        })
    }

    /// Constructs a unique value whose only current form is a string.
    pub fn from_string(value: impl Into<String>) -> Self {
        Self {
            string: Some(value.into()),
            internal: None,
            sharing: SharingState::Unique,
        }
    }

    /// Constructs a unique value whose only current form is a typed intrep.
    pub fn from_internal(kind: K, value: V) -> Self {
        Self {
            string: None,
            internal: Some((kind, value)),
            sharing: SharingState::Unique,
        }
    }

    /// Returns the current string representation, if materialised.
    pub fn string(&self) -> Option<&str> {
        self.string.as_deref()
    }

    /// Returns the current typed internal representation, if present.
    pub fn internal(&self) -> Option<(&K, &V)> {
        self.internal.as_ref().map(|(kind, value)| (kind, value))
    }

    /// Returns the sharing state used to decide whether mutation needs a copy.
    pub const fn sharing(&self) -> SharingState {
        self.sharing
    }

    /// Records that another reference may observe this value.
    pub fn mark_shared(&mut self) {
        self.sharing = SharingState::Shared;
    }

    /// Records that analysis has produced an independent unique copy.
    ///
    /// Callers may use this only after the copy required by
    /// [`MutationTransition::CopyOnWrite`] has been provided by a lower layer.
    pub fn mark_unique_copy(&mut self) {
        self.sharing = SharingState::Unique;
    }

    /// Installs a derived string while preserving the current intrep.
    pub fn materialise_string(&mut self, value: impl Into<String>) -> RepresentationTransition {
        self.string = Some(value.into());
        RepresentationTransition::MaterialisedString
    }

    /// Installs a derived intrep while preserving the current string.
    pub fn shimmer_to_internal(&mut self, kind: K, value: V) -> RepresentationTransition {
        self.internal = Some((kind, value));
        RepresentationTransition::ShimmeredToInternal
    }

    /// Mutates the string value and invalidates the cached intrep.
    pub fn mutate_string(&mut self, value: impl Into<String>) -> RepresentationTransition {
        let ownership = self.prepare_mutation();
        self.string = Some(value.into());
        self.internal = None;
        RepresentationTransition::Mutated {
            ownership,
            invalidated: InvalidatedRepresentation::Internal,
        }
    }

    /// Mutates the intrep and invalidates the cached string.
    pub fn mutate_internal(&mut self, kind: K, value: V) -> RepresentationTransition {
        let ownership = self.prepare_mutation();
        self.string = None;
        self.internal = Some((kind, value));
        RepresentationTransition::Mutated {
            ownership,
            invalidated: InvalidatedRepresentation::String,
        }
    }

    fn prepare_mutation(&mut self) -> MutationTransition {
        match self.sharing {
            SharingState::Unique => MutationTransition::InPlace,
            SharingState::Shared => {
                self.sharing = SharingState::Unique;
                MutationTransition::CopyOnWrite
            }
        }
    }
}

/// Where a Tcl variable's authoritative value lives while compiled code runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VarStorage {
    /// The runtime frame is authoritative; compiled code does not cache it.
    FrameResident,
    /// A native slot caches a value that also has a runtime-frame home.
    CachedSlot,
    /// A native slot has a known recipe for creating its runtime-frame home.
    MaterializableSlot,
    /// The value exists only in native state and cannot cross opaque Tcl code.
    NativeOnly,
}

/// A semantic boundary across which runtime code may observe or change state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryKind {
    /// Registry facts prove that the operation is closed and non-reentrant.
    Closed,
    /// Dispatch goes through the shared generic runtime invocation path.
    GenericRuntime,
    /// The invoked operation cannot be resolved precisely.
    Unknown,
    /// The operation may invoke Tcl callbacks or traces before returning.
    Reentrant,
}

impl BoundaryKind {
    /// Returns whether native cached state must be made runtime-observable.
    #[must_use]
    pub const fn exposes_runtime_state(self) -> bool {
        !matches!(self, Self::Closed)
    }
}

/// A synchronisation action placed around a semantic boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryAction {
    /// Publish the current native value into runtime-visible storage first.
    MaterialiseBefore,
    /// Invalidate the native cache after the operation returns.
    InvalidateAfter,
    /// Reload the native cache from runtime-visible storage after invalidation.
    ReloadAfter,
}

/// A proposed storage synchronisation plan for one semantic boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundaryPlan {
    /// Storage selected for the value crossing the boundary.
    pub storage: VarStorage,
    /// The semantic kind of boundary being crossed.
    pub boundary: BoundaryKind,
    /// Ordered synchronisation actions around the boundary.
    pub actions: Vec<BoundaryAction>,
}

/// Why a variable storage boundary plan is not safe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryPlanError {
    /// Native-only storage cannot be exposed to an opaque runtime operation.
    NativeOnlyEscapes,
    /// A required synchronisation action is missing.
    MissingAction(BoundaryAction),
    /// Synchronisation actions occur in an order that cannot preserve state.
    InvalidActionOrder,
    /// A closed boundary contains unnecessary synchronisation actions.
    UnexpectedAction,
}

impl fmt::Display for BoundaryPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NativeOnlyEscapes => {
                f.write_str("native-only storage cannot cross a runtime-visible boundary")
            }
            Self::MissingAction(action) => write!(f, "boundary plan is missing {action:?}"),
            Self::InvalidActionOrder => f.write_str("boundary actions are not in semantic order"),
            Self::UnexpectedAction => {
                f.write_str("closed frame-resident boundary has unexpected actions")
            }
        }
    }
}

impl Error for BoundaryPlanError {}

impl BoundaryPlan {
    /// Validates materialisation, invalidation, and reload obligations.
    pub fn validate(&self) -> Result<(), BoundaryPlanError> {
        if !self.boundary.exposes_runtime_state() {
            if !self.actions.is_empty() {
                return Err(BoundaryPlanError::UnexpectedAction);
            }
            return Ok(());
        }

        match self.storage {
            VarStorage::FrameResident => {
                if self.actions.is_empty() {
                    Ok(())
                } else {
                    Err(BoundaryPlanError::UnexpectedAction)
                }
            }
            VarStorage::NativeOnly => Err(BoundaryPlanError::NativeOnlyEscapes),
            VarStorage::CachedSlot | VarStorage::MaterializableSlot => {
                let materialise = self.position(BoundaryAction::MaterialiseBefore)?;
                let invalidate = self.position(BoundaryAction::InvalidateAfter)?;
                let reload = self.position(BoundaryAction::ReloadAfter)?;
                if self.actions.len() != 3 {
                    Err(BoundaryPlanError::UnexpectedAction)
                } else if materialise < invalidate && invalidate < reload {
                    Ok(())
                } else {
                    Err(BoundaryPlanError::InvalidActionOrder)
                }
            }
        }
    }

    fn position(&self, action: BoundaryAction) -> Result<usize, BoundaryPlanError> {
        self.actions
            .iter()
            .position(|candidate| *candidate == action)
            .ok_or(BoundaryPlanError::MissingAction(action))
    }
}

/// Semantic ownership role of a runtime-visible value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnershipRole {
    /// An invocation argument is borrowed for the duration of the call.
    BorrowedArgument,
    /// The owned result value of a Tcl completion.
    CompletionResult,
    /// The owned options dictionary of a Tcl completion.
    CompletionOptions,
    /// An owned temporary whose lifetime is internal to a compiled operation.
    Temporary,
}

/// Current state of an entry in an [`OwnershipLedger`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnershipState {
    /// The plan currently owns the value and must eventually release it.
    Owned,
    /// The owned reference has been released exactly once.
    Released,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OwnershipRecord {
    role: OwnershipRole,
    state: OwnershipState,
}

/// Compile-time ledger proving that owned values are released exactly once.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnershipLedger<R> {
    records: BTreeMap<R, OwnershipRecord>,
}

/// Failure while building or validating an ownership ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnershipError {
    /// The resource already has a ledger entry.
    DuplicateResource,
    /// The resource does not have a ledger entry.
    UnknownResource,
    /// The requested operation does not match the resource's recorded role.
    RoleMismatch,
    /// A borrowed argument was incorrectly treated as owned storage.
    BorrowedResource,
    /// The resource was released more than once.
    DoubleRelease,
    /// At least one owned resource remains live at the end of the plan.
    LeakedResource,
    /// Result and options refer to the same ownership identity.
    AliasedCompletionParts,
}

impl fmt::Display for OwnershipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateResource => f.write_str("resource is already tracked"),
            Self::UnknownResource => f.write_str("resource is not tracked"),
            Self::RoleMismatch => f.write_str("resource ownership role does not match"),
            Self::BorrowedResource => f.write_str("borrowed resource cannot enter owned ledger"),
            Self::DoubleRelease => f.write_str("resource was released more than once"),
            Self::LeakedResource => f.write_str("owned resource was not released"),
            Self::AliasedCompletionParts => {
                f.write_str("completion result and options must have distinct ownership identities")
            }
        }
    }
}

impl Error for OwnershipError {}

impl<R: Ord> Default for OwnershipLedger<R> {
    fn default() -> Self {
        Self {
            records: BTreeMap::new(),
        }
    }
}

impl<R: Ord> OwnershipLedger<R> {
    /// Creates an empty ownership ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a newly owned resource.
    pub fn acquire(&mut self, resource: R, role: OwnershipRole) -> Result<(), OwnershipError> {
        if role == OwnershipRole::BorrowedArgument {
            return Err(OwnershipError::BorrowedResource);
        }
        if self.records.contains_key(&resource) {
            return Err(OwnershipError::DuplicateResource);
        }
        self.records.insert(
            resource,
            OwnershipRecord {
                role,
                state: OwnershipState::Owned,
            },
        );
        Ok(())
    }

    /// Records the separately owned result and options of one completion.
    pub fn acquire_completion(&mut self, result: R, options: R) -> Result<(), OwnershipError> {
        if result == options {
            return Err(OwnershipError::AliasedCompletionParts);
        }
        if self.records.contains_key(&result) || self.records.contains_key(&options) {
            return Err(OwnershipError::DuplicateResource);
        }
        self.records.insert(
            result,
            OwnershipRecord {
                role: OwnershipRole::CompletionResult,
                state: OwnershipState::Owned,
            },
        );
        self.records.insert(
            options,
            OwnershipRecord {
                role: OwnershipRole::CompletionOptions,
                state: OwnershipState::Owned,
            },
        );
        Ok(())
    }

    /// Releases one resource and checks its semantic ownership role.
    pub fn release(&mut self, resource: &R, role: OwnershipRole) -> Result<(), OwnershipError> {
        let record = self
            .records
            .get_mut(resource)
            .ok_or(OwnershipError::UnknownResource)?;
        if record.role != role {
            return Err(OwnershipError::RoleMismatch);
        }
        if record.state == OwnershipState::Released {
            return Err(OwnershipError::DoubleRelease);
        }
        record.state = OwnershipState::Released;
        Ok(())
    }

    /// Returns the current state of a tracked resource.
    pub fn state(&self, resource: &R) -> Option<OwnershipState> {
        self.records.get(resource).map(|record| record.state)
    }

    /// Proves that every owned resource in the plan has been released.
    pub fn validate_released(&self) -> Result<(), OwnershipError> {
        if self
            .records
            .values()
            .all(|record| record.state == OwnershipState::Released)
        {
            Ok(())
        } else {
            Err(OwnershipError::LeakedResource)
        }
    }
}

/// Whether an operation can suspend the current compiled activation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuspensionClass {
    /// The operation is proven not to suspend.
    CannotSuspend,
    /// The operation may suspend depending on runtime behaviour.
    MaySuspend,
    /// The operation necessarily creates a suspension point.
    MustSuspend,
}

/// State that remains live across a possible suspension.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SuspensionRequirements {
    /// Runtime frame state must be made observable before suspension.
    pub frame_state: bool,
    /// Native values must be spilled and restored.
    pub native_values: bool,
    /// Owned values must be retained across the suspension.
    pub owned_values: bool,
}

/// One target-independent action in a suspension plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuspensionAction {
    /// Materialise the runtime-visible frame before suspending.
    MaterialiseFrame,
    /// Spill native live values before suspending.
    SpillLiveValues,
    /// Retain owned values before suspending.
    RetainOwnedValues,
    /// The point at which control may leave the activation.
    Suspend,
    /// Restore spilled native values on the resume edge.
    RestoreLiveValues,
    /// Release values retained for the suspension.
    ReleaseRetainedValues,
}

/// A proposed sequence for preserving state across a suspension point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuspensionPlan {
    /// Static suspension classification of the operation.
    pub class: SuspensionClass,
    /// Categories of state live across the operation.
    pub requirements: SuspensionRequirements,
    /// Ordered actions on the suspend and resume paths.
    pub actions: Vec<SuspensionAction>,
}

/// Why a suspension plan cannot preserve the live activation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuspensionPlanError {
    /// A non-suspending operation contains suspension machinery.
    UnexpectedAction,
    /// A potentially suspending operation has no suspension point.
    MissingSuspend,
    /// A suspension action occurs more than once.
    DuplicateAction,
    /// A required preservation action is missing.
    MissingAction(SuspensionAction),
    /// A preservation or restoration action is on the wrong side of suspension.
    InvalidActionOrder,
}

impl fmt::Display for SuspensionPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedAction => f.write_str("non-suspending plan contains actions"),
            Self::MissingSuspend => f.write_str("suspending plan has no suspension point"),
            Self::DuplicateAction => f.write_str("suspension action occurs more than once"),
            Self::MissingAction(action) => write!(f, "suspension plan is missing {action:?}"),
            Self::InvalidActionOrder => f.write_str("suspension actions are not in semantic order"),
        }
    }
}

impl Error for SuspensionPlanError {}

impl SuspensionPlan {
    /// Validates the suspend/resume preservation protocol.
    pub fn validate(&self) -> Result<(), SuspensionPlanError> {
        if self.class == SuspensionClass::CannotSuspend {
            return if self.actions.is_empty() {
                Ok(())
            } else {
                Err(SuspensionPlanError::UnexpectedAction)
            };
        }

        for (index, action) in self.actions.iter().enumerate() {
            if self.actions[index + 1..].contains(action) {
                return Err(SuspensionPlanError::DuplicateAction);
            }
            let expected = match action {
                SuspensionAction::MaterialiseFrame => self.requirements.frame_state,
                SuspensionAction::SpillLiveValues | SuspensionAction::RestoreLiveValues => {
                    self.requirements.native_values
                }
                SuspensionAction::RetainOwnedValues | SuspensionAction::ReleaseRetainedValues => {
                    self.requirements.owned_values
                }
                SuspensionAction::Suspend => true,
            };
            if !expected {
                return Err(SuspensionPlanError::UnexpectedAction);
            }
        }

        let suspend = self.unique_position(SuspensionAction::Suspend)?;
        self.require_before(
            self.requirements.frame_state,
            SuspensionAction::MaterialiseFrame,
            suspend,
        )?;
        self.require_pair(
            self.requirements.native_values,
            SuspensionAction::SpillLiveValues,
            SuspensionAction::RestoreLiveValues,
            suspend,
        )?;
        self.require_pair(
            self.requirements.owned_values,
            SuspensionAction::RetainOwnedValues,
            SuspensionAction::ReleaseRetainedValues,
            suspend,
        )
    }

    fn unique_position(&self, action: SuspensionAction) -> Result<usize, SuspensionPlanError> {
        let mut positions = self
            .actions
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| (*candidate == action).then_some(index));
        let first = positions.next().ok_or_else(|| {
            if action == SuspensionAction::Suspend {
                SuspensionPlanError::MissingSuspend
            } else {
                SuspensionPlanError::MissingAction(action)
            }
        })?;
        if positions.next().is_some() {
            Err(SuspensionPlanError::DuplicateAction)
        } else {
            Ok(first)
        }
    }

    fn require_before(
        &self,
        required: bool,
        action: SuspensionAction,
        suspend: usize,
    ) -> Result<(), SuspensionPlanError> {
        if !required {
            return Ok(());
        }
        if self.unique_position(action)? < suspend {
            Ok(())
        } else {
            Err(SuspensionPlanError::InvalidActionOrder)
        }
    }

    fn require_pair(
        &self,
        required: bool,
        before: SuspensionAction,
        after: SuspensionAction,
        suspend: usize,
    ) -> Result<(), SuspensionPlanError> {
        if !required {
            return Ok(());
        }
        let before = self.unique_position(before)?;
        let after = self.unique_position(after)?;
        if before < suspend && suspend < after {
            Ok(())
        } else {
            Err(SuspensionPlanError::InvalidActionOrder)
        }
    }
}

/// Runtime domain whose stability protects a speculative fast path.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GuardDomain {
    /// Command definition, rename, and alias state.
    CommandEnvironment,
    /// The handler selected when ordinary command lookup fails.
    UnknownHandling,
    /// Namespace resolution and namespace membership state.
    Namespace,
    /// Variable trace state.
    VariableTrace,
    /// Command execution trace state.
    CommandTrace,
    /// Object and method dispatch state.
    ObjectDispatch,
    /// Safe- or child-interpreter policy and command state.
    Interpreter,
}

/// An epoch comparison that must hold before entering a fast path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuardCondition {
    /// Semantic state domain being guarded.
    domain: GuardDomain,
    /// Epoch value observed when the fast path was planned.
    expected_epoch: u64,
}

impl GuardCondition {
    /// Construct an epoch condition inside the common-analysis layer.
    ///
    /// A backend may inspect and emit a guard selected by common analysis, but
    /// cannot obtain the unforgeable provenance required to manufacture the
    /// semantic evidence that authorises its own fast path.
    #[must_use]
    pub const fn from_analysis(
        domain: GuardDomain,
        expected_epoch: u64,
        _provenance: &crate::semantic_analysis::CommonAnalysisProvenance,
    ) -> Self {
        Self {
            domain,
            expected_epoch,
        }
    }

    /// Semantic state domain protected by this condition.
    #[must_use]
    pub const fn domain(&self) -> GuardDomain {
        self.domain
    }

    /// Epoch observed by common analysis when the fast path was planned.
    #[must_use]
    pub const fn expected_epoch(&self) -> u64 {
        self.expected_epoch
    }
}

/// A speculative implementation paired with its exact semantic fallback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardedPlan<Fast, Slow> {
    /// Conditions required to select `fast` safely.
    guards: Vec<GuardCondition>,
    /// Specialised path selected when every guard succeeds.
    fast: Fast,
    /// Semantically exact path selected when any guard fails.
    slow: Slow,
}

/// Failure to form a useful guarded plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuardedPlanError {
    /// A guarded fast path must name at least one invalidation condition.
    MissingGuard,
    /// The same semantic domain cannot be guarded by conflicting epochs.
    ConflictingEpochs,
}

impl fmt::Display for GuardedPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingGuard => f.write_str("guarded plan requires at least one guard"),
            Self::ConflictingEpochs => f.write_str("guard domain has conflicting expected epochs"),
        }
    }
}

impl Error for GuardedPlanError {}

impl<Fast, Slow> GuardedPlan<Fast, Slow> {
    /// Construct a guarded plan inside the common-analysis layer after checking
    /// its guard set.
    ///
    /// Backends receive the resulting immutable plan through accessors. They
    /// cannot create guard evidence and feed it back into backend selection.
    pub fn from_analysis(
        mut guards: Vec<GuardCondition>,
        fast: Fast,
        slow: Slow,
        _provenance: &crate::semantic_analysis::CommonAnalysisProvenance,
    ) -> Result<Self, GuardedPlanError> {
        if guards.is_empty() {
            return Err(GuardedPlanError::MissingGuard);
        }
        guards.sort_unstable_by_key(|guard| (guard.domain, guard.expected_epoch));
        for pair in guards.windows(2) {
            if pair[0].domain == pair[1].domain && pair[0].expected_epoch != pair[1].expected_epoch
            {
                return Err(GuardedPlanError::ConflictingEpochs);
            }
        }
        guards.dedup();
        Ok(Self { guards, fast, slow })
    }

    /// Canonically ordered conditions protecting the fast path.
    #[must_use]
    pub fn guards(&self) -> &[GuardCondition] {
        &self.guards
    }

    /// Specialised path selected when every guard succeeds.
    #[must_use]
    pub const fn fast(&self) -> &Fast {
        &self.fast
    }

    /// Semantically exact path selected when any guard fails.
    #[must_use]
    pub const fn slow(&self) -> &Slow {
        &self.slow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shimmering_preserves_both_forms_but_mutation_invalidates_one() {
        let mut value = DualRepresentation::from_string("41");
        assert_eq!(
            value.shimmer_to_internal("integer", 41_i64),
            RepresentationTransition::ShimmeredToInternal
        );
        assert_eq!(value.string(), Some("41"));
        assert_eq!(value.internal(), Some((&"integer", &41)));

        value.mark_shared();
        assert_eq!(
            value.mutate_internal("integer", 42),
            RepresentationTransition::Mutated {
                ownership: MutationTransition::CopyOnWrite,
                invalidated: InvalidatedRepresentation::String,
            }
        );
        assert_eq!(value.sharing(), SharingState::Unique);
        assert_eq!(value.string(), None);
    }

    #[test]
    fn dual_representation_rejects_an_empty_value() {
        assert_eq!(
            DualRepresentation::<(), ()>::new(None, None, SharingState::Unique),
            Err(RepresentationError::MissingRepresentation)
        );
    }

    #[test]
    fn cached_slots_are_synchronised_across_generic_runtime() {
        let plan = BoundaryPlan {
            storage: VarStorage::CachedSlot,
            boundary: BoundaryKind::GenericRuntime,
            actions: vec![
                BoundaryAction::MaterialiseBefore,
                BoundaryAction::InvalidateAfter,
                BoundaryAction::ReloadAfter,
            ],
        };
        assert_eq!(plan.validate(), Ok(()));

        let missing_reload = BoundaryPlan {
            actions: plan.actions[..2].to_vec(),
            ..plan
        };
        assert_eq!(
            missing_reload.validate(),
            Err(BoundaryPlanError::MissingAction(
                BoundaryAction::ReloadAfter
            ))
        );
    }

    #[test]
    fn native_only_storage_cannot_cross_reentry() {
        let plan = BoundaryPlan {
            storage: VarStorage::NativeOnly,
            boundary: BoundaryKind::Reentrant,
            actions: Vec::new(),
        };
        assert_eq!(plan.validate(), Err(BoundaryPlanError::NativeOnlyEscapes));
    }

    #[test]
    fn completion_ownership_is_separate_and_exactly_once() {
        let mut ledger = OwnershipLedger::new();
        ledger.acquire_completion(10_u32, 11_u32).unwrap();
        assert_eq!(
            ledger.validate_released(),
            Err(OwnershipError::LeakedResource)
        );
        ledger
            .release(&10, OwnershipRole::CompletionResult)
            .unwrap();
        ledger
            .release(&11, OwnershipRole::CompletionOptions)
            .unwrap();
        assert_eq!(ledger.validate_released(), Ok(()));
        assert_eq!(
            ledger.release(&11, OwnershipRole::CompletionOptions),
            Err(OwnershipError::DoubleRelease)
        );
    }

    #[test]
    fn completion_acquisition_is_atomic_when_one_identity_exists() {
        let mut ledger = OwnershipLedger::new();
        ledger.acquire(11_u32, OwnershipRole::Temporary).unwrap();

        assert_eq!(
            ledger.acquire_completion(10, 11),
            Err(OwnershipError::DuplicateResource)
        );
        assert_eq!(ledger.state(&10), None);
        assert_eq!(ledger.state(&11), Some(OwnershipState::Owned));
    }

    #[test]
    fn suspension_plan_orders_preservation_around_suspend() {
        let plan = SuspensionPlan {
            class: SuspensionClass::MaySuspend,
            requirements: SuspensionRequirements {
                frame_state: true,
                native_values: true,
                owned_values: true,
            },
            actions: vec![
                SuspensionAction::MaterialiseFrame,
                SuspensionAction::SpillLiveValues,
                SuspensionAction::RetainOwnedValues,
                SuspensionAction::Suspend,
                SuspensionAction::RestoreLiveValues,
                SuspensionAction::ReleaseRetainedValues,
            ],
        };
        assert_eq!(plan.validate(), Ok(()));

        let mut invalid = plan;
        invalid.actions.swap(2, 4);
        assert_eq!(
            invalid.validate(),
            Err(SuspensionPlanError::InvalidActionOrder)
        );
    }

    #[test]
    fn guarded_plan_rejects_conflicting_epochs() {
        let provenance = crate::semantic_analysis::test_common_analysis_provenance();
        let guards = vec![
            GuardCondition::from_analysis(GuardDomain::CommandEnvironment, 7, &provenance),
            GuardCondition::from_analysis(GuardDomain::CommandEnvironment, 8, &provenance),
        ];
        assert_eq!(
            GuardedPlan::from_analysis(guards, "direct", "invoke", &provenance),
            Err(GuardedPlanError::ConflictingEpochs)
        );
    }

    #[test]
    fn guarded_plan_canonicalises_order_and_identical_duplicates() {
        let provenance = crate::semantic_analysis::test_common_analysis_provenance();
        let namespace = GuardCondition::from_analysis(GuardDomain::Namespace, 2, &provenance);
        let commands =
            GuardCondition::from_analysis(GuardDomain::CommandEnvironment, 7, &provenance);
        let plan = GuardedPlan::from_analysis(
            vec![namespace, commands, namespace],
            "direct",
            "invoke",
            &provenance,
        )
        .unwrap();

        assert_eq!(plan.guards(), [commands, namespace]);
    }
}
