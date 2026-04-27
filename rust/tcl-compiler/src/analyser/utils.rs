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

/// Scan `source` for inline ``# tcl-lsp: stub NAME {ARGS} ?FLAGS?`` and
/// ``# tcl-lsp: stub expr-func NAME ?ARITY?`` (also ``expr-op``)
/// declarations bounded by the ``stubs-begin`` / ``stubs-end``
/// markers, returning ``(commands, expr_defs)``.
///
/// Mirrors `core/analysis/stub_comments.py::scan_source_for_stubs`,
/// including ``_parse_args`` and ``_parse_flags`` parity for the
/// command form and arity extraction for the expression form.
/// Command stubs *require* the ``{ARGS}`` brace block (Python's
/// `_STUB_RE` rejects bare ``stub NAME``).  Stubs whose ``:role``
/// annotation is unrecognised are silently dropped to match
/// Python's ``_parse_args`` returning ``None``.
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
        // ``# tcl-lsp:`` prefix is optional inside a stubs block —
        // strip it once if present.
        let after_prefix = strip_tcl_lsp_prefix(body);
        // Try the expr-func / expr-op shape first because ``stub
        // expr-func NAME`` would otherwise match the command shape
        // with NAME == "expr-func".
        if let Some((kind, name, arity)) = parse_expr_stub(after_prefix) {
            exprs.push(super::types::StubExprDef {
                name: name.to_string(),
                kind: kind.to_string(),
                arity,
                range: span,
            });
            continue;
        }
        if let Some(stub) = parse_command_stub(after_prefix, span) {
            cmds.push(stub);
        }
    }
    (cmds, exprs)
}

/// Strip a leading ``tcl-lsp:`` keyword (case-insensitive) from
/// the comment body, if present.  Whitespace flexible.
fn strip_tcl_lsp_prefix(body: &str) -> &str {
    let s = body.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    if s.len() < kw_end || !s[..kw_end].eq_ignore_ascii_case(lower_prefix) {
        return s;
    }
    let rest = s[kw_end..].trim_start();
    rest.strip_prefix(':').map_or(s, str::trim_start)
}

/// Valid argument-role annotations recognised after the ``:``
/// separator in a stub argument token.  Mirrors
/// ``_VALID_ROLES`` in ``core/analysis/stub_comments.py``.
const VALID_STUB_ROLES: &[&str] = &[
    "body", "expr", "var", "var_read", "name", "pattern", "channel", "value",
];

/// Recognised ``-flag`` tokens in a stub trailing-flag run.
/// Mirrors ``_VALID_FLAGS`` in ``core/analysis/stub_comments.py``.
const VALID_STUB_FLAGS: &[&str] = &[
    "-barrier",
    "-loop",
    "-pure",
    "-mutator",
    "-unsafe",
    "-scope_alias",
];

/// Parse a ``stub NAME {ARGS} ?FLAGS?`` line (case-insensitive on
/// the ``stub`` keyword).  Returns a fully-populated
/// `StubCommandDef` (name, args, flags, range) on match, or
/// ``None`` when the brace block is missing or an argument's
/// ``:role`` annotation is unrecognised.  Mirrors Python's
/// ``_STUB_RE`` + ``_parse_args`` + ``_parse_flags``.
fn parse_command_stub(line: &str, range: tcl_lexer::Span) -> Option<super::types::StubCommandDef> {
    let s = line.trim_start();
    let stub_kw = "stub";
    if s.len() < stub_kw.len() || !s[..stub_kw.len()].eq_ignore_ascii_case(stub_kw) {
        return None;
    }
    let after = s[stub_kw.len()..].trim_start();
    if after.is_empty() {
        return None;
    }
    // First whitespace-delimited token is the command name.
    let name_end = after
        .find(|c: char| c.is_whitespace())
        .unwrap_or(after.len());
    if name_end == 0 {
        return None;
    }
    let name = &after[..name_end];
    // Reject ``expr-func`` / ``expr-op`` here — those are handled
    // by ``parse_expr_stub``.
    if name.eq_ignore_ascii_case("expr-func") || name.eq_ignore_ascii_case("expr-op") {
        return None;
    }
    let after_name = after[name_end..].trim_start();
    // Python's ``_STUB_RE`` matches ``stub NAME {ARGS}`` with a
    // closing ``}``.  We mirror that — find the matching close,
    // capture the args body, then parse the trailing flag run.
    let after_open = after_name.strip_prefix('{')?;
    let close_rel = after_open.find('}')?;
    let args_body = &after_open[..close_rel];
    let after_close = after_open[close_rel + 1..].trim_start();
    let args = parse_stub_args(args_body)?;
    let flags = parse_stub_flags(after_close);
    Some(super::types::StubCommandDef {
        name: name.to_string(),
        args,
        range,
        barrier: flags.contains("-barrier"),
        r#loop: flags.contains("-loop"),
        pure: flags.contains("-pure"),
        mutator: flags.contains("-mutator"),
        r#unsafe: flags.contains("-unsafe"),
        scope_alias: flags.contains("-scope_alias"),
    })
}

