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

//! Variables a command writes: a direct `set`, and the loop and output variables a call's argument roles name.

use super::super::state::Analyser;
use super::interpreters::created_interpreter;
use super::role_position;
use tcl_lexer::{Span, Token, TokenType};

/// Whether a word substitutes at all: a braced word never does, whatever it
/// holds; any other word does when it carries a `$` or `[` or is itself a
/// variable or command substitution.
fn word_substitutes(text: &str, tok: Token) -> bool {
    tok.kind != TokenType::Str && super::super::diagnostics::helpers::has_substitution(text, &tok)
}

/// Whether a written-variable word names a static variable: a non-empty word
/// that substitutes nothing, or an element of an array whose name is written
/// literally — the key may substitute (`incr hits($word)`), the base may not.
fn names_static_variable(text: &str, tok: Token) -> bool {
    if text.is_empty() {
        return false;
    }
    if !word_substitutes(text, tok) {
        return true;
    }
    crate::naming::split_element_ref(text)
        .is_some_and(|(base, _)| !base.is_empty() && !crate::naming::is_dynamic_word(base))
}

impl Analyser {
    /// Bind the variable a direct one-target write names to the value word
    /// it stores — `set name value` — for any invocation whose declared
    /// semantics is that write
    /// ([`tcl_registry::value_transfer::ResolvedSemantics::writes_value_word`]),
    /// the registry's declaration rather than the command's spelling or an
    /// analyser hook:
    ///
    /// - the written name is an assignment the unused-variable hint may
    ///   report, so its definition escalates `warn_if_unused` over the one
    ///   the generic role binding ([`Self::handle_var_binding_command`])
    ///   gave the same word;
    /// - the constant-string environment reads the value word's `CellWrite`
    ///   evaluation (`value_word_write`) when the word is one literal
    ///   token, so a regex source or a namespace a later word names
    ///   resolves through the variable;
    /// - a value word that is one `[…]` substitution creating a child
    ///   interpreter binds the name to that interpreter's key, the
    ///   interpreter domain reached from this binding;
    /// - a value word that is one `[…]` substitution the registry folds
    ///   binds the folded constant, and anything else clears both;
    /// - `set auto_path …` records the search path the value word names.
    ///
    /// The one-word read form (`set name`) binds nothing: the walk's
    /// `VarRead`-role reference recording
    /// ([`Self::record_arg_var_reads`]) takes it.
    pub fn bind_value_word_assignment(
        &mut self,
        command_token: Token,
        argument_tokens: &[Token],
        scope_path: &[usize],
    ) {
        use tcl_registry::arg_role::ArgRole;
        let Some(original) = self.original_diagnostic_call_at(command_token, argument_tokens)
        else {
            return;
        };
        let Some(Some((Some(roles), semantics, canonical))) = original.with_schema(|call| {
            call.semantics.value.writes_value_word().then(|| {
                (
                    call.arg_roles(),
                    call.semantics.value,
                    call.canonical_command.to_owned(),
                )
            })
        }) else {
            return;
        };
        let Some((arguments, tokens, single_token_word)) = original.source_arguments() else {
            return;
        };
        let args = arguments.as_slice();
        let arg_tokens = tokens.as_slice();
        // The write's shape (`CellWriteSemantics`): one `VarWrite` word, and
        // the value word after it, last.
        let Some(target) = role_position(&roles, ArgRole::VarWrite) else {
            return;
        };
        let value = target + 1;
        let (Some(name), Some(&name_tok), Some(value_text), Some(&value_tok)) = (
            args.get(target),
            arg_tokens.get(target),
            args.get(value),
            arg_tokens.get(value),
        ) else {
            return;
        };
        if value + 1 != args.len() {
            return;
        }
        if names_static_variable(name, name_tok) {
            self.define_var(name, name_tok, scope_path, true, None);
        }
        if original.literal(target).is_none() {
            return;
        }
        self.record_search_path_write(
            args,
            arg_tokens,
            target,
            value..value + 1,
            super::super::types::AutoPathForm::Assign,
        );
        let single = single_token_word.get(value).copied().unwrap_or(false);
        let written = single
            .then(|| {
                self.value_word_write(
                    semantics,
                    &canonical,
                    (args, roles.as_slice()),
                    (target, value),
                    original.literal(value)?,
                )
            })
            .flatten();
        if let Some(written) = written {
            self.set_const_string(name, written, value_tok.span, scope_path);
            self.clear_interp_var_binding(name, scope_path);
        } else if let Some((path, safe)) = original
            .word(value)
            .and_then(|word| self.original_substitution_call(word))
            .and_then(|call| {
                call.with_schema(|schema| created_interpreter(&schema.state_transitions()))
                    .flatten()
            })
        {
            self.bind_created_interpreter_value(path, safe, name, value_tok, scope_path);
        } else if single
            && value_tok.kind == TokenType::Cmd
            && let Some(folded) =
                self.try_fold_const_cmd_subst_rhs(value_text, value_tok, scope_path)
        {
            // `set VAR [cmd …]` whose substitution is a compile-time
            // constant: the registry `const_fold` / frame-fact engine proves
            // the value, so VAR enters the same constant-string lattice a
            // literal value does — unblocking the `${ns}::setdef`-style
            // navigation chain (`resolve_dynamic_command_head`) for the
            // `set ns [namespace qualifiers ::tc::X]` shape.
            self.set_const_string(name, folded, value_tok.span, scope_path);
            self.clear_interp_var_binding(name, scope_path);
        } else {
            self.clear_const_string(name, scope_path);
            self.clear_interp_var_binding(name, scope_path);
        }
    }

