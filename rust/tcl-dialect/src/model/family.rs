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
use crate::grammar::{
    BraceLineContinuation, BracedVarStyle, EscapeSyntax, ExprCommentStyle, NumberSyntax,
};
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
    /// F5's shared Tcl fork — the **trunk** of the two-level F5 tree
    /// (owner rulings 2026-08-26, measured in
    /// `docs/design/bigip-irule-parser-measurements.md` §4a/§4b): a fork
    /// of Tcl at patchlevel 8.4.6 ([`Family::F5_FORK_POINT`]) that
    /// evolved independently, with a ladder keyed by TMOS release. Every
    /// BIG-IP-hosted Tcl context — TMM iRules, tmsh cli scripts, iApp
    /// implementations — carries this one parser: the implicit word
    /// break (R-rules), the brace-line continuation (N-rules), the inert
    /// `{*}` (the separator wins; expansion does not exist), 8.4
    /// numerals, and the nine word-form `expr` operators, all measured
    /// byte-identical across the three contexts. `f5-tmsh` and
    /// `f5-iapps` are environments riding this trunk directly, differing
    /// only in ambient packages and host facts.
    F5Tcl,
    /// F5 iRules — a dialect **offshoot of [`Family::F5Tcl`]** (a fork of
    /// a fork, owner ruling 2026-08-26). It inherits the trunk grammar
    /// whole — grammar/axis resolution walks the fork edge
    /// ([`Family::ancestry`]), so an axis the offshoot does not
    /// override answers from the trunk, and the trunk from Tcl at the
    /// fork point. What the offshoot adds is not lexical grammar but the
    /// rule compiler's load-time language rules: the declaration-only top
    /// level, closed-world command resolution at rule load, the event
    /// model, and `expr` math-function validation at load (measurements
    /// §4a/§4b/§6). The K36322151 command bans and the closed-world
    /// guarantee are environment policy, not part of this identity
    /// (review B12).
    F5Irules,
    /// Jim Tcl — the 0.76 … 0.84 release ladder, a **reimplementation**
    /// of Tcl rather than a source fork ([`Lineage::Reimplementation`],
    /// [`Family::ancestry`]): "Jim Tcl is a small footprint
    /// reimplementation of the Tcl scripting language. The core language
    /// engine is compatible with Tcl 8.5+, while implementing a
    /// significant subset of the Tcl 8.6 command set, plus additional
    /// features available only in Jim Tcl" (`jim_tcl.txt`, INTRODUCTION,
    /// upstream tag 0.84). It shares no source with Tcl and overrides
    /// every lexical and expr axis with its own measured values, but its
    /// *command surface* is derived from Tcl 8.6's, which is what the
    /// ancestry edge carries.
    Jim,
}

/// How a family's language derives from its ancestor's — provenance, not
/// a mechanism: the mechanism ([`Family::ancestry`]) is the same edge
/// either way, and stating the kind keeps the model from calling a
/// clean-room reimplementation a fork.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lineage {
    /// A source fork that then evolved independently — the `f5-tcl`
    /// trunk (a fork of Tcl 8.4.6) and the `f5-irules` offshoot (a fork
    /// of the trunk), owner rulings 2026-08-26.
    Fork,
    /// An independent implementation written to be compatible with the
    /// ancestor's language at a stated point — Jim Tcl, which shares no
    /// C source with Tcl yet targets its 8.5+ engine and 8.6 command
    /// set.
    Reimplementation,
}

/// One family's derivation edge: the ancestor family, the release on the
/// ancestor's ladder the derivation is anchored at, the version spelling
/// of that anchor, and the [`Lineage`] kind.
///
/// Axis and surface resolution walk this edge: an axis the descendant
/// does not override answers from the ancestor, and consumers that need a
/// point on the ancestor's version axis (the registry's lineage floors)
/// read [`Ancestry::anchor`] rather than re-deriving it per family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ancestry {
    /// The ancestor family.
    pub parent: Family,
    /// The release on the ancestor's ladder the derivation is anchored
    /// at.
    pub release: Release,
    /// The anchor's version spelling — the F5 trunk's measured
    /// patchlevel `8.4.6`, Jim's documented `8.6` command-set target. A
    /// spelling that is not a version (the iRules `tmos` line) yields no
    /// point on the ancestor's axis, which is exactly the intent.
    pub anchor: &'static str,
    /// Fork or reimplementation.
    pub lineage: Lineage,
}

/// The core Tcl commands Jim's ancestry edge inherits but Jim does not
/// have — the **override** half of inherit-then-override, recorded as
/// data so the gap is visible rather than silent.
///
/// Straight from `jim_tcl.txt`'s INTRODUCTION, which lists Jim's notable
/// differences from Tcl 8.5/8.6/8.7: "Threads and coroutines are not
/// supported. Command and variable traces are not supported." The
/// inherited Tcl 8.6 surface therefore over-admits these three heads
/// under a `jim` environment.
///
/// This list is deliberately *short and cited*, not a guess at Jim's full
/// surface delta: the whole delta — including everything Jim **adds**
/// (`ref`/`getref`/`setref`, `os.fork`, `lsubst`, `timerate`, the `$(…)`
/// expr shorthand) — is the jim surface pack, design **Q6**. The pack is
/// where a per-command negative declaration belongs; until it exists the
/// model states the known over-admission here rather than pretending
/// there is none.
pub const JIM_ABSENT_FROM_THE_INHERITED_SURFACE: [&str; 3] = ["coroutine", "thread", "trace"];

