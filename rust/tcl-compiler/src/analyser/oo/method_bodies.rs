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

//! Walking method, constructor and class-initialisation bodies in their own scopes.

use super::super::scope::scope_at_mut;
use super::super::state::Analyser;
use super::super::types::{ClassDef, MethodDef, Scope, ScopeKind};
use super::super::utils::{param_name_spans_for_token, parse_param_list};
use super::{ClassBodyCtx, CollectedMethodBody, MemberForm, MethodBodyClass, record_method};
use crate::signature_scan::types::ParamDef;
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::Traits;
use tcl_registry::arg_role::ArgRole;

impl Analyser {
    /// Walk one class-level `initialise` / `initialize` body in a scope
    /// **keyed on the class**, and report the variables it declared there.
    ///
    /// The body is not a method — calling `self` / `classvariable` / `link`
    /// inside it is a runtime error ("`self` may only be called from inside
    /// a method", tclsh 9.0.4) — but it does run in a *per-class* frame:
    /// `namespace current` there is that class object's own namespace
    /// (`::oo::Obj20` for one class, `::oo::Obj22` for the next), with
    /// `namespace path` = `::oo::Helpers ::oo`. Walking every sibling
    /// class's init body in the one shared enclosing scope collided their
    /// declarations: a `classvariable NAME` read in *any* class's property
    /// setter then resolved to whichever class came first in the file, and
    /// find-references merged both classes' declarations into a single
    /// symbol — a definitive wrong answer on a statically decidable case,
    /// and one a rename would act on. tclsh 9.0.4 keeps
    /// the two independent: two `oo::configurable` classes whose setters
    /// each check their own `initialize`-declared list correctly reject the
    /// other's values.
    ///
    /// Returned names are seeded into this class's method bodies by the
    /// caller, which is what makes a setter's `classvariable NAME` read
    /// reach its own class's declaration. Deliberately unconditional, the
    /// same approximation the instance-`variable` seeding beside it already
    /// makes: the analyser does not track which methods actually issued the
    /// `classvariable` link, so a name declared in the init body is treated
    /// as visible (defined, never unused-warned) throughout the class.
    ///
    /// Walked inline rather than through `deferred_bodies` — a class-level
    /// init script is not one of the per-item-rebuildable method bodies,
    /// and this is the same inline walk it always had.
    pub(super) fn walk_class_init_body(
        &mut self,
        class_variables: &[String],
        class_qualified: &str,
        scope_path: &[usize],
        mb: &CollectedMethodBody,
    ) -> Vec<(String, Span)> {
        if mb.body_tok.kind != TokenType::Str {
            return Vec::new();
        }
        let init_qn = if class_qualified.is_empty() {
            mb.name.clone()
        } else {
            crate::naming::rooted_member_key(class_qualified, &mb.name)
        };
        let Some(init_idx) =
            scope_at_mut(&mut self.result.global_scope, scope_path).map(|parent| {
                let mut child = Scope::new(ScopeKind::Method, init_qn);
                child.body_span = Some(mb.body_tok.span);
                // The object frame resolves bare commands globally, exactly as a
                // method body's does — `namespace path` is `::oo::Helpers ::oo`
                // and the class's *defining* namespace is not searched.
                child.oo_global_resolution = true;
                // ...but it is NOT a method invocation, so `oo_method_frame`
                // stays false. The two facts
                // diverge here and nowhere else: the family *resolves*
                // (`namespace which -command link` → `::oo::Helpers::link`,
                // so W123 must stay silent) yet all but `my` raise `… may
                // only be called from inside a method` when called, so
                // completion and hover must not offer them. `my` is the
                // exception because it is `::oo::ObjN::my` rather than an
                // `::oo::Helpers` member and a class is an object — `my new`
                // in an `initialize` body really does make an instance.
                child.oo_method_frame = false;
                parent.children.push(child);
                parent.children.len() - 1
            })
        else {
            return Vec::new();
        };
        let mut init_path = scope_path.to_vec();
        init_path.push(init_idx);
        let body_start = mb.body_tok.span.start();
        for var in class_variables {
            let base =
                tcl_syntax::naming::split_element_ref(var).map_or(var.as_str(), |(root, _)| root);
            if base.is_empty() {
                continue;
            }
            self.define_var(
                base,
                mb.body_tok,
                &init_path,
                false,
                Some(Span::new(body_start, body_start)),
            );
        }
        let seeded: std::collections::HashSet<String> = class_variables
            .iter()
            .map(|v| {
                tcl_syntax::naming::split_element_ref(v)
                    .map_or(v.as_str(), |(root, _)| root)
                    .to_owned()
            })
            .collect();
        self.analyse_body(&mb.body_text, mb.body_tok, &init_path);
        // Whatever the walk itself recorded in this scope — no second,
        // name-matching parser here; the registry-driven walk already knows
        // which words a `variable` / `const` statement declares.
        super::super::scope::scope_at(&self.result.global_scope, &init_path).map_or_else(
            Vec::new,
            |scope| {
                scope
                    .variables
                    .iter()
                    .filter(|(name, _)| !seeded.contains(name.as_str()))
                    .map(|(name, def)| (name.clone(), def.definition_span))
                    .collect()
            },
        )
    }

