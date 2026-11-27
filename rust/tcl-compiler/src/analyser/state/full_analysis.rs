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

//! `analyse` and its source-region and namespace-seeded variants: the walk set-up, directives and the top-level command walk.

use super::super::commands::WorkspaceWalkPhases;
use super::super::tk_checks::tk_checks_could_apply;
use super::super::types::AnalysisResult;
use super::super::utils::parse_provides_directives;
use super::{Analyser, compute_line_offsets, merge_noqa_line_suppressions};
use std::sync::Arc;

impl Analyser {
    /// Analyse exactly one issued conditional suffix, reusing its kernel world.
    /// The result remains a partial view, never a whole-document analysis/CU.
    pub fn analyse_source_region(
        &mut self,
        region: &crate::command_binding::OriginalSourceConditionalRegion,
        parent: &AnalysisResult,
        child_prefix: &AnalysisResult,
    ) -> Option<AnalysisResult> {
        if self.source_analysis_entry.is_some()
            || self.source_region_override.is_some()
            || self.cu_override.is_some()
            || self.seed_namespace_key.is_some()
            || !region.matches_dependencies(parent, child_prefix)
            || self
                .resolved_input
                .as_ref()
                .is_some_and(|input| input != region.input())
        {
            return None;
        }
        region.configure_analyser(self);
        self.resolved_input = Some(region.input().clone());
        let source = region.source_image().try_text().ok()?;
        let entry = region.entry()?;
        let contracts = crate::command_binding::SourceDeclaredCommandContracts::for_document(
            source,
            self.file_path.as_deref(),
            region.input(),
            Some(&entry),
            self.workspace_declared_commands.as_deref(),
            self.workspace_source_modules
                .as_deref()
                .map_or(&[][..], Vec::as_slice),
        );
        if !region.accepts_analyser(self)
            || !entry
                .source_point_entry
                .as_ref()?
                .accepts_child_contracts(contracts.with_options(entry.options()))
        {
            return None;
        }
        self.source_analysis_entry = Some(Arc::new(entry));
        self.source_region_override = Some(region.clone());
        self.result.original_source_region = Some(region.clone());
        let previous_scope = std::mem::take(&mut self.current_scope_path);
        let mut result = self.analyse_commands(
            source,
            region.commands(),
            region.input().analyser_profile().name,
            false,
        );
        self.source_analysis_entry = None;
        self.source_region_override = None;
        self.current_scope_path = previous_scope;
        // The unavailable early-return path does not take the per-run result.
        // Never leave this private partial marker on a reused analyser.
        self.result.original_source_region = None;
        result.original_source_region = Some(region.clone());
        region.matches_region_analysis(&result).then_some(result)
    }

    /// Analyse one source-only child view under the exact retained parent point.
    /// Missing or incompatible ownership refuses without standalone reanalysis.
    pub fn analyse_with_source_point_entry(
        &mut self,
        source: &str,
        dialect: &str,
        point: &crate::command_binding::OriginalSourcePointEntry,
    ) -> Option<AnalysisResult> {
        if self.source_analysis_entry.is_some()
            || self.cu_override.is_some()
            || self.seed_namespace_key.is_some()
            || point.namespace_scope().is_none()
            || !point.accepts_analyser_source_settings(self)
        {
            return None;
        }
        self.resolve_walk_environment(dialect);
        let input = self.resolved_analysis_input();
        let context = input.context_registry();
        let entry = point.source_analysis_entry(context.commands(), &input)?;
        let contracts = crate::command_binding::SourceDeclaredCommandContracts::for_document(
            source,
            self.file_path.as_deref(),
            &input,
            Some(&entry),
            self.workspace_declared_commands.as_deref(),
            self.workspace_source_modules
                .as_deref()
                .map_or(&[][..], Vec::as_slice),
        );
        if !point.accepts_child_contracts(contracts.with_options(entry.options())) {
            return None;
        }
        self.source_analysis_entry = Some(Arc::new(entry));
        let result = self.analyse(source, dialect);
        self.source_analysis_entry = None;
        (result.resolved_input.as_ref() == Some(point.input())
            && result.matches_original_source_image(
                &tcl_lexer::SourceImage::document(source),
                input.lexer_config(),
            ))
        .then_some(result)
    }

