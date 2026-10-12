// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Complete original point projections under one immutable source realm.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::CommandBindingRealm;
use crate::command_binding::SourceInvocationBinding;

const MAX_RETAINED_BINDINGS: usize = 256;

/// Immutable realm clones share derived point results. A replacement realm,
/// including one built from changed source, config, Registry or entry, starts
/// fresh. The complete original inventories remain owned by that realm.
#[derive(Debug, Clone, Default)]
pub(super) struct OriginalInvocationBindingCache {
    entries: Arc<Mutex<HashMap<u32, Arc<SourceInvocationBinding>>>>,
}

impl OriginalInvocationBindingCache {
    fn get_or_prepare(
        &self,
        offset: u32,
        prepare: impl FnOnce() -> SourceInvocationBinding,
    ) -> Arc<SourceInvocationBinding> {
        // Implementation contract: naming.source.original-invocation-projection-memo
        // docs/design/analysis/name-resolution-proofs/original-invocation-projection-memo.md
        if let Some(retained) = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&offset)
            .cloned()
        {
            return retained;
        }
        let prepared = Arc::new(prepare());
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(retained) = entries.get(&offset) {
            return Arc::clone(retained);
        }
        if entries.len() < MAX_RETAINED_BINDINGS {
            entries.insert(offset, Arc::clone(&prepared));
        }
        prepared
    }
}

impl CommandBindingRealm {
    /// The shared original query ignores its presentation head and selects
    /// every genuine point at this offset under the sealed realm's retained
    /// root/config/Registry/entry/lookup/variable inventories. Only that complete
    /// derived result is shared; compiler and current-purpose guards remain on
    /// the same receipt. A cache hit cannot select another interpretation.
    pub(super) fn retained_original_invocation(&self, offset: u32) -> Arc<SourceInvocationBinding> {
        // Implementation contract: naming.source.original-invocation-projection-memo
        // docs/design/analysis/name-resolution-proofs/original-invocation-projection-memo.md
        self.original_bindings.get_or_prepare(offset, || {
            #[cfg(debug_assertions)]
            let started = std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
                .is_some()
                .then(std::time::Instant::now);
            let binding = self.bindings.invocation_at_source("", offset);
            #[cfg(debug_assertions)]
            if let Some(started) = started {
                eprintln!(
                    "SOURCE_INVOCATION_PROJECT site={offset} ms={} targets={} unknown={}",
                    started.elapsed().as_millis(),
                    binding.targets.len(),
                    binding.unknown
                );
            }
            binding
        })
    }

