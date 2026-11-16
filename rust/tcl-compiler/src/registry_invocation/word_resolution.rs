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

//! Effective invocation words and their resolution to a registry invocation in a realm.

use super::effective_words::{effective_command_words, frozen_argument_words};
use super::metadata_context::InvocationMetadataContext;
use super::{AuthoredSourceCaseBodySelection, EffectiveCommandWords};
use crate::ir::{CommandTokens, WordExpr, WordPart};
use crate::segmenter::SegmentedCommand;
use tcl_dialect::EscapeSyntax;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{
    CommandRegistry, InvocationFacts, InvocationResolutionUnresolved, InvocationWord,
    InvocationWordKind, InvocationWords,
};
use tcl_syntax::word_rules::WordValueRules;

/// Owned source-aware word fact for callers that must decode a static Tcl
/// word before lending it to the registry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EffectiveInvocationWord {
    /// A Tcl value proven static after applying the active escape syntax.
    Literal(String),
    /// A decoded native string whose exact bytes are not Unicode.
    /// This remains one argv operand and grants no text, name or object proof.
    ByteLiteral(std::sync::Arc<[u8]>),
    /// One argv word produced by substitution.
    Dynamic,
    /// A computed variable name whose evaluated root and array form are proved.
    ArrayElementName {
        /// Root before the first opening parenthesis; the element remains unknown.
        root: String,
    },
    /// An expansion with an unknown resulting argv length.
    Expanded,
    /// Native list elements retained at a written expansion evaluation.
    /// This grants cardinality and bytes, never object-representation proof.
    KnownExpansion(Vec<String>),
    /// Native list elements whose payloads cannot all be represented as Unicode.
    /// Each element remains one argv operand, with no object or source-word grant.
    KnownByteExpansion(Vec<std::sync::Arc<[u8]>>),
    /// A compatibility/recovery word whose source meaning is unavailable.
    Opaque,
}

/// Whether a source word may name a compiled local-variable slot.
///
/// This is deliberately stricter than [`EffectiveInvocationWord`]: a bare
/// word with backslash processing has a static *value*, but C Tcl still emits
/// the stack form for it and does not intern that value in the LVT.  Braced
/// and quoted/plain text words whose spelling needs no backslash processing
/// retain the direct-name form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledLocalNameWord {
    /// The source form is eligible for a direct local-name opcode.
    Direct,
    /// The word must be evaluated or decoded before it can name a variable.
    Stack,
}

/// Classify one source word for bytecode local-name selection.
///
/// The word-parts owner has already retained substitution and raw backslash
/// facts in [`WordExpr`].  Consumers must ask this adapter rather than infer
/// source provenance from compatibility argv text.  In particular, `{p\\x}`
/// is direct (the backslash is part of the name), while bare `p\\x` is stack
/// (the backslash is decoded before lookup).
#[must_use]
pub fn compiled_local_name_word(word: &WordExpr) -> CompiledLocalNameWord {
    match word {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => CompiledLocalNameWord::Direct,
        WordExpr::Template { parts, .. }
            if parts.iter().all(
                |part| matches!(part, WordPart::Text { text, .. } if !text.contains('\\')),
            ) =>
        {
            // A quoted literal reaches this arm: grouping quotes are outside
            // its text parts, while a quoted backslash remains in one.
            CompiledLocalNameWord::Direct
        }
        WordExpr::Variable { .. }
        | WordExpr::CommandSubstitution { .. }
        | WordExpr::Template { .. }
        | WordExpr::Expand { .. }
        | WordExpr::Opaque { .. } => CompiledLocalNameWord::Stack,
    }
}

