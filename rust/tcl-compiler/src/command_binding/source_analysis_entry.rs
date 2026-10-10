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

//! Immutable source-entry contracts carried into the shared binding driver.
//!
//! Document declarations are retained for `DocumentCommandSurface` assistance.
//! This carrier selects entry policies without querying declaration metadata.

use std::sync::Arc;

use super::{TrustedPackageLoader, TrustedSourceModuleLoader};

/// Immutable document/workspace contract carrier. The document-surface owner
/// remains the only metadata query door; no runtime lookup follows from it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SourceDeclaredCommandContracts(tcl_registry::model::DeclaredSurface);
impl SourceDeclaredCommandContracts {
    pub(crate) fn for_document(
        source: &str,
        file_path: Option<&str>,
        input: &crate::analyser::ResolvedAnalysisInput,
        entry: Option<&SourceAnalysisEntry>,
    ) -> Arc<Self> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let context = input.context_registry();
        let document = crate::analyser::utils::document_declared_surface(
            source,
            file_path,
            input.analyser_profile().name,
        );
        let mut result = entry
            .and_then(|entry| entry.declared_commands.as_ref())
            .cloned()
            .unwrap_or_default();
        let document =
            tcl_registry::model::DocumentCommandSurface::new(context.commands(), Some(&document));
        for name in document.declared_names() {
            if let Some(command) = document.declared_command(name) {
                result.declare(command.clone());
            }
        }
        Arc::new(Self(result))
    }

    pub(crate) fn with_options<'a>(
        &'a self,
        mut options: SourceAnalysisOptions<'a>,
    ) -> SourceAnalysisOptions<'a> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        options.declared_commands = Some(&self.0);
        options
    }

    pub(super) fn document_surface<'a>(
        &'a self,
        registry: &'a tcl_registry::CommandRegistry,
    ) -> tcl_registry::model::DocumentCommandSurface<'a> {
        tcl_registry::model::DocumentCommandSurface::new(registry, Some(&self.0))
    }
}

/// Entry facts supplied to source interpretation rather than inferred from
/// catalogue availability or a package require spelling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourceAnalysisOptions<'a> {
    /// Actual metadata ingress, independent of naming/frame/handler entry.
    /// Supplied missing ownership must retain its explicit tag.
    pub metadata_context: crate::registry_invocation::InvocationMetadataInput<'a>,
    /// Host storage policy supplied independently of source naming and catalogue.
    /// This selects no actual worker, publication, variable lifetime or value.
    pub hosted_execution_context: Option<tcl_registry::f5::BigIpExecutionContext>,
    /// Independently selected naming context, separate from producer strings and compiler recipes.
    pub execution_name_policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    /// Independently selected complete Logical source advice input. This
    /// grants no native name recipe, entered handler or execution authority.
    pub logical_source_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    /// Complete hosted source advice input, independent of runtime naming,
    /// storage, entered frames and native implementation recipes.
    pub vendor_source_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    /// Compilation request's body inventory; independent of native admission.
    pub compilation_scope: tcl_runtime_api::SourceCompilationScope,
    /// Availability phase selected by the entry owner, independent of source origin.
    pub invocation_realm: tcl_dialect::model::InvocationRealm,
    /// Actual live runtime table; supplied rows replace fresh-world assumptions.
    pub native_entry: Option<&'a tcl_runtime_api::NativeCompilationEntry>,
    /// Ordinary incoming local slots proved by the activation binding plan.
    /// Reference aliases and retained static slots must not appear here.
    pub incoming_formals: &'a [String],
    /// Untrusted role assistance, independent of runtime entry bindings.
    pub declared_commands: Option<&'a tcl_registry::model::DeclaredSurface>,
    /// Selected loaders justified by the driver execution environment.
    pub trusted_package_loaders: &'a [TrustedPackageLoader],
    /// Actual file-read contracts selected by the execution driver.
    pub trusted_source_modules: &'a [TrustedSourceModuleLoader],
    /// Prior interpreter history is unavailable; no fresh-table proof applies.
    pub unknown_entry: bool,
    /// Logical handler, value and frame policies supplied by the entry owner.
    /// Physical compiler support is queried separately with `native_compiler_dialect`.
    pub invocation_dialect: Option<tcl_registry::InvocationDialect>,
    /// Explicit authored compiler-local provider, separate from physical engine evidence.
    pub compiled_variable_provider:
        Option<tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider>,
    /// Native script evaluation protocol and compiler frame supplied by the entry owner.
    pub native_compilation: tcl_registry::native_compilation::NativeCompilationContext,
}

