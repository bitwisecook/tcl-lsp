//! Optimisation profiles — named tiers controlling which optimisation
//! passes surface as diagnostics.
//!
//! Ports `shared/optimisation_profiles.py` + the `@opt(opt_category=…)`
//! metadata from the Python optimiser passes. Each optimisation code
//! belongs to one *category*; a profile enables a set of categories, and
//! [`profile_to_disabled`] returns the codes a profile turns off (the
//! complement of its enabled categories).
//!
//! The default editor profile is [`DEFAULT_EDITOR_PROFILE`]
//! (`Readability`) — idiomatic rewrites only; constant folding, DCE,
//! code motion, etc. are opt-in via a richer profile.

use std::collections::HashSet;

/// Optimisation-pass category — mirrors the Python `opt_category`
/// metadata declared on each `@opt(...)` pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptCategory {
    /// Idiomatic rewrites, no code removal/restructuring.
    Readability,
    /// Constant folding and propagation.
    ConstantFolding,
    /// Pattern recognition rewrites.
    Pattern,
    /// Dead-code / dead-store elimination.
    Dce,
    /// Code motion (hoisting / sinking).
    CodeMotion,
    /// Tail-call / recursion transforms.
    Recursion,
}

/// Named optimisation tiers — mirrors Python `OptimisationProfile`.
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

/// The default profile for editor / LSP surfaces — mirrors Python
/// `DEFAULT_EDITOR_PROFILE`.
pub const DEFAULT_EDITOR_PROFILE: OptimisationProfile = OptimisationProfile::Readability;

/// Every optimisation code with its category — the single Rust home for
/// the `opt_category` metadata Python declares per `@opt(...)` pass. Kept
/// in sync with `shared/codes.py::optimisation_codes_by_category`.
const OPT_CATEGORIES: &[(&str, OptCategory)] = &[
    // readability
    ("O111", OptCategory::Readability),
    ("O114", OptCategory::Readability),
    ("O115", OptCategory::Readability),
    ("O117", OptCategory::Readability),
    ("O120", OptCategory::Readability),
    ("O128", OptCategory::Readability),
    // constant_folding
    ("O100", OptCategory::ConstantFolding),
    ("O101", OptCategory::ConstantFolding),
    ("O102", OptCategory::ConstantFolding),
    ("O103", OptCategory::ConstantFolding),
    ("O105", OptCategory::ConstantFolding),
    ("O110", OptCategory::ConstantFolding),
    ("O113", OptCategory::ConstantFolding),
    ("O116", OptCategory::ConstantFolding),
    ("O118", OptCategory::ConstantFolding),
    ("O129", OptCategory::ConstantFolding),
    // pattern
    ("O104", OptCategory::Pattern),
    ("O119", OptCategory::Pattern),
    ("O130", OptCategory::Pattern),
    // dce
    ("O107", OptCategory::Dce),
    ("O108", OptCategory::Dce),
    ("O109", OptCategory::Dce),
    ("O112", OptCategory::Dce),
    ("O124", OptCategory::Dce),
    ("O126", OptCategory::Dce),
    // code_motion
    ("O125", OptCategory::CodeMotion),
    ("O127", OptCategory::CodeMotion),
    // recursion
    ("O121", OptCategory::Recursion),
    ("O122", OptCategory::Recursion),
    ("O123", OptCategory::Recursion),
];

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

    /// Whether this profile enables `category`.
    #[must_use]
    fn enables(self, category: OptCategory) -> bool {
        match self {
            Self::Off => false,
            Self::Full | Self::Aggressive => true,
            Self::Readability => category == OptCategory::Readability,
            Self::Standard => matches!(
                category,
                OptCategory::Readability | OptCategory::ConstantFolding | OptCategory::Pattern
            ),
        }
    }
}

/// The set of optimisation codes a `profile` turns *off* — the complement
/// of its enabled categories. Mirrors Python
/// `profile_to_disabled(profile)`.
#[must_use]
pub fn profile_to_disabled(profile: OptimisationProfile) -> HashSet<&'static str> {
    OPT_CATEGORIES
        .iter()
        .filter(|(_, cat)| !profile.enables(*cat))
        .map(|(code, _)| *code)
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
    }

    #[test]
    fn full_disables_nothing_off_disables_all() {
        assert!(profile_to_disabled(OptimisationProfile::Full).is_empty());
        assert_eq!(
            profile_to_disabled(OptimisationProfile::Off).len(),
            OPT_CATEGORIES.len()
        );
    }

    #[test]
    fn unknown_profile_falls_back_to_default() {
        assert_eq!(OptimisationProfile::parse("nope"), DEFAULT_EDITOR_PROFILE);
    }
}
