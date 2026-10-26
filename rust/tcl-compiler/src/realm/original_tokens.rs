// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded original command preparation under one immutable retained realm.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tcl_lexer::{LexerConfig, SourceImage, SourceMap, Token};

use super::CommandBindingRealm;
use crate::ir::CommandTokens;

const MAX_RETAINED_COMMANDS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OriginalCommandTokenKey {
    image: SourceImage,
    config: LexerConfig,
    offset: u32,
    representatives: Vec<Token>,
}

/// Clones belong to the same immutable interpretation and may share derived
/// tokens. A newly constructed realm receives an independent bounded cache.
#[derive(Debug, Clone, Default)]
pub(super) struct OriginalCommandTokenCache {
    entries: Arc<Mutex<HashMap<OriginalCommandTokenKey, Option<Arc<CommandTokens>>>>>,
}

impl OriginalCommandTokenCache {
    fn get_or_prepare(
        &self,
        key: OriginalCommandTokenKey,
        prepare: impl FnOnce() -> Option<Arc<CommandTokens>>,
    ) -> Option<Arc<CommandTokens>> {
        if let Some(retained) = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
            .cloned()
        {
            return retained;
        }
        // Preparation calls the ordinary lookup owners without retaining a lock.
        let prepared = prepare();
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(retained) = entries.get(&key) {
            return retained.clone();
        }
        if entries.len() < MAX_RETAINED_COMMANDS {
            entries.insert(key, prepared.clone());
        }
        prepared
    }
}

impl CommandBindingRealm {
    /// Reborrow the real representative tokens from the exact original word
    /// projection. Compatibility spans and kinds cannot create a token owner.
    pub(crate) fn retained_original_tokens_for_command(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        offset: u32,
        tokens: &CommandTokens,
    ) -> Option<Arc<CommandTokens>> {
        if tokens.synthetic.is_some() || !self.matches_original_source_image(image, config) {
            return None;
        }
        let origin = self.bindings.source_origin()?;
        let original = origin.original_lexical_command(tokens.words(), offset, config)?;
        let command = original.command();
        if tokens.argv.len() != command.argv.len()
            || tokens.argv_kinds.len() != command.argv.len()
            || tokens
                .argv
                .iter()
                .zip(&command.argv)
                .any(|(span, token)| *span != token.span)
            || tokens
                .argv_kinds
                .iter()
                .zip(&command.argv)
                .any(|(kind, token)| *kind != token.kind)
        {
            return None;
        }
        self.retained_original_tokens(image, config, offset, &command.argv)
    }

    /// Retain the existing original lexical preparation and source stamping
    /// under this immutable realm. Complete image/channel, full config, site
    /// and representative tokens distinguish queries; no reporting name,
    /// digest-only identity or newly inferred entry replaces those inputs.
    pub(crate) fn retained_original_tokens(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        offset: u32,
        representatives: &[Token],
    ) -> Option<Arc<CommandTokens>> {
        let key = OriginalCommandTokenKey {
            image: image.clone(),
            config,
            offset,
            representatives: representatives.to_vec(),
        };
        self.original_tokens.get_or_prepare(key, || {
            let source_map = SourceMap::from_image(image);
            let end = if let Some(last) = representatives.last() {
                tcl_lexer::word_span(&source_map, *last).end()
            } else {
                self.original_zero_argument_head_end(image, config, offset)?
            };
            let text = image.try_text().ok()?.get(offset as usize..end as usize)?;
            // This is an original command slice, including at site zero.
            // Its file prologue has already been handled by the parent producer.
            let segmented = crate::segmenter::segment_commands_with_offset_and_config(
                text,
                offset,
                config.nested(),
            )
            .into_iter()
            .next()?;
            let mut tokens = CommandTokens::from_segmented(&source_map, config, &segmented);
            self.stamp_original_tokens(&mut tokens);
            Some(Arc::new(tokens))
        })
    }
    /// An empty written-argument vector still has one original head. The
    /// positioned whole source owner supplies its extent, never a captured
    /// prefix or an arbitrary later word in a reconstructed argv.
    fn original_zero_argument_head_end(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        offset: u32,
    ) -> Option<u32> {
        self.source_bindings_ref()
            .matches_original_source_image(image, config)
            .then_some(())?;
        let binding = self.invocation_at_source("", offset);
        let (command, tokens) = binding.original_recorded_command()?;
        if command.argv.len() != 1 || command.argv.first()?.span.start() != offset {
            return None;
        }
        let original = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            offset,
            config,
        )?;
        (original.len() == 1).then(|| original[0].span().end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(source: &str) -> (CommandBindingRealm, SourceImage, LexerConfig, Vec<Token>) {
        let registry = tcl_registry::CommandRegistry::build_default();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let realm = crate::realm::document_realm_bindings(source, profile, &registry);
        let representatives =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)[0]
                .argv
                .clone();
        (
            realm,
            SourceImage::document(source),
            config,
            representatives,
        )
    }

