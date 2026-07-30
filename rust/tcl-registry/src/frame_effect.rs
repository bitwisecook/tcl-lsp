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
//! Every rule below is pinned against `tclsh 9.0.4` **and** `tclsh 8.6.14`,
//! which agree on all of it:
//!
//! ```tcl
//! proc t3 {} { return [catch {set b} e]:$e }   ;# body: upvar 1 b
//! proc h3 {} { set 1 ONE; return [t3] }
//! h3   ;# → 0:ONE   — `1` is the *otherVar*, not a level
//!
//! proc t6 {} { return [catch {upvar foo bar baz} e]:$e }
//! t6   ;# → 1:bad level "foo"   — 3 words ⇒ the first IS the level
//! ```

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

    /// Parse a level word.
    ///
    /// Returns `None` when the word cannot be a level at all — C Tcl's
    /// `TclObjGetFrame` accepts an optional `#`, then an integer parsed by
    /// `Tcl_GetIntFromObj`, so `+1` and `0x1` are levels (tclsh 9.0.4 /
    /// 8.6.14: `upvar 0x1 target alias` reaches the caller) while `1.0` and
    /// `-1` are `bad level` errors.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        if word.contains('$') || word.contains('[') {
            return Some(Self::Dynamic);
        }
        let trimmed = word.trim();
        let (absolute, digits) = match trimmed.strip_prefix('#') {
            Some(rest) => (true, rest.trim_start()),
            None => (false, trimmed),
        };
        let digits = digits.strip_prefix('+').unwrap_or(digits);
        let value = if let Some(hex) = digits
            .strip_prefix("0x")
            .or_else(|| digits.strip_prefix("0X"))
        {
            u32::from_str_radix(hex, 16).ok()?
        } else {
            digits.parse::<u32>().ok()?
        };
        Some(if absolute {
            Self::Absolute(value)
        } else {
            Self::Relative(value)
        })
    }

    /// Whether *word* could be the level word of a command that probes its
    /// leading argument for one ([`FrameLevelWord::LeadingProbe`]).
    #[must_use]
    pub fn word_could_be_level(word: &str) -> bool {
        Self::parse(word).is_some()
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
    /// **Leading-word probe**, `uplevel`'s rule: the first word is the level
    /// when it parses as one, or when it substitutes *and* a further word
    /// follows (`uplevel $lvl {…}`; a lone `uplevel $body` is a body).
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
    /// How many leading words of `args` (the argument list *after* the
    /// command name) the level word occupies — `1` or `0`.
    ///
    /// `args` must be the whole post-command word list, because
    /// [`FrameLevelWord::ArityParity`] answers from its length.
    #[must_use]
    pub fn level_word_len(&self, args: &[&str]) -> usize {
        match self.level_word {
            FrameLevelWord::None => 0,
            FrameLevelWord::ArityParity => usize::from(args.len() % 2 == 1),
            FrameLevelWord::LeadingProbe => match args.first() {
                // A word that parses as a level is one, whatever follows:
                // `uplevel 1 2 3` runs the script `2 3` at level 1 (tclsh
                // 9.0.4 / 8.6.14 both report `invalid command name "2"`).
                Some(w) if FrameLevel::parse(w).is_some_and(|l| l != FrameLevel::Dynamic) => 1,
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
        let taken = self.level_word_len(args);
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

    #[test]
    fn level_words_c_tcl_accepts() {
        // tclsh 9.0.4 / 8.6.14: `upvar 0x1 target alias` and `upvar +1 …`
        // both reach the caller; `1.0` and `-1` raise `bad level`.
        assert_eq!(FrameLevel::parse("1"), Some(FrameLevel::Relative(1)));
        assert_eq!(FrameLevel::parse("0"), Some(FrameLevel::Relative(0)));
        assert_eq!(FrameLevel::parse("#0"), Some(FrameLevel::Absolute(0)));
        assert_eq!(FrameLevel::parse("#1"), Some(FrameLevel::Absolute(1)));
        assert_eq!(FrameLevel::parse(" 1"), Some(FrameLevel::Relative(1)));
        assert_eq!(FrameLevel::parse("+1"), Some(FrameLevel::Relative(1)));
        assert_eq!(FrameLevel::parse("0x1"), Some(FrameLevel::Relative(1)));
        assert_eq!(FrameLevel::parse("$lvl"), Some(FrameLevel::Dynamic));
        assert_eq!(
            FrameLevel::parse("[expr {$n-1}]"),
            Some(FrameLevel::Dynamic)
        );
        assert_eq!(FrameLevel::parse("1.0"), None);
        assert_eq!(FrameLevel::parse("-1"), None);
        assert_eq!(FrameLevel::parse("foo"), None);
        assert_eq!(FrameLevel::parse(""), None);
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
