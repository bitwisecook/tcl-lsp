// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native entry inventory retained for conditional source syntax only.

use super::{AdviceCell, AdviceGraph};
use tcl_registry::model::ContextRegistry;
use tcl_runtime_api::native_compilation::{NativeCommandImplementation, NativeCompilationEntry};

impl AdviceGraph {
    pub(super) fn retain_native_baseline(
        &mut self,
        context: &ContextRegistry,
        entry: &NativeCompilationEntry,
    ) -> Option<()> {
        // naming.source.native-baseline-conditional-source-roles
        // docs/design/analysis/name-resolution-proofs/native-baseline-conditional-source-roles.md
        if !entry.closed || self.policy.native() != entry.command_name_policy() {
            return None;
        }
        let current = entry.namespace_context(entry.current_namespace).ok()?;
        if !current.visible || !current.path.is_root() {
            return None;
        }
        if entry.command_name_policy()?.recipe().is_jim084()
            && current.jim_namespace_object.as_ref()?.as_bytes() != b""
        {
            return None;
        }
        // Catalogue availability cannot restore an implementation absent from
        // the authentic closed native entry. Installed opaque rows shadow it.
        for cell in self.cells.values_mut() {
            *cell = AdviceCell::Deleted;
        }
        self.namespaces.clear();
        self.namespaces.extend(
            entry
                .namespaces
                .iter()
                .filter(|row| row.visible)
                .map(|row| row.path.clone()),
        );
        let mut installed = std::collections::BTreeSet::new();
        for row in &entry.commands {
            let namespace = entry.namespace_context(row.namespace_token).ok()?;
            if !namespace.visible {
                continue;
            }
            if namespace.path != row.slot.namespace {
                return None;
            }
            let cell = match &row.implementation {
                NativeCommandImplementation::Registry { identity, .. }
                    if context
                        .context()
                        .resolve_spec(context.commands(), identity)
                        .is_some() =>
                {
                    AdviceCell::Descriptor(identity.clone(), Vec::new())
                }
                NativeCommandImplementation::Registry { .. }
                | NativeCommandImplementation::Alias { .. }
                | NativeCommandImplementation::PartialAlias { .. }
                | NativeCommandImplementation::Imported { .. }
                | NativeCommandImplementation::Opaque => AdviceCell::Shadowed,
            };
            let key = row.slot.clone();
            if installed.insert(key.clone()) {
                self.cells.insert(key, cell);
            } else {
                self.cells.insert(key, AdviceCell::Shadowed);
            }
        }
        self.native_baseline = true;
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, source_registry_words,
    };

    fn native_analysis(source: &str) -> crate::analyser::AnalysisResult {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        Analyser::new()
            .with_source_analysis_entry(std::sync::Arc::new(
                crate::command_binding::SourceAnalysisEntry {
                    native_entry: Some(std::sync::Arc::new(entry)),
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            ))
            .analyse(source, profile.name)
    }

    #[test]
    fn native_baseline_source_roles_keep_unknown_actual_lookup_independent() {
        // naming.source.native-baseline-conditional-source-roles
        // docs/design/analysis/name-resolution-proofs/native-baseline-conditional-source-roles.md
        let source = "unavailable\ninfo body missing";
        let analysis = native_analysis(source);
        let config = analysis.body_lexer_config.unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        let realm = analysis.retained_command_realm().unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        realm.stamp_original_tokens(&mut tokens);
        let before = tokens.source_binding.as_ref().unwrap().clone();
        assert!(before.execution_is_unknown());
        assert!(before.proved_execution_target().is_none());
        let words = source_registry_words(source, &analysis, &segment).unwrap();
        assert_eq!(
            words.written_argument_roles(),
            [(1, tcl_registry::ArgRole::CommandName)]
        );
        assert!(!words.operands_preserve_source_lookup());
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("actual native inventory supplies its own conditional source receipt");
        };
        assert!(advice.obligations().contains(
            &crate::command_binding::SourceCommandTransitionObligation::NativeBaselineSourceApplicability,
        ));
        assert!(advice.obligations().contains(
            &crate::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
        ));
        let registry = analysis.resolved_registry().unwrap();
        assert!(
            crate::registry_invocation::invocation_argument_role_consensus(
                registry,
                tcl_registry::model::semantic::SemanticContext::for_environment("tcl8.6"),
                &tokens,
            )
            .is_empty()
        );
        assert_eq!(tokens.source_binding.as_ref(), Some(&before));
        assert!(source_registry_words(&format!("#{source}"), &analysis, &segment).is_none());
    }

    #[test]
    fn native_baseline_source_roles_refuse_original_shadows_and_deletions() {
        // naming.source.native-baseline-conditional-source-roles
        // docs/design/analysis/name-resolution-proofs/native-baseline-conditional-source-roles.md
        for source in [
            "proc info args {}; info body missing",
            "rename info {}; info body missing",
            "proc info args {}; unavailable; info body missing",
            "rename info {}; unavailable; info body missing",
        ] {
            let analysis = native_analysis(source);
            let config = analysis.body_lexer_config.unwrap();
            let segment =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .pop()
                    .unwrap();
            assert!(
                source_registry_words(source, &analysis, &segment).is_none(),
                "{source}"
            );
        }
    }
}
