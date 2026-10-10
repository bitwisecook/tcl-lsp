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

//! Source-level text diagnostics — the line/text checks, and the orchestrator
//! every one of them reaches the editor through.
//!
//! These style codes are *source-text* checks: they read the
//! raw document, not the lexer / segmenter / CST, because the
//! questions they answer ("is this line too long", "does this line
//! have trailing whitespace", "what line endings does the file
//! use", "is this a backslash-continued comment") are inherently
//! textual and have no structural Tcl representation.  They operate
//! directly on the source split into lines.
//!
//! Checks:
//!
//! * **W111** — line length.
//! * **W112** — trailing whitespace, with a remove-whitespace
//!   quick-fix.
//! * **W115** — backslash-newline in a comment swallows the next
//!   line, with a convert-to-per-line-comments quick-fix.
//! * **W118** — inconsistent line endings, a single file-level
//!   diagnostic.
//!
//! [`style_diagnostics`] additionally runs the byte-backed integrity checks in
//! [`crate::source_decode`] — **W107** (the source is not valid UTF-8) and
//! **W109** (the source is not UTF-8 text at all). **W305** is a Unicode-text
//! analyser diagnostic instead, so analyser-only consumers such as MCP see the
//! same whole-source security finding without depending on an LSP style pass.
//!
//! The orchestrator ([`style_diagnostics`]) wires into the native
//! server's `publish_analyser_diagnostics` so these source-style
//! codes reach the editor alongside the analyser / compiler-check /
//! optimiser sets.
//!
//! **Range convention.**  Ranges are end-exclusive, matching the LSP `Range`
//! contract (`end` is one-past the last covered character) and the structural
//! `tcl-lsp-core` providers — the native server passes these columns straight
//! into the wire `Range`.  An inclusive `length - 1` end collapsed a
//! single-column span to an empty range and left the last trailing-whitespace
//! character outside the W112 remove-fix (issue 186).
//! Character columns are UTF-16 code units, matching the LSP convention.
//!
//! W112 / W115 corrections are carried on [`StyleDiagnostic::fix`].
//! The W115 source action obtains its comment facts from the same actual
//! analysis facade as diagnostics. The W111 line length is configurable
//! (`tclLsp.style.lineLength`, resolved per folder by the server and
//! passed into [`style_diagnostics`]); the expected line ending is
//! still the default `\n`.  The pass applies no policy: every check's
//! findings are returned, and `# noqa`, `# tcl-lsp: disable` and the
//! `tclLsp.diagnostics.<CODE>` set reach them through
//! [`crate::diagnostic_policy::apply`] like every other code.

use tcl_compiler::analyser::AnalysisResult;
use tcl_core_types::DiagCode;

use crate::definition::{LspRange, utf16_len};

/// Severity of a source-text diagnostic. W111 / W115 / W107 / W109 are
/// warnings, and W112 / W118 are hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleSeverity {
    /// LSP `Warning`.
    Warning,
    /// LSP `Hint`.
    Hint,
}

/// A quick-fix attached to a style diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleFix {
    /// Document range the fix replaces.
    pub range: LspRange,
    /// Replacement text.
    pub new_text: String,
    /// Human-readable description (the code-action title).
    pub description: String,
}

/// One source-style diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleDiagnostic {
    /// Source range the diagnostic anchors at (see the module-level
    /// "Range convention" note — `end` is the last affected char).
    pub range: LspRange,
    /// Diagnostic message.
    pub message: String,
    /// Severity.
    pub severity: StyleSeverity,
    /// Diagnostic code (`W111` / `W112` / `W115` / `W118`, or the byte-integrity
    /// pass's `W107` / `W109`).
    pub code: DiagCode,
    /// Optional quick-fix (W112 / W115 carry one; W111 / W118 do
    /// not).
    pub fix: Option<StyleFix>,
}

/// Default maximum line length.
pub const DEFAULT_LINE_LENGTH: usize = 120;

/// Default expected line ending.
pub const DEFAULT_LINE_ENDING: &str = "\n";

