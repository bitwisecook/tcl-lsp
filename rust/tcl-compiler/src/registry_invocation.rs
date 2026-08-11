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

//! Shared compiler adapter from semantic source words to registry facts.
//!
//! The registry must only receive a Tcl word's evaluated value when the
//! semantic IR proves it literal. This module centralises that conversion for
//! every common compiler pass: executable IR, memory SSA, and later shared
//! analyses all resolve exactly the same structured [`WordExpr`] facts under
//! an explicitly supplied dialect profile.

use tcl_registry::dialects::DialectSet;
use tcl_registry::{
    CommandRegistry, InvocationFacts, InvocationResolutionUnresolved, InvocationWord,
    InvocationWordKind, InvocationWords,
};

use crate::ir::{CommandTokens, WordExpr};

/// An owned, target-neutral result from registry invocation resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryInvocationResolution {
    /// The registry selected complete common facts for the invocation.
    Resolved(Box<InvocationFacts>),
    /// The command head could not safely select a registry descriptor.
    Unresolved(OwnedInvocationResolutionUnresolved),
}

/// A typed unresolved command-head outcome retained across the registry's
/// borrowing boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedInvocationResolutionUnresolved {
    /// The command head was substituted, expanded, or opaque.
    ComputedHead {
        /// Source-word knowledge that prevented registry lookup.
        word_kind: InvocationWordKind,
    },
    /// A literal head was absent for the explicitly selected dialect profile.
    UnknownLiteralHead {
        /// Literal source spelling consulted in the registry.
        spelling: String,
    },
}

impl OwnedInvocationResolutionUnresolved {
    fn from_registry(unresolved: InvocationResolutionUnresolved<'_>) -> Self {
        match unresolved {
            InvocationResolutionUnresolved::ComputedHead { word_kind } => {
                Self::ComputedHead { word_kind }
            }
            InvocationResolutionUnresolved::UnknownLiteralHead { spelling } => {
                Self::UnknownLiteralHead {
                    spelling: spelling.to_owned(),
                }
            }
        }
    }
}

/// Why no registry invocation outcome could be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryInvocationDecline {
    /// The semantic word sequence contained no command head.
    MissingCommandHead,
    /// The registry violated its structured-resolution contract by producing
    /// neither a resolved invocation nor an unresolved reason.
    IncompleteResolution,
}

/// Convert one compiler source word to the registry's value-conservative
/// invocation vocabulary.
///
/// Literal and braced-literal words expose their Tcl value. Substitutions and
/// compound templates contribute exactly one argv word but never expose their
/// source spelling as a value. Expansion keeps only its argv-shape fact, and
/// compatibility recovery remains opaque.
#[must_use]
pub fn invocation_word(word: &WordExpr) -> InvocationWord<'_> {
    match word {
        WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } => {
            InvocationWord::Literal(text)
        }
        WordExpr::Variable { .. }
        | WordExpr::CommandSubstitution { .. }
        | WordExpr::Template { .. } => InvocationWord::Dynamic,
        WordExpr::Expand { .. } => InvocationWord::Expanded,
        WordExpr::Opaque { .. } => InvocationWord::Opaque,
    }
}

/// Resolve semantic source words through the registry under `dialect`.
///
/// The borrowed registry resolution is materialised immediately into owned
/// facts or an owned typed unresolved outcome, making this suitable for
/// common IR and analysis state.
pub fn resolve_word_exprs(
    registry: &CommandRegistry,
    dialect: DialectSet,
    words: &[WordExpr],
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    let Some((head, arguments)) = words.split_first() else {
        return Err(RegistryInvocationDecline::MissingCommandHead);
    };
    let arguments: Vec<_> = arguments.iter().map(invocation_word).collect();
    let resolution = registry.resolve_structured_invocation(
        InvocationWords::structured(invocation_word(head), &arguments),
        dialect,
    );
    if let Some(resolved) = resolution.resolved() {
        return Ok(RegistryInvocationResolution::Resolved(Box::new(
            resolved.facts(),
        )));
    }
    let Some(unresolved) = resolution.unresolved() else {
        return Err(RegistryInvocationDecline::IncompleteResolution);
    };
    Ok(RegistryInvocationResolution::Unresolved(
        OwnedInvocationResolutionUnresolved::from_registry(unresolved),
    ))
}

