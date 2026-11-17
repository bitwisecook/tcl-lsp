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

//! The body-role arguments of a command: the depths and event context each body runs in, clause variables and concatenated scripts.

use super::super::state::Analyser;
use std::sync::Arc;
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::ArgRole;

/// Which depths a body word raises while it is walked: `conditional_depth`
/// (branch-selected — nothing inside dominates the code after the command)
/// and `control_flow_body_depth` (not straight-line — it may run zero times
/// or many).
///
/// A body the call's clause plan places answers by its clause's timing: a
/// selected body is both, a per-iteration body or a loop's `next` fixture is
/// control flow, a protected body is a guarded probe, and a body that runs
/// exactly once whenever the call does (`for`'s `start`, `dict update`'s
/// body, `finally`) is neither. Any other body keeps the command's traits'
/// reading: `BRANCH_SELECTED_BODY` and `CONTROL_FLOW`.
fn body_depths(
    plan: Option<&tcl_registry::ClausePlan>,
    body: usize,
    traits: tcl_registry::Traits,
) -> (bool, bool) {
    use tcl_registry::{ClauseTiming, LoopPhase};
    let timing = plan.and_then(|plan| {
        plan.clauses
            .iter()
            .find(|clause| clause.operand(tcl_registry::arg_role::ArgRole::Body) == Some(body))
            .map(|clause| clause.timing)
    });
    match timing {
        Some(ClauseTiming::Selected) => (true, true),
        Some(ClauseTiming::PerIteration | ClauseTiming::LoopFixture(LoopPhase::Next)) => {
            (false, true)
        }
        Some(ClauseTiming::Protected) => (true, false),
        Some(ClauseTiming::Always | ClauseTiming::LoopFixture(LoopPhase::Init)) => (false, false),
        None => (
            traits.contains(tcl_registry::Traits::BRANCH_SELECTED_BODY),
            traits.contains(tcl_registry::Traits::CONTROL_FLOW),
        ),
    }
}

#[cfg(debug_assertions)]
fn trace_jim_body_entry(site: u32) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_JIM_SOURCE_OWNER").is_some() {
        eprintln!("JIM_SOURCE_OWNER stage=jim.body.dispatch_entry offset={site}");
    }
}

#[cfg(debug_assertions)]
fn trace_jim_body_token_fields(site: u32, tokens: Option<&crate::ir::CommandTokens>) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_JIM_SOURCE_OWNER").is_some() {
        eprintln!(
            "JIM_SOURCE_OWNER stage=jim.body.token_fields offset={site} tokens={} binding={}",
            tokens.is_some(),
            tokens.is_some_and(|tokens| tokens.source_binding.is_some()),
        );
    }
}

fn body_argument_indices(
    view: &crate::registry_invocation::InvocationBodyAssistance,
    logical: Option<&crate::registry_invocation::LogicalStructuredInvocation>,
) -> Vec<usize> {
    let mut indices: Vec<_> = view
        .possible_roles
        .iter()
        .filter_map(|&(index, role)| (role == ArgRole::Body).then_some(index))
        .collect();
    if let Some(source) = logical.and_then(
        crate::registry_invocation::LogicalStructuredInvocation::conditional_logical_body_indices,
    ) {
        for index in source {
            if !indices.contains(&index) {
                indices.push(index);
            }
        }
        indices.sort_unstable();
    }
    indices
}

fn is_dynamic_eval_body(view: &crate::registry_invocation::InvocationBodyAssistance) -> bool {
    view.definite_traits
        .contains(tcl_registry::Traits::DYNAMIC_EVAL_BODY)
        && view.definite_invocation.as_ref().is_some_and(|invocation| {
            tcl_registry::irules_policy::irules_disabled_class(&invocation.facts.canonical_command)
                .is_none()
        })
}

fn body_written_effective_index(
    words: Option<&crate::registry_invocation::source_structure::OriginalRegistryWords>,
    written: usize,
) -> Option<usize> {
    words?
        .origins()
        .iter()
        .position(|origin| {
            *origin == crate::registry_invocation::InvocationWordOrigin::Written(written + 1)
        })
        .and_then(|index| index.checked_sub(1))
}

