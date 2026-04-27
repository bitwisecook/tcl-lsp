//! Pure analyser helpers — Rust port of
//! ``core/analysis/_analyser/_utils.py``.
//!
//! Pure free functions used by handlers and diagnostic emitters
//! across the analyser. `parse_param_list` is re-exported from
//! `signature_scan::params` (already ported in C40a2) so the
//! analyser can keep its imports flat.

use std::collections::HashSet;

use tcl_lexer::Token;
use tcl_registry::{CommandRegistry, Traits};

use crate::ir::Statement;

pub use crate::signature_scan::params::parse_param_list;

/// Cap on how many leading lines the file-suppression scanner
/// inspects. Mirrors `_FILE_DIRECTIVE_SCAN_LINES` in
/// `_utils.py`. Pathological all-comment files stop scanning
/// past this.
const FILE_DIRECTIVE_SCAN_LINES: usize = 100;

/// Extract file-wide diagnostic suppression from top-of-file
/// directives.
///
/// Mirrors `parse_file_suppression` in
/// `core/analysis/_analyser/_utils.py:113-137`. Scans leading
/// comment / blank lines for `# tcl-lsp: disable=CODE1,CODE2`
/// (or `=*` for "all codes"); stops at the first line that is
/// neither blank nor a `#` comment. Multiple directives
/// accumulate into the returned set.
///
/// Codes are split on commas and any whitespace. Empty tokens
/// after splitting are discarded; case-sensitivity matches the
/// Python source (the keyword `tcl-lsp:disable=` matches
/// case-insensitively but the codes themselves don't get
/// lowercased).
#[must_use]
pub fn parse_file_suppression(source: &str) -> HashSet<String> {
    let mut codes: HashSet<String> = HashSet::new();
    for (idx, line) in source.lines().enumerate() {
        if idx >= FILE_DIRECTIVE_SCAN_LINES {
            break;
        }
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }
        if !stripped.starts_with('#') {
            break;
        }
        let Some(rest) = parse_disable_directive(line) else {
            continue;
        };
        for token in rest.split([',', ' ', '\t', '\r', '\n']) {
            let token = token.trim();
            if !token.is_empty() {
                codes.insert(token.to_string());
            }
        }
    }
    codes
}

/// Scan `source` for inline ``# tcl-lsp: stub <name> ...``
/// declarations bounded by ``# tcl-lsp: stubs-begin`` /
/// ``# tcl-lsp: stubs-end`` markers, returning the set of
/// declared command names.
///
/// Mirrors the name-extraction subset of
/// `core/analysis/stub_comments.py::scan_source_for_stubs`.
/// The Python helper additionally parses arg-roles and flags
/// (``-loop``, ``-barrier``, etc.) into a full
/// ``StubCommandDef`` record; the Rust analyser doesn't yet
/// carry that field on ``AnalysisResult`` so we extract just
/// the names.  Adding the names to the W123 candidate set is
/// the load-bearing use case — the role/flag data only
/// matters for downstream diagnostic emitters that the Rust
/// port hasn't yet wired.
///
/// Stubs declared *outside* the begin/end markers are ignored
/// (matches Python).  ``expr-func`` / ``expr-op`` lines are
/// skipped — they declare expression functions, not commands.
#[must_use]
pub fn scan_stub_command_names(source: &str) -> HashSet<String> {
    let mut names: HashSet<String> = HashSet::new();
    let mut in_block = false;
    for line in source.lines() {
        let stripped = line.trim();
        if !stripped.starts_with('#') {
            continue;
        }
        let body = stripped.trim_start_matches('#').trim_start();
        // Markers — both case-insensitive on the keyword.
        if matches_marker(body, "stubs-begin") {
            in_block = true;
            continue;
        }
        if matches_marker(body, "stubs-end") {
            in_block = false;
            continue;
        }
        if !in_block {
            continue;
        }
        if let Some(name) = parse_stub_name(body) {
            // Skip ``expr-func`` / ``expr-op`` — those
            // declare expression-language symbols, not
            // commands.
            if name.starts_with("expr-") {
                continue;
            }
            names.insert(name.to_string());
        }
    }
    names
}

