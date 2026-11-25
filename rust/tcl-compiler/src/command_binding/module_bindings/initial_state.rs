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

//! The initial binding state of a registry baseline and its entry options.

use super::super::namespace_context::SourceNamespaceKey;
use super::super::source_analysis_entry::SourceAnalysisOptions;
use super::super::trusted_packages::interpreter_provided_packages;
use super::super::{
    BindingBaseline, ModuleCommandBindings, OriginalEntryPackages, source_command_world,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tcl_registry::CommandRegistry;
#[cfg(test)]
use tcl_registry::EffectiveRegistrySemantics;

impl ModuleCommandBindings {
    pub(in super::super) fn initial(registry: &CommandRegistry) -> Self {
        let baseline = Arc::new(BindingBaseline::for_registry(registry));
        Self::initial_from_baseline(registry, baseline)
    }

    fn initial_from_baseline(registry: &CommandRegistry, baseline: Arc<BindingBaseline>) -> Self {
        let state_baseline_release = (baseline.native_entry.is_none() && !baseline.unknown_entry)
            .then_some(baseline.release)
            .flatten();
        let mut exports: BTreeMap<String, Vec<tcl_core_types::NameBytes>> = BTreeMap::new();
        let mut namespaces = BTreeSet::from(["::".to_owned()]);
        if let Some(context) = baseline.hosted_execution_context {
            namespaces.extend(
                tcl_registry::f5::runtime_namespaces(context)
                    .iter()
                    .map(|namespace| (*namespace).to_owned()),
            );
        }
        for command in baseline.semantics.binding_names() {
            let (holder, tail) = tcl_syntax::naming::key_holder_and_tail(command);
            namespaces.insert(if holder.is_empty() {
                "::".to_owned()
            } else {
                holder.to_owned()
            });
            if registry
                .get(command)
                .is_some_and(|spec| spec.is_namespace_exported)
                && command.is_ascii()
                && !command.as_bytes().contains(&0)
            {
                exports
                    .entry(holder.to_owned())
                    .or_default()
                    .push(tcl_core_types::NameBytes::from(tail.as_bytes()));
            }
        }
        let mut state = Self {
            stock_tclsh_stdout: baseline.stock_tclsh_stdout,
            baseline,
            namespace_exports: Arc::new(
                exports
                    .into_iter()
                    .map(|(name, patterns)| (name, BTreeSet::from([patterns])))
                    .collect(),
            ),
            namespaces: Arc::new(namespaces.into_iter().collect()),
            ..Self::default()
        };
        state.original_command_world =
            Arc::new(source_command_world::OriginalSourceCommandWorld::for_baseline(&state));
        if let (Some(release), Some(policy)) = (
            state_baseline_release,
            state.original_command_world.selected_policy(),
        ) {
            state.packages = Arc::new(interpreter_provided_packages(release, policy));
        }
        if state.baseline.native_entry.is_none()
            && !state.baseline.unknown_entry
            && let Some(dialect) = state.baseline.dialect
            && let Some(support) = registry
                .native_class_factory_recipe(
                    "oo::configurable",
                    dialect,
                    state.baseline.invocation_realm,
                )
                .and_then(tcl_registry::native_tcloo_bootstrap::NativeClassFactoryRecipe::support)
        {
            for receiver in [
                tcl_registry::definer::DefinitionReceiver::Instance,
                tcl_registry::definer::DefinitionReceiver::Class,
            ] {
                let namespace =
                    SourceNamespaceKey::authored(support.definition_namespace(receiver));
                let path = support
                    .definition_path(receiver)
                    .iter()
                    .map(|name| SourceNamespaceKey::authored(*name))
                    .collect::<Vec<_>>();
                if state.namespaces.contains(&namespace)
                    && path.iter().all(|key| state.namespaces.contains(key))
                {
                    Arc::make_mut(&mut state.namespace_paths)
                        .insert(namespace, BTreeSet::from([path]));
                }
            }
        }
        state
    }

    pub(in super::super) fn initial_with_options(
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
        config: Option<tcl_lexer::LexerConfig>,
    ) -> Self {
        if let Some(entry) = options.source_point_entry {
            if let Some(state) = entry.child_state(registry, options, config) {
                return state;
            }
            // An invalid inherited point is terminal unavailable history.
            // It must never fall through to a fresh authoring baseline.
            let mut baseline = BindingBaseline {
                source_point_entry: Some(entry.clone()),
                metadata_context:
                    crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
                execution_name_policy: None,
                unknown_entry: true,
                ..BindingBaseline::for_registry(registry)
            };
            baseline.refresh_fingerprint();
            return Self::initial_entry_from_baseline(registry, Arc::new(baseline));
        }
        let logical_source_input = options.retained_logical_source_input(registry, config);
        let logical = config.map_or_else(
            || options.logical_invocation_dialect(),
            |config| options.source_invocation_dialect(config),
        );
        // Select the final roster once. A supplied policy previously replaced
        // an already-built whole-registry roster before any state used it.
        let semantics = logical.map(|dialect| match logical_source_input.as_ref() {
            Some(input) => registry.authored_source_semantics_in_context(
                input.context_registry().context(),
                options.invocation_realm,
            ),
            None => registry.effective_semantics_for_dialect_in_realm(
                dialect,
                tcl_dialect::model::InvocationRealm::InterpreterRuntime,
            ),
        });
        let mut baseline = BindingBaseline::for_registry_with_semantics(registry, semantics);
        if options.native_entry.is_some()
            || options.execution_name_policy.is_some()
            || options.invocation_dialect.is_some()
            || options.vendor_source_input.is_some()
        {
            baseline.execution_name_policy = options.execution_name_policy();
        }
        baseline.metadata_context = options.retained_metadata_context();
        baseline.stock_tclsh_stdout =
            options.starts_with_stock_tclsh_stdout(&baseline.metadata_context, config);
        baseline.source_read_config = config.map_or(
            crate::var_resolve::SourceReadConfiguration::Standalone,
            crate::var_resolve::SourceReadConfiguration::Selected,
        );
        baseline.logical_source_input = logical_source_input;
        baseline.vendor_source_input = options.retained_vendor_source_input(registry, config);
        baseline.hosted_execution_context = options.hosted_execution_context;
        baseline.native_compilation = options.native_compilation;
        baseline.compiled_variable_provider = options.compiled_variable_provider;
        baseline.expression_parser = options.expression_parser;
        baseline.invocation_realm = options.invocation_realm;
        baseline.unknown_entry = options.unknown_entry;
        baseline.host_extensible_command_table = options.host_extensible_command_table;
        baseline.native_entry = options.native_entry.cloned().map(Arc::new);
        baseline.declared_surface = options.retained_declared_command_contracts();
        // A declared command's stated frame effect binds it in every walk's
        // baseline, not only in one built from a lowered module.
        baseline.declared = Arc::new(
            crate::declared_frame_effects::collect_declared_frame_effects(
                registry,
                options.declared_commands,
            ),
        );
        baseline.declared_commands = Arc::new(
            tcl_registry::unique_command_descriptor_coordinates(
                tcl_registry::model::DocumentCommandSurface::new(
                    registry,
                    options.declared_commands,
                )
                .declared_descriptor_keys(),
            )
            .map(|(coordinate, key)| (coordinate, key.label().to_owned()))
            .collect(),
        );
        baseline.trusted_source_modules = options.effective_source_modules().to_vec();
        baseline.trusted_loaders = options
            .trusted_package_loaders
            .iter()
            .map(|loader| (loader.package.clone(), loader.clone()))
            .collect();
        if options.native_entry.is_some() && logical.is_none() {
            baseline.dialect = None;
            baseline.release = None;
            baseline.import_binding = None;
        }
        if let Some(dialect) = logical {
            baseline.import_binding = dialect.namespace_import_binding;
            baseline.release = dialect.tcl_version;
            baseline.dialect = Some(dialect);
        }
        baseline.refresh_fingerprint();
        Self::initial_entry_from_baseline(registry, Arc::new(baseline))
    }

    /// Rebuild entry state from immutable entry facts, never a dispatch world's
    /// already-applied substitutions, declarations or command mutations.
    pub(in super::super) fn initial_entry_from_baseline(
        registry: &CommandRegistry,
        baseline: Arc<BindingBaseline>,
    ) -> Self {
        let mut state = Self::initial_from_baseline(registry, baseline);
        let mut variables = crate::var_resolve::ResolveContext::for_namespace("::");
        variables.frame_kind = crate::var_resolve::VariableFrameKind::Global;
        variables.invocation_dialect = state.baseline.dialect;
        variables.source_read_configuration = state.baseline.source_read_config;
        variables.execution_name_policy = state.baseline.execution_name_policy;
        variables.expression_parser = state.baseline.expression_parser;
        variables.hosted_execution_context = state.baseline.hosted_execution_context;
        variables.namespace_identities = state.namespaces.iter().cloned().collect();
        variables.known_namespaces = state
            .namespaces
            .iter()
            .filter_map(SourceNamespaceKey::advisory_key)
            .collect();
        if !state.baseline.unknown_entry {
            variables.namespace_cells = crate::variable_bindings::fresh_namespace_cells(registry);
        }
        state.source_variables = Arc::new(variables);
        state.variable_frame = crate::var_resolve::VariableExecutionFrame::Global;
        if let Some(entry) = state.baseline.native_entry.clone() {
            state.install_runtime_entry(&entry);
        }
        if state.baseline.unknown_entry && state.baseline.native_entry.is_none() {
            state.mark_opaque_binding_mutation();
            state.loader_handler_unknown = true;
        }
        state.original_entry_packages = Some(Arc::new(OriginalEntryPackages {
            packages: Arc::clone(&state.packages),
            revoked_loaders: Arc::clone(&state.revoked_loaders),
            loader_handler_unknown: state.loader_handler_unknown,
            command_world: if state.baseline.host_extensible_command_table {
                // An embedding host may hook any command: nothing about the
                // command world is proved before the first command runs.
                let mut world = (*state.original_command_world).clone();
                world.withdraw();
                Arc::new(world)
            } else {
                Arc::clone(&state.original_command_world)
            },
        }));
        state
    }

    #[cfg(test)]
    pub(crate) fn effective_semantics(&self) -> &Arc<EffectiveRegistrySemantics> {
        &self.baseline.semantics
    }
}

impl ModuleCommandBindings {
    /// Fresh F5 rule-loader world ([`BindingBaseline::hosted_rule_loader_world`]).
    pub(in super::super::super) fn hosted_rule_loader_world(&self) -> bool {
        self.baseline.hosted_rule_loader_world()
    }
}
