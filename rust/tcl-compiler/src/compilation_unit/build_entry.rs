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

//! The `CompilationUnit::build*` entry points, which all reach the retained-entry build.

use super::lattice_request::ProcLatticeCache;
use super::{BodyLoweringCache, CompilationUnit, OriginalBodyCacheInputs, UnitBuildOptions};
#[cfg(test)]
use crate::compilation_unit::FunctionUnit;
#[cfg(test)]
use std::collections::HashMap;
use std::sync::Arc;
use tcl_registry::CommandRegistry;

impl CompilationUnit {
    #[cfg(test)]
    pub(crate) fn from_branch_folding_fixture(source: &str, fu: FunctionUnit) -> Self {
        CompilationUnit {
            source: source.into(),
            ir_module: crate::ir::Module {
                retained_source_bindings: None,
                source_metadata_input: None,
                // dialect-drift-ok: a #[cfg(test)] fixture constructor
                lexer_config: tcl_lexer::LexerConfig::default(),
                source_entry: crate::command_binding::SourceAnalysisEntry::default(),
                future_call_sites: Vec::new(),
                installed_procedure_body_units: std::collections::BTreeMap::default(),
                carried_installed_lowerings: crate::ir::CarriedInstalledLowerings::default(),
                original_declaration_body_units: std::collections::BTreeMap::default(),
                original_native_declaration_body_units: std::collections::BTreeMap::default(),
                original_logical_declaration_body_units: std::collections::BTreeMap::default(),
                original_hosted_declaration_body_units: std::collections::BTreeMap::default(),
                original_authored_declaration_body_units: std::collections::BTreeMap::default(),
                irules_event_bodies: HashMap::new(),
                procedure_implementation_bodies: std::sync::Arc::from([]),
                top_level_kind: crate::ir::TopLevelKind::Script,
                top_level_evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
                source: tcl_lexer::SourceImage::default(),
                native_namespace: None,
                original_byte_source_namespace: None,
                top_level_namespace: "::".to_owned(),
                top_level_namespace_context: None,
                dialect: None,
                dialect_profile: None,
                registry_snapshot: None,
                plain_command_dispatch: false,
                top_level: crate::ir::Script::new(),
                procedure_ids: crate::ir::ProcedureIdIssuer::default(),
                original_definition_scripts: std::collections::BTreeMap::new(),
                procedures: crate::ir::ProcedureInventory::default(),
                methods: crate::ir::MethodInventory::default(),
                body_units: crate::ir::ProcedureInventory::default(),
                lambda_body_units: std::collections::BTreeSet::new(),
                redefined_procedures: std::collections::HashSet::new(),
                redefined_methods: HashMap::new(),
                oo_unanalysed_classes: std::collections::HashSet::new(),
                oo_evidence: crate::ir::OoDefinitionEvidence::default(),
                class_relations: Vec::new(),
                namespace_imports: Vec::new(),
                namespace_exports: Vec::new(),
                traced_commands: std::collections::BTreeSet::new(),
                has_dynamic_trace: false,
                traced_variables: std::collections::BTreeSet::new(),
                has_dynamic_variable_trace: false,
                deferred_writes: crate::ir::DeferredWrites::default(),
                reference_bodies: crate::ir::ReferenceBodies::default(),
                declared_frame_effects: std::collections::BTreeMap::new(),
            },
            cfg_module: crate::cfg::CfgModule {
                top_level: fu.cfg.clone(),
                procedures: HashMap::new(),
            },
            command_mutations: crate::command_binding::ModuleCommandMutations::default(),
            source_class_facts: None,
            top_level: fu,
            procedures: HashMap::new(),
            methods: HashMap::new(),
            body_units: HashMap::new(),
            carried_installed_lowerings: HashMap::new(),
            source_body_cfgs: HashMap::new(),
            interproc: None,
            connection_scope: None,
            caller_scope: crate::compilation_unit::UnitCallerScope::default(),
            declared_commands: tcl_registry::model::DeclaredSurface::new(),
            transfers: crate::interprocedural::TransferSummaries::default(),
            optimiser_source_owner: None,
        }
    }

