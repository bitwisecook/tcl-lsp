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

//! Analysis of pre-segmented commands: chunked, incremental and the shared dispatch loop.

use super::super::types::AnalysisResult;
use super::realms::PrebuiltRealm;
use super::{Analyser, compute_line_offsets};
use std::sync::Arc;

impl Analyser {
    /// Analyse pre-segmented commands chunk-by-chunk and capture
    /// per-chunk snapshots.
    ///
    /// Used by the LSP for incremental document re-analysis: when the
    /// user types into the document, dirty chunks are re-segmented
    /// and fed back through this entry while clean chunks are
    /// restored from a prior snapshot.
    ///
    /// Returns the final [`AnalysisResult`] plus a list of
    /// [`super::super::AnalyserSnapshot`]s, one per chunk in the input
    /// order.  The caller stores the snapshots alongside the
    /// chunk segmentation so a later edit can rewind to the
    /// matching prefix.
    ///
    /// `chunk_commands` is grouped already — each inner `Vec`
    /// is one chunk's worth of commands.  `dialect` matches the
    /// argument to [`Self::analyse`].
    pub fn analyse_chunked(
        &mut self,
        source: &str,
        chunk_commands: Vec<Vec<tcl_source::segment::SegmentedCommand>>,
        dialect: &str,
    ) -> (
        AnalysisResult,
        Vec<super::super::snapshot::AnalyserSnapshot>,
    ) {
        let retained = self.retain_walk_source(source);
        let source: &str = &retained;
        self.original_static_source_names.clear();
        // Same-source memo, cleared with the source it was derived from.
        self.irules_event_bodies = None;
        self.irules_debug_flags = None;
        let tk_ambient = self.resolve_walk_environment(dialect);
        if let Some(result) = self.unavailable_analysis_result(dialect) {
            return (result, Vec::new());
        }
        self.result.dialect = dialect.to_string();
        self.result.body_lexer_config = Some(self.file_lexer_config());
        self.result.lexical_declaration_advice = self.selected_logical_declaration_advice();
        self.result.resolved_input = Some(self.resolved_analysis_input());
        self.result.library_versions = self.library_versions.clone();
        self.tk_accumulation_enabled = super::super::tk_checks::tk_checks_could_apply(
            source,
            tk_ambient,
            &self.package_provides,
        );
        self.directive_provides = super::super::utils::parse_provides_directives(source);
        self.tk_ambient = tk_ambient;
        // §5.4 range targeting — same resolution as `analyse`.
        self.resolve_declared_targets(source);
        self.unresolved_commands_emitted = false;

        let file_codes = super::super::utils::parse_file_suppression(source);
        for code in &file_codes {
            self.disabled_diagnostics.insert(code.clone());
        }
        if !file_codes.is_empty() {
            // Record the same ``result.suppressed_lines[-1]`` sentinel
            // as ``analyse`` so downstream consumers see file-wide
            // ``# tcl-lsp: disable=`` directives via the same surface
            // regardless of which entry point dispatched the analyse.
            self.result
                .suppressed_lines
                .insert(-1, file_codes.iter().cloned().collect());
        }

        // Build + stash the dialect-aware registry so
        // ``process_command`` 's body-iteration loop has access
        // to it on every chunked / incremental call (the entry
        // point used by the LSP's incremental update path).
        // Without this, body recursion silently no-ops.  Same
        // for the ``line_offsets`` index used by
        // ``apply_preceding_noqa``.
        self.registry = Some(self.profile_registry());
        self.head_identities = self.document_command_realm(source);
        self.result.command_realm = Some(Arc::new(self.head_identities.clone()));
        self.result.original_command_world =
            self.head_identities.original_completed_command_world();
        self.retain_original_comment_suppressions(source);
        self.recovery_known_commands = super::super::utils::recovery_known_commands(
            source,
            self.registry.as_deref().expect("registry just stashed"),
            &self.extra_commands,
            self.file_lexer_config(),
        );
        self.line_offsets = Some(compute_line_offsets(source));
        self.cached_line_index = tcl_lexer::LineIndex::new(source);
        self.cached_line_index_source_len = source.len();

        // Whole-file scoped environment (tclpkg manifests) — same seeding as
        // `analyse`, spanning every chunk's walk.
        let file_env_pushed = self.seed_file_scope_env(source);
        let mut snapshots: Vec<super::super::snapshot::AnalyserSnapshot> =
            Vec::with_capacity(chunk_commands.len());
        for cmds in chunk_commands {
            self.analyse_commands_inner(&cmds);
            snapshots.push(self.snapshot());
        }
        if file_env_pushed {
            self.body_scope_stack.pop();
        }

        // Same structural + diagnostic-emission tail as ``analyse``.
        self.record_literal_parameter_definitions();
        self.retain_original_callback_signature_lookups(source);
        self.run_diagnostic_emitters(source);

        let mut result = std::mem::take(&mut self.result);
        result.source_words_memo.clear();
        self.clear_run_state();
        (result, snapshots)
    }

