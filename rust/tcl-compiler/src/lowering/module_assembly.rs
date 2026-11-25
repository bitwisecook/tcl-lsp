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

//! Starting and finishing a module: the entry frame and runtime target namespace, and the final passes.

use super::execution_regions::stock_body_provider_loader;
use super::script_bodies::unavailable_original_body_script;
use super::{Lowerer, LoweringMetadataOrigin, SourceEntryOrigin};
use crate::ir::Module;

impl Lowerer<'_> {
    pub(super) fn retain_module_source_bindings(&mut self) {
        self.module.retained_source_bindings =
            self.module_source_bindings.as_deref().and_then(|bindings| {
                crate::command_binding::RetainedSourceModuleBindings::retain(
                    bindings,
                    &self.module,
                    self.registry,
                    self.supplied_source.as_ref(),
                )
            });
    }

    pub(super) fn refuse_inherited_runtime_target(&mut self, source: &str) -> bool {
        if self.source_point_entry.is_none() {
            return false;
        }
        self.module.top_level = unavailable_original_body_script(
            0,
            source.len(),
            "inherited source view has no runtime target",
        );
        true
    }

    pub(super) fn runtime_target_namespace(&mut self, namespace: &str) -> String {
        if let Some(entry) = &self.source_point_entry {
            self.module.top_level_namespace_context = Some(entry.namespace().clone());
            return entry.namespace().display().unwrap_or_default();
        }
        let key = self.native_entry.as_ref().map_or_else(
            || {
                Some(crate::command_binding::SourceNamespaceKey::authored(
                    tcl_syntax::naming::root_unrooted_key(namespace),
                ))
            },
            |entry| crate::command_binding::SourceNamespaceKey::from_native_entry(entry).ok(),
        );
        let display = key
            .as_ref()
            .and_then(crate::command_binding::SourceNamespaceKey::display)
            .unwrap_or_else(|| tcl_syntax::naming::root_unrooted_key(namespace));
        self.module.top_level_namespace_context = key;
        display
    }

    pub(super) fn source_entry_frame(
        &self,
        namespace: &str,
    ) -> crate::var_resolve::VariableExecutionFrame {
        if self.nest_depth == 1
            && let Some(entry) = &self.source_point_entry
        {
            return entry.frame().clone();
        }
        crate::command_binding::source_analysis_frame(
            self.native_entry
                .as_deref()
                .filter(|_| self.nest_depth == 1),
            namespace,
            self.module.top_level_kind,
        )
    }

    pub(super) fn start_module(&mut self) {
        if self.source_entry_origin == SourceEntryOrigin::Authoring
            && self.metadata_origin == LoweringMetadataOrigin::Standalone
            && self.metadata_input.is_none()
            && self.logical_source_input.is_none()
            && self.vendor_source_input.is_none()
            && self.native_entry.is_none()
            && let Some(profile) = self.dialect.or_else(|| self.registry.profile())
            && let Some(context) = self.dialect_context.clone()
        {
            let input =
                crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, self.config);
            self.retain_standalone_source_metadata(input);
        }
        self.module.top_level_namespace_context = None;
        self.module_source_bindings = None;
        self.module.lexer_config = self.config;
        self.module.dialect_profile = self.dialect.or_else(|| self.registry.profile());
        self.module.registry_snapshot = Some(self.registry.snapshot());
        self.module.source_metadata_input = self
            .metadata_input
            .clone()
            .or_else(|| self.logical_source_input.clone())
            .or_else(|| self.vendor_source_input.clone());
        self.module.dialect = self
            .module
            .dialect_profile
            .map(|profile| profile.name.to_owned());
        // Static authoring uses a fresh standard-distribution entry. Runtime
        // compilation has no authority to assert an installed package loader.
        if self.source_entry_origin == SourceEntryOrigin::Authoring && !self.target.is_bytecode() {
            let profile = self.dialect.or_else(|| self.registry.profile());
            if profile.is_some_and(|profile| {
                !profile.is_irules()
                    && profile.package_protocol() == Some(tcl_dialect::PackageProtocol::Tcl)
            }) {
                let mut providers = std::collections::BTreeMap::new();
                for name in self.registry.command_names() {
                    if let Some(tcl_registry::body_execution::BodyExecutionSpec::CapturedLifecycle(
                        contract,
                    )) = self.registry.get(name).and_then(|spec| spec.body_execution)
                        && let Some(loader) = stock_body_provider_loader(contract.provider, None)
                    {
                        providers.insert(loader.package.clone(), loader);
                    }
                }
                self.trusted_package_loaders = providers.into_values().collect();
            }
        }
        self.module.plain_command_dispatch = self.target.is_trace_visible();
        self.module.source_entry = crate::command_binding::SourceAnalysisEntry {
            source_point_entry: self.source_point_entry.clone(),
            analyser_source_settings: self.analyser_source_settings.clone(),
            metadata_context: self.source_metadata_input().retain(),
            hosted_execution_context: self.hosted_execution_context,
            execution_name_policy: self.execution_name_policy,
            logical_source_input: self.logical_source_input.clone(),
            vendor_source_input: self.vendor_source_input.clone(),
            compilation_scope: self.compilation_scope,
            invocation_realm: self.invocation_realm,
            declared_commands: self
                .source_declared_commands
                .as_ref()
                .or(self.declared_commands)
                .cloned(),
            trusted_package_loaders: self.trusted_package_loaders.clone(),
            trusted_source_modules: self.effective_source_modules().to_vec(),
            unknown_entry: self.unknown_entry,
            host_extensible_command_table: self.host_command_table.is_extensible(),
            invocation_dialect: self.invocation_dialect,
            compiled_variable_provider: self.compiled_variable_provider,
            expression_parser: self.expression_parser,
            native_compilation: self.native_compilation,
            native_entry: self.native_entry.clone(),
            stock_tclsh_stdout: self.stock_tclsh_stdout.clone(),
            incoming_formals: self.incoming_formals.clone(),
        };
    }
}

