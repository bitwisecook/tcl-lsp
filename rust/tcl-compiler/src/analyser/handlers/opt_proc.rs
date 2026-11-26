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

//! `tcl::OptProc`: the `opt` package's procedure form, its option list and the locals it binds.

use super::super::state::Analyser;
use super::super::types::ProcDef;
use super::super::utils::{param_name_spans_for_token, parse_param_list};
use super::scope_name_for_routine;
use crate::signature_scan::types::ParamDef;
use tcl_lexer::{Span, Token, TokenType};

impl Analyser {
    fn bind_opt_proc_locals(
        &mut self,
        params_tok: Token,
        name_tok: Token,
        child_path: &[usize],
        opt_locals: &[ParamDef],
    ) {
        // Bind the real `args` catch-all — a body reference to
        // `$args` (inspecting leftovers `::tcl::OptKeyParse` didn't
        // consume) is legitimate, exactly like an ordinary proc's own
        // `args` parameter. No literal `args` word is ever written for
        // this idiom, so — unlike an ordinary proc's own parameters,
        // each anchored to its own written span — there is no sensible
        // non-synthetic span to anchor it to: `name_tok` collides with
        // the proc *name*'s own span (`$args` hover would resolve to
        // the declaration token instead of `greet`), and the whole
        // `optlist` word collides with every one of its own descriptor
        // sub-spans (`child`'s / `-use`'s own hover would resolve to
        // `args` instead). A zero-width span at the `optlist` word's
        // own opening brace sits before any descriptor's span starts,
        // so it collides with neither.
        let args_span = Span::new(params_tok.span.start(), params_tok.span.start());
        self.define_var("args", params_tok, child_path, false, Some(args_span));
        // Bind every optlist-derived local, anchored to its own
        // descriptor's span (dash included — the written token, not a
        // byte-sliced substring) so go-to-definition / references /
        // rename on the parameter land on the real declaration.
        let param_spans = param_name_spans_for_token(&self.source, params_tok);
        for (i, p) in opt_locals.iter().enumerate() {
            self.define_var(
                &p.name,
                name_tok,
                child_path,
                false,
                param_spans.get(i).copied(),
            );
        }
    }

    /// `tcl::OptProc name optlist body` — the `opt` package's automatic-
    /// option-parsing proc definer.
    ///
    /// At runtime this installs a REAL proc via `uplevel 1 [list ::proc
    /// $name args ...]` — the Tcl-level formal parameter is always the
    /// single literal word `args` (any call arity is accepted; `optlist`
    /// itself is never arity-checked). `optlist`'s own descriptor entries
    /// (`{child -use -display}`) share `proc`'s own `{name default}` /
    /// bare-`name` list shape, so [`parse_param_list`] applies directly —
    /// but they are bound as LOCAL VARIABLES inside the body by
    /// `::tcl::OptKeyParse`, with a leading `-` on a flag descriptor
    /// STRIPPED for the bound name (tclsh9.0/8.6-verified: `-use`/
    /// `-display` bind as `use`/`display`, never with the dash).
    ///
    /// Mirrors [`Self::handle_proc_command`]'s register/scope/walk glue
    /// largely as a separate function rather than a shared abstraction —
    /// the two definers' arity/local-binding stories diverge enough
    /// (`ProcDef.params` is `[args]` here, never `optlist`'s own entries)
    /// that factoring out a shared helper would need as many branches as
    /// duplicating the glue outright.
    pub fn handle_opt_proc_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 3 || arg_tokens.len() < 3 {
            return false;
        }

        let (raw_name, name_tok) = (&args[0], arg_tokens[0]);
        let resolved_name = self
            .resolve_dynamic_word(
                raw_name,
                Some(name_tok),
                arg_single.first().copied().unwrap_or(false),
                scope_path,
            )
            .unwrap_or_else(|| raw_name.clone());
        let ns_prefix = self.command_resolution_namespace(scope_path);
        let original_name = self.original_procedure_declaration_name(name_tok);
        let (source_name, qualified, simple) = self.procedure_reporting_names(
            original_name.as_ref(),
            name_tok,
            scope_path,
            &ns_prefix,
            &resolved_name,
        );
        let name_span = name_tok.span;
        let body_tok = arg_tokens[2];