/// Parse the inside of ``stub NAME { … }``.  Returns the
/// argument list, or `None` if any token uses an unrecognised
/// ``:role`` annotation or has an empty name (matches Python's
/// ``_parse_args`` returning ``None`` to drop the whole stub).
/// Empty input yields an empty list.
fn parse_stub_args(args_str: &str) -> Option<Vec<super::types::StubArgDef>> {
    let trimmed = args_str.trim();
    if trimmed.is_empty() {
        return Some(Vec::new());
    }
    let mut result = Vec::new();
    for token in trimmed.split_whitespace() {
        let mut name = token;
        let optional = name.len() >= 2 && name.starts_with('?') && name.ends_with('?');
        if optional {
            name = &name[1..name.len() - 1];
        }
        let (arg_name, role) = if let Some(idx) = name.find(':') {
            let arg_name = &name[..idx];
            let role = &name[idx + 1..];
            let role_lower = role.to_ascii_lowercase();
            if !VALID_STUB_ROLES.iter().any(|r| *r == role_lower.as_str()) {
                return None;
            }
            (arg_name, role_lower)
        } else {
            (name, "value".to_string())
        };
        if arg_name.is_empty() {
            return None;
        }
        result.push(super::types::StubArgDef {
            name: arg_name.to_string(),
            role,
            optional,
        });
    }
    Some(result)
}

/// Parse the trailing ``?-flag…?`` run after the ``{ARGS}`` block.
/// Unrecognised tokens are ignored (matches Python's
/// ``_parse_flags`` filtering on ``_VALID_FLAGS``).
fn parse_stub_flags(flags_str: &str) -> std::collections::HashSet<&'static str> {
    let mut set = std::collections::HashSet::new();
    for token in flags_str.split_whitespace() {
        if let Some(canonical) = VALID_STUB_FLAGS.iter().find(|f| **f == token) {
            set.insert(*canonical);
        }
    }
    set
}

/// Parse a ``stub expr-func NAME ?ARITY?`` / ``stub expr-op NAME
/// ?ARITY?`` line.  Returns ``(kind, name, arity)`` on match.
/// ``arity`` defaults to 1 for functions and 2 for operators when
/// the trailing arity word is absent.  Mirrors Python's
/// ``_EXPR_STUB_RE`` + ``parse_expr_stub_line``.
fn parse_expr_stub(line: &str) -> Option<(&'static str, &str, u32)> {
    let s = line.trim_start();
    let stub_kw = "stub";
    if s.len() < stub_kw.len() || !s[..stub_kw.len()].eq_ignore_ascii_case(stub_kw) {
        return None;
    }
    let after = s[stub_kw.len()..].trim_start();
    let kind: &'static str;
    let default_arity: u32;
    let rest;
    if after.len() >= "expr-func".len()
        && after[.."expr-func".len()].eq_ignore_ascii_case("expr-func")
    {
        kind = "function";
        default_arity = 1;
        rest = after["expr-func".len()..].trim_start();
    } else if after.len() >= "expr-op".len()
        && after[.."expr-op".len()].eq_ignore_ascii_case("expr-op")
    {
        kind = "operator";
        default_arity = 2;
        rest = after["expr-op".len()..].trim_start();
    } else {
        return None;
    }
    if rest.is_empty() {
        return None;
    }
    let name_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    if name_end == 0 {
        return None;
    }
    let name = &rest[..name_end];
    let after_name = rest[name_end..].trim_start();
    let arity = if after_name.is_empty() {
        default_arity
    } else {
        let arity_end = after_name
            .find(|c: char| c.is_whitespace())
            .unwrap_or(after_name.len());
        after_name[..arity_end]
            .parse::<u32>()
            .unwrap_or(default_arity)
    };
    Some((kind, name, arity))
}