impl Family {
    /// Every admitted family, in declaration order.
    pub const ALL: [Self; 4] = [Self::Tcl, Self::F5Tcl, Self::F5Irules, Self::Jim];

    /// The Tcl patchlevel the F5 trunk forked from, after which it
    /// evolved independently (owner-attested, 2026-08-26; measured — all
    /// three BIG-IP contexts report `info patchlevel` 8.4.6 and fail
    /// every 8.4/8.5 discriminator as 8.4, measurements §4/§4a). This is
    /// fork *provenance*: it seeds the trunk's baseline grammar and
    /// surface, and nothing later on the Tcl ladder applies to the F5
    /// tree through it.
    pub const F5_FORK_POINT: &'static str = "8.4.6";

    /// The point on the Tcl ladder Jim's **command surface** is derived
    /// from: "implementing a significant subset of the Tcl 8.6 command
    /// set" (`jim_tcl.txt`, INTRODUCTION, upstream tag 0.84). The same
    /// paragraph states the *engine* compatibility separately ("compatible
    /// with Tcl 8.5+"), and the two are different facts — but nothing
    /// grammatical is inherited along this edge (Jim overrides every
    /// lexical and expr axis), so the anchor the model needs is the
    /// surface one. Anchoring at 8.5 instead would deny Jim `lmap`,
    /// `lassign`'s 8.6 siblings and `dict`, all of which it ships.
    pub const JIM_SURFACE_ANCHOR: &'static str = "8.6";

    /// The family's stable lower-case name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tcl => "tcl",
            Self::F5Tcl => "f5-tcl",
            Self::F5Irules => "f5-irules",
            Self::Jim => "jim",
        }
    }

    /// The family's release ladder, oldest first.
    #[must_use]
    pub const fn releases(self) -> &'static [Release] {
        match self {
            Self::Tcl => TCL_LADDER,
            Self::F5Tcl => F5_TCL_LADDER,
            Self::F5Irules => IRULES_LADDER,
            Self::Jim => JIM_LADDER,
        }
    }

    /// The derivation edge this family's grammar/axis/surface resolution
    /// walks up when it does not override an axis itself (§3.1). `None`
    /// for a root family — only [`Family::Tcl`] is one.
    ///
    /// [`Family::F5Tcl`] forks from Tcl at 8.4 (patchlevel
    /// [`Family::F5_FORK_POINT`]); [`Family::F5Irules`] forks from the
    /// `f5-tcl` trunk — a fork of a fork. The compiled grammar and expr
    /// tables below are *derived along these edges* (the trunk values are
    /// struct updates over the 8.4 values; the offshoot overrides
    /// nothing, so its values are the trunk's), and the tests pin that
    /// derivation so the edge and the data cannot drift apart.
    ///
    /// [`Family::Jim`]'s edge is a [`Lineage::Reimplementation`], not a
    /// fork, and it carries a different weight: Jim overrides **every**
    /// lexical and expr axis with its own measured values, so nothing
    /// grammatical is inherited. What the edge carries is the *command
    /// surface* — "implementing a significant subset of the Tcl 8.6
    /// command set" (`jim_tcl.txt`, INTRODUCTION) — which is precisely
    /// the inherit-then-override mechanism the old bare-vendor-bit model
    /// lacked, and whose absence made the jim branch re-author 76 core
    /// commands by hand (§1's wiring tax). The *override* half — Jim's
    /// documented absences (threads, coroutines, command and variable
    /// traces) and its own additions — is the jim surface pack, design
    /// **Q6**; until it lands the inherited surface over-admits exactly
    /// those, which `jim_inherits_the_tcl_surface_it_does_not_override`
    /// names rather than leaves silent.
    #[must_use]
    pub const fn ancestry(self) -> Option<Ancestry> {
        match self {
            Self::Tcl => None,
            Self::F5Tcl => Some(Ancestry {
                parent: Family::Tcl,
                release: Release::TCL_8_4,
                anchor: Self::F5_FORK_POINT,
                lineage: Lineage::Fork,
            }),
            Self::F5Irules => Some(Ancestry {
                parent: Family::F5Tcl,
                release: Release::F5_TCL_TMOS,
                anchor: Release::F5_TCL_TMOS.as_str(),
                lineage: Lineage::Fork,
            }),
            Self::Jim => Some(Ancestry {
                parent: Family::Tcl,
                release: Release::TCL_8_6,
                anchor: Self::JIM_SURFACE_ANCHOR,
                lineage: Lineage::Reimplementation,
            }),
        }
    }

    /// Whether the family's compiler resolves literal command heads at
    /// **load time** against a closed, explicit surface — the `f5-irules`
    /// offshoot marker (measurements §4a: a literal reference to a
    /// command TMM does not have is rejected when the rule is loaded,
    /// regardless of `catch`, where the other F5 contexts resolve at
    /// runtime; §4b: the surface itself splits into interpreter-absent
    /// and compiler-refused halves).
    ///
    /// Consumers use this to keep the offshoot's embedded ancestor core
    /// surface **explicit per spec** rather than implicitly admitted
    /// along the fork edge, and to grade "unknown command" as a hard
    /// error rather than a hint.
    #[must_use]
    pub const fn closed_load_time_resolution(self) -> bool {
        matches!(self, Self::F5Irules)
    }

    /// Whether the family's file top level is **declaration-only** — the
    /// second `f5-irules` offshoot marker (measurements §4b/§6): only
    /// `when`, `proc`, `priority`, `timing` are legal at the root of a
    /// rule (a bare `set` is `"set" unknown property` from the config
    /// layer), and a top-level `proc` is reachable only through the
    /// iRules-only `call` command. The enforcement machinery
    /// (`IrulesExecutionContext`, IRULE5006/5007) is consumer-side; the
    /// family carries the flag that gates it.
    #[must_use]
    pub const fn declaration_only_top_level(self) -> bool {
        matches!(self, Self::F5Irules)
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

/// The F5 trunk ladder: a single TMOS-keyed release line for now (§3.1);
/// post-fork deltas per TMOS release come from the evidence corpus
/// (F5 evidence review F2).
const F5_TCL_LADDER: &[Release] = &[Release::F5_TCL_TMOS];

/// The iRules offshoot ladder: the single TMM-hosted release line riding
/// the trunk's TMOS train (§3.1).
const IRULES_LADDER: &[Release] = &[Release::F5_IRULES_TMM];

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
    /// The F5 trunk's single TMOS-keyed release line (spelled `tmos`).
    pub const F5_TCL_TMOS: Self = Self::new(Family::F5Tcl, 0);
    /// The iRules offshoot's single release line (spelled `tmm` — the
    /// TMM-hosted rule engine riding the trunk's TMOS train).
    pub const F5_IRULES_TMM: Self = Self::new(Family::F5Irules, 0);
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
            (Family::F5Tcl, _) => "tmos",
            (Family::F5Irules, _) => "tmm",
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
    brace_line_continuation: BraceLineContinuation::Terminates,
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
    brace_line_continuation: BraceLineContinuation::Terminates,
    braced_var: BracedVarStyle::Tcl9Nesting,
    script_skips_leading_bom: true,
    expr_comments: ExprCommentStyle::Hash,
    numbers: NumberSyntax::Tcl90,
    escapes: EscapeSyntax::Tcl90,
};

