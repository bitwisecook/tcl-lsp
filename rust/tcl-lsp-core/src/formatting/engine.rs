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

//! Core formatting engine.
//!
//! Parses Tcl source into commands, identifies body / expr /
//! param-list arguments via the registry, recursively formats
//! bodies, and reconstructs the output (K&R braces, blank-line
//! policy, comment normalisation, switch bodies, long-line
//! backslash splitting, and `&&`/`||` expression wrapping).
//!
//! The entry point is [`format_tcl`]; everything else is the
//! private machinery [`format_tcl`] drives.

use tcl_compiler::lambda_literal::{
    DecodedLambdaLiteral, split_original_lambda_literal_decoded,
    split_original_lambda_literal_lexical_decoded,
};
use tcl_lexer::{Lexer, SourceMap, Token, TokenType};
use tcl_registry::{ArgRole, CaseListSpec, CommandRegistry, Traits};

use super::config::FormatterConfig;
use tcl_syntax::word_rules::WordValueRules;

/// Depth cap for [`format_body`]'s (and [`format_case_list_body`]'s) recursion
/// over nested control-flow bodies. Reuses their existing
/// `indent_level` parameter as the depth signal rather than threading a
/// separate one: `indent_level` already increments by exactly one per
/// nested body, the same shape the depth cap needs.
///
/// This crate is consumed both by binaries that run formatting on a
/// generously-sized dedicated thread (`tcl-lsp-server`, the `tcl` CLI,
/// `f5-cli`) and, via `bigip-query-wasm`, from a WASM host whose stack
/// budget is outside this crate's control — so, like
/// `tcl_runtime::interp::NATIVE_EVAL_DEPTH_LIMIT`, this must be safe on a
/// small ambient stack, not just a generously-sized one.
///
/// Empirically measured on this crate's native build, run on a plain 2 MiB
/// thread stack (`cargo test`'s per-test default): unguarded nested `if`
/// bodies overflow the stack (SIGABRT) between depth 800 and 1200 — a much
/// larger margin than the analyser's or the optimiser passes' (this
/// recursion's per-level frame cost is lighter), but 128 is kept
/// consistent with the same conservative reasoning used for the WASM
/// runtime: comfortably safe even against a meaningfully smaller WASM
/// stack, while still far more headroom than realistic (even
/// generated/templated) Tcl needs to format.
const MAX_FORMAT_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(128);

/// Check the actual lexical resource budget before retaining semantic source
/// structure. Excessively nested data is conservatively preserved as a whole;
/// lexical containment alone never identifies a command or script operand.
pub(crate) fn source_within_formatting_budget(source: &str, config: &FormatterConfig) -> bool {
    // Implementation contract: naming.editor.original-source-formatting-budget
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting-budget.md
    matches!(
        Lexer::with_config(source, config.lexer_config())
            .braced_word_nesting_within(MAX_FORMAT_DEPTH),
        Ok(true)
    )
}

/// What kind of argument this is for formatting purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgKind {
    /// Plain argument — reconstruct from tokens.
    Word,
    /// Script body — recursively format, wrap in `{}`.
    Body,
    /// Structural keyword (`else`, `elseif`, `then`, `finally`,
    /// `on`, `trap`).
    Keyword,
    /// Parameter list — normalise internal whitespace.
    ParamList,
    /// `ArgRole::LambdaLiteral` argument (`apply`'s `{argList body ?ns?}`
    /// shape) — the parameter-list element is normalised and the body
    /// element recursively formatted, then reassembled; never fed to
    /// `format_body` as a whole (that would misread the parameter word as a
    /// command name).
    LambdaLiteral,
}

/// A single argument to a command.
struct CommandArg {
    kind: ArgKind,
    tokens: Vec<Token>,
    /// Concatenated token text (delimiters stripped by the lexer).
    text: String,
    /// First token was a braced `{…}` literal.
    is_braced: bool,
    /// The argument was `"`-quoted.
    is_quoted: bool,
    /// The recursively reformatted script content of a `Body` argument, or
    /// (for `LambdaLiteral`) of just its body *element* — the parameter-list
    /// element is never fed here, only normalised at reconstruction time
    /// (`render_lambda_literal_arg`).
    formatted_body: Option<String>,
    /// Source-only layout of this genuine selected definition-member body.
    member_body_presentation: Option<tcl_registry::definer::SourceDefinitionMemberBodyPresentation>,
    /// Original local lexical word, independently of analysis/value receipts.
    lexical_word: tcl_lexer::NativeWord,
    original_word: Option<tcl_lexer::NativeWord>,
    original_input: Option<tcl_compiler::signature_scan::scope::SignatureSourceNameInput>,
}

/// A comment line captured during parsing, with its original source
/// indentation so the formatter can either re-indent it to the code column
/// (`align_comments_to_code`, the default) or preserve where the author put it.
#[derive(Clone)]
struct CommentLine {
    /// Leading whitespace on the comment's original line, when the comment
    /// stood alone at the start of its line. `None` for a relocated inline
    /// (`;#`) comment, which has no standalone column to preserve.
    orig_indent: Option<String>,
    /// The comment text, starting at `#`.
    text: String,
}