        self.emit_w113_proc_shadows_builtin(
            &resolved_name,
            &qualified,
            name_span,
            source_name.as_ref(),
        );
        self.emit_w314_no_absolute_name(raw_name, name_span);

        let (real_params, opt_locals) = Self::opt_proc_params(&args[1], self.word_rules());

        let doc = self.procedure_definition_doc(args.get(2).map(String::as_str));

        // Combined list — the real `args` catch-all plus every
        // optlist-derived local — feeds hover/param-trait inference and
        // the body's own local-variable scope (never `ProcDef.params`,
        // which stays `[args]`-only for correct arity).
        let mut combined_params = real_params.clone();
        combined_params.extend(opt_locals.iter().cloned());

        let body_text = &args[2];
        // `tcl::OptProc`'s callers pass an option list, never a formal's
        // script, so its optlist locals carry no script contract.
        let (param_traits, caller_frame_params, caller_frame_literals, _) =
            self.infer_proc_param_traits(&combined_params, body_text, body_tok.span);

        let proc = ProcDef {
            source_name,
            name: simple,
            qualified_name: qualified.clone(),
            params: real_params,
            // `tcl::OptProc`'s Tcl-level signature is always the single
            // catch-all `args`, derived above — a known list, not a computed
            // one.
            params_computed: false,
            formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                tcl_dialect::ParameterGrammar::Tcl,
            ),
            name_span,
            body_span: body_tok.span,
            doc,
            param_traits,
            caller_frame_params,
            caller_frame_literals,
            script_parameters: std::collections::HashMap::new(),
        };

        // The optlist's locals remain distinct from native formal syntax;
        // this record retains only its genuine declaration name and report.
        self.retain_procedure_declaration_metadata(original_name.as_ref(), &proc, name_span);
        self.register_proc_definition(&qualified, &proc, name_span);
        let simple_key = proc.name.clone();
        let path = scope_path.to_vec();
        if let Some(scope) = super::super::scope::scope_at_mut(&mut self.result.global_scope, &path)
        {
            scope.procs.insert(simple_key.clone(), proc);
        }

        if body_tok.kind == TokenType::Str {
            // The *resolved* name keys the scope — see
            // `scope_name_for_routine`; the raw written word would invent a
            // namespace out of a substitution.
            let scope_name = scope_name_for_routine(&resolved_name).to_string();
            let child_path = self.push_proc_body_scope(&path, &scope_name, body_tok.span);

            self.bind_opt_proc_locals(arg_tokens[1], name_tok, &child_path, &opt_locals);

            let saved_comment = std::mem::take(&mut self.last_comment);
            let body_text: std::sync::Arc<str> = std::sync::Arc::from(args[2].as_str());
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
                        namespace: ns_prefix.clone(),
                        scope_name,
                        params: combined_params,
                        seeded_variables: Vec::new(),
                        // Attached later by `fill_deferred_bodies` for bodies
                        // with a fold candidate.
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
        }

        true
    }

    /// Split `tcl::OptProc`'s `optlist` argument into `(real_params,
    /// opt_locals)`: the real, arity-relevant Tcl-level
    /// signature — always the single catch-all `args`, regardless of what
    /// `optlist` declares — and `optlist`'s own descriptors, dash-stripped
    /// to the LOCAL VARIABLE name `::tcl::OptKeyParse` actually binds at
    /// runtime (used only for local-variable binding in the body, never
    /// for arity). A pure text transform — no analyser state needed — kept
    /// out of [`Self::handle_opt_proc_command`] purely to stay within the
    /// line-count lint.
    fn opt_proc_params(
        optlist_text: &str,
        rules: tcl_syntax::word_rules::WordValueRules,
    ) -> (
        Vec<crate::signature_scan::types::ParamDef>,
        Vec<crate::signature_scan::types::ParamDef>,
    ) {
        let real_params = vec![crate::signature_scan::types::ParamDef {
            name: "args".to_string(),
            has_default: false,
            default_value: None,
        }];
        let opt_locals: Vec<crate::signature_scan::types::ParamDef> =
            parse_param_list(optlist_text, rules)
                .into_iter()
                .map(|p| crate::signature_scan::types::ParamDef {
                    name: p
                        .name
                        .strip_prefix('-')
                        .map(str::to_string)
                        .unwrap_or(p.name),
                    has_default: p.has_default,
                    default_value: p.default_value,
                })
                .collect();
        (real_params, opt_locals)
    }
}
