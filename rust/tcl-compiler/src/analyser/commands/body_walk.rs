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

//! `analyse_body`: re-segmenting a body script and dispatching each of its commands, with the nesting-depth limit.

use super::super::state::Analyser;
use super::super::types::Severity;
use crate::parsing::syntax::descend::descend_token;
use crate::parsing::syntax::segment::segments_from_tree;
use tcl_core_types::DiagCode;
use tcl_lexer::{Token, TokenType};

/// Maximum nested-body recursion depth for [`Analyser::analyse_body`].
/// `analyse_body` ↔ `process_command` ↔ `dispatch_body_arguments`
/// recurse one frame group per braced-body nesting level; generated /
/// minified Tcl and machine-emitted iRules can nest deeply enough to
/// overflow the stack — an uncatchable SIGABRT that crashes the LSP
/// diagnostics worker and the `tcl diag` / `lint` / `validate` CLIs. At
/// the cap we stop descending into further nested bodies (the diagnostics
/// already collected stand); no real source nests anywhere near this.
///
/// The number itself is [`crate::depth_guard::MAX_SOURCE_NEST_DEPTH`],
/// derived there from a stack budget and a measured per-level cost rather
/// than picked to match a convention.
pub(super) const MAX_BODY_DEPTH: tcl_core_types::RecursionLimit =
    crate::depth_guard::MAX_SOURCE_NEST_DEPTH;

