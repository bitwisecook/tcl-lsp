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

//! Frame-effect descriptors — which arguments of a command select a stack
//! frame, name a variable *in* that frame, or carry a script that runs
//! there.
//!
//! Tcl's frame-crossing primitives are few but their argument grammars are
//! all different, and every consumer that reasons about caller-frame
//! injection (`tcl_compiler`'s frame-effect summaries, the parameter-trait
//! inference, the analyser's alias handlers) needs the same three answers:
//!
//! 1. **Is there a level word, and where?** `upvar` probes its leading word
//!    before Tcl 8.6 and uses count parity from 8.6 and in current Jim;
//!    `uplevel` uses its selected leading-word probe.
//! 2. **What do the remaining arguments do?**  `upvar` takes
//!    `otherVar myVar` pairs; `uplevel` concatenates a script.
//! 3. **Which frame does the effect land in?**  The one the level word
//!    selects, the current one, or the command's own caller's.
//!
//! Answering them by matching command names in the consumer is exactly the
//! duplication the registry exists to prevent — three copies of the level
//! rule had already drifted apart before this descriptor existed, two of
//! them wrong (see [`FrameLevelWord::ArityParity`]).
//!
//! # C Tcl provenance
//!
//! The modern parity example below is pinned against `tclsh 9.0.4` and
//! `tclsh 8.6.14`. Native Tcl 8.4/8.5 instead consumes a digit/hash-leading
//! first word as the level; with two arguments that branch errors before
//! installing a link. Successful-path projection preserves that distinction:
//!
//! ```tcl
//! proc t3 {} { return [catch {set b} e]:$e }   ;# body: upvar 1 b
//! proc h3 {} { set 1 ONE; return [t3] }
//! h3   ;# → 0:ONE   — `1` is the *otherVar*, not a level
//!
//! proc t6 {} { return [catch {upvar foo bar baz} e]:$e }
//! t6   ;# → 1:bad level "foo"   — 3 words ⇒ the first IS the level
//! ```
//!
//! Neither the **level-value** grammar ([`FrameLevel::parse_for`]) nor the
//! **level-word presence** rule of a [`FrameLevelWord::LeadingProbe`] command
//! (`uplevel`) is version-invariant.
//!
//! The level value is read by `Tcl_GetIntFromObj`, so it inherits every
//! difference in the release's numeral grammar: 8.6 and 9.0 select *different
//! frames* for `010` (8 up vs 10 up), and one errors where the other succeeds
//! for `08`, `0d1` and `1_0`.  See [`FrameLevel::parse_for`] for the pinned
//! matrix.  A small sample can hide this: a 20-spelling matrix that
//! happens to contain no divergent spelling, plus a call chain too shallow
//! to distinguish a parse failure from an out-of-range level (C reports
//! `bad level` for both), can look invariant when it is not.
//!
//! For the presence rule's two divergent classes and their transcripts, see
//! [`FrameLevelWord::LeadingProbe`].

mod native_object;
pub use native_object::{
    NativeFrameLevelFailure, NativeFrameLevelObject, NativeFrameLevelResolution,
};

use tcl_dialect::{FrameLevelPresence, NumberSyntax, TclVersion};
use tcl_syntax::number::ParseFlags;

/// Which stack frame an `upvar` / `uplevel` level word selects.
///
/// `Relative(1)` — the caller — is the default whenever the word is absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameLevel {
    /// `N` — *N* frames up from the current one. `Relative(0)` is the
    /// current frame itself (`uplevel 0 …` re-enters it; `upvar 0 x y`
    /// aliases a *local*).
    Relative(u32),
    /// `#N` — absolute frame number counted down from the global frame.
    /// `Absolute(0)` is the global frame, whatever the call depth.
    Absolute(u32),
    /// The word is present but its value is computed at run time
    /// (`upvar $lvl x y`, `uplevel [expr {$n-1}] $s`).
    Dynamic,
}

/// Original native level-reference cache, independently of a valid target frame.
/// C8.5 can retain a signed relative distance even when frame lookup fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeFrameLevelCache {
    /// C8.5's signed relative distance.
    Relative(i32),
    /// An absolute native frame level.
    Absolute(i32),
}

/// Actual object conversion order for native frame-level lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeFrameLevelProtocol {
    version: Option<tcl_dialect::TclVersion>,
}

impl NativeFrameLevelProtocol {
    /// C8.6+ probes the original integer cache before generating its string.
    #[must_use]
    pub fn probes_integer_first(self) -> bool {
        self.version
            .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_6)
    }

    /// Jim obtains the original string before its native long conversion.
    #[must_use]
    pub const fn is_jim084(self) -> bool {
        self.version.is_none()
    }

    /// Actual C release, independently of authoring or lexical overrides.
    #[must_use]
    pub const fn tcl_version(self) -> Option<tcl_dialect::TclVersion> {
        self.version
    }

    /// Original C frame-error append/format String producer. Jim's error
    /// constructor supplies no C String primary. This retains result birth,
    /// independently of selector spelling, error codes and getter caches.
    #[must_use]
    pub fn bad_level_string_result(
        self,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        self.version
            .map(tcl_syntax::native_string::NativeStringProtocol::C)
    }

    /// Whether this original level-reference cache belongs to the native recipe.
    /// Cache validity does not imply the selected call frame exists.
    #[must_use]
    pub fn accepts_cache(self, cache: NativeFrameLevelCache) -> bool {
        match (self.version, cache) {
            (Some(tcl_dialect::TclVersion::V8_5), NativeFrameLevelCache::Relative(_)) => true,
            (Some(version), NativeFrameLevelCache::Absolute(level)) => {
                version >= tcl_dialect::TclVersion::V8_5 && level >= 0
            }
            _ => false,
        }
    }
}

impl crate::InvocationDialect {
    /// Select the actual native frame-object conversion protocol.
    /// A compatible vendor or logical simulation does not issue native caches.
    #[must_use]
    pub fn native_frame_level_protocol(self) -> Option<NativeFrameLevelProtocol> {
        let getter = self.native_scalar_getter_protocol()?;
        Some(NativeFrameLevelProtocol {
            version: getter.tcl_version(),
        })
    }
}

impl FrameLevel {
    /// The frame every level-taking command targets when its level word is
    /// omitted — the immediate caller.
    pub const DEFAULT: Self = Self::Relative(1);

    /// True when this level names the **immediate caller's** frame, the one
    /// a per-proc frame-effect summary can hand to a call site.
    #[must_use]
    pub const fn is_caller_frame(self) -> bool {
        matches!(self, Self::Relative(1))
    }

    /// True when this level names the frame the command is *written* in —
    /// `uplevel 0 …`, whose script shares the current frame's variables.
    #[must_use]
    pub const fn is_current_frame(self) -> bool {
        matches!(self, Self::Relative(0))
    }

    /// True when this level names the global frame (`#0`).
    #[must_use]
    pub const fn is_global_frame(self) -> bool {
        matches!(self, Self::Absolute(0))
    }

