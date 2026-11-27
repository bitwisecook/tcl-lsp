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

//! Central command dispatch.
//!
//! Walks one segmented Tcl command and routes it through the
//! per-command handlers. Which handler family owns a command form is
//! registry data — the [`tcl_registry::hooks::AnalyserHookId`] stamped
//! on its `CommandSpec` / `SubCommand` — so the dispatch is a single
//! typed `match` ([`Analyser::dispatch_analyser_hook`]); adding a new
//! handler means adding a hook variant and stamping the spec.

use std::sync::Arc;

#[cfg(test)]
use tcl_core_types::DiagCode;
use tcl_lexer::Token;
#[cfg(test)]
use tcl_lexer::{LexerConfig, SourceMap, TokenType};

#[cfg(test)]
use crate::segmenter::SegmentedCommand;

use super::state::Analyser;

/// Fixed-size timing evidence for the opt-in workspace scan diagnostic only.
/// Stage intervals include nested work; they do not grant source authority.
struct WorkspaceCommandPhases {
    started: std::time::Instant,
    bytes: usize,
    site: u32,
    previous_us: std::cell::Cell<u128>,
    walk_previous_us: std::cell::Cell<u128>,
    stage_us: [std::cell::Cell<u128>; 15],
    seen: std::cell::Cell<u16>,
}

impl WorkspaceCommandPhases {
    // Array order: original names, scoped bodies, variable receivers, direct
    // invocations, nested invocations, callback invocations, command-name
    // invocations, namespace roles, nested command diagnostics, nested expression
    // diagnostics, variable-command sites,
    // procedure resolution, expression diagnostics, dispatch diagnostics, handlers.
    const STAGES: [&str; 15] = [
        "original-names",
        "scoped-bodies",
        "variable-receivers",
        "direct-invocations",
        "nested-invocations",
        "callback-invocations",
        "command-name-invocations",
        "namespace-role-facts",
        "nested-command-diagnostics",
        "nested-diagnostics",
        "variable-command-sites",
        "procedure-resolution",
        "expression-diagnostics",
        "dispatch-diagnostics",
        "handlers-complete",
    ];

    fn start(bytes: usize, site: u32) -> Option<Self> {
        tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_WORKSPACE_SCAN").map(|_| {
            WORKSPACE_WALK_TOTALS.with(|totals| {
                if let Some(mut value) = totals.get() {
                    value.commands = value.commands.saturating_add(1);
                    totals.set(Some(value));
                }
            });
            Self {
                started: std::time::Instant::now(),
                bytes,
                site,
                previous_us: std::cell::Cell::new(0),
                walk_previous_us: std::cell::Cell::new(0),
                stage_us: [const { std::cell::Cell::new(0) }; 15],
                seen: std::cell::Cell::new(0),
            }
        })
    }

    // Order: original names, direct interpreter visibility, indirect visibility,
    // constructor recording, scoped bodies, variable receivers, dynamic names,
    // optional diagnostic passes, handlers. Intervals include nested calls.
    fn record_walk(&self, index: usize) {
        WORKSPACE_WALK_TOTALS.with(|totals| {
            if let Some(mut value) = totals.get() {
                let elapsed = self.started.elapsed().as_micros();
                let previous = self.walk_previous_us.replace(elapsed);
                value.phase_us[index] =
                    value.phase_us[index].saturating_add(elapsed.saturating_sub(previous));
                value.phase_calls[index] = value.phase_calls[index].saturating_add(1);
                totals.set(Some(value));
            }
        });
    }

    fn walk(trace: Option<&Self>, index: usize) {
        if let Some(trace) = trace {
            trace.record_walk(index);
        }
    }

    fn record(&self, stage: &str) {
        match stage {
            "original-names" => self.record_walk(0),
            "scoped-bodies" => self.record_walk(4),
            "variable-receivers" => self.record_walk(5),
            "dispatch-diagnostics" => self.record_walk(7),
            "handlers-complete" => self.record_walk(8),
            _ => {}
        }
        let Some(index) = Self::STAGES
            .iter()
            .position(|candidate| *candidate == stage)
        else {
            return;
        };
        let elapsed = self.started.elapsed().as_micros();
        let previous = self.previous_us.replace(elapsed);
        self.stage_us[index].set(elapsed.saturating_sub(previous));
        self.seen.set(self.seen.get() | (1 << index));
        if index == 14 && elapsed >= 20_000 {
            let stage_us = self.stage_us.each_ref().map(std::cell::Cell::get);
            eprintln!(
                "WORKSPACE_COMMAND_PHASE bytes={} site={} total_us={elapsed} seen={} stage_us={stage_us:?}",
                self.bytes,
                self.site,
                self.seen.get(),
            );
        }
    }
}

