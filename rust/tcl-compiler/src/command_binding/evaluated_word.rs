// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Freeze each reached template component before the next component executes.

use super::{
    Arc, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceOutcomes,
};
use crate::ir::WordPart;

impl SourceCommandBindings {
    pub(super) fn walk_template_substitutions(
        &mut self,
        word: &crate::ir::WordExpr,
        parts: &[WordPart],
        document: &str,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut outcomes = SourceOutcomes::normal(state);
        let mut value = Some(String::new());
        let native = super::original_name_value::original_native_word(word, state, context.config);
        // This closes evaluation of source text only. The independent literal
        // world excludes custom object methods; value bytes do not close a
        // substituted object's rendering or any command handler.
        // Implementation contract: naming.source.original-jim-text-evaluation
        // docs/design/analysis/name-resolution-proofs/original-jim-text-evaluation.md
        let static_text_complete = native.is_some()
            && parts
                .iter()
                .all(|part| matches!(part, WordPart::Text { .. }))
            && state.ordinary_literal_pool.as_ref().is_some_and(
                super::literal_object_pool::SourceOrdinaryLiteralPool::text_effects_current,
            );
        let mut native_value: Option<super::original_name_value::OriginalProducedNameValue> = None;
        let mut native_complete = true;
        let mut index = native
            .as_ref()
            .and_then(|word| EvaluatedVariableIndex::new(word, &state.source_variables));
        for part in parts {
            let Some(continuing) = outcomes.normal.take() else {
                break;
            };
            *state = *continuing;
            let next = match part {
                WordPart::Variable { spelling, source } => self.observe_variable_substitution(
                    spelling, source, document, base, state, context,
                ),
                WordPart::CommandSubstitution { spelling, source } => {
                    let script = spelling
                        .strip_prefix('[')
                        .and_then(|text| text.strip_suffix(']'))
                        .unwrap_or(spelling);
                    self.walk_source(script, source.span.start() + 1, state, &context)
                }
                _ => SourceOutcomes::normal(state),
            };
            let component = template_text_component(part, &next, context.config);
            let native_component =
                template_native_component(part, &next, native.as_ref(), &state.source_variables);
            if let Some(index) = &mut index {
                index.observe(part, &next, &state.source_variables);
            }
            if native_complete {
                native_value = match (native_value, native_component) {
                    (None, Some(component)) => Some(component),
                    (Some(value), Some(component)) => value.concatenated(&component),
                    _ => None,
                };
                native_complete = native_value.is_some();
            }
            value = value.zip(component).map(|(mut value, component)| {
                value.push_str(&component);
                value
            });
            outcomes.join(&next);
        }
        outcomes.normal_name_value = outcomes.normal.as_ref().and_then(|normal| {
            let mut value = if parts.is_empty() {
                super::original_name_value::capture_word(word, normal, context.config)
            } else {
                native_value
            }?;
            if let Some(index) = index.and_then(EvaluatedVariableIndex::finish) {
                value = value.with_evaluated_variable_index(
                    index.0,
                    index.1,
                    index.2,
                    &normal.source_variables,
                )?;
            }
            value
                .is_current(&normal.source_variables)
                .then(|| Arc::new(value))
        });
        outcomes.normal_representation = None;
        outcomes.normal_rhs_read = None;
        outcomes.normal_value = outcomes.normal.as_ref().and(value).map(|text| {
            Arc::new(super::native_result::EvaluatedSourceValue {
                text,
                representation: tcl_syntax::value::ValueRepresentation::Unknown,
                numeric: None,
            })
        });
        if static_text_complete
            && outcomes.normal_name_value.is_some()
            && outcomes.abrupt.is_empty()
        {
            outcomes.retain_complete_normal_evaluation();
        }
        outcomes.publish(state);
        outcomes
    }
}

fn template_text_component(
    part: &WordPart,
    outcomes: &SourceOutcomes,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    match part {
        WordPart::Text { text, .. } => {
            let bytes = tcl_syntax::backslash::decode_bytes_in(text.as_bytes(), config.escapes);
            std::str::from_utf8(&bytes).ok().map(str::to_owned)
        }
        WordPart::Variable { .. } | WordPart::CommandSubstitution { .. } => outcomes
            .normal_value
            .as_ref()
            .map(|value| value.text.clone()),
        WordPart::Opaque { .. } => None,
    }
}

