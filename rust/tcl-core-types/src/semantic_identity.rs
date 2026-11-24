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

//! Typed identity keys. Display names are deliberately absent from this API.

use alloc::{collections::BTreeMap, vec::Vec};

/// The owner within which an allocation token is meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdentityDomain {
    /// A retained source-analysis instance, independent of every live runtime.
    Analysis(u64),
    /// An actual runtime owner and its non-reused interpreter token.
    Runtime { owner: u64, interpreter: u64 },
}

/// Source or runtime provenance of an identity allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdentityProvenance {
    /// Exact source instance and byte offset of the allocation instruction.
    Source { instance: u64, offset: u32 },
    /// An actual runtime allocation token, not a source prediction.
    Runtime,
    /// A declaration supplied by the selected registry.
    Registry,
}

/// Fields common to every typed identity, never reconstructed from presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentityKey {
    /// The analysis or runtime owner.
    pub domain: IdentityDomain,
    /// Non-reused allocation token within that owner.
    pub token: u64,
    /// Incarnation or implementation generation selected by the owner.
    pub generation: u64,
    /// Independently allocated interner scope; native tokens use their runtime owner.
    pub issuer: Option<crate::ProcedureModuleId>,
}

/// A domain-specific key constructed only from already selected identity fields.
pub trait SemanticRef: Copy {
    /// Retain structured owner/token/generation fields.
    fn from_key(key: IdentityKey) -> Self;
    /// Expose the structured identity for adapters and conformance comparison.
    fn key(self) -> IdentityKey;
}

macro_rules! identity_refs {
    ($(#[$doc:meta] $name:ident),+ $(,)?) => {$ (
        #[$doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(IdentityKey);
        impl SemanticRef for $name {
            fn from_key(key: IdentityKey) -> Self { Self(key) }
            fn key(self) -> IdentityKey { self.0 }
        }
    )+ };
}

identity_refs! {
    /// A command allocation, independent of its current table slot or name.
    CommandRef,
    /// A namespace allocation, including its incarnation after recreation.
    NamespaceRef,
    /// A resolved variable place, independent of a written variable spelling.
    VariablePlaceRef,
    /// One frame activation, independent of its numeric level.
    FrameRef,
    /// One retained executable body unit.
    BodyUnitRef,
    /// One object-system instance allocation.
    InstanceRef,
    /// A package provider allocation, independent of a reported version string.
    PackageRef,
    /// One interpreter allocation, independent of a written selector path.
    InterpreterRef,
}

/// An interned typed key and the allocation provenance retained by its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentityRecord<R> {
    /// The typed identity key.
    pub reference: R,
    /// Evidence identifying the original allocation instruction or runtime.
    pub provenance: IdentityProvenance,
}

/// An owned interner over structured allocation descriptors, never display text.
/// Callers share an owner rather than clone its allocation counter. Separate
/// interners retain independent issuer scopes, even in the same naming domain.
#[derive(Debug)]
pub struct IdentityInterner<D, R> {
    domain: IdentityDomain,
    descriptors: BTreeMap<(D, u64), usize>,
    records: Vec<IdentityRecord<R>>,
    issuer: crate::ProcedureModuleId,
}

impl<D: Ord, R: SemanticRef> IdentityInterner<D, R> {
    /// Create an interner in a retained analysis or runtime domain.
    #[must_use]
    pub fn new(domain: IdentityDomain) -> Self {
        Self {
            domain,
            descriptors: BTreeMap::new(),
            records: Vec::new(),
            issuer: crate::ProcedureIdIssuer::default().module_id(),
        }
    }

    /// Intern an allocation descriptor and its incarnation. Names used for
    /// presentation belong outside the descriptor and cannot change this key.
    ///
    /// # Panics
    /// Panics only when the identity token address space is exhausted.
    pub fn intern(&mut self, descriptor: D, generation: u64, provenance: IdentityProvenance) -> R {
        let next = self.records.len();
        let index = *self
            .descriptors
            .entry((descriptor, generation))
            .or_insert(next);
        if index == next {
            let token = u64::try_from(index).expect("identity token space exhausted");
            self.records.push(IdentityRecord {
                reference: R::from_key(IdentityKey {
                    domain: self.domain,
                    token,
                    generation,
                    issuer: Some(self.issuer),
                }),
                provenance,
            });
        }
        self.records[index].reference
    }

    /// Read only records genuinely issued by this interner.
    pub fn record(&self, reference: R) -> Option<&IdentityRecord<R>> {
        let key = reference.key();
        if key.domain != self.domain || key.issuer != Some(self.issuer) {
            return None;
        }
        let record = self.records.get(usize::try_from(key.token).ok()?)?;
        (record.reference.key() == key).then_some(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ByteCommandSlot, ByteNamespacePath};

    #[test]
    fn same_allocation_is_interned_but_recreation_has_a_distinct_key() {
        let mut interner = IdentityInterner::<_, CommandRef>::new(IdentityDomain::Analysis(7));
        let provenance = IdentityProvenance::Source {
            instance: 7,
            offset: 12,
        };
        let first = interner.intern((12, 0), 1, provenance);
        assert_eq!(first, interner.intern((12, 0), 1, provenance));
        let recreated = interner.intern((12, 0), 2, provenance);
        assert_ne!(first, recreated);
        assert_eq!(interner.record(first).unwrap().provenance, provenance);
        assert_eq!(
            interner
                .record(recreated)
                .unwrap()
                .reference
                .key()
                .generation,
            2
        );
    }

    #[test]
    fn independent_interners_in_the_same_domain_do_not_alias() {
        let mut first = IdentityInterner::<_, CommandRef>::new(IdentityDomain::Analysis(7));
        let mut second = IdentityInterner::<_, CommandRef>::new(IdentityDomain::Analysis(7));
        let left = first.intern((1, 2), 1, IdentityProvenance::Registry);
        let right = second.intern((3, 4), 1, IdentityProvenance::Registry);
        assert_ne!(left, right);
        assert!(first.record(right).is_none());
        assert!(second.record(left).is_none());
    }

    #[test]
    fn colon_edges_are_structured_before_interning() {
        let mut interner = IdentityInterner::<_, CommandRef>::new(IdentityDomain::Analysis(3));
        let first = ByteCommandSlot::new(ByteNamespacePath::from_segments(["a:"]), "p".into());
        let second = ByteCommandSlot::new(ByteNamespacePath::from_segments(["a"]), ":p".into());
        assert_ne!(
            interner.intern(first, 1, IdentityProvenance::Registry),
            interner.intern(second, 1, IdentityProvenance::Registry)
        );
    }

    #[test]
    fn runtime_and_source_domains_cannot_alias() {
        let source = CommandRef::from_key(IdentityKey {
            domain: IdentityDomain::Analysis(1),
            token: 2,
            generation: 3,
            issuer: None,
        });
        let runtime = CommandRef::from_key(IdentityKey {
            domain: IdentityDomain::Runtime {
                owner: 1,
                interpreter: 1,
            },
            token: 2,
            generation: 3,
            issuer: None,
        });
        assert_ne!(source, runtime);
    }
}
