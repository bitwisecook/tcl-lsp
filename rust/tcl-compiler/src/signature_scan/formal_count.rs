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
    /// Unanimous descriptive count from a current Logical declaration header.
    LogicalSource,
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
    pub(crate) fn from_logical_formals(
        text: &str,
        parameters: &[tcl_syntax::formal_params::FormalParameter],
    ) -> Self {
        if !logical_formals_agree(text, parameters) {
            return SourceFormalCount::unknown_projection();
        }
        let shape = |grammar| {
            tcl_syntax::formal_params::formal_argument_count_shape(
                parameters
                    .iter()
                    .map(|parameter| (parameter.name == "args", parameter.default.is_some())),
                grammar,
            )
        };
        let c_shape = shape(ParameterGrammar::Tcl);
        let jim_shape = shape(ParameterGrammar::Jim);
        if c_shape != jim_shape {
            return SourceFormalCount::unknown_projection();
        }
        let Some(arity) = super::arity::arity_from_count_shape(c_shape) else {
            return SourceFormalCount::unknown_projection();
        };
        Self {
            arity,
            origin: SourceFormalCountOrigin::LogicalSource,
            grammar: None,
        }
    }

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

fn logical_formals_agree(
    text: &str,
    parameters: &[tcl_syntax::formal_params::FormalParameter],
) -> bool {
    !text.as_bytes().contains(&0)
        && !parameters.iter().any(|parameter| {
            parameter.name.as_bytes().contains(&0)
                || parameter
                    .default
                    .as_ref()
                    .is_some_and(|value| value.as_bytes().contains(&0))
        })
        && [ParameterGrammar::Tcl, ParameterGrammar::Jim]
            .into_iter()
            .all(|grammar| {
                tcl_syntax::formal_params::parse_formal_parameters_in(text, grammar)
                    .is_ok_and(|parsed| parsed == parameters)
            })
}

/// Conditional source-role ordinals agreed by both formal grammars. This does
/// not select an engine or activate a formal/caller link.
pub(crate) fn logical_formal_argument_bindings(
    text: &str,
    parameters: &[tcl_syntax::formal_params::FormalParameter],
    argument_count: usize,
) -> Option<Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>> {
    use tcl_syntax::formal_params::{ByteFormalParameter, bind_formal_argument_bytes};
    logical_formals_agree(text, parameters).then_some(())?;
    let mut names = std::collections::HashSet::new();
    parameters
        .iter()
        .all(|parameter| names.insert(&parameter.name))
        .then_some(())?;
    let parameters = parameters
        .iter()
        .map(|parameter| ByteFormalParameter {
            name: parameter.name.as_bytes().to_vec(),
            default: parameter
                .default
                .as_ref()
                .map(|value| value.as_bytes().to_vec()),
        })
        .collect::<Vec<_>>();
    let tcl =
        bind_formal_argument_bytes(&parameters, argument_count, ParameterGrammar::Tcl).ok()?;
    let jim =
        bind_formal_argument_bytes(&parameters, argument_count, ParameterGrammar::Jim).ok()?;
    (tcl == jim).then_some(tcl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::formal_params::parse_formal_parameters_in;

    #[test]
    fn logical_source_role_ordinals_require_exact_dual_grammar_agreement() {
        for (text, count, expected) in [
            ("value {optional DEFAULT} args", 1, true),
            ("value {optional DEFAULT} args", 4, true),
            ("{optional DEFAULT} required", 1, false),
            ("&value", 1, false),
            ("value value", 2, false),
        ] {
            let parameters = parse_formal_parameters_in(text, ParameterGrammar::Tcl).unwrap();
            assert_eq!(
                logical_formal_argument_bindings(text, &parameters, count).is_some(),
                expected,
                "{text}"
            );
        }
        let parameters = parse_formal_parameters_in("value", ParameterGrammar::Tcl).unwrap();
        assert!(logical_formal_argument_bindings("different", &parameters, 1).is_none());
        assert!(logical_formal_argument_bindings("value", &parameters, 0).is_none());
    }
}
