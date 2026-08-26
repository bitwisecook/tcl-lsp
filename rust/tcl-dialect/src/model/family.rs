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

//! The core-profile layer of the registry redesign: language *family* ×
//! *release* × *build profile* (design doc
//! `docs/design/dialect-and-package-registry-redesign.md` §0 layer 1 and
//! §3.1, review finding B1).
//!
//! A [`Family`] is a genuine core-language variant justified by an
//! observable lexical/syntactic or core-evaluation fingerprint (§2). A
//! [`Release`] is a point on one family's version ladder — releases are
//! data on the ladder, never separate catalogue rows. A [`BuildProfileId`]
//! is the semantic build axis (B1): the same release built differently has
//! a different character model, expr-function acceptance, and command
//! surface, and an unknown build answers capability queries with
//! [`CapabilityAnswer::Unknown`], never with a silently assumed default.
//!
//! [`CoreProfileId`] is the identity triple; [`CoreProfile`] is the
//! resolved grammar/expr/character-model/capability record every consumer
//! reads.

use crate::LexerGrammar;
use crate::grammar::{BracedVarStyle, EscapeSyntax, ExprCommentStyle, NumberSyntax};
use crate::model::expr_grammar::{self, ExprGrammar};
use crate::version::StringCharacterModel;

/// A core-language family: a variant with its own lexical/syntactic or
/// core-evaluation fingerprint no other family's ladder provides (§2).
///
/// Room to grow is deliberate: a future `SslicTcl` (issue #1543) becomes a
/// variant here **only** if it earns a grammar axis under the §2
/// classification rule; otherwise it is an environment. Picol is the
/// negative control: it is rejected explicitly rather than misdescribed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Family {
    /// Plain Tcl — the 8.4 … 9.1 release ladder.
    Tcl,
    /// F5 iRules — an embedded Tcl 8.4.6 core with its own lexical/expr
    /// fingerprint (the `}{` ghost separator, nine expr word operators).
    /// The K36322151 bans and closed-world guarantee are environment
    /// policy, not part of this identity (review B12).
    F5Irules,
    /// Jim Tcl — the 0.76 … 0.84 release ladder measured on the jim
    /// branch; its per-release grammar axis values land with P6.
    Jim,
}

impl Family {
    /// Every admitted family, in declaration order.
    pub const ALL: [Self; 3] = [Self::Tcl, Self::F5Irules, Self::Jim];

    /// The family's stable lower-case name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tcl => "tcl",
            Self::F5Irules => "f5-irules",
            Self::Jim => "jim",
        }
    }

    /// The family's release ladder, oldest first.
    #[must_use]
    pub const fn releases(self) -> &'static [Release] {
        match self {
            Self::Tcl => TCL_LADDER,
            Self::F5Irules => IRULES_LADDER,
            Self::Jim => JIM_LADDER,
        }
    }
}

impl std::fmt::Display for Family {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// One release on a family's ladder — an ordinal within the family plus
/// the family it belongs to, so a release value can never be applied to
/// the wrong ladder.
///
/// The derived total order sorts by family first, then by ladder
/// position; only the within-family order is semantically meaningful, and
/// consumers comparing releases across families hold a modelling bug the
/// family component makes visible rather than silent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Release {
    family: Family,
    ordinal: u8,
}

/// The Tcl family ladder.
const TCL_LADDER: &[Release] = &[
    Release::TCL_8_4,
    Release::TCL_8_5,
    Release::TCL_8_6,
    Release::TCL_9_0,
    Release::TCL_9_1,
];

/// The iRules ladder: a single TMOS-keyed release line for now (§3.1).
const IRULES_LADDER: &[Release] = &[Release::F5_IRULES_TMOS];

/// The Jim ladder measured on the jim branch.
const JIM_LADDER: &[Release] = &[
    Release::JIM_0_76,
    Release::JIM_0_77,
    Release::JIM_0_78,
    Release::JIM_0_79,
    Release::JIM_0_80,
    Release::JIM_0_81,
    Release::JIM_0_82,
    Release::JIM_0_83,
    Release::JIM_0_84,
];