/// Aggregate evidence for one synchronous top-level walk. Nested command work
/// remains inclusive; these counters do not identify isolated CPU costs.
#[derive(Clone, Copy, Default)]
struct WorkspaceWalkTotals {
    commands: u64,
    phase_us: [u128; 9],
    phase_calls: [u64; 9],
}

std::thread_local! {
    static WORKSPACE_WALK_TOTALS: std::cell::Cell<Option<WorkspaceWalkTotals>> = const {
        std::cell::Cell::new(None)
    };
}

mod argument_role_facts;
mod body_arguments;
mod body_walk;
mod command_references;
mod dispatch_sites;
mod hook_dispatch;
mod instance_classes;
mod invocation_records;
mod nested_diagnostics;
mod nested_invocations;
mod original_source;
mod site_diagnostics;

#[cfg(test)]
use body_walk::MAX_BODY_DEPTH;
#[cfg(test)]
pub(in crate::analyser) use instance_classes::class_var_head_constructor_subst;
#[cfg(test)]
use nested_diagnostics::{collect_segment_recursive, collect_substitution_segments};
#[cfg(test)]
pub(crate) use nested_invocations::scan_nested_command_heads;
#[cfg(test)]
use nested_invocations::{CollectedHead, collect_substitution_heads};
pub(in crate::analyser) use original_source::OriginalFormatTemplate;

pub(super) struct WorkspaceWalkPhases {
    previous: Option<WorkspaceWalkTotals>,
}

impl WorkspaceWalkPhases {
    pub(super) fn start() -> Option<Self> {
        tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_WORKSPACE_SCAN").map(|_| Self {
            previous: WORKSPACE_WALK_TOTALS
                .with(|totals| totals.replace(Some(WorkspaceWalkTotals::default()))),
        })
    }

    pub(super) fn finish(self, bytes: usize, structure_only: bool) {
        let totals = WORKSPACE_WALK_TOTALS.with(std::cell::Cell::get);
        drop(self);
        if let Some(totals) = totals {
            eprintln!(
                "WORKSPACE_WALK_PHASE bytes={bytes} structure_only={structure_only} commands={} phase_us={:?} phase_calls={:?}",
                totals.commands, totals.phase_us, totals.phase_calls,
            );
        }
    }
}

impl Drop for WorkspaceWalkPhases {
    fn drop(&mut self) {
        WORKSPACE_WALK_TOTALS.with(|totals| totals.set(self.previous));
    }
}

/// Reuse immutable document bytes and line geometry; no source proof follows.
fn original_document_source_image(
    source: &str,
    retained: Option<&tcl_lexer::SourceImage>,
) -> tcl_lexer::SourceImage {
    retained
        .filter(|image| {
            image.channel() == tcl_lexer::SourceChannel::Document && image.has_bytes_of(source)
        })
        .cloned()
        .unwrap_or_else(|| tcl_lexer::SourceImage::document(source))
}

/// The borrowed word-level view of one command, threaded into the
/// dispatch-site diagnostic emitter ([`Analyser::emit_dispatch_site_diagnostics`]).
/// Bundling these co-varying slices keeps the emitter under the argument
/// limit; it destructures them back into locals so the per-code emitter
/// calls read unchanged.
struct DispatchSite<'a> {
    cmd_name: &'a str,
    args: &'a [String],
    arg_tokens: &'a [Token],
    arg_single: &'a [bool],
    arg_expand_in: &'a [bool],
    cmd_tok: Token,
    scope_path: &'a [usize],
    /// This command's words are pre-substituted `list` elements — see
    /// [`super::state::AnalyserState::presubstituted_args`].  A `$word` here
    /// was replaced by its value while the *building* frame ran, before the
    /// script existed, so it is not the written-source shape any
    /// substitution-vs-literal check is about.
    presubstituted_args: bool,
}