/// The `f5-tcl` **trunk** grammar: everything unoverridden answers from
/// the fork point (`tcl@8.4.6` — the `..GRAMMAR_TCL84` update *is* the
/// [`Family::ancestry`] edge, stated as const derivation), plus the two
/// measured fork axes (`docs/design/bigip-irule-parser-measurements.md`
/// §1–§3, §4a): the implicit word break (R-rules) and the brace-line
/// continuation (N-rules). `expand_syntax` stays false from the 8.4 base,
/// which together with the separator makes `{*}` **inert** — a literal
/// `*` word plus the following word, never expansion, never an error
/// (measurements §1, §3 row 6).
const GRAMMAR_F5_TCL: LexerGrammar = LexerGrammar {
    irules_brace_separator: true,
    brace_line_continuation: BraceLineContinuation::Continues,
    ..GRAMMAR_TCL84
};

/// The `f5-irules` **offshoot** grammar: the offshoot overrides no
/// lexical axis, so every value answers from the trunk along the fork
/// edge (measurements §4a — the three F5 contexts are one parser). Its
/// deltas are load-time language rules and environment policy, not lexer
/// grammar.
const GRAMMAR_IRULES: LexerGrammar = GRAMMAR_F5_TCL;

/// Jim through 0.80, read out of the upstream sources at each tag (the
/// clone the P6 lane worked from; every claim below cites `jim.c` at
/// `0.84` unless a per-release note says otherwise):
///
/// - `expand_syntax`: **true**. Jim implements `{*}` — "A new addition to
///   Tcl 8.5 is the ability to expand a list into separate arguments.
///   Support for this feature is also available in Jim" (`jim_tcl.txt`,
///   LIST EXPANSION, present at 0.76 through 0.84).
/// - `braced_var`: **`FirstClose`**, the 8.x rule and *not* 9.x nesting.
///   `JimParseVar` scans `while (pc->len && *pc->p != '}')` — the first
///   close wins, byte-identical at 0.76, 0.80 and 0.84.
/// - `script_skips_leading_bom`: **false**. `jim.c` contains no BOM
///   handling at any modelled tag.
/// - `escapes`: **`Tcl90`**. `JimEscape` caps `\x` at two digits, has
///   `case 'U'` (up to eight), supports the `\u{NNN}` form, and
///   `utf8.h`'s `MAX_UTF8_LEN 4` gives UCS-4 internals — 9.0's grammar
///   and 9.0's width, not 8.6's U+FFFD degradation.
/// - `brace_line_continuation` / `irules_brace_separator`: **off**. Both
///   are F5 fork axes; Jim has neither.
///
/// One axis stays an honest approximation: `numbers` is `Tcl90` because
/// Jim's own numeral grammar is a fifth value the enum does not have.
/// `JimNumberBase` accepts `0x`/`0o`/`0b`/`0d` and — the load-bearing
/// half — "leading zeros do *not* imply octal", so `010` is ten. `Tcl90`
/// is right on both of those and on `0d`; it is wrong only in accepting
/// Tcl 9's `_` digit separators, which Jim rejects. `Tcl85` would be
/// wrong the dangerous way round (it reads `010` as octal 8), so the
/// closer value ships and the residue is recorded in P6's honest-gaps
/// list: the missing piece is a `NumberSyntax::Jim` variant, and adding
/// one is a 43-file, 231-site lexer change, not a data edit.
const GRAMMAR_JIM: LexerGrammar = LexerGrammar {
    expand_syntax: true,
    irules_brace_separator: false,
    brace_line_continuation: BraceLineContinuation::Terminates,
    braced_var: BracedVarStyle::FirstClose,
    script_skips_leading_bom: false,
    expr_comments: ExprCommentStyle::None,
    numbers: NumberSyntax::Tcl90,
    escapes: EscapeSyntax::Tcl90,
};

