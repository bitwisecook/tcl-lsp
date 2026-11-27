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

//! Live runtime ingress replaces the compiler driver's fresh-table contract.

use super::{Arc, BTreeMap, BTreeSet, MayBinding, ModuleCommandBindings, ResolvedCommandTarget};
use tcl_names::command::{
    BindingKind, CommandIdentity, RuntimeCommandTokenIdentity, SourceCommandKey,
};
use tcl_names::namespace::SourceNamespaceKey;
use tcl_runtime_api::native_compilation::{NativeCommandImplementation, NativeCompilationBinding};

impl ModuleCommandBindings {
    /// Raw compiler token at this exact lookup, independently of callable import
    /// traversal. Relocation carries this token with the captured binding.
    pub(super) fn compiler_identity_at_lookup(
        &self,
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> Option<&CommandIdentity> {
        if !self.source_lookup_is_closed(head, namespace) {
            return None;
        }
        let slots = self.source_keys(head, namespace);
        let [slot] = slots.as_slice() else {
            return None;
        };
        let bindings = self.bindings.get(slot)?;
        if bindings.len() != 1 {
            return None;
        }
        match bindings.first()? {
            MayBinding::Target(target) => target.token.as_ref(),
            MayBinding::Imported(import) => import.compiler.as_ref(),
            MayBinding::Missing | MayBinding::Unknown => None,
        }
    }

    /// Exact compiler row selected without following a callable import origin.
    pub(super) fn runtime_compiler_row(
        &self,
        identity: &CommandIdentity,
    ) -> Option<&NativeCompilationBinding> {
        let runtime = identity.runtime?;
        let entry = self.baseline.native_entry.as_ref()?;
        (runtime.interpreter == entry.interpreter).then_some(())?;
        entry.commands.iter().find(|row| row.token == runtime.token)
    }

    /// Preserve the actual registration and lookup incarnation for runtime
    /// validation. This receipt proves a compiler, never a callable handler.
    pub(super) fn runtime_command_compiler_prerequisite(
        &self,
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
        guard: tcl_runtime_api::CommandBindingGuard,
    ) -> Option<tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite> {
        let entry = self.baseline.native_entry.as_ref()?;
        let identity = self.compiler_identity_at_lookup(head, namespace)?;
        let row = self.runtime_compiler_row(identity)?;
        if self.runtime_compiler_hook(Some(identity)) != Some(true) {
            return None;
        }
        let lookup_key = if tcl_syntax::naming::is_absolute(head) {
            self.native_root_namespace_key()?
        } else {
            namespace.namespace_key().into_owned()
        };
        let context = lookup_key.native_context()?;
        if context.interpreter != entry.interpreter {
            return None;
        }
        let lookup = entry.namespace_context(context.token).ok()?;
        if lookup.path != context.path {
            return None;
        }
        Some(
            tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite {
                interpreter: entry.interpreter,
                lookup_namespace_token: lookup.token,
                invocation_word: head.into(),
                slot: row.slot.clone(),
                namespace_token: row.namespace_token,
                token: row.token,
                implementation_generation: row.implementation_generation,
                compiler: row.compiler.clone()?,
                selected_worker: None,
                nested_compilers: Vec::new(),
                guard,
            },
        )
    }

    pub(super) fn runtime_implementation_generation(
        &self,
        token: Option<&CommandIdentity>,
    ) -> Option<u64> {
        let runtime = token?.runtime?;
        let entry = self.baseline.native_entry.as_ref()?;
        entry
            .commands
            .iter()
            .find(|command| {
                command.token == runtime.token && runtime.interpreter == entry.interpreter
            })
            .map(|command| command.implementation_generation)
    }

    pub(super) fn runtime_compiler_hook(&self, token: Option<&CommandIdentity>) -> Option<bool> {
        let runtime = token?.runtime?;
        let entry = self.baseline.native_entry.as_ref()?;
        let command = entry.commands.iter().find(|command| {
            command.token == runtime.token && runtime.interpreter == entry.interpreter
        })?;
        if command.has_execution_trace {
            return Some(false);
        }
        match command.compiler_hook {
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent => Some(false),
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Present => Some(true),
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Unknown => None,
        }
    }

    /// Compiler presence from the actual runtime row or an exact installed provider token.
    pub(super) fn installed_compiler_hook(
        &self,
        target: &super::ResolvedCommandTarget,
    ) -> Option<bool> {
        let token = target.token.as_ref()?;
        if token.runtime.is_some() {
            return self.runtime_compiler_hook(Some(token));
        }
        // This bounded Snit recipe installs the default ordinary ensemble
        // without a compiler hook; pragma-selected variants are declined. Compiler
        // presence is independent of its returned-name protocol, and retains
        // the exact live installation receipt and native engine release.
        if let Some(receipt) = self.class_definitions.get(token) {
            let supported_recipe = receipt.dispatcher.is_some_and(|dispatcher| {
                self.baseline
                    .dialect
                    .is_some_and(|dialect| dispatcher.native_installation_is_audited(dialect))
            });
            if supported_recipe
                && receipt.implementation_generation == target.implementation_generation
                && self.definition_dispatcher_receipt_is_live(receipt)
            {
                return Some(false);
            }
        }
        if target.kind != tcl_names::command::BindingKind::Builtin
            || !target.registry_backed
            || token.declaration != 0
            || token.allocation.is_some()
            || target.implementation_generation != 0
            || target.implementation_allocation.is_some()
        {
            return None;
        }
        let hook = self.baseline.trusted_loaders.values().find_map(|loader| {
            let selected = self.loaded_provider(&loader.package)?;
            selected.compiler_hooks.iter().find_map(|(command, hook)| {
                tcl_registry::command_descriptor_labels_match(command, &token.origin)
                    .then_some(*hook)
            })
        })?;
        match hook {
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent => Some(false),
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Present => Some(true),
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Unknown => None,
        }
    }

    pub(super) fn runtime_execution_observed(&self, token: Option<&CommandIdentity>) -> bool {
        let Some(runtime) = token.and_then(|token| token.runtime) else {
            return false;
        };
        self.baseline.native_entry.as_ref().is_some_and(|entry| {
            runtime.interpreter == entry.interpreter
                && entry
                    .commands
                    .iter()
                    .any(|command| command.token == runtime.token && command.has_execution_trace)
        })
    }

    pub(super) fn runtime_noop_header(
        &self,
        token: Option<&CommandIdentity>,
        generation: u32,
        allocation: Option<&tcl_names::command::CommandAllocation>,
    ) -> Option<tcl_runtime_api::native_compilation::NativeProcedureHeaderPrerequisite> {
        if generation != 0 || allocation.is_some() {
            return None;
        }
        let runtime = token?.runtime?;
        let entry = self.baseline.native_entry.as_ref()?;
        let command = entry.commands.iter().find(|command| {
            command.token == runtime.token && runtime.interpreter == entry.interpreter
        })?;
        if command.has_execution_trace
            || command.compiler_hook
                != tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Present
            || command.procedure_header != Some(tcl_dialect::NativeProcedureHeaderCompilation::NoOp)
        {
            return None;
        }
        Some(
            tcl_runtime_api::native_compilation::NativeProcedureHeaderPrerequisite {
                interpreter: entry.interpreter,
                lookup_namespace_token: entry.current_namespace,
                invocation_word: analytical_command_name(entry, &command.slot)?.into(),
                slot: command.slot.clone(),
                namespace_token: command.namespace_token,
                token: command.token,
                implementation_generation: command.implementation_generation,
                header: tcl_dialect::NativeProcedureHeaderCompilation::NoOp,
                guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
            },
        )
    }

    fn retain_imported_observer_uncertainty(
        &mut self,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        command: &NativeCompilationBinding,
        name: SourceCommandKey,
    ) {
        // A trace may alter execution when invoked, but the closed entry
        // still proves this token's compiler hook is disabled. Retain
        // observer uncertainty in runtime lookup rather than the initial
        // compilation table. Imported wrappers without their own retained
        // token still require an unknown execution envelope.
        if command.has_execution_trace
            && !self.bindings.get(&name).is_some_and(|bindings| {
                bindings.iter().any(|binding| {
                    let identity = match binding {
                        MayBinding::Target(target) => target.token.as_ref(),
                        MayBinding::Imported(import) => import.compiler.as_ref(),
                        MayBinding::Missing | MayBinding::Unknown => None,
                    };
                    identity
                        .and_then(|token| token.runtime)
                        .is_some_and(|runtime| {
                            runtime.interpreter == entry.interpreter
                                && runtime.token == command.token
                        })
                })
            })
        {
            Arc::make_mut(&mut self.bindings)
                .entry_or_default(name)
                .insert(MayBinding::Unknown);
        }
    }

    fn install_runtime_variable_entry(&mut self, entry: &tcl_runtime_api::NativeCompilationEntry) {
        // Live entries never promise fresh cell contents, trace registrations,
        // loader discovery or provider-private state.
        Arc::make_mut(&mut self.source_variables).widen();
        super::native_variable_tables::install(Arc::make_mut(&mut self.source_variables), entry);
        {
            let variables = Arc::make_mut(&mut self.source_variables);
            variables
                .authored_tmm_static
                .clone_from(&entry.authored_tmm_static);
            variables.namespace_objects = entry
                .namespaces
                .iter()
                .filter_map(|row| {
                    Some((
                        native_namespace_key(entry, row.token)?,
                        row.jim_namespace_object.clone()?,
                    ))
                })
                .collect();
        }
        if entry.variable_observers.permits_no_callbacks() {
            // This independent runtime table proof closes only unenumerated
            // callbacks. Cell contents, links, lifetimes and native read hooks
            // remain governed by their own retained state and descriptors.
            Arc::make_mut(&mut self.source_variables).dynamic_traces = false;
        }
    }

    fn clear_runtime_entry_tables(&mut self) {
        // Command/namespace snapshots contain no literal-object pool facts.
        self.ordinary_literal_pool = None;
        self.stock_tclsh_stdout = false;
        self.bindings = Arc::new(
            self.baseline
                .semantics
                .binding_names()
                .iter()
                .map(|name| (name.clone(), BTreeSet::from([MayBinding::Missing])))
                .collect(),
        );
        self.objects = Arc::default();
        self.namespace_exports = Arc::default();
        self.namespace_paths = Arc::default();
        self.namespace_unknown_handlers = Arc::default();
        self.ensemble_owner_namespaces = Arc::default();
        self.ensemble_dispatches = Arc::default();
        self.namespaces = Arc::default();
    }

    pub(super) fn install_runtime_entry(
        &mut self,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) {
        self.clear_runtime_entry_tables();
        let mut namespaces = BTreeMap::new();
        for namespace in &entry.namespaces {
            let Some(name) = native_namespace_key(entry, namespace.token) else {
                continue;
            };
            namespaces.insert(namespace.token, name.clone());
            if !namespace.visible {
                // The actual retained current token still owns its lookup
                // base. Descendant geometry and later fallback stay lazy.
                continue;
            }
            Arc::make_mut(&mut self.namespaces).insert(name.clone());
            Arc::make_mut(&mut self.namespace_exports).insert(
                name.clone(),
                BTreeSet::from([namespace
                    .exports
                    .iter()
                    .map(|word| tcl_core_types::NameBytes::from(word.as_bytes()))
                    .collect()]),
            );
            if namespace
                .unknown_handler
                .as_ref()
                .is_some_and(|prefix| !prefix.is_empty())
            {
                Arc::make_mut(&mut self.namespace_unknown_handlers).insert(name);
            }
        }
        for namespace in entry
            .namespaces
            .iter()
            .filter(|namespace| namespace.visible)
        {
            let path = namespace
                .command_path
                .iter()
                .map(|token| namespaces.get(token).cloned())
                .collect::<Option<Vec<_>>>();
            let Some(name) = namespaces.get(&namespace.token) else {
                continue;
            };
            if let Some(path) = path {
                Arc::make_mut(&mut self.namespace_paths)
                    .insert(name.clone(), BTreeSet::from([path]));
            } else {
                Arc::make_mut(&mut self.unknown_namespace_paths).insert(name.clone());
            }
        }
        let tokens = entry
            .commands
            .iter()
            .map(|command| (command.token, runtime_token(entry, command)))
            .collect::<BTreeMap<_, _>>();
        let mut installed = BTreeSet::new();
        for command in &entry.commands {
            let name = native_command_key(entry, command);
            let binding = runtime_binding(entry, command, &tokens);
            Arc::make_mut(&mut self.objects)
                .entry(tokens[&command.token].clone())
                .or_default()
                .insert(binding.clone());
            if !entry
                .namespaces
                .iter()
                .any(|namespace| namespace.token == command.namespace_token && namespace.visible)
            {
                continue;
            }
            let Some(name) = name else {
                continue;
            };
            let visible_binding = binding.clone();
            let bindings = Arc::make_mut(&mut self.bindings);
            if installed.insert(name.clone()) {
                bindings.insert(name.clone(), BTreeSet::from([visible_binding.clone()]));
            } else {
                bindings
                    .entry_or_default(name.clone())
                    .extend([visible_binding, MayBinding::Unknown]);
            }
            self.retain_imported_observer_uncertainty(entry, command, name);
        }
        self.original_entry_bindings = Arc::clone(&self.bindings);
        self.install_runtime_variable_entry(entry);
        self.loader_handler_unknown = true;
        self.opaque_domain = !entry.closed
            || entry.command_name_policy().is_none()
            || !entry.namespaces.iter().any(|row| {
                row.token == entry.current_namespace
                    && native_namespace_key(entry, row.token).is_some()
            });
        self.original_command_world = Arc::new(
            super::source_command_world::OriginalSourceCommandWorld::for_runtime_entry(self, entry),
        );
    }
}

/// Optional source-addressable view; the complete byte row remains in the entry.
pub(super) fn analytical_command_name(
    entry: &tcl_runtime_api::NativeCompilationEntry,
    slot: &tcl_core_types::NativeByteCommandSlot,
) -> Option<String> {
    tcl_syntax::naming::native_command_source_spelling(entry.command_name_policy()?.recipe(), slot)
}

/// Checked authored presentation used only by compatibility projection tests.
#[cfg(test)]
fn analytical_namespace_context_key(
    entry: &tcl_runtime_api::NativeCompilationEntry,
    namespace: &tcl_runtime_api::native_compilation::NativeCompilationNamespace,
) -> Option<String> {
    let protocol = entry.command_name_policy()?.recipe();
    if protocol.is_jim084() {
        tcl_syntax::naming::native_jim_namespace_source_spelling(
            namespace.jim_namespace_object.as_ref()?.as_bytes(),
        )
    } else {
        tcl_syntax::naming::checked_namespace_path_key(&namespace.path)
    }
}

pub(super) fn native_namespace_key(
    entry: &tcl_runtime_api::NativeCompilationEntry,
    token: u64,
) -> Option<SourceNamespaceKey> {
    entry
        .retained_namespace_context(token)
        .ok()
        .map(SourceNamespaceKey::Native)
}

pub(super) fn native_command_key(
    entry: &tcl_runtime_api::NativeCompilationEntry,
    command: &NativeCompilationBinding,
) -> Option<SourceCommandKey> {
    let namespace = native_namespace_key(entry, command.namespace_token)?;
    (namespace.exact_native_path()? == &command.slot.namespace).then_some(())?;
    Some(SourceCommandKey::slot(
        namespace,
        command.slot.simple.clone(),
    ))
}

fn runtime_command_label(
    entry: &tcl_runtime_api::NativeCompilationEntry,
    command: &NativeCompilationBinding,
) -> String {
    analytical_command_name(entry, &command.slot).unwrap_or_else(|| {
        // This label is never installed as a lookup spelling. Exact runtime
        // identity and the complete byte slot remain in the retained entry.
        format!(
            "native token {}:{}:{}",
            entry.interpreter.owner, entry.interpreter.interpreter, command.token
        )
    })
}

fn runtime_token(
    entry: &tcl_runtime_api::NativeCompilationEntry,
    command: &NativeCompilationBinding,
) -> CommandIdentity {
    CommandIdentity {
        runtime: Some(RuntimeCommandTokenIdentity {
            interpreter: entry.interpreter,
            token: command.token,
        }),
        origin: runtime_command_label(entry, command),
        declaration: 0,
        allocation: None,
    }
}

fn runtime_alias_arguments<'a>(
    words: impl Iterator<Item = Option<&'a tcl_runtime_api::NameBytes>>,
) -> Vec<crate::registry_invocation::EffectiveInvocationWord> {
    words
        .map(|word| {
            word.and_then(|word| word.try_utf8().ok()).map_or(
                crate::registry_invocation::EffectiveInvocationWord::Dynamic,
                |text| {
                    crate::registry_invocation::EffectiveInvocationWord::Literal(text.to_owned())
                },
            )
        })
        .collect()
}

