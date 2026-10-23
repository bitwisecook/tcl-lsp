// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Opaque source-module body coordinates, without execution or name authority.

use alloc::{format, string::String, sync::Arc};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

static NEXT_SCOPE: AtomicU64 = AtomicU64::new(1);

/// Opaque identity of one independently retained analytical body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcedureId {
    scope: u64,
    ordinal: u32,
}

/// Opaque retained module identity, independent of its source labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcedureModuleId(u64);

/// Analytical body owner; this coordinate grants no entered runtime frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodyFrameId {
    /// Whole-source analysis in one retained module.
    Top(ProcedureModuleId),
    /// One independently retained body.
    Body(ProcedureId),
}

impl ProcedureId {
    /// Opaque module-scoped output key; it carries no lookup or execution authority.
    #[must_use]
    pub fn reporting_key(self) -> String {
        format!("body:{}:{}", self.scope, self.ordinal)
    }

    /// Deterministic body order within its owning module, for output labels only.
    #[must_use]
    pub const fn ordinal(self) -> u32 {
        self.ordinal
    }
}

#[derive(Debug)]
struct ProcedureScope {
    id: u64,
    next: AtomicU32,
}

/// Clones share allocation state so transformed modules cannot reuse an ID.
#[derive(Debug, Clone)]
pub struct ProcedureIdIssuer(Arc<ProcedureScope>);

impl Default for ProcedureIdIssuer {
    fn default() -> Self {
        let id = NEXT_SCOPE
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .expect("procedure module identity space exhausted");
        Self(Arc::new(ProcedureScope {
            id,
            next: AtomicU32::new(0),
        }))
    }
}

impl PartialEq for ProcedureIdIssuer {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl ProcedureIdIssuer {
    /// Identity of this module and its retained top-level analytical frame.
    #[must_use]
    pub fn module_id(&self) -> ProcedureModuleId {
        ProcedureModuleId(self.0.id)
    }

    /// Whether this retained issuer allocated the analytical coordinate.
    #[must_use]
    pub fn owns(&self, id: ProcedureId) -> bool {
        id.scope == self.0.id && id.ordinal < self.0.next.load(Ordering::Relaxed)
    }

    /// Opaque output scope shared by clones, including their top-level reports.
    #[must_use]
    pub fn reporting_scope(&self) -> String {
        format!("module:{}", self.0.id)
    }

    /// Retain a fresh body coordinate without name or execution authority.
    #[must_use]
    pub fn allocate(&self) -> ProcedureId {
        let ordinal = self
            .0
            .next
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .expect("procedure body identity space exhausted");
        ProcedureId {
            scope: self.0.id,
            ordinal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_ordinals_from_separate_issuers_do_not_join() {
        let left_issuer = ProcedureIdIssuer::default();
        let right_issuer = ProcedureIdIssuer::default();
        let left = left_issuer.allocate();
        let right = right_issuer.allocate();
        assert!(left_issuer.owns(left));
        assert!(right_issuer.owns(right));
        assert!(!left_issuer.owns(right));
        assert!(!right_issuer.owns(left));
        assert_eq!(left.ordinal(), right.ordinal());
        assert_ne!(left, right);
        assert_ne!(left.reporting_key(), right.reporting_key());
    }

    #[test]
    fn cloned_issuers_cannot_reuse_body_coordinates() {
        let issuer = ProcedureIdIssuer::default();
        let clone = issuer.clone();
        assert_eq!(issuer.reporting_scope(), clone.reporting_scope());
        assert_ne!(
            issuer.reporting_scope(),
            ProcedureIdIssuer::default().reporting_scope()
        );
        let first = issuer.allocate();
        let second = clone.allocate();
        let third = issuer.allocate();
        assert!(issuer.owns(first) && issuer.owns(second) && issuer.owns(third));
        assert!(clone.owns(first) && clone.owns(second) && clone.owns(third));
        assert_eq!(
            (first.ordinal(), second.ordinal(), third.ordinal()),
            (0, 1, 2)
        );
        assert_ne!(first, second);
        assert_ne!(second, third);
    }
}
