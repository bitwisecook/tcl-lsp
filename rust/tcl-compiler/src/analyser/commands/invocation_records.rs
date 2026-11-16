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

//! The invocation one command records: its site facts, retained tokens and the iRules `call` target.

use super::super::state::Analyser;
use super::{DispatchSite, VarOrCmdSite, WorkspaceCommandPhases};
use tcl_lexer::{Token, TokenType};

impl Analyser {
    pub(super) fn record_command_site_facts(
        &mut self,
        site: &DispatchSite<'_>,
        arg_tokens_in: &[Token],
        report_phase: &impl Fn(&str),
        workspace_phase: Option<&WorkspaceCommandPhases>,
    ) {
        let DispatchSite {
            cmd_name,
            args,
            arg_tokens,
            arg_single: _,
            arg_expand_in,
            cmd_tok,
            scope_path,
            presubstituted_args,
        } = *site;
        // Walk every argument's source slice for ``[cmd ...]``
        // substitutions and record each nested head as its own
        // ``CommandInvocation``.
        if !presubstituted_args {
            self.record_nested_invocations_from_args(arg_tokens_in, scope_path);
        }

        report_phase("nested-invocations");
        // Record `ArgRole::CommandPrefix` callback heads (`lsort -command
        // myCompare`, `trace add … cb`) as command invocations too, so
        // find-references / rename / call-hierarchy / code-lens / W123 /
        // callback-arity see the callback exactly like a direct call.
        self.record_command_prefix_invocations(cmd_tok.span.start(), arg_tokens);

        report_phase("callback-invocations");
        // Record `ArgRole::CommandName` arguments (`info body PROC`,
        // `namespace which -command NAME`) — a bare command name held as
        // data — as command invocations too, so the named command is
        // reached by find-references / go-to-definition / rename without
        // any arity check (it is introspected, not called).
        self.record_command_name_invocations(args, arg_tokens, scope_path, cmd_tok.span.start());

        report_phase("command-name-invocations");
        // The occurrence tables the registry's *argument roles* produce —
        // namespace names and computed variable names.
        self.record_arg_role_facts(
            args,
            arg_tokens,
            scope_path,
            cmd_tok.span.start(),
            presubstituted_args,
        );

        report_phase("namespace-role-facts");
        // Run the per-command syntactic checks on commands nested inside
        // ``[…]`` substitutions (bare-`Cmd` args *and* braced-expr args)
        // — the main walk never descends a substitution (it treats
        // `[cmd …]` as a value), so a command like `set fh [open "|$cmd"
        // r]`, `set x [string index abc 99]` or `if { [matchclass …] }`
        // would otherwise escape the security / bounds / arity / style
        // families (IRULE2001/2002, W100, …) entirely.
        if !presubstituted_args {
            self.run_nested_command_diagnostics(arg_tokens_in, scope_path);
            if let Some(trace) = workspace_phase {
                trace.record("nested-command-diagnostics");
            }
            self.run_nested_expr_diagnostics(site);
        }

        report_phase("nested-diagnostics");
        // Record variable-as-command and
        // command-substitution-as-command call sites so the
        // post-walk W307 / W308 emitters can resolve them.
        self.record_var_or_cmd_command_site(VarOrCmdSite {
            cmd_name,
            cmd_tok,
            head_expanded: arg_expand_in.first().copied().unwrap_or(false),
            args,
            arg_tokens,
            arg_expand: arg_expand_in.get(1..).unwrap_or(&[]),
            scope_path,
        });

        report_phase("variable-command-sites");
        // W125 (orphaned control-flow keyword) and IRULE5005 (direct
        // iRules-proc call without `call`) retain their separate original
        // lookup and hosted source-procedure purposes.
        self.emit_proc_resolution_diagnostics(cmd_name, args, cmd_tok, scope_path);

        // When the constructor's class head is a `$var` reference
        // instead of a literal bareword, defer to the flow-sensitive
        // value model rather than dropping the
        // instance's class entirely.
        self.record_pending_instance_class_site(cmd_tok.span.start());

        report_phase("procedure-resolution");
        // Registry-owned expression arguments must be diagnosed before
        // body-owning handlers take their early-return paths.
        self.dispatch_expr_arguments(site);

        report_phase("expression-diagnostics");
        // Dispatch-site diagnostic emitters (W302 / W001 / E004 / W101
        // / W304 / W004 / E002-E003).  Extracted from this function so
        // it stays within the line budget; see the method for the
        // per-code rationale and ordering.  Run before the
        // early-returning handlers so option-bearing / body-owning
        // commands still get checked.
        self.emit_dispatch_site_diagnostics(site);
    }

