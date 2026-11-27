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

//! The occurrence tables argument roles fill: scoped bodies, variable receivers and aliases, namespace names and computed variable names.

use super::super::state::Analyser;
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::ArgRole;

impl Analyser {
    /// The per-command occurrence tables the registry's **argument roles**
    /// produce, recorded together because they are the same walk over the same
    /// role query:
    ///
    /// * [`tcl_registry::ArgRole::NamespaceName`] words → `namespace_refs`, so
    ///   a namespace name is a navigable symbol rather than an inert word;
    /// * computed [`tcl_registry::ArgRole::VarWrite`] /
    ///   [`tcl_registry::ArgRole::VarRead`] words → `dynamic_variable_names`,
    ///   with what the constant lattice proves about the value.
    pub(super) fn record_arg_role_facts(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        invocation_offset: u32,
        presubstituted: bool,
    ) {
        self.record_namespace_name_references(
            args,
            arg_tokens,
            scope_path,
            invocation_offset,
            presubstituted,
        );
    }

    pub(super) fn record_original_scoped_body_advice(
        &mut self,
        invocation_offset: u32,
        argument_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let Some(tokens) = self.retained_invocation_tokens(invocation_offset, argument_tokens)
        else {
            return;
        };
        let Some(head_word) = tokens.words().first() else {
            return;
        };
        let image = self.command_document_source_image();
        let config = self.file_lexer_config();
        let vendor = self
            .result
            .original_vendor_source_name_in_source(&image, config, head_word.source().span)
            .cloned();
        // An owned hosted producer cannot fall through into C/Jim source advice.
        if self.vendor_source_name_policy().is_some() && vendor.is_none() {
            return;
        }
        let namespace = super::super::scope::scope_at(&self.result.global_scope, scope_path)
            .and_then(|scope| scope.naming_scope.as_ref());
        if vendor.is_none() {
            let metadata = self
                .original_static_source_name_at_span(head_word.source().span)
                .and_then(|head| {
                    crate::registry_invocation::original_conditional_registry_metadata(
                        &self.analysis_context(),
                        &tokens,
                        head,
                        namespace,
                    )
                });
            match self
                .result
                .original_conditional_registry_metadata
                .entry(invocation_offset)
            {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(metadata);
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    if entry.get() != &metadata {
                        entry.insert(None);
                        self.result.source_words_memo.clear();
                    }
                }
            }
        }
        for body in crate::registry_invocation::original_source_scoped_bodies(
            &self.analysis_context(),
            &tokens,
            self.original_static_source_name_at_span(head_word.source().span),
            namespace,
            vendor.as_ref(),
        ) {
            if !self.result.original_scoped_bodies.contains(&body) {
                self.result.original_scoped_bodies.push(body);
            }
        }
    }

    pub(super) fn record_original_variable_receivers(
        &mut self,
        invocation_offset: u32,
        argument_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let generation = self.analysis_context();
        let registry = generation.commands();
        let Some(tokens) = self.retained_invocation_tokens(invocation_offset, argument_tokens)
        else {
            return;
        };
        let assistance = self
            .head_identities
            .original_declaration_assistance(&tokens, registry);
        let source_aliases =
            crate::registry_invocation::source_structure::original_registry_words_for_tokens(
                &self.source,
                &self.result,
                &tokens,
            )
            .and_then(|words| {
                crate::registry_invocation::OriginalSourceVariableAliasOperands::capture(
                    words,
                    &generation,
                )
            });
        if let Some(source_aliases) = source_aliases {
            self.retain_original_variable_aliases(
                source_aliases,
                invocation_offset,
                &tokens,
                scope_path,
            );
        }
        let writes = if self.selected_logical_declaration_advice() {
            crate::registry_invocation::logical_source_variable_write_advice(
                &self.source,
                &self.result,
                &tokens,
            )
        } else if self.vendor_source_name_policy().is_some() {
            let image = self.command_document_source_image();
            tokens
                .words()
                .first()
                .and_then(|head| {
                    self.result.original_vendor_source_name_in_source(
                        &image,
                        self.file_lexer_config(),
                        head.source().span,
                    )
                })
                .and_then(|original| self.original_vendor_variable_metadata(original))
                .and_then(|shape| crate::registry_invocation::vendor_variable_write_advice(&shape))
        } else {
            crate::registry_invocation::original_variable_write_advice(
                registry,
                self.analysis_context().as_ref(),
                &tokens,
            )
        };
        let missing_receiver = assistance
            .as_ref()
            .is_none_or(|advice| !advice.variable_name_obligations.is_empty());
        self.record_conditional_variable_receivers(
            &tokens,
            argument_tokens,
            scope_path,
            missing_receiver,
            writes.is_none(),
        );
        for receiver in writes.unwrap_or_default() {
            if let Some(token) = argument_tokens.get(receiver.argument) {
                self.record_original_variable_write_advice(
                    token.span,
                    scope_path,
                    receiver.form,
                    None,
                );
            }
        }
        let Some(assistance) = assistance else {
            return;
        };
        self.retain_variable_declaration_receivers(
            invocation_offset,
            argument_tokens,
            scope_path,
            assistance,
        );
    }

    fn retain_original_variable_aliases(
        &mut self,
        source_aliases: crate::registry_invocation::OriginalSourceVariableAliasOperands,
        invocation_offset: u32,
        tokens: &crate::ir::CommandTokens,
        scope_path: &[usize],
    ) {
        let input = self.resolved_analysis_input();
        let logical_declarations = source_aliases
            .logical_declarations(&input, &self.head_identities)
            .unwrap_or_default();
        if let Some(spans) =
            source_aliases.unresolved_logical_name_spans(&input, &self.head_identities)
        {
            for span in spans {
                if !self.result.original_variable_name_unknowns.contains(&span) {
                    self.result.original_variable_name_unknowns.push(span);
                }
            }
        }
        for &(local, target, purpose) in source_aliases.operands() {
            let operand = (invocation_offset, local, target, purpose);
            if !self
                .result
                .original_variable_alias_operands
                .contains(&operand)
            {
                self.result.original_variable_alias_operands.push(operand);
            }
        }
        if !self
            .result
            .original_variable_alias_source_operands
            .contains(&source_aliases)
        {
            self.result
                .original_variable_alias_source_operands
                .push(source_aliases);
        }
        for declaration in logical_declarations {
            let (local, word) = declaration.local();
            let Some(&token) = word.tokens().first() else {
                continue;
            };
            self.define_var(local, token, scope_path, false, None);
            let Some(target) = declaration.target() else {
                continue;
            };
            let Some(target_input) = self.original_qualified_variable_input(target.span(), false)
            else {
                continue;
            };
            let purpose =
                crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                    &input,
                    self.source_analysis_entry.as_deref(),
                    target_input.policy(),
                );
            let Some(original) = declaration.target_occurrence(&input, target_input, purpose)
            else {
                continue;
            };
            if self
                .result
                .original_variable_symbol_conflicts
                .contains(&target.span())
            {
                continue;
            }
            if let Some(existing) = self
                .result
                .original_variable_symbols
                .iter_mut()
                .find(|site| site.span() == target.span())
            {
                if !existing.merge_declaration(&original) {
                    self.result
                        .original_variable_symbol_conflicts
                        .push(target.span());
                    continue;
                }
            } else {
                self.result.original_variable_symbols.push(original.clone());
            }
            self.define_original_namespace_alias_target(&original);
            // The link's display is attached only after actual original
            // target geometry. It never supplies symbol identity.
            if let Some(report) = declaration.alias().and_then(|alias| {
                self.alias_target_report(&alias.target, &tokens.argv_texts[1..], scope_path)
            }) {
                self.set_var_link_report(local, scope_path, report, target.span());
            }
        }
    }

    fn retain_variable_declaration_receivers(
        &mut self,
        invocation_offset: u32,
        argument_tokens: &[Token],
        scope_path: &[usize],
        assistance: crate::registry_invocation::OriginalDeclarationAssistance,
    ) {
        for &argument in &assistance.variable_name_obligations {
            if let Some(token) = argument_tokens.get(argument)
                && !self
                    .result
                    .original_variable_name_unknowns
                    .contains(&token.span)
            {
                self.result.original_variable_name_unknowns.push(token.span);
            }
        }
        if assistance.variable_alias_declarations != 0
            && !self
                .result
                .original_variable_alias_obligations
                .contains(&(invocation_offset, assistance.variable_alias_declarations))
        {
            self.result
                .original_variable_alias_obligations
                .push((invocation_offset, assistance.variable_alias_declarations));
        }
        for alias in assistance.variable_alias_operands {
            let Some(local) = argument_tokens.get(alias.local) else {
                continue;
            };
            let Some(target) = argument_tokens.get(alias.target) else {
                continue;
            };
            let operand = (invocation_offset, local.span, target.span, alias.purpose);
            if !self
                .result
                .original_variable_alias_operands
                .contains(&operand)
            {
                self.result.original_variable_alias_operands.push(operand);
            }
        }
        for receiver in assistance.variable_receivers {
            let Some(token) = argument_tokens.get(receiver.argument) else {
                continue;
            };
            self.record_original_variable_receiver(token.span, scope_path, receiver.form, false);
        }
    }

    fn record_conditional_variable_receivers(
        &mut self,
        tokens: &crate::ir::CommandTokens,
        argument_tokens: &[Token],
        scope_path: &[usize],
        missing_receiver: bool,
        missing_write: bool,
    ) {
        // Conditional catalogue metadata owns source cards separately from
        // executed handler facts. Alias grammar retains its own target/local
        // producer and cannot borrow an ordinary variable receiver here.
        if self.vendor_source_name_policy().is_none() && (missing_receiver || missing_write) {
            let conditional = tokens
                .words()
                .first()
                .and_then(|head| self.original_static_source_name_at_span(head.source().span))
                .and_then(|head| {
                    crate::registry_invocation::original_conditional_registry_metadata(
                        &self.analysis_context(),
                        tokens,
                        head,
                        self.original_variable_namespace_at(scope_path).as_ref(),
                    )
                });
            if let Some(metadata) = conditional
                && metadata.roles_complete()
                && !metadata.possible_traits().intersects(
                    tcl_registry::Traits::CREATES_SCOPE_ALIAS
                        | tcl_registry::Traits::DESTROYS_VARIABLE,
                )
            {
                for &(index, role) in metadata.roles() {
                    if !matches!(
                        role,
                        tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                    ) {
                        continue;
                    }
                    let Some(argument) = metadata.argument_offset().checked_add(usize::from(index))
                    else {
                        continue;
                    };
                    let (Some(token), Some(form)) = (
                        argument_tokens.get(argument),
                        metadata.possible_variable_receiver_operand_form(argument),
                    ) else {
                        continue;
                    };
                    let declaration = role == tcl_registry::ArgRole::VarWrite
                        && form == tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined;
                    if declaration {
                        self.record_original_variable_write_advice(
                            token.span,
                            scope_path,
                            form,
                            Some(&metadata),
                        );
                    }
                    self.record_original_variable_receiver(
                        token.span,
                        scope_path,
                        form,
                        declaration,
                    );
                    if !metadata.obligations().is_empty()
                        && !self
                            .result
                            .original_variable_name_unknowns
                            .contains(&token.span)
                    {
                        self.result.original_variable_name_unknowns.push(token.span);
                    }
                }
            }
        }
    }

    /// Record each [`tcl_registry::arg_role::ArgRole::NamespaceName`]
    /// argument as a [`NamespaceRef`](super::super::types::NamespaceRef): a word
    /// naming a namespace, which navigation must reach.
    ///
    /// A **relative** name roots against the call site's own
    /// command-resolution namespace, which is what Tcl does — pinned on
    /// tclsh 9.0.4 and 8.6.16, byte-identically: inside `namespace eval
    /// ::outer`, `namespace exists inner` answers `1` (it means
    /// `::outer::inner`) while the same words at global scope answer `0`.
    /// A proc body's current namespace is its *defining* namespace, which is
    /// exactly what [`Self::command_resolution_namespace`] reports.
    ///
    /// A computed word requires its genuine retained value input; unsupported
    /// substitutions supply no namespace identity. The declaring flag comes from the registry's
    /// [`tcl_registry::Traits::DECLARES_NAMESPACE`], never from the
    /// subcommand's spelling: `namespace eval` declares, `namespace inscope`
    /// — same argument layout, same analyser hook — does not.
    ///
    /// The **empty** literal is a real namespace name and is recorded like
    /// any other relative one.  It needs no special case, because it *is* the
    /// ordinary relative rule: `crate::naming::qualify` maps it to `::` at
    /// global scope and to `::outer::` inside `namespace eval ::outer`, which
    /// is exactly what tclsh 9.0.4 and 8.6.16 do (byte-identical). At global
    /// scope `namespace exists {}` is `1`, `namespace children {}` equals
    /// `namespace children ::`, `namespace inscope {} {namespace current}` is
    /// `::`, and `namespace eval {} {…}` reopens the global namespace
    /// (`namespace current` inside is `::`, and the variables it sets land in
    /// `::`). Inside `namespace eval ::outer` the very same word means a
    /// namespace that cannot exist: `namespace exists {}` is `0`, `namespace
    /// children {}` fails `namespace "" not found in "::outer"`, and
    /// `namespace eval {} {…}` fails `can't create namespace "": only global
    /// namespace can have empty name`.  Recording it as `::outer::` — a name
    /// nothing ever declares — makes navigation abstain there, which is the
    /// right answer.
    ///
    /// A **whitespace-only** word is not empty and is not special either:
    /// `namespace eval " " {…}` genuinely creates a namespace named `" "`,
    /// which both interpreters list as `{:: }` among `namespace children ::`.
    pub(super) fn record_namespace_name_references(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        invocation_offset: u32,
        presubstituted: bool,
    ) {
        let Some(original) =
            self.original_name_source(invocation_offset, arg_tokens, presubstituted)
        else {
            return;
        };
        let roles = original.argument_roles();
        let declares = original
            .registry()
            .and_then(|registry| {
                registry.with_schema(|schema| {
                    schema
                        .semantics
                        .traits
                        .contains(tcl_registry::Traits::DECLARES_NAMESPACE)
                })
            })
            .unwrap_or(false);
        let here = self.command_resolution_namespace(scope_path);
        for argument in roles
            .iter()
            .filter_map(|(argument, role)| (*role == ArgRole::NamespaceName).then_some(*argument))
        {
            let Some(idx) = original.written_index(argument) else {
                continue;
            };
            let (Some(name), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) else {
                continue;
            };
            // Implementation contract: naming.source.original-point-operand-projection
            // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
            let logical = self.selected_logical_declaration_advice();
            let (original_input, source_context) = self.original_namespace_reference_input(
                original.word(argument),
                tok.span,
                invocation_offset,
                scope_path,
            );
            if original_input.is_none() || (logical && source_context.is_none()) {
                // The selected source role remains a coverage obligation
                // when its value or independent Logical scope is unavailable.
                if !self.result.namespace_name_unknowns.contains(&tok.span) {
                    self.result.namespace_name_unknowns.push(tok.span);
                }
                if original_input.is_none() && crate::naming::is_dynamic_word(name) {
                    continue;
                }
            }
            self.result.namespace_refs.push(
                super::super::types::NamespaceRef::from_original(
                    source_context,
                    self.declaration_name_policy(),
                    name,
                    tok.span,
                    u16::from(tok.content_offset),
                    declares,
                    crate::naming::qualify(&here, name),
                )
                .with_original_input(original_input),
            );
        }
    }

    /// Join the role-selected original word to its source naming purpose.
    /// Logical context comes from the independently retained invocation/frame,
    /// never the reporting walk's current namespace. Native construction stays
    /// terminal even when its original key is unavailable.
    pub(super) fn original_namespace_reference_input(
        &self,
        word: Option<&tcl_lexer::NativeWord>,
        span: Span,
        invocation_offset: u32,
        scope_path: &[usize],
    ) -> (
        Option<crate::signature_scan::scope::SignatureSourceNameInput>,
        Option<tcl_names::namespace::SignatureNamespaceScope>,
    ) {
        use crate::signature_scan::original_name::OriginalSourceNamePurpose;
        use crate::signature_scan::scope::SignatureSourceNameInput;
        use tcl_names::word::SignatureSourceNameKey;

        if !self.selected_logical_declaration_advice() {
            let input = self.declaration_name_policy().and_then(|policy| {
                SignatureSourceNameKey::from_original_native_word(word?, self.word_rules(), policy)
                    .map(SignatureSourceNameInput::OriginalWord)
            });
            return (input, self.declaration_namespace_scope(scope_path));
        }
        let Some(owner) = self.result.resolved_input.as_ref() else {
            return (None, None);
        };
        let image = self.command_document_source_image();
        if !self
            .result
            .matches_original_source_image(&image, owner.lexer_config())
            || !self.head_identities.matches_resolved_analysis_input(owner)
        {
            return (None, None);
        }
        let Some((word, policy)) = word.zip(self.declaration_name_policy()) else {
            return (None, None);
        };
        if word.image() != &image || word.config() != owner.lexer_config() {
            return (None, None);
        }
        let purpose = OriginalSourceNamePurpose::for_analysis_input(
            owner,
            self.source_analysis_entry.as_deref(),
            policy,
        );
        let OriginalSourceNamePurpose::Logical(_, _) = purpose else {
            return (None, None);
        };
        let Some(input) = purpose
            .key(word, self.word_rules())
            .map(SignatureSourceNameInput::OriginalWord)
        else {
            return (None, None);
        };
        let context = self
            .head_identities
            .original_namespace_operand_at_span_in_source(&image, span, owner.lexer_config())
            .filter(|operand| {
                operand.site().offset == invocation_offset && operand.name_input() == &input
            })
            .map(|operand| operand.context().clone());
        (Some(input), context)
    }

    /// Record each computed variable-name argument as a
    /// [`DynamicVariableNameSite`](super::super::types::DynamicVariableNameSite):
    /// the word's span plus the name it provably evaluates to, when the
    /// constant lattice dominates the site.
    ///
    /// Which argument of which command names a variable is the registry's
    /// answer ([`tcl_registry::ArgRole::VarWrite`] /
    /// [`tcl_registry::ArgRole::VarRead`]); whether the word is computed is
    /// [`crate::dynamic_names::names_a_dynamic_variable`]'s.  No command name
    /// is matched here.
    ///
    /// A **brace-quoted** word is skipped: `set {$n} 1` names the variable
    /// literally called `$n` and substitutes nothing (tclsh 8.6.14: `set {$n}
    /// v; info exists {$n}` -> 1 while `info exists n` -> 0).  The word text
    /// alone cannot tell the two spellings apart, so the token kind decides —
    /// the same authority `dynamic_names::scan_command` uses for the compiler
    /// -side barrier.
    ///
    /// The resolution is [`Self::resolve_dynamic_word`]'s, which folds only
    /// through values that **dominate** the site: a branch-conditional
    /// binding, a parameter, or a `[…]` substitution yields `None`, which a
    /// consumer must read as "could be anything".
    pub(super) fn record_dynamic_variable_name_sites(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        offset: u32,
        presubstituted: bool,
    ) {
        let Some(original) = self.original_name_source(offset, arg_tokens, presubstituted) else {
            return;
        };
        let roles = original.variable_name_arguments();
        for (role, writes) in [(ArgRole::VarWrite, true), (ArgRole::VarRead, false)] {
            for argument in roles
                .iter()
                .filter_map(|(argument, selected)| (*selected == role).then_some(*argument))
            {
                let Some(idx) = original.written_index(argument) else {
                    continue;
                };
                let (Some(word), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) else {
                    continue;
                };
                if tok.kind == TokenType::Str
                    || !crate::dynamic_names::names_a_dynamic_variable(word)
                {
                    continue;
                }
                // A word carrying both roles (a read-modify-write target)
                // is one site, recorded under the write it also performs.
                if self
                    .result
                    .dynamic_variable_names
                    .iter()
                    .any(|s| s.span == tok.span)
                {
                    continue;
                }
                let is_single = arg_single.get(idx).copied().unwrap_or(false);
                let resolved = self.resolve_dynamic_word(word, Some(*tok), is_single, scope_path);
                self.result.dynamic_variable_names.push(
                    super::super::types::DynamicVariableNameSite {
                        span: tok.span,
                        resolved,
                        writes,
                    },
                );
            }
        }
    }
}
