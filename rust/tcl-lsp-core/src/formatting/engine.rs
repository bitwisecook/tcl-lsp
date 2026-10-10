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
    config: &FormatterConfig,
) -> String {
    match tok.kind {
        TokenType::Str if in_quotes => sm.text(tok.span).to_owned(),
        TokenType::Str => format!("{{{}}}", sm.token_text(tok)),
        TokenType::Cmd => format!(
            "[{}]",
            super::source_layout::collapse_script_separator_continuations(
                sm.token_text(tok),
                config.lexer_config(),
                MAX_FORMAT_DEPTH,
            )
            .unwrap_or_else(|| sm.token_text(tok).to_owned())
        ),
        TokenType::Var => {
            let raw = sm.text(tok.span);
            if raw.starts_with("${") {
                format!("${{{}}}", sm.token_text(tok))
            } else {
                format!("${}", sm.token_text(tok))
            }
        }
        TokenType::Expand => "{*}".to_owned(),
        // `JimTcl`'s `$(…)`: reconstruct the sugar around the expression
        // body, or formatting a Jim file rewrites `$($a*2)` to `$a*2`.
        TokenType::ExprSugar => format!("$({})", sm.token_text(tok)),
        _ => sm.token_text(tok).to_owned(),
    }
}

/// Rebuild an argument's source text from its tokens, optionally
/// rewriting `$var` → `${var}`.
fn reconstruct_arg(
    sm: &SourceMap,
    arg: &CommandArg,
    braced_vars: bool,
    config: &FormatterConfig,
) -> String {
    use std::fmt::Write as _;
    let mut raw = String::new();
    for &tok in &arg.tokens {
        if braced_vars && tok.kind == TokenType::Var {
            let _ = write!(raw, "${{{}}}", sm.token_text(tok));
        } else {
            raw.push_str(&reconstruct_raw(sm, tok, arg.is_quoted, config));
        }
    }
    if arg.is_quoted {
        format!("\"{raw}\"")
    } else {
        raw
    }
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
            .unwrap_or_else(|| reconstruct_arg(sm, arg, false, config)),
        (Some(word), None) if logical && word.config() == config.lexer_config() => {
            normalise_param_list(&arg.text, config.word_rules())
        }
        _ => reconstruct_arg(sm, arg, false, config),
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
    (original.get(2).map(|namespace| namespace.bytes())
        == proposed.get(2).map(|namespace| namespace.as_ref()))
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
) -> (Vec<ParsedCommand>, Vec<CommentLine>) {
    let mut commands: Vec<ParsedCommand> = Vec::new();
    let mut pending_comments: Vec<CommentLine> = Vec::new();
    let mut pending_blank_lines = 0usize;

    let mut argv: Vec<CommandArg> = Vec::new();
    let mut prev_type = TokenType::Eol;

    let flush = |argv: &mut Vec<CommandArg>,
                 pending_comments: &mut Vec<CommentLine>,
                 pending_blank_lines: usize,
                 terminated_by_semicolon: bool,
                 commands: &mut Vec<ParsedCommand>| {
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
    };

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
                    flush(
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

    flush(
        &mut argv,
        &mut pending_comments,
        pending_blank_lines,
        false,
        &mut commands,
    );
    (commands, pending_comments)
}

/// Current source grammar for authoring layout. This carries no execution or
/// value-equivalence permission; individual expression rewrites remain separate.
pub(crate) struct FormattingSourceLayout {
    realm: tcl_compiler::realm::CommandBindingRealm,
    analysis: Option<tcl_compiler::analyser::AnalysisResult>,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    original: Option<super::analysis_layout::AnalysisFormattingLayout>,
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
    if let Some(analysis) = &identities.analysis {
        cmd.source_roles = Some(Vec::new());
        cmd.logical_roles = None;
        cmd.logical_context = None;
        cmd.source_traits = Some(Traits::empty());
        cmd.source_words = None;
        cmd.resolved_name.clear();
        if identities.input.lexer_config() != config.lexer_config()
            || analysis.body_lexer_config != Some(config.lexer_config())
            || identities
                .input
                .context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
        {
            return;
        }
        let Some(image) = identities.realm.original_source_image() else {
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
        if let (Some(original), Some((_, words))) = (&identities.original, &aligned) {
            let bodies = match original.member_body_arguments(head, words) {
                Ok(bodies) => bodies,
                Err(_) => return,
            };
            if let Some(bodies) = bodies {
                // The selected declaration vocabulary precedes global lookup even
                // when an unrelated global command has the same member spelling.
                for (arg, word) in cmd.args.iter_mut().zip(words) {
                    arg.original_word = Some(word.clone());
                }
                for &written in &bodies {
                    if let Some(arg) = cmd.args.get_mut(written)
                        && arg.is_braced
                    {
                        arg.kind = ArgKind::Body;
                    }
                }
                cmd.source_roles = Some(
                    bodies
                        .into_iter()
                        .map(|written| (written, ArgRole::Body))
                        .collect(),
                );
                return;
            }
        }
        let Some(selected) =
            crate::original_invocation::source_registry_words(whole, analysis, segment)
        else {
            // The shared original syntax inventory retains declaration-member
            // bodies independently of a global command schema. It donates no
            // formal value, expression rewrite or executed member identity.
            if let (Some(original), Some((_, words))) = (&identities.original, &aligned) {
                let mut roles = Vec::new();
                for (written, (arg, word)) in cmd.args.iter_mut().zip(words).enumerate().skip(1) {
                    arg.original_word = Some(word.clone());
                    if arg.is_braced
                        && word
                            .content_span()
                            .ok()
                            .is_some_and(|span| original.has_script(span))
                    {
                        arg.kind = ArgKind::Body;
                        roles.push((written, ArgRole::Body));
                    }
                }
                cmd.source_roles = Some(roles);
            }
            return;
        };
        let logical = match &selected.source {
            crate::original_invocation::OriginalRegistrySource::SourceTransitions(advice) => {
                advice.original_head().logical_input().is_some()
                    && advice.logical_source_input() == Some(&identities.input)
            }
            _ => false,
        };
        let roles = selected.roles.as_deref().unwrap_or_default();
        let Some(input) = analysis.resolved_input.as_ref() else {
            return;
        };
        let context = input.context_registry();
        let Some((traits, presentations, case)) = selected.with_source_schema(&context, |schema| {
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
        }) else {
            return;
        };
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
        if config.enforce_braced_expr {
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
        // Geometry belongs to every genuine written operand, independently
        // of whether the selected schema assigns that operand a source role.
        for (ordinal, operand) in selected.operands.iter().enumerate() {
            let Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)) =
                selected
                    .origins
                    .get(ordinal.checked_add(1).unwrap_or(usize::MAX))
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
                            *index == ordinal && presentation.is_some_and(|style| style.is_block())
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
        let identity_arguments =
            selected
                .origins
                .iter()
                .skip(1)
                .enumerate()
                .all(|(index, origin)| {
                    *origin
                        == tcl_compiler::registry_invocation::InvocationWordOrigin::Written(
                            index + 1,
                        )
                });
        if identity_arguments && selected.arguments.len() + 1 == cmd.args.len() {
            cmd.resolved_name.clone_from(&selected.command);
        }
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
                        action_source_offset,
                        config,
                        registry,
                        identities,
                        inner_level,
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
    let fallback = || reconstruct_arg(sm, arg, false, config);
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
    let mut raw = reconstruct_arg(sm, arg, config.enforce_braced_variables, config);
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
    registry: &CommandRegistry,
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
    // `cmd.args[0]` is the command *name*; the keyword machinery indexes
    // arguments from the first word after it.
    let words: Vec<String> = cmd.args.iter().skip(1).map(|a| a.text.clone()).collect();
    let dynamic: Vec<bool> = cmd
        .args
        .iter()
        .skip(1)
        .map(|a| {
            a.kind != ArgKind::Word
                || a.is_braced
                || a.is_quoted
                || a.tokens
                    .iter()
                    .any(|t| matches!(t.kind, TokenType::Var | TokenType::Cmd | TokenType::Expand))
        })
        .collect();
    // The document's own release filters the candidate table, so a keyword a
    // later Tcl adds is not counted against a prefix the target resolves
    // uniquely.  The forward-compatibility half — "and it must still mean the
    // same thing in every later release of the target range" — is enforced
    // inside `rewrites_for_command` from `config.target_range()`. Explicit
    // Logical compatibility metadata uses its retained whole-context query;
    // original source schemas require a separate operand rewrite receipt.
    super::keywords::rewrites_for_command(
        registry,
        cmd.logical_context
            .as_ref()
            .map(|context| context.context().authoring_query())
            .or_else(|| config.dialect_query()),
        config,
        &cmd.resolved_name,
        &words,
        &dynamic,
    )
    .into_iter()
    .map(|r| (r.index + 1, r.text))
    .collect()
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
                    never_inline,
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
        .map(|a| reconstruct_arg(sm, a, config.enforce_braced_variables, config))
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
            let raw = reconstruct_arg(sm, arg, config.enforce_braced_variables, config);
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
pub(crate) fn formatting_range_indent(
    source: &str,
    selection: tcl_lexer::Span,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
) -> Option<usize> {
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
        let tokens = Lexer::with_config(text, config.lexer_config())
            .tokenise_all()
            .ok()?;
        let (mut commands, _) = parse_commands(text, &map, &tokens);
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
            return usize::try_from(depth).ok();
        }
    }
}

// Main entry points

/// Format a Tcl script body at the given indent level.  The core
/// recursive function.
/// Format a script body at `indent_level`, applying every engine
/// rule (comments, switch bodies, recursion, long-line wrapping, …).
/// [`format_tcl`] calls this at level 0 for a whole document; range
/// formatting calls it for a line slice at its selected script depth so
/// both paths share identical layout rules.
pub(crate) fn format_body(
    source: &str,
    source_offset: u32,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
    indent_level: usize,
) -> String {
    // Native-stack safety net — see `MAX_FORMAT_DEPTH`'s doc comment.
    // Past the cap, leave this (deeply nested) body unformatted rather than
    // recursing further, matching the give-up-gracefully fallback just below
    // for an unparseable body.
    if MAX_FORMAT_DEPTH.exceeded(u32::try_from(indent_level).unwrap_or(u32::MAX)) {
        return source.to_owned();
    }
    let sm = SourceMap::new(source);
    let Ok(tokens) =
        Lexer::with_source_map(SourceMap::new(source), config.lexer_config()).tokenise_all()
    else {
        return source.to_owned();
    };
    let (mut commands, trailing_comments) = parse_commands(source, &sm, &tokens);

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
                let body_text = commands[i].args[a].text.clone();
                let body_source_offset = commands[i].args[a]
                    .tokens
                    .first()
                    .map_or(source_offset, |&token| {
                        token_content_source_offset(source_offset, token)
                    });
                let formatted =
                    if let Some((_, case_list)) = case_list_body.filter(|(index, _)| *index == a) {
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
                            body_source_offset,
                            config,
                            registry,
                            identities,
                            inner_level,
                        )
                    };
                commands[i].args[a].formatted_body = Some(formatted);
            } else if commands[i].args[a].kind == ArgKind::LambdaLiteral
                && commands[i].args[a].is_braced
                && let Some(&tok) = commands[i].args[a].tokens.first()
                && let Some(elems) =
                    decoded_lambda(&commands[i].args[a], commands[i].logical_context.is_some())
                && let Some(body_text) = elems.body
            {
                // Format only the real, *decoded* body element —
                // `render_lambda_literal_arg` re-derives the parameter list /
                // namespace elements from the original source at
                // reconstruction time and reassembles them. Decoding first
                // (rather than reformatting the raw source spelling) matters
                // for a non-literal (bare/quoted) body: its backslash escapes
                // must be collapsed before the result is parsed as a script,
                // exactly as Tcl's own list-then-script evaluation would.
                let original_body = commands[i].args[a].original_word.as_ref().and_then(|word| {
                    tcl_compiler::lambda_literal::split_original_lambda_literal(word)
                        .and_then(|elements| elements.braced_body())
                        .filter(|span| {
                            word.image()
                                .try_text()
                                .ok()
                                .and_then(|text| text.get(span.as_range()))
                                == Some(body_text.as_ref())
                                || identities.original.as_ref().and_then(|layout| {
                                    layout
                                        .presentation_span(*span)
                                        .and_then(|span| layout.text().get(span.as_range()))
                                }) == Some(body_text.as_ref())
                        })
                });
                let presented_body =
                    original_body.and_then(|span| identities.presentation_span(span));
                let lambda_body_offset = presented_body.map_or_else(
                    || {
                        if commands[i].args[a].original_word.is_some() {
                            u32::MAX
                        } else {
                            lambda_body_source_offset(source, source_offset, tok)
                        }
                    },
                    |span| span.start(),
                );
                let presented_text = identities
                    .original
                    .as_ref()
                    .and_then(|layout| layout.text().get(presented_body?.as_range()));
                let formatted = format_body(
                    presented_text.unwrap_or(&body_text),
                    lambda_body_offset,
                    config,
                    registry,
                    identities,
                    inner_level,
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
    let mut result = format_body(&source, 0, config, registry, &identities, 0);

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

#[cfg(test)]
mod tests {
    use super::*;

    // Implementation contract: naming.editor.original-source-formatting
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
    #[test]
    fn original_source_formatting_uses_effective_roles_without_widening_captured_operands() {
        let profile = profile_of("tcl8.6");
        let registry = tcl_registry::model::ingress::context_for_profile(profile);
        let config = FormatterConfig::for_profile(profile);
        let source = "interp alias {} branch {} if 1\nbranch {puts    selected}\n";
        let result = format_tcl(source, &config, registry.commands());
        assert!(
            result.contains("branch {\n    puts selected\n}"),
            "{result}"
        );
        let source = "proc branch {args} {}\nbranch {puts    data}\n";
        let result = format_tcl(source, &config, registry.commands());
        assert!(result.contains("branch {puts    data}"), "{result}");
        let source = "for {set i 0} {$i < 2} {incr i} {puts    selected}\n";
        let result = format_tcl(source, &config, registry.commands());
        assert!(
            result.starts_with("for {set i 0} {$i < 2} {incr i} {\n"),
            "{result}"
        );
        assert!(result.contains("    puts selected"), "{result}");
    }

    // Implementation contract: naming.editor.original-source-formatting
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
    #[test]
    fn original_source_formatting_braces_only_a_retained_literal_equivalence() {
        let profile = profile_of("tcl8.6");
        let registry = tcl_registry::model::ingress::context_for_profile(profile);
        let config = FormatterConfig {
            enforce_braced_expr: true,
            ..FormatterConfig::for_profile(profile)
        };
        let literal = format_tcl("expr \"1 + 2\"\n", &config, registry.commands());
        assert_eq!(literal, "expr {1 + 2}\n");
        let dynamic = format_tcl("expr $value\n", &config, registry.commands());
        assert_eq!(dynamic, "expr $value\n");
        let shadowed = format_tcl(
            "proc expr {args} {}\nexpr \"1 + 2\"\n",
            &config,
            registry.commands(),
        );
        assert!(shadowed.ends_with("expr \"1 + 2\"\n"), "{shadowed}");
    }

    /// The seam's analyser ingress — the profile every test names a
    /// dialect through (the exact twin of the retired name resolver).
    fn profile_of(name: &str) -> &'static tcl_dialect::DialectProfile {
        tcl_registry::model::ingress::resolve_environment(name).analyser_profile()
    }

    fn fmt(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        format_tcl(src, &FormatterConfig::default(), &registry)
    }

    fn fmt_with(src: &str, config: &FormatterConfig) -> String {
        let registry = CommandRegistry::build_default();
        format_tcl(src, config, &registry)
    }

    fn fmt_dialect(src: &str, dialect: &'static tcl_dialect::DialectProfile) -> String {
        let registry = crate::registry_for_dialect_profile(dialect);
        format_tcl(src, &FormatterConfig::for_profile(dialect), registry)
    }

    /// `format_body` recurses once per
    /// nested control-flow body. Empirically, unguarded nested `if` bodies
    /// overflowed the native stack (SIGABRT) between depth 800 and 1200 on
    /// a 2 MiB thread (`cargo test`'s per-test default). 2000 is
    /// comfortably past both that crash range and `MAX_FORMAT_DEPTH` (128);
    /// the assertion is that formatting returns at all, not what it
    /// returns — past the cap, `format_body` leaves the excess nesting
    /// unformatted rather than crashing.
    #[test]
    fn deeply_nested_if_survives_formatting() {
        // Implementation contract: naming.editor.original-source-formatting-budget
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting-budget.md
        const DEPTH: usize = 2000;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("set done 1\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        let _ = fmt(&src);
    }

    #[test]
    fn formatting_budget_preserves_complete_source_before_analysis() {
        // Implementation contract: naming.editor.original-source-formatting-budget
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting-budget.md
        let source = format!(
            "set value {{{}}}\r\n",
            "{".repeat(128) + "opaque  " + &"}".repeat(128)
        );
        let config = FormatterConfig {
            ensure_final_newline: true,
            trim_trailing_whitespace: true,
            ..FormatterConfig::default()
        };
        assert!(!source_within_formatting_budget(&source, &config));
        assert_eq!(fmt_with(&source, &config), source);
        assert!(source_within_formatting_budget("puts \"{{{{\"", &config));
    }

    /// A moderately nested body (well under `MAX_FORMAT_DEPTH`) still
    /// formats normally — the safety net must not fire on realistic
    /// nesting depths.
    #[test]
    fn moderately_nested_if_still_formats() {
        let src = "if {1} {\nif {2} {\nset x 1\n}\n}\n";
        let out = fmt(src);
        assert!(out.contains("set x 1"), "{out:?}");
    }

    #[test]
    fn count_body_commands_counts_top_level_statements() {
        // naming.editor.original-source-whitespace-geometry
        // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
        let config = FormatterConfig::default().lexer_config();
        for (source, expected) in [
            ("", 0),
            ("  \n ", 0),
            ("puts hi", 1),
            ("puts a\nputs b", 2),
            ("puts a; puts b", 2),
            ("set x {a; b}", 1),
            ("set x [expr {1; 2}]", 1),
            ("puts \"a; b\"", 1),
            ("# ignored; comment\nputs hi", 1),
        ] {
            assert_eq!(
                count_body_commands(source, config),
                Some(expected),
                "{source:?}"
            );
        }
        assert!(count_body_commands("puts \"missing", config).is_none());
    }

    #[test]
    fn command_count_retains_complete_body_grammar() {
        // naming.editor.original-source-whitespace-geometry
        // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
        let source = "puts x\n{y}";
        let config = FormatterConfig::default().lexer_config();
        assert_eq!(count_body_commands(source, config), Some(2));
        let continued = tcl_lexer::LexerConfig {
            brace_line_continuation: tcl_dialect::BraceLineContinuation::Continues,
            ..config
        };
        assert_eq!(count_body_commands(source, continued), Some(1));
        let strict = tcl_lexer::LexerConfig {
            strict_quoting: true,
            ..config
        };
        assert!(count_body_commands("puts {missing", strict).is_none());
    }

    #[test]
    fn min_body_commands_keeps_single_command_body_inline() {
        // With expansion on but the threshold at 2, a
        // one-command body stays inline while a two-command body expands.
        // `catch` has an inline-able body (not NEVER_INLINE_BODY), so it
        // exercises the `expand_single_line_bodies` path.
        let config = FormatterConfig {
            expand_single_line_bodies: true,
            min_body_commands_for_expansion: 2,
            ..FormatterConfig::default()
        };
        let one = fmt_with("catch { puts hi }\n", &config);
        assert!(
            one.contains("{ puts hi }") && !one.contains("puts hi\n"),
            "single-command body should stay inline: {one:?}"
        );
        let two = fmt_with("catch { puts a; puts b }\n", &config);
        assert!(
            two.contains("puts a\n") && two.contains("puts b\n"),
            "two-command body should expand: {two:?}"
        );
    }

    #[test]
    fn enforce_braced_expr_braces_expr_and_conditions() {
        // `enforceBracedExpr` braces bare expressions.
        let config = FormatterConfig {
            enforce_braced_expr: true,
            ..FormatterConfig::default()
        };
        // `expr` concatenates its whole argument tail into one expression, so
        // the entire tail is braced (not just arg 0, which would corrupt it).
        assert_eq!(
            fmt_with("expr $a + $b\n", &config),
            "expr {$a + $b}\n",
            "expr tail must be braced as one group"
        );
        // A single-argument condition (bounded expr) is braced in place.
        assert_eq!(
            fmt_with("while $x {\n    incr y\n}\n", &config),
            "while {$x} {\n    incr y\n}\n"
        );
        // Already-braced expressions are left unchanged (idempotent).
        assert_eq!(fmt_with("expr {$a + $b}\n", &config), "expr {$a + $b}\n");
        assert_eq!(
            fmt_with("if {$x} {\n    puts hi\n}\n", &config),
            "if {$x} {\n    puts hi\n}\n"
        );
    }

    #[test]
    fn align_comments_to_code_toggles_reindentation() {
        // Default (true): a mis-indented standalone comment is re-indented to
        // the code column.
        let src = "proc f {} {\n        # over-indented\n    set x 1\n}\n";
        let aligned = fmt(src);
        assert!(
            aligned.contains("\n    # over-indented\n"),
            "default should re-indent comment to code column: {aligned:?}"
        );
        // Disabled: the comment keeps its original column.
        let config = FormatterConfig {
            align_comments_to_code: false,
            ..FormatterConfig::default()
        };
        let preserved = fmt_with(src, &config);
        assert!(
            preserved.contains("\n        # over-indented\n"),
            "disabled should preserve the original comment column: {preserved:?}"
        );
    }

    #[test]
    fn enforce_braced_expr_off_by_default() {
        // FP-guard: with the default config (off) bare exprs are untouched.
        assert_eq!(fmt("expr $a + $b\n"), "expr $a + $b\n");
    }

    #[test]
    fn enforce_braced_expr_preserves_expansion_tail() {
        // `expr {*}$pieces` expands the list before evaluating;
        // bracing it (`expr {{*}$pieces}`) would demote `{*}` to literal text
        // and break the expression. The rewrite must leave it alone.
        let config = FormatterConfig {
            enforce_braced_expr: true,
            ..FormatterConfig::default()
        };
        assert_eq!(
            fmt_with("expr {*}$pieces\n", &config),
            "expr {*}$pieces\n",
            "an expansion tail must not be braced"
        );
    }

    #[test]
    fn replace_semicolons_disabled_keeps_commands_inline() {
        // Default splits `a; b` onto two lines; disabling the
        // setting keeps them on one line joined by `; `.
        let split = fmt("puts a; puts b\n");
        assert!(
            split.contains("puts a\nputs b"),
            "default should split on `;`: {split:?}"
        );
        let config = FormatterConfig {
            replace_semicolons_with_newlines: false,
            ..FormatterConfig::default()
        };
        let joined = fmt_with("puts a; puts b\n", &config);
        assert!(
            joined.contains("puts a; puts b"),
            "disabled should keep `;`-joined commands inline: {joined:?}"
        );
    }

    #[test]
    fn bare_dollar_in_quoted_string_is_not_brace_wrapped() {
        // A lone `$` inside a double-quoted string is literal
        // data; formatting must not turn `"cost: $"` into `"cost: {$}"`.
        let out = fmt("puts \"cost: $\"\n");
        assert!(out.contains("\"cost: $\""), "{out:?}");
        assert!(!out.contains("{$}"), "bare $ was brace-wrapped: {out:?}");
    }

    #[test]
    fn backslash_newline_keeps_preceding_spaces_in_quoted_string() {
        // Tcl replaces `\<nl><following-ws>` with a single
        // space but keeps whitespace *before* the backslash. `"a \<nl> b"` is
        // the value `a  b` (two spaces) — the pre-backslash space must survive.
        let out = fmt("puts \"a \\\n b\"\n");
        assert!(
            out.contains("\"a  b\""),
            "expected two spaces preserved: {out:?}"
        );
    }

    #[test]
    fn backslash_newline_collapse_handles_crlf() {
        // A `\<CR><LF>` continuation collapses exactly like `\<LF>` (the old
        // LF-only normalisation left the `\` and CRLF in place). In a quoted
        // string the pre-backslash space is data and survives (`a  b`)…
        let out = fmt("puts \"a \\\r\n b\"\n");
        assert!(
            out.contains("\"a  b\""),
            "expected CRLF continuation collapsed with data space kept: {out:?}"
        );
        // …while in a command substitution the whole run is one separator.
        let out = fmt("set y [string cat a \\\r\n  b]\n");
        assert!(
            out.contains("[string cat a b]"),
            "expected CRLF continuation collapsed to one separator: {out:?}"
        );
    }

    #[test]
    fn trim_preserves_multiline_braced_string_interior() {
        // Trailing spaces *inside* a multi-line braced word are
        // string data and must survive the trailing-whitespace pass.
        let input = "set x {line1   \nline2}\n";
        let out = fmt(input);
        assert!(
            out.contains("line1   \n"),
            "trailing spaces inside the braced string were stripped:\n{out:?}"
        );
    }

    #[test]
    fn trailing_trim_preserves_spaces_inside_multiline_braces() {
        // The spaces before the newline live inside the braced word, so they
        // are part of the Tcl string value — trimming them would change the
        // runtime string, not just presentation (default trim is enabled).
        let out = fmt("set x {foo   \n   bar}\n");
        assert!(
            out.contains("foo   \n"),
            "significant whitespace inside braces preserved: {out:?}",
        );
    }

    #[test]
    fn trim_preserves_multiline_quoted_string_interior() {
        let input = "set x \"line1   \nline2\"\n";
        let out = fmt(input);
        assert!(
            out.contains("line1   \n"),
            "trailing spaces inside the quoted string were stripped:\n{out:?}"
        );
    }

    #[test]
    fn trim_still_strips_structural_trailing_whitespace() {
        // Ordinary code lines (outside any literal) are still trimmed.
        let out = trim_trailing_ws_preserving_literals(
            "set a 1   \nset b 2   ",
            &FormatterConfig::default(),
        );
        assert_eq!(out, "set a 1\nset b 2");
    }

    #[test]
    fn trailing_trim_still_trims_ordinary_code_lines() {
        // A trailing run of spaces on a real code line (not inside a literal)
        // is still trimmed.
        let out = fmt("set a 1   \nset b 2\n");
        assert_eq!(out, "set a 1\nset b 2\n");
    }

    #[test]
    fn irule_brace_chain_gets_a_space() {
        // TMM accepts `}{` (e.g. `if {expr}{body}`); with the f5-irules lexer
        // preset the formatter parses it as two words and re-emits `} {`.
        let registry = CommandRegistry::build_default();
        let config = FormatterConfig::for_profile(tcl_dialect::DialectProfile::irules());
        let out = format_tcl("if { 1 }{\n    pool p\n}\n", &config, &registry);
        assert!(!out.contains("}{"), "left `}}{{` unfixed:\n{out}");
        assert!(out.contains("} {"), "no `}} {{`:\n{out}");

        // Plain-Tcl default preset does not synthesise the separator, so it
        // leaves the (invalid-in-stock-Tcl) input alone rather than inventing a
        // parse — no accidental change to non-iRule formatting.
        let plain = format_tcl(
            "if { 1 }{\n    pool p\n}\n",
            &FormatterConfig::default(),
            &registry,
        );
        assert!(
            plain.contains("}{"),
            "default preset should not rewrite `}}{{`"
        );
    }

    #[test]
    fn the_irules_profile_alone_decides_the_ghost_separator() {
        // The dialect reaches the formatter as one resolved
        // profile, so a caller that names iRules — under either spelling —
        // gets the iRules lexer, and one that names a Tcl release does not.
        // `}{` is the discriminator: TMM parses it as two words, stock Tcl
        // does not.
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let source = "when HTTP_REQUEST {\n    if { 1 }{\n        pool p\n    }\n}\n";
        for spelling in ["f5-irules", "irules", "tcl-irule"] {
            let out = format_tcl(
                source,
                &FormatterConfig::for_profile(profile_of(spelling)),
                registry,
            );
            assert!(out.contains("} {"), "{spelling} emitted no `}} {{`:\n{out}");
            assert!(
                !out.contains("}{"),
                "{spelling} left `}}{{` unfixed:\n{out}"
            );
        }
        // The mismatched modern-Tcl profile — what a caller that forgets the
        // dialect gets — leaves the same bytes alone.
        let tcl9 = format_tcl(
            source,
            &FormatterConfig::for_profile(profile_of("tcl9.0")),
            registry,
        );
        assert!(
            tcl9.contains("}{"),
            "the Tcl 9 profile must not synthesise the separator:\n{tcl9}"
        );
    }

    /// Each `(input, expected)` pair is the expected formatted output
    /// for the same input.
    fn check(input: &str, expected: &str) {
        let got = fmt(input);
        assert_eq!(
            got, expected,
            "\ninput:    {input:?}\ngot:      {got:?}\nexpected: {expected:?}"
        );
    }

    #[test]
    fn logical_default_formatting_retains_its_whole_context_over_mixed_specs() {
        // naming.editor.logical-formatting-context
        // docs/design/analysis/name-resolution-proofs/logical-formatting-context.md
        let registry = CommandRegistry::build_default();
        assert!(
            !registry
                .get("expr")
                .unwrap()
                .traits
                .contains(Traits::EXPR_CONCATENATES_ARGS)
        );
        let config = FormatterConfig::default();
        let identities = FormattingSourceLayout::new("expr $a + $b", &config, &registry);
        assert!(identities.analysis.is_some());
        let context = identities.input.context_registry();
        assert_eq!(context.context().environment.id.as_str(), "tcl");
        assert!(
            tcl_registry::InvocationDialect::of_profile(identities.input.analyser_profile(),)
                .native_string_protocol()
                .is_none()
        );
        let source = "expr $a + $b";
        let map = SourceMap::new(source);
        let tokens = Lexer::with_config(source, config.lexer_config())
            .tokenise_all()
            .unwrap();
        let mut command = parse_commands(source, &map, &tokens).0.remove(0);
        identify_body_args(&mut command, &registry, &identities, 0, &config, source);
        assert!(command.source_roles.is_none());
        assert!(command.logical_roles.is_some());
        assert!(command.logical_context.is_some());
        assert!(
            command
                .source_traits
                .unwrap()
                .contains(Traits::EXPR_CONCATENATES_ARGS)
        );
        assert!(command.args.iter().all(|arg| arg.original_input.is_none()));
        assert!(
            command
                .args
                .iter()
                .skip(1)
                .all(|arg| arg.original_word.is_some())
        );
        assert!(
            matches!(command.source_words.as_ref().map(|words| &words.source),
            Some(crate::original_invocation::OriginalRegistrySource::SourceTransitions(advice))
                if advice.logical_source_input() == Some(&identities.input))
        );
        let mut foreign = FormatterConfig::default();
        foreign.lexer_config_override = Some(tcl_lexer::LexerConfig {
            strict_quoting: true,
            ..config.lexer_config()
        });
        let mut command = parse_commands(source, &map, &tokens).0.remove(0);
        identify_body_args(&mut command, &registry, &identities, 0, &foreign, source);
        assert_eq!(command.source_traits, Some(Traits::empty()));
        assert_eq!(command.source_roles, Some(Vec::new()));
        assert!(command.logical_roles.is_none());
        assert!(command.logical_context.is_none());
        assert!(keyword_rewrites_for(&command, &registry, &foreign).is_empty());
    }

    #[test]
    fn analysed_layout_retains_supplied_entry_and_provider_world_without_recapture() {
        // naming.editor.original-source-formatting
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
        // Actual source entry/provider coverage remains independent of source roles.
        use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
        use tcl_compiler::command_binding::SourceAnalysisEntry;
        let source = "if 1 {puts α}\r\n";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let mut entry = SourceAnalysisEntry::for_logical_source(&input).unwrap();
        entry.unknown_entry = true;
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .with_source_analysis_entry(std::sync::Arc::new(entry))
            .analyse(source, profile.name);
        analysis.has_dynamic_providers = true;
        let layout = FormattingSourceLayout::with_analysis(source, &analysis).unwrap();
        assert_eq!(layout.realm, *analysis.retained_command_realm().unwrap());
        assert!(layout.analysis.as_ref().unwrap().has_dynamic_providers);
        assert_eq!(layout.input, input);
        let fresh =
            FormattingSourceLayout::with_input(layout.original.as_ref().unwrap().text(), &input);
        assert_ne!(
            layout.realm, fresh.realm,
            "unknown supplied history cannot become a fresh source world"
        );
    }

    #[test]
    fn simple_command() {
        check("puts hi\n", "puts hi\n");
    }

    #[test]
    fn proc_body_indents() {
        check("proc f {} {\nset x 1\n}\n", "proc f {} {\n    set x 1\n}\n");
    }

    #[test]
    fn if_else() {
        check(
            "if {$x} {\nputs a\n} else {\nputs b\n}\n",
            "if {$x} {\n    puts a\n} else {\n    puts b\n}\n",
        );
    }

    #[test]
    fn nested_indentation() {
        check(
            "proc f {} {\nif {1} {\nset x 1\n}\n}\n",
            "proc f {} {\n    if {1} {\n        set x 1\n    }\n}\n",
        );
    }

    #[test]
    fn comments_preserved() {
        check("#hello\n# spaced\nputs hi\n", "#hello\n# spaced\nputs hi\n");
    }

    #[test]
    fn collapses_blank_lines_to_max() {
        check("set x 1\n\n\n\nset y 2\n", "set x 1\n\n\nset y 2\n");
    }

    #[test]
    fn switch_body() {
        check(
            "switch $x {\na {\nputs 1\n}\nb {\nputs 2\n}\n}\n",
            "switch $x {\n    a {\n        puts 1\n    }\n    b {\n        puts 2\n    }\n}\n",
        );
    }

    #[test]
    fn switch_body_preserves_comments() {
        // A comment line inside a case-list body must survive
        // formatting rather than being silently deleted.
        let out = fmt("switch $x {\n# note\na { puts 1 }\n}\n");
        assert!(
            out.contains("# note"),
            "switch-body comment was deleted:\n{out}"
        );
        assert!(out.contains("puts 1"), "arm body lost:\n{out}");
    }

    #[test]
    fn switch_case_list_formatter_follows_release_matrix() {
        let valid = "switch subject {\ndefault {\nputs hit\n}\n}\n";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"] {
            let out = fmt_dialect(valid, profile_of(dialect));
            assert!(
                out.contains("    default {\n        puts hit\n    }"),
                "{dialect} must format a normal case-list body:\n{out}"
            );
        }
        for subject in ["-regexp", "--"] {
            let ambiguous = format!("switch {subject} {{\ndefault {{\nputs hit\n}}\n}}\n");
            let old = fmt_dialect(&ambiguous, profile_of("tcl8.4"));
            assert!(
                !old.contains("        puts hit"),
                "Tcl 8.4 must not descend the invalid option-like two-word form {subject:?}:\n{old}"
            );
            for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
                let out = fmt_dialect(&ambiguous, profile_of(dialect));
                assert!(
                    out.contains("    default {\n        puts hit\n    }"),
                    "{dialect} must format the optionless two-word case list with subject {subject:?}:\n{out}"
                );
            }
        }
    }

    #[test]
    fn incomplete_case_list_recovery_is_presentation_only_and_idempotent() {
        fn assert_fixed_point(
            source: &str,
            dialect: &'static tcl_dialect::DialectProfile,
            nested: &str,
        ) -> String {
            let once = fmt_dialect(source, dialect);
            assert!(
                once.contains(nested),
                "{} did not present the incomplete case list:\n{once}",
                dialect.name
            );
            assert_eq!(
                fmt_dialect(&once, dialect),
                once,
                "{} formatter recovery is not idempotent",
                dialect.name
            );
            once
        }

        let ordinary = "switch subject {\na {\nputs hit\n}\norphan\n}\n";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"] {
            assert_fixed_point(
                ordinary,
                profile_of(dialect),
                "a {\n        puts hit\n    }",
            );
        }

        // An empty list is not a semantic case invocation, but remains a
        // formatter-local recoverable body position. Rendering it must stay
        // safe and must not change Tcl's erroneous source into a made-up
        // clause.
        let empty = "switch subject {}\n";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"] {
            assert_eq!(fmt_dialect(empty, profile_of(dialect)), empty, "{dialect}");
        }

        let option_like = "switch -regexp {\na {\nputs hit\n}\norphan\n}\n";
        let old = fmt_dialect(option_like, profile_of("tcl8.4"));
        assert!(
            !old.contains("        puts hit"),
            "Tcl 8.4 must not recover the invalid two-word option form:\n{old}"
        );
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
            assert_fixed_point(
                option_like,
                profile_of(dialect),
                "a {\n        puts hit\n    }",
            );
        }

        let aliased =
            fmt("interp alias {} pick {} switch\npick subject {\na {\nputs hit\n}\norphan\n}\n");
        assert!(
            aliased.contains("a {\n        puts hit\n    }"),
            "resolved alias did not inherit formatter recovery:\n{aliased}"
        );
        assert_eq!(fmt(&aliased), aliased);
    }

    #[test]
    fn expect_case_list_formatter_preserves_descriptor_fields_and_formats_only_actions() {
        let source = "expect {\n-regexp {a; b} {puts canonical}\n-re {c; d} {puts abbreviated}\n-glob \"hello world\" {puts quoted}\n-exact \"escaped\\ pattern\" {puts escaped}\n-timeout 5 timeout {puts timed}\n-i $spawn_id eof {puts eof}\n-- {-literal} {puts literal}\nfull_buffer\n}\n";
        let once = fmt_dialect(source, profile_of("expect"));
        assert_eq!(fmt_dialect(&once, profile_of("expect")), once, "{once}");

        for literal in [
            "-regexp {a; b} {",
            "-re {c; d} {",
            "-glob \"hello world\" {",
            "-exact \"escaped\\ pattern\" {",
            "-timeout 5 timeout {",
            "-i $spawn_id eof {",
            "-- {-literal} {",
        ] {
            assert!(
                once.contains(literal),
                "descriptor field changed: {literal:?}\n{once}"
            );
        }
        for action in [
            "puts canonical",
            "puts abbreviated",
            "puts quoted",
            "puts escaped",
            "puts timed",
            "puts eof",
            "puts literal",
        ] {
            assert!(
                once.contains(&format!("        {action}")),
                "action was not formatted: {action:?}\n{once}"
            );
        }
        assert!(
            once.contains("    full_buffer\n"),
            "omitted final action was invented or lost:\n{once}"
        );
        assert!(
            !once.contains("        a\n"),
            "regex data was formatted as a script:\n{once}"
        );
        assert!(
            !once.contains("        c\n"),
            "abbreviated regex data was formatted as a script:\n{once}"
        );

        for source in [
            "expect {\n-bogus {a; b} {puts opaque}\n}\n",
            "expect {\n-timeout\n}\n",
        ] {
            let formatted = fmt_dialect(source, profile_of("expect"));
            assert_eq!(
                fmt_dialect(&formatted, profile_of("expect")),
                formatted,
                "{formatted}"
            );
            assert!(
                !formatted.contains("        a\n"),
                "malformed data was treated as an action:\n{formatted}"
            );
        }

        // Once a descriptor flag cannot be resolved, no later apparent
        // pattern/action pairing is proven. The formatter must keep the
        // entire case-list value opaque rather than resynchronising at `q`
        // or `x` and interpreting data as Tcl scripts.
        for malformed in [
            "expect {\n-bogus p q {puts falsely_formatted}\nx {puts also_formatted}\n}\n",
            "expect {\n-n p {puts ambiguous}\nx {puts also_ambiguous}\n}\n",
            "expect {\n-timeout\n}\n",
        ] {
            let formatted = fmt_dialect(malformed, profile_of("expect"));
            assert_eq!(formatted, malformed, "malformed list was rewritten");
            assert_eq!(fmt_dialect(&formatted, profile_of("expect")), formatted);
        }
        let dynamic = fmt_dialect(
            "expect {\n-re $pattern {puts dynamic}\n}\n",
            profile_of("expect"),
        );
        assert!(
            dynamic.contains("-re $pattern {"),
            "dynamic pattern changed:\n{dynamic}"
        );
        assert!(
            dynamic.contains("        puts dynamic"),
            "proven action was not formatted:\n{dynamic}"
        );
    }

    #[test]
    fn control_flow_always_expands() {
        check("if {$x} { return }\n", "if {$x} {\n    return\n}\n");
    }

    #[test]
    fn while_body() {
        check("while {$x} {\nincr x\n}\n", "while {$x} {\n    incr x\n}\n");
    }

    #[test]
    fn foreach_body() {
        check(
            "foreach i {1 2 3} {\nputs $i\n}\n",
            "foreach i {1 2 3} {\n    puts $i\n}\n",
        );
    }

    // The formatting engine holds no `if` / `try` / `for` name checks; every
    // layout decision comes from the registry.

    /// TP / FN — C Tcl resolves the absolute global spelling to the same
    /// command (`namespace which -command ::if` → `::if`), so `::if`,
    /// `::for`, and `::try` must format exactly like their bare forms. The
    /// old `name == "if"` comparisons simply did not fire for them.
    #[test]
    fn qualified_control_flow_formats_like_the_bare_form() {
        for (bare, qualified) in [
            ("if {$x} then {\nputs yes\n} else {\nputs no\n}\n", "::if"),
            ("for {set i 0} {$i < 3} {incr i} {\nputs $i\n}\n", "::for"),
            (
                "try {\nrisky\n} on error {msg opts} {\nputs $msg\n}\n",
                "::try",
            ),
            ("while {$x} {\nincr x\n}\n", "::while"),
        ] {
            let head = bare.split_whitespace().next().expect("a head word");
            let qualified_src = bare.replacen(head, qualified, 1);
            let want = fmt(bare).replacen(head, qualified, 1);
            assert_eq!(
                fmt(&qualified_src),
                want,
                "{qualified} formatted differently from {head}"
            );
        }
    }

    /// TP — `for`'s `start` and `next` scripts stay inline while only `body`
    /// expands. That preference is now registry data
    /// (`arg_presentation: InlineScript`), not a formatter branch.
    #[test]
    fn for_keeps_start_and_next_inline() {
        check(
            "for {set i 0} {$i < 3} {incr i} {\nputs $i\n}\n",
            "for {set i 0} {$i < 3} {incr i} {\n    puts $i\n}\n",
        );
    }

    /// TP — every `if` clause shape: implicit and explicit `then`, repeated
    /// `elseif`, and the bare implicit-else body.
    #[test]
    fn every_if_clause_shape_formats() {
        check(
            "if {$a} {\nx\n} elseif {$b} then {\ny\n} elseif {$c} {\nz\n} else {\nw\n}\n",
            "if {$a} {\n    x\n} elseif {$b} then {\n    y\n} elseif {$c} {\n    z\n} else {\n    w\n}\n",
        );
        // Implicit else — the trailing bare body is still a body.
        check(
            "if {$a} {\nx\n} {\ny\n}\n",
            "if {$a} {\n    x\n} {\n    y\n}\n",
        );
    }

    /// TP — every `try` handler shape.
    #[test]
    fn every_try_handler_shape_formats() {
        let out =
            fmt("try {\na\n} on error {m o} {\nb\n} trap {POSIX} {m o} {\nc\n} finally {\nd\n}\n");
        assert!(out.contains("} on error {m o} {"), "{out}");
        assert!(out.contains("} trap {POSIX} {m o} {"), "{out}");
        assert!(out.contains("} finally {"), "{out}");
    }

    /// FP guard — a *data* word that merely spells a keyword is not one. The
    /// old scan matched by word value across every argument, so `puts else`
    /// and `lappend l on trap` had their words retyped as keywords. The
    /// registry answers by grammar position instead, so nothing fires for a
    /// command whose spec declares no keyword there.
    #[test]
    fn keyword_lookalike_data_words_are_not_keywords() {
        check("puts else\n", "puts else\n");
        check("lappend l on trap finally\n", "lappend l on trap finally\n");
        // A user proc named `if` in a *namespace* is a different command
        // entirely — `ns::if` does not resolve to the built-in (only the
        // absolute global `::if` does), so its braced argument is left as a
        // plain word rather than reformatted as a control-flow body.
        check("ns::if {$a} {\nx\n}\n", "ns::if {$a} {\nx\n}\n");
    }

    /// FP guard — `if {1} {a} else then`: the trailing `then` sits in the
    /// else-branch **body** slot, so the grammar walk gives it the body role,
    /// not `Keyword` (verified against tclsh 8.6 / 9.0.4, which run `a` and
    /// treat `then` as the else-branch script). The old value-matching scan
    /// retyped it as a keyword wherever it appeared.
    #[test]
    fn then_in_the_else_body_slot_is_not_a_keyword() {
        let registry = CommandRegistry::build_default();
        let keywords =
            registry.arg_indices_for_role("if", &["1", "a", "else", "then"], ArgRole::Keyword);
        assert_eq!(
            keywords,
            vec![2],
            "only the real `else` at index 2 is a keyword"
        );
        // And formatting stays a fixed point over the shape.
        let once = fmt("if {1} {a} else then\n");
        assert_eq!(fmt(&once), once, "{once}");
    }

    /// TN — a dynamic command head (`{*}$cmd`) has no resolvable identity, so
    /// the engine leaves it alone entirely.
    #[test]
    fn dynamic_head_is_left_alone() {
        check(
            "{*}$cmd {$x} {\nputs hi\n}\n",
            "{*}$cmd {$x} {\nputs hi\n}\n",
        );
    }

    /// TN — an incomplete/malformed clause must stay semantics-preserving and
    /// idempotent rather than being reshaped into a guess.
    #[test]
    fn malformed_clauses_stay_stable_and_idempotent() {
        for src in [
            "if {$a}\n",
            "if {$a} {\nx\n} elseif\n",
            "try\n",
            "for {set i 0} {$i < 3}\n",
        ] {
            let once = fmt(src);
            let twice = fmt(&once);
            assert_eq!(once, twice, "not idempotent for {src:?}:\n{once}\n{twice}");
        }
    }

    fn original_list_arg(
        source: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> (FormatterConfig, CommandArg) {
        let config = FormatterConfig::for_profile(profile);
        let lexer_config = config.lexer_config();
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            lexer_config,
        )
        .unwrap();
        let word = &plan.commands[0].words[0];
        let policy = tcl_registry::InvocationDialect::of_profile(profile)
            .authored_name_policy()
            .unwrap();
        let key =
            tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                word,
                WordValueRules::from_config(&lexer_config),
                policy,
            )
            .unwrap();
        let map = SourceMap::new(source);
        let tokens = Lexer::with_config(source, lexer_config)
            .tokenise_all()
            .unwrap();
        let mut command = parse_commands(source, &map, &tokens).0.remove(0);
        let mut arg = command.args.remove(0);
        arg.kind = ArgKind::ParamList;
        arg.original_word = Some(word.clone());
        arg.original_input =
            Some(tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key));
        (config, arg)
    }

    #[test]
    fn original_param_spacing_retains_native_children_and_opaque_units() {
        // Implementation contract: naming.editor.original-native-formal-list-formatting
        // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = profile_of(name);
            for (source, expected) in [
                ("{a    b}", "{a b}"),
                (r"{a\ b    c}", "{{a b} c}"),
                (r"{{a\uD800 default}    b}", r"{{a\uD800 default} b}"),
            ] {
                let (config, arg) = original_list_arg(source, profile);
                let map = SourceMap::new(source);
                assert_eq!(
                    render_param_list_arg(&map, &arg, &config, false),
                    expected,
                    "{name}"
                );
            }
            let opaque = r"{a\uD800    b}";
            let (config, arg) = original_list_arg(opaque, profile);
            let map = SourceMap::new(opaque);
            assert!(
                normalise_original_param_list(
                    arg.original_word.as_ref().unwrap(),
                    arg.original_input.as_ref().unwrap(),
                    &config,
                )
                .is_none(),
                "{name}"
            );
            assert_eq!(
                render_param_list_arg(&map, &arg, &config, false),
                opaque,
                "{name}"
            );
            let source = "{😀    b}";
            let (config, arg) = original_list_arg(source, profile);
            let expected = if name.starts_with("tcl8.") {
                source
            } else {
                "{😀 b}"
            };
            assert_eq!(
                render_param_list_arg(&SourceMap::new(source), &arg, &config, false),
                expected,
                "{name}"
            );
        }
    }

    #[test]
    fn original_param_spacing_declines_foreign_word_and_full_config() {
        // Implementation contract: naming.editor.original-native-formal-list-formatting
        // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
        let source = "{a    b}";
        let (config, mut arg) = original_list_arg(source, profile_of("tcl8.6"));
        let word = arg.original_word.as_ref().unwrap();
        let input = arg.original_input.as_ref().unwrap();
        let mut changed = config.clone();
        let mut lexer = config.lexer_config();
        lexer.strict_quoting = !lexer.strict_quoting;
        changed.lexer_config_override = Some(lexer);
        assert!(normalise_original_param_list(word, input, &changed).is_none());
        let (_, foreign) = original_list_arg(" {a    b}", profile_of("tcl8.6"));
        assert!(
            normalise_original_param_list(word, foreign.original_input.as_ref().unwrap(), &config)
                .is_none()
        );
        arg.original_input = None;
        assert_eq!(
            render_param_list_arg(&SourceMap::new(source), &arg, &config, false),
            source
        );
    }

    #[test]
    fn original_lambda_spacing_checks_nested_formals_before_emission() {
        // Implementation contract: naming.editor.original-native-formal-list-formatting
        // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = profile_of(name);
            for source in [r"{{a\uD800    b} {puts    ok}}", r"{a\\uD800 {puts    ok}}"] {
                let (config, mut arg) = original_list_arg(source, profile);
                arg.kind = ArgKind::LambdaLiteral;
                arg.formatted_body = Some("puts ok".to_owned());
                assert_eq!(
                    render_lambda_literal_arg(
                        &SourceMap::new(source),
                        &arg,
                        &config,
                        "",
                        0,
                        false,
                        false,
                    ),
                    source,
                    "{name}"
                );
            }
            let source = "{{a    b} {puts    ok} ::N}";
            let (config, mut arg) = original_list_arg(source, profile);
            arg.kind = ArgKind::LambdaLiteral;
            arg.formatted_body = Some("puts ok".to_owned());
            assert_eq!(
                render_lambda_literal_arg(
                    &SourceMap::new(source),
                    &arg,
                    &config,
                    "",
                    0,
                    false,
                    false,
                ),
                "{{a b} { puts ok } ::N}",
                "{name}"
            );
        }
    }

    #[test]
    fn original_source_formatting_keeps_opaque_formal_word_verbatim() {
        // Implementation contract: naming.editor.original-native-formal-list-formatting
        // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let output = fmt_dialect(r"proc p {a\uD800    b} {return}", profile_of(name));
            assert!(output.contains(r"{a\uD800    b}"), "{name}: {output}");
            let output = fmt_dialect("proc p {a    b} {return}", profile_of(name));
            assert!(output.starts_with("proc p {a b} "), "{name}: {output}");
        }
    }

    #[test]
    fn param_list_normalised() {
        check(
            "proc f {a    b   c} {\nreturn\n}\n",
            "proc f {a b c} {\n    return\n}\n",
        );
    }

    /// C Tcl 9 collapses the
    /// backslash-newline in a pre-pass *before* the parameter word is
    /// list-parsed (even inside braces), so `proc f {a\<newline> b}` has the
    /// two required parameters `a` and `b`:
    ///
    /// ```text
    /// % proc f {a\
    ///  b} {return}
    /// % list [llength [info args f]] [info args f]
    /// 2 {a b}
    /// ```
    ///
    /// The old hand-rolled scanner emitted `{a\ b}` — one *optional*
    /// parameter `a` defaulting to `b`, a silent arity change.
    #[test]
    fn param_list_backslash_newline_preserves_arity() {
        let out = fmt("proc f {a\\\n b} {return}\n");
        assert!(
            out.starts_with("proc f {a b} {"),
            "backslash-newline changed the parameter list:\n{out}"
        );
        assert!(
            !out.contains(r"a\ b"),
            "two parameters were fused into one defaulted parameter:\n{out}"
        );
        // The parsed signature agrees: two params, neither defaulted.
        let params =
            tcl_compiler::signature_scan::params::parse_param_list("a b", WordValueRules::TCL);
        assert_eq!(params.len(), 2);
        assert!(params.iter().all(|p| !p.has_default));
    }

    /// FP guard — an *escaped space* really is one element (parameter `a`
    /// defaulting to `b`), and must stay one element. The renderer re-quotes
    /// it canonically as `{a b}`, which C Tcl reads identically.
    #[test]
    fn param_list_escaped_space_stays_one_parameter() {
        let out = fmt("proc f {a\\ b c} {return}\n");
        assert!(
            out.starts_with("proc f {{a b} c} {"),
            "escaped-space spec lost its element identity:\n{out}"
        );
        let params =
            tcl_compiler::signature_scan::params::parse_param_list("{a b} c", WordValueRules::TCL);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "a");
        assert_eq!(params[0].default_value.as_deref(), Some("b"));
    }

    /// TN — a parameter list that is not a well-formed Tcl list (mid-edit
    /// unmatched brace) has no canonical rendering, so the source is
    /// preserved verbatim rather than rewritten into something else.
    #[test]
    fn param_list_malformed_preserved_verbatim() {
        // `a "b` is a brace-balanced word, so the formatter sees it as the
        // parameter list — but it is not a well-formed *list* (unmatched
        // quote), so there is nothing safe to re-render.
        let out = fmt("proc f {a \"b} {return}\n");
        assert!(
            out.contains("{a \"b}"),
            "malformed parameter list was rewritten:\n{out}"
        );
    }

    /// Structure-preserving cases: defaults, nested braces, `args`, empty
    /// defaults, and Unicode all survive a format round-trip, and formatting
    /// is idempotent.
    #[test]
    fn param_list_shapes_round_trip_and_are_idempotent() {
        for src in [
            "proc f {a {b 1} c} {return}\n",
            "proc f {args} {return}\n",
            "proc f {{a {}} args} {return}\n",
            "proc f {{opts {-x 1 -y 2}}} {return}\n",
            "proc f {naïve {héllo wörld}} {return}\n",
            "proc f {a\\\r\n b} {return}\n",
        ] {
            let once = fmt(src);
            let twice = fmt(&once);
            assert_eq!(once, twice, "not idempotent for {src:?}:\n{once}\n{twice}");
        }
        // Defaults keep their element boundaries.
        assert!(fmt("proc f {a   {b 1}   c} {return}\n").starts_with("proc f {a {b 1} c} {"));
        // A nested-brace default is one element, braces intact.
        assert!(fmt("proc f {{opts {-x 1}}} {return}\n").starts_with("proc f {{opts {-x 1}}} {"));
        // CRLF continuations behave exactly like LF ones.
        assert!(fmt("proc f {a\\\r\n b} {return}\n").starts_with("proc f {a b} {"));
    }

    #[test]
    fn quoted_string_preserved() {
        check("puts \"hello $name\"\n", "puts \"hello $name\"\n");
    }

    /// `apply`'s lambda-literal argument (`ArgRole::LambdaLiteral`,
    /// not `Body`) must have its real body element reformatted — not the
    /// whole `{argList} {body}` blob re-segmented as one script (which
    /// misreads the parameter word as a command name and never reaches the
    /// real body). The parameter list is normalised the same way a `proc`'s
    /// is (bare `dir` → braced `{dir}`), matching `param_list_normalised`.
    #[test]
    fn apply_lambda_body_indents_and_params_normalise() {
        check(
            "apply {dir {\nputs    $dir\nset x 1\n}} /tmp\n",
            "apply {{dir} {\n    puts $dir\n    set x 1\n}} /tmp\n",
        );
    }

    /// The optional namespace element (lambda literal position 2) survives
    /// reassembly untouched. A short single-command body inlines (matching
    /// ordinary body formatting), so this also confirms the inline path
    /// reassembles the namespace element correctly.
    #[test]
    fn apply_lambda_namespace_element_preserved() {
        check(
            "apply {dir {\nputs    $dir\n} ::foo} /tmp\n",
            "apply {{dir} { puts $dir } ::foo} /tmp\n",
        );
    }

    /// A bare body element's backslash
    /// escape must be decoded before reformatting, not reformatted from its
    /// raw source spelling — `puts\ hi`'s real runtime body is the two-word
    /// command `puts hi`, not one word containing a literal backslash.
    #[test]
    fn apply_lambda_body_with_backslash_escape_decodes() {
        check(r"apply {{} puts\ hi}", "apply {{} { puts hi }}\n");
    }

    #[test]
    fn force_split_long_line() {
        check(
            "mycommand argumentone argumenttwo argumentthree argumentfour argumentfive argumentsix argumentseven argumenteight argumentnine\n",
            "mycommand argumentone argumenttwo argumentthree argumentfour argumentfive argumentsix argumentseven argumenteight \\\n    argumentnine\n",
        );
    }

    #[test]
    fn expression_wrapping_preserves_string_terms_and_declines_comments() {
        // naming.editor.original-source-whitespace-geometry
        // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
        let config = FormatterConfig::for_profile(profile_of("tcl9.0"));
        let expression = "\"a  b\" eq \"a  b\" && [string equal {c   d} {c   d}]";
        let wrapped = wrap_braced_expr(expression, &config, 0).unwrap();
        assert!(wrapped.contains("\"a  b\" eq \"a  b\""), "{wrapped}");
        assert!(
            wrapped.contains("[string equal {c   d} {c   d}]"),
            "{wrapped}"
        );
        assert!(wrap_braced_expr("1 # && comment\n&& 2", &config, 0).is_none());
        assert!(wrap_braced_expr("1 && \"missing quote", &config, 0).is_none());
    }

    #[test]
    fn bracket_continuations_preserve_nested_word_data() {
        // naming.editor.original-source-whitespace-geometry
        // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
        let source = "set x [list \\\n \"a \\\n b\" {c \\\n d}]\n";
        let result = fmt(source);
        assert!(
            result.contains("[list \"a \\\n b\" {c \\\n d}]"),
            "{result:?}"
        );
    }

    #[test]
    fn multiline_braced_data_keeps_its_original_continuation() {
        // naming.editor.original-source-whitespace-geometry
        // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
        let source = "set x {a \\\n b}\n";
        for name in ["tcl8.4", "tcl8.6", "tcl9.0", "jim"] {
            let profile = profile_of(name);
            let context = tcl_registry::model::ingress::context_for_profile(profile);
            assert_eq!(
                format_tcl(
                    source,
                    &FormatterConfig::for_profile(profile),
                    context.commands()
                ),
                source,
                "{name}"
            );
        }
    }

    #[test]
    fn force_expr_wrap() {
        check(
            "if {$variableaaaa == 1 && $variablebbbb == 2 && $variablecccc == 3 && $variabledddd == 4 && $variableeeee == 5 && $variableffff == 6} {\nputs hi\n}\n",
            "if {\n    $variableaaaa == 1\n    && $variablebbbb == 2\n    && $variablecccc == 3\n    && $variabledddd == 4\n    && $variableeeee == 5\n    && $variableffff == 6\n} {\n    puts hi\n}\n",
        );
    }

    #[test]
    fn semicolon_splits_commands() {
        check("set x 1; set y 2\n", "set x 1\nset y 2\n");
    }

    #[test]
    fn empty_proc_body() {
        check("proc f {} {}\n", "proc f {} {}\n");
    }

    #[test]
    fn try_finally() {
        check(
            "try {\nfoo\n} finally {\nbar\n}\n",
            "try {\n    foo\n} finally {\n    bar\n}\n",
        );
    }

    #[test]
    fn command_substitution_preserved() {
        check("set y [expr {1+2}]\n", "set y [expr {1+2}]\n");
    }

    #[test]
    fn expand_prefix_preserved() {
        check("puts {*}$args\n", "puts {*}$args\n");
    }

    #[test]
    fn multi_hash_comment() {
        check("## section\nputs hi\n", "## section\nputs hi\n");
    }

    #[test]
    fn trailing_comment() {
        check("puts hi\n# trailing\n", "puts hi\n# trailing\n");
    }

    #[test]
    fn deeply_nested_bodies() {
        check(
            "proc f {} {\nforeach x $list {\nif {$x} {\nputs $x\n}\n}\n}\n",
            "proc f {} {\n    foreach x $list {\n        if {$x} {\n            puts $x\n        }\n    }\n}\n",
        );
    }

    #[test]
    fn enforce_braced_variables_rewrites_dollar_refs() {
        let registry = CommandRegistry::build_default();
        let cfg = FormatterConfig {
            enforce_braced_variables: true,
            ..FormatterConfig::default()
        };
        assert_eq!(format_tcl("puts $x\n", &cfg, &registry), "puts ${x}\n");
    }

    #[test]
    fn procedure_spacing_uses_selected_source_traits() {
        // naming.editor.original-source-formatting
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
        let profile = profile_of("tcl8.6");
        let registry = tcl_registry::model::ingress::context_for_profile(profile);
        let config = FormatterConfig {
            blank_lines_between_procs: 2,
            blank_lines_between_blocks: 1,
            ..FormatterConfig::for_profile(profile)
        };
        let qualified = format_tcl(
            "::proc first {} {}\n::proc second {} {}\n",
            &config,
            registry.commands(),
        );
        assert_eq!(qualified, "::proc first {} {}\n\n\n::proc second {} {}\n");
        let shadowed = format_tcl(
            "proc proc {args} {}\nproc first\nproc second\n",
            &config,
            registry.commands(),
        );
        assert!(
            shadowed.ends_with("proc first\nproc second\n"),
            "{shadowed}"
        );
        let source = "proc first {} {}\nproc second {} {}\n";
        let identities = FormattingSourceLayout::new(source, &config, registry.commands());
        let mut foreign = config.clone();
        foreign.lexer_config_override = Some(tcl_lexer::LexerConfig {
            strict_quoting: !config.lexer_config().strict_quoting,
            ..config.lexer_config()
        });
        let unchanged = format_body(source, 0, &foreign, registry.commands(), &identities, 0);
        assert_eq!(unchanged, source.trim_end_matches('\n'));
    }

    #[test]
    fn blank_lines_between_procs() {
        check(
            "proc a {} {\nreturn\n}\nproc b {} {\nreturn\n}\n",
            "proc a {} {\n    return\n}\n\nproc b {} {\n    return\n}\n",
        );
    }
    /// The formatter must lay a command out under the grammar
    /// of the command it *is*, not the one it is spelled as.
    ///
    /// An `ArgRole::Body` argument is expanded onto its own lines, so a
    /// rebound body-bearing command resolved by spelling would be formatted
    /// under a grammar it no longer has.
    ///
    /// tclsh oracle (8.6.16 and 9.0.4, byte-identical): `interp alias {} maybe
    /// {} if` makes `maybe` run `if`; `rename if maybe` moves it and leaves
    /// `if` gone; a top-level `proc if …` takes the name over.
    fn body_was_expanded(src: &str) -> bool {
        fmt(src).contains("{\n    puts a\n}")
    }

    #[test]
    fn formatting_follows_an_aliased_body_command() {
        assert!(body_was_expanded(
            "interp alias {} maybe {} if\nmaybe {$x} {puts a}\n"
        ));
        // The `::`-qualified spelling of the alias classifies alike.
        assert!(body_was_expanded(
            "interp alias {} maybe {} if\n::maybe {$x} {puts a}\n"
        ));
        // Guard: an unbound `maybe` has no body argument, so its braced word
        // is left exactly as written.
        assert!(!body_was_expanded("set y 1\nmaybe {$x} {puts a}\n"));
    }

    #[test]
    fn formatting_follows_a_renamed_body_command() {
        assert!(body_was_expanded("rename if maybe\nmaybe {$x} {puts a}\n"));
        // The old spelling is gone from the rename onwards.
        assert!(
            !body_was_expanded("rename if maybe\nif {$x} {puts a}\n"),
            "a renamed-away `if` must not keep the built-in's layout"
        );
    }

    #[test]
    fn formatting_abstains_for_a_builtin_shadowed_by_a_user_proc() {
        assert!(
            !body_was_expanded("proc if {c b} { return 1 }\nif {$x} {puts a}\n"),
            "a user `proc if` takes the name over; no registry layout applies"
        );
        // Guard: the unshadowed built-in still expands.
        assert!(body_was_expanded("set y 1\nif {$x} {puts a}\n"));
    }

    #[test]
    fn formatting_abstains_for_a_dynamic_binding() {
        assert!(
            !body_was_expanded("rename $old maybe\nmaybe {$x} {puts a}\n"),
            "a dynamic rename must not give `maybe` a body layout"
        );
        assert!(
            body_was_expanded("rename $old maybe\nif {$x} {puts a}\n"),
            "a dynamic rename must not take `if`'s layout away either"
        );
    }

    /// The clause-list layout is head-driven too: `switch`'s pattern/body
    /// pairs are laid out by [`format_case_list_body`], which the reconstruction
    /// picks by the *resolved* head.
    #[test]
    fn formatting_follows_an_aliased_clause_list_command() {
        let formatted = fmt("interp alias {} pick {} switch\npick $v {a {puts 1} b {puts 2}}\n");
        assert!(
            formatted.contains("a {\n        puts 1\n    }"),
            "an alias of `switch` must get the clause-list layout; got:\n{formatted}"
        );
        // Guard: an unbound `pick` keeps its braced word verbatim.
        let formatted = fmt("set y 1\npick $v {a {puts 1} b {puts 2}}\n");
        assert!(!formatted.contains("a {\n        puts 1\n    }"));
    }

    fn case_list_action_was_expanded(formatted: &str, marker: &str) -> bool {
        formatted.contains(&format!("{{\n        puts {marker}\n    }}"))
    }

    #[test]
    fn case_list_alias_formatting_uses_the_binding_at_each_call_position() {
        let source = concat!(
            "pick subject {default {puts before_alias}}\n",
            "interp alias {} pick {} switch\n",
            "pick subject {default {puts through_alias}}\n",
            "interp alias {} pick {}\n",
            "pick subject {default {puts deleted_alias}}\n",
            "interp alias {} pick {} puts\n",
            "pick subject {default {puts rebound_other}}\n",
            "interp alias {} pick {} switch\n",
            "pick subject {default {puts rebound_switch}}\n",
        );
        let formatted = fmt(source);

        for opaque in ["before_alias", "deleted_alias", "rebound_other"] {
            assert!(
                formatted.contains(&format!("default {{puts {opaque}}}")),
                "binding state at {opaque} was applied retroactively:\n{formatted}"
            );
            assert!(!case_list_action_was_expanded(&formatted, opaque));
        }
        for active in ["through_alias", "rebound_switch"] {
            assert!(
                case_list_action_was_expanded(&formatted, active),
                "live switch alias did not format {active}:\n{formatted}"
            );
        }
        assert_eq!(fmt(&formatted), formatted);
    }

    #[test]
    fn case_list_rename_formatting_uses_the_binding_at_each_call_position() {
        let source = concat!(
            "pick subject {default {puts before_rename}}\n",
            "rename switch pick\n",
            "pick subject {default {puts after_rename}}\n",
            "rename pick {}\n",
            "pick subject {default {puts after_delete}}\n",
        );
        let formatted = fmt(source);
        for opaque in ["before_rename", "after_delete"] {
            assert!(formatted.contains(&format!("default {{puts {opaque}}}")));
            assert!(!case_list_action_was_expanded(&formatted, opaque));
        }
        assert!(case_list_action_was_expanded(&formatted, "after_rename"));
        assert_eq!(fmt(&formatted), formatted);
    }

    #[test]
    fn rooted_and_namespaced_case_list_aliases_format_at_their_positions() {
        let source = concat!(
            "::root_pick subject {default {puts before_root}}\n",
            "::case_alias::pick subject {default {puts before_namespace}}\n",
            "namespace eval ::case_alias {}\n",
            "interp alias {} ::root_pick {} ::switch\n",
            "interp alias {} ::case_alias::pick {} switch\n",
            "::root_pick subject {default {puts rooted_alias}}\n",
            "::case_alias::pick subject {default {puts namespaced_alias}}\n",
        );
        let formatted = fmt(source);
        for opaque in ["before_root", "before_namespace"] {
            assert!(formatted.contains(&format!("default {{puts {opaque}}}")));
            assert!(!case_list_action_was_expanded(&formatted, opaque));
        }
        for active in ["rooted_alias", "namespaced_alias"] {
            assert!(
                case_list_action_was_expanded(&formatted, active),
                "qualified switch alias did not format {active}:\n{formatted}"
            );
        }
        assert_eq!(fmt(&formatted), formatted);
    }
}