/// Return the evaluated name for a source word eligible for a compiled local
/// opcode. The same source owner supplies both the eligibility decision and
/// the literal value so brace grouping cannot reach an opcode as data.
#[must_use]
pub fn compiled_local_name_value(
    word: &WordExpr,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> Option<String> {
    if compiled_local_name_word(word) != CompiledLocalNameWord::Direct {
        return None;
    }
    match effective_invocation_word(word, escapes, word_rules) {
        EffectiveInvocationWord::Literal(value) => Some(value),
        EffectiveInvocationWord::ByteLiteral(_)
        | EffectiveInvocationWord::Dynamic
        | EffectiveInvocationWord::ArrayElementName { .. }
        | EffectiveInvocationWord::Expanded
        | EffectiveInvocationWord::KnownExpansion(_)
        | EffectiveInvocationWord::KnownByteExpansion(_)
        | EffectiveInvocationWord::Opaque => None,
    }
}

impl EffectiveInvocationWord {
    /// Cardinality retained by a proved native list expansion.
    #[must_use]
    pub fn expansion_len(&self) -> Option<usize> {
        match self {
            Self::KnownExpansion(values) => Some(values.len()),
            Self::KnownByteExpansion(values) => Some(values.len()),
            _ => None,
        }
    }

    /// Exact native bytes of one proved expansion child.
    #[must_use]
    pub fn expansion_element(&self, ordinal: usize) -> Option<&[u8]> {
        match self {
            Self::KnownExpansion(values) => values.get(ordinal).map(String::as_bytes),
            Self::KnownByteExpansion(values) => values.get(ordinal).map(AsRef::as_ref),
            _ => None,
        }
    }

    /// Retain decoded native bytes with a checked Unicode projection.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        std::str::from_utf8(bytes).map_or_else(
            |_| Self::ByteLiteral(std::sync::Arc::from(bytes)),
            |text| Self::Literal(text.to_owned()),
        )
    }

    /// Exact payload when this word has a known native string value.
    #[must_use]
    pub fn literal_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Literal(text) => Some(text.as_bytes()),
            Self::ByteLiteral(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// Lend this owned fact to the registry's allocation-free vocabulary.
    #[must_use]
    pub fn as_registry_word(&self) -> InvocationWord<'_> {
        match self {
            Self::Literal(value) => InvocationWord::Literal(value),
            Self::ByteLiteral(value) => InvocationWord::KnownBytes(value),
            Self::Dynamic => InvocationWord::Dynamic,
            Self::ArrayElementName { root } => InvocationWord::ArrayElementName { root },
            Self::Expanded | Self::KnownExpansion(_) | Self::KnownByteExpansion(_) => {
                InvocationWord::Expanded
            }
            Self::Opaque => InvocationWord::Opaque,
        }
    }
}

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
    /// Point-specific lookup cannot prove one live registry implementation.
    UnprovedBinding {
        /// Original command spelling, retained for presentation only.
        spelling: String,
        /// At least one represented execution path may have no command.
        may_be_absent: bool,
        /// Some execution path has no bounded target implementation.
        unknown: bool,
    },
    /// A selected semantic implementation requires an original private slot
    /// and dispatch mapping whose live identity is not established.
    UnprovedNativeImplementationLookup {
        /// Exact registry-authored lookup; catalogue presence is insufficient.
        lookup: tcl_registry::native_compilation::NativeCompilerImplementationLookup,
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

/// Preserve a proved non-option prefix without exposing a substituted value.
/// Known evaluated bytes take precedence; template prefix bytes are decoded
/// under the original ingress escape grammar.
#[must_use]
pub fn invocation_word_with_source<'a>(
    source: &WordExpr,
    effective: &'a EffectiveInvocationWord,
    escapes: EscapeSyntax,
) -> InvocationWord<'a> {
    if matches!(effective, EffectiveInvocationWord::Dynamic)
        && let WordExpr::Template { parts, .. } = source
    {
        let prefix: String = parts
            .iter()
            .take_while(|part| matches!(part, WordPart::Text { .. }))
            .filter_map(|part| match part {
                WordPart::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        if tcl_syntax::backslash::decode_bytes_in(prefix.as_bytes(), escapes)
            .first()
            .is_some_and(|first| *first != b'-')
        {
            return InvocationWord::DynamicNonOption;
        }
    }
    effective.as_registry_word()
}

/// Evaluate the statically-known portion of a source word under the active
/// escape grammar. Braced words keep their backslashes except for the
/// dialect-defined line-continuation collapse.
#[must_use]
pub fn effective_invocation_word(
    word: &WordExpr,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> EffectiveInvocationWord {
    match word {
        WordExpr::Literal { text, .. } => EffectiveInvocationWord::Literal(text.clone()),
        WordExpr::BracedLiteral { text, .. } => {
            EffectiveInvocationWord::Literal(word_rules.collapse_braced_word(text).into_owned())
        }
        WordExpr::Template { parts, .. }
            if parts
                .iter()
                .all(|part| matches!(part, WordPart::Text { .. })) =>
        {
            let raw: String = parts
                .iter()
                .map(|part| match part {
                    WordPart::Text { text, .. } => text.as_str(),
                    _ => unreachable!("guard admits text parts only"),
                })
                .collect();
            EffectiveInvocationWord::from_bytes(&tcl_syntax::backslash::decode_bytes_in(
                raw.as_bytes(),
                escapes,
            ))
        }
        WordExpr::Template { .. } => EffectiveInvocationWord::Dynamic,
        WordExpr::Variable { .. } | WordExpr::CommandSubstitution { .. } => {
            EffectiveInvocationWord::Dynamic
        }
        WordExpr::Expand { .. } => EffectiveInvocationWord::Expanded,
        WordExpr::Opaque { .. } => EffectiveInvocationWord::Opaque,
    }
}

/// Partial variable-operand shape from one actual original C compiler word.
/// This retains the independently selected recipe and complete source grammar;
/// it supplies neither an evaluated whole value nor a variable handler or cell.
pub(crate) fn original_array_element_word(
    original: &tcl_lexer::NativeWord,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Option<EffectiveInvocationWord> {
    let tcl_syntax::naming::NativeNameProtocol::C(version) = policy.recipe() else {
        return None;
    };
    let tcl_syntax::native_variable_words::NativeVariableWordOperand::CompoundArray {
        name, ..
    } = tcl_syntax::native_variable_words::native_variable_word(
        original,
        version,
        policy.string_protocol(),
    )
    .ok()?
    else {
        return None;
    };
    Some(EffectiveInvocationWord::ArrayElementName {
        root: String::from_utf8(name).ok()?,
    })
}

/// Convert a command-token snapshot into source-aware effective argv facts.
#[must_use]
pub fn effective_command_arguments(
    tokens: &CommandTokens,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> Vec<EffectiveInvocationWord> {
    tokens
        .words()
        .get(1..)
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(argument, word)| {
            let value = effective_invocation_word(word, escapes, word_rules);
            if value == EffectiveInvocationWord::Dynamic {
                tokens
                    .source_binding
                    .as_ref()
                    .and_then(|binding| {
                        binding.original_compound_array_argument(
                            tokens,
                            argument + 1,
                            escapes,
                            word_rules,
                        )
                    })
                    .unwrap_or(value)
            } else {
                value
            }
        })
        .collect()
}

/// Borrow source-aware post-head words from a segmented command.
///
/// The segmenter's compatibility text is still the literal value bridge for
/// callers that do not need escape decoding. Its lossless fragment list is
/// what prevents a `$word` / `[command]` / `{*}word` from becoming a literal
/// option or operand during a registry query, while retaining the distinct
/// safe shape of `prefix-$word`: it remains dynamic as a value but cannot
/// start a leading option.
#[must_use]
pub fn segmented_command_arguments(command: &SegmentedCommand) -> Vec<InvocationWord<'_>> {
    command
        .texts
        .iter()
        .enumerate()
        .skip(1)
        .map(|(index, text)| {
            if command
                .expand_word
                .as_ref()
                .and_then(|markers| markers.get(index))
                == Some(&true)
            {
                return InvocationWord::Expanded;
            }
            let Some(fragments) = command.word_fragments.get(index) else {
                return InvocationWord::Opaque;
            };
            if fragments.iter().any(|fragment| {
                matches!(
                    fragment.token.kind,
                    tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                )
            }) {
                // A non-empty literal prefix before the first substitution
                // proves the evaluated word cannot begin with `-`. Keep that
                // narrower source fact so option-dependent descriptors may
                // still locate its positional embedded language; it is never
                // exposed as a literal value to registry hooks.
                let literal_prefix = fragments
                    .iter()
                    .take_while(|fragment| {
                        !matches!(
                            fragment.token.kind,
                            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                        )
                    })
                    .map(|fragment| fragment.text.as_str())
                    .collect::<String>();
                // `word_piece` retains a quoted word's opening quote in an
                // empty leading `Esc` fragment, and a leading backslash can
                // decode to `-`; neither proves a non-option value. Every
                // other direct first character is stable under Tcl word
                // substitution.
                let proves_non_option = literal_prefix
                    .chars()
                    .next()
                    .is_some_and(|first| !matches!(first, '-' | '\\' | '"'));
                if proves_non_option {
                    InvocationWord::DynamicNonOption
                } else {
                    InvocationWord::Dynamic
                }
            } else {
                InvocationWord::Literal(text)
            }
        })
        .collect()
}

/// Resolve semantic source words through the registry in `context`.
///
/// The borrowed registry resolution is materialised immediately into owned
/// facts or an owned typed unresolved outcome, making this suitable for
/// common IR and analysis state.
///
/// `context` is `None` for a caller that carries no environment (a unit
/// harness, a shape-only query); the selection is then dialect-blind, exactly
/// as the retired `None` argument behaved.
pub fn resolve_word_exprs(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    words: &[WordExpr],
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    let dialect = context
        .and_then(|context| context.context().environment.point())
        .map(tcl_registry::InvocationDialect::of_point)
        .or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        });
    resolve_word_exprs_with_dialect(registry, context, words, dialect)
}

