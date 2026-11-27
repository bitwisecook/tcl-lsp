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

//! `apply`: the lambda's parameters, namespace and body.

use super::super::state::Analyser;
use super::super::utils::{param_name_spans_for_token, parse_param_list};
use super::qualify;
use crate::signature_scan::types::ParamDef;
use tcl_lexer::{Span, Token, TokenType};

struct ApplyLambdaSourceFacts {
    body_ns: String,
    original_namespace: Option<(
        tcl_names::namespace::SignatureNamespaceScope,
        Option<std::sync::Arc<super::super::interp_visibility::OriginalInterpreterSourceDomain>>,
    )>,
    logical_formals: Option<(Vec<ParamDef>, Vec<Span>)>,
    logical_namespace_source:
        Option<std::sync::Arc<super::super::interp_visibility::OriginalInterpreterLambdaBody>>,
}

struct ApplyLambdaBody {
    pub(super) params: Vec<ParamDef>,
    original_param_spans: Option<Vec<Span>>,
    pub(super) params_tok: Token,
    body_text: std::sync::Arc<str>,
    body_tok: Token,
    body_ns: String,
    scope_name: String,
    original_namespace: Option<(
        tcl_names::namespace::SignatureNamespaceScope,
        Option<std::sync::Arc<super::super::interp_visibility::OriginalInterpreterSourceDomain>>,
    )>,
    logical_namespace_source:
        Option<std::sync::Arc<super::super::interp_visibility::OriginalInterpreterLambdaBody>>,
}

impl Analyser {
    fn record_apply_namespace_reference(
        &mut self,
        elements: &[(Token, String)],
        body_ns: &str,
        arg_tokens: &[Token],
    ) {
        // That element is also a first-class *reference* to the namespace it
        // names, not merely a semantic input: it sits
        // inside an `ArgRole::LambdaLiteral` word, which no whole-word role
        // reaches, so the identity is recorded here, where the element has
        // already been split. Non-declaring — `apply` looks the namespace up,
        // it does not create it (tclsh 8.6.16 / 9.0.4: `apply {{} {} ::nope}`
        // fails `namespace "::nope" not found`).
        if let Some((ns_tok, ns_text)) = elements.get(2)
            && body_ns != "::"
            && !crate::naming::is_dynamic_word(ns_text)
        {
            let span = tcl_lexer::Span::new(
                ns_tok.span.start() + u32::from(ns_tok.content_offset),
                ns_tok.span.end(),
            );
            if span.start() < span.end() {
                self.result.namespace_refs.push(
                    super::super::types::NamespaceRef::from_original(
                        self.declaration_name_policy().map(|policy| {
                            tcl_names::namespace::SignatureNamespaceScope::root(Some(policy))
                        }),
                        self.declaration_name_policy(),
                        ns_text,
                        span,
                        0,
                        false,
                        body_ns.to_owned(),
                    )
                    .with_original_input(arg_tokens.first().and_then(
                        |token| {
                            self.head_identities
                                .original_written_name_input_at_span(
                                    token.span,
                                    self.file_lexer_config(),
                                )?
                                .original_list_element(2)
                        },
                    )),
                );
            }
        }
    }