impl Analyser {
    fn define_clause_loop_variables(
        &mut self,
        plan: &tcl_registry::clause_grammar::ClausePlan,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        scope_path: &[usize],
    ) {
        for clause in &plan.clauses {
            for index in clause.operands(ArgRole::LoopVarList) {
                let Some(text) = words
                    .arguments()
                    .get(index)
                    .and_then(|word| word.literal_bytes())
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                else {
                    continue;
                };
                let Some(token) = words
                    .operands()
                    .get(index)
                    .and_then(Option::as_ref)
                    .and_then(|operand| operand.word())
                    .and_then(|word| word.tokens().first())
                    .copied()
                else {
                    continue;
                };
                self.define_vars_from_list(text, token, scope_path);
            }
        }
    }

    fn prepare_body_clause_variables(
        &mut self,
        words: Option<&crate::registry_invocation::source_structure::OriginalRegistryWords>,
        generation: &tcl_registry::model::ContextRegistry,
        scope_path: &[usize],
    ) -> Option<tcl_registry::clause_grammar::ClausePlan> {
        let words = words?;
        let plan = words
            .with_source_schema(
                generation,
                crate::registry_invocation::source_schema_queries::clause_plan,
            )
            .flatten();
        if let Some(plan) = plan.as_ref() {
            self.define_clause_loop_variables(plan, words, scope_path);
        }
        plan
    }

    /// Generic body recursion via the command registry's
    /// `ArgRole::Body`, including `if`, `when`, `eval`, and every other
    /// registry-owned body. Sets event / conditional context and emits W105
    /// on the body argument before recursing, rather than on a nested
    /// re-segmentation.
    fn enter_body_depths(&mut self, conditional: bool, control_flow: bool) {
        if conditional {
            self.conditional_depth += 1;
        }
        if control_flow {
            self.control_flow_body_depth += 1;
        }
    }

    fn leave_body_depths(&mut self, conditional: bool, control_flow: bool) {
        if conditional {
            self.conditional_depth -= 1;
        }
        if control_flow {
            self.control_flow_body_depth -= 1;
        }
    }

