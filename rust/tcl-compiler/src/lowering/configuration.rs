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

//! Constructing and configuring a `Lowerer`: dialect, registries, source metadata, body caches and the bytecode entry.

use super::source_body_syntax::SourceBodySyntaxCache;
use super::{
    BodyCacheFn, CompileTarget, HostCommandTable, Lowerer, LoweringMetadataOrigin,
    OriginalBodyLoweringCache, SourceBodyMode, SourceEntryOrigin, StandaloneSourceInput, WordSpace,
};
use crate::alias::CommandAliasMap;
use crate::command_binding::SourceAnalysisOptions;
use crate::ir::Module;
#[cfg(test)]
use crate::lowering::SourceBodySyntaxCacheVerification;
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;
use tcl_registry::events::IrulesExecutionContext;

/// [`ContextRegistry::with_command_store`] is a pure function of the
/// context generation and the store, and a lowering rebuilds it for every
/// snippet; reuse a few recent assemblies. Held handles keep both addresses
/// from being reused while an entry remains.
fn context_with_command_store(
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
    commands: std::sync::Arc<CommandRegistry>,
) -> std::sync::Arc<tcl_registry::model::ContextRegistry> {
    type Entry = (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        std::sync::Arc<CommandRegistry>,
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
    );
    const MAX_RETAINED: usize = 8;
    static MEMO: std::sync::OnceLock<std::sync::Mutex<Vec<Entry>>> = std::sync::OnceLock::new();
    let memo = MEMO.get_or_init(Default::default);
    let lock = || {
        memo.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    };
    if let Some((_, _, assembled)) = lock().iter().find(|(base, store, _)| {
        std::sync::Arc::ptr_eq(base, context) && std::sync::Arc::ptr_eq(store, &commands)
    }) {
        return std::sync::Arc::clone(assembled);
    }
    let assembled =
        std::sync::Arc::new(context.with_command_store(std::sync::Arc::clone(&commands)));
    let mut entries = lock();
    if entries.len() >= MAX_RETAINED {
        entries.remove(0);
    }
    entries.push((
        std::sync::Arc::clone(context),
        commands,
        std::sync::Arc::clone(&assembled),
    ));
    assembled
}

impl<'r> Lowerer<'r> {
    /// Create a new lowerer with the default (Tcl-8.5+) lexer config.
    #[must_use]
    pub fn new(registry: &'r CommandRegistry) -> Self {
        // dialect-drift-ok: the test-only constructor — every production
        // construction now goes through `Lowerer::with_config` with the
        // unit's own config; the crate's (and downstream crates') test
        // fixtures still spell the plain-Tcl default this way.
        Self::with_config(registry, tcl_lexer::LexerConfig::default())
    }

    /// Create a lowerer with an explicit dialect [`tcl_lexer::LexerConfig`]
    /// (see the `config` field).
    #[must_use]
    pub fn with_config(registry: &'r CommandRegistry, config: tcl_lexer::LexerConfig) -> Self {
        Self {
            module: Module::default(),
            aliases: CommandAliasMap::new(),
            source_bindings: None,
            original_native_compiler_formals: None,
            original_body_word_source: None,
            module_source_bindings: None,
            compilation_scope: tcl_runtime_api::SourceCompilationScope::WholeModule,
            trusted_package_loaders: Vec::new(),
            trusted_source_modules: Vec::new(),
            unknown_entry: false,
            host_command_table: HostCommandTable::Catalogue,
            source_entry_origin: SourceEntryOrigin::Authoring,
            supplied_source: None,
            invocation_realm: tcl_dialect::model::InvocationRealm::RuleLoader,
            invocation_dialect: Some(crate::environment_ingress::authoring_invocation_dialect(
                registry, None, config,
            )),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            compiled_variable_provider: None,
            expression_parser: None,
            hosted_execution_context: None,
            execution_name_policy: None,
            metadata_input: None,
            logical_source_input: None,
            vendor_source_input: None,
            native_entry: None,
            source_point_entry: None,
            analyser_source_settings: None,
            source_channel: tcl_lexer::SourceChannel::Document,
            incoming_formals: Vec::new(),
            class_instance_vars: HashMap::new(),
            when_counts: HashMap::new(),
            irules_execution_context: IrulesExecutionContext::TopLevel,
            irules_priority: 500,
            body_unit_count: 0,
            in_namespace_eval: false,
            registry,
            declared_commands: None,
            source_declared_commands: None,
            const_map_stack: Vec::new(),
            command_binding_site_stack: Vec::new(),
            structured_source_parent: None,
            proc_depth: 0,
            namespace_imports: Vec::new(),
            namespace_exports: Vec::new(),
            namespace_directive_sites: HashSet::new(),
            dead_code_depth: 0,
            suppress_proc_register: false,
            readonly_logical_body: SourceBodyMode::Ordinary,
            readonly_native_declaration_body: SourceBodyMode::Ordinary,
            native_source_children: None,
            readonly_hosted_body: SourceBodyMode::Ordinary,
            readonly_authored_body: SourceBodyMode::Ordinary,
            authored_source_scope: None,
            config,
            dialect: None,
            dialect_context: None,
            metadata_origin: LoweringMetadataOrigin::Standalone,
            target: CompileTarget::Analysis,
            body_cache: None,
            original_body_cache: None,
            optimiser_lowering_integrity: None,
            source_body_syntax_cache: None,
            #[cfg(test)]
            verify_source_body_syntax_cache: SourceBodySyntaxCacheVerification::Disabled,
            nest_depth: 0,
            source: String::new(),
            word_space: WordSpace::new("", 0, tcl_lexer::SourceChannel::Document),
        }
    }