impl Release {
    /// Tcl 8.4.
    pub const TCL_8_4: Self = Self::new(Family::Tcl, 0);
    /// Tcl 8.5.
    pub const TCL_8_5: Self = Self::new(Family::Tcl, 1);
    /// Tcl 8.6.
    pub const TCL_8_6: Self = Self::new(Family::Tcl, 2);
    /// Tcl 9.0.
    pub const TCL_9_0: Self = Self::new(Family::Tcl, 3);
    /// Tcl 9.1.
    pub const TCL_9_1: Self = Self::new(Family::Tcl, 4);
    /// The iRules single TMOS-keyed release line (spelled `tmos`).
    pub const F5_IRULES_TMOS: Self = Self::new(Family::F5Irules, 0);
    /// Jim 0.76.
    pub const JIM_0_76: Self = Self::new(Family::Jim, 0);
    /// Jim 0.77.
    pub const JIM_0_77: Self = Self::new(Family::Jim, 1);
    /// Jim 0.78.
    pub const JIM_0_78: Self = Self::new(Family::Jim, 2);
    /// Jim 0.79.
    pub const JIM_0_79: Self = Self::new(Family::Jim, 3);
    /// Jim 0.80.
    pub const JIM_0_80: Self = Self::new(Family::Jim, 4);
    /// Jim 0.81.
    pub const JIM_0_81: Self = Self::new(Family::Jim, 5);
    /// Jim 0.82.
    pub const JIM_0_82: Self = Self::new(Family::Jim, 6);
    /// Jim 0.83.
    pub const JIM_0_83: Self = Self::new(Family::Jim, 7);
    /// Jim 0.84.
    pub const JIM_0_84: Self = Self::new(Family::Jim, 8);

    const fn new(family: Family, ordinal: u8) -> Self {
        Self { family, ordinal }
    }

    /// The family whose ladder this release sits on.
    #[must_use]
    pub const fn family(self) -> Family {
        self.family
    }

    /// The release's position on its family ladder, oldest = 0.
    #[must_use]
    pub const fn ordinal(self) -> u8 {
        self.ordinal
    }

    /// The canonical spelling (`"8.6"`, `"tmos"`, `"0.84"`), unique across
    /// every family so [`Release::from_str`](std::str::FromStr) can resolve
    /// the family from the spelling alone.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match (self.family, self.ordinal) {
            (Family::Tcl, 0) => "8.4",
            (Family::Tcl, 1) => "8.5",
            (Family::Tcl, 2) => "8.6",
            (Family::Tcl, 3) => "9.0",
            (Family::Tcl, _) => "9.1",
            (Family::F5Irules, _) => "tmos",
            (Family::Jim, 0) => "0.76",
            (Family::Jim, 1) => "0.77",
            (Family::Jim, 2) => "0.78",
            (Family::Jim, 3) => "0.79",
            (Family::Jim, 4) => "0.80",
            (Family::Jim, 5) => "0.81",
            (Family::Jim, 6) => "0.82",
            (Family::Jim, 7) => "0.83",
            (Family::Jim, _) => "0.84",
        }
    }
}

impl std::fmt::Display for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A release spelling no family's ladder knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseParseError {
    /// The rejected spelling.
    pub spelling: String,
}

impl std::fmt::Display for ReleaseParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown release spelling `{}`", self.spelling)
    }
}

impl std::error::Error for ReleaseParseError {}

impl std::str::FromStr for Release {
    type Err = ReleaseParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Family::ALL
            .iter()
            .flat_map(|family| family.releases())
            .find(|release| release.as_str() == s)
            .copied()
            .ok_or_else(|| ReleaseParseError {
                spelling: s.to_owned(),
            })
    }
}

// The five lexer grammars, one value per (family, release ladder step)
// with a genuine delta. These mirror the private `GRAMMAR_*` constants in
// `profile.rs` — which cannot be referenced from here without modifying
// that module, and which die with it in a later phase — and the
// `grammar_values_match_the_old_catalogue` test pins byte-equality against
// the public catalogue so the two cannot drift while both exist.

const GRAMMAR_TCL84: LexerGrammar = LexerGrammar {
    expand_syntax: false,
    irules_brace_separator: false,
    braced_var: BracedVarStyle::FirstClose,
    script_skips_leading_bom: false,
    expr_comments: ExprCommentStyle::None,
    numbers: NumberSyntax::Tcl84,
    escapes: EscapeSyntax::Tcl84,
};

const GRAMMAR_TCL85: LexerGrammar = LexerGrammar {
    expand_syntax: true,
    numbers: NumberSyntax::Tcl85,
    ..GRAMMAR_TCL84
};

const GRAMMAR_TCL86: LexerGrammar = LexerGrammar {
    escapes: EscapeSyntax::Tcl86,
    ..GRAMMAR_TCL85
};