    /// The value a direct one-target write stores when its value word is
    /// one literal token: the declared semantics' `CellWrite` evaluation
    /// over the call's words, the value word cooked as Tcl reads it
    /// (backslashes substituted in a bare or quoted word, the braces' content
    /// in a braced one), the resolver's roles on its operands. `None` for a
    /// word that is no literal, or an evaluation that writes no exact value
    /// to the target.
    fn value_word_write(
        &self,
        semantics: tcl_registry::value_transfer::ResolvedSemantics,
        canonical: &str,
        (args, roles): (&[String], &[(usize, tcl_registry::ArgRole)]),
        (target, value): (usize, usize),
        value_literal: &str,
    ) -> Option<String> {
        use tcl_registry::value_transfer::{
            Budget, EvalAnswer, LiteralInputs, OperandId, StoreOutcome,
        };
        let words: Vec<&str> = args
            .iter()
            .enumerate()
            .map(|(index, word)| {
                if index == value {
                    value_literal
                } else {
                    word.as_str()
                }
            })
            .collect();
        let input = self.resolved_analysis_input();
        let generation = input.context_registry();
        let mut context =
            tcl_registry::value_transfer::AnalysisContext::detached(Some(input.unit_profile()));
        context.grammar = self
            .lexer_config()
            .grammar_over(input.unit_profile().grammar);
        context.script_encoding = input.script_encoding();
        context.registry_generation = generation.commands().generation();
        context.overlay_generation = generation.commands().overlay_generation();
        let mut inputs = LiteralInputs::new(canonical, None, &words, Some(input.unit_profile()))
            .with_context(context);
        for &(index, role) in roles {
            inputs = inputs.with_role(OperandId(index), role);
        }
        match semantics
            .semantics()?
            .evaluate(&inputs, &mut Budget::evaluation())
        {
            EvalAnswer::Evaluated(outcome) => {
                outcome
                    .ordered_stores
                    .into_iter()
                    .find_map(|store| match store {
                        StoreOutcome::Write { target: t, value } if t.0 == OperandId(target) => {
                            String::from_utf8(value.bytes).ok()
                        }
                        _ => None,
                    })
            }
            EvalAnswer::Pending | EvalAnswer::Declined(_) => None,
        }
    }

    /// Registry folding keeps whole-module command trust and exact original variable sources.
    /// Declaration labels cannot supply entered object-frame facts.
    fn try_fold_const_cmd_subst_rhs(
        &mut self,
        value_text: &str,
        value_token: Token,
        scope_path: &[usize],
    ) -> Option<String> {
        let registry = self.registry.clone()?;
        let trimmed = value_text.trim();
        let inner = trimmed
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(trimmed);
        if !crate::const_subst::head_may_fold(&registry, inner) {
            return None;
        }
        let trust = self.whole_file_command_trust()?;
        let version = (!self.result.dialect.is_empty())
            .then(|| self.profile.const_fold_version())
            .flatten();
        let trusts = |name: &str| trust.trusts(name);
        let lookup = |arena: &tcl_lexer::ExecutablePartArena,
                      component: &tcl_lexer::SpannedExecutablePart| {
            self.lookup_logical_const_component(arena, component)
        };
        let operand = self.retained_value_word(value_token)?;
        crate::const_subst::ConstSubstCtx {
            registry: &registry,
            resolution_namespace: &self.command_resolution_namespace(scope_path),
            namespace_context: None,
            version,
            defining_class: None,
            trusts: &trusts,
            lookup_var: &|_| None,
            script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
        }
        .fold_original_source_substitution(&operand, &lookup)
    }