    pub(crate) fn with_original_optimiser_lowering(
        mut self,
        cache: Option<&'r OriginalBodyLoweringCache<'r>>,
        integrity: &'r std::cell::Cell<bool>,
    ) -> Self {
        self.original_body_cache = cache;
        self.optimiser_lowering_integrity = Some(integrity);
        self
    }

    /// Install the document's own command declarations (see
    /// [`declared_commands`](Self::declared_commands)).
    ///
    /// A stub declares the same kind of fact a `CommandSpec` states, so it
    /// reaches lowering through the same argument-role query the catalogue
    /// does — [`Self::command_surface`] — rather than a parallel table.
    #[must_use]
    pub fn with_declared_commands(
        mut self,
        declared: Option<&'r tcl_registry::model::DeclaredSurface>,
    ) -> Self {
        self.declared_commands = declared;
        self
    }

    /// The driver's module contracts, else the workspace modules the document
    /// input offers for auto-loading.
    pub(super) fn effective_source_modules(
        &self,
    ) -> &[crate::command_binding::TrustedSourceModuleLoader] {
        if self.trusted_source_modules.is_empty() {
            self.metadata_input
                .as_ref()
                .or(self.logical_source_input.as_ref())
                .or(self.vendor_source_input.as_ref())
                .map_or(
                    &[],
                    crate::analyser::ResolvedAnalysisInput::workspace_source_modules,
                )
        } else {
            &self.trusted_source_modules
        }
    }

    /// Set the document's analysis dialect (see [`Lowerer::dialect`]).
    ///
    /// `None` means the caller named *no* dialect — deliberately distinct
    /// from `Some(plain_tcl)`, which is an explicit plain-Tcl document. The
    /// two select different numeral sources downstream (`None` defers to the
    /// thread-ambient target; `Some` pins the profile's grammar), so a caller
    /// holding an optional ingress spelling must map the unstated case to
    /// `None` rather than defaulting to the plain profile. Matches
    /// [`crate::compilation_unit::UnitBuildOptions::dialect`].
    #[must_use]
    pub fn with_dialect(mut self, dialect: Option<&'static tcl_dialect::DialectProfile>) -> Self {
        self.retain_authoring_dialect(dialect);
        if self.source_entry_origin == SourceEntryOrigin::Authoring
            && self.metadata_origin == LoweringMetadataOrigin::Standalone
        {
            self.dialect_context = dialect.map(|profile| {
                let context = crate::environment_ingress::context_for_profile(profile);
                if context.commands().snapshot().semantic_key()
                    == self.registry.snapshot().semantic_key()
                {
                    context
                } else {
                    context_with_command_store(&context, self.registry.snapshot().shared_registry())
                }
            });
        }
        self
    }

    fn retain_authoring_dialect(&mut self, dialect: Option<&'static tcl_dialect::DialectProfile>) {
        self.dialect = dialect;
        if self.source_entry_origin == SourceEntryOrigin::Authoring {
            self.invocation_dialect =
                Some(crate::environment_ingress::authoring_invocation_dialect(
                    self.registry,
                    dialect,
                    self.config,
                ));
        }
    }