/// W111: flag lines exceeding `max_length` characters.
///
/// The length is the line's codepoint count *after* stripping a
/// trailing `\r` so CRLF endings don't inflate the count.
#[must_use]
pub fn check_line_length(source: &str, max_length: usize) -> Vec<StyleDiagnostic> {
    let mut out = Vec::new();
    for (lineno, line) in source.split('\n').enumerate() {
        let line = line.trim_end_matches('\r');
        let length = line.chars().count();
        if length > max_length {
            let lineno = u32::try_from(lineno).expect("line index fits u32");
            // LSP ranges are end-exclusive: the end column is one-past the last
            // character, i.e. the line's full UTF-16 length (issue 186).
            let end_char = utf16_len(line);
            out.push(StyleDiagnostic {
                range: LspRange {
                    start_line: lineno,
                    start_character: 0,
                    end_line: lineno,
                    end_character: end_char,
                },
                message: format!("Line exceeds {max_length} characters ({length} characters)"),
                severity: StyleSeverity::Warning,
                code: DiagCode::W111,
                fix: None,
            });
        }
    }
    out
}

/// W112: flag trailing whitespace, with a remove-whitespace fix.
///
/// A trailing `\r` is stripped first so CRLF endings aren't
/// themselves flagged.
#[must_use]
pub fn check_trailing_whitespace(source: &str) -> Vec<StyleDiagnostic> {
    let mut out = Vec::new();
    for (lineno, line) in source.split('\n').enumerate() {
        let line_no_cr = line.trim_end_matches('\r');
        let stripped = line_no_cr.trim_end();
        if stripped.len() < line_no_cr.len() {
            let lineno = u32::try_from(lineno).expect("line index fits u32");
            let ws_start = utf16_len(stripped);
            let ws_end = utf16_len(line_no_cr);
            // End-exclusive: the range/fix must reach one-past the final
            // whitespace char (`ws_end`), or the last space is left uncovered
            // and the remove-whitespace fix leaves it behind (issue 186).
            let range = LspRange {
                start_line: lineno,
                start_character: ws_start,
                end_line: lineno,
                end_character: ws_end,
            };
            out.push(StyleDiagnostic {
                range,
                message: "Trailing whitespace".to_string(),
                severity: StyleSeverity::Hint,
                code: DiagCode::W112,
                fix: Some(StyleFix {
                    range,
                    new_text: String::new(),
                    description: "Remove trailing whitespace".to_string(),
                }),
            });
        }
    }
    out
}

/// Human-readable label for a line-ending byte sequence.
fn eol_label(ending: &str) -> String {
    match ending {
        "\n" => "LF".to_string(),
        "\r\n" => "CRLF".to_string(),
        "\r" => "CR".to_string(),
        other => format!("{other:?}"),
    }
}

/// W118: flag files whose line endings differ from `expected`.
///
/// Emits at most one file-level diagnostic anchored at `(0, 0)`.
#[must_use]
pub fn check_line_endings(source: &str, expected: &str) -> Vec<StyleDiagnostic> {
    let crlf = source.matches("\r\n").count();
    // Lone \r = total \r minus those that are part of \r\n.
    let cr = source.matches('\r').count() - crlf;
    let lf = source.matches('\n').count() - crlf;

    // Build the set of endings present, in insertion order (LF,
    // CRLF, CR) so the "Mixed line endings" message is stable.
    let mut present: Vec<(&str, usize)> = Vec::new();
    if lf > 0 {
        present.push(("\n", lf));
    }
    if crlf > 0 {
        present.push(("\r\n", crlf));
    }
    if cr > 0 {
        present.push(("\r", cr));
    }

    if present.is_empty() {
        return Vec::new();
    }

    let unexpected: Vec<(&str, usize)> = present
        .iter()
        .copied()
        .filter(|(k, _)| *k != expected)
        .collect();
    if unexpected.is_empty() {
        return Vec::new();
    }

    let expected_label = eol_label(expected);
    let message = if present.len() > 1 {
        let parts: Vec<String> = present
            .iter()
            .map(|(k, v)| format!("{} ({v})", eol_label(k)))
            .collect();
        format!(
            "Mixed line endings: {}; expected {expected_label}",
            parts.join(", ")
        )
    } else {
        let (actual_ending, count) = unexpected[0];
        let actual_label = eol_label(actual_ending);
        format!("File uses {actual_label} line endings ({count}); expected {expected_label}")
    };

    let zero = LspRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 0,
    };
    vec![StyleDiagnostic {
        range: zero,
        message,
        severity: StyleSeverity::Hint,
        code: DiagCode::W118,
        fix: None,
    }]
}

