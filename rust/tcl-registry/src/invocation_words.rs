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

use tcl_native_model::InvocationDialect;

/// Contents alternatives for one actual ordinary argv position. These are
/// value-analysis facts, not a literal, object or execution binding proof.
#[derive(Debug, Clone, Copy)]
pub enum RoleOperandValues<'a> {
    /// Every possible value is retained; the set must be nonempty and bounded.
    Closed(&'a [String]),
    /// An opaque or missing alternative remains possible.
    Unknown,
}

/// Closed value information for role/layout consensus, preserving argc.
#[derive(Debug, Clone, Copy)]
pub struct RoleOperandAlternatives<'a> {
    /// Evaluated argv index after the command head, not a source word offset.
    pub argument: usize,
    /// Contents possibilities with an explicit unknown residual.
    pub values: RoleOperandValues<'a>,
}

/// The source-level knowledge available for one Tcl invocation word.
///
/// Only [`Self::Literal`] exposes a logical string. [`Self::KnownBytes`]
/// retains an independently produced native byte value without a Unicode
/// projection. Neither facet certifies an object, source word or execution
/// binding; computed source spelling such as `$kind` is never a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvocationWord<'w> {
    /// A word whose Tcl value is known to be exactly this literal string.
    Literal(&'w str),
    /// One actual argv operand with an exact native byte value. This carries
    /// neither logical text nor a native object, name or compiler receipt.
    KnownBytes(&'w [u8]),
    /// One non-expanded argv word whose value is produced by substitution.
    Dynamic,
    /// One substituted argv word whose non-empty literal prefix proves its
    /// evaluated value cannot begin with `-` and therefore cannot be a
    /// leading option. For example, `"prefix-$value"` remains dynamic as a
    /// value, but it cannot select a command option before the known
    /// positional layout.
    DynamicNonOption,
    /// A single evaluated variable-name value with a proved literal array root
    /// and an unknown element. No complete string or element value is exposed.
    ArrayElementName {
        /// Literal root before the first opening parenthesis in the value.
        root: &'w str,
    },
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
    /// The native byte value is known, without a logical string projection.
    KnownBytes,
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
            Self::KnownBytes(_)
            | Self::Dynamic
            | Self::DynamicNonOption
            | Self::ArrayElementName { .. }
            | Self::Expanded
            | Self::Opaque => None,
        }
    }

    /// Borrow the exact native byte facet, without encoding a logical string.
    #[must_use]
    pub const fn native_bytes(self) -> Option<&'w [u8]> {
        match self {
            Self::KnownBytes(value) => Some(value),
            _ => None,
        }
    }

    /// Return the source knowledge category without exposing a non-literal
    /// spelling as a runtime value.
    #[must_use]
    pub const fn kind(self) -> InvocationWordKind {
        match self {
            Self::Literal(_) => InvocationWordKind::Literal,
            Self::KnownBytes(_) => InvocationWordKind::KnownBytes,
            Self::Dynamic | Self::DynamicNonOption | Self::ArrayElementName { .. } => {
                InvocationWordKind::Dynamic
            }
            Self::Expanded => InvocationWordKind::Expanded,
            Self::Opaque => InvocationWordKind::Opaque,
        }
    }

    /// Whether this source word is guaranteed to contribute exactly one argv
    /// entry after evaluation.
    #[must_use]
    pub const fn has_exactly_one_argv_entry(self) -> bool {
        matches!(
            self,
            Self::Literal(_)
                | Self::KnownBytes(_)
                | Self::Dynamic
                | Self::DynamicNonOption
                | Self::ArrayElementName { .. }
        )
    }

    /// Whether the evaluated word has a stable first character other than an option dash.
    #[must_use]
    pub fn proves_non_option(self) -> bool {
        match self {
            Self::DynamicNonOption => true,
            Self::KnownBytes(value) => value.first().is_some_and(|first| *first != b'-'),
            Self::ArrayElementName { root } => !root.starts_with('-'),
            _ => false,
        }
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
    /// Literal arguments carrying the ingress execution policies.
    ContextualLiterals(&'w [&'w str], InvocationDialect),
    /// Source-aware arguments carrying the ingress execution policies.
    ContextualStructured(&'w [InvocationWord<'w>], InvocationDialect),
}

