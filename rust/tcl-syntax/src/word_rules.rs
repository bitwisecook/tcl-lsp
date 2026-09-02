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

//! The dialect's answer to "what does this *word* mean as a value" — the
//! single owner of the two axes every layer needs and none should re-derive.
//!
//! [`LexerGrammar`] states the axes; [`crate::list`] and [`crate::backslash`]
//! hold the algorithms. This module is the join: one type carrying both axes,
//! with the operations on it, so the lexer, the compiler, codegen, the runtime
//! and the tooling all get the *same* answer for the same document.
//!
//! It exists because they did not. `brace_backslash_newline` reached only
//! lowering's parameter/variable-list helpers while `Codegen::push_lit_verbatim`,
//! the taint walker and the signature scanner each called the unconditional
//! collapse; `list_parse` reached nothing at all while the VM's list
//! conversions called the strict splitter. Three consumers, three answers to a
//! question the dialect owns — the shape [`tcl_registry::CommandSpec::return_type_for_call`]
//! was introduced to end for per-call result types (issue #1720). A consumer
//! that needs either axis takes a `WordValueRules` and asks it; it does not
//! reach for [`crate::list::split_list`] or
//! [`crate::backslash::collapse_brace_continuations_str`] directly and decide
//! for itself.
//!
//! The axes travel together because the two questions are one question at
//! every call site that splits a word-shaped list: the brace rule decides what
//! bytes the word contains, and the list rule decides how those bytes divide.
//! Answering one per dialect and the other per hardcoded default is precisely
//! the bug this replaces.

use crate::list::ListError;
use std::borrow::Cow;
use tcl_dialect::{BraceBackslashNewline, LexerGrammar, ListParse};

/// The word-value rules of one dialect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordValueRules {
    /// Whether a `\<newline>` inside a braced word folds to a space.
    pub brace: BraceBackslashNewline,
    /// Whether malformed list text raises or is split anyway.
    pub list: ListParse,
}

impl Default for WordValueRules {
    /// Every build of the Tcl core: fold the continuation, raise on malformed
    /// list text. A caller with no dialect in hand gets C Tcl, which is what
    /// the unconditional helpers used to do unconditionally.
    fn default() -> Self {
        Self::TCL
    }
}

impl WordValueRules {
    /// Every build of the Tcl core, and the F5 fork.
    pub const TCL: Self = Self {
        brace: BraceBackslashNewline::Folds,
        list: ListParse::Strict,
    };

    /// `JimTcl`, every modelled release.
    pub const JIM: Self = Self {
        brace: BraceBackslashNewline::Literal,
        list: ListParse::Lenient,
    };

    /// The rules a compiled or pack-declared grammar states.
    #[must_use]
    pub fn from_grammar(grammar: &LexerGrammar) -> Self {
        Self {
            brace: grammar.brace_backslash_newline,
            list: grammar.list_parse,
        }
    }

    /// The rules a live lexer configuration carries — the form production
    /// callers use, since a document's config is what they already hold.
    #[must_use]
    pub fn from_config(config: &tcl_lexer::LexerConfig) -> Self {
        Self {
            brace: config.brace_backslash_newline,
            list: config.list_parse,
        }
    }

    /// The rules of a dialect named at the compile's entry point, or C Tcl
    /// when the caller has none.
    ///
    /// The counterpart of [`BraceBackslashNewline::of_dialect_name`] and the
    /// other axis constructors, and here for the same stated reason: "no
    /// dialect means the default rule" is written once, so a layer cannot
    /// answer differently and read the same bytes under a rule the document
    /// was not lexed with.
    #[must_use]
    pub fn of_dialect_name(name: Option<&str>) -> Self {
        Self::from_grammar(&tcl_dialect::grammar_of_dialect_name(name))
    }

    /// The rules of an already-resolved profile, or C Tcl when the caller has
    /// none — for the layers that carry an `Option<&DialectProfile>`.
    #[must_use]
    pub fn of_profile(profile: Option<&tcl_dialect::DialectProfile>) -> Self {
        profile.map_or(Self::TCL, |p| Self::from_grammar(&p.grammar))
    }

