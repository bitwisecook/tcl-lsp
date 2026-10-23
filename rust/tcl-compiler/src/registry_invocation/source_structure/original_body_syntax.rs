// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original body compiler syntax, independently closed by the current handler.

use super::{OriginalExpressionSyntaxGapCompaction, OriginalRegistrySource, OriginalRegistryWords};
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use crate::ir::{CommandTokens, Script};
use std::sync::Arc;
use tcl_lexer::SourceImage;

/// Authentic body tokens from one current retained compilation request. Compiler
/// visitation alone supplies no runtime lookup, entered frame or edit grant.
/// Each query still requires the existing selected quiet handler envelope.
pub struct OriginalBodySyntaxInventory {
    image: SourceImage,
    input: ResolvedAnalysisInput,
    realm: Arc<crate::realm::CommandBindingRealm>,
    request: crate::compilation_unit::OriginalOptimiserSourceOrigin,
    commands: Vec<CommandTokens>,
}

impl OriginalBodySyntaxInventory {
    /// Capture actual original/installed body lowerings from the retained CU.
    /// No fragment analysis, body preview or reconstructed execution entry is
    /// substituted when its current source/compiler request is unavailable.
    #[must_use]
    pub fn capture(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        let input = analysis.resolved_input.as_ref()?;
        let image = SourceImage::document(source);
        let realm = Arc::clone(analysis.retained_command_realm_arc()?);
        let registry = analysis.resolved_registry()?;
        let request = analysis
            .original_optimiser_request_owner(source, registry)?
            .clone();
        let unit = analysis.original_optimiser_source_unit(source, registry, None)?;
        let module = &unit.ir_module;
        let retained = module.retained_source_bindings.as_ref()?;
        if !analysis.matches_original_source_image(&image, input.lexer_config())
            || !retained.matches_module(module, registry)
            || module.source != source
            || module.source_metadata_input.as_ref() != Some(input)
        {
            return None;
        }
        let mut inventory = Self {
            image,
            input: input.clone(),
            realm,
            request,
            commands: Vec::new(),
        };
        let mut bodies = std::collections::BTreeSet::new();
        for (id, procedure) in module
            .original_source_procedure_bodies()
            .chain(module.body_lowerings())
        {
            #[cfg(debug_assertions)]
            if std::env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_some() {
                eprintln!(
                    "ORIGINAL_BODY_SYNTAX_CANDIDATE duplicate={} original_summary={} installed_summary={} statements={}",
                    bodies.contains(id),
                    retained.matches_original_procedure_summary(*id, procedure),
                    retained.matches_installed_procedure_summary(*id, procedure),
                    procedure.body.statements.len(),
                );
            }
            if !bodies.insert(*id)
                || !(retained.matches_original_procedure_summary(*id, procedure)
                    || retained.matches_installed_procedure_summary(*id, procedure))
            {
                continue;
            }
            let mut scripts = vec![&procedure.body];
            while let Some(script) = scripts.pop() {
                if inventory.commands.len() > 65_536 {
                    return None;
                }
                inventory.retain_script(script, retained, registry);
                for statement in &script.statements {
                    scripts.extend(statement.child_scripts());
                }
            }
        }
        Some(inventory)
    }

    fn retain_script(
        &mut self,
        script: &Script,
        retained: &crate::command_binding::RetainedSourceModuleBindings,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let sites = script
            .command_binding_sites
            .iter()
            .filter_map(|site| site.source_tokens.as_deref());
        let statements = script
            .statements
            .iter()
            .filter_map(|statement| script.retained_source_tokens_for_statement(statement));
        for tokens in sites.chain(statements) {
            let Some(binding) = tokens.source_binding.as_ref() else {
                continue;
            };
            #[cfg(debug_assertions)]
            binding.trace_original_body_syntax(
                tokens,
                &self.input,
                registry,
                retained.owns_original_analysis_attachment(tokens),
            );
            let Some(syntax) =
                binding.original_compiler_argument_syntax(tokens, &self.input, registry)
            else {
                continue;
            };
            if retained.owns_original_analysis_attachment(tokens)
                && syntax.image() == &self.image
                && syntax.config() == self.input.lexer_config()
                && tokens.evaluated_body.is_none()
                && binding.original_registry_syntax_is_closed(tokens, registry)
                && !self.commands.contains(tokens)
            {
                self.commands.push(tokens.clone());
            }
        }
    }