    /// Analyse a Tcl source for the given dialect, returning a
    /// fully-populated [`AnalysisResult`].
    ///
    /// Drives end-to-end:
    ///
    /// 1. Pre-scans the leading comment block for
    ///    `# tcl-lsp: disable=CODE` directives via
    ///    [`super::super::utils::parse_file_suppression`].
    /// 2. Segments `source` with the registry's known-commands
    ///    set (uses re-segmentation recovery) so unclosed delimiters mid-file
    ///    don't drop later declarations.
    /// 3. Walks each segmented command through
    ///    [`Self::process_command`].
    ///
    /// As [`Self::analyse`], but the whole file is walked **inside**
    /// `source_namespace` (a constructed `::`-rooted key) — the static
    /// equivalent of C Tcl evaluating a `source`d file in the caller's
    /// current namespace.  Relative definitions home under the seed
    /// (`proc helper` → `<seed>::helper`), absolute ones are unaffected, and
    /// bare call sites gain the seeded namespace as their first resolution
    /// candidate — exactly the `namespace eval <seed> { <file> }` semantics,
    /// via the ordinary scope machinery.
    pub fn analyse_with_source_namespace(
        &mut self,
        source: &str,
        dialect: &str,
        source_namespace: &str,
    ) -> AnalysisResult {
        if source_namespace.is_empty() || source_namespace == "::" {
            return self.analyse(source, dialect);
        }
        self.seed_namespace_key = Some(source_namespace.to_owned());
        self.analyse(source, dialect)
    }

    /// Test fixtures with private post-analysis queries retain the genuine result.
    #[cfg(test)]
    pub(crate) fn analyse_and_retain_result_for_test(
        &mut self,
        source: &str,
        dialect: &str,
    ) -> &AnalysisResult {
        self.result = self.analyse(source, dialect);
        &self.result
    }

    /// Own all provisional analysis until its interruption boundary has closed.
    /// A stopped job is consumed, so its partial realm/scope cannot be reused.
    pub fn try_analyse_owned(
        mut self,
        source: &str,
        dialect: &str,
        namespace: Option<&str>,
        control: crate::analysis_control::AnalysisControl,
    ) -> Result<AnalysisResult, crate::analysis_control::AnalysisInterrupted> {
        crate::analysis_control::run(control, || match namespace {
            Some(namespace) => self.analyse_with_source_namespace(source, dialect, namespace),
            None => self.analyse(source, dialect),
        })
    }

