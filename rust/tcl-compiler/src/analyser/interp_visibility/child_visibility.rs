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

//! The analyser side of child visibility: the declaration an `interp` command selects, its hide and expose deltas and the observation of each invocation.

use super::super::Analyser;
use super::{
    ConditionalInterpreterVisibilitySubject, InterpreterVisibilityObligation, SourceCommandOrigin,
    SourceInterpreterVisibilitySnapshot, SourceVisibleSlot,
};
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage, Span, Token};
use tcl_registry::{
    ChildInterpreterSafety, InterpreterTransition, StateTransition, TransitionSubject,
};
use tcl_syntax::naming::NativeNameProtocol;

pub(super) struct OriginalInterpreterInvocation {
    pub(super) native: Vec<NativeWord>,
    pub(super) words: crate::registry_invocation::source_structure::OriginalRegistryWords,
    pub(super) transitions: tcl_registry::StateTransitions,
}

impl OriginalInterpreterInvocation {
    pub(super) fn input(&self, subject: &TransitionSubject) -> Option<SignatureSourceNameInput> {
        self.words
            .operands()
            .get(subject.argument_index()?)?
            .as_ref()?
            .input()
            .cloned()
    }
}

impl Analyser {
    /// Namespace source geometry belongs to the original child ledger's
    /// selected Namespace Ensure schema. It supplies no entered namespace.
    pub(in super::super) fn original_interp_namespace_source_scope(
        &self,
        offset: u32,
        arguments: &[Token],
        scope_path: &[usize],
    ) -> Option<SignatureNamespaceScope> {
        let snapshot = self.safe_interp_stack.last()?;
        let input = self.resolved_analysis_input();
        snapshot
            .matches(&SourceImage::document(&self.source), &input)
            .then_some(())?;
        let retained = self.retained_invocation_tokens(offset, arguments);
        let tokens = retained?;
        let native = crate::registry_invocation::original_native_compiler_words(
            snapshot.image(),
            tokens.words(),
            offset,
            input.lexer_config(),
        )?;
        let namespace = self.declaration_namespace_scope(scope_path)?;
        let schema = snapshot.possible_source_schema(&native, &namespace)?;
        if schema.hook != Some(tcl_registry::hooks::AnalyserHookId::NamespaceEval) {
            return None;
        }
        let mut ensure = schema.transitions.facts().iter().filter_map(|fact| {
            let StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
                namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
            }) = &fact.transition
            else {
                return None;
            };
            snapshot.mutation_operand(&native, subject)
        });
        let name = ensure.next()?;
        if ensure.any(|other| other != name) {
            return None;
        }
        namespace.child_from_input(&name)
    }

    pub(in super::super) fn original_interp_procedure_source_name(
        &self,
        name: Token,
        scope_path: &[usize],
    ) -> Option<crate::signature_scan::scope::SignatureSourceCommand> {
        let snapshot = self.safe_interp_stack.last()?;
        let input = self.resolved_analysis_input();
        snapshot
            .matches(&SourceImage::document(&self.source), &input)
            .then_some(())?;
        let namespace = self.declaration_namespace_scope(scope_path)?;
        let span = tcl_lexer::word_span_at(&*self.source, name.span);
        let mut declarations = snapshot.slots.values().filter_map(|slot| {
            let SourceVisibleSlot::Visible(SourceCommandOrigin::Declaration {
                name,
                kind,
                operands,
            }) = slot
            else {
                return None;
            };
            if *kind != tcl_registry::CommandBindingDefinitionKind::Procedure
                || operands.is_empty()
                || operands.iter().any(|word| {
                    word.image() != snapshot.image() || word.config() != input.lexer_config()
                })
            {
                return None;
            }
            let key = name.original_word_key()?;
            (key.original_word().span() == span).then_some(key)
        });
        let key = declarations.next()?;
        if declarations.any(|other| other != key) {
            return None;
        }
        crate::signature_scan::scope::SignatureSourceCommand::procedure_from_key(&namespace, key)
    }

    /// A typed source path selects its original Create vector independently of
    /// compatibility parent-command reporting keys. No runtime child is issued.
    pub(super) fn original_interp_state_key(
        &self,
        invocation: &OriginalInterpreterInvocation,
        path: &SignatureSourceNameInput,
    ) -> Option<String> {
        if path.bytes().is_empty() {
            return self.interp_path_stack.last().map(|frame| frame.key.clone());
        }
        let input = self.resolved_analysis_input();
        let binding = self
            .head_identities
            .original_source_interpreter_path_binding(&input, &invocation.native, path)?;
        let image = SourceImage::document(&self.source);
        let mut matches = self.interpreters.iter().filter_map(|(key, state)| {
            let snapshot = state.visibility.as_ref()?;
            (!state.tainted
                && snapshot.matches(&image, &input)
                && snapshot.declaration == binding.creation().original_words())
            .then_some(key)
        });
        let key = matches.next()?;
        matches.next().is_none().then(|| key.clone())
    }

    pub(in super::super) fn original_interp_eval_state_key(
        &self,
        head: Token,
        arguments: &[Token],
    ) -> Option<String> {
        let invocation = self.original_interpreter_invocation(
            head,
            arguments,
            tcl_registry::hooks::AnalyserHookId::InterpEval,
        )?;
        let path = invocation.words.operands().get(1)?.as_ref()?.input()?;
        self.original_interp_state_key(&invocation, path)
    }

    pub(in super::super) fn handle_interp_delete_command_original(
        &mut self,
        args: &[String],
        head: Token,
        arguments: &[Token],
    ) {
        let invocation = self.original_interpreter_invocation(
            head,
            arguments,
            tcl_registry::hooks::AnalyserHookId::InterpDelete,
        );
        let keys = invocation.as_ref().map_or_default(|invocation| {
            invocation
                    .transitions
                    .facts()
                    .iter()
                    .filter_map(|fact| {
                        let StateTransition::Interpreter(InterpreterTransition::Delete {
                            interpreter,
                        }) = &fact.transition
                        else {
                            return None;
                        };
                        self.original_interp_state_key(invocation, &invocation.input(interpreter)?)
                    })
                    .collect::<Vec<_>>()
        });
        for key in keys {
            if self.interpreters.remove(&key).is_some() {
                *self.interp_epochs.entry(key).or_insert(0) += 1;
            }
        }
        self.handle_interp_delete_command(args);
    }

    pub(super) fn original_interpreter_invocation(
        &self,
        head: Token,
        arguments: &[Token],
        hook: tcl_registry::hooks::AnalyserHookId,
    ) -> Option<OriginalInterpreterInvocation> {
        let mut argv = vec![head];
        argv.extend_from_slice(arguments);
        let native = self.original_visibility_words(&argv)?;
        let end = native.last()?.span().end();
        let text = self.source.get(head.span.start() as usize..end as usize)?;
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            text,
            head.span.start(),
            self.file_lexer_config().nested(),
        )
        .into_iter()
        .next()?;
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &segment,
        )?;
        let context = self.resolved_analysis_input().context_registry();
        let transitions = words.with_source_schema(&context, |schema| {
            (schema.semantics.analyser_hook == Some(hook)).then(|| schema.state_transitions())
        })??;
        Some(OriginalInterpreterInvocation {
            native,
            words,
            transitions,
        })
    }

    pub(super) fn original_visibility_words(&self, argv: &[Token]) -> Option<Vec<NativeWord>> {
        let input = self.resolved_analysis_input();
        let image = SourceImage::document(&self.source);
        if !self.head_identities.matches_resolved_analysis_input(&input)
            || !self
                .head_identities
                .matches_original_source_image(&image, self.file_lexer_config())
        {
            return None;
        }
        // Prefer the complete original child arena when it owns this exact
        // vector. Flattened invocation tokens cannot supply expansion markers.
        if let Some(original) = self.original_child_body_words(argv) {
            return Some(original);
        }
        let tokens = self.retained_invocation_tokens(argv.first()?.span.start(), argv);
        let Some(tokens) = tokens else {
            return self.original_child_body_words(argv);
        };
        if tokens.argv.len() != argv.len()
            || tokens
                .argv
                .iter()
                .zip(argv)
                .any(|(span, token)| *span != token.span)
        {
            return None;
        }
        crate::registry_invocation::original_native_compiler_words(
            &image,
            tokens.words(),
            argv.first()?.span.start(),
            self.file_lexer_config(),
        )
        .or_else(|| self.original_child_body_words(argv))
    }

    /// Readonly lexical vectors of the privately retained original child root
    /// body. Only actual root commands and executable brackets are traversed;
    /// inert braced values do not become source scripts.
    fn original_child_body_words(&self, argv: &[Token]) -> Option<Vec<NativeWord>> {
        let snapshot = self.safe_interp_stack.last()?;
        let input = self.resolved_analysis_input();
        snapshot
            .matches(&SourceImage::document(&self.source), &input)
            .then_some(())?;
        let body = snapshot.body.as_ref()?;
        let mut pending = vec![body.content_span().ok()?];
        let mut visited = std::collections::HashSet::new();
        while let Some(span) = pending.pop() {
            if !visited.insert(span) {
                continue;
            }
            let plan = tcl_lexer::native_script_words_in(
                snapshot.image().clone(),
                span,
                input.lexer_config(),
            )
            .ok()?;
            for command in plan.commands {
                if command.words.len() == argv.len()
                    && command
                        .words
                        .iter()
                        .zip(argv)
                        .all(|(word, token)| word.group().span == token.span)
                {
                    return Some(command.words);
                }
                for word in command.words {
                    for part in word.executable_parts().all_parts() {
                        if let tcl_lexer::ExecutablePart::Command { body } = part.part {
                            pending.push(body);
                        }
                    }
                }
            }
        }
        None
    }

    pub(in super::super) fn retain_interp_visibility_declaration(
        &mut self,
        path: &str,
        head: Token,
        arguments: &[Token],
    ) {
        let captured = (|| {
            let invocation = self.original_interpreter_invocation(
                head,
                arguments,
                tcl_registry::hooks::AnalyserHookId::InterpCreate,
            )?;
            let policy = self.declaration_name_policy()?;
            if !matches!(policy.recipe(), NativeNameProtocol::C(_)) {
                return None;
            }
            for fact in invocation.transitions.facts() {
                if let StateTransition::Interpreter(InterpreterTransition::Create {
                    interpreter: Some(interpreter),
                    safety,
                }) = &fact.transition
                {
                    let input = invocation.input(interpreter)?;
                    if input.bytes() != path.as_bytes() {
                        return None;
                    }
                    return SourceInterpreterVisibilitySnapshot::initialize(
                        self.resolved_analysis_input(),
                        policy,
                        invocation.native.clone(),
                        match safety {
                            ChildInterpreterSafety::Safe => true,
                            ChildInterpreterSafety::Inherited => self
                                .safe_interp_stack
                                .last()
                                .is_some_and(|parent| parent.safe),
                            ChildInterpreterSafety::Unknown => return None,
                        },
                    );
                }
            }
            None
        })();
        let key = self.qualified_interp_key(path);
        if let Some(state) = self.interpreters.get_mut(&key) {
            state.visibility = captured;
        }
    }

    pub(in super::super) fn original_interp_visibility_body(
        &self,
        key: &str,
        body: Token,
        head: Token,
        arguments: &[Token],
    ) -> Option<SourceInterpreterVisibilitySnapshot> {
        let state = self.interpreters.get(key)?;
        if state.tainted {
            return None;
        }
        let mut snapshot = state.visibility.clone()?;
        let image = SourceImage::document(&self.source);
        if !snapshot.matches(&image, &self.resolved_analysis_input()) {
            return None;
        }
        let invocation = self.original_interpreter_invocation(
            head,
            arguments,
            tcl_registry::hooks::AnalyserHookId::InterpEval,
        )?;
        let path = invocation.words.operands().get(1)?.as_ref()?.input()?;
        if self.original_interp_state_key(&invocation, path).as_deref() != Some(key) {
            return None;
        }
        let context = self.resolved_analysis_input().context_registry();
        let requested = tcl_lexer::word_span(
            &Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            ),
            body,
        );
        for region in invocation.words.source_script_bodies_for(
            &context,
            crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
        ) {
            if region.original_container().span() != requested
                || !region.matches_source(&image, self.file_lexer_config())
                || !region.matches_context(&context)
            {
                continue;
            }
            snapshot.body = Some(region.original_container().clone());
            snapshot.body_invocation = invocation.native;
            return Some(snapshot);
        }
        None
    }

    pub(in super::super) fn original_interp_handle_visibility_body(
        &self,
        body: Token,
        head: Token,
        arguments: &[Token],
    ) -> Option<(String, SourceInterpreterVisibilitySnapshot)> {
        let input = self.resolved_analysis_input();
        let image = SourceImage::document(&self.source);
        let mut argv = vec![head];
        argv.extend_from_slice(arguments);
        let original = self.original_visibility_words(&argv)?;
        let receipt = self
            .head_identities
            .original_source_interpreter_handle_body(&input, &original)?;
        let key = self
            .qualified_interp_key(std::str::from_utf8(receipt.interpreter_input().bytes()).ok()?);
        // The retained Create vector identifies the child independently of
        // its moved parent command's compatibility reporting key.
        let mut candidates = self.interpreters.values().filter_map(|state| {
            let snapshot = state.visibility.as_ref()?;
            (!state.tainted
                && snapshot.matches(&image, &input)
                && receipt.creation().original_words() == snapshot.declaration)
                .then_some(snapshot)
        });
        let mut snapshot = candidates.next()?.clone();
        if candidates.next().is_some() {
            return None;
        }
        if receipt.creation().original_words() != snapshot.declaration
            || receipt.body_word().span() != tcl_lexer::word_span_at(&*self.source, body.span)
        {
            return None;
        }
        snapshot.body = Some(receipt.body_word().clone());
        snapshot.body_invocation = original;
        Some((key, snapshot))
    }

    pub(in super::super) fn apply_original_interp_visibility_delta(
        &mut self,
        path: &str,
        head: Token,
        arguments: &[Token],
        hide: bool,
    ) {
        if crate::naming::is_dynamic_word(path) {
            self.dynamic_interp_ops = true;
            for state in self.interpreters.values_mut() {
                state.visibility = None;
                state.tainted = true;
            }
            return;
        }
        let hook = if hide {
            tcl_registry::hooks::AnalyserHookId::InterpHide
        } else {
            tcl_registry::hooks::AnalyserHookId::InterpExpose
        };
        let delta = self
            .original_interpreter_invocation(head, arguments, hook)
            .and_then(|invocation| {
                for fact in invocation.transitions.facts() {
                    if let StateTransition::Interpreter(
                        InterpreterTransition::Hide {
                            interpreter,
                            visible,
                            hidden,
                        }
                        | InterpreterTransition::Expose {
                            interpreter,
                            visible,
                            hidden,
                        },
                    ) = &fact.transition
                    {
                        if invocation.input(interpreter)?.bytes() != path.as_bytes() {
                            return None;
                        }
                        let key = self.original_interp_state_key(
                            &invocation,
                            &invocation.input(interpreter)?,
                        )?;
                        return Some((
                            key,
                            invocation.input(visible)?,
                            invocation.input(hidden)?,
                            invocation.native.clone(),
                        ));
                    }
                }
                None
            });
        let Some((key, visible, hidden, operands)) = delta else {
            return;
        };
        let Some(state) = self.interpreters.get_mut(&key) else {
            return;
        };
        let applied = (|| {
            let snapshot = state.visibility.as_mut()?;
            if hide {
                snapshot.hide(&visible, &hidden, &operands)
            } else {
                snapshot.expose(&hidden, &visible, &operands)
            }
        })();
        if applied.is_none() {
            state.tainted = true;
            state.visibility = None;
        }
    }

    pub(super) fn original_interp_declared_receiver_body(
        &self,
        native: &[NativeWord],
        scope_path: &[usize],
    ) -> Option<Arc<crate::command_binding::SourceDeclaredReceiverBodyEntry>> {
        let scope = super::super::scope::scope_at(&self.result.global_scope, scope_path)?;
        if scope.kind != super::super::types::ScopeKind::Method || scope.naming_scope.is_some() {
            return None;
        }
        let input = self.resolved_analysis_input();
        let image = SourceImage::document(&self.source);
        self.head_identities
            .matches_resolved_analysis_input(&input)
            .then_some(())?;
        let bindings = self.head_identities.source_bindings_ref();
        let origin = bindings.source_origin()?;
        if origin.source_image() != &image {
            return None;
        }
        let body = bindings.declared_receiver_body_entry_at(origin, native.first()?.span().start());
        let body = body?;
        let source = body.source();
        let crate::command_binding::ExecutedScriptMapping::Contiguous { base } = source.mapping
        else {
            return None;
        };
        let end = base.checked_add(u32::try_from(source.text.bytes().len()).ok()?)?;
        (source.origin == *origin
            && native.iter().all(|word| {
                word.image() == &image
                    && word.config() == input.lexer_config()
                    && word.span().start() >= base
                    && word.span().end() <= end
            }))
        .then_some(body)
    }

    pub(super) fn original_interp_source_receiver_declaration(
        &self,
        native: &[NativeWord],
        scope_path: &[usize],
    ) -> Option<Arc<super::super::types::OriginalSourceReceiverBodyDeclaration>> {
        let scope = super::super::scope::scope_at(&self.result.global_scope, scope_path)?;
        if scope.kind != super::super::types::ScopeKind::Method || scope.naming_scope.is_some() {
            return None;
        }
        let body = scope.original_receiver_body_declaration.as_ref()?;
        body.owns_invocation(native, &self.resolved_analysis_input())
            .then(|| Arc::clone(body))
    }

    /// Conditional source advice never skips an invocation or removes edges.
    pub(in super::super) fn observe_interp_visibility(
        &mut self,
        argv: &[Token],
        scope_path: &[usize],
    ) {
        self.retain_original_interp_source_loads(argv, scope_path);
        let observation = (|| {
            let snapshot = self.safe_interp_stack.last()?;
            if !snapshot.matches(
                &SourceImage::document(&self.source),
                &self.resolved_analysis_input(),
            ) {
                return None;
            }
            let native = self.original_visibility_words(argv);
            let native = native?;
            let name = snapshot.original_input(native.first()?)?;
            if let Some(namespace) = self.declaration_namespace_scope(scope_path) {
                return snapshot.observe(name, namespace, native);
            }
            let declared = self.original_interp_declared_receiver_body(&native, scope_path);
            let source_declaration =
                self.original_interp_source_receiver_declaration(&native, scope_path);
            if declared.is_none() && source_declaration.is_none() {
                return None;
            }
            // This is an explicit possible root-fallback hypothesis for the
            // actual declared body, never a root namespace stamped on a receiver.
            let mut subject = snapshot.observe(
                name,
                SignatureNamespaceScope::C(tcl_core_types::ByteNamespacePath::root()),
                native,
            )?;
            subject.namespace = None;
            subject.declared_receiver = declared;
            subject.source_receiver_declaration = source_declaration;
            subject.obligations.extend([
                InterpreterVisibilityObligation::UnknownExecutingReceiverNamespace,
                InterpreterVisibilityObligation::NoEarlierReceiverNamespaceBinding,
            ]);
            Some(subject)
        })();
        if let Some(subject) = observation {
            self.emit_interp_visibility(subject, argv[0].span);
        }
        let mutation = (|| {
            let snapshot = self.safe_interp_stack.last()?;
            let native = self.original_visibility_words(argv)?;
            let namespace = self.declaration_namespace_scope(scope_path)?;
            let schema = snapshot.possible_source_schema(&native, &namespace)?;
            let mut updated = snapshot.clone();
            let closed = updated
                .apply_command_transitions(&native, &namespace, &schema.transitions)
                .is_some();
            Some((updated, closed))
        })();
        if let Some((updated, closed)) = mutation {
            if closed {
                if let Some(current) = self.safe_interp_stack.last_mut() {
                    *current = updated.clone();
                }
            } else if let Some(current) = self.safe_interp_stack.last_mut() {
                current.slots.clear();
                current.hidden.clear();
                current.unavailable.clear();
            }
            if let Some(frame) = self.interp_path_stack.last()
                && let Some(state) = self.interpreters.get_mut(&frame.key)
            {
                state.visibility = closed.then_some(updated);
                state.tainted |= !closed;
            }
        }
    }

    pub(super) fn emit_interp_visibility(
        &mut self,
        subject: ConditionalInterpreterVisibilitySubject,
        span: Span,
    ) {
        if self.structure_only {
            return;
        }
        let label = tcl_syntax::native_string::resident_name_label(subject.name_input().bytes());
        self.result.diagnostics.push(super::super::Diagnostic::new(tcl_core_types::DiagCode::W129,span,format!("'{label}' may be unavailable in this child interpreter under the retained source visibility assumptions"),super::super::types::Severity::Warning).with_subject(super::super::DiagnosticSubject::ConditionalInterpreterVisibility(Arc::new(subject))));
    }

    pub(in super::super) fn safe_interp_ctx_snapshot(
        &self,
    ) -> Option<SourceInterpreterVisibilitySnapshot> {
        self.safe_interp_stack.last().cloned()
    }
}