    /// Collapse a braced word's line continuations under this dialect.
    ///
    /// C Tcl folds `\<newline>` and any following blanks to one space; Jim
    /// keeps the bytes, deliberately, to preserve line numbers.
    #[must_use]
    pub fn collapse_braced_word(self, text: &str) -> Cow<'_, str> {
        crate::backslash::collapse_brace_continuations_str_for(text, self.brace)
    }

    /// Split list text under this dialect.
    ///
    /// `Lenient` never returns `Err` — `JimTcl`'s list parser does not raise —
    /// so a caller that has already established the dialect is Jim may
    /// `expect` on it; one that has not must handle the error as before.
    pub fn split_list(self, text: &str) -> Result<Vec<Cow<'_, str>>, ListError> {
        match self.list {
            ListParse::Strict => crate::list::split_list(text),
            ListParse::Lenient => Ok(crate::list::split_list_jim(text)),
        }
    }

    /// The **tolerant** element split under this dialect's list grammar —
    /// the best-effort sibling of [`Self::split_list`] for a fold or a
    /// scan that must still yield the elements *before* a malformed tail
    /// rather than nothing. Strict list parsing tolerates through
    /// [`crate::list::split_list_lenient`]; Jim's grammar is tolerant by construction
    /// ([`crate::list::split_list_jim`]), so the same axis chooses.
    #[must_use]
    pub fn split_list_tolerant(self, text: &str) -> Vec<Cow<'_, str>> {
        match self.list {
            ListParse::Strict => crate::list::split_list_lenient(text),
            ListParse::Lenient => crate::list::split_list_jim(text),
        }
    }

    /// The word-shaped-list helper: collapse the braced word, then split it,
    /// yielding owned names. `None` is "this dialect raises on this text",
    /// which a static consumer turns into a barrier and never into a guess.
    #[must_use]
    pub fn split_word_names(self, text: &str) -> Option<Vec<String>> {
        let collapsed = self.collapse_braced_word(text);
        self.split_list(&collapsed)
            .ok()
            .map(|v| v.into_iter().map(Cow::into_owned).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::{Family, grammar};

    /// The constants are what the compiled catalogue says, for every release
    /// on both ladders — so a consumer may use `WordValueRules::JIM` in a test
    /// without it drifting from the real grammar.
    #[test]
    fn the_constants_match_the_catalogue() {
        for &release in Family::Jim.releases() {
            assert_eq!(
                WordValueRules::from_grammar(&grammar(Family::Jim, release)),
                WordValueRules::JIM,
                "{release}"
            );
        }
        for &release in Family::Tcl.releases() {
            assert_eq!(
                WordValueRules::from_grammar(&grammar(Family::Tcl, release)),
                WordValueRules::TCL,
                "{release}"
            );
        }
    }

    /// A grammar and the config built from it must answer identically —
    /// the property that makes `from_config` safe for production callers.
    #[test]
    fn grammar_and_config_agree() {
        for family in [Family::Tcl, Family::Jim] {
            for &release in family.releases() {
                let g = grammar(family, release);
                assert_eq!(
                    WordValueRules::from_grammar(&g),
                    WordValueRules::from_config(&tcl_lexer::LexerConfig::from_grammar(g)),
                    "{family:?} {release}"
                );
            }
        }
    }

    /// `of_dialect_name` and `of_profile` agree for every dialect the legacy
    /// catalogue can name — and *cannot* agree for one it cannot.
    ///
    /// `jim` is deliberately not a catalogue profile (P6: a grammar is a
    /// function of `(family, release, build)`, so an environment names the
    /// family and ladder instead), which means `of_profile` receives `None`
    /// for a Jim document and answers C Tcl. That asymmetry is a property of
    /// the `Option<&DialectProfile>` currency, not of this type, and it is why
    /// a layer should carry a resolved point rather than a profile handle.
    #[test]
    fn the_name_and_profile_constructors_agree_where_a_profile_exists() {
        for (name, expected) in [
            ("tcl8.6", WordValueRules::TCL),
            ("tcl9.0", WordValueRules::TCL),
            ("f5-irules", WordValueRules::TCL),
        ] {
            let profile = tcl_dialect::DialectProfile::find(name);
            assert!(profile.is_some(), "{name} is a catalogue profile");
            assert_eq!(
                WordValueRules::of_dialect_name(Some(name)),
                expected,
                "{name}"
            );
            assert_eq!(WordValueRules::of_profile(profile), expected, "{name}");
        }
        assert_eq!(WordValueRules::of_dialect_name(None), WordValueRules::TCL);
        assert_eq!(WordValueRules::of_profile(None), WordValueRules::TCL);
    }

    /// The name route sees Jim; the profile route structurally cannot.
    #[test]
    fn the_profile_route_cannot_name_jim() {
        assert!(tcl_dialect::DialectProfile::find("jim").is_none());
        assert_eq!(
            WordValueRules::of_dialect_name(Some("jim")),
            WordValueRules::JIM,
            "the name resolves through the environment model"
        );
    }

    /// The default is C Tcl, so a caller with no dialect behaves exactly as
    /// the unconditional helpers did.
    #[test]
    fn the_default_is_tcl() {
        assert_eq!(WordValueRules::default(), WordValueRules::TCL);
        assert_eq!(
            WordValueRules::default().collapse_braced_word("a\\\n  b"),
            "a b"
        );
        assert!(WordValueRules::default().split_list("a {b").is_err());
    }

    /// The two axes move together, and both are visible through one call.
    #[test]
    fn the_two_dialects_differ_on_both_axes() {
        let wrapped = "a b\\\nc";
        assert_eq!(
            WordValueRules::TCL.split_word_names(wrapped).unwrap(),
            vec!["a", "b", "c"]
        );
        assert_eq!(
            WordValueRules::JIM.split_word_names(wrapped).unwrap(),
            vec!["a", "b c"]
        );

        assert_eq!(WordValueRules::TCL.split_word_names("a {b"), None);
        assert_eq!(
            WordValueRules::JIM.split_word_names("a {b").unwrap(),
            vec!["a", "b"]
        );
    }
}
