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

//! Starting and finishing a module: the entry frame and runtime target namespace, the final passes and the variable-trace facts.

use super::execution_regions::stock_body_provider_loader;
use super::script_bodies::unavailable_original_body_script;
use super::{Lowerer, LoweringMetadataOrigin, SourceEntryOrigin};
use crate::ir::{Module, Script, Statement};
use tcl_registry::CommandRegistry;

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
            populate_trace_facts(&mut module, registry, standalone_metadata);
            module.deferred_writes = crate::deferred_writes::scan_module(&module, registry);
        }
        module
    }
}

/// Post-lower scan for `trace add ...` calls that populates
/// `Module::traced_commands` / `has_dynamic_trace` (execution traces)
/// and `Module::traced_variables` / `has_dynamic_variable_trace`
/// (variable traces).
///
/// Runs over the top-level script + every procedure body + every `TclOO`
/// method body (`module.methods`, already populated by
/// `extract_oo_methods_pass` before this runs). Original command operands
/// contribute source-renderable keys from their retained lookup geometry;
/// missing or opaque geometry sets `has_dynamic_trace` so GVN treats every
/// call as potentially traced. Authored compatibility has its separate written
/// grammar; a descriptor label cannot supply an original trace operand.
/// Literal variable names targeted by any `Traits::
/// ESTABLISHES_VARIABLE_TRACE` subcommand — the registry's
/// `ArgRole::VarWrite` resolution for `trace add|remove|variable|vdelete`,
/// covering both the modern and deprecated legacy spellings — land in
/// `traced_variables`; non-literal targets flip
/// `has_dynamic_variable_trace`.
pub(super) fn populate_trace_facts(
    module: &mut Module,
    registry: &CommandRegistry,
    standalone: bool,
) {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)
            .or_else(|| {
                (standalone && module.source_metadata_input.is_none())
                    .then(|| {
                        registry
                            .profile()
                            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                            .map(crate::registry_invocation::InvocationMetadataContext::from)
                    })
                    .flatten()
            });
    // Borrow every retained body instead of cloning it: a derived `Clone` of
    // cap-deep structured IR overflows a 2 MiB thread. The facts collect
    // separately and are applied once the walk has finished.
    let mut facts = TraceFacts::default();
    walk_for_trace(&module.top_level, &mut facts, registry, context);
    // Every statically-known frame, not just named procedures: a `trace`
    // call inside a `namespace eval` / `apply` body (`Module::body_units`)
    // or a `TclOO` method (`Module::methods`) is just as live as one inside
    // a `proc` — omitting either class here would silently under-populate
    // `traced_variables` the moment an optimiser pass starts reaching into
    // those bodies (`CompilationUnit::all_body_function_units`).
    for body in module
        .procedures
        .values()
        .map(|p| &p.body)
        .chain(module.body_lowerings().map(|(_, p)| &p.body))
        .chain(module.methods.values().map(|m| &m.body))
    {
        walk_for_trace(body, &mut facts, registry, context);
    }
    module.traced_variables.extend(facts.variables);
    module.traced_commands.extend(facts.commands);
    module.has_dynamic_variable_trace |= facts.dynamic_variable;
    module.has_dynamic_trace |= facts.dynamic;
}

/// Trace facts one retained-body walk collects for its [`Module`].
#[derive(Default)]
pub(super) struct TraceFacts {
    variables: std::collections::BTreeSet<String>,
    pub(super) commands: std::collections::BTreeSet<String>,
    dynamic_variable: bool,
    dynamic: bool,
}

/// Possible variable registrations are observer hazards even when receiver
/// lookup cannot select a unique normal handler. Execution traces retain their
/// stricter dispatch proof in the separate path below.
fn record_possible_variable_trace(
    transition: &tcl_registry::TraceTransition,
    module: &mut TraceFacts,
) {
    use tcl_registry::{TraceTarget, TraceTransition};
    let (TraceTransition::Add { target, .. } | TraceTransition::Remove { target, .. }) = transition;
    let TraceTarget::Variable(subject) = target else {
        return;
    };
    if let Some(target) = subject.literal() {
        let name = crate::variable_observation_names::module_observation_key(target);
        module.variables.insert(name.to_owned());
    } else {
        module.dynamic_variable = true;
    }
}

/// Collect typed trace transitions through the shared retained-body visitor.
pub(super) fn walk_for_trace(
    script: &Script,
    module: &mut TraceFacts,
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) {
    crate::ir::for_each_statement(script, &mut |stmt| {
        if !matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. }) {
            return;
        }
        let Some(context) = context else {
            // Missing source metadata cannot close either observer-hazard inventory.
            module.dynamic_variable = true;
            module.dynamic = true;
            return;
        };
        if let Some(possible) = stmt.tokens().and_then(|tokens| {
            crate::registry_invocation::possible_variable_trace_transitions_with_metadata_context(
                registry,
                Some(context),
                tokens,
            )
        }) {
            for transition in possible.transitions() {
                record_possible_variable_trace(transition, module);
            }
            if possible.metadata_unavailable() {
                module.dynamic_variable = true;
                module.dynamic = true;
            }
        }
        let Some(invocation) =
            crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
                registry,
                Some(context),
                stmt,
            )
        else {
            return;
        };
        let Some(transitions) = invocation.facts.state_transitions.declared() else {
            return;
        };
        for fact in transitions.facts() {
            use tcl_registry::{StateTransition, TraceTarget, TraceTransition};
            let StateTransition::Trace(transition) = &fact.transition else {
                continue;
            };
            let (target, add) = match transition {
                TraceTransition::Add { target, .. } => (target, true),
                TraceTransition::Remove { target, .. } => (target, false),
            };
            match target {
                TraceTarget::Execution(subject) if add => {
                    let keys = stmt.tokens().and_then(|tokens| {
                        crate::registry_invocation::original_execution_trace_keys(
                            registry, context, tokens, subject,
                        )
                    });
                    if let Some(keys) = keys {
                        module.commands.extend(keys);
                    } else {
                        module.dynamic = true;
                    }
                }
                TraceTarget::Variable(_) | TraceTarget::Command(_) | TraceTarget::Execution(_) => {}
            }
        }
    });
}