    /// `source` is consumed by reference so the analyser can hold
    /// per-walk references back into it; `dialect` is one of
    /// `"tcl"`, `"f5-irules"`, `"irules"`, `"iapps"`, etc. (kept in
    /// the analyser's per-walk state via [`Self::current_event`]
    /// elsewhere; this entry just records it for future use).
    pub fn analyse(&mut self, source: &str, dialect: &str) -> AnalysisResult {
        use crate::analysis_control::{AnalysisCheckpoint, checkpoint};
        use std::collections::HashSet;
        if checkpoint(AnalysisCheckpoint::Start) {
            return AnalysisResult::default();
        }

        let phase_start = ((cfg!(debug_assertions)
            && tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some())
            || tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_WORKSPACE_SCAN").is_some())
        .then(std::time::Instant::now);
        Self::trace_analysis_phase(phase_start, source.len(), "initial");
        self.unresolved_commands_emitted = false;
        // Stash the source so handlers (recovery, diagnostic
        // emitters) can re-slice it.
        let retained = self.retain_walk_source(source);
        let source: &str = &retained;
        self.original_static_source_names.clear();
        self.original_class_definition_body = None;
        let tk_ambient = self.resolve_walk_environment(dialect);
        if let Some(result) = self.unavailable_analysis_result(dialect) {
            return result;
        }
        // Tell pack hooks which dialect they are running under, for the
        // length of this walk. Deriving a hook's `ctx.dialect` from the call's
        // `TclVersion` instead can only spell a release — an iRules document
        // would report `tcl9.0` and no hook could tell a dialect from a
        // version. The guard restores the previous value on the way
        // out, since one worker analyses documents of different dialects in
        // turn.
        let _dialect_scope = tcl_registry::pack_hooks::DialectScope::enter(Some(self.profile.name));
        self.result.dialect = dialect.to_string();
        self.result.body_lexer_config = Some(self.file_lexer_config());
        self.result.lexical_declaration_advice = self.selected_logical_declaration_advice();
        self.result.resolved_input = Some(self.resolved_analysis_input());
        self.result.library_versions = self.library_versions.clone();
        self.tk_accumulation_enabled =
            tk_checks_could_apply(source, tk_ambient, &self.package_provides);
        self.directive_provides = parse_provides_directives(source);
        self.tk_ambient = tk_ambient;
        // §5.4 range targeting: resolve configured + directive-declared
        // version targets for this walk (a no-op for the undeclared
        // majority).
        self.resolve_declared_targets(source);
        // Clear the per-run iRules file-profile memo so a reused analyser
        // instance recomputes it for the new source / dialect.
        self.irules_file_profiles = None;
        self.irules_event_bodies = None;
        self.irules_debug_flags = None;
        self.retain_walk_directives(source, dialect);

        // Segment with re-segmentation recovery so an unclosed delimiter
        // mid-file doesn't drop later top-level declarations.
        // Build the dialect-aware registry once and stash on
        // ``self`` so per-command handlers (registry-driven body
        // iteration in ``process_command``) reuse it.
        self.registry = Some(self.profile_registry());
        Self::trace_analysis_phase(phase_start, source.len(), "realm-start");
        if checkpoint(AnalysisCheckpoint::Realm) {
            return AnalysisResult::default();
        }
        self.head_identities = self.document_command_realm_reusing(source);
        Self::trace_analysis_phase(phase_start, source.len(), "realm-complete");
        if crate::analysis_control::interrupted() {
            return AnalysisResult::default();
        }
        self.result.command_realm = Some(Arc::new(self.head_identities.clone()));
        self.result.original_command_world =
            self.head_identities.original_completed_command_world();
        self.retain_original_comment_suppressions(source);
        self.record_original_formal_variables();
        Self::trace_analysis_phase(phase_start, source.len(), "retained-inventory");
        // Precompute the iRules file-profile stack (no-op off f5-irules) so
        // the per-command IRULE1001 hint can consult it without recomputing.
        self.prepare_source_walk_indexes(source);
        let known_commands: HashSet<&str> = self.recovery_known_commands.iter().collect();
        let mut commands = tcl_source::segment::segment_commands_with_recovery_and_config(
            source,
            &known_commands,
            self.file_lexer_config(),
        );
        drop(known_commands);

        // Ghost-token recovery (see method doc).
        let ghost_recovery_applied = self.apply_ghost_recovery(source, &mut commands);
        self.recovered_source =
            ghost_recovery_applied || commands.iter().any(|command| command.is_partial);
        Self::trace_analysis_phase(phase_start, source.len(), "segmented");

        // Walk each command through the dispatcher.  The dispatcher
        // wires ``recover_stray_close_bracket``,
        // ``recover_missing_open_brace`` (for switch with a forgotten
        // body brace), ``detect_stolen_close_brace`` (E103), and the
        // generic E200 partial-command emitter.
        // A seeded analysis (`analyse_with_source_namespace`) walks the
        // whole file inside the source-site namespace, exactly as if wrapped
        // in `namespace eval <ns> { ... }` — relative definitions re-home,
        // absolute ones stay put, and call-site candidates gain the seeded
        // tier, all through the ordinary scope machinery.
        self.seed_walk_namespace(source);
        let file_env_pushed = self.seed_file_scope_env(source);
        Self::trace_analysis_phase(phase_start, source.len(), "walk-start");
        let workspace_walk = WorkspaceWalkPhases::start();
        self.walk_commands_top_level(&commands, ghost_recovery_applied);
        self.recovered_tail = self.recovered_tail_verdicts(&commands);
        if let Some(trace) = workspace_walk {
            trace.finish(source.len(), self.structure_only);
        }
        Self::trace_analysis_phase(phase_start, source.len(), "walk-complete");
        if crate::analysis_control::interrupted() {
            return AnalysisResult::default();
        }
        if file_env_pushed {
            self.body_scope_stack.pop();
        }

        // Run the diagnostic-emission orchestrator and the post-pass
        // filters:
        //
        // 1. ``emit_unresolved_command_diagnostics``.
        // 2. ``emit_variable_usage_diagnostics``.
        // 3. ``emit_cfg_ssa_diagnostics(source)``.
        // 4. ``apply_disabled_diagnostics`` — filter codes the
        //    caller asked to silence (also covers the
        //    file-suppression directives merged at the top of
        //    ``analyse``).
        // 5. ``dedupe_diagnostics`` — drop exact duplicates and
        //    the line-based suppression pairs.
        // Structure-only mode (item_tree extraction) skips the entire
        // diagnostic-emission tail — the unresolved/arity/variable/CFG-SSA
        // emitters are the dominant cost and produce no structural facts.
        // Record the classes a proc manufactures under a computed name whose
        // value a literal call-site argument proves.  A
        // *structural* fact, so it runs on the item-tree path too — that is
        // the path the workspace class-factory index is computed from, and a
        // metaclass missing there is invisible to every other document.
        self.record_literal_parameter_definitions();
        self.retain_original_callback_signature_lookups(source);
        Self::trace_analysis_phase(phase_start, source.len(), "parameter-definitions");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return AnalysisResult::default();
        }
        if self.structure_only {
            if let Some(registry) = self.registry.as_deref() {
                self.result.retain_logical_class_source_factories(registry);
            }
        } else {
            self.run_diagnostic_emitters(source);
        }
        Self::trace_analysis_phase(phase_start, source.len(), "diagnostics-complete");
        if crate::analysis_control::interrupted() {
            return AnalysisResult::default();
        }

        let mut result = std::mem::take(&mut self.result);
        result.source_words_memo.clear();
        self.clear_run_state();
        Self::trace_analysis_phase(phase_start, source.len(), "complete");
        result
    }