    /// Parse a level word into the frame it names, or `None` when the value
    /// is not a frame at all (C Tcl's `bad level "…"`), **abstaining** with
    /// [`Self::Dynamic`] for a word the releases read differently.
    ///
    /// Equivalent to [`Self::parse_for`] with no version. Prefer `parse_for`
    /// wherever the target release is known — it resolves the spellings this
    /// one has to abstain on.
    ///
    /// | word | frame | | word | frame |
    /// |---|---|---|---|---|
    /// | `1`, `+1`, `" 1"`, `"  1  "`, `0x1`, `0X1`, `0b1`, `0o1`, `"0x1 "` | caller | | `#0`, `#-0`, `"# 0"`, `"#0 "` | global |
    /// | `0`, `+0`, `-0` | current | | `#1`, `#+1`, `#0x1` | frame 1 |
    /// | `2` | caller's caller | | `7`, `007` | 7 up |
    /// | `-1`, `-2`, `1.0`, `1e0`, `"+ 1"`, `--0`, `x`, `#x`, `#-1`, `""`, `" #0"` | none — `bad level` |
    /// | `010`, `08`, `0d1`, `1_0` | `Dynamic` — release-dependent, see [`Self::parse_for`] |
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::parse_for(word, None)
    }

    /// Parse a level word into the frame it names under `version`.
    ///
    /// A level word is read by `Tcl_GetIntFromObj`, optionally behind a leading
    /// `#`, so its grammar **is** the release's numeral grammar and inherits
    /// every version difference in it. A **negative** value names no frame, so
    /// only `-0` survives its sign. The `#` must be the word's first byte — a
    /// space before it is a `bad level`, though a space *after* it is fine.
    ///
    /// This grammar is *not* version-invariant, contrary to what this module
    /// claimed before: 8.6 and 9.0 select **different frames** for the same
    /// word, and for some spellings one errors where the other succeeds.
    /// Pinned on tclsh 8.6.16 and 9.0.4 (`level_value_matrix_diverges_by_release`):
    ///
    /// | word  | 8.6           | 9.0         | why |
    /// |-------|---------------|-------------|-----|
    /// | `010` | 8 frames up   | 10 frames up | leading-zero octal, retired in 9.0 |
    /// | `08`  | `bad level`   | 8 frames up  | invalid octal digit vs plain decimal |
    /// | `0d1` | `bad level`   | caller       | `0d` prefix is 9.0+ |
    /// | `1_0` | `bad level`   | 10 frames up | `_` separators are 9.0+ |
    /// | `007` | 7 frames up   | 7 frames up  | octal 7 and decimal 7 coincide |
    /// | `0x1`, `0o1` | caller | caller       | prefix in both |
    ///
    /// The earlier "version-invariant" claim came from a matrix that contained
    /// no divergent spelling, measured against a call chain too shallow to tell
    /// a parse failure from an out-of-range level — both report `bad level`.
    ///
    /// With `version` `None`, a word every release reads the same way resolves
    /// normally and a word they disagree about yields [`Self::Dynamic`]: the
    /// frame is real but unknown, which is the abstaining answer every consumer
    /// already handles. Returning `None` there would instead assert "not a
    /// level", which is false on at least one release.
    #[must_use]
    pub fn parse_for(word: &str, version: Option<TclVersion>) -> Option<Self> {
        if word.contains('$') || word.contains('[') {
            return Some(Self::Dynamic);
        }
        let Some(numbers) = version.map(TclVersion::number_syntax) else {
            return Self::parse_across_releases(word);
        };
        Self::parse_under(word, numbers)
    }

    /// Parse a selector under explicitly resolved execution policies.
    #[must_use]
    pub fn parse_for_dialect(
        word: &str,
        dialect: crate::invocation_words::InvocationDialect,
    ) -> Option<Self> {
        if word.contains('$') || word.contains('[') {
            Some(Self::Dynamic)
        } else {
            Self::parse_under(word, dialect.numbers)
        }
    }

    /// Parse a materialized native level spelling without treating `$` or `[` as
    /// source substitutions. Non-Unicode input cannot name a numeric frame.
    /// Embedded raw NUL requires a cache-aware primitive `GetInt` recipe and is
    /// explicitly unresolved by this byte grammar projection.
    #[must_use]
    pub fn parse_native_bytes(
        word: &[u8],
        dialect: crate::InvocationDialect,
    ) -> Option<Option<Self>> {
        if word.contains(&0) {
            return None;
        }
        Some(
            core::str::from_utf8(word)
                .ok()
                .and_then(|word| Self::parse_under(word, dialect.numbers)),
        )
    }

    /// [`Self::parse_for`] with the release taken from `registry`'s loaded
    /// dialect profile — the ordinary way a consumer inside the compiler names
    /// the release it is analysing for.
    ///
    /// A registry with no profile loaded falls back to [`Self::parse`]'s
    /// abstaining behaviour, which is the same answer that consumer would have
    /// got before it had a release to name.
    #[must_use]
    pub fn parse_in(word: &str, registry: &crate::registry::CommandRegistry) -> Option<Self> {
        registry.profile().map_or_else(
            || Self::parse_for(word, registry.runtime_version()),
            |profile| {
                Self::parse_for_dialect(
                    word,
                    crate::invocation_words::InvocationDialect::of_profile(profile),
                )
            },
        )
    }

    /// [`Self::parse_for`] with the numeral grammar already chosen.
    fn parse_under(word: &str, numbers: NumberSyntax) -> Option<Self> {
        match word.strip_prefix('#') {
            Some(rest) => parse_level_value(rest, numbers).map(Self::Absolute),
            None => parse_level_value(word, numbers).map(Self::Relative),
        }
    }

    /// The answer when no release is named: unanimous across every grammar, or
    /// [`Self::Dynamic`] when they differ.
    ///
    /// A `None` counts as an answer in its own right rather than as "no
    /// opinion" — `08` is a level on 9.0 and not one on 8.6, which is a
    /// disagreement to abstain on, not a shared rejection to pass through.
    fn parse_across_releases(word: &str) -> Option<Self> {
        NumberSyntax::unanimous(|numbers| Self::parse_under(word, numbers))
            .unwrap_or(Some(Self::Dynamic))
    }

    /// Whether *word* is taken as the level word by a command that probes its
    /// leading argument for one ([`FrameLevelWord::LeadingProbe`]) under
    /// `version`.
    ///
    /// With no version in hand this answers the **intersection** — only a word
    /// *every* release consumes.  That is the abstaining direction for this
    /// question: a word the releases disagree about makes the script
    /// unrunnable on at least one of them, so a consumer must not go on to
    /// claim the *next* word is a readable body running in the caller's frame.
    /// It also keeps the answer monotone with the level-value grammar — a word
    /// only one release consumes is a `bad level` there anyway.
    ///
    /// Distinct from [`Self::parse`]: this asks whether the word is *consumed*
    /// as the level, not whether its value names a real frame.  `#x` is
    /// consumed and then rejected (`bad level "#x"`) on both interpreters;
    /// `x` is not consumed at all and becomes the script (`invalid command
    /// name "x"`).  See [`FrameLevelWord::LeadingProbe`] for the two classes
    /// where the releases disagree.
    #[must_use]
    pub fn word_could_be_level(word: &str, version: Option<TclVersion>) -> bool {
        if word.contains('$') || word.contains('[') {
            // A substituted word's text says nothing; whether it separates
            // from the script is the caller's arity question.
            return false;
        }
        if word.starts_with('#') {
            return true;
        }
        match version {
            Some(version) => word_has_level_presence(
                word,
                version.uplevel_level_presence(),
                version.number_syntax(),
            ),
            None => TclVersion::ALL.into_iter().all(|version| {
                word_has_level_presence(
                    word,
                    version.uplevel_level_presence(),
                    version.number_syntax(),
                )
            }),
        }
    }
}

