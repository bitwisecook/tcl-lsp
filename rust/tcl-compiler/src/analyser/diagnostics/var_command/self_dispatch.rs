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

//! W308 on `my`, `[self]` and the other receivers that name the object whose method is running.

use super::W308DiagnosticSite;
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use tcl_core_types::DiagCode;

impl Analyser {
    /// The `ClassDef` whose body contains `offset` — the enclosing `TclOO`
    /// class for a call site inside a method body, if any. Naive
    /// first-match over [`AnalysisResult::all_classes`] (class bodies don't
    /// nest in practice); shared by [`Self::oo_self_method_returns_literal`]
    /// and the `[self]`/`[self object]` self-receiver W308 check, so the two
    /// "what class is this dispatch inside" answers
    /// cannot drift apart.
    fn enclosing_class_at_offset(&self, offset: u32) -> Option<&super::super::types::ClassDef> {
        self.result.all_classes.values().find(|class_def| {
            let body = class_def.body_span;
            body.start() <= offset && offset <= body.end()
        })
    }

    /// W308 for a `[self]` / `[self object]` dispatch head confirmed by
    /// [`tcl_registry::CommandRegistry::is_self_receiver_call`] — the
    /// receiver is the *enclosing* class, found via
    /// [`Self::enclosing_class_at_offset`], exactly as a bareword `my
    /// <method>` dispatch's target is.
    ///
    /// The substitution yields the object's own **command**, so it reaches
    /// only exported methods —
    /// [`MethodReach::ObjectCommand`](tcl_registry::definer::MethodReach::ObjectCommand),
    /// not `SelfDispatch`. tclsh 9.0.4 and 8.6.16 agree: `[self] varname v`
    /// fails with `unknown method "varname"` where `my varname v` succeeds.
    pub(super) fn w308_for_self_receiver(
        &self,
        site: &crate::analyser::state::CmdCommandSite,
        hierarchy: Option<&super::super::class_hierarchy::ClassHierarchy>,
    ) -> Option<super::super::types::Diagnostic> {
        self.w308_for_enclosing_receiver(
            site.method_name.as_deref()?,
            site.method_span,
            site.cmd_span,
            tcl_registry::definer::MethodReach::ObjectCommand,
            hierarchy,
        )
    }

    /// W308 for a bareword **self-dispatch keyword** head — `my <method>`,
    /// the commonest same-object spelling in `TclOO`.
    ///
    /// The receiver is the same enclosing object `[self]` names, so the
    /// class lookup is shared with [`Self::w308_for_self_receiver`]; what
    /// differs is the reach. `my` bypasses export filtering, so it can call
    /// `oo::object`'s unexported members — which is why `my variable v` and
    /// `my varname v`, both idiomatic, must stay silent while `$obj
    /// variable v` does not.
    pub(super) fn w308_for_self_dispatch(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        hierarchy: Option<&super::super::class_hierarchy::ClassHierarchy>,
    ) -> Option<super::super::types::Diagnostic> {
        if self.self_dispatch_keyword_disturbed(&site.var_name) {
            return None;
        }
        self.w308_for_enclosing_receiver(
            site.method_name.as_deref()?,
            site.method_span,
            site.cmd_span,
            tcl_registry::definer::MethodReach::SelfDispatch,
            hierarchy,
        )
    }

    /// Whether this document renames, aliases, deletes or shadows the
    /// self-dispatch keyword `head` anywhere — in which case a bare `my`
    /// may not be `TclOO`'s dispatcher at all and every conclusion about
    /// the word after it is unfounded, so W308 abstains.
    ///
    /// `rename my mine`, `interp alias {} my {} …`, `rename my {}`, and a
    /// user `proc my {…}` all qualify. Asked here rather than in the walker
    /// because it is **order-independent by design**: a `rename` written
    /// *after* the method body still governs, since the body only runs
    /// later, and a `proc my` in a deferred fragment is invisible to the
    /// walk that records the site. Post-walk, every table is complete and
    /// the answer is the same on the whole-file and per-item paths.
    ///
    /// Deliberately coarse — "disturbed anywhere" rather than "disturbed
    /// before this offset". Modelling which redefinition wins at each
    /// offset would buy back warnings only in files that redefine a core
    /// `TclOO` keyword, and would risk a confident wrong answer in exactly
    /// the files least able to afford one.
    ///
    /// Both spellings are checked, since each table keys bare or
    /// `::`-qualified depending on how the disturbing command was written.
    pub(super) fn self_dispatch_keyword_disturbed(&self, head: &str) -> bool {
        let Ok(generation) = self.try_analysis_context() else {
            return true;
        };
        let Some(spec) = generation
            .context()
            .resolve_spec(generation.commands(), head)
        else {
            return true;
        };
        if !spec
            .traits
            .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
        {
            return true;
        }
        let Some(probe) = crate::source_command_observation::SelfDispatchSourceTableProbe::for_selected_descriptor(
            spec.descriptor_key(),
        ) else {
            return true;
        };
        probe.any_reported_key(|name| {
            self.command_aliases.contains_key(name)
                || self.renamed_commands.contains_key(name)
                || self.renamed_commands.values().any(|old| old == name)
                || self.deleted_commands.contains_key(name)
                || self.result.all_procs.contains_key(name)
                || self.result.all_classes.contains_key(name)
        })
    }

