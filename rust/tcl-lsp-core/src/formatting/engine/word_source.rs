// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The source text of one argument word rebuilt from its tokens, and parameter-list spelling.

use super::super::config::FormatterConfig;
use super::{CommandArg, MAX_FORMAT_DEPTH};
use tcl_lexer::{SourceMap, Token, TokenType};
use tcl_syntax::word_rules::WordValueRules;

// Token → raw source reconstruction

/// Collapse `\<newline>` continuations to a single space.
///
/// `keep_preceding` controls whitespace *before* the backslash. Inside a
/// double-quoted string that whitespace is literal data, so it must survive
/// (`"a \<nl> b"` is the value `a  b`, two spaces); pass
/// `true`. Inside a command substitution `[…]` (and bare word contexts) the
/// whole `<ws>\<nl><ws>` run is inter-word spacing the lexer collapses to a
/// single space, so the preceding whitespace must be trimmed (`[cat a \<nl> b]`
/// → `[cat a b]`); pass `false`. Both delegate to the shared `tcl-syntax`
/// collapse, which also leaves an escaped backslash (`\\<nl>` — a literal `\`
/// before a real newline) alone rather than treating it as a continuation.
pub(super) fn normalise_backslash_newline(text: &str, keep_preceding: bool) -> String {
    if keep_preceding {
        // dialect-drift-ok: this reconstructs a *quoted* argument's or a
        // command substitution's interior, never a braced word, so the
        // dialect's `brace_backslash_newline` axis (which only governs what a
        // `{…}` word's bytes are) does not apply — every modelled runtime,
        // JimTcl included, folds a continuation here. The helper's name is the
        // shared collapse's, not a claim about braces.
        tcl_syntax::backslash::collapse_brace_continuations_str(text).into_owned()
    } else {
        tcl_syntax::backslash::collapse_separator_continuations_str(text).into_owned()
    }
}

/// Rebuild source text from a single token, re-adding delimiters.
///
/// `in_quotes` is set when the token is being reconstructed inside a
/// double-quoted argument (which the caller re-wraps in `"…"`). In that context
/// a `Str` token is literal string data — a lone `$` the lexer classified as
/// `Str`, for instance — so it is emitted verbatim rather than brace-wrapped as
/// `{$}`, which would change the string's value.
fn reconstruct_raw(
    sm: &SourceMap,
    tok: Token,
    in_quotes: bool,
    lexical_config: tcl_lexer::LexerConfig,
) -> Option<String> {
    Some(match tok.kind {
        TokenType::Str if in_quotes => sm.text(tok.span).to_owned(),
        TokenType::Str => format!("{{{}}}", sm.token_text(tok)),
        TokenType::Cmd => format!(
            "[{}]",
            super::super::source_layout::collapse_script_separator_continuations(
                sm.token_text(tok),
                lexical_config,
                MAX_FORMAT_DEPTH,
            )
            .unwrap_or_else(|| sm.token_text(tok).to_owned())
        ),
        TokenType::Var => {
            return original_variable_source(sm, tok, lexical_config)
                .map(|(source, _)| source.to_owned());
        }
        TokenType::Expand => "{*}".to_owned(),
        // `JimTcl`'s `$(…)`: reconstruct the sugar around the expression
        // body, or formatting a Jim file rewrites `$($a*2)` to `$a*2`.
        TokenType::ExprSugar => format!("$({})", sm.token_text(tok)),
        _ => sm.token_text(tok).to_owned(),
    })
}