fn word_has_level_presence(
    word: &str,
    presence: FrameLevelPresence,
    numbers: NumberSyntax,
) -> bool {
    if word.starts_with('#') {
        return true;
    }
    let leading_digit = word.as_bytes().first().is_some_and(u8::is_ascii_digit);
    match presence {
        FrameLevelPresence::DigitOrHash => leading_digit,
        FrameLevelPresence::DigitOrNonNegativeInteger => {
            leading_digit
                || match parse_integer_word(word, numbers) {
                    Some(tcl_syntax::number::Number::Int(value)) => {
                        value >= 0 || i32::try_from(value).is_err()
                    }
                    Some(tcl_syntax::number::Number::Big { .. }) => true,
                    _ => false,
                }
        }
        FrameLevelPresence::IntegerOrHash => matches!(
            parse_integer_word(word, numbers),
            Some(tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. })
        ),
        FrameLevelPresence::ArgumentParity => false,
    }
}

fn parse_integer_word(text: &str, numbers: NumberSyntax) -> Option<tcl_syntax::number::Number> {
    tcl_syntax::number::parse_whole_with(
        text,
        ParseFlags {
            integer_only: true,
            ..ParseFlags::for_syntax(numbers)
        },
    )
}

/// The magnitude of a `Tcl_GetInt`-shaped level word, with its sign reported
/// separately — the shared core of [`FrameLevel::parse`] and
/// [`FrameLevel::word_could_be_level`].
fn parse_level_magnitude(text: &str, numbers: NumberSyntax) -> Option<(bool, u32)> {
    // A level word is read by `Tcl_GetIntFromObj`, so its grammar *is* the
    // release's numeral grammar — the level word inherits every version
    // difference. Pinned on tclsh 8.6.16 and 9.0.4 with a 16-frame stack (deep
    // enough that an out-of-range level cannot be mistaken for a parse
    // failure — both report `bad level`):
    //
    // | word  | 8.6            | 9.0            |
    // |-------|----------------|----------------|
    // | `1`   | 1              | 1              |
    // | `007` | 7 (octal)      | 7 (decimal)    |
    // | `010` | **8** (octal)  | **10**         |
    // | `08`  | bad level      | **8**          |
    // | `1_0` | bad level      | **10**         |
    // | `0d1` | bad level      | **1**          |
    // | `0x1` | 1              | 1              |
    // | `0o1` | 1              | 1              |
    //
    // Going through the shared facility gets all eight rows for free; the
    // hand-rolled prefix scan this replaced got four of them wrong.
    //
    // `no_whitespace` is deliberately *not* set: `Tcl_GetIntFromObj` tolerates
    // surrounding space (`"  1  "` is level 1). It does not tolerate space
    // *between* the sign and the digits (`upvar "+ 1" …` → `bad level "+ 1"`),
    // which the facility already rejects — it reads a sign only immediately
    // before the digits.
    let flags = ParseFlags {
        integer_only: true,
        ..ParseFlags::for_syntax(numbers)
    };
    // A magnitude past a wide names no frame on any release, and a float or NaN
    // is not an integer word at all.
    let tcl_syntax::number::Number::Int(value) = tcl_syntax::number::parse_whole_with(text, flags)?
    else {
        return None;
    };
    // Report the sign separately: the *presence* question distinguishes `-1`
    // (consumed then rejected by 9.0) from a word 8.6 dispatches as a command.
    let negative = value < 0;
    Some((negative, u32::try_from(value.unsigned_abs()).ok()?))
}

/// A level word's frame number, or `None` when it names no frame.  A negative
/// value is `bad level` on every release, so `-0` is the only signed spelling
/// that survives.
fn parse_level_value(text: &str, numbers: NumberSyntax) -> Option<u32> {
    match parse_level_magnitude(text, numbers)? {
        (true, value) if value != 0 => None,
        (_, value) => Some(value),
    }
}

/// How a command spells its optional frame-level word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameLevelWord {
    /// No level word at all.
    None,
    /// **Argument-count parity**, `upvar`'s rule: the level word is present
    /// exactly when the number of words after the command name is *odd*.
    ///
    /// C Tcl decides this on `objc`, never on the word's text
    /// (`Tcl_UpvarObjCmd`), which is why the two spellings below mean
    /// opposite things — pinned on tclsh 9.0.4 and 8.6.14, identical:
    ///
    /// | written | words | level | pairs |
    /// |---|---|---|---|
    /// | `upvar 1 a b`     | 3 | `1`      | `(a, b)` |
    /// | `upvar a b`       | 2 | default  | `(a, b)` |
    /// | `upvar 1 b`       | 2 | default  | `(1, b)` — `1` is a *variable name* |
    /// | `upvar $lvl a b`  | 3 | `$lvl`   | `(a, b)` |
    /// | `upvar 1 a b c`   | 4 | default  | `(1, a)`, `(b, c)` |
    /// | `upvar foo bar baz` | 3 | `foo` → `bad level "foo"` | — |
    ///
    /// A text-sniffing consumer gets the third and fourth rows backwards:
    /// it drops a real binding for `upvar $lvl a b` (the commonest
    /// by-reference idiom of all) and invents a level for `upvar 1 b`.
    ArityParity,
    /// Dialect-aware `upvar`: leading-word probe before Tcl 8.6, count parity
    /// in Tcl 8.6+ and Jim. Unknown dialects require agreement.
    Upvar,
    /// **Leading-word probe**, `uplevel`'s rule: the first word is the level
    /// when it parses as one, or when it substitutes *and* a further word
    /// follows (`uplevel $lvl {…}`; a lone `uplevel $body` is a body).
    ///
    /// Unlike the level *value* grammar ([`FrameLevel::parse`]), this
    /// presence rule is the one place `upvar`/`uplevel` semantics genuinely
    /// diverge between releases — `TclObjGetFrame` was reworked in 9.0.  Two
    /// classes differ; both are hard errors under both interpreters, so no
    /// *working* script is shaped differently, but the registry records the
    /// fact rather than asserting one release's answer for all.
    /// `uplevel W {oops}`, tclsh 9.0.4 vs 8.6.14:
    ///
    /// | `W` | 9.0.4 | 8.6.14 | consumed as level |
    /// |---|---|---|---|
    /// | `-1`, `-2` | `bad level "-1"` | `invalid command name "-1"` | 9.0 only |
    /// | `1.0`, `1e0` | `invalid command name "1.0"` | `bad level "1.0"` | 8.6 only |
    /// | `1`, `0`, `2`, `007`, `+1`, `+0`, `-0`, `" 1"`, `0x1`, `0b1`, `0o1` | `invalid command name "oops"` | same | both |
    /// | `#0`, `#1`, `#-0` | `invalid command name "oops"` | same | both |
    /// | `#x` | `bad level "#x"` | same | both |
    /// | `x` | `invalid command name "x"` | same | neither |
    ///
    /// [`FrameLevel::word_could_be_level`] is that table; with no version in
    /// hand it answers the *shared* rows only.
    LeadingProbe,
}