    /// Analyse pre-segmented commands without re-segmenting `source`.
    ///
    /// This is the single-chunk variant used by the LSP's incremental
    /// path after a prior `restore` — the analyser starts from
    /// a snapshot covering earlier clean chunks, then walks the
    /// dirty chunk's commands through the dispatcher.
    ///
    /// When `finalise` is `true` the diagnostic-emission tail
    /// (orchestrator + filters) runs.  When `false` only the
    /// command walk happens — the caller is building a partial
    /// snapshot and will run the tail later.
    pub fn analyse_commands(
        &mut self,
        source: &str,
        commands: &[tcl_source::segment::SegmentedCommand],
        dialect: &str,
        finalise: bool,
    ) -> AnalysisResult {
        let retained = self.retain_walk_source(source);
        let source: &str = &retained;
        self.original_static_source_names.clear();
        // Same-source memo, cleared with the source it was derived from.
        self.irules_event_bodies = None;
        self.irules_debug_flags = None;
        let tk_ambient = self.resolve_walk_environment(dialect);
        if let Some(result) = self.unavailable_analysis_result(dialect) {
            return result;
        }
        self.result.dialect = dialect.to_string();
        self.result.body_lexer_config = Some(self.file_lexer_config());
        self.result.lexical_declaration_advice = self.selected_logical_declaration_advice();
        self.result.resolved_input = Some(self.resolved_analysis_input());
        self.result.library_versions = self.library_versions.clone();
        self.tk_accumulation_enabled = super::super::tk_checks::tk_checks_could_apply(
            source,
            tk_ambient,
            &self.package_provides,
        );
        self.directive_provides = super::super::utils::parse_provides_directives(source);
        self.tk_ambient = tk_ambient;
        // §5.4 range targeting — same resolution as `analyse`.
        self.resolve_declared_targets(source);
        self.unresolved_commands_emitted = false;

        let file_codes = super::super::utils::parse_file_suppression(source);
        for code in &file_codes {
            self.disabled_diagnostics.insert(code.clone());
        }
        if !file_codes.is_empty() {
            // The same ``-1`` sentinel ``analyse`` uses — see the
            // matching block in ``analyse_chunked``.
            self.result
                .suppressed_lines
                .insert(-1, file_codes.iter().cloned().collect());
        }

        // Same registry + line-index prelude as
        // ``analyse_chunked`` — see that doc-comment.  Without
        // these the registry-driven body loop in
        // ``process_command`` silently skips body recursion on
        // the incremental path.
        self.registry = Some(self.profile_registry());
        self.head_identities = self.document_command_realm(source);
        self.result.command_realm = Some(Arc::new(self.head_identities.clone()));
        self.result.original_command_world =
            self.head_identities.original_completed_command_world();
        self.retain_original_comment_suppressions(source);
        self.recovery_known_commands = super::super::utils::recovery_known_commands(
            source,
            self.registry.as_deref().expect("registry just stashed"),
            &self.extra_commands,
            self.file_lexer_config(),
        );
        self.line_offsets = Some(compute_line_offsets(source));
        self.cached_line_index = tcl_lexer::LineIndex::new(source);
        self.cached_line_index_source_len = source.len();

        // Stub-directive pre-scan + overlay, matching ``analyse`` so command
        // resolution (W123 / W307 / param-trait inference) sees the same stub
        // surface and ``analyse_commands`` stays byte-identical to ``analyse``.
        let (overlay_cmds, overlay_exprs, outside) =
            super::super::utils::document_stub_declarations_with_outside(
                source,
                self.file_path.as_deref(),
                dialect,
                self.file_lexer_config(),
            );
        self.result
            .diagnostics
            .extend(super::super::utils::outside_stub_diagnostics(outside));
        self.declared_commands = Some(self.with_workspace_declarations(
            super::super::types::build_declared_surface(&overlay_cmds),
        ));
        self.result.stub_commands = overlay_cmds;
        self.result.stub_expr_defs = overlay_exprs;

        if self.source_region_override.is_some() {
            self.seed_walk_namespace(source);
            self.current_scope_path.clone_from(&self.seed_scope_path);
        }
        let file_env_pushed = self.seed_file_scope_env(source);
        self.analyse_commands_inner(commands);
        if file_env_pushed {
            self.body_scope_stack.pop();
        }

        if finalise {
            self.record_literal_parameter_definitions();
            self.retain_original_callback_signature_lookups(source);
            self.run_diagnostic_emitters(source);
        }

        let mut result = std::mem::take(&mut self.result);
        result.source_words_memo.clear();
        self.clear_run_state();
        result
    }

