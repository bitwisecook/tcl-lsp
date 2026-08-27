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

//! Formatting provider.
//!
//! [`formatting`] produces a single full-document `TextEdit`
//! by running the token-aware [`engine::format_tcl`] with a
//! default [`FormatterConfig`].  The engine is more than a
//! whitespace pass — it normalises:
//!
//! * Brace placement (K&R: `{` kept on the command line, the
//!   matching `}` re-anchored).
//! * Indentation tracking brace nesting (configurable width),
//!   including continuation lines inside an open brace.
//! * Trailing-whitespace trimming and tab-to-space conversion.
//! * Blank-line policy (collapsing runs, blank lines around
//!   procs) and a single trailing newline.
//! * Comment normalisation, switch-body formatting, long-line
//!   backslash splitting, and `&&` / `||` expression wrapping.
//!
//! Formatting never reorders or rewrites command words — only
//! their layout changes.
//!
//! [`range_formatting`] re-normalises just the requested line
//! slice (extended to whole lines), computing the brace depth
//! at the slice start from the source prefix above it, so
//! `textDocument/rangeFormatting` ("format selection") leaves
//! the rest of the document untouched.
//!
//! Docstring reflow and the expr-brace knobs are not
//! implemented.

pub mod config;
pub mod docstring;
pub mod engine;
pub(crate) mod keywords;

pub use config::{
    DocstringStyle, DocstringTagStyle, FormatterConfig, IndentStyle, LINE_ENDING_AUTO,
    detect_line_ending,
};
pub use docstring::{
    DocstringInfo, ParamDoc, generate_stub_for_proc, parse_docstring, render_comment_block,
    resolve_tag_style,
};
pub use engine::format_tcl;

use crate::definition::LspRange;
use crate::rename::TextEdit;
use tcl_lexer::LineIndex;
use tcl_registry::CommandRegistry;

/// Compute formatting edits for the entire document.
///
/// Runs the token-aware [`engine::format_tcl`] with default
/// (F5 iRules) settings and returns a single `TextEdit` that
/// replaces the whole document with its normalised form, or an
/// empty `Vec` when the document is already normalised.
#[must_use]
pub fn formatting(source: &str, registry: &CommandRegistry) -> Vec<TextEdit> {
    formatting_with(source, &FormatterConfig::default(), registry)
}

/// Compute whole-document formatting edits with an explicit
/// [`FormatterConfig`].
///
/// Identical to [`formatting`] but honours a caller-supplied
/// config so the server can map an LSP request's `tabSize` /
/// `insertSpaces` onto the formatter's indentation (an explicit
/// client `FormattingOptions` overrides the server default by LSP
/// contract).
#[must_use]
pub fn formatting_with(
    source: &str,
    config: &FormatterConfig,
    registry: &CommandRegistry,
) -> Vec<TextEdit> {
    let formatted = engine::format_tcl(source, config, registry);
    if formatted == source {
        return Vec::new();
    }
    // The replace range is expressed in the **client's** coordinates, so it
    // must be built on the client's EOL model (`\n`, `\r\n`, *and* a lone
    // `\r`), not the lexer/CST `\n`-only one.  With `LineIndex::new` an
    // old-Mac document reported an end position on line 0 and the client
    // spliced the formatted text over only the first line, duplicating the
    // rest of the file (and a mixed document lost its tail the same way).
    let line_index = LineIndex::new_lsp(source);
    let end_pos = line_index.position_at_utf16(u32::try_from(source.len()).unwrap_or(0), source);
    vec![TextEdit {
        range: LspRange {
            start_line: 0,
            start_character: 0,
            end_line: end_pos.line,
            end_character: end_pos.character.get(),
        },
        new_text: formatted,
    }]
}

