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
    /// Permit the native lowering of executable IR into NLIR
    /// (`crate::native_lowering`) and its consumption by a backend.
    NativeLowering,
    /// Permit the representation lattice to keep values native between
    /// operations; disabled, every value is boxed and every operation dynamic.
    RepresentationInference,
    /// Permit trace-barrier elision: values may stay in native shadows across
    /// cell accesses the variable-trace ledger proves unobserved.
    TraceBarrierElision,
    /// Permit demoting a proven-local procedure variable from a named cell to
    /// an indexed runtime slot.
    CellDemotion,
}

impl SemanticOptimisationPassId {
    /// Every pass identifier in a stable order, for explicit configuration UIs.
    #[must_use]
    pub const fn all() -> [Self; 12] {
        [
            Self::LegacyAnalysisSpecialisation,
            Self::GuardedIntrinsic,
            Self::CachedBoxedSlot,
            Self::MaterialisableSlot,
            Self::DirectProc,
            Self::NativeInteger,
            Self::FrameElision,
            Self::SemanticOperationSpecialisation,
            Self::NativeLowering,
            Self::RepresentationInference,
            Self::TraceBarrierElision,
            Self::CellDemotion,
        ]
    }

    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LegacyAnalysisSpecialisation => "legacy-analysis-specialisation",
            Self::GuardedIntrinsic => "guarded-intrinsic",
            Self::CachedBoxedSlot => "cached-boxed-slot",
            Self::MaterialisableSlot => "materialisable-slot",
            Self::DirectProc => "direct-proc",
            Self::NativeInteger => "native-integer",
            Self::FrameElision => "frame-elision",
            Self::SemanticOperationSpecialisation => "semantic-operation-specialisation",
            Self::NativeLowering => "native-lowering",
            Self::RepresentationInference => "representation-inference",
            Self::TraceBarrierElision => "trace-barrier-elision",
            Self::CellDemotion => "cell-demotion",
        }
    }

    /// Resolve a pass from its [`Self::as_str`] spelling.
    ///
    /// The inverse of `as_str`, so a CLI flag, an Explorer toggle and a JSON
    /// contract all name a pass identically. Unknown names return `None`
    /// rather than being ignored: an optimisation the caller asked for and
    /// did not get is a wrong answer, not a lenient one.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::all().into_iter().find(|id| id.as_str() == name)
    }

    const fn bit(self) -> u16 {
        match self {
            Self::LegacyAnalysisSpecialisation => 1 << 0,
            Self::GuardedIntrinsic => 1 << 1,
            Self::CachedBoxedSlot => 1 << 2,
            Self::MaterialisableSlot => 1 << 3,
            Self::DirectProc => 1 << 4,
            Self::NativeInteger => 1 << 5,
            Self::FrameElision => 1 << 6,
            Self::SemanticOperationSpecialisation => 1 << 7,
            Self::NativeLowering => 1 << 8,
            Self::RepresentationInference => 1 << 9,
            Self::TraceBarrierElision => 1 << 10,
            Self::CellDemotion => 1 << 11,
        }
    }
}