    #[test]
    fn original_token_memo_keeps_zero_written_arguments_and_captured_prefix_separate() {
        let source = "interp alias {} clean {} unset -nocomplain\nclean";
        let (realm, image, config, _) = fixture(source);
        let offset = u32::try_from(source.rfind("clean").unwrap()).unwrap();
        let first = realm
            .retained_original_tokens(&image, config, offset, &[])
            .unwrap();
        assert_eq!(first.argv.len(), 1);
        assert_eq!(first.argv_texts, ["clean"]);
        let again = realm
            .retained_original_tokens(&image, config, offset, &[])
            .unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        assert!(
            realm
                .retained_original_tokens(
                    &SourceImage::document(&source.replace("-nocomplain", "--")),
                    config,
                    offset,
                    &[]
                )
                .is_none()
        );
        assert!(
            realm
                .retained_original_tokens(&image, config, 0, &[])
                .is_none()
        );
    }

    #[test]
    fn original_token_memo_shares_only_the_same_immutable_query() {
        let (realm, image, config, representatives) = fixture("namespace eval A {set x 1}");
        let clone = realm.clone();
        let first = realm
            .retained_original_tokens(&image, config, 0, &representatives)
            .unwrap();
        let again = realm
            .retained_original_tokens(&image, config, 0, &representatives)
            .unwrap();
        let from_clone = clone
            .retained_original_tokens(&image, config, 0, &representatives)
            .unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        assert!(Arc::ptr_eq(&first, &from_clone));
        assert_eq!(realm, clone);
        assert!(!first.nested_bindings.is_empty());
    }

