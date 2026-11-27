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

//! Invalidation: the compilation epochs and lookup stamps, the native compiler policy stamp, and the mutations that outdate the native compiler cache.

use super::super::commands::CommandSidecarKey;
use super::super::{
    ActivationFreshnessWorld, BindingStamp, CommandLookupStamp, CommandSlot, NamespacePath,
    NativeCompilerPolicyInputs, Vm,
};
use super::CommandSemantics;
use crate::command::{Command, EnsembleDef};
use crate::compiled::{NativeCompilerCacheEpoch, NativeCompilerCacheStamp};
#[cfg(test)]
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_runtime_api::guard::GuardDomain;
use tcl_runtime_api::{NsId, ROOT_NS};

impl Vm {
    /// End the engine-owned command-registration phase. Every subsequent
    /// command-table mutation participates in compiled-command invalidation.
    pub(in super::super) fn enable_command_semantics_tracking(&self) {
        debug_assert_eq!(
            self.compilation_epochs.command_semantics.get(),
            CommandSemantics::Bootstrapping
        );
        self.compilation_epochs
            .command_semantics
            .set(CommandSemantics::Tracking);
    }

    /// Whether new dynamic units must retain ordinary command dispatch.
    pub(in super::super) fn requires_plain_command_dispatch(&self) -> bool {
        self.step_trace_active()
    }

    /// Advance the command-binding generation after a live table mutation.
    /// Artifacts carry the exact bindings they assume, so a later entry can
    /// remain optimised when those identities still match (including after a
    /// rename is reversed) and deoptimise only when they do not.
    pub(in super::super) fn invalidate_compiled_command_semantics_for_key(&mut self, _key: &str) {
        self.invalidate_compiled_command_semantics();
    }

    /// Advance the compilation generation after a non-table mutation changes
    /// command resolution (for example, `namespace path`).
    pub(in super::super) fn invalidate_compiled_command_semantics(&self) {
        if self.compilation_epochs.command_semantics.get() != CommandSemantics::Tracking {
            return;
        }
        self.bump_trace_deopt_epoch();
    }

    /// Bump the shared compilation-deopt epoch.
    pub(crate) fn command_lookup_stamp(&self) -> CommandLookupStamp {
        let epochs = &self.compilation_epochs;
        CommandLookupStamp {
            binding_epoch: epochs.binding_epoch.get(),
            cmd_epoch: epochs.cmd_epoch.get(),
            trace_deopt_epoch: epochs.trace_deopt_epoch.get(),
            name_world: self.name_world.generation(),
            interpreter: self.cur.0,
            profile: std::ptr::from_ref(self.actual_native_execution_profile()).addr(),
        }
    }

    pub(super) fn binding_stamp(&self) -> BindingStamp {
        let epochs = &self.compilation_epochs;
        BindingStamp {
            binding_epoch: epochs.binding_epoch.get(),
            cmd_epoch: epochs.cmd_epoch.get(),
            trace_deopt_epoch: epochs.trace_deopt_epoch.get(),
            interpreter: self.cur.0,
            profile_generation: self.profile_generation,
            compiler_generation: self.compiler_generation,
        }
    }

    pub(crate) fn bump_trace_deopt_epoch(&self) {
        self.advance_binding_epoch();
        self.compilation_epochs.trace_deopt_epoch.set(
            self.compilation_epochs
                .trace_deopt_epoch
                .get()
                .wrapping_add(1),
        );
    }

    /// Invalidate every offline guard that depends on `domain`.
    ///
    /// Keep this at the mutation owner: guard epochs are useful only when no
    /// registration path can bypass the bump.
    pub(in super::super) fn invalidate_guard_domain(&self, domain: GuardDomain) {
        if matches!(
            domain,
            GuardDomain::CommandTrace | GuardDomain::VariableTrace | GuardDomain::Interpreter
        ) {
            self.native_compiler_pass_owner
                .invalidate_ordinary_entry_observation();
        }
        self.guards.borrow_mut().invalidate(domain);
    }

    /// The current shared compilation-deopt epoch.
    pub(crate) fn trace_deopt_epoch(&self) -> u64 {
        self.compilation_epochs.trace_deopt_epoch.get()
    }

