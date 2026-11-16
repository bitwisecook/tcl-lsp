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

//! W308 and E001 on `$obj` and `[cmd]` receivers: an unknown method, or no method word at all.

use super::W308DiagnosticSite;
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use rustc_hash::FxHashSet;
use std::collections::HashSet;
use tcl_core_types::{BodyFrameId, DiagCode};

impl Analyser {
    /// Source-only classification of one selected method's actual body.
    /// It retains declared source applicability and supplies no entered method,
    /// MRO dispatch, normal completion or physical returned-value proof.
    pub(super) fn oo_self_method_returns_literal(&self, tokens: &crate::ir::CommandTokens) -> bool {
        self.oo_selected_method_body(tokens)
            .is_some_and(|body| self.oo_original_literal_return_body(&body))
    }

    fn oo_selected_method_body(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_lexer::NativeWord> {
        let input = self.result.resolved_input.as_ref()?;
        let config = self.file_lexer_config();
        let binding = tokens.source_binding.as_ref()?;
        let context = input.context_registry();
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            context.commands(),
            input,
        )?;
        let purpose =
            binding.original_source_advice_purpose(tokens, metadata, context.commands())?;
        let site = binding.invocation_site()?;
        let command = site
            .source
            .original_lexical_command(tokens.words(), site.offset, config)?;
        let selector = command.native_words().get(1)?;
        if selector.group().expand {
            return None;
        }
        if purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical {
            let request = self
                .result
                .logical_source_self_selector_at(&self.source, selector.span().start())?;
            if request.command_start() != site.offset || request.original_selector() != selector {
                return None;
            }
            let member = self.result.logical_source_member_at(
                selector.image(),
                config,
                selector.span().start(),
            )?;
            return member.original_body_word().cloned();
        }
        let references = super::super::super::class_hierarchy::original_metadata::original_source_self_method_references(&self.source, &self.result)?;
        let mut matching = references.iter().filter(|reference| {
            reference.source_schema().site().offset == site.offset
                && reference.selector_span()
                    == selector.content_span().ok().unwrap_or(selector.span())
        });
        let selected = matching.next()?;
        matching.next().is_none().then_some(())?;
        selected.method().body_word().cloned()
    }

    fn oo_original_literal_return_body(&self, body: &tcl_lexer::NativeWord) -> bool {
        self.oo_original_literal_return_operand(body).is_some()
    }

    fn oo_original_literal_return_operand(
        &self,
        body: &tcl_lexer::NativeWord,
    ) -> Option<tcl_lexer::NativeWord> {
        let input = self.result.resolved_input.as_ref()?;
        let context = input.context_registry();
        if body.group().kind != tcl_lexer::WordKind::Braced
            || body.group().expand
            || !self
                .result
                .matches_original_source_image(body.image(), input.lexer_config())
        {
            return None;
        }
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            body.image().clone(),
            body.content_span().ok()?,
            body.config(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        if plan.fatal_tail.is_some() || command.words.len() != 2 {
            return None;
        }
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            &self.source,
            &self.result,
            command.words.first()?.span().start(),
        )?;
        if words.arguments().len() != 1
            || words.with_source_schema(&context, |schema| {
                schema.facts().operation
                    == tcl_registry::SemanticOperationId::StructuredLowering(
                        tcl_registry::hooks::LoweringHookId::Return,
                    )
            }) != Some(true)
            || words.origins().get(1)
                != Some(&crate::registry_invocation::InvocationWordOrigin::Written(
                    1,
                ))
            || words.original_argument_word(0) != command.words.get(1)
        {
            return None;
        }
        let value = words.original_argument_word(0)?;
        tcl_syntax::word_rules::original_static_word_unicode_value(value)?;
        Some(value.clone())
    }