/// W115: flag backslash-newline continuation inside comments.
///
/// In Tcl, a `\` immediately
/// before a newline inside a comment silently swallows the next
/// line into the comment, which can hide live code.  The quick-fix
/// converts the continued comment into separate per-line `#`
/// comments.
#[must_use]
pub fn check_comment_continuation(source: &str) -> Vec<StyleDiagnostic> {
    check_comment_continuation_for_dialect(source, crate::profile_for_dialect("tcl9.0"))
}

/// Dialect-aware W115 detector. Comment position is a lexer/registry fact,
/// not a textual `#` prefix, so quoted and braced data never become comments.
#[must_use]
pub fn check_comment_continuation_for_dialect(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<StyleDiagnostic> {
    let profile = dialect;
    let comments = tcl_compiler::analyser::utils::script_comment_facts(
        source,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
        crate::registry_for_dialect_profile(profile),
    );
    comment_continuation_diagnostics(source, &comments)
}

/// Original physical comment facts from the caller's complete current analysis.
/// Conditional and reference-only schemas supply source syntax only. No
/// command dispatch, entered frame or rewrite equivalence follows from them.
#[must_use]
pub fn comment_facts_from_analysis(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<tcl_compiler::analyser::utils::ScriptCommentFact>> {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    tcl_compiler::analyser::utils::script_comment_facts_from_analysis(source, analysis)
}

/// W115 under the actual source image, full lexer policy and retained context.
/// Unavailable or stale source ownership yields no diagnostic or correction.
#[must_use]
pub fn check_comment_continuation_from_analysis(
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<StyleDiagnostic> {
    let Some(comments) = comment_facts_from_analysis(source, analysis) else {
        return Vec::new();
    };
    comment_continuation_diagnostics(source, &comments)
}

fn comment_continuation_diagnostics(
    source: &str,
    comments: &[tcl_compiler::analyser::utils::ScriptCommentFact],
) -> Vec<StyleDiagnostic> {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut out = Vec::new();

    let mut i = 0usize;
    while i < lines.len() {
        let Some(run_end) = comment_continuation_run_with_facts(&lines, comments, i) else {
            i += 1;
            continue;
        };
        let line = lines[i];
        let stripped = line.trim_start();

        // Found a comment with backslash continuation.  `indent`
        // is the leading-whitespace prefix; because `stripped`
        // drops a leading run, the byte arithmetic is exact.
        let indent = &line[..line.len() - stripped.len()];
        let block_start = i;
        let block_lines = &lines[i..run_end];
        let block_end = run_end - 1;

        // Build the replacement text: per-line comments.
        let mut fixed: Vec<String> = Vec::with_capacity(block_lines.len());
        let last_idx = block_lines.len() - 1;
        for (k, orig) in block_lines.iter().enumerate() {
            let raw = orig.trim_end_matches('\r');
            if k == 0 {
                // First line: remove the trailing backslash (1
                // ASCII byte), then strip trailing whitespace.
                fixed.push(raw[..raw.len() - 1].trim_end().to_string());
            } else {
                let mut content = raw.trim_start();
                if raw.ends_with('\\') && k < last_idx {
                    content = content[..content.len() - 1].trim_end();
                }
                if content.starts_with('#') {
                    // Already looks like a comment — keep indent
                    // only.
                    fixed.push(format!("{indent}{content}"));
                } else {
                    fixed.push(format!("{indent}# {content}"));
                }
            }
        }
        let new_text = fixed.join("\n");

        // Diagnostic range: from the start of the first line to one-past the
        // last content character of the last line (end-exclusive per LSP). A
        // trailing `\r` is excluded so a CRLF file's `\r` is neither counted
        // nor left dangling by the fix (issue 186).
        let end_line_text = lines[block_end].trim_end_matches('\r');
        let end_char = utf16_len(end_line_text);
        let range = LspRange {
            start_line: u32::try_from(block_start).expect("line index fits u32"),
            start_character: 0,
            end_line: u32::try_from(block_end).expect("line index fits u32"),
            end_character: end_char,
        };
        out.push(StyleDiagnostic {
            range,
            message: "Backslash-newline in comment silently swallows the next line".to_string(),
            severity: StyleSeverity::Warning,
            code: DiagCode::W115,
            fix: Some(StyleFix {
                range,
                new_text,
                description: "Convert to per-line comments".to_string(),
            }),
        });

        i = run_end;
    }
    out
}

/// Return the exclusive physical-line end of a backslash-continued Tcl
/// comment beginning at `start_line`.
///
/// A continuation marker is exact: a carriage return before the newline is
/// ignored, but ordinary trailing whitespace remains significant. Both W115
/// and its rewrite action consume this detector.
#[must_use]
pub fn comment_continuation_run(lines: &[&str], start_line: usize) -> Option<usize> {
    let first = *lines.get(start_line)?;
    first
        .trim_start()
        .trim_end_matches('\r')
        .ends_with('\\')
        .then_some(())?;
    let mut end = start_line + 1;
    while end < lines.len() && lines[end - 1].trim_end_matches('\r').ends_with('\\') {
        end += 1;
    }
    Some(end.min(lines.len()))
}

/// W115 run detector using lexer-confirmed physical comment lines.
#[must_use]
pub fn comment_continuation_run_with_facts(
    lines: &[&str],
    comment_facts: &[tcl_compiler::analyser::utils::ScriptCommentFact],
    start_line: usize,
) -> Option<usize> {
    let comment = comment_facts.iter().find(|fact| fact.line == start_line)?;
    let first_comment_line = comment.text.split('\n').next().unwrap_or_default();
    if !first_comment_line.trim_end_matches('\r').ends_with('\\') {
        return None;
    }
    let mut end = start_line + 1;
    while end < lines.len() && lines[end - 1].trim_end_matches('\r').ends_with('\\') {
        end += 1;
    }
    Some(end.min(lines.len()))
}

/// Run every source-text check and return the merged diagnostics.
///
/// No policy is applied here: `# noqa`, `# tcl-lsp: disable=…` and the
/// disabled set reach these codes through
/// [`crate::diagnostic_policy::apply`], where W107, W109 and W118 are the
/// whole-file codes an inline directive never reaches.
///
/// `decode` is the byte-level decoder's report when the caller can prove the
/// bytes belong to this exact text. `None` means only Unicode text is
/// available, so W107/W109 abstain rather than infer malformed bytes from a
/// valid character such as a literal `U+FFFD`.
#[must_use]
pub fn style_diagnostics(
    source: &str,
    line_length: usize,
    line_ending: &str,
    decode: Option<&crate::source_decode::DecodeReport>,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<StyleDiagnostic> {
    style_diagnostics_with_comment_check(source, line_length, line_ending, decode, |text| {
        check_comment_continuation_for_dialect(text, dialect)
    })
}

/// Source-style facts using the actual retained analysis for W115.
/// Report policy independently decides which facts are shown.
#[must_use]
pub fn style_facts_from_analysis(
    source: &str,
    line_length: usize,
    line_ending: &str,
    decode: Option<&crate::source_decode::DecodeReport>,
    analysis: &AnalysisResult,
) -> Vec<StyleDiagnostic> {
    style_diagnostics_with_comment_check(source, line_length, line_ending, decode, |text| {
        check_comment_continuation_from_analysis(text, analysis)
    })
}

/// Compatibility adapter for callers supplying an already resolved skip set.
/// The source facts still use the retained analysis; report consumers use
/// `style_facts_from_analysis` and apply policy once.
#[must_use]
pub fn style_diagnostics_from_analysis<
    SD: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
    I: std::hash::BuildHasher,
>(
    source: &str,
    line_length: usize,
    line_ending: &str,
    disabled: &std::collections::HashSet<String, SD>,
    suppressed: &std::collections::HashMap<i32, std::collections::HashSet<String, I>, H>,
    decode: Option<&crate::source_decode::DecodeReport>,
    analysis: &AnalysisResult,
) -> Vec<StyleDiagnostic> {
    style_facts_from_analysis(source, line_length, line_ending, decode, analysis)
        .into_iter()
        .filter(|diagnostic| {
            !disabled.contains("*")
                && !disabled.contains(diagnostic.code.as_str())
                && !tcl_compiler::analyser::line_suppressed(
                    diagnostic.code.as_str(),
                    i32::try_from(diagnostic.range.start_line).unwrap_or(i32::MAX),
                    suppressed,
                )
        })
        .collect()
}

fn style_diagnostics_with_comment_check(
    source: &str,
    line_length: usize,
    line_ending: &str,
    decode: Option<&crate::source_decode::DecodeReport>,
    comment_check: impl FnOnce(&str) -> Vec<StyleDiagnostic>,
) -> Vec<StyleDiagnostic> {
    // The line-oriented lints below split on `\n`, so a lone `\r` — a line
    // break to the editor and a command terminator to `tclsh` — must become
    // one first, or their line numbers drift from the client's (and from the
    // analyser's, whose `suppressed_lines` the policy step keys by).  W118 is
    // the one lint that must see the *real* terminators, so it keeps
    // `source`.
    let lines_source = tcl_lexer::normalise_lone_cr(source);

    let mut out = check_line_length(&lines_source, line_length);
    out.extend(check_trailing_whitespace(&lines_source));
    out.extend(comment_check(&lines_source));
    out.extend(check_line_endings(source, line_ending));

    // Source-text *integrity*.  These run on `source`, not
    // `lines_source`: a mis-decoded file's byte offsets must not be shifted by
    // a lone-CR rewrite.
    out.extend(crate::source_decode::encoding_integrity_diagnostics(
        source, decode,
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn no_disable() -> std::collections::HashSet<String> {
        std::collections::HashSet::new()
    }
    fn no_suppress() -> std::collections::HashMap<i32, std::collections::HashSet<String>> {
        std::collections::HashMap::new()
    }

    #[test]
    fn w111_flags_long_line() {
        let line = "x".repeat(125);
        let diags = check_line_length(&line, DEFAULT_LINE_LENGTH);
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.code, DiagCode::W111);
        assert_eq!(d.severity, StyleSeverity::Warning);
        assert_eq!(d.message, "Line exceeds 120 characters (125 characters)");
        assert_eq!(d.range.start_line, 0);
        assert_eq!(d.range.start_character, 0);
        assert_eq!(d.range.end_character, 125); // end-exclusive: one past the 125th char
        assert!(d.fix.is_none());
    }

    #[test]
    fn w111_ignores_trailing_cr() {
        // 120 chars + CRLF must not fire (length counts exclude \r).
        let src = format!("{}\r\nshort", "y".repeat(120));
        let diags = check_line_length(&src, DEFAULT_LINE_LENGTH);
        assert!(diags.is_empty());
    }

    #[test]
    fn w111_boundary_exactly_max_is_clean() {
        let line = "z".repeat(120);
        assert!(check_line_length(&line, DEFAULT_LINE_LENGTH).is_empty());
    }

    #[test]
    fn w112_flags_trailing_whitespace_with_fix() {
        let src = "set x 1   \nset y 2";
        let diags = check_trailing_whitespace(src);
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.code, DiagCode::W112);
        assert_eq!(d.severity, StyleSeverity::Hint);
        assert_eq!(d.range.start_line, 0);
        assert_eq!(d.range.start_character, 7);
        assert_eq!(d.range.end_character, 10); // end-exclusive: covers all 3 spaces
        let fix = d.fix.as_ref().expect("W112 carries a fix");
        assert_eq!(fix.new_text, "");
        assert_eq!(fix.range.start_character, 7);
        assert_eq!(fix.range.end_character, 10);
        assert_eq!(fix.description, "Remove trailing whitespace");
    }

    #[test]
    fn w112_range_uses_utf16_columns() {
        let diags = check_trailing_whitespace("set 😀  \n");
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.range.start_character, 6);
        assert_eq!(d.range.end_character, 8); // end-exclusive: covers both trailing spaces
        let fix = d.fix.as_ref().expect("W112 carries a fix");
        assert_eq!(fix.range.start_character, 6);
        assert_eq!(fix.range.end_character, 8);
    }

    #[test]
    fn w112_does_not_flag_crlf() {
        let diags = check_trailing_whitespace("set x 1\r\nset y 2\r\n");
        assert!(diags.is_empty());
    }

    #[test]
    fn w118_flags_crlf_when_lf_expected() {
        let diags = check_line_endings("a\r\nb\r\n", "\n");
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.code, DiagCode::W118);
        assert_eq!(d.severity, StyleSeverity::Hint);
        assert_eq!(d.message, "File uses CRLF line endings (2); expected LF");
        assert_eq!(d.range.start_line, 0);
        assert_eq!(d.range.start_character, 0);
    }

    #[test]
    fn w118_clean_when_all_lf_and_lf_expected() {
        assert!(check_line_endings("a\nb\nc\n", "\n").is_empty());
    }

    #[test]
    fn w118_mixed_message_preserves_order() {
        // One LF, one CRLF, one lone CR.
        let diags = check_line_endings("a\nb\r\nc\rd", "\n");
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].message,
            "Mixed line endings: LF (1), CRLF (1), CR (1); expected LF"
        );
    }

    #[test]
    fn line_lints_use_the_client_line_model_but_w118_sees_the_real_endings() {
        // An old-Mac document: the editor (and `tclsh`, whose script channel
        // rewrites `\r` to `\n` before parsing) sees three lines, so the
        // trailing whitespace on the second one is a line-1 diagnostic — not
        // a column deep inside a single 30-character line.
        let src = "set a 1\rset b 2   \rset c 3\r";
        let diags = style_diagnostics(
            src,
            120,
            "\n",
            None,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let w112: Vec<_> = diags.iter().filter(|d| d.code == DiagCode::W112).collect();
        assert_eq!(w112.len(), 1, "{diags:?}");
        assert_eq!(w112[0].range.start_line, 1);
        assert_eq!(w112[0].range.start_character, 7);
        // W118 still reports the document's actual terminators.
        let w118: Vec<_> = diags.iter().filter(|d| d.code == DiagCode::W118).collect();
        assert_eq!(w118.len(), 1, "{diags:?}");
        assert_eq!(
            w118[0].message,
            "File uses CR line endings (3); expected LF"
        );
    }

    #[test]
    fn w115_flags_comment_continuation_with_perline_fix() {
        let src = "# first line \\\nsecond line\nputs ok";
        let diags = check_comment_continuation(src);
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.code, DiagCode::W115);
        assert_eq!(d.severity, StyleSeverity::Warning);
        assert_eq!(d.range.start_line, 0);
        assert_eq!(d.range.end_line, 1);
        let fix = d.fix.as_ref().expect("W115 carries a fix");
        // First line loses its backslash; the swallowed line
        // becomes its own `#` comment.
        assert_eq!(fix.new_text, "# first line\n# second line");
        assert_eq!(fix.description, "Convert to per-line comments");
    }

    #[test]
    fn w115_uses_lexer_comment_positions_not_braced_or_quoted_data() {
        let braced = "set payload {# hidden \\\nputs live}\n";
        let quoted = "set payload \"# hidden \\\nputs live\"\n";
        let tail = "set marker \"noqa\"; # ordinary comment \\\nputs live\n";
        assert!(
            check_comment_continuation_for_dialect(
                braced,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
            )
            .is_empty()
        );
        assert!(
            check_comment_continuation_for_dialect(
                quoted,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
            )
            .is_empty()
        );
        assert!(
            check_comment_continuation_for_dialect(
                tail,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
            )
            .is_empty()
        );
    }

    #[test]
    fn w115_reaches_a_proven_alias_proc_body() {
        let src = "interp alias {} define {} proc\ndefine f {} {\n    # swallowed \\\n    puts hidden\n}\n";
        let diags = check_comment_continuation_for_dialect(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].range.start_line, 2);
        assert!(
            diags[0]
                .fix
                .as_ref()
                .is_some_and(|fix| fix.new_text.contains("# puts hidden"))
        );
    }

    #[test]
    fn w115_reaches_a_switch_case_list_arm_in_a_command_substitution() {
        let src = "set result [switch $kind {\n    alpha {\n        # swallowed \\\n        puts hidden\n    }\n}]\n";
        let diags = check_comment_continuation_for_dialect(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].code, DiagCode::W115);
        assert_eq!(diags[0].range.start_line, 2);
        let fix = diags[0].fix.as_ref().expect("W115 has an action vector");
        assert!(fix.new_text.contains("# puts hidden"), "{fix:?}");
    }

    fn original_comment_registry() -> tcl_registry::CommandRegistry {
        fn reference_only(
            _args: tcl_registry::InvocationArguments<'_>,
        ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
            vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
        }
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "hold-script",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::Body)],
            ..tcl_registry::CommandSpec::DEFAULT
        });
        registry.insert(tcl_registry::CommandSpec {
            name: "reference-script",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::Body)],
            script_timing_resolver: Some(reference_only),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        registry
    }

    fn original_comment_analysis(source: &str) -> AnalysisResult {
        let profile = crate::profile_for_dialect("tcl8.6");
        let config = tcl_lexer::LexerConfig {
            strict_quoting: true,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        crate::source_structure::analyse_document(
            source,
            profile,
            &original_comment_registry(),
            config,
        )
    }

    #[test]
    fn original_comment_facts_keep_actual_registry_config_and_reference_syntax() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        let source = "hold-script {\n    # café 😀 \\\n    puts hidden\n    puts visible\n}\nreference-script {\n    # reference \\\n    format hidden\n    format visible\n}\n";
        let analysis = original_comment_analysis(source);
        let facts = comment_facts_from_analysis(source, &analysis).unwrap();
        assert_eq!(
            facts.iter().map(|fact| fact.line).collect::<Vec<_>>(),
            [1, 6]
        );
        for fact in &facts {
            assert_eq!(source.get(fact.span.as_range()), Some(fact.text.as_str()));
        }
        assert!(analysis.body_lexer_config.unwrap().strict_quoting);
        let diags = check_comment_continuation_from_analysis(source, &analysis);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].range.start_line, 1);
        assert_eq!(
            diags[0].fix.as_ref().unwrap().new_text,
            "    # café 😀\n    # puts hidden"
        );
        let mut evaluated = Vec::new();
        crate::executable_regions::visit_analysis_executable_commands(
            source,
            &analysis,
            &mut |command, _, _| {
                evaluated.push(command.span.start());
                false
            },
        );
        let positive = u32::try_from(source.find("puts visible").unwrap()).unwrap();
        assert!(
            evaluated.contains(&positive),
            "the actual potential Body positive must be visited: {evaluated:?}"
        );
        let reference_content = source.find("    # reference").unwrap();
        assert!(
            !evaluated
                .iter()
                .any(|&offset| { usize::try_from(offset).unwrap() >= reference_content }),
            "reference-only comment syntax must not supply evaluation authority"
        );
        assert!(
            check_comment_continuation_for_dialect(source, crate::profile_for_dialect("tcl8.6"))
                .is_empty(),
            "the static dialect store does not contain these actual custom schemas"
        );
    }

    #[test]
    fn original_comment_facts_share_genuine_stub_body_source_geometry() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub hold {script:body}\n# tcl-lsp: stubs-end\nhold {\n    # café 😀 \\\n    puts hidden\n}\n";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let facts = comment_facts_from_analysis(source, &analysis).unwrap();
        let fact = facts.iter().find(|fact| fact.line == 4).unwrap();
        assert_eq!(source.get(fact.span.as_range()), Some(fact.text.as_str()));
        let diags = check_comment_continuation_from_analysis(source, &analysis);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].range.start_line, 4);
        for suffix in [
            "hold $script",
            r#"hold "\u0023 note \\\nputs hidden\n""#,
            "proc hold {value} {}\nhold {\n# note \\\nputs hidden\n}",
        ] {
            let changed = format!(
                "# tcl-lsp: stubs-begin\n# tcl-lsp: stub hold {{script:body}}\n# tcl-lsp: stubs-end\n{suffix}"
            );
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(&changed, "tcl8.6");
            assert!(
                check_comment_continuation_from_analysis(&changed, &analysis).is_empty(),
                "{suffix}"
            );
        }
    }

    #[test]
    fn original_comment_facts_refuse_stale_source_input_and_config() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        let source = "hold-script {\n# note \\\nputs hidden\n}\n";
        let analysis = original_comment_analysis(source);
        assert_eq!(
            comment_facts_from_analysis(source, &analysis)
                .unwrap()
                .len(),
            1
        );
        assert!(comment_facts_from_analysis(&format!("{source} "), &analysis).is_none());
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting = false;
        assert!(comment_facts_from_analysis(source, &changed).is_none());
        let mut changed = analysis.clone();
        changed.resolved_input = None;
        assert!(comment_facts_from_analysis(source, &changed).is_none());
        assert!(comment_facts_from_analysis(source, &AnalysisResult::default()).is_none());
        let mut changed = analysis.clone();
        let profile = crate::profile_for_dialect("tcl8.6");
        changed.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::context_for_profile(profile),
            analysis.body_lexer_config.unwrap(),
        ));
        assert!(
            comment_facts_from_analysis(source, &changed).is_none(),
            "same nominal dialect does not preserve the actual command store"
        );
    }

    #[test]
    fn original_comment_facts_do_not_invent_body_or_physical_line_mapping() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        for source in [
            "unknown-body {\n# note \\\nputs hidden\n}\n",
            "proc hold-script {value} {}\nhold-script {\n# note \\\nputs hidden\n}\n",
            r#"hold-script "\u0023 note \\\nputs hidden\n""#,
            "hold-script {\n# note \\\nputs hidden\n}suffix\n",
            "hold-script {# inline \\\nputs hidden\n}\n",
        ] {
            let analysis = original_comment_analysis(source);
            assert!(
                check_comment_continuation_from_analysis(source, &analysis).is_empty(),
                "{source:?}"
            );
        }
    }

    #[test]
    fn original_style_orchestrator_keeps_actual_normalised_image_and_independent_checks() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        let source = "hold-script {\r# café 😀 \\\nputs 😀\r}\r";
        let normalised = tcl_lexer::normalise_lone_cr(source);
        let analysis = original_comment_analysis(&normalised);
        let diags = style_diagnostics_from_analysis(
            source,
            DEFAULT_LINE_LENGTH,
            DEFAULT_LINE_ENDING,
            &no_disable(),
            &no_suppress(),
            None,
            &analysis,
        );
        let warning = diags.iter().find(|diag| diag.code == "W115").unwrap();
        assert_eq!(warning.range.start_line, 1);
        assert_eq!(warning.range.end_line, 2);
        assert_eq!(warning.range.end_character, 7);
        assert!(diags.iter().any(|diag| diag.code == "W118"));
        let stale = style_diagnostics_from_analysis(
            &format!("{source} "),
            DEFAULT_LINE_LENGTH,
            DEFAULT_LINE_ENDING,
            &no_disable(),
            &no_suppress(),
            None,
            &analysis,
        );
        assert!(!stale.iter().any(|diag| diag.code == "W115"));
        assert!(stale.iter().any(|diag| diag.code == "W118"));
    }

    #[test]
    fn w115_preserves_indent_and_existing_hash() {
        let src = "    # head \\\n    # already\nbody";
        let diags = check_comment_continuation(src);
        assert_eq!(diags.len(), 1);
        let fix = diags[0].fix.as_ref().unwrap();
        // The continuation line already starts with `#`, so only
        // the indent is re-applied (no extra `# `).
        assert_eq!(fix.new_text, "    # head\n    # already");
    }

    #[test]
    fn w115_skips_plain_comment() {
        assert!(check_comment_continuation("# normal comment\nputs hi").is_empty());
    }

    #[test]
    fn w115_trailing_space_breaks_the_continuation() {
        assert!(check_comment_continuation("# note \\ \nputs next\n").is_empty());
    }
}
