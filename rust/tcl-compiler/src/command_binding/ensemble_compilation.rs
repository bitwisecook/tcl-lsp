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

//! Actual compiler registration and map selection, independent of live semantics.

use super::{ModuleCommandBindings, SourceCommandTarget};
use tcl_registry::native_compilation::NativeCompilationWordShape;
use tcl_registry::native_ensemble::NativeEnsembleWorkerSelection;
use tcl_registry::native_selected_worker::{
    OriginalSelectedWorkerCompilation, OriginalSelectedWorkerInvocation,
};
use tcl_runtime_api::native_compilation::{
    NativeCompilerHookPresence, NativeEnsembleCompilerPrerequisite,
};

pub(super) enum ActualEnsemblePlan {
    Generic,
    Named {
        preparations: Vec<tcl_registry::native_control_compilation::NativeControlPreparationStep>,
        name: String,
        replacement_words: Vec<String>,
        prerequisite: Box<NativeEnsembleCompilerPrerequisite>,
        protocol: tcl_registry::native_compilation::NativeNamedInvocationProtocol,
        arguments_from: usize,
    },
    Operation {
        selection: tcl_registry::native_compilation::NativeCompilationSelection,
        recipe: Box<tcl_registry::native_instruction_plan::NativeInstructionPlan>,
        prerequisite: Box<NativeEnsembleCompilerPrerequisite>,
    },
    Unknown,
}

fn project_original_worker_plan(
    selected: tcl_registry::native_selected_worker::OriginalSelectedWorkerPathCompilation,
) -> Option<ActualEnsemblePlan> {
    let prerequisite = selected.prerequisite;
    Some(match selected.compilation {
        OriginalSelectedWorkerCompilation::PublicGeneric => ActualEnsemblePlan::Generic,
        OriginalSelectedWorkerCompilation::Unavailable => ActualEnsemblePlan::Unknown,
        OriginalSelectedWorkerCompilation::Named(recipe) => ActualEnsemblePlan::Named {
            preparations: recipe.preparations,
            name: String::from_utf8(recipe.name).ok()?,
            replacement_words: recipe.words.iter().filter_map(|word| match word {
                tcl_registry::native_instruction_plan::NativeNamedInvocationWord::Replacement(bytes) => Some(String::from_utf8(bytes.clone())),
                tcl_registry::native_instruction_plan::NativeNamedInvocationWord::Original(_) => None,
            }).collect::<Result<Vec<_>, _>>().ok()?,
            prerequisite: Box::new(prerequisite),
            protocol: recipe.protocol,
            arguments_from: recipe.arguments_from,
        },
        OriginalSelectedWorkerCompilation::Operation { selection, plan, .. } => ActualEnsemblePlan::Operation {
            selection,
            recipe: plan,
            prerequisite: Box::new(prerequisite),
        },
    })
}

impl ModuleCommandBindings {
    pub(super) fn original_ensemble_mapping_holds(
        &self,
        target: &SourceCommandTarget,
        lookup: &tcl_registry::native_compilation::NativeCompilerImplementationLookup,
    ) -> bool {
        if super::nqn(&target.command) == super::nqn(lookup.slot) {
            return true;
        }
        if super::nqn(&target.command) != super::nqn(lookup.ensemble) {
            return false;
        }
        if self.baseline.native_entry.is_none() {
            return true;
        }
        self.runtime_ensemble_configuration_for_identity(
            target.identity.as_ref(), target.implementation_generation,
            target.implementation_allocation.as_ref(),
        )
            .and_then(|row| row.compiler.as_ref()?.ensemble.as_ref())
            .is_some_and(|configuration| {
                configuration.parameters.is_empty()
                    && configuration.unknown_handler.is_none()
                    && configuration.subcommands.as_ref().is_none_or(|members| {
                        members.iter().any(|member| member.as_bytes() == lookup.member.as_bytes())
                    })
                    && configuration.map.iter().any(|(member, prefix)| {
                        member.as_bytes() == lookup.member.as_bytes()
                            && matches!(prefix.as_slice(), [Some(worker)] if worker.try_utf8().is_ok_and(|worker| super::nqn(worker) == super::nqn(lookup.slot)))
                    })
            })
    }