/// Rebuild an argument's source text from its tokens, optionally
/// rewriting `$var` → `${var}`.
pub(super) fn reconstruct_arg(sm: &SourceMap, arg: &CommandArg, braced_vars: bool) -> String {
    let word = &arg.lexical_word;
    let lexical_config = word.config();
    let original = || sm.text(word.span()).to_owned();
    let mut baseline = String::new();
    let mut proposal = String::new();
    for &tok in &arg.tokens {
        let Some(raw) = reconstruct_raw(sm, tok, arg.is_quoted, lexical_config) else {
            return original();
        };
        baseline.push_str(&raw);
        let selected = if braced_vars && tok.kind == TokenType::Var {
            original_variable_source(sm, tok, lexical_config).and_then(|(_, reference)| {
                // A separately substituted index cannot become literal name data.
                if reference.index.is_some() {
                    return None;
                }
                let name = std::str::from_utf8(reference.name).ok()?;
                tcl_syntax::naming::scalar_source_reference(name, lexical_config, true)
            })
        } else {
            None
        };
        proposal.push_str(selected.as_deref().unwrap_or(&raw));
    }
    if arg.is_quoted {
        baseline = format!("\"{baseline}\"");
        proposal = format!("\"{proposal}\"");
    }
    if baseline == proposal {
        return baseline;
    }
    if same_variable_word_components(&baseline, &proposal, word) {
        proposal
    } else {
        // Other independently selected layout remains available, while the
        // unproved variable spelling stays exactly as it was written.
        baseline
    }
}

/// Reborrow the complete configured reference and authenticate the token's
/// exact span/content offset. The scanner owns every original closing byte.
pub(super) fn original_variable_source<'a>(
    map: &SourceMap<'a>,
    token: Token,
    config: tcl_lexer::LexerConfig,
) -> Option<(&'a str, tcl_lexer::word_parts::RawVarRef<'a>)> {
    if token.kind != TokenType::Var {
        return None;
    }
    let source = map.source_bytes();
    let at = usize::try_from(token.span.start().checked_sub(map.base_offset())?).ok()?;
    (source.get(at) == Some(&b'$')).then_some(())?;
    let reference = tcl_lexer::word_parts::scan_var_ref(source, at, config).ok()??;
    if reference.source_span(source, at, map.base_offset())? != token.span
        || reference.name_range_in(source)?.start
            != at.checked_add(usize::from(token.content_offset))?
    {
        return None;
    }
    Some((
        std::str::from_utf8(source.get(at..reference.next)?).ok()?,
        reference,
    ))
}

/// Compare full lexical components after proposing a reference style change.
/// This supplies source syntax only, independently of Native formatting/edit
/// eligibility, variable identity, cell ownership or evaluation equivalence.
pub(super) fn same_variable_word_components(
    baseline: &str,
    proposal: &str,
    original: &tcl_lexer::NativeWord,
) -> bool {
    let capture = |source: &str| {
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?),
            original.config(),
        )
        .ok()?;
        let [command] = parsed.commands.as_slice() else {
            return None;
        };
        let [word] = command.words.as_slice() else {
            return None;
        };
        (parsed.fatal_tail.is_none()
            && word.group().expand == original.group().expand
            && word.written_bytes() == source.as_bytes())
        .then(|| word.clone())
    };
    let Some(baseline) = capture(baseline) else {
        return false;
    };
    let Some(proposal) = capture(proposal) else {
        return false;
    };
    let (Ok(before), Ok(after)) = (baseline.parts(), proposal.parts()) else {
        return false;
    };
    before
        .iter()
        .map(|part| &part.part)
        .eq(after.iter().map(|part| &part.part))
}

/// Explicit Logical parameter-list spacing under the supplied word rules.
/// Native operands use `normalise_original_param_list` and its retained byte
/// proof instead. Malformed list text keeps its original spelling.
pub(super) fn normalise_param_list(text: &str, rules: WordValueRules) -> String {
    let collapsed = rules.collapse_braced_word(text);
    match rules.split_list(&collapsed) {
        Ok(elements) => format!("{{{}}}", tcl_syntax::list::join_list(elements)),
        Err(_) => format!("{{{text}}}"),
    }
}

/// Render only exact Unicode views of the actual native formal-list elements.
/// An opaque child cannot be repaired by a Unicode list decoder.
pub(super) fn native_param_list_source(
    input: &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
) -> Option<String> {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    let elements = input.original_list_elements()?;
    let values = elements
        .iter()
        .map(|element| std::str::from_utf8(element.bytes()).ok())
        .collect::<Option<Vec<_>>>()?;
    Some(format!("{{{}}}", tcl_syntax::list::join_list(values)))
}