/// Jim from 0.81: `JimParseExpression` gained a `if (*pc->p == '#')
/// JimParseComment(pc)` arm at that tag (absent at 0.76 and 0.80,
/// present at 0.81 and 0.84), so `#` begins a comment inside `[expr]`.
/// Nothing else on the lexical ladder moves — which is the whole point:
/// nine near-identical profiles become one value and one struct update.
const GRAMMAR_JIM_0_81: LexerGrammar = LexerGrammar {
    expr_comments: ExprCommentStyle::Hash,
    ..GRAMMAR_JIM
};

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
        | (Family::F5Tcl, Family::F5Tcl)
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
        Family::F5Tcl => GRAMMAR_F5_TCL,
        Family::F5Irules => GRAMMAR_IRULES,
        Family::Jim => {
            if release.ordinal <= Release::JIM_0_80.ordinal {
                GRAMMAR_JIM
            } else {
                GRAMMAR_JIM_0_81
            }
        }
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
    /// The family's measured canonical build for the release — defined
    /// as *what a bare `./configure && make` produces at that release*,
    /// which is why [`CapabilitySet::canonical`] is keyed by
    /// [`Release`] and not by [`Family`]. Jim is the case that forces the
    /// distinction: its `auto.def` flipped the default at 0.82 ("Note
    /// that full is now the default", `auto.def:27`), so `Canonical`
    /// resolves to [`Self::JimMinimal`]'s capabilities through 0.81 and
    /// to [`Self::JimFull`]'s from 0.82. The same three words —
    /// "the canonical build" — name two different capability records on
    /// one ladder; a build axis that were mere metadata could not say
    /// that.
    #[default]
    Canonical,
    /// Jim's `--full` configure profile: UTF-8 string handling, the
    /// `expr` math functions, IPv6, SSL and the optional extensions
    /// (`binary`, `ensemble`, `json`, `tclprefix`, `zlib`) compiled in.
    /// The default from 0.82; before that it needed `--full` explicitly.
    JimFull,
    /// Jim's `--minimal` configure profile — "Disable some optional
    /// features: ipv6, ssl, math, utf8 and some extensions"
    /// (`auto.def:26`). Two of those are *language* facts, not library
    /// facts, which is review B1's whole claim: without `JIM_UTF8`,
    /// `utf8.h` defines `utf8_strlen` as `strlen` ("No utf-8 support.
    /// 1 byte = 1 char"), so `string length é` is 2; without
    /// `JIM_MATH_FUNCTIONS` the nineteen `#ifdef`-guarded rows of
    /// `Jim_ExprOperators` are not compiled, so `expr {sqrt(4)}` is a
    /// syntax error while `expr {int(4)}` still works — the seven
    /// unguarded functions survive. Nothing shipped in a `package
    /// require` can recover either. This is also the shape of a bare
    /// `./configure` through 0.81, where `utf8` and `math` were
    /// `--full`-only options.
    JimMinimal,
    /// The 32-bit `scriptd` build of the F5 trunk hosting iApp
    /// implementations: measured `tcl_platform(wordSize) == 4` against
    /// TMM's 8 (`docs/design/bigip-irule-parser-measurements.md` §4/§4a)
    /// — the same fork grammar and surface, a different word size. The
    /// build axis earning its place again (review B1).
    F5Scriptd32,
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
    /// Whether the build is a 64-bit word-size build
    /// (`tcl_platform(wordSize) == 8`). Measured to vary within one F5
    /// release: TMM reports 8, the iApp `scriptd` host reports **4** — a
    /// 32-bit build of the same trunk
    /// (`docs/design/bigip-irule-parser-measurements.md` §4).
    pub word_size_64: CapabilityAnswer,
}

