//! Optimisation profiles — named tiers controlling which optimisation
//! passes surface as diagnostics.
//!
//! Each optimisation code belongs to one *category*; a profile enables a
//! set of categories, and [`profile_to_disabled`] returns the codes a
//! profile turns off (the complement of its enabled categories).
//!
//! The default editor profile is [`DEFAULT_EDITOR_PROFILE`]
//! (`Readability`) — idiomatic rewrites only; constant folding, DCE,
//! code motion, etc. are opt-in via a richer profile.

use std::collections::HashSet;

use tcl_core_types::DiagCode;
// The category vocabulary lives with `DiagCode` in `tcl-core-types`, where each
// optimisation code's `opt_category` metadata is declared — the single source
// the doc tables and this gate both read.
pub use tcl_core_types::OptCategory;

/// Named optimisation tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimisationProfile {
    /// All optimisations disabled.
    Off,
    /// Readability rewrites only.
    Readability,
    /// Readability + constant folding + pattern recognition.
    Standard,
    /// All passes, single pass.
    Full,
    /// All passes, multi-pass to fixpoint.
    Aggressive,
}

/// The default profile for editor / LSP surfaces.
pub const DEFAULT_EDITOR_PROFILE: OptimisationProfile = OptimisationProfile::Readability;

/// Every optimisation code with its category, derived from the `DiagCode`
/// metadata in `tcl-core-types` (the single home for the per-pass
/// `opt_category` — see [`DiagCode::opt_category`]).
fn opt_code_categories() -> impl Iterator<Item = (&'static str, OptCategory)> {
    DiagCode::ALL
        .iter()
        .filter_map(|c| c.opt_category().map(|cat| (c.as_str(), cat)))
}

impl OptimisationProfile {
    /// Parse a profile name (`"off"` / `"readability"` / `"standard"` /
    /// `"full"` / `"aggressive"`); unknown names fall back to
    /// [`DEFAULT_EDITOR_PROFILE`].
    #[must_use]
    pub fn parse(name: &str) -> Self {
        match name {
            "off" => Self::Off,
            "standard" => Self::Standard,
            "full" => Self::Full,
            "aggressive" => Self::Aggressive,
            "readability" => Self::Readability,
            _ => DEFAULT_EDITOR_PROFILE,
        }
    }

    /// The canonical lower-case profile name (round-trips through [`parse`]).
    ///
    /// [`parse`]: OptimisationProfile::parse
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Readability => "readability",
            Self::Standard => "standard",
            Self::Full => "full",
            Self::Aggressive => "aggressive",
        }
    }

    /// Whether this profile runs the optimiser to a multi-pass fixpoint. Only
    /// `aggressive` is multi-pass; every other profile is a single pass.
    #[must_use]
    pub fn is_multi_pass(self) -> bool {
        matches!(self, Self::Aggressive)
    }

    /// The maximum optimiser passes for this profile: 5 for the multi-pass
    /// `aggressive` tier, 1 for every single-pass profile.
    #[must_use]
    pub fn max_iterations(self) -> usize {
        if self.is_multi_pass() { 5 } else { 1 }
    }

    /// Whether this profile enables `category`. The `readability` / `standard`
    /// membership lives on [`OptCategory`] so the doc-table columns and this
    /// gate stay in lock-step.
    #[must_use]
    fn enables(self, category: OptCategory) -> bool {
        match self {
            Self::Off => false,
            Self::Full | Self::Aggressive => true,
            Self::Readability => category.in_readability_profile(),
            Self::Standard => category.in_standard_profile(),
        }
    }
}

/// The set of optimisation codes a `profile` turns *off* — the complement
/// of its enabled categories.
#[must_use]
pub fn profile_to_disabled(profile: OptimisationProfile) -> HashSet<&'static str> {
    opt_code_categories()
        .filter(|(_, cat)| !profile.enables(*cat))
        .map(|(code, _)| code)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readability_disables_constant_folding_and_dce() {
        let d = profile_to_disabled(OptimisationProfile::Readability);
        assert!(d.contains("O129")); // constant_folding
        assert!(d.contains("O116")); // constant_folding
        assert!(d.contains("O102")); // constant_folding
        assert!(d.contains("O109")); // dce
        assert!(!d.contains("O111")); // readability stays on
        assert!(!d.contains("O128")); // readability stays on
    }

    #[test]
    fn standard_keeps_constant_folding_drops_dce() {
        let d = profile_to_disabled(OptimisationProfile::Standard);
        assert!(!d.contains("O116")); // constant_folding on
        assert!(!d.contains("O129")); // constant_folding on
        assert!(d.contains("O109")); // dce off
        assert!(d.contains("O121")); // recursion off
        assert!(d.contains("O106")); // code_motion off under standard
    }

    #[test]
    fn o106_is_suppressable_code_motion() {
        // O106 (LICM) must be a known category entry so non-full profiles
        // can suppress it; readability/standard turn code_motion off.
        assert!(profile_to_disabled(OptimisationProfile::Readability).contains("O106"));
        assert!(profile_to_disabled(OptimisationProfile::Standard).contains("O106"));
        assert!(!profile_to_disabled(OptimisationProfile::Full).contains("O106"));
    }

    #[test]
    fn full_disables_nothing_off_disables_all() {
        assert!(profile_to_disabled(OptimisationProfile::Full).is_empty());
        assert_eq!(
            profile_to_disabled(OptimisationProfile::Off).len(),
            opt_code_categories().count()
        );
    }

    #[test]
    fn unknown_profile_falls_back_to_default() {
        assert_eq!(OptimisationProfile::parse("nope"), DEFAULT_EDITOR_PROFILE);
    }
}
