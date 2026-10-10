// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly grammar in an original incomplete procedure-body prefix.

use super::declaration_layout::original_declaration_layouts;
use super::{
    CommandAllocationSite, ModuleCommandBindings, OriginalCompilationLookupAdvice,
    SourceCommandBindings, SourceExecutionContext,
};
use crate::registry_invocation::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    effective_words_for_target, resolve_registry_words_in_realm,
};
use std::sync::Arc;
use tcl_lexer::{
    ExecutablePart, ExecutablePartArena, ExecutableText, LexerConfig, SourceImage, Span, SubstFlags,
};

struct IncompleteHeader {
    tokens: Box<crate::ir::CommandTokens>,
    prefix: Box<crate::ir::CommandTokens>,
    content: Span,
}

/// The real parse cut and independently complete header words; no invented closer.
fn header(image: &SourceImage, region: Span, config: LexerConfig) -> Option<IncompleteHeader> {
    if region.end().checked_sub(region.start())? > 64 * 1024 {
        return None;
    }
    let plan = tcl_lexer::native_script_words_in(image.clone(), region, config).ok()?;
    let fatal = plan.fatal_tail?;
    let delimiter = match fatal.cut.message {
        tcl_lexer::MISSING_QUOTE => b'"',
        tcl_lexer::MISSING_CLOSE_BRACE => b'{',
        _ => return None,
    };
    if fatal.cut.command != 0
        || fatal.cut.offset != region.end()
        || image.bytes().get(fatal.cut.term as usize) != Some(&delimiter)
    {
        return None;
    }
    let content = Span::new(fatal.cut.term.checked_add(1)?, region.end());
    if delimiter == b'"' {
        // An open quote would substitute its outer word before entering a body.
        // A brace cut instead owns literal source content, including inner code.
        let arena =
            ExecutablePartArena::decompose(image.clone(), content, SubstFlags::default(), config)
                .ok()?;
        if arena
            .all_parts()
            .any(|part| !matches!(part.part, ExecutablePart::Text(ExecutableText::Original)))
        {
            return None;
        }
    }
    let local = SourceImage::from_bytes(image.bytes().get(region.as_range())?, image.channel());
    let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
        &local,
        region.start(),
        config,
    )?;
    let [segment] = segments.as_slice() else {
        return None;
    };
    if segment.span.start() != fatal.command_start {
        return None;
    }
    let tokens = super::source_command_tokens_boxed(&local, region.start(), config, segment);
    if tcl_lexer::word_span_at(image.try_text().ok()?, tokens.words().last()?.source().span).start()
        != fatal.cut.term
    {
        return None;
    }
    let complete = tokens.words().get(..tokens.words().len().checked_sub(1)?)?;
    if complete.len() > 16 {
        return None;
    }
    let native = crate::registry_invocation::original_native_compiler_words(
        image,
        complete,
        segment.span.start(),
        config,
    )?;
    let end = native.last()?.span().end();
    let prefix_image = SourceImage::from_bytes(
        image
            .bytes()
            .get(segment.span.start() as usize..end as usize)?,
        image.channel(),
    );
    let prefixes = crate::segmenter::segment_commands_image_with_offset_and_config(
        &prefix_image,
        segment.span.start(),
        config,
    )?;
    let [prefix] = prefixes.as_slice() else {
        return None;
    };
    let prefix =
        super::source_command_tokens_boxed(&prefix_image, segment.span.start(), config, prefix);
    (prefix.words() == complete).then_some(IncompleteHeader {
        tokens,
        prefix,
        content,
    })
}

