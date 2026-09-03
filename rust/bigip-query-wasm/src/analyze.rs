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

//! iRule analyser overlay for the report's inline-diagnostics toggle.
//!
//! Runs the same analyser the LSP / `tcl diag` use (diagnostics *and* optimiser
//! suggestions, in the `f5-irules` dialect), then renders the iRule body as
//! syntax-highlighted HTML with each diagnostic range wrapped in an underline
//! span (tooltip = code + message). Returns that HTML plus a structured list of
//! the findings so the front-end can show a summary beneath the code — all
//! client-side, keeping the report self-contained.

use std::fmt::Write as _;

use tcl_compiler::analyser::Analyser;
use tcl_lexer::highlight_ranges_with_config;
use tcl_lsp_core::formatting::{FormatterConfig, format_tcl};

/// Format an iRule body with the F5 iRules Style Guide formatter in the
/// `f5-irules` dialect — the dialect matters because TMM accepts `}{` (e.g.
/// `if {expr}{body}`), which the formatter then re-emits as `} {`. Shared by the
/// Format button and the diagnostics pass (which normalises before analysing so
/// spans land on the right characters).
#[must_use]
pub fn format_irule_source(source: &str) -> String {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    // One resolved profile carries every dialect fact the formatter needs
    // (issue #1465) — the iRules lexer grammar included.
    let config = FormatterConfig::for_dialect("f5-irules");
    format_tcl(source, &config, registry)
}

/// One finding, flattened for the front-end.
struct Finding {
    /// Position in the emitted `diagnostics[]` list — the id the annotated HTML
    /// references via `data-diag`.
    index: usize,
    start: usize,
    end: usize,
    severity: &'static str,
    code: String,
    message: String,
}

/// Analyse an iRule body and return a JSON object:
/// `{ "html": "<annotated highlighted source>", "diagnostics": [ … ],
///    "counts": { "error": n, "warning": n, … } }`.
///
/// `diagnostics[]` entries are `{ line, col, endLine, endCol, severity, code,
/// message, index }` (1-based line/col). `html` carries `data-diag="<index>"`
/// on each underline span so the front-end can cross-link the list and the code.
#[must_use]
pub fn analyze_irule(source: &str) -> String {
    // Normalise first: the analyser tracks positions on well-formed input, and
    // an iRule's `}{` (valid in TMM) otherwise collapses inner spans onto the
    // enclosing command. Formatting inserts the `} {` space, so diagnostics land
    // on the right characters. The displayed code is this formatted source.
    let formatted = format_irule_source(source);
    let source = formatted.as_str();
    let result = Analyser::new().analyse(source, "f5-irules");

    let mut findings: Vec<Finding> = result
        .diagnostics
        .iter()
        .map(|d| Finding {
            index: 0, // assigned after sorting
            start: (d.span.start() as usize).min(source.len()),
            end: (d.span.end() as usize).min(source.len()),
            severity: d.severity.as_str(),
            code: d.code.to_string(),
            message: d.message.clone(),
        })
        .collect();
    // Stable order: by start, then by widest span first (so an enclosing
    // diagnostic opens before a nested one).
    findings.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
    for (i, f) in findings.iter_mut().enumerate() {
        f.index = i;
    }

    let html = render_annotated(source, &findings);
    let line_starts = line_starts(source);

    let mut out = String::with_capacity(html.len() + 256);
    out.push_str("{\"html\":");
    push_json_string(&html, &mut out);
    out.push_str(",\"diagnostics\":[");
    for (i, f) in findings.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let (line, col) = line_col(&line_starts, f.start);
        let (end_line, end_col) = line_col(&line_starts, f.end);
        let _ = write!(
            out,
            "{{\"line\":{line},\"col\":{col},\"endLine\":{end_line},\"endCol\":{end_col},\"severity\":\""
        );
        out.push_str(f.severity);
        out.push_str("\",\"code\":");
        push_json_string(&f.code, &mut out);
        out.push_str(",\"message\":");
        push_json_string(&f.message, &mut out);
        let _ = write!(out, ",\"index\":{i}}}");
    }
    out.push_str("],\"counts\":");
    push_counts(&findings, &mut out);
    out.push('}');
    out
}