pub(super) fn native_compilation_dialect(
    entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
    authored: Option<tcl_registry::InvocationDialect>,
) -> Option<tcl_registry::InvocationDialect> {
    match entry {
        Some(entry) => entry.execution_point.map(|point| {
            let mut dialect = tcl_registry::InvocationDialect::of_point(point);
            if let Some(grammar) = entry.lexer_grammar {
                dialect.lexer_grammar = grammar;
                dialect.word_values =
                    tcl_syntax::word_rules::WordValueRules::from_grammar(&grammar);
            }
            dialect
        }),
        None => authored,
    }
}

pub(crate) fn source_input_dialect(
    input: &crate::analyser::ResolvedAnalysisInput,
) -> tcl_registry::InvocationDialect {
    let mut dialect = tcl_registry::InvocationDialect::of_profile(input.unit_profile());
    dialect.lexer_grammar = input.lexer_config().grammar_over(dialect.lexer_grammar);
    dialect.word_values =
        tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
    dialect
}

impl<'a> SourceAnalysisOptions<'a> {
    /// Source-only driver options from the positive retained Logical input.
    /// The exact word grammar and dialect are kept together; this supplies
    /// no native entry, compiler admission or hosted execution context.
    #[must_use]
    pub fn for_logical_source(input: &'a crate::analyser::ResolvedAnalysisInput) -> Option<Self> {
        input.has_logical_source_name_context().then(|| Self {
            logical_source_input: Some(input),
            metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                Some(input),
            ),
            invocation_dialect: Some(source_input_dialect(input)),
            ..Self::default()
        })
    }

    /// Retain positively selected hosted source advice without assuming runtime
    /// history, storage, a native worker or a TMM execution context.
    #[must_use]
    pub fn for_hosted_source(input: &'a crate::analyser::ResolvedAnalysisInput) -> Option<Self> {
        input.has_hosted_source_name_context().then(|| Self {
            vendor_source_input: Some(input),
            metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                Some(input),
            ),
            invocation_dialect: Some(source_input_dialect(input)),
            unknown_entry: true,
            ..Self::default()
        })
    }
}

