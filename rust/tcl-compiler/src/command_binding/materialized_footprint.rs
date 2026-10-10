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
    state: &'a ModuleCommandBindings,
    namespace: &'a SourceNamespaceKey,
    registry: &'a CommandRegistry,
    metadata: InvocationMetadataContext<'a>,
    config: LexerConfig,
    head: tcl_lexer::NativeWord,
}

impl SourceInvocationBinding {
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
}