/// Compute formatting edits for a range within the
/// document.
///
/// True range-aware formatting: only the line slice
/// `[range.start_line, range.end_line]` (extended to whole
/// lines) is re-normalised, with the brace depth at the
/// start of the slice computed from the source prefix above
/// it.  Emits a single `TextEdit` that replaces the slice
/// with its formatted form, or an empty `Vec` when the
/// slice is already normalised.
///
/// Range-formatting only touches the line range requested;
/// edits outside it are left untouched.  Editors that
/// invoke `textDocument/rangeFormatting` (eg. `format
/// selection`) only need the selected slice to change.
#[must_use]
pub fn range_formatting(
    source: &str,
    range: LspRange,
    config: &FormatterConfig,
    registry: &CommandRegistry,
) -> Vec<TextEdit> {
    // `range` is in the client's coordinates, so the slice must be cut on the
    // client's EOL model.  Normalising every lone `\r` to `\n` makes a plain
    // `split('\n')` agree with it line-for-line (a CRLF still breaks at its
    // `\n`, leaving the `\r` at the end of the line the engine trims), and the
    // rewrite is byte-length preserving so offsets into `normalised` index the
    // raw `source` unchanged.
    let normalised = tcl_lexer::normalise_lone_cr(source);
    let lines: Vec<&str> = normalised.split('\n').collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let line_count = u32::try_from(lines.len()).unwrap_or(u32::MAX);
    let start_line = range.start_line.min(line_count.saturating_sub(1));
    let end_line = range
        .end_line
        .min(line_count.saturating_sub(1))
        .max(start_line);

    // Brace depth at the start of `start_line` — count
    // running `{` / `}` over every line before it.  This is the
    // nesting level the engine would indent the slice's first
    // command at (`config.make_indent(level)`).
    let mut prefix_depth: i32 = 0;
    for prior in lines.iter().take(start_line as usize) {
        prefix_depth = (prefix_depth + brace_delta(prior)).max(0);
    }

    // Slice of lines we re-format, run through the same
    // token-aware engine the full-document path uses
    // ([`engine::format_body`]) at the slice's brace depth, so
    // range formatting and full-document formatting share every
    // rule (comment normalisation, switch bodies, line wrapping,
    // configurable indent width, …).
    let slice_end = (end_line as usize) + 1;
    let slice_lines: Vec<&str> = lines[start_line as usize..slice_end].to_vec();
    let slice_text = slice_lines.join("\n");
    let depth = usize::try_from(prefix_depth).unwrap_or(0);
    // The document's own line ending (or the configured one) — resolved
    // against the whole document, not the slice, so a one-line selection in a
    // CRLF file is not re-emitted with `\n`.
    let line_ending = config.resolved_line_ending(source);
    // The identity facts come from the **whole document**, not the slice: a
    // `rename` above the selection still governs what the selected commands
    // are (issue #1275).
    let identities = tcl_compiler::realm::document_realm_bindings_with_config(
        source,
        config.lexer_config(),
        registry,
    );
    let slice_source_offset = LineIndex::new(&normalised).line_start(start_line);
    let formatted_slice = finalise_slice(
        &engine::format_body(
            &slice_text,
            slice_source_offset,
            config,
            registry,
            &identities,
            depth,
        ),
        config,
        line_ending,
    );
    // Skip no-op edits: compare the formatted slice against the slice
    // text *as it currently is in the document* (raw, untrimmed) plus
    // the trailing newline the replacement range carries.  Finalising
    // the original here would hide trailing-whitespace-only changes.
    // The comparison uses the *raw* bytes of the same span so an already
    // formatted CRLF / old-Mac slice compares equal instead of producing a
    // spurious edit that rewrites its own terminators.
    let raw_slice = raw_span(source, &normalised, start_line, end_line, line_count);
    let original_with_nl = if raw_slice.ends_with('\n') || raw_slice.ends_with('\r') {
        raw_slice.to_owned()
    } else {
        format!("{raw_slice}{line_ending}")
    };
    if formatted_slice == original_with_nl {
        return Vec::new();
    }

    // Replacement range covers the full slice, line-anchored
    // (column 0 of `start_line` to column 0 of the line
    // *after* `end_line`).  When `end_line` is the last line
    // of the document, anchor the end at the post-final-char
    // position so editors interpret the edit correctly.
    let edit_range = if (end_line + 1) < line_count {
        LspRange {
            start_line,
            start_character: 0,
            end_line: end_line + 1,
            end_character: 0,
        }
    } else {
        // `end_line` is the document's last line, so the slice runs
        // to EOF.  Derive the end column via `LineIndex` (the same
        // position encoding the full-document path uses) so the
        // range is correct for non-ASCII text rather than counting
        // raw `char`s — on the client's EOL model, as the range is
        // sent back to the client.
        let line_index = LineIndex::new_lsp(source);
        let end_pos =
            line_index.position_at_utf16(u32::try_from(source.len()).unwrap_or(u32::MAX), source);
        LspRange {
            start_line,
            start_character: 0,
            end_line: end_pos.line,
            end_character: end_pos.character.get(),
        }
    };
    vec![TextEdit {
        range: edit_range,
        new_text: formatted_slice,
    }]
}

/// The raw bytes of the document span the formatted slice replaces.
///
/// `normalised` is `source` with every lone `\r` rewritten to `\n`, so it has
/// the same length and the same byte offsets — the span is located on the
/// normalised text (whose `\n`s match the client's line model) and then cut
/// out of `source`.
fn raw_span<'a>(
    source: &'a str,
    normalised: &str,
    start_line: u32,
    end_line: u32,
    line_count: u32,
) -> &'a str {
    let index = LineIndex::new(normalised);
    let start = index.line_start(start_line) as usize;
    let end = if end_line + 1 < line_count {
        index.line_start(end_line + 1) as usize
    } else {
        source.len()
    };
    source.get(start..end).unwrap_or(source)
}

