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

//! The formatter configuration built from `tclLsp.formatting`.

use tcl_lsp_core::formatting as core_formatting;

/// Build a [`core_formatting::FormatterConfig`] from the resolved
/// `tclLsp.formatting` settings object, then apply the request's LSP
/// `FormattingOptions` (`tabSize` / `insertSpaces`) as the indentation override
/// (the editor's per-request indentation wins, per the LSP contract). Applies
/// the formatter field mapping so editor- and
/// config-file-set formatter options actually take effect.
/// Apply the `tclLsp.formatting.*` JSON object (camelCase keys) onto `cfg`,
/// coercing each key per its type. Split out of [`formatter_config_from`] so
/// the per-key mapping stays a single flat block without tripping the
/// `too_many_lines` lint on the caller.
pub(in super::super) fn apply_formatting_object(
    obj: &serde_json::Map<String, serde_json::Value>,
    cfg: &mut core_formatting::FormatterConfig,
) {
    use core_formatting::IndentStyle;
    let as_usize = |v: &serde_json::Value| v.as_u64().map(|n| usize::try_from(n).unwrap_or(0));
    if let Some(n) = obj.get("indentSize").and_then(as_usize) {
        cfg.indent_size = n.max(1);
    }
    if let Some(s) = obj.get("indentStyle").and_then(serde_json::Value::as_str) {
        cfg.indent_style = match s.to_ascii_lowercase().as_str() {
            "tabs" | "tab" => IndentStyle::Tabs,
            _ => IndentStyle::Spaces,
        };
    }
    if let Some(n) = obj.get("continuationIndent").and_then(as_usize) {
        cfg.continuation_indent = n;
    }
    // `braceStyle` is accepted but only K&R is implemented, so it stays the
    // default (a no-op rather than an error).
    if let Some(b) = obj
        .get("spaceBetweenBraces")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.space_between_braces = b;
    }
    // Both `maxLineLength` and the legacy `lineLength` map to the hard limit.
    if let Some(n) = obj
        .get("maxLineLength")
        .or_else(|| obj.get("lineLength"))
        .and_then(as_usize)
    {
        cfg.max_line_length = n.max(1);
    }
    if let Some(n) = obj.get("goalLineLength").and_then(as_usize) {
        cfg.goal_line_length = n.max(1);
    }
    if let Some(b) = obj
        .get("spaceAfterCommentHash")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.space_after_comment_hash = b;
    }
    if let Some(b) = obj
        .get("trimTrailingWhitespace")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.trim_trailing_whitespace = b;
    }
    if let Some(b) = obj
        .get("enforceBracedVariables")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.enforce_braced_variables = b;
    }
    if let Some(s) = obj.get("lineEnding").and_then(serde_json::Value::as_str) {
        cfg.line_ending = match s.to_ascii_lowercase().as_str() {
            "crlf" => "\r\n".to_owned(),
            "cr" => "\r".to_owned(),
            "lf" => "\n".to_owned(),
            // `auto` (the default) is resolved against the document being
            // formatted by `FormatterConfig::resolved_line_ending`; any other
            // value is passed through as a literal terminator.
            other => other.to_owned(),
        };
    }
    if let Some(b) = obj
        .get("ensureFinalNewline")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.ensure_final_newline = b;
    }
    if let Some(b) = obj
        .get("expandSingleLineBodies")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.expand_single_line_bodies = b;
    }
    // These four were shipped by every editor but never mapped here, so
    // toggling them did nothing. `minBodyCommandsForExpansion`
    // and `replaceSemicolonsWithNewlines` are engine-consumed;
    // `enforceBracedExpr` / `alignCommentsToCode` are carried through so the
    // resolved config round-trips (and so they take effect once the engine
    // consumes them).
    if let Some(n) = obj.get("minBodyCommandsForExpansion").and_then(as_usize) {
        cfg.min_body_commands_for_expansion = n.max(1);
    }
    if let Some(b) = obj
        .get("replaceSemicolonsWithNewlines")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.replace_semicolons_with_newlines = b;
    }
    if let Some(b) = obj
        .get("enforceBracedExpr")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.enforce_braced_expr = b;
    }
    if let Some(b) = obj
        .get("alignCommentsToCode")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.align_comments_to_code = b;
    }
    apply_keyword_formatting(obj, cfg);
    if let Some(n) = obj.get("blankLinesBetweenProcs").and_then(as_usize) {
        cfg.blank_lines_between_procs = n;
    }
    if let Some(n) = obj.get("blankLinesBetweenBlocks").and_then(as_usize) {
        cfg.blank_lines_between_blocks = n;
    }
    if let Some(n) = obj.get("maxConsecutiveBlankLines").and_then(as_usize) {
        cfg.max_consecutive_blank_lines = n;
    }
    apply_docstring_formatting(obj, cfg);
}