    pub(crate) fn native_compiler_policy(&self) -> crate::compiled::NativeCompilerPolicy {
        self.stamped_native_compiler_policy().0
    }

    /// [`Self::native_compiler_policy`] with a stamp that is equal between two
    /// calls only when they return equal policies.
    pub(crate) fn stamped_native_compiler_policy(
        &self,
    ) -> (crate::compiled::NativeCompilerPolicy, u64) {
        let stamp = self.native_compiler_policy_stamp();
        let memo = self.compiler_policy_memo.borrow();
        let (_, policy, _) = memo.as_ref().expect("stamped compiler policy");
        (*policy, stamp)
    }

    /// The stamp of the current [`Self::native_compiler_policy`], projecting it
    /// again only when one of its inputs has changed.
    pub(crate) fn activation_freshness_world(&self) -> ActivationFreshnessWorld {
        let address =
            |profile: &'static tcl_dialect::DialectProfile| std::ptr::from_ref(profile).addr();
        ActivationFreshnessWorld {
            profile: self.profile_generation,
            compiler: self.compiler_generation,
            policy: NativeCompilerPolicyInputs {
                pinned: address(self.pin.profile),
                active: self.active_native_profile.map(address),
                engine: self.actual_engine_profile.map(address),
                release: self.runtime_version,
                providers: self.logical_providers,
            },
            name_world: self.name_world_generation(),
            namespace: self.current_namespace_execution_context(),
            execution_profile: address(self.actual_native_execution_profile()),
        }
    }

    pub(crate) fn native_compiler_policy_stamp(&self) -> u64 {
        static NEXT_STAMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let address =
            |profile: &'static tcl_dialect::DialectProfile| std::ptr::from_ref(profile).addr();
        let inputs = NativeCompilerPolicyInputs {
            pinned: address(self.pin.profile),
            active: self.active_native_profile.map(address),
            engine: self.actual_engine_profile.map(address),
            release: self.runtime_version,
            providers: self.logical_providers,
        };
        if let Some((memo, _, stamp)) = self.compiler_policy_memo.borrow().as_ref()
            && *memo == inputs
        {
            return *stamp;
        }
        let policy = self.project_native_compiler_policy();
        let mut memo = self.compiler_policy_memo.borrow_mut();
        let stamp = match memo.as_ref() {
            Some((_, old, stamp)) if *old == policy => *stamp,
            _ => NEXT_STAMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        };
        *memo = Some((inputs, policy, stamp));
        stamp
    }

    /// Whether `token` currently names exactly the constructed `path`.
    pub(crate) fn namespace_token_has_path(&self, token: NsId, path: &NamespacePath) -> bool {
        self.name_world
            .borrow()
            .ns_arena
            .get(token.0 as usize)
            .is_some_and(|current| current == path)
    }

    fn project_native_compiler_policy(&self) -> crate::compiled::NativeCompilerPolicy {
        crate::compiled::NativeCompilerPolicy {
            expression_evaluation: self.expression_evaluation_policy(),
            expression_provider: self.logical_providers.expression_parse,
            source_word_provider: self.logical_providers.source_words,
            eval_object_provider: self.logical_providers.eval_object,
            name_provider: self.logical_providers.names,
            compiled_variable_provider: self.logical_providers.compiled_variables,
            engine: crate::compiled::NativeProfileReceipt::of(
                self.actual_native_execution_profile(),
            ),
            invocation: crate::compiled::NativeProfileReceipt::of(self.native_execution_profile()),
            quote_provider: self.logical_providers.quote,
            numeric_provider: self.logical_providers.numeric,
        }
    }

    pub(in super::super) fn native_cache_stamp(&self, namespace: NsId) -> NativeCompilerCacheStamp {
        NativeCompilerCacheStamp {
            policy: self.native_compiler_policy(),
            interpreter_epoch: self.compilation_epochs.native_compiler_epoch.get(),
            namespace,
            resolver_epoch: self
                .compilation_epochs
                .native_namespace_epochs
                .get(&namespace)
                .copied()
                .unwrap_or(0),
        }
    }