    pub(super) fn runtime_ensemble_compiler_for_identity(
        &self,
        identity: Option<&super::CommandIdentity>,
        generation: u32,
        allocation: Option<&super::CommandAllocation>,
    ) -> Option<&tcl_runtime_api::native_compilation::NativeCompilationBinding> {
        self.runtime_ensemble_configuration_for_identity(identity, generation, allocation)
            .filter(|row| {
                !row.has_execution_trace && row.compiler_hook == NativeCompilerHookPresence::Present
            })
    }

    fn runtime_ensemble_configuration_for_identity(
        &self,
        identity: Option<&super::CommandIdentity>,
        generation: u32,
        allocation: Option<&super::CommandAllocation>,
    ) -> Option<&tcl_runtime_api::native_compilation::NativeCompilationBinding> {
        if generation != 0 || allocation.is_some() {
            return None;
        }
        let runtime = identity?.runtime?;
        let entry = self.baseline.native_entry.as_ref()?;
        entry.commands.iter().find(|binding| {
            runtime.interpreter == entry.interpreter
                && binding.token == runtime.token
                && binding
                    .compiler
                    .as_ref()
                    .is_some_and(|compiler| compiler.ensemble.is_some())
        })
    }

    pub(super) fn actual_ensemble_plan(
        &self,
        target: &SourceCommandTarget,
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
        original: Option<(
            crate::registry_invocation::OriginalNativeCompilerInvocation<'_>,
            &tcl_registry::CommandRegistry,
        )>,
    ) -> Option<ActualEnsemblePlan> {
        let public = self.runtime_ensemble_compiler_for_identity(
            self.compiler_identity_at_lookup(head, namespace)
                .or(target.identity.as_ref()),
            target.implementation_generation,
            target.implementation_allocation.as_ref(),
        )?;
        let compiler = public.compiler.as_ref()?;
        let configuration = compiler.ensemble.as_ref()?;
        let entry = self.baseline.native_entry.as_ref()?;
        let (original, registry) = original?;
        let Some(native) = crate::registry_invocation::original_native_compiler_words(
            original.image,
            original.words,
            original.offset,
            original.config,
        ) else {
            return Some(ActualEnsemblePlan::Unknown);
        };
        let Some(protocol) = original.source_protocol else {
            return Some(ActualEnsemblePlan::Unknown);
        };
        let Ok(captured) =
            tcl_registry::native_compiler_words::NativeCompilerWords::capture(&native, protocol)
        else {
            return Some(ActualEnsemblePlan::Unknown);
        };
        let Some(dialect) = self.baseline.compilation_dialect() else {
            return Some(ActualEnsemblePlan::Unknown);
        };
        let selector =
            tcl_registry::native_ensemble::original_ensemble_selector(&captured, dialect).ok()?;
        let selected = tcl_registry::native_ensemble::select_worker_in_entry(
            entry,
            public.token,
            configuration,
            selector.as_ref().and_then(|word| word.literal.as_deref()),
            selector.as_ref().map(|word| word.shape),
            self.baseline.compilation_dialect(),
        );
        let (replacement, worker) = match selected {
            NativeEnsembleWorkerSelection::Generic => return Some(ActualEnsemblePlan::Generic),
            NativeEnsembleWorkerSelection::Unknown => return Some(ActualEnsemblePlan::Unknown),
            NativeEnsembleWorkerSelection::Worker { member, binding } => (member, binding),
        };
        if !self.original_compiler_worker_node_holds(&worker) {
            return Some(ActualEnsemblePlan::Unknown);
        }
        let Ok(replacement) = replacement.try_utf8() else {
            return Some(ActualEnsemblePlan::Unknown);
        };
        let Some(prerequisite) =
            self.runtime_ensemble_prerequisite(public, &worker, head, namespace)
        else {
            return Some(ActualEnsemblePlan::Unknown);
        };
        let replacements = [replacement.as_bytes().to_vec()];
        let selected = tcl_registry::native_selected_worker::compile_original_selected_worker_path(
            registry,
            &worker,
            OriginalSelectedWorkerInvocation {
                words: &captured,
                operand_from: 2,
                replacements: &replacements,
                dialect,
                context: original.context,
            },
            prerequisite,
            |ensemble, configuration, member, shape| {
                let selected = tcl_registry::native_ensemble::select_worker_in_entry(
                    entry,
                    ensemble.token,
                    configuration,
                    member,
                    shape,
                    Some(dialect),
                );
                match &selected {
                    NativeEnsembleWorkerSelection::Worker { binding, .. }
                        if !self.original_compiler_worker_node_holds(binding) =>
                    {
                        NativeEnsembleWorkerSelection::Unknown
                    }
                    _ => selected,
                }
            },
        );
        project_original_worker_plan(selected)
    }