    /// Hold `source` through its document image's buffer and return it.
    pub(in super::super) fn retain_walk_source(&mut self, source: &str) -> Arc<str> {
        let text = tcl_lexer::SourceImage::document(source)
            .shared_text()
            .unwrap_or_else(|| Arc::from(source));
        self.source = Arc::clone(&text);
        text
    }

    fn trace_analysis_phase(
        phase_start: Option<std::time::Instant>,
        source_bytes: usize,
        stage: &str,
    ) {
        if let Some(start) = phase_start {
            eprintln!(
                "ANALYSER_PHASE bytes={source_bytes} stage={stage} ms={}",
                start.elapsed().as_millis()
            );
        }
    }

    fn prepare_source_walk_indexes(&mut self, source: &str) {
        self.compute_irules_file_profiles();
        self.line_offsets = Some(compute_line_offsets(source));
        self.cached_line_index = tcl_lexer::LineIndex::new(source);
        self.cached_line_index_source_len = source.len();
        self.recovery_known_commands = super::super::utils::recovery_known_commands(
            source,
            self.registry.as_deref().expect("registry just stashed"),
            &self.extra_commands,
            self.file_lexer_config(),
        );
    }

    pub(super) fn seed_walk_namespace(&mut self, source: &str) {
        if let Some(point) = self
            .source_analysis_entry
            .as_ref()
            .and_then(|entry| entry.source_point_entry.as_ref())
        {
            let kind = match point.frame().layout() {
                crate::var_resolve::VariableExecutionFrame::Procedure { .. } => {
                    super::super::types::ScopeKind::Proc
                }
                crate::var_resolve::VariableExecutionFrame::Global => {
                    super::super::types::ScopeKind::Global
                }
                _ => super::super::types::ScopeKind::Namespace,
            };
            let mut scope = super::super::types::Scope::new(
                kind,
                point.namespace().display().as_deref().unwrap_or(""),
            );
            scope.naming_scope = point.namespace_scope();
            scope.body_span = Some(tcl_lexer::Span::new(
                0,
                u32::try_from(source.len()).unwrap_or(u32::MAX),
            ));
            self.result.global_scope.children.push(scope);
            self.seed_scope_path = vec![self.result.global_scope.children.len() - 1];
            return;
        }
        if let Some(seed) = self.seed_namespace_key.take() {
            self.path_constant_source_namespace = Some(seed.clone());
            let end = u32::try_from(source.len()).unwrap_or(u32::MAX);
            let mut base: Vec<usize> = Vec::new();
            let policy = self.declaration_name_policy();
            let mut naming_scope = self
                .selected_logical_declaration_advice()
                .then(|| crate::signature_scan::scope::SignatureNamespaceScope::root(policy));
            for segment in crate::naming::key_segments(&seed) {
                naming_scope = naming_scope.and_then(|scope| scope.child(&segment, policy));
                let mut child = super::super::types::Scope::new(
                    super::super::types::ScopeKind::Namespace,
                    &segment,
                );
                child.body_span = Some(tcl_lexer::Span::new(0, end));
                child.naming_scope.clone_from(&naming_scope);
                let parent =
                    super::super::scope::scope_at_mut(&mut self.result.global_scope, &base)
                        .expect("seed scope path is self-built");
                parent.children.push(child);
                base.push(parent.children.len() - 1);
            }
            self.seed_scope_path = base;
        }
    }