/// Resolve source words under explicitly retained interpreter policies. This
/// keeps escape and frame grammars independent of catalogue availability.
pub fn resolve_word_exprs_with_dialect(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    words: &[WordExpr],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_word_exprs_with_dialect_and_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        words,
        dialect,
    )
}

fn resolve_word_exprs_with_dialect_and_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    words: &[WordExpr],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    if words.is_empty() {
        return Err(RegistryInvocationDecline::MissingCommandHead);
    }
    let effective: Option<Vec<_>> = dialect.map(|dialect| {
        words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect()
    });
    let facts: Vec<_> = if let Some(effective) = &effective {
        effective
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect()
    } else {
        words.iter().map(invocation_word).collect()
    };
    resolve_registry_words_in_realm_with_metadata_context(
        registry,
        context,
        &facts,
        dialect,
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
}

pub(crate) fn resolve_registry_words_in_realm(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    facts: &[InvocationWord<'_>],
    dialect: Option<tcl_registry::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_registry_words_in_realm_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        facts,
        dialect,
        realm,
    )
}

pub(crate) fn resolve_registry_words_in_realm_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    facts: &[InvocationWord<'_>],
    dialect: Option<tcl_registry::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_registry_words_in_realm_with_metadata_context_and_case(
        registry, context, facts, dialect, realm,
    )
    .map(|(resolution, _)| resolution)
}