    /// Incrementally analyse `new_text` given the previous document text
    /// and its top-level segmentation (`prev_commands` from
    /// [`tcl_source::segment::segment_commands`]). Reuses the unchanged
    /// command prefix via [`tcl_source::segment::segment_commands_incremental`]
    /// — avoiding a re-lex of everything before the edit — then runs the
    /// full analysis over the spliced command stream via
    /// [`Self::analyse_commands`].
    ///
    /// **Correctness-first.** The result is identical to a full
    /// [`Self::analyse`] of `new_text` (pinned by a differential fuzz
    /// oracle). The incremental path uses the plain segmenter and the
    /// pre-segmented `analyse_commands` entry, which match the full
    /// `analyse` only when `new_text` needs no error recovery and carries
    /// no inline stub directives; those cases fall back to a full
    /// re-analysis. The fast path is the common well-formed edit.
    pub fn analyse_incremental(
        &mut self,
        prev_text: &str,
        prev_commands: &[tcl_source::segment::SegmentedCommand],
        new_text: &str,
        dialect: &str,
    ) -> AnalysisResult {
        self.resolve_walk_environment(dialect);
        if let Some(result) = self.unavailable_analysis_result(dialect) {
            return result;
        }
        // Error recovery (ghost-token re-lex, stray-closer repair) and
        // inline `# tcl-lsp: stub` overlays are only applied on the full
        // `analyse` path; when either could be in play, re-analyse fully
        // so the incremental result can never diverge.
        if tcl_lexer::script_image_is_complete(
            &tcl_lexer::SourceImage::document(new_text),
            self.file_lexer_config(),
        ) != Ok(true)
            || new_text.contains("tcl-lsp: stub")
        {
            return self.analyse(new_text, dialect);
        }
        let cmds = tcl_source::segment::segment_commands_incremental(
            prev_text,
            prev_commands,
            new_text,
            // The document's whole-file grammar, resolved once through the
            // ingress exactly as `analyse` resolves it for the full walk.
            self.file_lexer_config(),
        );
        // `analyse` segments with *error recovery*; the fast path uses plain
        // incremental segmentation. Configured completeness accepts terminal
        // syntax cuts as complete; that does not prove the recovery stream.
        // A locally-unbalanced brace can still make recovery segmentation
        // split off a partial command — emitting E200/E102/E202 that the plain
        // walk never sees.  Replicate the recovery segmentation `analyse` uses
        // and fall back to a full rebuild when it differs from what the fast
        // path would walk, or leaves any partial command.  (This also subsumes
        // the plain-segmentation-metadata check: the recovery segmenter is the
        // authority on the command stream + its attached comments.)
        let generation = self.analysis_context();
        let known: std::collections::HashSet<&str> =
            generation.commands().command_names().collect();
        let recovery_cmds = tcl_source::segment::segment_commands_with_recovery_and_config(
            new_text,
            &known,
            self.file_lexer_config(),
        );
        if recovery_cmds != cmds || recovery_cmds.iter().any(|c| c.is_partial) {
            return self.fresh_full_analyse(new_text, dialect);
        }
        let result = self.analyse_commands(new_text, &cmds, dialect, true);
        // Correctness fallback (the maintainer's mandate: incremental must
        // always converge to a full rebuild).  The pre-segmented
        // `analyse_commands` walk is byte-identical to a full `analyse` for
        // well-formed input (verified over the whole corpus), but it diverges
        // from `analyse`'s error-*recovery* handling when the document carries
        // syntax errors: `analyse` runs ghost-token re-lexing,
        // recovery-segmentation, and body-level partial detection that the
        // pre-segmented entry only partially mirrors.  Every observed
        // `incremental != fresh` divergence on a well-segmented document carries
        // at least one syntax-error (E-code) diagnostic, so when the fast path
        // reports any such error we re-analyse fully — guaranteeing the result
        // equals a from-scratch `analyse`.  The fast path stays the common
        // well-formed edit; only mid-edit broken states (transient) take the
        // slow path.
        if result.diagnostics.iter().any(|d| d.code.is_error()) {
            return self.fresh_full_analyse(new_text, dialect);
        }
        result
    }