impl Lowerer<'_> {
    /// Complete either module entry path after its top-level script has been
    /// lowered.  Namespace tables, OO extraction, source/profile stamps, and
    /// trace and callback-write facts deliberately have this one owner so
    /// compiling a runtime procedure target cannot drift from ordinary module
    /// lowering.
    #[doc(hidden)]
    #[must_use]
    pub fn finish_module(self, source: &str) -> Module {
        self.finish_module_with(source, true)
    }

    /// The module a command-mutation scan reads: everything
    /// [`Self::finish_module`] retains except trace and deferred-write
    /// observer facts, which no command-binding analysis consults. Each of
    /// those scans re-walks every retained body (one prepares a whole-source
    /// callback tape), so the analyser's constant-folding trust oracle skips
    /// them.
    pub(crate) fn finish_module_for_command_trust(self, source: &str) -> Module {
        self.finish_module_with(source, false)
    }

    fn finish_module_with(mut self, source: &str, observer_facts: bool) -> Module {
        source.clone_into(&mut self.source);
        self.module.source =
            tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_channel);
        // Surface namespace import / export directives onto the module for
        // downstream consumers (codegen import resolution and warning passes).
        self.module.namespace_imports = std::mem::take(&mut self.namespace_imports);
        self.module.namespace_exports = std::mem::take(&mut self.namespace_exports);
        // Extract TclOO method bodies from the fully-assembled module
        // (cache-independent — see `extract_oo_methods_pass`).
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::WholeModule {
            self.retain_installed_procedure_body_units();
            self.retain_hosted_procedure_body_units();
            self.extract_oo_methods_pass();
            self.retain_declared_frame_bodies();
        }
        let registry = self.registry;
        let standalone_metadata = self.metadata_origin == LoweringMetadataOrigin::Standalone
            && self.source_entry_origin == SourceEntryOrigin::Authoring;
        let declared_frame_effects = crate::declared_frame_effects::collect_declared_frame_effects(
            registry,
            self.source_declared_commands
                .as_ref()
                .or(self.declared_commands),
        );
        let mut module = self.module;
        module.declared_frame_effects = declared_frame_effects;
        module.retained_source_bindings =
            self.module_source_bindings.as_deref().and_then(|bindings| {
                crate::command_binding::RetainedSourceModuleBindings::retain(
                    bindings,
                    &module,
                    registry,
                    self.supplied_source.as_ref(),
                )
            });
        if observer_facts {
            crate::command_binding::trace_registrations::populate_trace_facts(
                &mut module,
                registry,
                standalone_metadata,
            );
            module.deferred_writes = crate::deferred_writes::scan_module(&module, registry);
        }
        module
    }
}