    /// Reuse this immutable realm's exact invocation for a readonly original
    /// operand. A caller-supplied span cannot select another point, image,
    /// configuration or frozen value context.
    pub(crate) fn original_written_name_input_at_invocation_span(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
        span: tcl_lexer::Span,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        // Implementation contract: naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        self.matches_original_source_image(image, config)
            .then_some(())?;
        self.retained_original_invocation(offset)
            .original_written_name_input_at_invocation_span(image, config, span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, SourceMap};

    fn fixture(source: &str) -> CommandBindingRealm {
        let registry = tcl_registry::CommandRegistry::build_default();
        crate::realm::document_realm_bindings(
            source,
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            &registry,
        )
    }

    #[test]
    fn original_point_memo_shares_complete_exact_results_and_keeps_points_distinct() {
        // Implementation contract: naming.source.original-invocation-projection-memo
        // docs/design/analysis/name-resolution-proofs/original-invocation-projection-memo.md
        let source = "set x 1; set y 2";
        let realm = fixture(source);
        let clone = realm.clone();
        let first = realm.retained_original_invocation(0);
        assert_eq!(
            first.as_ref(),
            &realm
                .bindings
                .invocation_at_source("presentation-ignored", 0)
        );
        assert!(!first.unknown);
        assert!(!first.targets.is_empty());
        assert!(Arc::ptr_eq(&first, &realm.retained_original_invocation(0)));
        assert!(Arc::ptr_eq(&first, &clone.retained_original_invocation(0)));
        let second = realm.retained_original_invocation(9);
        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(second.as_ref(), &realm.bindings.invocation_at_source("", 9));
        assert_ne!(first.invocation_site(), second.invocation_site());
        assert_eq!(realm, clone);
    }

    #[test]
    fn original_point_memo_shares_queries_across_genuine_enclosing_vectors() {
        // Implementation contract: naming.source.original-invocation-projection-memo
        // docs/design/analysis/name-resolution-proofs/original-invocation-projection-memo.md
        let source = "set x [set y [set z 1]]";
        let realm = fixture(source);
        let config = LexerConfig::for_file_grammar(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap().grammar,
        );
        let image = SourceImage::document(source);
        let map = SourceMap::from_image(&image);
        let outer =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let mut original = crate::ir::CommandTokens::from_segmented(&map, config, &outer);
        realm.bindings.stamp_original_tokens(&mut original);
        let mut shared = crate::ir::CommandTokens::from_segmented(&map, config, &outer);
        realm.stamp_original_tokens(&mut shared);
        assert_eq!(original, shared);
        assert!(shared.nested_bindings.len() >= 2);
        let inner =
            crate::segmenter::segment_commands_with_offset_and_config("set y [set z 1]", 7, config)
                .remove(0);
        let mut nested = crate::ir::CommandTokens::from_segmented(&map, config, &inner);
        realm.stamp_original_tokens(&mut nested);
        let retained = realm.retained_original_invocation(14);
        assert!(
            shared
                .nested_bindings
                .iter()
                .any(|(offset, binding)| *offset == 14 && binding == retained.as_ref())
        );
        assert!(
            nested
                .nested_bindings
                .iter()
                .any(|(offset, binding)| *offset == 14 && binding == retained.as_ref())
        );
        assert_eq!(realm.original_bindings.entries.lock().unwrap().len(), 3);
    }

    #[test]
    fn original_point_memo_does_not_reuse_foreign_source_config_or_unknown_entry() {
        // Implementation contract: naming.source.original-invocation-projection-memo
        // docs/design/analysis/name-resolution-proofs/original-invocation-projection-memo.md
        let source = "set x 1";
        let realm = fixture(source);
        let first = realm.retained_original_invocation(0);
        let independent = fixture(source);
        assert!(!Arc::ptr_eq(
            &first,
            &independent.retained_original_invocation(0)
        ));
        let changed = fixture("set x 2");
        let changed_binding = changed.retained_original_invocation(0);
        assert!(!Arc::ptr_eq(&first, &changed_binding));
        assert_ne!(
            first.evaluated_argument_words,
            changed_binding.evaluated_argument_words
        );
        let registry = tcl_registry::CommandRegistry::build_default();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = LexerConfig {
            strict_quoting: true,
            ..LexerConfig::for_file_grammar(profile.grammar)
        };
        let configured = crate::realm::realm_from_source_bindings(
            crate::command_binding::SourceCommandBindings::analyse_with_options(
                source,
                config,
                &registry,
                crate::command_binding::SourceAnalysisOptions {
                    invocation_dialect: Some(
                        crate::environment_ingress::authoring_invocation_dialect(
                            &registry,
                            Some(profile),
                            config,
                        ),
                    ),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            ),
            &registry,
        );
        assert!(!Arc::ptr_eq(
            &first,
            &configured.retained_original_invocation(0)
        ));
        assert!(!configured.matches_original_source_image(
            &SourceImage::document(source),
            LexerConfig::for_file_grammar(profile.grammar)
        ));
        let unknown = crate::realm::realm_from_source_bindings(
            crate::command_binding::SourceCommandBindings::analyse_with_options(
                source,
                config,
                &registry,
                crate::command_binding::SourceAnalysisOptions {
                    unknown_entry: true,
                    invocation_dialect: Some(
                        crate::environment_ingress::authoring_invocation_dialect(
                            &registry,
                            Some(profile),
                            config,
                        ),
                    ),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            ),
            &registry,
        );
        let withdrawn = unknown.retained_original_invocation(0);
        assert!(!Arc::ptr_eq(&first, &withdrawn));
        assert!(withdrawn.unknown);
    }

    #[test]
    fn original_point_memo_has_bounded_retention_without_closing_absence() {
        // Implementation contract: naming.source.original-invocation-projection-memo
        // docs/design/analysis/name-resolution-proofs/original-invocation-projection-memo.md
        let realm = fixture("set x 1");
        for offset in 0..=u32::try_from(MAX_RETAINED_BINDINGS).unwrap() {
            let binding = realm.retained_original_invocation(offset);
            if offset != 0 {
                assert!(binding.unknown);
            }
        }
        assert_eq!(
            realm.original_bindings.entries.lock().unwrap().len(),
            MAX_RETAINED_BINDINGS
        );
        let beyond = u32::try_from(MAX_RETAINED_BINDINGS).unwrap();
        let first = realm.retained_original_invocation(beyond);
        let again = realm.retained_original_invocation(beyond);
        assert!(!Arc::ptr_eq(&first, &again));
        assert_eq!(first, again);
        assert!(first.unknown);
    }
    #[test]
    fn original_point_operand_projection_keeps_the_exact_invocation_owner() {
        // naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        let source = "namespace eval A {namespace eval B {set x 1}}";
        let realm = fixture(source);
        let image = SourceImage::document(source);
        let config = LexerConfig::for_file_grammar(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap().grammar,
        );
        let outer =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let offset = u32::try_from(source.find("namespace eval B").unwrap()).unwrap();
        let inner = crate::segmenter::segment_commands_with_offset_and_config(
            "namespace eval B {set x 1}",
            offset,
            config,
        )
        .remove(0);
        for (offset, command, expected) in [(0, &outer, b"A"), (offset, &inner, b"B")] {
            let span = command.argv[2].span;
            let input = realm
                .original_written_name_input_at_invocation_span(&image, config, offset, span)
                .unwrap();
            assert_eq!(input.bytes(), expected);
            assert_eq!(
                Some(input.clone()),
                realm
                    .bindings
                    .original_written_name_input_at_span(span, config)
            );
            assert_eq!(
                Some(input),
                realm
                    .clone()
                    .original_written_name_input_at_invocation_span(&image, config, offset, span,)
            );
        }
        assert!(
            realm
                .original_written_name_input_at_invocation_span(
                    &image,
                    config,
                    0,
                    inner.argv[2].span,
                )
                .is_none()
        );
        assert!(
            realm
                .original_written_name_input_at_invocation_span(
                    &image,
                    config,
                    offset,
                    outer.argv[2].span,
                )
                .is_none()
        );
    }

    #[test]
    fn original_point_operand_projection_retains_frozen_values_and_full_currency() {
        // naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        let source = "set target A; namespace eval $target {}";
        let realm = fixture(source);
        let image = SourceImage::document(source);
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(1);
        let offset = command.span.start();
        let span = command.argv[2].span;
        let input = realm
            .original_written_name_input_at_invocation_span(&image, config, offset, span)
            .unwrap();
        assert_eq!(input.bytes(), b"A");
        assert!(matches!(
            input,
            crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(_)
        ));
        assert_eq!(
            Some(input),
            realm
                .bindings
                .original_written_name_input_at_span(span, config)
        );
        let mut different_config = config;
        different_config.strict_quoting = !different_config.strict_quoting;
        assert!(
            realm
                .original_written_name_input_at_invocation_span(
                    &image,
                    different_config,
                    offset,
                    span,
                )
                .is_none()
        );
        assert!(
            realm
                .original_written_name_input_at_invocation_span(
                    &SourceImage::native(image.bytes()),
                    config,
                    offset,
                    span,
                )
                .is_none()
        );
        assert!(
            realm
                .original_written_name_input_at_invocation_span(
                    &SourceImage::document("set target B; namespace eval $target {}"),
                    config,
                    offset,
                    span,
                )
                .is_none()
        );
        let registry = tcl_registry::CommandRegistry::build_default();
        let unknown = crate::realm::realm_from_source_bindings(
            crate::command_binding::SourceCommandBindings::analyse_with_options(
                source,
                config,
                &registry,
                crate::command_binding::SourceAnalysisOptions {
                    unknown_entry: true,
                    invocation_dialect: Some(
                        crate::environment_ingress::authoring_invocation_dialect(
                            &registry,
                            Some(profile),
                            config,
                        ),
                    ),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            ),
            &registry,
        );
        assert!(
            unknown
                .original_written_name_input_at_invocation_span(&image, config, offset, span,)
                .is_none()
        );
    }
}
