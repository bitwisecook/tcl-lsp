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

//! Structured source-word facts for target-neutral registry resolution.
//!
//! A Tcl source word is not necessarily the value eventually presented to a
//! command: substitution can replace it, and `{*}` expansion can change the
//! number of argv entries altogether.  This module makes that distinction
//! explicit before registry matching reaches literal-sensitive metadata such
//! as subcommand tables and argument-dependent effect resolvers.

/// The source-level knowledge available for one Tcl invocation word.
///
/// Only [`Self::Literal`] exposes a string.  The remaining variants
/// deliberately retain no raw spelling: treating source text such as `$kind`
/// as its runtime value would make a registry decision unsound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvocationWord<'w> {
    /// A word whose Tcl value is known to be exactly this literal string.
    Literal(&'w str),
    /// One non-expanded argv word whose value is produced by substitution.
    Dynamic,
    /// A `{*}` word whose evaluated Tcl list contributes an unknown number of
    /// argv entries.
    Expanded,
    /// A source region whose word and argv shape are unavailable to this
    /// resolver.
    Opaque,
}

/// The non-value classification of an [`InvocationWord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvocationWordKind {
    /// The word has a known literal value.
    Literal,
    /// The word's value is computed by substitution.
    Dynamic,
    /// The word expands a runtime Tcl list into argv entries.
    Expanded,
    /// The source region has no reliable word or argv representation.
    Opaque,
}

impl<'w> InvocationWord<'w> {
    /// Return this word's known Tcl value, if and only if it is literal.
    #[must_use]
    pub const fn literal(self) -> Option<&'w str> {
        match self {
            Self::Literal(value) => Some(value),
            Self::Dynamic | Self::Expanded | Self::Opaque => None,
        }
    }

    /// Return the source knowledge category without exposing a non-literal
    /// spelling as a runtime value.
    #[must_use]
    pub const fn kind(self) -> InvocationWordKind {
        match self {
            Self::Literal(_) => InvocationWordKind::Literal,
            Self::Dynamic => InvocationWordKind::Dynamic,
            Self::Expanded => InvocationWordKind::Expanded,
            Self::Opaque => InvocationWordKind::Opaque,
        }
    }

    /// Whether this source word is guaranteed to contribute exactly one argv
    /// entry after evaluation.
    #[must_use]
    pub const fn has_exactly_one_argv_entry(self) -> bool {
        matches!(self, Self::Literal(_) | Self::Dynamic)
    }
}

/// A borrowed, allocation-free view of post-head invocation words.
///
/// The literal representation keeps the historic `&[&str]` path zero-copy.
/// A structured representation is used when a source consumer knows that a
/// word was substituted, expanded, or opaque.  Query methods intentionally
/// expose values only through [`Self::literal_at`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationArguments<'w> {
    /// A compatibility view in which every word is known literal.
    Literals(&'w [&'w str]),
    /// A source-aware view containing individual word facts.
    Structured(&'w [InvocationWord<'w>]),
}

impl<'w> InvocationArguments<'w> {
    /// Construct an all-literal, zero-copy argument view.
    #[must_use]
    pub const fn literals(words: &'w [&'w str]) -> Self {
        Self::Literals(words)
    }

    /// Construct a source-aware argument view.
    #[must_use]
    pub const fn structured(words: &'w [InvocationWord<'w>]) -> Self {
        Self::Structured(words)
    }

    /// Number of source words following the command head.
    ///
    /// This is not necessarily the final argv count; see
    /// [`Self::exact_argv_len`].
    #[must_use]
    pub const fn len(self) -> usize {
        match self {
            Self::Literals(words) => words.len(),
            Self::Structured(words) => words.len(),
        }
    }

    /// Whether there are no post-head source words.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Return the word fact at `index`.
    #[must_use]
    pub fn get(self, index: usize) -> Option<InvocationWord<'w>> {
        match self {
            Self::Literals(words) => words.get(index).copied().map(InvocationWord::Literal),
            Self::Structured(words) => words.get(index).copied(),
        }
    }

    /// Return a post-head value only when it is known literal.
    #[must_use]
    pub fn literal_at(self, index: usize) -> Option<&'w str> {
        self.get(index).and_then(InvocationWord::literal)
    }

    /// Whether every source word has a known literal Tcl value.
    #[must_use]
    pub fn are_all_literals(self) -> bool {
        match self {
            Self::Literals(_) => true,
            Self::Structured(words) => words
                .iter()
                .all(|word| matches!(word, InvocationWord::Literal(_))),
        }
    }

    /// Whether every source word is guaranteed to contribute one argv entry.
    #[must_use]
    pub fn has_exact_argv_len(self) -> bool {
        match self {
            Self::Literals(_) => true,
            Self::Structured(words) => words.iter().all(|word| word.has_exactly_one_argv_entry()),
        }
    }

    /// Return the final argv count when expansion cannot change it.
    #[must_use]
    pub fn exact_argv_len(self) -> Option<usize> {
        self.has_exact_argv_len().then_some(self.len())
    }

    /// Whether any word is dynamic, expanded, or opaque.
    #[must_use]
    pub fn has_non_literal(self) -> bool {
        !self.are_all_literals()
    }

    /// Return every post-head Tcl value when all source words are literal.
    ///
    /// This is the explicit compatibility bridge for established registry
    /// resolvers whose input is `&[&str]`.  A dynamic, expanded, or opaque
    /// word makes the whole projection unavailable; callers must retain their
    /// typed unknown obligation instead of passing source spelling to the
    /// resolver.
    #[must_use]
    pub fn literal_values(self) -> Option<Vec<&'w str>> {
        match self {
            Self::Literals(words) => Some(words.to_vec()),
            Self::Structured(words) => words.iter().map(|word| word.literal()).collect(),
        }
    }

    /// Borrow the arguments from `start` onwards while preserving their
    /// source-knowledge representation.
    #[must_use]
    pub fn slice_from(self, start: usize) -> Self {
        match self {
            Self::Literals(words) => Self::Literals(words.get(start..).unwrap_or(&[])),
            Self::Structured(words) => Self::Structured(words.get(start..).unwrap_or(&[])),
        }
    }
}