/// A single Tcl command with its arguments, ready for reformatting.
struct ParsedCommand {
    /// Byte offset of the written command head in this formatter slice.
    head_start: u32,
    /// Canonical schema selected by the complete original source head.
    /// Empty when advice is unavailable or effective argv has a captured prefix.
    resolved_name: String,
    /// Original Registry source roles mapped to their own written operands.
    /// An empty vector is a terminal refusal to borrow nominal head metadata.
    source_roles: Option<Vec<(usize, ArgRole)>>,
    /// Explicit compatibility metadata retains the complete lenient context.
    /// These roles retain original words without Native inputs or recipes.
    logical_roles: Option<Vec<(usize, ArgRole)>>,
    logical_context: Option<std::sync::Arc<tcl_registry::model::ContextRegistry>>,
    source_traits: Option<Traits>,
    source_words: Option<crate::original_invocation::OriginalRegistryWords>,
    source_case: Option<(usize, CaseListSpec)>,
    expression_bracing: Option<(usize, String)>,
    args: Vec<CommandArg>,
    preceding_comments: Vec<CommentLine>,
    preceding_blank_lines: usize,
    /// The separator that terminated this command was a bare `;` on the same
    /// line (no newline). Lets the emitter keep `a; b` on one line when
    /// `replace_semicolons_with_newlines` is disabled.
    terminated_by_semicolon: bool,
}

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
fn normalise_backslash_newline(text: &str, keep_preceding: bool) -> String {
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
            super::source_layout::collapse_script_separator_continuations(
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
fn reconstruct_arg(sm: &SourceMap, arg: &CommandArg, braced_vars: bool) -> String {
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
fn original_variable_source<'a>(
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
fn same_variable_word_components(
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
fn normalise_param_list(text: &str, rules: WordValueRules) -> String {
    let collapsed = rules.collapse_braced_word(text);
    match rules.split_list(&collapsed) {
        Ok(elements) => format!("{{{}}}", tcl_syntax::list::join_list(elements)),
        Err(_) => format!("{{{text}}}"),
    }
}

/// Render only exact Unicode views of the actual native formal-list elements.
/// An opaque child cannot be repaired by a Unicode list decoder.
fn native_param_list_source(
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

fn normalise_original_param_list(
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

fn render_param_list_arg(
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
fn original_lambda_parameters_match(
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

// Command parsing

/// Count blank lines represented by an EOL token's text (each
/// newline beyond the first is a blank line).
fn count_newlines(text: &str) -> usize {
    text.matches('\n').count().saturating_sub(1)
}

/// The leading whitespace on the source line containing `offset`, but only when
/// everything before `offset` on that line is whitespace — i.e. the comment
/// stands alone at line start. Returns `None` for a mid-line (inline `;#`)
/// comment, which has no standalone column to preserve.
fn comment_orig_indent(source: &str, offset: usize) -> Option<String> {
    let line_start = source.get(..offset)?.rfind('\n').map_or(0, |p| p + 1);
    let prefix = source.get(line_start..offset)?;
    if prefix.bytes().all(|b| b == b' ' || b == b'\t') {
        Some(prefix.to_owned())
    } else {
        None
    }
}

/// Parse Tcl source into structured commands plus any trailing
/// comments.
fn parse_commands(
    source: &str,
    sm: &SourceMap,
    tokens: &[Token],
    config: tcl_lexer::LexerConfig,
) -> Option<(Vec<ParsedCommand>, Vec<CommentLine>)> {
    if sm.base_offset() != 0 || sm.source_bytes() != source.as_bytes() {
        return None;
    }
    let image = tcl_lexer::SourceImage::document(source);
    let mut words = std::collections::HashMap::new();
    for command in tcl_lexer::group_commands(tokens, source, config) {
        for group in command.words {
            let word =
                tcl_lexer::NativeWord::from_group(image.clone(), config, tokens, &group).ok()?;
            if words.insert(word.span().start(), word).is_some() {
                return None;
            }
        }
    }
    let mut commands: Vec<ParsedCommand> = Vec::new();
    let mut pending_comments: Vec<CommentLine> = Vec::new();
    let mut pending_blank_lines = 0usize;

    let mut argv: Vec<CommandArg> = Vec::new();
    let mut prev_type = TokenType::Eol;

    for &tok in tokens {
        match tok.kind {
            TokenType::Eof => break,
            TokenType::Comment => {
                pending_comments.push(CommentLine {
                    orig_indent: comment_orig_indent(source, tok.span.start() as usize),
                    text: sm.text(tok.span).to_owned(),
                });
                continue;
            }
            TokenType::Sep => {
                prev_type = TokenType::Sep;
                continue;
            }
            TokenType::Eol => {
                let newlines = count_newlines(sm.text(tok.span));
                if argv.is_empty() {
                    pending_blank_lines += newlines;
                } else {
                    // A pure `;` separator (no newline in the Eol run) means
                    // this command shared a line with the next.
                    flush_parsed_command(
                        &mut argv,
                        &mut pending_comments,
                        pending_blank_lines,
                        newlines == 0,
                        &mut commands,
                    );
                    pending_blank_lines = newlines;
                }
                prev_type = TokenType::Eol;
                continue;
            }
            _ => {}
        }

        // segmentation-drift-ok: the formatter's own
        // `parse_commands` is a word grouper that must also keep the trivia
        // (comments, blank-line runs) `WordSpan` deliberately does not carry.
        let is_start_of_new_arg = matches!(prev_type, TokenType::Sep | TokenType::Eol);
        let detected_quoted =
            is_start_of_new_arg && source.as_bytes().get(tok.span.start() as usize) == Some(&b'"');
        let text = sm.token_text(tok).to_owned();

        if is_start_of_new_arg || argv.is_empty() {
            argv.push(CommandArg {
                kind: ArgKind::Word,
                tokens: vec![tok],
                text,
                is_braced: tok.kind == TokenType::Str,
                is_quoted: detected_quoted,
                formatted_body: None,
                member_body_presentation: None,
                lexical_word: words.get(&tok.span.start())?.clone(),
                original_word: None,
                original_input: None,
            });
        } else {
            let last = argv.last_mut().expect("argv non-empty");
            last.tokens.push(tok);
            last.text.push_str(&text);
        }
        prev_type = tok.kind;
    }

    flush_parsed_command(
        &mut argv,
        &mut pending_comments,
        pending_blank_lines,
        false,
        &mut commands,
    );
    for arg in commands.iter().flat_map(|command| &command.args) {
        if !arg
            .tokens
            .iter()
            .filter(|token| token.kind != TokenType::Expand)
            .eq(arg.lexical_word.tokens().iter())
            || arg
                .tokens
                .iter()
                .any(|token| token.kind == TokenType::Expand)
                != arg.lexical_word.group().expand
        {
            return None;
        }
    }
    Some((commands, pending_comments))
}

/// Current source grammar for authoring layout. This carries no execution or
/// value-equivalence permission; individual expression rewrites remain separate.
pub(crate) struct FormattingSourceLayout {
    realm: tcl_compiler::realm::CommandBindingRealm,
    analysis: Option<tcl_compiler::analyser::AnalysisResult>,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    original: Option<super::analysis_layout::AnalysisFormattingLayout>,
    decoded_phase: Option<tcl_compiler::lambda_literal::OriginalDecodedLambdaFormattingBody>,
    conditional_phase: Option<
        std::sync::Arc<tcl_compiler::command_binding::OriginalConditionalLambdaFormattingBody>,
    >,
}

impl FormattingSourceLayout {
    pub(crate) fn new(source: &str, config: &FormatterConfig, registry: &CommandRegistry) -> Self {
        let generation = tcl_registry::model::ingress::context_for_profile(config.profile);
        let context = std::sync::Arc::new(
            generation.with_command_store(registry.snapshot().shared_registry()),
        );
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            config.profile,
            config.profile,
            context,
            config.lexer_config(),
        );
        Self::with_input(source, &input)
    }

    pub(crate) fn with_input(
        source: &str,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    ) -> Self {
        let profile = input.analyser_profile();
        let mut analyser =
            tcl_compiler::analyser::Analyser::new().with_resolved_input(input.clone());
        let analysis = analyser.analyse(source, profile.name);
        Self {
            realm: analysis
                .retained_command_realm()
                .cloned()
                .unwrap_or_default(),
            analysis: Some(analysis),
            input: input.clone(),
            original: None,
            decoded_phase: None,
            conditional_phase: None,
        }
    }

    pub(crate) fn with_analysis(
        source: &str,
        analysis: &tcl_compiler::analyser::AnalysisResult,
    ) -> Option<Self> {
        let original = super::analysis_layout::AnalysisFormattingLayout::capture(source, analysis)?;
        Some(Self {
            realm: analysis.retained_command_realm()?.clone(),
            analysis: Some(analysis.clone()),
            input: analysis.resolved_input.as_ref()?.clone(),
            original: Some(original),
            decoded_phase: None,
            conditional_phase: None,
        })
    }

    fn presentation_span(&self, span: tcl_lexer::Span) -> Option<tcl_lexer::Span> {
        self.original
            .as_ref()
            .map_or(Some(span), |layout| layout.presentation_span(span))
    }

    fn original_offset(&self, offset: u32) -> Option<u32> {
        self.original
            .as_ref()
            .map_or(Some(offset), |layout| layout.original_offset(offset))
    }
}

// Body / expr / param-list argument identification

/// Mark body / keyword / param-list arguments in place, entirely from
/// registry data.
///
/// Every classification here comes from the command's spec, so no command
/// name appears:
///
/// * **Bodies** are the [`ArgRole::Body`] positions the spec (or its dynamic
///   resolver) reports — for `if` and `try` that is the C-Tcl-shaped clause
///   walk, which knows where a body may legally sit and is the same walk that
///   drives the E004 shape diagnostic.
/// * **Block vs inline** comes from [`tcl_registry::ArgPresentation`]: a body
///   argument is expanded onto its own lines unless the spec declares it
///   `InlineScript`, which is how `for`'s `start` and
///   `next` scripts stay on the header line.
/// * **Structural keywords** are the [`ArgRole::Keyword`] positions the same
///   resolvers report. That is *structural*, not textual: a data word that
///   merely spells `else` is not a keyword unless the grammar puts a keyword
///   there, and `if {1} {a} else then` correctly treats the trailing `then`
///   as the else-branch body, not a keyword.
/// * **Parameter lists** and **lambda literals** are their own roles.
///
/// Because the lookup goes through the registry, the explicitly-global
/// spellings (`::if`, `::for`, `::try`) resolve to the same grammar as their
/// bare forms — a false negative the old literal name comparisons had.
fn identify_body_args(
    cmd: &mut ParsedCommand,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
    source_offset: u32,
    config: &FormatterConfig,
    source: &str,
) {
    if identities.analysis.is_some()
        || identities.decoded_phase.is_some()
        || identities.conditional_phase.is_some()
    {
        reset_selected_command_roles(cmd);
        let Some(image) = selected_formatting_image(identities, config, registry) else {
            return;
        };
        let Ok(whole) = std::str::from_utf8(image.bytes()) else {
            return;
        };
        let Some(head) = source_offset.checked_add(cmd.head_start) else {
            return;
        };
        let aligned = if let Some(original) = &identities.original {
            let Some((segment, words)) = original.command(
                source,
                source_offset,
                head,
                cmd.args.len(),
                config.lexer_config(),
            ) else {
                return;
            };
            Some((segment.clone(), words))
        } else {
            let Ok(start) = usize::try_from(source_offset) else {
                return;
            };
            if whole.get(start..start.saturating_add(source.len())) != Some(source) {
                return;
            }
            None
        };
        let segments = if aligned.is_none() {
            tcl_compiler::segmenter::segment_commands_with_offset_and_config(
                source,
                source_offset,
                config.lexer_config(),
            )
        } else {
            Vec::new()
        };
        let Some(segment) = aligned.as_ref().map(|(segment, _)| segment).or_else(|| {
            segments.iter().find(|segment| {
                segment
                    .argv
                    .first()
                    .is_some_and(|token| token.span.start() == head)
            })
        }) else {
            return;
        };
        if let (Some(original), Some((_, words))) = (&identities.original, &aligned)
            && apply_selected_member_bodies(cmd, original, head, words)
        {
            return;
        }
        let selected = if let Some(phase) = &identities.conditional_phase {
            crate::original_invocation::conditional_lambda_registry_words(
                phase,
                segment,
                &identities.input,
            )
        } else if let Some(phase) = &identities.decoded_phase {
            crate::original_invocation::decoded_lambda_registry_words(
                phase,
                segment,
                &identities.input,
            )
        } else {
            identities.analysis.as_ref().and_then(|analysis| {
                crate::original_invocation::source_registry_words(whole, analysis, segment)
            })
        };
        let Some(selected) = selected else {
            // The shared original syntax inventory retains declaration-member
            // bodies independently of a global command schema. It donates no
            // formal value, expression rewrite or executed member identity.
            if let (Some(original), Some((_, words))) = (&identities.original, &aligned) {
                apply_original_fallback_bodies(cmd, original, words);
            }
            return;
        };
        let Some(_selected_role_owners) = apply_selected_command_roles(
            cmd,
            &selected,
            identities,
            config,
            registry,
            (whole, segment),
        ) else {
            return;
        };
        cmd.source_words = Some(selected);
        return;
    }
    // Missing whole-source advice is terminal; no nominal head reconstruction.
    cmd.source_roles = Some(Vec::new());
    cmd.logical_roles = None;
    cmd.logical_context = None;
    cmd.source_traits = Some(Traits::empty());
    cmd.resolved_name.clear();
}

/// Indices into `cmd.args` of braced expression arguments.
fn identify_expr_args(cmd: &ParsedCommand, registry: &CommandRegistry) -> Vec<usize> {
    if let Some(roles) = cmd.source_roles.as_ref().or(cmd.logical_roles.as_ref()) {
        return roles
            .iter()
            .filter_map(|&(written, role)| (role == ArgRole::Expr).then_some(written))
            .collect();
    }
    let arg_texts: Vec<&str> = cmd.args.iter().skip(1).map(|a| a.text.as_str()).collect();
    registry
        .arg_indices_for_role(&cmd.resolved_name, &arg_texts, ArgRole::Expr)
        .into_iter()
        .map(|idx| idx + 1)
        .collect()
}

// Comment formatting

/// Format a comment per config.
fn format_comment(comment_text: &str, config: &FormatterConfig) -> String {
    if comment_text.is_empty() || comment_text == "#" {
        return "#".to_owned();
    }
    let after_hashes = comment_text.trim_start_matches('#');
    let num_hashes = comment_text.len() - after_hashes.len();
    if after_hashes.is_empty() {
        return "#".repeat(num_hashes);
    }
    let is_commented_code = !after_hashes.chars().next().is_some_and(char::is_whitespace);
    let hashes = "#".repeat(num_hashes);
    if is_commented_code {
        // Commented-out code is not string data; collapse continuations fully.
        format!(
            "{hashes}{}",
            normalise_backslash_newline(after_hashes, false)
        )
    } else if config.space_after_comment_hash {
        format!("{hashes}{}", after_hashes.trim_end())
    } else {
        format!("{hashes}{}", after_hashes.trim())
    }
}

// Blank-line computation

/// How many blank lines to insert before `commands[index]`.
fn compute_blank_lines(
    commands: &[ParsedCommand],
    index: usize,
    config: &FormatterConfig,
) -> usize {
    if index == 0 {
        return 0;
    }
    let current = &commands[index];
    let prev = &commands[index - 1];
    // naming.editor.original-source-formatting
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
    let defines_procedure = |command: &ParsedCommand| {
        command
            .source_traits
            .is_some_and(|traits| traits.contains(Traits::DEFINES_PROCEDURE))
    };
    if defines_procedure(prev) && defines_procedure(current) {
        return config.blank_lines_between_procs;
    }
    if defines_procedure(prev) || defines_procedure(current) {
        return config.blank_lines_between_blocks;
    }
    current
        .preceding_blank_lines
        .min(config.max_consecutive_blank_lines)
}

// Case-list body formatting

/// Format a registry-declared case list, recursing only into elements the
/// shared descriptor parser proved are action bodies.
fn format_case_list_body(
    body_text: &str,
    body_source_offset: u32,
    case_list: &CaseListSpec,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
    indent_level: usize,
) -> String {
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: case_list.clause_flags,
        clause_value_flags: case_list.clause_value_flags,
    };
    let clauses = tcl_syntax::case_list::split_case_list_with_syntax(
        body_text,
        &shape,
        config.lexer_config().list_parse,
    );
    let mut elements = clauses
        .iter()
        .flat_map(|clause| {
            clause
                .flags
                .iter()
                .copied()
                .chain(clause.pattern)
                .chain(clause.body)
        })
        .collect::<Vec<_>>();
    elements.sort_unstable_by_key(|element| element.start);
    elements.dedup_by_key(|element| element.start);
    if elements.is_empty() {
        return body_text.to_owned();
    }
    let action_starts = clauses
        .iter()
        .filter(|clause| clause.valid)
        .filter_map(|clause| clause.body.map(|body| body.start))
        .collect::<std::collections::BTreeSet<_>>();

    let indent = config.make_indent(indent_level);
    let inner_level = indent_level + 1;
    let mut lines: Vec<String> = Vec::new();
    let mut prefix = Vec::new();
    for element in elements {
        // `case_list::Element` includes braces in `start`, but (like Tcl's
        // list parser) reports the interior range for a quoted element.  A
        // formatter must reconstruct the original list *word*, not merely
        // its value: dropping quotes can split one pattern into several
        // elements and change the clause pairing.
        let quoted = !element.braced
            && element.start > 0
            && body_text.as_bytes().get(element.start - 1) == Some(&b'"')
            && body_text.as_bytes().get(element.end) == Some(&b'"');
        let raw_start = element.start - usize::from(quoted);
        let raw_end = element.end + usize::from(element.braced || quoted);
        let Some(raw) = body_text.get(raw_start..raw_end) else {
            return body_text.to_owned();
        };
        if action_starts.contains(&element.start) {
            if element.braced {
                let content_range = element.content_range();
                let Some(content) = body_text.get(content_range.clone()) else {
                    return body_text.to_owned();
                };
                if content == case_list.fallthrough_body.unwrap_or("\0") {
                    prefix.push(raw.to_owned());
                    lines.push(format!("{indent}{}", prefix.join(" ")));
                } else {
                    let action_source_offset = body_source_offset
                        .saturating_add(u32::try_from(content_range.start).unwrap_or(u32::MAX));
                    let formatted = format_body(
                        content,
                        config,
                        registry,
                        identities,
                        FormattingRegion::body(config, action_source_offset, inner_level),
                    );
                    let head = prefix.join(" ");
                    if formatted.trim().is_empty() {
                        lines.push(format!("{indent}{head} {{}}"));
                    } else {
                        lines.push(format!("{indent}{head} {{"));
                        lines.push(formatted);
                        lines.push(format!("{indent}}}"));
                    }
                }
            } else {
                prefix.push(raw.to_owned());
                lines.push(format!("{indent}{}", prefix.join(" ")));
            }
            prefix.clear();
        } else {
            prefix.push(raw.to_owned());
        }
    }
    if !prefix.is_empty() {
        lines.push(format!("{indent}{}", prefix.join(" ")));
    }
    lines.join("\n")
}

// Long-line expression wrapping

/// Operator geometry from the shared expression lexer under the complete
/// selected source grammar. Literal terms and comments are never scanned again.
fn find_expr_break_points(text: &str, config: &FormatterConfig) -> Option<Vec<usize>> {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    use tcl_lexer::ExprTokenType as E;
    let (tokens, unknown) = tcl_lexer::tokenise_expr_checked_with_expression_grammar(
        text,
        &config.lexer_config().grammar_over(config.profile.grammar),
        config.profile.expr_grammar_base,
        config.profile.f5_core_expr_grammar(),
    );
    if unknown || tokens.iter().any(|token| token.kind == E::Comment) {
        return None;
    }
    let mut depth = 0_u32;
    let mut breaks = Vec::new();
    for token in tokens {
        match token.kind {
            E::ParenOpen => depth = depth.checked_add(1)?,
            E::ParenClose => depth = depth.checked_sub(1)?,
            E::Operator if depth == 0 && matches!(token.text.as_str(), "&&" | "||") => {
                breaks.push(usize::try_from(token.start).ok()?);
            }
            _ => {}
        }
    }
    (depth == 0).then_some(breaks)
}

/// Try to wrap a braced expression at `&&` / `||`; returns the
/// wrapped inner text (no braces) or `None`.
fn wrap_braced_expr(text: &str, config: &FormatterConfig, indent_level: usize) -> Option<String> {
    let breaks = find_expr_break_points(text, config)?;
    if breaks.is_empty() {
        return None;
    }
    let mut chunks: Vec<&str> = Vec::new();
    let mut last = 0;
    for &pos in &breaks {
        chunks.push(text[last..pos].trim());
        last = pos;
    }
    chunks.push(text[last..].trim());

    let expr_indent = config.make_indent(indent_level + 1);
    let cmd_indent = config.make_indent(indent_level);
    let inner = chunks
        .iter()
        .map(|c| format!("{expr_indent}{c}"))
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!("\n{inner}\n{cmd_indent}"))
}

/// Estimate the content length after `args[start]` on the first
/// line.
fn estimate_trailing_len(args: &[CommandArg], start: usize) -> usize {
    let mut length = 0;
    for arg in &args[start..] {
        match arg.kind {
            ArgKind::Body => {
                length += 2; // " {"
                break;
            }
            ArgKind::Keyword => length += 1 + arg.text.len(),
            _ => length += 1 + arg.text.len() + if arg.is_braced { 2 } else { 0 },
        }
    }
    length
}

/// Current column from accumulated parts.
fn current_col(parts: &[String], indent_len: usize) -> usize {
    let text: String = parts.concat();
    match text.rfind('\n') {
        None => indent_len + text.len(),
        Some(nl) => text.len() - nl - 1,
    }
}

// Backslash-continuation line splitting

/// Greedy line splitting at the given space positions. Callers pass only
/// positions that are safe word separators (outside double-quoted strings); a
/// space *inside* a `"…"` is string data, not a separator, and breaking there
/// would alter the string's value, so such positions are never offered here.
fn greedy_split(
    line: &str,
    spaces: &[usize],
    max_len: usize,
    cont_indent: &str,
) -> Option<Vec<String>> {
    if spaces.is_empty() {
        return None;
    }
    let indent_len = line.len() - line.trim_start().len();
    let mut segments: Vec<String> = Vec::new();
    let mut seg_start = 0usize;
    let mut last_good: Option<usize> = None;

    let commit_break = |segments: &mut Vec<String>,
                        seg_start: &mut usize,
                        last_good: &mut Option<usize>,
                        break_pos: usize| {
        let mut text = line[*seg_start..break_pos].to_owned();
        if *seg_start > 0 && *seg_start > indent_len {
            text = format!("{cont_indent}{text}");
        }
        segments.push(format!("{text} \\"));
        *seg_start = break_pos + 1;
        *last_good = None;
    };

    for &sp in spaces {
        let seg_len = if seg_start == 0 {
            sp + 2
        } else {
            cont_indent.len() + (sp - seg_start) + 2
        };
        if seg_len <= max_len {
            last_good = Some(sp);
        } else if let Some(lg) = last_good {
            commit_break(&mut segments, &mut seg_start, &mut last_good, lg);
            let new_len = cont_indent.len() + (sp - seg_start) + 2;
            last_good = if new_len <= max_len { Some(sp) } else { None };
        } else {
            commit_break(&mut segments, &mut seg_start, &mut last_good, sp);
        }
    }

    if segments.is_empty() {
        {
            let lg = last_good?;
            commit_break(&mut segments, &mut seg_start, &mut last_good, lg);
        }
    }

    let mut remainder = line[seg_start..].to_owned();
    if seg_start > 0 && seg_start > indent_len {
        remainder = format!("{cont_indent}{remainder}");
    }
    segments.push(remainder);
    Some(segments)
}

/// Split a long line using `\` continuation, preferring shallow
/// breaks.
fn split_long_line(line: &str, config: &FormatterConfig, cont_indent: &str) -> Option<String> {
    if line.len() <= config.max_line_length {
        return None;
    }
    let indent_len = line.len() - line.trim_start().len();
    let all_spaces =
        super::source_layout::separator_spaces(line, config.lexer_config(), MAX_FORMAT_DEPTH)?
            .into_iter()
            .filter(|&(offset, _)| offset >= indent_len)
            .collect::<Vec<_>>();
    let max_len = config.max_line_length;
    let mut segments: Option<Vec<String>> = None;

    if !all_spaces.is_empty() {
        let max_depth = all_spaces.iter().map(|&(_, d)| d).max().unwrap_or(0);
        for target_depth in 0..=max_depth {
            let spaces: Vec<usize> = all_spaces
                .iter()
                .filter(|&&(_, d)| d <= target_depth)
                .map(|&(p, _)| p)
                .collect();
            let seg = greedy_split(line, &spaces, max_len, cont_indent);
            if seg.as_ref().is_some_and(|s| s.len() > 1) {
                segments = seg;
                break;
            }
        }
    }

    // No safe split found among word separators outside quotes. We deliberately
    // do NOT fall back to breaking at a space *inside* a double-quoted string:
    // that space is string data, and inserting `\<newline>` + continuation
    // indent there changes the string's value (Tcl collapses the
    // `\<newline><leading-ws>` to a single space, but the original space is kept,
    // so the literal silently gains a space). A line that can only be split
    // inside a string literal is left over-length instead — layout is best
    // effort, but the program's data must never change.
    if segments.as_ref().is_none_or(|s| s.len() < 2) {
        return None;
    }

    // Every break belongs to the original complete line. A remainder is not
    // re-parsed as a new script with a different bracket or quote context.
    Some(segments?.join("\n"))
}

/// Try to split a long commented-out command using `\`
/// continuation.
fn split_commented_code(
    comment_text: &str,
    config: &FormatterConfig,
    indent: &str,
    indent_level: usize,
) -> Option<String> {
    if comment_text.len() < 2 {
        return None;
    }
    let after_hash = &comment_text[1..];
    if after_hash.is_empty() || after_hash.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    let full_line = format!("{indent}{comment_text}");
    if full_line.len() <= config.max_line_length {
        return None;
    }
    let cmd_line = format!("{indent}{after_hash}");
    let cont_indent = config.make_indent(indent_level + 1);
    let split = split_long_line(&cmd_line, config, &cont_indent)?;
    let mut split_lines: Vec<String> = split.split('\n').map(str::to_owned).collect();
    let first = split_lines[0]
        .strip_prefix(indent)
        .unwrap_or(&split_lines[0]);
    split_lines[0] = format!("{indent}#{first}");
    Some(split_lines.join("\n"))
}

// Inline body detection

/// Count complete commands through the shared lexer under the full body grammar.
/// Unavailable source remains outside the inline presentation threshold.
fn count_body_commands(body_text: &str, config: tcl_lexer::LexerConfig) -> Option<usize> {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    let image = tcl_lexer::SourceImage::document(body_text);
    let end = u32::try_from(body_text.len()).ok()?;
    super::source_layout::words_in(&image, tcl_lexer::Span::new(0, end), config)
        .map(|plan| plan.commands.len())
}

/// Whether a body is short enough to keep on one line.
fn body_can_be_inline(
    body_text: &str,
    config: &FormatterConfig,
    current_line_len: usize,
    never_inline: bool,
) -> bool {
    if config.expand_single_line_bodies
        && count_body_commands(body_text, config.lexer_config())
            .is_none_or(|count| count >= config.min_body_commands_for_expansion)
    {
        // Only force expansion once the body carries at least
        // `min_body_commands_for_expansion` commands; a trivial one-command
        // body stays inline.
        return false;
    }
    let stripped = body_text.trim();
    if stripped.is_empty() {
        return true;
    }
    if never_inline {
        return false;
    }
    if stripped.contains('\n') || stripped.contains('{') || stripped.contains('}') {
        return false;
    }
    let inline_len = current_line_len + "{ ".len() + stripped.len() + " }".len();
    inline_len <= config.goal_line_length
}

// Command reconstruction

/// Push a leading separator space before the next part, unless
/// we're continuing a `}{` brace chain with
/// `space_between_braces == false`.
fn maybe_space(parts: &mut Vec<String>, in_brace_chain: bool, space_between_braces: bool) {
    let suppress = in_brace_chain && !space_between_braces;
    if !suppress && !parts.is_empty() {
        parts.push(" ".to_owned());
    }
}

/// Render a formatted body argument (inline `{ … }` or expanded
/// K&R block) into `parts`.  The caller has already emitted any
/// required leading separator space.
fn append_body_no_space(
    parts: &mut Vec<String>,
    body: &str,
    config: &FormatterConfig,
    indent: &str,
    current_line_len: usize,
    never_inline: bool,
) {
    let inline = body_can_be_inline(body, config, current_line_len, never_inline);
    let stripped = body.trim();
    if stripped.is_empty() {
        parts.push("{}".to_owned());
    } else if inline {
        parts.push(format!("{{ {stripped} }}"));
    } else {
        parts.push("{\n".to_owned());
        parts.push(body.to_owned());
        parts.push(format!("\n{indent}}}"));
    }
}

/// Reassemble an `ArgKind::LambdaLiteral` argument: its parameter-list
/// element (normalised, not code) and its body element (`arg.formatted_body`
/// — already recursively formatted by the caller from just the *decoded*
/// body span, never the whole lambda literal, so the parameter word is
/// never misread as a command name), plus an optional namespace element,
/// wrapped back in the outer `{}`. Falls back to the original literal
/// verbatim if the source can no longer be split (should not happen once
/// `identify_body_args` and this function agree, but a stale token must not
/// panic).
///
/// The parameter-list and namespace elements are decoded (backslash escapes
/// collapsed for a bare/quoted element — [`tcl_compiler::lambda_literal::split_lambda_literal_decoded`])
/// before use, not pasted through as raw source spelling: a non-literal
/// element's escapes would otherwise survive reformatting and change what it
/// means. The namespace is re-quoted with
/// [`tcl_syntax::list::list_element`] on reassembly rather than always left
/// bare, so an element that needs quoting (e.g. an escaped space) still
/// round-trips safely. Native parameters use their retained nested list
/// children, then the complete spelling proposal is checked against the same
/// original native values. Missing or unrepresentable children preserve the
/// whole original word; explicit Logical compatibility uses the String split.
fn decoded_lambda(arg: &CommandArg, logical: bool) -> Option<DecodedLambdaLiteral<'static>> {
    match (&arg.original_word, &arg.original_input) {
        (Some(word), Some(input)) => split_original_lambda_literal_decoded(word, input),
        (Some(word), None) if logical => split_original_lambda_literal_lexical_decoded(word),
        _ => None,
    }
}

fn render_lambda_literal_arg(
    sm: &SourceMap,
    arg: &CommandArg,
    config: &FormatterConfig,
    indent: &str,
    current_line_len: usize,
    never_inline: bool,
    logical: bool,
) -> String {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    let fallback = || reconstruct_arg(sm, arg, false);
    let Some(elems) = decoded_lambda(arg, logical) else {
        return fallback();
    };
    let Some(formatted_body) = arg.formatted_body.as_deref() else {
        return fallback();
    };
    let mut body_parts: Vec<String> = Vec::new();
    append_body_no_space(
        &mut body_parts,
        formatted_body,
        config,
        indent,
        current_line_len,
        never_inline,
    );
    let params_rendered = match (&arg.original_word, &arg.original_input) {
        (Some(_), Some(input)) => {
            let Some(params) = input
                .original_list_element(0)
                .and_then(|params| native_param_list_source(&params))
            else {
                return fallback();
            };
            params
        }
        (Some(word), None) if logical && word.config() == config.lexer_config() => {
            normalise_param_list(&elems.params, config.word_rules())
        }
        _ => return fallback(),
    };
    let body_rendered = body_parts.concat();
    let rendered = match elems.namespace.as_deref() {
        Some(ns) => {
            let ns_rendered = tcl_syntax::list::list_element(ns);
            format!("{{{params_rendered} {body_rendered} {ns_rendered}}}")
        }
        None => format!("{{{params_rendered} {body_rendered}}}"),
    };
    if let (Some(word), Some(input)) = (&arg.original_word, &arg.original_input)
        && original_lambda_parameters_match(&rendered, word, input, config).is_none()
    {
        return fallback();
    }
    rendered
}

/// Everything `append_word_arg` needs about the argument it is emitting,
/// beyond the mutable output buffer.
struct WordArgContext<'a> {
    sm: &'a SourceMap<'a>,
    args: &'a [CommandArg],
    /// Index of the argument being emitted.
    index: usize,
    /// Argument indices the registry marked `ArgRole::Expr`.
    expr_args: &'a [usize],
    config: &'a FormatterConfig,
    indent_level: usize,
    /// Canonical replacement text for this word, if any.
    keyword_rewrite: Option<&'a str>,
}

/// Append a plain word argument (with optional expression wrapping).
/// Returns `false` when nothing was emitted (an all-continuation artifact),
/// so the caller leaves the brace chain untouched.
fn append_word_arg(ctx: &WordArgContext<'_>, parts: &mut Vec<String>) -> bool {
    let WordArgContext {
        sm,
        args,
        index: i,
        expr_args,
        config,
        indent_level,
        keyword_rewrite,
    } = *ctx;
    let arg = &args[i];
    // A keyword rewrite
    // replaces the whole word. It is only ever computed for a plain, static,
    // unbraced, unquoted keyword word, so there are no delimiters to preserve
    // and no expression/body handling to run.
    if let Some(text) = keyword_rewrite {
        if !parts.is_empty() {
            parts.push(" ".to_owned());
        }
        parts.push(text.to_owned());
        return true;
    }
    let mut raw = reconstruct_arg(sm, arg, config.enforce_braced_variables);
    if raw.contains('\n')
        && !arg.is_braced
        && !arg.tokens.iter().any(|token| {
            token.kind == TokenType::Cmd
                || (matches!(token.kind, TokenType::Var | TokenType::ExprSugar)
                    && sm.token_text(*token).contains('\n'))
        })
    {
        // Keep whitespace before the backslash only for a quoted string, where
        // it is literal data; a bare/continued word collapses.
        let collapsed = normalise_backslash_newline(&raw, arg.is_quoted)
            .trim()
            .to_owned();
        if collapsed.is_empty() {
            return false;
        }
        raw = collapsed;
    }
    // `enforceBracedExpr`, bounded form: a single unbraced expression argument
    // (an `if` / `while` / `for` condition, `control::assert`'s first arg) is
    // wrapped in braces (`if $x …` → `if {$x} …`). A `"…"`-quoted operand, or
    // one carrying a `{*}` expansion (whose expansion braces would demote to
    // literal text), is left alone.
    let has_expansion = arg.tokens.iter().any(|t| t.kind == TokenType::Expand);
    if config.enforce_braced_expr
        && !arg.is_braced
        && !arg.is_quoted
        && !has_expansion
        && expr_args.contains(&i)
    {
        maybe_space(parts, false, config.space_between_braces);
        parts.push(format!("{{{raw}}}"));
        return true;
    }
    if arg.is_braced && expr_args.contains(&i) {
        let indent_len = config.make_indent(indent_level).len();
        let mut col = current_col(parts, indent_len);
        if !parts.is_empty() {
            col += 1;
        }
        let trailing = estimate_trailing_len(args, i + 1);
        if col + raw.len() + trailing > config.max_line_length
            && let Some(wrapped) = wrap_braced_expr(&arg.text, config, indent_level)
        {
            maybe_space(parts, false, config.space_between_braces);
            parts.push(format!("{{{wrapped}}}"));
            return true;
        }
    }
    if !parts.is_empty() {
        parts.push(" ".to_owned());
    }
    parts.push(raw);
    true
}

/// The keyword rewrites
/// that apply to this command, keyed by argument index.
///
/// Only plain `Word` arguments that are neither braced nor quoted are
/// eligible: a braced or quoted word's bytes are data the author chose to
/// delimit, and a `Body`/`ParamList`/`Keyword`/`LambdaLiteral` argument is
/// not a keyword-table position at all. Everything else — the dialect gate,
/// the strictness flag, ambiguity, dynamic words — is decided by
/// [`super::keywords::rewrites_for_command`].
fn keyword_rewrites_for(
    cmd: &ParsedCommand,
    _registry: &CommandRegistry,
    config: &FormatterConfig,
) -> std::collections::HashMap<usize, String> {
    // A source schema identifies possible positions. Operand-byte rewriting
    // requires a separate equivalence receipt; this view supplies none.
    if cmd.source_roles.is_some() || (cmd.logical_roles.is_some() && cmd.logical_context.is_none())
    {
        return std::collections::HashMap::new();
    }
    if !config.expand_abbreviations && config.boolean_form == super::config::BooleanForm::Preserve {
        return std::collections::HashMap::new();
    }
    if cmd.resolved_name.is_empty() {
        return std::collections::HashMap::new();
    }
    let (Some(source), Some(context)) = (&cmd.source_words, &cmd.logical_context) else {
        return std::collections::HashMap::new();
    };
    if source
        .head_source()
        .and_then(|head| head.word())
        .is_none_or(|word| word.config() != config.lexer_config())
    {
        return std::collections::HashMap::new();
    }
    source
        .with_source_schema(context, |selected| {
            let surviving = super::keywords::rewrites_for_selected_invocation(selected, config)
                .into_iter()
                .filter_map(|rewrite| {
                    let tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written) =
                        source.origins.get(rewrite.index.checked_add(1)?)?
                    else {
                        return None;
                    };
                    if *written == 0 {
                        return None;
                    }
                    let argument = cmd.args.get(*written)?;
                    if argument.kind != ArgKind::Word
                        || argument.is_braced
                        || argument.is_quoted
                        || argument.tokens.iter().any(|token| {
                            matches!(
                                token.kind,
                                TokenType::Var | TokenType::Cmd | TokenType::Expand
                            )
                        })
                        || selected.words.arguments().literal_at(rewrite.index)
                            != Some(argument.text.as_str())
                        || source.operands.get(rewrite.index)?.as_ref()?.word.as_ref()
                            != argument.original_word.as_ref()
                    {
                        return None;
                    }
                    Some((rewrite.index, *written, rewrite.text))
                })
                .collect::<Vec<_>>();
            let replacements = surviving
                .iter()
                .map(|(effective, _, text)| (*effective, text.as_str()))
                .collect::<Vec<_>>();
            if !super::keywords::emitted_replacements_keep_selection(
                selected,
                config,
                &replacements,
            ) {
                return std::collections::HashMap::new();
            }
            surviving
                .into_iter()
                .map(|(_, written, text)| (written, text))
                .collect()
        })
        .unwrap_or_default()
}

/// Reconstruct a single command as formatted text.
fn case_list_body_index(
    cmd: &ParsedCommand,
    config: &FormatterConfig,
    registry: &CommandRegistry,
) -> Option<(usize, CaseListSpec)> {
    if cmd.source_roles.is_some() || cmd.logical_roles.is_some() {
        return cmd.source_case;
    }
    let case_list = registry
        .get(&cmd.resolved_name)
        .and_then(|spec| spec.case_list)?;
    let args: Vec<&str> = cmd
        .args
        .iter()
        .skip(1)
        .map(|arg| arg.text.as_str())
        .collect();
    let dialect = registry.profile().map_or_else(
        || config.dialect_query(),
        |profile| Some(crate::document_context_for_profile(profile).authoring_query()),
    );
    if let Some(index) = registry
        .case_invocation(&cmd.resolved_name, &args, dialect)
        .and_then(|(_, invocation)| invocation.clause_list_index)
    {
        return Some((index + 1, *case_list));
    }

    // Formatting may present an incomplete flag-free (switch-shaped) case
    // list even when semantic consumers correctly abstain. A flagged
    // descriptor cannot recover safely: after an unknown, ambiguous, or
    // truncated flag, later words have no proven pattern/body roles and the
    // shared splitter deliberately resynchronises only for diagnostics. Keep
    // that whole value opaque instead of formatting an apparent later body.
    if !case_list.clause_flags.is_empty() || !case_list.clause_value_flags.is_empty() {
        return None;
    }

    // Prove that the original value is a complete Tcl list with precisely the
    // odd pattern/body prefix shape formatter recovery supports. Empty and
    // otherwise malformed values remain byte-semantically opaque.
    let last = args.len().checked_sub(1)?;
    let elements = config.word_rules().split_list(args[last]).ok()?;
    if elements.is_empty() || elements.len().is_multiple_of(2) {
        return None;
    }

    // Probe the registry-owned *outer* grammar with a known-valid list in the
    // final word. This proves that the original word occupies the clause-list
    // slot without blessing its dangling pattern as executable. Inline
    // pattern/body forms remain inline because replacing their final body does
    // not change their arity.
    let mut recovery_args = args;
    recovery_args[last] = "default {}";
    registry
        .case_invocation(&cmd.resolved_name, &recovery_args, dialect)
        .and_then(|(_, invocation)| {
            (invocation.clause_list_index == Some(last)).then_some((last + 1, *case_list))
        })
}

fn reconstruct_command(
    sm: &SourceMap,
    cmd: &ParsedCommand,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    indent: &str,
    indent_level: usize,
) -> String {
    if case_list_body_index(cmd, config, registry).is_some() {
        return reconstruct_case_list(sm, cmd, config, indent);
    }

    let expr_args = identify_expr_args(cmd, registry);
    let mut effective_config = config.clone();
    if cmd.source_roles.is_some() {
        effective_config.enforce_braced_expr = false;
    }
    let config = &effective_config;
    let keyword_rewrites = keyword_rewrites_for(cmd, registry, config);
    let spec_traits = if cmd.source_roles.is_some() || cmd.logical_roles.is_some() {
        cmd.source_traits
    } else {
        registry.get(&cmd.resolved_name).map(|s| s.traits)
    };
    let never_inline = spec_traits.is_some_and(|t| t.contains(Traits::NEVER_INLINE_BODY));

    let mut parts: Vec<String> = Vec::new();
    let mut in_brace_chain = false;

    // `enforceBracedExpr`, concatenating form: a command whose entire argument
    // tail is one expression (`expr $a + $b`) — brace the joined tail
    // (`expr {$a + $b}`) rather than the single marked argument, which would
    // corrupt it. Bounded-expression commands fall through to
    // the per-argument bracing in `append_word_arg`.
    if config.enforce_braced_expr
        && spec_traits.is_some_and(|t| t.contains(Traits::EXPR_CONCATENATES_ARGS))
        && let Some(braced) = concat_expr_parts(sm, cmd, config)
    {
        return finish_command_line(&braced, config, indent, indent_level);
    }

    for (i, arg) in cmd.args.iter().enumerate() {
        match arg.kind {
            ArgKind::Body if arg.formatted_body.is_some() => {
                let body = arg.formatted_body.as_deref().unwrap();
                let current_line_len = indent.len() + parts.concat().len();
                // The body's leading space depends on the running
                // brace chain; apply it here, then delegate the
                // body rendering.
                maybe_space(&mut parts, in_brace_chain, config.space_between_braces);
                append_body_no_space(
                    &mut parts,
                    body,
                    config,
                    indent,
                    current_line_len,
                    never_inline || arg.member_body_presentation == Some(
                        tcl_registry::definer::SourceDefinitionMemberBodyPresentation::CallableBlock,
                    ),
                );
                in_brace_chain = true;
            }
            ArgKind::LambdaLiteral if arg.formatted_body.is_some() => {
                let current_line_len = indent.len() + parts.concat().len();
                maybe_space(&mut parts, in_brace_chain, config.space_between_braces);
                parts.push(render_lambda_literal_arg(
                    sm,
                    arg,
                    config,
                    indent,
                    current_line_len,
                    never_inline,
                    cmd.logical_context.is_some(),
                ));
                in_brace_chain = true;
            }
            ArgKind::ParamList => {
                maybe_space(&mut parts, false, config.space_between_braces);
                parts.push(render_param_list_arg(
                    sm,
                    arg,
                    config,
                    cmd.source_roles.is_none(),
                ));
                in_brace_chain = false;
            }
            ArgKind::Keyword => {
                maybe_space(&mut parts, false, config.space_between_braces);
                parts.push(arg.text.clone());
                in_brace_chain = false;
            }
            _ => {
                if let Some((written, replacement)) = &cmd.expression_bracing
                    && *written == i
                {
                    maybe_space(&mut parts, false, config.space_between_braces);
                    parts.push(replacement.clone());
                    in_brace_chain = false;
                    continue;
                }
                if append_word_arg(
                    &WordArgContext {
                        sm,
                        args: &cmd.args,
                        index: i,
                        expr_args: &expr_args,
                        config,
                        indent_level,
                        keyword_rewrite: keyword_rewrites.get(&i).map(String::as_str),
                    },
                    &mut parts,
                ) {
                    in_brace_chain = false;
                }
            }
        }
    }

    finish_command_line(&parts, config, indent, indent_level)
}

/// Concatenate `parts` under `indent` and apply the long-line split. Shared by
/// the normal command path and the `enforceBracedExpr` concatenating path.
fn finish_command_line(
    parts: &[String],
    config: &FormatterConfig,
    indent: &str,
    indent_level: usize,
) -> String {
    let line = format!("{indent}{}", parts.concat());

    // Split the first line if it exceeds max_line_length.
    let first_nl = line.find('\n');
    let first_line = match first_nl {
        Some(nl) => &line[..nl],
        None => &line,
    };
    if first_line.len() > config.max_line_length {
        let cont_indent = config.make_indent(indent_level + 1);
        if let Some(split) = split_long_line(first_line, config, &cont_indent) {
            return match first_nl {
                Some(nl) => format!("{split}{}", &line[nl..]),
                None => split,
            };
        }
    }
    line
}

/// Build the `parts` for an `enforceBracedExpr` command whose entire argument
/// tail is one expression (`EXPR_CONCATENATES_ARGS`). Returns `None` when there
/// is nothing to brace — no arguments, or the tail is already a single braced
/// `{ … }` word (which the normal path renders, keeping its long-line wrap).
fn concat_expr_parts(
    sm: &SourceMap,
    cmd: &ParsedCommand,
    config: &FormatterConfig,
) -> Option<Vec<String>> {
    let args = &cmd.args;
    if args.len() < 2 {
        return None; // bare `expr` with no operands
    }
    if args.len() == 2 && args[1].is_braced {
        return None; // already `expr {…}`
    }
    // Never brace a tail containing an expansion: `expr {*}$pieces` expands the
    // list before evaluating, and `expr {{*}$pieces}` would demote `{*}` to
    // literal text, breaking the expression. Leave such a command untouched.
    if args[1..]
        .iter()
        .any(|a| a.tokens.iter().any(|t| t.kind == TokenType::Expand))
    {
        return None;
    }
    let joined = args[1..]
        .iter()
        .map(|a| reconstruct_arg(sm, a, config.enforce_braced_variables))
        .collect::<Vec<_>>()
        .join(" ");
    Some(vec![
        args[0].text.clone(),
        " ".to_owned(),
        format!("{{{joined}}}"),
    ])
}

/// Reconstruct a `switch` command, handling the braced body form.
fn reconstruct_case_list(
    sm: &SourceMap,
    cmd: &ParsedCommand,
    config: &FormatterConfig,
    indent: &str,
) -> String {
    // The case-list body is the last braced arg; recompute its
    // formatting through `format_case_list_body` (parsed below by
    // re-deriving from the registry-marked Body arg or the last
    // braced arg).
    let mut parts: Vec<String> = Vec::new();
    for arg in &cmd.args {
        if arg.kind == ArgKind::Body && arg.is_braced {
            let formatted = arg.formatted_body.clone().unwrap_or_default();
            if !parts.is_empty() {
                parts.push(" ".to_owned());
            }
            if formatted.trim().is_empty() {
                parts.push("{}".to_owned());
            } else {
                parts.push("{\n".to_owned());
                parts.push(formatted);
                parts.push(format!("\n{indent}}}"));
            }
        } else {
            let raw = reconstruct_arg(sm, arg, config.enforce_braced_variables);
            if !parts.is_empty() {
                parts.push(" ".to_owned());
            }
            parts.push(raw);
        }
    }
    format!("{indent}{}", parts.concat())
}

fn token_content_source_offset(source_offset: u32, token: Token) -> u32 {
    source_offset
        .saturating_add(token.span.start())
        .saturating_add(u32::from(token.content_offset))
}

fn lambda_body_source_offset(source: &str, source_offset: u32, token: Token) -> u32 {
    // For a braced lambda body this is its exact source position. A decoded
    // bare/quoted element can contract escapes, but the element's source start
    // still keeps outer binding facts on the correct side of the containing
    // command.
    tcl_compiler::lambda_literal::split_lambda_literal(source, token)
        .and_then(|elements| elements.body)
        .map_or_else(
            || token_content_source_offset(source_offset, token),
            |body| source_offset.saturating_add(body.start()),
        )
}

/// Script regions selected by the same source presentation used for whole-file
/// formatting. Whole original words supply geometry; schema roles supply only
/// readonly applicability, independently from handler execution.
fn presentation_children(
    command: &ParsedCommand,
    words: &[tcl_lexer::NativeWord],
    source_offset: u32,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
) -> Option<Vec<tcl_lexer::Span>> {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    use tcl_lexer::{ExecutablePart, Span, WordKind};
    let mut children = Vec::new();
    for word in words {
        for part in word.executable_parts().all_parts() {
            if let ExecutablePart::Command { body } = part.part {
                children.push(body);
            }
        }
    }
    let case = case_list_body_index(command, config, registry);
    if let Some(source) = &command.source_words {
        let context = identities.input.context_registry();
        for body in source.source_script_bodies(&context) {
            let parent = body.original_container();
            if parent.group().kind != WordKind::Braced
                || parent.group().expand
                || !body.matches_source(parent.image(), config.lexer_config())
                || !body.matches_context(&context)
            {
                continue;
            }
            let written = command.args.iter().position(|argument| {
                argument.tokens.first().is_some_and(|first| {
                    source_offset
                        .checked_add(first.span.start())
                        .and_then(|offset| identities.original_offset(offset))
                        == Some(parent.span().start())
                })
            });
            if let Some(written) = written
                && (command.args[written].kind == ArgKind::Body
                    || case.is_some_and(|(ordinal, _)| ordinal == written))
            {
                children.push(identities.presentation_span(body.content_span())?);
            }
        }
    }
    for (written, argument) in command.args.iter().enumerate().skip(1) {
        let Some(first) = argument.tokens.first() else {
            continue;
        };
        let start = source_offset.checked_add(first.span.start())?;
        let Some(word) = words.iter().find(|word| word.span().start() == start) else {
            continue;
        };
        if word.group().kind != WordKind::Braced || word.group().expand {
            continue;
        }
        if command.source_words.is_some() {
            if argument.kind == ArgKind::LambdaLiteral
                && let Some(original) = argument.original_word.as_ref()
                && let Some(body) =
                    tcl_compiler::lambda_literal::split_original_lambda_literal(original)
                        .and_then(|lambda| lambda.braced_body())
            {
                children.push(identities.presentation_span(body)?);
            }
            continue;
        }
        if let Some((_, descriptor)) = case.filter(|(ordinal, _)| *ordinal == written) {
            let content = word.content_span().ok()?;
            let source = word.image().try_text().ok()?;
            let text = source.get(content.as_range())?;
            let shape = tcl_syntax::case_list::CaseListShape {
                clause_flags: descriptor.clause_flags,
                clause_value_flags: descriptor.clause_value_flags,
            };
            for clause in tcl_syntax::case_list::split_case_list_with_syntax(
                text,
                &shape,
                config.lexer_config().list_parse,
            ) {
                if !clause.valid {
                    continue;
                }
                let Some(body) = clause.body.filter(|body| body.braced) else {
                    continue;
                };
                let range = body.content_range();
                children.push(Span::new(
                    content
                        .start()
                        .checked_add(u32::try_from(range.start).ok()?)?,
                    content
                        .start()
                        .checked_add(u32::try_from(range.end).ok()?)?,
                ));
            }
        } else if argument.kind == ArgKind::Body {
            children.push(word.content_span().ok()?);
        } else if argument.kind == ArgKind::LambdaLiteral
            && let Some(body) = tcl_compiler::lambda_literal::split_original_lambda_literal(word)
                .and_then(|lambda| lambda.braced_body())
        {
            children.push(body);
        }
    }
    Some(children)
}

/// Format a line selection only inside a genuine selected script region and
/// across complete lexical words. Data words and unavailable bodies are kept.
pub(crate) fn formatting_range_region(
    source: &str,
    selection: tcl_lexer::Span,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
) -> Option<FormattingRegion> {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    use tcl_lexer::{SourceImage, Span};
    let image = SourceImage::document(source);
    let end = u32::try_from(source.len()).ok()?;
    if selection.end() > end
        || identities.input.lexer_config() != config.lexer_config()
        || identities
            .input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            != registry.snapshot().semantic_key()
        || identities.original.as_ref().map_or_else(
            || {
                identities.analysis.as_ref().is_some_and(|analysis| {
                    !analysis.matches_original_source_image(&image, config.lexer_config())
                })
            },
            |original| original.text() != source,
        )
    {
        return None;
    }
    let mut region = Span::new(0, end);
    let mut depth = 0;
    loop {
        if MAX_FORMAT_DEPTH.exceeded(depth) {
            return None;
        }
        let plan = super::source_layout::words_in(&image, region, config.lexer_config())?;
        let text = source.get(region.as_range())?;
        let map = SourceMap::new(text);
        let local_config = if region == Span::new(0, end) {
            config.lexer_config()
        } else {
            config.lexer_config().nested()
        };
        let tokens = Lexer::with_config(text, local_config).tokenise_all().ok()?;
        let (mut commands, _) = parse_commands(text, &map, &tokens, local_config)?;
        let mut children = Vec::new();
        for command in &mut commands {
            identify_body_args(command, registry, identities, region.start(), config, text);
            let start = region.start().checked_add(command.head_start)?;
            let Some(words) = plan.commands.iter().find(|row| row.span.start() == start) else {
                continue;
            };
            children.extend(presentation_children(
                command,
                &words.words,
                region.start(),
                config,
                registry,
                identities,
            )?);
        }
        let child = children
            .into_iter()
            .filter(|child| {
                child.start() <= selection.start()
                    && selection.end() <= child.end()
                    && *child != region
            })
            .min_by_key(|child| child.end() - child.start());
        if let Some(child) = child {
            region = child;
            depth = depth.checked_add(1)?;
        } else {
            if !super::source_layout::selection_keeps_whole_words(&plan, selection) {
                return None;
            }
            return Some(FormattingRegion {
                source_offset: selection.start(),
                indent_level: usize::try_from(depth).ok()?,
                local_config: if region == Span::new(0, end) && selection.start() == region.start()
                {
                    config.lexer_config()
                } else {
                    config.lexer_config().nested()
                },
            });
        }
    }
}

// Main entry points

/// Local parsing belongs to the genuine selected region. Original source
/// joins continue to use the complete parent `FormatterConfig` and layout.
#[derive(Clone, Copy)]
pub(crate) struct FormattingRegion {
    source_offset: u32,
    indent_level: usize,
    local_config: tcl_lexer::LexerConfig,
}

impl FormattingRegion {
    fn document(config: &FormatterConfig) -> Self {
        Self {
            source_offset: 0,
            indent_level: 0,
            local_config: config.lexer_config(),
        }
    }

    fn body(config: &FormatterConfig, source_offset: u32, indent_level: usize) -> Self {
        Self {
            source_offset,
            indent_level,
            local_config: config.lexer_config().nested(),
        }
    }
}

/// Format a Tcl script body at the given indent level.  The core
/// recursive function.
/// Format a script body at `indent_level`, applying every engine
/// rule (comments, switch bodies, recursion, long-line wrapping, …).
/// [`format_tcl`] calls this at level 0 for a whole document; range
/// formatting calls it for a line slice at its selected script depth so
/// both paths share identical layout rules.
pub(crate) fn format_body(
    source: &str,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
    region: FormattingRegion,
) -> String {
    let FormattingRegion {
        source_offset,
        indent_level,
        local_config,
    } = region;
    // Native-stack safety net — see `MAX_FORMAT_DEPTH`'s doc comment.
    // Past the cap, leave this (deeply nested) body unformatted rather than
    // recursing further, matching the give-up-gracefully fallback just below
    // for an unparseable body.
    if MAX_FORMAT_DEPTH.exceeded(u32::try_from(indent_level).unwrap_or(u32::MAX)) {
        return source.to_owned();
    }
    let sm = SourceMap::new(source);
    let Ok(tokens) = Lexer::with_source_map(SourceMap::new(source), local_config).tokenise_all()
    else {
        return source.to_owned();
    };
    let Some((mut commands, trailing_comments)) =
        parse_commands(source, &sm, &tokens, local_config)
    else {
        return source.to_owned();
    };

    let indent = config.make_indent(indent_level);
    let inner_level = indent_level + 1;
    let mut lines: Vec<String> = Vec::new();

    for i in 0..commands.len() {
        identify_body_args(
            &mut commands[i],
            registry,
            identities,
            source_offset,
            config,
            source,
        );
        let blank_count = compute_blank_lines(&commands, i, config);
        for _ in 0..blank_count {
            lines.push(String::new());
        }

        // Preceding comments.
        let comments = commands[i].preceding_comments.clone();
        for comment in &comments {
            emit_comment_lines(comment, config, &indent, indent_level, &mut lines);
        }

        // A registry-declared case-list body is formatted from arg.text via
        // the pattern/body splitter; other bodies recurse normally.
        let case_list_body = case_list_body_index(&commands[i], config, registry);
        if let Some((index, _)) = case_list_body {
            // This is formatter-local presentation recovery. Semantic
            // consumers continue to require a valid `case_invocation`.
            commands[i].args[index].kind = ArgKind::Body;
        }
        let arg_count = commands[i].args.len();
        for a in 0..arg_count {
            if commands[i].args[a].kind == ArgKind::Body && commands[i].args[a].is_braced {
                format_plain_body_argument(
                    &mut commands[i].args[a],
                    case_list_body,
                    config,
                    registry,
                    identities,
                    (a, source_offset, inner_level),
                );
            } else if commands[i].args[a].kind == ArgKind::LambdaLiteral
                && commands[i].args[a].is_braced
                && let Some(&tok) = commands[i].args[a].tokens.first()
                && let Some(elems) =
                    decoded_lambda(&commands[i].args[a], commands[i].logical_context.is_some())
                && let Some(body_text) = elems.body
            {
                let (formatted, _retained_conditional_phase) = format_lambda_body_argument(
                    (&commands[i], a),
                    (source, source_offset, tok, inner_level),
                    config,
                    registry,
                    identities,
                    body_text.as_ref(),
                );
                commands[i].args[a].formatted_body = Some(formatted);
            }
        }

        let line = reconstruct_command(&sm, &commands[i], config, registry, &indent, indent_level);

        // Keep `a; b` on one line when the user disabled
        // `replace_semicolons_with_newlines`, but only in the simple case:
        // the previous command ended with a bare `;`, neither this command
        // nor the previous rendered line spans multiple lines, and this
        // command has no leading comment/blank that would force a break.
        // Any other shape falls back to the default
        // one-command-per-line layout.
        let joinable = !config.replace_semicolons_with_newlines
            && i > 0
            && commands[i - 1].terminated_by_semicolon
            && commands[i].preceding_comments.is_empty()
            && !line.contains('\n')
            && lines.last().is_some_and(|prev| !prev.contains('\n'));
        if joinable {
            let prev = lines.last_mut().expect("previous line exists");
            let trimmed = line.trim_start();
            prev.push_str("; ");
            prev.push_str(trimmed);
        } else {
            lines.push(line);
        }
    }

    for comment in &trailing_comments {
        emit_comment_lines(comment, config, &indent, indent_level, &mut lines);
    }

    lines.join("\n")
}

/// Emit a comment into `out`, either re-indented to the code column
/// (`align_comments_to_code`, the default) or at its original source column.
/// The commented-code long-line split only applies when
/// aligning — preserving the author's column means leaving the line as-is.
fn emit_comment_lines(
    comment: &CommentLine,
    config: &FormatterConfig,
    indent: &str,
    indent_level: usize,
    out: &mut Vec<String>,
) {
    let formatted = format_comment(&comment.text, config);
    if config.align_comments_to_code {
        match split_commented_code(&formatted, config, indent, indent_level) {
            Some(split) => out.push(split),
            None => out.push(format!("{indent}{formatted}")),
        }
    } else {
        let prefix = comment.orig_indent.as_deref().unwrap_or(indent);
        out.push(format!("{prefix}{formatted}"));
    }
}

/// Trim only lexer-owned trivia under the formatter's complete grammar.
/// Interior word data, malformed input and opaque source spellings remain.
pub(crate) fn trim_trailing_ws_preserving_literals(text: &str, config: &FormatterConfig) -> String {
    super::source_layout::trim_trailing_whitespace(text, config.lexer_config())
}

/// Format a Tcl source string.  Pure function: source in,
/// formatted source out.
#[must_use]
pub fn format_tcl(source: &str, config: &FormatterConfig, registry: &CommandRegistry) -> String {
    format_tcl_impl(source, config, registry, None, None)
}

/// Format using the caller's actual complete availability context and grammar.
/// Style settings come from `config`; naming and lexer rules come from `input`.
#[must_use]
pub fn format_tcl_with_input(
    source: &str,
    config: &FormatterConfig,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> String {
    let config = config.for_resolved_input(input);
    let context = input.context_registry();
    format_tcl_impl(source, &config, context.commands(), Some(input), None)
}

pub(crate) fn format_tcl_with_layout(
    source: &str,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    layout: FormattingSourceLayout,
) -> String {
    format_tcl_impl(source, config, registry, None, Some(layout))
}

fn format_tcl_impl(
    source: &str,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    input: Option<&tcl_compiler::analyser::ResolvedAnalysisInput>,
    layout: Option<FormattingSourceLayout>,
) -> String {
    // Resolve this from the original document, before canonicalising its input
    // for Tcl parsing; output still honours the document's established EOL.
    let line_ending = config.resolved_line_ending(source).to_owned();
    // Formatting operates on document text, so match Tcl's source-channel
    // newline translation before handing it to lexer-owned parsing. This is
    // deliberately outside the shared raw escape decoder: run-time strings
    // with raw CR/CRLF retain their Tcl value semantics.
    // Implementation contract: naming.editor.original-source-formatting-budget
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting-budget.md
    let normalised = super::normalise_document_line_endings(source);
    if !source_within_formatting_budget(&normalised, config) {
        return source.to_owned();
    }
    let source = normalised;
    // The document's command-identity facts, computed once for the whole
    // file.  Empty — and lookup-free — unless the document binds something.
    let identities = layout.unwrap_or_else(|| {
        input.map_or_else(
            || FormattingSourceLayout::new(&source, config, registry),
            |input| FormattingSourceLayout::with_input(&source, input),
        )
    });
    let mut result = format_body(
        &source,
        config,
        registry,
        &identities,
        FormattingRegion::document(config),
    );

    if config.trim_trailing_whitespace {
        result = trim_trailing_ws_preserving_literals(&result, config);
    }
    if config.ensure_final_newline && !result.ends_with('\n') {
        result.push('\n');
    }
    if line_ending != "\n" {
        result = result.replace('\n', &line_ending);
    }
    result
}

fn flush_parsed_command(
    argv: &mut Vec<CommandArg>,
    pending_comments: &mut Vec<CommentLine>,
    pending_blank_lines: usize,
    terminated_by_semicolon: bool,
    commands: &mut Vec<ParsedCommand>,
) {
    if argv.is_empty() {
        return;
    }
    let taken_args = std::mem::take(argv);
    let head_start = taken_args
        .first()
        .and_then(|arg| arg.tokens.first())
        .map_or(0, |token| token.span.start());
    commands.push(ParsedCommand {
        resolved_name: String::new(),
        source_roles: None,
        logical_roles: None,
        logical_context: None,
        source_traits: None,
        source_words: None,
        source_case: None,
        expression_bracing: None,
        head_start,
        args: taken_args,
        preceding_comments: std::mem::take(pending_comments),
        preceding_blank_lines: pending_blank_lines,
        terminated_by_semicolon,
    });
}

fn reset_selected_command_roles(cmd: &mut ParsedCommand) {
    cmd.source_roles = Some(Vec::new());
    cmd.logical_roles = None;
    cmd.logical_context = None;
    cmd.source_traits = Some(Traits::empty());
    cmd.source_words = None;
    cmd.resolved_name.clear();
}

fn selected_formatting_image<'a>(
    identities: &'a FormattingSourceLayout,
    config: &FormatterConfig,
    registry: &CommandRegistry,
) -> Option<&'a tcl_lexer::SourceImage> {
    if identities.input.lexer_config() != config.lexer_config()
        || identities
            .analysis
            .as_ref()
            .is_some_and(|analysis| analysis.body_lexer_config != Some(config.lexer_config()))
        || identities
            .input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            != registry.snapshot().semantic_key()
    {
        return None;
    }
    identities
        .conditional_phase
        .as_ref()
        .map(|phase| phase.image())
        .or_else(|| identities.realm.original_source_image())
}

fn apply_selected_member_bodies(
    cmd: &mut ParsedCommand,
    original: &super::analysis_layout::AnalysisFormattingLayout,
    head: u32,
    words: &[tcl_lexer::NativeWord],
) -> bool {
    let bodies = match original.member_body_arguments(head, words) {
        Ok(super::analysis_layout::MemberBodyArguments::CompleteNonMember) => None,
        Ok(super::analysis_layout::MemberBodyArguments::CompleteMember(bodies)) => Some(bodies),
        Err(_) => return true,
    };
    if let Some(bodies) = bodies {
        // The selected declaration vocabulary precedes global lookup even
        // when an unrelated global command has the same member spelling.
        for (arg, word) in cmd.args.iter_mut().zip(words) {
            arg.original_word = Some(word.clone());
        }
        for &(written, presentation) in &bodies {
            if let Some(arg) = cmd.args.get_mut(written)
                && arg.is_braced
            {
                arg.kind = ArgKind::Body;
                arg.member_body_presentation = Some(presentation);
            }
        }
        cmd.source_roles = Some(
            bodies
                .into_iter()
                .map(|(written, _)| (written, ArgRole::Body))
                .collect(),
        );
        return true;
    }
    false
}

fn apply_original_fallback_bodies(
    cmd: &mut ParsedCommand,
    original: &super::analysis_layout::AnalysisFormattingLayout,
    words: &[tcl_lexer::NativeWord],
) {
    let mut roles = Vec::new();
    for (written, (arg, word)) in cmd.args.iter_mut().zip(words).enumerate().skip(1) {
        arg.original_word = Some(word.clone());
        if arg.is_braced
            && word
                .content_span()
                .is_ok_and(|span| original.has_script(span))
        {
            arg.kind = ArgKind::Body;
            roles.push((written, ArgRole::Body));
        }
    }
    cmd.source_roles = Some(roles);
}

fn apply_expression_bracing(
    cmd: &mut ParsedCommand,
    whole: &str,
    segment: &tcl_compiler::segmenter::SegmentedCommand,
    identities: &FormattingSourceLayout,
    config: &FormatterConfig,
    registry: &CommandRegistry,
) {
    let mut tokens = tcl_compiler::ir::CommandTokens::from_segmented(
        &SourceMap::new(whole),
        config.lexer_config(),
        segment,
    );
    identities.realm.stamp_original_tokens(&mut tokens);
    cmd.expression_bracing = tokens.source_binding.as_ref().and_then(|binding| {
        let proof = binding.original_literal_expression_bracing(&tokens, registry)?;
        let original = proof.original_operand();
        let written = segment
            .argv
            .iter()
            .position(|token| token.span.start() == original.span().start())?;
        (written > 0).then(|| (written, proof.replacement_source_word().to_owned()))
    });
}

fn apply_selected_operand_geometry(
    cmd: &mut ParsedCommand,
    selected: &crate::original_invocation::OriginalRegistryWords,
) {
    for (ordinal, operand) in selected.operands.iter().enumerate() {
        let Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)) =
            selected.origins.get(ordinal.saturating_add(1))
        else {
            continue;
        };
        let (Some(arg), Some(operand)) = (cmd.args.get_mut(*written), operand.as_ref()) else {
            continue;
        };
        if *written == 0 {
            continue;
        }
        arg.original_word.clone_from(&operand.word);
        arg.original_input.clone_from(&operand.input);
    }
}

