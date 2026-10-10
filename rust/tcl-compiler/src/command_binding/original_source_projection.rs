// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Derived lexical projections of one immutable original source instance.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::ir::WordExpr;
use crate::registry_invocation::OriginalSourceCommandProjection;
use tcl_lexer::{LexerConfig, SourceImage};

type ProjectionKey = (u32, LexerConfig);
type ProjectionEntries = HashMap<ProjectionKey, Vec<Arc<ProjectionEntry>>>;

struct ProjectionEntry {
    words: Arc<[WordExpr]>,
    original: OnceLock<Option<Arc<OriginalSourceCommandProjection>>>,
}

/// The containing SourceOriginId owns the immutable image and source channel.
/// A bucket key never substitutes for exact complete vector correspondence.
pub(super) struct OriginalSourceProjectionMemo {
    entries: Mutex<ProjectionEntries>,
    #[cfg(any(test, debug_assertions))]
    trace: bool,
    #[cfg(any(test, debug_assertions))]
    requests: std::sync::atomic::AtomicUsize,
    #[cfg(any(test, debug_assertions))]
    captures: std::sync::atomic::AtomicUsize,
    #[cfg(any(test, debug_assertions))]
    rejected: std::sync::atomic::AtomicUsize,
}

impl Default for OriginalSourceProjectionMemo {
    fn default() -> Self {
        Self {
            entries: Mutex::default(),
            #[cfg(any(test, debug_assertions))]
            trace: std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some(),
            #[cfg(any(test, debug_assertions))]
            requests: std::sync::atomic::AtomicUsize::default(),
            #[cfg(any(test, debug_assertions))]
            captures: std::sync::atomic::AtomicUsize::default(),
            #[cfg(any(test, debug_assertions))]
            rejected: std::sync::atomic::AtomicUsize::default(),
        }
    }
}

// Derived cache warmth is neither semantic identity nor diagnostic output.
impl std::fmt::Debug for OriginalSourceProjectionMemo {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("OriginalSourceProjectionMemo")
    }
}

