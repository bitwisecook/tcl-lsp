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

    #[test]
    fn condition_constant_classifies() {
        assert_eq!(super::condition_constant("0"), Some(false));
        assert_eq!(super::condition_constant("1"), Some(true));
        assert_eq!(super::condition_constant("{true}"), Some(true));
        assert_eq!(super::condition_constant("$x < 10"), None);
    }
}
