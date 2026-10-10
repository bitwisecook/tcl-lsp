// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional hosted taint metadata, independent of handler/effect receipts.

use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tcl_lexer::{LexerConfig, SourceImage};
use tcl_registry::{CommandRegistry, model::ContextRegistry};

/// Actual lowering source/context retained only for conditional taint advice.
/// No constructor accepts a profile label or reconstructed native name.
#[derive(Debug, Clone)]
pub struct HostedSourceTaintContext {
    image: SourceImage,
    config: LexerConfig,
    context: Arc<ContextRegistry>,
}
impl PartialEq for HostedSourceTaintContext {
    fn eq(&self, other: &Self) -> bool {
        self.image == other.image
            && self.config == other.config
            && self.context.context() == other.context.context()
            && self.context.commands().snapshot().semantic_key()
                == other.context.commands().snapshot().semantic_key()
    }
}
impl Eq for HostedSourceTaintContext {}
impl Hash for HostedSourceTaintContext {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.image.hash(state);
        self.config.hash(state);
        // Structural equality remains authoritative; equivalent version
        // spellings can deliberately share these stable coordinate hashes.
        self.context.context().environment.id.hash(state);
        for floor in self.context.context().floors.entries() {
            floor.axis.hash(state);
        }
        self.context
            .commands()
            .snapshot()
            .semantic_key()
            .hash(state);
    }
}
#[cfg(debug_assertions)]
fn trace_hosted_source(tokens: &crate::ir::CommandTokens, stage: &str) {
    if std::env::var_os("TCL_LSP_TRACE_HOSTED_TAINT").is_some() {
        let site = tokens
            .words()
            .first()
            .map(|word| word.source().span.start());
        eprintln!(
            "HOSTED_TAINT_SOURCE site={site:?} stage={stage} words={} binding={} context={}",
            tokens.words().len(),
            tokens.source_binding.is_some(),
            tokens.hosted_taint_context.is_some()
        );
    }
}

#[cfg(not(debug_assertions))]
fn trace_hosted_source(_: &crate::ir::CommandTokens, _: &str) {}

impl HostedSourceTaintContext {
    pub(crate) fn from_lowering(
        image: SourceImage,
        config: LexerConfig,
        context: Arc<ContextRegistry>,
    ) -> Option<Self> {
        let hosted =
            tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
                &context.context().environment.id,
            )?;
        tcl_syntax::naming::VendorSourceNamePolicy::authored(hosted)?;
        Some(Self {
            image,
            config,
            context,
        })
    }
    fn invocation(
        &self,
        registry: &CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<HostedSourceTaintInvocation> {
        trace_hosted_source(tokens, "issuer-enter");
        if tokens.synthetic.is_some()
            || self.context.commands().snapshot().semantic_key()
                != registry.snapshot().semantic_key()
        {
            trace_hosted_source(tokens, "synthetic-or-registry-mismatch");
            return None;
        }
        if let Some(binding) = tokens.source_binding.as_ref()
            && binding.original_lexer_config_for_tokens(tokens) != Some(self.config)
        {
            trace_hosted_source(tokens, "binding-vector-config-mismatch");
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let original =
            super::original_native_compiler_words(&self.image, tokens.words(), offset, self.config)
                .or_else(|| {
                    trace_hosted_source(tokens, "original-vector-unavailable");
                    None
                })?;
        let hosted =
            tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
                &self.context.context().environment.id,
            )?;
        let policy = tcl_syntax::naming::VendorSourceNamePolicy::authored(hosted)?;
        let head = crate::signature_scan::vendor_name::VendorSourceNameInput::from_original_word(
            original.first()?,
            policy,
        );
        let metadata = super::original_conditional_vendor_registry_metadata(
            &self.context,
            tokens,
            &head,
            &original,
        )
        .or_else(|| {
            trace_hosted_source(tokens, "guarded-catalogue-unavailable");
            None
        })?;
        if !metadata.matches_source(&self.image, self.config)
            || !metadata.matches_registry(registry)
        {
            trace_hosted_source(tokens, "metadata-owner-mismatch");
            return None;
        }
        let shape = metadata.shape().clone();
        if original.iter().any(|word| word.group().expand) {
            return None;
        }
        let effective =
            super::compose_effective_words(tokens, shape.command(), &[]).or_else(|| {
                trace_hosted_source(tokens, "effective-vector-unavailable");
                None
            })?;
        trace_hosted_source(tokens, "issuer-selected");
        Some(HostedSourceTaintInvocation {
            metadata,
            effective,
        })
    }
}