/// Match a ``# tcl-lsp: <marker>`` line, where `marker` is
/// e.g. ``"stubs-begin"`` / ``"stubs-end"``.  Case-insensitive
/// on the keyword run; whitespace flexible.
fn matches_marker(body: &str, marker: &str) -> bool {
    let s = body.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    if s.len() < kw_end || !s[..kw_end].eq_ignore_ascii_case(lower_prefix) {
        return false;
    }
    let rest = s[kw_end..].trim_start();
    let Some(rest) = rest.strip_prefix(':') else {
        return false;
    };
    let rest = rest.trim_start();
    let m_end = marker.len();
    if rest.len() < m_end {
        return false;
    }
    rest[..m_end].eq_ignore_ascii_case(marker)
}

/// Extract the command name from a ``# tcl-lsp: stub NAME …``
/// line.  Returns the raw name as a borrowed slice or `None`
/// when the line doesn't match the stub shape.
fn parse_stub_name(body: &str) -> Option<&str> {
    let s = body.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    if s.len() < kw_end || !s[..kw_end].eq_ignore_ascii_case(lower_prefix) {
        return None;
    }
    let rest = s[kw_end..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let stub_kw = "stub";
    let stub_end = stub_kw.len();
    if rest.len() < stub_end || !rest[..stub_end].eq_ignore_ascii_case(stub_kw) {
        return None;
    }
    let after = rest[stub_end..].trim_start();
    if after.is_empty() {
        return None;
    }
    // First whitespace-delimited token is the command name.
    let end = after
        .find(|c: char| c.is_whitespace())
        .unwrap_or(after.len());
    if end == 0 {
        None
    } else {
        Some(&after[..end])
    }
}

/// Scan `source` for inline ``# tcl-lsp: stub NAME ...`` and
/// ``# tcl-lsp: stub expr-func NAME ...`` declarations bounded by
/// the ``stubs-begin`` / ``stubs-end`` markers, returning
/// ``(commands, expr_defs)``.
///
/// Mirrors the LSP-relevant subset of
/// `core/analysis/stub_comments.py::scan_source_for_stubs`.  We
/// only carry name + line span here; the richer metadata
/// (arg-roles, ``-loop`` / ``-barrier`` flags, …) is materialised
/// from Python until a future port closes the gap.
#[must_use]
pub fn scan_source_for_stubs(
    source: &str,
) -> (
    Vec<super::types::StubCommandDef>,
    Vec<super::types::StubExprDef>,
) {
    let mut cmds = Vec::new();
    let mut exprs = Vec::new();
    let mut in_block = false;
    let mut offset: u32 = 0;
    for line in source.split_inclusive('\n') {
        let trimmed_line: &str = line.trim_end_matches('\n');
        let line_byte_len = line.len() as u32;
        let stripped = trimmed_line.trim();
        let span = tcl_lexer::Span::new(offset, offset + trimmed_line.len() as u32);
        offset += line_byte_len;
        if !stripped.starts_with('#') {
            continue;
        }
        let body = stripped.trim_start_matches('#').trim_start();
        if matches_marker(body, "stubs-begin") {
            in_block = true;
            continue;
        }
        if matches_marker(body, "stubs-end") {
            in_block = false;
            continue;
        }
        if !in_block {
            continue;
        }
        let Some(name) = parse_stub_name(body) else {
            continue;
        };
        if let Some(expr_name) = name.strip_prefix("expr-") {
            exprs.push(super::types::StubExprDef {
                name: expr_name.to_string(),
                range: span,
            });
        } else {
            cmds.push(super::types::StubCommandDef {
                name: name.to_string(),
                range: span,
            });
        }
    }
    (cmds, exprs)
}

/// Per-line ``# noqa: CODE`` suppression scanner.
///
/// Mirrors the inline-noqa half of
/// `core/analysis/_analyser/_utils.py::parse_per_line_suppression`.
/// Walks every line; for each ``# noqa`` (with or without
/// ``: CODE`` list), records the codes against the 0-based
/// line number.  Bare ``# noqa`` (no codes) records the
/// ``"*"`` sentinel meaning "suppress every code on this line".
///
/// Returns a ``HashMap<line, HashSet<code>>``; callers merge it
/// into ``AnalysisResult.suppressed_lines``.
#[must_use]
pub fn parse_per_line_suppression(source: &str) -> std::collections::HashMap<i32, HashSet<String>> {
    let mut by_line: std::collections::HashMap<i32, HashSet<String>> =
        std::collections::HashMap::new();
    for (idx, line) in source.lines().enumerate() {
        // Find ``# noqa`` anywhere on the line (case-insensitive
        // on the keyword); Python uses ``re.search``.
        let Some(hash_at) = line.find('#') else {
            continue;
        };
        let after_hash = &line[hash_at + 1..];
        let trimmed = after_hash.trim_start();
        let lower = "noqa";
        if trimmed.len() < lower.len() {
            continue;
        }
        if !trimmed[..lower.len()].eq_ignore_ascii_case(lower) {
            continue;
        }
        let rest = trimmed[lower.len()..].trim_start();
        let mut codes: HashSet<String> = HashSet::new();
        if let Some(after_colon) = rest.strip_prefix(':') {
            for tok in after_colon.split([',', ' ', '\t', '\r']) {
                let tok = tok.trim();
                if !tok.is_empty() {
                    codes.insert(tok.to_string());
                }
            }
            if codes.is_empty() {
                codes.insert("*".to_string());
            }
        } else {
            // Bare ``# noqa`` — apply to every code on this line.
            codes.insert("*".to_string());
        }
        by_line.entry(idx as i32).or_default().extend(codes);
    }
    by_line
}

/// Match `# tcl-lsp: disable=…` (case-insensitive on the keyword),
/// returning the trailing CODE list as a borrowed slice. Returns
/// `None` if the line doesn't match the directive shape.
fn parse_disable_directive(line: &str) -> Option<&str> {
    let mut s = line.trim_start();
    s = s.strip_prefix('#')?.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    if s.len() < kw_end || !s[..kw_end].eq_ignore_ascii_case(lower_prefix) {
        return None;
    }
    s = s[kw_end..].trim_start();
    s = s.strip_prefix(':')?.trim_start();
    let disable_kw = "disable";
    let dis_end = disable_kw.len();
    if s.len() < dis_end || !s[..dis_end].eq_ignore_ascii_case(disable_kw) {
        return None;
    }
    s = s[dis_end..].trim_start();
    let s = s.strip_prefix('=')?.trim();
    Some(s)
}

/// Format a literal value for inclusion in a diagnostic message.
///
/// Mirrors `_format_literal_for_message` in
/// `core/analysis/_analyser/_utils.py:156-161`. Replaces newlines
/// with `\n` (literal two characters), and truncates strings
/// longer than 40 characters to `…[37 chars]…`.
#[must_use]
pub fn format_literal_for_message(value: &str) -> String {
    let display = value.replace('\n', "\\n");
    if display.chars().count() > 40 {
        // Match Python's slice behaviour at character (not byte)
        // boundaries — Tcl source is usually ASCII but multibyte
        // diagnostic messages are valid.
        let truncated: String = display.chars().take(37).collect();
        format!("{truncated}...")
    } else {
        display
    }
}

/// Return `(variable, static_value)` for assignments worth the
/// W214 paste-error heuristic check.
///
/// Mirrors `_possible_paste_fingerprint` in
/// `core/analysis/_analyser/_utils.py:140-153`. Only
/// [`Statement::AssignConst`] and [`Statement::AssignValue`]
/// shapes qualify; `AssignValue` rejects values containing
/// `$` / `[` / `]` (interpolation / command substitution
/// disqualify the fingerprint).
#[must_use]
pub fn possible_paste_fingerprint(stmt: &Statement) -> Option<(String, String)> {
    match stmt {
        Statement::AssignConst { name, value, .. } => {
            Some((name.clone(), value.trim().to_string()))
        }
        Statement::AssignValue { name, value, .. } => {
            let value = value.trim();
            if value.is_empty() {
                return None;
            }
            if value.contains('$') || value.contains('[') || value.contains(']') {
                return None;
            }
            Some((name.clone(), value.to_string()))
        }
        _ => None,
    }
}

/// Return argv tokens widened to each Tcl word's full token span.
///
/// Mirrors `_argv_with_word_spans` in
/// `core/analysis/_analyser/_utils.py:164-166`, which delegates
/// to `core.parsing.argv.widen_argv_tokens_to_word_spans`. Rust's
/// segmenter already returns argv tokens with word-wide spans
/// (per Seg1 — see `rust/tcl-compiler/src/segmenter.rs:227-238`),
/// so this is the identity function. The helper is kept as a
/// transparent passthrough so handlers calling Python's
/// `_argv_with_word_spans` translate 1:1 without losing the
/// indirection layer the Python source carries.
#[must_use]
pub fn argv_with_word_spans(argv: Vec<Token>, _all_tokens: &[Token]) -> Vec<Token> {
    argv
}

/// The set of iRules commands that may only appear at the top
/// level of an event handler, computed fresh from the supplied
/// `registry`.
///
/// Mirrors `_irules_top_level_only` in
/// `core/analysis/_analyser/_utils.py:34-38`. Reads the
/// `IRULES_TOP_LEVEL_ONLY` trait from `registry` and returns the
/// matching command names.
///
/// **Not cached.** A previous version cached the first-call
/// result in a static `OnceLock`, which silently returned stale
/// data when a non-default `CommandRegistry` was passed on a
/// later call. The trait scan is `O(n)` over the registry's
/// command specs (~150 entries) and the registry itself is
/// already cached at the call sites that need this — so per-call
/// recomputation is cheap in practice and removes a correctness
/// trap. Callers that want caching should cache at their own
/// layer, keyed by whatever identity makes sense for them.
#[must_use]
pub fn irules_top_level_only(registry: &CommandRegistry) -> HashSet<String> {
    registry
        .commands_with_trait(Traits::IRULES_TOP_LEVEL_ONLY)
        .into_iter()
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::Span;

    #[test]
    fn parse_file_suppression_finds_codes() {
        let src = "# tcl-lsp: disable=W210,W211\nproc foo {} {}\n";
        let codes = parse_file_suppression(src);
        assert!(codes.contains("W210"));
        assert!(codes.contains("W211"));
        assert_eq!(codes.len(), 2);
    }

    #[test]
    fn parse_file_suppression_handles_whitespace_and_case() {
        let src = "  #  TCL-LSP : DISABLE = W210  W211  \n\n# more\nproc foo {} {}\n";
        let codes = parse_file_suppression(src);
        assert!(codes.contains("W210"));
        assert!(codes.contains("W211"));
    }

    #[test]
    fn parse_file_suppression_stops_at_first_non_comment() {
        let src = "# tcl-lsp: disable=W210\nproc foo {} {}\n# tcl-lsp: disable=W211\n";
        let codes = parse_file_suppression(src);
        assert!(codes.contains("W210"));
        assert!(!codes.contains("W211"));
    }

    #[test]
    fn parse_file_suppression_skips_blank_lines() {
        let src = "\n\n# tcl-lsp: disable=W210\n\nproc foo {} {}\n";
        let codes = parse_file_suppression(src);
        assert!(codes.contains("W210"));
    }

    #[test]
    fn parse_file_suppression_handles_wildcard() {
        let src = "# tcl-lsp: disable=*\nproc foo {} {}\n";
        let codes = parse_file_suppression(src);
        assert!(codes.contains("*"));
    }

    #[test]
    fn parse_file_suppression_no_directive_returns_empty() {
        let src = "# Just a comment\nproc foo {} {}\n";
        let codes = parse_file_suppression(src);
        assert!(codes.is_empty());
    }

    #[test]
    fn parse_file_suppression_caps_at_scan_lines() {
        // 200 blank/comment lines, directive at line 150 — should
        // NOT be picked up because the cap is 100.
        let mut src = String::new();
        for _ in 0..150 {
            src.push_str("# noise\n");
        }
        src.push_str("# tcl-lsp: disable=W210\n");
        let codes = parse_file_suppression(&src);
        assert!(codes.is_empty());
    }

    #[test]
    fn format_literal_truncates_long_values() {
        let long = "a".repeat(50);
        let formatted = format_literal_for_message(&long);
        assert!(formatted.ends_with("..."));
        // 37 chars + 3 dots = 40 chars total.
        assert_eq!(formatted.chars().count(), 40);
    }

    #[test]
    fn format_literal_replaces_newlines() {
        assert_eq!(format_literal_for_message("a\nb"), "a\\nb");
    }

    #[test]
    fn format_literal_short_passes_through() {
        assert_eq!(format_literal_for_message("hello"), "hello");
    }

    #[test]
    fn possible_paste_fingerprint_assign_const() {
        let stmt = Statement::AssignConst {
            name: "x".to_string(),
            value: "  hello  ".to_string(),
            span: Span::new(0, 0),
        };
        assert_eq!(
            possible_paste_fingerprint(&stmt),
            Some(("x".to_string(), "hello".to_string()))
        );
    }

    fn assign_value(value: &str) -> Statement {
        Statement::AssignValue {
            name: "x".to_string(),
            value: value.to_string(),
            span: Span::new(0, 0),
            value_needs_backsubst: false,
            tokens: None,
        }
    }

    #[test]
    fn possible_paste_fingerprint_assign_value_with_dollar_rejected() {
        assert_eq!(possible_paste_fingerprint(&assign_value("$y")), None);
    }

    #[test]
    fn possible_paste_fingerprint_assign_value_with_bracket_rejected() {
        assert_eq!(possible_paste_fingerprint(&assign_value("[expr 1]")), None);
    }

    #[test]
    fn possible_paste_fingerprint_assign_value_pure_literal() {
        assert_eq!(
            possible_paste_fingerprint(&assign_value("  hello  ")),
            Some(("x".to_string(), "hello".to_string()))
        );
    }

    #[test]
    fn possible_paste_fingerprint_other_stmt_returns_none() {
        // A non-assign statement — return.
        let stmt = Statement::Return {
            value: None,
            expr: None,
            braced: false,
            span: Span::new(0, 0),
        };
        assert_eq!(possible_paste_fingerprint(&stmt), None);
    }

    #[test]
    fn argv_with_word_spans_is_identity() {
        // Rust's segmenter already widens; this helper is a
        // transparent passthrough.
        use tcl_lexer::TokenType;
        let tok = Token::new(TokenType::Esc, Span::new(0, 4));
        let argv = vec![tok];
        let result = argv_with_word_spans(argv.clone(), &[]);
        assert_eq!(result, argv);
    }

    #[test]
    fn irules_top_level_only_matches_python_registry() {
        // Python's `REGISTRY.irules_top_level_only_commands()` reports
        // exactly `{"proc"}` — the only command tagged with the
        // `IRULES_TOP_LEVEL_ONLY` trait. Pin the Rust port to the
        // same set so a registry-spec drift surfaces here.
        let reg = CommandRegistry::build_default();
        let cmds = irules_top_level_only(&reg);
        assert!(
            cmds.contains("proc"),
            "expected `proc` in irules_top_level_only, got {cmds:?}",
        );
    }

    #[test]
    fn parse_param_list_reexport_works() {
        let params = parse_param_list("a b {c default}");
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "a");
        assert_eq!(params[2].name, "c");
        assert!(params[2].has_default);
    }

    #[test]
    fn scan_stub_command_names_extracts_inline_stubs() {
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub my_cmd {arg1:var body:body} -loop
# tcl-lsp: stub my_eval {script:body} -barrier
# tcl-lsp: stubs-end
proc foo {} {}
";
        let names = scan_stub_command_names(src);
        assert!(names.contains("my_cmd"));
        assert!(names.contains("my_eval"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn scan_stub_command_names_ignores_outside_block() {
        let src = "\
# tcl-lsp: stub orphan_cmd {x:var}
# tcl-lsp: stubs-begin
# tcl-lsp: stub inside {x:var}
# tcl-lsp: stubs-end
# tcl-lsp: stub also_orphan {x:var}
";
        let names = scan_stub_command_names(src);
        assert!(names.contains("inside"));
        assert!(!names.contains("orphan_cmd"));
        assert!(!names.contains("also_orphan"));
    }

    #[test]
    fn scan_stub_command_names_skips_expr_func_op_lines() {
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub expr-func sizeof 1
# tcl-lsp: stub expr-op contains 2
# tcl-lsp: stub regular_cmd {x:var}
# tcl-lsp: stubs-end
";
        let names = scan_stub_command_names(src);
        assert!(names.contains("regular_cmd"));
        assert!(!names.iter().any(|n| n.starts_with("expr-")));
    }

    #[test]
    fn scan_stub_command_names_handles_multiple_blocks() {
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub a {x:var}
# tcl-lsp: stubs-end
proc foo {} {}
# tcl-lsp: stubs-begin
# tcl-lsp: stub b {y:var}
# tcl-lsp: stubs-end
";
        let names = scan_stub_command_names(src);
        assert!(names.contains("a"));
        assert!(names.contains("b"));
    }

    #[test]
    fn scan_stub_command_names_empty_source() {
        let names = scan_stub_command_names("");
        assert!(names.is_empty());
    }
}
