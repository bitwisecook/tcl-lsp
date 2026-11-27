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

//! The default lowering of a command to a call: argument roles, uninitialised-target safety and the reads hidden in a value word.

use super::super::{Lowerer, source_body_syntax};
use crate::expr_parser::parse_expr_for_profile_with_lexer_config;
use crate::ir::{CommandTokens, Statement};
use crate::lowering_hooks::extract_proved_expr_arg;
use tcl_dialect::model::surface_admits;
use tcl_lexer::TokenType;
use tcl_registry::ArgRole;
use tcl_registry::hooks::LoweringHookId;
use tcl_source::segment::SegmentedCommand;

impl Lowerer<'_> {
    /// Whether this concrete invocation is declared to initialise an unset
    /// target under the lowerer's active dialect profile.
    ///
    /// This is registry-driven: command form and Tcl release decide the
    /// result, not a consumer-side command-name or read-modify-write rule.
    pub(in super::super) fn safe_on_uninit(&self, command: &str, args: &[String]) -> bool {
        let Some(context) = self
            .invocation_metadata_context()
            .filter(|context| context.matches_registry(self.registry))
        else {
            return false;
        };
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let dialect = Some(context.context().authoring_query());
        self.registry
            .resolve_invocation(command, &arg_refs, dialect)
            .is_some_and(|resolved| {
                resolved
                    .semantics
                    .safe_on_uninit
                    .is_none_or(|allowed| surface_admits(allowed, dialect.as_ref()))
            })
    }
}