/// What the arguments after the (optional) level word do, and which frame
/// the effect lands in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameArgLayout {
    /// `otherVar myVar` pairs — each `otherVar` names a variable in the
    /// **selected** frame and each `myVar` the local alias bound to it
    /// (`upvar`).
    AliasPairs,
    /// The remaining words concatenate into a script evaluated in the
    /// **selected** frame (`uplevel`), so its variable accesses belong to
    /// that frame, not to the one the call is written in.
    ScriptInSelectedFrame,
    /// The command's [`crate::ArgRole::Body`] argument runs in the
    /// **current** frame, sharing its variables (`eval`).
    ScriptInCurrentFrame,
    /// The command injects variables into the frame of **its own caller**
    /// under names it derives from an argument mini-language this analysis
    /// does not interpret (`argparse`'s definition list).  Every such name
    /// is unknowable, so a consumer must widen rather than enumerate.
    OpaqueCallerVars,
}

/// A frame invocation's grammar outcome, before executing its bodies or links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameArgumentResolution {
    /// The argument layout and selected frame are known independently.
    Valid {
        /// Width of the optional leading level word.
        level_word_len: usize,
        /// Selected frame; a computed level retains `Dynamic`.
        level: FrameLevel,
    },
    /// The literal invocation cannot satisfy the command grammar.
    Invalid,
    /// Expansion, dynamic presence or an unspecified dialect changes the layout.
    Unknown,
}

/// Argument layout on a successful command completion, kept separate from
/// the ordinary pre-dispatch grammar result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSuccessProjection {
    /// Layout on the normal path. `Invalid` means no normal path exists;
    /// `Unknown` retains unresolved expansion or distinct successful layouts.
    pub layout: FrameArgumentResolution,
    /// Whether an operand can still produce a grammar error before any links
    /// or script execution. This does not describe later execution errors.
    pub may_argument_error: bool,
}

/// How a command crosses stack frames — the registry's answer to "which
/// argument is the level word, which names a variable in another frame, and
/// which carries a script that runs there".
///
/// Attached to a [`CommandSpec`](crate::CommandSpec) via
/// `CommandSpec::frame_effect`; absent means the command has no
/// frame-crossing argument grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameEffectSpec {
    /// How the optional level word is located.
    pub level_word: FrameLevelWord,
    /// What the post-level arguments do.
    pub layout: FrameArgLayout,
}

impl FrameEffectSpec {
    /// The native `upvar` grammar, shared with runtime adapters.
    pub const UPVAR: Self = Self {
        level_word: FrameLevelWord::Upvar,
        layout: FrameArgLayout::AliasPairs,
    };
    /// The native `uplevel` grammar, shared with runtime adapters.
    pub const UPLEVEL: Self = Self {
        level_word: FrameLevelWord::LeadingProbe,
        layout: FrameArgLayout::ScriptInSelectedFrame,
    };

    /// Determine the level-word width from a structured argument count when
    /// no source spelling is needed.
    ///
    /// [`FrameLevelWord::ArityParity`] is entirely positional, so registry
    /// descriptors that receive [`crate::InvocationArguments`] can share the
    /// exact `upvar` rule without reconstructing literal arguments. A
    /// leading-probe layout needs a literal first word and therefore returns
    /// `None` for the caller to widen conservatively.
    #[must_use]
    pub const fn level_word_len_for_argument_count(self, argument_count: usize) -> Option<usize> {
        match self.level_word {
            FrameLevelWord::None => Some(0),
            FrameLevelWord::ArityParity => {
                if argument_count % 2 == 1 {
                    Some(1)
                } else {
                    Some(0)
                }
            }
            FrameLevelWord::LeadingProbe | FrameLevelWord::Upvar => None,
        }
    }

    /// Resolve complete frame grammar before assigning roles or transitions.
    #[must_use]
    pub fn resolve_arguments(
        self,
        arguments: crate::InvocationArguments<'_>,
    ) -> FrameArgumentResolution {
        let Some(count) = arguments.exact_argv_len() else {
            return FrameArgumentResolution::Unknown;
        };
        if (self.layout == FrameArgLayout::AliasPairs && count < 2)
            || (self.layout == FrameArgLayout::ScriptInSelectedFrame && count == 0)
        {
            return FrameArgumentResolution::Invalid;
        }
        let Some(width) = self.level_word_len_for_arguments(arguments) else {
            return FrameArgumentResolution::Unknown;
        };
        let remaining = count.saturating_sub(width);
        let valid = match self.layout {
            FrameArgLayout::AliasPairs => remaining >= 2 && remaining % 2 == 0,
            FrameArgLayout::ScriptInSelectedFrame => remaining >= 1,
            FrameArgLayout::ScriptInCurrentFrame | FrameArgLayout::OpaqueCallerVars => true,
        };
        if !valid {
            return FrameArgumentResolution::Invalid;
        }
        let level = if width == 0 {
            FrameLevel::DEFAULT
        } else if let Some(word) = arguments.literal_at(0) {
            let parsed = arguments.dialect().map_or_else(
                || FrameLevel::parse_across_releases(word),
                |dialect| FrameLevel::parse_under(word, dialect.numbers),
            );
            let Some(level) = parsed else {
                return FrameArgumentResolution::Invalid;
            };
            level
        } else {
            FrameLevel::Dynamic
        };
        FrameArgumentResolution::Valid {
            level_word_len: width,
            level,
        }
    }

    /// Consensus of the existing authored Tcl frame grammars for a separate
    /// Logical source-role question. The same values and cardinality remain;
    /// this does not select a Native frame grammar or establish a frame effect.
    pub(crate) fn logical_source_layout(
        self,
        arguments: crate::InvocationArguments<'_>,
    ) -> FrameArgumentResolution {
        let source = match arguments {
            crate::InvocationArguments::Literals(words)
            | crate::InvocationArguments::ContextualLiterals(words, _) => {
                crate::InvocationArguments::Literals(words)
            }
            crate::InvocationArguments::Structured(words)
            | crate::InvocationArguments::ContextualStructured(words, _) => {
                crate::InvocationArguments::Structured(words)
            }
        };
        self.resolve_arguments(source)
    }

