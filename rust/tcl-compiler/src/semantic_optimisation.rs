// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target-neutral configuration for semantic and AOT optimisation passes.
//!
//! These passes transform or specialise executable semantics; they are not
//! source-rewrite diagnostics and therefore deliberately do not share
//! [`crate::optimiser::PassId`].  The default is empty: generic runtime
//! invocation and general lowering remain available without authorising any
//! optimisation.

/// One individually selectable target-neutral semantic/AOT optimisation pass.
///
/// A pass identifies a semantic authorisation, not a target instruction
/// sequence. A backend may consume an enabled pass only after its own target
/// requirements are met.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticOptimisationPassId {
    /// Permit the pre-common-proof analysis-derived specialisation tier.
    ///
    /// Its current consumer is the compatibility WASM structured emitter. It
    /// remains separately named so replacing that consumer with common proof
    /// plans does not silently make it a default optimisation.
    LegacyAnalysisSpecialisation,
    /// Permit a future common guarded fast path with an exact generic slow path.
    GuardedIntrinsic,
    /// Permit a future cached boxed Tcl-object slot.
    CachedBoxedSlot,
    /// Permit a future slot that materialises a Tcl object at its boundaries.
    MaterialisableSlot,
    /// Permit a future direct procedure-call plan.
    DirectProc,
    /// Permit a future integer-specialised value plan.
    NativeInteger,
    /// Permit a future frame-elision plan.
    FrameElision,
    /// Permit registry-resolved semantic operation and boundary proofs.
    SemanticOperationSpecialisation,
}

impl SemanticOptimisationPassId {
    /// Every pass identifier in a stable order, for explicit configuration UIs.
    #[must_use]
    pub const fn all() -> [Self; 8] {
        [
            Self::LegacyAnalysisSpecialisation,
            Self::GuardedIntrinsic,
            Self::CachedBoxedSlot,
            Self::MaterialisableSlot,
            Self::DirectProc,
            Self::NativeInteger,
            Self::FrameElision,
            Self::SemanticOperationSpecialisation,
        ]
    }

    const fn bit(self) -> u8 {
        match self {
            Self::LegacyAnalysisSpecialisation => 1 << 0,
            Self::GuardedIntrinsic => 1 << 1,
            Self::CachedBoxedSlot => 1 << 2,
            Self::MaterialisableSlot => 1 << 3,
            Self::DirectProc => 1 << 4,
            Self::NativeInteger => 1 << 5,
            Self::FrameElision => 1 << 6,
            Self::SemanticOperationSpecialisation => 1 << 7,
        }
    }
}

/// Explicit enablement for target-neutral semantic/AOT optimisation passes.
///
/// The compact bitset keeps compilation options `Copy`, while each bit remains
/// reachable only through a named [`SemanticOptimisationPassId`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SemanticOptimisationConfig {
    enabled: u8,
}

impl SemanticOptimisationConfig {
    /// Construct a configuration with every semantic/AOT optimisation disabled.
    #[must_use]
    pub const fn new() -> Self {
        Self { enabled: 0 }
    }

    /// Return whether no semantic/AOT optimisation pass is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.enabled == 0
    }

    /// Return whether `pass` is explicitly enabled.
    #[must_use]
    pub const fn is_enabled(self, pass: SemanticOptimisationPassId) -> bool {
        self.enabled & pass.bit() != 0
    }

    /// Return a copy with `pass` enabled.
    #[must_use]
    pub const fn with_enabled(mut self, pass: SemanticOptimisationPassId) -> Self {
        self.enable(pass);
        self
    }

    /// Enable one semantic/AOT optimisation pass.
    pub const fn enable(&mut self, pass: SemanticOptimisationPassId) {
        self.enabled |= pass.bit();
    }

    /// Disable one semantic/AOT optimisation pass.
    pub const fn disable(&mut self, pass: SemanticOptimisationPassId) {
        self.enabled &= !pass.bit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty_and_passes_are_individually_toggleable() {
        let mut config = SemanticOptimisationConfig::default();
        assert!(config.is_empty());
        for pass in SemanticOptimisationPassId::all() {
            assert!(!config.is_enabled(pass));
            config.enable(pass);
            assert!(config.is_enabled(pass));
            config.disable(pass);
            assert!(!config.is_enabled(pass));
        }
        assert!(config.is_empty());
    }
}
