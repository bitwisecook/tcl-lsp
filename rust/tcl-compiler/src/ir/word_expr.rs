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

//! The lexical word expression the semantic-IR compatibility layer retains.

use super::provenance::SourceSite;
use tcl_lexer::Span;

/// A lexical word expression retained by the semantic-IR compatibility layer.
///
/// This preserves ordering and source ranges for substitutions that the lexer
/// identifies.  It deliberately does **not** claim to fully model Tcl's
/// backslash substitution or all parser recovery forms: those remain
/// [`Self::Opaque`] until a later executable lowering can give them an exact
/// evaluation model.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WordExpr {
    /// A single bare word with no substitutions or backslash processing.
    Literal {
        /// Compatibility argv spelling.
        text: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A single braced word, whose content undergoes no substitution.
    BracedLiteral {
        /// De-braced word content.
        text: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A single variable substitution word.
    Variable {
        /// Verbatim source spelling, including its Tcl variable wrapper.
        spelling: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A single command-substitution word.
    CommandSubstitution {
        /// Compatibility argv spelling, including `[` and `]` where present.
        spelling: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A compound word whose parts are evaluated left-to-right.
    Template {
        /// Ordered lexical parts.
        parts: Vec<WordPart>,
        /// Full source range of the word.
        source: SourceSite,
        /// C's message where its parser rejects the word for what follows
        /// a closing brace or quote (`{a}b`, `"a"b`, `{*}$x` under 8.4),
        /// which the parts still model leniently so the analyser reads
        /// them; a consumer that must not run past a parse error reads it.
        rejected: Option<&'static str>,
    },
    /// Tcl 8.5+ argument expansion around another word expression.
    Expand {
        /// Source range from the `{*}` marker through the inner word.
        source: SourceSite,
        /// Expanded word.
        word: Box<WordExpr>,
    },
    /// A word whose exact substitution behaviour is not represented by the
    /// legacy token snapshot.
    Opaque {
        /// Compatibility argv text.
        text: String,
        /// Best available source site.
        source: SourceSite,
        /// Why the semantic layer must not infer stronger meaning yet.
        reason: WordOpacity,
    },
}

impl WordExpr {
    /// A sole variable reference, including the reference inside quotes.
    /// The returned site belongs to the read, rather than its enclosing word;
    /// compound, expanded and opaque words provide no single read identity.
    #[must_use]
    pub fn sole_variable_substitution(&self) -> Option<(&str, &SourceSite)> {
        match self {
            Self::Variable { spelling, source } => Some((spelling, source)),
            Self::Template { parts, .. } => match parts.as_slice() {
                [WordPart::Variable { spelling, source }] => Some((spelling, source)),
                _ => None,
            },
            _ => None,
        }
    }

    /// A sole command substitution retains the command result object, including
    /// when quotes surround it. Compound words concatenate their values.
    #[must_use]
    pub fn sole_command_substitution(&self) -> Option<(&str, &SourceSite)> {
        match self {
            Self::CommandSubstitution { spelling, source } => Some((spelling, source)),
            Self::Template { parts, .. } => match parts.as_slice() {
                [WordPart::CommandSubstitution { spelling, source }] => Some((spelling, source)),
                _ => None,
            },
            _ => None,
        }
    }

    /// Return this word's source site.
    #[must_use]
    pub const fn source(&self) -> &SourceSite {
        match self {
            Self::Literal { source, .. }
            | Self::BracedLiteral { source, .. }
            | Self::Variable { source, .. }
            | Self::CommandSubstitution { source, .. }
            | Self::Template { source, .. }
            | Self::Expand { source, .. }
            | Self::Opaque { source, .. } => source,
        }
    }

    /// Recover the compatibility argv text used by existing consumers.
    #[must_use]
    pub fn legacy_text(&self) -> String {
        match self {
            Self::Literal { text, .. }
            | Self::BracedLiteral { text, .. }
            | Self::Opaque { text, .. } => text.clone(),
            Self::Variable { spelling, .. } => {
                crate::word_expr::compatibility_variable_spelling(spelling)
            }
            Self::CommandSubstitution { spelling, .. } => spelling.clone(),
            Self::Template { parts, .. } => parts.iter().map(WordPart::legacy_text).collect(),
            Self::Expand { word, .. } => word.legacy_text(),
        }
    }

    pub(super) fn from_lossy_snapshot(
        text: &str,
        span: Span,
        kind: tcl_lexer::TokenType,
        single_token: bool,
        expanded: bool,
    ) -> Self {
        let source = SourceSite::opaque(span);
        let word = if single_token {
            match kind {
                tcl_lexer::TokenType::Str => Self::BracedLiteral {
                    text: text.to_owned(),
                    source,
                },
                tcl_lexer::TokenType::Var => Self::Variable {
                    spelling: text.to_owned(),
                    source,
                },
                tcl_lexer::TokenType::Cmd => Self::CommandSubstitution {
                    spelling: text.to_owned(),
                    source,
                },
                _ => Self::Opaque {
                    text: text.to_owned(),
                    source,
                    reason: WordOpacity::LossySnapshot,
                },
            }
        } else {
            Self::Opaque {
                text: text.to_owned(),
                source,
                reason: WordOpacity::LossySnapshot,
            }
        };
        if expanded {
            Self::Expand {
                source: SourceSite::opaque(span),
                word: Box::new(word),
            }
        } else {
            word
        }
    }
}

/// One ordered lexical component of a [`WordExpr::Template`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WordPart {
    /// A plain or escaped text fragment. Backslash semantics remain explicit
    /// but unevaluated at this compatibility boundary.
    Text {
        /// Compatibility argv spelling.
        text: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A variable substitution.
    Variable {
        /// Verbatim source spelling of this individual reference.
        spelling: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A command substitution.
    CommandSubstitution {
        /// Compatibility argv spelling.
        spelling: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A recovery or otherwise unmodelled token fragment.
    Opaque {
        /// Compatibility argv spelling.
        text: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
}

impl WordPart {
    pub(super) fn legacy_text(&self) -> String {
        match self {
            Self::Text { text, .. } | Self::Opaque { text, .. } => text.clone(),
            Self::Variable { spelling, .. } => {
                crate::word_expr::compatibility_variable_spelling(spelling)
            }
            Self::CommandSubstitution { spelling, .. } => spelling.clone(),
        }
    }
}

/// Why a [`WordExpr::Opaque`] node cannot make stronger semantic claims yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WordOpacity {
    /// The compatibility snapshot lacked ordered lexical fragments.
    MissingFragments,
    /// The snapshot was synthetically constructed or retained only its legacy
    /// representative-token fields.
    LossySnapshot,
    /// C Tcl's parser rejects the word; the payload is its exact message
    /// (`missing close-bracket`, `missing )`, …) for diagnostics.
    ParseError(&'static str),
    /// The word carries a dialect substitution the word-component owner does
    /// not model — today only `JimTcl`'s `$(expr)` (`TokenType::ExprSugar`).
    ///
    /// `word_parts::decompose` applies Tcl's `$` rules, under which `$(` is
    /// ordinary text, so such a word would otherwise decompose to a single
    /// `Text` part and be promoted to [`WordExpr::Literal`] — and lowering
    /// would emit the *spelling* `$(1+2)` instead of evaluating it. The word
    /// is opaque instead.
    DialectSubstitution,
}