    /// The shared body of the two "the receiver is the object whose method
    /// body encloses this call" W308 checks — `[self] <method>` and bareword
    /// `my <method>`.
    ///
    /// Unlike the `Object`-return-type path, the class here is never
    /// *inferred* from a type lattice: the registry says the head denotes
    /// the current object outright, so the only question is which class's
    /// body the offset falls in.
    ///
    /// # What abstains, and why that is the right answer
    ///
    /// * **No enclosing class** — a top-level `my`, a `my` inside a plain
    ///   `proc`, or one inside an `oo::define` / `oo::objdefine` body whose
    ///   members are not folded into a recorded `ClassDef`. There is no
    ///   receiver to check against, and guessing at one would invent errors
    ///   in code that runs fine.
    /// * **A non-literal method word** (`my $action`, `my get$suffix`) —
    ///   the name dispatched is computed at run time, so no static method
    ///   set can contradict it ([`Self::method_word_is_literal`]).
    /// * **An unprovable method set** — a `mixin` or `superclass` outside
    ///   the local index, a class manufactured by an unreadable metaclass,
    ///   or a body that installs members reflectively. Handled inside
    ///   [`Self::validate_method_on_class`] via
    ///   [`Self::method_set_unknowable`], so a dynamically-extended class
    ///   never draws this warning.
    /// * **An `unknown` handler** anywhere in the MRO — every unresolved
    ///   name is then legal by construction.
    /// * **A template method, on `my` dispatch only** — the method resolves
    ///   nowhere on the enclosing class's MRO but a known subclass defines
    ///   it ([`ClassHierarchy::subclass_provides_method`](super::super::class_hierarchy::ClassHierarchy::subclass_provides_method)).
    ///   `my` late-binds on the actual receiver — always a
    ///   subclass instance when the base is abstract — and reaches
    ///   unexported members, so the call is the deliberate pattern, not a
    ///   typo.  `[self] M` keeps the warning: it dispatches through the
    ///   object's command, where an unexported subclass method really is
    ///   unreachable.
    ///
    /// A `forward`ed method needs no carve-out: `forward NAME …` records a
    /// real member under `NAME`, so `my NAME` resolves through the ordinary
    /// member tables. Nor do inherited methods, which the MRO lookup finds.
    ///
    /// `None` when the method resolves, when any of the above abstains, or
    /// when the diagnostic is disabled.
    fn w308_for_enclosing_receiver(
        &self,
        method: &str,
        method_span: Option<tcl_lexer::Span>,
        cmd_span: tcl_lexer::Span,
        reach: tcl_registry::definer::MethodReach,
        hierarchy: Option<&super::super::class_hierarchy::ClassHierarchy>,
    ) -> Option<super::super::types::Diagnostic> {
        if self.result.allows_retained_logical_declaration_advice() {
            return self.w308_for_logical_enclosing_receiver(
                method,
                method_span,
                cmd_span,
                reach,
                hierarchy,
            );
        }
        if self.disabled_diagnostics.contains("W308") {
            return None;
        }
        if !Self::method_word_is_literal(method) {
            return None;
        }
        if self.site_in_child_interp(cmd_span.start()) {
            return None;
        }
        let class_def = self.enclosing_class_at_offset(cmd_span.start())?;
        let cls_qn = class_def.qualified_name.as_str();
        if self.validate_method_on_class(cls_qn, method, Some(class_def), hierarchy, reach) {
            return None;
        }
        // Template-method pattern: a base-class body calling
        // `my M` where `M` is written only by subclasses runs fine — `my`
        // late-binds on the actual receiver, always a subclass instance,
        // and bypasses export filtering.  A known defining subclass is the
        // evidence; with none anywhere in the index the warning stands.
        // `SelfDispatch` only: `[self] M` goes through the object's own
        // command, where an unexported subclass method really is
        // unreachable (tclsh 9.0.4: `[self] varname v` fails where
        // `my varname v` succeeds — same reach split as the doc above).
        if reach == tcl_registry::definer::MethodReach::SelfDispatch
            && (hierarchy.is_some_and(|h| h.subclass_provides_method(cls_qn, method))
                || self.workspace_subclass_methods.as_ref().is_some_and(|m| {
                    m.get(cls_qn)
                        .is_some_and(|provided| provided.contains(method))
                }))
        {
            return None;
        }
        Some(self.w308_diagnostic(
            method,
            cls_qn,
            &[cls_qn],
            hierarchy,
            W308DiagnosticSite {
                reach,
                method_span,
                cmd_span,
            },
        ))
    }