fn runtime_binding(
    entry: &tcl_runtime_api::NativeCompilationEntry,
    command: &NativeCompilationBinding,
    tokens: &BTreeMap<u64, CommandIdentity>,
) -> MayBinding {
    let mut target = ResolvedCommandTarget {
        command: runtime_command_label(entry, command),
        prepended: Vec::new(),
        registry_backed: false,
        kind: BindingKind::Command,
        implementation_generation: 0,
        implementation_allocation: None,
        terminal: true,
        target_lookup: tcl_registry::AliasTargetLookup::Global,
        token: Some(tokens[&command.token].clone()),
    };
    match &command.implementation {
        NativeCommandImplementation::Registry { identity, .. } => {
            target.command.clone_from(identity);
            target.registry_backed = true;
            target.kind = BindingKind::Builtin;
        }
        NativeCommandImplementation::Alias {
            interpreter,
            words,
            target_lookup,
        } if *interpreter == entry.interpreter
            && words.first().is_some_and(|word| word.try_utf8().is_ok()) =>
        {
            target.target_lookup = match target_lookup {
                tcl_runtime_api::native_compilation::NativeAliasTargetLookup::Global => {
                    tcl_registry::AliasTargetLookup::Global
                }
                tcl_runtime_api::native_compilation::NativeAliasTargetLookup::CallerNamespace => {
                    tcl_registry::AliasTargetLookup::CallerNamespace
                }
            };
            words[0]
                .try_utf8()
                .expect("checked alias head")
                .clone_into(&mut target.command);
            target.prepended = runtime_alias_arguments(words[1..].iter().map(Some));
            target.kind = BindingKind::Alias;
            target.terminal = false;
        }
        NativeCommandImplementation::PartialAlias {
            interpreter,
            words,
            target_lookup,
        } if *interpreter == entry.interpreter
            && words
                .first()
                .and_then(Option::as_ref)
                .is_some_and(|word| word.try_utf8().is_ok()) =>
        {
            words[0]
                .as_ref()
                .unwrap()
                .try_utf8()
                .expect("checked alias head")
                .clone_into(&mut target.command);
            target.prepended = runtime_alias_arguments(words[1..].iter().map(Option::as_ref));
            target.target_lookup = match target_lookup {
                tcl_runtime_api::native_compilation::NativeAliasTargetLookup::Global => {
                    tcl_registry::AliasTargetLookup::Global
                }
                tcl_runtime_api::native_compilation::NativeAliasTargetLookup::CallerNamespace => {
                    tcl_registry::AliasTargetLookup::CallerNamespace
                }
            };
            target.kind = BindingKind::Alias;
            target.terminal = false;
        }
        NativeCommandImplementation::Imported {
            interpreter, token, ..
        } if *interpreter == entry.interpreter => {
            return tokens
                .get(token)
                .cloned()
                .map_or(MayBinding::Unknown, |origin| {
                    MayBinding::Imported(super::ImportedCommandBinding {
                        origin,
                        compiler: Some(tokens[&command.token].clone()),
                    })
                });
        }
        NativeCommandImplementation::Opaque
        | NativeCommandImplementation::Alias { .. }
        | NativeCommandImplementation::PartialAlias { .. } => {
            // The interpreter owns this exact opaque command token. A wrapper
            // also remains opaque when its forwarding destination is outside
            // the retained interpreter. Keep only that
            // opaque installation so its independent compiler-hook row remains
            // available; no parent target, prefix or handler facts are borrowed.
        }
        NativeCommandImplementation::Imported { .. } => return MayBinding::Unknown,
    }
    MayBinding::Target(target)
}

#[cfg(test)]
mod tests;
