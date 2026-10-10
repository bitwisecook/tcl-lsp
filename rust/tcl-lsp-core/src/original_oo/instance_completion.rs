// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional source instance candidates, independent of receiver identity.

use tcl_compiler::analyser::{AnalysisResult, ClassDef};
use tcl_compiler::command_binding::OriginalSourceClassInstanceWords;
use tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata;
use tcl_compiler::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};

/// Complete source geometry retains either its conditional constructor receipt
/// or the separate positioned current receiver. Neither is an editable member.
pub(crate) struct InstanceCompletionSource<'a> {
    pub(crate) class: &'a SourceDeclarationMetadata<ClassDef>,
    words: InstanceCompletionWords,
}

enum InstanceCompletionWords {
    Current(Vec<NativeWord>),
    Conditional(Box<OriginalSourceClassInstanceWords>),
}

impl InstanceCompletionSource<'_> {
    pub(crate) fn original_words(&self) -> &[NativeWord] {
        match &self.words {
            InstanceCompletionWords::Current(words) => words,
            InstanceCompletionWords::Conditional(receipt) => receipt.original_words(),
        }
    }

    pub(crate) fn argument_count(&self) -> Option<usize> {
        let count = match &self.words {
            InstanceCompletionWords::Current(words) => words.len().checked_sub(1)?,
            InstanceCompletionWords::Conditional(receipt) => receipt.arguments().len(),
        };
        (0..count)
            .all(|ordinal| {
                self.argument_word(ordinal)
                    .is_some_and(|word| !word.group().expand)
            })
            .then_some(count)
    }

    pub(crate) fn argument_word(&self, ordinal: usize) -> Option<&NativeWord> {
        match &self.words {
            InstanceCompletionWords::Current(words) => words.get(ordinal.checked_add(1)?),
            InstanceCompletionWords::Conditional(receipt) => receipt.argument_word(ordinal),
        }
    }

    pub(crate) fn is_written_argument(&self, ordinal: usize) -> bool {
        self.argument_word(ordinal).is_some_and(|word| {
            self.original_words()
                .iter()
                .skip(1)
                .any(|written| written == word)
        })
    }

    pub(crate) fn argument_input(&self, ordinal: usize) -> Option<SignatureSourceNameInput> {
        if let InstanceCompletionWords::Conditional(receipt) = &self.words {
            // A canonical class cannot donate a Native naming purpose to
            // Logical source operands or an unretained conditional input.
            return receipt.argument_input(ordinal)?.native_input().cloned();
        }
        let word = self.argument_word(ordinal)?;
        let key = SignatureSourceNameKey::from_original_native_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&word.config()),
            self.class.name_input().policy(),
        )?;
        Some(SignatureSourceNameInput::OriginalWord(key))
    }

    pub(crate) fn selector_replacement(
        &self,
        source: &str,
        cursor: u32,
        wanted: &[u8],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<(Span, String)> {
        let config = self.original_words().first()?.config();
        if self.class.name_input().policy() != policy
            || !self.is_method_position(source, cursor, config)
        {
            return None;
        }
        let image = SourceImage::document(source);
        if self
            .argument_word(0)
            .is_some_and(|word| word.word_span().start() <= cursor)
        {
            let input = self.argument_input(0)?;
            let edits = crate::original_name_edit::original_name_input_edits(
                &image,
                &[(input, tcl_core_types::NameBytes::from(wanted))],
            )?;
            let [edit] = edits.as_slice() else {
                return None;
            };
            return Some((edit.span(), edit.text().to_owned()));
        }
        Some((
            Span::empty(cursor),
            tcl_syntax::backslash::native_literal_source_word(
                wanted,
                image.channel(),
                config,
                policy.string_protocol(),
            )?,
        ))
    }

    fn is_method_position(&self, source: &str, cursor: u32, config: LexerConfig) -> bool {
        let Some(head) = self.original_words().first() else {
            return false;
        };
        if self.argument_count().is_none() || head.word_span().end() > cursor {
            return false;
        }
        if let Some(selector) = self.argument_word(0) {
            // A captured alias selector already supplies the method. Its
            // declaration word never borrows the call's first written span.
            if !self.is_written_argument(0) {
                return false;
            }
            if selector.word_span().start() <= cursor {
                return cursor <= selector.word_span().end();
            }
        }
        completion_gap(source, head.word_span().end(), cursor, config)
    }
}

/// Conditional source candidates keep their whole receipt and canonical class
/// join. Missing dispatch cannot be reconstructed from document-final maps.
pub(crate) fn instance_completion_source<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cursor: u32,
) -> Option<InstanceCompletionSource<'a>> {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let config = analysis.body_lexer_config?;
    let structure =
        crate::source_structure::SourceStructure::capture(source, Some(analysis), config)?;
    let command = structure
        .commands
        .iter()
        .filter(|command| {
            command
                .argv
                .first()
                .is_some_and(|head| head.span.start() < cursor)
        })
        .max_by_key(|command| command.argv[0].span.start())?;
    let offset = command.argv.first()?.span.start();
    if let Some(receipt) =
        tcl_compiler::registry_invocation::source_structure::source_class_instance_words_at(
            source, analysis, offset,
        )
    {
        let class = receipt
            .instance()
            .class_declaration()
            .source_class(analysis)?;
        return Some(InstanceCompletionSource {
            class,
            words: InstanceCompletionWords::Conditional(Box::new(receipt)),
        });
    }
    // Current instance allocations remain an independent candidate source.
    // Argument-prepending aliases require their complete source receipt above.
    let realm = analysis.retained_command_realm()?;
    if realm
        .invocation_at_source(command.name(), offset)
        .proved_target()
        .is_some_and(|target| !target.prepended.is_empty())
    {
        return None;
    }
    let class = crate::receiver_identity::class_at_command_head(analysis, source, command)?;
    let mut records = analysis
        .original_class_declarations()
        .filter(|record| std::ptr::eq(record.metadata(), class));
    let class = records.next()?;
    if records.next().is_some() {
        return None;
    }
    let image = SourceImage::document(source);
    let plan = tcl_lexer::native_script_words_in(
        image,
        Span::new(offset, u32::try_from(source.len()).ok()?),
        config,
    )
    .ok()?;
    let words = plan.commands.first()?.words.clone();
    (words.first()?.span() == command.argv.first()?.span).then_some(InstanceCompletionSource {
        class,
        words: InstanceCompletionWords::Current(words),
    })
}

pub(crate) fn instance_method_completion_record<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cursor: u32,
) -> Option<&'a SourceDeclarationMetadata<ClassDef>> {
    let receiver = instance_completion_source(analysis, source, cursor)?;
    receiver
        .is_method_position(source, cursor, analysis.body_lexer_config?)
        .then_some(receiver.class)
}

/// Original intra-command separators use the selected grammar and shared
/// continuation owner. A bare newline, separator or comment is another site.
pub(crate) fn completion_gap(source: &str, start: u32, cursor: u32, config: LexerConfig) -> bool {
    let Some(gap) = source.get(start as usize..cursor as usize) else {
        return false;
    };
    if gap.is_empty() {
        return false;
    }
    let bytes = gap.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if config.word_separators.is_separator(bytes[at]) {
            at += 1;
        } else if let Some(end) = tcl_lexer::source_backslash_continuation_end(
            bytes,
            at,
            tcl_lexer::SourceChannel::Document,
        ) {
            at = end;
        } else {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests;
