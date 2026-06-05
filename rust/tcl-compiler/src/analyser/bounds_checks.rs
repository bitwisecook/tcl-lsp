//! Loop-termination index-bounds checks (W230-W242) — GAP-A4.
//!
//! Port of `core/analysis/checks/_bounds.py`.  This module currently
//! lands the **loop-termination** family (W240 / W241) over `while` /
//! `for`; the index-bounds family (W230 / W231 / W232) and the
//! default-off W242 / the `for`-step infinite-loop heuristic are
//! follow-ups.
//!
//! The analysis is intentionally shallow — it inspects the literal text
//! of the condition and body.  A dynamic condition (anything not a
//! constant true/false literal) yields no diagnostic, avoiding false
//! positives.

use tcl_lexer::Token;

use super::types::{Diagnostic, Severity};

/// W240 (constant-false condition → dead body) / W241 (constant-true
/// condition with no `break`/`return`/`error`/`exit` → provably
/// infinite) for `while` / `for`.  `args` / `arg_tokens` exclude the
/// command name.  Mirrors the constant-condition arm of
/// `check_loop_termination`.
pub(crate) fn loop_termination_diagnostics(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[Token],
) -> Vec<Diagnostic> {
    let (cond_text, body_text, cond_tok) = match cmd_name {
        "while" if args.len() >= 2 && arg_tokens.len() >= 2 => {
            (args[0].as_str(), args[1].as_str(), &arg_tokens[0])
        }
        "for" if args.len() >= 4 && arg_tokens.len() >= 4 => {
            (args[1].as_str(), args[3].as_str(), &arg_tokens[1])
        }
        _ => return Vec::new(),
    };

    match condition_constant(cond_text) {
        Some(false) => vec![Diagnostic {
            code: "W240".to_string(),
            span: cond_tok.span,
            message: format!("{cmd_name} condition is constant false; body never executes."),
            severity: Severity::Warning,
            fixes: Vec::new(),
        }],
        Some(true) if !body_may_exit(body_text) => vec![Diagnostic {
            code: "W241".to_string(),
            span: cond_tok.span,
            message: format!(
                "{cmd_name} is provably infinite: condition is constant true and body has no \
                 break/return/error/exit."
            ),
            severity: Severity::Warning,
            fixes: Vec::new(),
        }],
        _ => Vec::new(),
    }
}

/// W230: a constant list literal with a constant out-of-range index
/// (`lindex`) or a provably-empty slice (`lrange` / `lreplace`)
/// silently returns empty / clamps.  `args` / `arg_tokens` exclude the
/// command name.  Mirrors `check_list_index_out_of_range` (W231 `lset`
/// needs const-var tracking and is a follow-up).
#[allow(clippy::similar_names)] // first_text/first_tok/first_val read clearly
pub(crate) fn list_index_diagnostics(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[Token],
) -> Vec<Diagnostic> {
    if !matches!(cmd_name, "lindex" | "lrange" | "lreplace")
        || args.len() < 2
        || arg_tokens.len() < 2
    {
        return Vec::new();
    }
    let list_tok = &arg_tokens[0];
    if !is_braced_or_esc(list_tok) || has_subst(&args[0], list_tok) {
        return Vec::new();
    }
    let length = i64::try_from(crate::tcl_expr_eval::split_tcl_list(strip_braces(&args[0])).len())
        .unwrap_or(i64::MAX);

    if cmd_name == "lindex" {
        return lindex_diagnostics(args, arg_tokens, length);
    }

    // lrange / lreplace: a (first, last) pair that resolves to an empty
    // slice.
    if args.len() < 3 || arg_tokens.len() < 3 || (cmd_name == "lrange" && args.len() != 3) {
        return Vec::new();
    }
    let (first_text, last_text) = (&args[1], &args[2]);
    let (first_tok, last_tok) = (&arg_tokens[1], &arg_tokens[2]);
    if has_subst(first_text, first_tok)
        || !is_literal_index(first_text)
        || has_subst(last_text, last_tok)
        || !is_literal_index(last_text)
    {
        return Vec::new();
    }
    let (Some(first_val), Some(last_val)) = (
        resolve_index(first_text, length),
        resolve_index(last_text, length),
    ) else {
        return Vec::new();
    };
    if !pair_slice_empty(first_val, last_val, length) {
        return Vec::new();
    }
    let verb = if cmd_name == "lrange" {
        "lrange slice is empty".to_string()
    } else if first_val < 0 && last_val < 0 {
        "lreplace prepends instead of replacing (both indices resolve before the list)".to_string()
    } else if first_val >= length && last_val >= length {
        "lreplace appends instead of replacing (both indices resolve past the list)".to_string()
    } else {
        "lreplace touches no element (first > last after clamping)".to_string()
    };
    vec![Diagnostic {
        code: "W230".to_string(),
        span: tcl_lexer::Span::new(first_tok.span.start(), last_tok.span.end()),
        message: format!(
            "{verb}: first='{first_text}' resolves to {first_val}, last='{last_text}' resolves \
             to {last_val} (list has {length} element{}).",
            if length == 1 { "" } else { "s" }
        ),
        severity: Severity::Warning,
        fixes: Vec::new(),
    }]
}