/// Explicit enablement for target-neutral semantic/AOT optimisation passes.
///
/// The compact bitset keeps compilation options `Copy`, while each bit remains
/// reachable only through a named [`SemanticOptimisationPassId`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SemanticOptimisationConfig {
    enabled: u16,
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

    /// The enabled passes, in [`SemanticOptimisationPassId::all`] order.
    pub fn enabled_passes(self) -> impl Iterator<Item = SemanticOptimisationPassId> {
        SemanticOptimisationPassId::all()
            .into_iter()
            .filter(move |pass| self.is_enabled(*pass))
    }

    /// Parse a comma-separated pass selection.
    ///
    /// Each element is a [`SemanticOptimisationPassId::as_str`] spelling or
    /// one of the two group names in [`PASS_GROUPS`]; empty elements and
    /// surrounding whitespace are ignored, so `"native-tier,"` and
    /// `" direct-proc , frame-elision "` both parse. An unrecognised name is
    /// an error naming the offender — silently dropping it would report an
    /// optimised build that was never optimised.
    ///
    /// ```
    /// use tcl_compiler::semantic_optimisation::{
    ///     SemanticOptimisationConfig as Config, SemanticOptimisationPassId as Pass,
    /// };
    /// let config = Config::from_names("direct-proc, frame-elision").unwrap();
    /// assert!(config.is_enabled(Pass::DirectProc));
    /// assert!(!config.is_enabled(Pass::NativeLowering));
    /// assert!(Config::from_names("no-such-pass").is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the unrecognised name, with the accepted spellings.
    pub fn from_names(spec: &str) -> Result<Self, String> {
        let mut config = Self::new();
        for name in spec.split(',').map(str::trim).filter(|n| !n.is_empty()) {
            if let Some(group) = PASS_GROUPS.iter().find(|(id, _)| *id == name) {
                for pass in group.1 {
                    config.enable(*pass);
                }
            } else if let Some(pass) = SemanticOptimisationPassId::from_name(name) {
                config.enable(pass);
            } else {
                return Err(format!(
                    "unknown optimisation pass `{name}`; expected one of: {}",
                    Self::accepted_names().join(", ")
                ));
            }
        }
        Ok(config)
    }

    /// Every name [`Self::from_names`] accepts: the group names first, then
    /// the individual passes in declaration order.
    #[must_use]
    pub fn accepted_names() -> Vec<&'static str> {
        PASS_GROUPS
            .iter()
            .map(|(id, _)| *id)
            .chain(
                SemanticOptimisationPassId::all()
                    .into_iter()
                    .map(Pass::as_str),
            )
            .collect()
    }
}

use SemanticOptimisationPassId as Pass;

/// Named sets a caller can select instead of listing passes one by one.
///
/// `native-tier` is the four passes
/// [`WasmCompileOptions::native_tier`](crate::codegen::wasm::WasmCompileOptions::native_tier)
/// enables, kept in step with it by
/// `native_tier_group_matches_the_wasm_option`; `all` is every pass, which is
/// what a "turn everything on and see what changes" run wants.
pub const PASS_GROUPS: &[(&str, &[SemanticOptimisationPassId])] = &[
    (
        "native-tier",
        &[
            Pass::NativeLowering,
            Pass::RepresentationInference,
            Pass::TraceBarrierElision,
            Pass::CellDemotion,
        ],
    ),
    ("all", &ALL_PASSES),
];

const ALL_PASSES: [SemanticOptimisationPassId; 12] = SemanticOptimisationPassId::all();

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

    #[test]
    fn every_pass_name_round_trips() {
        for pass in Pass::all() {
            assert_eq!(Pass::from_name(pass.as_str()), Some(pass), "{pass:?}");
        }
        assert_eq!(
            Pass::from_name("native-tier"),
            None,
            "a group is not a pass"
        );
        assert_eq!(Pass::from_name(""), None);
    }

    #[test]
    fn from_names_parses_passes_groups_and_whitespace() {
        let config = SemanticOptimisationConfig::from_names(" direct-proc , frame-elision ,")
            .expect("a valid selection");
        assert_eq!(
            config.enabled_passes().collect::<Vec<_>>(),
            vec![Pass::DirectProc, Pass::FrameElision]
        );
        assert!(
            SemanticOptimisationConfig::from_names("")
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            SemanticOptimisationConfig::from_names("all")
                .unwrap()
                .enabled_passes()
                .count(),
            Pass::all().len()
        );
        let error = SemanticOptimisationConfig::from_names("direct-proc,nope").unwrap_err();
        assert!(error.contains("`nope`"), "{error}");
        assert!(error.contains("native-tier"), "{error}");
    }

    /// The `native-tier` group and `WasmCompileOptions::native_tier` are two
    /// spellings of one set; a pass added to the option and not the group
    /// would make the CLI and the Explorer quietly weaker than the harness.
    #[test]
    fn native_tier_group_matches_the_wasm_option() {
        let group = SemanticOptimisationConfig::from_names("native-tier").unwrap();
        let option = crate::codegen::wasm::WasmCompileOptions::hosted()
            .native_tier()
            .semantic_optimisations();
        assert_eq!(group, option);
    }
}