    /// Run a full [`Self::analyse`] on a *fresh* analyser carrying this one's
    /// config.  The incremental fast path consumes per-walk state (the scope
    /// walk leaves `var_command_sites` / `ensemble_namespaces` / … populated and
    /// `clear_run_state` resets only the registry + line index), so a second
    /// `analyse` on `self` would run on dirty state; a fresh analyser is the
    /// safe way to take the full-rebuild fallback mid-call.
    ///
    /// A supplied `cu_override` describes the same source, so it moves to the
    /// fresh analyser rather than paying for a second whole-file unit build.
    pub(in super::super) fn fresh_full_analyse(
        &mut self,
        new_text: &str,
        dialect: &str,
    ) -> AnalysisResult {
        let mut fresh = Analyser::with_disabled_diagnostics(self.disabled_diagnostics.clone())
            .with_resolved_input(self.resolved_analysis_input())
            .with_non_ascii_mode(self.non_ascii_mode)
            .with_file_path(self.file_path.clone());
        // The same configured analysis the fast path ran under: dropping the
        // driver's workspace, target and version settings here would give the
        // fallback (taken on most edits) a different answer from the fast path.
        super::super::source_settings::OriginalAnalyserSourceSettings::capture(self)
            .apply(&mut fresh);
        fresh.defer_proc_bodies = false;
        fresh
            .source_analysis_entry
            .clone_from(&self.source_analysis_entry);
        fresh.cu_override = self.cu_override.take();
        if let Some((source, inputs)) = self.per_item_realm_inputs.take()
            && source == new_text
        {
            fresh.prebuilt_realm = Some(Box::new(PrebuiltRealm {
                source,
                inputs,
                realm: std::mem::take(&mut self.head_identities),
                command_trust: self.command_trust.take(),
            }));
        }
        fresh.analyse(new_text, dialect)
    }