/// Read the keyword-normalisation `tclLsp.formatting.*` settings into `cfg`
/// (`expandAbbreviations`, `booleanForm`).
fn apply_keyword_formatting(
    obj: &serde_json::Map<String, serde_json::Value>,
    cfg: &mut core_formatting::FormatterConfig,
) {
    if let Some(b) = obj
        .get("expandAbbreviations")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.expand_abbreviations = b;
    }
    if let Some(v) = obj.get("booleanForm").and_then(serde_json::Value::as_str) {
        cfg.boolean_form = core_formatting::config::BooleanForm::parse(v);
    }
}

/// Read the `docstring*` `tclLsp.formatting.*` settings into `cfg`.
/// `docstringTagStyle` / `docstringDecoration*` are consumed by
/// `generate_stub_for_proc` (the docstring-stub content); `docstringStyle`
/// (placement) is round-tripped here for config-consumer parity, but the
/// `code_action` handler resolves it independently via
/// [`Backend::resolved_docstring_style`], which is the actual consumer —
/// code actions run off the resolved settings object directly
/// rather than this `FormatterConfig`. Split out of [`apply_formatting_object`]
/// to keep each per-key block under the `too_many_lines` lint.
fn apply_docstring_formatting(
    obj: &serde_json::Map<String, serde_json::Value>,
    cfg: &mut core_formatting::FormatterConfig,
) {
    if let Some(s) = obj
        .get("docstringStyle")
        .and_then(serde_json::Value::as_str)
    {
        cfg.docstring_style = core_formatting::DocstringStyle::parse(s);
    }
    if let Some(s) = obj
        .get("docstringTagStyle")
        .and_then(serde_json::Value::as_str)
    {
        cfg.docstring_tag_style = match s.to_ascii_lowercase().as_str() {
            "plain" => core_formatting::DocstringTagStyle::Plain,
            "none" => core_formatting::DocstringTagStyle::None,
            _ => core_formatting::DocstringTagStyle::Doxygen,
        };
    }
    if let Some(b) = obj
        .get("docstringDecoration")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.docstring_decoration = b;
    }
    if let Some(c) = obj
        .get("docstringDecorationChar")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| s.chars().next())
    {
        cfg.docstring_decoration_char = c;
    }
    if let Some(n) = obj
        .get("docstringDecorationWidth")
        .and_then(|v| v.as_u64().map(|n| usize::try_from(n).unwrap_or(0)))
    {
        cfg.docstring_decoration_width = n;
    }
}

pub(in super::super) fn formatter_config_from(
    formatting: &serde_json::Value,
    options: &tower_lsp_server::ls_types::FormattingOptions,
    dialect: &'static tcl_dialect::DialectProfile,
) -> core_formatting::FormatterConfig {
    use core_formatting::IndentStyle;
    // The document's dialect, as one resolved profile: it
    // supplies the lexer preset — so an `.irul` file's `if {expr}{body}`
    // (`}{` valid in TMM) is parsed and re-emitted as `} {` rather than left
    // unchanged by the stock-Tcl lexer — the release its rewrite candidates
    // are filtered against, and the forward range a rewrite must stay correct
    // across.
    let mut cfg = core_formatting::FormatterConfig::for_profile(dialect);
    if let Some(obj) = formatting.as_object() {
        apply_formatting_object(obj, &mut cfg);
    }
    // LSP `FormattingOptions` indentation overrides the config (per contract);
    // a real editor always sends `tabSize >= 1`, so a degenerate 0 falls back to
    // the configured indent size rather than clamping it away.
    if options.tab_size >= 1 {
        cfg.indent_size = usize::try_from(options.tab_size).unwrap_or(cfg.indent_size);
        cfg.indent_style = if options.insert_spaces {
            IndentStyle::Spaces
        } else {
            IndentStyle::Tabs
        };
    }
    cfg.indent_size = cfg.indent_size.max(1);
    cfg
}