    pub(crate) fn matches_analysis(&self, source: &str, analysis: &AnalysisResult) -> bool {
        let Some(registry) = analysis.resolved_registry() else {
            return false;
        };
        analysis
            .original_optimiser_request_owner(source, registry)
            .is_some_and(|current| self.request.same_request_allocation(current))
            && source.as_bytes() == self.image.bytes()
            && analysis.resolved_input.as_ref() == Some(&self.input)
            && analysis.matches_original_source_image(&self.image, self.input.lexer_config())
            && analysis
                .retained_command_realm_arc()
                .is_some_and(|current| Arc::ptr_eq(current, &self.realm))
    }

    fn tokens(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        segment: &crate::segmenter::SegmentedCommand,
    ) -> Option<&CommandTokens> {
        self.matches_analysis(source, analysis).then_some(())?;
        let parsed = CommandTokens::from_segmented(
            &self.image.source_map(),
            self.input.lexer_config(),
            segment,
        );
        let mut matching = self.commands.iter().filter(|tokens| {
            tokens.words() == parsed.words()
                && tokens.argv == parsed.argv
                && tokens.argv_texts == parsed.argv_texts
                && tokens.original_words_match_argv_text()
        });
        let first = matching.next()?;
        matching.all(|other| other == first).then_some(first)
    }

    /// Selected words from this actual body token vector. This remains separate
    /// from conditional source schemas and supplies no fresh frame or handler.
    #[must_use]
    pub fn words(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        segment: &crate::segmenter::SegmentedCommand,
    ) -> Option<OriginalRegistryWords> {
        let tokens = self.tokens(source, analysis, segment)?;
        let registry = analysis.resolved_registry()?;
        let binding = tokens.source_binding.as_ref()?;
        binding
            .original_registry_syntax_is_closed(tokens, registry)
            .then_some(())?;
        let assistance = crate::registry_invocation::original_registry_invocation_assistance(
            registry, None, tokens,
        )?;
        let words = super::selected_registry_words_from_tokens(
            &self.image,
            self.input.lexer_config(),
            tokens,
            segment,
            registry,
            assistance,
        )?;
        (matches!(words.source(), OriginalRegistrySource::Selected)
            && words.operands_preserve_source_lookup())
        .then_some(words)
    }

    /// Same configured token-gap proof through the unchanged Native syntax
    /// owner, using the actual body tokens rather than a whole-realm restamp.
    #[must_use]
    pub fn expression_gaps(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        segment: &crate::segmenter::SegmentedCommand,
        argument: usize,
        candidate: &str,
    ) -> Option<OriginalExpressionSyntaxGapCompaction> {
        let words = self.words(source, analysis, segment)?;
        words
            .roles()?
            .contains(&(argument, tcl_registry::ArgRole::Expr))
            .then_some(())?;
        let context = self.input.context_registry();
        words.with_source_schema(&context, |_| ())?;
        let original = words.original_argument_word(argument)?.clone();
        let tokens = self.tokens(source, analysis, segment)?;
        let gaps = tokens
            .source_binding
            .as_ref()?
            .original_expression_syntax_gaps(
                tokens,
                context.commands(),
                &self.input,
                &original,
                candidate,
            )?;
        Some(OriginalExpressionSyntaxGapCompaction {
            original,
            gaps,
            registry: context.commands().snapshot().semantic_key(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_syntax_inventory_cannot_borrow_an_equal_foreign_analysis() {
        let source = "proc sample {x} {if {$x <= 1} { return 1 }}\nsample 0\n";
        let first = crate::analyser::Analyser::new()
            .analyse(source, "tcl8.6")
            .clone();
        let other = crate::analyser::Analyser::new()
            .analyse(source, "tcl8.6")
            .clone();
        let inventory = OriginalBodySyntaxInventory::capture(source, &first)
            .expect("actual retained complete source compilation");
        assert!(inventory.matches_analysis(source, &first));
        assert!(!inventory.matches_analysis(source, &other));
        assert!(!inventory.matches_analysis("# changed", &first));
        let mut unavailable_context = first.clone();
        unavailable_context.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
            environment: "tcl8.6".to_owned(),
            overlay: 1,
        });
        assert!(!inventory.matches_analysis(source, &unavailable_context));
        let mut unavailable = first.clone();
        unavailable.body_lexer_config = None;
        assert!(!inventory.matches_analysis(source, &unavailable));
    }
}
