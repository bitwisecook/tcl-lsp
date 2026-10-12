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

use super::{
    Arc, BTreeMap, BTreeSet, BindingKind, CommandIdentity, MayBinding, ModuleCommandBindings,
    ResolvedCommandTarget, RuntimeCommandTokenIdentity, SourceCommandKey, SourceNamespaceKey,
};
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
        let identity = self.compiler_identity_at_lookup(head, namespace)?;
        let row = self.runtime_compiler_row(identity)?;
        if self.runtime_compiler_hook(Some(identity)) != Some(true) {
            return None;
        }
        let entry = self.baseline.native_entry.as_ref()?;
        let lookup_key = if head.starts_with("::") {
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
        if target.kind != super::BindingKind::Builtin
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
            selected
                .compiler_hooks
                .iter()
                .find_map(|(command, hook)| (super::nqn(command) == token.origin).then_some(*hook))
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
        allocation: Option<&super::CommandAllocation>,
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
                .entry(name)
                .or_default()
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
                    .entry(name.clone())
                    .or_default()
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
        constructed_namespace_key(&namespace.path)
    }
}

#[cfg(test)]
fn constructed_namespace_key(path: &tcl_core_types::ByteNamespacePath) -> Option<String> {
    let components = tcl_syntax::naming::checked_namespace_path_utf8(path).ok()?;
    let key = if components.is_empty() {
        String::from("::")
    } else {
        format!("::{}", components.join("::"))
    };
    // This is an optional compatibility projection, not a native identity.
    (tcl_syntax::naming::key_segments(&key) == components).then_some(key)
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
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
    use tcl_registry::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    };
    use tcl_runtime_api::native_compilation::{
        NativeCompilationEntry, NativeCompilationNamespace, NativeInterpreterIdentity,
    };

    fn entry(implementation: NativeCommandImplementation) -> NativeCompilationEntry {
        NativeCompilationEntry {
            interpreter: NativeInterpreterIdentity {
                owner: 1,
                interpreter: 2,
            },
            epoch: 3,
            profile: tcl_dialect::DialectProfile::plain_tcl().cache_key(),
            invocation_policy: Some(tcl_dialect::DialectProfile::plain_tcl().cache_key()),
            expression_policy: None,
            execution_point: tcl_registry::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V9_0,
            )
            .execution_point(),
            execution_name_policy: None,
            name_protocol: Some(
                tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                    tcl_dialect::model::DialectPoint::canonical(
                        tcl_dialect::model::Release::TCL_9_0,
                    ),
                )
                .unwrap(),
            ),
            compiled_variable_protocol: None,
            compiled_local_layout: None,
            oo_classes: None,
            ensemble_target_objects: None,
            source_string_protocol: None,
            lexer_grammar: None,
            inline_compilation_disabled: false,
            authored_tmm_static: None,
            namespace_variable_tables: None,
            empty_literal_world: None,
            compiler_pass_environment: None,
            command_resolvers: None,
            variable_observers:
                tcl_runtime_api::native_compilation::NativeVariableObserverPresence::Unknown,
            math_functions: None,
            closed: true,
            current_namespace: 0,
            frame: tcl_runtime_api::native_compilation::NativeCompilationFrame::Global,
            commands: vec![NativeCompilationBinding {
                slot: tcl_core_types::NativeByteCommandSlot::new(
                    tcl_core_types::ByteNamespacePath::root(),
                    "set".into(),
                ),
                namespace_token: 0,
                token: 4,
                implementation_generation: u64::MAX,
                compiler_hook: match &implementation {
                    NativeCommandImplementation::Registry {
                        compiler_hook: true,
                        ..
                    } => tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Present,
                    NativeCommandImplementation::Registry {
                        compiler_hook: false,
                        ..
                    } => tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
                    _ => tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Unknown,
                },
                implementation,
                compiler: None,
                procedure_header: None,
                has_execution_trace: false,
            }],
            namespaces: vec![NativeCompilationNamespace {
                path: tcl_core_types::ByteNamespacePath::root(),
                jim_namespace_object: None,
                token: 0,
                visible: true,
                exports: Vec::new(),
                command_path: Vec::new(),
                unknown_handler: None,
            }],
        }
    }

    fn select_fixture_native_point(
        snapshot: &mut NativeCompilationEntry,
        version: tcl_dialect::TclVersion,
    ) {
        let point = tcl_registry::InvocationDialect::for_version(version)
            .execution_point()
            .expect("measured C fixture point");
        snapshot.execution_point = Some(point);
        snapshot.name_protocol = tcl_syntax::naming::NamePolicyProtocol::for_native_point(point);
        snapshot.source_string_protocol =
            Some(tcl_syntax::native_string::NativeStringProtocol::C(version));
        snapshot.lexer_grammar = Some(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name())
                .unwrap()
                .grammar,
        );
        let mut identity = 1;
        snapshot.ensemble_target_objects = Some(snapshot.commands.iter().flat_map(|command| {
            command.compiler.as_ref().and_then(|compiler| compiler.ensemble.as_ref()).into_iter().flat_map(move |configuration| {
                configuration.map.iter().flat_map(move |(member, prefix)| prefix.iter().enumerate().map(move |(position, name)| (command.token, member.clone(), position, name.clone())))
            })
        }).map(|(token, member, position, name)| {
            let row = tcl_runtime_api::native_compilation::NativeEnsembleTargetObservation {
                ensemble_token: token, member, prefix_index: position, object_identity: identity,
                resident_name: name.clone(), primary: if name.is_some() { tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::StockString } else { tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::Unavailable },
            };
            identity += 1; row
        }).collect());
    }

    #[test]
    fn observed_variable_policy_keeps_independent_command_provider_authority() {
        use tcl_syntax::naming::{
            ExecutionNamePolicy, MeasuredBigIpNameScope, NamePolicyProtocol,
            ObservedBigIpNamePolicy,
        };
        let mut snapshot = entry(NativeCommandImplementation::Opaque);
        let observed =
            ExecutionNamePolicy::ObservedBigIp(ObservedBigIpNamePolicy::for_measured_scope(
                MeasuredBigIpNameScope::BigIp21_1_0_1Build0_0_26TmmHttpRequest,
            ));
        snapshot.execution_name_policy = Some(observed);
        let native = snapshot.name_protocol.take().unwrap();
        assert_eq!(snapshot.command_name_policy(), None);
        assert_eq!(snapshot.execution_name_policy(), Some(observed));
        snapshot.name_protocol = Some(native);
        assert_eq!(snapshot.command_name_policy(), Some(native));
        let authored = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4);
        snapshot.name_protocol = Some(authored);
        assert_eq!(snapshot.command_name_policy(), Some(authored));
        assert_eq!(snapshot.execution_name_policy(), Some(observed));
    }

    fn analyse(entry: &NativeCompilationEntry) -> SourceCommandBindings {
        analyse_source(entry, "set x 1")
    }

    fn analyse_source(entry: &NativeCompilationEntry, source: &str) -> SourceCommandBindings {
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
            SourceAnalysisOptions {
                native_entry: Some(entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V9_0,
                )),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ScriptCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                ..SourceAnalysisOptions::default()
            },
        )
    }

    #[test]
    fn original_native_world_selects_policy_before_any_branch_and_never_reseeds_withdrawal() {
        // Implementation contract: naming.command.original-native-entry-world-bootstrap
        // docs/design/analysis/name-resolution-proofs/command-original-native-entry-world-bootstrap.md
        let registry = tcl_registry::CommandRegistry::build_default();
        for version in tcl_dialect::TclVersion::ALL {
            let mut snapshot = entry(NativeCommandImplementation::Opaque);
            select_fixture_native_point(&mut snapshot, version);
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let mut state = super::super::ModuleCommandBindings::initial_with_options(
                &registry,
                SourceAnalysisOptions {
                    native_entry: Some(&snapshot),
                    invocation_dialect: Some(dialect),
                    ..Default::default()
                },
                None,
            );
            let policy = snapshot.command_name_policy().unwrap();
            let root = state.source_root_namespace_key().unwrap();
            let before = (*state.original_command_world).clone();
            let mut selected_branch = before.clone();
            assert_eq!(selected_branch.select_policy(&state), Some(policy));
            assert_eq!(selected_branch, before);
            let mut joined = before.clone();
            assert!(!joined.join(&selected_branch));
            assert!(joined.scope(&root, policy).is_some());
            assert!(
                state
                    .original_publication_at(
                        &tcl_core_types::ByteCommandSlot {
                            namespace: tcl_core_types::ByteNamespacePath::root(),
                            simple: "proc".into(),
                        },
                        policy
                    )
                    .is_none()
            );
            // A missing actual native row remains missing despite Registry
            // knowledge. No namespace geometry supplies that implementation.
            let key = super::super::SourceCommandKey::slot(root.clone(), "proc".into());
            assert_eq!(
                state.original_bindings_for_key(&key),
                Some(std::collections::BTreeSet::from([
                    super::super::MayBinding::Missing
                ]))
            );
            std::sync::Arc::make_mut(&mut state.original_command_world).withdraw();
            state.install_runtime_entry(&snapshot);
            assert!(state.original_namespace_geometry(&root, policy).is_none());
        }
        let mut incomplete = entry(NativeCommandImplementation::Opaque);
        incomplete.closed = false;
        let state = super::super::ModuleCommandBindings::initial_with_options(
            &registry,
            SourceAnalysisOptions {
                native_entry: Some(&incomplete),
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V9_0,
                )),
                ..Default::default()
            },
            None,
        );
        assert!(
            state
                .original_namespace_geometry(
                    &state.source_root_namespace_key().unwrap(),
                    incomplete.command_name_policy().unwrap()
                )
                .is_none()
        );
    }

    #[test]
    fn original_jim_native_world_keeps_root_lookup_without_unrelated_holder_geometry() {
        // Implementation contract: naming.command.original-native-entry-world-bootstrap
        // docs/design/analysis/name-resolution-proofs/command-original-native-entry-world-bootstrap.md
        let registry = tcl_registry::CommandRegistry::build_default();
        let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some("jimtcl")).unwrap();
        let dialect = tcl_registry::InvocationDialect::of_point(point);
        let mut snapshot = entry(NativeCommandImplementation::Registry {
            identity: "set".to_owned(),
            compiler_hook: false,
        });
        snapshot.execution_point = Some(point);
        snapshot.name_protocol = tcl_syntax::naming::NamePolicyProtocol::for_native_point(point);
        snapshot.source_string_protocol =
            Some(tcl_syntax::native_string::NativeStringProtocol::Jim084);
        snapshot.lexer_grammar = Some(dialect.lexer_grammar);
        snapshot.namespaces[0].jim_namespace_object = Some(tcl_core_types::NameBytes::default());
        snapshot.command_resolvers = Some(
            tcl_runtime_api::native_compilation::NativeCommandResolverInventory::captured(
                snapshot.interpreter,
                snapshot.epoch,
                tcl_runtime_api::native_compilation::NativeCommandResolverPresence::Absent,
            ),
        );
        let mut unrelated = snapshot.namespaces[0].clone();
        unrelated.token = 55;
        unrelated.path = tcl_core_types::ByteNamespacePath::from_segments(["unrelated"]);
        unrelated.jim_namespace_object = None;
        snapshot.namespaces.push(unrelated);
        let make_state = |snapshot: &NativeCompilationEntry| {
            super::super::ModuleCommandBindings::initial_with_options(
                &registry,
                SourceAnalysisOptions {
                    native_entry: Some(snapshot),
                    invocation_dialect: Some(dialect),
                    ..Default::default()
                },
                None,
            )
        };
        let mut state = make_state(&snapshot);
        let policy = snapshot.command_name_policy().unwrap();
        let root = state.source_root_namespace_key().unwrap();
        assert_eq!(
            state.original_namespace_geometry(&root, policy),
            Some(crate::signature_scan::scope::SignatureNamespaceScope::Jim(
                tcl_core_types::NameBytes::default()
            ))
        );
        let absent_geometry = native_namespace_key(&snapshot, 55).unwrap();
        assert!(
            state
                .original_namespace_geometry(&absent_geometry, policy)
                .is_none()
        );
        let key = super::super::SourceCommandKey::slot(root.clone(), "set".into());
        let bindings = state.original_bindings_for_key(&key).unwrap();
        assert!(
            matches!(bindings.iter().next(), Some(super::super::MayBinding::Target(target))
            if target.token.as_ref().and_then(|token| token.runtime).is_some_and(|runtime|
                runtime.interpreter == snapshot.interpreter && runtime.token == 4))
        );
        assert_eq!(bindings.len(), 1);
        std::sync::Arc::make_mut(&mut state.original_command_world).withdraw();
        state.install_runtime_entry(&snapshot);
        assert!(state.original_namespace_geometry(&root, policy).is_none());

        let mut current_without_object = snapshot.clone();
        current_without_object.current_namespace = 55;
        assert!(
            make_state(&current_without_object)
                .original_namespace_geometry(&root, policy)
                .is_none()
        );
        let mut root_without_object = snapshot;
        root_without_object.namespaces[0].jim_namespace_object = None;
        assert!(
            make_state(&root_without_object)
                .original_namespace_geometry(&root, policy)
                .is_none()
        );
    }

    #[test]
    fn native_lookup_cursor_keeps_ambiguous_paths_and_later_failures_separate() {
        use tcl_core_types::{ByteNamespacePath, NativeByteCommandSlot};
        use tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable;
        for version in tcl_dialect::TclVersion::ALL {
            let mut snapshot = entry(NativeCommandImplementation::Opaque);
            select_fixture_native_point(&mut snapshot, version);
            for (token, components) in [(1, ["a:", "b"]), (2, ["a", ":b"])] {
                let path = ByteNamespacePath::from_segments(components);
                let mut namespace = snapshot.namespaces[0].clone();
                namespace.path = path.clone();
                namespace.token = token;
                // This invalid fallback must not invalidate an earlier hit.
                namespace.command_path = vec![99];
                snapshot.namespaces.push(namespace);
                let mut command = snapshot.commands[0].clone();
                command.namespace_token = token;
                command.token = token + 10;
                command.slot = NativeByteCommandSlot::new(path, "p".into());
                snapshot.commands.push(command);
            }
            assert_ne!(snapshot.namespaces[1].path, snapshot.namespaces[2].path);
            for token in [1, 2] {
                assert_eq!(
                    snapshot
                        .lookup_command_bytes(token, b"p")
                        .unwrap()
                        .unwrap()
                        .token,
                    token + 10
                );
                let mut cursor = snapshot.command_lookup_cursor(token, b"p").unwrap();
                let first = cursor.next_candidate().unwrap().unwrap();
                assert_eq!(first.namespace_token, token);
                assert_eq!(
                    first.slot.namespace,
                    snapshot.namespaces[usize::try_from(token).unwrap()].path
                );
                if version.has_namespace_path() {
                    assert_eq!(
                        cursor.next_candidate(),
                        Err(NativeCommandLookupUnavailable::Namespace)
                    );
                }
            }
            let duplicate = snapshot.namespaces[1].clone();
            snapshot.namespaces.push(duplicate);
            assert!(matches!(
                snapshot.command_lookup_cursor(1, b"p"),
                Err(NativeCommandLookupUnavailable::Namespace)
            ));
        }
    }

    #[test]
    fn mutable_native_source_paths_stop_at_original_local_hits() {
        use super::super::{ModuleCommandBindings, SourceCommandKey, SourceNamespaceKey};
        use tcl_core_types::{ByteNamespacePath, NativeByteCommandSlot};
        for version in tcl_dialect::TclVersion::ALL {
            let mut snapshot = entry(NativeCommandImplementation::Opaque);
            select_fixture_native_point(&mut snapshot, version);
            snapshot.commands.clear();
            for (token, components) in [
                (1, vec!["a:", "b"]),
                (2, vec!["a", ":b"]),
                (3, vec!["caller"]),
            ] {
                let path = ByteNamespacePath::from_segments(components);
                let mut row = snapshot.namespaces[0].clone();
                row.token = token;
                row.path = path.clone();
                snapshot.namespaces.push(row);
                if token != 3 {
                    let mut command = entry(NativeCommandImplementation::Opaque)
                        .commands
                        .remove(0);
                    command.namespace_token = token;
                    command.token = token + 10;
                    command.slot = NativeByteCommandSlot::new(path, "p".into());
                    snapshot.commands.push(command);
                }
            }
            let registry = tcl_registry::CommandRegistry::build_default();
            let mut state = ModuleCommandBindings::initial_with_options(
                &registry,
                SourceAnalysisOptions {
                    native_entry: Some(&snapshot),
                    invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(version)),
                    ..SourceAnalysisOptions::default()
                },
                None,
            );
            let first = SourceNamespaceKey::Native(snapshot.retained_namespace_context(1).unwrap());
            let second =
                SourceNamespaceKey::Native(snapshot.retained_namespace_context(2).unwrap());
            let caller =
                SourceNamespaceKey::Native(snapshot.retained_namespace_context(3).unwrap());
            assert_eq!(first.display(), second.display());
            assert_ne!(first, second);
            // Unknown fallback cannot spoil an actual local hit.
            std::sync::Arc::make_mut(&mut state.unknown_namespace_paths).insert(first.clone());
            assert_eq!(
                state.source_keys_checked("p", &first).unwrap(),
                vec![SourceCommandKey::slot(first.clone(), "p".into())]
            );
            if version.has_namespace_path() {
                assert!(state.source_keys_checked("missing", &first).is_err());
                for selected in [&first, &second] {
                    std::sync::Arc::make_mut(&mut state.namespace_paths).insert(
                        caller.clone(),
                        std::collections::BTreeSet::from([vec![selected.clone()]]),
                    );
                    assert_eq!(
                        state.source_keys_checked("p", &caller).unwrap(),
                        vec![SourceCommandKey::slot(selected.clone(), "p".into())]
                    );
                }
                // A current-table shadow stops before the retained path.
                let local = SourceCommandKey::slot(caller.clone(), "p".into());
                let original = state
                    .bindings
                    .get(&SourceCommandKey::slot(first.clone(), "p".into()))
                    .unwrap()
                    .clone();
                std::sync::Arc::make_mut(&mut state.bindings).insert(local.clone(), original);
                assert_eq!(
                    state.source_keys_checked("p", &caller).unwrap(),
                    vec![local]
                );
            }
        }
    }

    #[test]
    fn handler_namespace_queries_keep_original_context_and_declared_operands_separate() {
        use super::super::{ModuleCommandBindings, SourceNamespaceKey};
        use tcl_core_types::ByteNamespacePath;
        let mut snapshot = entry(NativeCommandImplementation::Opaque);
        select_fixture_native_point(&mut snapshot, tcl_dialect::TclVersion::V8_6);
        for (token, path) in [(1, ["a:", "b"]), (2, ["a", ":b"])] {
            let mut row = snapshot.namespaces[0].clone();
            row.token = token;
            row.path = ByteNamespacePath::from_segments(path);
            snapshot.namespaces.push(row);
        }
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut state = ModuleCommandBindings::initial_with_options(
            &registry,
            SourceAnalysisOptions {
                native_entry: Some(&snapshot),
                ..SourceAnalysisOptions::default()
            },
            None,
        );
        let root = SourceNamespaceKey::Native(snapshot.retained_namespace_context(0).unwrap());
        let left = SourceNamespaceKey::Native(snapshot.retained_namespace_context(1).unwrap());
        let right = SourceNamespaceKey::Native(snapshot.retained_namespace_context(2).unwrap());
        assert_eq!(left.display(), right.display());
        assert_ne!(left, right);
        assert_eq!(state.namespace_for_rooted_operand("::"), Some(root));
        // A declared written operand cannot impersonate either colliding report.
        assert_eq!(state.namespace_for_rooted_operand("::a:::b"), None);
        std::sync::Arc::make_mut(&mut state.source_variables).namespace_known = true;
        std::sync::Arc::make_mut(&mut state.source_variables).namespace = left.display().unwrap();
        std::sync::Arc::make_mut(&mut state.source_variables).namespace_identity =
            Some(left.clone());
        assert_eq!(state.variable_frame_namespace_key(), Some(left));
        std::sync::Arc::make_mut(&mut state.source_variables).namespace_identity =
            Some(right.clone());
        assert_eq!(state.variable_frame_namespace_key(), Some(right));
        std::sync::Arc::make_mut(&mut state.source_variables).namespace_identity = None;
        assert_eq!(state.variable_frame_namespace_key(), None);
    }

    #[test]
    fn allocated_native_namespace_replacement_does_not_revive_old_context() {
        use super::super::{
            ModuleCommandBindings, SourceCommandKey, SourceNamespaceKey, SourceOriginId,
        };
        use tcl_core_types::ByteNamespacePath;
        use tcl_registry::{NamespaceTransitionTarget, TransitionSubject};
        let mut snapshot = entry(NativeCommandImplementation::Opaque);
        select_fixture_native_point(&mut snapshot, tcl_dialect::TclVersion::V8_6);
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut state = ModuleCommandBindings::initial_with_options(
            &registry,
            SourceAnalysisOptions {
                native_entry: Some(&snapshot),
                ..SourceAnalysisOptions::default()
            },
            None,
        );
        state.current_source_origin = Some(std::sync::Arc::new(SourceOriginId::authored(
            &std::sync::Arc::from("namespace eval holder {}"),
        )));
        let root = SourceNamespaceKey::Native(snapshot.retained_namespace_context(0).unwrap());
        let target =
            NamespaceTransitionTarget::Named(TransitionSubject::Literal("holder".to_owned()));
        let old = state.ensure_namespace_key_at(&target, &root, 0).unwrap();
        std::sync::Arc::make_mut(&mut state.namespaces).remove(&old);
        let fresh = state.ensure_namespace_key_at(&target, &root, 0).unwrap();
        assert_ne!(old, fresh);
        assert_eq!(old.exact_native_path(), fresh.exact_native_path());
        assert_eq!(
            fresh.exact_native_path(),
            Some(&ByteNamespacePath::from_segments(["holder"]))
        );
        let child_target =
            NamespaceTransitionTarget::Named(TransitionSubject::Literal("child".to_owned()));
        let child = state
            .ensure_namespace_key_at(&child_target, &fresh, 1)
            .unwrap();
        assert!(state.namespace_target_key_at(&child_target, &old).is_none());
        assert!(
            state
                .ensure_namespace_key_at(&child_target, &old, 2)
                .is_none()
        );
        let command = SourceCommandKey::slot(child.clone(), "p".into());
        let original = state
            .bindings
            .get(&SourceCommandKey::slot(root.clone(), "set".into()))
            .unwrap()
            .clone();
        std::sync::Arc::make_mut(&mut state.bindings).insert(command.clone(), original);
        assert_eq!(
            state.source_keys_checked("child::p", &fresh).unwrap(),
            vec![command]
        );
        assert!(state.source_keys_checked("child::p", &old).is_err());
        assert_eq!(
            state.source_keys_checked("p", &child).unwrap(),
            vec![SourceCommandKey::slot(child, "p".into())]
        );
    }

    #[test]
    fn retained_deleted_current_context_keeps_lazy_global_fallback() {
        use super::super::{ModuleCommandBindings, SourceCommandKey, SourceNamespaceKey};
        use tcl_core_types::ByteNamespacePath;
        use tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable;
        for version in tcl_dialect::TclVersion::ALL {
            let mut snapshot = entry(NativeCommandImplementation::Opaque);
            select_fixture_native_point(&mut snapshot, version);
            let mut retired = snapshot.namespaces[0].clone();
            retired.token = 1;
            retired.path = ByteNamespacePath::from_segments(["retired"]);
            retired.visible = false;
            snapshot.namespaces.push(retired);
            snapshot.current_namespace = 1;
            assert_eq!(
                snapshot
                    .lookup_command_bytes(1, b"set")
                    .unwrap()
                    .unwrap()
                    .token,
                4
            );
            assert_eq!(
                snapshot.lookup_command_bytes(1, b"child::p"),
                Err(NativeCommandLookupUnavailable::RetainedDescendant)
            );
            let registry = tcl_registry::CommandRegistry::build_default();
            let state = ModuleCommandBindings::initial_with_options(
                &registry,
                SourceAnalysisOptions {
                    native_entry: Some(&snapshot),
                    ..SourceAnalysisOptions::default()
                },
                None,
            );
            let retained =
                SourceNamespaceKey::Native(snapshot.retained_namespace_context(1).unwrap());
            let root = SourceNamespaceKey::Native(snapshot.retained_namespace_context(0).unwrap());
            assert!(!state.namespaces.contains(&retained));
            assert_eq!(
                state.source_keys_checked("set", &retained).unwrap(),
                vec![SourceCommandKey::slot(root, "set".into())]
            );
            assert_eq!(
                state.source_keys_checked("child::p", &retained),
                Err(NativeCommandLookupUnavailable::RetainedDescendant)
            );
        }
    }

    #[test]
    fn native_colon_context_lookup_does_not_publish_a_global_source_alias() {
        use crate::command_binding::SourceCommandSlotPresence as Presence;
        use tcl_core_types::{ByteNamespacePath, NativeByteCommandSlot};
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
        for version in tcl_dialect::TclVersion::ALL {
            let mut snapshot = entry(NativeCommandImplementation::Registry {
                identity: "set".to_owned(),
                compiler_hook: false,
            });
            select_fixture_native_point(&mut snapshot, version);
            let path = ByteNamespacePath::from_segments([b":".as_slice()]);
            snapshot.current_namespace = 1;
            snapshot.frame = tcl_runtime_api::native_compilation::NativeCompilationFrame::Namespace;
            let mut namespace = snapshot.namespaces[0].clone();
            namespace.path = path.clone();
            namespace.token = 1;
            snapshot.namespaces.push(namespace);
            let mut command = snapshot.commands[0].clone();
            command.slot = NativeByteCommandSlot::new(path, "p".into());
            command.namespace_token = 1;
            command.token = 5;
            command.compiler_hook = NativeCompilerHookPresence::Absent;
            snapshot.commands.push(command);
            assert_eq!(
                analytical_namespace_context_key(&snapshot, &snapshot.namespaces[1]).as_deref(),
                Some(":::")
            );
            assert!(analytical_command_name(&snapshot, &snapshot.commands[1].slot).is_none());
            assert_eq!(
                snapshot
                    .lookup_command_bytes(1, b"p")
                    .unwrap()
                    .unwrap()
                    .token,
                5
            );
            assert!(
                snapshot
                    .lookup_command_bytes(1, b":::::p")
                    .unwrap()
                    .is_none()
            );
            let analysis = SourceCommandBindings::analyse_in_namespace_with_options(
                "p x 1",
                ":::",
                tcl_lexer::LexerConfig::default(),
                &tcl_registry::CommandRegistry::build_default(),
                SourceAnalysisOptions {
                    native_entry: Some(&snapshot),
                    native_compilation: NativeCompilationContext {
                        mode: NativeCompilationMode::BytecodeObject,
                        frame: NativeCompilationFrame::ScriptCode,
                        loop_depth: 0,
                        catch_depth: Some(0),
                    },
                    ..SourceAnalysisOptions::default()
                },
            );
            let local = analysis.invocation_at_source("p", 0);
            assert_eq!(
                local.selected_slot_presence(),
                Presence::Present,
                "{version:?}"
            );
            assert_eq!(
                local.lookup_command_word(":::::p").selected_slot_presence(),
                Presence::Absent,
                "{version:?}"
            );
            assert!(
                !analysis.native_compilation_provider_required_at(0),
                "{version:?}"
            );
        }
    }

    fn imported_entry(mut snapshot: NativeCompilationEntry) -> NativeCompilationEntry {
        let mut imported = snapshot.commands[0].clone();
        if imported.compiler.is_none()
            && let NativeCommandImplementation::Registry {
                identity,
                compiler_hook: true,
            } = &imported.implementation
        {
            imported.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
                registry_identity: identity.clone(),
                ensemble: None,
            });
        }
        snapshot.commands[0].slot.simple = "origin".into();
        imported.token = 100;
        imported.implementation_generation = 101;
        imported.implementation = NativeCommandImplementation::Imported {
            interpreter: snapshot.interpreter,
            token: snapshot.commands[0].token,
            slot: Some(snapshot.commands[0].slot.clone()),
        };
        snapshot.commands.push(imported);
        snapshot
    }

    // Native proof: naming.namespace.unknown-compile-dispatch
    // docs/design/analysis/name-resolution-proofs/namespace-unknown-compile-dispatch.md
    #[test]
    fn native_namespace_unknown_lookup_fixture_separates_compile_and_dispatch() {
        let expected = "existing KNOWN 0\ncompiler OK 0\nfallback FALLBACK 1\nrootedExisting KNOWN 1\nrootedFallback FALLBACK 2\n";
        for output in [
            include_str!("../../tests/data/native_namespace_unknown_lookup/8.5.19.tsv"),
            include_str!("../../tests/data/native_namespace_unknown_lookup/8.6.18.tsv"),
            include_str!("../../tests/data/native_namespace_unknown_lookup/9.0.4.tsv"),
            include_str!("../../tests/data/native_namespace_unknown_lookup/9.1.0.tsv"),
        ] {
            assert_eq!(output, expected);
        }
        for output in [
            include_str!("../../tests/data/native_namespace_unknown_lookup/8.4.20.tsv"),
            include_str!("../../tests/data/native_namespace_unknown_lookup/jim.tsv"),
        ] {
            assert!(output.starts_with("unsupported 1"));
        }
    }

    #[test]
    fn namespace_unknown_handler_does_not_taint_known_native_compiler_lookup() {
        use crate::command_binding::SourceCommandSlotPresence as Presence;
        use tcl_registry::native_compilation::NativeCompilationSelection as Selection;
        let mut snapshot = entry(NativeCommandImplementation::Registry {
            identity: "set".to_owned(),
            compiler_hook: true,
        });
        snapshot.commands[0].compiler =
            Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
                registry_identity: "set".into(),
                ensemble: None,
            });
        select_fixture_native_point(&mut snapshot, tcl_dialect::TclVersion::V9_0);
        snapshot.namespaces[0].unknown_handler = Some(vec![None]);
        let mut default_handler = snapshot.commands[0].clone();
        default_handler.slot.simple = "unknown".into();
        default_handler.token = 5;
        default_handler.compiler_hook =
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent;
        default_handler.implementation = NativeCommandImplementation::Registry {
            identity: "unknown".to_owned(),
            compiler_hook: false,
        };
        snapshot.commands.push(default_handler);
        let inventory = analyse(&snapshot);
        let known = inventory.invocation_at_source("set", 0);
        assert_eq!(known.selected_slot_presence(), Presence::Present);
        assert!(matches!(
            known.native_compilation_admission_selection(),
            Selection::Inline { .. }
        ));
        assert!(!inventory.native_compilation_provider_required_at(0));
        for name in ["missing", "::missing"] {
            let missing = known.lookup_command_word(name);
            assert_eq!(missing.selected_slot_presence(), Presence::Absent);
            assert_eq!(
                missing.selected_slot_diagnostic_presence(),
                Presence::Unknown
            );
            assert!(missing.unknown);
            assert!(missing.targets.is_empty());
        }
        snapshot.commands[0].compiler_hook =
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent;
        let inventory = analyse(&snapshot);
        assert_eq!(
            inventory
                .invocation_at_source("set", 0)
                .native_compilation_admission_selection(),
            Selection::Generic
        );
        assert!(!inventory.native_compilation_provider_required_at(0));
        snapshot.namespaces[0].command_path = vec![999];
        let inventory = analyse(&snapshot);
        // The actual local row stops lookup before the unavailable later
        // path. Native path-stop controls preserve LOCAL/ROOT without running
        // the unknown handler; only a reached miss needs the later geometry.
        assert_eq!(
            inventory
                .invocation_at_source("set", 0)
                .native_compilation_admission_selection(),
            Selection::Generic
        );
        assert!(!inventory.native_compilation_provider_required_at(0));
        let missing = analyse_source(&snapshot, "missing value");
        assert_eq!(
            missing
                .invocation_at_source("missing", 0)
                .native_compilation_admission_selection(),
            Selection::Unknown
        );
        assert!(missing.native_compilation_provider_required_at(0));
    }

    #[test]
    fn logical_invocation_policy_cannot_supply_a_missing_actual_compiler_point() {
        let mut snapshot = imported_entry(entry(NativeCommandImplementation::Registry {
            identity: "set".to_owned(),
            compiler_hook: true,
        }));
        snapshot.execution_point = None;
        let proof = analyse(&snapshot).invocation_at_source("set", 0);
        assert_eq!(
            proof.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
        assert!(proof.admitted_inline_invocation().is_none());
        assert!(analyse(&snapshot).native_compilation_provider_required_at(0));
    }

    #[test]
    fn imported_raw_hook_is_independent_of_current_callable_origin() {
        use tcl_registry::native_compilation::NativeCompilationSelection as Selection;
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
        for (raw, origin, expected) in [
            (Hook::Absent, Hook::Present, Some(Selection::Generic)),
            (Hook::Present, Hook::Absent, None),
            (Hook::Unknown, Hook::Present, Some(Selection::Unknown)),
        ] {
            let mut snapshot = imported_entry(entry(NativeCommandImplementation::Registry {
                identity: "set".to_owned(),
                compiler_hook: true,
            }));
            select_fixture_native_point(&mut snapshot, tcl_dialect::TclVersion::V9_0);
            snapshot.commands[0].compiler_hook = origin;
            snapshot.commands[1].compiler_hook = raw;
            let proof = analyse(&snapshot).invocation_at_source("set", 0);
            if let Some(expected) = expected {
                assert_eq!(
                    proof.native_compilation_admission_selection(),
                    expected,
                    "{raw:?}/{origin:?}"
                );
            } else {
                assert!(
                    matches!(
                        proof.native_compilation_admission_selection(),
                        Selection::Inline { .. }
                    ),
                    "{raw:?}/{origin:?}"
                );
            }
            assert!(
                proof
                    .targets
                    .iter()
                    .any(|target| target.registry_backed && target.command == "set")
            );
            snapshot.commands[1].has_execution_trace = true;
            assert_eq!(
                analyse(&snapshot)
                    .invocation_at_source("set", 0)
                    .native_compilation_admission_selection(),
                Selection::Generic
            );
        }
    }

    #[test]
    fn copied_imported_registration_requires_original_source_string_protocol() {
        use tcl_registry::native_compilation::NativeCompilationSelection as Selection;
        for implementation in [
            NativeCommandImplementation::Opaque,
            NativeCommandImplementation::Registry {
                identity: "list".to_owned(),
                compiler_hook: true,
            },
        ] {
            let mut snapshot = imported_entry(entry(NativeCommandImplementation::Registry {
                identity: "set".to_owned(),
                compiler_hook: true,
            }));
            select_fixture_native_point(&mut snapshot, tcl_dialect::TclVersion::V9_0);
            snapshot.commands[0].implementation = implementation;
            let admitted = analyse(&snapshot).invocation_at_source("set", 0);
            let proof = admitted
                .admitted_inline_invocation()
                .expect("original C9 source issuer");
            assert_eq!(proof.target.registry_identity(), Some("set"));
            assert_eq!(proof.compiler_prerequisite.as_ref().unwrap().token, 100);
            assert_eq!(
                proof
                    .target
                    .identity
                    .as_ref()
                    .unwrap()
                    .runtime
                    .unwrap()
                    .token,
                100
            );

            // Hook, recipe, raw wrapper and changed callable remain identical;
            // absence of the physical source-string issuer withdraws CPP only.
            snapshot.source_string_protocol = None;
            let missing = analyse(&snapshot).invocation_at_source("set", 0);
            assert_eq!(
                missing.native_compilation_admission_selection(),
                Selection::Unknown
            );
            assert!(missing.admitted_inline_invocation().is_none());
            assert_eq!(snapshot.commands[1].token, 100);
            assert_eq!(
                snapshot.commands[1]
                    .compiler
                    .as_ref()
                    .unwrap()
                    .registry_identity,
                "set"
            );
        }
    }

    #[test]
    fn imported_registration_does_not_follow_replaced_callable_semantics() {
        use tcl_registry::native_compilation::NativeCompilationSelection as Selection;
        for implementation in [
            NativeCommandImplementation::Opaque,
            NativeCommandImplementation::Registry {
                identity: "list".to_owned(),
                compiler_hook: true,
            },
        ] {
            let mut snapshot = imported_entry(entry(NativeCommandImplementation::Registry {
                identity: "set".to_owned(),
                compiler_hook: true,
            }));
            select_fixture_native_point(&mut snapshot, tcl_dialect::TclVersion::V9_0);
            snapshot.commands[0].implementation = implementation;
            let analysis = analyse(&snapshot);
            let proof = analysis.invocation_at_source("set", 0);
            let Selection::Inline { operation, .. } =
                proof.native_compilation_admission_selection()
            else {
                panic!("copied Set compiler remains available independently of callable origin");
            };
            let admitted = proof.admitted_inline_invocation().unwrap();
            assert_eq!(admitted.target.command, "set");
            assert_eq!(admitted.operation, operation);
            assert_eq!(admitted.compiler_prerequisite.as_ref().unwrap().token, 100);
            assert!(proof.targets.iter().all(|target| target.command != "set"));
            assert!(
                proof
                    .proved_target()
                    .is_none_or(|target| target.command != "set")
            );
        }
    }

    #[test]
    fn imported_noop_prerequisite_names_the_actual_raw_wrapper() {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
        let mut original = entry(NativeCommandImplementation::Opaque);
        original.commands[0].compiler_hook = Hook::Present;
        original.commands[0].procedure_header =
            Some(tcl_dialect::NativeProcedureHeaderCompilation::NoOp);
        let mut snapshot = imported_entry(original);
        snapshot.commands[0].compiler_hook = Hook::Absent;
        snapshot.commands[0].procedure_header =
            Some(tcl_dialect::NativeProcedureHeaderCompilation::Absent);
        let analysis = analyse(&snapshot);
        let proof = analysis.invocation_at_source("set", 0);
        let [selected] = proof.compiled_candidates.as_slice() else {
            panic!("raw imported NoOp must emit its actual operation")
        };
        let prerequisite = selected.procedure_header_prerequisite.as_ref().unwrap();
        assert_eq!(prerequisite.token, 100);
        assert_eq!(prerequisite.implementation_generation, 101);
    }

    #[test]
    fn imported_ensemble_retains_copied_hook_and_actual_raw_prerequisite() {
        use tcl_registry::native_compilation::NativeCompilationSelection as Selection;
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut snapshot = imported_entry(mapped_ensemble_entry(Some("::helper")));
            let raw = snapshot.commands.len() - 1;
            snapshot.commands[0].compiler_hook = Hook::Absent;
            let analysis = analyse_mapped_ensemble(&snapshot, version);
            let proof = analysis.invocation_at_source("info", 0);
            if version == tcl_dialect::TclVersion::V8_5 {
                assert_eq!(
                    proof.native_compilation_admission_selection(),
                    Selection::Generic
                );
            } else {
                let prerequisite = proof
                    .named_invocation()
                    .expect("actual copied ensemble hook")
                    .compiler_prerequisite
                    .as_ref()
                    .unwrap();
                assert_eq!(prerequisite.token, snapshot.commands[raw].token);
                assert_eq!(prerequisite.implementation_generation, 101);
            }
            snapshot.commands[raw].compiler_hook = Hook::Absent;
            snapshot.commands[0].compiler_hook = Hook::Present;
            assert_eq!(
                analyse_mapped_ensemble(&snapshot, version)
                    .invocation_at_source("info", 0)
                    .native_compilation_admission_selection(),
                Selection::Generic
            );
        }
    }

    #[test]
    fn actual_observer_table_closes_only_callback_uncertainty() {
        use tcl_runtime_api::native_compilation::NativeVariableObserverPresence as Presence;
        let source = "incr ::count; return x";
        let registry = tcl_registry::CommandRegistry::build_default();
        for presence in [
            Presence::Absent,
            Presence::BoundedNativeOnly,
            Presence::Present,
            Presence::Unknown,
        ] {
            let mut snapshot = entry(NativeCommandImplementation::Registry {
                identity: "incr".to_owned(),
                compiler_hook: true,
            });
            snapshot.commands[0].slot.simple = "incr".into();
            let mut return_row = snapshot.commands[0].clone();
            return_row.slot.simple = "return".into();
            return_row.token += 1;
            return_row.implementation = NativeCommandImplementation::Registry {
                identity: "return".to_owned(),
                compiler_hook: true,
            };
            snapshot.commands.push(return_row);
            snapshot.variable_observers = presence;
            select_fixture_native_point(&mut snapshot, tcl_dialect::TclVersion::V8_5);
            let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::".to_owned(),
                identity: "actual P".to_owned(),
            };
            let analysis = SourceCommandBindings::analyse_in_frame_with_options(
                source,
                &frame,
                tcl_lexer::LexerConfig::default(),
                &registry,
                SourceAnalysisOptions {
                    native_entry: Some(&snapshot),
                    invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                        tcl_dialect::TclVersion::V8_5,
                    )),
                    native_compilation: NativeCompilationContext {
                        mode: NativeCompilationMode::BytecodeObject,
                        frame: NativeCompilationFrame::ProcedureCode,
                        loop_depth: 0,
                        catch_depth: Some(0),
                    },
                    ..SourceAnalysisOptions::default()
                },
            );
            let first = analysis.invocation_at_source("incr", 0);
            assert_eq!(
                first.variable_context.dynamic_traces,
                !presence.permits_no_callbacks()
            );
            assert!(
                first
                    .variable_context
                    .literal_value("::count", &registry)
                    .is_none()
            );
            assert!(!first.variable_context.namespace_cells.closed);
            let next = analysis.invocation_at_source("return", 14);
            assert!(next.proved_execution_target().is_none(), "{presence:?}");
            assert!(
                matches!(
                    next.native_compilation_admission_selection(),
                    tcl_registry::native_compilation::NativeCompilationSelection::Inline { .. }
                ),
                "{presence:?}: observer absence does not describe incoming value objects"
            );
        }
    }

    #[test]
    fn observer_effects_do_not_revoke_closed_compiler_admission() {
        use tcl_registry::native_compilation::NativeCompilationSelection as Selection;
        use tcl_runtime_api::native_compilation::NativeVariableObserverPresence as Presence;
        let source = "incr ::count; return x";
        let registry = tcl_registry::CommandRegistry::build_default();
        for version in [
            tcl_dialect::TclVersion::V8_4,
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut snapshot = entry(NativeCommandImplementation::Registry {
                identity: "incr".to_owned(),
                compiler_hook: true,
            });
            snapshot.commands[0].slot.simple = "incr".into();
            let mut return_row = snapshot.commands[0].clone();
            return_row.slot.simple = "return".into();
            return_row.token += 1;
            return_row.implementation = NativeCommandImplementation::Registry {
                identity: "return".to_owned(),
                compiler_hook: true,
            };
            snapshot.commands.push(return_row);
            snapshot.variable_observers = Presence::Present;
            select_fixture_native_point(&mut snapshot, version);
            let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::".to_owned(),
                identity: "actual P".to_owned(),
            };
            let analysis = SourceCommandBindings::analyse_in_frame_with_options(
                source,
                &frame,
                tcl_lexer::LexerConfig::default(),
                &registry,
                SourceAnalysisOptions {
                    native_entry: Some(&snapshot),
                    invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(version)),
                    native_compilation: NativeCompilationContext {
                        mode: NativeCompilationMode::BytecodeObject,
                        frame: NativeCompilationFrame::ProcedureCode,
                        loop_depth: 0,
                        catch_depth: Some(0),
                    },
                    ..SourceAnalysisOptions::default()
                },
            );
            assert_observed_compiler_plan(&analysis, version);
        }
        let foreign = analyse(&entry(NativeCommandImplementation::Opaque));
        assert_eq!(
            foreign
                .invocation_at_source("set", 0)
                .native_compilation_admission_selection(),
            Selection::Unknown
        );
    }

    fn assert_observed_compiler_plan(
        analysis: &SourceCommandBindings,
        version: tcl_dialect::TclVersion,
    ) {
        use tcl_registry::native_compilation::NativeCompilationSelection as Selection;
        let source = "incr ::count; return x";
        let next = analysis.invocation_at_source("return", 14);
        assert_ne!(
            next.native_compilation_admission_selection(),
            Selection::Unknown,
            "{version:?}"
        );
        assert!(next.variable_context.dynamic_traces, "{version:?}");
        let segments = crate::segmenter::segment_commands(source);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            tcl_lexer::LexerConfig::default(),
            &segments[1],
        );
        analysis.stamp_original_tokens(&mut tokens);
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let plan = crate::registry_invocation::native_operation_selection_plan(
            &tokens,
            dialect.lexer_grammar.escapes,
            dialect.word_values,
        )
        .unwrap();
        let plan = plan.expect("closed inline admission retains its replay plan");
        assert_eq!(plan.source, "return x");
        assert_ne!(
            plan.requirements,
            [] as [tcl_runtime_api::CommandBindingIdentity; 0]
        );
        if version != tcl_dialect::TclVersion::V8_4 {
            assert!(crate::registry_invocation::proved_native_inline_operation(&tokens).is_none());
            assert!(next.proved_execution_target().is_none());
        }
    }

    #[test]
    fn opaque_noop_header_selects_only_the_certified_empty_operation() {
        let mut snapshot = entry(NativeCommandImplementation::Opaque);
        snapshot.commands[0].compiler_hook =
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Present;
        snapshot.commands[0].procedure_header =
            Some(tcl_dialect::NativeProcedureHeaderCompilation::NoOp);
        let analysis = analyse(&snapshot);
        let binding = analysis.invocation_at_source("set", 0);
        let [selected] = binding.compiled_candidates.as_slice() else {
            panic!("exact native noop operation required");
        };
        assert_eq!(
            selected.operation,
            tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::ProcedureNoOp)
        );
        assert!(!selected.target.registry_backed);
        let prerequisite = selected.procedure_header_prerequisite.as_ref().unwrap();
        assert_eq!(prerequisite.token, snapshot.commands[0].token);
        assert_eq!(prerequisite.implementation_generation, u64::MAX);
        assert_eq!(prerequisite.invocation_word.as_bytes(), b"set");
        assert_eq!(prerequisite.lookup_namespace_token, 0);
        assert!(
            binding
                .variable_context
                .literal_value("x", &tcl_registry::CommandRegistry::build_default())
                .is_none()
        );
        assert!(!analysis.native_compilation_provider_required_at(0));
    }

    #[test]
    fn nested_noop_keeps_the_actual_header_selected_before_argument_replacement() {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut snapshot = entry(NativeCommandImplementation::Opaque);
        snapshot.commands[0].slot.simple = "noop".into();
        snapshot.commands[0].compiler_hook = NativeCompilerHookPresence::Present;
        snapshot.commands[0].procedure_header =
            Some(tcl_dialect::NativeProcedureHeaderCompilation::NoOp);
        snapshot.frame = tcl_runtime_api::native_compilation::NativeCompilationFrame::Procedure;
        for (index, name) in ["set", "incr", "proc", "list", "return"]
            .into_iter()
            .enumerate()
        {
            let hook = registry
                .get(name)
                .unwrap()
                .native_compilation
                .and_then(|spec| spec.compiler_hook_presence(dialect))
                .unwrap();
            snapshot.commands.push(NativeCompilationBinding {
                slot: tcl_core_types::NativeByteCommandSlot::new(
                    tcl_core_types::ByteNamespacePath::root(),
                    name.into(),
                ),
                namespace_token: 0,
                token: 10 + index as u64,
                implementation_generation: 0,
                compiler_hook: if hook {
                    NativeCompilerHookPresence::Present
                } else {
                    NativeCompilerHookPresence::Absent
                },
                implementation: NativeCommandImplementation::Registry {
                    identity: format!("::{name}"),
                    compiler_hook: hook,
                },
                compiler: None,
                procedure_header: None,
                has_execution_trace: false,
            });
        }
        let source =
            "set value [noop [incr ::count; proc noop args {return CUSTOM}]]; list $value $::count";
        let analysis = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &registry,
            SourceAnalysisOptions {
                native_entry: Some(&snapshot),
                invocation_dialect: Some(dialect),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ProcedureCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                ..SourceAnalysisOptions::default()
            },
        );
        let offset = u32::try_from(source.find("noop [").unwrap()).unwrap();
        let proof = analysis.invocation_at_source("", offset);
        assert!(
            proof
                .compiled_candidates
                .iter()
                .any(|candidate| candidate.operation
                    == tcl_registry::SemanticOperationId::Intrinsic(
                        tcl_registry::IntrinsicId::ProcedureNoOp
                    ))
        );
        // The original parent carrier must retain this exact nested operation.
        let segments = crate::segmenter::segment_commands(source);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            tcl_lexer::LexerConfig::default(),
            &segments[0],
        );
        analysis.stamp_original_tokens(&mut tokens);
        assert!(tokens.nested_bindings.iter().any(|(site, binding)| {
            *site == offset
                && binding
                    .compiled_candidates
                    .iter()
                    .any(|candidate| candidate.procedure_header_prerequisite.is_some())
        }));
    }

    #[test]
    fn traced_noop_closes_generic_compilation_but_keeps_execution_unknown() {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
        let mut snapshot = entry(NativeCommandImplementation::Opaque);
        snapshot.commands[0].compiler_hook = NativeCompilerHookPresence::Present;
        snapshot.commands[0].procedure_header =
            Some(tcl_dialect::NativeProcedureHeaderCompilation::NoOp);
        snapshot.commands[0].has_execution_trace = true;
        let analysis = analyse(&snapshot);
        let binding = analysis.invocation_at_source("", 0);
        assert!(!analysis.native_compilation_provider_required_at(0));
        assert_eq!(
            binding.compiled_candidates,
            [] as [super::super::SourceCompiledInvocationProof; 0]
        );
        assert!(binding.may_use_live_dispatch);
        assert!(binding.unknown);
        assert!(binding.proved_execution_target().is_none());
    }

    #[test]
    fn opaque_runtime_hook_absence_closes_only_generic_compilation() {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
        let mut snapshot = entry(NativeCommandImplementation::Opaque);
        snapshot.commands[0].compiler_hook = NativeCompilerHookPresence::Absent;
        let generic = analyse(&snapshot);
        assert!(!generic.native_compilation_provider_required_at(0));
        let proof = generic.invocation_at_source("set", 0);
        assert_eq!(
            proof.compiled_candidates,
            [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
        );
        assert!(
            proof
                .proved_execution_target()
                .is_some_and(|target| !target.registry_backed)
        );
        for presence in [
            NativeCompilerHookPresence::Present,
            NativeCompilerHookPresence::Unknown,
        ] {
            snapshot.commands[0].compiler_hook = presence;
            let uncertain = analyse(&snapshot);
            assert!(uncertain.native_compilation_provider_required_at(0));
            assert!(
                uncertain
                    .invocation_at_source("set", 0)
                    .proved_execution_target()
                    .is_none()
            );
        }
    }

    #[test]
    fn foreign_alias_hook_absence_retains_only_the_actual_wrapper() {
        use tcl_runtime_api::native_compilation::{
            NativeAliasTargetLookup, NativeCompilerHookPresence,
        };
        let mut snapshot = entry(NativeCommandImplementation::Alias {
            interpreter: NativeInterpreterIdentity {
                owner: 1,
                interpreter: 1,
            },
            target_lookup: NativeAliasTargetLookup::Global,
            words: vec!["set".into(), "parent_cell".into()],
        });
        snapshot.commands[0].compiler_hook = NativeCompilerHookPresence::Absent;
        let generic = analyse(&snapshot);
        assert!(!generic.native_compilation_provider_required_at(0));
        let proof = generic.invocation_at_source("set", 0);
        assert_eq!(
            proof.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Generic
        );
        let target = proof
            .proved_execution_target()
            .expect("actual installed child wrapper");
        assert!(!target.registry_backed);
        assert!(target.prepended.is_empty());
        assert_eq!(target.kind, BindingKind::Command);
        assert_eq!(
            target
                .identity
                .as_ref()
                .unwrap()
                .runtime
                .unwrap()
                .interpreter,
            snapshot.interpreter
        );
        for presence in [
            NativeCompilerHookPresence::Present,
            NativeCompilerHookPresence::Unknown,
        ] {
            snapshot.commands[0].compiler_hook = presence;
            assert!(analyse(&snapshot).native_compilation_provider_required_at(0));
        }
    }

    #[test]
    fn live_custom_slot_never_acquires_fresh_stock_identity() {
        let mut actual = entry(NativeCommandImplementation::Opaque);
        actual.commands[0].compiler_hook =
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent;
        let analysis = analyse(&actual);
        let proof = analysis.invocation_at_source("set", 0);
        assert_eq!(
            proof.compiled_candidates,
            [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
        );
        assert!(
            proof
                .proved_target()
                .is_some_and(|target| !target.registry_backed)
        );
        assert!(
            proof
                .proved_execution_target()
                .is_some_and(|target| !target.registry_backed)
        );
    }

    #[test]
    fn opaque_slot_with_unknown_hook_retains_compilation_obligation() {
        let analysis = analyse(&entry(NativeCommandImplementation::Opaque));
        let proof = analysis.invocation_at_source("set", 0);
        assert!(
            proof
                .proved_target()
                .is_some_and(|target| !target.registry_backed)
        );
        assert!(proof.proved_execution_target().is_none());
        assert!(analysis.native_compilation_provider_required_at(0));
    }

    #[test]
    fn actual_native_hook_and_full_generation_are_retained() {
        let mut snapshot = entry(NativeCommandImplementation::Registry {
            identity: "::set".to_owned(),
            compiler_hook: true,
        });
        snapshot.commands[0].compiler =
            Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
                registry_identity: "set".to_owned(),
                ensemble: None,
            });
        select_fixture_native_point(&mut snapshot, tcl_dialect::TclVersion::V9_0);
        let analysis = analyse(&snapshot);
        let proof = analysis.invocation_at_source("set", 0);
        let target = proof.proved_execution_target().expect("actual native hook");
        assert_eq!(target.runtime_implementation_generation, Some(u64::MAX));
        assert_eq!(target.identity.as_ref().unwrap().runtime.unwrap().token, 4);
        assert!(!proof.may_use_live_dispatch);
    }

    #[test]
    fn runtime_no_hook_and_command_observers_block_native_selection() {
        let without = analyse(&entry(NativeCommandImplementation::Registry {
            identity: "::set".to_owned(),
            compiler_hook: false,
        }));
        assert_eq!(
            without.invocation_at_source("set", 0).compiled_candidates,
            [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
        );
        let mut traced = entry(NativeCommandImplementation::Registry {
            identity: "::set".to_owned(),
            compiler_hook: true,
        });
        traced.commands[0].has_execution_trace = true;
        let proof = analyse(&traced).invocation_at_source("set", 0);
        assert_eq!(
            proof.compiled_candidates,
            [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
        );
        assert!(proof.proved_execution_target().is_none());
    }

    #[test]
    fn live_missing_rows_do_not_fall_back_to_catalogue_bindings() {
        let mut entry = entry(NativeCommandImplementation::Opaque);
        entry.commands.clear();
        let proof = analyse(&entry).invocation_at_source("set", 0);
        assert_eq!(
            proof.targets,
            [] as [crate::command_binding::SourceCommandTarget; 0]
        );
        assert!(proof.may_be_absent);
    }
    fn mapped_ensemble_entry(worker_bytes: Option<&str>) -> NativeCompilationEntry {
        use tcl_runtime_api::native_compilation::{
            NativeCommandCompiler, NativeCompilerHookPresence, NativeEnsembleCompiler,
        };
        let mut snapshot = entry(NativeCommandImplementation::Opaque);
        let public = &mut snapshot.commands[0];
        public.slot = tcl_core_types::NativeByteCommandSlot::new(
            tcl_core_types::ByteNamespacePath::root(),
            "info".into(),
        );
        public.compiler_hook = NativeCompilerHookPresence::Present;
        public.compiler = Some(NativeCommandCompiler {
            registry_identity: "info".to_owned(),
            ensemble: Some(NativeEnsembleCompiler {
                namespace_token: 0,
                map: vec![("exists".into(), vec![worker_bytes.map(Into::into)])],
                subcommands: None,
                prefixes: true,
                parameters: Vec::new(),
                unknown_handler: None,
            }),
        });
        let mut worker = public.clone();
        worker.slot = tcl_core_types::NativeByteCommandSlot::new(
            tcl_core_types::ByteNamespacePath::root(),
            "helper".into(),
        );
        worker.token = 5;
        worker.compiler_hook = NativeCompilerHookPresence::Absent;
        worker.compiler = None;
        snapshot.commands.push(worker);
        snapshot
    }

    fn analyse_mapped_ensemble(
        snapshot: &NativeCompilationEntry,
        version: tcl_dialect::TclVersion,
    ) -> SourceCommandBindings {
        analyse_mapped_ensemble_source(snapshot, version, "info exists missing")
    }

    fn analyse_mapped_ensemble_source(
        snapshot: &NativeCompilationEntry,
        version: tcl_dialect::TclVersion,
        source: &str,
    ) -> SourceCommandBindings {
        let mut snapshot = snapshot.clone();
        select_fixture_native_point(&mut snapshot, version);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(version)),
                native_entry: Some(&snapshot),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ScriptCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                ..SourceAnalysisOptions::default()
            },
        )
    }

    #[test]
    fn changed_ensemble_map_captures_custom_name_without_stock_semantics() {
        let snapshot = mapped_ensemble_entry(Some("::helper"));
        let analysis = analyse_mapped_ensemble(&snapshot, tcl_dialect::TclVersion::V8_6);
        assert!(!analysis.native_compilation_provider_required_at(0));
        let binding = analysis.invocation_at_source("info", 0);
        let plan = binding
            .named_invocation()
            .expect("actual compiler captures mapped name");
        assert_eq!(plan.captured_name(), "::helper");
        assert!(plan.lookup.is_none());
        assert_eq!(plan.arguments_from, 1);
        assert!(
            plan.proved_target()
                .is_some_and(|target| !target.registry_backed)
        );
        assert_eq!(
            plan.compiler_prerequisite.as_ref().unwrap().compiler,
            snapshot.commands[0].compiler.clone().unwrap()
        );
    }

    #[test]
    fn original_captured_named_admission_preserves_source_and_unknown_axes() {
        let source = "info exists missing";
        let snapshot = mapped_ensemble_entry(Some("::helper"));
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let analysis = analyse_mapped_ensemble_source(&snapshot, version, source);
            let segments = crate::segmenter::segment_commands(source);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                tcl_lexer::LexerConfig::default(),
                &segments[0],
            );
            analysis.stamp_original_tokens(&mut tokens);
            let binding = tokens.source_binding.as_ref().unwrap();
            // A mutable captured name is not a static registry selection.
            assert_eq!(
                binding.native_compilation_admission_selection(),
                tcl_registry::native_compilation::NativeCompilationSelection::Unknown
            );
            let original = binding.original_named_compiler_admission(&tokens).unwrap();
            assert_eq!(original.captured_name(), "::helper");
            assert!(original.lookup.is_none());
            assert!(binding.admitted_inline_invocation().is_none());

            let mut rewritten = tokens.clone();
            rewritten.argv_texts[1] = "commands".to_owned();
            assert!(
                binding
                    .original_named_compiler_admission(&rewritten)
                    .is_none()
            );
            let mut unknown = binding.clone();
            unknown.native_compilation_admission =
                Some(tcl_registry::native_compilation::NativeCompilationSelection::Unknown);
            assert!(unknown.original_named_compiler_admission(&tokens).is_none());
            let mut missing_context = binding.clone();
            missing_context.compiler_policy = None;
            assert!(
                missing_context
                    .original_named_compiler_admission(&tokens)
                    .is_none()
            );
            let mut missing_source = binding.clone();
            missing_source.original_compiler_words = None;
            assert!(
                missing_source
                    .original_named_compiler_admission(&tokens)
                    .is_none()
            );
        }
        let analysis =
            analyse_mapped_ensemble_source(&snapshot, tcl_dialect::TclVersion::V8_5, source);
        assert_eq!(
            analysis
                .invocation_at_source("info", 0)
                .native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Generic
        );
    }

    #[test]
    fn compile_service_emits_original_mutable_named_invocation_without_static_tag() {
        use tcl_runtime_api::CompileService;
        for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(name).analyser_profile();
            let version = tcl_registry::InvocationDialect::of_profile(profile)
                .tcl_version
                .unwrap();
            let mut snapshot = mapped_ensemble_entry(Some("::helper"));
            select_fixture_native_point(&mut snapshot, version);
            snapshot.profile = profile.cache_key();
            snapshot.invocation_policy = Some(profile.cache_key());
            let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
            let compiled = service
                .compile_script_with_entry(
                    tcl_runtime_api::ScriptCompileTarget {
                        source: "info exists missing",
                        namespace: "",
                    },
                    profile,
                    &snapshot,
                )
                .expect("authentic captured Named compiler admission");
            assert_eq!(
                compiled.top_level.native_compilation_preflight,
                tcl_runtime_api::NativeCompilationPreflight::NotRequired
            );
            assert!(!compiled.top_level.native_compiler_prerequisites.is_empty());
            let selections: Vec<_> = compiled
                .top_level
                .instructions
                .iter()
                .filter_map(|instruction| instruction.native_compiler_selection.as_ref())
                .collect();
            assert!(!selections.is_empty());
            assert!(
                selections
                    .iter()
                    .all(|selection| selection.prerequisite.guard()
                        == tcl_runtime_api::CommandBindingGuard::BeforeArguments)
            );
            assert!(
                compiled
                    .top_level
                    .native_compiler_prerequisites
                    .iter()
                    .all(|required| required.guard()
                        == tcl_runtime_api::CommandBindingGuard::ChunkEntry
                        && selections.iter().any(|selection| {
                            use tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite;
                            let (NativeCompilerSelectionPrerequisite::Ensemble(original)
                                | NativeCompilerSelectionPrerequisite::Command(original)) = &selection.prerequisite
                            else {
                                return false;
                            };
                            let mut entry_required = original.as_ref().clone();
                            entry_required.guard = tcl_runtime_api::CommandBindingGuard::ChunkEntry;
                            NativeCompilerSelectionPrerequisite::from_command_registration(
                                std::sync::Arc::new(entry_required),
                            ) == *required
                        }))
            );
        }
    }

    #[test]
    fn c85_no_hook_mapped_worker_keeps_generic_public_dispatch() {
        let snapshot = mapped_ensemble_entry(Some("::helper"));
        let analysis = analyse_mapped_ensemble(&snapshot, tcl_dialect::TclVersion::V8_5);
        assert!(!analysis.native_compilation_provider_required_at(0));
        let binding = analysis.invocation_at_source("info", 0);
        assert!(binding.named_invocation().is_none());
        assert!(binding.may_use_live_dispatch);
        assert!(
            binding
                .proved_execution_target()
                .is_some_and(|target| !target.registry_backed)
        );
    }

    #[test]
    fn relative_mapped_worker_follows_closed_namespace_misses_to_root() {
        let mut snapshot = mapped_ensemble_entry(Some("helper"));
        let mut namespace = snapshot.namespaces[0].clone();
        namespace.path = tcl_core_types::ByteNamespacePath::from_segments(["tcl", "info"]);
        namespace.token = 8;
        snapshot.namespaces.push(namespace);
        snapshot.commands[0]
            .compiler
            .as_mut()
            .unwrap()
            .ensemble
            .as_mut()
            .unwrap()
            .namespace_token = 8;
        for version in [tcl_dialect::TclVersion::V8_5, tcl_dialect::TclVersion::V8_6] {
            let analysis = analyse_mapped_ensemble(&snapshot, version);
            assert!(
                !analysis.native_compilation_provider_required_at(0),
                "{version:?}"
            );
            let binding = analysis.invocation_at_source("info", 0);
            if version == tcl_dialect::TclVersion::V8_6 {
                assert_eq!(
                    binding.named_invocation().unwrap().captured_name(),
                    "::helper"
                );
            } else {
                assert!(binding.named_invocation().is_none());
            }
        }
    }

    #[test]
    fn unmaterialised_ensemble_worker_keeps_compiler_obligation() {
        let snapshot = mapped_ensemble_entry(None);
        let analysis = analyse_mapped_ensemble(&snapshot, tcl_dialect::TclVersion::V8_6);
        assert!(analysis.native_compilation_provider_required_at(0));
        assert!(
            analysis
                .invocation_at_source("info", 0)
                .named_invocation()
                .is_none()
        );
    }

    #[test]
    fn original_registered_worker_closes_delegated_compiler_traversal() {
        use tcl_runtime_api::native_compilation::{
            NativeCommandCompiler, NativeCompilerHookPresence,
        };
        let mut snapshot = mapped_ensemble_entry(Some("::tcl::info::exists"));
        let mut namespace = snapshot.namespaces[0].clone();
        namespace.path = tcl_core_types::ByteNamespacePath::from_segments(["tcl", "info"]);
        namespace.token = 8;
        snapshot.namespaces.push(namespace);
        let worker = &mut snapshot.commands[1];
        worker.slot = tcl_core_types::NativeByteCommandSlot::new(
            tcl_core_types::ByteNamespacePath::from_segments(["tcl", "info"]),
            "exists".into(),
        );
        worker.namespace_token = 8;
        worker.implementation = NativeCommandImplementation::Registry {
            identity: "tcl::info::exists".into(),
            compiler_hook: true,
        };
        worker.compiler_hook = NativeCompilerHookPresence::Present;
        worker.compiler = Some(NativeCommandCompiler {
            registry_identity: "tcl::info::exists".into(),
            ensemble: None,
        });
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let analysis = analyse_mapped_ensemble(&snapshot, version);
            assert!(
                !analysis.native_compilation_provider_required_at(0),
                "{version:?}: original private compiler and map are retained"
            );
            let mut unavailable = snapshot.clone();
            unavailable.commands[1].implementation = NativeCommandImplementation::Opaque;
            unavailable.commands[1].compiler_hook = NativeCompilerHookPresence::Unknown;
            unavailable.commands[1].compiler = None;
            assert!(
                analyse_mapped_ensemble(&unavailable, version)
                    .native_compilation_provider_required_at(0),
                "{version:?}: a public descriptor cannot supply the private compiler"
            );
        }
    }
    #[test]
    fn file_named_worker_requires_original_map_and_independent_registration() {
        use tcl_runtime_api::native_compilation::{
            NativeCommandCompiler, NativeCompilerHookPresence,
        };
        for member in ["exists", "dirname", "join"] {
            let private = format!("::tcl::file::{member}");
            let mut snapshot = mapped_ensemble_entry(Some(&private));
            let mut namespace = snapshot.namespaces[0].clone();
            namespace.path = tcl_core_types::ByteNamespacePath::from_segments(["tcl", "file"]);
            namespace.token = 8;
            snapshot.namespaces.push(namespace);
            let public = &mut snapshot.commands[0];
            public.slot.simple = "file".into();
            public.implementation = NativeCommandImplementation::Registry {
                identity: "file".into(),
                compiler_hook: true,
            };
            let compiler = public.compiler.as_mut().unwrap();
            compiler.registry_identity = "file".into();
            let ensemble = compiler.ensemble.as_mut().unwrap();
            ensemble.namespace_token = 8;
            ensemble.map[0].0 = member.into();
            let worker = &mut snapshot.commands[1];
            worker.slot = tcl_core_types::NativeByteCommandSlot::new(
                tcl_core_types::ByteNamespacePath::from_segments(["tcl", "file"]),
                member.into(),
            );
            worker.namespace_token = 8;
            worker.implementation = NativeCommandImplementation::Registry {
                identity: private.clone(),
                compiler_hook: true,
            };
            worker.compiler_hook = NativeCompilerHookPresence::Present;
            worker.compiler = Some(NativeCommandCompiler {
                registry_identity: private.clone(),
                ensemble: None,
            });
            let source = format!("file {member} missing");
            for version in [
                tcl_dialect::TclVersion::V8_6,
                tcl_dialect::TclVersion::V9_0,
                tcl_dialect::TclVersion::V9_1,
            ] {
                let analysis = analyse_mapped_ensemble_source(&snapshot, version, &source);
                assert!(
                    !analysis.native_compilation_provider_required_at(0),
                    "{version:?}/{member}"
                );
                assert_eq!(
                    analysis
                        .invocation_at_source("file", 0)
                        .named_invocation()
                        .unwrap()
                        .captured_name(),
                    private
                );
                let mut unknown = snapshot.clone();
                unknown.commands[1].implementation = NativeCommandImplementation::Opaque;
                unknown.commands[1].compiler_hook = NativeCompilerHookPresence::Unknown;
                unknown.commands[1].compiler = None;
                assert!(
                    analyse_mapped_ensemble_source(&unknown, version, &source)
                        .native_compilation_provider_required_at(0),
                    "{version:?}/{member}: original public map cannot supply worker registration"
                );
            }
        }
    }

    #[test]
    fn actual_disabled_inline_compilation_keeps_generic_unknown_handler() {
        let mut snapshot = entry(NativeCommandImplementation::Opaque);
        snapshot.inline_compilation_disabled = true;
        let analysis = analyse(&snapshot);
        assert!(!analysis.native_compilation_provider_required_at(0));
        let binding = analysis.invocation_at_source("set", 0);
        assert!(binding.may_use_live_dispatch);
        assert_eq!(
            binding.compiled_candidates,
            [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
        );
        assert_eq!(
            binding.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Generic
        );
        assert!(
            binding
                .proved_execution_target()
                .is_some_and(|target| !target.registry_backed)
        );
    }
    #[test]
    fn mapped_alias_compiler_captures_the_wrapper_name_and_keeps_its_prefix() {
        let mut snapshot = mapped_ensemble_entry(Some("::helper"));
        let mut terminal = snapshot.commands[1].clone();
        terminal.slot = tcl_core_types::NativeByteCommandSlot::new(
            tcl_core_types::ByteNamespacePath::root(),
            "target".into(),
        );
        terminal.token = 6;
        snapshot.commands[1].implementation = NativeCommandImplementation::Alias {
            interpreter: snapshot.interpreter,
            target_lookup: tcl_runtime_api::native_compilation::NativeAliasTargetLookup::Global,
            words: vec!["::target".into(), "PREFIX".into()],
        };
        snapshot.commands.push(terminal);
        let analysis = analyse_mapped_ensemble(&snapshot, tcl_dialect::TclVersion::V8_6);
        assert!(!analysis.native_compilation_provider_required_at(0));
        let binding = analysis.invocation_at_source("info", 0);
        let named = binding
            .named_invocation()
            .expect("captures actual mapped alias wrapper");
        assert_eq!(named.captured_name(), "::helper");
        let target = named
            .proved_target()
            .expect("late alias target remains precise");
        assert_eq!(target.command, "::target");
        assert_eq!(
            target.prepended,
            [crate::registry_invocation::EffectiveInvocationWord::Literal("PREFIX".to_owned())]
        );
    }
}