impl CapabilitySet {
    /// Every capability unmeasured — the [`BuildProfileId::Unknown`]
    /// resolution.
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            utf8_character_model: CapabilityAnswer::Unknown,
            math_extension: CapabilityAnswer::Unknown,
            word_size_64: CapabilityAnswer::Unknown,
        }
    }

    /// Jim's `--full` capability column, read from `auto.def`: `utf8` and
    /// `math` are both on (`opt-bool-unless-minimal`), and Jim's integers
    /// are 64-bit by design ("Integers are 64bit", `jim_tcl.txt`
    /// INTRODUCTION — `jim_wide` is `long long` wherever
    /// `HAVE_LONG_LONG`).
    #[must_use]
    pub const fn jim_full() -> Self {
        Self {
            utf8_character_model: CapabilityAnswer::Yes,
            math_extension: CapabilityAnswer::Yes,
            word_size_64: CapabilityAnswer::Yes,
        }
    }

    /// Jim's `--minimal` capability column: `utf8` and `math` compiled
    /// out (`auto.def:26`), the 64-bit integer width untouched — it is
    /// not a configure option.
    #[must_use]
    pub const fn jim_minimal() -> Self {
        Self {
            utf8_character_model: CapabilityAnswer::No,
            math_extension: CapabilityAnswer::No,
            ..Self::jim_full()
        }
    }

    /// The measured capabilities of a bare `./configure` build at
    /// `release` — the [`BuildProfileId::Canonical`] resolution.
    ///
    /// The Tcl and F5 canonical builds ship full character handling, the
    /// math functions, and a 64-bit word size on every modelled release
    /// (TMM's fabricated `tcl_platform` reports `wordSize 8` —
    /// measurements §4).
    ///
    /// Jim's column is **release-keyed**, which is exactly why this
    /// function takes a [`Release`]. Through 0.81 `auto.def` guarded
    /// `utf8` and `math` behind `opt-bool <name> full`, so a default
    /// build had neither; 0.82's rewrite made `--full` the default and
    /// introduced `--minimal` to turn them off. So `jimsh 0.81` rejects
    /// `expr {sqrt(4)}` and `jimsh 0.82` answers `2.0`, from the same
    /// command with no flags — a release delta that lives entirely on
    /// the build axis.
    #[must_use]
    pub const fn canonical(release: Release) -> Self {
        match release.family {
            Family::Tcl | Family::F5Tcl | Family::F5Irules => Self {
                utf8_character_model: CapabilityAnswer::Yes,
                math_extension: CapabilityAnswer::Yes,
                word_size_64: CapabilityAnswer::Yes,
            },
            Family::Jim => {
                if release.ordinal <= Release::JIM_0_81.ordinal {
                    Self::jim_minimal()
                } else {
                    Self::jim_full()
                }
            }
        }
    }

    /// The measured capabilities of the F5 trunk's 32-bit `scriptd`
    /// build ([`BuildProfileId::F5Scriptd32`]): the canonical column with
    /// the word size measured **No** (`wordSize 4`, measurements §4).
    #[must_use]
    pub const fn f5_scriptd32(release: Release) -> Self {
        Self {
            word_size_64: CapabilityAnswer::No,
            ..Self::canonical(release)
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
        let capabilities = self.resolve_capabilities();
        CoreProfile {
            grammar: grammar(family, self.release),
            expr: expr_grammar::expr(family, self.release),
            character_model: self.character_model(),
            capabilities,
        }
    }

    /// The string/character model of this core, or `None` when this
    /// vocabulary cannot name it — either because the build is
    /// unmeasured, or because the measured answer is not one of the two
    /// **Tcl** counting models [`StringCharacterModel`] defines.
    ///
    /// Jim is the second case. Built with `JIM_UTF8` it counts Unicode
    /// scalars (`utf8.h`'s `MAX_UTF8_LEN 4`, code points to 0x1FFFFF), so
    /// the answer is [`StringCharacterModel::UnicodeScalars`]. Built
    /// without it, `utf8.h` says "No utf-8 support. 1 byte = 1 char" and
    /// the model counts **bytes** — a third counting rule the enum has no
    /// variant for. Answering with either Tcl model would be a wrong
    /// count rather than a missing one, so the honest answer is `None`
    /// (every consumer already abstains on it) and the measured fact
    /// travels on [`CapabilitySet::utf8_character_model`] instead. The
    /// missing piece is a `StringCharacterModel::Bytes` variant plus its
    /// `count_for` agreement rule; adding it is a change to constant
    /// folding for *every* dialect, so it is recorded, not smuggled in
    /// here.
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
            // The F5 tree forks from a real Tcl 8.4.6 — both the trunk
            // and the iRules offshoot keep the 8.x model (measurements
            // §4: every context reports patchlevel 8.4.6).
            Family::F5Tcl | Family::F5Irules => Some(StringCharacterModel::Utf16CodeUnits),
            Family::Jim => match self.resolve_capabilities().utf8_character_model {
                CapabilityAnswer::Yes => Some(StringCharacterModel::UnicodeScalars),
                CapabilityAnswer::No | CapabilityAnswer::Unknown => None,
            },
        }
    }

    /// The build's capability record — the half of [`Self::resolve`] that
    /// [`Self::character_model`] also needs.
    fn resolve_capabilities(self) -> CapabilitySet {
        match self.build {
            BuildProfileId::Canonical => CapabilitySet::canonical(self.release),
            BuildProfileId::JimFull => CapabilitySet::jim_full(),
            BuildProfileId::JimMinimal => CapabilitySet::jim_minimal(),
            BuildProfileId::F5Scriptd32 => CapabilitySet::f5_scriptd32(self.release),
            BuildProfileId::Unknown => CapabilitySet::unknown(),
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
    /// Whether the expr math function `name` is callable on this core.
    ///
    /// Two gates, in order: the release's mathfunc **set** must contain
    /// `name` at all, and — for the functions that need it — the build's
    /// math extension must be compiled in. The second gate is per
    /// function, not per build, because Jim proves the coarse reading
    /// wrong: `--minimal` drops the nineteen `#ifdef JIM_MATH_FUNCTIONS`
    /// rows of `Jim_ExprOperators` and keeps the seven unguarded ones, so
    /// `sqrt(4)` is a syntax error on a build where `int(4)`, `abs(-1)`
    /// and `rand()` all still answer. A build whose math capability is
    /// [`CapabilityAnswer::Unknown`] still abstains wholesale, per B1 —
    /// an unmeasured build is not evidence for either half.
    #[must_use]
    pub fn mathfunc(&self, name: &str) -> CapabilityAnswer {
        if self.capabilities.math_extension == CapabilityAnswer::Unknown {
            return CapabilityAnswer::Unknown;
        }
        let Some(func) = self.expr.mathfuncs.get(name) else {
            return CapabilityAnswer::No;
        };
        if func.needs_math_extension {
            self.capabilities.math_extension
        } else {
            CapabilityAnswer::Yes
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
        assert_eq!(Family::F5Tcl.releases().len(), 1);
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
                DialectProfile::find(name)
                    .expect("catalogue profile")
                    .grammar,
                "{name}"
            );
        }
        assert_eq!(
            grammar(Family::F5Irules, Release::F5_IRULES_TMM),
            DialectProfile::irules().grammar
        );
        // Jim is no longer the permissive stand-in: P6 replaced the
        // interim value with the measured one, and the two differ on
        // exactly the axes the sources name.
        let jim = grammar(Family::Jim, Release::JIM_0_84);
        assert_ne!(jim, DialectProfile::plain_tcl().grammar);
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

        // The F5 tree forks from a real 8.4.6: no TIP 232 additions, on
        // the trunk or the offshoot.
        for release in [Release::F5_TCL_TMOS, Release::F5_IRULES_TMM] {
            let core = CoreProfileId::new(release, BuildProfileId::Canonical).resolve();
            assert_eq!(
                core.character_model,
                Some(StringCharacterModel::Utf16CodeUnits),
                "{release}"
            );
            assert_eq!(core.mathfunc("sqrt"), CapabilityAnswer::Yes, "{release}");
            assert_eq!(core.mathfunc("min"), CapabilityAnswer::No, "{release}");
            assert_eq!(
                core.capabilities.word_size_64,
                CapabilityAnswer::Yes,
                "{release}: TMM wordSize 8 (measurements §4)"
            );
        }

        // The iApp scriptd host is a 32-bit build of the trunk —
        // measured `tcl_platform(wordSize) == 4` (measurements §4) —
        // with every other capability the canonical column's.
        let scriptd =
            CoreProfileId::new(Release::F5_TCL_TMOS, BuildProfileId::F5Scriptd32).resolve();
        assert_eq!(scriptd.capabilities.word_size_64, CapabilityAnswer::No);
        assert_eq!(scriptd.capabilities.math_extension, CapabilityAnswer::Yes);
        assert_eq!(
            scriptd.character_model,
            Some(StringCharacterModel::Utf16CodeUnits)
        );
        assert_eq!(scriptd.mathfunc("sqrt"), CapabilityAnswer::Yes);

        // Jim's canonical column is measured from `auto.def` as of P6 —
        // and it is release-keyed, because 0.82 flipped the default.
        let jim = CoreProfileId::new(Release::JIM_0_84, BuildProfileId::Canonical).resolve();
        assert_eq!(jim.mathfunc("sqrt"), CapabilityAnswer::Yes);
        assert_eq!(
            jim.mathfunc("min"),
            CapabilityAnswer::No,
            "Jim never had it"
        );
        assert_eq!(
            jim.character_model,
            Some(StringCharacterModel::UnicodeScalars)
        );
    }

    /// The two-level F5 tree (§0.2/§2, owner rulings 2026-08-26): grammar
    /// resolution walks the fork edges, so an axis the offshoot does not
    /// override answers from the trunk, and the trunk from `tcl@8.4.6`
    /// (`docs/design/bigip-irule-parser-measurements.md` §1–§4a).
    #[test]
    fn f5_grammar_resolution_walks_the_fork_edges() {
        assert_eq!(Family::Tcl.ancestry(), None, "Tcl is the only root");
        assert_eq!(
            Family::F5Tcl.ancestry(),
            Some(Ancestry {
                parent: Family::Tcl,
                release: Release::TCL_8_4,
                anchor: Family::F5_FORK_POINT,
                lineage: Lineage::Fork,
            }),
            "the trunk forks from Tcl at patchlevel {}",
            Family::F5_FORK_POINT
        );
        assert_eq!(
            Family::F5Irules.ancestry(),
            Some(Ancestry {
                parent: Family::F5Tcl,
                release: Release::F5_TCL_TMOS,
                anchor: "tmos",
                lineage: Lineage::Fork,
            }),
            "the offshoot is a fork of a fork"
        );

        // The offshoot overrides no lexical axis: its grammar IS the
        // trunk's (measurements §4a — one parser in all three contexts).
        let trunk = grammar(Family::F5Tcl, Release::F5_TCL_TMOS);
        let offshoot = grammar(Family::F5Irules, Release::F5_IRULES_TMM);
        assert_eq!(trunk, offshoot);

        // The trunk overrides exactly the two measured fork axes; every
        // other axis answers from the fork point tcl@8.4.6.
        let fork_point = grammar(Family::Tcl, Release::TCL_8_4);
        assert!(trunk.irules_brace_separator, "R-rules (measurements §1)");
        assert!(
            trunk.brace_line_continuation.continues(),
            "N-rules (measurements §2)"
        );
        assert_eq!(
            LexerGrammar {
                irules_brace_separator: false,
                brace_line_continuation: BraceLineContinuation::Terminates,
                ..trunk
            },
            fork_point,
            "unoverridden axes answer from tcl@8.4.6"
        );
        // `{*}` is inert on the whole tree: no expansion axis anywhere
        // (measurements §1 `{*}`, §3 row 6 — the separator wins).
        assert!(!trunk.expand_syntax);
        assert!(!offshoot.expand_syntax);
    }

    /// P6: Jim's lexer grammar is measured, not the permissive stand-in,
    /// and the nine `jim0.76`–`jim0.84` profiles collapse into one value
    /// plus one struct update.
    #[test]
    fn the_jim_grammar_is_measured_and_is_a_ladder() {
        let early = grammar(Family::Jim, Release::JIM_0_76);
        let late = grammar(Family::Jim, Release::JIM_0_84);

        // The two measured corrections against the interim value, which
        // was Tcl 9's grammar wholesale.
        assert_eq!(
            late.braced_var,
            BracedVarStyle::FirstClose,
            "`JimParseVar` stops at the first `}}`, the 8.x rule"
        );
        assert!(
            !late.script_skips_leading_bom,
            "`jim.c` has no BOM handling at any modelled tag"
        );
        let tcl9 = grammar(Family::Tcl, Release::TCL_9_0);
        assert_ne!(late.braced_var, tcl9.braced_var);
        assert_ne!(late.script_skips_leading_bom, tcl9.script_skips_leading_bom);

        // What Jim does share with the modern Tcl values.
        assert!(late.expand_syntax, "Jim implements `{{*}}`");
        assert_eq!(late.escapes, EscapeSyntax::Tcl90);
        assert_eq!(late.numbers, NumberSyntax::Tcl90);

        // Neither F5 fork axis is Jim's.
        for g in [early, late] {
            assert!(!g.irules_brace_separator);
            assert_eq!(g.brace_line_continuation, BraceLineContinuation::Terminates);
        }

        // The one lexical step on the ladder: expr comments at 0.81.
        for release in [Release::JIM_0_76, Release::JIM_0_80] {
            assert_eq!(
                grammar(Family::Jim, release).expr_comments,
                ExprCommentStyle::None,
                "{release}"
            );
        }
        for release in [Release::JIM_0_81, Release::JIM_0_84] {
            assert_eq!(
                grammar(Family::Jim, release).expr_comments,
                ExprCommentStyle::Hash,
                "{release}"
            );
        }
        // Every other axis is constant across the whole nine-release
        // ladder — which is why nine profiles were nine copies.
        for &release in Family::Jim.releases() {
            let g = grammar(Family::Jim, release);
            assert_eq!(
                LexerGrammar {
                    expr_comments: ExprCommentStyle::None,
                    ..g
                },
                early,
                "{release}"
            );
        }
    }

    /// Jim's ancestry edge is a **reimplementation**, not a fork, and it
    /// carries the command surface rather than any grammar: every
    /// lexical axis above is Jim's own, while the surface anchor points
    /// at Tcl 8.6.
    #[test]
    fn jim_inherits_the_tcl_surface_it_does_not_override() {
        let ancestry = Family::Jim.ancestry().expect("Jim derives from Tcl");
        assert_eq!(ancestry.parent, Family::Tcl);
        assert_eq!(ancestry.release, Release::TCL_8_6);
        assert_eq!(ancestry.anchor, Family::JIM_SURFACE_ANCHOR);
        assert_eq!(
            ancestry.lineage,
            Lineage::Reimplementation,
            "Jim shares no source with Tcl"
        );
        assert_eq!(
            Family::F5Tcl.ancestry().expect("edge").lineage,
            Lineage::Fork,
            "the F5 trunk really is a fork"
        );
        // Grammar is emphatically NOT inherited along this edge: the
        // anchor's own grammar and Jim's disagree.
        assert_ne!(
            grammar(Family::Jim, Release::JIM_0_84),
            grammar(Family::Tcl, ancestry.release)
        );
        // The override half of inherit-then-override is design Q6's jim
        // surface pack. Until it lands the inherited Tcl 8.6 surface
        // over-admits exactly what `jim_tcl.txt` says Jim does not have
        // — "Threads and coroutines are not supported. Command and
        // variable traces are not supported." This assertion is the
        // marker: it fails the day the pack lands, which is when the
        // list must move into pack data.
        assert_eq!(
            JIM_ABSENT_FROM_THE_INHERITED_SURFACE,
            ["coroutine", "thread", "trace"],
            "the recorded over-admission, pending the Q6 surface pack"
        );
    }

    /// Review B1, proved twice over on one ladder: `--minimal` is a
    /// different *language*, and "the canonical build" names two
    /// different capability records on the Jim ladder because 0.82
    /// flipped `auto.def`'s default.
    #[test]
    fn the_jim_build_axis_is_semantic() {
        let full = CoreProfileId::new(Release::JIM_0_84, BuildProfileId::JimFull).resolve();
        let minimal = CoreProfileId::new(Release::JIM_0_84, BuildProfileId::JimMinimal).resolve();

        // The math extension is per *function*, not per build: the seven
        // unguarded rows survive `--minimal`.
        assert_eq!(full.mathfunc("sqrt"), CapabilityAnswer::Yes);
        assert_eq!(minimal.mathfunc("sqrt"), CapabilityAnswer::No);
        for survivor in ["int", "wide", "abs", "double", "round", "rand", "srand"] {
            assert_eq!(
                minimal.mathfunc(survivor),
                CapabilityAnswer::Yes,
                "{survivor} is outside `#ifdef JIM_MATH_FUNCTIONS`"
            );
        }
        // A function no Jim release ever had is `No` under either build.
        for never in ["min", "max", "entier", "bool", "isqrt"] {
            assert_eq!(full.mathfunc(never), CapabilityAnswer::No, "{never}");
            assert_eq!(minimal.mathfunc(never), CapabilityAnswer::No, "{never}");
        }

        // The character model differs too, and the honest answer for the
        // byte-counting build is "this vocabulary cannot name it".
        assert_eq!(
            full.character_model,
            Some(StringCharacterModel::UnicodeScalars)
        );
        assert_eq!(minimal.character_model, None);
        assert_eq!(
            minimal.capabilities.utf8_character_model,
            CapabilityAnswer::No,
            "the measured fact travels on the capability record"
        );
        // Word size is not a configure option: 64-bit either way.
        assert_eq!(full.capabilities.word_size_64, CapabilityAnswer::Yes);
        assert_eq!(minimal.capabilities.word_size_64, CapabilityAnswer::Yes);

        // The default flip: `./configure` with no flags is `--minimal`'s
        // column through 0.81 and `--full`'s from 0.82.
        for release in [Release::JIM_0_76, Release::JIM_0_81] {
            assert_eq!(
                CapabilitySet::canonical(release),
                CapabilitySet::jim_minimal(),
                "{release}: utf8 and math were `--full`-only options"
            );
        }
        for release in [Release::JIM_0_82, Release::JIM_0_84] {
            assert_eq!(
                CapabilitySet::canonical(release),
                CapabilitySet::jim_full(),
                "{release}: `--full` is now the default"
            );
        }
        // So `expr {sqrt(4)}` is a syntax error on a stock 0.81 and
        // answers on a stock 0.82 — one command, no flags, two answers.
        assert_eq!(
            CoreProfileId::new(Release::JIM_0_81, BuildProfileId::Canonical)
                .resolve()
                .mathfunc("sqrt"),
            CapabilityAnswer::No
        );
        assert_eq!(
            CoreProfileId::new(Release::JIM_0_82, BuildProfileId::Canonical)
                .resolve()
                .mathfunc("sqrt"),
            CapabilityAnswer::Yes
        );
        // An unmeasured build still abstains wholesale (B1).
        let unknown = CoreProfileId::new(Release::JIM_0_84, BuildProfileId::Unknown).resolve();
        assert_eq!(unknown.mathfunc("sqrt"), CapabilityAnswer::Unknown);
        assert_eq!(unknown.mathfunc("int"), CapabilityAnswer::Unknown);
    }

    #[test]
    fn core_profile_id_family_follows_the_release() {
        let id = CoreProfileId::new(Release::JIM_0_80, BuildProfileId::default());
        assert_eq!(id.family(), Family::Jim);
        assert_eq!(id.build, BuildProfileId::Canonical);
    }
}
