// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Structural list-build compatibility for script-argument source assistance.
//! Callback inventory uses `OriginalCallbackPrefix` and its original issuer.

use crate::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::{Token, TokenType};
use tcl_registry::CommandRegistry;

/// Legacy structural list-build assistance for the explicit script-argument
/// compatibility path. Original callback consumers use their typed issuer.
pub(crate) fn list_quoted_command_segment(
    registry: &CommandRegistry,
    tok: Token,
    text: &str,
) -> Option<SegmentedCommand> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    let content_start = tok.span.start() + u32::from(tok.content_offset);
    let mut segs = segment_commands_with_offset_and_config(
        inner,
        content_start,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    );
    if segs.len() != 1 {
        return None;
    }
    let seg = segs.pop()?;
    list_build_is_literal(registry, &seg).then_some(seg)
}

/// Whether an already-segmented command is a call to a
/// [`tcl_registry::Traits::BUILDS_COMMAND_PREFIX`] command (`list`) whose own
/// first argument is a literal, resolvable bareword.
///
/// Split out of [`list_quoted_command_segment`] so
/// [`crate::script_arg::list_build_effective_command`] applies exactly the
/// same guard to a segment it already holds, rather than re-deriving one.
pub(crate) fn list_build_is_literal(registry: &CommandRegistry, seg: &SegmentedCommand) -> bool {
    if seg.texts.len() < 2 {
        return false;
    }
    if !registry.get(&seg.texts[0]).is_some_and(|s| {
        s.traits
            .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
    }) {
        return false;
    }
    let Some(head_tok) = seg.argv.get(1) else {
        return false;
    };
    if head_tok.kind != TokenType::Esc
        || head_tok.in_quote
        || seg.single_token_word.get(1) != Some(&true)
    {
        return false;
    }
    // The whole-word token guard owns the absence of substitutions. Decoded
    // name characters do not replace that syntax fact with a spelling test.
    seg.texts.get(1).is_some_and(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::{AppendedArity, AppendedAritySet};

    fn trace_callback_arity(source: &str) -> tcl_registry::AppendedArity {
        let registry = CommandRegistry::build_default();
        let result = super::super::extract_signatures(source, &registry);
        let callbacks: Vec<_> = result
            .command_invocations
            .iter()
            .filter(|row| row.original_callback_prefix.is_some() && row.callback_arity.is_some())
            .collect();
        assert_eq!(callbacks.len(), 1);
        callbacks[0]
            .original_callback_prefix
            .as_ref()
            .unwrap()
            .appended_arity()
            .unwrap()
    }

    #[test]
    fn source_aware_callback_resolution_preserves_execution_arity_alternatives() {
        assert_eq!(
            trace_callback_arity("trace add execution target enter callback\n"),
            AppendedArity::Exactly(2)
        );
        assert_eq!(
            trace_callback_arity("trace add execution target leave callback\n"),
            AppendedArity::Exactly(4)
        );
        assert_eq!(
            trace_callback_arity("trace add execution target {enter leave} callback\n"),
            AppendedArity::OneOf(AppendedAritySet::from_sorted_unique(&[2, 4]))
        );
    }

    #[test]
    fn source_aware_callback_resolution_abstains_on_dynamic_and_malformed_lists() {
        assert_eq!(
            trace_callback_arity(
                "set operations {enter leave}\ntrace add execution target $operations callback\n"
            ),
            AppendedArity::Unknown
        );
        assert_eq!(
            trace_callback_arity("trace add execution target \"{enter\" callback\n"),
            AppendedArity::Unknown
        );
    }
}
