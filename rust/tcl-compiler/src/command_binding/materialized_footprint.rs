// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional materialized text footprints anchored at an original source lookup.

use super::{ModuleCommandBindings, SourceInvocationBinding, SourceNamespaceKey};
use crate::ir::CommandTokens;
use crate::registry_invocation::InvocationMetadataContext;
use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;

/// Readonly source lookup for possible writes in a materialized script value.
/// It supplies no authored child word/span, Native entry, frame or store.
pub(crate) struct OriginalSourceMaterializedFootprint<'a> {
    binding: &'a SourceInvocationBinding,
    state: &'a ModuleCommandBindings,
    namespace: &'a SourceNamespaceKey,
    registry: &'a CommandRegistry,
    metadata: InvocationMetadataContext<'a>,
    config: LexerConfig,
    head: tcl_lexer::NativeWord,
}

impl SourceInvocationBinding {
    /// Conditional original procedure allocations in the retained Logical
    /// model. Native handler/activation purposes consume their separate owners.
    pub(crate) fn original_logical_procedure_call_targets(
        &self,
        tokens: &CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<super::SourceCommandTarget>> {
        if !input.has_logical_source_name_context()
            || self.logical_source_name_advice_input() != Some(input)
            || self.original_lexer_config_for_tokens(tokens)?.normalized()
                != input.lexer_config().normalized()
        {
            return None;
        }
        let advice = self.original_declared_layout_advice(tokens, true)?;
        if !advice.closed_logical_source_lookup()
            || advice.targets().iter().any(|target| {
                target.kind != super::BindingKind::Proc
                    || target.implementation_allocation.is_none()
            })
        {
            return None;
        }
        Some(advice.targets().to_vec())
    }

    /// Retain this genuine installer's closed source lookup independently of
    /// the child script's text. Missing source, context or namespace refuses.
    pub(crate) fn original_materialized_footprint<'a>(
        &'a self,
        tokens: &CommandTokens,
        source: &str,
        registry: &'a CommandRegistry,
        metadata: Option<InvocationMetadataContext<'a>>,
    ) -> Option<OriginalSourceMaterializedFootprint<'a>> {
        // naming.diagnostic.original-materialized-write-footprint
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
        let metadata = metadata.filter(|metadata| metadata.matches_registry(registry))?;
        let input = metadata.source_analysis_input()?;
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let site = self.invocation_site()?;
        let snapshot = self.lookup_state.as_ref()?;
        let state = &snapshot.state;
        if self.unknown
            || self.may_be_absent
            || state.has_opaque_domain()
            || !state.command_observers.is_quiet()
            || state.source_step_observed()
            || state.current_source_origin.as_ref() != Some(&site.source)
            || site.source.source_image().try_text().ok() != Some(source)
            || state.baseline.registry_snapshot != Some(registry.snapshot().semantic_key())
            || config.normalized() != input.lexer_config().normalized()
            || state
                .baseline
                .logical_source_input
                .as_ref()
                .is_some_and(|original| original != input)
            || state
                .baseline
                .vendor_source_input
                .as_ref()
                .is_some_and(|original| original != input)
        {
            return None;
        }
        if state.baseline.logical_source_input.is_none()
            && state.baseline.vendor_source_input.is_none()
            && state.baseline.dialect
                != Some(super::source_analysis_entry::source_input_dialect(input))
        {
            return None;
        }
        let head = crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            config,
        )?
        .into_iter()
        .next()?;
        Some(OriginalSourceMaterializedFootprint {
            binding: self,
            state,
            namespace: &self.lookup_namespace_key,
            registry,
            metadata,
            config: config.nested(),
            head,
        })
    }
}

