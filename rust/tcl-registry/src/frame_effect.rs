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
//! 1. **Is there a level word, and where?**  `upvar` decides on argument
//!    *count parity*; `uplevel` probes the leading word's text.
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
//! Every rule below is pinned against `tclsh 9.0.4` **and** `tclsh 8.6.14`:
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
//! matrix.  This module previously claimed the grammar was invariant, on the
//! strength of a 20-spelling matrix that happened to contain no divergent
//! spelling and a call chain too shallow to distinguish a parse failure from an
//! out-of-range level — C reports `bad level` for both.
//!
//! For the presence rule's two divergent classes and their transcripts, see
//! [`FrameLevelWord::LeadingProbe`] ([issue #1069][]).
//!
//! [issue #1069]: https://github.com/bitwisecook/tcl-lsp/issues/1069

use tcl_dialect::{NumberSyntax, TclVersion};
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

    /// [`Self::parse_for`] with the release taken from `registry`'s loaded
    /// dialect profile — the ordinary way a consumer inside the compiler names
    /// the release it is analysing for.
    ///
    /// A registry with no profile loaded falls back to [`Self::parse`]'s
    /// abstaining behaviour, which is the same answer that consumer would have
    /// got before it had a release to name.
    #[must_use]
    pub fn parse_in(word: &str, registry: &crate::registry::CommandRegistry) -> Option<Self> {
        Self::parse_for(word, registry.runtime_version())
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
        let signed = parse_signed_level_value(
            word,
            version.map_or_else(NumberSyntax::default, TclVersion::number_syntax),
        );
        let leading_digit = word.as_bytes().first().is_some_and(u8::is_ascii_digit);
        match version {
            // 9.0 / 9.1: the whole word goes to `Tcl_GetIntFromObj`, sign and
            // all; a digit-led non-integer (`1.0`) is never a level.
            Some(TclVersion::V9_0 | TclVersion::V9_1) => signed.is_some(),
            // 8.4 – 8.6: a leading digit alone commits the word to the level
            // slot, and a negative integer is dispatched as a command instead.
            Some(TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6) => {
                leading_digit || signed.is_some_and(|v| v >= 0)
            }
            // No dialect in hand — intersect both eras: only a word every
            // release consumes goes in the level slot.  A negative integer is
            // never digit-led, so the 8.x `leading_digit` disjunct adds
            // nothing the 9.x integer test does not already cover here.
            None => signed.is_some_and(|v| v >= 0),
        }
    }
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

/// The same word read as a *signed* integer, for the level-word **presence**
/// question — 9.0 consumes `-1` as a level (and then rejects it) where 8.6
/// dispatches it as a command.
fn parse_signed_level_value(text: &str, numbers: NumberSyntax) -> Option<i64> {
    let (negative, magnitude) = parse_level_magnitude(text, numbers)?;
    let value = i64::from(magnitude);
    Some(if negative { -value } else { value })
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
    /// **Leading-word probe**, `uplevel`'s rule: the first word is the level
    /// when it parses as one, or when it substitutes *and* a further word
    /// follows (`uplevel $lvl {…}`; a lone `uplevel $body` is a body).
    ///
    /// Unlike the level *value* grammar ([`FrameLevel::parse`]), this
    /// presence rule is the one place `upvar`/`uplevel` semantics genuinely
    /// diverge between releases — `TclObjGetFrame` was reworked in 9.0.  Two
    /// classes differ; both are hard errors under both interpreters, so no
    /// *working* script is shaped differently, but the registry records the
    /// fact rather than asserting one release's answer for all
    /// ([issue #1069][]).  `uplevel W {oops}`, tclsh 9.0.4 vs 8.6.14:
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
    ///
    /// [issue #1069]: https://github.com/bitwisecook/tcl-lsp/issues/1069
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
            FrameLevelWord::LeadingProbe => None,
        }
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
            FrameLevel::parse(args[0]).unwrap_or(FrameLevel::Dynamic)
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
        let both = [
            "1", "0", "2", "007", "#0", "#1", "#-0", "#x", "+1", "+0", "-0", " 1", "1 ", "0x1",
            "0X1", "0b1", "0o1",
        ];
        let nine_only = ["-1", "-2"];
        let eight_only = ["1.0", "1e0"];
        // An empty word is deliberately absent: `uplevel {} {oops}` concats
        // to " oops" whichever way it is read, so C Tcl cannot be asked.
        let neither = ["x", "foo"];
        for v in [TclVersion::V9_0, TclVersion::V9_1] {
            for w in both {
                assert!(FrameLevel::word_could_be_level(w, Some(v)), "{v:?} {w:?}");
            }
            for w in nine_only {
                assert!(FrameLevel::word_could_be_level(w, Some(v)), "{v:?} {w:?}");
            }
            for w in eight_only.iter().chain(neither.iter()) {
                assert!(!FrameLevel::word_could_be_level(w, Some(v)), "{v:?} {w:?}");
            }
        }
        for v in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            for w in both {
                assert!(FrameLevel::word_could_be_level(w, Some(v)), "{v:?} {w:?}");
            }
            for w in eight_only {
                assert!(FrameLevel::word_could_be_level(w, Some(v)), "{v:?} {w:?}");
            }
            for w in nine_only.iter().chain(neither.iter()) {
                assert!(!FrameLevel::word_could_be_level(w, Some(v)), "{v:?} {w:?}");
            }
        }
        // No dialect in hand — the *intersection*: a word only one release
        // consumes leaves the script unrunnable on the other, so the level
        // slot stays empty and the word is treated as script text.
        for w in both {
            assert!(FrameLevel::word_could_be_level(w, None), "shared {w:?}");
        }
        for w in nine_only
            .iter()
            .chain(eight_only.iter())
            .chain(neither.iter())
        {
            assert!(!FrameLevel::word_could_be_level(w, None), "split {w:?}");
        }
        // A substituted word's *text* is never the presence answer; its
        // arity is (see `level_word_len_for_version`).
        assert!(!FrameLevel::word_could_be_level("$lvl", None));
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
