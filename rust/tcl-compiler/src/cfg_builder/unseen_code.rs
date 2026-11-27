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

//! Whether a call, or a command its substitutions run, reaches code the module cannot see.

use super::{CODE_RUNNING_TRAITS, CfgBuilder, statement_has_literal_head};
use crate::command_binding::ModuleCommandBindings;
use crate::ir::Statement;
use tcl_lexer::Span;
use tcl_registry::Traits;

impl CfgBuilder<'_> {
    /// The command bindings in force where the call at `span` dispatches: the
    /// source-order state the timeline recorded before it, or the module's
    /// closed state where the timeline holds none.
    fn bindings_before_call(&self, span: Span) -> &ModuleCommandBindings {
        self.source_binding_timeline
            .as_ref()
            .and_then(|timeline| timeline.before_direct_call(span))
            .unwrap_or(&self.command_bindings)
    }

    /// Whether a command a `[…]` substitution of `stmt` runs is one the module
    /// cannot see ([`Self::command_reaches_unseen`]). The substitutions run in
    /// Tcl's evaluation order before the statement does, and each is asked
    /// under the command bindings the ones before it leave: a `[rename …]`
    /// earlier in the statement decides what a later head reaches.
    pub(super) fn embedded_reaches_unseen(&self, stmt: &Statement) -> bool {
        if !self.command_bindings.holds_a_command_table() {
            return false;
        }
        let bindings = self
            .source_binding_timeline
            .as_ref()
            .and_then(|timeline| timeline.before_substitutions(stmt.span()))
            .cloned()
            .unwrap_or_else(|| self.command_bindings.clone());
        let state = std::cell::RefCell::new(bindings);
        let unseen = std::cell::Cell::new(false);
        let resolve = |head: &str| {
            state
                .borrow()
                .resolved_embedded_head(head, &self.invocation_namespace)
        };
        let observe = |words: &[crate::ir_helpers::CommandWord], conditional: bool| {
            self.observe_in_source_order(&state, &unseen, words, conditional);
        };
        let _ = crate::ir_helpers::evaluated_command_substitutions_with_replay(
            stmt,
            self.registry,
            Some(&resolve),
            Some(&observe),
        );
        unseen.get()
    }

    /// One step of a source-order replay over the commands a statement's
    /// substitutions run: whether `words` reaches code the module cannot see
    /// under the bindings in `state`, then the binding transition the command
    /// makes, which the next command is resolved after.
    pub(super) fn observe_in_source_order(
        &self,
        state: &std::cell::RefCell<ModuleCommandBindings>,
        unseen: &std::cell::Cell<bool>,
        words: &[crate::ir_helpers::CommandWord],
        conditional: bool,
    ) {
        if !unseen.get()
            && self.command_bindings.holds_a_command_table()
            && self.command_reaches_unseen(&state.borrow(), words, 0)
        {
            unseen.set(true);
        }
        state.borrow_mut().advance_source_order_for_command(
            words,
            conditional,
            self.registry,
            &self.invocation_namespace,
        );
    }

    /// Whether the call statement `stmt` itself runs code the module cannot
    /// see: its head is computed, or is a literal the module cannot name
    /// ([`Self::head_is_unseen`]) under the command bindings in force where it
    /// dispatches, or it sources a file ([`Traits::SOURCES_FILE`]).
    ///
    /// The code such a call reaches can write, unset or read any name of the
    /// frame it runs from with nothing in this function's text to show it: a
    /// plain name at the top level is the global `::name`, and a procedure's
    /// local is in the reach of a callee that runs `upvar 1` or `uplevel 1`.
    /// An autoloaded or unknown-handled callee can do either on every release,
    /// so the CFG marks the call ([`Statement::unseen_call_marker`]) in a procedure
    /// body as at the top level. A head the module can name brings its frame
    /// effect from the registry or from the procedure summary instead. A
    /// builder that holds no module's command table names no head, so it marks
    /// nothing.
    pub(super) fn call_is_unseen(&self, stmt: &Statement) -> bool {
        let Statement::Call {
            command,
            canonical_command,
            args,
            tokens,
            ..
        } = stmt
        else {
            return false;
        };
        if !self.command_bindings.holds_a_command_table()
            || tokens
                .as_ref()
                .is_some_and(|tokens| tokens.synthetic.is_some())
        {
            return false;
        }
        if !statement_has_literal_head(stmt) {
            return true;
        }
        let spellings: Vec<&str> = args.iter().map(String::as_str).collect();
        let lookup = canonical_command.as_deref().unwrap_or(command.as_str());
        self.sources_a_file(lookup, &spellings)
            || self.head_is_unseen(self.bindings_before_call(stmt.span()), command, &spellings)
    }

    /// Whether the command `head args…` sources a file, which runs in the
    /// frame of the call ([`Traits::SOURCES_FILE`]).
    fn sources_a_file(&self, head: &str, args: &[&str]) -> bool {
        self.registry
            .invocation_traits(head, args, self.registry.own_surface_query())
            .contains(Traits::SOURCES_FILE)
    }

    /// Whether the command `words`, or code it runs, runs code the module cannot
    /// see under `bindings`: a computed head, a literal one that sources a file
    /// or that the module cannot name ([`Self::head_is_unseen`]). Every word the
    /// command runs as code is read, whichever frame it runs in: a body of this
    /// frame (the script of a `catch`), a body that runs in another (`uplevel`,
    /// `namespace eval`), a lambda's body (`apply`), the commands in the text a
    /// `subst` substitutes and those in an expression word; a computed one is
    /// code the module cannot see. Whether such code runs is all this answers:
    /// the names the commands of a substitution write are not stated.
    fn command_reaches_unseen(
        &self,
        bindings: &ModuleCommandBindings,
        words: &[crate::ir_helpers::CommandWord],
        depth: u32,
    ) -> bool {
        let Some(head) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            return true;
        };
        let spellings: Vec<&str> = words
            .iter()
            .skip(1)
            .map(|word| word.literal().unwrap_or_default())
            .collect();
        if self.sources_a_file(head, &spellings) || self.head_is_unseen(bindings, head, &spellings)
        {
            return true;
        }
        // Past the cap the text is not read, so it is taken to run anything.
        if tcl_source::limits::MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            return true;
        }
        let reaches = |text: &str| {
            crate::ir_helpers::tokenise_command_words(text, self.config)
                .iter()
                .any(|inner| self.command_reaches_unseen(bindings, inner, depth + 1))
        };
        // A script's own substitutions are not among the commands recovered
        // from the statement, whose words are the only ones scanned for them.
        if depth > 0
            && words.iter().filter(|word| word.substituted).any(|word| {
                crate::var_refs::command_subst_texts_with_config(&word.raw, self.config)
                    .iter()
                    .any(|inner| reaches(inner))
            })
        {
            return true;
        }
        let runs = |index: usize, text_runs: &dyn Fn(&str) -> bool| match words.get(index + 1) {
            Some(word) if !word.substituted => text_runs(&word.text),
            Some(_) => true,
            None => false,
        };
        let role_indices = |role| self.registry.arg_indices_for_role(head, &spellings, role);
        if role_indices(tcl_registry::ArgRole::Body)
            .into_iter()
            .any(|index| runs(index, &reaches))
        {
            return true;
        }
        // A lambda literal is a list of its parameters, its body and a namespace.
        let lambda_runs =
            |text: &str| match tcl_syntax::list::split_list_in(text, self.config.escapes) {
                Ok(elements) => elements.get(1).is_some_and(|body| reaches(body)),
                Err(_) => true,
            };
        if role_indices(tcl_registry::ArgRole::LambdaLiteral)
            .into_iter()
            .any(|index| runs(index, &lambda_runs))
        {
            return true;
        }
        let substitutes = self
            .registry
            .substitutions_performed(head, &spellings)
            .is_some_and(|kinds| kinds.commands);
        let concatenates = self
            .registry
            .get(head)
            .is_some_and(|spec| spec.traits.contains(Traits::EXPR_CONCATENATES_ARGS));
        let expressions = role_indices(tcl_registry::ArgRole::Expr);
        (0..spellings.len())
            .filter(|index| substitutes || concatenates || expressions.contains(index))
            .any(|index| match words.get(index + 1) {
                Some(word) if !word.substituted => {
                    crate::var_refs::command_subst_texts_with_config(&word.text, self.config)
                        .iter()
                        .any(|inner| reaches(inner))
                }
                _ => false,
            })
    }

    /// Whether the literal head `head`, called with `args` in this function's
    /// namespace under `bindings`, may reach code the module cannot name: a
    /// spelling neither the registry ships for the dialect nor the module binds,
    /// which Tcl dispatches to the unresolved-command handler
    /// ([`ModuleCommandBindings::may_dispatch_unresolved`]); a binding the
    /// source-order timeline cannot name — one a `rename` or an alias of a
    /// computed name may have replaced, or any spelling once a `rename` or an
    /// alias moved a name the timeline cannot name; and a registry command the
    /// head reaches through another binding — an alias, a rename, the
    /// unresolved-command handler — that declares it runs code
    /// ([`CODE_RUNNING_TRAITS`]), which the graph never lowers. A head whose
    /// namespace the builder does not know is taken to.
    pub(super) fn head_is_unseen(
        &self,
        bindings: &ModuleCommandBindings,
        head: &str,
        args: &[&str],
    ) -> bool {
        // The same identity-aware selector the frame projection reads
        // ([`Self::variable_write_projection`]): a source-context frame (the
        // fresh-interpreter entry's authored `::`) names its heads as an
        // exact one does; only a receiver-selected relative head is unnamed.
        let Some(namespace) = self.invocation_namespace.for_head_context(head) else {
            return true;
        };
        let namespace = namespace.as_ref();
        // A document-declared command is one the module names: its stated
        // frame effect, not the unknown command's, is what the call brings.
        if bindings.names_only_a_declared_command(head, namespace)
            || bindings.absent_head_runs_nothing(head, namespace)
        {
            return false;
        }
        if bindings.has_unnameable_rebinding_subject()
            || bindings.may_dispatch_unresolved(head, namespace)
            || bindings.target_resolution_may_be_unknown(head, namespace)
        {
            return true;
        }
        let selected = self.registry.get(head);
        bindings.targets(head, namespace).iter().any(|target| {
            if !target.registry_backed
                || (target.prepended.is_empty()
                    && selected.is_some_and(|spec| {
                        tcl_registry::command_descriptor_labels_match(
                            &target.command,
                            spec.descriptor_key().label(),
                        )
                    }))
            {
                return false;
            }
            let Some(mut words) = target
                .prepended
                .iter()
                .map(|word| match word {
                    crate::registry_invocation::EffectiveInvocationWord::Literal(value) => {
                        Some(value.as_str())
                    }
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
            else {
                return true;
            };
            words.extend(args.iter().copied());
            self.registry
                .invocation_traits(&target.command, &words, self.registry.own_surface_query())
                .intersects(CODE_RUNNING_TRAITS)
        })
    }
}
