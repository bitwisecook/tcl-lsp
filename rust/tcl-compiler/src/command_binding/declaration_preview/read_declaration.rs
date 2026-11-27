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

//! The declaration a procedure read resolves against: Logical declarations and native recipes.

use super::super::SourceCommandBindings;
use super::super::declaration_layout::{
    DeclarationLayoutObservation, original_declaration_layouts,
};
use super::LogicalProcedureDeclaration;
use super::procedure_header::{declaration_tokens, original_declaration_operand_input};
use crate::command_binding::ExecutedScriptWords;
use crate::registry_invocation::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    effective_words_for_target,
};
use tcl_names::command::CommandAllocationSite;
use tcl_names::origin::ExecutedScriptSource;

impl LogicalProcedureDeclaration {
    pub(crate) fn qualified_name(&self) -> Option<&str> {
        self.body.slot.authored_spelling()
    }
    pub(crate) fn name(&self) -> Option<&str> {
        self.body.logical_name.as_deref()
    }
    pub(crate) fn parameters_text(&self) -> Option<&str> {
        self.body.logical_parameters_text.as_deref()
    }
    pub(crate) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        &self.body.parameters
    }
    pub(crate) fn source(&self) -> &ExecutedScriptSource {
        &self.body.source
    }
    pub(crate) fn namespace(&self) -> &tcl_names::namespace::SourceNamespaceKey {
        &self.body.namespace
    }
}