impl Lowerer<'_> {
    /// Registry-resolved reads hidden inside the value word of a generic
    /// value-passthrough store.
    ///
    /// An alias/rename of `set` must remain a runtime [`Statement::Call`], but
    /// its one value word still has the same expression semantics as a direct
    /// `set`.  Capture those reads while the lowering alias map and resolved
    /// dialect context are live, rather than asking SSA to reconstruct command
    /// bindings from a flattened call later.
    fn nested_expr_reads_in_set_value(
        &self,
        seg: &SegmentedCommand,
        namespace: &str,
        resolved_command: &str,
        prepend_n: usize,
    ) -> Vec<String> {
        if prepend_n != 0
            || seg.args().len() != 2
            || !matches!(
                seg.argv.get(2).map(|token| token.kind),
                Some(TokenType::Cmd)
            )
            || !seg.single_token_word.get(2).copied().unwrap_or(false)
            || self
                .registry
                .get(resolved_command)
                .and_then(|spec| spec.lowering_hook)
                != Some(LoweringHookId::Set)
        {
            return Vec::new();
        }

        let value = &seg.args()[1];
        let inner = value
            .strip_prefix('[')
            .and_then(|text| text.strip_suffix(']'))
            .unwrap_or(value);
        let context = self
            .dialect_context
            .as_deref()
            .map(tcl_registry::model::ContextRegistry::context);
        let tokens = self.cmd_tokens(seg);
        let Some((_, _, expression, _)) = extract_proved_expr_arg(
            (Some(&tokens), 2),
            inner,
            &self.aliases,
            namespace,
            self.registry,
            context,
            self.config,
        ) else {
            return Vec::new();
        };
        parse_expr_for_profile_with_lexer_config(&expression, self.dialect, self.config)
            .vars_element_qualified_with_config(self.config)
            .into_iter()
            .collect()
    }

    /// Default lowering: generic [`Statement::Call`] with registry-based
    /// arg roles.
    pub(in super::super) fn lower_default(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Statement {
        let tokens = self.attach_evaluated_body(seg, namespace, self.cmd_tokens_boxed(seg));
        // Only the evaluated-body attachment recurses; the rest of the
        // statement is built in a non-inlined leaf, so its locals are not
        // held in the frame of every enclosing body level.
        self.lower_default_with_tokens(seg, namespace, tokens)
    }

    // The generic path deliberately assembles every registry-owned call fact
    // in one place so consumers never need per-command recovery logic.
    #[allow(clippy::too_many_lines)]
    #[inline(never)]
    /// Whether the document declares `command` with a word this call runs as
    /// code: a body, a lambda or a command prefix.
    fn declares_executable_word(&self, command: &str, args: &[String]) -> bool {
        let Some(declared) = self
            .source_declared_commands
            .as_ref()
            .or(self.declared_commands)
        else {
            return false;
        };
        let Some(command) =
            tcl_registry::model::DocumentCommandSurface::new(self.registry, Some(declared))
                .declared_command(command)
        else {
            return false;
        };
        [
            ArgRole::Body,
            ArgRole::LambdaLiteral,
            ArgRole::CommandPrefix,
        ]
        .into_iter()
        .any(|role| !command.arg_indices_for_role(role, args.len()).is_empty())
    }

    fn tokens_realm(&self, tokens: &CommandTokens) -> tcl_dialect::model::InvocationRealm {
        tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
            .unwrap_or(self.invocation_realm)
    }

    fn lower_default_with_tokens(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: Box<CommandTokens>,
    ) -> Statement {
        let tokens = *tokens;
        let cmd_name = seg.name();
        let args = seg.args();
        let Some(target) = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.proved_execution_target())
            .filter(|target| target.registry_backed)
        else {
            // A document-declared command with a word it runs as code lowers
            // to the barrier a registry body command lowers to, whatever the
            // frame effect it states (#2533).
            if self.declares_executable_word(cmd_name, args) {
                return body_barrier(seg, None, tokens);
            }
            return source_body_syntax::generic_call(seg.span, cmd_name, args, Some(tokens));
        };
        let Some(role_cmd) = self.retained_metadata_descriptor_label(&tokens) else {
            return untyped_call(seg, None, tokens);
        };
        let prepend_n = target.prepended.len();
        let Some(prefix) = literal_words(&target.prepended) else {
            // A retained alias operand still contributes one argv slot when
            // its value is unknown. Presentation text must never license
            // specialised lowering or move the remaining operand positions.
            return untyped_call(seg, Some(role_cmd), tokens);
        };
        let role_args: Vec<String> = prefix.into_iter().chain(args.iter().cloned()).collect();
        let canonical = (role_cmd != cmd_name).then(|| role_cmd.clone());

        let context = self.invocation_metadata_context();
        let invocation = context.and_then(|context| {
            crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                self.registry,
                Some(context),
                &tokens,
            )
        });
        let facts = invocation.as_ref().map(|invocation| &*invocation.facts);
        let realm = self.tokens_realm(&tokens);
        let RoleIndices {
            executable: executable_indices,
            var_writes: var_indices,
            var_reads: var_read_indices,
            reads_before_write,
        } = RoleIndices::of(facts);

        // A stored callback is data for this invocation, not executable code
        // that can complete or mutate the caller before the command returns.
        // Every live Body, LambdaLiteral, or CommandPrefix whose exact timing
        // is SameInvocation needs the opaque runtime barrier. This is checked
        // per index because one invocation may mix immediate and deferred code.
        let has_same_invocation_executable = executable_indices.iter().any(|&index| {
            invocation
                .as_ref()
                .and_then(|invocation| {
                    invocation.with_metadata_schema(self.registry, context?, realm, |selected| {
                        selected.authored_source_script_timing_at(index)
                    })
                })
                .is_none_or(|timing| timing == tcl_registry::ScriptTiming::SameInvocation)
        });
        if has_same_invocation_executable && tokens.evaluated_body().is_none() {
            return body_barrier(seg, canonical, tokens);
        }

        if !var_indices.is_empty() || !var_read_indices.is_empty() {
            // A brace-quoted name word (`set {$n} 1`, `unset {$n}`) is Tcl's
            // literal spelling for a name that contains `$` / `[`: it
            // substitutes nothing, so the word's content **is** the variable
            // name. The de-braced `args` text cannot show that;
            // the word's own token kind can.
            let variable_at = |index: usize| -> Option<String> {
                let invocation = invocation.as_ref()?;
                let effective = &invocation.effective;
                let word = effective.words.get(index + 1)?;
                let value = invocation.argument_literal(index)?;
                let braced = matches!(word, tcl_source::words::WordExpr::BracedLiteral { .. })
                    || effective.written_argument(index).is_none();
                let name = crate::naming::element_var_name_braced(&value, braced);
                if invocation.with_metadata_schema(self.registry, context?, realm, |selected| {
                    selected.authored_source_option_variable_scope_at(index)
                }) == Some(tcl_registry::VariableScope::Global)
                    && !tcl_syntax::naming::is_absolute(name)
                {
                    Some(tcl_syntax::naming::root_unrooted_key(name))
                } else {
                    Some(name.to_owned())
                }
            };
            let var_defs: Vec<String> = var_indices
                .iter()
                .filter_map(|&index| variable_at(index))
                .collect();
            let mut var_reads: Vec<String> = var_read_indices
                .iter()
                .filter_map(|&index| variable_at(index))
                .collect();
            let nested_expr_reads =
                self.nested_expr_reads_in_set_value(seg, namespace, &role_cmd, prepend_n);
            var_reads.extend(nested_expr_reads);
            let reads_own_defs = reads_before_write
                || var_reads
                    .iter()
                    .any(|read| var_defs.iter().any(|definition| definition == read));
            return Statement::Call {
                span: seg.span,
                command: cmd_name.into(),
                canonical_command: canonical.clone(),
                args: args.to_vec(),
                defs: var_defs,
                reads: var_reads,
                reads_own_defs,
                safe_on_uninit: self.safe_on_uninit(&role_cmd, &role_args),
                tokens: Some(tokens.clone()),
                foreach_groups: None,
            };
        }

        untyped_call(seg, canonical, tokens)
    }
}

