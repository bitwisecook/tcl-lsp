// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original class relation operands and their ordered source-metadata effects.

use super::MemberSide;
use crate::{
    command_binding::CommandAllocationSite,
    signature_scan::scope::{SignatureSourceLookup, SignatureSourceNameInput},
};
use tcl_registry::definer::{SlotOp, SlotSpec};

/// The own class relation list selected by the definition grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalSourceClassRelationKind {
    /// Instance superclass order; class-object maker ancestry is independent.
    Superclass,
    /// The selected receiver's own mixin order.
    Mixin,
}

/// One retained class operand and its original caller naming geometry.
/// Neither spelling nor candidates supply class existence or native identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassRelation {
    input: SignatureSourceNameInput,
    lookup: SignatureSourceLookup,
}

impl OriginalSourceClassRelation {
    pub(crate) fn new(
        input: SignatureSourceNameInput,
        lookup: SignatureSourceLookup,
    ) -> Option<Self> {
        (lookup.original_name_input() == Some(&input)).then_some(Self { input, lookup })
    }
    /// Exact original operand producer, independent of a reporting name.
    #[must_use]
    pub const fn name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Original caller scope and byte candidates; no reached provider grant.
    #[must_use]
    pub const fn lookup(&self) -> &SignatureSourceLookup {
        &self.lookup
    }
}

/// An effect on one relation list at its genuine source command site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassRelationEffect {
    site: CommandAllocationSite,
    side: MemberSide,
    kind: OriginalSourceClassRelationKind,
    slot: SlotSpec,
    operation: Option<SlotOp>,
    values: Option<Vec<OriginalSourceClassRelation>>,
}

impl OriginalSourceClassRelationEffect {
    pub(crate) fn new(
        allocation_site: CommandAllocationSite,
        side: MemberSide,
        kind: OriginalSourceClassRelationKind,
        slot: SlotSpec,
        operation: Option<SlotOp>,
        values: Option<Vec<OriginalSourceClassRelation>>,
    ) -> Self {
        Self {
            site: allocation_site,
            side,
            kind,
            slot,
            operation,
            values,
        }
    }
    /// Exact original operation site; offsets never reconstruct operation order.
    #[must_use]
    pub const fn site(&self) -> &CommandAllocationSite {
        &self.site
    }
    /// Independently selected own receiver table.
    #[must_use]
    pub const fn side(&self) -> MemberSide {
        self.side
    }
    /// Relation list affected by this operation.
    #[must_use]
    pub const fn kind(&self) -> OriginalSourceClassRelationKind {
        self.kind
    }
    /// Selected operation, or an unresolved operation barrier.
    #[must_use]
    pub const fn operation(&self) -> Option<SlotOp> {
        self.operation
    }
    /// Original targets, or incomplete operand coverage.
    #[must_use]
    pub fn values(&self) -> Option<&[OriginalSourceClassRelation]> {
        self.values.as_deref()
    }
}

/// Ordered own relation metadata. Provider joins occur before deduplication
/// and removal, so equal reporting labels cannot identify two class objects.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OriginalSourceClassRelationLedger {
    initial_declaration: bool,
    effects: Vec<OriginalSourceClassRelationEffect>,
    unavailable: bool,
}

impl OriginalSourceClassRelationLedger {
    pub(crate) fn begin_declaration(&mut self) {
        self.initial_declaration = true;
    }
    pub(crate) fn withdraw(&mut self) {
        self.unavailable = true;
    }
    pub(crate) fn effect(&mut self, effect: OriginalSourceClassRelationEffect) {
        self.effects.push(effect);
    }
    pub(crate) fn absorb(&mut self, other: &Self, other_ran_second: bool) {
        self.initial_declaration |= other.initial_declaration;
        self.unavailable |= other.unavailable;
        let mut effects = if other_ran_second {
            std::mem::take(&mut self.effects)
        } else {
            other.effects.clone()
        };
        let tail = if other_ran_second {
            &other.effects
        } else {
            &self.effects
        };
        for effect in tail {
            if !effects.contains(effect) {
                effects.push(effect.clone());
            }
        }
        self.effects = effects;
    }
    /// Effects in the observed definition walk order, including unknown barriers.
    pub fn effects(&self) -> impl Iterator<Item = &OriginalSourceClassRelationEffect> {
        self.effects.iter()
    }
    /// Resolve every target independently, then apply the sole Registry slot
    /// fold to actual provider keys. Unknown operations remain terminal until
    /// an independently retained replacement or clear renews that list.
    #[must_use]
    pub fn resolve<K: Clone + Eq>(
        &self,
        side: MemberSide,
        kind: OriginalSourceClassRelationKind,
        mut provider: impl FnMut(&OriginalSourceClassRelation) -> Option<K>,
    ) -> Option<Vec<K>> {
        if self.unavailable {
            return None;
        }
        let mut current = self.initial_declaration.then(Vec::new);
        for effect in self
            .effects
            .iter()
            .filter(|effect| effect.side == side && effect.kind == kind)
        {
            let Some(operation) = effect.operation else {
                current = None;
                continue;
            };
            let values = effect
                .values
                .as_ref()
                .and_then(|values| values.iter().map(&mut provider).collect::<Option<Vec<_>>>());
            if operation == SlotOp::Clear {
                current = Some(Vec::new());
                continue;
            }
            let Some(values) = values else {
                current = None;
                continue;
            };
            if operation == SlotOp::Set {
                current = Some(Vec::new());
            }
            if let Some(current) = &mut current {
                effect.slot.apply_values(current, operation, &values);
            }
        }
        current
    }
}