/// Possible authored source/sink at unchanged words in the actual host context.
/// Unknown runtime targets, effects, mutations, barriers and Normal stay intact.
pub(crate) struct HostedSourceTaintInvocation {
    pub(crate) metadata: super::OriginalConditionalVendorRegistryMetadata,
    pub(crate) effective: super::EffectiveCommandWords,
}
pub(crate) fn hosted_source_taint_invocation(
    registry: &CommandRegistry,
    tokens: &crate::ir::CommandTokens,
) -> Option<HostedSourceTaintInvocation> {
    let context = tokens.hosted_taint_context.as_ref().or_else(|| {
        trace_hosted_source(tokens, "lowering-context-unavailable");
        None
    })?;
    context.invocation(registry, tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::CommandTokens;
    #[test]
    fn hosted_taint_lowering_keeps_the_invocations_whole_original_image() {
        // Implementation contract: naming.consumer.hosted-source-taint-descriptor
        // docs/design/analysis/name-resolution-proofs/hosted-source-taint-descriptor.md
        let source = "when HTTP_REQUEST {set x [HTTP::query]; HTTP::respond 200 content $x}";
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let registry = context.commands();
        let config = LexerConfig::for_profile(registry.profile());
        let options = crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(tcl_dialect::DialectProfile::irules()),
            external_call_sites: None,
            declared_commands: None,
        };
        let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            source,
            options,
            None,
            Arc::clone(&context),
        );
        let function = unit.function("::when::HTTP_REQUEST").unwrap();
        let sink = function
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .filter_map(|statement| statement.statement.tokens())
            .find(|tokens| {
                tokens.words().first().is_some_and(|head| {
                    head.source().span.start() as usize == source.find("HTTP::respond").unwrap()
                })
            })
            .expect("the genuine lowered sink retains its complete original vector");
        let retained = sink.hosted_taint_context.as_ref().unwrap();
        assert_eq!(retained.image, SourceImage::document(source));
        let selected = hosted_source_taint_invocation(registry, sink)
            .expect("nested hosted source metadata keeps the actual original owner");
        assert_eq!(selected.metadata.shape().command(), "HTTP::respond");
        assert!(!selected.metadata.obligations().is_empty());
        assert!(
            super::super::resolved_tokens_invocation(registry, None, sink).is_none(),
            "source advice supplies no native handler"
        );
        let mut foreign = sink.clone();
        let mut changed = retained.as_ref().clone();
        let mut padded = source.as_bytes().to_vec();
        padded[..source.find("set x").unwrap()].fill(b' ');
        changed.image = SourceImage::from_bytes(padded.as_slice(), retained.image.channel());
        foreign.hosted_taint_context = Some(Arc::new(changed));
        assert!(
            hosted_source_taint_invocation(registry, &foreign).is_none(),
            "equal command offsets cannot substitute a padded body for its whole source"
        );
    }

    #[test]
    fn hosted_source_taint_metadata_requires_actual_words_context_and_registry() {
        // Implementation contract: naming.consumer.hosted-source-taint-descriptor
        // docs/design/analysis/name-resolution-proofs/hosted-source-taint-descriptor.md
        let source = "HTTP::respond 200 content $x";
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let registry = context.commands();
        let config = LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let image = SourceImage::document(source);
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let mut tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&image),
            config,
            &segments[0],
        );
        assert!(hosted_source_taint_invocation(registry, &tokens).is_none());
        tokens.hosted_taint_context =
            HostedSourceTaintContext::from_lowering(image.clone(), config, Arc::clone(&context))
                .map(Arc::new);
        assert!(
            hosted_source_taint_invocation(registry, &tokens).is_none(),
            "context alone supplies no current source binding"
        );
        let input = crate::analyser::ResolvedAnalysisInput::new(
            tcl_dialect::DialectProfile::irules(),
            tcl_dialect::DialectProfile::irules(),
            Arc::clone(&context),
            config,
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "report-only");
        analysis
            .retained_command_realm()
            .unwrap()
            .stamp_original_tokens(&mut tokens);
        let selected = hosted_source_taint_invocation(registry, &tokens).unwrap();
        assert!(!selected.metadata.obligations().is_empty());
        assert_eq!(selected.metadata.shape().command(), "HTTP::respond");
        assert_eq!(selected.effective.origins.len(), 4);
        assert!(
            super::super::resolved_tokens_invocation(registry, None, &tokens).is_none(),
            "source metadata issues no native invocation"
        );
        let old = tokens.clone();
        tokens.word_exprs[0] = tokens.word_exprs[1].clone();
        assert!(hosted_source_taint_invocation(registry, &tokens).is_none());
        tokens = old;
        let changed_image = SourceImage::document("HTTP::respond 200 content $y");
        tokens.hosted_taint_context =
            HostedSourceTaintContext::from_lowering(changed_image, config, Arc::clone(&context))
                .map(Arc::new);
        assert!(hosted_source_taint_invocation(registry, &tokens).is_none());
        let mut changed_config = config;
        changed_config.strict_quoting = !config.strict_quoting;
        tokens.hosted_taint_context = HostedSourceTaintContext::from_lowering(
            image.clone(),
            changed_config,
            Arc::clone(&context),
        )
        .map(Arc::new);
        assert!(hosted_source_taint_invocation(registry, &tokens).is_none());
        let environment = tcl_registry::model::ingress::resolve_environment("f5-irules");
        let keyed = tcl_registry::model::context::KeyedVersions {
            bigip: Some(tcl_dialect::model::Version::parse("21.1.0").unwrap()),
            ..Default::default()
        };
        let keyed_context = environment.context_registry(&keyed, 0);
        let original_context =
            HostedSourceTaintContext::from_lowering(image.clone(), config, Arc::clone(&context))
                .unwrap();
        let keyed_context =
            HostedSourceTaintContext::from_lowering(image.clone(), config, keyed_context).unwrap();
        assert_ne!(original_context, keyed_context);
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        assert!(
            HostedSourceTaintContext::from_lowering(image, config, Arc::clone(&foreign)).is_none()
        );
    }
}
