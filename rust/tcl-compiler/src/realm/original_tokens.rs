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
        // Implementation contract: naming.source.original-command-token-memo
        // docs/design/analysis/name-resolution-proofs/original-command-token-memo.md
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
        // Implementation contract: naming.source.original-command-token-memo
        // docs/design/analysis/name-resolution-proofs/original-command-token-memo.md
        let key = OriginalCommandTokenKey {
            image: image.clone(),
            config,
            offset,
            representatives: representatives.to_vec(),
        };
        self.original_tokens.get_or_prepare(key, || {
            #[cfg(debug_assertions)]
            let started = std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
                .is_some()
                .then(std::time::Instant::now);
            let source_map = SourceMap::from_image(image);
            let end = if let Some(last) = representatives.last() {
                tcl_lexer::word_span(&source_map, *last).end()
            } else {
                self.original_zero_argument_head_end(image, config, offset)?
            };
            let text = image.try_text().ok()?.get(offset as usize..end as usize)?;
            let segmented =
                crate::segmenter::segment_commands_with_offset_and_config(text, offset, config)
                    .into_iter()
                    .next()?;
            let mut tokens = CommandTokens::from_segmented(&source_map, config, &segmented);
            self.stamp_original_tokens(&mut tokens);
            #[cfg(debug_assertions)]
            if let Some(started) = started {
                eprintln!(
                    "ANALYSER_TOKEN_PREPARE bytes={} site={offset} ms={} nested={}",
                    image.bytes().len(),
                    started.elapsed().as_millis(),
                    tokens.nested_bindings.len()
                );
            }
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
        // naming.source.original-command-token-memo
        // docs/design/analysis/name-resolution-proofs/original-command-token-memo.md
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
        // Implementation contract: naming.source.original-command-token-memo
        // docs/design/analysis/name-resolution-proofs/original-command-token-memo.md
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
        // Implementation contract: naming.source.original-command-token-memo
        // docs/design/analysis/name-resolution-proofs/original-command-token-memo.md
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
        // Implementation contract: naming.source.original-command-token-memo
        // docs/design/analysis/name-resolution-proofs/original-command-token-memo.md
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
        // Implementation contract: naming.source.original-command-token-memo
        // docs/design/analysis/name-resolution-proofs/original-command-token-memo.md
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