    /// Define a list of variables from a varList token (e.g. the
    /// loop-variable list of `foreach`).
    ///
    /// Each name gets its **own** definition span, located inside the
    /// varList token's content (the token's `content_offset` skips a
    /// leading `{`/`"`). This — crucially — gives same-token bindings
    /// like `foreach {b a}` / `dict for {k v}` distinct,
    /// declaration-ordered spans, so downstream offset-sorted consumers
    /// (the `symbols` / `symbolgraph` CLI verbs) stay deterministic and
    /// source-ordered. Tcl list grouping and backslash substitutions are
    /// decoded for the variable name while each definition span remains the
    /// exact source range of its list element.
    pub(in super::super) fn define_vars_from_list(
        &mut self,
        var_list_text: &str,
        tok: Token,
        scope_path: &[usize],
    ) {
        let Ok(names) = self.word_rules().split_list(var_list_text) else {
            return;
        };
        let content_start = tok.span.start() + u32::from(tok.content_offset);
        let mut pos = 0usize;
        for name in names {
            let Ok(Some(element)) = tcl_syntax::list::find_element(var_list_text, pos) else {
                return;
            };
            let start = content_start + u32::try_from(element.value.start).unwrap_or(0);
            let end = content_start + u32::try_from(element.value.end).unwrap_or(0);
            self.define_var(
                name.as_ref(),
                tok,
                scope_path,
                true,
                Some(Span::new(start, end)),
            );
            // The element is a written variable name of its own, declared
            // here as a whole-word receiver is.
            self.record_original_variable_receiver(
                Span::new(start, end),
                scope_path,
                tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined,
                false,
            );
            pos = element.next;
        }
    }

