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

//! The registry baseline a binding state starts from, its fingerprint and equality.

use super::BindingBaseline;
use super::source_analysis_entry::native_compilation_dialect;
use std::collections::BTreeMap;
use std::hash::Hash;
use std::sync::{Arc, OnceLock};
use tcl_registry::{CommandRegistry, EffectiveRegistrySemantics};

impl PartialEq for BindingBaseline {
    fn eq(&self, other: &Self) -> bool {
        self.source_point_entry == other.source_point_entry
            && self.metadata_context == other.metadata_context
            && self.source_read_config == other.source_read_config
            && self.logical_source_input == other.logical_source_input
            && self.vendor_source_input == other.vendor_source_input
            && self.hosted_execution_context == other.hosted_execution_context
            && self.invocation_realm == other.invocation_realm
            && self.execution_name_policy == other.execution_name_policy
            && self.unknown_entry == other.unknown_entry
            && self.host_extensible_command_table == other.host_extensible_command_table
            && self.stock_tclsh_stdout == other.stock_tclsh_stdout
            && self.registry_snapshot == other.registry_snapshot
            && self
                .registry_owner
                .as_ref()
                .map(tcl_registry::RegistrySnapshot::semantic_key)
                == other
                    .registry_owner
                    .as_ref()
                    .map(tcl_registry::RegistrySnapshot::semantic_key)
            && self.native_entry == other.native_entry
            && self.native_compilation == other.native_compilation
            && self.compiled_variable_provider == other.compiled_variable_provider
            && self.expression_parser == other.expression_parser
            && self.trusted_loaders == other.trusted_loaders
            && self.trusted_source_modules == other.trusted_source_modules
            && self.declared_commands == other.declared_commands
            && self.declared_surface == other.declared_surface
            && self.catalogue_commands == other.catalogue_commands
            && self.release == other.release
            && self.dialect == other.dialect
            && self.import_binding == other.import_binding
            && self.semantics.binding_names() == other.semantics.binding_names()
            && self.semantics.unresolved_command_handlers()
                == other.semantics.unresolved_command_handlers()
            && self.declared == other.declared
    }
}

impl Eq for BindingBaseline {}

impl BindingBaseline {
    /// Original compiler engine only; handler policies remain in `dialect`.
    pub(super) fn compilation_dialect(&self) -> Option<tcl_native_model::InvocationDialect> {
        if self.source_point_entry.is_some() {
            return None;
        }
        native_compilation_dialect(self.native_entry.as_deref(), self.dialect)
    }

    /// Fresh F5 rule-loader world: hosted source entered from a known entry in
    /// the rule loader's realm (user decision 2026-10-21T00:10Z;
    /// [`SourceAnalysisEntry::for_fresh_hosted_source`]).
    pub(super) fn hosted_rule_loader_world(&self) -> bool {
        self.native_entry.is_none()
            && !self.unknown_entry
            && self.invocation_realm == tcl_dialect::model::InvocationRealm::RuleLoader
            && self
                .vendor_source_input
                .as_ref()
                .is_some_and(crate::analyser::ResolvedAnalysisInput::has_hosted_source_name_context)
    }

    /// Compilation context of an entered procedure or event body. In the fresh
    /// F5 world each body runs in the TMM execution context, whose vendor
    /// compiler protocol is unmeasured: its commands are selected when they
    /// execute, after argv, as `Direct` states. Other worlds keep the
    /// dialect's procedure-object protocol.
    pub(super) fn body_compilation(
        &self,
    ) -> tcl_registry::native_compilation::NativeCompilationContext {
        if self.source_point_entry.is_some() {
            return tcl_registry::native_compilation::NativeCompilationContext::default();
        }
        if self.hosted_rule_loader_world() {
            return crate::environment_ingress::authoring_native_compilation();
        }
        procedure_compilation(self.compilation_dialect())
    }

    pub(super) fn for_registry(registry: &CommandRegistry) -> Self {
        Self::for_registry_with_semantics(registry, None)
    }

