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

//! Class creation calls and `oo::define`: the class each records and the walk of its definition body.

use super::super::super::state::Analyser;
use super::super::super::types::{ClassDef, MetaclassProvenance};
use super::super::qualify;
use super::{InjectedMember, ManufacturerLayout, ResolvedClassCreation};
use tcl_lexer::Token;

// Immutable original source-body query shared by the actual handler and
// direct source-capability controls. This supplies no handler/body entry.
pub(in super::super) fn original_class_definition_body_in_analysis(
    source: &str,
    analysis: &super::super::super::types::AnalysisResult,
    offset: u32,
    token: Token,
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
) -> Option<std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBody>> {
    let input = analysis.resolved_input.as_ref()?;
    let context = input.context_registry();
    let original = crate::registry_invocation::source_structure::source_registry_words_at(
        source, analysis, offset,
    );
    let words = match original {
        Some(words)
            if matches!(words.source(), crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(_)) => words,
        original => crate::registry_invocation::source_structure::source_class_declaration_at(
            source, analysis, offset,
        )
        .and_then(|declaration| {
            crate::registry_invocation::source_structure::source_transition_words_from_advice(
                source,
                input.lexer_config(),
                &context,
                declaration.factory().clone(),
            )
        })
        .or(original)?,
    };
    (words.with_source_schema(&context, |schema| {
        schema
            .authored_source_definition_body_grammar()
            .is_some_and(|selected| std::ptr::eq(selected, grammar))
    }) == Some(true))
    .then_some(())?;
    let mut bodies = words
        .source_script_bodies(&context)
        .into_iter()
        .filter(|body| body.original_container().tokens().first() == Some(&token));
    let body = bodies.next()?;
    bodies.next().is_none().then(|| std::sync::Arc::new(body))
}

impl Analyser {
    fn retain_manufactured_class_declaration(
        &mut self,
        original_name: Option<&crate::signature_scan::original_name::SourceOriginalNameOccurrence>,
        qualified: String,
        class: ClassDef,
        scope_path: &[usize],
    ) {
        self.retain_original_provider_class(original_name, &class);
        let simple_key = class.name.clone();
        // The creation site's own body span, for `my`-dispatch resolution
        // to find — see `class_body_spans`'s doc.
        self.result
            .class_body_spans
            .push((qualified.clone(), class.body_span));
        self.result
            .retain_class_declaration(qualified, class.clone());
        let path = scope_path.to_vec();
        if let Some(scope) =
            super::super::super::scope::scope_at_mut(&mut self.result.global_scope, &path)
        {
            scope.classes.insert(simple_key, class);
        }
    }

    fn apply_manufactured_class_members(
        &self,
        grammar: Option<&tcl_registry::definer::DefinitionBodyGrammar>,
        injected_members: &[InjectedMember],
        class: &mut ClassDef,
    ) {
        if let Some(grammar) = grammar {
            for injected in injected_members {
                super::super::super::oo::apply_oo_subcommand_in(
                    grammar,
                    &injected.texts,
                    &injected.argv,
                    class,
                    super::super::super::oo::MemberDialect {
                        surface: Some(self.analysis_context().context().authoring_query()),
                        rules: self.word_rules(),
                    },
                );
            }
        }
    }

    /// A creation the load walk proves unreached (it follows a proved error
    /// in an executed unit) runs in no proof environment, so it declares no
    /// class. A body never called is a separate, display case.
    pub(in super::super::super) fn load_creation_proved_unreached(
        &self,
        cmd_name: &str,
        start: u32,
    ) -> bool {
        !self.result.offset_is_inside_any_definition_body(start)
            && self
                .head_identities
                .invocation_at_source(cmd_name, start)
                .runtime_reachability()
                == crate::command_binding::SourceRuntimeReachability::NotEntered
    }