type SelectedCommandRoleOwners = (
    Vec<(usize, Option<tcl_registry::ArgPresentation>)>,
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
);

fn apply_selected_command_roles(
    cmd: &mut ParsedCommand,
    selected: &crate::original_invocation::OriginalRegistryWords,
    identities: &FormattingSourceLayout,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    position: (&str, &tcl_compiler::segmenter::SegmentedCommand),
) -> Option<SelectedCommandRoleOwners> {
    let (whole, segment) = position;
    let logical = match &selected.source {
        crate::original_invocation::OriginalRegistrySource::SourceTransitions(advice) => {
            advice.original_head().logical_input().is_some()
                && advice.logical_source_input() == Some(&identities.input)
        }
        _ => false,
    };
    let roles = selected.roles.as_deref().unwrap_or_default();
    let input = &identities.input;
    let context = input.context_registry();
    let (traits, presentations, case) = selected.with_source_schema(&context, |schema| {
        (
            schema.semantics.traits,
            roles
                .iter()
                .map(|&(ordinal, _)| {
                    (
                        ordinal,
                        schema.authored_source_argument_presentation(ordinal),
                    )
                })
                .collect::<Vec<_>>(),
            schema
                .authored_source_case_invocation()
                .and_then(|(descriptor, layout)| Some((descriptor, layout.clause_list_index?)))
                .or_else(|| schema.authored_source_case_presentation()),
        )
    })?;
    cmd.source_traits = Some(traits);
    cmd.source_case = case.and_then(|(descriptor, ordinal)| {
        match selected.origins.get(ordinal.checked_add(1)?)? {
            tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)
                if *written > 0 && *written < cmd.args.len() =>
            {
                Some((*written, descriptor))
            }
            _ => None,
        }
    });
    if config.enforce_braced_expr && identities.conditional_phase.is_none() {
        apply_expression_bracing(cmd, whole, segment, identities, config, registry);
    }
    // Geometry belongs to every genuine written operand, independently
    // of whether the selected schema assigns that operand a source role.
    apply_selected_operand_geometry(cmd, selected);
    let mut written_roles = Vec::new();
    for &(ordinal, role) in roles {
        let Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)) =
            selected.origins.get(ordinal.saturating_add(1))
        else {
            continue;
        };
        if *written == 0 || *written >= cmd.args.len() {
            continue;
        }
        written_roles.push((*written, role));
        let arg = &mut cmd.args[*written];
        arg.kind = match role {
            ArgRole::Body
                if arg.is_braced
                    && presentations.iter().any(|(index, presentation)| {
                        *index == ordinal
                            && presentation.is_some_and(tcl_registry::ArgPresentation::is_block)
                    }) =>
            {
                ArgKind::Body
            }
            ArgRole::LambdaLiteral if arg.is_braced && arg.original_word.is_some() => {
                ArgKind::LambdaLiteral
            }
            ArgRole::Keyword => ArgKind::Keyword,
            ArgRole::ParamList if arg.is_braced => ArgKind::ParamList,
            _ => ArgKind::Word,
        };
    }
    if logical {
        cmd.source_roles = None;
        cmd.logical_roles = Some(written_roles);
        cmd.logical_context = Some(std::sync::Arc::clone(&context));
    } else {
        cmd.source_roles = Some(written_roles);
    }
    let identity_arguments = selected
        .origins
        .iter()
        .skip(1)
        .enumerate()
        .all(|(index, origin)| {
            *origin == tcl_compiler::registry_invocation::InvocationWordOrigin::Written(index + 1)
        });
    if identity_arguments && selected.arguments.len() + 1 == cmd.args.len() {
        cmd.resolved_name.clone_from(&selected.command);
    }
    Some((presentations, context))
}