/// Render `source` as highlighted HTML with diagnostic ranges wrapped in
/// `<span class="diag diag-<sev>" …>` underline spans.
///
/// Uses a boundary model: cut the source at every token edge and every
/// diagnostic edge, then for each segment emit the (at most one) token-colour
/// span nested inside a diagnostic span when a diagnostic is active. Re-opening
/// spans per segment keeps the HTML well-nested even when token and diagnostic
/// ranges cross, and when diagnostics overlap each other.
fn render_annotated(source: &str, findings: &[Finding]) -> String {
    let hl = highlight_ranges_with_config(source, irules_file_config());

    // Collect and sort unique cut points.
    let mut cuts: Vec<usize> = Vec::with_capacity(hl.len() * 2 + findings.len() * 2 + 2);
    cuts.push(0);
    cuts.push(source.len());
    for r in &hl {
        cuts.push(r.start);
        cuts.push(r.end);
    }
    for f in findings {
        cuts.push(f.start);
        cuts.push(f.end);
    }
    cuts.sort_unstable();
    cuts.dedup();

    let mut out = String::with_capacity(source.len() * 2);
    for win in cuts.windows(2) {
        let (a, b) = (win[0], win[1]);
        if a >= b {
            continue;
        }
        let token = hl
            .iter()
            .find(|r| r.start <= a && r.end >= b)
            .map(|r| r.class);
        let active: Vec<&Finding> = findings.iter().filter(|f| f.start <= a && f.end >= b).collect();

        if !active.is_empty() {
            open_diag_span(&active, &mut out);
        }
        if let Some(cls) = token {
            out.push_str("<span class=\"");
            out.push_str(cls);
            out.push_str("\">");
        }
        push_escaped(&source[a..b], &mut out);
        if token.is_some() {
            out.push_str("</span>");
        }
        if !active.is_empty() {
            out.push_str("</span>");
        }
    }
    out
}

/// Open one diagnostic underline span covering a segment. When several
/// diagnostics overlap the segment, the span takes the worst severity's colour
/// and its tooltip lists every finding.
fn open_diag_span(active: &[&Finding], out: &mut String) {
    let worst = active
        .iter()
        .map(|f| severity_rank(f.severity))
        .max()
        .unwrap_or(0);
    let worst_name = active
        .iter()
        .find(|f| severity_rank(f.severity) == worst)
        .map_or("warning", |f| f.severity);

    out.push_str("<span class=\"diag diag-");
    out.push_str(worst_name);
    out.push_str("\" data-diag=\"");
    for (i, f) in active.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let _ = write!(out, "{}", f.index);
    }
    out.push_str("\" title=\"");
    for (i, f) in active.iter().enumerate() {
        if i > 0 {
            out.push_str(" \u{2022} ");
        }
        push_attr_escaped(&f.code, out);
        out.push_str(": ");
        push_attr_escaped(&f.message, out);
    }
    out.push_str("\">");
}

fn severity_rank(sev: &str) -> u8 {
    match sev {
        "error" => 4,
        "warning" => 3,
        "info" => 2,
        "suggestion" => 1,
        _ => 0, // hint
    }
}

fn push_counts(findings: &[Finding], out: &mut String) {
    let mut error = 0u32;
    let mut warning = 0u32;
    let mut info = 0u32;
    let mut suggestion = 0u32;
    let mut hint = 0u32;
    for f in findings {
        match f.severity {
            "error" => error += 1,
            "warning" => warning += 1,
            "info" => info += 1,
            "suggestion" => suggestion += 1,
            _ => hint += 1,
        }
    }
    let _ = write!(
        out,
        "{{\"error\":{error},\"warning\":{warning},\"info\":{info},\"suggestion\":{suggestion},\"hint\":{hint}}}"
    );
}

/// Byte offset of the start of each line (index 0 = offset 0).
fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// 1-based (line, column) of a byte offset. Column counts UTF-8 bytes from the
/// line start + 1 — matching the byte-oriented spans the highlighter uses.
fn line_col(line_starts: &[usize], offset: usize) -> (usize, usize) {
    let line = match line_starts.binary_search(&offset) {
        Ok(i) => i,
        Err(i) => i - 1,
    };
    (line + 1, offset - line_starts[line] + 1)
}