    /// Project argument layout after a successful native invocation.
    ///
    /// For alias pairs, count parity excludes one dynamic presence branch:
    /// consuming a level from an even argument count, or omitting it from
    /// an odd count, leaves an invalid pair list. That branch may error, but
    /// it cannot supply a normal state. This never turns pre-dispatch
    /// `Unknown` into unconditional validity.
    #[must_use]
    pub fn successful_layout(
        self,
        arguments: crate::InvocationArguments<'_>,
    ) -> FrameSuccessProjection {
        let ordinary = self.resolve_arguments(arguments);
        let may_argument_error = match ordinary {
            FrameArgumentResolution::Valid { level, .. } => level == FrameLevel::Dynamic,
            FrameArgumentResolution::Invalid | FrameArgumentResolution::Unknown => true,
        };
        let fallback = FrameSuccessProjection {
            layout: ordinary,
            may_argument_error,
        };
        // A single computed script operand can complete normally only when
        // it is not consumed as the optional selector: a consumed selector
        // would leave no script. Keep that rejected branch as may-error.
        if ordinary == FrameArgumentResolution::Unknown
            && self.layout == FrameArgLayout::ScriptInSelectedFrame
            && self.level_word == FrameLevelWord::LeadingProbe
            && arguments.exact_argv_len() == Some(1)
            && arguments.literal_at(0).is_none()
            && arguments
                .dialect()
                .is_some_and(|dialect| dialect.uplevel_level_presence.is_some())
        {
            return FrameSuccessProjection {
                layout: FrameArgumentResolution::Valid {
                    level: FrameLevel::DEFAULT,
                    level_word_len: 0,
                },
                may_argument_error: true,
            };
        }
        if ordinary != FrameArgumentResolution::Unknown
            || self.layout != FrameArgLayout::AliasPairs
            || self.level_word != FrameLevelWord::Upvar
        {
            return fallback;
        }
        let Some(count) = arguments.exact_argv_len() else {
            return fallback;
        };
        let Some(dialect) = arguments.dialect() else {
            return fallback;
        };
        if dialect.upvar_level_presence.is_none() || arguments.literal_at(0).is_some() {
            return fallback;
        }
        if count < 2 {
            return FrameSuccessProjection {
                layout: FrameArgumentResolution::Invalid,
                may_argument_error: true,
            };
        }
        let width = usize::from(count % 2 == 1);
        FrameSuccessProjection {
            layout: FrameArgumentResolution::Valid {
                level_word_len: width,
                level: if width == 0 {
                    FrameLevel::DEFAULT
                } else {
                    FrameLevel::Dynamic
                },
            },
            may_argument_error: true,
        }
    }