    /// An absent method name is diagnosed from the immutable method inventory
    /// captured by the actual instance receipt. A present one is held to the
    /// declaring class's own formals ([`Self::method_arity_diagnostic`]).
    pub(super) fn w308_for_object_var(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        instance: &crate::command_binding::SourceObjectInstanceProof,
        hierarchy: Option<&super::super::class_hierarchy::ClassHierarchy>,
    ) -> Option<super::super::types::Diagnostic> {
        let class = &instance.class_target().command;
        let Some(method) = site.method_name.as_ref() else {
            return self.e001_for_bare_object_dispatch(site, &HashSet::from([class.clone()]));
        };
        let input = self
            .head_identities
            .source_bindings_ref()
            .original_written_name_input_at_span(site.method_span?, self.file_lexer_config())?;
        input
            .policy()
            .recipe()
            .oo_method_input(input.bytes())
            .ok()?;
        if !Self::method_word_is_literal(method) {
            return None;
        }
        if instance.instance_method_names()?.contains(input.bytes()) {
            return self.method_arity_diagnostic(site, method, class, hierarchy?);
        }
        if self.disabled_diagnostics.contains("W308") {
            return None;
        }
        // The receipt includes private names only to avoid false absence
        // claims. It cannot safely supply public-method suggestions either.
        Some(super::super::types::Diagnostic::new(
            DiagCode::W308,
            site.method_span.unwrap_or(site.cmd_span),
            format!("Unknown method '{method}' on class '{class}'"),
            Severity::Warning,
        ))
    }

    /// The classes a profile without a native name protocol holds for a
    /// whole-variable receiver: the scoped constructor candidates completion
    /// reads. They union every assignment in the scope and carry no
    /// allocation proof, so they never reach a release profile's strict path.
    pub(super) fn logical_receiver_classes(
        &self,
        site: &crate::analyser::state::VarCommandSite,
    ) -> Option<Vec<String>> {
        if site.receiver != crate::analyser::state::DispatchReceiver::Variable {
            return None;
        }
        let head = self.original_dispatch_word(site.cmd_span)?;
        let mut classes = self
            .result
            .logical_source_receiver_class_candidates(&head, self.registry.as_deref()?)?
            .classes()
            .cloned()
            .collect::<Vec<_>>();
        classes.sort();
        Some(classes)
    }

    /// A method no candidate class provides draws W308; with one candidate a
    /// provided method is held to its formals. Tcl raises `unknown method`
    /// and `wrong # args` for these on every release with `TclOO`.
    pub(super) fn w308_for_logical_object_var(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        classes: &[String],
        hierarchy: Option<&super::super::class_hierarchy::ClassHierarchy>,
    ) -> Option<super::super::types::Diagnostic> {
        let Some(method) = site.method_name.as_ref() else {
            return self.e001_for_bare_object_dispatch(site, &classes.iter().cloned().collect());
        };
        if !Self::method_word_is_literal(method) || self.site_in_child_interp(site.cmd_span.start())
        {
            return None;
        }
        let reach = site.receiver.method_reach();
        if classes.iter().any(|class| {
            self.validate_method_on_class(
                class,
                method,
                self.result.all_classes.get(class),
                hierarchy,
                reach,
            )
        }) {
            let [class] = classes else {
                return None;
            };
            return self.method_arity_diagnostic(site, method, class, hierarchy?);
        }
        if self.disabled_diagnostics.contains("W308") {
            return None;
        }
        let names = classes.iter().map(String::as_str).collect::<Vec<_>>();
        Some(self.w308_diagnostic(method, &names.join(" | "), &names, hierarchy, site.into()))
    }

