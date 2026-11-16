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

//! Bodies retained as their own analysis unit: `apply` lambdas, `array for` and `namespace eval` bodies.

use super::super::{Lowerer, join_namespace, seg_word_substitutes};
use crate::ir::{CommandTokens, ForeachIterator, Procedure, Script, Statement};
use crate::segmenter::SegmentedCommand;
use tcl_registry::hooks::LoweringHookId;

impl Lowerer<'_> {
    /// Retain a synthetic analysis body under its issued Module ID.
    /// Labels are displays; the body retains its own namespace/source receipt.
    /// Lambda parameters bind a fresh frame, while namespace bodies do not.
    fn register_body_unit(
        &mut self,
        label: &str,
        params: Vec<String>,
        span: tcl_lexer::Span,
        body: Script,
        is_lambda: bool,
    ) {
        let n = self.body_unit_count;
        self.body_unit_count += 1;
        let prefix = tcl_syntax::naming::unroot_rooted_key(label).unwrap_or(label);
        let qualified = tcl_syntax::naming::root_unrooted_key(&format!("{prefix}#{n}"));
        let short_name = tcl_syntax::naming::key_tail(label).to_owned();
        let id = self.module.retain_body_unit(Procedure {
            name: short_name,
            qualified_name: qualified,
            publication: None,
            declaration_site: body.executed_source.as_ref().map(|source| {
                crate::command_binding::CommandAllocationSite {
                    source: std::sync::Arc::clone(&source.origin),
                    offset: source.base(),
                }
            }),
            body_kind: if is_lambda {
                crate::ir::ProcedureBodyKind::Lambda
            } else {
                crate::ir::ProcedureBodyKind::NamespaceScript
            },
            params,
            span,
            body,
            params_raw: String::new(),
            body_source: None,
            body_offset: 0,
            namespace_scoped: false,
            base_priority: 500,
        });
        if is_lambda {
            self.module.lambda_body_units.insert(id);
        }
    }

    pub(in super::super) fn lower_apply(&mut self, seg: &SegmentedCommand) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();

        let tokens = self.cmd_tokens(seg);
        let selected = self
            .resolved_metadata_invocation(&tokens)
            .and_then(|invocation| {
                let dialect = invocation.dialect?;
                let values = (0..invocation.arguments.len())
                    .map(|index| invocation.argument_literal(index))
                    .collect::<Vec<_>>();
                let words = values
                    .iter()
                    .map(|value| {
                        value.as_deref().map_or(
                            tcl_registry::InvocationWord::Dynamic,
                            tcl_registry::InvocationWord::Literal,
                        )
                    })
                    .collect::<Vec<_>>();
                let arguments =
                    tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect);
                let tcl_registry::lambda_invocation::LambdaInvocationSelection::Selected(lambda) =
                    tcl_registry::lambda_invocation::select_lambda_invocation(arguments, 0)
                else {
                    return None;
                };
                let parameters = tcl_syntax::formal_params::parse_formal_parameters_in(
                    &lambda.parameters,
                    dialect.parameter_grammar()?,
                )
                .ok()?
                .into_iter()
                .map(|parameter| parameter.name)
                .collect();
                let source = self
                    .source_bindings
                    .as_ref()?
                    .executed_script_for_list_element(arg_tokens.first()?.span, 1)?
                    .clone();
                let namespace = self.original_body_namespace_context(&tokens, 0, &source);
                Some((lambda, parameters, source, namespace))
            });

        if let Some((lambda, params, source, namespace)) = selected {
            let body_offset = source.base();
            let body_length = source.text.len();
            let body_ns = join_namespace("::", &lambda.namespace);
            let body = self.in_procedure_frame(None, |lowerer| {
                lowerer.lower_original_body(source, namespace.as_ref())
            });

            // A body defined inside a TclOO method body is not registered
            // globally (`suppress_proc_register`), but a body unit is
            // analysis-only (never codegen-emitted), so it is always safe to
            // record for coverage.
            let span = tcl_lexer::Span::new(
                body_offset,
                body_offset + u32::try_from(body_length).unwrap_or(u32::MAX),
            );
            // The body retains its own lambda namespace; the label is a display.
            // tclsh8.6/9.0-confirmed: inside
            // `::foo::runIt`, `apply {{x} { helper $x } ::foo} b` calls
            // `::foo::helper`, not `::helper`.
            self.register_body_unit(&join_namespace(&body_ns, "apply"), params, span, body, true);
        }

        Statement::Barrier {
            span: seg.span,
            reason: "unsupported body command".into(),
            command: seg.name().into(),
            canonical_command: None,
            args: args.to_vec(),
            tokens: Some(self.cmd_tokens(seg)),
        }
    }

    /// `array for {keyVar valueVar} arrayName body` (Tcl 9.0).
    ///
    /// The body runs in the caller's frame with the two loop variables bound
    /// per entry. C Tcl compiles this to an `invokeStk` of `::tcl::array::for`
    /// with the body pushed as an unparsed literal — it does not compile the
    /// body — so, exactly like [`Self::lower_apply`], the call itself stays a
    /// runtime [`Statement::Barrier`] (codegen unchanged, byte-identical to C
    /// Tcl). But a braced literal body is walked in a fresh frame bound to the
    /// loop variables and recorded as a body unit so static analysis reaches
    /// inside it. A `$var` / `[cmd]` body stays fully opaque.
    pub(in super::super) fn lower_array_for(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Statement {
        use tcl_lexer::TokenType;
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        // `array for {k v} arr body` → args = [for, {k v}, arr, body].
        // The body (index 3) must be a single braced-literal token to walk.
        let body_is_braced = arg_tokens.get(3).is_some_and(|t| t.kind == TokenType::Str)
            && arg_single.get(3).copied() == Some(true);
        // A varList Tcl would refuse to split binds nothing statically, so it
        // falls through to the opaque runtime barrier below. Decided before the
        // body is walked, so a rejected loop leaves no const-map traces behind.
        if args.len() == 4
            && body_is_braced
            && let Some(vars) = crate::registry_invocation::original_name_role_values(
                self.registry,
                &self.cmd_tokens(seg),
                1,
                tcl_registry::ArgRole::LoopVarList,
            )
        {
            let body_tok = &arg_tokens[3];
            // The body runs in the *caller's* frame (`vm.eval_source` in place),
            // so lower it inline — inheriting the caller's const-map — into a
            // loop-variable-bound `Foreach` rather than isolating it in a
            // fresh-frame body unit. The analysis CFG inlines this into the
            // caller unit (so a body `$re` resolves to a caller literal for
            // regex-source / taint), while codegen barriers it to the byte-
            // identical `::tcl::array::for` invoke — see `lower_foreach_dispatch`.
            let body = self.lower_body_from_tok(&args[3], Some(body_tok), namespace);
            return Statement::Foreach {
                span: seg.span,
                iterators: vec![ForeachIterator {
                    vars,
                    list_arg: args[2].clone(),
                    // `array for {k v} {arr} …` — a braced array-name word
                    // is a literal name, not a substitution.
                    list_braced: self.cmd_tokens(seg).arg_is_braced_literal(2),
                }],
                body,
                body_span: self.script_word_span(body_tok),
                is_lmap: false,
                raw_args: args.to_vec(),
                is_dict_iteration: false,
                is_array_iteration: true,
                raw_tokens: Some(self.cmd_tokens(seg)),
            };
        }

        // A dynamic (`$body` / `[cmd]`) body stays fully opaque: the runtime
        // barrier re-emits `array for …`.
        self.lower_default(seg, namespace)
    }

    pub(in super::super) fn lower_namespace_eval(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Statement {
        // The body is walked from its *written* spelling, rebased by the body
        // word's own content offset, so the word must not substitute.
        // A `$body` / `[list …]` word's value is run-time data
        // that appears nowhere in this document: rebasing its unsubstituted
        // spelling fabricates statement spans that overrun the source —
        // `namespace eval ::ns [list $o fw]` rebases the whole `[list $o fw]`
        // text past the opening `[`, one byte off the end.
        // [`Self::guarded_body_text`] deliberately passes such words through
        // untouched because clamping run-time data proves nothing; refusing to
        // walk them is the caller-side half of that contract.  The namespace is
        // still entered at run time — the `Barrier` below re-emits the call —
        // the body just stays opaque to static analysis, exactly as
        // [`Self::lower_apply`] and [`Self::lower_array_for`] leave a dynamic
        // body opaque.
        if !seg_word_substitutes(seg, 3) {
            self.walk_namespace_eval_body(seg, namespace);
        }

        Statement::Barrier {
            span: seg.span,
            reason: "namespace eval".into(),
            command: "namespace".into(),
            canonical_command: None,
            args: seg.args().to_vec(),
            tokens: Some(self.cmd_tokens(seg)),
        }
    }

    /// A generic namespace handler can retain entered declarations while its
    /// original invocation remains generic. The exact source carrier supplies
    /// the body; no lexical fallback or executable hook is inferred here.
    pub(in super::super) fn retain_entered_namespace_body(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: &CommandTokens,
    ) {
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource {
            return;
        }
        let Some(invocation) = self.resolved_metadata_invocation(tokens) else {
            return;
        };
        if invocation.facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(LoweringHookId::NamespaceEval)
            || tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.proved_execution_target())
                .is_none_or(|target| !target.prepended.is_empty())
        {
            return;
        }
        let name_at = invocation.facts.argument_offset;
        let body_at = name_at + 1;
        if seg.args().len() != body_at + 1 {
            return;
        }
        let Some(child) = invocation.argument_literal(name_at) else {
            return;
        };
        let Some(token) = seg.arg_tokens().get(body_at) else {
            return;
        };
        let Some(source) = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.executed_script_for_word(token.span))
            .cloned()
        else {
            return;
        };
        let child_namespace = join_namespace(namespace, &child);
        let body_span = tcl_lexer::Span::new(
            source.base(),
            source.base() + u32::try_from(source.text.len()).unwrap_or(u32::MAX),
        );
        let previous = self.in_namespace_eval;
        self.in_namespace_eval = true;
        let namespace = self.original_body_namespace_context(tokens, body_at, &source);
        let body = self.lower_original_body(source, namespace.as_ref());
        self.in_namespace_eval = previous;
        self.register_body_unit(
            &join_namespace(&child_namespace, "namespace-eval"),
            vec![],
            body_span,
            body,
            false,
        );
    }

    /// Walk a `namespace eval` body inline and record it as a body unit.
    ///
    /// Only reached for a body word that does not substitute — see
    /// [`Self::lower_namespace_eval`] for why.
    fn walk_namespace_eval_body(&mut self, seg: &SegmentedCommand, namespace: &str) {
        let args = seg.args();
        let child_ns = join_namespace(namespace, &args[1]);
        let body_tok = seg.arg_tokens()[2];
        // `namespace eval` inlines whatever literal body word it is given,
        // including a compound `{…}x` one, so the rebase below needs the
        // clamp in [`Self::guarded_body_text`] — both for the body itself and
        // for the body unit's span derived from its length.
        let body_text = self.guarded_body_text(body_tok, &args[2]);
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let prev = self.in_namespace_eval;
        self.in_namespace_eval = true;
        let body = self.lower_body_from_tok(&args[2], Some(&body_tok), &child_ns);
        self.in_namespace_eval = prev;

        // A `namespace eval` body runs in the child namespace's own frame — its
        // straight-line statements are a real analysable script, so record them
        // as a body unit (no bound params: namespace vars, not locals).
        let span = tcl_lexer::Span::new(
            body_offset,
            body_offset + u32::try_from(body_text.len()).unwrap_or(u32::MAX),
        );
        // Original body context selects the child namespace; labels do not.
        // tclsh8.6-confirmed:
        // `namespace eval ::foo { helper }` calls `::foo::helper` even when
        // nested inside an unrelated proc, never a same-named proc in the
        // caller's own namespace.
        self.register_body_unit(
            &join_namespace(&child_ns, "namespace-eval"),
            vec![],
            span,
            body,
            false,
        );
    }
}