    /// Conditional Logical source inventory, independently of Native dispatch.
    /// Only actual receiver-frame lineage chooses the class; source relation
    /// providers and complete retained ledgers own names and candidate data.
    fn w308_for_logical_enclosing_receiver(
        &self,
        method: &str,
        method_span: Option<tcl_lexer::Span>,
        cmd_span: tcl_lexer::Span,
        reach: tcl_registry::definer::MethodReach,
        hierarchy: Option<&super::super::class_hierarchy::ClassHierarchy>,
    ) -> Option<super::super::types::Diagnostic> {
        if self.disabled_diagnostics.contains("W308") || self.site_in_child_interp(cmd_span.start())
        {
            return None;
        }
        let input = self.result.resolved_input.as_ref()?;
        let image = tcl_lexer::SourceImage::document(&self.source);
        let caller = self.result.logical_source_member_frame_at(
            &image,
            input.lexer_config(),
            cmd_span.start(),
        )?;
        let class = self.result.logical_source_member_class(&caller)?;
        let (names, grammar) = self.logical_receiver_method_inventory(class, reach)?;
        let span = method_span?;
        let (word, _) = self
            .head_identities
            .original_written_word_at_span_in_source(&image, span, input.lexer_config())?;
        let value = tcl_syntax::word_rules::original_static_word_unicode_value(&word)?;
        if value != method.as_bytes()
            || names.contains(method)
            || grammar.dynamic_method_dispatch
            || grammar
                .unknown_dispatch_method
                .is_some_and(|unknown| names.contains(unknown))
        {
            return None;
        }
        // Existing positive template-method hints remain conservative vetoes;
        // they supply no missing-member or suggestion authority.
        if reach == tcl_registry::definer::MethodReach::SelfDispatch
            && (hierarchy
                .is_some_and(|h| h.subclass_provides_method(&class.qualified_name, method))
                || self.workspace_subclass_methods.as_ref().is_some_and(|all| {
                    all.get(&class.qualified_name)
                        .is_some_and(|names| names.contains(method))
                }))
        {
            return None;
        }
        let best = crate::text::suggest_similar(
            method,
            names.iter().map(String::as_str),
            1,
            crate::text::scaled_max_distance(method),
        );
        let mut message = format!(
            "Unknown method '{method}' on class '{}'",
            class.qualified_name
        );
        let mut fixes = Vec::new();
        if let Some(best) = best.first() {
            use std::fmt::Write as _;
            let _ = write!(message, "; did you mean '{best}'?");
            if let Some((span, new_text)) = self.w308_configured_method_proposal(
                W308DiagnosticSite {
                    reach,
                    method_span,
                    cmd_span,
                },
                best,
            ) {
                fixes.push(super::super::types::CodeFix {
                    span,
                    new_text,
                    description: format!("Replace with '{best}'"),
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                });
            }
        }
        Some(
            super::super::types::Diagnostic::new(DiagCode::W308, span, message, Severity::Warning)
                .with_fixes(fixes),
        )
    }