impl SourceAnalysisOptions<'_> {
    /// Retain supplied metadata independently of naming and frame facts. An
    /// explicit missing tag remains terminal even when name advice is retained.
    pub(crate) fn retained_metadata_context(
        &self,
    ) -> crate::registry_invocation::OwnedInvocationMetadataContext {
        let owner = self.metadata_context.retain();
        if owner.is_standalone()
            && let Some(input) = self.logical_source_input.or(self.vendor_source_input)
        {
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                input,
            ))
        } else {
            owner
        }
    }

    pub(super) fn retained_declared_command_contracts(
        &self,
    ) -> Option<Arc<SourceDeclaredCommandContracts>> {
        self.declared_commands
            .cloned()
            .map(|surface| Arc::new(SourceDeclaredCommandContracts(surface)))
    }

    pub(crate) fn retained_vendor_source_input(
        &self,
        registry: &tcl_registry::CommandRegistry,
        config: Option<tcl_lexer::LexerConfig>,
    ) -> Option<crate::analyser::ResolvedAnalysisInput> {
        let input = self.vendor_source_input?;
        if self.native_entry.is_some()
            || self.logical_source_input.is_some()
            || !input.has_hosted_source_name_context()
            || config != Some(input.lexer_config())
            || self.source_invocation_dialect(input.lexer_config())
                != Some(source_input_dialect(input))
            || input
                .context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
        {
            return None;
        }
        Some(input.clone())
    }

    pub(crate) fn retained_logical_source_input(
        &self,
        registry: &tcl_registry::CommandRegistry,
        config: Option<tcl_lexer::LexerConfig>,
    ) -> Option<crate::analyser::ResolvedAnalysisInput> {
        let input = self.logical_source_input?;
        let retained_config = input.lexer_config();
        let mut dialect = tcl_registry::InvocationDialect::of_profile(input.unit_profile());
        dialect.lexer_grammar = retained_config.grammar_over(dialect.lexer_grammar);
        dialect.word_values =
            tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
        if self.native_entry.is_some()
            || self.execution_name_policy().is_some()
            || self.hosted_execution_context.is_some()
            || self.vendor_source_input.is_some()
            || input.has_hosted_source_name_context()
            || config != Some(retained_config)
            || self.source_invocation_dialect(retained_config) != Some(dialect)
            || !input.has_logical_source_name_context()
            || input
                .context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
        {
            return None;
        }
        Some(input.clone())
    }

    /// A supplied live entry owns naming selection, including unsupported purposes.
    #[must_use]
    pub fn execution_name_policy(&self) -> Option<tcl_syntax::naming::ExecutionNamePolicy> {
        match self.native_entry {
            Some(entry) => entry.execution_name_policy(),
            None => self.execution_name_policy.or_else(|| {
                // The explicit source driver selects Logical metadata. Its
                // inherited unversioned authored simulation grants no recipe
                // to this independently retained source-only purpose.
                if self.logical_source_input.is_some() {
                    return None;
                }
                if self.vendor_source_input.is_some_and(
                    crate::analyser::ResolvedAnalysisInput::has_hosted_source_name_context,
                ) {
                    return None;
                }
                self.invocation_dialect?
                    .authored_name_policy()
                    .map(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe)
            }),
        }
    }
    /// Selected compiler-local recipe. A supplied live entry is authoritative,
    /// including missing policy; its physical C host cannot fill that gap.
    #[must_use]
    pub fn compiled_variable_protocol(
        &self,
    ) -> Option<tcl_syntax::naming::NativeCompiledVariableProtocol> {
        match self.native_entry {
            Some(entry) => entry.compiled_variable_protocol,
            None => match self.compiled_variable_provider {
                Some(provider) => self
                    .logical_invocation_dialect()?
                    .authored_logical_compiled_variable_protocol(provider),
                None => self
                    .native_compiler_dialect()?
                    .native_compiled_variable_protocol(),
            },
        }
    }

    /// Logical handler/value/frame policy. A supplied live entry is
    /// authoritative, including an unknown policy; authoring policy applies
    /// only when no live entry was supplied.
    #[must_use]
    pub fn logical_invocation_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        match self.native_entry {
            Some(entry) => entry.invocation_policy.as_ref().map(|policy| {
                let mut dialect = tcl_registry::InvocationDialect::of_profile(policy.profile());
                if let Some(grammar) = entry.lexer_grammar {
                    dialect.lexer_grammar = grammar;
                    dialect.word_values =
                        tcl_syntax::word_rules::WordValueRules::from_grammar(&grammar);
                }
                dialect
            }),
            None => self.invocation_dialect,
        }
    }

    pub(crate) fn source_invocation_dialect(
        &self,
        config: tcl_lexer::LexerConfig,
    ) -> Option<tcl_registry::InvocationDialect> {
        self.logical_invocation_dialect().map(|mut dialect| {
            dialect.lexer_grammar = config.grammar_over(dialect.lexer_grammar);
            dialect.word_values =
                tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
            dialect
        })
    }

    /// Select original source grammar without borrowing physical compiler or
    /// logical handler defaults for a supplied live entry. Missing measured
    /// grammar keeps the driver's configuration, including mode and coordinates.
    #[must_use]
    pub fn native_lexer_config(&self, config: tcl_lexer::LexerConfig) -> tcl_lexer::LexerConfig {
        let grammar = match self.native_entry {
            Some(entry) => entry.lexer_grammar,
            None => self.invocation_dialect.map(|dialect| dialect.lexer_grammar),
        };
        grammar.map_or(config, |grammar| config.with_grammar(grammar))
    }

    /// Physical compiler policy only. A supplied incomplete native entry stays
    /// unknown; logical value/frame policies cannot substitute for its engine.
    #[must_use]
    pub fn native_compiler_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        native_compilation_dialect(self.native_entry, self.invocation_dialect)
    }
}