/// Knowledge about one post-expansion argv position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvocationArgument<'w> {
    /// The position maps to one source word with the stated value knowledge.
    Word(InvocationWord<'w>),
    /// The source is known to contribute fewer argv entries than this index.
    Missing,
    /// An earlier expansion or opaque region makes this position unknowable.
    Indeterminate,
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

    /// Stamp explicitly resolved policies, including non-catalogue environments.
    #[must_use]
    pub const fn with_dialect(self, dialect: InvocationDialect) -> Self {
        match self {
            Self::Literals(words) | Self::ContextualLiterals(words, _) => {
                Self::ContextualLiterals(words, dialect)
            }
            Self::Structured(words) | Self::ContextualStructured(words, _) => {
                Self::ContextualStructured(words, dialect)
            }
        }
    }

    /// Stamp the execution policies while retaining source-value knowledge.
    #[must_use]
    pub fn with_profile(self, profile: Option<&tcl_dialect::DialectProfile>) -> Self {
        if self.dialect().is_some() {
            return self;
        }
        let Some(profile) = profile else { return self };
        self.with_dialect(InvocationDialect::of_profile(profile))
    }

    /// The selected execution policies, when the caller supplied a profile.
    #[must_use]
    pub const fn dialect(self) -> Option<InvocationDialect> {
        match self {
            Self::ContextualLiterals(_, dialect) | Self::ContextualStructured(_, dialect) => {
                Some(dialect)
            }
            Self::Literals(_) | Self::Structured(_) => None,
        }
    }

    /// Number of source words following the command head.
    ///
    /// This is not necessarily the final argv count; see
    /// [`Self::exact_argv_len`].
    #[must_use]
    pub const fn len(self) -> usize {
        match self {
            Self::Literals(words) | Self::ContextualLiterals(words, _) => words.len(),
            Self::Structured(words) | Self::ContextualStructured(words, _) => words.len(),
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
            Self::Literals(words) | Self::ContextualLiterals(words, _) => {
                words.get(index).copied().map(InvocationWord::Literal)
            }
            Self::Structured(words) | Self::ContextualStructured(words, _) => {
                words.get(index).copied()
            }
        }
    }

    /// Project one final argv position while retaining precise source prefixes.
    ///
    /// A `{*}` or opaque word makes its own position and every following
    /// position indeterminate, but it does not erase exact words before it.
    #[must_use]
    pub fn argv_at(self, index: usize) -> InvocationArgument<'w> {
        match self {
            Self::Literals(words) | Self::ContextualLiterals(words, _) => words
                .get(index)
                .copied()
                .map_or(InvocationArgument::Missing, |word| {
                    InvocationArgument::Word(InvocationWord::Literal(word))
                }),
            Self::Structured(words) | Self::ContextualStructured(words, _) => {
                let mut argv_index = 0;
                for word in words {
                    match word {
                        InvocationWord::Expanded | InvocationWord::Opaque => {
                            return InvocationArgument::Indeterminate;
                        }
                        InvocationWord::Literal(_)
                        | InvocationWord::KnownBytes(_)
                        | InvocationWord::Dynamic
                        | InvocationWord::DynamicNonOption
                        | InvocationWord::ArrayElementName { .. } => {
                            if argv_index == index {
                                return InvocationArgument::Word(*word);
                            }
                            argv_index += 1;
                        }
                    }
                }
                InvocationArgument::Missing
            }
        }
    }

    /// Return a post-head value only when it is known literal.
    #[must_use]
    pub fn literal_at(self, index: usize) -> Option<&'w str> {
        self.get(index).and_then(InvocationWord::literal)
    }

    /// Borrow an actual native byte value at this source position. Logical
    /// literals are not encoded into native units by this metadata query.
    #[must_use]
    pub fn native_bytes_at(self, index: usize) -> Option<&'w [u8]> {
        self.get(index).and_then(InvocationWord::native_bytes)
    }

    /// Proved array-root shape at one final argv position, excluding uncertain expansion offsets.
    #[must_use]
    pub fn array_element_root_at(self, index: usize) -> Option<&'w str> {
        match self.argv_at(index) {
            InvocationArgument::Word(InvocationWord::ArrayElementName { root }) => Some(root),
            _ => None,
        }
    }

    /// Whether every source word has a known literal Tcl value.
    #[must_use]
    pub fn are_all_literals(self) -> bool {
        match self {
            Self::Literals(_) | Self::ContextualLiterals(_, _) => true,
            Self::Structured(words) | Self::ContextualStructured(words, _) => words
                .iter()
                .all(|word| matches!(word, InvocationWord::Literal(_))),
        }
    }

    /// Whether every source word is guaranteed to contribute one argv entry.
    #[must_use]
    pub fn has_exact_argv_len(self) -> bool {
        match self {
            Self::Literals(_) | Self::ContextualLiterals(_, _) => true,
            Self::Structured(words) | Self::ContextualStructured(words, _) => {
                words.iter().all(|word| word.has_exactly_one_argv_entry())
            }
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

    /// Whether this view retained source word shape rather than accepting the
    /// legacy string-only compatibility projection.
    #[must_use]
    pub const fn is_source_aware(self) -> bool {
        matches!(self, Self::Structured(_) | Self::ContextualStructured(_, _))
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
            Self::Literals(words) | Self::ContextualLiterals(words, _) => Some(words.to_vec()),
            Self::Structured(words) | Self::ContextualStructured(words, _) => {
                words.iter().map(|word| word.literal()).collect()
            }
        }
    }

    /// Borrow the arguments from `start` onwards while preserving their
    /// source-knowledge representation.
    #[must_use]
    pub fn slice_from(self, start: usize) -> Self {
        match self {
            Self::Literals(words) => Self::Literals(words.get(start..).unwrap_or(&[])),
            Self::Structured(words) => Self::Structured(words.get(start..).unwrap_or(&[])),
            Self::ContextualLiterals(words, dialect) => {
                Self::ContextualLiterals(words.get(start..).unwrap_or(&[]), dialect)
            }
            Self::ContextualStructured(words, dialect) => {
                Self::ContextualStructured(words.get(start..).unwrap_or(&[]), dialect)
            }
        }
    }
}

