// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original body compiler syntax, independently closed in its source worlds.

use super::{OriginalExpressionSyntaxGapCompaction, OriginalRegistrySource, OriginalRegistryWords};
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use crate::ir::{CommandTokens, Script};
use std::sync::Arc;
use tcl_lexer::SourceImage;

/// Authentic body tokens from one current retained compilation request. Compiler
/// visitation alone supplies no runtime lookup, entered frame or edit grant.
/// Syntax queries close every original compiler world independently. The
/// Native handler envelope remains a separate query purpose.
pub struct OriginalBodySyntaxInventory {
    image: SourceImage,
    input: ResolvedAnalysisInput,
    realm: Arc<crate::realm::CommandBindingRealm>,
    request: crate::compilation_unit::OriginalOptimiserSourceOrigin,
    retained: Arc<crate::command_binding::RetainedSourceModuleBindings>,
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
            retained: Arc::clone(retained),
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
            let native = binding
                .original_compiler_argument_syntax(tokens, &self.input, registry)
                .is_some_and(|syntax| {
                    retained.owns_original_analysis_attachment(tokens)
                        && syntax.image() == &self.image
                        && syntax.config() == self.input.lexer_config()
                        && tokens.evaluated_body.is_none()
                        && binding.original_registry_syntax_is_closed(tokens, registry)
                });
            let source = !native
                && retained
                    .original_compiler_syntax_alternatives(tokens, &self.input, registry)
                    .is_some_and(|syntax| {
                        syntax.iter().all(|syntax| {
                            syntax.image() == &self.image
                                && syntax.config() == self.input.lexer_config()
                        })
                    });
            if (native || source) && !self.commands.contains(tokens) {
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
        if tokens.evaluated_body.is_some() {
            return None;
        }
        let registry = analysis.resolved_registry()?;
        let binding = tokens.source_binding.as_ref()?;
        let syntax = binding.original_compiler_argument_syntax(tokens, &self.input, registry)?;
        if syntax.image() != &self.image || syntax.config() != self.input.lexer_config() {
            return None;
        }
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

    /// Script grammar under every actual original compiler snapshot. This
    /// supplies syntax compaction only, never a runtime body or completion.
    #[must_use]
    pub fn script_bodies(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        segment: &crate::segmenter::SegmentedCommand,
    ) -> Option<Vec<crate::registry_invocation::OriginalSourceScriptBody>> {
        self.source_syntax_words(source, analysis, segment)?
            .source_script_bodies_for_syntax_coverage(self.input.borrowed_context_registry())
    }

    /// Exact Expr-role operands under the same source-only compiler proof.
    /// A bare operand receipt does not authorise an arbitrary replacement.
    #[must_use]
    pub fn expression_operands(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        segment: &crate::segmenter::SegmentedCommand,
    ) -> Option<Vec<(usize, tcl_lexer::NativeWord)>> {
        let words = self.source_syntax_words(source, analysis, segment)?;
        words
            .roles()?
            .iter()
            .filter_map(|&(argument, role)| {
                (role == tcl_registry::ArgRole::Expr).then_some(argument)
            })
            .map(|argument| Some((argument, words.original_argument_word(argument)?.clone())))
            .collect()
    }

    fn source_syntax_words(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        segment: &crate::segmenter::SegmentedCommand,
    ) -> Option<OriginalRegistryWords> {
        let tokens = self.tokens(source, analysis, segment)?;
        let context = self.input.borrowed_context_registry();
        let registry = context.commands();
        let alternatives =
            self.retained
                .original_compiler_syntax_alternatives(tokens, &self.input, registry)?;
        let mut unanimous = None;
        for syntax in alternatives {
            let metadata = syntax.metadata();
            let advice = syntax.source_lookup_advice(tokens)?;
            if !advice.closed_authored_simulation_source_lookup(tokens, metadata, registry) {
                return None;
            }
            let selected = super::super::resolve_original_declared_layout(
                registry, metadata, tokens, &advice,
            )?;
            let candidate = compiler_source_words(
                &self.image,
                &self.input,
                tokens,
                segment,
                &syntax,
                &selected,
            )?;
            if let Some(prior) = &unanimous {
                if prior != &candidate || !same_source_descriptor(prior, &candidate, context) {
                    return None;
                }
            } else {
                unanimous = Some(candidate);
            }
        }
        unanimous
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

    /// Same configured token-gap verifier after independently closed source
    /// compiler syntax, without borrowing a Native handler envelope.

    #[must_use]
    pub fn source_expression_gaps(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        segment: &crate::segmenter::SegmentedCommand,
        argument: usize,
        candidate: &str,
    ) -> Option<OriginalExpressionSyntaxGapCompaction> {
        let words = self.source_syntax_words(source, analysis, segment)?;
        words
            .roles()?
            .contains(&(argument, tcl_registry::ArgRole::Expr))
            .then_some(())?;
        let original = words.original_argument_word(argument)?.clone();
        let context = self.input.context_registry();
        let gaps = crate::command_binding::configured_expression_syntax_gaps(
            &original,
            &self.input,
            candidate,
        )?;
        Some(OriginalExpressionSyntaxGapCompaction {
            original,
            gaps,
            registry: context.commands().snapshot().semantic_key(),
        })
    }
}

fn compiler_source_words(
    image: &SourceImage,
    input: &ResolvedAnalysisInput,
    tokens: &CommandTokens,
    segment: &crate::segmenter::SegmentedCommand,
    syntax: &crate::command_binding::OriginalCompilerArgumentSyntax<'_>,
    selected: &super::super::ResolvedStatementInvocation,
) -> Option<OriginalRegistryWords> {
    use crate::registry_invocation::InvocationWordOrigin;
    let config = input.lexer_config();
    if syntax.image() != image
        || syntax.config() != config
        || selected.effective.words.len() != tokens.words().len()
        || selected.effective.words.get(1..) != tokens.words().get(1..)
        || selected.effective.origins.len() != tokens.words().len()
        || selected.effective.origins.first() != Some(&InvocationWordOrigin::ResolvedHead)
        || selected
            .effective
            .origins
            .iter()
            .enumerate()
            .skip(1)
            .any(|(at, origin)| *origin != InvocationWordOrigin::Written(at))
        || selected.facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    let dialect = syntax.source_dialect()?;
    let native = super::super::original_native_compiler_words(
        image,
        tokens.words(),
        syntax.site().offset,
        config,
    )?;
    if native.iter().any(|word| word.group().expand) {
        return None;
    }
    let arguments = super::original_source_argument_words(
        tokens,
        &selected.effective,
        &native,
        dialect.native_source_string_protocol()?,
    )?;
    let operands = super::effective_operand_sources(
        segment,
        tokens,
        &selected.effective,
        &arguments,
        Some(&native),
    )?;
    let context = input.borrowed_context_registry();
    let roles = selected.with_metadata_schema(
        context.commands(),
        syntax.metadata(),
        syntax.realm(),
        |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            super::conditional_roles(&roles, schema.semantics.argument_offset, complete)
        },
    )?;
    let words = OriginalRegistryWords {
        command: selected.facts.canonical_command.clone(),
        dialect: Some(dialect),
        arguments,
        operands,
        origins: selected.effective.origins.clone(),
        head: Some(super::OriginalOperandSource {
            span: native.first()?.span(),
            input: None,
            word: native.first().cloned(),
        }),
        roles: Some(roles),
        source: OriginalRegistrySource::ConditionalNativeSource(Arc::new(
            super::OriginalNativeSourceApplicability {
                unknown_residual: true,
                may_be_absent: true,
                source_advice: None,
            },
        )),
        captured_values: None,
        operands_preserve_source_lookup: false,
        image: image.clone(),
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: None,
    };
    let same = selected.with_metadata_schema(
        context.commands(),
        syntax.metadata(),
        syntax.realm(),
        |original| {
            words.with_source_schema(context, |current| same_source_schema(original, current))
        },
    )?;
    same.then_some(words)
}

fn same_source_descriptor(
    left: &OriginalRegistryWords,
    right: &OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
) -> bool {
    left.with_source_schema(context, |left| {
        right.with_source_schema(context, |right| same_source_schema(left, right))
    }) == Some(Some(true))
}

fn same_source_schema(
    left: &tcl_registry::ResolvedInvocation<'_, '_>,
    right: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> bool {
    let a = left.authored_source_descriptors();
    let b = right.authored_source_descriptors();
    std::ptr::eq(a.command, b.command)
        && match (a.subcommand, b.subcommand) {
            (None, None) => true,
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            _ => false,
        }
        && left.semantics.argument_offset == right.semantics.argument_offset
        && left.authored_source_argument_roles() == right.authored_source_argument_roles()
        && match (
            left.authored_source_definition_body_grammar(),
            right.authored_source_definition_body_grammar(),
        ) {
            (None, None) => true,
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minifier_analysis(source: &str) -> AnalysisResult {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let generation = tcl_registry::model::static_context_for_profile(profile);
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let context =
            Arc::new(generation.with_command_store(registry.snapshot().shared_registry()));
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        assert!(!analysis.allows_lexical_declaration_advice());
        analysis
    }

    fn original_segment(
        source: &str,
        config: tcl_lexer::LexerConfig,
        offset: usize,
    ) -> crate::segmenter::SegmentedCommand {
        crate::segmenter::segment_commands_with_offset_and_config(
            &source[offset..],
            u32::try_from(offset).unwrap(),
            config,
        )
        .remove(0)
    }

    #[test]
    fn actual_factorial_compiler_syntax_does_not_borrow_native_attachment() {
        let source = "proc fact {n} {\n# base case\nif {$n <= 1} { return 1 }\nreturn [expr {$n * [fact [expr {$n - 1}]]}]\n}\nputs [fact 5]\n";
        let analysis = minifier_analysis(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        let inventory = OriginalBodySyntaxInventory::capture(source, &analysis).unwrap();
        let segment = original_segment(source, input.lexer_config(), source.find("if {").unwrap());
        let tokens = inventory.tokens(source, &analysis, &segment).unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.compiler_lookup_state.is_none());
        let alternatives = inventory
            .retained
            .original_compiler_syntax_alternatives(
                tokens,
                input,
                analysis.resolved_registry().unwrap(),
            )
            .unwrap();
        assert!(!alternatives.is_empty());
        let syntax = inventory
            .source_syntax_words(source, &analysis, &segment)
            .unwrap();
        assert!(matches!(
            syntax.source(),
            OriginalRegistrySource::ConditionalNativeSource(_)
        ));
        assert!(!syntax.operands_preserve_source_lookup());
        assert!(inventory.words(source, &analysis, &segment).is_none());
        let operands = inventory
            .expression_operands(source, &analysis, &segment)
            .unwrap();
        assert_eq!(operands.len(), 1);
        assert_eq!(operands[0].0, 0);
        let proof = inventory
            .source_expression_gaps(source, &analysis, &segment, 0, "$n<=1")
            .unwrap();
        assert!(!proof.removed_gaps().is_empty());
        assert_eq!(proof.original_operand(), &operands[0].1);
        assert_eq!(
            inventory
                .script_bodies(source, &analysis, &segment)
                .unwrap()
                .len(),
            1
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .compiler_lookup_state
                .is_none()
        );
        let foreign = minifier_analysis(source);
        assert!(
            inventory
                .expression_operands(source, &foreign, &segment)
                .is_none()
        );
        assert!(
            inventory
                .source_expression_gaps(source, &foreign, &segment, 0, "$n<=1")
                .is_none()
        );
    }

    #[test]
    fn source_compiler_syntax_refuses_changed_handlers_and_observers() {
        for source in [
            "proc f {x} {if {$x <= 1} {return 1}}; rename if saved; proc if args {return changed}",
            "proc f {x} {if {$x <= 1} {return 1}}; trace add execution if enter callback",
            "proc f {x} {if {$x <= 1} {return 1}}; unknown_future_entry",
        ] {
            let analysis = minifier_analysis(source);
            let input = analysis.resolved_input.as_ref().unwrap();
            let inventory = OriginalBodySyntaxInventory::capture(source, &analysis).unwrap();
            let segment =
                original_segment(source, input.lexer_config(), source.find("if {").unwrap());
            assert!(
                inventory
                    .source_expression_gaps(source, &analysis, &segment, 0, "$x<=1")
                    .is_none(),
                "{source}"
            );
            assert!(
                inventory
                    .script_bodies(source, &analysis, &segment)
                    .is_none(),
                "{source}"
            );
        }
    }

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