    pub(super) fn for_registry_with_semantics(
        registry: &CommandRegistry,
        semantics: Option<Arc<EffectiveRegistrySemantics>>,
    ) -> Self {
        let mut baseline = Self {
            source_point_entry: None,
            metadata_context:
                crate::registry_invocation::OwnedInvocationMetadataContext::Standalone,
            declared: Arc::new(crate::ir::DeclaredFrameEffects::default()),
            logical_source_input: None,
            source_read_config: crate::var_resolve::SourceReadConfiguration::Standalone,
            vendor_source_input: None,
            hosted_execution_context: None,
            execution_name_policy: registry
                .profile()
                .and_then(|profile| {
                    tcl_native_model::InvocationDialect::of_profile(profile).authored_name_policy()
                })
                .map(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe),
            invocation_realm: tcl_dialect::model::InvocationRealm::RuleLoader,
            unknown_entry: false,
            host_extensible_command_table: false,
            stock_tclsh_stdout: false,
            native_entry: None,
            compiled_variable_provider: None,
            expression_parser: None,
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext::default(
            ),
            fingerprint: 0,
            registry_snapshot: Some(registry.snapshot().semantic_key()),
            registry_owner: Some(registry.snapshot()),
            declared_commands: Arc::default(),
            declared_surface: None,
            catalogue_commands: registry.snapshot().command_descriptor_coordinates(),
            semantics: semantics.unwrap_or_else(|| {
                registry.profile().map_or_else(
                    || registry.effective_semantics(),
                    |profile| {
                        registry.effective_semantics_for_dialect_in_realm(
                            tcl_native_model::InvocationDialect::of_profile(profile),
                            tcl_dialect::model::InvocationRealm::InterpreterRuntime,
                        )
                    },
                )
            }),
            metadata_slots: OnceLock::new(),
            advice_slots: Arc::default(),
            import_binding: registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::namespace_import_binding),
            trusted_loaders: BTreeMap::new(),
            trusted_source_modules: Vec::new(),
            release: registry.runtime_version(),
            dialect: registry
                .profile()
                .map(tcl_native_model::InvocationDialect::of_profile),
        };
        baseline.refresh_fingerprint();
        baseline
    }

    pub(super) fn refresh_fingerprint(&mut self) {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        self.source_point_entry.hash(&mut state);
        self.registry_snapshot.hash(&mut state);
        self.registry_owner
            .as_ref()
            .map(tcl_registry::RegistrySnapshot::semantic_key)
            .hash(&mut state);
        self.execution_name_policy.hash(&mut state);
        self.metadata_context.hash(&mut state);
        self.declared.hash(&mut state);
        self.source_read_config.hash(&mut state);
        self.logical_source_input.hash(&mut state);
        self.vendor_source_input.hash(&mut state);
        self.hosted_execution_context.hash(&mut state);
        self.native_entry.hash(&mut state);
        self.native_compilation.hash(&mut state);
        self.compiled_variable_provider.hash(&mut state);
        self.expression_parser.hash(&mut state);
        self.invocation_realm.hash(&mut state);
        self.unknown_entry.hash(&mut state);
        self.host_extensible_command_table.hash(&mut state);
        self.stock_tclsh_stdout.hash(&mut state);
        self.declared_commands.hash(&mut state);
        self.declared_surface.hash(&mut state);
        self.catalogue_commands.hash(&mut state);
        self.semantics.binding_fingerprint().hash(&mut state);
        self.import_binding.hash(&mut state);
        self.trusted_loaders.hash(&mut state);
        self.trusted_source_modules.hash(&mut state);
        self.release.hash(&mut state);
        self.dialect.hash(&mut state);
        self.fingerprint = std::hash::Hasher::finish(&state);
    }
}

fn procedure_compilation(
    dialect: Option<tcl_native_model::InvocationDialect>,
) -> tcl_registry::native_compilation::NativeCompilationContext {
    use tcl_registry::native_compilation::{
        NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
    };
    NativeCompilationSpec {
        grammar: NativeCompilationGrammar::NoHook,
        operation: tcl_registry::SemanticOperationId::Invoke,
        body: NativeBodyCompilation::ProcedureObject,
    }
    .body_context(
        dialect,
        tcl_registry::native_compilation::NativeCompilationContext::default(),
    )
}

impl Hash for BindingBaseline {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.fingerprint.hash(state);
    }
}
