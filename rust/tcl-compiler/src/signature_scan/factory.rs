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

use std::collections::HashSet;

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
}