    /// Explicit authoring ingress: a profile supplies grammar and metadata
    /// recipes, independently of any supplied availability or runtime entry.
    pub(crate) fn for_standalone_source(
        registry: &'r CommandRegistry,
        config: tcl_lexer::LexerConfig,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Self {
        let mut lowerer = Self::with_config(registry, config);
        lowerer.retain_authoring_dialect(dialect);
        lowerer
    }

    /// Explicit Standalone driver with complete, independently retained metadata.
    /// Supplied-document ingress never calls this constructor or obtains its model.
    pub(crate) fn for_standalone_source_with_input(owner: StandaloneSourceInput<'r>) -> Self {
        let input = owner.input.clone();
        let mut lowerer = Self::for_standalone_source(
            owner.registry,
            input.lexer_config(),
            Some(input.unit_profile()),
        );
        lowerer.dialect_context = Some(input.context_registry());
        // A rewrite of standalone source must hold wherever it later runs,
        // including under a host that registers its own commands.
        lowerer.host_command_table = HostCommandTable::Extensible;
        lowerer.retain_standalone_source_metadata(input);
        lowerer
    }

    pub(super) fn retain_standalone_source_metadata(
        &mut self,
        input: crate::analyser::ResolvedAnalysisInput,
    ) {
        if input.has_logical_source_name_context() || input.has_hosted_source_name_context() {
            self.retain_resolved_analysis_input(&input);
            drop(input);
        } else {
            // The explicit driver already selected authoring independently. Metadata
            // retains its exact store/configuration without replacing that purpose.
            self.metadata_input = Some(input);
            self.metadata_origin = LoweringMetadataOrigin::Supplied;
        }
    }

    /// The host may register commands the catalogue does not describe, so an
    /// undefined head is not bounded by the stock `unknown`.
    #[must_use]
    pub const fn with_host_extensible_command_table(mut self) -> Self {
        self.host_command_table = HostCommandTable::Extensible;
        self
    }

    /// Use the caller's complete availability context for conditional source
    /// metadata. A foreign command generation declines instead of resolving
    /// a replacement from the profile label.
    #[must_use]
    pub fn with_context_registry(
        mut self,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> Self {
        // Preserve supplied ownership even when it is foreign. Shared metadata
        // consumers then refuse it instead of rebuilding a profile fallback.
        self.dialect_context = Some(context);
        self.metadata_origin = LoweringMetadataOrigin::Supplied;
        self
    }

    /// Replace the default fresh entry of the retained input with an explicit
    /// unknown prior history.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_explicit_unknown_entry(mut self) -> Self {
        let input = self
            .metadata_input
            .clone()
            .expect("retained analysis input");
        let entry = crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input);
        self.set_source_analysis_options(entry.options());
        self
    }

    /// Retain the complete caller input independently of execution entry.
    /// Missing or foreign source metadata cannot acquire a profile fallback.
    #[must_use]
    pub fn with_resolved_analysis_input(
        mut self,
        input: crate::analyser::ResolvedAnalysisInput,
    ) -> Self {
        self.retain_resolved_analysis_input(&input);
        drop(input);
        self
    }

    fn retain_resolved_analysis_input(&mut self, input: &crate::analyser::ResolvedAnalysisInput) {
        self.dialect = self.dialect.or(Some(input.unit_profile()));
        let fresh = self.source_entry_origin == SourceEntryOrigin::Authoring;
        self.dialect_context = Some(input.context_registry());
        self.metadata_input = Some(input.clone());
        self.metadata_origin = LoweringMetadataOrigin::Supplied;
        if fresh {
            self.set_supplied_source_analysis_input(input, None);
        }
    }

    /// Choose the fresh supplied-document factory, independently of effective naming policy.
    pub(crate) fn set_supplied_source_analysis_input(
        &mut self,
        input: &crate::analyser::ResolvedAnalysisInput,
        settings: Option<&std::sync::Arc<crate::analyser::OriginalAnalyserSourceSettings>>,
    ) {
        let (entry, issued) = crate::command_binding::SuppliedSourceAnalysisIssuance::issue(
            self.registry,
            input,
            self.config,
            self.dialect,
            settings,
            self.declared_commands,
        );
        self.set_source_analysis_options(entry.options());
        self.supplied_source = Some(issued);
    }

    /// Install a memoised per-procedure body-lowering callback (see
    /// [`body_cache`](Self::body_cache) and [`lower_to_ir_with_body_cache`]).
    #[must_use]
    pub fn with_body_cache(mut self, cache: &'r BodyCacheFn<'r>) -> Self {
        self.body_cache = Some(cache);
        self
    }