/// The per-index `lindex` arm of W230.
fn lindex_diagnostics(args: &[String], arg_tokens: &[Token], length: i64) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (pos, idx_text) in args.iter().enumerate().skip(1) {
        let Some(idx_tok) = arg_tokens.get(pos) else {
            continue;
        };
        if has_subst(idx_text, idx_tok) || !is_literal_index(idx_text) {
            continue;
        }
        let Some(resolved) = resolve_index(idx_text, length) else {
            continue;
        };
        if (0..length).contains(&resolved) {
            continue;
        }
        out.push(Diagnostic {
            code: "W230".to_string(),
            span: idx_tok.span,
            message: format!(
                "Index '{}' {}; lindex silently returns empty string.",
                idx_text.trim(),
                describe_index(resolved, length)
            ),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }
    out
}

/// True when a `(first, last)` index pair resolves to a provably-empty
/// slice over `length`: both below, both above, clamped first > clamped
/// last, or an empty container.  Mirrors the shared clamp logic in
/// `check_list_index_out_of_range` / `check_string_index_out_of_range`.
fn pair_slice_empty(first: i64, last: i64, length: i64) -> bool {
    if length == 0 {
        return true;
    }
    let both_below = first < 0 && last < 0;
    let both_above = first >= length && last >= length;
    let clamped_first = first.clamp(0, length - 1);
    let clamped_last = last.clamp(0, length - 1);
    both_below || both_above || clamped_first > clamped_last
}

/// W232: a constant `string index` / `range` / `replace` / `insert`
/// into a literal string with a constant out-of-range (or negative)
/// index returns empty / is a no-op.  Mirrors
/// `check_string_index_out_of_range`.
pub(crate) fn string_index_diagnostics(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[Token],
) -> Vec<Diagnostic> {
    if cmd_name != "string" || args.len() < 2 {
        return Vec::new();
    }
    let sub = args[0].as_str();
    let min_args = match sub {
        "index" => 3,
        "range" | "replace" | "insert" => 4,
        _ => return Vec::new(),
    };
    if args.len() < min_args || arg_tokens.len() < min_args {
        return Vec::new();
    }
    let str_tok = &arg_tokens[1];
    let str_text = &args[1];
    // Safe runtime length: braced strings do no backslash processing;
    // a backslash-free ESC word is byte-identical to its runtime form.
    // Back off (None) otherwise.
    let str_len: Option<i64> = if has_subst(str_text, str_tok) || !is_braced_or_esc(str_tok) {
        None
    } else if str_tok.kind == tcl_lexer::TokenType::Str {
        i64::try_from(strip_braces(str_text).chars().count()).ok()
    } else if str_text.contains('\\') {
        None
    } else {
        i64::try_from(str_text.chars().count()).ok()
    };

    if sub == "index" || sub == "insert" {
        return string_single_index(sub, args, arg_tokens, str_len);
    }
    string_pair_index(sub, args, arg_tokens, str_len)
}

/// The single-index `string index` / `string insert` arm of W232.
fn string_single_index(
    sub: &str,
    args: &[String],
    arg_tokens: &[Token],
    str_len: Option<i64>,
) -> Vec<Diagnostic> {
    let (idx_text, idx_tok) = (&args[2], &arg_tokens[2]);
    if has_subst(idx_text, idx_tok) || !is_literal_index(idx_text) {
        return Vec::new();
    }
    let stripped = idx_text.trim();
    // A plain negative literal is always invalid.
    if let Some(n) = parse_strict_int(stripped) {
        if n < 0 {
            return vec![Diagnostic {
                code: "W232".to_string(),
                span: idx_tok.span,
                message: format!(
                    "string {sub}: index '{stripped}' is negative; result is empty or a no-op."
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            }];
        }
    }
    // `string insert` clamps other overshoots; only `string index`
    // flags an in-bounds miss.
    if sub == "index" {
        if let Some(len) = str_len {
            if let Some(resolved) = resolve_index(stripped, len) {
                if !(0..len).contains(&resolved) {
                    return vec![Diagnostic {
                        code: "W232".to_string(),
                        span: idx_tok.span,
                        message: format!(
                            "string index: '{stripped}' {}; returns empty string.",
                            describe_index_string(resolved, len)
                        ),
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    }];
                }
            }
        }
    }
    Vec::new()
}

/// The `string range` / `string replace` `(first, last)` arm of W232.
fn string_pair_index(
    sub: &str,
    args: &[String],
    arg_tokens: &[Token],
    str_len: Option<i64>,
) -> Vec<Diagnostic> {
    let (first_text, last_text) = (&args[2], &args[3]);
    let (first_tok, last_tok) = (&arg_tokens[2], &arg_tokens[3]);
    if has_subst(first_text, first_tok)
        || !is_literal_index(first_text)
        || has_subst(last_text, last_tok)
        || !is_literal_index(last_text)
    {
        return Vec::new();
    }
    let verb = if sub == "range" {
        "slice is empty"
    } else {
        "replace is a no-op"
    };
    let span = tcl_lexer::Span::new(first_tok.span.start(), last_tok.span.end());

    // Both plain negative literals → always empty, even for a dynamic
    // string.
    if let (Some(f), Some(l)) = (
        parse_strict_int(first_text.trim()),
        parse_strict_int(last_text.trim()),
    ) {
        if f < 0 && l < 0 {
            return vec![Diagnostic {
                code: "W232".to_string(),
                span,
                message: format!(
                    "string {sub}: both indices are negative ('{first_text}', '{last_text}'); \
                     {verb}."
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            }];
        }
    }
    let Some(len) = str_len else {
        return Vec::new();
    };
    let (Some(first_val), Some(last_val)) = (
        resolve_index(first_text, len),
        resolve_index(last_text, len),
    ) else {
        return Vec::new();
    };
    if !pair_slice_empty(first_val, last_val, len) {
        return Vec::new();
    }
    vec![Diagnostic {
        code: "W232".to_string(),
        span,
        message: format!(
            "string {sub}: {verb}: first='{first_text}' resolves to {first_val}, \
             last='{last_text}' resolves to {last_val} (string has {len} character{}).",
            if len == 1 { "" } else { "s" }
        ),
        severity: Severity::Warning,
        fixes: Vec::new(),
    }]
}

/// Human-readable description of a resolved out-of-range string index.
/// Mirrors `_describe_index_string`.
fn describe_index_string(resolved: i64, length: i64) -> String {
    if resolved < 0 {
        format!("resolves to {resolved} (before start of string)")
    } else {
        format!(
            "resolves to {resolved} (string has {length} character{})",
            if length == 1 { "" } else { "s" }
        )
    }
}

/// Token is a literal word (braced string or plain word).  Mirrors
/// `_is_braced_or_esc`.
fn is_braced_or_esc(tok: &Token) -> bool {
    matches!(
        tok.kind,
        tcl_lexer::TokenType::Str | tcl_lexer::TokenType::Esc
    )
}

/// Word contains a variable / command substitution.  Mirrors
/// `_has_subst`.
fn has_subst(text: &str, tok: &Token) -> bool {
    matches!(
        tok.kind,
        tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
    ) || text.contains('$')
        || text.contains('[')
}

/// `s` is a constant index expression we can evaluate (`end`, an
/// integer, or `end±N`).  Mirrors `_is_literal_index`.
fn is_literal_index(s: &str) -> bool {
    let s = s.trim();
    s == "end" || parse_strict_int(s).is_some() || end_offset(s).is_some()
}

/// Resolve a constant index to an absolute offset given `length`, or
/// `None` when `s` is not a literal index.  Mirrors `_resolve_index`.
fn resolve_index(s: &str, length: i64) -> Option<i64> {
    let s = s.trim();
    if s == "end" {
        return Some(length - 1);
    }
    if let Some((sign, n)) = end_offset(s) {
        return Some(length - 1 + sign * n);
    }
    parse_strict_int(s)
}

/// Parse a strict `-?\d+` integer (no `+` sign, matching `_INT_RE`).
fn parse_strict_int(s: &str) -> Option<i64> {
    let body = s.strip_prefix('-').unwrap_or(s);
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse::<i64>().ok()
}

/// Parse an `end-N` / `end+N` offset, returning `(sign, n)` where sign
/// is -1 / +1.  Mirrors `_END_MINUS_RE` / `_END_PLUS_RE` (whitespace
/// around the operator allowed).
fn end_offset(s: &str) -> Option<(i64, i64)> {
    let rest = s.strip_prefix("end")?.trim_start();
    let (sign, digits) = if let Some(d) = rest.strip_prefix('-') {
        (-1, d)
    } else if let Some(d) = rest.strip_prefix('+') {
        (1, d)
    } else {
        return None;
    };
    let digits = digits.trim_start();
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<i64>().ok().map(|n| (sign, n))
}

/// Human-readable description of a resolved out-of-range index.
/// Mirrors `_describe_index`.
fn describe_index(resolved: i64, length: i64) -> String {
    if resolved < 0 {
        format!("resolves to {resolved} (before start of list)")
    } else {
        format!(
            "resolves to {resolved} (list has {length} element{})",
            if length == 1 { "" } else { "s" }
        )
    }
}

const TRUE_LITERALS: &[&str] = &["1", "true", "yes", "on", "!0"];
const FALSE_LITERALS: &[&str] = &["0", "false", "no", "off", "!1"];

/// Strip a single enclosing `{ … }` and surrounding whitespace.
/// Mirrors `_strip_braces`.
fn strip_braces(s: &str) -> &str {
    let t = s.trim();
    if t.starts_with('{') && t.ends_with('}') && t.len() >= 2 {
        t[1..t.len() - 1].trim()
    } else {
        t
    }
}

/// `Some(true)` / `Some(false)` when `cond` is a constant-true /
/// constant-false expression; `None` when dynamic.  Mirrors
/// `_condition_constant`.
fn condition_constant(cond: &str) -> Option<bool> {
    let c = strip_braces(cond).to_ascii_lowercase();
    if TRUE_LITERALS.contains(&c.as_str()) {
        return Some(true);
    }
    if FALSE_LITERALS.contains(&c.as_str()) {
        return Some(false);
    }
    c.trim().parse::<f64>().ok().map(|v| v != 0.0)
}

/// True when `body` contains a `break` / `return` / `error` / `exit`
/// keyword (shallow scan).  Mirrors `_body_may_exit` /
/// `_BREAK_RE = (?<![\w:-])(break|return|error|exit)\b` — Rust regex
/// has no look-behind, so the `:` / `-` prefix exclusion is applied as
/// a post-filter on each `\b`-anchored match.
fn body_may_exit(body: &str) -> bool {
    let re = regex::Regex::new(r"\b(break|return|error|exit)\b").expect("valid keyword regex");
    let bytes = body.as_bytes();
    for m in re.find_iter(body) {
        let start = m.start();
        if start == 0 || !matches!(bytes[start - 1], b':' | b'-') {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;

    fn codes(src: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W240" | "W241"))
            .map(|d| d.code.clone())
            .collect()
    }

    #[test]
    fn w240_constant_false_condition() {
        assert_eq!(codes("while 0 {puts hi}\n"), vec!["W240"]);
        assert_eq!(codes("for {set i 0} 0 {incr i} {}\n"), vec!["W240"]);
    }

    #[test]
    fn w241_constant_true_no_exit() {
        assert_eq!(codes("while 1 {puts hi}\n"), vec!["W241"]);
        // A `break` in the body suppresses W241.
        assert!(codes("while 1 {break}\n").is_empty());
        assert!(codes("while 1 {return}\n").is_empty());
    }

    #[test]
    fn dynamic_condition_is_silent() {
        assert!(codes("while {$x < 10} {incr x}\n").is_empty());
    }

    fn idx_codes(src: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W230" | "W232"))
            .map(|d| d.code.clone())
            .collect()
    }

    #[test]
    fn w230_list_index_out_of_range() {
        assert_eq!(idx_codes("lindex {a b c} 5\n"), vec!["W230"]);
        assert_eq!(idx_codes("lindex {a b c} -1\n"), vec!["W230"]);
        assert_eq!(idx_codes("lindex {a b c} end-5\n"), vec!["W230"]);
        // In range and dynamic list → none.
        assert!(idx_codes("lindex {a b c} 1\n").is_empty());
        assert!(idx_codes("lindex {a b c} end\n").is_empty());
        assert!(idx_codes("lindex $x 5\n").is_empty());
    }

    #[test]
    fn w232_string_index_out_of_range() {
        assert_eq!(idx_codes("string index abc 10\n"), vec!["W232"]);
        assert_eq!(idx_codes("string index abc -1\n"), vec!["W232"]);
        assert!(idx_codes("string index abc 1\n").is_empty());
        assert!(idx_codes("string index abc end\n").is_empty());
    }

    #[test]
    fn w230_lrange_lreplace_empty_slice() {
        assert_eq!(idx_codes("lrange {a b c} 5 7\n"), vec!["W230"]);
        assert_eq!(idx_codes("lrange {a b c} -3 -1\n"), vec!["W230"]);
        assert_eq!(idx_codes("lrange {a b c} 2 0\n"), vec!["W230"]); // clamped first>last
        assert_eq!(idx_codes("lreplace {a b c} 5 7 X\n"), vec!["W230"]);
        assert!(idx_codes("lrange {a b c} 0 1\n").is_empty());
    }

    #[test]
    fn w232_string_range_replace_empty_slice() {
        assert_eq!(idx_codes("string range abc 5 7\n"), vec!["W232"]);
        assert_eq!(idx_codes("string range abc -3 -1\n"), vec!["W232"]);
        assert_eq!(idx_codes("string replace abc 5 7 X\n"), vec!["W232"]);
        assert!(idx_codes("string range abc 0 1\n").is_empty());
    }

    #[test]
    fn pair_slice_empty_logic() {
        assert!(super::pair_slice_empty(5, 7, 3)); // both above
        assert!(super::pair_slice_empty(-3, -1, 3)); // both below
        assert!(super::pair_slice_empty(2, 0, 3)); // clamped first>last
        assert!(super::pair_slice_empty(0, 0, 0)); // empty container
        assert!(!super::pair_slice_empty(0, 1, 3)); // valid
    }

    #[test]
    fn index_helpers() {
        assert_eq!(super::resolve_index("end", 3), Some(2));
        assert_eq!(super::resolve_index("end-5", 3), Some(-3));
        assert_eq!(super::resolve_index("end+1", 3), Some(3));
        assert_eq!(super::resolve_index("2", 3), Some(2));
        assert_eq!(super::resolve_index("$x", 3), None);
        assert!(super::is_literal_index("end-2"));
        assert!(!super::is_literal_index("+5")); // strict: no leading +
    }

    #[test]
    fn condition_constant_classifies() {
        assert_eq!(super::condition_constant("0"), Some(false));
        assert_eq!(super::condition_constant("1"), Some(true));
        assert_eq!(super::condition_constant("{true}"), Some(true));
        assert_eq!(super::condition_constant("$x < 10"), None);
    }
}