impl OriginalSourceMaterializedFootprint<'_> {
    /// Possible local output names, preserving current aliases and replacements.
    /// The materialized value receives no original child-source geometry.
    pub(crate) fn script_writes(&self, script: &str) -> crate::ir_helpers::VariableWriteEffects {
        crate::ir_helpers::script_value_possible_writes_with_metadata_context(
            script,
            self.registry,
            self.state,
            &crate::ir::ExecutionNamespace::SourceContext(self.namespace.clone()),
            Some(self.metadata),
            self.config,
        )
    }

    /// Possible local bindings and by-name reads in an original whole body.
    /// The selected interpreter must be the installer's current interpreter;
    /// named children require their independently retained command world.
    pub(crate) fn body_name_ownership(
        &self,
        body: &crate::registry_invocation::OriginalSourceScriptBody,
    ) -> crate::ir_helpers::VariableWriteEffects {
        let unavailable = || crate::ir_helpers::VariableWriteEffects {
            opaque: true,
            ..Default::default()
        };
        let Some(input) = self.metadata.source_analysis_input() else {
            return unavailable();
        };
        let context = input.context_registry();
        let words = body.source_words();
        if words.head_source().and_then(|head| head.word()) != Some(&self.head)
            || !body.matches_source(self.head.image(), input.lexer_config())
            || !body.matches_context(&context)
            || body.original_container().content_span().ok() != Some(body.content_span())
        {
            return unavailable();
        }
        let current = words.with_source_schema(&context, |schema| {
            schema
                .semantics
                .body_interpreter
                .resolve(schema.words.arguments())
                == tcl_registry::InterpreterScope::Current
        });
        if current != Some(true) {
            return unavailable();
        }
        let Some(value) =
            words
                .operands()
                .iter()
                .zip(words.arguments())
                .find_map(|(operand, value)| {
                    (operand.as_ref().and_then(|operand| operand.word())
                        == Some(body.original_container()))
                    .then(|| value.as_registry_word().literal().map(str::to_owned))
                    .flatten()
                })
        else {
            return unavailable();
        };
        crate::ir_helpers::script_value_name_ownership_with_metadata_context(
            &value,
            self.registry,
            self.state,
            &crate::ir::ExecutionNamespace::SourceContext(self.namespace.clone()),
            self.metadata,
            self.config,
            crate::script_binds::Ownership::BindingsOrNameReads,
        )
    }

    /// Lexical ownership in one unchanged, literal written body operand.
    /// Selected immediate/current-interpreter roles retain aliases and their
    /// captured argument offset. This supplies no original child words or frame.
    pub(crate) fn literal_body_name_ownership(
        &self,
        tokens: &CommandTokens,
        written_argument: usize,
        purpose: crate::script_binds::Ownership,
    ) -> Option<crate::ir_helpers::VariableWriteEffects> {
        let (command, original) = self.binding.original_recorded_command()?;
        if original.words() != tokens.words() || tokens.synthetic.is_some() {
            return None;
        }
        let config = self.binding.original_lexer_config_for_tokens(tokens)?;
        let value = crate::registry_invocation::effective_invocation_word(
            tokens.words().get(written_argument.checked_add(1)?)?,
            config.escapes,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
        );
        let crate::registry_invocation::EffectiveInvocationWord::Literal(value) = value else {
            return None;
        };
        let words = crate::ir_helpers::footprint_command_words(
            &tcl_lexer::SourceMap::from_image(self.head.image()),
            config,
            &command,
        );
        let mut selected = false;
        let mut accepted = true;
        self.state.for_each_resolved_command_words(&words, self.namespace, |target, invocation| {
            selected = true;
            if !target.registry_backed { accepted = false; return; }
            let resolution = tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.registry, Some(self.metadata.context()), invocation, self.state.invocation_realm(),
            );
            let Some(schema) = resolution.resolved() else { accepted = false; return; };
            let Some(argument) = written_argument.checked_add(target.prepended.len()) else {
                accepted = false; return;
            };
            accepted &= schema.semantics.body_kind == tcl_registry::BodyKind::Plain
                && schema.semantics.body_interpreter.resolve(schema.words.arguments())
                    == tcl_registry::InterpreterScope::Current
                && (if self.metadata.permits_logical_source_names() {
                    schema.authored_logical_source_plain_script_arguments()
                } else { schema.authored_source_plain_script_arguments() })
                    .is_some_and(|arguments| arguments.contains(&argument))
                && schema.authored_source_script_timing_at(argument)
                    == Some(tcl_registry::ScriptTiming::SameInvocation);
        });
        (selected && accepted).then(|| {
            crate::ir_helpers::script_value_name_ownership_with_metadata_context(
                &value,
                self.registry,
                self.state,
                &crate::ir::ExecutionNamespace::SourceContext(self.namespace.clone()),
                self.metadata,
                config.nested(),
                purpose,
            )
        })
    }

    /// Conditional by-name and re-evaluated-body reads of this exact original
    /// command. Its own lookup horizon supplies the alias target; no newly
    /// parsed child command receives an original source or entered frame.
    pub(crate) fn invocation_reads(&self) -> crate::ir_helpers::VariableWriteEffects {
        self.read_footprint(true)
    }

    /// Reads in selected re-evaluated expressions and immediate source values.
    pub(crate) fn reevaluated_reads(&self) -> crate::ir_helpers::VariableWriteEffects {
        self.read_footprint(false)
    }

    /// Possible names of this selected original command, including templates
    /// re-evaluated under its own retained source lookup and grammar.
    pub(crate) fn invocation_footprint(&self) -> crate::ir_helpers::VariableWriteEffects {
        self.command_footprint(true, true)
    }

    fn read_footprint(&self, include_invocation: bool) -> crate::ir_helpers::VariableWriteEffects {
        self.command_footprint(include_invocation, false)
    }

    fn command_footprint(
        &self,
        include_invocation: bool,
        include_writes: bool,
    ) -> crate::ir_helpers::VariableWriteEffects {
        let Some((command, tokens)) = self.binding.original_recorded_command() else {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        };
        let image = self.head.image();
        let map = tcl_lexer::SourceMap::from_image(image);
        let Some(config) = self.binding.original_lexer_config_for_tokens(&tokens) else {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        };
        let Some(site) = self.binding.invocation_site() else {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        };
        let Some(native) = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            site.offset,
            config,
        ) else {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        };
        if native.first() != Some(&self.head) {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        }
        let words = crate::ir_helpers::footprint_command_words(&map, config, &command);
        if include_writes {
            return crate::ir_helpers::command_possible_footprint_with_metadata_context(
                &words,
                self.registry,
                self.state,
                &crate::ir::ExecutionNamespace::SourceContext(self.namespace.clone()),
                self.metadata,
                self.config,
            );
        }
        crate::ir_helpers::command_possible_reads_with_metadata_context(
            &words,
            self.registry,
            self.state,
            &crate::ir::ExecutionNamespace::SourceContext(self.namespace.clone()),
            self.metadata,
            self.config,
            include_invocation,
        )
    }

    /// Possible writes of the exact selected original invocation and its
    /// immediate same-frame script values. The source head is revalidated;
    /// captured values do not acquire a child word or entered frame.
    pub(crate) fn invocation_writes(
        &self,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> crate::ir_helpers::VariableWriteEffects {
        let input = self
            .metadata
            .source_analysis_input()
            .expect("sealed complete input");
        if words.head_source().and_then(|head| head.word()) != Some(&self.head)
            || !words.matches_source(self.head.image(), input.lexer_config())
        {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        }
        let context = input.context_registry();
        let Some((projection, bodies)) = words.with_source_schema(&context, |schema| {
            (
                tcl_registry::CommandRegistry::variable_write_projection_for_selected_source(
                    schema,
                ),
                crate::ir_helpers::immediate_same_frame_script_values(schema),
            )
        }) else {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        };
        let mut writes = crate::ir_helpers::VariableWriteEffects {
            names: projection.literal_names,
            read_names: projection.read_before_write_names,
            opaque: projection.opaque_variable_frame,
        };
        for body in bodies {
            writes.include_possible(&self.script_writes(&body));
        }
        writes
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::ResolvedAnalysisInput;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use crate::registry_invocation::InvocationMetadataContext;
    use std::sync::Arc;

    fn unit(source: &str, context: &Arc<tcl_registry::model::ContextRegistry>) -> CompilationUnit {
        let profile = tcl_dialect::DialectProfile::find("tcl").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
    }

    #[test]
    fn original_materialized_footprint_keeps_alias_targets_and_literal_roots() {
        // naming.diagnostic.original-materialized-write-footprint
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
        // Source/API possible names only; no process result, child argv or store.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        for (source, text, expected) in [
            (
                "eval set {café(open} VALUE",
                "set {café(open} VALUE",
                Some("café(open"),
            ),
            (
                "interp alias {} put {} set {$literal}; eval put VALUE",
                "put VALUE",
                Some("$literal"),
            ),
            (
                "rename set moved; interp alias {} put {} moved café; eval put VALUE",
                "put VALUE",
                Some("café"),
            ),
            (
                "proc set args {}; eval set hidden VALUE",
                "set hidden VALUE",
                None,
            ),
            (
                "interp alias {} put {} set fixed; rename set moved; eval put VALUE",
                "put VALUE",
                None,
            ),
        ] {
            let unit = unit(source, &context);
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            let metadata =
                InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
            let footprint = binding
                .original_materialized_footprint(tokens, source, context.commands(), metadata)
                .expect("original closed installer horizon");
            let writes = footprint.script_writes(text);
            if let Some(expected) = expected {
                assert!(
                    writes.names.iter().any(|name| name == expected),
                    "{source}: {:?}",
                    writes.names
                );
            } else {
                assert!(
                    writes.names.is_empty(),
                    "known replacement: {:?}",
                    writes.names
                );
            }
        }
    }

    #[test]
    fn original_materialized_footprint_uses_actual_availability_on_the_same_store() {
        // naming.diagnostic.original-materialized-write-footprint
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
        // The command generation is identical; only actual availability changes.
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let setter = registry.get("set").unwrap().clone();
        registry.insert(tcl_registry::CommandSpec {
            name: "metadata_write",
            surface: registry.get("dict").unwrap().surface,
            ..setter
        });
        let current = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let source = "eval metadata_write output VALUE";
        for (context, expected) in [(&current, true), (&older, false)] {
            let unit = unit(source, context);
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            let metadata = InvocationMetadataContext::for_module(&registry, &unit.ir_module);
            let footprint = binding
                .original_materialized_footprint(tokens, source, &registry, metadata)
                .unwrap();
            let writes = footprint.script_writes("metadata_write output VALUE");
            assert_eq!(
                writes.names.iter().any(|name| name == "output"),
                expected,
                "{expected}: {writes:?}"
            );
            if !expected {
                assert!(
                    writes.opaque,
                    "unavailable source target keeps its residual"
                );
            }
        }
    }

    #[test]
    fn materialized_bodies_keep_the_original_entry_availability_phase() {
        // naming.diagnostic.original-materialized-write-footprint
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
        // Exact source/API phase selection only. The explicit entry has no
        // native interpreter, body-entry, worker or successful store receipt.
        use tcl_dialect::model::InvocationRealm;
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let registry = context.commands();
        let profile = tcl_dialect::DialectProfile::find("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
        assert!(context.context().resolve_spec(registry, "time").is_none());
        assert!(
            context
                .context()
                .resolve_spec_in_realm(registry, "time", InvocationRealm::InterpreterRuntime,)
                .is_some()
        );
        let source = "eval time {set output VALUE}";
        for (realm, expected) in [
            (InvocationRealm::RuleLoader, false),
            (InvocationRealm::InterpreterRuntime, true),
        ] {
            let entry = super::super::SourceAnalysisEntry {
                invocation_realm: realm,
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                ..Default::default()
            };
            let unit = CompilationUnit::build_with_analysis_input(
                source,
                UnitBuildOptions {
                    registry,
                    defer_top_level: false,
                    config,
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                Some(&entry),
                &input,
            );
            assert!(entry.native_entry.is_none());
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            assert_eq!(binding.invocation_realm(), Some(realm));
            let footprint = binding
                .original_materialized_footprint(
                    tokens,
                    source,
                    registry,
                    InvocationMetadataContext::for_module(registry, &unit.ir_module),
                )
                .unwrap();
            let writes = footprint.script_writes("time {set output VALUE}");
            assert_eq!(
                writes.names.iter().any(|name| name == "output"),
                expected,
                "{realm:?}: {writes:?}"
            );
            if !expected {
                assert!(writes.opaque, "loader refusal keeps the residual");
            }
        }
    }

    #[test]
    fn original_materialized_footprint_refuses_missing_foreign_and_stale_owners() {
        // naming.diagnostic.original-materialized-write-footprint
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let source = "eval set x VALUE";
        let unit = unit(source, &context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let metadata = InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
        assert!(
            binding
                .original_materialized_footprint(tokens, source, context.commands(), metadata)
                .is_some()
        );
        assert!(
            binding
                .original_materialized_footprint(tokens, source, context.commands(), None)
                .is_none()
        );
        assert!(
            binding
                .original_materialized_footprint(
                    tokens,
                    "eval set x OTHER",
                    context.commands(),
                    metadata
                )
                .is_none()
        );
        let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
        let foreign = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            input.lexer_config(),
        );
        assert!(
            binding
                .original_materialized_footprint(
                    tokens,
                    source,
                    context.commands(),
                    InvocationMetadataContext::for_analysis_input(context.commands(), &foreign)
                )
                .is_none()
        );
        let stale = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::clone(&context),
            tcl_lexer::LexerConfig {
                expand_syntax: tcl_lexer::LexerConfig::for_dialect("tcl8.4").expand_syntax,
                ..input.lexer_config()
            },
        );
        assert!(
            binding
                .original_materialized_footprint(
                    tokens,
                    source,
                    context.commands(),
                    InvocationMetadataContext::for_analysis_input(context.commands(), &stale)
                )
                .is_none()
        );
    }
    #[test]
    fn original_literal_body_ownership_keeps_named_purposes_and_literal_roots() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Conditional lexical names only, never installed aliases or current reads.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let source = "catch {global ::N::x; set {$literal} VALUE; set {café(open} VALUE; puts $missing; set byname}";
        let unit = unit(source, &context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let metadata = InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
        for (purpose, expected_names, expected_reads) in [
            (
                crate::script_binds::Ownership::Bindings,
                vec!["x", "$literal", "café(open"],
                Vec::<&str>::new(),
            ),
            (
                crate::script_binds::Ownership::BindingsOrNameReads,
                vec!["x", "$literal", "café(open"],
                vec!["byname"],
            ),
            (
                crate::script_binds::Ownership::ScopeAliases,
                vec!["x"],
                Vec::<&str>::new(),
            ),
        ] {
            let names = crate::script_binds::original_literal_body_ownership(
                context.commands(),
                metadata,
                tokens,
                0,
                purpose,
            )
            .unwrap();
            assert!(!names.opaque, "{names:?}");
            assert_eq!(
                names
                    .names
                    .iter()
                    .map(String::as_str)
                    .collect::<std::collections::BTreeSet<_>>(),
                expected_names.into_iter().collect()
            );
            assert_eq!(
                names
                    .read_names
                    .iter()
                    .map(String::as_str)
                    .collect::<std::collections::BTreeSet<_>>(),
                expected_reads.into_iter().collect()
            );
            assert!(
                !names
                    .names
                    .iter()
                    .chain(&names.read_names)
                    .any(|name| name == "missing")
            );
        }
        assert!(
            crate::script_binds::original_literal_body_ownership(
                context.commands(),
                None,
                tokens,
                0,
                crate::script_binds::Ownership::Bindings,
            )
            .is_none()
        );
        assert!(
            crate::script_binds::original_literal_body_ownership(
                context.commands(),
                metadata,
                tokens,
                1,
                crate::script_binds::Ownership::Bindings,
            )
            .is_none()
        );
    }

    #[test]
    fn original_literal_body_aliases_keep_local_names_and_opaque_residuals() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Held alias names/captures and a conditional residual, no frame or execution.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        for (source, expected, opaque) in [
            (
                "interp alias {} expose {} global ::N::x; catch {expose}",
                Some("x"),
                false,
            ),
            (
                "rename global retired; interp alias {} expose {} retired ::N::x; catch {expose}",
                Some("x"),
                false,
            ),
            (
                "interp alias {} expose {} global ::N::x; rename global {}; catch {expose}",
                None,
                true,
            ),
            ("catch {set known VALUE; eval $script}", None, true),
        ] {
            let unit = unit(source, &context);
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let names = crate::script_binds::original_literal_body_ownership(
                context.commands(),
                InvocationMetadataContext::for_module(context.commands(), &unit.ir_module),
                tokens,
                0,
                crate::script_binds::Ownership::ScopeAliases,
            )
            .unwrap();
            assert_eq!(
                names.names.first().map(String::as_str),
                expected,
                "{source}: {names:?}"
            );
            assert_eq!(names.opaque, opaque, "{source}: {names:?}");
        }
    }

    #[test]
    fn original_literal_body_ownership_keeps_availability_and_source_currency() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Conditional source names only; availability is not actual execution.
        let current =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.commands())),
        );
        for (context, expected_opaque) in [(&current, false), (&older, true)] {
            let unit = unit("catch {throw {ERR} message}", context);
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let metadata =
                InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
            let ownership = crate::script_binds::original_literal_body_ownership(
                context.commands(),
                metadata,
                tokens,
                0,
                crate::script_binds::Ownership::Bindings,
            )
            .unwrap();
            assert_eq!(ownership.opaque, expected_opaque, "{ownership:?}");
            let foreign = tcl_registry::model::ingress::resolve_environment("tcl9.1")
                .default_context_registry();
            assert!(
                crate::script_binds::original_literal_body_ownership(
                    foreign.commands(),
                    metadata,
                    tokens,
                    0,
                    crate::script_binds::Ownership::Bindings,
                )
                .is_none()
            );
            let mut changed = tokens.clone();
            let crate::ir::WordExpr::BracedLiteral { text, .. } = &mut changed.word_exprs[1] else {
                panic!("original body word");
            };
            *text = "set fabricated VALUE".to_owned();
            assert!(
                crate::script_binds::original_literal_body_ownership(
                    context.commands(),
                    metadata,
                    &changed,
                    0,
                    crate::script_binds::Ownership::Bindings,
                )
                .is_none()
            );
        }
    }
}