    #[test]
    fn original_token_memo_retains_full_source_config_and_representatives() {
        let (realm, image, config, representatives) = fixture("set x 1");
        let first = realm
            .retained_original_tokens(&image, config, 0, &representatives)
            .unwrap();
        let changed = realm
            .retained_original_tokens(
                &SourceImage::document("set x 2"),
                config,
                0,
                &representatives,
            )
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &changed));
        assert_eq!(first.argv_texts[2], "1");
        assert_eq!(changed.argv_texts[2], "2");
        let mut changed_config = config;
        changed_config.strict_quoting = !changed_config.strict_quoting;
        let configured = realm
            .retained_original_tokens(&image, changed_config, 0, &representatives)
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &configured));
        let mut changed_representatives = representatives.clone();
        changed_representatives[0].content_offset += 1;
        let represented = realm
            .retained_original_tokens(&image, config, 0, &changed_representatives)
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &represented));
        let native = SourceImage::native(image.bytes());
        let channel = realm
            .retained_original_tokens(&native, config, 0, &representatives)
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &channel));
    }

    #[test]
    fn original_token_memo_does_not_reuse_a_foreign_or_unknown_realm() {
        let source = "set x 1";
        let (realm, image, config, representatives) = fixture(source);
        let first = realm
            .retained_original_tokens(&image, config, 0, &representatives)
            .unwrap();
        let (foreign, _, _, _) = fixture(source);
        let foreign_tokens = foreign
            .retained_original_tokens(&image, config, 0, &representatives)
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &foreign_tokens));
        let registry = tcl_registry::CommandRegistry::build_default();
        let unknown = crate::realm::realm_from_source_bindings(
            crate::command_binding::SourceCommandBindings::analyse_with_options(
                source,
                config,
                &registry,
                crate::command_binding::SourceAnalysisOptions {
                    unknown_entry: true,
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
                    )),
                    ..Default::default()
                },
            ),
            &registry,
        );
        let unknown_tokens = unknown
            .retained_original_tokens(&image, config, 0, &representatives)
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &unknown_tokens));
        assert!(!first.source_binding.as_ref().unwrap().unknown);
        assert!(unknown_tokens.source_binding.as_ref().unwrap().unknown);
    }

    #[test]
    fn original_token_memo_has_bounded_retention() {
        let (realm, _, config, _) = fixture("set x 1");
        for value in 0..=MAX_RETAINED_COMMANDS {
            let source = format!("set x {value}");
            let image = SourceImage::document(&source);
            let representatives =
                crate::segmenter::segment_commands_with_offset_and_config(&source, 0, config)[0]
                    .argv
                    .clone();
            assert!(
                realm
                    .retained_original_tokens(&image, config, 0, &representatives)
                    .is_some()
            );
        }
        assert_eq!(
            realm.original_tokens.entries.lock().unwrap().len(),
            MAX_RETAINED_COMMANDS
        );
    }
}

#[cfg(test)]
mod original_command_slice_purpose_tests {
    use super::*;

    #[test]
    fn original_token_memo_preserves_bom_heads_after_the_actual_file_prologue() {
        // Original command geometry/cache sharing only; an unknown head stays unknown.
        let mut prepared = crate::analyser::Analyser::new();
        let input = prepared.prepare_analysis_input("tcl9.0").unwrap();
        for (source, bom, expected_offset, expected_head) in [
            (
                "\u{feff}\u{feff}puts value",
                tcl_lexer::LeadingBom::Skip,
                3,
                "\u{feff}puts",
            ),
            (
                "\u{feff}puts value",
                tcl_lexer::LeadingBom::Content,
                0,
                "\u{feff}puts",
            ),
            ("\u{feff}puts value", tcl_lexer::LeadingBom::Skip, 3, "puts"),
        ] {
            let config = LexerConfig {
                leading_bom: bom,
                braced_var: tcl_lexer::BracedVarStyle::FirstClose,
                ..input.lexer_config()
            };
            let actual_input = crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                input.context_registry(),
                config,
            );
            let realm = crate::realm::document_realm_bindings_with_resolved_input(
                source,
                &actual_input,
                None,
            );
            let image = SourceImage::document(source);
            let parent =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            assert_eq!(parent[0].span.start(), expected_offset);
            let first = realm
                .retained_original_tokens(&image, config, expected_offset, &parent[0].argv)
                .unwrap();
            assert_eq!(first.argv_texts[0], expected_head);
            assert_eq!(first.argv[0], parent[0].argv[0].span);
            assert_eq!(
                first
                    .source_binding
                    .as_ref()
                    .and_then(|binding| binding.original_lexer_config_for_tokens(&first)),
                Some(config)
            );
            let again = realm
                .retained_original_tokens(&image, config, expected_offset, &parent[0].argv)
                .unwrap();
            assert!(Arc::ptr_eq(&first, &again));
            assert!(realm.matches_resolved_analysis_input(&actual_input));
            assert!(realm.matches_original_source_image(&image, config));
            if expected_head.starts_with('\u{feff}') {
                assert!(
                    realm
                        .invocation_at_source(expected_head, expected_offset)
                        .proved_execution_target()
                        .is_none()
                );
            }
        }
    }
}