const GRAMMAR_TCL9X: LexerGrammar = LexerGrammar {
    expand_syntax: true,
    irules_brace_separator: false,
    braced_var: BracedVarStyle::Tcl9Nesting,
    script_skips_leading_bom: true,
    expr_comments: ExprCommentStyle::Hash,
    numbers: NumberSyntax::Tcl90,
    escapes: EscapeSyntax::Tcl90,
};

const GRAMMAR_IRULES: LexerGrammar = LexerGrammar {
    irules_brace_separator: true,
    ..GRAMMAR_TCL84
};

/// Interim Jim grammar: the permissive modern default, exactly as the old
/// model treats a dialect it has no measured values for. Jim's five
/// measured lexical axes (word separators, brace continuation, quote
/// termination, `$(…)` variable syntax, list parse) land with the jim
/// branch in P6 and replace this value.
const GRAMMAR_JIM_INTERIM: LexerGrammar = GRAMMAR_TCL9X;

/// The lexer grammar of `release` on `family`'s ladder — a total function
/// over admitted `(family, release)` pairs (design §3.1: grammar is a
/// function of family × release, not a catalogue row).
///
/// # Panics
/// If `release` does not sit on `family`'s ladder — a caller holding that
/// pair has already confused two ladders, which the typed [`Release`]
/// exists to prevent.
#[must_use]
pub const fn grammar(family: Family, release: Release) -> LexerGrammar {
    match (family, release.family) {
        (Family::Tcl, Family::Tcl)
        | (Family::F5Irules, Family::F5Irules)
        | (Family::Jim, Family::Jim) => {}
        _ => panic!("release is not on this family's ladder"),
    }
    match family {
        Family::Tcl => match release.ordinal {
            0 => GRAMMAR_TCL84,
            1 => GRAMMAR_TCL85,
            2 => GRAMMAR_TCL86,
            _ => GRAMMAR_TCL9X,
        },
        Family::F5Irules => GRAMMAR_IRULES,
        Family::Jim => GRAMMAR_JIM_INTERIM,
    }
}

/// The build/capability profile of a core — semantic, not metadata
/// (review B1): the same release built differently has a different
/// character model, expr-function acceptance, and command surface.
///
/// Minimal on purpose: families that are genuinely build-invariant
/// declare one canonical build. Named profiles (a Jim `--minimal`, a Tcl
/// `tcl-utf6` for the 8.x `TCL_UTF_MAX 6` EDA vendor builds — Q20) become
/// variants here once their probe columns exist; they are data, not
/// surgery, because every capability query already flows through
/// [`CapabilitySet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BuildProfileId {
    /// The family's measured canonical build for the release.
    #[default]
    Canonical,
    /// An unmeasured build: every capability query answers
    /// [`CapabilityAnswer::Unknown`], never the canonical default (B1).
    Unknown,
}

/// A three-valued capability answer: measured yes, measured no, or not
/// measured — the honest reading for an [`BuildProfileId::Unknown`]
/// build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityAnswer {
    /// Measured present.
    Yes,
    /// Measured absent.
    No,
    /// Not measured for this build — consumers abstain.
    Unknown,
}

/// The typed capability record of one core build (B1). Minimal to start;
/// each new probed axis is a field, so adding one reaches every consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilitySet {
    /// Whether the build's string machinery counts characters rather than
    /// bytes (Jim's `--utf8` configure choice: `é` is length 1 vs 2).
    pub utf8_character_model: CapabilityAnswer,
    /// Whether the expr math-function extension is compiled in (Jim's
    /// math extension is a configure choice; a `--minimal` build rejects
    /// `sqrt(4)` outright — §3.1).
    pub math_extension: CapabilityAnswer,
}

impl CapabilitySet {
    /// Every capability unmeasured — the [`BuildProfileId::Unknown`]
    /// resolution.
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            utf8_character_model: CapabilityAnswer::Unknown,
            math_extension: CapabilityAnswer::Unknown,
        }
    }

    /// The measured canonical-build capabilities of `family`.
    ///
    /// Tcl and iRules canonical builds ship full character handling and
    /// the math functions on every modelled release. Jim's canonical
    /// column is deliberately [`CapabilityAnswer::Unknown`] until the P6
    /// probe matrix lands in this tree — the jim branch's measurements are
    /// not yet data here, and guessing is what B1 forbids.
    #[must_use]
    pub const fn canonical(family: Family) -> Self {
        match family {
            Family::Tcl | Family::F5Irules => Self {
                utf8_character_model: CapabilityAnswer::Yes,
                math_extension: CapabilityAnswer::Yes,
            },
            Family::Jim => Self::unknown(),
        }
    }
}

