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

//! The builder methods that configure an analyser: disabled diagnostics, pack overlays, targets, workspace declarations and the retained input.

use super::super::types::AnalysisResult;
use super::{Analyser, NonAsciiMode};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

impl Analyser {
    /// Construct an analyser with a fixed set of diagnostic codes
    /// disabled (e.g. `"W210"`, `"W211"`).
    // One line per field of a struct with more than a hundred of them, which
    // rustfmt will not pack and splitting would only scatter across helpers
    // that each initialise a fifth of one value.
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn with_disabled_diagnostics(disabled: HashSet<String>) -> Self {
        Self {
            authenticated_report_request: None,
            result: AnalysisResult::default(),
            current_scope_path: Vec::new(),
            source: Arc::from(""),
            original_static_source_names: HashMap::new(),
            profile: tcl_dialect::DialectProfile::plain_tcl(),
            ingress_grammar: None,
            unit_profile: None,
            resolved_input: None,
            environment: None,
            context: None,
            pack_overlay: 0,
            disabled_diagnostics: disabled,
            extra_commands: Arc::new(HashSet::new()),
            last_comment: String::new(),
            original_class_definition_body: None,
            file_path: None,
            const_strings: HashMap::new(),
            finite_foreach_names: super::super::handlers::FiniteForeachReportingState::default(),
            logical_const_string_provenance: HashMap::new(),
            logical_interp_path_provenance: HashMap::new(),
            nondominating_consts: HashMap::new(),
            regex_vars: HashSet::new(),
            current_event: None,
            tk_accumulation_enabled: false,
            tk_ambient: false,
            tk_pending_diags: Vec::new(),
            tk_domains: std::collections::BTreeMap::new(),
            version_gate_sites: Vec::new(),
            dsl_gate_sites: Vec::new(),
            proven_sites: Vec::new(),
            interval_index_sites: HashSet::new(),
            loop_candidates: Vec::new(),
            loop_unseen_writes: super::super::bounds_checks::ModuleUnseenWrites::default(),
            pending_option_conflicts: Vec::new(),
            pending_gated_arity: Vec::new(),
            pending_gated_bare_ensemble: Vec::new(),
            library_versions: tcl_dialect::LibraryVersionOverrides::default(),
            declared_targets: Vec::new(),
            package_provides: Vec::new(),
            script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
            directive_provides: Vec::new(),
            range_context: None,
            range_numeral_grammars: Vec::new(),
            range_gate_sites: Vec::new(),
            builtin_names: None,
            builtin_dialect: None,
            conditional_depth: 0,
            control_flow_body_depth: 0,
            irules_debug_gate_depth: 0,
            body_depth: 0,
            presubstituted_args: false,
            evaluated_body_invocation: None,
            e207_emitted: false,
            body_scope_stack: Vec::new(),
            command_aliases: HashMap::new(),
            renamed_commands: HashMap::new(),
            alias_offsets: HashMap::new(),
            rename_offsets: HashMap::new(),
            deleted_commands: HashMap::new(),
            reachable_call_offsets: HashMap::new(),
            namespace_paths: HashMap::new(),
            var_command_sites: Vec::new(),
            pending_const_dispatches: Vec::new(),
            pending_instance_class_sites: Vec::new(),
            divergent_table_literals: HashSet::new(),
            seed_namespace_key: None,
            path_constant_source_namespace: None,
            seed_scope_path: Vec::new(),
            widget_dispatch_sites: Vec::new(),
            cmd_command_sites: Vec::new(),
            ensemble_namespaces: HashSet::new(),
            ensemble_command_maps: HashMap::new(),
            ensemble_record_offsets: HashMap::new(),
            pending_ensemble_subcommands: Vec::new(),
            objdefined_vars: HashSet::new(),
            objdefine_bindings: HashMap::new(),
            objdefine_abort_candidates: Vec::new(),
            objdefine_unresolved_receiver: false,
            interpreters: HashMap::new(),
            dynamic_interp_ops: false,
            interp_epochs: HashMap::new(),
            interp_path_stack: Vec::new(),
            recovered_source: false,
            concatenated_script: false,
            literal_foreach_bindings: Vec::new(),
            concatenated_windows: Vec::new(),
            recovered_tail: None,
            safe_interp_stack: Vec::new(),
            interp_var_bindings: HashMap::new(),
            unresolved_commands_emitted: false,
            registry: None,
            command_trust: None,
            recovery_known_commands: super::super::utils::RecoveryKnownCommands::default(),
            deep_param_traits: false,
            declared_commands: None,
            head_identities: crate::realm::CommandBindingRealm::default(),
            line_offsets: None,
            cached_line_index: tcl_lexer::LineIndex::new(""),
            cached_line_index_source_len: 0,
            static_eval_site: None,
            pending_arity: Vec::new(),
            pending_user_call_arity: Vec::new(),
            pending_ctor_arity: Vec::new(),
            pending_next_arity: Vec::new(),
            non_ascii_mode: NonAsciiMode::Default,
            structure_only: false,
            source_shell: false,
            #[cfg(test)]
            cfg_ssa_capture_for_test: None,
            workspace_classes: std::collections::HashSet::new(),
            workspace_bare_word_classes: std::collections::HashSet::new(),
            workspace_class_factories: None,
            workspace_subclass_methods: None,
            workspace_declared_commands: None,
            workspace_source_modules: None,
            defer_proc_bodies: false,
            pack_definer_seen: false,
            structural_rebind: false,
            deferred_bodies: Vec::new(),
            minted_synthetic_names: std::collections::HashSet::new(),
            cu_override: None,
            per_item_realm_inputs: None,
            prebuilt_realm: None,
            source_analysis_entry: None,
            source_region_override: None,
            capture_global_reads: None,
            capture_global_defs: None,
            pending_disabled_commands: Vec::new(),
            pending_w143: Vec::new(),
            pending_w304: Vec::new(),
            pending_instances: None,
            deferred_instance_replays: Vec::new(),
            pending_bareword_dispatch_sites: None,
            probe_skip_enclosing_fallback: false,
            probe_skip_duplicate_fallback: false,
            took_fast_path: false,
            per_item_fallback: None,
            irules_file_profiles: None,
            irules_event_bodies: None,
            irules_debug_flags: None,
            irules_debug_flags_open: false,
            deferred_class_creations: Vec::new(),
        }
    }