impl OriginalSourceProjectionMemo {
    pub(super) fn capture(
        &self,
        image: &SourceImage,
        words: &[WordExpr],
        offset: u32,
        config: LexerConfig,
    ) -> Option<Arc<OriginalSourceCommandProjection>> {
        #[cfg(any(test, debug_assertions))]
        if self.trace {
            self.requests
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        let entry = {
            let mut entries = self
                .entries
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let candidates = entries.entry((offset, config)).or_default();
            if let Some(entry) = candidates
                .iter()
                .find(|entry| entry.words.as_ref() == words)
            {
                Arc::clone(entry)
            } else {
                let entry = Arc::new(ProjectionEntry {
                    words: Arc::from(words),
                    original: OnceLock::new(),
                });
                candidates.push(Arc::clone(&entry));
                entry
            }
        };
        // Neither the original parser nor a future callback runs under the
        // memo mutex. Failure depends only on this immutable lexical producer.
        entry
            .original
            .get_or_init(|| {
                #[cfg(any(test, debug_assertions))]
                if cfg!(test) || self.trace {
                    self.captures
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                let original =
                    OriginalSourceCommandProjection::capture(image, &entry.words, offset, config)
                        .map(Arc::new);
                #[cfg(any(test, debug_assertions))]
                if self.trace && original.is_none() {
                    self.rejected
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                original
            })
            .clone()
    }

    #[cfg(any(test, debug_assertions))]
    pub(super) fn trace_counts(&self) -> (usize, usize, usize) {
        use std::sync::atomic::Ordering::Relaxed;
        (
            self.requests.load(Relaxed),
            self.captures.load(Relaxed),
            self.rejected.load(Relaxed),
        )
    }

    #[cfg(test)]
    pub(super) fn capture_count(&self) -> usize {
        self.captures.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::SourceOriginId;
    use crate::ir::{CommandTokens, Provenance};

    fn original_words(image: &SourceImage, config: LexerConfig) -> (u32, Vec<WordExpr>) {
        let commands =
            crate::segmenter::segment_commands_image_with_offset_and_config(image, 0, config)
                .unwrap();
        let command = commands.last().unwrap();
        (
            command.span.start(),
            CommandTokens::from_segmented(&image.source_map(), config, command)
                .words()
                .to_vec(),
        )
    }

    #[test]
    fn original_lexical_projection_reuses_complete_geometry_across_source_queries() {
        // naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        let source = "# original prefix\r\nlist {} \"\" café \"\\x00\" {A\0B} {a\\}} [list value]";
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(name).analyser_profile();
            let config = LexerConfig::from_grammar(profile.grammar);
            for image in [
                SourceImage::document(source),
                SourceImage::native(source.as_bytes()),
            ] {
                let (offset, words) = original_words(&image, config);
                assert!(offset > 0);
                let owner = SourceOriginId::authored_image(image.clone());
                let first = owner
                    .original_lexical_command(&words, offset, config)
                    .unwrap();
                let second = owner
                    .original_lexical_command(&words, offset, config)
                    .unwrap();
                assert!(Arc::ptr_eq(&first, &second));
                assert_eq!(owner.original_lexical_projection.capture_count(), 1);
                let uncached = crate::registry_invocation::original_native_compiler_words(
                    &image, &words, offset, config,
                )
                .unwrap();
                assert_eq!(first.native_words(), uncached);
                assert_eq!(
                    CommandTokens::from_segmented(&image.source_map(), config, first.command())
                        .words(),
                    words,
                );
                assert_eq!(
                    &image.bytes()[first.native_words()[1].span().as_range()],
                    b"{}"
                );
                assert_eq!(
                    &image.bytes()[first.native_words()[2].span().as_range()],
                    b"\"\""
                );
                assert!(
                    first
                        .native_words()
                        .iter()
                        .all(|word| word.image().channel() == image.channel())
                );
            }
        }
    }

    #[test]
    fn original_lexical_projection_refuses_changed_vector_grammar_and_partial_source() {
        // naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let config = LexerConfig::from_grammar(profile.grammar);
        let image = SourceImage::document("list {} {*}{one two}");
        let (offset, words) = original_words(&image, config);
        let owner = SourceOriginId::authored_image(image.clone());
        assert!(
            owner
                .original_lexical_command(&words, offset, config)
                .is_some()
        );
        let mut changed = words.clone();
        let WordExpr::BracedLiteral { text, .. } = &mut changed[1] else {
            panic!("braced original")
        };
        *text = "changed".into();
        let mut derived = words.clone();
        let WordExpr::Literal { source, .. } = &mut derived[0] else {
            panic!("source head")
        };
        source.provenance = Provenance::Opaque;
        let mut old_grammar = config;
        old_grammar.expand_syntax = false;
        for (vector, start, lexer) in [
            (changed.as_slice(), offset, config),
            (derived.as_slice(), offset, config),
            (words.as_slice(), offset + 1, config),
            (words.as_slice(), offset, old_grammar),
        ] {
            assert!(
                crate::registry_invocation::original_native_compiler_words(
                    &image, vector, start, lexer
                )
                .is_none()
            );
            assert!(
                owner
                    .original_lexical_command(vector, start, lexer)
                    .is_none()
            );
            let count = owner.original_lexical_projection.capture_count();
            assert!(
                owner
                    .original_lexical_command(vector, start, lexer)
                    .is_none()
            );
            assert_eq!(owner.original_lexical_projection.capture_count(), count);
        }
        let partial = SourceImage::document("list {");
        let (start, vector) = original_words(&partial, config);
        let malformed = SourceOriginId::authored_image(partial.clone());
        assert!(
            crate::registry_invocation::original_native_compiler_words(
                &partial, &vector, start, config
            )
            .is_none()
        );
        assert!(
            malformed
                .original_lexical_command(&vector, start, config)
                .is_none()
        );
    }

    #[test]
    fn original_lexical_projection_keeps_cache_warmth_and_independent_origins_separate() {
        // naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        use std::hash::{Hash, Hasher};
        let config = LexerConfig::default();
        let image = SourceImage::document("list {literal} café");
        let (offset, words) = original_words(&image, config);
        let owner = SourceOriginId::authored_image(image.clone());
        let cloned = owner.clone();
        let independent = SourceOriginId::authored_image(image);
        let debug = format!("{owner:?}");
        let mut before = std::collections::hash_map::DefaultHasher::new();
        owner.hash(&mut before);
        let original = owner
            .original_lexical_command(&words, offset, config)
            .unwrap();
        let from_clone = cloned
            .original_lexical_command(&words, offset, config)
            .unwrap();
        let separate = independent
            .original_lexical_command(&words, offset, config)
            .unwrap();
        assert!(Arc::ptr_eq(&original, &from_clone));
        assert!(!Arc::ptr_eq(&original, &separate));
        assert_eq!(original.native_words(), separate.native_words());
        assert_eq!(owner.original_lexical_projection.capture_count(), 1);
        assert_eq!(independent.original_lexical_projection.capture_count(), 1);
        assert_eq!(owner, independent);
        assert_eq!(owner.cmp(&independent), std::cmp::Ordering::Equal);
        let mut after = std::collections::hash_map::DefaultHasher::new();
        owner.hash(&mut after);
        assert_eq!(before.finish(), after.finish());
        assert_eq!(debug, format!("{owner:?}"));
        let concurrent =
            SourceOriginId::authored_image(SourceImage::document("list {literal} café"));
        let barrier = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            let handles = (0..8)
                .map(|_| {
                    let concurrent = &concurrent;
                    let barrier = &barrier;
                    let words = &words;
                    scope.spawn(move || {
                        barrier.wait();
                        concurrent
                            .original_lexical_command(words, offset, config)
                            .unwrap()
                    })
                })
                .collect::<Vec<_>>();
            let results = handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>();
            assert!(
                results
                    .iter()
                    .all(|result| Arc::ptr_eq(result, &results[0]))
            );
        });
        assert_eq!(concurrent.original_lexical_projection.capture_count(), 1);
    }
}
