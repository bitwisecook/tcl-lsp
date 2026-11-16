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

//! Commands named as data: command prefixes, command-name arguments, math functions and ensemble subcommands.

use super::super::state::Analyser;
use super::nested_invocations::nested_source_prefixes;
use tcl_lexer::{Span, Token};

impl Analyser {
    /// Retain conditional callback source metadata from the original selected
    /// command or instance-method schema and the genuine whole prefix operand.
    pub(super) fn record_command_prefix_invocations(
        &mut self,
        invocation_offset: u32,
        argument_tokens: &[Token],
    ) {
        let Some(tokens) = self.retained_invocation_tokens(invocation_offset, argument_tokens)
        else {
            return;
        };
        let Some((segment, _)) =
            crate::registry_invocation::source_structure::original_segment_for_tokens(
                &self.source,
                &self.result,
                &tokens,
            )
        else {
            return;
        };
        let source_map = Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let heads = nested_source_prefixes(
            &source_map,
            &self.result,
            &segment,
            self.file_lexer_config(),
        );
        for head in heads {
            let Some(original) = head.callback else {
                continue;
            };
            let original_lookup = original.lookup().cloned();
            let original_name_input = Some(original.name_input().clone());
            let callback_arity = original.appended_arity();
            let callback_baked_args = original.baked_argument_count();
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    original_callback_signature_lookup: None,
                    original_callback_prefix: Some(std::sync::Arc::new(original)),
                    original_lookup,
                    original_name_input,
                    lookup: crate::signature_scan::types::SignatureCommandLookup::DeferredReference,
                    name: head.name,
                    range: head.span,
                    resolved_qualified_name: None,
                    resolved_user_definition: false,
                    resolved_definition: None,
                    resolved_command_reference: None,
                    resolution_candidates: Vec::new(),
                    argc: None,
                    callback_arity,
                    callback_baked_args,
                    indirect: false,
                    rename_safe: false,
                    existence_probe: false,
                    is_mathfunc_call: false,
                    ensemble_dispatch: None,
                },
            );
        }
    }

    /// Record a **command reference** — a command named as data rather than
    /// invoked with a fixed argument list at this span (an `info body` target,
    /// a `forward` / ensemble `-map` target, an expression math function).
    /// `written` is the head as it appears, `span` its token, `resolved` the
    /// qualified command it denotes; [`Self::finalise_invocation_resolutions`]
    /// recovers the namespace from the `(written, resolved)` pair and settles
    /// the candidate list, so a rename rewrites only the written token.  `argc`
    /// is the call-site argument count for arity checking, or `None` when the
    /// site merely names the command.
    pub(in crate::analyser) fn push_command_reference(
        &mut self,
        written: String,
        span: Span,
        resolved: String,
        argc: Option<usize>,
    ) {
        self.push_command_reference_with_policy(
            written,
            span,
            resolved,
            argc,
            false,
            crate::signature_scan::types::SignatureCommandLookup::DeferredReference,
        );
    }

    /// [`Self::push_command_reference`] with an explicit existence policy:
    /// `existence_probe: true` records a reference the W123 pass must skip
    /// (the probed command legitimately may not exist).
    pub(in crate::analyser) fn push_command_reference_with_policy(
        &mut self,
        written: String,
        span: Span,
        resolved: String,
        argc: Option<usize>,
        existence_probe: bool,
        lookup: crate::signature_scan::types::SignatureCommandLookup,
    ) {
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                original_callback_signature_lookup: None,
                original_callback_prefix: None,
                original_lookup: None,
                original_name_input: None,
                lookup,
                name: written,
                range: span,
                resolved_qualified_name: Some(resolved),
                resolved_user_definition: false,
                resolved_definition: None,
                resolved_command_reference: None,
                resolution_candidates: Vec::new(),
                argc,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                rename_safe: true,
                existence_probe,
                is_mathfunc_call: false,
                ensemble_dispatch: None,
            },
        );
    }

    /// [`Self::push_command_reference`] for the **subcommand word** of an
    /// `<ensemble> <sub> …` dispatch: an existence-probed reference to the
    /// mapped target that also carries the mapping's provenance
    /// ([`crate::signature_scan::types::SignatureCommandInvocation::ensemble_dispatch`],
    /// so rename can tell a `-map` key — an arbitrary name it
    /// must leave alone — from a `-subcommands` entry, which is the target's
    /// own tail and has to move with it.
    ///
    /// A dedicated method rather than another parameter on
    /// [`Self::push_command_reference_with_policy`]: every dispatch word is
    /// existence-probed for the same reason (`make` is never independently
    /// callable), so the two facts always travel
    /// together and callers cannot pair them wrongly.
    pub(in crate::analyser) fn push_ensemble_dispatch_reference(
        &mut self,
        written: String,
        span: Span,
        head: Option<u32>,
        entry: &super::super::types::EnsembleSubcommandTarget,
        argc: Option<usize>,
    ) {
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                original_callback_signature_lookup: None,
                original_callback_prefix: None,
                original_lookup: None,
                original_name_input: None,
                lookup: head.map_or(
                    crate::signature_scan::types::SignatureCommandLookup::DeferredReference,
                    |invocation_offset| {
                        crate::signature_scan::types::SignatureCommandLookup::EnsembleDispatch {
                            invocation_offset,
                        }
                    },
                ),
                name: written,
                range: span,
                resolved_qualified_name: Some(entry.target.clone()),
                resolved_user_definition: false,
                resolved_definition: None,
                resolved_command_reference: None,
                resolution_candidates: Vec::new(),
                argc,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                rename_safe: true,
                existence_probe: true,
                is_mathfunc_call: false,
                ensemble_dispatch: Some(entry.provenance),
            },
        );
    }

    /// [`Self::push_command_reference`] for an `expr` math-function call —
    /// see
    /// [`crate::signature_scan::types::SignatureCommandInvocation::is_mathfunc_call`].
    /// A dedicated method rather than another positional bool on
    /// [`Self::push_command_reference_with_policy`]: it has exactly one
    /// caller, and a same-typed `existence_probe, is_mathfunc_call` pair
    /// invites a silently-transposed call.
    pub(in crate::analyser) fn push_mathfunc_command_reference(
        &mut self,
        written: String,
        span: Span,
        resolved: String,
        argc: Option<usize>,
    ) {
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                original_callback_signature_lookup: None,
                original_callback_prefix: None,
                original_lookup: None,
                original_name_input: None,
                lookup: crate::signature_scan::types::SignatureCommandLookup::InvocationHead,
                name: written,
                range: span,
                resolved_qualified_name: Some(resolved),
                resolved_user_definition: false,
                resolved_definition: None,
                resolved_command_reference: None,
                resolution_candidates: Vec::new(),
                argc,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                rename_safe: true,
                existence_probe: false,
                is_mathfunc_call: true,
                ensemble_dispatch: None,
            },
        );
    }

    /// Record each [`tcl_registry::arg_role::ArgRole::CommandName`] argument as
    /// a command invocation: a bare command name held as data (`info body
    /// PROC`, `info args PROC`, `info default PROC …`) that navigation must
    /// reach, but which is *not* invoked here — so it carries no call arity.
    /// A dynamic word (`info body $p`) names no static command and is skipped.
    pub(super) fn record_command_name_invocations(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        invocation_offset: u32,
    ) {
        let roles = self.retained_argument_role_assistance(invocation_offset, arg_tokens);
        let definite = self.retained_argument_role_consensus(invocation_offset, arg_tokens);
        // Required-existence references and probe references share the
        // recording; only the existence policy carried on the record
        // differs (a probe never feeds W123).
        for (role, probe) in [
            (tcl_registry::arg_role::ArgRole::CommandName, false),
            (tcl_registry::arg_role::ArgRole::CommandNameProbe, true),
        ] {
            for idx in roles
                .iter()
                .filter_map(|(index, candidate)| (*candidate == role).then_some(*index))
            {
                let (Some(name), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) else {
                    continue;
                };
                if name.is_empty() || crate::naming::is_dynamic_word(name) {
                    continue;
                }
                // A glob pattern probes a *set* of commands, not one exact
                // name — no single reference identity exists, so abstain.
                if probe && name.contains(['*', '?']) {
                    continue;
                }
                let resolved = self.resolve_command_qualified_name(name, scope_path);
                let unanimous = definite.contains(&(idx, role));
                let lookup = if unanimous {
                    crate::signature_scan::types::SignatureCommandLookup::ConsumedName {
                        invocation_offset,
                    }
                } else {
                    crate::signature_scan::types::SignatureCommandLookup::PossibleConsumedName {
                        invocation_offset,
                    }
                };
                self.push_command_reference_with_policy(
                    name.clone(),
                    tok.span,
                    resolved,
                    None,
                    probe,
                    lookup,
                );
                if !unanimous && let Some(reference) = self.result.command_invocations.last_mut() {
                    reference.rename_safe = false;
                }
            }
        }
    }

    /// `<ensemble> <subcommand> …` — when `resolved_cmd` names a known
    /// ensemble (a key in [`AnalysisResult::ensemble_subcommand_targets`])
    /// and the first actual argument is a static, non-`{*}`-expanded
    /// subcommand word present in that ensemble's map, record a second,
    /// existence-probed [`SignatureCommandInvocation`] for the subcommand
    /// word pointing at its resolved target — the same "referenceable but
    /// never independently callable" shape [`Self::record_command_name_invocations`]
    /// already uses for `CommandNameProbe`: `make` is
    /// never itself a valid command name (only the pair `widget make`
    /// dispatches), so it must never feed W123. Lets `definition`/`hover`/
    /// `references`/rename/call-hierarchy resolve `widget make` to
    /// `::widget::Make` through the same
    /// `resolved_qualified_name`-matching path every other indirection
    /// (alias, rename, iRules `call`) already uses — no separate per
    /// -provider ensemble-aware code needed there.
    ///
    /// [`AnalysisResult::ensemble_subcommand_targets`]: super::super::types::AnalysisResult::ensemble_subcommand_targets
    /// [`SignatureCommandInvocation`]: crate::signature_scan::types::SignatureCommandInvocation
    pub(super) fn record_ensemble_subcommand_invocation(
        &mut self,
        resolved_cmd: &str,
        args: &[String],
        arg_tokens_in: &[Token],
        arg_expand_in: &[bool],
    ) {
        // `arg_expand_in[0]` is the command name's own (always-`false`)
        // flag; `arg_expand_in[1]` is the subcommand word's — a `{*}`
        // -expanded subcommand names no static word at all.
        if arg_expand_in.get(1).copied().unwrap_or(false) {
            return;
        }
        let Some(sub) = args.first() else { return };
        if crate::naming::is_dynamic_word(sub) {
            return;
        }
        let Some(tok) = arg_tokens_in.get(1) else {
            return;
        };
        // Args *after* the consumed subcommand word — not `args.len()`,
        // which would double-count the subcommand word itself against the
        // target proc's real arity.  `{*}` anywhere in the target's own
        // arguments makes the runtime count unknown, same convention as the
        // head invocation just above.
        let sub_argc = if arg_expand_in
            .get(2..)
            .is_some_and(|rest| rest.iter().any(|&e| e))
        {
            None
        } else {
            Some(args.len() - 1)
        };
        let head = arg_tokens_in.first().map(|head| head.span.start());
        self.record_or_defer_ensemble_subcommand(resolved_cmd, sub, tok.span, head, sub_argc);
    }

    /// Record the existence-probed subcommand reference for
    /// `<ensemble> <sub>` — now if the ensemble's map is already known,
    /// else queued for [`Self::flush_pending_ensemble_subcommand_invocations`].
    ///
    /// The queue is filled **only** by the per-item shell pass. The
    /// whole-file DFS walks each proc/method body at its definition point,
    /// so its map already holds every ensemble a body declared earlier in
    /// the file; the shell pass defers those bodies, so an ensemble created
    /// inside `proc ::app::widget::Setup {…}` is invisible to it and every
    /// later `::app::widget show` call site was silently dropped — leaving
    /// find-references / rename / code-lens / call-hierarchy unable to
    /// enumerate call sites that go-to-definition (an on-demand lookup
    /// against the finished analysis) resolved perfectly well.
    ///
    /// The replay is gated on the ensemble's own recording preceding the
    /// call site, which is exactly the visibility the whole-file DFS has —
    /// so the two walk strategies produce identical `command_invocations`
    /// (the `per_item == analyse` corpus gate holds).
    pub(super) fn record_or_defer_ensemble_subcommand(
        &mut self,
        resolved_cmd: &str,
        sub: &str,
        span: Span,
        head: Option<u32>,
        argc: Option<usize>,
    ) {
        if let Some(entry) = self
            .result
            .resolve_ensemble_subcommand(resolved_cmd, sub)
            .cloned()
        {
            self.push_ensemble_dispatch_reference(sub.to_owned(), span, head, &entry, argc);
            return;
        }
        // Nothing deferred yet means nothing can *become* visible before
        // this offset either, so there is no candidate to hold.
        if !self.defer_proc_bodies || self.deferred_bodies.is_empty() {
            return;
        }
        self.pending_ensemble_subcommands
            .push(super::super::state::PendingEnsembleSubcommand {
                ensemble: resolved_cmd.to_owned(),
                sub: sub.to_owned(),
                span,
                head,
                argc,
            });
    }

    /// Replay every [`Self::record_or_defer_ensemble_subcommand`] miss
    /// against the finished `ensemble_subcommand_targets` map, once every
    /// deferred body has been walked and grafted.
    ///
    /// A candidate counts only when the ensemble's own
    /// `namespace ensemble create|configure` recording **precedes** the call
    /// site — the whole-file DFS's visibility rule, reproduced here so the
    /// per-item walk records the same invocation set. Where the ensemble's
    /// name or its `-map` is dynamic nothing is recorded at all (the map
    /// never gains the entry), so the abstention is inherited from the one
    /// place that decides it rather than re-decided here.
    pub(in super::super) fn flush_pending_ensemble_subcommand_invocations(&mut self) {
        let pending = std::mem::take(&mut self.pending_ensemble_subcommands);
        for cand in pending {
            let Some(entry) = self
                .result
                .resolve_ensemble_subcommand(&cand.ensemble, &cand.sub)
                .cloned()
            else {
                continue;
            };
            let Some(&declared_at) = self.ensemble_record_offsets.get(&cand.ensemble) else {
                continue;
            };
            if declared_at >= cand.span.start() {
                continue;
            }
            self.push_ensemble_dispatch_reference(
                cand.sub, cand.span, cand.head, &entry, cand.argc,
            );
        }
    }
}
