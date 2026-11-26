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

//! The typed dispatch of one command to its handler family through the resolved `AnalyserHookId`, and the state transitions it applies.

use super::super::state::Analyser;
use super::{DispatchSite, ResolvedAnalyserHook, WordFacts};
use tcl_lexer::Token;

impl Analyser {
    /// Typed per-command dispatch for [`Self::process_command`].
    ///
    /// Which handler family owns a command form is registry data — the
    /// [`AnalyserHookId`] stamped on the `CommandSpec` (or, for the
    /// subcommand-shaped families like `namespace eval` / `dict for` /
    /// `interp alias`, on the `SubCommand`) — so the dispatch is one
    /// typed `match`, not a chain of name-guarded calls.  The
    /// early-return families stop the walk by returning `true`; the void
    /// families fall through to the shared
    /// tail: the registry-role-driven handlers that consider every
    /// command (`VarWrite` bindings, symbol definers, the tcllib
    /// `::import` wrapper idiom) and the generic `ArgRole::Body`
    /// recursion.
    pub(super) fn dispatch_command_handlers(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        words: WordFacts<'_>,
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        // IRULE5001's debug gate spans everything below: the hook handlers
        // that own their own body walk (`switch`, `foreach`, `catch`) and the
        // generic `ArgRole::Body` recursion (`if`, `while`, `for`, `try`)
        // alike. Bracketing the whole dispatch is what makes nested bodies
        // inherit the gate.
        let gated = self.irules_debug_gate_opens(cmd_name, args, arg_tokens, cmd_tok);
        if gated {
            self.irules_debug_gate_depth += 1;
        }
        self.dispatch_command_handlers_inner(
            cmd_name, args, arg_tokens, words, cmd_tok, scope_path,
        );
        if gated {
            self.irules_debug_gate_depth -= 1;
        }
    }

    /// The dispatch itself — see [`Self::dispatch_command_handlers`], which
    /// wraps this in IRULE5001's debug-gate bracket.
    fn dispatch_command_handlers_inner(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        words: WordFacts<'_>,
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        let arg_single = words.single;
        if self.dispatch_analyser_hook(cmd_name, args, arg_tokens, words, cmd_tok, scope_path) {
            return;
        }

        // The scope aliases the call's state transitions state — `global`,
        // `variable`, `upvar`, `namespace upvar`, a pack command's alias
        // facts — resolved over the words' source facts, so a computed word
        // reaches the registry resolver as computed.
        self.apply_invocation_transitions(cmd_name, args, arg_tokens, words, cmd_tok, scope_path);
        // The loop and output variables the call's roles name (`foreach`'s
        // var lists, `dict for`'s pair, `lassign`, `scan`, `regexp`, `incr`,
        // `append`, …), so completion/hover/definition see the bound names.
        self.handle_var_binding_command(cmd_tok, arg_tokens, scope_path);
        // A direct one-target write of a value word (`set name value`) binds
        // its name to that word — the constant-string environment, a
        // created interpreter's key and the search-path record — from the
        // registry's `CellWrite` declaration, not the command's spelling.
        self.bind_value_word_assignment(cmd_tok, arg_tokens, scope_path);
        // Registry symbol-definer commands (`tcltest::test NAME …`) contribute a
        // lightweight named definition to the outline.  Void handler — it only
        // records the symbol; the body still recurses via the generic
        // `ArgRole::Body` walk below.
        // Definition and body descriptors are registry semantics, so read the
        // document's offset-keyed identity before asking either one.  This is
        // what lets an iRules event handler reached through a proven alias or
        // rename contribute the same outline/context as `when`, while a
        // spelling a user `proc` took over cannot inherit `when`'s body.
        let generation = self.analysis_context();
        let symbol_advice = (|| {
            let tokens = self.retained_invocation_tokens(cmd_tok.span.start(), arg_tokens)?;
            if self.result.has_original_vendor_source_names() {
                let image = self.command_document_source_image();
                let config = self.file_lexer_config();
                let original = self.result.original_vendor_source_name_in_source(
                    &image,
                    config,
                    cmd_tok.span,
                )?;
                let words = crate::registry_invocation::original_native_compiler_words(
                    &image,
                    tokens.words(),
                    cmd_tok.span.start(),
                    config,
                )?;
                return crate::registry_invocation::vendor_symbol_declaration_advice(
                    &generation,
                    &tokens,
                    original.name_input(),
                    &words,
                );
            }
            crate::registry_invocation::original_symbol_declaration_advice(
                generation.commands(),
                generation.as_ref(),
                &tokens,
            )
            .or_else(|| {
                let head = self.original_static_source_name_at_span(cmd_tok.span)?;
                let namespace =
                    super::super::scope::scope_at(&self.result.global_scope, scope_path)
                        .and_then(|scope| scope.naming_scope.as_ref());
                let metadata = crate::registry_invocation::original_conditional_registry_metadata(
                    &generation,
                    &tokens,
                    head,
                    namespace,
                )?;
                let symbol = metadata.symbol_definition()?;
                let supplied = metadata.original_words().len().checked_sub(1)?;
                if usize::from(symbol.name_arg) >= supplied
                    || symbol
                        .requires_arg
                        .is_some_and(|ordinal| usize::from(ordinal) >= supplied)
                {
                    return None;
                }
                Some(
                    crate::registry_invocation::OriginalSymbolDeclarationAdvice {
                        command: metadata.command().to_owned(),
                        symbol,
                        traits: metadata.possible_traits(),
                        conditional_metadata: Some(metadata),
                    },
                )
            })
        })();
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_SYMBOL_ADVICE").is_some() {
            eprintln!(
                "ORIGINAL_SYMBOL_ADVICE consumer offset={} tokens={} advice={} logical={}",
                cmd_tok.span.start(),
                self.retained_invocation_tokens(cmd_tok.span.start(), arg_tokens)
                    .is_some(),
                symbol_advice.is_some(),
                self.result.allows_lexical_declaration_advice()
            );
        }
        if let Some(advice) = symbol_advice {
            self.handle_defines_symbol(&advice, args, arg_tokens, arg_single, cmd_tok, scope_path);
        }
        // The tcllib `<NS>::import <alias>` wrapper idiom — recognised by
        // the call's own `::import` tail, not a registry name.
        self.handle_tcllib_import_wrapper(cmd_name, cmd_tok, args, scope_path);

