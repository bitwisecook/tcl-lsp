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

//! Second-pass factory-wrapper resolution.
//!
//! After the main walker collects every candidate four-token call
//! (in `ctx.candidates`) and every proc body's text + params (in
//! `ctx.proc_bodies`), [`resolve_factory_defs`]:
//!
//! 1. classifies each proc body as a real factory wrapper iff its
//!    body contains a `proc $a $b $c` shape using the wrapper's own
//!    parameters ([`is_factory_body`]);
//! 2. for each candidate, looks up the factory it binds to using
//!    the original authored slot and retained caller scope;
//! 3. emits a synthetic [`SignatureProc`] under the factory's home
//!    namespace, idempotently skipping any qualified name already
//!    present in [`super::ctx::ScanCtx::result`]`.procs`.
//!
//! [`SignatureProc`]: super::types::SignatureProc

#[cfg(test)]
use std::collections::HashMap;
use std::collections::HashSet;

#[cfg(test)]
use super::ctx::FactoryCandidate;
use super::ctx::ScanCtx;
use super::types::SignatureProc;
use crate::segmenter::segment_commands_with_offset_and_config;

/// Return `true` when `body_text` contains a top-level
/// `proc $p1 $p2 $p3` command using exactly the wrapper's three
/// parameters in some order.
///
/// Both `$name` and `${name}`
/// substitution forms are accepted (the segmenter reconstructs
/// substitutions as `${name}`, but bare `$name` still matches the
/// equality check). Wrappers with fewer than three parameters
/// cannot match the canonical `proc $name $args $body` shape.
#[must_use]
pub(super) fn is_factory_body(
    body_text: &str,
    params: &[String],
    config: tcl_lexer::LexerConfig,
) -> bool {
    if params.len() < 3 {
        return false;
    }
    let mut param_vars: HashSet<String> = HashSet::with_capacity(params.len() * 2);
    for p in params {
        param_vars.insert(format!("${p}"));
        param_vars.insert(format!("${{{p}}}"));
    }
    let commands = segment_commands_with_offset_and_config(body_text, 0, config);
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
/// Absolute written heads use the selected naming recipe. Relative heads try the
/// call-site qualified name first, then the global namespace —
/// they never fall through to "any factory with this bare name",
/// which would bind calls in one namespace to a wrapper in an
/// unrelated one (Tcl itself refuses to cross those boundaries).
#[must_use]
#[cfg(test)]
pub(super) fn lookup_factory<'a>(
    cand: &FactoryCandidate,
    factories: &'a HashMap<String, String>,
) -> Option<&'a str> {
    for key in ScanCtx::command_keys(&cand.ns_prefix, &cand.head) {
        if let Some((key, _)) = factories.get_key_value(&key) {
            return Some(key.as_str());
        }
    }
    None
}