/// Original declaration applicability for existential local-read advice.
/// Native formals retain their selected byte topology; Logical formals retain
/// their distinct authored declaration recipe. Neither supplies body entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalProcedureReadDeclaration {
    pub(super) source: ProcedureReadSource,
    pub(super) formals: ProcedureReadFormals,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ProcedureReadSource {
    Contiguous(ExecutedScriptSource),
    OriginalBraced {
        producer: tcl_names::word::SignatureSourceNameKey,
        correspondence: tcl_syntax::backslash::NativeBracedWordSourceMap,
    },
    /// Whole original source geometry under the selected configuration. This
    /// variant is confined to the tagged operand-layout carrier below.
    ConfiguredSourceWord {
        producer: tcl_names::word::SignatureSourceNameKey,
        correspondence: tcl_syntax::backslash::NativeBracedWordSourceMap,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ProcedureReadFormals {
    Native(super::super::formal_topology::OriginalFormalTopology),
    Logical {
        parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
        text: String,
    },
}

impl OriginalProcedureReadDeclaration {
    pub(crate) fn original_image(&self) -> &tcl_lexer::SourceImage {
        match &self.source {
            ProcedureReadSource::Contiguous(source) => source.origin.source_image(),
            ProcedureReadSource::OriginalBraced { producer, .. }
            | ProcedureReadSource::ConfiguredSourceWord { producer, .. } => {
                producer.original_word().image()
            }
        }
    }
    pub(crate) fn original_body_span(&self) -> Option<tcl_lexer::Span> {
        match &self.source {
            ProcedureReadSource::Contiguous(source) => Some(tcl_lexer::Span::new(
                source.base(),
                source
                    .base()
                    .checked_add(u32::try_from(source.text.len()).ok()?)?,
            )),
            ProcedureReadSource::OriginalBraced { producer, .. }
            | ProcedureReadSource::ConfiguredSourceWord { producer, .. } => {
                producer.original_word().content_span().ok()
            }
        }
    }
    pub(crate) fn original_body_text(&self) -> Option<&str> {
        std::str::from_utf8(
            self.original_image()
                .bytes()
                .get(self.original_body_span()?.as_range())?,
        )
        .ok()
    }
    pub(crate) fn owns_original_read(&self, span: tcl_lexer::Span) -> bool {
        let Some(body) = self.original_body_span() else {
            return false;
        };
        if span.start() < body.start() || span.end() > body.end() {
            return false;
        }
        match &self.source {
            ProcedureReadSource::Contiguous(_) => true,
            ProcedureReadSource::ConfiguredSourceWord { .. } => false,
            ProcedureReadSource::OriginalBraced { correspondence, .. } => correspondence
                .native_extent(
                    (span.start() - body.start()) as usize..(span.end() - body.start()) as usize,
                )
                .is_some(),
        }
    }
    pub(crate) fn parameter_names(&self) -> Vec<String> {
        match &self.formals {
            ProcedureReadFormals::Native(topology) => topology.advisory_parameters(),
            ProcedureReadFormals::Logical { parameters, .. } => parameters.clone(),
        }
        .into_iter()
        .map(|formal| formal.name)
        .collect()
    }
    pub(crate) fn parameters_text(&self) -> Option<&str> {
        match &self.formals {
            ProcedureReadFormals::Native(topology) => {
                std::str::from_utf8(topology.original_input().bytes()).ok()
            }
            ProcedureReadFormals::Logical { text, .. } => Some(text),
        }
    }
}

impl SourceCommandBindings {
    /// Conditional declaration from complete supplied Logical input and exact
    /// original source observations. Labels cannot reconstruct this receipt.
    pub(crate) fn original_logical_procedure_declaration(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<LogicalProcedureDeclaration> {
        let site = tokens.source_binding.as_ref()?.invocation_site()?;
        let declaration = self.original_logical_procedure_declaration_at(site, registry, input)?;
        for row in original_declaration_layouts(self.declaration_layouts.get(site)?)? {
            if declaration_tokens(site, row)?.words() != tokens.words() {
                return None;
            }
        }
        Some(declaration)
    }

    fn original_logical_procedure_declaration_at(
        &self,
        site: &CommandAllocationSite,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<LogicalProcedureDeclaration> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        if !input.has_logical_source_name_context()
            || input
                .context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
            || !self.matches_original_source_image(site.source.source_image(), input.lexer_config())
        {
            return None;
        }
        for row in original_declaration_layouts(self.declaration_layouts.get(site)?)? {
            if row.config != input.lexer_config()
                || row.snapshot.state.logical_source_name_advice_input() != Some(input)
            {
                return None;
            }
        }
        let body = self.original_declared_procedure_at(site, registry)?;
        body.slot.authored_spelling()?;
        body.original_parameters
            .is_none()
            .then_some(LogicalProcedureDeclaration { body })
    }

    pub(in super::super) fn original_logical_procedure_source_slot(
        &self,
        procedure: &crate::ir::Procedure,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<tcl_names::command::SourceCommandKey> {
        let declaration = self.original_logical_procedure_declaration_at(
            procedure.declaration_site.as_ref()?,
            registry,
            input,
        )?;
        (declaration.name() == Some(procedure.name.as_str())
            && declaration.body.matches_original_procedure_source(
                procedure,
                self.root_origin.as_ref()?.source_image(),
            ))
        .then_some(declaration.body.slot)
    }

    /// Selected original header grammar for conditional diagnostic advice.
    /// Namespace publication, allocated procedure and physical frame are not
    /// prerequisites; the retained declaration and formal producer are.
    pub(crate) fn original_procedure_read_declaration(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<OriginalProcedureReadDeclaration> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, input,
        )?;
        let site = tokens.source_binding.as_ref()?.invocation_site()?;
        if !self.matches_original_source_image(site.source.source_image(), input.lexer_config()) {
            return None;
        }
        if tokens
            .source_binding
            .as_ref()?
            .original_source_advice_purpose(tokens, metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical
        {
            let recipe = self.original_logical_procedure_declaration(tokens, registry, input)?;
            return Some(OriginalProcedureReadDeclaration {
                source: ProcedureReadSource::Contiguous(recipe.source().clone()),
                formals: ProcedureReadFormals::Logical {
                    parameters: recipe.parameters().to_vec(),
                    text: recipe.parameters_text()?.to_owned(),
                },
            });
        }
        let mut agreed = None;
        for row in original_declaration_layouts(self.declaration_layouts.get(site)?)? {
            if row.config != input.lexer_config()
                || declaration_tokens(site, row)?.words() != tokens.words()
            {
                return None;
            }
            let advice = super::super::original_site_operand_layout_advice(
                site,
                tokens,
                &row.snapshot,
                &row.namespace,
                row.config,
            )?;
            if advice.dialect() != super::super::source_analysis_entry::source_input_dialect(input)
            {
                return None;
            }
            for target in advice.targets() {
                if !target.registry_backed {
                    return None;
                }
                let recipe = native_procedure_read_recipe(
                    site, tokens, row, target, &advice, registry, metadata,
                )?;
                if agreed.as_ref().is_some_and(|previous| previous != &recipe) {
                    return None;
                }
                agreed = Some(recipe);
            }
        }
        agreed
    }
}

fn native_procedure_read_recipe(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    row: &DeclarationLayoutObservation,
    target: &super::super::SourceCommandTarget,
    advice: &super::super::OriginalCompilationLookupAdvice,
    registry: &tcl_registry::CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
) -> Option<OriginalProcedureReadDeclaration> {
    let dialect = advice.dialect();
    let effective = effective_words_for_target(tokens, target)?;
    let values: Vec<_> = effective
        .words
        .iter()
        .map(|word| {
            effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
        })
        .collect();
    let words: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let RegistryInvocationResolution::Resolved(facts) =
        crate::registry_invocation::resolve_registry_words_in_realm_with_metadata_context(
            registry,
            Some(metadata),
            &words,
            Some(dialect),
            advice.realm(),
        )
        .ok()?
    else {
        return None;
    };
    let tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(shape) =
        facts.procedure_definition?
    else {
        return None;
    };
    let crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(parameters) =
        original_declaration_operand_input(
            site,
            tokens,
            row,
            target,
            &effective,
            shape.parameters_at.checked_add(1)?,
        )?
    else {
        return None;
    };
    let topology = super::super::formal_topology::OriginalFormalTopology::from_original_key(
        parameters, dialect,
    )?;
    let body_at = shape.body_at.checked_add(1)?;
    let crate::registry_invocation::InvocationWordOrigin::Written(written) =
        effective.origins.get(body_at)?
    else {
        return None;
    };
    let source = ExecutedScriptSource::from_word(
        site.clone(),
        written.checked_sub(1)?,
        tokens.words().get(*written)?,
        words.get(body_at)?.literal()?,
        row.config,
    );
    let source = if source.origin == site.source {
        ProcedureReadSource::Contiguous(source)
    } else {
        let crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(producer) =
            original_declaration_operand_input(site, tokens, row, target, &effective, body_at)?
        else {
            return None;
        };
        original_braced_procedure_read_source(producer, &words, body_at, site)?
    };
    Some(OriginalProcedureReadDeclaration {
        source,
        formals: ProcedureReadFormals::Native(topology),
    })
}

fn original_braced_procedure_read_source(
    producer: tcl_names::word::SignatureSourceNameKey,
    words: &[tcl_registry::InvocationWord<'_>],
    body_at: usize,
    site: &CommandAllocationSite,
) -> Option<ProcedureReadSource> {
    let original = producer.original_word();
    if original.group().kind != tcl_lexer::WordKind::Braced
        || original.image() != site.source.source_image()
    {
        return None;
    }
    let correspondence = tcl_syntax::backslash::native_source_braced_word_map(
        original
            .image()
            .bytes()
            .get(original.content_span().ok()?.as_range())?,
        original.image().channel(),
        original.config().brace_backslash_newline,
        producer.policy().string_protocol(),
    )
    .ok()?;
    let evaluated = tcl_syntax::backslash::native_source_literal_bytes(
        words.get(body_at)?.literal()?.as_bytes(),
        original.image().channel(),
        producer.policy().string_protocol(),
    )
    .ok()?;
    if correspondence.bytes() != producer.bytes() || correspondence.bytes() != evaluated.as_ref() {
        return None;
    }
    Some(ProcedureReadSource::OriginalBraced {
        producer,
        correspondence,
    })
}