    /// Next-line comment syntax is selected after retaining the actual
    /// complete input, declared contracts and original source realm.
    pub(in super::super) fn retain_original_comment_suppressions(&mut self, source: &str) {
        // Every next-line marker spells `noqa`; a source without it carries
        // no marker, so the registry-shaped comment walk has nothing to find.
        if !source
            .as_bytes()
            .windows(4)
            .any(|w| w.eq_ignore_ascii_case(b"noqa"))
        {
            return;
        }
        if let Some(lines) =
            super::super::utils::parse_noqa_line_suppressions_from_analysis(source, &self.result)
        {
            merge_noqa_line_suppressions(&mut self.result.suppressed_lines, lines);
        }
    }

    fn retain_walk_directives(&mut self, source: &str, dialect: &str) {
        // File-suppression pre-scan: merge codes from any
        // top-of-file ``# tcl-lsp: disable=CODE`` directives into
        // ``self.disabled_diagnostics`` so later emitter passes
        // honour them. The constructor-provided
        // ``disabled_diagnostics`` set (LSP user-config) and the
        // file-directive set are unioned — both sources should
        // take effect.
        //
        // File-level suppression also lives in
        // ``result.suppressed_lines[-1]`` (a per-line map keyed by a
        // sentinel ``-1`` for file-wide); merging the codes into
        // ``disabled_diagnostics`` gives the directives effect at the
        // analyser-internal level.
        let file_codes = super::super::utils::parse_file_suppression(source);
        for code in &file_codes {
            self.disabled_diagnostics.insert(code.clone());
        }
        if !file_codes.is_empty() {
            // Record the ``result.suppressed_lines[-1]`` sentinel so
            // downstream consumers (the LSP suppression filter,
            // code-action UX) see the file-wide directive set in one
            // place.
            self.result
                .suppressed_lines
                .insert(-1, file_codes.iter().cloned().collect());
        }
        // Inline ``# tcl-lsp: stub …`` block scan.  After
        // capturing the parsed records, build the per-document
        // overlay so analyser / compiler queries see the
        // user-declared stubs as first-class commands (without
        // mutating the global registry).
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
    }