    /// Layer the `SpecTcl` pack set with this identity onto the registry this
    /// analysis reads (`PackSet::key`), returning `self` for builder-style
    /// configuration.
    ///
    /// `0` — the default — means "no packs" and is the plain per-profile
    /// registry. Any other value is looked up, never built: see
    /// [`Self::profile_registry`].
    #[must_use]
    pub fn with_pack_overlay(mut self, key: u64) -> Self {
        self.pack_overlay = key;
        if self.resolved_input.is_none() {
            self.builtin_names = None;
            self.builtin_dialect = None;
            self.context = None;
            self.registry = None;
            self.result.analysis_context_unavailable = None;
        }
        self
    }

    /// Current command store after a checked analysis ingress.
    #[must_use]
    pub fn profile_registry(&self) -> std::sync::Arc<tcl_registry::registry::CommandRegistry> {
        std::sync::Arc::clone(self.analysis_context().commands())
    }

    /// Set the W108 non-ASCII detection mode (`tclLsp.style.nonAscii`),
    /// returning `self` for builder-style configuration.
    #[must_use]
    pub fn with_non_ascii_mode(mut self, mode: NonAsciiMode) -> Self {
        self.non_ascii_mode = mode;
        self
    }

    /// Pin the session's target BIG-IP release (`tclLsp.bigipVersion` /
    /// `--bigip-version`): the keyed library-version axis every declared
    /// range compares against (baseline semantics, §7.1).
    #[must_use]
    pub fn with_bigip_version(mut self, version: Option<String>) -> Self {
        self.library_versions.bigip_version = version;
        self
    }

    /// Set the configuration-declared version targets (`tclLsp.targets`,
    /// §5.4 range targeting) as `(provider, range clauses)` pairs,
    /// returning `self` for builder-style configuration. A source-level
    /// `# tcl-lsp: supports NAME RANGE` directive overrides the
    /// configured pair for the same provider.
    #[must_use]
    pub fn with_declared_targets(mut self, targets: Vec<(String, String)>) -> Self {
        self.declared_targets = targets;
        self
    }

    /// Declare what a `package require` additionally loads
    /// ([`Self::package_provides`]) — `("myExtension", ["Tk"])` says a
    /// document that requires `myExtension` has `Tk` available too.
    #[must_use]
    pub fn with_package_provides(mut self, provides: Vec<(String, Vec<String>)>) -> Self {
        self.package_provides = provides;
        self
    }

    /// Declare the encoding the target reads this script in
    /// (`tclLsp.sourceEncoding`). The caller decoded the text with it, so a
    /// fold over a non-ASCII literal is exact under every release; `Auto`
    /// leaves 8.x folds over such literals declined.
    #[must_use]
    pub const fn with_script_encoding(
        mut self,
        encoding: tcl_registry::value_transfer::ScriptEncoding,
    ) -> Self {
        self.script_encoding = encoding;
        self
    }

    /// Set the user-declared extra command names (`tclLsp.extraCommands`),
    /// returning `self` for builder-style configuration. These names are
    /// treated as known commands by the unknown-command (W123) check.
    #[must_use]
    pub fn with_extra_commands(self, extra: HashSet<String>) -> Self {
        self.with_shared_extra_commands(Arc::new(extra))
    }

    /// [`Self::with_extra_commands`] taking an already-shared set, so a caller
    /// that caches the (potentially very large) widened recovery name set hands
    /// it over as a refcount bump rather than a deep copy.
    #[must_use]
    pub fn with_shared_extra_commands(mut self, extra: Arc<HashSet<String>>) -> Self {
        self.extra_commands = extra;
        self
    }

    /// Enable structure-only mode (see [`Self::structure_only`]): the walk
    /// builds the declaration/scope structure but skips all diagnostic
    /// emission.  Returns `self` for builder-style configuration.  Used by the
    /// `item_tree` query for cheap offset-stable item extraction.
    #[must_use]
    pub fn structure_only(mut self) -> Self {
        self.structure_only = true;
        self
    }