/// Bundled arguments for [`Analyser::record_var_or_cmd_command_site`] — the
/// command-head token plus the co-varying slices `process_command` and its
/// nested-substitution twin already have in hand.  Keeps the recorder under
/// the argument-count limit; it destructures them back into locals so the
/// body reads unchanged.  Every field is a reference or `Token` (itself
/// `Copy`), so the bundle is `Copy` too — passed by value, no
/// `needless_pass_by_value` lint to work around.
#[derive(Clone, Copy)]
struct VarOrCmdSite<'a> {
    cmd_name: &'a str,
    cmd_tok: Token,
    head_expanded: bool,
    args: &'a [String],
    arg_tokens: &'a [Token],
    arg_expand: &'a [bool],
    scope_path: &'a [usize],
}

/// Bundled arguments for [`Analyser::record_bareword_dispatch_site`] — the
/// `Esc`-head slice of [`VarOrCmdSite`], destructured back into locals in
/// the body.  `Copy` for the same reason [`VarOrCmdSite`] is: every field is
/// a reference or a `Copy` `Token`, so it passes by value with no
/// `needless_pass_by_value` lint to work around.
#[derive(Clone, Copy)]
struct BarewordDispatch<'a> {
    cmd_name: &'a str,
    cmd_tok: Token,
    args: &'a [String],
    method_span: Option<tcl_lexer::Span>,
    in_method: bool,
    arg_expand: &'a [bool],
}

/// A dispatched call's per-word source facts, parallel to its post-head
/// words.
#[derive(Clone, Copy)]
struct WordFacts<'a> {
    /// Whether each word is a single token.
    single: &'a [bool],
}

/// Hook and traits from one retained authoring-context resolution.
pub(super) struct ResolvedAnalyserHook {
    pub(super) hook: tcl_registry::hooks::AnalyserHookId,
    pub(super) traits: tcl_registry::Traits,
}