    fn body_tokens_with_trace(
        &self,
        site: u32,
        arguments: &[Token],
    ) -> Option<Arc<crate::ir::CommandTokens>> {
        #[cfg(debug_assertions)]
        trace_jim_body_entry(site);
        let tokens = self.retained_invocation_tokens(site, arguments);
        #[cfg(debug_assertions)]
        trace_jim_body_token_fields(site, tokens.as_deref());
        crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.body.tokens",
            tokens,
        )
    }

    fn selected_body_source_structure(
        &self,
        generation: &tcl_registry::model::ContextRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<crate::registry_invocation::LogicalStructuredInvocation> {
        let registry = generation.commands();
        let metadata = match self.result.resolved_input.as_ref() {
            Some(input) if input.has_logical_source_name_context() => {
                if !std::ptr::eq(input.borrowed_context_registry(), generation) {
                    return None;
                }
                crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                    registry, input,
                )?
            }
            _ => generation.into(),
        };
        crate::registry_invocation::logical_structured_invocation_with_metadata_context(
            registry,
            metadata,
            tokens,
            Some(self.head_identities.source_bindings_ref()),
        )
    }

    pub(super) fn dispatch_body_arguments(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        invocation_offset: u32,
        scope_path: &[usize],
    ) {
        let generation = self.analysis_context();
        let registry = generation.commands();
        let Some(tokens) = self.body_tokens_with_trace(invocation_offset, arg_tokens) else {
            return;
        };
        if self.vendor_source_name_policy().is_some() {
            self.dispatch_vendor_body_arguments(&tokens, args, arg_tokens, arg_single, scope_path);
            return;
        }
        let view = crate::registry_invocation::invocation_body_assistance(
            registry,
            generation.as_ref(),
            &self.command_surface(registry),
            &tokens,
        );
        let logical = self.selected_body_source_structure(generation.as_ref(), &tokens);
        let logical_traits = logical.as_ref().map_or(
            view.definite_traits,
            super::super::super::registry_invocation::LogicalStructuredInvocation::traits,
        );
        let body_indices = body_argument_indices(&view, logical.as_ref());
        #[cfg(debug_assertions)]
        crate::registry_invocation::trace_jim_body_dispatch(
            invocation_offset,
            logical.as_ref(),
            &body_indices,
            arg_tokens,
        );
        if body_indices.is_empty() {
            return;
        }
        let dynamic_eval = is_dynamic_eval_body(&view);
        self.widen_irules_dynamic_eval_bindings(
            dynamic_eval,
            args,
            arg_tokens,
            &body_indices,
            scope_path,
        );
        // The `Tcl_ConcatObj` eval family: when the spec carries
        // `SCRIPT_CONCATENATES_ARGS` and words follow the first body index,
        // the script is the *concatenation* of every trailing word, not the
        // first one on its own.  Walking only the first would analyse
        // `eval set l2 hello` as the one-word script `set` — a false E002
        // plus a lost write to `l2` that then draws a false W210.
        if body_indices.first().is_some_and(|&first| {
            first + 1 < args.len()
                && logical_traits.contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
        }) {
            self.dispatch_concatenated_script(
                cmd_name,
                args,
                arg_tokens,
                arg_single,
                body_indices[0],
                scope_path,
            );
            return;
        }
        let body_scope = self.selected_body_scope(registry, &view, logical.as_ref(), args);
        let (_, entered_event, prev_event) =
            self.enter_body_event_context(cmd_name, &view, args, arg_tokens, arg_single);
        let source_words =
            crate::registry_invocation::source_structure::original_registry_words_for_tokens(
                &self.source,
                &self.result,
                &tokens,
            );
        let plan = self.prepare_body_clause_variables(
            source_words.as_ref(),
            generation.as_ref(),
            scope_path,
        );
        for idx in body_indices {
            if let (Some(body_text), Some(body_tok)) = (args.get(idx), arg_tokens.get(idx).copied())
            {
                self.retain_body_event_declaration(entered_event, tokens.argv.first(), body_tok);
                let effective = body_written_effective_index(source_words.as_ref(), idx);
                let (conditional, control_flow) = body_depths(
                    effective.and(plan.as_ref()),
                    effective.unwrap_or(idx),
                    logical_traits,
                );
                self.enter_body_depths(conditional, control_flow);
                let is_single_token = arg_single.get(idx).copied().unwrap_or(false);
                self.dispatch_one_body_argument(
                    cmd_name,
                    body_text,
                    body_tok,
                    is_single_token,
                    scope_path,
                    body_scope,
                );
                self.leave_body_depths(conditional, control_flow);
            }
        }
        if entered_event {
            self.current_event = prev_event;
        }
    }

    fn retain_body_event_declaration(
        &mut self,
        entered_event: bool,
        declaration: Option<&Span>,
        body_tok: Token,
    ) {
        if entered_event && let Some(declaration) = declaration {
            self.record_original_vendor_variable_body(
                *declaration,
                body_tok.span,
                None,
                crate::signature_scan::vendor_variable::VendorSourceVariableBodyKind::Event,
            );
        }
    }

    fn selected_body_scope(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
        view: &crate::registry_invocation::InvocationBodyAssistance,
        logical: Option<&crate::registry_invocation::LogicalStructuredInvocation>,
        args: &[String],
    ) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
        let logical_roles = logical.map(
            super::super::super::registry_invocation::LogicalStructuredInvocation::written_roles,
        );
        let logical_scope = logical
            .and_then(|selected| selected.body_scope(registry, self.analysis_context().as_ref()));
        self.record_scoped_sibling_definition(
            logical_scope.or(view.definite_scope),
            logical_roles.as_deref().unwrap_or(&view.definite_roles),
            args,
        )
    }

    /// Walk selected authored body positions independently of executable
    /// invocation admission. The retained body owns source advice only.
    fn dispatch_vendor_body_arguments(
        &mut self,
        tokens: &crate::ir::CommandTokens,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) {
        let image = self.command_document_source_image();
        let config = self.file_lexer_config();
        let Some(head) = tokens.words().first().and_then(|word| {
            self.result
                .original_vendor_source_name_in_source(&image, config, word.source().span)
        }) else {
            return;
        };
        let Some(metadata) = self.original_vendor_variable_metadata(head) else {
            return;
        };
        if !metadata.roles_complete()
            || metadata.original_words().len() != args.len() + 1
            || args.len() != arg_tokens.len()
            || metadata
                .original_words()
                .iter()
                .any(|word| word.group().expand)
        {
            return;
        }
        let bodies: Option<Vec<_>> = metadata
            .roles()
            .iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::Body)
            .map(|(argument, _)| {
                metadata
                    .argument_offset()
                    .checked_add(usize::from(*argument))
            })
            .collect();
        let Some(bodies) = bodies else {
            return;
        };
        if bodies.iter().any(|argument| *argument >= args.len()) {
            return;
        }
        let traits = metadata.possible_traits();
        let command = metadata.command();
        if bodies.first().is_some_and(|&first| first + 1 < args.len())
            && traits.contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
        {
            self.dispatch_concatenated_script(
                command, args, arg_tokens, arg_single, bodies[0], scope_path,
            );
            return;
        }
        let (_, event, previous_event) =
            self.enter_registry_event_context(command, traits, args, arg_tokens, arg_single);
        let conditional = traits.contains(tcl_registry::Traits::BRANCH_SELECTED_BODY);
        let control_flow = traits.contains(tcl_registry::Traits::CONTROL_FLOW);
        if conditional {
            self.conditional_depth += 1;
        }
        if control_flow {
            self.control_flow_body_depth += 1;
        }
        for argument in bodies {
            let body = arg_tokens[argument];
            if event && let Some(declaration) = tokens.argv.first() {
                self.record_original_vendor_variable_body(
                    *declaration,
                    body.span,
                    None,
                    crate::signature_scan::vendor_variable::VendorSourceVariableBodyKind::Event,
                );
            }
            self.dispatch_one_body_argument(
                command,
                &args[argument],
                body,
                arg_single.get(argument).copied().unwrap_or(false),
                scope_path,
                None,
            );
        }
        if conditional {
            self.conditional_depth -= 1;
        }
        if control_flow {
            self.control_flow_body_depth -= 1;
        }
        if event {
            self.current_event = previous_event;
        }
    }

    fn enter_body_event_context(
        &mut self,
        cmd_name: &str,
        view: &crate::registry_invocation::InvocationBodyAssistance,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) -> (bool, bool, Option<String>) {
        let spec_traits = view.definite_traits;
        // iRules event handlers are a file-level declaration surface.  A
        // handler-shaped command reached while already walking any body is
        // invalid (IRULE5006) and must not manufacture or replace event
        // context.  In particular, a nested handler inside an event retains
        // the outer event's context, while one inside a proc retains `None`.
        let direct_event_layout = view.definite_invocation.as_ref().filter(|invocation| {
            invocation.arguments.len() == args.len()
                && (0..args.len())
                    .all(|index| invocation.effective.written_argument(index) == Some(index))
        });
        self.enter_registry_event_context(
            direct_event_layout.map_or(cmd_name, |invocation| {
                invocation.facts.canonical_command.as_str()
            }),
            if direct_event_layout.is_some() {
                spec_traits
            } else {
                tcl_registry::Traits::empty()
            },
            args,
            arg_tokens,
            arg_single,
        )
    }

    /// Scoped definitions belong to a selected logical body contract and an
    /// original written name operand. The scoped analysis inventory grants no
    /// actual command-table publication, entered activation or outward write.
    fn record_scoped_sibling_definition(
        &mut self,
        body_scope: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
        roles: &[(usize, ArgRole)],
        args: &[String],
    ) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
        let sibling_name_idx = body_scope
            .filter(|env| env.include_sibling_definitions)
            .and_then(|_| {
                roles
                    .iter()
                    .find_map(|&(index, role)| (role == ArgRole::Name).then_some(index))
            });
        if let (Some(env), Some(ni)) = (body_scope, sibling_name_idx)
            && let Some(name) = args.get(ni)
            && !name.is_empty()
        {
            self.result
                .scoped_sibling_defs
                .entry(env.name)
                .or_default()
                .insert(name.clone());
        }
        body_scope
    }

    /// **Measurements §4c** — the iRules dynamic-code widening. The rule
    /// compiler's load-time bans are *lexical*: they scan braced script
    /// literals (so `eval {proc …}` is rejected like bare `proc` — the
    /// recursion above handles that), but script text held in a variable
    /// escapes the scan entirely. A dynamic `eval`/`uplevel` (any level
    /// spelling) whose script head the analyser cannot read therefore
    /// makes every literal-surface check abstain AND widens the realm
    /// state: the hidden script may define procs, and a runtime-defined
    /// proc is a **persistent per-TMM global** — created once (typically
    /// in `RULE_INIT`), surviving across events, connections, and separate
    /// requests on the same TMM (§4c: `persist=YES`, identical on a
    /// second request). That is exactly the "runtime set of commands is
    /// unknowable" fact the existing dynamic-provider machinery models
    /// ([`super::super::types::AnalysisResult::has_dynamic_providers`] — the
    /// oracle's `CommandDomainWidening::DynamicProviders`, under which
    /// every head answers `BindingKnowledge::Unknown`), so it is wired
    /// through that flag rather than new state: W123/W120-class
    /// unresolved-name conclusions abstain for the rest of the document.
    ///
    /// Scope: iRules only, and only at top-level-or-event scope — the
    /// scopes §4c measured; the load-time *lexical* rejections themselves
    /// (a literal `proc` in a `when` body, a literal call head) are NOT
    /// softened, because the rule compiler cannot see runtime definitions
    /// either. Commands in the §4b disabled split never widen: an
    /// interpreter-absent head does not run, and a compiler-refused head
    /// never loads.
    fn widen_irules_dynamic_eval_bindings(
        &mut self,
        dynamic_eval: bool,
        args: &[String],
        arg_tokens: &[Token],
        body_indices: &[usize],
        scope_path: &[usize],
    ) {
        if self.result.has_dynamic_providers || !self.profile.is_irules() {
            return;
        }
        if !dynamic_eval {
            return;
        }
        // The script *head* is hidden exactly when the first script word is
        // a substitution (`$s`, `[build]`) or interpolates one; a bare
        // literal head (`uplevel 1 helper`) is still resolved at load time.
        let hidden_head = body_indices.first().is_some_and(|&first| {
            arg_tokens.get(first).is_some_and(|tok| {
                matches!(tok.kind, TokenType::Var | TokenType::Cmd)
                    || args.get(first).is_some_and(|text| {
                        super::super::diagnostics::helpers::has_substitution(text.trim(), tok)
                    })
            })
        });
        if !hidden_head {
            return;
        }
        if matches!(
            self.irules_execution_context(scope_path),
            tcl_registry::events::IrulesExecutionContext::TopLevel
                | tcl_registry::events::IrulesExecutionContext::EventBody
        ) {
            self.result.has_dynamic_providers = true;
        }
    }

    fn enter_event_context(
        &mut self,
        enters_event_context: bool,
        args: &[String],
    ) -> (bool, Option<String>) {
        if !enters_event_context {
            return (false, None);
        }
        let previous = self.current_event.clone();
        self.current_event = args.first().cloned();
        (true, previous)
    }

    fn enter_registry_event_context(
        &mut self,
        command: &str,
        traits: tcl_registry::Traits,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) -> (bool, bool, Option<String>) {
        let is_event_handler = traits.contains(tcl_registry::Traits::IS_EVENT_HANDLER);
        let valid = self.irules_event_body_is_valid(
            command,
            is_event_handler,
            args,
            arg_tokens,
            arg_single,
        );
        let (entered, previous) =
            self.enter_event_context(is_event_handler && self.body_depth == 0 && valid, args);
        (valid, entered, previous)
    }

    fn irules_event_body_is_valid(
        &self,
        command: &str,
        is_event_handler: bool,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) -> bool {
        if !self.profile.is_irules() || !is_event_handler {
            return true;
        }
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        let generation = self.analysis_context();
        let registry = generation.commands();
        let closed = tcl_registry::events::closed_braced_argument_words(
            &self.source,
            arg_tokens,
            arg_single,
        );
        let declaration = closed
            .as_deref()
            .and_then(|closed| {
                tcl_registry::events::IrulesDeclarationArguments::new(
                    &words, arg_tokens, arg_single, closed,
                )
            })
            .and_then(|arguments| {
                registry.irules_top_level_declaration(
                    command,
                    arguments,
                    &tcl_registry::events::EventRegistry::build(),
                )
            });
        self.body_depth == 0
            && matches!(
                declaration,
                Some(tcl_registry::events::IrulesTopLevelDeclaration::Event { body_index, .. })
                    if arg_tokens.get(body_index).is_some_and(|token| token.kind == TokenType::Str)
            )
    }

    /// Analyse the script a [`tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS`]
    /// command actually evaluates: the `Tcl_ConcatObj` join of every word from
    /// `first` (its first `ArgRole::Body` index) to the end of the call.
    ///
    /// Two outcomes, both sound:
    ///
    /// - **Every trailing word is static script text** — walk the tail as
    ///   one script via [`super::super::utils::concat_script_window`], which
    ///   rebuilds it *in place* (content bytes at their true source offsets,
    ///   delimiters and gaps blanked to spaces) so every span the walk
    ///   records — command references, variable reads and writes,
    ///   diagnostics — lands on the exact bytes it describes, rename-safe.
    ///   `eval set l2 hello` is analysed as `set l2 hello`, so `l2` is
    ///   recorded as written and no arity error is invented.
    /// - **Any word is dynamic** (or its bytes cannot be mapped) — consume
    ///   the command without walking, the same answer
    ///   `handle_interp_eval_command` gives a multi-word `interp eval`:
    ///   substitution happens before concatenation, so the real script is
    ///   unknowable and every definedness or arity fact derived from the
    ///   written words would be a guess.
    fn dispatch_concatenated_script(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        first: usize,
        scope_path: &[usize],
    ) {
        let (Some(words), Some(tokens)) = (args.get(first..), arg_tokens.get(first..)) else {
            return;
        };
        let Some(first_tok) = tokens.first() else {
            return;
        };
        // W105 still belongs to the written first word (`eval set …` is an
        // unbraced body however the tail concatenates), so fire it before the
        // shape decision — with that word's own single-token flag, so a bare
        // `eval $cmd arg` stays exempt exactly as `eval $cmd` is.
        self.emit_w105_unbraced_body(
            cmd_name,
            &words[0],
            *first_tok,
            arg_single.get(first).copied().unwrap_or(false),
        );
        let Some((script, span)) =
            super::super::utils::concat_script_window(words, tokens, &self.source)
        else {
            // The tail cannot be walked (a dynamic word). A braced first word
            // is still a literal script prefix — concatenation appends after
            // it, so its own commands run as written — and walking it keeps
            // every scope/definition it declares visible to the editor. The
            // `eval {script} $extra` shape (and a brace-mangled document
            // whose mis-extended body word drags real code into this arm)
            // must not lose the script to the decline.
            if first_tok.kind == TokenType::Str {
                self.analyse_concatenated_script(&words[0], *first_tok, scope_path);
                return;
            }
            // Declining the walk must not lose the *reference* a `$cmd` script
            // word carries — `uplevel #0 $cmd [list x y]` still dispatches
            // whatever `$cmd` holds.
            self.record_var_body_const_dispatch(*first_tok, scope_path);
            return;
        };
        // `analyse_body` walks only a `Str` (braced) body and anchors at
        // `span.start() + content_offset`; the window's text starts at its
        // span's own first byte, so the anchor carries no delimiter to skip.
        let anchor = Token::new(TokenType::Str, span);
        self.analyse_concatenated_script(&script, anchor, scope_path);
        self.concatenated_windows.push((script, span));
    }

    /// Walk one eval-family script that concatenation builds, marking its
    /// commands as having no lowered source of their own.
    pub(in crate::analyser) fn analyse_concatenated_script(
        &mut self,
        script: &str,
        anchor: Token,
        scope_path: &[usize],
    ) {
        let outer = std::mem::replace(&mut self.concatenated_script, true);
        self.analyse_body(script, anchor, scope_path);
        self.concatenated_script = outer;
    }

    /// Analyse a single body-role argument: fires `W105`, walks the script
    /// (through a scoped command environment when `body_scope` names one),
    /// and — for a bareword (unbraced) body — also dispatches it as a
    /// zero-arg command call. Split out of [`Self::dispatch_body_arguments`]
    /// purely to keep that function under the line-count lint; the two
    /// always run together, once per body-role index.
    fn dispatch_one_body_argument(
        &mut self,
        cmd_name: &str,
        body_text: &str,
        body_tok: Token,
        is_single_token: bool,
        scope_path: &[usize],
        body_scope: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
    ) {
        self.emit_w105_unbraced_body(cmd_name, body_text, body_tok, is_single_token);
        // A bareword body (`if {$cond} mymod::foo`, `uplevel 1
        // mymod::qux`) is a single, statically-known zero-arg command
        // call — a legitimate alternative form real Tcl accepts
        // identically to a braced block (the exact shape
        // `emit_w105_unbraced_body` above already exempts from its
        // own warning). `analyse_body` below only ever recurses into
        // a `Str` (braced) body, so without this such a call is
        // invisible to `command_invocations` entirely: found by
        // hover/definition (which resolve independently off the
        // cursor token) but missed by references/rename — silently
        // producing an incomplete rename that breaks the program at
        // the missed call site.
        // Dispatched through the ordinary `process_command` path
        // (not a hand-rolled invocation record) so it gets full
        // treatment: arity checking, W123, nested diagnostics —
        // everything a real call site deserves.
        if body_tok.kind == TokenType::Esc
            && !body_text.trim().contains(char::is_whitespace)
            && !super::super::diagnostics::helpers::has_substitution(body_text.trim(), &body_tok)
        {
            self.process_command(
                &[body_text.trim().to_string()],
                &[body_tok],
                &[true],
                &[false],
                scope_path,
            );
        }
        self.record_var_body_const_dispatch(body_tok, scope_path);
        // When the body runs in a scoped command environment, record its
        // region (so the post-walk W123 pass and the LSP providers can
        // resolve the scoped heads by position) and push the environment
        // so the in-walk arity / subcommand checks resolve them too.
        if let Some(env) = body_scope {
            let start = body_tok.span.start() + u32::from(body_tok.content_offset);
            self.result
                .scoped_command_regions
                .push(super::super::types::ScopedBodyRegion {
                    span: tcl_lexer::Span::new(start, body_tok.span.end()),
                    env,
                });
            self.body_scope_stack.push(env);
            self.analyse_body(body_text, body_tok, scope_path);
            self.body_scope_stack.pop();
        } else {
            self.analyse_body(body_text, body_tok, scope_path);
        }
    }

    /// Record a bare `$var` script word (`eval $cmd`, `uplevel #0 $cmd …`) as
    /// a pending const-dispatch site: the variable's value is the command
    /// prefix actually dispatched, exactly the "value is a command prefix"
    /// shape `{*}$cmd` gets via `head_expanded`.
    ///
    /// `analyse_body` only ever recurses a literal `Str` body, so without
    /// this the site is invisible to `command_invocations` — found by
    /// hover/definition (which resolve independently off the cursor token)
    /// but missed by references/rename. Recorded the same way
    /// `record_var_or_cmd_command_site` records a command's own `$cmd`-headed
    /// dispatch, for `settle_const_dispatches` to resolve in the CFG/SSA
    /// phase.
    ///
    /// Shared by the ordinary body walk and the
    /// [`tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS`] decline path: a
    /// multi-word `uplevel #0 $cmd [list x y]` yields no analysable script,
    /// but `$cmd` is still a real dispatch whose reference must be recorded.
    ///
    /// The complete actual variable component and shared configured literal
    /// name owner decide eligibility. Produced array indexes remain unavailable
    /// rather than acquiring their root's different value or source identity.
    pub(super) fn record_var_body_const_dispatch(&mut self, body_tok: Token, scope_path: &[usize]) {
        let Some(word) = self.original_whole_variable_source_word(body_tok, false) else {
            return;
        };
        let Some(content) = word
            .content_span()
            .ok()
            .and_then(|span| word.image().bytes().get(span.as_range()))
        else {
            return;
        };
        let Some(var_name) = std::str::from_utf8(content)
            .ok()
            .and_then(|source| {
                tcl_syntax::naming::whole_variable_reference_literal_name(source, word.config())
            })
            .map(str::to_owned)
        else {
            return;
        };
        let ns = self.command_resolution_namespace(scope_path);
        self.pending_const_dispatches
            .push(super::super::state::ConstDispatchSite {
                var_name,
                span: body_tok.span,
                ns,
                head_expanded: true,
            });
    }
}
