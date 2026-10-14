// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Namespace syntax of a genuine original immediate body, without body entry.

use super::{ExecutedScriptSource, SourceInvocationBinding, SourceNamespaceKey};
use crate::ir::CommandTokens;
use crate::registry_invocation::{
    EffectiveInvocationWord, InvocationMetadataContext, InvocationWordOrigin,
    effective_invocation_word, original_structured_invocation_with_metadata_context,
};
use tcl_lexer::{LexerConfig, Span};
use tcl_registry::{ArgRole, BodyInterpreter, BodyKind, CommandRegistry, ScriptTiming};

impl SourceInvocationBinding {
    /// Retain a static whole original operand at its genuine parent allocation
    /// and written ordinal. This lexical projection creates no namespace,
    /// reached body, native parser entry or successful argument evaluation.
    pub(crate) fn original_body_source_for_word(
        &self,
        parent: &CommandTokens,
        span: Span,
        config: LexerConfig,
    ) -> Option<ExecutedScriptSource> {
        if self.original_lexer_config_for_tokens(parent)? != config {
            return None;
        }
        let site = self.invocation_site().or_else(|| {
            self.original_inline_control_compilation(parent).map(
                super::compiled_invocation::SourceNativeStructuredPreparation::compilation_site,
            )
        })?;
        let original = site
            .source
            .original_lexical_command(parent.words(), site.offset, config)?;
        let mut selected = None;
        for (index, (word, native)) in parent
            .words()
            .iter()
            .zip(original.native_words())
            .enumerate()
            .skip(1)
        {
            // Braced literal IR retains the lexer's inner-end span. The
            // original NativeWord owns the whole written extent, including its
            // closer; only the shared extent owner may reconcile those views.
            if word.source().span != span
                || native.span()
                    != tcl_lexer::word_span_at(site.source.source_image().bytes(), span)
            {
                continue;
            }
            let EffectiveInvocationWord::Literal(value) = effective_invocation_word(
                word,
                config.escapes,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
            ) else {
                return None;
            };
            let source =
                ExecutedScriptSource::from_word(site.clone(), index - 1, word, &value, config);
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &source)
            {
                return None;
            }
            selected = Some(source);
        }
        selected
    }

    /// A known Logical current/root body recipe retains only source coordinates.
    /// Relative caller levels cannot borrow the declaration namespace.
    pub(crate) fn original_logical_frame_body_namespace(
        &self,
        registry: &CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
        parent: &CommandTokens,
    ) -> Option<SourceNamespaceKey> {
        let metadata = self.original_invocation_metadata_for_input(parent, input, registry)?;
        if self.original_source_advice_purpose(parent, metadata, registry)?
            != super::OriginalInvocationSourcePurpose::Logical
        {
            return None;
        }
        let advice = self.original_logical_declaration_layout_advice(parent)?;
        if !advice.closed_logical_source_lookup() {
            return None;
        }
        let selected =
            crate::registry_invocation::original_logical_frame_invocation_with_metadata_context(
                registry, metadata, parent,
            )?;
        let tcl_registry::frame_effect::FrameArgumentResolution::Valid { level, .. } =
            selected.arguments()?
        else {
            return None;
        };
        match selected.layout()? {
            tcl_registry::FrameArgLayout::ScriptInCurrentFrame => Some(advice.namespace().clone()),
            tcl_registry::FrameArgLayout::ScriptInSelectedFrame if level.is_current_frame() => {
                Some(advice.namespace().clone())
            }
            tcl_registry::FrameArgLayout::ScriptInSelectedFrame if level.is_global_frame() => {
                advice.snapshot.state.source_root_namespace_key()
            }
            _ => None,
        }
    }

    /// Inherit namespace syntax only for an independently selected immediate
    /// Plain body in the Current interpreter. Exact source, full supplied
    /// metadata and the retained namespace incarnation remain prerequisites.
    /// This issues no reached lookup, entered frame, Normal completion,
    /// Native body preparation or inline compiler admission.
    pub(crate) fn original_body_namespace_context(
        &self,
        registry: &CommandRegistry,
        request: InvocationMetadataContext<'_>,
        parent: &CommandTokens,
        source: &ExecutedScriptSource,
    ) -> Option<SourceNamespaceKey> {
        let input = request.source_analysis_input()?;
        let advice = self.declaration_operand_layout_advice(parent)?;
        let metadata = advice
            .original_declaration_metadata_context(registry, parent)
            .ok()??;
        if metadata.source_analysis_input() != Some(input) || !request.matches_registry(registry) {
            return None;
        }
        let namespace = advice.namespace();
        if let Some(native) = namespace.native_context() {
            if !advice.closed_lookup() || advice.has_opaque_handler_alternatives() {
                return None;
            }
            let entry = advice.snapshot.state.baseline.native_entry.as_deref()?;
            if entry.retained_namespace_context(native.token).ok().as_ref() != Some(native) {
                return None;
            }
        } else if !input.has_logical_source_name_context() || !advice.closed_logical_source_lookup()
        {
            return None;
        }
        let invocation =
            original_structured_invocation_with_metadata_context(registry, metadata, parent)?;
        let config = self.original_lexer_config_for_tokens(parent)?;
        let roles = invocation.written_argument_roles();
        let mut selected_body = false;
        for (index, origin) in invocation.effective.origins.iter().enumerate().skip(1) {
            let InvocationWordOrigin::Written(written) = origin else {
                continue;
            };
            let argument = index - 1;
            let written_argument = written.checked_sub(1)?;
            if !roles.contains(&(written_argument, ArgRole::Body)) {
                continue;
            }
            let word = parent.words().get(*written)?;
            let expected =
                self.original_body_source_for_word(parent, word.source().span, config)?;
            if &expected != source {
                continue;
            }
            let immediate =
                invocation.with_metadata_schema(registry, metadata, advice.realm(), |schema| {
                    Some(
                        schema.semantics.body_kind == BodyKind::Plain
                            && schema.semantics.body_interpreter == BodyInterpreter::Current
                            && schema.authored_source_script_timing_at(argument)
                                == Some(ScriptTiming::SameInvocation),
                    )
                })?;
            if !immediate {
                return None;
            }
            selected_body = true;
        }
        selected_body.then(|| namespace.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::ResolvedAnalysisInput;
    use crate::command_binding::{SourceAnalysisEntry, SourceCommandBindings};
    use crate::registry_invocation::OwnedInvocationMetadataContext;
    use std::sync::Arc;

    struct OriginalBodyFixture {
        context: Arc<tcl_registry::model::ContextRegistry>,
        input: ResolvedAnalysisInput,
        entry: SourceAnalysisEntry,
        bindings: SourceCommandBindings,
        tokens: Vec<CommandTokens>,
        _owner: tcl_vm::Vm,
    }

    fn original_fixture(source: &str, environment: &str) -> OriginalBodyFixture {
        let environment = tcl_registry::model::ingress::resolve_known_environment(environment)
            .expect("actual selected native profile");
        let context = environment.default_context_registry();
        let profile = environment.unit_profile();
        let config = LexerConfig::for_profile(Some(profile));
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
        let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        assert_eq!(native.execution_point, environment.point());
        let entry = SourceAnalysisEntry {
            metadata_context: OwnedInvocationMetadataContext::for_source_input(Some(&input)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            native_entry: Some(Arc::new(native)),
            ..SourceAnalysisEntry::default()
        };
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            context.commands(),
            entry.options(),
        );
        let image = tcl_lexer::SourceImage::document(source);
        let segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .expect("original whole-source segmentation");
        let tokens = segments
            .iter()
            .map(|segment| {
                let mut tokens =
                    CommandTokens::from_segmented(&image.source_map(), config, segment);
                bindings.stamp_original_tokens(&mut tokens);
                tokens
            })
            .collect();
        OriginalBodyFixture {
            _owner: owner,
            context,
            input,
            entry,
            bindings,
            tokens,
        }
    }

    struct OriginalBodyGeometryFixture {
        input: ResolvedAnalysisInput,
        tokens: Vec<CommandTokens>,
    }

    /// Original lexical geometry under each selected source grammar. Logical
    /// source ownership supplies no Native lookup, compiler or namespace entry.
    fn original_geometry_fixture(source: &str, environment: &str) -> OriginalBodyGeometryFixture {
        let selected = tcl_registry::model::ingress::resolve_known_environment(environment)
            .expect("actual selected lexical environment");
        let config = LexerConfig::for_profile(Some(selected.unit_profile()));
        let mut analyser = crate::analyser::Analyser::new();
        let original = analyser.prepare_analysis_input("tcl").unwrap();
        let input = ResolvedAnalysisInput::new(
            original.analyser_profile(),
            original.unit_profile(),
            original.context_registry(),
            config,
        );
        assert!(input.has_logical_source_name_context());
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            input.borrowed_context_registry().commands(),
            crate::command_binding::SourceAnalysisOptions::for_logical_source(&input).unwrap(),
        );
        let image = tcl_lexer::SourceImage::document(source);
        let segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .expect("original whole-source geometry segmentation");
        let tokens = segments
            .iter()
            .map(|segment| {
                let mut tokens =
                    CommandTokens::from_segmented(&image.source_map(), config, segment);
                bindings.stamp_original_tokens(&mut tokens);
                tokens
            })
            .collect();
        OriginalBodyGeometryFixture { input, tokens }
    }

    fn body_at(
        tokens: &CommandTokens,
        written: usize,
        config: LexerConfig,
    ) -> ExecutedScriptSource {
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .original_body_source_for_word(tokens, tokens.words()[written].source().span, config)
            .expect("whole original written body")
    }

    #[test]
    fn original_body_word_extent_keeps_inner_and_whole_geometry_separate() {
        // naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        // Lexical projection alone: the full native extent and the compiler's
        // original inner-end span describe the same written word, without any
        // body entry, completion, namespace or native compiler admission.
        let source = "if {$condition} {puts A}; if {$condition} {}; if {$condition} \"puts B\"; if {$condition} puts; if {$condition} {puts {A}}; if {$condition} {puts café}; if {$condition} {puts\nA}";
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let fixture = original_geometry_fixture(source, environment);
            let config = fixture.input.lexer_config();
            assert_eq!(fixture.tokens.len(), 7);
            for (parent, expected) in fixture.tokens.iter().zip([
                "puts A",
                "",
                "puts B",
                "puts",
                "puts {A}",
                "puts café",
                "puts\nA",
            ]) {
                let binding = parent.source_binding.as_ref().unwrap();
                let span = parent.words()[2].source().span;
                let site = binding.invocation_site().unwrap();
                let original = site
                    .source
                    .original_lexical_command(parent.words(), site.offset, config)
                    .unwrap();
                let whole = original.native_words()[2].span();
                assert_eq!(
                    whole,
                    tcl_lexer::word_span_at(site.source.source_image().bytes(), span)
                );
                if expected == "puts A" {
                    assert_ne!(
                        whole, span,
                        "nonempty braces retain their inner-end compiler span"
                    );
                }
                let body = body_at(parent, 2, config);
                assert_eq!(body.text.bytes(), expected.as_bytes());
                assert_eq!(&body.origin, &site.source);
                assert_eq!(
                    body.mapping,
                    super::super::ExecutedScriptMapping::Contiguous { base: body.base() }
                );
                assert_eq!(
                    site.source
                        .source_image()
                        .bytes()
                        .get(body.base() as usize..body.base() as usize + expected.len()),
                    Some(expected.as_bytes())
                );
                let changed = Span::new(span.start(), span.end().checked_add(1).unwrap());
                assert!(
                    binding
                        .original_body_source_for_word(parent, changed, config)
                        .is_none()
                );
                assert!(
                    binding
                        .original_body_source_for_word(
                            parent,
                            Span::new(span.start() + 1, span.end()),
                            config
                        )
                        .is_none()
                );
            }
            let continuation =
                original_geometry_fixture("if {$condition} {puts \\\n A}", environment);
            let parent = &continuation.tokens[0];
            let body = body_at(parent, 2, continuation.input.lexer_config());
            if config.brace_backslash_newline == tcl_dialect::BraceBackslashNewline::Folds {
                assert_eq!(body.text.bytes(), b"puts  A");
                assert_eq!(
                    body.mapping,
                    super::super::ExecutedScriptMapping::Materialised
                );
                assert_eq!(body.base(), 0);
                assert_ne!(
                    &body.origin,
                    &parent
                        .source_binding
                        .as_ref()
                        .unwrap()
                        .invocation_site()
                        .unwrap()
                        .source
                );
            } else {
                assert_eq!(body.text.bytes(), b"puts \\\n A");
                assert!(matches!(
                    body.mapping,
                    super::super::ExecutedScriptMapping::Contiguous { .. }
                ));
            }
            for dynamic in ["if {$condition} \"$body\"", "if {$condition} $body"] {
                let fixture = original_geometry_fixture(dynamic, environment);
                let parent = &fixture.tokens[0];
                assert!(
                    parent
                        .source_binding
                        .as_ref()
                        .unwrap()
                        .original_body_source_for_word(
                            parent,
                            parent.words()[2].source().span,
                            fixture.input.lexer_config()
                        )
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn original_immediate_body_namespace_keeps_actual_native_conditional_and_error_prefix() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Namespace syntax is independent of a reached body, compiler child,
        // successful condition, variable frame or executable IR admission.
        let source = "if {$condition} {set result A}; if {[set condition]} {set result B}";
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let fixture = original_fixture(source, environment);
            let config = fixture.input.lexer_config();
            let expected = SourceNamespaceKey::from_native_entry(
                fixture.entry.native_entry.as_deref().unwrap(),
            )
            .unwrap();
            let request = InvocationMetadataContext::for_analysis_input(
                fixture.context.commands(),
                &fixture.input,
            )
            .unwrap();
            assert_eq!(fixture.tokens.len(), 2);
            for parent in &fixture.tokens {
                let binding = parent.source_binding.as_ref().unwrap();
                let body = body_at(parent, 2, config);
                assert_eq!(
                    binding.original_body_namespace_context(
                        fixture.context.commands(),
                        request,
                        parent,
                        &body,
                    ),
                    Some(expected.clone()),
                    "{environment}: {parent:?}"
                );
                assert!(binding.native_structured_preparation.is_none());
                assert!(
                    fixture
                        .bindings
                        .executed_script_namespace_context(&body)
                        .is_none()
                );
                assert!(
                    fixture
                        .bindings
                        .compiled_script_namespace_context(&body)
                        .is_none()
                );
            }
            let mut lowerer =
                crate::lowering::Lowerer::with_config(fixture.context.commands(), config)
                    .with_dialect(Some(fixture.input.unit_profile()))
                    .with_resolved_analysis_input(fixture.input.clone());
            lowerer.set_source_analysis_options(fixture.entry.options());
            let module = lowerer.lower(source);
            assert_eq!(module.top_level.statements.len(), 2);
            for statement in &module.top_level.statements {
                let children = statement.child_scripts();
                let [child] = children.as_slice() else {
                    panic!("each original conditional retains one body");
                };
                assert_eq!(child.namespace_context.as_deref(), Some(&expected));
                assert_eq!(child.statements.len(), 1);
                let tokens = child
                    .retained_source_tokens_for_statement(&child.statements[0])
                    .expect("original child vector survives namespace syntax projection");
                assert_eq!(tokens.words().len(), 3);
                assert_eq!(
                    tokens
                        .source_binding
                        .as_ref()
                        .unwrap()
                        .original_lexer_config_for_tokens(tokens),
                    Some(config)
                );
                assert!(
                    !matches!(&child.statements[0], crate::ir::Statement::Barrier { reason, .. }
                    if reason == "original body namespace context is unavailable")
                );
            }
        }
    }

    #[test]
    fn original_immediate_body_namespace_refuses_changed_and_missing_original_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let fixture = original_fixture("if {$condition} {set result A}", "tcl8.6");
        let parent = &fixture.tokens[0];
        let binding = parent.source_binding.as_ref().unwrap();
        let config = fixture.input.lexer_config();
        let body = body_at(parent, 2, config);
        let request = InvocationMetadataContext::for_analysis_input(
            fixture.context.commands(),
            &fixture.input,
        )
        .unwrap();
        assert!(binding.original_body_namespace_context(
            fixture.context.commands(), request, parent, &body,
        ).is_some());
        let older_context = Arc::new(
            tcl_registry::model::ingress::resolve_environment("tcl8.4")
                .default_context_registry()
                .with_command_store(fixture.context.commands().snapshot().shared_registry()),
        );
        assert_eq!(
            older_context.commands().snapshot().semantic_key(),
            fixture.context.commands().snapshot().semantic_key()
        );
        assert_ne!(older_context.context(), fixture.context.context());
        let older = ResolvedAnalysisInput::new(
            fixture.input.analyser_profile(),
            fixture.input.unit_profile(),
            older_context,
            config,
        );
        let mut foreign_registry = CommandRegistry::build_default();
        foreign_registry.insert(tcl_registry::CommandSpec {
            name: "foreign_original_body_owner",
            ..foreign_registry.get("set").unwrap().clone()
        });
        let foreign_context = Arc::new(
            fixture
                .context
                .with_command_store(foreign_registry.snapshot().shared_registry()),
        );
        let foreign = ResolvedAnalysisInput::new(
            fixture.input.analyser_profile(),
            fixture.input.unit_profile(),
            foreign_context,
            config,
        );
        let grammar = ResolvedAnalysisInput::new(
            fixture.input.analyser_profile(),
            fixture.input.unit_profile(),
            Arc::clone(&fixture.context),
            LexerConfig {
                strict_quoting: !config.strict_quoting,
                ..config
            },
        );
        for changed in [&older, &foreign, &grammar] {
            assert!(
                binding
                    .original_body_namespace_context(
                        fixture.context.commands(),
                        InvocationMetadataContext::for_analysis_input(
                            changed.context_registry().commands(),
                            changed
                        )
                        .unwrap(),
                        parent,
                        &body
                    )
                    .is_none()
            );
        }
        for owner in [
            OwnedInvocationMetadataContext::Unavailable,
            OwnedInvocationMetadataContext::Supplied(Arc::clone(&fixture.context)),
        ] {
            let mut missing = parent.clone();
            let binding = missing.source_binding.as_mut().unwrap();
            binding.lookup_state = None;
            for row in Arc::make_mut(binding.declaration_layout_observations.as_mut().unwrap()) {
                Arc::make_mut(&mut Arc::make_mut(&mut row.snapshot).state.baseline)
                    .metadata_context = owner.clone();
            }
            assert!(
                missing
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_body_namespace_context(
                        fixture.context.commands(),
                        request,
                        &missing,
                        &body,
                    )
                    .is_none()
            );
        }
        let mut missing = parent.clone();
        missing
            .source_binding
            .as_mut()
            .unwrap()
            .declaration_layout_observations = None;
        assert!(
            missing
                .source_binding
                .as_ref()
                .unwrap()
                .original_body_namespace_context(
                    fixture.context.commands(),
                    request,
                    &missing,
                    &body,
                )
                .is_none()
        );
        let mut changed = parent.clone();
        changed.word_exprs.pop();
        assert!(
            binding
                .original_body_namespace_context(
                    fixture.context.commands(),
                    request,
                    &changed,
                    &body
                )
                .is_none()
        );
        let foreign_body = body_at(
            &original_fixture("if {$condition} {set result B}", "tcl8.6").tokens[0],
            2,
            config,
        );
        assert!(
            binding
                .original_body_namespace_context(
                    fixture.context.commands(),
                    request,
                    parent,
                    &foreign_body
                )
                .is_none()
        );
        let mut missing_entry = parent.clone();
        let binding = missing_entry.source_binding.as_mut().unwrap();
        binding.lookup_state = None;
        for row in Arc::make_mut(binding.declaration_layout_observations.as_mut().unwrap()) {
            Arc::make_mut(&mut Arc::make_mut(&mut row.snapshot).state.baseline).native_entry = None;
        }
        assert!(
            missing_entry
                .source_binding
                .as_ref()
                .unwrap()
                .original_body_namespace_context(
                    fixture.context.commands(),
                    request,
                    &missing_entry,
                    &body,
                )
                .is_none()
        );
    }

    #[test]
    fn original_immediate_body_namespace_refuses_noncurrent_and_structural_descriptors() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Genuine selected body metadata cannot be inferred from a body-like
        // string or from a command's printed namespace.
        for (source, written) in [
            ("proc p {} {set local VALUE}", 3),
            ("interp eval child {set local VALUE}", 3),
            ("set holder {set local VALUE}", 2),
            ("namespace code {set local VALUE}", 2),
        ] {
            let fixture = original_fixture(source, "tcl8.6");
            let parent = &fixture.tokens[0];
            let body = body_at(parent, written, fixture.input.lexer_config());
            let binding = parent.source_binding.as_ref().unwrap();
            assert!(
                binding
                    .original_body_namespace_context(
                        fixture.context.commands(),
                        InvocationMetadataContext::for_analysis_input(
                            fixture.context.commands(),
                            &fixture.input
                        )
                        .unwrap(),
                        parent,
                        &body
                    )
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_immediate_body_namespace_keeps_logical_source_and_native_entry_separate() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let source = "if {$condition} {set result VALUE}";
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig::for_profile(Some(profile));
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
        let entry = SourceAnalysisEntry::for_logical_source(&input).unwrap();
        assert!(entry.native_entry.is_none());
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            context.commands(),
            entry.options(),
        );
        let image = tcl_lexer::SourceImage::document(source);
        let segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap();
        let mut parent = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
        bindings.stamp_original_tokens(&mut parent);
        let body = body_at(&parent, 2, config);
        let binding = parent.source_binding.as_ref().unwrap();
        let namespace = binding
            .original_body_namespace_context(
                context.commands(),
                InvocationMetadataContext::for_analysis_input(context.commands(), &input).unwrap(),
                &parent,
                &body,
            )
            .expect("positively retained Logical namespace syntax");
        assert!(matches!(namespace, SourceNamespaceKey::Authored(_)));
        assert!(namespace.native_context().is_none());
        assert!(binding.native_structured_preparation.is_none());
        let native_profile = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .unwrap()
            .unit_profile();
        let native = ResolvedAnalysisInput::new(
            native_profile,
            native_profile,
            Arc::clone(&context),
            config,
        );
        assert!(!native.has_logical_source_name_context());
        assert!(
            binding
                .original_body_namespace_context(
                    context.commands(),
                    InvocationMetadataContext::for_analysis_input(context.commands(), &native)
                        .unwrap(),
                    &parent,
                    &body
                )
                .is_none()
        );
    }

    #[test]
    fn original_immediate_body_namespace_keeps_the_retained_nonroot_incarnation() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // The original namespace row supplies identity; its presentation is
        // used only to author the software fixture's namespace-eval operand.
        let fixture = original_fixture("if {$condition} {set result VALUE}", "tcl8.6");
        let native = fixture.entry.native_entry.as_deref().unwrap();
        let row = native
            .namespaces
            .iter()
            .find(|row| row.visible && !row.path.is_root())
            .expect("captured native registration includes a nonroot namespace");
        let expected =
            SourceNamespaceKey::Native(native.retained_namespace_context(row.token).unwrap());
        assert_ne!(
            expected,
            SourceNamespaceKey::from_native_entry(native).unwrap()
        );
        let command = "if {$condition} {set result VALUE}";
        let source = format!(
            "namespace eval {} {{{command}}}",
            expected.display().unwrap()
        );
        let config = fixture.input.lexer_config();
        let bindings = SourceCommandBindings::analyse_with_options(
            &source,
            config,
            fixture.context.commands(),
            fixture.entry.options(),
        );
        let offset = u32::try_from(source.find(command).unwrap()).unwrap();
        let part = tcl_lexer::SourceImage::document(command);
        let segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(&part, offset, config)
                .unwrap();
        let image = tcl_lexer::SourceImage::document(&source);
        let mut parent = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
        bindings.stamp_original_tokens(&mut parent);
        let body = body_at(&parent, 2, config);
        let binding = parent.source_binding.as_ref().unwrap();
        assert_eq!(
            binding.original_body_namespace_context(
                fixture.context.commands(),
                InvocationMetadataContext::for_analysis_input(
                    fixture.context.commands(),
                    &fixture.input
                )
                .unwrap(),
                &parent,
                &body
            ),
            Some(expected)
        );
    }
    #[test]
    fn original_conditional_entry_retains_selected_native_source_without_entered_frames() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Actual captured software entry/input; source selection is independent
        // of an entered handler frame or a physical Native callback measurement.
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let source =
                "if {$unknown} {if {1} {set selected YES}} elseif {1} {if {1} {set later YES}}";
            let fixture = original_fixture(source, environment);
            let config = fixture.input.lexer_config();
            let request = InvocationMetadataContext::for_analysis_input(
                fixture.context.commands(),
                &fixture.input,
            )
            .unwrap();
            let parent = &fixture.tokens[0];
            let binding = parent.source_binding.as_ref().unwrap();
            let expected = SourceNamespaceKey::from_native_entry(
                fixture.entry.native_entry.as_deref().unwrap(),
            )
            .unwrap();
            for index in [2, 5] {
                let body = body_at(parent, index, config);
                assert_eq!(
                    binding.original_body_namespace_context(
                        fixture.context.commands(),
                        request,
                        parent,
                        &body,
                    ),
                    Some(expected.clone())
                );
                assert!(
                    fixture
                        .bindings
                        .executed_script_namespace_context(&body)
                        .is_none()
                );
                assert!(
                    fixture
                        .bindings
                        .compiled_script_namespace_context(&body)
                        .is_none()
                );
            }
            let observations = fixture
                .bindings
                .entered_scripts
                .values()
                .flat_map(|arguments| arguments.values().flatten())
                .collect::<Vec<_>>();
            assert!(
                !observations.is_empty(),
                "conditional source must stay inventoried: {environment}"
            );
            assert!(
                observations
                    .iter()
                    .any(|entry| entry.source.text.bytes() == b"set selected YES")
            );
            assert!(
                observations
                    .iter()
                    .any(|entry| entry.source.text.bytes() == b"set later YES")
            );
            assert!(observations.iter().all(|entry| entry.frame.is_none()
                && entry.namespace.is_none()
                && entry.receiver_context.is_none()));
        }
    }

    #[test]
    fn original_conditional_calls_lambda_and_materialized_sources_preserve_the_entry_purpose() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // These are retained source continuations under an actual selected C
        // software entry. The control observes no external Native execution.
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut sources = vec![
                "proc p {} {if {1} {set selected YES}}; if {$unknown} {p}",
                "if {$unknown} {eval {if {1} {set selected YES}}}",
            ];
            if environment != "tcl8.4" {
                sources.push("if {$unknown} {apply {{} {if {1} {set selected YES}}}}");
            }
            for source in sources {
                let fixture = original_fixture(source, environment);
                let observations = fixture
                    .bindings
                    .entered_scripts
                    .values()
                    .flat_map(|arguments| arguments.values().flatten())
                    .collect::<Vec<_>>();
                assert!(
                    observations
                        .iter()
                        .any(|entry| entry.source.text.bytes() == b"set selected YES"),
                    "original descendant source remains available: {environment}: {source}"
                );
                assert!(
                    observations.iter().all(|entry| entry.frame.is_none()
                        && entry.namespace.is_none()
                        && entry.receiver_context.is_none()),
                    "a conditional or declaration preview cannot report entered descendants: {environment}: {source}"
                );
                assert!(!fixture.bindings.points.is_empty());
            }
        }
    }

    fn original_selected_compiler_entry_fixture(
        source: &str,
        environment: &str,
    ) -> OriginalBodyFixture {
        let mut fixture = original_fixture(source, environment);
        fixture.entry.native_compilation.mode =
            tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject;
        fixture.bindings = SourceCommandBindings::analyse_with_options(
            source,
            fixture.input.lexer_config(),
            fixture.context.commands(),
            fixture.entry.options(),
        );
        let image = tcl_lexer::SourceImage::document(source);
        fixture.tokens = crate::segmenter::segment_commands_image_with_offset_and_config(
            &image,
            0,
            fixture.input.lexer_config(),
        )
        .unwrap()
        .iter()
        .map(|segment| {
            let mut tokens = CommandTokens::from_segmented(
                &image.source_map(),
                fixture.input.lexer_config(),
                segment,
            );
            fixture.bindings.stamp_original_tokens(&mut tokens);
            tokens
        })
        .collect();
        fixture
    }

    #[test]
    fn original_known_compiler_branch_and_finally_keep_the_selected_frame() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // The genuine held compiler entry selects BytecodeObject independently
        // of namespace syntax. No Direct handler or absent Jim compiler is inferred.
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let fixture =
                original_selected_compiler_entry_fixture("if {1} {set selected YES}", environment);
            let body = body_at(&fixture.tokens[0], 2, fixture.input.lexer_config());
            let expected = SourceNamespaceKey::from_native_entry(
                fixture.entry.native_entry.as_deref().unwrap(),
            )
            .unwrap();
            assert_eq!(
                fixture.bindings.executed_script_namespace_context(&body),
                Some(expected)
            );
            assert!(
                fixture
                    .bindings
                    .entered_scripts
                    .values()
                    .flat_map(|arguments| arguments.values().flatten())
                    .any(|entry| entry.source == body && entry.frame.is_some())
            );
        }
        for environment in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let fixture = original_selected_compiler_entry_fixture(
                "try {error caught} finally {set selected YES}",
                environment,
            );
            let body = body_at(&fixture.tokens[0], 3, fixture.input.lexer_config());
            let expected = SourceNamespaceKey::from_native_entry(
                fixture.entry.native_entry.as_deref().unwrap(),
            )
            .unwrap();
            assert_eq!(
                fixture.bindings.executed_script_namespace_context(&body),
                Some(expected)
            );
        }
    }
}