fn template_native_component(
    part: &WordPart,
    outcomes: &SourceOutcomes,
    native: Option<&tcl_lexer::NativeWord>,
    context: &crate::var_resolve::ResolveContext,
) -> Option<super::original_name_value::OriginalProducedNameValue> {
    match part {
        WordPart::Text { source, .. } => {
            let policy = context
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe);
            native.zip(policy).and_then(|(native, policy)| {
                let fragment = crate::signature_scan::scope::SignatureSourceNameValue::from_original_text_fragment(native, source.span, policy)?;
                super::original_name_value::OriginalProducedNameValue::from_source_input(
                    &crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(fragment),
                    context,
                )
            })
        }
        WordPart::Variable { .. } | WordPart::CommandSubstitution { .. } => {
            outcomes.normal_name_value.as_deref().cloned()
        }
        WordPart::Opaque { .. } => None,
    }
}

/// Collect values while the existing evaluator visits the corresponding
/// original components once. Index text may omit the literal root wrappers;
/// substituted components must match the arena's exact source token extent.
struct EvaluatedVariableIndex {
    word: tcl_lexer::NativeWord,
    arena: tcl_lexer::ExecutablePartArena,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    next: usize,
    value: Option<super::original_name_value::OriginalProducedNameValue>,
    closed: bool,
}

impl EvaluatedVariableIndex {
    fn new(
        word: &tcl_lexer::NativeWord,
        context: &crate::var_resolve::ResolveContext,
    ) -> Option<Self> {
        let policy = context.execution_name_policy?.native_recipe()?;
        let tcl_syntax::naming::NativeNameProtocol::C(version) = policy.recipe() else {
            return None;
        };
        let tcl_syntax::native_variable_words::NativeVariableWordOperand::CompoundArray {
            index,
            ..
        } = tcl_syntax::native_variable_words::native_variable_word(
            word,
            version,
            policy.string_protocol(),
        )
        .ok()?
        else {
            return None;
        };
        Some(Self {
            word: word.clone(),
            arena: index,
            policy,
            next: 0,
            value: None,
            closed: true,
        })
    }

    fn observe(
        &mut self,
        part: &WordPart,
        outcomes: &SourceOutcomes,
        context: &crate::var_resolve::ResolveContext,
    ) {
        if !self.closed {
            return;
        }
        let source = match part {
            WordPart::Text { source, .. }
            | WordPart::Variable { source, .. }
            | WordPart::CommandSubstitution { source, .. }
            | WordPart::Opaque { source, .. } => source,
        };
        if source.provenance != crate::ir::Provenance::Source {
            self.closed = false;
            return;
        }
        while let Some(component) = self.arena.list(self.arena.root()).get(self.next) {
            let Some(span) = self.arena.source_span(component) else {
                self.closed = false;
                return;
            };
            if span.start() >= source.span.end() {
                break;
            }
            if span.end() <= source.span.start() {
                self.closed = false;
                return;
            }
            let value = match (&component.part, part) {
                (tcl_lexer::ExecutablePart::Text(_), WordPart::Text { .. })
                    if source.span.start() <= span.start() && span.end() <= source.span.end() => {
                    crate::signature_scan::scope::SignatureSourceNameValue::from_original_executable_text_fragment(
                        &self.arena, self.word.image(), self.word.config(), component.span, self.policy,
                    ).and_then(|value| super::original_name_value::OriginalProducedNameValue::from_source_input(
                        &crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(value), context,
                    ))
                }
                (tcl_lexer::ExecutablePart::Variable { .. }, WordPart::Variable { .. })
                | (tcl_lexer::ExecutablePart::Command { .. }, WordPart::CommandSubstitution { .. })
                    if span == source.span => outcomes.normal_name_value.as_deref().cloned(),
                _ => None,
            };
            self.value = match (self.value.take(), value) {
                (None, Some(value)) => Some(value),
                (Some(previous), Some(value)) => previous.concatenated(&value),
                _ => None,
            };
            if self.value.is_none() {
                self.closed = false;
                return;
            }
            self.next += 1;
        }
    }