pub(super) fn resolve_registry_words_in_realm_with_metadata_context_and_case(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    facts: &[InvocationWord<'_>],
    dialect: Option<tcl_registry::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Result<
    (
        RegistryInvocationResolution,
        AuthoredSourceCaseBodySelection,
    ),
    RegistryInvocationDecline,
> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return Err(RegistryInvocationDecline::IncompleteResolution);
    }
    let Some(head) = facts.first().copied() else {
        return Err(RegistryInvocationDecline::MissingCommandHead);
    };
    let mut invocation = InvocationWords::structured(head, &facts[1..]);
    if let Some(dialect) = dialect {
        invocation = invocation.with_dialect(dialect);
    }
    let resolution =
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            registry,
            context.map(InvocationMetadataContext::context),
            invocation,
            realm,
        );
    if let Some(resolved) = resolution.resolved() {
        if tcl_registry::trace_deferred_writes_any() {
            eprintln!(
                "TCL_LSP_TRACE_METADATA_SELECTION head={:?} canonical={:?} realm={realm:?} context={:?} query={:?}",
                invocation.head_literal(),
                resolved.canonical_command,
                context.map(|context| std::ptr::from_ref(context.context())),
                resolved.dialect,
            );
        }
        let case_selection = if resolved.semantics.options.case_list.is_none() {
            AuthoredSourceCaseBodySelection::NotCase
        } else {
            dialect
                .and_then(|dialect| {
                    resolved.authored_source_case_clause_projection(dialect.word_values)
                })
                .map_or(
                    AuthoredSourceCaseBodySelection::Unavailable,
                    AuthoredSourceCaseBodySelection::Selected,
                )
        };
        return Ok((
            RegistryInvocationResolution::Resolved(Box::new(resolved.facts())),
            case_selection,
        ));
    }
    if context.is_none_or(|context| !context.is_actual())
        && let Some(facts) = registry.native_registration_invocation_facts(invocation)
    {
        return Ok((
            RegistryInvocationResolution::Resolved(Box::new(facts)),
            AuthoredSourceCaseBodySelection::Unavailable,
        ));
    }
    let Some(unresolved) = resolution.unresolved() else {
        return Err(RegistryInvocationDecline::IncompleteResolution);
    };
    Ok((
        RegistryInvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::from_registry(unresolved),
        ),
        AuthoredSourceCaseBodySelection::Unavailable,
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
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_command_tokens_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

#[doc(hidden)]
pub fn resolve_command_tokens_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_command_tokens_with_metadata_context_and_case(registry, context, tokens)
        .map(|(resolution, _)| resolution)
}