    /// Build a [`CompilationUnit`] by running the pipeline
    /// end-to-end: `lower_to_ir` → `build_cfg` → per-function
    /// SSA / def-use / SCCP.
    ///
    /// `defer_top_level = false` gives analyses the fully-inlined
    /// CFG; passing `true` matches the codegen behaviour where
    /// top-level `foreach` / `catch` / `try` are compiled as
    /// opaque calls.
    ///
    /// A registry built for a profile is that profile's command world, so the
    /// unit analyses under it ([`Self::build_for_profile`]): its procedures
    /// install and its commands bind. A registry with no profile gives the
    /// dialect-blind unit of [`Self::build_standalone_for`].
    #[must_use]
    pub fn build_for(source: &str, registry: &CommandRegistry, defer_top_level: bool) -> Self {
        if let Some(profile) = registry.profile() {
            return Self::build_for_profile(source, registry, defer_top_level, profile);
        }
        Self::build_standalone_for(source, registry, defer_top_level)
    }

    /// A **dialect-blind** standalone unit, whatever profile the registry was
    /// built for: the default (Tcl-8.5+) lexer config and an empty
    /// [`UnitBuildOptions::dialect`], so word tokenisation, the expression
    /// grammar and the fold policy are plain Tcl, and an authored `proc`
    /// stays an authored declaration body rather than an installed procedure.
    /// A caller that knows the dialect must use [`Self::build_for_dialect`],
    /// or an iRules word operator will neither fold nor draw a
    /// constant-condition diagnostic.
    #[must_use]
    pub fn build_standalone_for(
        source: &str,
        registry: &CommandRegistry,
        defer_top_level: bool,
    ) -> Self {
        Self::build_for_with_config(
            source,
            registry,
            defer_top_level,
            // dialect-drift-ok: `build_for` is a test-only entry point —
            // every production build goes through `build_for_with_config` /
            // `build_for_profile` / `build_for_dialect` with the document's
            // own config; the workspace's test fixtures spell the plain-Tcl
            // default this way.
            tcl_lexer::LexerConfig::default(),
        )
    }

    /// Like [`Self::build_for`] but lowers with an explicit dialect
    /// [`tcl_lexer::LexerConfig`] so `{*}` / `}{` tokenisation follows the
    /// document's dialect.
    ///
    /// The *analysis* dialect is still empty — this only fixes word
    /// tokenisation.  Prefer [`Self::build_for_dialect`], which derives the
    /// same config from the dialect name and additionally gives the lowering
    /// and the lattice pipeline that dialect's expression grammar and fold
    /// policy.
    #[must_use]
    pub fn build_for_with_config(
        source: &str,
        registry: &CommandRegistry,
        defer_top_level: bool,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        Self::build_with(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level,
                config,
                dialect: None,
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            None,
        )
    }

    /// Compatibility entry point for a host which has not yet resolved its
    /// dialect name. New compiler-facing callers should resolve at their
    /// boundary and use [`Self::build_for_profile`].
    ///
    /// Derives the lexer config from `dialect`
    /// ([`tcl_lexer::LexerConfig::for_dialect`]) *and* records the dialect in
    /// [`UnitBuildOptions::dialect`], so all three dialect-sensitive layers
    /// agree: word tokenisation, the expression grammar the lowering parses
    /// conditions with, and the fold policy the lattice pipeline runs under.
    /// Pass `""` for plain Tcl.
    ///
    /// `registry` must be the registry for the same dialect
    /// ([`tcl_registry::registry_for_dialect`]).
    #[must_use]
    pub fn build_for_dialect(
        source: &str,
        registry: &CommandRegistry,
        defer_top_level: bool,
        dialect: &str,
    ) -> Self {
        // One environment resolution (centralisation R-a); the unit
        // profile is derived from the resolved canonical id — the `tk`
        // environment keeps its typed additive ingress profile (the old
        // `resolve_known` promotion), everything else its same-named
        // catalogue profile, unknown names the permissive fallback.
        let effective_dialect =
            crate::environment_ingress::resolve_environment(dialect).unit_profile();
        Self::build_with(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level,
                config: tcl_lexer::LexerConfig::from_grammar(effective_dialect.grammar),
                dialect: Some(effective_dialect),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            None,
        )
    }