fn untyped_call(
    seg: &SegmentedCommand,
    canonical_command: Option<String>,
    tokens: CommandTokens,
) -> Statement {
    Statement::Call {
        span: seg.span,
        command: seg.name().into(),
        canonical_command,
        args: seg.args().to_vec(),
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: Some(tokens),
        foreach_groups: None,
    }
}

fn body_barrier(
    seg: &SegmentedCommand,
    canonical_command: Option<String>,
    tokens: CommandTokens,
) -> Statement {
    Statement::Barrier {
        span: seg.span,
        reason: Statement::OPAQUE_SCRIPT_BODY.into(),
        command: seg.name().into(),
        canonical_command,
        args: seg.args().to_vec(),
        tokens: Some(tokens),
    }
}

fn literal_words(
    words: &[crate::registry_invocation::EffectiveInvocationWord],
) -> Option<Vec<String>> {
    words
        .iter()
        .map(|word| match word {
            crate::registry_invocation::EffectiveInvocationWord::Literal(value) => {
                Some(value.clone())
            }
            _ => None,
        })
        .collect()
}

struct RoleIndices {
    executable: Vec<usize>,
    var_writes: Vec<usize>,
    var_reads: Vec<usize>,
    reads_before_write: bool,
}

impl RoleIndices {
    pub(in super::super) fn of(
        facts: Option<&tcl_registry::resolved_invocation::InvocationFacts>,
    ) -> Self {
        let Some(facts) = facts else {
            return Self {
                executable: Vec::new(),
                var_writes: Vec::new(),
                var_reads: Vec::new(),
                reads_before_write: false,
            };
        };
        let indices = |role| -> Vec<usize> {
            facts
                .arg_roles
                .iter()
                .filter_map(|&(index, found)| {
                    (found == role).then_some(facts.argument_offset + usize::from(index))
                })
                .collect()
        };
        let mut executable: Vec<usize> = [
            ArgRole::Body,
            ArgRole::LambdaLiteral,
            ArgRole::CommandPrefix,
        ]
        .into_iter()
        .flat_map(indices)
        .collect();
        executable.sort_unstable();
        executable.dedup();
        // Incomplete dynamic option/argument resolution cannot establish a
        // strong definition. The shared place owner retains the uncertainty.
        let var_writes = if !facts.arg_roles_complete
            || facts.frame_effect.is_some_and(|effect| {
                effect.layout == tcl_registry::frame_effect::FrameArgLayout::AliasPairs
            }) {
            Vec::new()
        } else {
            indices(ArgRole::VarWrite)
        };
        Self {
            executable,
            var_writes,
            var_reads: indices(ArgRole::VarRead),
            reads_before_write: facts
                .traits
                .contains(tcl_registry::Traits::READS_BEFORE_WRITE),
        }
    }
}
