// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional lexical name ownership in one retained original body world.

use super::declaration_layout::DeclarationLayoutObservation;

impl DeclarationLayoutObservation {
    /// Possible names from this exact body's source, grammar and lookup
    /// horizon. Missing supplied metadata, unknown source worlds and opaque
    /// command/body residuals cannot supply a complete negative answer.
    /// Standalone compatibility remains an explicit retained owner; neither
    /// path supplies an entered script, installed alias or current frame.
    pub(super) fn source_body_name_ownership(
        &self,
        registry: &tcl_registry::CommandRegistry,
        purpose: crate::script_binds::Ownership,
    ) -> Option<crate::ir_helpers::VariableWriteEffects> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let source = self.entry.source();
        let state = &self.snapshot.state;
        if state.baseline.registry_snapshot != Some(registry.snapshot().semantic_key())
            || state.current_source_origin.as_ref() != Some(&source.origin)
            || state.has_opaque_domain()
            || !state.command_observers.is_quiet()
            || state.source_step_observed()
        {
            return None;
        }
        let owner = &state.baseline.metadata_context;
        let metadata = if let Some(input) = owner.source_analysis_input() {
            let original = input.lexer_config();
            if self.config.normalized() != original.normalized()
                && self.config.normalized() != original.nested().normalized()
            {
                return None;
            }
            // Source-instance ownership justifies the nested grammar projection;
            // it never relabels the full input or donates a physical frame.
            owner.metadata_context_for_source(registry, original, Some(input.unit_profile()))?
        } else {
            owner.metadata_context_for_source(registry, self.config, registry.profile())?
        };
        Some(
            crate::ir_helpers::script_value_name_ownership_with_metadata_context(
                source.text.try_text().ok()?,
                registry,
                state,
                &crate::ir::ExecutionNamespace::SourceContext(self.namespace.clone()),
                metadata,
                self.config,
                purpose,
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::ResolvedAnalysisInput;
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
    use crate::registry_invocation::OwnedInvocationMetadataContext;
    use std::sync::Arc;

    #[test]
    fn original_body_ownership_keeps_explicit_standalone_and_missing_supplied_separate() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // A conditional source alias name is not an installed alias/frame.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let config =
            tcl_lexer::LexerConfig::from_grammar(context.commands().profile().unwrap().grammar);
        let bindings = SourceCommandBindings::analyse(
            "proc p {} {upvar 1 remote local; puts $other}",
            config,
            context.commands(),
        );
        let row = bindings
            .declaration_layouts
            .values()
            .flat_map(|rows| rows.iter())
            .find(|row| {
                row.entry.source().text.try_text().ok() == Some("upvar 1 remote local; puts $other")
            })
            .expect("original declaration body");
        let effects = row
            .source_body_name_ownership(
                context.commands(),
                crate::script_binds::Ownership::ScopeAliases,
            )
            .expect("explicit standalone source query");
        assert!(!effects.opaque, "{effects:?}");
        assert_eq!(effects.names, ["local"]);
        assert!(!effects.read_names.iter().any(|name| name == "other"));
        let mut missing = row.clone();
        Arc::make_mut(&mut Arc::make_mut(&mut missing.snapshot).state.baseline).metadata_context =
            OwnedInvocationMetadataContext::Unavailable;
        assert!(
            missing
                .source_body_name_ownership(
                    context.commands(),
                    crate::script_binds::Ownership::ScopeAliases,
                )
                .is_none()
        );
    }

    #[test]
    fn original_body_ownership_rejects_foreign_input_and_changed_body_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::find("tcl").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
        let bindings = SourceCommandBindings::analyse_with_options(
            "proc p {} {upvar 1 remote local; puts $other}",
            config,
            context.commands(),
            SourceAnalysisOptions::for_logical_source(&input)
                .expect("genuine Logical source input"),
        );
        let row = bindings
            .declaration_layouts
            .values()
            .flat_map(|rows| rows.iter())
            .find(|row| {
                row.entry.source().text.try_text().ok() == Some("upvar 1 remote local; puts $other")
            })
            .expect("original Logical declaration body");
        let effects = row
            .source_body_name_ownership(
                context.commands(),
                crate::script_binds::Ownership::ScopeAliases,
            )
            .expect("complete supplied source owner");
        assert!(!effects.opaque, "{effects:?}");
        assert_eq!(effects.names, ["local"]);
        let mut changed = row.clone();
        changed.config.strict_quoting = !changed.config.strict_quoting;
        assert!(
            changed
                .source_body_name_ownership(
                    context.commands(),
                    crate::script_binds::Ownership::ScopeAliases,
                )
                .is_none()
        );
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let mut changed = row.clone();
        Arc::make_mut(&mut Arc::make_mut(&mut changed.snapshot).state.baseline).metadata_context =
            OwnedInvocationMetadataContext::for_source_input(Some(&ResolvedAnalysisInput::new(
                profile, profile, foreign, config,
            )));
        assert!(
            changed
                .source_body_name_ownership(
                    context.commands(),
                    crate::script_binds::Ownership::ScopeAliases,
                )
                .is_none()
        );
    }
}