    /// Cache certified conditional source syntax without borrowing executed-body proofs.
    #[must_use]
    pub fn with_source_body_syntax_cache(mut self, cache: &'r SourceBodySyntaxCache<'r>) -> Self {
        self.source_body_syntax_cache = Some(cache);
        self
    }

    /// Mark this as the bytecode/VM compile path, barriering constructs the
    /// backend can't compile (see [`CompileTarget::Bytecode`]). A no-op if
    /// [`Self::trace_visible`] already upgraded this to
    /// [`CompileTarget::BytecodeTraced`] (still bytecode-backend, just also
    /// trace-visible) — order-independent whichever builder call comes first.
    #[must_use]
    pub fn for_bytecode_backend(mut self) -> Self {
        if self.target == CompileTarget::Analysis {
            self.target = CompileTarget::Bytecode;
            self.select_bytecode_entry();
        }
        self
    }

    /// Suppress every inline/structured lowering hook so the whole lowering
    /// pass produces plain runtime dispatches (see
    /// [`CompileTarget::BytecodeTraced`]). Implies the bytecode backend.
    #[must_use]
    pub fn trace_visible(mut self) -> Self {
        self.target = CompileTarget::BytecodeTraced;
        self.select_bytecode_entry();
        self
    }

    fn select_bytecode_entry(&mut self) {
        if self.source_entry_origin == SourceEntryOrigin::Authoring {
            self.native_compilation.mode =
                tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject;
        }
    }

    /// Supply the execution entry contract before lowering. A previously used
    /// interpreter must pass `unknown_entry`; package advertisements alone do
    /// not establish loader implementation identity.
    pub fn set_source_analysis_options(&mut self, options: SourceAnalysisOptions<'_>) {
        // Explicit replacement withdraws factory intent, even for equal options.
        self.supplied_source = None;
        let options = if options.source_point_entry.is_some_and(|point| {
            point
                .child_state(
                    self.registry,
                    options,
                    Some(options.native_lexer_config(self.config)),
                )
                .is_none()
        }) {
            SourceAnalysisOptions {
                source_point_entry: options.source_point_entry,
                analyser_source_settings: options.analyser_source_settings,
                metadata_context:
                    crate::registry_invocation::InvocationMetadataInput::SuppliedSource(None),
                unknown_entry: true,
                host_extensible_command_table: options.host_extensible_command_table,
                ..SourceAnalysisOptions::default()
            }
        } else {
            options
        };
        self.source_point_entry = options.source_point_entry.cloned();
        self.analyser_source_settings = options.analyser_source_settings.cloned();
        self.compilation_scope = options.compilation_scope;
        self.invocation_realm = options.invocation_realm;
        self.incoming_formals = options.incoming_formals.to_vec();
        self.source_declared_commands = options.declared_commands.cloned();
        self.trusted_package_loaders = options.trusted_package_loaders.to_vec();
        self.trusted_source_modules = options.trusted_source_modules.to_vec();
        self.unknown_entry = options.unknown_entry;
        self.host_command_table = HostCommandTable::of(options.host_extensible_command_table);
        self.source_entry_origin = SourceEntryOrigin::Driver;
        self.config = options.native_lexer_config(self.config);
        self.invocation_dialect = options.source_invocation_dialect(self.config);
        self.native_compilation = options.native_compilation;
        self.compiled_variable_provider = options.compiled_variable_provider;
        self.expression_parser = options.expression_parser;
        self.hosted_execution_context = options.hosted_execution_context;
        self.execution_name_policy = options.execution_name_policy();
        self.logical_source_input = options.logical_source_input.cloned();
        self.vendor_source_input = options.vendor_source_input.cloned();
        if let Some(input) = self
            .logical_source_input
            .as_ref()
            .or(self.vendor_source_input.as_ref())
        {
            self.dialect = self.dialect.or(Some(input.unit_profile()));
            self.dialect_context = Some(input.context_registry());
            self.metadata_origin = LoweringMetadataOrigin::Supplied;
        } else if self.metadata_origin == LoweringMetadataOrigin::Standalone {
            self.dialect_context = None;
        }
        self.native_entry = options
            .native_entry
            .map(|entry| std::sync::Arc::new(entry.clone()));
    }

    /// Retain the original entry channel for checked structural lowering.
    /// The module and source-origin owners retain the image bytes separately;
    /// this policy never grants native compiler admission.
    pub fn set_source_image_channel(&mut self, image: &tcl_lexer::SourceImage) {
        self.source_channel = image.channel();
    }
}
