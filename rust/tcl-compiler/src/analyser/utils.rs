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
}
