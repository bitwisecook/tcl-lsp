// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Diagnostic operands selected by their retained original source owners.

use std::sync::Arc;

use crate::analyser::diagnostic_registry::{
    DeclaredSourceDiagnosticKind, DeclaredSourceDiagnosticSubject, OriginalDiagnosticInvocation,
    RegistrySourceDiagnosticKind,
};

pub(super) struct OriginalChannelArgument {
    pub word: tcl_lexer::NativeWord,
    pub literal: Option<Vec<u8>>,
    pub subject: crate::analyser::DiagnosticSubject,
}

/// Complete roles and genuine written operands are independent requirements.
/// Captured prefix values without a call-site word cannot acquire an anchor.
pub(super) fn channel_arguments(
    source: &str,
    analysis: &crate::analyser::AnalysisResult,
    tokens: &crate::ir::CommandTokens,
    context: Arc<tcl_registry::model::ContextRegistry>,
) -> Vec<OriginalChannelArgument> {
    use crate::registry_invocation::source_structure::{
        original_declared_words_for_tokens, original_registry_words_for_tokens,
    };
    if let Some(original) = original_registry_words_for_tokens(source, analysis, tokens)
        .and_then(|words| OriginalDiagnosticInvocation::new(words, context))
    {
        return original
            .words()
            .roles()
            .into_iter()
            .flatten()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::Channel)
            .filter_map(|&(argument, _)| {
                Some(OriginalChannelArgument {
                    word: original.word(argument)?.clone(),
                    literal: original
                        .words()
                        .arguments()
                        .get(argument)?
                        .literal_bytes()
                        .map(<[u8]>::to_vec),
                    subject: original.subject(
                        RegistrySourceDiagnosticKind::ChannelArgument,
                        Some(argument),
                    )?,
                })
            })
            .collect();
    }
    let Some(words) = original_declared_words_for_tokens(source, analysis, tokens) else {
        return Vec::new();
    };
    let Some(roles) = words.supplied_argument_roles() else {
        return Vec::new();
    };
    let words = Arc::new(words);
    roles
        .into_iter()
        .filter(|(_, role)| *role == tcl_registry::ArgRole::Channel)
        .filter_map(|(argument, _)| {
            let word = words.argument_word(argument)?;
            if word.group().expand {
                return None;
            }
            Some(OriginalChannelArgument {
                word: word.clone(),
                literal: words
                    .arguments()
                    .get(argument)?
                    .literal_bytes()
                    .map(<[u8]>::to_vec),
                subject: DeclaredSourceDiagnosticSubject::at_extent(
                    Arc::clone(&words),
                    DeclaredSourceDiagnosticKind::ChannelArgument,
                    word.span(),
                )?,
            })
        })
        .collect()
}

/// Whole scalar substitution, as decomposed by the original word scanner.
/// Interpolation, array indices and braced literal dollars do not qualify.
pub(super) fn scalar_variable_reference(
    word: &tcl_lexer::NativeWord,
) -> Option<(String, tcl_lexer::Span)> {
    let parts = word.parts().ok()?;
    let [part] = parts.as_slice() else {
        return None;
    };
    let tcl_lexer::WordPart::Variable(variable) = &part.part else {
        return None;
    };
    if variable.index.is_some() {
        return None;
    }
    Some((
        std::str::from_utf8(variable.name).ok()?.to_owned(),
        tcl_lexer::Span::new(
            u32::try_from(part.start).ok()?,
            u32::try_from(part.end).ok()?,
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channels(source: &str, profile: &str) -> Vec<OriginalChannelArgument> {
        let analysis = crate::analyser::Analyser::new().analyse(source, profile);
        let config = analysis.body_lexer_config.unwrap();
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            segments.last().unwrap(),
        );
        analysis
            .retained_command_realm()
            .unwrap()
            .stamp_original_tokens(&mut tokens);
        channel_arguments(
            source,
            &analysis,
            &tokens,
            analysis.resolved_input.as_ref().unwrap().context_registry(),
        )
    }

    #[test]
    fn original_channel_roles_keep_values_words_and_declaration_owners() {
        // Implementation ownership check; does not establish channel existence.
        // naming.diagnostic.original-channel-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-channel-source-ownership.md
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let source = "puts {$literal} message";
            let operands = channels(source, profile);
            assert_eq!(operands.len(), 1, "{profile}");
            assert_eq!(operands[0].literal.as_deref(), Some(b"$literal".as_slice()));
            assert!(scalar_variable_reference(&operands[0].word).is_none());
            assert_eq!(&source[operands[0].word.span().as_range()], "{$literal}");
            let source = "puts ${channel} message";
            // An unknown first selector can change the optional flag layout.
            assert!(channels(source, profile).is_empty());
            let source = "puts -nonewline ${channel} message";
            let operands = channels(source, profile);
            assert_eq!(operands.len(), 1, "{profile}");
            assert_eq!(
                scalar_variable_reference(&operands[0].word)
                    .map(|(name, _)| name)
                    .as_deref(),
                Some("channel")
            );
            assert!(operands[0].literal.is_none());
            assert!(channels("proc puts {a b} {}\nputs invalid message", profile).is_empty());
        }
        let orphan =
            "# tcl-lsp: stub write_channel {channel:channel value}\nwrite_channel bogus hello";
        assert!(channels(orphan, "tcl8.6").is_empty());
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub write_channel {channel:channel value}\n# tcl-lsp: stubs-end\nwrite_channel bogus hello";
        let operands = channels(source, "tcl8.6");
        assert_eq!(operands.len(), 1);
        assert!(matches!(
            operands[0].subject,
            crate::analyser::DiagnosticSubject::DeclaredSource(_)
        ));
    }

    #[test]
    fn original_channel_diagnostics_preserve_typed_anchor_and_shadow_controls() {
        // naming.diagnostic.original-channel-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-channel-source-ownership.md
        for source in [
            "puts bogus hello",
            "puts {bogus} hello",
            "puts {$literal} hello",
            "rename puts output\noutput bogus hello",
        ] {
            let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            let diagnostic = result
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W126)
                .unwrap_or_else(|| panic!("missing channel diagnostic for {source}"));
            let expected = if source.contains("{$literal}") {
                "{$literal}"
            } else if source.contains("{bogus}") {
                "{bogus}"
            } else {
                "bogus"
            };
            assert_eq!(&source[diagnostic.span.as_range()], expected);
            assert_eq!(
                diagnostic.registry_source().unwrap().kind(),
                RegistrySourceDiagnosticKind::ChannelArgument
            );
        }
        for source in [
            "puts stdout hello",
            "puts stderr hello",
            "puts stdin hello",
            "puts {stdout} hello",
            "puts hello",
            "interp alias {} output {} puts bogus\noutput hello",
            "proc puts {a b} {}\nputs bogus hello",
        ] {
            let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            assert!(
                result
                    .diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.code != tcl_core_types::DiagCode::W126),
                "{source}"
            );
        }
    }
}