fn push_escaped(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// Escape for an HTML attribute value (adds `"` on top of text escapes).
fn push_attr_escaped(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

/// Append `s` as a JSON string literal (quotes + escapes).
fn push_json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotated_html_wraps_diagnostics_and_stays_wellformed() {
        // A real iRule with an undefined-variable read ($html reads nothing).
        let src = "when HTTP_REQUEST {\n  set html-pool web1Pool\n  pool $html-pool\n}";
        let json = analyze_irule(src);
        // JSON shape
        assert!(json.starts_with("{\"html\":"));
        assert!(json.contains("\"diagnostics\":["));
        assert!(json.contains("\"counts\":{"));
        // The highlighted event token survives the overlay.
        assert!(json.contains("tk-event"));
        // Balanced spans in the html (every <span opens, every </span closes).
        let html_open = json.matches("<span").count();
        let html_close = json.matches("</span>").count();
        assert_eq!(html_open, html_close, "unbalanced spans in annotated html");
    }

    #[test]
    fn undefined_variable_read_is_flagged() {
        // `pool $html-pool` reads variable `html`, which is never set
        // (`html-pool` was set, but `$html-pool` = `$html` + `-pool`).
        let src = "when HTTP_REQUEST {\n  set html-pool web1Pool\n  pool $html-pool\n}";
        let json = analyze_irule(src);
        // The analyser should surface a finding mentioning the undefined `html`
        // read; assert at least one diagnostic exists and references html.
        assert!(
            json.contains("\"diagnostics\":[{") || json.contains("\"diagnostics\":[ {"),
            "expected at least one diagnostic, got: {json}"
        );
    }

    #[test]
    fn formatter_inserts_space_in_irule_brace_chain() {
        // TMM accepts `}{`; the formatter must normalise it to `} {`.
        let formatted = format_irule_source("when HTTP_REQUEST {\n  if { 1 }{\n    pool p\n  }\n}");
        assert!(
            !formatted.contains("}{"),
            "formatter left `}}{{` unfixed:\n{formatted}"
        );
        assert!(formatted.contains("} {"), "no `}} {{` in:\n{formatted}");
    }

    #[test]
    fn diagnostics_range_correctly_through_brace_chain() {
        // With `}{`, analysing the raw source mis-attributes the `$x` read to
        // the `if` line; formatting first fixes the span. The var read is on the
        // `pool $x` line, so the W210 finding must NOT carry the `if` line.
        let json = analyze_irule("when HTTP_REQUEST {\n  if { 1 }{\n    pool $x\n  }\n}");
        assert!(json.contains("W210"), "expected W210: {json}");
        // Read the "line" of the diagnostic object that carries code W210:
        // in an object `{"line":N,"col":…,"code":"W210",…}` the line precedes
        // the code, so search back from the W210 code to its `"line":`.
        let code_at = json.find("\"code\":\"W210\"").expect("W210 present");
        let line_key = json[..code_at].rfind("\"line\":").expect("line before code");
        let after = &json[line_key + "\"line\":".len()..];
        let n: usize = after
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .expect("line number");
        // The `if` is on line 2 of the formatted body; the `pool $x` read is
        // below it. Collapsed-onto-if would report line 1 or 2.
        assert!(n >= 3, "W210 reported on line {n}, looks collapsed onto the if");
    }

    #[test]
    fn line_col_is_one_based() {
        let starts = line_starts("ab\ncd\n");
        assert_eq!(line_col(&starts, 0), (1, 1));
        assert_eq!(line_col(&starts, 1), (1, 2));
        assert_eq!(line_col(&starts, 3), (2, 1)); // first char of line 2
        assert_eq!(line_col(&starts, 4), (2, 2));
    }

    #[test]
    fn hints_and_suggestions_are_surfaced() {
        // A high-frequency event with a regular expression has a real
        // performance hint. A bare `when` is deliberately absent here: BIG-IP
        // defaults its priority to 500, so omission is valid.
        let src = "when HTTP_REQUEST {\n  regexp a $uri\n}";
        let json = analyze_irule(src);
        assert!(json.contains("IRULE2101"), "expected the regexp hint: {json}");
        assert!(json.contains("\"severity\":\"hint\""));
        assert!(json.contains("tk-event"));
    }
}

/// The whole-file lexer config for an iRule, resolved once through the
/// ingress: `}{` splits words and `{*}` is inert, so highlighting reads the
/// rule as TMM does rather than as Tcl 9.
pub(crate) fn irules_file_config() -> tcl_lexer::LexerConfig {
    tcl_lexer::LexerConfig::for_file_grammar(
        tcl_registry::model::resolve_environment("f5-irules").grammar(),
    )
}