/// Apply the engine's tail post-processing to a formatted slice:
/// trailing-whitespace trimming, a single trailing newline (the
/// range edit replaces through the start of the following line), and
/// the resolved `line_ending`.  Mirrors the tail of
/// [`engine::format_tcl`] minus the document-level final-newline
/// policy.
fn finalise_slice(text: &str, config: &FormatterConfig, line_ending: &str) -> String {
    let mut out = if config.trim_trailing_whitespace {
        // Brace/quote-aware trim so a multi-line string literal's interior is
        // preserved.
        engine::trim_trailing_ws_preserving_literals(text)
    } else {
        text.to_owned()
    };
    if !out.ends_with('\n') {
        out.push('\n');
    }
    if line_ending != "\n" {
        out = out.replace('\n', line_ending);
    }
    out
}

/// Net brace delta for a logical line.  Ignores braces
/// inside `"..."` strings and inside the body of a brace-
/// literal that fully nests in the line (e.g. `proc f {}
/// {body}` is depth-neutral).  Conservative — we count
/// every `{` / `}` outside double-quoted strings.
fn brace_delta(line: &str) -> i32 {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;
    for c in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if in_comment {
            // Tcl comments run to end of line.
            continue;
        }
        if in_string {
            if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '#' if depth == 0 && line.trim_start().starts_with('#') => {
                in_comment = true;
            }
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range_fmt(source: &str, range: LspRange) -> Vec<TextEdit> {
        let registry = tcl_registry::CommandRegistry::build_default();
        range_formatting(source, range, &FormatterConfig::default(), &registry)
    }

    #[test]
    fn range_formatting_honours_configurable_indent_width() {
        // The engine (not the old hardcoded 4-space formatter) drives
        // range formatting now, so a non-default `indent_size` is
        // respected — proving range and full-document formatting share
        // the same `FormatterConfig`-aware engine path.
        let registry = tcl_registry::CommandRegistry::build_default();
        let config = FormatterConfig {
            indent_size: 2,
            ..FormatterConfig::default()
        };
        let src = "proc foo {} {\nset x 1\n}\n";
        let edits = range_formatting(
            src,
            LspRange {
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 100,
            },
            &config,
            &registry,
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert!(
            edits[0].new_text.starts_with("  set x 1"),
            "expected 2-space indent; got {:?}",
            edits[0].new_text,
        );
    }

    #[test]
    fn already_formatted_returns_no_edits() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "proc foo {} {\n    set x 1\n}\n";
        assert!(
            formatting(src, &registry).is_empty(),
            "{:?}",
            formatting(src, &registry)
        );
    }

    #[test]
    fn brace_delta_ignores_string_contents() {
        assert_eq!(brace_delta(r#"set x "}{"; # comment"#), 0);
    }

    #[test]
    fn brace_delta_counts_nested() {
        assert_eq!(brace_delta("foo { bar { baz"), 2);
        assert_eq!(brace_delta("} }"), -2);
    }

    #[test]
    fn range_formatting_emits_edit_for_dirty_range() {
        let src = "set x 1   \n";
        let edits = range_fmt(
            src,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 5,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert!(edits[0].new_text.contains("set x 1"), "{edits:?}");
        // Trailing whitespace stripped.
        assert!(!edits[0].new_text.contains("   "), "{edits:?}");
    }

    #[test]
    fn range_formatting_no_edits_when_slice_is_clean() {
        // Whole document is already formatted — range over a
        // clean slice should emit no edits.
        let src = "proc foo {} {\n    set x 1\n}\n";
        let edits = range_fmt(
            src,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 2,
                end_character: 0,
            },
        );
        assert!(edits.is_empty(), "{edits:?}");
    }

    #[test]
    fn range_formatting_preserves_brace_depth_from_prefix() {
        // Inside a proc body, the lines should be indented
        // 4 spaces.  Format only line 1 (the body's `set x`
        // line) — the formatter must pick up `depth = 1`
        // from the prefix walk.
        let src = "proc foo {} {\nset x 1\n}\n";
        let edits = range_fmt(
            src,
            LspRange {
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 100,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        // Inside the proc body — should be indented 4 spaces.
        assert!(
            edits[0].new_text.starts_with("    set x 1"),
            "expected indented set; got {:?}",
            edits[0].new_text,
        );
    }

    #[test]
    fn range_formatting_resolves_aliases_at_the_slice_source_offset() {
        let src = concat!(
            "interp alias {} pick {} switch\n",
            "pick subject {default {puts    through_alias}}\n",
        );
        let edits = range_fmt(
            src,
            LspRange {
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 100,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert!(
            edits[0]
                .new_text
                .contains("default {\n        puts through_alias\n    }"),
            "the alias declaration before the selected slice was ignored: {edits:?}"
        );
    }

    #[test]
    fn range_formatting_clamps_end_at_eof() {
        // Source has 2 lines; request a range whose end
        // extends past EOF.  Should still emit one valid
        // edit anchored at the final line's end.
        let src = "set x 1   \nset y 2\n";
        let edits = range_fmt(
            src,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 99,
                end_character: 0,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
    }
}