/// Inputs supplied to a registry command-prefix resolver.
///
/// Optional `spellings` retain an explicitly supplied compatibility view.
/// `words` is the source-aware truth used by any resolver whose
/// appended arity depends on a literal argument value. Such a resolver must
/// use [`Self::literal_at`] and abstain when it returns `None`, never interpret
/// a dynamic word's source spelling as its runtime value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandPrefixArguments<'w> {
    spellings: Option<&'w [&'w str]>,
    words: InvocationArguments<'w>,
}

impl<'w> CommandPrefixArguments<'w> {
    /// Construct a compatibility view in which every argument is literal.
    #[must_use]
    pub const fn literals(spellings: &'w [&'w str]) -> Self {
        Self {
            spellings: Some(spellings),
            words: InvocationArguments::literals(spellings),
        }
    }

    /// Construct a source-aware view alongside reconstructed source
    /// spellings. Both slices must describe the same post-head words.
    #[must_use]
    pub fn structured(spellings: &'w [&'w str], words: &'w [InvocationWord<'w>]) -> Option<Self> {
        (spellings.len() == words.len()).then_some(Self {
            spellings: Some(spellings),
            words: InvocationArguments::structured(words),
        })
    }

    /// Retain actual argument knowledge without reconstructing source spellings.
    #[must_use]
    pub const fn from_invocation_arguments(words: InvocationArguments<'w>) -> Self {
        Self {
            spellings: None,
            words,
        }
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

    /// Explicit compatibility spellings, when the caller supplied them.
    /// Actual argument-only ingress returns `None`; unknown values are never
    /// reconstructed as empty strings. Do not use spellings for value decisions.
    #[must_use]
    pub const fn spellings(self) -> Option<&'w [&'w str]> {
        self.spellings
    }

    /// Return an argument value only when source analysis proves it literal.
    #[must_use]
    pub fn literal_at(self, index: usize) -> Option<&'w str> {
        self.words.literal_at(index)
    }

    /// The source-aware view: values where proven literal, and the
    /// dynamic / expanded / opaque classification everywhere else. A resolver
    /// that must report *why* a word is not usable — the `SpecTcl` hook host
    /// builds the DSL's `kinds` from it — reads this rather than inferring
    /// "not literal" from [`Self::literal_at`] returning `None`.
    #[must_use]
    pub const fn words(self) -> InvocationArguments<'w> {
        self.words
    }

    /// Borrow the arguments from `start` onwards, preserving both views.
    #[must_use]
    pub fn slice_from(self, start: usize) -> Self {
        Self {
            spellings: self
                .spellings
                .map(|words| words.get(start..).unwrap_or(&[])),
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

/// Registry-owned projection of the variable cells an invocation may write.
///
/// Literal names are effective Tcl cell names: an unqualified option value
/// whose registry descriptor declares [`VariableScope::Global`](crate::VariableScope)
/// is rooted with `::`. They remain useful even when another write target is
/// source-opaque. [`Self::opaque_variable_frame`] means substitution,
/// expansion, or a computed command head prevents the registry from
/// enumerating every possible target.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VariableWriteProjection {
    /// Statically known literal variable names written by the invocation.
    pub literal_names: Vec<String>,
    /// The subset of [`Self::literal_names`] the invocation **reads before
    /// writing** ([`Traits::READS_BEFORE_WRITE`](crate::Traits::READS_BEFORE_WRITE)
    /// — `incr` / `append` / `lappend` / `lset` / `lpop` / `ledit`). An
    /// effect consumer needs the read as much as the write: the store that
    /// feeds `[incr n]` is observed by it, not overwritten by it.
    pub read_before_write_names: Vec<String>,
    /// Whether the invocation may write another, unnameable variable cell.
    pub opaque_variable_frame: bool,
}

/// Registry-owned projection of the variable cells an invocation reads **by
/// name** — the read-only counterpart of [`VariableWriteProjection`].
///
/// An [`ArgRole::VarRead`](crate::ArgRole::VarRead) word names a cell the
/// command observes without storing to it: `info exists n`, `array exists a`,
/// `array size a`. The store feeding such a name is observed, not
/// overwritten, so an effect consumer that records only writes reports that
/// store dead (#2132).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VariableReadProjection {
    /// Statically known literal variable names read by the invocation.
    pub literal_names: Vec<String>,
    /// Whether the invocation may also read a cell that cannot be named — a
    /// substituted or `{*}`-expanded name word.
    pub opaque_variable_frame: bool,
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

    /// Construct an invocation while retaining the argument view's exact
    /// source knowledge and execution policies.
    #[must_use]
    pub const fn from_arguments(
        head: InvocationWord<'w>,
        arguments: InvocationArguments<'w>,
    ) -> Self {
        Self { head, arguments }
    }

    /// Stamp explicitly resolved execution policies on post-head words.
    #[must_use]
    pub const fn with_dialect(mut self, dialect: InvocationDialect) -> Self {
        self.arguments = self.arguments.with_dialect(dialect);
        self
    }

    /// Stamp execution policies on the post-head words.
    #[must_use]
    pub fn with_profile(mut self, profile: Option<&tcl_dialect::DialectProfile>) -> Self {
        self.arguments = self.arguments.with_profile(profile);
        self
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

    /// Borrow the selected argv and its dialect snapshot while recursively
    /// interpreting native bodies. The owning invocation outlives the borrow.
    #[must_use]
    pub const fn arguments_ref(&self) -> &InvocationArguments<'w> {
        &self.arguments
    }
}

#[cfg(test)]
mod tests;
