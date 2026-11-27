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

//! Original source cases: bodies, patterns, registry words and folded subjects.

use super::selected_words::effective_operand_sources;
use super::{
    OriginalOperandSource, OriginalRegistrySource, OriginalRegistryWords, OriginalSourceCase,
};
use tcl_registry::CommandRegistry;

impl OriginalSourceCase {
    pub(crate) fn body_operands(&self) -> &tcl_registry::case_bodies::CaseBodyOperands {
        &self.bodies
    }

    pub(crate) fn patterns(&self) -> &[String] {
        &self.patterns
    }

    pub(crate) const fn mode(&self) -> tcl_registry::spec::CaseMatchMode {
        self.layout.mode
    }

    pub(crate) fn references_parameter(&self, parameter: &str) -> bool {
        original_scalar_reference_is(&self.subject, parameter, self.config)
            || self.folded_subject.as_ref().is_some_and(|word| {
                original_scalar_reference_is(word, parameter, self.config.nested())
            })
    }

    pub(crate) const fn folds_case(&self) -> bool {
        self.layout.nocase || self.folded_subject.is_some()
    }

    /// Literal data or complete scalar syntax from the original subject word.
    pub(crate) fn subject_data(&self) -> Option<&str> {
        self.subject_literal.as_deref()
    }

    pub(crate) fn matches_subject_presentation(&self, subject: &str, braced: bool) -> bool {
        self.subject.legacy_text() == subject
            && matches!(
                &self.subject,
                tcl_source::words::WordExpr::BracedLiteral { .. }
            ) == braced
    }

    pub(crate) fn scalar_subject_name(&self) -> Option<&str> {
        let (reference, source) = self.subject.sole_variable_substitution()?;
        (source.provenance == tcl_source::words::Provenance::Source).then_some(())?;
        tcl_syntax::naming::whole_scalar_variable_reference_name(reference, self.config)
    }

    /// Match each current IR body to its genuine case-source script extent.
    /// Source correspondence grants no Native body entry or executable erasure.
    pub(crate) fn matches_statement_bodies(
        &self,
        arms: &[crate::ir::SwitchArm],
        default: Option<&crate::ir::Script>,
    ) -> bool {
        let actual = arms
            .iter()
            .filter_map(|arm| arm.body.as_ref())
            .chain(default)
            .collect::<Vec<_>>();
        actual.len() == self.scripts.len()
            && self.scripts.len() == self.bodies.bodies.len()
            && actual.iter().zip(&self.scripts).all(|(script, body)| {
                let Some(executed) = script.executed_source.as_deref() else {
                    return false;
                };
                let tcl_names::origin::ExecutedScriptMapping::Contiguous { base } =
                    executed.mapping
                else {
                    return false;
                };
                let span = body.content_span();
                executed.origin.source_image() == body.original_container().image()
                    && base == span.start()
                    && usize::try_from(span.end() - span.start()).ok()
                        == Some(executed.text.bytes().len())
                    && executed.origin.source_image().bytes().get(span.as_range())
                        == Some(executed.text.bytes())
            })
    }
}

fn original_scalar_reference_is(
    word: &tcl_source::words::WordExpr,
    parameter: &str,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let Some((spelling, source)) = word.sole_variable_substitution() else {
        return false;
    };
    if source.provenance != tcl_source::words::Provenance::Source {
        return false;
    }
    let Ok(Some(reference)) = tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config)
    else {
        return false;
    };
    reference.index.is_none()
        && tcl_syntax::naming::split_element_ref_bytes(reference.name).is_none()
        && reference.name == parameter.as_bytes()
}

/// Retain the separately selected source-role projection without strengthening
/// `InvocationFacts`. The original vector, full input and selected descriptor
/// must all agree before this facade can describe possible case bodies.
pub(crate) fn original_logical_source_case_for_tokens(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
    metadata: super::super::InvocationMetadataContext<'_>,
) -> Option<OriginalSourceCase> {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_input(
        tokens,
        metadata.source_analysis_input()?,
        registry,
    )?;
    let selected = super::super::original_logical_operation_invocation_with_metadata_context(
        registry, metadata, tokens,
    )?;
    original_source_case_projection(tokens, registry, metadata, &selected)
}