/// Decoration-character set for the body-docstring scanner.
/// Mirrors ``_DECORATION_CHARS = frozenset(".-=*~#")`` in
/// ``core/formatting/docstring.py``.
const DECORATION_CHARS: &[char] = &['.', '-', '=', '*', '~', '#'];

/// Extract a leading comment block from a proc body — the
/// fallback that fires when there's no preceding-comment harvest
/// from the segmenter.  Mirrors
/// ``core/formatting/docstring.py::extract_body_docstring``.
///
/// Lines containing only decoration characters
/// (``.-=*~#``) are skipped; remaining ``#``-prefixed lines have
/// the leading hash + whitespace stripped, then accumulated.
/// The first non-comment / non-blank line ends the block.
#[must_use]
pub fn extract_body_docstring(body: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for raw in body.lines() {
        let stripped = raw.trim();
        if stripped.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        if !stripped.starts_with('#') {
            break;
        }
        let text = stripped.trim_start_matches('#').trim().to_string();
        // Skip pure-hash decoration lines (``####`` etc.).
        if text.is_empty() && stripped.chars().all(|c| c == '#') {
            continue;
        }
        // Skip lines made entirely of decoration characters.
        if !text.is_empty() && text.chars().all(|c| DECORATION_CHARS.contains(&c)) {
            continue;
        }
        lines.push(text);
    }
    lines.join("\n")
}