    fn finish(
        self,
    ) -> Option<(
        tcl_lexer::NativeWord,
        tcl_lexer::ExecutablePartArena,
        super::original_name_value::OriginalProducedNameValue,
    )> {
        (self.closed && self.next == self.arena.list(self.arena.root()).len()).then_some(())?;
        Some((self.word, self.arena, self.value?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_expression_prepares_the_frozen_components() {
        let source = r#"set a 3; set b 4; expr "$a + $b"; set done 1"#;
        let registry = tcl_registry::CommandRegistry::build_default();
        let bindings =
            SourceCommandBindings::analyse(source, tcl_lexer::LexerConfig::default(), &registry);
        let script = super::super::ExecutedScriptSource::contiguous(
            Arc::clone(bindings.source_origin().unwrap()),
            source,
            0,
        )
        .unwrap();
        let prepared = bindings.expression_preparations_for_script(&script);
        assert_eq!(prepared.len(), 1);
        assert_eq!(prepared[0].witness.source(), "3 + 4");
        assert_ne!(prepared[0].source.origin, script.origin);
        let done = u32::try_from(source.find("set done").unwrap()).unwrap();
        assert!(
            bindings
                .invocation_at_source("set", done)
                .proved_execution_target()
                .is_some()
        );
    }

    #[test]
    fn original_static_template_completion_requires_its_literal_world() {
        // Implementation contract: naming.source.original-jim-text-evaluation
        // docs/design/analysis/name-resolution-proofs/original-jim-text-evaluation.md
        // Implementation contract: naming.source.static-template-normal-completion (docs/design/analysis/name-resolution-proofs/static-template-normal-completion.md).
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let owner = tcl_registry::model::ingress::static_context_for(engine);
            let registry = owner.commands();
            let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
            for source in [r"list p\uD800", r#"list "p\uD800""#, r#"list """#] {
                let bindings = SourceCommandBindings::analyse(source, config, registry);
                assert!(
                    bindings.points.iter().any(|point| point.offset == 0
                        && point.dispatch
                        && point.original_arguments_complete_normally),
                    "{engine}/{source}"
                );
                let binding = bindings.invocation_at_source("list", 0);
                assert!(binding.original_retained_written_name_input(1).is_some());
            }
            let source = r#"list "$unknown""#;
            let bindings = SourceCommandBindings::analyse(source, config, registry);
            assert!(!bindings.points.iter().any(|point| point.offset == 0
                && point.dispatch
                && point.original_arguments_complete_normally));
            let source = r#"list "p\uD800""#;
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                config,
                registry,
                super::super::SourceAnalysisOptions {
                    unknown_entry: true,
                    ..Default::default()
                },
            );
            assert!(!bindings.points.iter().any(|point| point.offset == 0
                && point.dispatch
                && point.original_arguments_complete_normally));
        }
    }

    const ORIGINAL_TEXT_SOURCE_CONTROLS: &[(&str, &str)] = &[
        (
            "tcl8.4",
            include_str!("../../tests/data/native_original_jim_text/8.4.20/stdout.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_original_jim_text/8.5.19/stdout.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_original_jim_text/8.6.18/stdout.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_original_jim_text/9.0.4/stdout.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_original_jim_text/9.1.0/stdout.tsv"),
        ),
        (
            "jim",
            include_str!("../../tests/data/native_original_jim_text/jim/stdout.tsv"),
        ),
    ];

    #[test]
    fn original_text_evaluation_matches_exact_native_source_controls() {
        // Native observation: naming.source.static-original-text-controls
        // docs/design/analysis/name-resolution-proofs/static-original-text-controls.md
        // Exact Direct C/Jim source rows test produced bytes and completion;
        // no physical object, C pool or compiler admission is compared.
        let decode = |hex: &str| -> Vec<u8> {
            let (pairs, suffix) = hex.as_bytes().as_chunks::<2>();
            assert!(suffix.is_empty());
            pairs
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        };
        for &(engine, fixture) in ORIGINAL_TEXT_SOURCE_CONTROLS {
            let owner = tcl_registry::model::ingress::static_context_for(engine);
            let registry = owner.commands();
            let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
            let mode = if engine == "jim" {
                "JIM_SOURCE"
            } else {
                "DIRECT_FLAG"
            };
            let prefix = format!("INPUT_{mode}_");
            let mut controls = 0;
            for row in fixture.lines().filter(|row| row.starts_with(&prefix)) {
                let (label, input) = row.split_once('|').unwrap();
                let (_, encoded) = input.split_once('|').unwrap();
                let source = String::from_utf8(decode(encoded)).unwrap();
                let case = label.strip_prefix(&prefix).unwrap();
                let result_label = format!("{mode}_{case}_RESULT|");
                let result = fixture
                    .lines()
                    .find_map(|line| line.strip_prefix(&result_label))
                    .unwrap();
                let (code, bytes) = result.split_once('|').unwrap();
                let expected = decode(bytes);
                let bindings = SourceCommandBindings::analyse_with_options(
                    &source,
                    config,
                    registry,
                    super::super::SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            registry.profile().unwrap(),
                        )),
                        native_compilation:
                            crate::environment_ingress::authoring_native_compilation(),
                        ..Default::default()
                    },
                );
                controls += 1;
                if case.starts_with("UNKNOWN_") {
                    assert_eq!(code, "1", "{engine}/{case}");
                    assert!(
                        bindings.original_completed_command_world().is_none(),
                        "{engine}/{case}"
                    );
                    continue;
                }
                assert_eq!(code, "0", "{engine}/{case}");
                assert!(
                    bindings.original_completed_command_world().is_some(),
                    "{engine}/{case}"
                );
                if case == "OPAQUE_PROC_HEAD" {
                    let offset = u32::try_from(source.rfind("p\\uD800 VALUE").unwrap()).unwrap();
                    let map = tcl_lexer::SourceMap::new(&source);
                    let command = crate::segmenter::segment_commands_with_offset_and_config(
                        &source, 0, config,
                    )
                    .into_iter()
                    .find(|command| command.span.start() == offset)
                    .unwrap();
                    let mut tokens =
                        crate::ir::CommandTokens::from_segmented(&map, config, &command);
                    bindings.stamp_original_tokens(&mut tokens);
                    let binding = bindings.invocation_at_source("", offset);
                    assert_eq!(
                        binding
                            .original_normal_result(&tokens)
                            .unwrap()
                            .text()
                            .as_bytes(),
                        expected
                    );
                } else {
                    let binding = bindings.invocation_at_source("set", 0);
                    assert_eq!(
                        binding
                            .original_retained_written_name_input(2)
                            .unwrap()
                            .bytes(),
                        expected
                    );
                    assert!(
                        bindings.points.iter().any(|point| point.offset == 0
                            && point.dispatch
                            && point.original_arguments_complete_normally),
                        "{engine}/{case}"
                    );
                }
            }
            assert_eq!(controls, 6, "{engine}");
        }
    }

    #[test]
    fn later_template_substitution_cannot_change_an_earlier_component() {
        let source = r#"set x OLD; set value "$x[set x NEW]"; list $value"#;
        let registry = tcl_registry::CommandRegistry::build_default();
        let bindings =
            SourceCommandBindings::analyse(source, tcl_lexer::LexerConfig::default(), &registry);
        let offset = u32::try_from(source.find("list").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("list", offset);
        assert_eq!(
            binding.evaluated_argument_values,
            [Some("OLDNEW".to_owned())]
        );
    }

    #[test]
    fn script_admission_realm_depends_on_operand_evaluation_not_source_mapping() {
        use super::super::{CommandAllocationSite, SourceAnalysisOptions};
        use tcl_dialect::model::InvocationRealm;

        let registry_context = tcl_registry::model::ingress::static_context_for("f5-irules");
        let registry = registry_context.commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        for (source, realm) in [
            ("eval {gets ch c}", InvocationRealm::RuleLoader),
            (r#"eval "gets\ ch c""#, InvocationRealm::RuleLoader),
            ("eval {gets} {ch c}", InvocationRealm::RuleLoader),
            (
                "set script {gets ch c}; eval $script",
                InvocationRealm::InterpreterRuntime,
            ),
            ("eval [list gets ch c]", InvocationRealm::InterpreterRuntime),
        ] {
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                config,
                registry,
                SourceAnalysisOptions {
                    invocation_dialect: registry
                        .profile()
                        .map(tcl_registry::InvocationDialect::of_profile),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..crate::environment_ingress::authoring_native_compilation()
                        },
                    ..SourceAnalysisOptions::default()
                },
            );
            let site = CommandAllocationSite {
                source: Arc::clone(bindings.source_origin().unwrap()),
                offset: u32::try_from(source.rfind("eval").unwrap()).unwrap(),
            };
            let script = bindings.executed_script_at(&site, 0).expect(source);
            assert_eq!(script.text.try_text().unwrap(), "gets ch c", "{source}");
            let child = bindings.invocation_at_origin(&script.origin, script.base());
            assert_eq!(child.invocation_realm(), Some(realm), "{source}");
            if realm == InvocationRealm::InterpreterRuntime {
                assert!(child.proved_target().is_some(), "{source}");
            }
        }
    }
}