impl Analyser {
    fn emit_body_depth_limit_if_exceeded(&mut self, span: tcl_lexer::Span) -> bool {
        if !MAX_BODY_DEPTH.exceeded(self.body_depth) {
            return false;
        }
        // Stop descending before the recursive body walk overflows the
        // stack — the diagnostics collected up to this nesting level
        // still stand. Report it as a diagnostic (once per walk, not
        // once per nested body past the cap) rather than truncating
        // silently: tclsh's own recursion limit raises a catchable
        // "too many nested evaluations (infinite loop?)" error at this
        // point rather than continuing quietly, and a process abort is
        // never the right failure mode either way.
        if !self.structure_only && !self.e207_emitted {
            self.e207_emitted = true;
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::E207,
                    span,
                    format!(
                        "nesting depth exceeds the analysis limit ({} levels) — \
                     diagnostics for this body and anything nested inside it are not \
                     collected",
                        MAX_BODY_DEPTH.0
                    ),
                    Severity::Error,
                ));
        }
        true
    }

    /// Re-segment a body script and dispatch each command at
    /// `scope_path`.
    ///
    /// Used by every body-walking handler (`handle_proc_command`,
    /// `handle_switch_command`, `handle_catch_command`, the generic
    /// `dispatch_body_arguments`, etc.).
    ///
    /// Body recursion does **not** use the segmenter's re-segmentation
    /// recovery — that splits a runaway top-level command and only
    /// fires at the top level. The per-command syntax *detectors* (E100 /
    /// E102 stray closers, E201 unterminated `[`, E202 unterminated `"`,
    /// E203 unterminated `{`) do run on every body. Dynamic bodies
    /// (`$body`, `[gen]`) are skipped because they can't be statically
    /// re-segmented.
    ///
    /// `body_depth` is bumped for the duration of the walk so
    /// top-level-only command checks can distinguish nested
    /// invocations.
    pub(in super::super) fn analyse_body(
        &mut self,
        body_text: &str,
        body_tok: Token,
        scope_path: &[usize],
    ) {
        if body_tok.kind != TokenType::Str {
            // A selected source list builder can retain conditional command
            // syntax and its original operand reads. Missing parent/builder/
            // target ancestry leaves the body opaque; no frame follows from
            // the builder's presentation or returned-list shape alone.
            self.analyse_list_quoted_body(body_tok, scope_path);
            return;
        }
        self.body_depth += 1;
        if self.emit_body_depth_limit_if_exceeded(body_tok.span) {
            self.body_depth -= 1;
            return;
        }
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        // The rebase below only produces truthful absolute spans while
        // `body_text` maps 1:1 onto its source region; clamp it to the part
        // that actually does.
        // A translating Document channel folds CRLF in the cooked value; the
        // body's spans are original offsets, so walk the region itself.
        let source_region = crate::segmenter::braced_body_source_region(
            &self.source,
            base_offset as usize,
            body_tok.span.end().saturating_sub(1) as usize,
            body_text,
        )
        .to_owned();
        let body_text = crate::segmenter::body_text_in_region(
            &self.source,
            base_offset as usize,
            body_tok.span.end() as usize,
            &source_region,
        );
        // Absolute byte offset at which this body region ends. The E202 /
        // E203 detectors test "reaches end of region" against this, not the
        // whole document — the body's tokens are absolute spans into
        // `self.source`, but a runaway `"` / `{` only swallows to the body's
        // own end.
        let region_end = base_offset as usize + body_text.len();
        let body_commands = crate::segmenter::segment_commands_with_offset_and_config(
            body_text,
            base_offset,
            self.file_lexer_config().nested(),
        );
        let total = body_commands.len();
        let mut cmd_idx: usize = 0;
        while cmd_idx < total {
            if crate::analysis_control::checkpoint(
                crate::analysis_control::AnalysisCheckpoint::Body,
            ) {
                break;
            }
            let cmd_ref = &body_commands[cmd_idx];
            if cmd_ref.argv.is_empty() {
                cmd_idx += 1;
                continue;
            }
            if cmd_ref.is_partial {
                // An unterminated `"` / `{` emits the precise E202 /
                // E203 (with a closing-delimiter fix) ahead of the generic
                // E200, matching the top-level `walk_commands_top_level`
                // branch. Stolen-close-brace detection ⇒ E103
                // (brace partials only); otherwise the generic E200 fires so
                // the user still sees a parse-error diagnostic.
                let brace_partial = matches!(
                    cmd_ref.partial_delimiter,
                    Some(crate::segmenter::UnclosedDelimiter::Brace)
                );
                if !(self.emit_unterminated_delimiter_diagnostics(cmd_ref, region_end)
                    || brace_partial && self.detect_stolen_close_brace(cmd_ref))
                {
                    self.emit_partial_command_diagnostic(cmd_ref);
                }
                cmd_idx += 1;
                continue;
            }
            // Run the E100 (stray `]`) / E102 (stray `}`) token
            // checks on every analysed body, not just the top
            // level. Run on the *original* token stream before
            // ``recover_stray_close_bracket`` repairs the clone,
            // matching the top-level loop's ordering.  Token spans
            // are absolute into the full document, so the full
            // ``self.source`` is the right slice base.
            let generation = self.analysis_context();
            let stray = super::super::syntax_checks::stray_closer_diagnostics(
                cmd_ref,
                &self.source,
                Some(generation.commands()),
                || self.user_command_tail_names(),
            );
            self.result.diagnostics.extend(stray);
            // E201 (unterminated `[`) inside a body — `proc p {} { set y
            // [foo }`.  The CST auto-closes the bracket so the command isn't
            // flagged `is_partial`, but the source carries no real `]`, so
            // it would otherwise go unreported.  Run the same E201
            // detector as `emit_syntax_recovery_diagnostics` at the
            // top level (the top-level ghost-recovery doesn't reach
            // into body scripts).
            let e201 = super::super::syntax_checks::unterminated_bracket_diagnostics(
                cmd_ref,
                &self.source,
                &self.recovery_known_commands,
                self.lexer_config(),
            );
            self.result.diagnostics.extend(e201);
            // E202 (unterminated `"`) / E203 (unterminated `{`) inside a
            // body — `proc p {} { set x "\n puts hi }`.  The brace word the
            // body sits in is balanced, so the body re-segments cleanly and
            // the run-away quote/brace is a non-partial command whose token
            // reaches the body's end.  Run the same E202/E203 detector
            // as `emit_syntax_recovery_diagnostics` at the top level,
            // which the top-level ghost-recovery doesn't reach into body
            // scripts.
            self.emit_unterminated_delimiter_diagnostics(cmd_ref, region_end);
            let mut cmd = cmd_ref.clone();
            // Repair stray ``]`` (missing ``[``) so downstream
            // handlers see the intended argv shape before dispatch.
            self.recover_stray_close_bracket(&mut cmd);
            // Splice orphaned switch case pairs when ``{`` was
            // forgotten.  The returned count is added to ``cmd_idx``
            // so we skip past the consumed orphans.
            let consumed = self.recover_missing_open_brace(&mut cmd, &body_commands, cmd_idx);
            // ``# noqa`` directives in the preceding-comment
            // attribute to this command's line range — same
            // shape as the top-level loop.
            if let Some(line_offsets) = self.line_offsets.as_deref() {
                super::super::utils::apply_preceding_noqa(
                    &cmd,
                    line_offsets,
                    &mut self.result.suppressed_lines,
                );
            }
            self.process_command(
                &cmd.texts,
                &cmd.argv,
                &cmd.single_token_word,
                cmd.expand_word.as_deref().unwrap_or(&[]),
                scope_path,
            );
            self.emit_w216_brace_then_paren(&cmd);
            // Record every `$var` substitution in arg positions so
            // `VarDef.references` carries the read spans the LSP
            // providers consume.
            self.record_arg_var_reads(&cmd, scope_path);
            cmd_idx += 1 + consumed;
        }
        self.body_depth -= 1;
    }

    /// Analyse one registry-declared control-flow body while recording that
    /// facts inside it may execute zero or multiple times.
    pub(in super::super) fn analyse_control_flow_body(
        &mut self,
        body_text: &str,
        body_tok: Token,
        scope_path: &[usize],
    ) {
        self.control_flow_body_depth += 1;
        self.analyse_body(body_text, body_tok, scope_path);
        self.control_flow_body_depth -= 1;
    }

    /// Walk conditional source syntax from a genuine selected list builder and
    /// its original parent/body and target schemas. All projected operands keep
    /// their builder extents. Execution requires its separately retained body
    /// invocation; source applicability cannot issue a frame or runtime argv.
    pub(super) fn analyse_list_quoted_body(
        &mut self,
        body_tok: Token,
        scope_path: &[usize],
    ) -> bool {
        if body_tok.kind != TokenType::Cmd {
            return false;
        }
        let config = self.file_lexer_config();
        let sm = Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let child = descend_token(&sm, body_tok, config);
        if !child.is_terminated() {
            return false;
        }
        let commands = segments_from_tree(child.tree(), &sm);
        let [producer] = commands.as_slice() else {
            return false;
        };
        let Some(input) = self
            .resolved_input
            .as_ref()
            .or(self.result.resolved_input.as_ref())
        else {
            return false;
        };
        let Some(words) =
            crate::registry_invocation::source_structure::source_produced_command_prefix_words_in(
                &self.source,
                input,
                &self.head_identities,
                producer,
            )
        else {
            return false;
        };
        let Some(cmd) = crate::script_arg::original_list_built_script_command(&words, producer)
        else {
            return false;
        };
        self.body_depth += 1;
        if MAX_BODY_DEPTH.exceeded(self.body_depth) {
            self.body_depth -= 1;
            return false;
        }
        self.presubstituted_args = true;
        let evaluated = self
            .head_identities
            .executed_script_for_word(body_tok.span)
            .and_then(|source| {
                let commands = crate::segmenter::segment_commands_image_with_offset_and_config(
                    &source.text,
                    source.base(),
                    self.file_lexer_config().nested(),
                )?;
                let [command] = commands.as_slice() else {
                    return None;
                };
                let proof = self
                    .head_identities
                    .source_bindings_ref()
                    .invocation_at_origin(&source.origin, command.argv.first()?.span.start());
                proof
                    .invocation_realm()
                    .map(|_| (cmd.argv[0].span.start(), proof))
            });
        let previous = std::mem::replace(&mut self.evaluated_body_invocation, evaluated);
        self.process_command(
            &cmd.texts,
            &cmd.argv,
            &cmd.single_token_word,
            &[],
            scope_path,
        );
        self.evaluated_body_invocation = previous;
        self.record_arg_var_reads(&cmd, scope_path);
        self.body_depth -= 1;
        true
    }
}