impl Analyser {
    /// Process a single segmented command.
    ///
    /// Walks `args` against every handler and stops at the first
    /// match. Non-matching commands fall through silently —
    /// they're either unknown (the W123 emitter handles reporting)
    /// or registry-known commands that don't need analyser-side
    /// intervention (the IR pass does the heavy lifting).
    ///
    /// `argv_texts` and `arg_tokens` parallel each other:
    /// `argv_texts[0]` is the command name, `argv_texts[1..]`
    /// the arguments. `arg_tokens[0]` is the command-name token.
    /// `single_token_word` is parallel to argv and indicates
    /// whether each word is a single atomic token (used by
    /// [`Self::bind_value_word_assignment`] for the const-string
    /// environment).
    ///
    /// Simple-command arity (E002 / E003) is emitted here via
    /// [`Self::emit_arity_diagnostics`]; the candidates are
    /// flushed post-walk by [`Self::flush_arity_diagnostics`].
    pub fn process_command(
        &mut self,
        argv_texts: &[String],
        arg_tokens_in: &[Token],
        single_token_word: &[bool],
        arg_expand_in: &[bool],
        scope_path: &[usize],
    ) {
        if argv_texts.is_empty() || arg_tokens_in.is_empty() {
            return;
        }
        let workspace_phase =
            WorkspaceCommandPhases::start(self.source.len(), arg_tokens_in[0].span.start());
        #[cfg(debug_assertions)]
        let phase_started = tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
            .map(|_| std::time::Instant::now());
        #[cfg(debug_assertions)]
        let phase_site = arg_tokens_in[0].span.start();
        #[cfg(debug_assertions)]
        let phase_bytes = self.source.len();
        let report_phase = |stage: &str| {
            #[cfg(debug_assertions)]
            if let Some(started) = &phase_started {
                eprintln!(
                    "ANALYSER_COMMAND_PHASE bytes={phase_bytes} site={phase_site} stage={stage} ms={}",
                    started.elapsed().as_millis()
                );
            }
            if let Some(trace) = &workspace_phase {
                trace.record(stage);
            }
        };
        // See `AnalyserState::presubstituted_args`: taken (and reset) so only
        // *this* command level skips the substitution re-walks below.
        let presubstituted_args = std::mem::take(&mut self.presubstituted_args);
        if !presubstituted_args {
            self.retain_original_static_source_names(arg_tokens_in);
        }
        report_phase("original-names");
        let cmd_name = argv_texts[0].as_str();
        // Conditional source visibility never removes possible invocation
        // effects or workspace edges without an actual hidden-slot receipt.
        self.observe_interp_visibility(arg_tokens_in, scope_path);
        WorkspaceCommandPhases::walk(workspace_phase.as_ref(), 1);
        let args = &argv_texts[1..];
        let arg_tokens = &arg_tokens_in[1..];
        let arg_single = single_token_word.get(1..).unwrap_or_default();
        self.observe_indirect_interp_visibility(arg_tokens_in, scope_path);
        WorkspaceCommandPhases::walk(workspace_phase.as_ref(), 2);
        // Record this invocation so the post-walk
        // ``emit_unresolved_command_diagnostics`` (W123) can iterate
        // every command head the analyser visited.  ``inv.range``
        // anchors at the command-head token so the W123 message
        // points at the unresolved name rather than the whole
        // command line.
        let cmd_tok = arg_tokens_in[0];
        // The head's conditional metadata is an input of this command's
        // registry words, so it is recorded before anything asks for them.
        if !presubstituted_args {
            self.record_original_scoped_body_advice(cmd_tok.span.start(), arg_tokens, scope_path);
        }
        report_phase("scoped-bodies");
        // Retain constructor reporting metadata in structure-only analyses
        // as well as full analyses. Source receiver consumers independently
        // require the positioned constructor or instance issuer; these maps
        // do not establish Native command selection or successful allocation.
        let creation_ns = self.command_resolution_namespace(scope_path);
        self.record_instance_creation(cmd_name, args, &creation_ns, cmd_tok.span.start());
        WorkspaceCommandPhases::walk(workspace_phase.as_ref(), 3);
        // Structure-only mode (item-tree extraction) skips every diagnostic /
        // cross-feature recording pass below — they don't affect the declared
        // proc / class / alias / ensemble structure and are the bulk of the
        // per-command cost.  The structural handlers further down still run, so
        // `file_decls` is identical to a full `analyse` (gated by the
        // `file_decls_corpus` corpus test).
        self.record_original_variable_receivers(cmd_tok.span.start(), arg_tokens, scope_path);
        report_phase("variable-receivers");
        self.record_dynamic_variable_name_sites(
            args,
            arg_tokens,
            arg_single,
            scope_path,
            cmd_tok.span.start(),
            presubstituted_args,
        );
        WorkspaceCommandPhases::walk(workspace_phase.as_ref(), 6);

        if !self.structure_only {
            self.record_direct_command_invocation(
                argv_texts,
                arg_tokens_in,
                single_token_word,
                arg_expand_in,
                scope_path,
            );

            // iRules ``call PROC ARG...`` — record an additional
            // ``CommandInvocation`` for the target proc so that
            // references, rename, and call-hierarchy see through the
            // indirection.
            self.record_irules_call_invocation(
                arg_tokens,
                scope_path,
                cmd_tok.span.start(),
                presubstituted_args,
            );
            report_phase("direct-invocations");
            self.record_command_site_facts(
                &DispatchSite {
                    cmd_name,
                    args,
                    arg_tokens,
                    arg_single,
                    arg_expand_in,
                    cmd_tok,
                    scope_path,
                    presubstituted_args,
                },
                arg_tokens_in,
                &report_phase,
                workspace_phase.as_ref(),
            );
        } // end `if !self.structure_only`
        report_phase("dispatch-diagnostics");
        self.dispatch_command_handlers(
            cmd_name,
            args,
            arg_tokens,
            WordFacts { single: arg_single },
            cmd_tok,
            scope_path,
        );
        report_phase("handlers-complete");
    }

    /// Share immutable document geometry after the complete byte/channel check.
    fn command_document_source_image(&self) -> tcl_lexer::SourceImage {
        original_document_source_image(&self.source, self.head_identities.original_source_image())
    }

    /// Reconstruct original lexical words once, then attach the retained
    /// lookup receipt. Consumers must not recover roles from final imports
    /// or a display head when this actual source site has no applicable shape.
    pub(super) fn retained_invocation_tokens(
        &self,
        invocation_offset: u32,
        argument_tokens: &[Token],
    ) -> Option<Arc<crate::ir::CommandTokens>> {
        // Implementation contract: naming.source.original-command-token-memo
        // docs/design/analysis/name-resolution-proofs/original-command-token-memo.md
        self.head_identities.retained_original_tokens(
            &original_document_source_image(
                &self.source,
                self.head_identities.original_source_image(),
            ),
            self.file_lexer_config(),
            invocation_offset,
            argument_tokens,
        )
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_conditional_variable_read_tests;

#[cfg(test)]
mod original_lexical_source_inventory_tests;

#[cfg(test)]
mod original_script_body_inventory_tests;

#[cfg(test)]
mod original_nested_source_tests;

#[cfg(test)]
mod original_command_variable_geometry_tests;