    /// Build for an already-resolved dialect profile.
    ///
    /// Keeping the profile through this boundary prevents an alias spelling
    /// from selecting a lexer configuration different from the lowering and
    /// analysis facts. The string compatibility wrapper above resolves once
    /// and delegates here.
    #[must_use]
    pub fn build_for_profile(
        source: &str,
        registry: &CommandRegistry,
        defer_top_level: bool,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Self {
        Self::build_with(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level,
                config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                // Carry the caller's profile through unchanged. Round-tripping
                // it through `by_name` is *not* identity: `tk` is an additive
                // command surface with no catalogue entry of its own, so
                // `by_name("tk")` sinks to plain Tcl and the unit would lose
                // both the `tk` spelling and the TK semantic bit. See
                // `build_for_profile_retains_the_tk_set_only_bit`.
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            None,
        )
    }

    /// Build under an explicit [`UnitBuildOptions`] — the entry point a host
    /// with a cross-file view uses to supply
    /// [`UnitBuildOptions::external_call_sites`].
    #[must_use]
    pub fn build_with_options(source: &str, options: UnitBuildOptions<'_>) -> Self {
        Self::build_with(source, options, None, None)
    }

    /// Like [`Self::build_for_with_config`] but routes each procedure's
    /// per-function lattice build through `cache`, a salsa-native memo (see
    /// [`ProcLatticeCache`] / [`LatticeRequest`]).
    ///
    /// Each procedure's body is normalised to offset 0 and handed to `cache`,
    /// which returns the (possibly memoised) offset-0 [`FunctionUnit`]; the
    /// builder then rebases it to the procedure's real position.  A
    /// shifted-but-unchanged body produces an identical request, so it is a
    /// memo hit (and reused, rebased).  The result is byte-identical to
    /// [`Self::build_for_with_config`]; only the redundant SSA/SCCP/type/
    /// rendered recompute for an unchanged procedure body is skipped.
    /// Interprocedural `param_constants` (caller-uniform-literal SCCP seeds) are
    /// folded into the memo key, so a procedure with them still memoises (it
    /// rebuilds only when a caller's literal at that position changes).  The top
    /// level and methods are always built fresh (no stable offset-0 key); the
    /// cross-function interproc taint re-run still runs over the whole unit in
    /// [`Self::with_interprocedural`].
    pub fn build_for_memoized(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: &mut ProcLatticeCache<'_>,
    ) -> Self {
        Self::build_with(source, options, Some(cache), None)
    }

    /// Like [`Self::build_for_memoized`] but also threads a memoised per-procedure
    /// **body-lowering** callback into the lowering phase,
    /// so an unchanged top-level proc's body IR is reused across edits.  The caller
    /// installs it only for context-free files (see [`crate::lowering::Lowerer`]'s
    /// `body_cache`); byte-identity is guarded by the corpus differential gates.
    pub fn build_for_memoized_with_body_cache(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: &mut ProcLatticeCache<'_>,
        body_cache: &BodyLoweringCache<'_>,
    ) -> Self {
        Self::build_with(source, options, Some(cache), Some(body_cache))
    }

    /// Memoise procedure lattices and body lowering under the driver's complete
    /// source input. Execution provenance remains the independent optional entry.
    #[must_use]
    pub fn build_memoized_with_analysis_input(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: &mut ProcLatticeCache<'_>,
        body_cache: &BodyLoweringCache<'_>,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Self {
        Self::build_with_retained_entry(
            source,
            options,
            Some(cache),
            Some(body_cache),
            entry,
            None,
            Some(input),
        )
    }

    /// Memoise conditional source syntax under the original complete driver
    /// input and entry. Execution-oriented body lowering remains independent.
    #[must_use]
    pub fn build_memoized_with_source_body_syntax_cache(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: &mut ProcLatticeCache<'_>,
        body_cache: &BodyLoweringCache<'_>,
        source_body_cache: &crate::lowering::SourceBodySyntaxCache<'_>,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
        input: Option<&crate::analyser::ResolvedAnalysisInput>,
    ) -> Self {
        Self::build_with_retained_entry_and_source_syntax(
            source,
            options,
            Some(cache),
            OriginalBodyCacheInputs {
                legacy: Some(body_cache),
                original: None,
            },
            entry,
            None,
            (input, Some(source_body_cache), None, None),
        )
    }

    /// Memoise original procedure lowering and lattices under the driver's
    /// complete actual source input. Legacy script callbacks remain separate.
    /// With neither an entry nor an input, a dialect-bearing source is the
    /// fresh document [`Self::build_with`] builds, so both paths agree.
    #[must_use]
    pub fn build_memoized_with_original_body_lowering(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: &mut ProcLatticeCache<'_>,
        body_cache: &crate::lowering::OriginalBodyLoweringCache<'_>,
        source_body_cache: &crate::lowering::SourceBodySyntaxCache<'_>,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
        input: Option<&crate::analyser::ResolvedAnalysisInput>,
    ) -> Self {
        let fresh = if entry.is_none() && input.is_none() {
            Self::fresh_document_input(&options)
        } else {
            None
        };
        Self::build_with_retained_entry_and_source_syntax(
            source,
            options,
            Some(cache),
            OriginalBodyCacheInputs {
                legacy: None,
                original: Some(body_cache),
            },
            entry,
            None,
            (
                input.or(fresh.as_ref()),
                Some(source_body_cache),
                None,
                None,
            ),
        )
    }

    /// Memoise original lowering under the actual analyser's producer settings.
    /// Settings retain no execution entry or factory intent of their own.
    #[must_use]
    pub fn build_memoized_with_original_body_lowering_and_source_settings(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: &mut ProcLatticeCache<'_>,
        body_cache: &crate::lowering::OriginalBodyLoweringCache<'_>,
        source_body_cache: &crate::lowering::SourceBodySyntaxCache<'_>,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
        (input, settings): (
            &crate::analyser::ResolvedAnalysisInput,
            &Arc<crate::analyser::OriginalAnalyserSourceSettings>,
        ),
    ) -> Self {
        Self::build_with_retained_entry_and_source_syntax(
            source,
            options,
            Some(cache),
            OriginalBodyCacheInputs {
                legacy: None,
                original: Some(body_cache),
            },
            entry,
            None,
            (Some(input), Some(source_body_cache), None, Some(settings)),
        )
    }

    /// Build with the driver's retained runtime and package-provider entry.
    /// The entry supplies execution provenance independently of catalogue metadata.
    #[must_use]
    pub fn build_with_source_entry(
        source: &str,
        options: UnitBuildOptions<'_>,
        entry: &crate::command_binding::SourceAnalysisEntry,
    ) -> Self {
        Self::build_with_entry(source, options, None, None, Some(entry), None)
    }

    pub(super) fn build_with(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: Option<&mut ProcLatticeCache<'_>>,
        body_cache: Option<&BodyLoweringCache<'_>>,
    ) -> Self {
        // A whole source with a known dialect is a document `tclsh file.tcl`
        // runs in a fresh interpreter, where `proc f` defines `::f`. It takes
        // the same supplied-source entry as a whole-document analysis
        // (`SourceAnalysisEntry::for_supplied_source`); a dialect-blind or
        // foreign command store keeps the standalone authoring purpose.
        if let Some(input) = Self::fresh_document_input(&options) {
            return Self::build_with_retained_entry(
                source,
                options,
                cache,
                body_cache,
                None,
                None,
                Some(&input),
            );
        }
        Self::build_with_entry(source, options, cache, body_cache, None, None)
    }

    /// The complete source input of a dialect-bearing standalone build: its
    /// profile, the dialect's default availability over the caller's exact
    /// command store, and its lexer configuration. Inputs whose metadata does
    /// not correspond (no dialect, a mismatched profile/config, or a store
    /// the availability context cannot carry) return `None`.
    fn fresh_document_input(
        options: &UnitBuildOptions<'_>,
    ) -> Option<crate::analyser::ResolvedAnalysisInput> {
        let profile = options.dialect?;
        let environment = tcl_registry::model::ingress::resolve_environment(profile.name);
        let mut context = environment.default_context_registry();
        if context.commands().snapshot().semantic_key()
            != options.registry.snapshot().semantic_key()
        {
            context = std::sync::Arc::new(
                tcl_registry::model::ingress::static_context_for_profile(profile)
                    .with_command_store(options.registry.snapshot().shared_registry()),
            );
        }
        let input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, options.config);
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            options.registry,
            &input,
            options.config,
            Some(profile),
        )?;
        Some(input)
    }

    /// Build conditional source diagnostics under the caller's complete
    /// availability context. This context supplies no Native execution entry.
    #[must_use]
    pub fn build_with_context_registry(
        source: &str,
        options: UnitBuildOptions<'_>,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> Self {
        Self::build_with_entry(source, options, None, None, entry, Some(context))
    }

    /// Build using the caller's complete source input and independent entry.
    /// With no supplied entry, matching Logical and hosted inputs retain their
    /// source advice and Native inputs start from a fresh interpreter
    /// ([`crate::command_binding::SourceAnalysisEntry::for_supplied_source`]).
    #[must_use]
    pub fn build_with_analysis_input(
        source: &str,
        options: UnitBuildOptions<'_>,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Self {
        Self::build_with_retained_entry(source, options, None, None, entry, None, Some(input))
    }

    /// Build under settings captured by the actual analyser before compilation.
    /// The independent optional entry retains its purpose: absent entries still
    /// use the supplied-document factory and explicit entries remain explicit.
    #[must_use]
    pub fn build_with_analysis_input_and_source_settings(
        source: &str,
        options: UnitBuildOptions<'_>,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
        input: &crate::analyser::ResolvedAnalysisInput,
        settings: &Arc<crate::analyser::OriginalAnalyserSourceSettings>,
    ) -> Self {
        Self::build_with_retained_entry_and_source_syntax(
            source,
            options,
            None,
            OriginalBodyCacheInputs {
                legacy: None,
                original: None,
            },
            entry,
            None,
            (Some(input), None, None, Some(settings)),
        )
    }

    /// Build an explicitly Standalone authoring request with its complete source
    /// metadata. Availability does not choose this purpose: the caller selects it
    /// independently. Missing/foreign store, profile or full grammar refuses.
    /// Supplied-document and driver-entry builders retain their existing purposes;
    /// this constructor supplies no runtime entry or Native activation.
    #[must_use]
    pub fn build_standalone_with_analysis_input(
        source: &str,
        options: UnitBuildOptions<'_>,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        let owner = crate::lowering::StandaloneSourceInput::retain(
            options.registry,
            input,
            options.config,
            options.dialect,
        )?;
        Some(Self::build_with_retained_entry_and_source_syntax(
            source,
            options,
            None,
            OriginalBodyCacheInputs {
                legacy: None,
                original: None,
            },
            None,
            None,
            (Some(input), None, Some(owner), None),
        ))
    }

    pub(super) fn build_with_entry(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: Option<&mut ProcLatticeCache<'_>>,
        body_cache: Option<&BodyLoweringCache<'_>>,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
        context: Option<std::sync::Arc<tcl_registry::model::ContextRegistry>>,
    ) -> Self {
        Self::build_with_retained_entry(source, options, cache, body_cache, entry, context, None)
    }

    fn build_with_retained_entry(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: Option<&mut ProcLatticeCache<'_>>,
        body_cache: Option<&BodyLoweringCache<'_>>,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
        context: Option<std::sync::Arc<tcl_registry::model::ContextRegistry>>,
        input: Option<&crate::analyser::ResolvedAnalysisInput>,
    ) -> Self {
        Self::build_with_retained_entry_and_source_syntax(
            source,
            options,
            cache,
            OriginalBodyCacheInputs {
                legacy: body_cache,
                original: None,
            },
            entry,
            context,
            (input, None, None, None),
        )
    }
}