    /// Enable source-shell mode (see [`Self::source_shell`]): keep the
    /// document's source identity and interpret nothing.
    #[must_use]
    pub fn source_shell(mut self) -> Self {
        self.source_shell = true;
        self
    }

    /// The workspace classes whose command bare-word-constructs — see
    /// [`Self::workspace_bare_word_classes`].
    #[must_use]
    pub fn with_workspace_bare_word_classes(
        mut self,
        classes: std::collections::HashSet<String>,
    ) -> Self {
        self.workspace_bare_word_classes = classes;
        self
    }

    /// Supply the fully-qualified names of classes defined elsewhere in the
    /// workspace, so instance inference can resolve a constructor whose class
    /// lives in another file.  Used by the cross-file method reference /
    /// definition path; the normal single-file analysis leaves this empty.
    #[must_use]
    pub fn with_workspace_classes(mut self, classes: std::collections::HashSet<String>) -> Self {
        self.workspace_classes = classes;
        self
    }

    /// Supply the workspace's **class factory** index — the user-defined
    /// `TclOO` metaclasses declared in other documents — so a
    /// `::tk::Megawidget create IconList …` call whose metaclass lives in
    /// another file is classified instead of abstained on.
    ///
    /// See [`Self::workspace_class_factories`]. The normal single-file
    /// analysis leaves this `None`, which keeps the abstention intact.
    #[must_use]
    pub fn with_workspace_class_factories(
        mut self,
        factories: Option<std::sync::Arc<super::super::types::ClassFactoryIndex>>,
    ) -> Self {
        self.workspace_class_factories = factories;
        self
    }

    /// Declare the procedures other workspace documents define that a call
    /// may trust, returning `self` for builder-style chaining.
    #[must_use]
    pub fn with_workspace_declared_commands(
        mut self,
        commands: Option<std::sync::Arc<tcl_registry::model::DeclaredSurface>>,
    ) -> Self {
        self.workspace_declared_commands = commands;
        self
    }

    /// Offer sibling workspace files for auto-loading, returning `self` for
    /// builder-style chaining.
    #[must_use]
    pub fn with_workspace_source_modules(
        mut self,
        modules: Option<std::sync::Arc<Vec<crate::command_binding::TrustedSourceModuleLoader>>>,
    ) -> Self {
        self.workspace_source_modules = modules;
        self
    }

    /// `own` over the workspace declarations: the document's own win.
    pub(in super::super) fn with_workspace_declarations(
        &self,
        own: tcl_registry::model::DeclaredSurface,
    ) -> tcl_registry::model::DeclaredSurface {
        let Some(workspace) = self.workspace_declared_commands.as_deref() else {
            return own;
        };
        // An unavailable generation performs no walk, so nothing reads this.
        let Ok(context) = self.try_analysis_context() else {
            return own;
        };
        own.over_workspace_procedures(Some(workspace), context.commands())
    }

    /// Supply the workspace's **subclass-provided method** view — the
    /// instance methods dispatchable on some descendant of each class, with
    /// the descendants written in other documents — so the template-method
    /// abstention holds across the workspace boundary.
    ///
    /// See [`Self::workspace_subclass_methods`]. The normal single-file
    /// analysis leaves this `None`, which keeps W308 firing on the per-file
    /// evidence alone.
    #[must_use]
    pub fn with_workspace_subclass_methods(
        mut self,
        methods: Option<std::sync::Arc<super::super::types::SubclassProvidedMethods>>,
    ) -> Self {
        self.workspace_subclass_methods = methods;
        self
    }

    /// Set the source-file path, returning `self` for builder-style
    /// configuration.  Path-keyed behaviour: `pkgIndex.tcl` suppresses
    /// dead-store / unused hints for the loader-supplied `$dir`, and a
    /// file with a registry whole-file scoped environment (`tclpkg.tcl`
    /// manifests — [`tcl_registry::scoped::file_scope_env`]) is analysed
    /// with that environment ambient.
    #[must_use]
    pub fn with_file_path(mut self, path: Option<String>) -> Self {
        self.file_path = path;
        self
    }

    /// Retain the driver's actual command/package entry for every analysis phase.
    /// Package advertisements and assistance declarations do not supply this proof.
    #[must_use]
    pub fn with_source_analysis_entry(
        mut self,
        entry: std::sync::Arc<crate::command_binding::SourceAnalysisEntry>,
    ) -> Self {
        self.source_analysis_entry = Some(entry);
        self
    }

    /// Use the driver's retained editing inputs at every full, chunked and
    /// per-item ingress. The dialect string passed to `analyse` remains a
    /// presentation label. Reusing this analyser preserves the explicit input;
    /// no display-name lookup replaces its grammar or command generation.
    #[must_use]
    pub fn with_resolved_input(
        mut self,
        input: super::super::input::ResolvedAnalysisInput,
    ) -> Self {
        self.builtin_names = None;
        self.builtin_dialect = None;
        self.resolved_input = Some(input);
        self
    }
}