    pub(crate) fn source_namespace_path(&self) -> NamespacePath {
        self.ns_path(self.current_ns_id())
    }

    pub(crate) fn native_cache_stamp_for_source_namespace(
        &self,
        path: &NamespacePath,
    ) -> Option<NativeCompilerCacheStamp> {
        let token = self
            .current_namespace_execution_context()
            .resolved_token()
            .filter(|current| path == &self.ns_path(*current))
            .or_else(|| self.namespace_token_at_path(path, true))?;
        Some(self.native_cache_stamp(token))
    }

    pub(in super::super) fn note_native_namespace_cache_mutation(
        &mut self,
        namespace: NsId,
        mutation: tcl_registry::native_procedure::NativeCompilerCacheMutation,
    ) {
        self.advance_binding_epoch();
        if self.compilation_epochs.command_semantics.get() != CommandSemantics::Tracking
            || tcl_registry::native_procedure::native_compiler_cache_invalidated(
                self.native_invocation_dialect(),
                mutation,
            ) == Some(false)
        {
            return;
        }
        let epoch = self
            .compilation_epochs
            .native_namespace_epochs
            .entry(namespace)
            .or_default();
        *epoch = epoch
            .checked_add(1)
            .expect("namespace compiler epoch exhausted");
    }

    pub(in super::super) fn note_native_shadowed_compiler_commands(&mut self, key: &str) {
        let Some(slot) = self.command_slot(key) else {
            return;
        };
        let mut namespace = slot.namespace;
        let mut trail = Vec::<NameBytes>::new();
        while namespace != ROOT_NS {
            let mut global_counterpart = ROOT_NS;
            let mut found = true;
            for segment in trail.iter().rev() {
                let Some(child) = self
                    .name_world
                    .borrow()
                    .ns_children
                    .get(&(global_counterpart, segment.clone()))
                    .copied()
                else {
                    found = false;
                    break;
                };
                global_counterpart = child;
            }
            if found {
                let counterpart = CommandSlot {
                    namespace: global_counterpart,
                    simple: slot.simple.clone(),
                };
                if let Some(shadowed) = self.command_storage_key_at_slot(&counterpart)
                    && self.visible_command_at_key(&shadowed).is_some()
                {
                    self.note_native_command_reference_changed(namespace);
                    self.note_native_command_path_dependents(namespace);
                    let hook = self.native_hook_at_sidecar(&CommandSidecarKey::visible(shadowed));
                    self.note_native_namespace_cache_mutation(namespace,
                        tcl_registry::native_procedure::NativeCompilerCacheMutation::NamespaceCommandShadow { hook });
                }
            }
            let Some(parent) = self
                .name_world
                .borrow()
                .ns_parents
                .get(namespace.0 as usize)
                .copied()
                .flatten()
            else {
                break;
            };
            if let Some(segment) = self
                .name_world
                .borrow()
                .ns_arena
                .get(namespace.0 as usize)
                .and_then(|path| path.last())
                .cloned()
            {
                trail.push(segment);
            }
            namespace = parent;
        }
    }

    pub(in super::super) fn note_native_compiler_cache_mutation(
        &self,
        mutation: tcl_registry::native_procedure::NativeCompilerCacheMutation,
    ) {
        self.advance_binding_epoch();
        if self.compilation_epochs.command_semantics.get() != CommandSemantics::Tracking {
            return;
        }
        if tcl_registry::native_procedure::native_compiler_cache_invalidated(
            self.native_invocation_dialect(),
            mutation,
        ) != Some(false)
        {
            let current = self.compilation_epochs.native_compiler_epoch.get();
            self.compilation_epochs
                .native_compiler_epoch
                .set(NativeCompilerCacheEpoch(
                    current
                        .0
                        .checked_add(1)
                        .expect("native compiler epoch exhausted"),
                ));
        }
    }