/// Resolve a source command-token snapshot through the registry.
///
/// [`CommandTokens::words`] is the compiler's canonical ordered source-word
/// representation. This adapter never falls back to the lossy argv text
/// arrays, so dynamic words cannot become accidental literal command or
/// subcommand values.
pub fn resolve_command_tokens(
    registry: &CommandRegistry,
    dialect: DialectSet,
    tokens: &CommandTokens,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_word_exprs(registry, dialect, tokens.words())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::{StateTransition, TransitionSubject};

    use crate::ir::{SourceSite, WordOpacity};

    fn literal(text: &str) -> WordExpr {
        WordExpr::Literal {
            text: text.to_owned(),
            source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
        }
    }

    #[test]
    fn literal_words_materialise_registry_transition_facts() {
        let words = [literal("global"), literal("::shared")];
        let resolution =
            resolve_word_exprs(&CommandRegistry::build_default(), DialectSet::TCL86, &words)
                .expect("literal command has a registry outcome");

        let RegistryInvocationResolution::Resolved(facts) = resolution else {
            panic!("literal global must resolve");
        };
        let transitions = facts
            .state_transitions
            .declared()
            .expect("global declares transition facts");
        assert!(matches!(
            transitions.facts(),
            [fact]
                if matches!(
                    &fact.transition,
                    StateTransition::VariableCellAlias(alias)
                        if alias.local == TransitionSubject::Literal("shared".to_owned())
                )
        ));
    }

    #[test]
    fn dynamic_and_expanded_words_never_become_literal_registry_values() {
        let dynamic = [
            literal("global"),
            WordExpr::Variable {
                spelling: "$name".to_owned(),
                source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
            },
        ];
        let RegistryInvocationResolution::Resolved(dynamic_facts) = resolve_word_exprs(
            &CommandRegistry::build_default(),
            DialectSet::TCL86,
            &dynamic,
        )
        .expect("dynamic global has a registry outcome") else {
            panic!("literal global head resolves");
        };
        assert!(
            dynamic_facts
                .state_transitions
                .declared()
                .expect("global declares transition facts")
                .facts()
                .iter()
                .any(|fact| matches!(fact.transition, StateTransition::Widen(_)))
        );

        let expanded = [
            literal("proc"),
            literal("p"),
            WordExpr::Expand {
                source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
                word: Box::new(WordExpr::Opaque {
                    text: "args".to_owned(),
                    source: SourceSite::opaque(tcl_lexer::Span::new(0, 0)),
                    reason: WordOpacity::LossySnapshot,
                }),
            },
            literal("body"),
        ];
        let RegistryInvocationResolution::Resolved(expanded_facts) = resolve_word_exprs(
            &CommandRegistry::build_default(),
            DialectSet::TCL86,
            &expanded,
        )
        .expect("expanded proc has a registry outcome") else {
            panic!("literal proc head resolves");
        };
        assert!(
            expanded_facts
                .state_transitions
                .declared()
                .expect("proc declares transition facts")
                .facts()
                .iter()
                .all(|fact| !matches!(fact.transition, StateTransition::CommandBinding(_)))
        );
    }

    #[test]
    fn dynamic_head_stays_typed_unresolved() {
        let words = [WordExpr::Variable {
            spelling: "$command".to_owned(),
            source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
        }];
        assert!(matches!(
            resolve_word_exprs(&CommandRegistry::build_default(), DialectSet::TCL86, &words,),
            Ok(RegistryInvocationResolution::Unresolved(
                OwnedInvocationResolutionUnresolved::ComputedHead {
                    word_kind: InvocationWordKind::Dynamic,
                }
            ))
        ));
    }
}
