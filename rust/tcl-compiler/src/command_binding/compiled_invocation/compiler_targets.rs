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

//! The compiler registrations a lookup reaches: native compiler targets, imports and snapshots.

use super::super::{
    Arc, BTreeSet, MayBinding, ModuleCommandBindings, SourceCommandTarget, SourceLookupSnapshot,
};
use tcl_names::command::{BindingKind, CommandAllocationSite};

/// Immutable command table at a native compilation boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompilationSnapshot {
    pub(in super::super) table: Arc<SourceLookupSnapshot>,
    pub(in super::super) generic_fallbacks: BTreeSet<CommandAllocationSite>,
    pub(in super::super) source_locals:
        Option<Arc<super::super::compiled_preflight::ordered_locals::SourceCompilerLocalInventory>>,
}

/// Compiler-hook-capable tokens found by the shared command lookup owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCompilerTargets {
    /// Original or imported native tokens; name aliases never forward compiler hooks.
    pub targets: Vec<SourceCommandTarget>,
    /// Lookup may instead reach a token without a native compiler hook.
    pub may_be_generic: bool,
    /// The table or lookup route contains an unbounded alternative.
    pub unknown: bool,
}

impl NativeCompilationSnapshot {
    pub(in super::super) fn compiler_targets_for_original_input(
        &self,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &tcl_names::namespace::SourceNamespaceKey,
    ) -> NativeCompilerTargets {
        self.table
            .state
            .native_compiler_targets_for_original_input(input, namespace)
    }

    /// Query the same immutable lookup paths used by ordinary command resolution.
    #[must_use]
    pub fn compiler_targets(&self, head: &str, namespace: &str) -> NativeCompilerTargets {
        self.compiler_targets_in_namespace(head, namespace)
    }

    pub(in super::super) fn compiler_targets_in_namespace(
        &self,
        head: &str,
        namespace: &(impl super::super::NamespaceKeyQuery + ?Sized),
    ) -> NativeCompilerTargets {
        self.table.state.native_compiler_targets(head, namespace)
    }
}

