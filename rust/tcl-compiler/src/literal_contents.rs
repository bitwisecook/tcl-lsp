// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded physical contents alternatives for operand-layout consensus.

use crate::{
    bounded_set::BoundedSet,
    place::{CellGeneration, Place},
    var_resolve::{ContentsPresence, ResolveContext, VariableProofRelocation},
};
use tcl_registry::CommandRegistry;

const MAX_ALTERNATIVES: usize = 8;
const MAX_TEXT_BYTES: usize = 8192;

/// A closed set of texts from the same selected physical receiver.
/// This purpose proof does not supply a constant or an execution target.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClosedLiteralContents {
    receiver: Place,
    values: BoundedSet<String, MAX_ALTERNATIVES>,
}

impl ClosedLiteralContents {
    /// Exact string alternatives, sorted without numeric or list equivalence.
    #[must_use]
    pub fn values(&self) -> &[String] {
        self.values.as_slice()
    }

    /// The physical receiver whose contents supply these alternatives.
    #[must_use]
    pub const fn receiver(&self) -> &Place {
        &self.receiver
    }

    fn from_values(receiver: Place, values: impl IntoIterator<Item = String>) -> Option<Self> {
        let mut values = BoundedSet::from_iter_bounded(values)?;
        if values.is_empty()
            || values
                .iter()
                .try_fold(0usize, |bytes, value| {
                    bytes
                        .checked_add(value.len())
                        .filter(|bytes| *bytes <= MAX_TEXT_BYTES)
                })
                .is_none()
        {
            return None;
        }
        values.canonicalise();
        Some(Self { receiver, values })
    }

    fn same_receiver(&self, receiver: &Place) -> bool {
        self.receiver.cell == receiver.cell
            && self.receiver.index == receiver.index
            && self.receiver.keys == receiver.keys
            && self.receiver.kind == receiver.kind
    }

    pub(crate) fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        Self {
            receiver: relocation.place(&self.receiver),
            values: self.values.clone(),
        }
    }

    pub(crate) fn at_receiver(&self, receiver: &Place) -> Self {
        Self {
            receiver: receiver.clone(),
            values: self.values.clone(),
        }
    }
}

/// Purpose-only contents alternatives with an explicit unenumerated residual.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LiteralContentsAlternatives {
    /// Every possible value is retained for the same current physical cell.
    Closed(Box<ClosedLiteralContents>),
    /// Contents, observer effects, receiver identity, or the bound are unproved.
    Unknown,
}

impl LiteralContentsAlternatives {
    fn closed(contents: ClosedLiteralContents) -> Self {
        Self::Closed(Box::new(contents))
    }
}

impl ResolveContext {
    /// Query bounded text alternatives at a selected read boundary. Missing or
    /// mixed presence, retired lifetimes and read observers remain unknown.
    /// Singleton literal and SSA constant queries retain their own contracts.
    #[must_use]
    pub fn literal_contents_alternatives_at(
        &self,
        receiver: &Place,
        registry: &CommandRegistry,
    ) -> LiteralContentsAlternatives {
        let Some(cell) = &receiver.cell else {
            return LiteralContentsAlternatives::Unknown;
        };
        if receiver.dynamic
            || receiver.observed
            || crate::var_resolve::project_access(
                receiver.clone(),
                self,
                tcl_registry::TraceOperation::Read,
            )
            .observed
            || self.contents_presence(receiver) != ContentsPresence::Defined
            || self.store_would_error(receiver)
            || cell.generation == CellGeneration::Unknown
            || cell.generation
                != self
                    .generations
                    .get(&crate::var_resolve::cell_key(receiver))
                    .copied()
                    .unwrap_or_default()
            || self.implicit_read_changes_value(receiver, registry)
        {
            return LiteralContentsAlternatives::Unknown;
        }
        let Some(key) = crate::var_resolve::canonical_binding_value_key(receiver) else {
            return LiteralContentsAlternatives::Unknown;
        };
        if let Some(value) = self.literal_contents_at(receiver, registry) {
            return ClosedLiteralContents::from_values(receiver.clone(), [value.to_owned()])
                .map_or(
                    LiteralContentsAlternatives::Unknown,
                    LiteralContentsAlternatives::closed,
                );
        }
        self.closed_literal_contents
            .get(&key)
            .filter(|contents| contents.same_receiver(receiver))
            .cloned()
            .map_or(
                LiteralContentsAlternatives::Unknown,
                LiteralContentsAlternatives::closed,
            )
    }

