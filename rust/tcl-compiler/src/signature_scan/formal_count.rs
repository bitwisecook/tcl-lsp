// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retained formal count contracts and body-free descriptive projections.

use super::formal_parameters::SignatureSourceFormalParameters;
use super::scope::SignatureSourceNameKey;
use super::types::ParamDef;
use std::sync::Arc;
use tcl_dialect::ParameterGrammar;
use tcl_registry::{Arity, InvocationDialect};

/// The source of a declaration's count contract. Original formal bytes retain
/// their selected grammar; authored metadata supplies no original/native issuer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFormalCount {
    /// No complete static parameter contract is available.
    Unknown,
    /// Authentic ordered formal storage from an original parameter-list producer.
    Original(Arc<SignatureSourceFormalParameters>),
    /// Explicit descriptive declaration metadata under its authored grammar.
    Authored(ParameterGrammar),
}

/// Reporting provenance of body-free count metadata, never a source capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormalCountOrigin {
    /// No count restriction can be asserted.
    Unknown,
    /// Projected from an authentic original formal topology.
    OriginalSource,
    /// Projected from explicit authored declaration metadata.
    Authored,
}

/// Stable declaration header metadata, independent of source-image/body bytes.
/// This cannot recreate formals, bind an argv, select a command or enter a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceFormalCountProjection {
    arity: Arity,
    origin: SourceFormalCountOrigin,
    grammar: Option<ParameterGrammar>,
}

impl SourceFormalCountProjection {
    /// Inclusive descriptive count bounds; unknown contracts remain open.
    #[must_use]
    pub const fn arity(self) -> Arity {
        self.arity
    }
    /// The retained count producer's reporting classification.
    #[must_use]
    pub const fn origin(self) -> SourceFormalCountOrigin {
        self.origin
    }
    /// Independently selected or explicitly authored formal grammar.
    #[must_use]
    pub const fn parameter_grammar(self) -> Option<ParameterGrammar> {
        self.grammar
    }
}

impl SourceFormalCount {
    /// Retain actual source formal storage under the independently selected
    /// grammar. Invalid lists and incompatible naming purposes remain unknown.
    #[must_use]
    pub fn from_original_input(input: &SignatureSourceNameKey, dialect: InvocationDialect) -> Self {
        SignatureSourceFormalParameters::from_original_input(input, dialect)
            .map_or(Self::Unknown, |formals| Self::Original(Arc::new(formals)))
    }

    /// One genuine original lexical parameter-list word, without a reporting
    /// string conversion or a declaration/activation grant.
    #[must_use]
    pub fn from_original_word(word: &tcl_lexer::NativeWord, dialect: InvocationDialect) -> Self {
        let Some(policy) = dialect.authored_name_policy() else {
            return Self::Unknown;
        };
        let Some(input) = SignatureSourceNameKey::from_original_native_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&word.config()),
            policy,
        ) else {
            return Self::Unknown;
        };
        Self::from_original_input(&input, dialect)
    }

    /// Project only count metadata for a declaration header. Original contracts
    /// never count display names; authored metadata remains a distinct purpose.
    #[must_use]
    pub fn projection(&self, params: &[ParamDef], computed: bool) -> SourceFormalCountProjection {
        if computed {
            return Self::unknown_projection();
        }
        let (arity, origin, grammar) = match self {
            Self::Unknown => return Self::unknown_projection(),
            Self::Original(formals) => (
                super::arity::arity_from_count_shape(formals.argument_count_shape()),
                SourceFormalCountOrigin::OriginalSource,
                formals.parameter_grammar(),
            ),
            Self::Authored(grammar) => (
                Some(super::arity::arity_of_in(params, *grammar)),
                SourceFormalCountOrigin::Authored,
                *grammar,
            ),
        };
        let Some(arity) = arity else {
            return Self::unknown_projection();
        };
        SourceFormalCountProjection {
            arity,
            origin,
            grammar: Some(grammar),
        }
    }

    fn unknown_projection() -> SourceFormalCountProjection {
        SourceFormalCountProjection {
            arity: Arity::any(),
            origin: SourceFormalCountOrigin::Unknown,
            grammar: None,
        }
    }
}
