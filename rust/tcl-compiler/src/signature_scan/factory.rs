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
use super::ctx::{ProcBodyInfo, ScanCtx};
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
    let commands = segment_commands_with_offset_and_config(body_text, 0, config.nested());
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
        .filter(|info| {
            if ctx.original_bindings.is_some() {
                is_selected_factory_body(info, ctx)
            } else {
                is_factory_body(&info.body_text, &info.params, ctx.config)
            }
        })
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
                Some(name) => match &cand.original_head {
                    Some(key) => name.matches_key(lookup, key),
                    None if ctx.original_image.is_none() => {
                        name.matches_written(lookup, &cand.head)
                    }
                    None => false,
                },
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
            ctx.procedure_name_in_context(&factory_scope, &cand.name, cand.original_name.as_ref())
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
            formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                tcl_dialect::ParameterGrammar::Tcl,
            ),
            name_range: cand.name_tok.span,
            body_range: cand.body_tok.span,
        });
    }
}

/// Recognise a wrapper only through the selected definer and procedure
/// layouts retained for this complete source. The compatibility recogniser
/// above remains for callers which intentionally have no source binding.
fn is_selected_factory_body(info: &ProcBodyInfo, ctx: &ScanCtx<'_>) -> bool {
    let Some(owner) = info.original_factory_body.as_ref() else {
        return false;
    };
    let Some(image) = ctx.original_image.as_ref() else {
        return false;
    };
    if owner.parameters.image() != image
        || owner.body.image() != image
        || owner.parameters.config() != ctx.config
        || owner.body.config() != ctx.config
        || owner.words.params == owner.words.body
        || owner.words.name == owner.words.params
        || owner.words.name == owner.words.body
        || owner.words.statics == Some(owner.words.params)
        || owner.words.statics == Some(owner.words.body)
        || owner.body.group().kind != tcl_lexer::WordKind::Braced
        || owner.body.group().expand
    {
        return false;
    }
    let Some(body_span) = owner.body.content_span().ok() else {
        return false;
    };
    let Ok(document) = image.try_text() else {
        return false;
    };
    let Some(body) = document.get(body_span.as_range()) else {
        return false;
    };
    // `body_text` is the selected definer's decoded display. This equality
    // admits only an exact source slice; a cooked value has no source map here.
    if body.as_bytes() != info.body_text.as_bytes() {
        return false;
    }
    let Some(parameter_variables) = selected_factory_parameter_variables(owner, info, ctx.rules)
    else {
        return false;
    };
    for command in
        segment_commands_with_offset_and_config(body, body_span.start(), ctx.config.nested())
    {
        if command.is_partial || command.argv.len() != 4 {
            continue;
        }
        let Some(original) = ctx.original_command_words(&command) else {
            continue;
        };
        if original.len() != 4
            || !original
                .iter()
                .all(|word| word.image() == image && word.config() == ctx.config)
        {
            continue;
        }
        let Some(_selection) = ctx
            .with_selected_schema(&command, |schema| {
                use tcl_registry::native_procedure::NativeProcedureDefinitionSelection;
                let descriptor = schema.semantics.procedure_definition?;
                match descriptor.select(schema.words.arguments()) {
                    NativeProcedureDefinitionSelection::Valid(definition)
                        if definition.name_at == 0
                            && definition.parameters_at == 1
                            && definition.statics_at.is_none()
                            && definition.body_at == 2 =>
                    {
                        Some(definition)
                    }
                    _ => None,
                }
            })
            .flatten()
        else {
            continue;
        };
        if [1usize, 2, 3].into_iter().all(|index| {
            selected_factory_parameter_reference(&original[index], &parameter_variables, ctx)
        }) {
            return true;
        }
    }
    false
}