    /// Original selected node for an independently admitted named compiler.
    /// This supplies no grammar, operand layout or dispatch permission.
    pub(super) fn original_named_worker_prerequisite(
        &self,
        target: &SourceCommandTarget,
        arguments: &[tcl_registry::InvocationWord<'_>],
        shapes: &[NativeCompilationWordShape],
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> Option<NativeEnsembleCompilerPrerequisite> {
        let public = self.runtime_ensemble_compiler_for_identity(
            self.compiler_identity_at_lookup(head, namespace)
                .or(target.identity.as_ref()),
            target.implementation_generation,
            target.implementation_allocation.as_ref(),
        )?;
        let configuration = public.compiler.as_ref()?.ensemble.as_ref()?;
        let entry = self.baseline.native_entry.as_ref()?;
        let selected = tcl_registry::native_ensemble::select_worker_in_entry(
            entry,
            public.token,
            configuration,
            arguments
                .first()
                .and_then(|word| word.literal())
                .map(str::as_bytes),
            shapes.first().copied(),
            self.baseline.compilation_dialect(),
        );
        let NativeEnsembleWorkerSelection::Worker { binding, .. } = selected else {
            return None;
        };
        self.original_compiler_worker_node_holds(&binding)
            .then(|| self.runtime_ensemble_prerequisite(public, &binding, head, namespace))
            .flatten()
    }

    fn original_compiler_worker_node_holds(
        &self,
        worker: &tcl_runtime_api::native_compilation::NativeCompilationBinding,
    ) -> bool {
        if self.has_opaque_domain() {
            return false;
        }
        let Some(entry) = self.baseline.native_entry.as_ref() else {
            return false;
        };
        let Some(slot) = super::runtime_entry::native_command_key(entry, worker) else {
            return false;
        };
        let bindings = self.binding_alternatives(&slot);
        let mut targets = bindings.iter();
        let Some(super::MayBinding::Target(target)) = targets.next() else {
            return false;
        };
        targets.next().is_none()
            && target.implementation_generation == 0
            && target.implementation_allocation.is_none()
            && target
                .token
                .as_ref()
                .and_then(|token| token.runtime)
                .is_some_and(|runtime| {
                    runtime.interpreter == entry.interpreter && runtime.token == worker.token
                })
    }

    fn runtime_ensemble_prerequisite(
        &self,
        public: &tcl_runtime_api::native_compilation::NativeCompilationBinding,
        worker: &tcl_runtime_api::native_compilation::NativeCompilationBinding,
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> Option<NativeEnsembleCompilerPrerequisite> {
        let entry = self.baseline.native_entry.as_ref()?;
        let key = if head.starts_with("::") {
            self.native_root_namespace_key()?
        } else {
            super::NamespaceKeyQuery::namespace_key(namespace).into_owned()
        };
        let context = key.native_context()?;
        if context.interpreter != entry.interpreter {
            return None;
        }
        let lookup_namespace = entry.namespace_context(context.token).ok()?;
        if lookup_namespace.path != context.path {
            return None;
        }
        Some(NativeEnsembleCompilerPrerequisite {
            interpreter: entry.interpreter,
            lookup_namespace_token: lookup_namespace.token,
            invocation_word: head.into(),
            slot: public.slot.clone(),
            namespace_token: public.namespace_token,
            token: public.token,
            implementation_generation: public.implementation_generation,
            compiler: public.compiler.clone()?,
            selected_worker: Some(worker.clone()),
            nested_compilers: Vec::new(),
            guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
        })
    }
}