/// Owned entry proof carried from source lowering into every later consumer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SourceAnalysisEntry {
    /// Complete actual availability/source ownership, independently of entry.
    pub metadata_context: crate::registry_invocation::OwnedInvocationMetadataContext,
    /// Host storage policy supplied independently of source naming and catalogue.
    /// This selects no actual worker, publication, variable lifetime or value.
    pub hosted_execution_context: Option<tcl_registry::f5::BigIpExecutionContext>,
    /// Naming issuer retained independently of source strings, physical compiler and catalogue.
    pub execution_name_policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    /// Independently selected complete Logical source advice input. This
    /// grants no native name recipe, entered handler or execution authority.
    pub logical_source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    /// Retained complete hosted source advice input. This supplies no native
    /// naming recipe, runtime storage or entered-frame authority.
    pub vendor_source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    /// Body inventory retained from the original compilation request.
    pub compilation_scope: tcl_runtime_api::SourceCompilationScope,
    /// Availability phase retained from the actual source entry contract.
    pub invocation_realm: tcl_dialect::model::InvocationRealm,
    /// Retained actual interpreter command/namespace compilation boundary.
    pub native_entry: Option<Arc<tcl_runtime_api::NativeCompilationEntry>>,
    /// Ordinary incoming local slots retained from the activation contract.
    pub incoming_formals: Vec<String>,
    /// Retained declaration contracts for assistance adapters.
    pub declared_commands: Option<tcl_registry::model::DeclaredSurface>,
    /// Selected package loaders with explicit implementation provenance.
    pub trusted_package_loaders: Vec<TrustedPackageLoader>,
    /// Actual file-read contracts retained through every pipeline consumer.
    pub trusted_source_modules: Vec<TrustedSourceModuleLoader>,
    /// Whether prior interpreter history is unavailable.
    pub unknown_entry: bool,
    /// Explicit runtime policies independent of catalogue metadata.
    pub invocation_dialect: Option<tcl_registry::InvocationDialect>,
    /// Explicit authored compiler-local provider, separate from physical engine evidence.
    pub compiled_variable_provider:
        Option<tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider>,
    /// Native script evaluation protocol and compiler frame supplied by the entry owner.
    pub native_compilation: tcl_registry::native_compilation::NativeCompilationContext,
}

impl SourceAnalysisEntry {
    /// Retain an owned source-only entry from a positive Logical input.
    /// The shared source options gate refuses Native and hosted axes; this
    /// entry carries no runtime activation or compiler admission.
    #[must_use]
    pub fn for_logical_source(input: &crate::analyser::ResolvedAnalysisInput) -> Option<Self> {
        let options = SourceAnalysisOptions::for_logical_source(input)?;
        Some(Self {
            logical_source_input: options.logical_source_input.cloned(),
            metadata_context: options.metadata_context.retain(),
            invocation_dialect: options.invocation_dialect,
            ..Self::default()
        })
    }

    /// Retain source-only hosted advice from the independent supplied policy.
    /// Prior runtime history stays unknown; this supplies no native or TMM entry.
    #[must_use]
    pub fn for_hosted_source(input: &crate::analyser::ResolvedAnalysisInput) -> Option<Self> {
        let options = SourceAnalysisOptions::for_hosted_source(input)?;
        Some(Self {
            vendor_source_input: options.vendor_source_input.cloned(),
            metadata_context: options.metadata_context.retain(),
            invocation_dialect: options.invocation_dialect,
            unknown_entry: options.unknown_entry,
            ..Self::default()
        })
    }

    /// Select only authored advice justified by the complete supplied input.
    /// An absent runtime entry stays unknown for Native, mismatched or unsupported
    /// source axes; catalogue metadata cannot assert a fresh interpreter.
    #[must_use]
    pub fn for_supplied_source(
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
        config: tcl_lexer::LexerConfig,
        profile: Option<&tcl_dialect::DialectProfile>,
    ) -> Self {
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry, input, config, profile,
        )
        .and_then(|_| Self::for_logical_source(input).or_else(|| Self::for_hosted_source(input)))
        .unwrap_or_else(|| Self {
            metadata_context:
                crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                    input,
                )),
            unknown_entry: true,
            ..Self::default()
        })
    }

    /// Borrow the entry contract for a shared binding interpretation.
    #[must_use]
    pub fn options(&self) -> SourceAnalysisOptions<'_> {
        SourceAnalysisOptions {
            metadata_context: crate::registry_invocation::InvocationMetadataInput::Retained(
                &self.metadata_context,
            ),
            hosted_execution_context: self.hosted_execution_context,
            execution_name_policy: self.execution_name_policy,
            logical_source_input: self.logical_source_input.as_ref(),
            vendor_source_input: self.vendor_source_input.as_ref(),
            compilation_scope: self.compilation_scope,
            invocation_realm: self.invocation_realm,
            native_entry: self.native_entry.as_deref(),
            incoming_formals: &self.incoming_formals,
            declared_commands: self.declared_commands.as_ref(),
            trusted_package_loaders: &self.trusted_package_loaders,
            trusted_source_modules: &self.trusted_source_modules,
            unknown_entry: self.unknown_entry,
            invocation_dialect: self.invocation_dialect,
            compiled_variable_provider: self.compiled_variable_provider,
            native_compilation: self.native_compilation,
        }
    }
}