    /// Extend conditional path source advice through the same actual input,
    /// original realm and complete image as command analysis. Batches preserve
    /// prior lexical homes; namespace seeds remain constructed source scopes.
    pub(super) fn record_path_constant_candidates(
        &mut self,
        commands: &[tcl_source::segment::SegmentedCommand],
    ) {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let mut inventory = std::mem::take(&mut self.result.path_constant_assignments);
        crate::auto_path_eval::extend_path_constant_assignments_from_analysis_commands(
            &self.source,
            &self.result,
            commands,
            self.path_constant_source_namespace.as_deref(),
            &mut inventory,
        );
        self.result.path_constant_assignments = inventory;
    }

    /// Inner dispatch loop shared by [`Self::analyse_chunked`]
    /// and [`Self::analyse_commands`].  Walks pre-segmented
    /// commands at the current scope path.  Covers the dispatch
    /// portion that's load-bearing for incremental analysis.
    fn analyse_commands_inner(&mut self, commands: &[tcl_source::segment::SegmentedCommand]) {
        self.result.global_scope.naming_scope = self.declaration_namespace_scope(&[]);
        let scope_path = self.current_scope_path.clone();
        let total = commands.len();
        let mut cmd_idx: usize = 0;
        while cmd_idx < total {
            if crate::analysis_control::checkpoint(
                crate::analysis_control::AnalysisCheckpoint::Command,
            ) {
                break;
            }
            let cmd_ref = &commands[cmd_idx];
            if cmd_ref.argv.is_empty() {
                cmd_idx += 1;
                continue;
            }
            if cmd_ref.is_partial {
                // Partial commands surface E103 / E200 in the chunked
                // path too so the LSP shows parse errors during
                // incremental analysis.  Stolen-close-brace (E103) is
                // brace-only.
                let brace_partial = matches!(
                    cmd_ref.partial_delimiter,
                    Some(tcl_source::segment::UnclosedDelimiter::Brace)
                );
                if !(brace_partial && self.detect_stolen_close_brace(cmd_ref)) {
                    self.emit_partial_command_diagnostic(cmd_ref);
                }
                cmd_idx += 1;
                continue;
            }
            // Emit the same source-recovery syntax diagnostics as the
            // top-level loop in ``analyse``: E100 / E102 stray closers
            // *and* E201 (unterminated `[`) / E202 / E203 (unterminated
            // `"` / `{`).  ``analyse_commands`` never runs ghost
            // recovery, so pass ``false``.  Run on the original token
            // stream before ``recover_stray_close_bracket`` repairs the
            // clone.  (Nested bodies dispatched below are covered by
            // ``analyse_body``'s own per-body check.)
            self.emit_syntax_recovery_diagnostics(cmd_ref, false);
            // Repair stray ``]`` and missing ``{`` in a clone of the
            // segmented command before dispatch — chunked analysis keeps
            // the original snapshot copies untouched so re-runs are
            // deterministic.
            let mut cmd = cmd_ref.clone();
            self.recover_stray_close_bracket(&mut cmd);
            let consumed = self.recover_missing_open_brace(&mut cmd, commands, cmd_idx);
            if let Some(line_offsets) = self.line_offsets.as_deref() {
                super::super::utils::apply_preceding_noqa(
                    &cmd,
                    line_offsets,
                    &mut self.result.suppressed_lines,
                );
            }
            self.last_comment = cmd.preceding_comment.clone().unwrap_or_default();
            self.process_command(
                &cmd.texts,
                &cmd.argv,
                &cmd.single_token_word,
                cmd.expand_word.as_deref().unwrap_or(&[]),
                &scope_path,
            );
            // Like the top-level loop: W216 (brace-then-paren
            // name/value confusion) fires on top-level commands here too.
            self.emit_w216_brace_then_paren(&cmd);
            self.record_arg_var_reads(&cmd, &scope_path);
            cmd_idx += 1 + consumed;
        }
        self.record_path_constant_candidates(commands);
    }
}