/// Inputs supplied to a registry command-prefix resolver.
///
/// `spellings` preserves the compatibility view used by position-only
/// resolvers. `words` is the source-aware truth used by any resolver whose
/// appended arity depends on a literal argument value. Such a resolver must
/// use [`Self::literal_at`] and abstain when it returns `None`, never interpret
/// a dynamic word's source spelling as its runtime value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandPrefixArguments<'w> {
    spellings: &'w [&'w str],
    words: InvocationArguments<'w>,
}

impl<'w> CommandPrefixArguments<'w> {
    /// Construct a compatibility view in which every argument is literal.
    #[must_use]
    pub const fn literals(spellings: &'w [&'w str]) -> Self {
        Self {
            spellings,
            words: InvocationArguments::literals(spellings),
        }
    }

    /// Construct a source-aware view alongside reconstructed source
    /// spellings. Both slices must describe the same post-head words.
    #[must_use]
    pub fn structured(spellings: &'w [&'w str], words: &'w [InvocationWord<'w>]) -> Option<Self> {
        (spellings.len() == words.len()).then_some(Self {
            spellings,
            words: InvocationArguments::structured(words),
        })
    }

    /// Number of post-head source words.
    #[must_use]
    pub const fn len(self) -> usize {
        self.words.len()
    }

    /// Whether there are no post-head source words.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.words.is_empty()
    }

    /// Reconstructed source spellings for position-only compatibility
    /// resolvers. Do not use these to make a value-dependent decision.
    #[must_use]
    pub const fn spellings(self) -> &'w [&'w str] {
        self.spellings
    }

    /// Return an argument value only when source analysis proves it literal.
    #[must_use]
    pub fn literal_at(self, index: usize) -> Option<&'w str> {
        self.words.literal_at(index)
    }

    /// Borrow the arguments from `start` onwards, preserving both views.
    #[must_use]
    pub fn slice_from(self, start: usize) -> Self {
        Self {
            spellings: self.spellings.get(start..).unwrap_or(&[]),
            words: self.words.slice_from(start),
        }
    }
}

/// A structured source-word view for one invocation.
///
/// The command head is structured too.  Registry resolution can only select
/// a command descriptor when [`Self::head_literal`] is present; it never
/// treats a substituted head's source spelling as a command name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvocationWords<'w> {
    head: InvocationWord<'w>,
    arguments: InvocationArguments<'w>,
}

impl<'w> InvocationWords<'w> {
    /// Construct an all-literal invocation without allocating.
    #[must_use]
    pub const fn literals(head: &'w str, arguments: &'w [&'w str]) -> Self {
        Self {
            head: InvocationWord::Literal(head),
            arguments: InvocationArguments::Literals(arguments),
        }
    }

    /// Construct an invocation from source-aware word facts.
    #[must_use]
    pub const fn structured(head: InvocationWord<'w>, arguments: &'w [InvocationWord<'w>]) -> Self {
        Self {
            head,
            arguments: InvocationArguments::Structured(arguments),
        }
    }

    /// Return the command-head word fact.
    #[must_use]
    pub const fn head(self) -> InvocationWord<'w> {
        self.head
    }

    /// Return the command head only when it is a known literal.
    #[must_use]
    pub const fn head_literal(self) -> Option<&'w str> {
        self.head.literal()
    }

    /// Return the structured post-head argument view.
    #[must_use]
    pub const fn arguments(self) -> InvocationArguments<'w> {
        self.arguments
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_words_only_expose_known_literals() {
        let words = [
            InvocationWord::Literal("fixed"),
            InvocationWord::Dynamic,
            InvocationWord::Expanded,
            InvocationWord::Opaque,
        ];
        let arguments = InvocationArguments::structured(&words);

        assert_eq!(arguments.literal_at(0), Some("fixed"));
        assert_eq!(arguments.literal_at(1), None);
        assert_eq!(arguments.literal_at(2), None);
        assert_eq!(arguments.literal_at(3), None);
        assert!(!arguments.are_all_literals());
        assert!(!arguments.has_exact_argv_len());
        assert_eq!(arguments.exact_argv_len(), None);
        assert_eq!(arguments.literal_values(), None);
    }

    #[test]
    fn literal_values_preserve_complete_known_argv() {
        let structured = [
            InvocationWord::Literal("first"),
            InvocationWord::Literal("second"),
        ];
        assert_eq!(
            InvocationArguments::structured(&structured).literal_values(),
            Some(vec!["first", "second"])
        );
    }
}