fn selected_factory_parameter_variables(
    owner: &super::ctx::OriginalFactoryBodySource,
    info: &ProcBodyInfo,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<HashSet<Vec<u8>>> {
    let parameter_bytes =
        tcl_syntax::word_rules::original_static_word_unicode_value(&owner.parameters)?;
    let parameter_text = std::str::from_utf8(&parameter_bytes).ok()?;
    let parameter_names: Vec<_> =
        crate::signature_scan::params::parse_param_list(parameter_text, rules)
            .into_iter()
            .map(|parameter| parameter.name)
            .collect();
    if parameter_names != info.params || parameter_names.len() < 3 {
        return None;
    }
    Some(
        parameter_names
            .iter()
            .map(|name| name.as_bytes().to_vec())
            .collect(),
    )
}

fn selected_factory_parameter_reference(
    word: &tcl_lexer::NativeWord,
    parameters: &HashSet<Vec<u8>>,
    ctx: &ScanCtx<'_>,
) -> bool {
    if word.group().kind != tcl_lexer::WordKind::Bare || word.group().expand {
        return false;
    }
    let mut parts = word.executable_parts().all_parts();
    let Some(part) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    let tcl_lexer::ExecutablePart::Variable { index: None, .. } = &part.part else {
        return false;
    };
    let Some(policy) = ctx.name_policy() else {
        return false;
    };
    let Some(root) =
        crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_word(
            word, part.span, ctx.rules, policy,
        )
    else {
        return false;
    };
    parameters.contains(root.bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_source_factory_uses_registry_definer_and_inner_procedure_layouts() {
        // Source-only selected form and body geometry; no procedure execution.
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let registry = input.borrowed_context_registry().commands();
        let config = input.lexer_config();
        let source =
            "# λ\nproc factory {name args body} {proc $name $args $body}\nfactory generated {} {}";
        let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            crate::command_binding::SourceAnalysisOptions::for_logical_source(&input).unwrap(),
        );
        let scan = super::super::extract_signatures_with_original_bindings(
            source, config, registry, &bindings,
        );
        assert!(scan.procs.contains_key("::generated"));
    }

    #[test]
    fn original_factory_scans_keep_body_bom_distinct_from_definer_and_call_heads() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        // Conditional wrapper/header shape only; no entered procedure or manufactured command.
        let mut prepared = crate::analyser::Analyser::new();
        let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
        let context = input.context_registry();
        let registry = context.commands();
        let params = vec!["name".to_owned(), "args".to_owned(), "body".to_owned()];
        let source = "proc factory {name args body} {proc $name $args $body}\nproc badFactory {name args body} {\u{feff}proc $name $args $body}\nproc init {} {\u{feff}factory hidden {} {}; factory shown {} {}; badFactory falseHit {} {}}";
        for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
            let mut config = input.lexer_config();
            config.leading_bom = bom;
            config.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
            assert!(is_factory_body(
                "proc ${name} ${args} ${body}",
                &params,
                config
            ));
            assert!(!is_factory_body(
                "\u{feff}proc $name $args $body",
                &params,
                config
            ));
            let result = super::super::extract_signatures_with_config(source, config, registry);
            assert!(result.procs.contains_key("::factory"));
            let shown = result.procs.get("::shown").unwrap();
            assert_eq!(source.get(shown.name_range.as_range()), Some("shown"));
            assert!(!result.procs.contains_key("::hidden"));
            assert!(!result.procs.contains_key("::falseHit"));
        }
    }

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
            original_head: None,
            original_name: None,
            head: head.to_string(),
            name: "X".to_string(),
            name_tok: Token::new(TokenType::Esc, Span::new(0, 0)),
            body_tok: Token::with_content_offset(TokenType::Str, Span::new(0, 0), 1),
            namespace_scope: None,
            ns_prefix: ns.to_string(),
            original_factory_body: None,
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
    fn selected_factory_requires_exact_scalar_formal_roots() {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let registry = context.commands();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let analyse = |source: &str| {
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context.clone(),
                config,
            );
            let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
                source,
                config,
                registry,
                crate::command_binding::SourceAnalysisOptions::for_logical_source(&input).unwrap(),
            );
            super::super::extract_signatures_with_original_bindings(
                source, config, registry, &bindings,
            )
        };
        let positive = analyse(
            "proc factory {name args body} {proc $name $args $body}\nfactory generated {} {}",
        );
        assert!(positive.procs.contains_key("::generated"));
        let punctuation = analyse(
            "proc factory {name-tail args body} {proc $name-tail $args $body}\nfactory generated {} {}",
        );
        assert!(!punctuation.procs.contains_key("::generated"));
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
            original_head: None,
            original_name: None,
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
            original_head: None,
            original_name: None,
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
                formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                    tcl_dialect::ParameterGrammar::Tcl,
                ),
                name_range: Span::new(99, 102),
                body_range: Span::new(110, 120),
            },
        );
        ctx.candidates.push(FactoryCandidate {
            original_head: None,
            original_name: None,
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
            original_head: None,
            original_name: None,
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
