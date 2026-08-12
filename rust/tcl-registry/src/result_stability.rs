// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-owned result stability for common semantic optimisation.
//!
//! A command being side-effect-free does not imply that two invocations with
//! equal arguments return the same value.  `clock seconds`, for example, is a
//! read-only operation whose result changes independently of Tcl state.  This
//! descriptor keeps that distinction in the command registry so optimisers do
//! not infer determinism from the legacy `PURE` or `CSE_CANDIDATE` traits.

use crate::world_effect::WorldStateDomain;

/// How an invocation's result depends on state beyond its explicit operands.
///
/// This is deliberately independent of effect declarations.  Effects answer
/// what an invocation observes or changes; result stability answers whether
/// replaying the invocation with the same operand values can reproduce its
/// result.  A common optimiser must fail closed for [`Self::Unknown`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultStability {
    /// The registry has not declared a result-dependency contract.
    #[default]
    Unknown,

    /// The result depends only on the evaluated argument values.
    ///
    /// The invocation may still raise an error.  Reuse is sound after a
    /// dominating successful evaluation because an error prevents execution
    /// from reaching the later occurrence.
    ReferentiallyTransparent,

    /// The result additionally reads the listed versioned world-state
    /// domains.
    ///
    /// Consumers may reuse the result only when their value key includes the
    /// corresponding world-state versions.  A consumer that does not number
    /// world state must abstain.
    ReadsVersionedWorld(&'static [WorldStateDomain]),

    /// The result may change independently of tracked Tcl-world versions.
    ///
    /// Wall-clock, monotonic-clock, entropy, and similar observations belong
    /// here and must never be commoned merely because they are read-only.
    Volatile,
}

impl ResultStability {
    /// Whether argument value numbers alone are a complete result key.
    #[must_use]
    pub const fn is_referentially_transparent(self) -> bool {
        matches!(self, Self::ReferentiallyTransparent)
    }

    /// Versioned world-state domains required in addition to argument values.
    #[must_use]
    pub const fn versioned_world_dependencies(self) -> &'static [WorldStateDomain] {
        match self {
            Self::ReadsVersionedWorld(domains) => domains,
            Self::Unknown | Self::ReferentiallyTransparent | Self::Volatile => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_referential_results_need_no_world_version_in_the_value_key() {
        const HOST: &[WorldStateDomain] = &[WorldStateDomain::HostCapabilities];

        assert!(ResultStability::ReferentiallyTransparent.is_referentially_transparent());
        assert!(!ResultStability::Unknown.is_referentially_transparent());
        assert!(!ResultStability::ReadsVersionedWorld(HOST).is_referentially_transparent());
        assert!(!ResultStability::Volatile.is_referentially_transparent());
        assert_eq!(
            ResultStability::ReadsVersionedWorld(HOST).versioned_world_dependencies(),
            HOST
        );
    }
}