    pub(super) fn record_direct_command_invocation(
        &mut self,
        argv_texts: &[String],
        arg_tokens_in: &[Token],
        single_token_word: &[bool],
        arg_expand_in: &[bool],
        scope_path: &[usize],
    ) {
        let cmd_name = argv_texts[0].as_str();
        let args = &argv_texts[1..];
        let cmd_tok = arg_tokens_in[0];
        // A `${ns}::setdef`-shaped head is only *written* dynamically —
        // when `ns` is a constant that dominates this call the target is
        // as statically determined as a literal one, so resolution runs
        // on the folded name.  A head that cannot be
        // folded keeps its written text and resolves to nothing, exactly
        // as before.
        let head = self.resolve_dynamic_command_head(
            cmd_name,
            cmd_tok,
            single_token_word.first().copied().unwrap_or(false),
            scope_path,
        );
        let resolved = self.resolve_command_qualified_name(&head, scope_path);
        let arg_count = call_arg_count(args, arg_expand_in);
        // A folded head resolves to a real command, but its span is not
        // the written name — `${ns}::setdef` spells only the tail — so it
        // is a *reference*, never a rename target: overwriting the span
        // would splice the new name over the substitution itself.
        let folded = matches!(head, std::borrow::Cow::Owned(_));
        let computed = folded
            || !single_token_word.first().copied().unwrap_or(false)
            || !matches!(cmd_tok.kind, TokenType::Esc | TokenType::Str)
            || arg_expand_in.first().copied().unwrap_or(false);
        // A folded head's *written* name (`${ns}::setdef`) is not the
        // `{ns}::{name}` shape `finalise_invocation_resolutions` recovers
        // the calling namespace from, so that pass cannot rebuild this
        // call's candidate list — and without one it also cannot demote
        // the walk's local-first guess to the global candidate Tcl really
        // dispatches to.  `set ns tk; ${ns}::setdef …` inside `::tk` was
        // left pinned to the non-existent `::tk::tk::setdef`, so
        // find-references from `::tk::setdef`'s own declaration missed the
        // call while go-to-definition (which re-resolves from the cursor)
        // found it.  The folded name *is*
        // known here, so the list is built now and finalise settles
        // against it instead of rebuilding.  The `namespace path` read
        // here is the walk's current one: a path declared *later* in the
        // file is not retro-applied to a folded head (every other
        // walk-time resolution has the same horizon; finalise's own
        // rebuild is what normally widens it).
        let folded_candidates = self.folded_head_candidates(&head, folded, scope_path);
        let (original_name_input, original_lookup) = self
            .retained_invocation_tokens(cmd_tok.span.start(), arg_tokens_in)
            .and_then(|tokens| {
                let binding = tokens.source_binding.as_ref()?;
                let input = binding.original_head_name_input(&tokens)?;
                let lookup = binding.original_command_lookup(&tokens, &input);
                Some((Some(input), lookup))
            })
            .unwrap_or_default();
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                original_name_input,
                original_callback_signature_lookup: None,
                original_callback_prefix: None,
                original_lookup,
                resolved_qualified_name: Some(resolved.clone()),
                resolution_candidates: folded_candidates,
                indirect: computed,
                rename_safe: !computed,
                ..crate::signature_scan::types::SignatureCommandInvocation::written(
                    cmd_name.to_owned(),
                    cmd_tok.span,
                    arg_count,
                )
            },
        );
        // `<ensemble> <subcommand> …` — record an additional, existence
        // -probed `CommandInvocation` for the subcommand word so
        // references, rename, call-hierarchy, and go-to-definition see
        // through a static `namespace ensemble create -map`/
        // `-subcommands` mapping the same way they already see through
        // an `interp alias`.
        self.record_ensemble_subcommand_invocation(&resolved, args, arg_tokens_in, arg_expand_in);
    }

    /// Record an iRules `call PROC ARG...`'s target as its own
    /// `CommandInvocation` so references / rename / call-hierarchy see
    /// through the indirection. Extracted from [`Self::process_command`]
    /// so it stays within the line budget.
    pub(super) fn record_irules_call_invocation(
        &mut self,
        argument_tokens: &[Token],
        scope_path: &[usize],
        offset: u32,
        presubstituted: bool,
    ) {
        if !self.profile.is_irules() {
            return;
        }
        let Some(original) = self.original_name_source(offset, argument_tokens, presubstituted)
        else {
            return;
        };
        let Some(registry) = original.registry() else {
            return;
        };
        let Some(argument) = registry
            .with_schema(selected_rule_procedure_operand)
            .flatten()
        else {
            return;
        };
        let Some(target_name) = registry.literal(argument).map(str::to_owned) else {
            return;
        };
        let Some(word) = original.word(argument) else {
            return;
        };
        let span = word.span();
        let original_name_input = registry
            .words()
            .operands()
            .get(argument)
            .and_then(Option::as_ref)
            .and_then(|operand| operand.input())
            .cloned();
        let resolved = self.resolve_command_qualified_name(&target_name, scope_path);
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                original_callback_signature_lookup: None,
                original_callback_prefix: None,
                original_lookup: None,
                original_name_input,
                lookup:
                    crate::signature_scan::types::SignatureCommandLookup::PossibleConsumedName {
                        invocation_offset: offset,
                    },
                name: target_name,
                range: span,
                resolved_qualified_name: Some(resolved),
                resolved_user_definition: false,
                resolved_definition: None,
                resolved_command_reference: None,
                resolution_candidates: Vec::new(),
                argc: None,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                // A selected source schema supplies a candidate reference,
                // not a current owning rule or writable command identity.
                rename_safe: false,
                existence_probe: false,
                is_mathfunc_call: false,
                ensemble_dispatch: None,
            },
        );
    }
}

fn selected_rule_procedure_operand(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<usize> {
    schema.authored_source_rule_procedure_operand()
}

/// The argument count a call site advertises for cross-file arity checking.
///
/// A `{*}`-expanded argument makes the runtime count unknown, so the answer
/// is `None` and arity checking skips conservatively.
fn call_arg_count(args: &[String], arg_expand_in: &[bool]) -> Option<usize> {
    if arg_expand_in.iter().skip(1).copied().any(|e| e) {
        None
    } else {
        Some(args.len())
    }
}