    /// Walk a top-level command stream through the dispatcher (the body of
    /// `analyse`'s main loop, extracted so the per-item path can reuse it
    /// verbatim).  `scope_path` is the global scope (`&[]`).  When
    /// `defer_proc_bodies` is set, `handle_proc_command` / OO method walks
    /// record the body for later isolated analysis instead of recursing.
    pub(in super::super) fn walk_commands_top_level(
        &mut self,
        commands: &[tcl_source::segment::SegmentedCommand],
        ghost_recovery_applied: bool,
    ) {
        // A source shell interprets nothing.
        if self.source_shell {
            return;
        }
        self.result.global_scope.naming_scope = self.declaration_namespace_scope(&[]);
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
                // An unterminated `"` / `{` emits E202 / E203
                // (with a closing-delimiter fix) instead of the generic
                // E200; only fall through to E103 / E200 when no
                // delimiter-recovery diagnostic applies.
                // Stolen-close-brace detection (E103) only applies to a
                // *brace* partial; a bracket / quote partial goes
                // straight to E200.
                let brace_partial = matches!(
                    cmd_ref.partial_delimiter,
                    Some(tcl_source::segment::UnclosedDelimiter::Brace)
                );
                let region_end = self.source.len();
                if !(self.emit_unterminated_delimiter_diagnostics(cmd_ref, region_end)
                    || brace_partial && self.detect_stolen_close_brace(cmd_ref))
                {
                    self.emit_partial_command_diagnostic(cmd_ref);
                }
                cmd_idx += 1;
                continue;
            }
            let mut cmd = cmd_ref.clone();
            // E100 (stray `]`) / E102 (stray `}`) token checks, run on
            // the *original* token stream before recovery mutates the
            // clone.  Distinct from the recovery repairs below, which
            // handle unclosed openers; a stray closer otherwise goes
            // unreported.  Runs at the top level; nested bodies are
            // covered by ``analyse_body``'s own per-body check.
            self.emit_syntax_recovery_diagnostics(cmd_ref, ghost_recovery_applied);
            self.recover_stray_close_bracket(&mut cmd);
            let consumed = self.recover_missing_open_brace(&mut cmd, commands, cmd_idx);
            let single = cmd.single_token_word.clone();
            // ``# noqa[: CODE,...]`` directives in
            // ``cmd.preceding_comment`` suppress diagnostics on
            // the *following* command's line range.
            if let Some(line_offsets) = self.line_offsets.as_deref() {
                super::super::utils::apply_preceding_noqa(
                    &cmd,
                    line_offsets,
                    &mut self.result.suppressed_lines,
                );
            }
            // Record the preceding comment: handlers that consume one
            // (proc, oo::class) ``std::mem::take`` it; everything else
            // clears it on the next command.
            self.last_comment = cmd.preceding_comment.clone().unwrap_or_default();
            let base_path = self.seed_scope_path.clone();
            self.process_command(
                &cmd.texts,
                &cmd.argv,
                &single,
                cmd.expand_word.as_deref().unwrap_or(&[]),
                &base_path,
            );
            self.emit_w216_brace_then_paren(&cmd);
            self.record_arg_var_reads(&cmd, &base_path);
            cmd_idx += 1 + consumed;
        }
        // After the walk, so the path inventory reads each command's words
        // under the same recorded head metadata as every later consumer.
        self.record_path_constant_candidates(commands);
    }
}