fn procedure_namespace(
    site: &CommandAllocationSite,
    row: &super::declaration_layout::DeclarationLayoutObservation,
    header: &IncompleteHeader,
    registry: &tcl_registry::CommandRegistry,
) -> Option<super::SourceNamespaceKey> {
    if row.snapshot.state.baseline.registry_snapshot.as_ref()
        != Some(&registry.snapshot().semantic_key())
    {
        return None;
    }
    let config = row.config;
    let origin = &site.source;
    let advice = super::original_site_operand_layout_advice(
        site,
        &header.prefix,
        &row.snapshot,
        &row.namespace,
        config,
    );
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_INCOMPLETE_HEADER").is_some() {
        eprintln!(
            "INCOMPLETE_HEADER site={} stage=prefix-lookup available={} closed={}",
            site.offset,
            advice.is_some(),
            advice
                .as_ref()
                .is_some_and(super::OriginalCompilationLookupAdvice::closed_lookup)
        );
    }
    let advice = advice?;
    if !advice.closed_lookup() {
        return None;
    }
    let mut namespace = None;
    for target in advice.targets() {
        let name = incomplete_procedure_name(site, header, target, &advice, registry, config)?;
        let original = crate::registry_invocation::original_native_compiler_words(
            origin.source_image(),
            header.prefix.words(),
            site.offset,
            config,
        )?;
        let policy = row
            .snapshot
            .state
            .source_variables
            .execution_name_policy?
            .native_recipe()?;
        let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
            crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                original.get(name.checked_add(1)?)?,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                policy,
            )?,
        );
        let selected = row
            .snapshot
            .state
            .original_conditional_procedure_key(&row.namespace, &input);
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_INCOMPLETE_HEADER").is_some() {
            eprintln!(
                "INCOMPLETE_HEADER site={} stage=procedure-geometry available={}",
                site.offset,
                selected.is_some()
            );
        }
        let selected = selected?.holder().into_owned();
        if namespace.as_ref().is_some_and(|old| old != &selected) {
            return None;
        }
        namespace = Some(selected);
    }
    namespace
}

fn incomplete_procedure_name(
    site: &CommandAllocationSite,
    header: &IncompleteHeader,
    target: &super::SourceCommandTarget,
    advice: &OriginalCompilationLookupAdvice,
    registry: &tcl_registry::CommandRegistry,
    config: LexerConfig,
) -> Option<usize> {
    if !target.prepended.is_empty() {
        return None;
    }
    let effective = effective_words_for_target(&header.tokens, target)?;
    let mut values: Vec<_> = effective
        .words
        .iter()
        .map(|word| effective_invocation_word(word, config.escapes, advice.dialect().word_values))
        .collect();
    // The actual cut retains one final source slot, but supplies no body value.
    *values.last_mut()? = EffectiveInvocationWord::Dynamic;
    let words: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let RegistryInvocationResolution::Resolved(facts) = resolve_registry_words_in_realm(
        registry,
        None,
        &words,
        Some(advice.dialect()),
        advice.realm(),
    )
    .ok()?
    else {
        return None;
    };
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_INCOMPLETE_HEADER").is_some() {
        eprintln!(
            "INCOMPLETE_HEADER site={} stage=procedure-roles complete={} accepted={:?} roles={} values={}",
            site.offset,
            facts.arg_roles_complete,
            facts.arity_accepts_frozen_arguments(),
            facts.arg_roles.len(),
            values.len()
        );
    }
    if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
        return None;
    }
    let name = facts
        .state_transitions
        .declared()?
        .command_bindings()
        .find_map(|transition| match transition {
            tcl_registry::CommandBindingTransition::Define {
                name,
                kind: tcl_registry::CommandBindingDefinitionKind::Procedure,
            } => name.argument_index(),
            _ => None,
        })?;
    let body = facts.arg_roles.iter().find_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::Body)
            .then_some(facts.argument_offset + usize::from(*index) + 1)
    })?;
    if body != values.len().checked_sub(1)? {
        return None;
    }
    super::native_procedure_parameters(
        &facts,
        tcl_registry::InvocationArguments::structured(words.get(1..)?)
            .with_dialect(advice.dialect()),
    )?;
    Some(name)
}

impl SourceCommandBindings {
    /// Keep the actual rejected chunk's readonly table snapshot when an open
    /// outer body delimiter prevents execution. Complete-body collectors still reject it.
    pub(super) fn record_incomplete_body_header(
        &mut self,
        image: &SourceImage,
        base: u32,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) {
        // Implementation contract: naming.source.incomplete-body-header-metadata
        // docs/design/analysis/name-resolution-proofs/incomplete-body-header-metadata.md
        let Some(origin) = &state.current_source_origin else {
            return;
        };
        let Ok(len) = u32::try_from(image.len()) else {
            return;
        };
        let Some(end) = base.checked_add(len) else {
            return;
        };
        let original = origin.source_image();
        if original.bytes().get(base as usize..end as usize) != Some(image.bytes()) {
            return;
        }
        let Some(header) = header(original, Span::new(base, end), context.config) else {
            return;
        };
        self.record_declaration_operand_layout(
            state,
            context,
            header.tokens.words()[0].source().span.start(),
            &header.tokens,
        );
    }