    /// Bind the loop and output variables a call's argument roles name — the
    /// one binder behind every command that binds a variable by position,
    /// read off the registry's roles and never a per-command layout:
    ///
    /// - an [`tcl_registry::ArgRole::LoopVarList`] word is a Tcl list of
    ///   names bound before a body runs — `foreach` / `lmap` var lists,
    ///   `dict for` / `dict map` / `array for` key-value pairs, `dict
    ///   update`'s locals — each name defined at its own element's span
    ///   ([`Self::define_vars_from_list`]);
    /// - an [`tcl_registry::ArgRole::VarWrite`] word names a variable the
    ///   command writes — `lassign`, `scan`, `regexp`, `regsub`, `gets`,
    ///   `binary scan`, `vwait`, `incr`, `append`, `lappend`, `dict set`,
    ///   `info default`, … — bound with `warn_if_unused = false`: like
    ///   `catch`'s result variable the binding is a documented side effect,
    ///   and a read-modify-write command (`incr`, `append`, `lappend`) reads
    ///   the target before it writes it.
    ///
    /// Only a word that names a static variable binds: one that substitutes
    /// nothing, or (for a written variable) an element of an array whose name
    /// is written literally — `incr hits($word)` writes an element of `hits`
    /// whatever the key. A computed name (`scan $s $fmt $dyn`, `foreach
    /// $names …`) names nothing statically.
    ///
    /// The roles are read over the words' spellings, a computed word taking
    /// one position and never standing for an option (the reading the editor
    /// has always given a call), under the invocation the walk's context
    /// proves. Two trait families opt out:
    ///
    /// - [`tcl_registry::Traits::CREATES_SCOPE_ALIAS`] (`global` /
    ///   `variable` / `upvar`): the alias an invocation states is
    ///   [`Self::apply_state_transitions`]'s — tail-stripping (`global
    ///   ::ns::v` binds the local alias `v`, not the qualified name) and
    ///   name/value pairing are the resolver's, which a flat `VarWrite` walk
    ///   would get wrong. The opt-out reads the command's trait only:
    ///   `SubCommand::creates_scope_alias` is not folded into the
    ///   invocation's traits, because `dict update`, `dict with` and `my
    ///   variable` carry it for aliases no `VariableCellAliasTransition`
    ///   states — this binder is what binds their names. So `namespace
    ///   upvar`, the one flagged subcommand whose aliases the resolver does
    ///   state, has its strided `VarWrite` locals bound here as well as
    ///   there: the same name at the same word, which [`Self::define_var`]
    ///   re-defines idempotently, and the link the alias set is kept.
    /// - [`tcl_registry::Traits::DESTROYS_VARIABLE`] (`unset`): its
    ///   `VarWrite` role marks a *removal* target (an SSA def that kills the
    ///   value), not a binding to record.
    ///
    /// A name [`Self::bind_value_word_assignment`] also binds (`set`'s) is
    /// bound twice for one word: [`Self::define_var`]'s re-definition path is
    /// idempotent for the same token span, and the assignment's
    /// `warn_if_unused = true` escalates this binding's `false`.
    ///
    /// Void-returning (self-guards on the role set) so it composes with the
    /// other side-effect handlers — `regexp` / `regsub` also feed
    /// `handle_regex_pattern_capture`, which must still run.
    pub fn handle_var_binding_command(
        &mut self,
        command_token: Token,
        argument_tokens: &[Token],
        scope_path: &[usize],
    ) {
        use tcl_registry::arg_role::ArgRole;
        let Some(original) = self.original_diagnostic_call_at(command_token, argument_tokens)
        else {
            return;
        };
        let Some((Some(roles), appends_from)) = original.with_schema(|call| {
            let opted_out = call.semantics.traits.intersects(
                tcl_registry::Traits::CREATES_SCOPE_ALIAS | tcl_registry::Traits::DESTROYS_VARIABLE,
            );
            let appends_from = match call.semantics.var_elements_effect {
                Some(tcl_registry::VarElementsEffect::AppendsListElements { values_from }) => {
                    Some(call.semantics.argument_offset + usize::from(values_from))
                }
                _ => None,
            };
            (
                (!opted_out).then(|| call.arg_roles()).flatten(),
                appends_from,
            )
        }) else {
            return;
        };
        let Some((arguments, tokens, _single)) = original.source_arguments() else {
            return;
        };
        let args = arguments.as_slice();
        let arg_tokens = tokens.as_slice();
        // A list append to the package search path is a new directory per
        // word (`lappend auto_path DIR…`).
        if let (Some(values_from), Some(written)) =
            (appends_from, role_position(&roles, ArgRole::VarWrite))
        {
            self.record_search_path_write(
                args,
                arg_tokens,
                written,
                values_from..args.len(),
                super::super::types::AutoPathForm::Append,
            );
        }
        for (index, role) in roles {
            let (Some(text), Some(&tok)) = (args.get(index), arg_tokens.get(index)) else {
                continue;
            };
            match role {
                ArgRole::LoopVarList if !word_substitutes(text, tok) => {
                    self.define_vars_from_list(text, tok, scope_path);
                }
                ArgRole::VarWrite if names_static_variable(text, tok) => {
                    self.define_var(text, tok, scope_path, false, None);
                }
                _ => {}
            }
        }
    }

    /// Register the loop variables of `dict for {keyVar valueVar}
    /// dictValue body`.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::DictFor`]
    /// (stamped on `dict`'s `for` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_dict_for_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() >= 4 && arg_tokens.len() >= 2 {
            self.define_vars_from_list(&args[1], arg_tokens[1], scope_path);
        }
    }

    /// Register the alias variables of `dict update dictVar key1 var1
    /// ?key2 var2 ...? body` — vars at 3, 5, 7, … (i.e. `len-2` last).
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::DictUpdate`] (stamped on
    /// `dict`'s `update` subcommand).
    pub fn handle_dict_update_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() < 5 || !(args.len() - 3).is_multiple_of(2) {
            return;
        }
        let mut i = 3;
        while i + 1 < args.len() {
            if let Some(tok) = arg_tokens.get(i) {
                self.define_var(&args[i], *tok, scope_path, false, None);
            }
            i += 2;
        }
    }

    /// Register the key variables of `dict with dictVar body` — only
    /// the no-path case is statically resolvable, and only when the
    /// dict came from a const literal.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::DictWith`]
    /// (stamped on `dict`'s `with` subcommand).
    pub fn handle_dict_with_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() != 3 || arg_tokens.len() < 2 {
            return;
        }
        let Some((value, value_span)) = self
            .lookup_logical_const_string_at(&args[1], arg_tokens[1].span)
            .map(|(value, span)| (value.to_owned(), span))
        else {
            return;
        };
        self.retain_readonly_dictionary_keys(&value, value_span, arg_tokens[1].span, scope_path);
    }
}