    /// One-hop lambda inspection from original list builders or checked Logical values.
    /// Element spans retain their defining source; unavailable receivers stay opaque.
    pub(super) fn resolve_dynamic_apply_lambda(
        &self,
        arg_tok: Token,
    ) -> Option<Vec<(Token, String)>> {
        let one_hop = |word_tok: Token| -> Option<(Token, String)> {
            if word_tok.kind != TokenType::Var {
                let word_text = Analyser::source_slice(
                    &self.source,
                    word_tok.span.start() as usize,
                    word_tok.span.end() as usize,
                )?;
                return Some((word_tok, word_text.to_string()));
            }
            let (value, span) = self.lookup_logical_const_substitution(word_tok)?;
            Some((
                Token::with_content_offset(TokenType::Str, span, 1),
                value.to_string(),
            ))
        };

        match arg_tok.kind {
            TokenType::Var => {
                let (tok, text) = one_hop(arg_tok)?;
                if text.as_bytes().first() != Some(&b'{') {
                    return None;
                }
                self.parse_apply_lambda_elements(&[text], &[tok])
            }
            TokenType::Cmd => {
                let (inner, base) = super::super::scope::inner_of(&self.source, arg_tok)?;
                let segmented = tcl_source::segment::segment_commands_with_offset_and_config(
                    inner,
                    base,
                    self.file_lexer_config().nested(),
                );
                let [cmd] = segmented.as_slice() else {
                    return None;
                };
                // A lambda built word by word: the head is the command whose
                // result quotes each of its words as one element — the
                // registry's `BUILDS_COMMAND_PREFIX` reading, not a spelling.
                let original =
                    self.original_diagnostic_call_at(*cmd.argv.first()?, &cmd.argv[1..])?;
                if !original.with_schema(|schema| {
                    schema
                        .semantics
                        .traits
                        .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
                })? || original.words().arguments().len() != cmd.argv.len().checked_sub(1)?
                    || (0..original.words().arguments().len())
                        .any(|ordinal| original.written_index(ordinal) != Some(ordinal))
                {
                    return None;
                }
                let mut out = Vec::with_capacity(cmd.texts.len().saturating_sub(1));
                for (tok, text) in cmd.argv.iter().skip(1).zip(cmd.texts.iter().skip(1)) {
                    if tok.kind == TokenType::Var {
                        // A `$body` / `$ns` list element still needs one hop
                        // through the constant-value lattice.
                        out.push(one_hop(*tok)?);
                    } else {
                        // Use the segmenter's already-delimiter-stripped
                        // element text (`cmd.texts`) paired with its token —
                        // the same shape `parse_apply_lambda_elements` yields
                        // for a literal lambda. Slicing the token's raw source
                        // span instead would keep a `{cleanup done}` element's
                        // braces, so the body re-segments as one braced word
                        // and the real `cleanup` call is missed.
                        out.push((*tok, text.clone()));
                    }
                }
                Some(out)
            }
            _ => None,
        }
    }

    pub(in crate::analyser) fn parse_apply_lambda_elements(
        &self,
        args: &[String],
        arg_tokens: &[Token],
    ) -> Option<Vec<(Token, String)>> {
        if args.is_empty() || arg_tokens.is_empty() {
            return None;
        }
        let lambda_tok = arg_tokens[0];
        // Only a *braced* literal lambda can be split and offset-mapped safely
        // (its content is verbatim source).
        let lambda_start = lambda_tok.span.start() as usize;
        if lambda_tok.kind != TokenType::Str
            || self.source.as_bytes().get(lambda_start) != Some(&b'{')
        {
            return None;
        }

        // Split the lambda literal into its list elements. Re-segmenting the
        // brace-stripped content (`args[0]`) at the lambda's absolute content
        // offset yields the elements as command words carrying absolute-span
        // tokens; flattening across commands keeps params / body paired even
        // when a multi-line lambda puts them on separate lines (a newline
        // splits *commands*, not *list elements*).
        let base = lambda_tok.span.start() + u32::from(lambda_tok.content_offset);
        let segmented = tcl_source::segment::segment_commands_with_offset_and_config(
            &args[0],
            base,
            self.file_lexer_config().nested(),
        );
        Some(
            segmented
                .iter()
                .flat_map(|c| c.argv.iter().copied().zip(c.texts.iter().cloned()))
                .collect(),
        )
    }