pub(super) fn resolve_command_tokens_with_metadata_context_and_case(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Result<
    (
        RegistryInvocationResolution,
        AuthoredSourceCaseBodySelection,
    ),
    RegistryInvocationDecline,
> {
    let Some(binding) = &tokens.source_binding else {
        let dialect = context
            .and_then(|context| context.context().environment.point())
            .map(tcl_registry::InvocationDialect::of_point)
            .or_else(|| {
                registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile)
            });
        let resolution = resolve_word_exprs_with_dialect_and_metadata_context(
            registry,
            context,
            tokens.words(),
            dialect,
        )?;
        let mut resolution = require_native_implementation_lookup(tokens, resolution, dialect);
        if let RegistryInvocationResolution::Resolved(facts) = &mut resolution {
            let words = (0..tokens.words().len().saturating_sub(1))
                .map(|_| InvocationWord::Dynamic)
                .collect::<Vec<_>>();
            let mut arguments = tcl_registry::InvocationArguments::structured(&words);
            if let Some(dialect) = dialect {
                arguments = arguments.with_dialect(dialect);
            }
            crate::command_binding::object_callbacks::refine_facts(facts, arguments, None);
        }
        return Ok((resolution, AuthoredSourceCaseBodySelection::Unavailable));
    };
    let Some(target) = binding
        .proved_execution_target()
        .filter(|target| target.registry_backed)
    else {
        return Ok((
            RegistryInvocationResolution::Unresolved(
                OwnedInvocationResolutionUnresolved::UnprovedBinding {
                    spelling: tokens.argv_texts.first().cloned().unwrap_or_default(),
                    may_be_absent: binding.may_be_absent,
                    unknown: binding.unknown,
                },
            ),
            AuthoredSourceCaseBodySelection::Unavailable,
        ));
    };
    // Selection above also establishes why an effective projection exists.
    let _ = target;
    let effective =
        effective_command_words(tokens).ok_or(RegistryInvocationDecline::IncompleteResolution)?;
    resolve_effective_tokens_with_metadata_context_and_case(
        registry,
        context,
        tokens,
        &effective,
        binding.variable_context.invocation_dialect.or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        }),
    )
}