/// Parse a spelling proposal with the original full grammar and native codec.
/// The proposal has lexical value geometry only; it never receives a source key.
fn original_static_spelling_value(
    spelling: &str,
    word: &tcl_lexer::NativeWord,
    input: &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
    config: &FormatterConfig,
) -> Option<Vec<u8>> {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    let key = input.original_word_key()?;
    if key.original_word() != word
        || word.config() != config.lexer_config()
        || word.image().channel() != tcl_lexer::SourceChannel::Document
        || word.group().kind != tcl_lexer::WordKind::Braced
        || word.group().expand
    {
        return None;
    }
    let original_content = word.content_span().ok()?;
    let raw = word.image().bytes().get(original_content.as_range())?;
    let source_units = tcl_syntax::backslash::source_literal_bytes(raw, word.image().channel());
    let native_units = tcl_syntax::backslash::native_source_literal_bytes(
        raw,
        word.image().channel(),
        input.policy().string_protocol(),
    )
    .ok()?;
    if source_units.as_ref() != native_units.as_ref() {
        return None;
    }
    let image = tcl_lexer::SourceImage::document(spelling);
    let plan = tcl_lexer::native_script_words_in(
        image,
        tcl_lexer::Span::new(0, u32::try_from(spelling.len()).ok()?),
        word.config(),
    )
    .ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    let [proposal] = command.words.as_slice() else {
        return None;
    };
    if plan.fatal_tail.is_some()
        || proposal.group().kind != tcl_lexer::WordKind::Braced
        || proposal.group().expand
    {
        return None;
    }
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        std::slice::from_ref(proposal),
        input.policy().string_protocol(),
    )
    .ok()?;
    Some(captured.literal(0)?.to_vec())
}

fn original_list_values_match(
    input: &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
    proposed_value: &[u8],
) -> Option<()> {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    let original = input.original_list_elements()?;
    let proposed =
        tcl_syntax::list::split_native_list_bytes(proposed_value, input.policy().string_protocol())
            .ok()?;
    (original.len() == proposed.len()
        && original
            .iter()
            .zip(&proposed)
            .all(|(original, proposed)| original.bytes() == proposed.as_ref()))
    .then_some(())
}

pub(super) fn normalise_original_param_list(
    word: &tcl_lexer::NativeWord,
    input: &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
    config: &FormatterConfig,
) -> Option<String> {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    let spelling = native_param_list_source(input)?;
    let proposed_value = original_static_spelling_value(&spelling, word, input, config)?;
    original_list_values_match(input, &proposed_value)?;
    Some(spelling)
}

pub(super) fn render_param_list_arg(
    sm: &SourceMap,
    arg: &CommandArg,
    config: &FormatterConfig,
    logical: bool,
) -> String {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    match (&arg.original_word, &arg.original_input) {
        (Some(word), Some(input)) => normalise_original_param_list(word, input, config)
            .unwrap_or_else(|| reconstruct_arg(sm, arg, false)),
        (Some(word), None) if logical && word.config() == config.lexer_config() => {
            normalise_param_list(&arg.text, config.word_rules())
        }
        _ => reconstruct_arg(sm, arg, false),
    }
}

/// The complete proposal must preserve the native parameter and namespace
/// values through both the source-word decoder and the lambda list parser.
pub(super) fn original_lambda_parameters_match(
    spelling: &str,
    word: &tcl_lexer::NativeWord,
    input: &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
    config: &FormatterConfig,
) -> Option<()> {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    let original = input.original_list_elements()?;
    let proposed_value = original_static_spelling_value(spelling, word, input, config)?;
    let proposed = tcl_syntax::list::split_native_list_bytes(
        &proposed_value,
        input.policy().string_protocol(),
    )
    .ok()?;
    if !matches!(original.len(), 2 | 3) || original.len() != proposed.len() {
        return None;
    }
    original_list_values_match(original.first()?, proposed.first()?.as_ref())?;
    (original
        .get(2)
        .map(tcl_compiler::signature_scan::scope::SignatureSourceNameInput::bytes)
        == proposed.get(2).map(std::convert::AsRef::as_ref))
    .then_some(())
}