    /// Handle an `apply {{params} body ?ns?} ?arg ...?` invocation.
    ///
    /// `apply`'s first argument is a *lambda* (an anonymous procedure), **not**
    /// a plain script body: element 0 is the parameter list and element 1 is
    /// the body. The `apply` registry spec marks the argument `ArgRole::Body`,
    /// so without this handler the generic body-recursion in
    /// [`Analyser::dispatch_body_arguments`] treats the whole `{{params} body}`
    /// literal as a *script* — mis-reading the parameter list as a command
    /// (`apply {{a} {…}}` ⇒ a spurious W123 "unknown command 'a'") and never
    /// linting the real body.
    ///
    /// This models the lambda like a `proc`: the parameters bind as locals in a
    /// fresh proc scope and element 1 is walked as the body (deferred during the
    /// per-item shell pass, inline otherwise — mirroring
    /// [`Analyser::handle_proc_command`], so the incremental and full paths stay
    /// byte-identical). The body is walked in the *lambda's* namespace — element
    /// 2 of the lambda, or the global namespace `::` when absent — not the
    /// caller's, so a nested `proc` registers under the qualified name the
    /// runtime `apply` gives it (`::p`, not `::caller::p`). Returns `true` when
    /// the command was an `apply` with a
    /// statically-inspectable braced lambda literal, so the caller skips the
    /// generic body recursion. A dynamic lambda (`apply $lambda …`) returns
    /// `false` and falls through to the generic path (whose `analyse_body` is a
    /// no-op on the non-braced word), preserving its existing behaviour.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Apply`].
    pub fn handle_apply_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        _scope_path: &[usize],
    ) -> bool {
        self.handle_apply_command_with_source_namespace(args, arg_tokens, None)
    }

    pub(in super::super) fn handle_apply_command_with_source_namespace(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        original: Option<super::super::interp_visibility::OriginalInterpreterLambdaBody>,
    ) -> bool {
        // A literal braced lambda parses directly; a dynamic one (`apply
        // $lambda`, `apply [list {params} $body ns]`) gets one hop through
        // the constant-value lattice before giving up
        // — same downstream shape either way, so nothing past this point
        // needs to know which path supplied `elements`.
        let elements = if let Some(elements) = self.parse_apply_lambda_elements(args, arg_tokens) {
            elements
        } else {
            let Some(&arg_tok) = arg_tokens.first() else {
                return false;
            };
            let Some(elements) = self.resolve_dynamic_apply_lambda(arg_tok) else {
                return false;
            };
            elements
        };
        // When a source-selected lambda receipt is present, the token-derived
        // list projection must name that exact original body in the current
        // input. A cooked or shifted value cannot borrow this source owner.
        if let Some(original) = original.as_ref() {
            let (Some((parameter_token, _)), Some((body_token, _))) =
                (elements.first(), elements.get(1))
            else {
                return true;
            };
            if original
                .for_lambda_tokens(
                    &self.source,
                    &self.resolved_analysis_input(),
                    *parameter_token,
                    *body_token,
                )
                .is_none()
            {
                return true;
            }
        }
        let Some((params_tok, body_tok)) = self.apply_lambda_body_tokens(&elements) else {
            return true;
        };
        let params_text = elements[0].1.as_str();
        let body_text = elements[1].1.as_str();

        // `apply` runs the lambda body in the namespace named by lambda element
        // 2, or the *global* namespace when it is absent — never the caller's
        // namespace. Element 2 is interpreted relative to the **global**
        // namespace even when it does not start with `::` (`doc/apply.n`:
        // "If given, namespace is interpreted relative to the global namespace
        // even if its name does not start with ::"; `tclProc.c`
        // `TclNRApplyObjCmd` literally `::`-prefixes the word before the
        // lookup). So `apply {{} {…} sub}` homes to `::sub` no matter which
        // namespace the call sits in, and a nested `proc` registers under the
        // qualified name the runtime `apply` would give it.
        let body_ns = match elements.get(2).map(|(_, t)| t.as_str()) {
            Some(ns) if !ns.is_empty() && !ns.starts_with('$') && !ns.starts_with('[') => {
                qualify("", ns)
            }
            _ => "::".to_string(),
        };
        let source_facts = self.resolve_apply_lambda_source_facts(
            &elements, arg_tokens, original, body_tok, params_tok, body_ns,
        );
        let ApplyLambdaSourceFacts {
            body_ns,
            original_namespace,
            logical_formals,
            logical_namespace_source,
        } = source_facts;
        let (params, original_param_spans) = match logical_formals {
            Some((parameters, spans)) => (parameters, Some(spans)),
            None => (parse_param_list(params_text, self.word_rules()), None),
        };
        let body_text: std::sync::Arc<str> = std::sync::Arc::from(body_text);
        // Anonymous, but keyed by source position so two lambdas never collide
        // in `all_variables` (keyed `"<scope_name>::<var>"`).
        let scope_name = format!("apply@{}", arg_tokens[0].span.start());

        self.finish_apply_lambda_body(ApplyLambdaBody {
            params,
            original_param_spans,
            params_tok,
            body_text,
            body_tok,
            body_ns,
            scope_name,
            original_namespace,
            logical_namespace_source,
        })
    }

    fn apply_lambda_body_tokens(&self, elements: &[(Token, String)]) -> Option<(Token, Token)> {
        if elements.len() < 2 {
            return None;
        }
        let params_tok = elements[0].0;
        let body_tok = elements[1].0;
        if body_tok.kind != TokenType::Str
            || self.source.as_bytes().get(body_tok.span.start() as usize) != Some(&b'{')
        {
            return None;
        }
        Some((params_tok, body_tok))
    }

    fn resolve_apply_lambda_source_facts(
        &mut self,
        elements: &[(Token, String)],
        arg_tokens: &[Token],
        original: Option<super::super::interp_visibility::OriginalInterpreterLambdaBody>,
        body_tok: Token,
        params_tok: Token,
        body_ns: String,
    ) -> ApplyLambdaSourceFacts {
        let original_namespace = original.as_ref().and_then(|original| {
            original.for_body_token(&self.source, &self.resolved_analysis_input(), body_tok)
        });
        let logical_formals = original.as_ref().and_then(|original| {
            original
                .logical_formals_for_body_token(
                    &self.source,
                    &self.resolved_analysis_input(),
                    body_tok,
                    params_tok,
                )
                .and_then(|formals| {
                    Some((
                        formals
                            .parameters()
                            .iter()
                            .map(|parameter| ParamDef {
                                name: parameter.name.clone(),
                                has_default: parameter.default.is_some(),
                                default_value: parameter.default.clone(),
                            })
                            .collect::<Vec<_>>(),
                        formals
                            .name_sources()
                            .iter()
                            .map(crate::signature_scan::formal_parameters::OriginalSourceFormalName::source_span)
                            .collect::<Option<Vec<_>>>()?,
                    ))
                })
        });
        if let Some(original) = original
            .as_ref()
            .filter(|original| original.has_logical_source_namespace())
        {
            if let Some(field) = original.logical_namespace_field_for_body_token(
                &self.source,
                &self.resolved_analysis_input(),
                body_tok,
            ) && let Some((namespace, _)) = original_namespace.as_ref()
                && !namespace.is_root()
                && let Some(span) = field.original_field().source_span()
                && !span.is_empty()
            {
                self.result
                    .namespace_refs
                    .push(super::super::types::NamespaceRef::from_original(
                        Some(tcl_names::namespace::SignatureNamespaceScope::root(Some(
                            field.policy(),
                        ))),
                        Some(field.policy()),
                        field.original_field().value(),
                        span,
                        0,
                        false,
                        namespace
                            .source_spelling(Some(field.policy()))
                            .unwrap_or_else(|| body_ns.clone()),
                    ));
            }
        } else {
            self.record_apply_namespace_reference(elements, &body_ns, arg_tokens);
        }
        let logical_namespace_source = original
            .filter(|original| {
                original_namespace.is_some() && original.has_logical_source_namespace()
            })
            .map(std::sync::Arc::new);
        let body_ns = original_namespace
            .as_ref()
            .and_then(|(namespace, domain)| {
                domain.as_ref().map_or_else(
                    || namespace.display(),
                    |domain| domain.reported_namespace(namespace),
                )
            })
            .unwrap_or(body_ns);
        ApplyLambdaSourceFacts {
            body_ns,
            original_namespace,
            logical_formals,
            logical_namespace_source,
        }
    }

    fn finish_apply_lambda_body(&mut self, apply: ApplyLambdaBody) -> bool {
        let ApplyLambdaBody {
            params,
            original_param_spans,
            params_tok,
            body_text,
            body_tok,
            body_ns,
            scope_name,
            original_namespace,
            logical_namespace_source,
        } = apply;
        let body_span = body_tok.span;
        // Record the namespace override for command-resolution lookups.
        self.result
            .namespace_overrides
            .push((body_span, body_ns.clone()));
        let child_path = super::super::per_item::reconstruct_proc_scope(
            &mut self.result.global_scope,
            &body_ns,
            &scope_name,
            super::super::types::ScopeKind::Proc,
        );
        if let Some(scope) =
            super::super::scope::scope_at_mut(&mut self.result.global_scope, &child_path)
        {
            scope.body_span = Some(body_span);
            scope.requires_original_lambda_source_context = true;
            if let Some((namespace, domain)) = original_namespace {
                scope.naming_scope = Some(namespace);
                scope.original_interpreter_source_domain = domain;
                scope.original_logical_lambda_namespace = logical_namespace_source;
            }
        }
        let param_spans = original_param_spans
            .unwrap_or_else(|| param_name_spans_for_token(&self.source, params_tok));
        for (i, p) in params.iter().enumerate() {
            self.define_var(
                &p.name,
                params_tok,
                &child_path,
                false,
                param_spans.get(i).copied(),
            );
        }
        self.emit_w218_args_not_final(&params, &param_spans, params_tok);
        let saved_comment = std::mem::take(&mut self.last_comment);
        if self.defer_proc_bodies {
            let safe_interp_ctx = self.safe_interp_ctx_snapshot();
            self.deferred_bodies
                .push(super::super::per_item::DeferredBody {
                    resolved_input: Some(self.resolved_analysis_input()),
                    body_text,
                    body_tok,
                    scope_path: child_path.clone(),
                    is_method: false,
                    oo_global_resolution: false,
                    namespace: body_ns,
                    scope_name,
                    params,
                    seeded_variables: Vec::new(),
                    command_trust: None,
                    ensemble_targets: Vec::new(),
                    prefixless_ensembles: Vec::new(),
                    oo_defining_class: None,
                    safe_interp_ctx,
                });
        } else {
            self.analyse_body(&body_text, body_tok, &child_path);
        }
        self.last_comment = saved_comment;
        true
    }
}