    /// Build the W308 (unknown method) diagnostic: anchored on the method
    /// *word* when the dispatch site recorded one (falling back to the
    /// command-head span), with a "did you mean…?" suggestion drawn from
    /// every method callable on the candidate classes — their MRO-resolved
    /// methods plus the implicit `TclOO` object builtins — and a replace
    /// fix targeting the same word.
    pub(super) fn w308_diagnostic(
        &self,
        method: &str,
        cls_display: &str,
        class_names: &[&str],
        hierarchy: Option<&super::super::class_hierarchy::ClassHierarchy>,
        site: W308DiagnosticSite,
    ) -> super::super::types::Diagnostic {
        let reach = site.reach;
        // Candidate methods for the suggestion: MRO methods of every
        // candidate class, locally-declared methods (for classes the
        // hierarchy may not index), and the implicit object builtins.
        let mut candidates: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for cls in class_names {
            if let Some(h) = hierarchy {
                candidates.extend(h.known_methods(self.registry.as_deref(), cls));
            }
            if let Some(cd) = self.result.all_classes.get(*cls) {
                candidates.extend(cd.methods.keys().cloned());
                candidates.extend(cd.class_methods.keys().cloned());
                if let Some(grammar) = self.class_definer_grammar(cls) {
                    candidates.extend(
                        grammar
                            .builtin_object_methods
                            .iter()
                            .filter(|builtin| {
                                grammar.builtin_object_method(builtin.name, reach).is_some()
                            })
                            .map(|builtin| builtin.name.to_string()),
                    );
                }
            }
        }
        let suggestions = crate::text::suggest_similar(
            method,
            candidates.iter().map(String::as_str),
            1,
            crate::text::scaled_max_distance(method),
        );
        let mut message = format!("Unknown method '{method}' on class '{cls_display}'");
        let mut fixes: Vec<super::super::types::CodeFix> = Vec::new();
        let span = site.method_span.unwrap_or(site.cmd_span);
        if let Some(best) = suggestions.first() {
            use std::fmt::Write as _;
            let _ = write!(message, "; did you mean '{best}'?");
            if let Some((fix_span, new_text)) = self.w308_configured_method_proposal(site, best) {
                fixes.push(super::super::types::CodeFix {
                    span: fix_span,
                    new_text,
                    description: format!("Replace with '{best}'"),
                    // W308: an edit-distance guess at the intended method.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                });
            }
        }
        crate::analyser::types::Diagnostic::new(DiagCode::W308, span, message, Severity::Warning)
            .with_fixes(fixes)
    }

    /// Deliberately proposed method text is data, not an original method key.
    /// Its sole receiver is the genuine second written word of this same
    /// original dispatch. Whole source/configuration and parent argv rejoin
    /// before the configured constructor can supply an edit.
    pub(super) fn w308_configured_method_proposal(
        &self,
        site: W308DiagnosticSite,
        proposed: &str,
    ) -> Option<(tcl_lexer::Span, String)> {
        let image = tcl_lexer::SourceImage::document(&self.source);
        let (word, offset) = self
            .head_identities
            .original_written_word_at_span_in_source(
                &image,
                site.method_span?,
                self.file_lexer_config(),
            )?;
        let (_, parent) = self
            .head_identities
            .invocation_at_source("", offset)
            .original_recorded_command()?;
        let [head, method, ..] = parent.words() else {
            return None;
        };
        if method.source().span != word.span()
            || word.group().expand
            || !(head.source().span.start() <= site.cmd_span.start()
                && site.cmd_span.end() <= head.source().span.end())
        {
            return None;
        }
        let source = tcl_syntax::word_rules::lexical_unicode_source_word(proposed, word.config())?;
        Some((word.span(), source))
    }

    /// **E001** (`TclOO` form) — `$obj` invoked with no method word at all.
    ///
    /// `TclOO`'s per-object command dispatcher requires a method name
    /// before it even attempts method resolution: `set o [C new]; $o`
    /// fails at run time with `wrong # args: should be "o method ?arg
    /// ...?"` regardless of whether the class declares an `unknown`
    /// handler, since the argument-count check runs before any method
    /// lookup — `unknown` is itself only reachable as the *result* of a
    /// failed lookup, and there is no name here to look up (confirmed
    /// against tclsh 9.0.4). This is the object-dispatch analogue of the
    /// registry ensemble's "missing subcommand" E001
    /// ([`Analyser::emit_arity_diagnostics`]) — same failure shape (a
    /// dispatcher invoked with no dispatch word), a different mechanism.
    ///
    /// Fires only when every candidate class is both locally known
    /// (`self.result.all_classes` — an external/unindexed class can't be
    /// vouched for) and a genuine `TclOO` metaclass
    /// (`DefinerFamily::TclOo` — `oo::class`, `oo::abstract`,
    /// `oo::configurable`, `oo::singleton`, …, resolved via the class's
    /// recorded `metaclass` and the registry's `definition_body`, never
    /// a hardcoded name). snit's generated dispatcher proc and `[incr
    /// Tcl]` are different mechanisms this analyser does not model
    /// precisely enough to make the same guarantee, so both abstain
    /// here — the same carve-out the with-method path already applies
    /// to snit (FP-OBJ-05).
    pub(super) fn e001_for_bare_object_dispatch(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        class_names: &HashSet<String>,
    ) -> Option<super::super::types::Diagnostic> {
        if site.argc != 0
            || class_names.is_empty()
            || self.site_in_child_interp(site.cmd_span.start())
        {
            return None;
        }
        let all_tcloo = class_names.iter().all(|cls| {
            self.result
                .all_classes
                .get(cls)
                .is_some_and(|cd| super::super::validity::is_tcloo_source_class(self, cd))
        });
        if !all_tcloo {
            return None;
        }
        Some(crate::analyser::types::Diagnostic::new(
            DiagCode::E001,
            site.cmd_span,
            format!("'{}' requires a method", site.var_name),
            Severity::Error,
        ))
    }

    /// **E001** (`TclOO` form) for a command-substitution head: a bare
    /// `[Dog new]` — or `[make]` where the lattice proves `make` an
    /// object-returning factory — invoked with no method word at all.
    ///
    /// Same failure and same gates as
    /// [`Self::e001_for_bare_object_dispatch`]: `TclOO`'s per-object
    /// dispatcher rejects a zero-word invocation before any method lookup
    /// (tclsh 9.0.4: `wrong # args: should be "::oo::Obj… method ?arg
    /// ...?"`, `-errorcode {TCL WRONGARGS}`), so an `unknown` handler
    /// cannot save it.  Fires only when the produced class is locally
    /// known **and** a genuine `TclOO` metaclass — snit / itcl dispatchers
    /// and external classes abstain, exactly as on the `$var` path.  The
    /// class identity comes from the type lattice / object-handle facts,
    /// never from a spelling match on the head.
    fn e001_for_bare_cmd_dispatch(
        &self,
        site: &crate::analyser::state::CmdCommandSite,
        class_name: Option<&str>,
    ) -> Option<super::super::types::Diagnostic> {
        if self.site_in_child_interp(site.cmd_span.start()) {
            return None;
        }
        let class_qn = self.canonicalise_class_name(class_name?)?;
        let is_tcloo = self
            .result
            .all_classes
            .get(&class_qn)
            .is_some_and(|cd| super::super::validity::is_tcloo_source_class(self, cd));
        if !is_tcloo {
            return None;
        }
        let inner = site.cmd_text.trim();
        let inner = inner
            .strip_prefix('[')
            .and_then(|w| w.strip_suffix(']'))
            .map_or(inner, str::trim);
        Some(crate::analyser::types::Diagnostic::new(
            DiagCode::E001,
            site.cmd_span,
            format!("'{inner}' requires a method"),
            Severity::Error,
        ))
    }

    /// Whether the dispatch site at `off` runs inside a **child
    /// interpreter's** actual isolated source body or an unavailable subject.
    /// The scope producer retains that relation independently of report names.  The object classes the E001
    /// gates vouch for live in the *main* interpreter's command table; a
    /// child interpreter has its own, in which the class command does not
    /// exist at all (`interp create sub; interp eval sub {[Dog new]}` fails
    /// `invalid command name "Dog"` in the child — a different error the
    /// unresolved-command machinery owns), so the `TclOO` zero-word E001
    /// abstains there rather than assert main-interp object semantics.
    pub(super) fn site_in_child_interp(&self, off: u32) -> bool {
        let image = tcl_lexer::SourceImage::document(&self.source);
        self.result
            .source_interpreter_scope_relation(&image, self.file_lexer_config(), off)
            != Some(crate::analyser::scope::SourceInterpreterScopeRelation::OriginalRoot)
    }

    /// The source-owned allocation and current dispatch receipt. Aggregate
    /// type labels and bare instance-name assistance supply no runtime proof.
    /// A named receiver (`C create obj; obj m`) has no variable read; its
    /// receipt is the object command selected at the dispatch itself.
    pub(super) fn live_instance_at_dispatch(
        &self,
        site: &crate::analyser::state::VarCommandSite,
    ) -> Option<crate::command_binding::SourceObjectInstanceProof> {
        if site.receiver == crate::analyser::state::DispatchReceiver::InstanceCommand {
            return self
                .head_identities
                .invocation_at_source("", site.cmd_span.start())
                .named_object_instance_at_dispatch()
                .cloned();
        }
        self.result
            .object_handle_facts
            .instance_in_scope(site.cmd_span.start(), &site.var_name)
            .cloned()
    }

    /// Emit W307/W308 for `[cmd] method` command-substitution dispatch sites.
    ///
    /// W307 fires only when the inner command's return type is unknown AND the
    /// call isn't an OO self-dispatch (`my` / `self`).  When the return type is
    /// a known class, the method is validated against the hierarchy and W308 is
    /// emitted instead.  Restores `cmd_command_sites` on exit.
    #[allow(
        clippy::too_many_lines,
        reason = "the diagnostic's closely coupled dispatch and type-validation cases share one restored-site lifecycle"
    )]
    pub(super) fn emit_cmd_command_diagnostics(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
        hierarchy: Option<&super::super::class_hierarchy::ClassHierarchy>,
        object_returning: &FxHashSet<BodyFrameId>,
        module: &crate::ir::Module,
    ) {
        let mut cmd_sites = std::mem::take(&mut self.cmd_command_sites);
        // A per-item body initially owns an isolated command world. Its
        // carriers have since been rebased into this document, but rebasing
        // cannot establish the surrounding class, alias, or mutation state.
        // Restore exact original-site proofs through the shared source owner
        // before either analysis strategy classifies a result. Missing sites
        // remain explicitly unknown; no catalogue or spelling fallback is used.
        let bindings = self.head_identities.source_bindings_ref();
        for site in &mut cmd_sites {
            for tokens in &mut site.commands {
                bindings.stamp_original_tokens(tokens);
            }
        }
        // The walk below takes `&mut self`, so this analysis's own registry
        // is held as a handle rather than re-borrowed from `self` per site.
        let context = self.analysis_context();
        let attached = self.registry.clone();
        let attached = attached.as_deref();
        for site in &cmd_sites {
            if self.is_namespaced_ensemble_dispatch(site) {
                continue;
            }
            // No blanket `in_method` suppression: an in-method `[cmd] method`
            // dispatch must earn its silence from a positive signal (a known
            // OBJECT return type, or `my`/`self` self-dispatch resolving to a
            // method that returns an object).
            //
            // Parse the command-substitution text into
            // ``head ?args...``.  ``cmd_text`` is what the
            // analyser captured from
            // ``SourceMap::token_text``; the leading ``[`` /
            // trailing ``]`` are stripped already because
            // ``content_offset`` skipped them.
            let Some(tokens) = site.commands.first().filter(|_| site.commands.len() == 1) else {
                continue;
            };
            let effective = crate::registry_invocation::effective_command_words(tokens);
            let words = effective
                .as_ref()
                .map_or(&[][..], |effective| effective.words.as_slice());
            let spellings = words
                .iter()
                .map(crate::ir::WordExpr::legacy_text)
                .collect::<Vec<_>>();
            let head = spellings.first().map_or("", String::as_str);
            let arg_strs = spellings
                .iter()
                .skip(1)
                .map(String::as_str)
                .collect::<Vec<_>>();
            let invocation =
                crate::registry_invocation::resolved_tokens_invocation_in_context(&context, tokens);

            // OO self-dispatch (`my <method>` / `self <method>`): by default
            // the return is treated as an object handle (suppress).  But when
            // the dispatched method resolves in the enclosing class and its
            // body is a simple `return <literal>`, the result is a plain
            // string, not an object — so the *outer* dispatch fires W307.
            // `my <method>` (self-dispatch) and `self <subcommand>`
            // (introspection) both return something the analyser treats as an
            // object handle by default. Both kinds come from the registry;
            // `next`/`nextto` are deliberately *not* included — they return
            // the next implementation's result, not a handle.
            if invocation
                .as_ref()
                .and_then(|invocation| {
                    attached.and_then(|registry| {
                        registry.method_dispatch_keyword(&invocation.facts.canonical_command)
                    })
                })
                .is_some_and(|kind| {
                    matches!(
                        kind,
                        tcl_registry::MethodDispatchKind::SelfDispatch
                            | tcl_registry::MethodDispatchKind::Introspection
                    )
                })
            {
                let returns_literal = self.oo_self_method_returns_literal(tokens);
                if returns_literal {
                    self.result
                        .diagnostics
                        .push(crate::analyser::types::Diagnostic::new(
                            DiagCode::W307,
                            site.cmd_span,
                            "Non-literal command name — cannot statically analyse".to_string(),
                            Severity::Warning,
                        ));
                    continue;
                }
                // `[self]` / `[self object]` is not just *some* self-dispatch
                // or introspection call whose return type happens to be
                // unknowable — the registry (`is_self_receiver_call`)
                // says this exact head/arg pair denotes the *current*
                // receiver, the same target `my <method>` dispatches on. So
                // the outer method word is validated against the enclosing
                // class (W308) instead of falling through as an opaque
                // object handle of unresolvable class.
                if attached
                    .is_some_and(|r| r.is_self_receiver_call(head, arg_strs.first().copied()))
                    && let Some(diag) = self.w308_for_self_receiver(site, hierarchy)
                {
                    self.result.diagnostics.push(diag);
                }
                continue;
            }

            let ret_type = self.cmd_head_return_type(tokens, registry);

            // ``Object`` return type — suppress W307; if the
            // class is known, validate the method (W308), and a dispatch
            // with *no* method word at all is the unconditional `TclOO`
            // "wrong # args" failure (E001).
            let is_object = ret_type.kind() == crate::types::TypeKind::Known
                && matches!(ret_type.tcl_type(), Some(tcl_registry::TclType::Object));
            if is_object {
                if site.method_name.is_none() {
                    if let Some(diag) = self.e001_for_bare_cmd_dispatch(site, ret_type.class_name())
                    {
                        self.result.diagnostics.push(diag);
                    }
                    continue;
                }
                if !self.disabled_diagnostics.contains("W308")
                    && let (Some(method), Some(class_name)) =
                        (site.method_name.as_ref(), ret_type.class_name().as_ref())
                {
                    let Some(cls_qn) = self.canonicalise_class_name(class_name) else {
                        continue;
                    };
                    let cd = self.result.all_classes.get(&cls_qn).cloned();
                    // `[Dog new] m` dispatches through the produced object's
                    // own command, so only its exported surface is reachable.
                    let method_ok = self.validate_method_on_class(
                        &cls_qn,
                        method,
                        cd.as_ref(),
                        hierarchy,
                        tcl_registry::definer::MethodReach::ObjectCommand,
                    );
                    if !method_ok && Self::method_word_is_literal(method) {
                        let diag = self.w308_diagnostic(
                            method,
                            class_name,
                            &[cls_qn.as_str()],
                            hierarchy,
                            W308DiagnosticSite {
                                reach: tcl_registry::definer::MethodReach::ObjectCommand,
                                method_span: site.method_span,
                                cmd_span: site.cmd_span,
                            },
                        );
                        self.result.diagnostics.push(diag);
                    }
                }
                continue;
            }

            // Nominal candidate metadata can suppress a type warning; it
            // cannot establish an object class or validate an outer method.
            let (object, callee) = self.nominal_tokens_object_result(tokens, registry, module);
            if object || callee.is_some_and(|callee| object_returning.contains(&callee)) {
                continue;
            }

            // Type is unknown — emit W307 (only the emit-half
            // for the residual unknown-type case).
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W307,
                    site.cmd_span,
                    "Non-literal command name — cannot statically analyse".to_string(),
                    Severity::Warning,
                ));
        }
        self.cmd_command_sites = cmd_sites;
    }

    /// Whether a recorded method word is a **literal** name the static
    /// method set can actually contradict.
    ///
    /// A word carrying a variable or command substitution (`$action`,
    /// `get$suffix`, `[pick]`) or a `{*}` expansion names something chosen
    /// at run time; "unknown method" is not a claim any static table is
    /// entitled to make about it. The recorded text is the reconstructed
    /// word, in which the walker re-brackets a substitution (`${x}` /
    /// `[…]`), so testing for the introducers covers both the written and
    /// the reconstructed spelling.
    ///
    /// Shared by every W308 emitter so a fix on one path cannot leave
    /// another false-positiving on the same input.
    pub(super) fn method_word_is_literal(method: &str) -> bool {
        !method.is_empty() && !method.contains(['$', '[']) && !method.contains("{*}")
    }

    /// Result type selected from the retained executed implementation.
    /// Concrete class results require construction closure or the actual
    /// procedure's return proof; unresolved results stay unknown.
    fn cmd_head_return_type(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> crate::types::TypeLattice {
        if let Some(binding) = &tokens.source_binding {
            if let Some(class) = binding.proved_construction_result(registry) {
                return crate::types::TypeLattice::object_of(class);
            }
            if let Some(target) = binding
                .proved_execution_target()
                .filter(|target| target.kind == crate::command_binding::BindingKind::Proc)
                && let Some(result) = self
                    .result
                    .object_handle_facts
                    .normal_procedure_result(target)
            {
                return result.clone();
            }
        }
        crate::registry_invocation::normal_representation_invocation_in_context(
            &self.analysis_context(),
            tokens,
        )
        .and_then(|invocation| invocation.result_representation_type())
        .map_or_else(
            crate::types::TypeLattice::overdefined,
            crate::types::TypeLattice::of,
        )
    }
}