/// Emit synthetic [`SignatureProc`] records for each factory-wrapper
/// call site.
///
/// Builds a factory map from
/// `ctx.proc_bodies` filtered through [`is_factory_body`]; for
/// each candidate selects the retained source slot, computes the emitted
/// qualified name (honouring an explicit `::` prefix on the
/// candidate name), and idempotently inserts a synthetic
/// [`SignatureProc`] with empty params (the wrapper-to-factory
/// argument-position map is not statically known, so we just
/// record a jump target).
pub(super) fn resolve_factory_defs(ctx: &mut ScanCtx) {
    if ctx.candidates.is_empty() || ctx.proc_bodies.is_empty() {
        return;
    }
    let factories = ctx
        .proc_bodies
        .iter()
        .filter(|info| is_factory_body(&info.body_text, &info.params, ctx.config))
        .cloned()
        .collect::<Vec<_>>();
    let candidates = ctx.candidates.clone();
    for cand in &candidates {
        let Some(scope) = cand
            .namespace_scope
            .clone()
            .or_else(|| ctx.current_namespace(&cand.ns_prefix))
        else {
            continue;
        };
        let root = super::scope::SignatureNamespaceScope::root(ctx.name_policy());
        let factory = [&scope, &root].into_iter().find_map(|lookup| {
            factories.iter().find(|info| match &info.source_name {
                Some(name) => name.matches_written(lookup, &cand.head),
                None => lookup.display().is_some_and(|namespace| {
                    crate::naming::qualify(&namespace, &cand.head) == info.qname
                }),
            })
        });
        let Some(factory) = factory else {
            continue;
        };
        let Some(factory_scope) = factory
            .namespace_scope
            .clone()
            .or_else(|| ctx.current_namespace(&factory.ns_prefix))
        else {
            continue;
        };
        let Some((emitted_q, simple, body_namespace, source_name)) =
            ctx.procedure_name_in_context(&factory_scope, &cand.name)
        else {
            continue;
        };
        if ctx
            .result
            .procs
            .get(&emitted_q)
            .is_some_and(|existing| existing.source_name == source_name)
            || ctx.result.procedure_declarations.iter().any(|existing| {
                existing.qualified_name == emitted_q && existing.source_name == source_name
            })
        {
            continue;
        }
        ctx.record_proc(SignatureProc {
            name: simple,
            qualified_name: emitted_q,
            source_name,
            body_namespace,
            params: Vec::new(),
            params_computed: true,
            name_range: cand.name_tok.span,
            body_range: cand.body_tok.span,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_factory_body_matches() {
        let body = "proc $name $args $body";
        let params = vec!["name".to_string(), "args".to_string(), "body".to_string()];
        assert!(is_factory_body(
            body,
            &params,
            tcl_lexer::LexerConfig::default()
        ));
    }

    #[test]
    fn wrong_arity_rejected() {
        // `proc $name $args` only — three tokens, not four.
        let body = "proc $name $args";
        let params = vec!["name".to_string(), "args".to_string(), "body".to_string()];
        assert!(!is_factory_body(
            body,
            &params,
            tcl_lexer::LexerConfig::default()
        ));
    }

    #[test]
    fn non_variable_arg_disqualifies() {
        // The middle arg is a literal `foo`, not a variable.
        let body = "proc $name foo $body";
        let params = vec!["name".to_string(), "args".to_string(), "body".to_string()];
        assert!(!is_factory_body(
            body,
            &params,
            tcl_lexer::LexerConfig::default()
        ));
    }

    #[test]
    fn no_proc_statement_no_match() {
        let body = "set x 1; return $x";
        let params = vec!["name".to_string(), "args".to_string(), "body".to_string()];
        assert!(!is_factory_body(
            body,
            &params,
            tcl_lexer::LexerConfig::default()
        ));
    }

    #[test]
    fn fewer_than_three_params_no_match() {
        let body = "proc $name $args $body";
        let params = vec!["name".to_string(), "args".to_string()];
        assert!(!is_factory_body(
            body,
            &params,
            tcl_lexer::LexerConfig::default()
        ));
    }

    use tcl_lexer::{Span, Token, TokenType};

    fn cand(head: &str, ns: &str) -> FactoryCandidate {
        FactoryCandidate {
            head: head.to_string(),
            name: "X".to_string(),
            name_tok: Token::new(TokenType::Esc, Span::new(0, 0)),
            body_tok: Token::with_content_offset(TokenType::Str, Span::new(0, 0), 1),
            namespace_scope: None,
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
            source_name: None,
            params: vec!["name".to_string(), "args".to_string(), "body".to_string()],
            body_text: body.to_string(),
            namespace_scope: None,
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
            namespace_scope: None,
            ns_prefix: "tcllib".to_string(),
        });
        resolve_factory_defs(&mut ctx);
        let proc = ctx.result.procs.get("::tcllib::Foo").expect("emitted");
        assert_eq!(proc.name, "Foo");
        assert_eq!(
            proc.params,
            [] as [crate::signature_scan::types::ParamDef; 0]
        );
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
            namespace_scope: None,
            ns_prefix: "ns".to_string(),
        });
        resolve_factory_defs(&mut ctx);
        assert_eq!(ctx.result.procs.len(), 0);
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
                source_name: None,
                body_namespace: super::super::scope::SignatureNamespaceScope::Symbolic(
                    "::".to_owned(),
                ),
                qualified_name: "::Foo".to_string(),
                params: vec![super::super::types::ParamDef {
                    name: "real".to_string(),
                    has_default: false,
                    default_value: None,
                }],
                params_computed: false,
                name_range: Span::new(99, 102),
                body_range: Span::new(110, 120),
            },
        );
        ctx.candidates.push(FactoryCandidate {
            head: "DEFC".to_string(),
            name: "Foo".to_string(),
            name_tok: Token::new(TokenType::Esc, Span::new(0, 3)),
            body_tok: Token::with_content_offset(TokenType::Str, Span::new(10, 20), 1),
            namespace_scope: None,
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
            namespace_scope: None,
            ns_prefix: "tcllib".to_string(),
        });
        resolve_factory_defs(&mut ctx);
        // Absolute name preserved; not under tcllib::Foo.
        let proc = ctx.result.procs.get("::Top::Foo").expect("present");
        assert_eq!(proc.name, "Foo");
        assert!(!ctx.result.procs.contains_key("::tcllib::Foo"));
    }
}