/// Per-line ``# noqa: CODE`` suppression scanner.
///
/// Mirrors the inline-noqa half of
/// `core/analysis/_analyser/_utils.py::parse_per_line_suppression`.
/// Walks every line; for each ``# noqa`` (with or without
/// ``: CODE`` list), records the codes against the 0-based
/// line range (read from the ``preceding_comment`` field of the
/// segmented command).  Mirrors
/// ``core/analysis/_analyser/_core.py`` lines 285-303 — the
/// segmented-command dispatch loop calls this on each command
/// to attach the noqa codes to the *following* command's line
/// range.
///
/// The Python helper uses ``str.lower().find("noqa")`` which is
/// substring-matching against the comment body alone; a comment
/// is only ever the source of a noqa directive, so there's no
/// risk of false-positiving on a ``#`` inside a Tcl string —
/// the segmenter's ``preceding_comment`` field carries comment
/// text only.  Bare ``# noqa`` (no ``: CODE`` list) suppresses
/// every code (`"*"` sentinel); ``# noqa: A, B`` suppresses
/// the named codes.
pub fn apply_preceding_noqa(
    cmd: &crate::segmenter::SegmentedCommand,
    line_offsets: &[usize],
    suppressed_lines: &mut std::collections::HashMap<i32, std::collections::HashSet<String>>,
) {
    let Some(comment) = cmd.preceding_comment.as_deref() else {
        return;
    };
    let lower = comment.to_ascii_lowercase();
    let Some(noqa_pos) = lower.find("noqa") else {
        return;
    };
    let rest = comment[noqa_pos + 4..].trim_start();
    let codes: std::collections::HashSet<String> = if let Some(after_colon) = rest.strip_prefix(':')
    {
        let parsed: std::collections::HashSet<String> = after_colon
            .split([',', ' ', '\t', '\r', '\n'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if parsed.is_empty() {
            std::iter::once("*".to_string()).collect()
        } else {
            parsed
        }
    } else {
        std::iter::once("*".to_string()).collect()
    };
    // Attribute to every line spanned by the command (matches
    // Python's ``range(cmd.range.start.line, cmd.range.end.line +
    // 1)``).  ``SegmentedCommand.span`` is byte offsets; convert
    // each via the precomputed ``line_offsets`` index in
    // ``O(log N)`` instead of a linear scan per call (the helper
    // runs once per segmented command).
    let span_start = cmd.span.start() as usize;
    let span_end = cmd.span.end() as usize;
    let start_line = super::state::line_at_offset(line_offsets, span_start);
    let end_line = super::state::line_at_offset(line_offsets, span_end);
    for line in start_line..=end_line {
        suppressed_lines
            .entry(line)
            .or_default()
            .extend(codes.iter().cloned());
    }
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

    // -- ``parse_command_stub`` + ``parse_expr_stub`` parity tests
    //
    // Mirror ``tests/test_stub_comments.py``'s ``TestParseStubLine``
    // and ``TestParseExprStubLine`` against the Rust port so the
    // ``stub_commands`` / ``stub_expr_defs`` supplement guards can
    // retire (the materialiser now consumes the new ``args`` /
    // flag bools / ``arity`` fields).

    fn cmd_stub(line: &str) -> Option<super::super::types::StubCommandDef> {
        let body = line.trim_start_matches('#').trim_start();
        let after_prefix = strip_tcl_lsp_prefix(body);
        parse_command_stub(after_prefix, Span::new(0, line.len() as u32))
    }

    fn expr_stub(line: &str) -> Option<(&'static str, String, u32)> {
        let body = line.trim_start_matches('#').trim_start();
        let after_prefix = strip_tcl_lsp_prefix(body);
        parse_expr_stub(after_prefix).map(|(k, n, a)| (k, n.to_string(), a))
    }

    #[test]
    fn parse_command_stub_simple_command() {
        let stub = cmd_stub("# tcl-lsp: stub my_cmd {arg1 arg2}").unwrap();
        assert_eq!(stub.name, "my_cmd");
        assert_eq!(stub.args.len(), 2);
        assert_eq!(stub.args[0].name, "arg1");
        assert_eq!(stub.args[0].role, "value");
        assert_eq!(stub.args[1].name, "arg2");
    }

    #[test]
    fn parse_command_stub_with_roles() {
        let stub = cmd_stub(
            "# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop",
        )
        .unwrap();
        assert_eq!(stub.name, "foreach_in_collection");
        assert_eq!(stub.args[0].role, "var");
        assert_eq!(stub.args[1].role, "value");
        assert_eq!(stub.args[2].role, "body");
        assert!(stub.r#loop);
        assert!(!stub.barrier);
    }

    #[test]
    fn parse_command_stub_all_flags() {
        let stub = cmd_stub(
            "# tcl-lsp: stub dangerous {script:body} -barrier -unsafe -mutator -scope_alias",
        )
        .unwrap();
        assert!(stub.barrier);
        assert!(stub.r#unsafe);
        assert!(stub.mutator);
        assert!(stub.scope_alias);
        assert!(!stub.pure);
        assert!(!stub.r#loop);
    }

    #[test]
    fn parse_command_stub_optional_args() {
        let stub = cmd_stub("# tcl-lsp: stub redirect {?-file? target body:body}").unwrap();
        assert!(stub.args[0].optional);
        assert_eq!(stub.args[0].name, "-file");
        assert!(!stub.args[1].optional);
    }

    #[test]
    fn parse_command_stub_bare_format() {
        let stub = cmd_stub("stub get_cells {pattern:pattern} -pure").unwrap();
        assert_eq!(stub.name, "get_cells");
        assert!(stub.pure);
    }

    #[test]
    fn parse_command_stub_invalid_role_returns_none() {
        assert!(cmd_stub("# tcl-lsp: stub bad {arg:invalid_role}").is_none());
    }

    #[test]
    fn parse_command_stub_not_a_stub_returns_none() {
        assert!(cmd_stub("# just a regular comment").is_none());
    }

    #[test]
    fn parse_command_stub_empty_args() {
        let stub = cmd_stub("# tcl-lsp: stub no_args {} -pure").unwrap();
        assert!(stub.args.is_empty());
        assert!(stub.pure);
    }

    #[test]
    fn parse_command_stub_no_command_name_returns_none() {
        assert!(cmd_stub("# tcl-lsp: stub").is_none());
    }

    #[test]
    fn parse_command_stub_unclosed_optional_marker() {
        // A ``?arg`` without closing ``?`` is treated as a
        // regular argument name (matches Python).
        let stub = cmd_stub("# tcl-lsp: stub cmd {?arg}").unwrap();
        assert!(!stub.args[0].optional);
        assert_eq!(stub.args[0].name, "?arg");
    }

    #[test]
    fn parse_command_stub_missing_brace_block_rejected() {
        // Python's ``_STUB_RE`` requires the ``{ARGS}`` block.
        assert!(cmd_stub("# tcl-lsp: stub bare_name").is_none());
    }

    #[test]
    fn parse_expr_stub_func() {
        let (kind, name, arity) = expr_stub("# tcl-lsp: stub expr-func sizeof 1").unwrap();
        assert_eq!(kind, "function");
        assert_eq!(name, "sizeof");
        assert_eq!(arity, 1);
    }

    #[test]
    fn parse_expr_stub_op() {
        let (kind, name, arity) = expr_stub("# tcl-lsp: stub expr-op contains 2").unwrap();
        assert_eq!(kind, "operator");
        assert_eq!(name, "contains");
        assert_eq!(arity, 2);
    }

    #[test]
    fn parse_expr_stub_default_func_arity() {
        let (_, _, arity) = expr_stub("stub expr-func myfunc").unwrap();
        assert_eq!(arity, 1);
    }

    #[test]
    fn parse_expr_stub_default_op_arity() {
        let (_, _, arity) = expr_stub("stub expr-op myop").unwrap();
        assert_eq!(arity, 2);
    }

    #[test]
    fn parse_expr_stub_not_expr_returns_none() {
        assert!(expr_stub("stub get_cells {pattern:pattern}").is_none());
    }

    #[test]
    fn scan_source_for_stubs_populates_args_and_flags() {
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop
# tcl-lsp: stub my_eval {script:body} -barrier
# tcl-lsp: stubs-end
proc foo {} {}
";
        let (cmds, exprs) = scan_source_for_stubs(src);
        assert!(exprs.is_empty());
        assert_eq!(cmds.len(), 2);
        let foreach = cmds
            .iter()
            .find(|c| c.name == "foreach_in_collection")
            .unwrap();
        assert!(foreach.r#loop);
        assert_eq!(foreach.args.len(), 3);
        assert_eq!(foreach.args[0].role, "var");
        assert_eq!(foreach.args[2].role, "body");
        let myeval = cmds.iter().find(|c| c.name == "my_eval").unwrap();
        assert!(myeval.barrier);
        assert!(!myeval.r#loop);
    }

    #[test]
    fn scan_source_for_stubs_carries_expr_arity() {
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub expr-func sizeof 3
# tcl-lsp: stub expr-op contains
# tcl-lsp: stubs-end
";
        let (_, exprs) = scan_source_for_stubs(src);
        assert_eq!(exprs.len(), 2);
        let sizeof = exprs.iter().find(|e| e.name == "sizeof").unwrap();
        assert_eq!(sizeof.kind, "function");
        assert_eq!(sizeof.arity, 3);
        let contains = exprs.iter().find(|e| e.name == "contains").unwrap();
        assert_eq!(contains.kind, "operator");
        assert_eq!(contains.arity, 2);
    }

    #[test]
    fn scan_source_for_stubs_drops_invalid_role() {
        // Matches Python's ``_parse_args`` returning ``None`` —
        // the whole stub is dropped when any token uses an
        // unrecognised role.
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub good {arg:var}
# tcl-lsp: stub bad {arg:bogus}
# tcl-lsp: stubs-end
";
        let (cmds, _) = scan_source_for_stubs(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name, "good");
    }
}
