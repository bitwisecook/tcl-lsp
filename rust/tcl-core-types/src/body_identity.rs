// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Opaque source-module body coordinates, without execution or name authority.

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

static NEXT_SCOPE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcedureId {
    scope: u64,
    ordinal: u32,
}

impl ProcedureId {
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
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
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
    pub fn allocate(&self) -> ProcedureId {
        let ordinal = self
            .0
            .next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
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
        let left = ProcedureIdIssuer::default().allocate();
        let right = ProcedureIdIssuer::default().allocate();
        assert_eq!(left.ordinal(), right.ordinal());
        assert_ne!(left, right);
    }

    #[test]
    fn cloned_issuers_cannot_reuse_body_coordinates() {
        let issuer = ProcedureIdIssuer::default();
        let clone = issuer.clone();
        let first = issuer.allocate();
        let second = clone.allocate();
        let third = issuer.allocate();
        assert_eq!(
            (first.ordinal(), second.ordinal(), third.ordinal()),
            (0, 1, 2)
        );
        assert_ne!(first, second);
        assert_ne!(second, third);
    }
}