fn format_plain_body_argument(
    arg: &mut CommandArg,
    case_list_body: Option<(usize, CaseListSpec)>,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
    position: (usize, u32, usize),
) {
    let (a, source_offset, inner_level) = position;
    let body_text = arg.text.clone();
    let body_source_offset = arg.tokens.first().map_or(source_offset, |&token| {
        token_content_source_offset(source_offset, token)
    });
    let formatted = if let Some((_, case_list)) = case_list_body.filter(|(index, _)| *index == a) {
        format_case_list_body(
            &body_text,
            body_source_offset,
            &case_list,
            config,
            registry,
            identities,
            inner_level,
        )
    } else {
        format_body(
            &body_text,
            config,
            registry,
            identities,
            FormattingRegion::body(config, body_source_offset, inner_level),
        )
    };
    arg.formatted_body = Some(formatted);
}

fn original_formatted_lambda_body(
    arg: &CommandArg,
    identities: &FormattingSourceLayout,
    body_text: &str,
) -> Option<tcl_lexer::Span> {
    arg.original_word.as_ref().and_then(|word| {
        tcl_compiler::lambda_literal::split_original_lambda_literal(word)
            .and_then(|elements| elements.braced_body())
            .filter(|span| {
                word.image()
                    .try_text()
                    .ok()
                    .and_then(|text| text.get(span.as_range()))
                    == Some(body_text)
                    || identities.original.as_ref().and_then(|layout| {
                        layout
                            .presentation_span(*span)
                            .and_then(|span| layout.text().get(span.as_range()))
                    }) == Some(body_text)
            })
    })
}