/// The identity of one core: a release on a family ladder under a build
/// profile (§3.1's `CoreProfileId`).
///
/// The family is carried by [`Release`] itself rather than stored as a
/// third field, so an id whose family and release disagree is
/// unrepresentable; [`CoreProfileId::family`] recovers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoreProfileId {
    /// The release on the family's ladder.
    pub release: Release,
    /// The build profile (review B1 — semantic, not metadata).
    pub build: BuildProfileId,
}

impl CoreProfileId {
    /// The id of `release` under `build`.
    #[must_use]
    pub const fn new(release: Release, build: BuildProfileId) -> Self {
        Self { release, build }
    }

    /// The family whose ladder [`Self::release`] sits on.
    #[must_use]
    pub const fn family(self) -> Family {
        self.release.family()
    }

    /// Resolve the id to its grammar/expr/character-model/capability
    /// record.
    #[must_use]
    pub fn resolve(self) -> CoreProfile {
        let family = self.family();
        let capabilities = match self.build {
            BuildProfileId::Canonical => CapabilitySet::canonical(family),
            BuildProfileId::Unknown => CapabilitySet::unknown(),
        };
        CoreProfile {
            grammar: grammar(family, self.release),
            expr: expr_grammar::expr(family, self.release),
            character_model: self.character_model(),
            capabilities,
        }
    }

    /// The string/character model of this core, or `None` when the build
    /// (or an unmeasured family column) leaves it unknown.
    fn character_model(self) -> Option<StringCharacterModel> {
        if matches!(self.build, BuildProfileId::Unknown) {
            return None;
        }
        match self.family() {
            // 8.x counts UTF-16 code units; 9.x counts Unicode scalars.
            Family::Tcl => Some(if self.release.ordinal() >= 3 {
                StringCharacterModel::UnicodeScalars
            } else {
                StringCharacterModel::Utf16CodeUnits
            }),
            // iRules embeds a real Tcl 8.4.6.
            Family::F5Irules => Some(StringCharacterModel::Utf16CodeUnits),
            // Unmeasured here until the P6 probe matrix lands.
            Family::Jim => None,
        }
    }
}

/// One resolved core: the record grammar-and-semantics consumers read
/// (§3.1's `CoreProfile`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreProfile {
    /// The lexer grammar.
    pub grammar: LexerGrammar,
    /// The full expr-grammar contract (§3.1) — see
    /// [`crate::model::expr_grammar`].
    pub expr: &'static ExprGrammar,
    /// The string/character model, or `None` when the build leaves it
    /// unmeasured.
    pub character_model: Option<StringCharacterModel>,
    /// The typed build capabilities (B1).
    pub capabilities: CapabilitySet,
}