    /// Conditional source region selected by the actual body-delimiter cut and
    /// independently complete procedure header. This supplies cursor geometry,
    /// not a complete body word, entered activation or current child lookup.
    pub(crate) fn original_incomplete_body_region_at(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        cursor: u32,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Span> {
        // Implementation contract: naming.source.incomplete-body-header-metadata
        // docs/design/analysis/name-resolution-proofs/incomplete-body-header-metadata.md
        if !self.matches_original_source_image(image, config) {
            return None;
        }
        let origin = self.root_origin.as_ref()?;
        let mut region = None;
        for (site, rows) in &self.declaration_layouts {
            if &site.source != origin {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            for row in rows {
                let source = row.entry.source();
                if source.base() != 0 || source.text != *image {
                    continue;
                }
                let end = source
                    .base()
                    .checked_add(u32::try_from(source.text.len()).ok()?)?;
                let Some(header) = header(image, Span::new(site.offset, end), config) else {
                    continue;
                };
                if row.config != config
                    || row.words.as_ref() != header.tokens.words()
                    || !row
                        .entry
                        .owns_original_context(&row.snapshot.state.source_variables)
                    || row.snapshot.state.variable_frame != *row.entry.frame()
                    || cursor < header.content.start()
                    || cursor > header.content.end()
                {
                    continue;
                }
                // The actual selected header owns the possible script position.
                // No result of an earlier body command selects this geometry.
                procedure_namespace(site, row, &header, registry)?;
                if region.is_some_and(|old| old != header.content) {
                    return None;
                }
                region = Some(header.content);
            }
        }
        region
    }

    pub(crate) fn incomplete_body_operand_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalCompilationLookupAdvice> {
        // Implementation contract: naming.source.incomplete-body-header-metadata
        // docs/design/analysis/name-resolution-proofs/incomplete-body-header-metadata.md
        if tokens.synthetic.is_some() {
            return None;
        }
        let origin = self.root_origin.as_ref()?;
        let config = self.lexer_config?;
        let offset = tokens.words().first()?.source().span.start();
        let mut result = None;
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_INCOMPLETE_HEADER").is_some() {
            eprintln!(
                "INCOMPLETE_HEADER site={offset} stage=inventory rows={}",
                self.declaration_layouts.len()
            );
        }
        for (site, rows) in &self.declaration_layouts {
            if &site.source != origin || site.offset >= offset {
                continue;
            }
            let originals = original_declaration_layouts(rows);
            #[cfg(debug_assertions)]
            if std::env::var_os("TCL_LSP_TRACE_INCOMPLETE_HEADER").is_some() {
                eprintln!(
                    "INCOMPLETE_HEADER site={} stage=original-row available={} observations={}",
                    site.offset,
                    originals.is_some(),
                    rows.len()
                );
            }
            let Some(rows) = originals else {
                continue;
            };
            for row in rows {
                let source = row.entry.source();
                let end = source
                    .base()
                    .checked_add(u32::try_from(source.text.len()).ok()?)?;
                let Some(header) =
                    header(origin.source_image(), Span::new(site.offset, end), config)
                else {
                    continue;
                };
                #[cfg(debug_assertions)]
                if std::env::var_os("TCL_LSP_TRACE_INCOMPLETE_HEADER").is_some() {
                    eprintln!(
                        "INCOMPLETE_HEADER site={} stage=row-owner config={} words={} context={} frame={} contains={}",
                        site.offset,
                        row.config == config,
                        row.words.as_ref() == header.tokens.words(),
                        row.entry
                            .owns_original_context(&row.snapshot.state.source_variables),
                        row.snapshot.state.variable_frame == *row.entry.frame(),
                        header.content.start() <= offset && offset < header.content.end()
                    );
                }
                if row.config != config
                    || row.words.as_ref() != header.tokens.words()
                    || !row
                        .entry
                        .owns_original_context(&row.snapshot.state.source_variables)
                    || row.snapshot.state.variable_frame != *row.entry.frame()
                    || header.content.start() > offset
                    || offset >= header.content.end()
                {
                    continue;
                }
                let selected = incomplete_child_layout_advice(
                    site, row, &header, tokens, origin, config, registry,
                )?;
                if selected.unknown || selected.may_be_absent {
                    return None;
                }
                if result
                    .as_ref()
                    .is_some_and(|old: &OriginalCompilationLookupAdvice| {
                        old.targets != selected.targets
                            || old.namespace != selected.namespace
                            || old.dialect != selected.dialect
                    })
                {
                    return None;
                }
                result = Some(selected);
            }
        }
        result
    }
}

fn incomplete_child_layout_advice(
    site: &CommandAllocationSite,
    row: &super::declaration_layout::DeclarationLayoutObservation,
    header: &IncompleteHeader,
    tokens: &crate::ir::CommandTokens,
    origin: &Arc<super::SourceOriginId>,
    config: LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> Option<OriginalCompilationLookupAdvice> {
    let offset = tokens.words().first()?.source().span.start();
    let namespace = procedure_namespace(site, row, header, registry)?;
    // No earlier body effects are simulated to donate a successor table.
    let child = SourceImage::from_bytes(
        origin
            .source_image()
            .bytes()
            .get(header.content.as_range())?,
        origin.source_image().channel(),
    );
    let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
        &child,
        header.content.start(),
        config,
    )?;
    let segment = segments.first()?;
    if segment.is_partial || segment.span.start() != offset {
        return None;
    }
    let original =
        super::source_command_tokens_boxed(&child, header.content.start(), config, segment);
    if original.words() != tokens.words() {
        return None;
    }
    let child_site = CommandAllocationSite {
        source: Arc::clone(origin),
        offset,
    };
    let selected = super::original_site_operand_layout_advice(
        &child_site,
        tokens,
        &row.snapshot,
        &namespace,
        config,
    );
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_INCOMPLETE_HEADER").is_some() {
        eprintln!(
            "INCOMPLETE_HEADER site={offset} stage=child-lookup available={} closed={}",
            selected.is_some(),
            selected
                .as_ref()
                .is_some_and(super::OriginalCompilationLookupAdvice::closed_lookup)
        );
    }
    let selected = selected?;
    Some(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inspect(
        source: &str,
        child: &str,
        unknown_entry: bool,
    ) -> (SourceCommandBindings, crate::ir::CommandTokens) {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap());
        let config = LexerConfig::from_grammar(dialect.lexer_grammar);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                unknown_entry,
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find(child).unwrap()).unwrap();
        let segments =
            crate::segmenter::segment_commands_with_offset_and_config(child, offset, config);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segments[0],
        );
        bindings.stamp_original_tokens(&mut tokens);
        (bindings, tokens)
    }

    fn assert_incomplete_region_currency(
        bindings: &SourceCommandBindings,
        image: &SourceImage,
        config: LexerConfig,
        registry: &tcl_registry::CommandRegistry,
        end: u32,
    ) {
        assert!(
            bindings
                .original_incomplete_body_region_at(
                    &SourceImage::document("proc q {} \"puts "),
                    config,
                    end,
                    registry,
                )
                .is_none()
        );
        assert!(
            bindings
                .original_incomplete_body_region_at(
                    &SourceImage::native(image.bytes()),
                    config,
                    end,
                    registry,
                )
                .is_none()
        );
        assert!(
            bindings
                .original_incomplete_body_region_at(
                    image,
                    LexerConfig {
                        strict_quoting: !config.strict_quoting,
                        ..config
                    },
                    end,
                    registry,
                )
                .is_none()
        );
    }

    #[test]
    fn incomplete_original_body_prefix_has_grammar_without_a_complete_body() {
        // Implementation contract: naming.source.incomplete-body-header-metadata
        // docs/design/analysis/name-resolution-proofs/incomplete-body-header-metadata.md
        let source = "proc p {} \"puts ";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let image = SourceImage::document(source);
        let region = Span::new(0, u32::try_from(image.len()).unwrap());
        let fatal = tcl_lexer::native_script_words_in(image.clone(), region, config)
            .unwrap()
            .fatal_tail
            .unwrap();
        assert_eq!(fatal.cut.message, tcl_lexer::MISSING_QUOTE);
        let segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap();
        // Editor recovery flags do not supply the original parser's cut.
        assert!(!segments[0].is_partial);
        assert!(header(&image, region, config).is_some());
        let (bindings, tokens) = inspect(source, "puts ", false);
        assert_eq!(
            bindings.original_incomplete_body_region_at(&image, config, region.end(), registry),
            Some(Span::new(11, region.end()))
        );
        assert_incomplete_region_currency(&bindings, &image, config, registry, region.end());
        let advice = bindings
            .incomplete_body_operand_layout_advice(&tokens, registry)
            .unwrap();
        assert_eq!(advice.targets().len(), 1);
        assert_eq!(advice.targets()[0].registry_identity(), Some("::puts"));
        assert_eq!(
            registry
                .get(advice.targets()[0].registry_identity().unwrap())
                .unwrap()
                .name,
            "puts"
        );
        assert!(
            bindings
                .original_declared_procedure_at(
                    &CommandAllocationSite {
                        source: Arc::clone(bindings.root_origin.as_ref().unwrap()),
                        offset: 0,
                    },
                    registry
                )
                .is_none()
        );
        assert!(
            bindings
                .conditional_body_entry_at(bindings.root_origin.as_ref().unwrap(), 11)
                .is_none()
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.original_normal_result(&tokens).is_none());
        assert!(!binding.original_invocation_completes_normally(&tokens));
        let mut changed = tokens.clone();
        let crate::ir::WordExpr::Literal { text, .. } = &mut changed.word_exprs[0] else {
            panic!("literal original head");
        };
        *text = "foreign".into();
        assert!(
            bindings
                .incomplete_body_operand_layout_advice(&changed, registry)
                .is_none()
        );
        let (unknown, tokens) = inspect(source, "puts ", true);
        assert!(
            unknown
                .original_incomplete_body_region_at(&image, config, region.end(), registry)
                .is_none()
        );
        let mut foreign_registry = tcl_registry::CommandRegistry::build_default()
            .project_for_profile(registry.profile().unwrap());
        foreign_registry.insert(tcl_registry::CommandSpec {
            name: "extra",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        assert!(
            bindings
                .original_incomplete_body_region_at(&image, config, region.end(), &foreign_registry)
                .is_none()
        );
        assert!(
            unknown
                .incomplete_body_operand_layout_advice(&tokens, registry)
                .is_none()
        );
    }

    #[test]
    fn incomplete_original_braced_body_owns_geometry_without_a_complete_word() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = "proc p {} {\n    puts [set x]\n    if {$flag} {puts x}\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let image = SourceImage::document(source);
        let registry = analysis.resolved_registry().unwrap();
        let realm = analysis.retained_command_realm().unwrap();
        let end = u32::try_from(source.len()).unwrap();
        let start = u32::try_from(source.find("{\n").unwrap() + 1).unwrap();
        assert_eq!(
            realm.original_incomplete_body_region_at(&image, config, end, registry),
            Some(Span::new(start, end))
        );
        assert!(
            realm
                .original_incomplete_body_region_at(
                    &SourceImage::native(source.as_bytes()),
                    config,
                    end,
                    registry,
                )
                .is_none()
        );
        let shadowed = "proc proc args {}; proc p {} {\n    puts x\n";
        let analysis = crate::analyser::Analyser::new().analyse(shadowed, "tcl8.6");
        assert!(
            analysis
                .retained_command_realm()
                .unwrap()
                .original_incomplete_body_region_at(
                    &SourceImage::document(shadowed),
                    analysis.body_lexer_config.unwrap(),
                    u32::try_from(shadowed.len()).unwrap(),
                    analysis.resolved_registry().unwrap(),
                )
                .is_none()
        );
    }

    #[test]
    fn incomplete_body_prefix_declines_outer_substitutions_and_successor_tables() {
        // Implementation contract: naming.source.incomplete-body-header-metadata
        // docs/design/analysis/name-resolution-proofs/incomplete-body-header-metadata.md
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for source in [
            "proc p {} \"$value puts ",
            "proc p {} \"[unknown] puts ",
            "proc p {} \"unknown; puts ",
            "proc p {} \"\\x70uts ",
        ] {
            let child = if source.contains("\\x70") {
                "\\x70uts "
            } else {
                "puts "
            };
            let (bindings, tokens) = inspect(source, child, false);
            assert!(
                bindings
                    .incomplete_body_operand_layout_advice(&tokens, registry)
                    .is_none(),
                "{source}"
            );
        }
    }
}