/// An already typed Native Switch may retain its original readonly case
/// syntax only through its actual normal handler and closed source lookup.
/// A generic Native call cannot acquire that compiler admission here.
pub(crate) fn original_native_source_case_for_statement(
    script: &crate::ir::Script,
    statement: &crate::ir::Statement,
    registry: &CommandRegistry,
    metadata: super::super::InvocationMetadataContext<'_>,
) -> Option<OriginalSourceCase> {
    matches!(statement, crate::ir::Statement::Switch { .. }).then_some(())?;
    let tokens = script.retained_source_tokens_for_statement(statement)?;
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_input(
        tokens,
        metadata.source_analysis_input()?,
        registry,
    )?;
    let selected = original_native_handler_for_case_source(tokens, registry, metadata)?;
    let case = original_source_case_projection(tokens, registry, metadata, &selected)?;
    (!case.body_operands().selection_unknown).then_some(case)
}

fn original_native_handler_for_case_source(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
    metadata: super::super::InvocationMetadataContext<'_>,
) -> Option<super::super::ResolvedStatementInvocation> {
    let binding = tokens.source_binding.as_ref()?;
    if binding.original_invocation_source_purpose(tokens, metadata, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical
    {
        return None;
    }
    binding
        .original_source_word_dialect_for_tokens(tokens)?
        .native_name_protocol()?;
    if binding.execution_is_unknown() || binding.execution_may_be_absent() {
        return None;
    }
    let selected = super::super::resolved_handler_invocation_with_metadata_context(
        registry,
        Some(metadata),
        tokens,
    )?;
    if !selected.facts.arg_roles_complete
        || selected.facts.arity_accepts_frozen_arguments() != Some(true)
        || matches!(
            selected.facts.subcommand,
            tcl_registry::OwnedSubcommandResolution::Unknown { .. }
                | tcl_registry::OwnedSubcommandResolution::Ambiguous { .. }
                | tcl_registry::OwnedSubcommandResolution::Indeterminate { .. }
        )
    {
        return None;
    }
    Some(selected)
}

fn original_source_case_projection(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
    metadata: super::super::InvocationMetadataContext<'_>,
    selected: &super::super::ResolvedStatementInvocation,
) -> Option<OriginalSourceCase> {
    let input = metadata.source_analysis_input()?;
    let binding = tokens.source_binding.as_ref()?;
    let logical_source = binding.original_source_advice_purpose(tokens, metadata, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let realm = if logical_source {
        binding.original_source_realm_for_input(tokens, input, registry)?
    } else {
        binding.invocation_realm()?
    };
    selected.with_metadata_schema(registry, metadata, realm, |schema| {
        schema.semantics.options.case_list.map(|_| ())
    })?;
    let words = original_case_registry_words(tokens, registry, metadata, selected)?;
    let (case, layout, bodies) =
        words.with_source_schema(input.borrowed_context_registry(), |schema| {
            selected.with_metadata_schema(registry, metadata, realm, |original| {
                let source_descriptors = schema.authored_source_descriptors();
                let selected_descriptors = original.authored_source_descriptors();
                let same_subcommand = match (
                    source_descriptors.subcommand,
                    selected_descriptors.subcommand,
                ) {
                    (Some(source), Some(selected)) => std::ptr::eq(source, selected),
                    (None, None) => true,
                    _ => false,
                };
                // Case grammar belongs directly to the immutable selected
                // command; form selection cannot replace this descriptor.
                // Copied transition values supply no further owner identity.
                if !std::ptr::eq(source_descriptors.command, selected_descriptors.command)
                    || !same_subcommand
                {
                    return None;
                }
                let (case, layout) = schema.authored_source_case_invocation()?;
                let bodies = if logical_source {
                    schema
                        .authored_logical_source_role_projection()?
                        .case_body_operands()?
                        .clone()
                } else {
                    let options = schema.semantics.options.available().collect::<Vec<_>>();
                    case.possible_body_operands(schema.words.arguments(), &options)?
                };
                Some((case, layout, bodies))
            })
        })??;
    let argument = layout.subject_index?;
    let written = selected.effective.written_argument(argument)?;
    let subject = tokens.words().get(written + 1)?.clone();
    let patterns = original_case_patterns(&words, case, &bodies)?;
    let folded_subject = original_folded_case_subject(tokens, &subject, registry, metadata);
    let subject_literal = super::super::original_literal_argument_data(tokens, written);
    let scripts = words.source_script_bodies_for(
        input.borrowed_context_registry(),
        super::super::OriginalSourceScriptPurpose::PotentialEvaluation,
    );
    Some(OriginalSourceCase {
        layout,
        bodies,
        patterns,
        subject,
        folded_subject,
        subject_literal,
        scripts,
        config,
    })
}

fn original_case_registry_words(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
    metadata: super::super::InvocationMetadataContext<'_>,
    selected: &super::super::ResolvedStatementInvocation,
) -> Option<OriginalRegistryWords> {
    use super::super::{EffectiveInvocationWord, InvocationWordOrigin};
    let binding = tokens.source_binding.as_ref()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let (segment, recorded) = binding.original_recorded_command()?;
    (recorded.words() == tokens.words()).then_some(())?;
    let site = binding.invocation_site()?;
    let image = site.source.source_image().clone();
    let native =
        super::super::original_native_compiler_words(&image, tokens.words(), site.offset, config)?;
    let frozen = super::super::frozen_argument_words(tokens, &selected.effective);
    let arguments = selected
        .effective
        .origins
        .iter()
        .skip(1)
        .zip(frozen)
        .map(|(origin, frozen)| {
            let InvocationWordOrigin::Written(ordinal) = origin else {
                return Some(frozen);
            };
            let word = native.get(*ordinal)?;
            Some(if word.group().expand {
                EffectiveInvocationWord::Expanded
            } else {
                tcl_syntax::word_rules::original_static_word_unicode_value(word)
                    .map_or(EffectiveInvocationWord::Dynamic, |value| {
                        EffectiveInvocationWord::ByteLiteral(std::sync::Arc::from(value))
                    })
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let operands = effective_operand_sources(
        &segment,
        tokens,
        &selected.effective,
        &arguments,
        Some(&native),
    )?;
    Some(OriginalRegistryWords {
        command: selected.facts.canonical_command.clone(),
        dialect: selected.dialect,
        arguments,
        operands,
        origins: selected.effective.origins.clone(),
        head: Some(OriginalOperandSource {
            span: native.first()?.span(),
            input: None,
            word: native.first().cloned(),
        }),
        roles: None,
        source: OriginalRegistrySource::Selected,
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: registry.snapshot().semantic_key(),
        context: Some(metadata.context().clone()),
        definition_parent: None,
    })
}

fn original_case_patterns(
    words: &OriginalRegistryWords,
    case: tcl_registry::CaseListSpec,
    bodies: &tcl_registry::case_bodies::CaseBodyOperands,
) -> Option<Vec<String>> {
    let mut patterns = Vec::new();
    for (index, pattern) in bodies.patterns.iter().enumerate() {
        let original = words.original_argument_word(pattern.argument)?;
        let value = words.arguments.get(pattern.argument)?.literal_bytes()?;
        if tcl_syntax::word_rules::original_static_word_unicode_value(original)?.as_slice() != value
        {
            return None;
        }
        if let Some(element) = pattern.list_element {
            let fields = tcl_syntax::word_rules::original_static_word_list_elements(original)?;
            let field = fields.get(element)?;
            if case.is_keyword_pattern(field.value(), index, bodies.patterns.len()) {
                continue;
            }
            if case.pattern_is_list(field.value()) {
                patterns.extend(
                    field
                        .elements()?
                        .iter()
                        .map(|field| field.value().to_owned()),
                );
            } else {
                patterns.push(field.value().to_owned());
            }
        } else {
            let value = std::str::from_utf8(value).ok()?;
            if !case.is_keyword_pattern(value, index, bodies.patterns.len()) {
                if case.pattern_is_list(value) {
                    patterns.extend(
                        tcl_syntax::word_rules::original_static_word_list_elements(original)?
                            .iter()
                            .map(|field| field.value().to_owned()),
                    );
                } else {
                    patterns.push(value.to_owned());
                }
            }
        }
    }
    Some(patterns)
}

#[cfg(test)]
fn trace_original_folded_case_guard(stage: &str, offset: Option<u32>, passed: bool) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some() {
        eprintln!("ORIGINAL_FACTORY_GUARD stage={stage} offset={offset:?} passed={passed}");
    }
}

fn original_folded_case_source_image(
    parent: &crate::ir::CommandTokens,
) -> Option<(tcl_lexer::LexerConfig, &tcl_lexer::SourceImage)> {
    let config = parent
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(parent)?;
    let image = parent
        .source_binding
        .as_ref()?
        .invocation_site()?
        .source
        .source_image();
    Some((config, image))
}

pub(super) fn original_folded_case_subject(
    parent: &crate::ir::CommandTokens,
    subject: &tcl_source::words::WordExpr,
    registry: &CommandRegistry,
    metadata: super::super::InvocationMetadataContext<'_>,
) -> Option<tcl_source::words::WordExpr> {
    #[cfg(test)]
    let trace_enabled =
        tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some();
    #[cfg(test)]
    let trace = trace_original_folded_case_guard;
    let substitution = subject.sole_command_substitution();
    #[cfg(test)]
    if trace_enabled {
        trace("case_fold.substitution", None, substitution.is_some());
    }
    substitution?;
    #[cfg(test)]
    let (_, source) = substitution?;
    let input = metadata.source_analysis_input()?;
    let (config, image) = original_folded_case_source_image(parent)?;
    let extent = subject.original_command_substitution_span(image.bytes())?;
    let calls = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
        parent, config, registry, metadata,
    );
    #[cfg(test)]
    if trace_enabled {
        trace(
            "case_fold.complete_children",
            Some(source.span.start()),
            calls.is_some(),
        );
    }
    let calls = calls?;
    let call = calls.iter().find(|call| call.span == extent);
    #[cfg(test)]
    if trace_enabled {
        trace(
            "case_fold.exact_child",
            Some(source.span.start()),
            call.is_some(),
        );
    }
    let call = call?;
    let child = call.tokens.as_ref()?;
    let binding = child.source_binding.as_ref()?;
    let child_metadata = binding.original_invocation_metadata_for_input(child, input, registry);
    #[cfg(test)]
    if trace_enabled {
        trace(
            "case_fold.child_metadata",
            Some(source.span.start()),
            child_metadata.is_some(),
        );
    }
    let child_metadata = child_metadata?;
    let logical_source =
        binding.original_invocation_source_purpose(child, child_metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    let selected = if logical_source {
        super::super::original_logical_operation_invocation_with_metadata_context(
            registry,
            child_metadata,
            child,
        )
    } else {
        original_native_handler_for_case_source(child, registry, child_metadata)
    };
    #[cfg(test)]
    if trace_enabled {
        trace(
            "case_fold.selected_child",
            Some(source.span.start()),
            selected.is_some(),
        );
    }
    let selected = selected?;
    let realm = if logical_source {
        binding.original_source_realm_for_input(child, input, registry)?
    } else {
        binding.invocation_realm()?
    };
    let value = selected.with_metadata_schema(registry, child_metadata, realm, |schema| {
        (schema.semantics.byte_array_effect == tcl_registry::ByteArrayEffect::CaseFolds
            && schema.words.arguments().exact_argv_len()
                == schema.semantics.argument_offset.checked_add(1))
        .then_some(schema.semantics.argument_offset)
    });
    #[cfg(test)]
    if trace_enabled {
        trace(
            "case_fold.selected_descriptor",
            Some(source.span.start()),
            value.is_some(),
        );
    }
    let value = value?;
    let written = selected.effective.written_argument(value)?;
    Some(child.words().get(written + 1)?.clone())
}