/// Namespace syntax from the actual source scope and selected original body.
/// Relative caller frames cannot borrow the declared procedure's namespace.
pub(crate) fn original_logical_frame_body_namespace_for_module(
    module: &crate::ir::Module,
    registry: &CommandRegistry,
    parent: &CommandTokens,
    body: &crate::registry_invocation::OriginalSourceScriptBody,
) -> Option<SourceNamespaceKey> {
    let input = module.source_metadata_input.as_ref()?;
    let words =
        crate::registry_invocation::source_structure::original_logical_registry_words_for_module(
            module, registry, parent,
        )?;
    if body.source_words() != &words {
        return None;
    }
    let binding = parent.source_binding.as_ref()?;
    if binding.has_original_source_layout_owner() {
        return binding.original_logical_frame_body_namespace(registry, input, parent);
    }
    let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
        advice,
    ) = words.source()
    else {
        return None;
    };
    let scope = advice.logical_source_scope()?;
    if !scope.matches_input(input)
        || !scope.owns_words(advice.original_words())
        || !body.matches_source(&module.source, input.lexer_config())
        || !body.matches_context(input.borrowed_context_registry())
    {
        return None;
    }
    let (layout, arguments) =
        words.with_source_schema(input.borrowed_context_registry(), |schema| {
            Some((
                schema.semantics.frame_effect?.layout,
                schema.authored_logical_source_frame_argument_layout()?,
            ))
        })??;
    let tcl_registry::frame_effect::FrameArgumentResolution::Valid { level, .. } = arguments else {
        return None;
    };
    let body = std::sync::Arc::new(body.clone());
    let selected = match layout {
        tcl_registry::FrameArgLayout::ScriptInCurrentFrame => scope.with_body(body)?,
        tcl_registry::FrameArgLayout::ScriptInSelectedFrame if level.is_current_frame() => {
            scope.with_body(body)?
        }
        tcl_registry::FrameArgLayout::ScriptInSelectedFrame if level.is_global_frame() => {
            scope.with_global_body(body)?
        }
        _ => return None,
    };
    tcl_syntax::naming::checked_namespace_path_key(selected.namespace_recipe())
        .map(SourceNamespaceKey::authored)
}
