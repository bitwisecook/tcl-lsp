//! Second-pass factory-wrapper resolution.
//!
//! After the main walker collects every candidate four-token call
//! (in `ctx.candidates`) and every proc body's text + params (in
//! `ctx.proc_bodies`), the resolver:
//!
//! 1. classifies each proc body as a real factory wrapper iff its
//!    body contains a `proc $a $b $c` shape using the wrapper's own
//!    parameters ([`is_factory_body`]);
//! 2. for each candidate, looks up the factory it binds to using
//!    Tcl's command-resolution path ([`lookup_factory`]);
//! 3. emits a synthetic [`SignatureProc`] under the factory's home
//!    namespace ([`resolve_factory_defs`]).
//!
//! Subsequent C40d sub-strips fill in items 2 and 3.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use super::ctx::FactoryCandidate;
use super::handlers::qualify;
use crate::segmenter::segment_commands;

/// Return `true` when `body_text` contains a top-level
/// `proc $p1 $p2 $p3` command using exactly the wrapper's three
/// parameters in some order.
///
/// Mirrors `_is_factory_body` in
/// `core/analysis/signature_scan.py`. Both `$name` and `${name}`
/// substitution forms are accepted (the segmenter reconstructs
/// substitutions as `${name}`, but bare `$name` still matches the
/// equality check). Wrappers with fewer than three parameters
/// cannot match the canonical `proc $name $args $body` shape.
#[must_use]
pub(super) fn is_factory_body(body_text: &str, params: &[String]) -> bool {
    if params.len() < 3 {
        return false;
    }
    let mut param_vars: HashSet<String> = HashSet::with_capacity(params.len() * 2);
    for p in params {
        param_vars.insert(format!("${p}"));
        param_vars.insert(format!("${{{p}}}"));
    }
    let commands = segment_commands(body_text);
    for cmd in commands {
        if cmd.is_partial || cmd.texts.is_empty() {
            continue;
        }
        let t = &cmd.texts;
        if t[0] != "proc" || t.len() != 4 {
            continue;
        }
        if param_vars.contains(&t[1]) && param_vars.contains(&t[2]) && param_vars.contains(&t[3]) {
            return true;
        }
    }
    false
}

/// Resolve `cand.head` to a factory's qualified name, following
/// Tcl's command-resolution order.
///
/// Mirrors `_lookup_factory` in
/// `core/analysis/signature_scan.py`. Absolute heads (those
/// starting with `::`) match verbatim. Relative heads try the
/// call-site qualified name first, then the global namespace —
/// they never fall through to "any factory with this bare name",
/// which would bind calls in one namespace to a wrapper in an
/// unrelated one (Tcl itself refuses to cross those boundaries).
#[must_use]
pub(super) fn lookup_factory<'a>(
    cand: &FactoryCandidate,
    factories: &'a HashMap<String, String>,
) -> Option<&'a String> {
    let head = &cand.head;
    if head.starts_with("::") {
        return factories.get(head);
    }
    let qualified = qualify(&cand.ns_prefix, head);
    if let Some(v) = factories.get(&qualified) {
        return Some(v);
    }
    let global_q = format!("::{head}");
    if global_q != qualified {
        return factories.get(&global_q);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_factory_body_matches() {
        let body = "proc $name $args $body";
        let params = vec!["name".to_string(), "args".to_string(), "body".to_string()];
        assert!(is_factory_body(body, &params));
    }

    #[test]
    fn wrong_arity_rejected() {
        // `proc $name $args` only — three tokens, not four.
        let body = "proc $name $args";
        let params = vec!["name".to_string(), "args".to_string(), "body".to_string()];
        assert!(!is_factory_body(body, &params));
    }

    #[test]
    fn non_variable_arg_disqualifies() {
        // The middle arg is a literal `foo`, not a variable.
        let body = "proc $name foo $body";
        let params = vec!["name".to_string(), "args".to_string(), "body".to_string()];
        assert!(!is_factory_body(body, &params));
    }

    #[test]
    fn no_proc_statement_no_match() {
        let body = "set x 1; return $x";
        let params = vec!["name".to_string(), "args".to_string(), "body".to_string()];
        assert!(!is_factory_body(body, &params));
    }

    #[test]
    fn fewer_than_three_params_no_match() {
        let body = "proc $name $args $body";
        let params = vec!["name".to_string(), "args".to_string()];
        assert!(!is_factory_body(body, &params));
    }

    use tcl_lexer::{Span, Token, TokenType};

    fn cand(head: &str, ns: &str) -> FactoryCandidate {
        FactoryCandidate {
            head: head.to_string(),
            name: "X".to_string(),
            name_tok: Token::new(TokenType::Esc, Span::new(0, 0)),
            body_tok: Token::with_content_offset(TokenType::Str, Span::new(0, 0), 1),
            ns_prefix: ns.to_string(),
        }
    }

    fn factories(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn lookup_absolute_head_matches_verbatim() {
        let f = factories(&[("::foo::DEFC", "foo")]);
        let c = cand("::foo::DEFC", "anywhere");
        assert_eq!(lookup_factory(&c, &f).map(String::as_str), Some("foo"));
    }

    #[test]
    fn lookup_call_namespace_qualified_first() {
        // Both `::ns::DEFC` and `::DEFC` exist; relative call from
        // `ns` should resolve to the call-site one.
        let f = factories(&[("::ns::DEFC", "ns_home"), ("::DEFC", "global_home")]);
        let c = cand("DEFC", "ns");
        assert_eq!(lookup_factory(&c, &f).map(String::as_str), Some("ns_home"));
    }

    #[test]
    fn lookup_global_fallback_when_call_ns_misses() {
        let f = factories(&[("::DEFC", "global_home")]);
        let c = cand("DEFC", "ns");
        assert_eq!(
            lookup_factory(&c, &f).map(String::as_str),
            Some("global_home"),
        );
    }

    #[test]
    fn lookup_cross_namespace_never_falls_through() {
        // A factory under `::other::DEFC` should NOT resolve a
        // bare `DEFC` call from namespace `ns`.
        let f = factories(&[("::other::DEFC", "other_home")]);
        let c = cand("DEFC", "ns");
        assert!(lookup_factory(&c, &f).is_none());
    }
}
