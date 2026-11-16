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

//! `FoldedType`: the semantic type, shape and representation evidence of one folded value.

use tcl_registry::TclType;
use tcl_registry::value_transfer::{
    ExactValueOrUnavailable, InvocationOutcome, RepresentationEvidence, StoreOutcome, TargetId,
    ValueShape,
};

/// The semantic type, shape and representation evidence of one value, from
/// the evaluation that produced it (`docs/design/compiler/value-transfers.md`
/// § *Exact values, types, and representation*): three facts, kept apart
/// from the exact value in [`crate::sccp::SccpResult::values`] and from one
/// another. A computed byte array's string is exact while its
/// representation is the byte array the route built, so a consumer asking
/// whether a use converts reads [`Self::representation`], never the type
/// alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldedType {
    /// The internal representation the evaluation's type facts state.
    pub intrep: Option<TclType>,
    /// The value's shape, when proven: a list's length, a dict's keys.
    pub shape: Option<ValueShape>,
    /// How the route constructed the value, when it says.
    pub representation: RepresentationEvidence,
}

impl FoldedType {
    /// The folded type of an outcome's result: its type facts' result type
    /// and the result value's representation evidence.
    #[must_use]
    pub fn of_result(outcome: &InvocationOutcome) -> Option<Self> {
        let representation = match &outcome.result {
            ExactValueOrUnavailable::Exact(value) => value.representation,
            ExactValueOrUnavailable::Unavailable(_) => RepresentationEvidence::Unknown,
        };
        Self::informative(outcome.types.result, None, representation)
    }

    /// The folded type a place holds after `stores`, the outcome's stores to
    /// it in execution order: a write's stated type and shape with the
    /// written value's representation; a may-write's bounds, its
    /// representation unknown; nothing after an unbind. A preserve keeps
    /// what an earlier store left and states nothing of the prior value.
    #[must_use]
    pub fn after_stores<'s>(
        outcome: &InvocationOutcome,
        stores: impl IntoIterator<Item = &'s StoreOutcome>,
    ) -> Option<Self> {
        let stated = |target: TargetId| {
            let intrep = outcome
                .types
                .per_target
                .iter()
                .rev()
                .find_map(|(at, ty)| (*at == target).then_some(*ty));
            let shape = outcome
                .types
                .shapes
                .iter()
                .rev()
                .find_map(|(at, shape)| (*at == target).then(|| shape.clone()));
            (intrep, shape)
        };
        let mut held: Option<Self> = None;
        for store in stores {
            held = match store {
                StoreOutcome::Write { target, value } => {
                    let (intrep, shape) = stated(*target);
                    Self::informative(intrep, shape, value.representation)
                }
                StoreOutcome::Preserve { .. } => held,
                StoreOutcome::MayWrite { facts, .. }
                | StoreOutcome::WriteUnavailable { facts, .. } => Self::informative(
                    facts.intrep,
                    facts.shape.clone(),
                    RepresentationEvidence::Unknown,
                ),
                StoreOutcome::Unbind { .. } => None,
                // The type facts state types per target, and the target of
                // an element write is the whole array: only the written
                // value's own evidence speaks for the element.
                StoreOutcome::WriteElement { value, .. } => {
                    Self::informative(None, None, value.representation)
                }
            };
        }
        held
    }

    /// The folded type, when any of its three facts says something.
    fn informative(
        intrep: Option<TclType>,
        shape: Option<ValueShape>,
        representation: RepresentationEvidence,
    ) -> Option<Self> {
        (intrep.is_some() || shape.is_some() || representation != RepresentationEvidence::Unknown)
            .then_some(Self {
                intrep,
                shape,
                representation,
            })
    }

    /// The join of two folded types: each fact survives only where both
    /// state it alike, so identical strings built with different
    /// representations keep neither representation, and "the last written
    /// type wins" never happens.
    #[must_use]
    pub fn join(&self, other: &Self) -> Option<Self> {
        Self::informative(
            (self.intrep == other.intrep)
                .then_some(self.intrep)
                .flatten(),
            (self.shape == other.shape)
                .then(|| self.shape.clone())
                .flatten(),
            if self.representation == other.representation {
                self.representation
            } else {
                RepresentationEvidence::Unknown
            },
        )
    }

    /// The join of every member's folded type: `None` once any member has
    /// none, and for no members at all.
    #[must_use]
    pub fn join_all(members: impl IntoIterator<Item = Option<Self>>) -> Option<Self> {
        let mut members = members.into_iter();
        let first = members.next()??;
        members.try_fold(first, |joined, member| joined.join(&member?))
    }

    /// The internal representation the value holds when it is created: the
    /// one the route constructed, when it says. A computed string holds
    /// none (a pure string), so this is `None` for it too.
    #[must_use]
    pub fn constructed_intrep(&self) -> Option<TclType> {
        match self.representation {
            RepresentationEvidence::Constructed(TclType::String)
            | RepresentationEvidence::Unknown => None,
            RepresentationEvidence::Constructed(built) => Some(built),
        }
    }

    /// The Explorer's spelling: the stated type, with how the route built
    /// the value — `bytearray (constructed)`, `string (constructed as
    /// int)`, or `list` when only the type is stated.
    #[must_use]
    pub fn label(&self) -> String {
        let named = |ty: TclType| crate::shimmer::type_name(ty);
        match (self.intrep, self.representation) {
            (Some(ty), RepresentationEvidence::Constructed(built)) if ty == built => {
                format!("{} (constructed)", named(ty))
            }
            (Some(ty), RepresentationEvidence::Constructed(built)) => {
                format!("{} (constructed as {})", named(ty), named(built))
            }
            (None, RepresentationEvidence::Constructed(built)) => {
                format!("{} (constructed)", named(built))
            }
            (Some(ty), RepresentationEvidence::Unknown) => named(ty),
            (None, RepresentationEvidence::Unknown) => "?".to_owned(),
        }
    }
}