#[cfg(test)]
mod source_entry_tests {
    use super::*;

    #[test]
    fn owned_source_entry_retains_metadata_without_naming_or_native_entry_donation() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let entry = SourceAnalysisEntry::for_supplied_source(
            context.commands(),
            &input,
            input.lexer_config(),
            Some(profile),
        );
        assert_eq!(entry.metadata_context.source_analysis_input(), Some(&input));
        assert!(entry.native_entry.is_none());
        assert!(entry.execution_name_policy.is_none());
        assert!(entry.logical_source_input.is_none());
        assert!(entry.unknown_entry);
        assert_eq!(
            entry.options().retained_metadata_context(),
            entry.metadata_context
        );
        let missing = SourceAnalysisOptions {
            metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                None,
            ),
            logical_source_input: Some(&input),
            ..SourceAnalysisOptions::default()
        };
        assert!(matches!(
            missing.retained_metadata_context(),
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable
        ));
        let retained = SourceAnalysisEntry {
            metadata_context: missing.retained_metadata_context(),
            ..SourceAnalysisEntry::default()
        };
        assert!(matches!(
            retained.options().retained_metadata_context(),
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable
        ));
    }

    #[test]
    fn owned_hosted_entry_keeps_source_policy_and_unknown_runtime_history() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        for environment in ["f5-irules", "f5-iapps"] {
            let environment = tcl_registry::model::ingress::resolve_environment(environment);
            let profile = environment.unit_profile();
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                environment.default_context_registry(),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            );
            let entry = SourceAnalysisEntry::for_hosted_source(&input).unwrap();
            assert_eq!(entry.vendor_source_input.as_ref(), Some(&input));
            assert!(entry.unknown_entry);
            assert!(entry.logical_source_input.is_none());
            assert!(entry.native_entry.is_none());
            assert!(entry.hosted_execution_context.is_none());
            assert!(entry.options().execution_name_policy().is_none());
            assert_eq!(
                entry.native_compilation,
                tcl_registry::native_compilation::NativeCompilationContext::default()
            );
            assert_eq!(
                entry.options().retained_vendor_source_input(
                    input.context_registry().commands(),
                    Some(input.lexer_config()),
                ),
                Some(input)
            );
        }
        let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
        let profile = environment.unit_profile();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            environment.default_context_registry(),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );
        assert!(SourceAnalysisEntry::for_hosted_source(&input).is_none());
        assert!(SourceAnalysisOptions::for_hosted_source(&input).is_none());
    }

    #[test]
    fn owned_logical_entry_preserves_actual_input_and_refuses_native_axes() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        let config = tcl_lexer::LexerConfig {
            escapes: tcl_dialect::EscapeSyntax::Tcl84,
            ..tcl_lexer::LexerConfig::from_grammar(profile.grammar)
        };
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let entry = SourceAnalysisEntry::for_logical_source(&input).unwrap();
        assert_eq!(entry.logical_source_input.as_ref(), Some(&input));
        assert!(entry.native_entry.is_none());
        assert!(entry.hosted_execution_context.is_none());
        assert!(entry.execution_name_policy.is_none());
        assert_eq!(
            entry.invocation_dialect,
            SourceAnalysisOptions::for_logical_source(&input)
                .unwrap()
                .invocation_dialect
        );
        for native in ["tcl8.4", "tcl8.6", "tcl9.1", "jim", "f5-irules", "f5-iapps"] {
            let environment = tcl_registry::model::ingress::resolve_environment(native);
            let native_profile = environment.unit_profile();
            let native_input = crate::analyser::ResolvedAnalysisInput::new(
                native_profile,
                native_profile,
                environment.default_context_registry(),
                tcl_lexer::LexerConfig::from_grammar(native_profile.grammar),
            );
            assert!(
                SourceAnalysisEntry::for_logical_source(&native_input).is_none(),
                "{native}"
            );
            let mixed = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                native_profile,
                std::sync::Arc::clone(&context),
                config,
            );
            assert!(
                SourceAnalysisEntry::for_logical_source(&mixed).is_none(),
                "{native}"
            );
        }
    }
}