impl CoreProfile {
    /// Whether the expr math function `name` is callable on this core —
    /// gated by the build's math-extension capability, so an unknown
    /// build answers [`CapabilityAnswer::Unknown`] rather than the
    /// canonical set.
    #[must_use]
    pub fn mathfunc(&self, name: &str) -> CapabilityAnswer {
        match self.capabilities.math_extension {
            CapabilityAnswer::Unknown => CapabilityAnswer::Unknown,
            CapabilityAnswer::No => CapabilityAnswer::No,
            CapabilityAnswer::Yes => {
                if self.expr.mathfuncs.contains(name) {
                    CapabilityAnswer::Yes
                } else {
                    CapabilityAnswer::No
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DialectProfile;
    use std::str::FromStr;

    #[test]
    fn ladders_are_ordered_and_complete() {
        assert_eq!(Family::Tcl.releases().len(), 5);
        assert_eq!(Family::F5Irules.releases().len(), 1);
        assert_eq!(Family::Jim.releases().len(), 9);
        for family in Family::ALL {
            let ladder = family.releases();
            for (i, release) in ladder.iter().enumerate() {
                assert_eq!(release.family(), family);
                assert_eq!(usize::from(release.ordinal()), i);
            }
            let mut sorted = ladder.to_vec();
            sorted.sort();
            assert_eq!(ladder, sorted.as_slice(), "{family}: ladder is ordered");
        }
        assert!(Release::TCL_9_1 > Release::TCL_9_0);
        assert!(Release::TCL_8_4 < Release::TCL_9_1);
        assert!(Release::JIM_0_84 > Release::JIM_0_76);
    }

    #[test]
    fn release_spellings_round_trip_and_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for family in Family::ALL {
            for &release in family.releases() {
                let spelling = release.as_str();
                assert!(seen.insert(spelling), "duplicate spelling {spelling}");
                assert_eq!(Release::from_str(spelling), Ok(release));
                assert_eq!(release.to_string(), spelling);
            }
        }
        assert!(Release::from_str("8.7").is_err());
        assert!(Release::from_str("tcl8.6").is_err());
        assert!(Release::from_str("").is_err());
    }

    /// The grammar values here must stay byte-equal to the shipping
    /// catalogue's while both exist — `profile.rs`'s constants are private
    /// (and die in a later phase), so the pin runs through the public
    /// catalogue rather than referencing them directly.
    #[test]
    fn grammar_values_match_the_old_catalogue() {
        for (release, name) in [
            (Release::TCL_8_4, "tcl8.4"),
            (Release::TCL_8_5, "tcl8.5"),
            (Release::TCL_8_6, "tcl8.6"),
            (Release::TCL_9_0, "tcl9.0"),
            (Release::TCL_9_1, "tcl9.1"),
        ] {
            assert_eq!(
                grammar(Family::Tcl, release),
                DialectProfile::by_name(name).grammar,
                "{name}"
            );
        }
        assert_eq!(
            grammar(Family::F5Irules, Release::F5_IRULES_TMOS),
            DialectProfile::irules().grammar
        );
        // The interim Jim value is the same permissive default the old
        // model gives an unmeasured dialect.
        assert_eq!(
            grammar(Family::Jim, Release::JIM_0_84),
            DialectProfile::plain_tcl().grammar
        );
    }

    #[test]
    #[should_panic(expected = "not on this family's ladder")]
    fn grammar_rejects_a_release_from_another_ladder() {
        let _ = grammar(Family::Tcl, Release::JIM_0_76);
    }

    #[test]
    fn unknown_builds_answer_unknown() {
        let core = CoreProfileId::new(Release::TCL_9_0, BuildProfileId::Unknown).resolve();
        assert_eq!(core.capabilities, CapabilitySet::unknown());
        assert_eq!(core.character_model, None);
        assert_eq!(core.mathfunc("sqrt"), CapabilityAnswer::Unknown);
        assert_eq!(core.mathfunc("no-such-func"), CapabilityAnswer::Unknown);
    }

    #[test]
    fn canonical_builds_resolve_measured_capabilities() {
        let tcl90 = CoreProfileId::new(Release::TCL_9_0, BuildProfileId::Canonical).resolve();
        assert_eq!(
            tcl90.character_model,
            Some(StringCharacterModel::UnicodeScalars)
        );
        assert_eq!(tcl90.mathfunc("isnan"), CapabilityAnswer::Yes);
        assert_eq!(tcl90.mathfunc("cbrt"), CapabilityAnswer::No);

        let tcl86 = CoreProfileId::new(Release::TCL_8_6, BuildProfileId::Canonical).resolve();
        assert_eq!(
            tcl86.character_model,
            Some(StringCharacterModel::Utf16CodeUnits)
        );
        assert_eq!(tcl86.mathfunc("isnan"), CapabilityAnswer::No);
        assert_eq!(tcl86.mathfunc("min"), CapabilityAnswer::Yes);

        // iRules embeds a real 8.4: no TIP 232 additions.
        let irules =
            CoreProfileId::new(Release::F5_IRULES_TMOS, BuildProfileId::Canonical).resolve();
        assert_eq!(
            irules.character_model,
            Some(StringCharacterModel::Utf16CodeUnits)
        );
        assert_eq!(irules.mathfunc("sqrt"), CapabilityAnswer::Yes);
        assert_eq!(irules.mathfunc("min"), CapabilityAnswer::No);

        // Jim's canonical column is unmeasured in this tree (P6).
        let jim = CoreProfileId::new(Release::JIM_0_84, BuildProfileId::Canonical).resolve();
        assert_eq!(jim.mathfunc("sqrt"), CapabilityAnswer::Unknown);
        assert_eq!(jim.character_model, None);
    }

    #[test]
    fn core_profile_id_family_follows_the_release() {
        let id = CoreProfileId::new(Release::JIM_0_80, BuildProfileId::default());
        assert_eq!(id.family(), Family::Jim);
        assert_eq!(id.build, BuildProfileId::Canonical);
    }
}
