// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A selected Logical procedure definition changes only the authored model.

use super::{
    Arc, BindingKind, CommandAllocationSite, MayBinding, ModuleCommandBindings,
    ResolvedCommandTarget, SourceCommandKey, SourceCommandTarget, SourceExecutionContext,
    SourceNamespaceKey, SourceOutcomes, source_binding_projection,
};
use crate::analyser::ResolvedAnalysisInput;
use tcl_lexer::{NativeWord, SourceChannel};

struct LogicalProcedureDefinition {
    input: ResolvedAnalysisInput,
    site: CommandAllocationSite,
    words: Vec<NativeWord>,
    values: Vec<super::logical_word_values::LogicalOriginalWordValue>,
    namespace: SourceNamespaceKey,
    target: SourceCommandTarget,
    key: SourceCommandKey,
}

pub(super) fn transfer(
    words: &[crate::ir::WordExpr],
    target: &SourceCommandTarget,
    offset: u32,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<SourceOutcomes> {
    // naming.source.logical-procedure-definition-model
    // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
    if let Some(definition) =
        LogicalProcedureDefinition::capture(words, target, offset, state, context)
    {
        return definition.apply(state, context);
    }
    super::logical_operation::transfer(words, target, offset, state, context)
}

/// The prepared argv boundary may select an authored model handler without
/// a Native compiler or head input. Exact quiet source capture stays mandatory.
pub(super) fn transfer_selected(
    words: &[crate::ir::WordExpr],
    offset: u32,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<SourceOutcomes> {
    // naming.source.logical-procedure-definition-model
    // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
    let original = original_invocation_words(words, offset, state, context)?;
    let head = static_value(original.first()?)?;
    let selected = source_binding_projection(state, &head, &context.namespace_identity());
    if selected.unknown || selected.may_be_absent {
        return None;
    }
    let [target] = selected.targets.as_slice() else {
        return None;
    };
    transfer(words, target, offset, state, context)
}

impl LogicalProcedureDefinition {
    fn capture(
        words: &[crate::ir::WordExpr],
        target: &SourceCommandTarget,
        offset: u32,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let input = logical_entry(state, context)?;
        let origin = Arc::clone(state.current_source_origin.as_ref()?);
        if origin.source_image().channel() != SourceChannel::Document
            || offset != context.invocation_offset
        {
            return None;
        }
        let values = super::logical_word_values::LogicalOriginalWordValue::capture_invocation(
            words, offset, state, context,
        )?;
        let original: Vec<_> = values.iter().map(|value| value.word().clone()).collect();
        let head = static_value(original.first()?)?;
        let namespace = context.namespace_identity();
        if !selected_target(state, &head, &namespace, target) {
            return None;
        }
        let actual_context = input.context_registry();
        let mut effective_values = vec![target.command.clone()];
        if !target.prepended.is_empty() {
            target
                .logical_original_prefix
                .as_ref()?
                .matches_state(state)
                .then_some(())?;
            effective_values.extend(
                target
                    .original_logical_prepended_name_inputs(
                        input,
                        origin.source_image(),
                        context.config,
                    )?
                    .iter()
                    .map(|value| std::str::from_utf8(value.bytes()).ok().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()?,
            );
        }
        effective_values.extend(values.iter().skip(1).map(|value| value.value().to_owned()));
        let arguments = effective_values
            .iter()
            .skip(1)
            .map(|value| tcl_registry::InvocationWord::Literal(value))
            .collect::<Vec<_>>();
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                actual_context.commands(),
                Some(actual_context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&target.command),
                    &arguments,
                )
                .with_dialect(super::source_analysis_entry::source_input_dialect(input)),
                context.realm,
            );
        let schema = resolution.resolved()?;
        let shape = schema.authored_source_procedure_arguments()?;
        static_formals(
            effective_values.get(shape.parameters + 1)?,
            context.config.list_parse,
        )?;
        let key = source_procedure_key(state, &namespace, effective_values.get(shape.name + 1)?)?;
        Some(Self {
            input: input.clone(),
            site: CommandAllocationSite {
                source: origin,
                offset,
            },
            words: original,
            values,
            namespace,
            target: target.clone(),
            key,
        })
    }

    fn apply(
        self,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        if logical_entry(state, context) != Some(&self.input)
            || state.current_source_origin.as_ref() != Some(&self.site.source)
            || context.invocation_offset != self.site.offset
            || context.namespace_identity() != self.namespace
            || self.words.iter().any(|word| {
                word.image() != self.site.source.source_image() || word.config() != context.config
            })
            || self.values.len() != self.words.len()
            || self
                .values
                .iter()
                .zip(&self.words)
                .any(|(value, word)| value.word() != word || !value.is_current(state, context))
            || !selected_target(
                state,
                &static_value(self.words.first()?)?,
                &self.namespace,
                &self.target,
            )
            || !state.namespaces.contains(self.key.holder().as_ref())
        {
            return None;
        }
        let label = self.key.authored_spelling()?.to_owned();
        state.extend_procedure_bodies([label.clone()]);
        state.record_proc_rebound_candidates(
            &label,
            &crate::ir_helpers::ExecutionNamespace::SourceContext(self.namespace),
        );
        state.install(
            self.key,
            MayBinding::Target(ResolvedCommandTarget {
                command: label,
                prepended: Vec::new(),
                registry_backed: false,
                kind: BindingKind::Proc,
                implementation_generation: self.site.offset,
                implementation_allocation: None,
                terminal: true,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: None,
            }),
        );
        // This is a continuation of the positively selected authored model.
        // It carries neither a Native publication nor a Normal certificate.
        Some(SourceOutcomes::normal(state))
    }
}

pub(super) fn logical_entry<'a>(
    state: &'a ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<&'a ResolvedAnalysisInput> {
    let input = state.logical_source_name_advice_input()?;
    if !input.has_logical_source_name_context()
        || input.lexer_config() != context.config
        || state.baseline.native_entry.is_some()
        || state.baseline.unknown_entry
        || state.baseline.execution_name_policy.is_some()
        || state.baseline.hosted_execution_context.is_some()
        || state.has_opaque_domain()
        || !state.command_observers.is_quiet()
        || !state.source_variables.namespace_known
        || !matches!(
            context.namespace_identity(),
            SourceNamespaceKey::Authored(_)
        )
        || !state.namespaces.contains(&context.namespace_identity())
        || input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            != context.registry.snapshot().semantic_key()
        || state.baseline.dialect != Some(super::source_analysis_entry::source_input_dialect(input))
    {
        return None;
    }
    Some(input)
}

fn selected_target(
    state: &ModuleCommandBindings,
    head: &str,
    namespace: &SourceNamespaceKey,
    target: &SourceCommandTarget,
) -> bool {
    if !matches!(target.kind, BindingKind::Builtin | BindingKind::Alias) || !target.registry_backed
    {
        return false;
    }
    let selected = source_binding_projection(state, head, namespace);
    !selected.unknown
        && !selected.may_be_absent
        && selected.targets.as_slice() == std::slice::from_ref(target)
}

/// Capture one complete quiet original vector in the retained Logical domain.
/// Authored model operands grant neither Native values nor execution receipts.
pub(super) fn original_invocation_words(
    words: &[crate::ir::WordExpr],
    offset: u32,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Vec<NativeWord>> {
    logical_entry(state, context)?;
    let origin = state.current_source_origin.as_ref()?;
    if origin.source_image().channel() != SourceChannel::Document
        || !matches!(origin.kind(), super::SourceOriginKind::Authored(_))
        || offset != context.invocation_offset
        || state.variable_frame != *context.frame
        || state.source_variables.namespace != context.namespace
    {
        return None;
    }
    let original = crate::registry_invocation::original_native_compiler_words(
        origin.source_image(),
        words,
        offset,
        context.config,
    )?;
    Some(original)
}

pub(super) fn original_static_invocation_words(
    words: &[crate::ir::WordExpr],
    offset: u32,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Vec<NativeWord>> {
    let original = original_invocation_words(words, offset, state, context)?;
    original
        .iter()
        .all(|word| static_value(word).is_some())
        .then_some(original)
}

pub(super) fn static_value(word: &NativeWord) -> Option<String> {
    let bytes = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
    static_bytes_text(&bytes)
}

fn static_bytes_text(bytes: &[u8]) -> Option<String> {
    if bytes.contains(&0) {
        return None;
    }
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

pub(super) fn static_formals(
    text: &str,
    syntax: tcl_dialect::ListParse,
) -> Option<Vec<tcl_syntax::formal_params::FormalParameter>> {
    // Proof: naming.source.logical-procedure-definition-model
    // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
    // The source model uses shared strict formal validity. The selected list
    // parser must agree at both levels; this is no Native parameter recipe.
    let parameters = tcl_syntax::formal_params::parse_formal_parameters(text).ok()?;
    matching_list_elements(text, syntax)?;
    for specifier in tcl_syntax::list::split_list(text).ok()? {
        matching_list_elements(&specifier, syntax)?;
    }
    Some(parameters)
}

/// Map names in the independently selected Logical recipe to original list
/// geometry. Escaped parent values keep names but withhold source extents.
pub(super) fn formal_name_sources(
    original: Option<&NativeWord>,
    text: &str,
    parameters: &[tcl_syntax::formal_params::FormalParameter],
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<crate::signature_scan::formal_parameters::OriginalSourceFormalName>> {
    use crate::signature_scan::formal_parameters::OriginalSourceFormalName;
    let syntax = config.list_parse;
    if static_formals(text, syntax)?.as_slice() != parameters {
        return None;
    }
    let content = original.and_then(|word| {
        let span = word.content_span().ok()?;
        let raw = word.image().bytes().get(span.as_range())?;
        (raw == text.as_bytes()).then_some(span)
    });
    let specifiers = tcl_syntax::word_rules::WordValueRules::from_config(&config)
        .split_list(text)
        .ok()?;
    let mut next = 0;
    let mut result = Vec::with_capacity(parameters.len());
    for (formal, specifier) in parameters.iter().zip(specifiers) {
        let outer = tcl_syntax::list::find_element_with_syntax(text, next, syntax).ok()??;
        next = outer.next;
        let inner = tcl_syntax::list::find_element_with_syntax(&specifier, 0, syntax).ok()??;
        let span = content.and_then(|content| {
            if !outer.literal {
                return None;
            }
            let start = outer.value.start.checked_add(inner.value.start)?;
            let end = outer.value.start.checked_add(inner.value.end)?;
            Some(tcl_lexer::Span::new(
                content.start().checked_add(u32::try_from(start).ok()?)?,
                content.start().checked_add(u32::try_from(end).ok()?)?,
            ))
        });
        let quoted = span.zip(original).is_some_and(|(span, word)| {
            OriginalSourceFormalName::source_is_quoted(word.image(), span)
        });
        result.push(OriginalSourceFormalName::new(
            formal.name.as_bytes().to_vec(),
            span,
            quoted,
        ));
    }
    (result.len() == parameters.len()).then_some(result)
}

/// Construct a conditional authored declaration slot in its retained namespace.
/// This does not invoke the native publication recipe or allocate a command.
pub(super) fn source_procedure_key(
    state: &ModuleCommandBindings,
    namespace: &SourceNamespaceKey,
    name: &str,
) -> Option<SourceCommandKey> {
    state.logical_source_name_advice_input()?;
    let SourceNamespaceKey::Authored(current) = namespace else {
        return None;
    };
    let key = SourceCommandKey::authored(tcl_syntax::naming::qualify(current, name));
    state
        .namespaces
        .contains(key.holder().as_ref())
        .then_some(key)
}

fn matching_list_elements(text: &str, syntax: tcl_dialect::ListParse) -> Option<()> {
    let mut strict_next = 0;
    let mut selected_next = 0;
    loop {
        let strict = tcl_syntax::list::find_element(text, strict_next).ok()?;
        let selected =
            tcl_syntax::list::find_element_with_syntax(text, selected_next, syntax).ok()?;
        match (strict, selected) {
            (None, None) => return Some(()),
            (Some(strict), Some(selected))
                if strict.value == selected.value
                    && strict.literal == selected.literal
                    && strict.braced == selected.braced =>
            {
                strict_next = strict.next;
                selected_next = selected.next;
            }
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandSlotPresence};

    fn input() -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "logical-definition-model",
            &[],
            "Logical definition model",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )
        .intern();
        ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        )
    }

    fn formal_source_input() -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        assert!(input.has_logical_source_name_context());
        assert!(
            tcl_registry::InvocationDialect::of_profile(profile)
                .authored_name_policy()
                .is_some()
        );
        input
    }

    #[test]
    fn logical_formal_source_geometry_survives_same_labels_and_missing_reports() {
        let input = formal_source_input();
        let context = input.context_registry();
        let registry = context.commands();
        let source = r#"proc p {first {"as" ""} {third value}} {return FIRST}
proc p "other {last default}" {return SECOND}"#;
        let mut analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, input.analyser_profile().name);
        let declarations = analysis
            .original_procedure_declarations()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(declarations.len(), 2);
        analysis.all_procs.clear();
        analysis.superseded_procs.clear();
        for (declaration, expected) in declarations
            .iter()
            .zip([vec!["first", "as", "third"], vec!["other", "last"]])
        {
            assert!(
                analysis
                    .original_procedure_formals(declaration, registry)
                    .is_none()
            );
            let fields = analysis
                .original_procedure_formal_name_sources(declaration, registry)
                .expect("independent actual Logical source recipe");
            assert_eq!(
                fields.iter().map(|field| field.name()).collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(|name| name.as_bytes())
                    .collect::<Vec<_>>()
            );
            for (field, name) in fields.iter().zip(expected) {
                assert_eq!(&source[field.source_span().unwrap().as_range()], name);
                assert_eq!(field.quoted_marker(), name == "as");
            }
        }
        let mut foreign = tcl_registry::CommandRegistry::build_default();
        let mut added = foreign.get("set").unwrap().clone();
        added.name = "foreign_formal_probe";
        foreign.insert(added);
        assert!(
            analysis
                .original_procedure_formal_name_sources(&declarations[0], &foreign)
                .is_none()
        );
        analysis.resolved_input = None;
        assert!(
            analysis
                .original_procedure_formal_name_sources(&declarations[0], registry)
                .is_none()
        );
    }

    #[test]
    fn escaped_logical_formal_parent_keeps_names_without_fabricating_spans() {
        let input = formal_source_input();
        let context = input.context_registry();
        let registry = context.commands();
        let source = r#"proc p "first\x20second" {return VALUE}"#;
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, input.analyser_profile().name);
        let declaration = analysis.original_procedure_declarations().next().unwrap();
        let fields = analysis
            .original_procedure_formal_name_sources(declaration, registry)
            .unwrap();
        assert_eq!(
            fields.iter().map(|field| field.name()).collect::<Vec<_>>(),
            [b"first".as_slice(), b"second".as_slice()]
        );
        assert!(fields.iter().all(|field| field.source_span().is_none()));
        assert!(
            analysis
                .original_procedure_formals(declaration, registry)
                .is_none()
        );
    }

    fn final_presence(source: &str, input: &ResolvedAnalysisInput) -> SourceCommandSlotPresence {
        assert!(input.has_logical_source_name_context());
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, input.analyser_profile().name);
        let realm = analysis.retained_command_realm().unwrap();
        assert!(realm.matches_resolved_analysis_input(input));
        assert_eq!(
            realm
                .source_bindings_ref()
                .original_logical_source_name_advice_input(),
            Some(input)
        );
        let offset = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .last()
        .unwrap()
        .span
        .start();
        let binding = realm.invocation_at_source("", offset);
        if binding.invocation_site().is_some() {
            assert_eq!(binding.logical_source_name_advice_input(), Some(input));
        } else {
            // A preceding unknown/abrupt operation may leave this written
            // call unrepresented. The retained realm keeps the source input;
            // it does not manufacture a reached lookup at the missing point.
            assert!(binding.unknown);
            assert!(binding.targets.is_empty());
        }
        assert!(binding.original_recorded_head_name_input().is_none());
        binding.selected_slot_diagnostic_presence()
    }

    #[test]
    fn logical_definition_publishes_general_static_procedures_in_the_authored_model() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let input = input();
        for source in [
            "proc defined {} {}; defined",
            "proc defined {left {right default} args} {return $left}; defined",
            "proc first {value} {return $value}; proc second {a b} {return \"$a$b\"}; second",
            "proc ::defined {value} {not a valid body yet [}; ::defined",
        ] {
            assert_eq!(
                final_presence(source, &input),
                SourceCommandSlotPresence::Present,
                "{source}"
            );
        }
    }

    #[test]
    fn logical_definition_preserves_empty_names_in_existing_namespaces() {
        let input = input();
        for source in [
            "proc {} {} {return ROOT}; {}",
            "proc :: {} {return ROOT}; ::",
            "namespace eval n {proc {} {} {return INNER}}; n::",
            "proc {} {} {return ROOT}; rename {} moved; moved",
        ] {
            assert_eq!(
                final_presence(source, &input),
                SourceCommandSlotPresence::Present,
                "{source}"
            );
        }
    }

    #[test]
    fn logical_definition_keeps_unicode_moves_and_captured_factory_operands() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        // Original source values remain conditional authored data, without
        // Native publication, handler entry or successful allocation.
        let input = input();
        for source in [
            "proc café {} {}; café",
            "rename proc ::α; ::α café {} {}; café",
            "interp alias {} ::α {} proc; ::α café {} {}; café",
            "rename proc ::α; interp alias {} ::β {} ::α café {}; ::β {return café}; café",
            "interp alias {} ::造 {} proc café {}; ::造 {return café}; café",
            "proc café {} {}; rename café déplacé; déplacé",
        ] {
            assert_eq!(
                final_presence(source, &input),
                SourceCommandSlotPresence::Present,
                "{source}"
            );
        }
    }

    #[test]
    fn logical_static_values_keep_checked_source_and_native_channel_boundaries() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let project = |image: tcl_lexer::SourceImage| {
            let tokens = tcl_lexer::Lexer::with_source_image(&image, config)
                .tokenise_all()
                .unwrap();
            let commands = tcl_lexer::group_commands_bytes(&tokens, image.bytes(), config);
            let word =
                NativeWord::from_group(image, config, &tokens, &commands[0].words[0]).unwrap();
            static_value(&word)
        };
        assert_eq!(
            project(tcl_lexer::SourceImage::document("café")),
            Some("café".into())
        );
        assert_eq!(
            project(tcl_lexer::SourceImage::document(r"caf\u00e9")),
            Some("café".into())
        );
        for source in ["café\0", r"café\u0000", r"café\uD800", "$name", "[name]"] {
            assert_eq!(
                project(tcl_lexer::SourceImage::document(source)),
                None,
                "{source:?}"
            );
        }
        assert_eq!(
            project(tcl_lexer::SourceImage::native("café".as_bytes().to_vec())),
            None
        );
    }

    #[test]
    fn logical_definition_keeps_unrepresentable_names_distinct_from_literal_replacement() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        // Authored source lookup remains separate from Native string/name units.
        let input = input();
        for source in [
            "proc café� {} {}; café�",
            r"proc café\uFFFD {} {}; café�",
            r"proc {café\uD800} {} {}; {café\uD800}",
        ] {
            assert_eq!(
                final_presence(source, &input),
                SourceCommandSlotPresence::Present,
                "{source}"
            );
        }
        for source in [
            r"proc café\uD800 {} {}; café�",
            r"proc café\uD801 {} {}; café�",
            r"proc café� {} {}; café\uD800",
        ] {
            assert_ne!(
                final_presence(source, &input),
                SourceCommandSlotPresence::Present,
                "{source}"
            );
        }
    }

    #[test]
    fn logical_captured_values_keep_exact_unicode_bytes_and_refuse_unknown_units() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        use crate::registry_invocation::EffectiveInvocationWord;
        let bytes = EffectiveInvocationWord::ByteLiteral(Arc::from("café $literal".as_bytes()));
        assert_eq!(
            static_bytes_text(bytes.literal_bytes().unwrap()),
            Some("café $literal".to_owned())
        );
        assert_eq!(bytes.literal_bytes(), Some("café $literal".as_bytes()));
        assert_eq!(
            static_bytes_text(
                EffectiveInvocationWord::Literal("café $literal".into())
                    .literal_bytes()
                    .unwrap()
            ),
            static_bytes_text(bytes.literal_bytes().unwrap())
        );
        assert_eq!(
            static_bytes_text(
                EffectiveInvocationWord::ByteLiteral(Arc::from(&b""[..]))
                    .literal_bytes()
                    .unwrap()
            ),
            Some(String::new())
        );
        for value in [
            EffectiveInvocationWord::ByteLiteral(Arc::from(&b"caf\xff"[..])),
            EffectiveInvocationWord::ByteLiteral(Arc::from(&b"cafe\0tail"[..])),
            EffectiveInvocationWord::Literal("cafe\0tail".into()),
            EffectiveInvocationWord::Dynamic,
            EffectiveInvocationWord::Opaque,
            EffectiveInvocationWord::Expanded,
            EffectiveInvocationWord::KnownExpansion(vec!["café".into()]),
        ] {
            assert_eq!(
                value.literal_bytes().and_then(static_bytes_text),
                None,
                "{value:?}"
            );
        }
    }

    #[test]
    fn logical_definition_declines_unknown_rebound_dynamic_and_invalid_inputs() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let input = input();
        for source in [
            "mystery; proc defined {} {}; defined",
            "proc proc args {}; proc defined {} {}; defined",
            "proc $name {} {}; defined",
            "proc defined $formals {}; defined",
            "proc defined {} $body; defined",
            "proc defined {{value default extra}} {}; defined",
            "proc defined {{array(index)}} {}; defined",
            "proc missing::defined {} {}; missing::defined",
            "trace add execution proc enter observer; proc defined {} {}; defined",
        ] {
            assert_ne!(
                final_presence(source, &input),
                SourceCommandSlotPresence::Present,
                "{source}"
            );
        }
    }

    #[test]
    fn logical_model_roster_requires_the_complete_current_input_and_source_mode() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let input = input();
        let context = input.context_registry();
        let registry = context.commands();
        let options = SourceAnalysisOptions {
            logical_source_input: Some(&input),
            invocation_dialect: Some(super::super::source_analysis_entry::source_input_dialect(
                &input,
            )),
            ..SourceAnalysisOptions::default()
        };
        let initial = |options, config| {
            ModuleCommandBindings::initial_with_options(registry, options, Some(config))
        };
        let state = initial(options, input.lexer_config());
        assert_eq!(state.logical_source_name_advice_input(), Some(&input));
        assert!(state.baseline.semantics.binding_names().contains("::proc"));
        assert_eq!(state.baseline.dialect, options.invocation_dialect);
        assert!(state.baseline.execution_name_policy.is_none());
        assert!(state.baseline.native_entry.is_none());
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let stale_config = initial(options, config);
        assert!(stale_config.logical_source_name_advice_input().is_none());
        assert!(stale_config.baseline.semantics.binding_names().is_empty());
        let foreign_context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let foreign_input = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            foreign_context,
            input.lexer_config(),
        );
        let foreign = initial(
            SourceAnalysisOptions {
                logical_source_input: Some(&foreign_input),
                ..options
            },
            input.lexer_config(),
        );
        assert!(foreign.logical_source_name_advice_input().is_none());
        assert!(foreign.baseline.semantics.binding_names().is_empty());
        let native_profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let native_input = ResolvedAnalysisInput::new(
            native_profile,
            native_profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(native_profile)),
        );
        let native_options = SourceAnalysisOptions {
            logical_source_input: Some(&native_input),
            invocation_dialect: Some(super::super::source_analysis_entry::source_input_dialect(
                &native_input,
            )),
            ..SourceAnalysisOptions::default()
        };
        let native = initial(native_options, native_input.lexer_config());
        assert!(native.logical_source_name_advice_input().is_none());
        let expected_native = registry.effective_semantics_for_dialect_in_realm(
            native_options.invocation_dialect.unwrap(),
            tcl_dialect::model::InvocationRealm::InterpreterRuntime,
        );
        assert_eq!(
            native.baseline.semantics.binding_names(),
            expected_native.binding_names()
        );
    }

    #[test]
    fn logical_definition_receipt_rechecks_source_configuration_and_entry_without_native_normal() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let input = input();
        let context_registry = input.context_registry();
        let registry = context_registry.commands();
        let options = SourceAnalysisOptions {
            logical_source_input: Some(&input),
            invocation_dialect: Some(super::super::source_analysis_entry::source_input_dialect(
                &input,
            )),
            ..SourceAnalysisOptions::default()
        };
        let source = tcl_lexer::SourceImage::document("proc café {élan} {return $élan}");
        let origin = Arc::new(super::super::SourceOriginId::authored_image(source.clone()));
        let mut state = ModuleCommandBindings::initial_with_options(
            registry,
            options,
            Some(input.lexer_config()),
        );
        state.current_source_origin = Some(Arc::clone(&origin));
        let namespace = SourceNamespaceKey::authored("::");
        let frame = crate::var_resolve::VariableExecutionFrame::Global;
        let context = super::super::root_source_execution_context(
            &frame,
            "::",
            &namespace,
            input.lexer_config(),
            registry,
            options,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        );
        let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
            &source,
            0,
            context.config,
        )
        .unwrap();
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&source),
            context.config,
            &segments[0],
        );
        let selected = source_binding_projection(&state, "proc", &namespace);
        let target = selected
            .targets
            .first()
            .expect("sealed Logical stock definer");
        let proof = || {
            LogicalProcedureDefinition::capture(tokens.words(), target, 0, &state, context).unwrap()
        };
        let mut foreign_source = state.clone();
        foreign_source.current_source_origin =
            Some(Arc::new(super::super::SourceOriginId::authored_image(
                tcl_lexer::SourceImage::document("proc other {value} {return $value}"),
            )));
        assert!(proof().apply(&mut foreign_source, context).is_none());
        let mut changed_config = context.config;
        changed_config.strict_quoting = !changed_config.strict_quoting;
        assert!(
            proof()
                .apply(
                    &mut state.clone(),
                    SourceExecutionContext {
                        config: changed_config,
                        ..context
                    }
                )
                .is_none()
        );
        let mut unknown = state.clone();
        unknown.mark_opaque_binding_mutation();
        assert!(proof().apply(&mut unknown, context).is_none());
        let mut normal_state = state.clone();
        let outcomes = proof().apply(&mut normal_state, context).unwrap();
        assert!(outcomes.normal.is_some());
        assert!(outcomes.normal_completion.is_none());
        assert!(outcomes.native_normal_completion.is_none());
        assert_eq!(
            normal_state.original_command_world,
            state.original_command_world
        );
        assert!(
            matches!(normal_state.binding_alternatives(&SourceCommandKey::authored("::café")).iter().next(), Some(MayBinding::Target(target)) if target.kind == BindingKind::Proc && !target.registry_backed)
        );
        let mut unsealed = options;
        unsealed.logical_source_input = None;
        let state = ModuleCommandBindings::initial_with_options(
            registry,
            unsealed,
            Some(input.lexer_config()),
        );
        assert!(
            LogicalProcedureDefinition::capture(tokens.words(), target, 0, &state, context)
                .is_none()
        );
    }
}