fn format_lambda_body_argument(
    selected: (&ParsedCommand, usize),
    position: (&str, u32, Token, usize),
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
    body_text: &str,
) -> (
    String,
    Option<std::sync::Arc<tcl_compiler::command_binding::OriginalConditionalLambdaFormattingBody>>,
) {
    let (command, arg_index) = selected;
    let (source, source_offset, tok, inner_level) = position;
    // Format only the real, *decoded* body element —
    // `render_lambda_literal_arg` re-derives the parameter list /
    // namespace elements from the original source at
    // reconstruction time and reassembles them. Decoding first
    // (rather than reformatting the raw source spelling) matters
    // for a non-literal (bare/quoted) body: its backslash escapes
    // must be collapsed before the result is parsed as a script,
    // exactly as Tcl's own list-then-script evaluation would.
    let original_body =
        original_formatted_lambda_body(&command.args[arg_index], identities, body_text);
    let presented_body = original_body.and_then(|span| identities.presentation_span(span));
    let lambda_body_offset = presented_body.map_or_else(
        || {
            if command.args[arg_index].original_word.is_some() {
                u32::MAX
            } else {
                lambda_body_source_offset(source, source_offset, tok)
            }
        },
        tcl_lexer::Span::start,
    );
    let presented_text = identities
        .original
        .as_ref()
        .and_then(|layout| layout.text().get(presented_body?.as_range()));
    let phase = original_body
        .is_none()
        .then(|| {
            let parent = command.args[arg_index].original_word.as_ref()?;
            command
                .source_words
                .as_ref()?
                .decoded_lambda_formatting_body(&identities.input, &identities.realm, parent)
        })
        .flatten()
        .filter(|phase| phase.text() == Some(body_text));
    let conditional_phase = original_body
        .is_none()
        .then(|| {
            let parent = command.args[arg_index].original_word.as_ref()?;
            if let Some(current) = &identities.conditional_phase {
                current.child(parent, &identities.input)
            } else {
                tcl_compiler::command_binding::OriginalConditionalLambdaFormattingBody::capture(
                    &identities.realm,
                    &identities.input,
                    parent,
                )
            }
        })
        .flatten()
        .filter(|phase| phase.image().try_text().ok() == Some(body_text));
    if let Some(phase) = phase {
        let base = phase.source_base();
        let phase_layout = FormattingSourceLayout {
            realm: phase.realm().clone(),
            analysis: None,
            input: identities.input.clone(),
            original: None,
            decoded_phase: Some(phase),
            conditional_phase: None,
        };
        (
            format_body(
                body_text,
                config,
                registry,
                &phase_layout,
                FormattingRegion::body(config, base, inner_level),
            ),
            conditional_phase,
        )
    } else if let Some(phase) = conditional_phase {
        let phase_layout = FormattingSourceLayout {
            realm: identities.realm.clone(),
            analysis: None,
            input: identities.input.clone(),
            original: None,
            decoded_phase: None,
            conditional_phase: Some(phase),
        };
        (
            format_body(
                body_text,
                config,
                registry,
                &phase_layout,
                FormattingRegion::body(config, 0, inner_level),
            ),
            None,
        )
    } else {
        (
            format_body(
                presented_text.unwrap_or(body_text),
                config,
                registry,
                identities,
                FormattingRegion::body(config, lambda_body_offset, inner_level),
            ),
            None,
        )
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_variable_reconstruction_tests;

#[cfg(test)]
mod decoded_logical_lambda_phase_tests;