    pub(super) fn logical_receiver_method_inventory<'a>(
        &'a self,
        root: &'a super::super::types::ClassDef,
        reach: tcl_registry::definer::MethodReach,
    ) -> Option<(
        std::collections::BTreeSet<String>,
        &'static tcl_registry::definer::DefinitionBodyGrammar,
    )> {
        let mut names = std::collections::BTreeSet::new();
        let mut pending = vec![(root, false)];
        let mut active = Vec::new();
        let mut complete = Vec::new();
        let mut root_grammar = None;
        while let Some((class, exiting)) = pending.pop() {
            let identity = std::ptr::from_ref(class);
            if exiting {
                active.retain(|current| *current != identity);
                complete.push(identity);
                continue;
            }
            if complete.contains(&identity) {
                continue;
            }
            if active.contains(&identity) {
                return None;
            }
            active.push(identity);
            pending.push((class, true));
            let grammar = self.logical_receiver_class_method_names(class, reach, &mut names)?;
            if std::ptr::eq(class, root) {
                root_grammar = Some(grammar);
            }
            pending.extend(
                self.logical_receiver_relation_providers(class)?
                    .into_iter()
                    .map(|provider| (provider, false)),
            );
        }
        Some((names, root_grammar?))
    }

    fn logical_receiver_class_method_names(
        &self,
        class: &super::super::types::ClassDef,
        reach: tcl_registry::definer::MethodReach,
        names: &mut std::collections::BTreeSet<String>,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        use super::super::super::types::MemberSide;
        let input = self.result.resolved_input.as_ref()?;
        let context = input.context_registry();
        let owner = self.result.logical_source_class_declaration(class)?;
        if class.member_set_incomplete
            || class.inheritance_unknown
            || !class.properties.is_empty()
            || !class.original_members.logical_relation_coverage_available()
        {
            return None;
        }
        let name_word = owner.original_name_word()?;
        let (_, factory_offset) = self
            .head_identities
            .original_written_word_at_span_in_source(
                name_word.image(),
                name_word.span(),
                input.lexer_config(),
            )?;
        let factory = crate::registry_invocation::source_structure::source_class_declaration_at(
            &self.source,
            &self.result,
            factory_offset,
        )?;
        if !factory
            .logical_source_class(&self.result)
            .is_some_and(|actual| std::ptr::eq(actual, class))
        {
            return None;
        }
        let grammar = factory.grammar(&context)?;
        if grammar.dynamic_method_dispatch {
            return None;
        }
        for builtin in grammar.builtin_object_methods {
            if grammar.builtin_object_method(builtin.name, reach).is_some() {
                names.insert(builtin.name.to_owned());
            }
        }
        for (side, methods) in [
            (MemberSide::Instance, &class.methods),
            (MemberSide::ClassObject, &class.class_methods),
        ] {
            class.original_members.methods(side)?;
            for method in methods.values() {
                let declaration = owner.member(side, method)?;
                let word = declaration.original_name_word()?;
                let name = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
                names.insert(String::from_utf8(name).ok()?);
            }
        }
        Some(grammar)
    }

    fn logical_receiver_relation_providers<'a>(
        &'a self,
        class: &'a super::super::types::ClassDef,
    ) -> Option<Vec<&'a super::super::types::ClassDef>> {
        use crate::command_binding::LogicalSourceClassRelationTarget as Target;
        // Original slot operations own final relation values; neither a
        // report superclass label nor registry name can select a provider.
        let mut relations: Vec<(
            tcl_registry::definer::RelationSlot,
            Vec<&super::super::types::ClassDef>,
        )> = Vec::new();
        for statement in self
            .result
            .logical_source_class_relation_candidates(class)?
        {
            let mut providers = Vec::new();
            for operand in statement.operands() {
                if operand.has_earlier_unknown_mutations() {
                    return None;
                }
                match operand.target(&self.result, class)? {
                    Target::Source(provider) => {
                        let actual = self
                            .result
                            .all_classes
                            .values()
                            .find(|actual| std::ptr::eq(*actual, provider))?;
                        providers.push(actual);
                    }
                    // This typed provider has no complete public method
                    // inventory here; its absence is never fabricated.
                    Target::Registry | Target::External(_) => return None,
                }
            }
            let index = if let Some(index) = relations
                .iter()
                .position(|(kind, _)| *kind == statement.relation())
            {
                index
            } else {
                relations.push((statement.relation(), Vec::new()));
                relations.len() - 1
            };
            statement.apply_values(&mut relations[index].1, &providers);
        }
        Some(
            relations
                .into_iter()
                .flat_map(|(_, selected)| selected)
                .collect(),
        )
    }
}