        // Generic body recursion via the command registry's
        // `ArgRole::Body`.  Picks up `if` / `while` / `for` / `when` /
        // `eval` / `uplevel` / `subst` / etc. — every command whose
        // registry spec marks an argument index as `BODY`, each body
        // walked at the depth its clause timing (or, without a clause
        // grammar, the command's traits) gives it.  The early-return hook
        // arms above already consumed the commands that own their body
        // walk (proc, oo::class, oo::define, namespace eval, foreach,
        // switch, catch), so this loop only fires for the rest — `try`
        // among them since its hook retired.
        //
        // For `when EVENT { body }` the iRules dialect spec
        // marks arg 1 as BODY; set `current_event` for the body
        // walk so race-detection diagnostics see the event name.
        self.dispatch_body_arguments(
            cmd_name,
            args,
            arg_tokens,
            arg_single,
            cmd_tok.span.start(),
            scope_path,
        );
        // A braced argument a proved user procedure runs as a script.
        self.dispatch_script_parameter_arguments(args, arg_tokens, cmd_tok, scope_path);
    }

    /// Resolve the [`AnalyserHookId`] for a command head through the shared
    /// registry/context owner. A leading global qualifier on a rootable
    /// command (`::proc`, `::set`, `::namespace`, …) denotes the same exact
    /// registry binding as its bare spelling; method-context-only commands
    /// without a global form are rejected by the registry's rooted-fallback
    /// policy. Subcommand selection remains exact.
    ///
    /// Outside an `analyse*` run (unit harnesses drive handlers on a
    /// bare `Analyser::new()`) the shared core registry stands in for
    /// the stashed dialect-aware one.
    pub(in crate::analyser) fn resolve_analyser_hook(
        &self,
        cmd_name: &str,
        args: &[String],
    ) -> Option<tcl_registry::hooks::AnalyserHookId> {
        self.resolve_analyser_hook_call(cmd_name, args)
            .map(|resolved| resolved.hook)
    }

    /// Declared transitions and their operands from the genuine selected source
    /// call. Captured alias operands keep their original producers and spans.
    fn original_registry_state_transitions(
        &self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        words: WordFacts<'_>,
        cmd_tok: Token,
        scope_path: &[usize],
    ) -> Option<(tcl_registry::StateTransitions, Vec<String>, Vec<Token>)> {
        let site = DispatchSite {
            cmd_name,
            args,
            arg_tokens,
            arg_single: words.single,
            arg_expand_in: &[],
            cmd_tok,
            scope_path,
            presubstituted_args: false,
        };
        let original = self.original_diagnostic_invocation(&site)?;
        let transitions = original
            .with_schema(crate::registry_invocation::source_schema_queries::state_transitions)?;
        let mut values = Vec::new();
        let mut tokens = Vec::new();
        for index in 0..original.words().arguments().len() {
            let word = original.word(index)?;
            tokens.push(*word.tokens().first()?);
            values.push(original.literal(index).unwrap_or("").to_owned());
        }
        Some((transitions, values, tokens))
    }

    fn apply_invocation_transitions(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        words: WordFacts<'_>,
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        if let Some((transitions, values, tokens)) = self.original_registry_state_transitions(
            cmd_name, args, arg_tokens, words, cmd_tok, scope_path,
        ) {
            self.apply_state_transitions(&transitions, &values, &tokens, scope_path);
        }
    }

    /// Note, in the per-item shell walk, a definer only the workspace's packs
    /// declare: the shell reads the un-overlaid store, where `cmd_name` has no
    /// definition-body grammar, while the walk's own generation (the overlaid
    /// one) gives it one. The per-item analysis then takes the full path
    /// ([`super::super::per_item::PerItemFallback::PackDefiner`]), so the class the
    /// definer makes is not lost.
    pub(super) fn note_pack_definer(&mut self, cmd_name: &str) {
        if self.defer_proc_bodies
            && self.pack_overlay != 0
            && self.definition_grammar(cmd_name).is_none()
            && self
                .analysis_context()
                .commands()
                .get(cmd_name)
                .is_some_and(|spec| spec.definition_body.is_some())
        {
            self.pack_definer_seen = true;
        }
    }

    /// [`Self::resolve_analyser_hook`] plus the traits and the clause plan of
    /// the concrete spec / subcommand the head resolved to — one resolution,
    /// every fact.
    ///
    /// A handler reached through hook dispatch must read its command's traits
    /// and clause structure from *this* resolution rather than re-fetching a
    /// spec by literal name: the name test puts per-command knowledge back in
    /// the analyser, and it silently diverges the moment a dialect variant (or
    /// a subcommand) shares the hook. Traits are composed
    /// `spec.traits | sub.traits`, matching
    /// [`tcl_registry::CommandRegistry::invocation_traits`].
    pub(in super::super) fn resolve_analyser_hook_call(
        &self,
        cmd_name: &str,
        args: &[String],
    ) -> Option<ResolvedAnalyserHook> {
        let generation = self.analysis_context();
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        // Select metadata under the complete retained authoring context.
        // This supplies neither head identity nor handler applicability.
        let resolved = tcl_registry::model::resolve_call_in_context(
            generation.commands(),
            Some(generation.context()),
            cmd_name,
            &arg_strs,
        )?;
        Some(ResolvedAnalyserHook {
            hook: resolved.analyser_hook?,
            traits: resolved.spec.traits
                | resolved
                    .sub
                    .map_or_else(tcl_registry::Traits::empty, |sub| sub.traits),
        })
    }

    /// The single typed `match` over the resolved [`AnalyserHookId`].
    ///
    /// Returns `true` when the command was consumed by an early-return
    /// family (the caller stops, skipping the shared tail), `false`
    /// when the walk should continue — either a void family ran, or no
    /// hook (and no definition-grammar definer) matched.
    ///
    /// `for`, `try`, `dict for`, `dict update`, `incr`, `append`, `lappend`,
    /// `upvar`, `namespace upvar`, `global` and `variable` carry no stamp at
    /// all: their only command-specific knowledge is a position or a
    /// keyword a descriptor states, so they take the
    /// "no stamped family" branch above and fall straight through to the
    /// shared tail below like any other command with no hook — the generic
    /// body walk reads when each body runs from its clause plan (`for`'s
    /// `start` once, `next` and the body per iteration; `try`'s handler
    /// bodies `Selected`), `apply_invocation_transitions` resolves a scope
    /// alias as the invocation's `VariableCellAliasTransition`, and
    /// `handle_var_binding_command` binds a loop or bound variable — with
    /// `lappend auto_path DIR…`'s record, the list append's
    /// `var_elements_effect` states — from its `LoopVarList` / `VarWrite`
    /// role, and the generic body walk binds the variable lists a clause
    /// fills (`try`'s handler variables, a slot the flat roles leave out).
    #[allow(
        clippy::too_many_lines,
        reason = "exhaustive registry-hook dispatch (one arm per AnalyserHookId \
                  variant, so a new variant is a compile error until wired); \
                  splitting hurts readability, per classify_side_effects"
    )]
    pub(super) fn dispatch_analyser_hook(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        words: WordFacts<'_>,
        cmd_tok: Token,
        scope_path: &[usize],
    ) -> bool {
        use tcl_registry::hooks::AnalyserHookId as Hook;
        let arg_single = words.single;
        if let Some(handled) = self.dispatch_original_class_definer(cmd_tok, scope_path) {
            return handled;
        }
        if let Some(handled) = self.dispatch_original_class_configuration(cmd_tok, scope_path) {
            return handled;
        }
        let Some(ResolvedAnalyserHook { hook, .. }) =
            self.resolve_analyser_hook_call(cmd_name, args)
        else {
            // No stamped family — the definition-grammar-driven definers
            // (TclOO metaclass create, snit::type/widget, itcl::class) get
            // their chance, then the tracked-interpreter-handle dispatch
            // (`sandbox eval { … }`, `cmd_name` never a registry name so it
            // can never reach a stamped hook the way literal `interp eval`
            // does).  Their registry/analysis-state conditions are disjoint
            // from every stamped hook and from each other (pinned by
            // tcl-registry's analyser-hook drift tests for the registry
            // ones), so running them only on the hookless path preserves the
            // dispatch order.
            self.note_pack_definer(cmd_name);
            return self.handle_oo_class_command(cmd_name, args, arg_tokens, scope_path, cmd_tok)
                || self.handle_snit_type_command(cmd_name, args, arg_tokens, scope_path)
                || self.handle_itcl_class_command(cmd_name, args, arg_tokens, scope_path)
                || self.handle_source_inventory_definition(
                    cmd_name, args, arg_tokens, scope_path, cmd_tok,
                )
                || self.handle_jim_class_command(cmd_name, args, arg_tokens, scope_path, cmd_tok)
                || self
                    .handle_jim_class_member_call(cmd_name, args, arg_tokens, scope_path, cmd_tok)
                || self.handle_interp_handle_eval_command_original(
                    cmd_name,
                    args,
                    arg_tokens,
                    scope_path,
                    Some(cmd_tok),
                );
        };
        match hook {
            // Early-return families: the handler owns the whole command
            // (including its body walk) when it returns `true`.
            Hook::Proc => self.handle_proc_command_at(
                cmd_name, args, arg_tokens, arg_single, scope_path, cmd_tok,
            ),
            Hook::OptProc => self.handle_opt_proc_command(args, arg_tokens, arg_single, scope_path),
            // `interp eval path { … }` — the child interpreter's script is
            // analysed in an isolated scope; a `{}`/multi-word/dynamic shape
            // falls through to the generic body walk in the current scope.
            Hook::InterpEval => self.handle_interp_eval_command_original(
                args,
                arg_tokens,
                scope_path,
                Some(cmd_tok),
            ),
            Hook::OoDefine => {
                self.handle_oo_define_command(cmd_name, args, arg_tokens, arg_single, scope_path)
            }
            Hook::NamespaceEval => self.handle_namespace_eval_command_at_invocation(
                args,
                arg_tokens,
                arg_single,
                scope_path,
                Some(cmd_tok.span.start()),
            ),
            // uplevel #0 { body } — opens a global-frame child scope so
            // the body's locals don't leak into the enclosing proc's
            // variable set.  Only the `#0` form is consumed; other
            // levels fall through to the generic body recursion.
            Hook::Uplevel => self.handle_uplevel_command(args, arg_tokens, scope_path),
            Hook::Foreach => {
                self.handle_var_binding_command(cmd_tok, arg_tokens, scope_path);
                self.handle_foreach_command_at(cmd_tok, cmd_name, args, arg_tokens, scope_path)
            }
            Hook::Switch => self.handle_switch_command(cmd_name, args, arg_tokens, scope_path),
            Hook::Catch => self.handle_catch_command(args, arg_tokens, scope_path),
            // apply {{params} body} — owns its body walk (binds params,
            // analyses element 1) so the generic `ArgRole::Body`
            // recursion never mis-reads the parameter list as a command.
            Hook::Apply => {
                let original = self.original_interp_apply_body(cmd_tok, arg_tokens, scope_path);
                self.handle_apply_command_with_source_namespace(args, arg_tokens, original)
            }

            // Void families: run the handler(s), then fall through to
            // the shared tail.
            Hook::InterpCreate => {
                if let Some((transitions, _, _)) = self.original_registry_state_transitions(
                    cmd_name, args, arg_tokens, words, cmd_tok, scope_path,
                ) {
                    if let Some(path) = self.handle_interp_create_command(&transitions) {
                        self.retain_interp_visibility_declaration(&path, cmd_tok, arg_tokens);
                    }
                } else {
                    // The binding owner refused this creation (an open glob
                    // import may rebind the head), so which children exist
                    // is unknowable and W140 must abstain rather than treat
                    // the child as never created.
                    self.dynamic_interp_ops = true;
                }
                false
            }
            Hook::InterpDelete => {
                self.handle_interp_delete_command_original(args, cmd_tok, arg_tokens);
                false
            }
            Hook::InterpHide => {
                self.handle_interp_hide_command_original(args, cmd_tok, arg_tokens);
                false
            }
            Hook::InterpExpose => {
                self.handle_interp_expose_command_original(args, cmd_tok, arg_tokens);
                false
            }
            Hook::DictWith => {
                self.handle_dict_with_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespaceEnsemble => {
                self.handle_namespace_ensemble(
                    args,
                    arg_tokens,
                    scope_path,
                    Some(cmd_tok.span.start()),
                );
                false
            }
            Hook::InterpAlias => {
                self.retain_interp_visibility_alias(cmd_tok, arg_tokens);
                self.handle_interp_alias_with_original_arguments(
                    args,
                    arg_tokens,
                    scope_path,
                    cmd_tok.span.start(),
                );
                false
            }
            Hook::OoObjdefine => {
                self.handle_oo_objdefine_at(cmd_tok, args, arg_tokens, arg_single, scope_path)
            }
            Hook::PackageRequire => {
                self.handle_package_require(cmd_name, cmd_tok, args, arg_tokens);
                false
            }
            Hook::PackageProvide => {
                self.handle_package_provide(cmd_tok, args, arg_tokens);
                false
            }
            Hook::PackageIfneeded => {
                self.handle_package_ifneeded(cmd_tok, args, arg_tokens);
                false
            }
            Hook::PackagePrefer => {
                self.handle_package_prefer(cmd_tok, args);
                false
            }
            Hook::ModulePathAdd => false,
            Hook::Source => {
                self.handle_source_command(cmd_tok, args, arg_tokens, arg_single, scope_path);
                false
            }
            Hook::NamespaceImport => {
                self.handle_namespace_import_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespaceExport => {
                self.handle_namespace_export_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespaceForget => {
                self.handle_namespace_forget_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespacePath => {
                self.handle_namespace_path_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespaceUnknown => {
                self.handle_namespace_unknown_command(args);
                false
            }
            Hook::RegexPatternCapture => {
                self.handle_regex_pattern_capture(cmd_name, args, arg_tokens, scope_path, cmd_tok);
                false
            }
            // ``load`` unconditionally flips ``has_dynamic_providers``:
            // it brings a shared library's commands into the interpreter
            // at runtime, which static W123 unknown-command analysis can
            // never predict.
            Hook::Load => {
                self.result.record_unresolved_provider_effect();
                false
            }
            // A *static* ``rename OLD NEW`` is recorded precisely by
            // ``handle_rename`` (``NEW`` resolves to whatever ``OLD``
            // denoted, including its arity) — only a genuinely *dynamic*
            // rename (``rename $x y`` / ``rename x [y]``) falls back to
            // the same conservative flag as ``load``, matching
            // ``command_binding.rs``'s wildcard-collapse convention for
            // the identical shape.
            Hook::Rename => {
                if self.handle_rename(
                    args,
                    arg_tokens,
                    arg_single,
                    scope_path,
                    cmd_tok.span.start(),
                ) {
                    self.result.record_unresolved_provider_effect();
                }
                false
            }
        }
    }
}