    /// Handle `oo::class create NAME ?BODY?` — record the class.
    ///
    /// Records a [`super::super::super::types::ClassDef`] in ``result.all_classes``
    /// (walking the body when present to populate method / mixin /
    /// superclass info) so consumers see the class in the workspace
    /// index. Returns `true` when the command shape matched.
    pub fn handle_oo_class_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        cmd_tok: Token,
    ) -> bool {
        self.handle_oo_class_source(super::super::super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            cmd_tok: Some(cmd_tok),
            original: None,
        })
    }

    pub(in super::super::super) fn handle_oo_class_source(
        &mut self,
        call: super::super::super::original_definer::ClassDefinerCall<'_>,
    ) -> bool {
        let super::super::super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            cmd_tok,
            original,
        } = call;
        let Some(cmd_tok) = cmd_tok else {
            return false;
        };
        if self.load_creation_proved_unreached(cmd_name, cmd_tok.span.start()) {
            return false;
        }
        // Keep the written head for original source and factory lookups.
        let Some(ResolvedClassCreation {
            user_factory,
            registry_metaclass,
            layout,
        }) = self.resolve_class_creation(args, arg_tokens, cmd_tok)
        else {
            // Anonymous class manufacturers have no source-level class name
            // to put in `all_classes`; ordinary instance manufacturers have
            // no class-definition body. Both are handled by their respective
            // object-value paths instead.
            //
            // …and a third case, which is not a verdict at all: the head may
            // name a metaclass this document proves only *after* the walk.
            // Keep it for the post-pass.
            self.defer_class_creation(cmd_name, args, arg_tokens, scope_path, cmd_tok);
            return false;
        };
        if layout.name_arg >= args.len() || layout.name_arg >= arg_tokens.len() {
            return false;
        }
        let raw_name = &args[layout.name_arg];
        // Home the class to the command-resolution namespace (see the same
        // reasoning in `handle_proc_command`): a class created inside a
        // qualified-name proc's body homes to that proc's defining namespace,
        // not the lexical global, so it can't overwrite a same-named global
        // class in `all_classes`.
        let ns_prefix = self.command_resolution_namespace(scope_path);
        let original_name = original
            .and_then(
                super::super::super::original_definer::OriginalClassDefinerSource::original_name,
            )
            .or_else(|| self.original_provider_class_name(arg_tokens[layout.name_arg].span));
        let source_name = original_name.as_ref().and_then(|original| {
            crate::signature_scan::scope::SignatureSourceCommand::object_from_key(
                &self.declaration_namespace_scope(scope_path)?,
                original.name_input(),
            )
        });
        let qualified = source_name
            .as_ref()
            .and_then(crate::signature_scan::scope::SignatureSourceCommand::reported_full_name)
            .unwrap_or_else(|| qualify(&ns_prefix, raw_name));
        let simple = source_name
            .as_ref()
            .and_then(crate::signature_scan::scope::SignatureSourceCommand::simple_name)
            .unwrap_or_else(|| crate::naming::key_tail(&qualified).to_string());
        let name_span = arg_tokens[layout.name_arg].span;
        // **W314** — the class name has no absolute written form.
        self.emit_w314_no_absolute_name(raw_name, name_span);
        let body_tok_opt = arg_tokens.get(layout.body_arg).copied();
        let body_span = body_tok_opt.map_or(name_span, |t| t.span);
        let doc = std::mem::take(&mut self.last_comment);
        let metaclass_lookup = self.declaration_name_policy().and_then(|policy| {
            crate::signature_scan::scope::SignatureSourceLookup::new(
                policy,
                self.declaration_namespace_scope(scope_path)?,
                cmd_name.to_owned(),
            )
        });
        let mut class = super::super::super::types::ClassDef {
            source_name,
            metaclass_lookup,
            name: simple,
            qualified_name: qualified.clone(),
            name_span,
            body_span,
            metaclass: registry_metaclass.map_or_else(
                || cmd_name.to_owned(),
                |descriptor| descriptor.name.to_owned(),
            ),
            metaclass_provenance: MetaclassProvenance::Observed,
            doc,
            inheritance_unknown: layout.inheritance_unknown,
            ..Default::default()
        };
        // A user metaclass has no spec of its own, so the bodies it makes are
        // governed by the definition grammar of the registry metaclass at the
        // root of its superclass chain — the same `TclOO` grammar, reached
        // without naming it here.
        let grammar = original
            .map(super::super::super::original_definer::OriginalClassDefinerSource::grammar)
            .or_else(|| {
                user_factory.as_ref().map_or_else(
                    || registry_metaclass.and_then(|descriptor| descriptor.definition_body),
                    |factory| self.definition_grammar(&factory.root_metaclass),
                )
            });
        self.apply_original_created_class_body(&mut class, &layout, grammar, call, body_tok_opt);
        // **W315** — a retraction the body could not legally make. Drained
        // here, after the whole body walk, so a class extended by several
        // `oo::define` blocks reports each block's own aborts once.
        self.emit_w315_definition_cannot_run(&mut class);
        // Is the class we just recorded *itself* a class factory?  Answered
        // here, once, so every later `ThisClass create …` — in this file or,
        // via the workspace factory index, in another — reads a derived fact
        // instead of re-walking the superclass chain and re-segmenting the
        // manufacturer override per call site.
        class.factory = self.class_factory_of(&class);
        // …and does the *manufacturer* let this class be constructed by a
        // bare word (Tk's `::tk::IconList .il`)?  Settled here too, for the
        // same reason: the proof lives on the metaclass, which a consuming
        // document may never see, so the answer travels on the class rather
        // than the question.
        if user_factory
            .as_ref()
            .is_some_and(|factory| factory.unknown_binds_instance)
        {
            class.class_command_fallback =
                super::super::super::types::ClassCommandFallback::ConstructsNamedInstance;
        }
        // Register globally and in the current scope, the same as
        // the proc registration path: ``result.all_classes`` is keyed
        // by the fully-qualified name; the per-scope
        // ``scope.classes`` map is keyed by the bare (unqualified)
        // name so per-scope lookups and shadowing rules work.
        self.retain_manufactured_class_declaration(
            original_name.as_ref(),
            qualified,
            class,
            scope_path,
        );
        true
    }

    fn apply_original_created_class_body(
        &mut self,
        class: &mut super::super::super::types::ClassDef,
        layout: &ManufacturerLayout,
        grammar: Option<&'static tcl_registry::definer::DefinitionBodyGrammar>,
        call: super::super::super::original_definer::ClassDefinerCall<'_>,
        body_token: Option<Token>,
    ) {
        self.apply_manufactured_class_members(grammar, &layout.injected, class);
        if let (Some(body_text), Some(body_tok)) = (call.args.get(layout.body_arg), body_token) {
            let disabled = self.command_dialect_disabled(call.cmd_name);
            let body = call.cmd_tok.zip(grammar).and_then(|(head, grammar)| {
                self.original_class_definition_body_at(head.span.start(), body_tok, grammar)
            });
            let enclosing = std::mem::replace(&mut self.original_class_definition_body, body);
            self.parse_oo_definition_body(
                body_text,
                body_tok,
                class,
                call.scope_path,
                grammar,
                disabled,
            );
            self.original_class_definition_body = enclosing;
        }
    }

    pub(in super::super::super) fn original_class_definition_body_at(
        &self,
        offset: u32,
        token: Token,
        grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    ) -> Option<std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBody>> {
        original_class_definition_body_in_analysis(
            &self.source,
            &self.result,
            offset,
            token,
            grammar,
        )
    }

    /// Class definition advice from the selected definition grammar.
    /// Native source targets require a genuine original class/configuration
    /// receipt; unjoined targets retain only lexical body analysis. Positively
    /// retained Logical inputs can also publish cross-document advisory stubs.
    /// Reporting tables establish neither Native identity nor definition entry.
    pub fn handle_oo_define_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        let enclosing = self.original_class_definition_body.take();
        let result = self
            .handle_oo_define_with_source(cmd_name, args, arg_tokens, arg_single, scope_path, None);
        self.original_class_definition_body = enclosing;
        result
    }

    pub(in super::super::super) fn handle_oo_define_source(
        &mut self,
        original: &super::super::super::original_definer::OriginalClassConfigurationSource,
        scope_path: &[usize],
    ) -> bool {
        let body = original
            .tokens
            .get(1)
            .zip(
                original
                    .target
                    .class_declaration()
                    .grammar(&self.analysis_context()),
            )
            .and_then(|(token, grammar)| {
                self.original_class_definition_body_at(
                    original.target.consumer().site().offset,
                    *token,
                    grammar,
                )
            });
        let enclosing = std::mem::replace(&mut self.original_class_definition_body, body);
        let result = self.handle_oo_define_with_source(
            original.target.consumer().command(),
            &original.arguments,
            &original.tokens,
            &original.single,
            scope_path,
            Some(original),
        );
        self.original_class_definition_body = enclosing;
        result
    }

    fn handle_oo_define_with_source(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        source: Option<&super::super::super::original_definer::OriginalClassConfigurationSource>,
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        // Distinguish body-form from inline-form by inspecting
        // ``args[1]``.  Body-form: ``oo::define Class { ... }``
        // — args[1] is a single body argument.  Inline-form:
        // ``oo::define Class method foo {} {}`` — args[1] is a
        // known define subcommand.
        if args.len() < 2 {
            return true;
        }

        let source_class = source.map(|source| source.target.class_declaration());
        if let Some(target) = arg_tokens.first() {
            self.retain_original_class_configuration_metadata_with_source(
                target.span,
                source_class,
            );
        }
        let original_class = source_class.cloned().or_else(|| {
            arg_tokens
                .first()
                .and_then(|token| self.original_oo_configuration_class_declaration(*token))
        });
        let selected = original_class.as_ref().and_then(|declaration| {
            declaration
                .source_class(&self.result)
                .map(|record| record.metadata().qualified_name.clone())
                .or_else(|| {
                    declaration
                        .logical_source_class(&self.result)
                        .map(|class| class.qualified_name.clone())
                })
        });
        if selected.is_none()
            && let Some(qualified) = arg_tokens
                .first()
                .and_then(|token| self.external_configuration_target(*token))
        {
            self.record_external_class_contribution(
                qualified, cmd_name, args, arg_tokens, scope_path,
            );
            return true;
        }
        let Some(qualified) = selected
            .or_else(|| self.oo_define_qualified_target(args, arg_tokens, arg_single, scope_path))
        else {
            // An unjoined target still owns its lexical definition body. This
            // temporary holder is not published under an invented class key.
            let mut holder = super::super::super::types::ClassDef {
                via_define: true,
                ..Default::default()
            };
            self.walk_oo_define_form(cmd_name, args, arg_tokens, scope_path, &mut holder);
            return true;
        };

        let (name_span, this_call_span) = Self::oo_definition_source_spans(arg_tokens, source);
        self.result
            .class_body_spans
            .push((qualified.clone(), this_call_span));
        let mut class_def = self.oo_definition_source_class(
            &qualified,
            original_class.as_ref(),
            name_span,
            this_call_span,
        );

        self.walk_oo_define_form(cmd_name, args, arg_tokens, scope_path, &mut class_def);

        // **W315** — as on the creation path, except that a `via_define` stub's
        // aborts are dropped rather than reported: a class created in another
        // file leaves this record with no member tables to judge against.
        self.emit_w315_definition_cannot_run(&mut class_def);
        // `oo::define C { superclass oo::class }` turns an ordinary class into
        // a factory, and `self method create …` changes the shape of the
        // classes it makes, so the derived factory record is recomputed here
        // exactly as it is on the creation path.
        class_def.factory = self.class_factory_of(&class_def);
        if let Some(original) = &original_class {
            self.retain_original_source_class_update(original, &class_def);
        }
        self.register_defined_class(qualified, class_def, scope_path);
        true
    }

    /// The class an original configuration names when this document
    /// declares no such class under a pinned release: the first path its
    /// original lookup tries. Another document may declare it.
    fn external_configuration_target(&self, token: Token) -> Option<String> {
        if self.result.allows_retained_logical_declaration_advice() {
            return None;
        }
        let configuration = self
            .result
            .original_class_configurations()
            .find(|configuration| {
                configuration
                    .target()
                    .name_input()
                    .original_word_key()
                    .is_some_and(|key| {
                        key.span() == token.span
                            || key
                                .original_word()
                                .tokens()
                                .first()
                                .is_some_and(|written| written.span == token.span)
                    })
            })?;
        let target = configuration.target();
        if target.local_allocation().is_some() || !target.is_external_candidate() {
            return None;
        }
        let lookup = target.lookup();
        let [candidates] = lookup.candidates() else {
            return None;
        };
        tcl_syntax::naming::native_command_source_spelling(
            lookup.policy().recipe(),
            candidates.first()?,
        )
    }

    /// Walk a configuration of a class another document may declare into
    /// its contribution record. The record is no class of this document:
    /// run alone, the configuration raises (`::C does not refer to an
    /// object`); the workspace index merges it into the declaring
    /// document's class.
    fn record_external_class_contribution(
        &mut self,
        qualified: String,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let (name_span, body_span) = Self::oo_definition_source_spans(arg_tokens, None);
        let mut contribution = self
            .result
            .external_class_contributions
            .remove(&qualified)
            .unwrap_or_else(|| super::super::super::types::ClassDef {
                name: crate::naming::key_tail(&qualified).to_owned(),
                qualified_name: qualified.clone(),
                name_span,
                body_span,
                via_define: true,
                ..Default::default()
            });
        self.walk_oo_define_form(cmd_name, args, arg_tokens, scope_path, &mut contribution);
        self.emit_w315_definition_cannot_run(&mut contribution);
        self.result
            .external_class_contributions
            .insert(qualified, contribution);
    }

    fn oo_definition_source_class(
        &mut self,
        qualified: &str,
        original: Option<&crate::command_binding::OriginalSourceClassDeclaration>,
        name_span: tcl_lexer::Span,
        body_span: tcl_lexer::Span,
    ) -> super::super::super::types::ClassDef {
        let canonical = original.and_then(|original| {
            original
                .source_class(&self.result)
                .map(|record| record.metadata().clone())
                .or_else(|| original.logical_source_class(&self.result).cloned())
        });
        canonical
            .or_else(|| {
                self.result
                    .allows_retained_logical_declaration_advice()
                    .then(|| self.result.all_classes.remove(qualified))
                    .flatten()
            })
            .unwrap_or_else(|| super::super::super::types::ClassDef {
                name: crate::naming::key_tail(qualified).to_owned(),
                qualified_name: qualified.to_owned(),
                name_span,
                body_span,
                via_define: true,
                ..Default::default()
            })
    }

    /// Report geometry follows the actual whole selected installer. A
    /// captured target's producer does not become the configuration site.
    fn oo_definition_source_spans(
        arguments: &[Token],
        source: Option<&super::super::super::original_definer::OriginalClassConfigurationSource>,
    ) -> (tcl_lexer::Span, tcl_lexer::Span) {
        let name = arguments
            .first()
            .map_or(tcl_lexer::Span::empty(0), |word| word.span);
        let original = source.and_then(|source| {
            let words = source.target.consumer().original_words();
            Some(tcl_lexer::Span::new(
                words.first()?.word_span().start(),
                words.last()?.word_span().end(),
            ))
        });
        let call = original.unwrap_or_else(|| {
            arguments.last().map_or(name, |last| {
                tcl_lexer::Span::new(name.start(), last.span.end())
            })
        });
        (name, call)
    }

    /// Keep the global class index and the enclosing lexical scope's class
    /// map in lockstep for any definition form that has produced a class fact.
    pub(in super::super::super) fn register_defined_class(
        &mut self,
        qualified: String,
        class_def: super::super::super::types::ClassDef,
        scope_path: &[usize],
    ) {
        let simple = class_def.name.clone();
        self.result
            .retain_class_declaration(qualified, class_def.clone());
        if let Some(scope) =
            super::super::super::scope::scope_at_mut(&mut self.result.global_scope, scope_path)
        {
            scope.classes.insert(simple, class_def);
        }
    }

    fn original_oo_configuration_class_declaration(
        &self,
        token: Token,
    ) -> Option<crate::command_binding::OriginalSourceClassDeclaration> {
        let configuration = self
            .result
            .original_class_configurations()
            .find(|configuration| {
                configuration
                    .target()
                    .name_input()
                    .original_word_key()
                    .is_some_and(|key| {
                        key.span() == token.span
                            || key
                                .original_word()
                                .tokens()
                                .first()
                                .is_some_and(|original| original.span == token.span)
                    })
            })?;
        let allocation = configuration.target().local_allocation()?;
        let mut originals = self
            .result
            .original_class_declarations()
            .filter(|declaration| declaration.declaration_site() == &allocation.site);
        let original = originals.next()?;
        if originals.next().is_some() {
            return None;
        }
        let declaration =
            crate::registry_invocation::source_structure::source_class_declaration_at(
                &self.source,
                &self.result,
                original.declaration_site().offset,
            )?;
        (declaration.source_class(&self.result)?.declaration_site() == original.declaration_site())
            .then_some(declaration)
    }

    /// Resolve the `oo::define` target to its class key. A statically known
    /// substitution retains its actual class name; an unresolved dynamic word
    /// gets a call-site-specific synthetic key so unrelated definitions never
    /// merge state merely because they spell the same variable.
    fn oo_define_qualified_target(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> Option<String> {
        if let Some(token) = arg_tokens.first()
            && let Some(configuration) =
                self.result
                    .original_class_configurations()
                    .find(|configuration| {
                        configuration
                            .target()
                            .name_input()
                            .original_word_key()
                            .is_some_and(|key| {
                                key.span() == token.span
                                    || key
                                        .original_word()
                                        .tokens()
                                        .first()
                                        .is_some_and(|written| written.span == token.span)
                            })
                    })
        {
            if let Some(allocation) = configuration.target().local_allocation() {
                let mut declarations = self
                    .result
                    .original_class_declarations()
                    .filter(|declaration| declaration.declaration_site() == &allocation.site);
                let declaration = declarations.next()?;
                if declarations.next().is_some() {
                    return None;
                }
                return Some(declaration.metadata().qualified_name.clone());
            }
            if !configuration.target().is_external_candidate() {
                return None;
            }
        }
        // Native targets require the genuine original configuration issuer.
        // Missing source category/layout never re-enters a reporting-name map.
        if !self.result.allows_retained_logical_declaration_advice() {
            return None;
        }
        let resolved = self
            .resolve_dynamic_word(
                &args[0],
                arg_tokens.first().copied(),
                arg_single.first().copied().unwrap_or(false),
                scope_path,
            )
            .filter(|name| !crate::naming::is_dynamic_word(name));
        let raw = resolved.as_ref().unwrap_or(&args[0]);
        let dynamic_key = crate::naming::is_dynamic_word(raw).then(|| {
            arg_tokens.first().map_or_else(
                || raw.clone(),
                |token| self.mint_synthetic_offset_name("@dynclass@", token.span.start()),
            )
        });
        if dynamic_key.is_none()
            && let Some(selected) = self.resolve_user_class_in(raw, scope_path)
        {
            return Some(selected);
        }
        let namespace = self.command_resolution_namespace(scope_path);
        let presentation = qualify(&namespace, dynamic_key.as_deref().unwrap_or(raw));
        if self.result.all_classes.contains_key(&presentation) {
            return None;
        }
        Some(presentation)
    }

    /// Apply an `oo::define` inline member or braced body through the selected
    /// definition grammar. Both forms receive identical dialect diagnostics
    /// and registry-driven command-reference recording.
    fn walk_oo_define_form(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        class_def: &mut super::super::super::types::ClassDef,
    ) {
        let inline_form = self
            .definition_grammar("oo::define")
            .is_some_and(|grammar| grammar.is_member(&args[1]));
        if inline_form {
            let inline_args: Vec<String> = args[1..].to_vec();
            let inline_tokens: Vec<Token> = arg_tokens.iter().skip(1).copied().collect();
            if let Some(grammar) = self.definition_grammar(cmd_name) {
                let definer_disabled = self.command_dialect_disabled(cmd_name);
                if let (Some(subcmd), Some(token)) = (inline_args.first(), inline_tokens.first()) {
                    self.emit_w002_oo_member_disabled(grammar, subcmd, *token, definer_disabled);
                }
                if let Some(member) = inline_args
                    .first()
                    .and_then(|subcmd| grammar.member(subcmd))
                {
                    self.emit_w002_oo_member_option_disabled(
                        member,
                        &inline_args[1..],
                        &inline_tokens[1..],
                        definer_disabled,
                    );
                }
                super::super::super::oo::parse_oo_define_inline_in(
                    self,
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    class_def,
                    Some(self.analysis_context().context().authoring_query()),
                    scope_path,
                );
                self.record_member_command_references(
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    scope_path,
                );
            }
        } else if let Some(body_tok) = arg_tokens.get(1).copied() {
            let grammar = self.definition_grammar(cmd_name);
            let definer_disabled = self.command_dialect_disabled(cmd_name);
            self.parse_oo_definition_body(
                &args[1],
                body_tok,
                class_def,
                scope_path,
                grammar,
                definer_disabled,
            );
        }
    }
}
