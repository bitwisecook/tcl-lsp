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

#[cfg(test)]
use tcl_lexer::TokenType;
use tcl_lexer::{Lexer, SourceMap, Token};
use tcl_registry::{ArgRole, CaseListSpec, CommandRegistry, Traits};

use super::config::FormatterConfig;
#[cfg(test)]
use tcl_syntax::word_rules::WordValueRules;

mod argument_roles;
mod command_parsing;
mod command_reconstruction;
mod line_wrapping;
mod nested_bodies;
mod range_formatting;
mod word_source;

use argument_roles::identify_body_args;
use command_parsing::parse_commands;
#[cfg(test)]
use command_reconstruction::render_lambda_literal_arg;
use command_reconstruction::{case_list_body_index, decoded_lambda, reconstruct_command};
#[cfg(test)]
use command_reconstruction::{count_body_commands, keyword_rewrites_for};
use line_wrapping::split_commented_code;
#[cfg(test)]
use line_wrapping::wrap_braced_expr;
use nested_bodies::{format_lambda_body_argument, format_plain_body_argument};
pub(crate) use range_formatting::formatting_range_region;
use word_source::normalise_backslash_newline;
#[cfg(test)]
use word_source::{
    normalise_original_param_list, original_variable_source, same_variable_word_components,
};
#[cfg(test)]
use word_source::{reconstruct_arg, render_param_list_arg};

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
            original: super::analysis_layout::AnalysisFormattingLayout::capture(source, &analysis),
            analysis: Some(analysis),
            input: input.clone(),
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_variable_reconstruction_tests;

#[cfg(test)]
mod decoded_logical_lambda_phase_tests;