pub(super) fn resolve_effective_tokens_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_effective_tokens_with_metadata_context_and_case(
        registry, context, tokens, effective, dialect,
    )
    .map(|(resolution, _)| resolution)
}

pub(super) fn resolve_effective_tokens_with_metadata_context_and_case(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Result<
    (
        RegistryInvocationResolution,
        AuthoredSourceCaseBodySelection,
    ),
    RegistryInvocationDecline,
> {
    let frozen = frozen_argument_words(tokens, effective);
    let values: Vec<_> = effective
        .words
        .iter()
        .enumerate()
        .map(|(index, word)| {
            if let Some(value) = index
                .checked_sub(1)
                .and_then(|argument| frozen.get(argument))
            {
                return value.clone();
            }
            dialect.map_or_else(
                || match invocation_word(word) {
                    InvocationWord::Literal(value) => {
                        EffectiveInvocationWord::Literal(value.to_owned())
                    }
                    InvocationWord::KnownBytes(value) => EffectiveInvocationWord::from_bytes(value),
                    InvocationWord::Expanded => EffectiveInvocationWord::Expanded,
                    InvocationWord::Opaque => EffectiveInvocationWord::Opaque,
                    _ => EffectiveInvocationWord::Dynamic,
                },
                |dialect| {
                    effective_invocation_word(
                        word,
                        dialect.lexer_grammar.escapes,
                        dialect.word_values,
                    )
                },
            )
        })
        .collect();
    let facts: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let realm = tokens
        .source_binding
        .as_ref()
        .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
        .unwrap_or_default();
    let (resolution, case_selection) =
        resolve_registry_words_in_realm_with_metadata_context_and_case(
            registry, context, &facts, dialect, realm,
        )?;
    let mut resolution = require_native_implementation_lookup(tokens, resolution, dialect);
    if let RegistryInvocationResolution::Resolved(resolved) = &mut resolution {
        let mut arguments = tcl_registry::InvocationArguments::structured(&facts[1..]);
        if let Some(dialect) = dialect {
            arguments = arguments.with_dialect(dialect);
        }
        crate::command_binding::object_callbacks::refine_facts(
            resolved,
            arguments,
            tokens.source_binding.as_ref(),
        );
    }
    let case_selection = if matches!(resolution, RegistryInvocationResolution::Resolved(_)) {
        case_selection
    } else {
        AuthoredSourceCaseBodySelection::Unavailable
    };
    Ok((resolution, case_selection))
}

/// Private ensemble implementation proof belongs to the semantic adapter,
/// including callers that carry source words without an interpreter snapshot.
/// Such callers may query syntax/assistance separately; they cannot borrow the
/// public ensemble's identity to specialize an unproved private implementation.
fn require_native_implementation_lookup(
    tokens: &CommandTokens,
    resolution: RegistryInvocationResolution,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> RegistryInvocationResolution {
    if tokens
        .source_binding
        .as_ref()
        .is_some_and(|binding| binding.named_invocation().is_some())
    {
        return resolution;
    }
    if let RegistryInvocationResolution::Resolved(facts) = &resolution
        && let Some(lookup) = match facts.normal_handler_implementation_lookup(dialect) {
            tcl_registry::native_compilation::NormalHandlerImplementationLookup::Required(
                lookup,
            ) => Some(lookup),
            _ => dialect.and_then(|dialect| {
                facts
                    .native_compilation
                    .and_then(|native| native.implementation_lookup(dialect))
            }),
        }
        && !tokens
            .source_binding
            .as_ref()
            .is_some_and(|binding| binding.proves_native_implementation_lookup(&lookup))
    {
        return RegistryInvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::UnprovedNativeImplementationLookup { lookup },
        );
    }
    resolution
}