    /// Resolve only the original substitution spelling before querying the same
    /// physical contents purpose. This does not turn alternatives into a word.
    #[must_use]
    pub fn substitution_literal_alternatives(
        &self,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> LiteralContentsAlternatives {
        let receiver = crate::var_resolve::resolve_substitution_access(
            spelling,
            self,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        self.literal_contents_alternatives_at(&receiver, registry)
    }

    pub(crate) fn retain_literal_values(
        &mut self,
        keep: impl Fn(&crate::var_resolve::VariableCellKey) -> bool,
    ) {
        self.constant_values.retain(|key, _| keep(key));
        self.closed_literal_contents.retain(|key, _| keep(key));
        self.original_name_values.retain(|key, _| keep(key));
    }

    pub(crate) fn forget_literal_value<Q: crate::var_resolve::VariableCellKeyQuery + ?Sized>(
        &mut self,
        key: &Q,
    ) {
        self.constant_values.remove(key);
        self.closed_literal_contents.remove(key);
        self.original_name_values.remove(key);
    }

    pub(crate) fn store_literal_value(
        &mut self,
        key: impl Into<crate::var_resolve::VariableCellKey>,
        value: String,
    ) {
        let key = key.into();
        self.closed_literal_contents.remove(&key);
        self.original_name_values.remove(&key);
        self.constant_values.insert(key, value);
    }

    pub(crate) fn join_literal_contents(&mut self, other: &Self) {
        let keys: std::collections::HashSet<_> = self
            .constant_values
            .keys()
            .chain(other.constant_values.keys())
            .chain(self.closed_literal_contents.keys())
            .chain(other.closed_literal_contents.keys())
            .cloned()
            .collect();
        let mut joined = crate::var_resolve::VariableCellTable::default();
        for key in keys {
            if self.constant_values.contains_key(&key)
                && self.constant_values.get(&key) == other.constant_values.get(&key)
            {
                continue;
            }
            let (Some(left), Some(right)) = (
                self.literal_contents_for_join(&key),
                other.literal_contents_for_join(&key),
            ) else {
                continue;
            };
            if !left.same_receiver(&right.receiver) {
                continue;
            }
            if let Some(contents) = ClosedLiteralContents::from_values(
                left.receiver.clone(),
                left.values.into_iter().chain(right.values),
            ) {
                joined.insert(key, contents);
            }
        }
        self.closed_literal_contents = joined;
    }

    fn literal_contents_for_join(
        &self,
        key: &crate::var_resolve::VariableCellKey,
    ) -> Option<ClosedLiteralContents> {
        let receiver = self
            .contents_presence_slots
            .get(key)
            .map(std::sync::Arc::as_ref)
            .or_else(|| {
                self.closed_literal_contents
                    .get(key)
                    .map(ClosedLiteralContents::receiver)
            })?;
        let cell = receiver.cell.as_ref()?;
        if receiver.dynamic
            || receiver.observed
            || self.contents_presence(receiver) != ContentsPresence::Defined
            || cell.generation == CellGeneration::Unknown
            || cell.generation
                != self
                    .generations
                    .get(&crate::var_resolve::cell_key(receiver))
                    .copied()
                    .unwrap_or_default()
        {
            return None;
        }
        if let Some(value) = self.constant_values.get(key) {
            return ClosedLiteralContents::from_values(receiver.clone(), [value.clone()]);
        }
        self.closed_literal_contents
            .get(key)
            .filter(|contents| contents.same_receiver(receiver))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::var_resolve::{BindingIdentity, resolve_literal_place};

    fn entry(registry: &CommandRegistry) -> ResolveContext {
        let mut state = ResolveContext::for_function("::worker");
        state.binding_identity = BindingIdentity::Bound;
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        state
    }

    fn values(state: &ResolveContext, registry: &CommandRegistry) -> Option<Vec<String>> {
        let receiver = resolve_literal_place("pattern", state, false, registry);
        match state.literal_contents_alternatives_at(&receiver, registry) {
            LiteralContentsAlternatives::Closed(contents) => Some(contents.values().to_vec()),
            LiteralContentsAlternatives::Unknown => None,
        }
    }

    #[test]
    fn closed_branch_values_keep_singleton_and_must_value_contracts() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut left = entry(registry);
        left.define_literal("pattern", "left", registry);
        let mut right = entry(registry);
        right.define_literal("pattern", "seed", registry);
        left.join(&right);
        assert_eq!(
            values(&left, registry),
            Some(vec!["left".into(), "seed".into()])
        );
        assert_eq!(left.literal_value("pattern", registry), None);
        let receiver = resolve_literal_place("pattern", &left, false, registry);
        assert_eq!(left.contents_presence(&receiver), ContentsPresence::Defined);
        let mut reverse = right.clone();
        reverse.join(&left);
        assert_eq!(values(&reverse, registry), values(&left, registry));
        left.define_literal("pattern", "replacement", registry);
        assert_eq!(values(&left, registry), Some(vec!["replacement".into()]));
        assert_eq!(left.literal_value("pattern", registry), Some("replacement"));
    }

    #[test]
    fn opaque_contents_missing_path_and_read_observers_retain_unknown_residual() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut known = entry(registry);
        known.define_literal("pattern", "seed", registry);
        let mut missing = known.clone();
        missing.join(&entry(registry));
        assert_eq!(values(&missing, registry), None);
        let mut opaque = known.clone();
        opaque.define_unknown_contents("pattern", registry);
        known.join(&opaque);
        assert_eq!(values(&known, registry), None);
        known.define_literal("pattern", "new", registry);
        known.dynamic_traces = true;
        assert_eq!(values(&known, registry), None);
        known.dynamic_traces = false;
        known.widen();
        assert_eq!(values(&known, registry), None);
    }

    #[test]
    fn different_activation_and_retired_receiver_cannot_share_text_evidence() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut left = entry(registry);
        left.define_literal("pattern", "left", registry);
        let mut right = entry(registry);
        right.activation = Some("other-activation".into());
        right.define_literal("pattern", "seed", registry);
        left.join(&right);
        assert_eq!(values(&left, registry), None);
        let mut state = entry(registry);
        state.define_literal("pattern", "old", registry);
        let old = resolve_literal_place("pattern", &state, false, registry);
        state.generations.insert(
            crate::var_resolve::cell_key(&old),
            CellGeneration::After(99),
        );
        state.define_literal("pattern", "new", registry);
        assert_eq!(
            state.literal_contents_alternatives_at(&old, registry),
            LiteralContentsAlternatives::Unknown
        );
        assert_eq!(values(&state, registry), Some(vec!["new".into()]));
    }

    #[test]
    fn bounded_union_withdraws_on_overflow_and_relocates_physical_receivers() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let base = entry(registry);
        let mut joined = base.clone();
        joined.define_literal("pattern", "0", registry);
        for number in 1..MAX_ALTERNATIVES {
            let mut branch = base.clone();
            branch.define_literal("pattern", &number.to_string(), registry);
            joined.join(&branch);
        }
        assert_eq!(values(&joined, registry).unwrap().len(), MAX_ALTERNATIVES);
        let mut relocation = VariableProofRelocation::default();
        relocation
            .activations
            .insert(base.activation.clone().unwrap(), "template".into());
        let relocated = joined.relocated(&relocation);
        assert_eq!(values(&relocated, registry), values(&joined, registry));
        let roundtrip = relocated.relocated(&relocation.inverse().unwrap());
        assert_eq!(roundtrip, joined);
        let mut overflow = base.clone();
        overflow.define_literal("pattern", "overflow", registry);
        joined.join(&overflow);
        assert_eq!(values(&joined, registry), None);
        joined.define_literal("pattern", &"x".repeat(MAX_TEXT_BYTES + 1), registry);
        assert_eq!(values(&joined, registry), None);
        assert!(joined.literal_value("pattern", registry).is_some());
    }
}