    /// Collect display-only alias/target pairs from the selected catalogue.
    /// Positive source consumers authenticate original invocation receipts.
    pub(super) fn collect_oo_links(
        &self,
        method_bodies: &[CollectedMethodBody],
        class_def: &mut ClassDef,
    ) {
        for mb in method_bodies {
            if mb.body_tok.kind != TokenType::Str {
                continue;
            }
            let base = mb.body_tok.span.start() + u32::from(mb.body_tok.content_offset);
            let cmds = crate::segmenter::segment_commands_with_offset_and_config(
                &mb.body_text,
                base,
                self.file_lexer_config().nested(),
            );
            for cmd in &cmds {
                if cmd.is_partial
                    || !cmd.texts.first().is_some_and(|head| {
                        let generation = self.analysis_context();
                        generation
                            .context()
                            .resolve_spec(generation.commands(), head)
                            .is_some_and(|spec| {
                                spec.traits.contains(Traits::TCLOO_BINDS_METHOD_ALIAS)
                            })
                    })
                {
                    continue;
                }
                for word in cmd.texts.iter().skip(1) {
                    if crate::naming::is_dynamic_word(word) {
                        continue;
                    }
                    match crate::tcl_expr_eval::split_tcl_list(word, self.word_rules()).as_slice() {
                        [name] if !name.is_empty() => {
                            let key = {
                                let written = tcl_syntax::naming::canonical_written_command(name);
                                tcl_syntax::naming::unroot_rooted_key(&written)
                                    .unwrap_or(&written)
                                    .to_owned()
                            };
                            class_def.linked_members.entry(key.clone()).or_insert(key);
                        }
                        [name, target] if !name.is_empty() => {
                            class_def
                                .linked_members
                                .entry({
                                    let written =
                                        tcl_syntax::naming::canonical_written_command(name);
                                    tcl_syntax::naming::unroot_rooted_key(&written)
                                        .unwrap_or(&written)
                                        .to_owned()
                                })
                                .or_insert_with(|| target.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Walk a single `TclOO` method body in a fresh [`ScopeKind::Method`] scope.
    ///
    /// Pre-binds the method's formal parameters and the class's instance
    /// `variable`s as defined-but-not-warned locals (so reads of them do not
    /// false-fire W210 read-before-set / W214 unused), then re-walks the body
    /// through [`Self::analyse_body`] so its `$obj method` / `[cmd] method`
    /// dispatch sites are recorded (with `in_method = true`) for the W307 /
    /// W308 post-pass.
    pub(super) fn walk_method_body(
        &mut self,
        class_variables: &[String],
        var_decl_spans: &std::collections::HashMap<String, Span>,
        class: MethodBodyClass<'_>,
        scope_path: &[usize],
        mb: &CollectedMethodBody,
        oo_global_resolution: bool,
    ) {
        if mb.body_tok.kind != TokenType::Str {
            return;
        }
        let class_qualified = class.display;
        let original_context = (oo_global_resolution && !mb.class_side)
            .then(|| {
                super::super::types::OriginalLexicalMemberContext::from_class_body(
                    class.declaration?,
                    &mb.body_tok,
                    mb.params_tok.as_ref(),
                    self.resolved_analysis_input(),
                    &tcl_lexer::SourceImage::document(&self.source),
                    self.file_lexer_config(),
                )
            })
            .flatten()
            .map(Box::new);
        let original_body_declaration = oo_global_resolution
            .then(|| {
                super::super::types::OriginalSourceReceiverBodyDeclaration::from_class_body(
                    class.declaration?,
                    &mb.body_tok,
                    mb.params_tok.as_ref(),
                    self.resolved_analysis_input(),
                    &tcl_lexer::SourceImage::document(&self.source),
                )
            })
            .flatten();
        let method_qn = if class_qualified.is_empty() {
            mb.name.clone()
        } else {
            crate::naming::rooted_member_key(class_qualified, &mb.name)
        };
        // Instance-side `TclOO` method of a statically-named class:
        // `[self class]` answers the defining class in this frame, so
        // record the fact for the constant command-substitution fold.
        // `oo_global_resolution` is exactly the
        // `TclOO`-family gate (snit / itcl walkers pass `false`);
        // class-side members abstain (`self class` never answers the
        // written class there); a synthetic key (`::@objdefine@…`) is
        // not a class name.
        let oo_defining_class = (oo_global_resolution
            && !mb.class_side
            && !class_qualified.is_empty()
            && !class_qualified.contains('@'))
        .then(|| class_qualified.to_owned());
        let Some(method_idx) = ({
            scope_at_mut(&mut self.result.global_scope, scope_path).map(|parent| {
                let mut child = Scope::new(ScopeKind::Method, method_qn.clone());
                child.body_span = Some(mb.body_tok.span);
                child.original_member_context = original_context;
                child.original_receiver_body_declaration = original_body_declaration;
                child.oo_global_resolution = oo_global_resolution;
                // A real method invocation, unlike the class-level
                // `initialise` frame `walk_class_init_body` opens: the whole
                // `oo::Helpers` family is callable here, not merely
                // resolvable (tclsh 9.0.4 inside a constructor: `link zzz`
                // returns `::oo::ObjN::zzz`).
                child.oo_method_frame = oo_global_resolution;
                child.oo_defining_class.clone_from(&oo_defining_class);
                parent.children.push(child);
                parent.children.len() - 1
            })
        }) else {
            return;
        };
        let mut method_path = scope_path.to_vec();
        method_path.push(method_idx);
        // Formal parameters — defined, never unused-warned.  Anchor each
        // param's definition span at its name in the param-list literal so
        // go-to-definition / references / rename resolve to the
        // parameter, not the whole method body.  Falls back to the body token
        // when the param-list word or a name can't be located.
        let param_spans = mb
            .params_tok
            .map(|pt| param_name_spans_for_token(&self.source, pt));
        for (i, p) in mb.params.iter().enumerate() {
            let def_span = param_spans.as_ref().and_then(|s| s.get(i).copied());
            self.define_var(&p.name, mb.body_tok, &method_path, false, def_span);
        }
        if let Some(pt) = mb.params_tok {
            self.emit_w218_args_not_final(&mb.params, param_spans.as_deref().unwrap_or(&[]), pt);
        }
        // Class instance variables — visible in every method body.  Anchor
        // each one's definition span at its `variable v` declaration token
        // A seeded var must NEVER take the whole method-body span as its
        // `definition_span`, or a rename would rewrite the entire body.  When
        // the declaration span is unknown (e.g. a snit implicit var) fall back
        // to a zero-width span at the body start — harmless, never destructive.
        let body_start = mb.body_tok.span.start();
        for var in class_variables {
            let base =
                tcl_syntax::naming::split_element_ref(var).map_or(var.as_str(), |(root, _)| root);
            if base.is_empty() || mb.params.iter().any(|p| p.name == base) {
                continue;
            }
            let def_span = var_decl_spans
                .get(base)
                .copied()
                .unwrap_or_else(|| Span::new(body_start, body_start));
            self.define_var(base, mb.body_tok, &method_path, false, Some(def_span));
        }
        self.finish_method_body_walk(
            class_variables,
            mb,
            scope_path,
            method_path,
            method_qn,
            (oo_defining_class, oo_global_resolution),
        );
    }

    fn finish_method_body_walk(
        &mut self,
        class_variables: &[String],
        mb: &CollectedMethodBody,
        scope_path: &[usize],
        method_path: Vec<usize>,
        method_qn: String,
        (oo_defining_class, oo_global_resolution): (Option<String>, bool),
    ) {
        // Per-item shell pass: defer the method body for an isolated pass like
        // `handle_proc_command`. Carry the qualified name, defining namespace,
        // formal parameters, and pre-bound class instance variables.
        if self.defer_proc_bodies {
            let namespace = self.command_resolution_namespace(scope_path);
            let safe_interp_ctx = self.safe_interp_ctx_snapshot();
            self.deferred_bodies
                .push(super::super::per_item::DeferredBody {
                    resolved_input: Some(self.resolved_analysis_input()),
                    body_text: std::sync::Arc::from(mb.body_text.as_str()),
                    body_tok: mb.body_tok,
                    scope_path: method_path,
                    is_method: true,
                    oo_global_resolution,
                    namespace,
                    scope_name: method_qn,
                    params: mb.params.clone(),
                    // The shell walk already seeded real declaration spans; the
                    // graft retains them, so this pass needs only variable names.
                    seeded_variables: class_variables.to_vec(),
                    command_trust: None,
                    ensemble_targets: Vec::new(),
                    prefixless_ensembles: Vec::new(),
                    oo_defining_class,
                    safe_interp_ctx,
                });
        } else {
            self.analyse_body(&mb.body_text, mb.body_tok, &method_path);
        }
    }

    /// Analyse one snit method / constructor / etc. body in a method scope
    /// seeded with `seed_vars` (snit's implicit names + declared instance or
    /// type variables) and the method's formal parameters.  The name /
    /// parameter-list / value-var / body word positions all come from
    /// `member`'s registry arg-roles — a body-less member is a no-op.
    pub(super) fn extract_class_member(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        ctx: &mut ClassBodyCtx<'_>,
        seed_vars: &[String],
        form: &MemberForm<'_>,
    ) {
        let MemberForm {
            member,
            kind,
            label,
            visibility,
        } = *form;
        let context = self.analysis_context();
        let mask = Some(context.context().authoring_query());
        let Some(body_idx) = crate::registry_invocation::source_structure::trace_jim_source_option(
            "member.body_role",
            member.indices_for_call_in(args, mask, ArgRole::Body).next(),
        ) else {
            return;
        };
        let Some(body_text) = args.get(body_idx).cloned() else {
            return;
        };
        let body_tok = crate::registry_invocation::source_structure::trace_jim_source_option(
            "member.body_token",
            arg_tokens.get(body_idx).copied(),
        );
        let name_idx = member.indices_for_call_in(args, mask, ArgRole::Name).next();
        let params_idx = member
            .indices_for_call_in(args, mask, ArgRole::ParamList)
            .next();

        // A named member (`method NAME …`) uses its name word; the nameless
        // forms fall back to the caller's synthetic label (`<constructor>`,
        // `<oncget -opt>`, …), or `<body>` when even that is empty.
        let name = name_idx
            .and_then(|i| args.get(i))
            .cloned()
            .unwrap_or_else(|| {
                if label.is_empty() {
                    "<body>".to_string()
                } else {
                    label.to_string()
                }
            });
        // Formal parameters: the parameter-list word, plus any `VarWrite` word
        // (snit 1.x `onconfigure`'s value variable) modelled as a bound local.
        let mut params: Vec<ParamDef> = params_idx
            .and_then(|i| args.get(i))
            .map_or_else(Vec::new, |p| parse_param_list(p, self.word_rules()));
        for i in member.indices_for_call_in(
            args,
            Some(self.analysis_context().context().authoring_query()),
            ArgRole::VarWrite,
        ) {
            if let Some(v) = args.get(i) {
                params.push(ParamDef {
                    name: v.clone(),
                    has_default: false,
                    default_value: None,
                });
            }
        }
        let params_tok = params_idx.and_then(|i| arg_tokens.get(i).copied());

        let zero = Span::new(0, 0);
        let name_span = name_idx
            .and_then(|i| arg_tokens.get(i))
            .or_else(|| arg_tokens.first())
            .map_or(zero, |t| t.span);
        let body_span = body_tok.map_or(name_span, |t| t.span);
        let method_def = MethodDef {
            name: name.clone(),
            params: params.clone(),
            params_computed: false,
            formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                tcl_dialect::ParameterGrammar::Tcl,
            ),
            name_span,
            body_span,
            kind: kind.as_str().to_string(),
            is_self_method: false,
            visibility: visibility.to_string(),
            doc: String::new(),
            forward_target: None,
        };
        record_method(ctx.class_def, kind, method_def);

        // Walk the body in a method scope seeded with the params + seed vars,
        // reusing the TclOO method-body walker (it pre-binds the params and the
        // supplied vars as never-warn locals, then analyses the body). snit /
        // itcl members resolve in the type / class namespace, so
        // `oo_global_resolution` stays false here.
        if let Some(bt) = body_tok {
            let mb = CollectedMethodBody {
                name,
                params,
                body_text,
                body_tok: bt,
                params_tok,
                // snit / itcl members are not `TclOO` frames — no
                // `[self class]` fact may be derived from them.
                class_side: true,
            };
            // snit / itcl seed vars are mostly grammar-injected implicits with no
            // source declaration token; an empty span map makes `walk_method_body`
            // fall back to a safe zero-width span (never the body span), so a
            // rename can't overwrite the body.  Precise snit declaration spans
            // are not modelled.
            let no_var_spans = std::collections::HashMap::new();
            self.walk_method_body(
                seed_vars,
                &no_var_spans,
                MethodBodyClass {
                    display: ctx.class_qualified,
                    declaration: None,
                },
                ctx.scope_path,
                &mb,
                false,
            );
        }
    }
}
