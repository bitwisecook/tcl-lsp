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

//! The per-run state: its reset between analyses, the file-scope environment seed, command trust and synthetic names.

use super::Analyser;
use tcl_lexer::Span;

impl Analyser {
    /// Reset transient run state so the next ``analyse`` call
    /// starts from a clean slate.  Called at the end of every
    /// public entry point (``analyse`` / ``analyse_chunked`` /
    /// ``analyse_commands``).
    /// The whole-module command-mutation trust oracle for constant
    /// command-substitution folding, built lazily on first
    /// demand and cached for the rest of this run — see the
    /// [`Self::command_trust`] field doc for why the flow-sensitive
    /// `renamed_commands` map cannot serve here. Returns `None` when no
    /// registry is active (no fold can run then anyway).
    ///
    /// Cost note: building the oracle lowers the whole document once
    /// ([`crate::lowering::lower_to_ir_with_dialect`] +
    /// [`crate::command_binding::scan_module_command_mutations`]). Callers
    /// gate the first call behind [`crate::const_subst::head_may_fold`], so
    /// a document with no foldable `set VAR [cmd …]` never pays it.
    pub(in super::super) fn whole_file_command_trust(
        &mut self,
    ) -> Option<std::sync::Arc<crate::command_binding::ModuleCommandMutations>> {
        if let Some(trust) = &self.command_trust {
            return Some(std::sync::Arc::clone(trust));
        }
        let registry = self.registry.as_deref()?;
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, self.file_lexer_config())
            .with_dialect(
                // `None`, not `Some(plain_tcl)`, when the analysis named no
                // dialect: an unstated dialect is not the same input as an
                // explicit plain-Tcl one, and `Lowerer::dialect` feeds numeral
                // source selection. `self.profile` is always populated (it
                // defaults to the plain fallback), so gate on the recorded
                // spelling the way the pre-refactor `&str` boundary did.
                (!self.result.dialect.is_empty()).then_some(self.profile),
            );
        if let Some(entry) = &self.source_analysis_entry {
            lowerer.set_source_analysis_options(entry.options());
        }
        lowerer.lower(&self.source);
        let module = lowerer.finish_module_for_command_trust(&self.source);
        let trust = std::sync::Arc::new(crate::command_binding::scan_module_command_mutations(
            &module, registry,
        ));
        self.command_trust = Some(std::sync::Arc::clone(&trust));
        Some(trust)
    }

    /// Mint (and record) one offset-keyed synthetic identity —
    /// `@dynns@<off>` / `@dynclass@<off>` / `@autoname@<off>`.  All minting
    /// goes through here so [`Self::minted_synthetic_names`] can never miss
    /// a name the per-item graft later needs to rebase.
    pub(in super::super) fn mint_synthetic_offset_name(
        &mut self,
        marker: &str,
        offset: u32,
    ) -> String {
        debug_assert!(
            crate::naming::SYNTHETIC_OFFSET_MARKERS.contains(&marker),
            "unknown synthetic marker {marker:?}"
        );
        let name = format!("{marker}{offset}");
        self.minted_synthetic_names.insert(name.clone());
        name
    }

    pub(in super::super) fn clear_run_state(&mut self) {
        // Command inventory is scoped to this resolved context, not a profile
        // display label shared by distinct pack generations.
        self.builtin_names = None;
        self.builtin_dialect = None;
        self.registry = None;
        self.context = None;
        self.environment = None;
        self.command_trust = None;
        self.objdefine_bindings.clear();
        self.objdefine_abort_candidates.clear();
        self.objdefine_unresolved_receiver = false;
        self.seed_namespace_key = None;
        self.path_constant_source_namespace = None;
        self.seed_scope_path.clear();
        self.recovery_known_commands.clear();
        self.minted_synthetic_names.clear();
        self.pending_instances = None;
        self.deferred_instance_replays.clear();
        // Normally drained by the replay, but a walk that never runs the
        // post-walk tail (`analyse_commands` with `finalise: false`) leaves
        // entries behind, and a reused analyser must not carry one document's
        // deferred calls into the next.
        self.deferred_class_creations.clear();
        self.concatenated_windows.clear();
        self.script_parameter_bodies.clear();
        self.script_parameter_contracts.clear();
        self.proven_sites.clear();
        self.interval_index_sites.clear();
        self.loop_candidates.clear();
        self.loop_unseen_writes = super::super::bounds_checks::ModuleUnseenWrites::default();
        self.pending_bareword_dispatch_sites = None;
        self.line_offsets = None;
        self.cached_line_index = tcl_lexer::LineIndex::new("");
        self.cached_line_index_source_len = 0;
        self.e207_emitted = false;
        self.defer_proc_bodies = false;
        self.deferred_bodies.clear();
        self.cu_override = None;
        self.per_item_realm_inputs = None;
        self.prebuilt_realm = None;
        self.source_region_override = None;
    }

    /// Seed the whole-file scoped command environment for this document, if
    /// its file name declares one
    /// ([`tcl_registry::scoped::file_scope_env`]) — the file-level analogue
    /// of the `body_scope` push in `dispatch_body_arguments`.  Records the
    /// whole-document region (so the post-walk W123/W120 passes and the LSP
    /// providers resolve scoped heads by position) and pushes the
    /// environment so the in-walk arity / subcommand checks resolve them
    /// too.  Returns whether an environment was pushed, so the caller can
    /// balance the stack after its walk.
    pub(in super::super) fn seed_file_scope_env(&mut self, source: &str) -> bool {
        let Some(env) = self
            .file_path
            .as_deref()
            .and_then(tcl_registry::scoped::file_scope_env)
        else {
            return false;
        };
        self.result
            .scoped_command_regions
            .push(super::super::types::ScopedBodyRegion {
                span: Span::new(0, u32::try_from(source.len()).unwrap_or(u32::MAX)),
                env,
            });
        self.body_scope_stack.push(env);
        true
    }
}