    pub(in super::super) fn native_hook_at_sidecar(
        &self,
        key: &CommandSidecarKey,
    ) -> tcl_runtime_api::native_compilation::NativeCompilerHookPresence {
        use tcl_runtime_api::native_compilation::{
            NativeCommandImplementation as Implementation, NativeCompilerHookPresence as Hook,
        };
        if let Some(import) = self.import_binding_at_sidecar_key(key) {
            return import.compiler_hook;
        }
        {
            let Some(command) = self.command_at_sidecar_key(key) else {
                return Hook::Unknown;
            };
            let implementation = match (key, &command) {
                (CommandSidecarKey::Visible(name), Command::Builtin(_)) => {
                    self.native_compilation_builtin(name)
                }
                (CommandSidecarKey::Hidden(name), Command::Builtin(_)) => {
                    self.hidden.builtin_identities.get(name).map_or(
                        Implementation::Opaque,
                        |identity| Implementation::Registry {
                            identity: identity.clone(),
                            compiler_hook: true,
                        },
                    )
                }
                (_, Command::Ensemble(token)) => token.config().native_implementation().map_or(
                    Implementation::Opaque,
                    |native| Implementation::Registry {
                        identity: native.identity.to_owned(),
                        compiler_hook: true,
                    },
                ),
                _ => Implementation::Opaque,
            };
            self.native_hook_for_implementation(&implementation, &command)
        }
    }

    pub(in super::super) fn note_native_command_token_changed(&self, key: &CommandSidecarKey) {
        if self.command_at_sidecar_key(key).is_none() {
            return;
        }
        self.note_native_compiler_cache_mutation(
            tcl_registry::native_procedure::NativeCompilerCacheMutation::CommandToken {
                hook: self.native_hook_at_sidecar(key),
            },
        );
    }

    pub(crate) fn native_interpreter_identity(
        &self,
    ) -> tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
        tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
            owner: self.owner_nonce,
            interpreter: u64::try_from(self.cur.0).expect("interpreter slot fits u64"),
        }
    }

    /// A mutable ensemble map changes compilation selection without replacing
    /// its command token. Publish that change at the same cache owner as table
    /// registration and namespace-path mutations.
    pub(crate) fn note_ensemble_configuration_changed(
        &self,
        token: &tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>,
    ) {
        self.advance_native_ensemble_export_epoch(token.config().namespace);
        if let Some(count) = tcl_native_model::native_ensemble::configuration_compiler_mutations(
            self.actual_native_invocation_dialect(),
        ) {
            for _ in 0..count {
                self.note_native_compiler_cache_mutation(
                    tcl_registry::native_procedure::NativeCompilerCacheMutation::EnsembleConfiguration {
                        hook: token.compiler_hook(),
                    },
                );
            }
        }
        self.bump_cmd_epoch();
        self.invalidate_compiled_command_semantics();
        self.invalidate_guard_domain(GuardDomain::CommandEnvironment);
    }

    /// Update the compiler function attached to this exact live ensemble token.
    /// The native flag setter preserves existing imports' copied functions.
    #[cfg(test)]
    pub(crate) fn set_native_ensemble_compiler_attachment(
        &mut self,
        token: &Rc<tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>>,
        compile: bool,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
        use tcl_syntax::value::ValueError;
        let protocol = self
            .actual_native_invocation_dialect()
            .native_ensemble_compiler_attachment_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native ensemble compiler attachment",
            ))?;
        if self.ensemble_lifecycle_key(token).is_none() || token.is_deleted() {
            return Err(ValueError::CommandProtocolUnavailable(
                "retired native ensemble token",
            ));
        }
        let previous = token.compiler_hook();
        if previous == Hook::Unknown {
            return Err(ValueError::CommandProtocolUnavailable(
                "native ensemble compiler flag",
            ));
        }
        let next = protocol.hook(compile);
        token.set_compiler_hook(next);
        if previous != next {
            self.note_native_compiler_cache_mutation(
                tcl_registry::native_procedure::NativeCompilerCacheMutation::EnsembleConfiguration {
                    hook: Hook::Present,
                },
            );
            self.bump_cmd_epoch();
            self.invalidate_compiled_command_semantics();
            self.invalidate_guard_domain(GuardDomain::CommandEnvironment);
        }
        Ok(())
    }
}