impl ModuleCommandBindings {
    /// Compiler-hook projection of the actual byte table cells. Callable
    /// target resolution cannot forward an alias's compiler registration.
    pub(in super::super) fn native_compiler_targets_for_original_input(
        &self,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &tcl_names::namespace::SourceNamespaceKey,
    ) -> NativeCompilerTargets {
        self.trace_original_compiler_world(input, namespace);
        if self.source_step_observed()
            || self
                .baseline
                .native_entry
                .as_ref()
                .is_some_and(|entry| entry.inline_compilation_disabled)
        {
            return NativeCompilerTargets {
                targets: Vec::new(),
                may_be_generic: true,
                unknown: false,
            };
        }
        let Some(selection) = self.original_targets_for_input(
            input,
            namespace,
            super::super::CommandTargetLookup::CompilerSlots,
        ) else {
            if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                eprintln!(
                    "NATIVE_ORIGINAL_COMPILER_LOOKUP head={:?} reason=target-unavailable namespace={namespace:?}",
                    input.bytes()
                );
            }
            return NativeCompilerTargets {
                targets: Vec::new(),
                may_be_generic: true,
                unknown: true,
            };
        };
        let Some(keys) = self.original_command_keys_for_input(namespace, input) else {
            if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                eprintln!(
                    "NATIVE_ORIGINAL_COMPILER_LOOKUP head={:?} reason=keys-unavailable namespace={namespace:?}",
                    input.bytes()
                );
            }
            return NativeCompilerTargets {
                targets: Vec::new(),
                may_be_generic: true,
                unknown: true,
            };
        };
        let mut eligible = BTreeSet::new();
        let mut generic = selection.may_be_absent;
        let mut unknown = selection.unknown;
        if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
            eprintln!(
                "NATIVE_ORIGINAL_COMPILER_LOOKUP head={:?} selected_unknown={} absent={} keys={}",
                input.bytes(),
                selection.unknown,
                selection.may_be_absent,
                keys.len(),
            );
        }
        for key in keys {
            let bindings = self.original_bindings_for_key(&key);
            self.trace_original_compiler_cell(&key, bindings.as_ref());
            let Some(bindings) = bindings else {
                unknown = true;
                continue;
            };
            for binding in &bindings {
                // A missing earlier cell falls through along the retained
                // lookup path; total absence is a separate selection axis.
                if matches!(binding, MayBinding::Missing) {
                    continue;
                }
                self.collect_native_compiler_tokens(
                    binding,
                    &mut eligible,
                    &mut generic,
                    &mut unknown,
                    &mut BTreeSet::new(),
                    None,
                );
            }
        }
        let targets = self.current_original_compiler_targets(eligible, &mut generic);
        NativeCompilerTargets {
            may_be_generic: generic || unknown || targets.is_empty(),
            targets,
            unknown,
        }
    }

    fn current_original_compiler_targets(
        &self,
        eligible: BTreeSet<super::super::ResolvedCommandTarget>,
        generic: &mut bool,
    ) -> Vec<SourceCommandTarget> {
        eligible
            .into_iter()
            .filter_map(|target| {
                if self.source_execution_observed(target.token.as_ref()) {
                    *generic = true;
                    return None;
                }
                Some(SourceCommandTarget {
                    runtime_implementation_generation: self
                        .runtime_implementation_generation(target.token.as_ref()),
                    command: target.command,
                    prepended: target.prepended,
                    original_prepended: None,
                    logical_original_prefix: None,
                    registry_backed: target.registry_backed,
                    kind: target.kind,
                    identity: target.token,
                    implementation_generation: target.implementation_generation,
                    implementation_allocation: target.implementation_allocation,
                })
            })
            .collect::<Vec<_>>()
    }

    fn trace_original_compiler_world(
        &self,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &tcl_names::namespace::SourceNamespaceKey,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
            let mut world = (*self.original_command_world).clone();
            let policy = world.select_policy(self);
            eprintln!(
                "NATIVE_ORIGINAL_COMPILER_WORLD head={:?} opaque={} policy={:?} input_policy={:?} scope={:?} root={:?} namespace_rows={:?}",
                input.bytes(),
                self.has_opaque_domain(),
                policy,
                input.policy(),
                policy.and_then(|policy| world.scope(namespace, policy)),
                self.source_root_namespace_key(),
                self.baseline.native_entry.as_ref().map(|entry| entry
                    .namespaces
                    .iter()
                    .map(|row| (
                        row.token,
                        row.path.is_root(),
                        row.jim_namespace_object.is_some()
                    ))
                    .collect::<Vec<_>>()),
            );
        }
    }

    fn trace_original_compiler_cell(
        &self,
        key: &tcl_names::command::SourceCommandKey,
        bindings: Option<&BTreeSet<MayBinding>>,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
            eprintln!(
                "NATIVE_ORIGINAL_COMPILER_CELL key={key:?} alternatives={:?}",
                bindings.as_ref().map(|alternatives| alternatives
                    .iter()
                    .map(|binding| {
                        let target = match binding {
                            MayBinding::Target(target) => Some(target),
                            _ => None,
                        };
                        (
                            match binding {
                                MayBinding::Target(_) => "target",
                                MayBinding::Imported(_) => "imported",
                                MayBinding::Missing => "missing",
                                MayBinding::Unknown => "unknown",
                            },
                            target.map(|target| target.command.as_str()),
                            target.map(|target| target.registry_backed),
                            target
                                .and_then(|target| target.token.as_ref())
                                .and_then(|token| token.runtime),
                            target.and_then(|target| self.installed_compiler_hook(target)),
                        )
                    })
                    .collect::<Vec<_>>()),
            );
        }
    }

    /// Freeze actual point knowledge; a catalogue alone never supplies an entry contract.
    #[must_use]
    pub fn native_compilation_snapshot(&self) -> NativeCompilationSnapshot {
        self.native_compilation_snapshot_in_realm(self.baseline.invocation_realm)
    }

    pub(in super::super) fn native_compilation_snapshot_in_realm(
        &self,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> NativeCompilationSnapshot {
        #[cfg(any(test, debug_assertions))]
        let _timing = super::super::root_owner_work::Timing::at(
            super::super::root_owner_work::Query::Snapshot,
        );
        NativeCompilationSnapshot {
            table: Arc::new(SourceLookupSnapshot::in_realm(self.clone(), realm)),
            generic_fallbacks: BTreeSet::new(),
            source_locals: None,
        }
    }

    pub(in super::super) fn native_compiler_targets(
        &self,
        head: &str,
        namespace: &(impl super::super::NamespaceKeyQuery + ?Sized),
    ) -> NativeCompilerTargets {
        if self.source_step_observed()
            || self
                .baseline
                .native_entry
                .as_ref()
                .is_some_and(|entry| entry.inline_compilation_disabled)
        {
            return NativeCompilerTargets {
                targets: Vec::new(),
                may_be_generic: true,
                unknown: false,
            };
        }
        let mut eligible = BTreeSet::new();
        let mut generic = false;
        // Execution observers widen the eventual dispatch, but a closed
        // compilation table still proves their compiler-hook suppression.
        let mut unknown = !self.source_lookup_is_closed(head, namespace);
        for slot in self.source_keys(head, namespace) {
            let bindings = self.binding_alternatives(&slot);
            for binding in &bindings {
                self.collect_native_compiler_tokens(
                    binding,
                    &mut eligible,
                    &mut generic,
                    &mut unknown,
                    &mut BTreeSet::new(),
                    None,
                );
            }
        }
        // Compiler targets are a separate projection. An imported token can
        // retain a compiler registration after its callable origin changes.
        let targets: Vec<_> = eligible
            .into_iter()
            .map(|target| SourceCommandTarget {
                runtime_implementation_generation: self
                    .runtime_implementation_generation(target.token.as_ref()),
                command: target.command,
                prepended: target.prepended,
                original_prepended: None,
                logical_original_prefix: None,
                registry_backed: target.registry_backed,
                kind: target.kind,
                identity: target.token,
                implementation_generation: target.implementation_generation,
                implementation_allocation: target.implementation_allocation,
            })
            .collect();
        let targets = targets
            .into_iter()
            .filter(|target| {
                if self.source_execution_observed(
                    self.compiler_identity_at_lookup(head, namespace)
                        .or(target.identity.as_ref()),
                ) {
                    generic = true;
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();
        NativeCompilerTargets {
            may_be_generic: generic || unknown || targets.is_empty(),
            targets,
            unknown,
        }
    }

    fn collect_native_compiler_tokens(
        &self,
        binding: &MayBinding,
        eligible: &mut BTreeSet<super::super::ResolvedCommandTarget>,
        generic: &mut bool,
        unknown: &mut bool,
        visiting: &mut BTreeSet<tcl_names::command::CommandIdentity>,
        raw_import: Option<&tcl_names::command::CommandIdentity>,
    ) {
        if let (MayBinding::Target(target), Some(raw)) = (binding, raw_import) {
            self.collect_original_imported_compiler(target, raw, eligible, unknown);
            return;
        }
        if let MayBinding::Target(target) = binding
            && target.terminal
            && target.implementation_generation == 0
            && target.implementation_allocation.is_none()
            && let Some(raw) = target.token.as_ref()
            && let Some(row) = self.runtime_compiler_row(raw)
            && row.compiler.is_some()
        {
            match self.runtime_compiler_hook(Some(raw)) {
                Some(true) => self.collect_native_compiler_tokens(
                    binding,
                    eligible,
                    generic,
                    unknown,
                    visiting,
                    Some(raw),
                ),
                Some(false) => *generic = true,
                None => *unknown = true,
            }
            return;
        }
        if let MayBinding::Target(target) = binding
            && target.kind == BindingKind::Alias
            && !target.terminal
            && target.implementation_generation == 0
            && target.implementation_allocation.is_none()
            && let Some(identity) = target.token.as_ref()
            && let Some(row) = self.runtime_compiler_row(identity)
            && matches!(row.implementation,
                tcl_runtime_api::native_compilation::NativeCommandImplementation::Alias { .. }
                    | tcl_runtime_api::native_compilation::NativeCommandImplementation::PartialAlias { .. })
        {
            match self.runtime_compiler_hook(Some(identity)) {
                Some(false) => *generic = true,
                Some(true) | None => *unknown = true,
            }
            return;
        }
        match binding {
            MayBinding::Target(target)
                if self
                    .runtime_noop_header(
                        target.token.as_ref(),
                        target.implementation_generation,
                        target.implementation_allocation.as_ref(),
                    )
                    .is_some() =>
            {
                eligible.insert(target.clone());
            }
            MayBinding::Target(target)
                if self
                    .runtime_ensemble_compiler_for_identity(
                        target.token.as_ref(),
                        target.implementation_generation,
                        target.implementation_allocation.as_ref(),
                    )
                    .is_some() =>
            {
                eligible.insert(target.clone());
            }
            MayBinding::Target(target)
                if target.terminal && self.installed_compiler_hook(target) == Some(false) =>
            {
                *generic = true;
            }
            MayBinding::Target(target)
                if target.terminal
                    && target.kind == BindingKind::Builtin
                    && target.registry_backed =>
            {
                eligible.insert(target.clone());
            }
            MayBinding::Target(target)
                if target.terminal
                    && target
                        .token
                        .as_ref()
                        .is_some_and(|token| token.runtime.is_some()) =>
            {
                if self.runtime_compiler_hook(target.token.as_ref()) == Some(false) {
                    *generic = true;
                } else {
                    // An opaque runtime implementation proves no semantic
                    // descriptor. Only its independent actual no-hook axis
                    // closes compilation as a generic invocation.
                    *unknown = true;
                }
            }
            MayBinding::Target(_) | MayBinding::Missing => *generic = true,
            MayBinding::Unknown => *unknown = true,
            MayBinding::Imported(import) => self.collect_native_compiler_import(
                import, eligible, generic, unknown, visiting, raw_import,
            ),
        }
    }

    fn collect_original_imported_compiler(
        &self,
        target: &super::super::ResolvedCommandTarget,
        raw: &tcl_names::command::CommandIdentity,
        eligible: &mut BTreeSet<super::super::ResolvedCommandTarget>,
        unknown: &mut bool,
    ) {
        let Some(row) = self.runtime_compiler_row(raw) else {
            *unknown = true;
            return;
        };
        let mut compiler = target.clone();
        compiler.token = Some(raw.clone());
        compiler.prepended.clear();
        compiler.implementation_generation = 0;
        compiler.implementation_allocation = None;
        compiler.terminal = true;
        if let Some(registration) = &row.compiler {
            compiler.command.clone_from(&registration.registry_identity);
            compiler.registry_backed = true;
            compiler.kind = BindingKind::Builtin;
            eligible.insert(compiler);
        } else if self.runtime_noop_header(Some(raw), 0, None).is_some() {
            // This header has no registry handler identity to donate.
            compiler.registry_backed = false;
            compiler.kind = BindingKind::Command;
            eligible.insert(compiler);
        } else {
            *unknown = true;
        }
    }
    fn collect_native_compiler_import(
        &self,
        import: &super::super::ImportedCommandBinding,
        eligible: &mut BTreeSet<super::super::ResolvedCommandTarget>,
        generic: &mut bool,
        unknown: &mut bool,
        visiting: &mut BTreeSet<tcl_names::command::CommandIdentity>,
        raw_import: Option<&tcl_names::command::CommandIdentity>,
    ) {
        let raw = raw_import.or(import.compiler.as_ref());
        if let Some(raw) = raw {
            match self.runtime_compiler_hook(Some(raw)) {
                Some(false) => {
                    *generic = true;
                    return;
                }
                None => {
                    *unknown = true;
                    return;
                }
                Some(true) => {}
            }
        } else if import.origin.runtime.is_some() {
            // A new import has no actual raw-token row in this closed
            // runtime entry. Its origin cannot donate that missing axis.
            *unknown = true;
            return;
        }
        if let Some(raw) = raw
            && let Some(row) = self.runtime_compiler_row(raw)
            && (row.compiler.is_some() || self.runtime_noop_header(Some(raw), 0, None).is_some())
        {
            let target = super::super::ResolvedCommandTarget {
                command: raw.origin.clone(),
                prepended: Vec::new(),
                registry_backed: false,
                kind: BindingKind::Command,
                implementation_generation: 0,
                implementation_allocation: None,
                terminal: true,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: Some(raw.clone()),
            };
            self.collect_native_compiler_tokens(
                &MayBinding::Target(target),
                eligible,
                generic,
                unknown,
                visiting,
                Some(raw),
            );
            return;
        }
        let token = &import.origin;
        if !visiting.insert(token.clone()) {
            *unknown = true;
            return;
        }
        if let Some(bindings) = self.objects.get(token) {
            for binding in bindings {
                self.collect_native_compiler_tokens(
                    binding, eligible, generic, unknown, visiting, raw,
                );
            }
        } else {
            *unknown = true;
        }
        visiting.remove(token);
    }
}