    /// Resolve level-word width from structured values and dialect policies.
    /// A dynamic head or expansion abstains when it can change the layout.
    #[must_use]
    pub fn level_word_len_for_arguments(
        self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<usize> {
        let count = arguments.exact_argv_len()?;
        if let Some(width) = self.level_word_len_for_argument_count(count) {
            return Some(width);
        }
        let decide = |presence, numbers| {
            if presence == FrameLevelPresence::ArgumentParity {
                return Some(usize::from(count % 2 == 1));
            }
            let word = arguments.literal_at(0)?;
            Some(usize::from(word_has_level_presence(
                word, presence, numbers,
            )))
        };
        if let Some(dialect) = arguments.dialect() {
            let presence = match self.level_word {
                FrameLevelWord::Upvar => dialect.upvar_level_presence?,
                FrameLevelWord::LeadingProbe => dialect.uplevel_level_presence?,
                FrameLevelWord::None | FrameLevelWord::ArityParity => {
                    unreachable!("count-only layout returned above")
                }
            };
            return decide(presence, dialect.numbers);
        }
        let mut answers = TclVersion::ALL.into_iter().map(|version| {
            let presence = if self.level_word == FrameLevelWord::Upvar {
                version.upvar_level_presence()
            } else {
                version.uplevel_level_presence()
            };
            decide(presence, version.number_syntax())
        });
        let first = answers.next()??;
        answers.all(|answer| answer == Some(first)).then_some(first)
    }

    /// Select frame operand width from evaluated argc and only the original
    /// first operand. Remaining name operands are never decoded for this query.
    /// An embedded NUL in a value-dependent probe leaves the layout unresolved.
    #[must_use]
    pub fn level_word_len_for_native_bytes(
        self,
        count: usize,
        first: Option<&[u8]>,
        dialect: crate::InvocationDialect,
    ) -> Option<usize> {
        if let Some(width) = self.level_word_len_for_argument_count(count) {
            return Some(width);
        }
        let presence = match self.level_word {
            FrameLevelWord::Upvar => dialect.upvar_level_presence?,
            FrameLevelWord::LeadingProbe => dialect.uplevel_level_presence?,
            FrameLevelWord::None | FrameLevelWord::ArityParity => {
                unreachable!("count-only frame layout")
            }
        };
        if presence == FrameLevelPresence::ArgumentParity {
            return Some(usize::from(count % 2 == 1));
        }
        let first = first?;
        if first.contains(&0) {
            return None;
        }
        if first.first() == Some(&b'#') {
            return Some(1);
        }
        let selected = core::str::from_utf8(first)
            .is_ok_and(|word| word_has_level_presence(word, presence, dialect.numbers));
        Some(usize::from(selected))
    }

    /// How many leading words of `args` (the argument list *after* the
    /// command name) the level word occupies — `1` or `0`.
    ///
    /// `args` must be the whole post-command word list, because
    /// [`FrameLevelWord::ArityParity`] answers from its length.
    ///
    /// Version-invariant: where the releases disagree
    /// ([`FrameLevelWord::LeadingProbe`]) this abstains — only a word every
    /// release consumes takes the level slot, so the following word is never
    /// claimed as a readable caller-frame body on the strength of a spelling
    /// one release rejects.  Callers that know the document's dialect should
    /// ask [`Self::level_word_len_for_version`] instead.
    #[must_use]
    pub fn level_word_len(&self, args: &[&str]) -> usize {
        self.level_word_len_for_version(args, None)
    }

    /// [`Self::level_word_len`] resolved against a specific release.
    ///
    /// `None` widens to the union of every release (see
    /// [`FrameLevel::word_could_be_level`]).
    #[must_use]
    pub fn level_word_len_for_version(&self, args: &[&str], version: Option<TclVersion>) -> usize {
        match self.level_word {
            FrameLevelWord::None | FrameLevelWord::ArityParity => self
                .level_word_len_for_argument_count(args.len())
                .expect("only leading probes need a literal argument"),
            FrameLevelWord::Upvar => {
                let arguments = crate::InvocationArguments::literals(args);
                let contextual = version.map_or(arguments, |version| {
                    let dialect = crate::invocation_words::InvocationDialect::for_version(version);
                    arguments.with_dialect(dialect)
                });
                self.level_word_len_for_arguments(contextual).unwrap_or(0)
            }
            FrameLevelWord::LeadingProbe => match args.first() {
                // A word the release consumes as a level is one, whatever
                // follows: `uplevel 1 2 3` runs the script `2 3` at level 1
                // (tclsh 9.0.4 / 8.6.14 both report `invalid command name
                // "2"`).
                Some(w) if FrameLevel::word_could_be_level(w, version) => 1,
                // A substituted word only separates from the script when a
                // script word follows it.
                Some(w) if args.len() >= 2 && FrameLevel::parse(w) == Some(FrameLevel::Dynamic) => {
                    1
                }
                _ => 0,
            },
        }
    }

    /// The frame this invocation targets, and the arguments that follow the
    /// level word.
    #[must_use]
    pub fn resolve<'a>(&self, args: &'a [&'a str]) -> (FrameLevel, &'a [&'a str]) {
        self.resolve_for_version(args, None)
    }

    /// [`Self::resolve`] against a specific release.
    #[must_use]
    pub fn resolve_for_version<'a>(
        &self,
        args: &'a [&'a str],
        version: Option<TclVersion>,
    ) -> (FrameLevel, &'a [&'a str]) {
        let taken = self.level_word_len_for_version(args, version);
        let level = if taken == 0 {
            FrameLevel::DEFAULT
        } else {
            FrameLevel::parse_for(args[0], version).unwrap_or(FrameLevel::Dynamic)
        };
        (level, &args[taken..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UPVAR: FrameEffectSpec = FrameEffectSpec {
        level_word: FrameLevelWord::ArityParity,
        layout: FrameArgLayout::AliasPairs,
    };
    const UPLEVEL: FrameEffectSpec = FrameEffectSpec {
        level_word: FrameLevelWord::LeadingProbe,
        layout: FrameArgLayout::ScriptInSelectedFrame,
    };

    #[test]
    fn impossible_frame_arity_is_invalid_before_selector_presence() {
        let jim = crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        for dialect in std::iter::once(None)
            .chain(
                TclVersion::ALL
                    .into_iter()
                    .map(|version| Some(crate::InvocationDialect::for_version(version))),
            )
            .chain(std::iter::once(Some(jim)))
        {
            for words in [&[][..], &[crate::InvocationWord::Dynamic][..]] {
                let mut arguments = crate::InvocationArguments::Structured(words);
                if let Some(dialect) = dialect {
                    arguments = arguments.with_dialect(dialect);
                }
                assert_eq!(
                    FrameEffectSpec::UPVAR.resolve_arguments(arguments),
                    FrameArgumentResolution::Invalid
                );
            }
            let mut empty = crate::InvocationArguments::Literals(&[]);
            if let Some(dialect) = dialect {
                empty = empty.with_dialect(dialect);
            }
            assert_eq!(
                FrameEffectSpec::UPLEVEL.resolve_arguments(empty),
                FrameArgumentResolution::Invalid
            );
        }
    }

    /// The level-**value** grammar, pinned against both interpreters.
    ///
    /// Transcript (identical on tclsh 9.0.4 and 8.6.14) — a nine-deep chain
    /// `f1 → … → f9` where `f9` runs `upvar $W src dst; set dst MARK` and
    /// each frame reports whether *its* `src` was written:
    ///
    /// ```text
    /// upvar 1      -> f8      upvar #0     -> global    upvar -1   -> bad level "-1"
    /// upvar 0      -> f9      upvar #1     -> f1        upvar -2   -> bad level "-2"
    /// upvar 2      -> f7      upvar #-0    -> global    upvar 1.0  -> bad level "1.0"
    /// upvar 7      -> f2      upvar "# 0"  -> global    upvar 1e0  -> bad level "1e0"
    /// upvar 007    -> f2      upvar "#0 "  -> global    upvar 1_0  -> bad level "1_0"
    /// upvar +1     -> f8      upvar #+1    -> f1        upvar "+ 1"-> bad level "+ 1"
    /// upvar +0     -> f9      upvar #0x1   -> f1        upvar --0  -> bad level "--0"
    /// upvar -0     -> f9      upvar #-1    -> bad level upvar x    -> bad level "x"
    /// upvar " 1"   -> f8      upvar #x     -> bad level upvar ""   -> bad level ""
    /// upvar "  1  "-> f8      upvar " #0"  -> bad level " #0"
    /// upvar 0x1 / 0X1 / 0b1 / 0o1 / "0x1 " -> f8
    /// ```
    #[test]
    fn computed_single_script_has_only_an_omitted_selector_normal_layout() {
        use crate::{InvocationArguments, InvocationDialect, InvocationWord};
        let mut dialects: Vec<_> = tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(InvocationDialect::for_version)
            .collect();
        dialects.push(InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        ));
        for dialect in dialects {
            let words = [InvocationWord::Dynamic];
            let arguments = InvocationArguments::structured(&words).with_dialect(dialect);
            assert_eq!(
                super::FrameEffectSpec::UPLEVEL.resolve_arguments(arguments),
                super::FrameArgumentResolution::Unknown
            );
            assert_eq!(
                super::FrameEffectSpec::UPLEVEL.successful_layout(arguments),
                super::FrameSuccessProjection {
                    layout: super::FrameArgumentResolution::Valid {
                        level_word_len: 0,
                        level: super::FrameLevel::DEFAULT
                    },
                    may_argument_error: true,
                }
            );
            assert_eq!(
                super::FrameEffectSpec::UPLEVEL
                    .resolve_arguments(InvocationArguments::Literals(&["1"]).with_dialect(dialect)),
                super::FrameArgumentResolution::Invalid
            );
        }
    }

    #[test]
    fn level_value_matrix_matches_c_tcl() {
        use FrameLevel::{Absolute, Dynamic, Relative};
        let accepted = [
            ("1", Relative(1)),
            ("0", Relative(0)),
            ("2", Relative(2)),
            ("7", Relative(7)),
            ("007", Relative(7)),
            ("+1", Relative(1)),
            ("+0", Relative(0)),
            ("-0", Relative(0)),
            (" 1", Relative(1)),
            ("1 ", Relative(1)),
            ("  1  ", Relative(1)),
            ("0x1", Relative(1)),
            ("0X1", Relative(1)),
            ("0b1", Relative(1)),
            ("0o1", Relative(1)),
            ("0x1 ", Relative(1)),
            ("#0", Absolute(0)),
            ("#1", Absolute(1)),
            ("#9", Absolute(9)),
            ("#-0", Absolute(0)),
            ("#+1", Absolute(1)),
            ("# 0", Absolute(0)),
            ("#0 ", Absolute(0)),
            ("#0x1", Absolute(1)),
            ("$lvl", Dynamic),
            ("[expr {$n-1}]", Dynamic),
        ];
        // Asserted per release rather than through the version-less `parse`:
        // the transcript above is 8.6.14 and 9.0.4, and those two agreeing is
        // exactly the claim. (`0b1` / `0o1` are *not* levels on 8.4, which has
        // no such prefixes, so the version-less answer for them abstains — see
        // `no_version_abstains_where_releases_disagree`.)
        for version in [TclVersion::V8_6, TclVersion::V9_0] {
            for (word, level) in accepted {
                assert_eq!(
                    FrameLevel::parse_for(word, Some(version)),
                    Some(level),
                    "level word {word:?} under {version:?}"
                );
            }
            // Every one of these is `bad level "…"` on both interpreters.
            for word in [
                "-1", "-2", "#-1", "1.0", "1e0", "+ 1", "--0", "x", "#x", "", " #0", "foo", "0x",
                "0b2",
            ] {
                assert_eq!(
                    FrameLevel::parse_for(word, Some(version)),
                    None,
                    "level word {word:?} under {version:?}"
                );
            }
        }
    }

    /// The level value is read by `Tcl_GetIntFromObj`, so it inherits every
    /// version difference in the numeral grammar — the module's former
    /// "version-invariant" claim was wrong.
    ///
    /// Pinned on tclsh 8.6.16 and 9.0.4 against a **16-deep** call chain. The
    /// depth matters: C reports `bad level` both for a word it cannot parse and
    /// for a level past the top of the stack, so a chain shallow enough for a
    /// divergent value to be out of range hides the difference. That is how the
    /// original 9-deep matrix concluded invariance — at depth 9, `1_0` is level
    /// 10 on 9.0, out of range, reported as `bad level` exactly as 8.6's parse
    /// failure is.
    ///
    /// ```text
    /// # 16 frames deep, so 8 and 10 are both in range:
    /// uplevel 010 -> 8 up  on 8.6   |  10 up on 9.0   (leading-zero octal, retired in 9.0)
    /// uplevel 08  -> bad level      |  8 up           (invalid octal digit vs plain decimal)
    /// uplevel 0d1 -> bad level      |  1 up           (`0d` prefix is 9.0+)
    /// uplevel 1_0 -> bad level      |  10 up          (`_` separators are 9.0+)
    /// uplevel 007 -> 7 up           |  7 up           (octal 7 and decimal 7 coincide)
    /// ```
    #[test]
    fn level_value_matrix_diverges_by_release() {
        use FrameLevel::Relative;
        // (word, 8.6 answer, 9.0 answer)
        let matrix = [
            ("010", Some(Relative(8)), Some(Relative(10))),
            ("08", None, Some(Relative(8))),
            ("0d1", None, Some(Relative(1))),
            ("1_0", None, Some(Relative(10))),
            // The coincidence that made `007` look invariant.
            ("007", Some(Relative(7)), Some(Relative(7))),
        ];
        for (word, on_86, on_90) in matrix {
            assert_eq!(
                FrameLevel::parse_for(word, Some(TclVersion::V8_6)),
                on_86,
                "level word {word:?} on 8.6"
            );
            assert_eq!(
                FrameLevel::parse_for(word, Some(TclVersion::V9_0)),
                on_90,
                "level word {word:?} on 9.0"
            );
        }
    }

    /// With no release named, a word the releases read differently is
    /// [`FrameLevel::Dynamic`] — the frame is real but unknown. Answering
    /// `None` would assert "not a level", which is false on at least one
    /// release; answering one release's value would be wrong on the others.
    #[test]
    fn successful_alias_layout_conditions_dynamic_presence_on_pair_arity() {
        use crate::{InvocationArguments, InvocationDialect, InvocationWord};
        let even = [InvocationWord::Dynamic, InvocationWord::Literal("linked")];
        let odd = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("target"),
            InvocationWord::Literal("linked"),
        ];
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let args = InvocationArguments::Structured(&even).with_dialect(dialect);
            let projection = FrameEffectSpec::UPVAR.successful_layout(args);
            assert_eq!(
                projection.layout,
                super::FrameArgumentResolution::Valid {
                    level_word_len: 0,
                    level: FrameLevel::DEFAULT,
                },
                "{version:?}"
            );
            assert_eq!(
                projection.may_argument_error,
                matches!(version, TclVersion::V8_4 | TclVersion::V8_5)
            );
            if matches!(version, TclVersion::V8_4 | TclVersion::V8_5) {
                assert_eq!(
                    FrameEffectSpec::UPVAR.resolve_arguments(args),
                    super::FrameArgumentResolution::Unknown
                );
            }
            let args = InvocationArguments::Structured(&odd).with_dialect(dialect);
            assert_eq!(
                FrameEffectSpec::UPVAR.successful_layout(args),
                super::FrameSuccessProjection {
                    layout: super::FrameArgumentResolution::Valid {
                        level_word_len: 1,
                        level: FrameLevel::Dynamic
                    },
                    may_argument_error: true,
                }
            );
            let expanded = [InvocationWord::Expanded, InvocationWord::Literal("linked")];
            assert_eq!(
                FrameEffectSpec::UPVAR
                    .successful_layout(
                        InvocationArguments::Structured(&expanded).with_dialect(dialect)
                    )
                    .layout,
                super::FrameArgumentResolution::Unknown
            );
            assert_eq!(
                FrameEffectSpec::UPVAR
                    .successful_layout(
                        InvocationArguments::Literals(&["1", "target", "linked", "dangling"])
                            .with_dialect(dialect)
                    )
                    .layout,
                if matches!(version, TclVersion::V8_4 | TclVersion::V8_5) {
                    super::FrameArgumentResolution::Invalid
                } else {
                    super::FrameArgumentResolution::Valid {
                        level_word_len: 0,
                        level: FrameLevel::DEFAULT,
                    }
                }
            );
        }
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert_eq!(
            FrameEffectSpec::UPVAR
                .successful_layout(InvocationArguments::Structured(&even).with_dialect(jim)),
            super::FrameSuccessProjection {
                layout: super::FrameArgumentResolution::Valid {
                    level_word_len: 0,
                    level: FrameLevel::DEFAULT
                },
                may_argument_error: false,
            }
        );
        assert_eq!(
            FrameEffectSpec::UPVAR
                .successful_layout(InvocationArguments::Structured(&even))
                .layout,
            super::FrameArgumentResolution::Unknown
        );
    }

    #[test]
    fn no_version_abstains_where_releases_disagree() {
        use FrameLevel::{Dynamic, Relative};
        // Read differently somewhere in 8.4 … 9.0.
        for word in [
            "010", // octal 8 up to 8.6, decimal 10 from 9.0
            "08",  // invalid octal before 9.0
            "0d1", // `0d` is 9.0+
            "1_0", // `_` is 9.0+
            "0b1", // `0b`/`0o` are 8.5+, so 8.4 disagrees
            "0o1",
        ] {
            assert_eq!(
                FrameLevel::parse(word),
                Some(Dynamic),
                "level word {word:?} should abstain"
            );
        }
        // Unanimous spellings still resolve exactly.
        for (word, level) in [
            ("1", Relative(1)),
            ("007", Relative(7)),
            ("0x1", Relative(1)),
        ] {
            assert_eq!(FrameLevel::parse(word), Some(level), "level word {word:?}");
        }
        // Unanimously not a level at all.
        for word in ["x", "1.0", "-1", ""] {
            assert_eq!(FrameLevel::parse(word), None, "level word {word:?}");
        }
    }

    /// The level-word **presence** rule of a [`FrameLevelWord::LeadingProbe`]
    /// command, per release.
    ///
    /// Transcript — `uplevel W {oops}` inside a proc, tclsh 9.0.4 | 8.6.14:
    ///
    /// ```text
    /// W = -1     bad level "-1"            | invalid command name "-1"
    /// W = -2     bad level "-2"            | invalid command name "-2"
    /// W = 1.0    invalid command name "1.0"| bad level "1.0"
    /// W = 1e0    invalid command name "1e0"| bad level "1e0"
    /// W = 007    bad level "007"           | bad level "007"
    /// W = #x     bad level "#x"            | bad level "#x"
    /// W = x      invalid command name "x"  | invalid command name "x"
    /// W ∈ {1,0,2,#0,#1,+1,+0,-0," 1","1 ",0x1,0X1,0b1,0o1,#-0}
    ///            invalid command name "oops" (level consumed) | same
    /// ```
    #[test]
    fn uplevel_level_word_presence_matrix_matches_c_tcl() {
        for release in TclVersion::ALL {
            for word in ["1", "0", "#0", "#x", "0x1", "1.0", "1e0"] {
                let expected = !matches!(release, TclVersion::V9_0 | TclVersion::V9_1)
                    || !matches!(word, "1.0" | "1e0");
                assert_eq!(
                    FrameLevel::word_could_be_level(word, Some(release)),
                    expected,
                    "{release:?} {word}"
                );
            }
            for word in ["+1", "-0", " 1"] {
                assert_eq!(
                    FrameLevel::word_could_be_level(word, Some(release)),
                    !matches!(release, TclVersion::V8_4 | TclVersion::V8_5),
                    "{release:?} {word}"
                );
            }
            assert_eq!(
                FrameLevel::word_could_be_level("-1", Some(release)),
                release >= TclVersion::V9_0
            );
        }
        assert!(!FrameLevel::word_could_be_level("+1", None));
        assert!(!FrameLevel::word_could_be_level("$level", None));
    }

    #[test]
    fn structured_upvar_resolution_obeys_legacy_presence_and_modern_parity() {
        use crate::{InvocationArguments, InvocationDialect, InvocationWord};
        for release in TclVersion::ALL {
            let context = InvocationDialect::for_version(release);
            let numeric_name = InvocationArguments::Literals(&["1", "local"]).with_dialect(context);
            let modern = release >= TclVersion::V8_6;
            assert_eq!(
                matches!(
                    FrameEffectSpec::UPVAR.resolve_arguments(numeric_name),
                    super::FrameArgumentResolution::Valid { .. }
                ),
                modern
            );
            let signed_name = InvocationArguments::Literals(&["+1", "local"]).with_dialect(context);
            assert!(matches!(
                FrameEffectSpec::UPVAR.resolve_arguments(signed_name),
                super::FrameArgumentResolution::Valid {
                    level_word_len: 0,
                    ..
                }
            ));
            let dynamic = [
                InvocationWord::Dynamic,
                InvocationWord::Literal("source"),
                InvocationWord::Literal("local"),
            ];
            let result = FrameEffectSpec::UPVAR
                .resolve_arguments(InvocationArguments::Structured(&dynamic).with_dialect(context));
            assert_eq!(
                matches!(
                    result,
                    super::FrameArgumentResolution::Valid {
                        level: FrameLevel::Dynamic,
                        level_word_len: 1
                    }
                ),
                modern
            );
        }
        assert_eq!(
            FrameEffectSpec::UPVAR
                .resolve_arguments(InvocationArguments::Literals(&["1", "local"])),
            super::FrameArgumentResolution::Unknown
        );
        let jim = tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        assert!(matches!(
            FrameEffectSpec::UPVAR.resolve_arguments(
                InvocationArguments::Literals(&["1", "local"]).with_profile(Some(&jim))
            ),
            super::FrameArgumentResolution::Valid {
                level_word_len: 0,
                ..
            }
        ));
        assert!(matches!(
            FrameEffectSpec::UPLEVEL.resolve_arguments(
                InvocationArguments::Literals(&["+1", "body"]).with_profile(Some(&jim))
            ),
            super::FrameArgumentResolution::Valid {
                level_word_len: 0,
                ..
            }
        ));
    }

    #[test]
    fn uplevel_script_position_follows_the_release() {
        // `uplevel -1 {oops}`: 9.0 consumes the level, 8.6 makes it the
        // script's first word.
        assert_eq!(
            UPLEVEL.level_word_len_for_version(&["-1", "{oops}"], Some(TclVersion::V9_0)),
            1
        );
        assert_eq!(
            UPLEVEL.level_word_len_for_version(&["-1", "{oops}"], Some(TclVersion::V8_6)),
            0
        );
        // `uplevel 1.0 {oops}`: the other way round.
        assert_eq!(
            UPLEVEL.level_word_len_for_version(&["1.0", "{oops}"], Some(TclVersion::V9_0)),
            0
        );
        assert_eq!(
            UPLEVEL.level_word_len_for_version(&["1.0", "{oops}"], Some(TclVersion::V8_6)),
            1
        );
        // `upvar` is parity, so no release-dependence at all.
        for v in [TclVersion::V8_6, TclVersion::V9_0] {
            assert_eq!(
                UPVAR.level_word_len_for_version(&["-1", "a", "b"], Some(v)),
                1
            );
            assert_eq!(UPVAR.level_word_len_for_version(&["a", "b"], Some(v)), 0);
        }
    }

    #[test]
    fn upvar_level_word_is_decided_by_arity_parity() {
        // The oracle table in `FrameLevelWord::ArityParity`'s doc.
        assert_eq!(UPVAR.resolve(&["1", "a", "b"]).0, FrameLevel::Relative(1));
        assert_eq!(UPVAR.resolve(&["1", "a", "b"]).1, ["a", "b"]);
        // Two words: no level, `1` is the caller-side *name*.
        assert_eq!(UPVAR.resolve(&["1", "b"]).0, FrameLevel::DEFAULT);
        assert_eq!(UPVAR.resolve(&["1", "b"]).1, ["1", "b"]);
        // A computed level is still a level — parity, not text, decides.
        assert_eq!(UPVAR.resolve(&["$lvl", "a", "b"]).0, FrameLevel::Dynamic);
        assert_eq!(UPVAR.resolve(&["$lvl", "a", "b"]).1, ["a", "b"]);
        // Four words: two pairs, no level.
        assert_eq!(UPVAR.resolve(&["1", "a", "b", "c"]).1, ["1", "a", "b", "c"]);
        // Three non-level words: the first IS taken as a level and errors.
        assert_eq!(UPVAR.resolve(&["foo", "bar", "baz"]).0, FrameLevel::Dynamic);
    }

    #[test]
    fn uplevel_level_word_is_probed_from_the_leading_text() {
        assert_eq!(
            UPLEVEL.resolve(&["1", "{set x 1}"]).0,
            FrameLevel::Relative(1)
        );
        assert_eq!(
            UPLEVEL.resolve(&["#0", "{set x 1}"]).0,
            FrameLevel::Absolute(0)
        );
        // A lone substituted word is the body, not a level.
        assert_eq!(UPLEVEL.resolve(&["$body"]).0, FrameLevel::DEFAULT);
        assert_eq!(UPLEVEL.resolve(&["$body"]).1, ["$body"]);
        // With a following word it separates.
        assert_eq!(UPLEVEL.resolve(&["$lvl", "$body"]).0, FrameLevel::Dynamic);
        assert_eq!(UPLEVEL.resolve(&["$lvl", "$body"]).1, ["$body"]);
        // No level word at all.
        assert_eq!(UPLEVEL.resolve(&["{expr {1+1}}"]).0, FrameLevel::DEFAULT);
    }

    #[test]
    fn frame_predicates() {
        assert!(FrameLevel::DEFAULT.is_caller_frame());
        assert!(FrameLevel::Relative(0).is_current_frame());
        assert!(FrameLevel::Absolute(0).is_global_frame());
        assert!(!FrameLevel::Relative(2).is_caller_frame());
        assert!(!FrameLevel::Dynamic.is_caller_frame());
        assert!(!FrameLevel::Absolute(1).is_global_frame());
    }
}
