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

use std::collections::{HashMap, HashSet};

use super::ctx::{FactoryCandidate, ScanCtx};
use super::handlers::qualify;
use super::types::SignatureProc;
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

/// Resolve `cand.head` to a factory's qualified name (the **key**
/// in `factories`), following Tcl's command-resolution order.
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
) -> Option<&'a str> {
    let head = &cand.head;
    if head.starts_with("::") {
        return factories
            .get_key_value(head.as_str())
            .map(|(k, _)| k.as_str());
    }
    let qualified = qualify(&cand.ns_prefix, head);
    if let Some((k, _)) = factories.get_key_value(qualified.as_str()) {
        return Some(k.as_str());
    }
    let global_q = format!("::{head}");
    if global_q != qualified {
        return factories
            .get_key_value(global_q.as_str())
            .map(|(k, _)| k.as_str());
    }
    None
}

/// Emit synthetic [`SignatureProc`] records for each factory-wrapper
/// call site.
///
/// Mirrors `_resolve_factory_defs` in
/// `core/analysis/signature_scan.py`. Builds a factory map from
/// `ctx.proc_bodies` filtered through [`is_factory_body`]; for
/// each candidate calls [`lookup_factory`], computes the emitted
/// qualified name (honouring an explicit `::` prefix on the
/// candidate name), and idempotently inserts a synthetic
/// [`SignatureProc`] with empty params (the wrapper-to-factory
/// argument-position map is not statically known, so we just
/// record a jump target).
pub(super) fn resolve_factory_defs(ctx: &mut ScanCtx) {
    if ctx.candidates.is_empty() || ctx.proc_bodies.is_empty() {
        return;
    }
    let mut factories: HashMap<String, String> = HashMap::new();
    for info in &ctx.proc_bodies {
        if !is_factory_body(&info.body_text, &info.params) {
            continue;
        }
        factories.insert(info.qname.clone(), info.ns_prefix.clone());
    }
    if factories.is_empty() {
        return;
    }
    // Snapshot candidates so we can mutate ctx.result.procs while
    // iterating. Candidates is a small Vec; cloning is cheap.
    let candidates = ctx.candidates.clone();
    for cand in &candidates {
        let Some(factory_qname) = lookup_factory(cand, &factories) else {
            continue;
        };
        let factory_ns = &factories[factory_qname];
        let emitted_q = if cand.name.starts_with("::") {
            cand.name.clone()
        } else if !factory_ns.is_empty() {
            format!("::{factory_ns}::{}", cand.name)
        } else {
            format!("::{}", cand.name)
        };
        let simple = emitted_q.rsplit("::").next().unwrap_or("").to_string();
        if ctx.result.procs.contains_key(&emitted_q) {
            continue;
        }
        ctx.result.procs.insert(
            emitted_q.clone(),
            SignatureProc {
                name: simple,
                qualified_name: emitted_q,
                params: Vec::new(),
                name_range: cand.name_tok.span,
                body_range: cand.body_tok.span,
            },
        );
    }
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
        assert_eq!(lookup_factory(&c, &f), Some("::foo::DEFC"));
    }

    #[test]
    fn lookup_call_namespace_qualified_first() {
        // Both `::ns::DEFC` and `::DEFC` exist; relative call from
        // `ns` should resolve to the call-site one.
        let f = factories(&[("::ns::DEFC", "ns_home"), ("::DEFC", "global_home")]);
        let c = cand("DEFC", "ns");
        assert_eq!(lookup_factory(&c, &f), Some("::ns::DEFC"));
    }

    #[test]
    fn lookup_global_fallback_when_call_ns_misses() {
        let f = factories(&[("::DEFC", "global_home")]);
        let c = cand("DEFC", "ns");
        assert_eq!(lookup_factory(&c, &f), Some("::DEFC"));
    }

    #[test]
    fn lookup_cross_namespace_never_falls_through() {
        // A factory under `::other::DEFC` should NOT resolve a
        // bare `DEFC` call from namespace `ns`.
        let f = factories(&[("::other::DEFC", "other_home")]);
        let c = cand("DEFC", "ns");
        assert!(lookup_factory(&c, &f).is_none());
    }

    use super::super::ctx::ProcBodyInfo;

    fn proc_body(qname: &str, ns: &str, body: &str) -> ProcBodyInfo {
        ProcBodyInfo {
            qname: qname.to_string(),
            params: vec!["name".to_string(), "args".to_string(), "body".to_string()],
            body_text: body.to_string(),
            ns_prefix: ns.to_string(),
        }
    }

    #[test]
    fn resolve_emits_synthetic_proc_for_tcllib_defc() {
        let mut ctx = ScanCtx::default();
        ctx.proc_bodies.push(proc_body(
            "::tcllib::DEFC",
            "tcllib",
            "proc $name $args $body",
        ));
        ctx.candidates.push(FactoryCandidate {
            head: "DEFC".to_string(),
            name: "Foo".to_string(),
            name_tok: Token::with_content_offset(TokenType::Esc, Span::new(10, 13), 0),
            body_tok: Token::with_content_offset(TokenType::Str, Span::new(20, 30), 1),
            ns_prefix: "tcllib".to_string(),
        });
        resolve_factory_defs(&mut ctx);
        let proc = ctx.result.procs.get("::tcllib::Foo").expect("emitted");
        assert_eq!(proc.name, "Foo");
        assert!(proc.params.is_empty());
        assert_eq!(proc.name_range, Span::new(10, 13));
        assert_eq!(proc.body_range, Span::new(20, 30));
    }

    #[test]
    fn resolve_no_factories_no_op() {
        let mut ctx = ScanCtx::default();
        // proc_bodies present but no factory body shape.
        ctx.proc_bodies
            .push(proc_body("::ns::regular", "ns", "set x 1"));
        ctx.candidates.push(FactoryCandidate {
            head: "regular".to_string(),
            name: "X".to_string(),
            name_tok: Token::new(TokenType::Esc, Span::new(0, 0)),
            body_tok: Token::with_content_offset(TokenType::Str, Span::new(0, 0), 1),
            ns_prefix: "ns".to_string(),
        });
        resolve_factory_defs(&mut ctx);
        assert!(ctx.result.procs.is_empty());
    }

    #[test]
    fn resolve_idempotent_re_emit_guard() {
        let mut ctx = ScanCtx::default();
        ctx.proc_bodies
            .push(proc_body("::DEFC", "", "proc $name $args $body"));
        // Pre-populate the result with a `::Foo` proc — the synthetic
        // emit must not overwrite it.
        ctx.result.procs.insert(
            "::Foo".to_string(),
            SignatureProc {
                name: "Foo".to_string(),
                qualified_name: "::Foo".to_string(),
                params: vec![super::super::types::ParamDef {
                    name: "real".to_string(),
                    has_default: false,
                    default_value: None,
                }],
                name_range: Span::new(99, 102),
                body_range: Span::new(110, 120),
            },
        );
        ctx.candidates.push(FactoryCandidate {
            head: "DEFC".to_string(),
            name: "Foo".to_string(),
            name_tok: Token::new(TokenType::Esc, Span::new(0, 3)),
            body_tok: Token::with_content_offset(TokenType::Str, Span::new(10, 20), 1),
            ns_prefix: String::new(),
        });
        resolve_factory_defs(&mut ctx);
        // Original record preserved.
        let proc = ctx.result.procs.get("::Foo").expect("present");
        assert_eq!(proc.params.len(), 1);
        assert_eq!(proc.name_range, Span::new(99, 102));
    }

    #[test]
    fn resolve_candidate_with_absolute_name() {
        let mut ctx = ScanCtx::default();
        ctx.proc_bodies.push(proc_body(
            "::tcllib::DEFC",
            "tcllib",
            "proc $name $args $body",
        ));
        ctx.candidates.push(FactoryCandidate {
            head: "DEFC".to_string(),
            name: "::Top::Foo".to_string(),
            name_tok: Token::new(TokenType::Esc, Span::new(0, 10)),
            body_tok: Token::with_content_offset(TokenType::Str, Span::new(11, 20), 1),
            ns_prefix: "tcllib".to_string(),
        });
        resolve_factory_defs(&mut ctx);
        // Absolute name preserved; not under tcllib::Foo.
        let proc = ctx.result.procs.get("::Top::Foo").expect("present");
        assert_eq!(proc.name, "Foo");
        assert!(!ctx.result.procs.contains_key("::tcllib::Foo"));
    }
}
