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

//! The targets a command word may reach: source keys, alias chains and unresolved dispatch.

use super::super::binding_projection::{CommandTargetLookup, source_binding_projection};
use super::super::command_presence::SourceCommandSlotPresence;
use super::super::namespace_context::{CommandKeyQuery, NamespaceKeyQuery, SourceNamespaceKey};
use super::super::source_walk::registry_head;
use super::super::trusted_packages::autoload_defines_nothing;
use super::super::{
    BindingKind, MayBinding, ModuleCommandBindings, ResolvedCommandTarget, SourceCommandKey,
    logical_prefix, original_command_table, runtime_entry,
};
use crate::naming::normalise_qualified_name as nqn;
use std::collections::BTreeSet;
use tcl_registry::CommandRegistry;

impl ModuleCommandBindings {
    /// Whether `::unknown` is still the fresh interpreter's handler (or
    /// absent). A redefined handler is a hook the closed-builtin shortcut
    /// does not model, so it withholds that shortcut.
    /// Whether `head` is absent from the command table and its dispatch
    /// reaches only the stock `unknown`, whose autoload defines nothing
    /// (#2427): the call runs no code the module cannot see.
    pub(crate) fn absent_head_runs_nothing(
        &self,
        head: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> bool {
        self.definitely_absent(head, namespace)
            && self.absent_head_reaches_only_stock_autoload(head, namespace)
    }

    /// Whether `head`, where it is absent, dispatches only to the stock
    /// `unknown` with no namespace handler in the way, and that handler's
    /// autoload defines nothing ([`autoload_defines_nothing`]). Every other
    /// alternative of the head is a terminal target.
    pub(crate) fn absent_head_reaches_only_stock_autoload(
        &self,
        head: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> bool {
        let Ok(keys) = self.source_keys_checked(head, namespace) else {
            return false;
        };
        !self.opaque_domain
            && !self.namespace_unknown_handlers.contains(namespace)
            && keys.iter().all(|key| {
                self.binding_alternatives(key).iter().all(|binding| {
                    matches!(binding, MayBinding::Missing)
                        || matches!(binding, MayBinding::Target(target) if target.terminal)
                })
            })
            && self.unknown_handler_is_original()
            && autoload_defines_nothing(self, Some(head))
    }

    pub(in super::super) fn unknown_handler_is_original(&self) -> bool {
        let root = self
            .native_root_namespace_key()
            .unwrap_or_else(|| SourceNamespaceKey::Authored("::".to_owned()));
        let binding = source_binding_projection(self, "::unknown", &root);
        // The stock handler is the interpreter's own; any source-allocated
        // or redefined implementation is a hook.
        !binding.unknown
            && binding.targets.iter().all(|target| {
                target.registry_backed
                    && target.prepended.is_empty()
                    && target.implementation_generation == 0
                    && target.implementation_allocation.is_none()
            })
    }
}

impl ModuleCommandBindings {
    pub(in super::super) fn definitely_absent(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> bool {
        self.source_slot_presence(name, namespace) == SourceCommandSlotPresence::Absent
    }
}

impl ModuleCommandBindings {
    /// Traits of a literal head that a runtime receiver's member body reaches
    /// in its private namespace or `::oo::Helpers` (`self`, `my`, `next`), when
    /// the call publishes no command itself. Pure introspection qualifies
    /// anywhere; a dispatch to the receiver's methods only in a declaration
    /// preview, whose effects stay in the preview.
    pub(in super::super) fn receiver_context_head(
        &self,
        registry: &CommandRegistry,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        preview: bool,
    ) -> Option<bool> {
        use tcl_registry::Traits;
        let spec = registry_head(registry, effective)?;
        if !matches!(
            self.variable_frame,
            crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
        ) {
            return None;
        }
        let traits = spec.traits;
        (traits.contains(Traits::TCLOO_METHOD_CONTEXT)
            && !traits.contains(Traits::TCLOO_BINDS_METHOD_ALIAS)
            && (preview || spec.is_pure()))
        .then_some(spec.is_pure())
    }

    /// The frame effect a declared command states when an authored call
    /// names it literally and nothing in this world defines it.
    pub(in super::super) fn declared_command_frame_effect(
        &self,
        registry: &CommandRegistry,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
    ) -> Option<tcl_registry::model::DeclaredFrameEffect> {
        if self.baseline.native_entry.is_some()
            || self.baseline.unknown_entry
            || self.has_opaque_domain()
        {
            return None;
        }
        let crate::registry_invocation::EffectiveInvocationWord::Literal(head) =
            effective.first()?
        else {
            return None;
        };
        let surface = self
            .baseline
            .declared_surface
            .as_ref()?
            .document_surface(registry);
        // Nothing in this world defines the name at any slot, so a relative
        // name falls back to the global namespace, where a workspace
        // procedure is declared under its rooted name.
        let command = surface.declared_command(head).or_else(|| {
            surface.declared_command(&tcl_registry::rooted_command_descriptor_label(head))
        })?;
        Some(command.frame_effect)
    }

    /// Resolve every terminal target a source spelling may invoke in
    /// `namespace`, including arguments prepended by alias chains.
    #[must_use]
    pub(crate) fn targets(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> BTreeSet<ResolvedCommandTarget> {
        self.resolve_targets(name, namespace, &mut BTreeSet::new())
    }

    /// A just-published declaration key is already constructed; resolving it
    /// again as written syntax could select a different namespace object.
    pub(in super::super) fn installed_procedure_targets(
        &self,
        key: &(impl CommandKeyQuery + ?Sized),
        declaration: u32,
    ) -> Vec<ResolvedCommandTarget> {
        self.bindings
            .get(key)
            .into_iter()
            .flatten()
            .filter_map(|binding| match binding {
                MayBinding::Target(target)
                    if target.terminal
                        && target.kind == BindingKind::Proc
                        && target.implementation_generation == declaration =>
                {
                    Some(target.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// Whether invoking `name` in `namespace` may reach an implementation that
    /// cannot be named statically.
    #[must_use]
    pub(crate) fn target_may_be_unknown(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> bool {
        self.has_opaque_domain() || self.target_resolution_may_be_unknown(name, namespace)
    }

    /// Whether this exact source spelling's resolved binding may be unknown,
    /// excluding unrelated module-wide opacity. Semantic effect consumers use
    /// this narrower query; runtime provenance still consumes
    /// [`Self::target_may_be_unknown`] and [`Self::has_opaque_domain`].
    #[must_use]
    pub(crate) fn target_resolution_may_be_unknown(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> bool {
        self.source_keys_checked(name, namespace).is_err()
            || self.resolve_target_may_be_unknown(name, namespace, &mut BTreeSet::new())
    }

    /// Original-word candidates in the exact retained namespace domain. A
    /// later native fallback is validated only after earlier slots permit it.
    pub(in super::super) fn original_source_keys_for_paths(
        &self,
        paths: Vec<Vec<SourceCommandKey>>,
    ) -> Result<
        Vec<SourceCommandKey>,
        tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
    > {
        let mut keys = Vec::new();
        for path in paths {
            for (index, key) in path.iter().enumerate() {
                let bindings = self.original_bindings_view(key).ok_or(
                    tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable::Namespace,
                )?;
                let absent = bindings.contains(&MayBinding::Missing);
                if (!absent || bindings.len() > 1 || index + 1 == path.len()) && !keys.contains(key)
                {
                    keys.push(key.clone());
                }
                if !absent {
                    break;
                }
            }
        }
        Ok(keys)
    }

    pub(in super::super) fn source_keys_checked<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        name: &str,
        namespace: &Q,
    ) -> Result<
        Vec<SourceCommandKey>,
        tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
    > {
        let namespace = namespace.namespace_key();
        if let Some(paths) = self.receiver_native_lookup_paths(name, namespace.as_ref()) {
            return paths.map(|paths| paths.into_iter().flatten().collect());
        }
        if name.is_ascii()
            && !name.as_bytes().contains(&0)
            && !matches!(namespace.as_ref(), SourceNamespaceKey::Native(_))
            && let Some(policy) = self
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        {
            let paths = self
                .original_registry_command_paths(namespace.as_ref(), name, policy)
                .ok_or(
                    tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable::Namespace,
                )?;
            return self.original_source_keys_for_paths(paths);
        }
        if !tcl_syntax::naming::is_absolute(name)
            && self.unknown_lookup_namespaces.contains(namespace.as_ref())
        {
            return Err(
                tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable::Namespace,
            );
        }
        if !matches!(namespace.as_ref(), SourceNamespaceKey::Authored(_)) {
            return self
                .native_source_lookup_paths(name, namespace.as_ref())
                .map(|paths| {
                    let mut result = Vec::new();
                    for key in paths.into_iter().flatten() {
                        if !result.contains(&key) {
                            result.push(key);
                        }
                    }
                    result
                });
        }
        let SourceNamespaceKey::Authored(namespace_name) = namespace.as_ref() else {
            return Err(
                tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable::Namespace,
            );
        };
        let paths = if let Some(path) = self.receiver_command_lookup_path(name) {
            vec![path]
        } else {
            let default_path = BTreeSet::from([Vec::new()]);
            self.namespace_paths
                .get(namespace.as_ref())
                .unwrap_or(&default_path)
                .iter()
                .map(|path| {
                    let path = path
                        .iter()
                        .map(|key| match key {
                            SourceNamespaceKey::Authored(text) => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()?;
                    Some(
                        tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
                            namespace_name,
                            &path,
                            name,
                        ),
                    )
                })
                .collect::<Option<Vec<_>>>()
                .ok_or(
                    tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable::Namespace,
                )?
        };
        let mut keys = Vec::new();
        for candidates in paths {
            for (index, text) in candidates.iter().enumerate() {
                let key = SourceCommandKey::authored(text.clone());
                let bindings = self.bindings.get(&key).cloned().unwrap_or_else(|| {
                    Self::unmodified_bindings(&key, self.baseline.semantics.binding_names())
                });
                let absent = bindings.contains(&MayBinding::Missing);
                if (!absent || bindings.len() > 1 || index + 1 == candidates.len())
                    && !keys.contains(&key)
                {
                    keys.push(key);
                }
                if !absent {
                    break;
                }
            }
        }
        Ok(keys)
    }

    pub(in super::super) fn source_lookup_paths<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        name: &str,
        namespace: &Q,
    ) -> Vec<Vec<SourceCommandKey>> {
        let key = namespace.namespace_key();
        if let Some(paths) = self.receiver_native_lookup_paths(name, key.as_ref()) {
            return paths.unwrap_or_default();
        }
        if name.is_ascii()
            && !name.as_bytes().contains(&0)
            && let Some(policy) = self
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        {
            return self
                .original_registry_command_paths(key.as_ref(), name, policy)
                .unwrap_or_default();
        }
        if !matches!(key.as_ref(), SourceNamespaceKey::Authored(_)) {
            return self
                .native_source_lookup_paths(name, key.as_ref())
                .unwrap_or_default();
        }
        let SourceNamespaceKey::Authored(namespace) = key.as_ref() else {
            return Vec::new();
        };
        if let Some(path) = self.receiver_command_lookup_path(name) {
            return vec![path.into_iter().map(SourceCommandKey::authored).collect()];
        }
        let default_path = BTreeSet::from([Vec::new()]);
        self.namespace_paths
            .get(key.as_ref())
            .unwrap_or(&default_path)
            .iter()
            .map(|path| {
                let path = path
                    .iter()
                    .filter_map(|context| match context {
                        SourceNamespaceKey::Authored(text) => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
                    namespace, &path, name,
                )
                .into_iter()
                .map(SourceCommandKey::authored)
                .collect()
            })
            .collect()
    }

    pub(in super::super) fn source_keys<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        name: &str,
        namespace: &Q,
    ) -> Vec<SourceCommandKey> {
        self.source_keys_checked(name, namespace)
            .unwrap_or_default()
    }

    /// Conditional unresolved dispatch against this exact original source table.
    /// Historical missing alternatives do not erase a named module target.
    pub(crate) fn may_dispatch_unresolved<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        name: &str,
        namespace: &Q,
    ) -> bool {
        self.dispatches_unresolved(name, namespace, &mut BTreeSet::new())
    }

    fn dispatches_unresolved<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        name: &str,
        namespace: &Q,
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> bool {
        let Ok(keys) = self.source_keys_checked(name, namespace) else {
            return true;
        };
        let mut named = false;
        let mut unresolved = false;
        for key in keys {
            if !visiting.insert(key.clone()) {
                continue;
            }
            if !self.bindings.contains_key(&key) && self.declares_command_key(&key) {
                named = true;
            }
            for binding in self.binding_alternatives(&key) {
                match binding {
                    MayBinding::Unknown => unresolved = true,
                    MayBinding::Missing => {}
                    MayBinding::Imported(import) => {
                        named = true;
                        unresolved |= self.objects.get(&import.origin).is_none_or(|alternatives| {
                            alternatives
                                .iter()
                                .any(|binding| matches!(binding, MayBinding::Unknown))
                        });
                    }
                    MayBinding::Target(target) if target.terminal => named = true,
                    MayBinding::Target(target) => {
                        named = true;
                        unresolved |= self.dispatches_unresolved(
                            &target.command,
                            &self.alias_target_namespace_key(&target, namespace),
                            visiting,
                        );
                    }
                }
            }
            visiting.remove(&key);
        }
        unresolved || !named
    }

    /// Whether `name` reaches only document-declared commands: each lookup
    /// key is one a declaration names and nothing in this world binds.
    pub(crate) fn names_only_a_declared_command<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        name: &str,
        namespace: &Q,
    ) -> bool {
        !self.baseline.declared.is_empty()
            && self.source_keys_checked(name, namespace).is_ok_and(|keys| {
                !keys.is_empty()
                    && keys.iter().all(|key| {
                        !self.bindings.contains_key(key) && self.declares_command_key(key)
                    })
            })
    }

    /// Whether a document declaration (`# tcl-lsp: stub …`) names `key`. A
    /// symbolic key is the declared spelling; a selected slot is the one the
    /// declaration's metadata coordinate occupies under the execution policy.
    fn declares_command_key(&self, key: &SourceCommandKey) -> bool {
        if let Some(name) = key.authored_spelling() {
            return self.baseline.declared.contains_key(name);
        }
        let Some(policy) = self
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        else {
            return false;
        };
        self.baseline.declared.keys().any(|name| {
            original_command_table::original_registry_metadata_slot(policy, name)
                .and_then(|slot| self.original_command_key_for_slot(&slot, policy))
                .as_ref()
                == Some(key)
        })
    }

    pub(in super::super) fn alias_target_namespace_key<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        target: &ResolvedCommandTarget,
        caller: &Q,
    ) -> SourceNamespaceKey {
        if target.target_lookup == tcl_registry::AliasTargetLookup::CallerNamespace {
            return caller.namespace_key().into_owned();
        }
        self.native_root_namespace_key()
            .unwrap_or_else(|| SourceNamespaceKey::authored("::"))
    }

    pub(in super::super) fn source_root_namespace_key(&self) -> Option<SourceNamespaceKey> {
        if self.baseline.native_entry.is_some() {
            self.native_root_namespace_key()
        } else {
            Some(SourceNamespaceKey::default())
        }
    }

    pub(in super::super) fn native_root_namespace_key(&self) -> Option<SourceNamespaceKey> {
        let entry = self.baseline.native_entry.as_ref()?;
        let mut roots = entry
            .namespaces
            .iter()
            .filter(|row| row.visible && row.path.is_root());
        let root = roots.next()?;
        if roots.next().is_some() {
            return None;
        }
        runtime_entry::native_namespace_key(entry, root.token)
    }

    fn resolve_target_may_be_unknown(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> bool {
        if self.source_keys_checked(name, namespace).is_err() {
            return true;
        }
        let keys = self.source_keys(name, namespace);
        let mut unknown = false;
        for (index, key) in keys.iter().enumerate() {
            if !visiting.insert(key.clone()) {
                return true;
            }
            let bindings = self.binding_alternatives(key);
            unknown |= bindings.iter().any(|binding| match binding {
                MayBinding::Unknown => true,
                MayBinding::Imported(token) => {
                    self.runtime_execution_observed(token.compiler.as_ref())
                        || self
                            .objects
                            .get(&token.origin)
                            .is_none_or(|implementations| {
                                implementations
                                    .iter()
                                    .any(|implementation| match implementation {
                                        MayBinding::Target(target) if target.terminal => false,
                                        MayBinding::Target(target) => self
                                            .resolve_target_may_be_unknown(
                                                &target.command,
                                                &self.alias_target_namespace_key(target, namespace),
                                                visiting,
                                            ),
                                        MayBinding::Missing
                                        | MayBinding::Unknown
                                        | MayBinding::Imported(_) => true,
                                    })
                            })
                }
                MayBinding::Missing => {
                    index + 1 == keys.len()
                        && (self.namespace_unknown_handlers.contains(namespace)
                            || self.unresolved_target_may_be_unknown(name, visiting))
                }
                MayBinding::Target(target)
                    if self.runtime_execution_observed(target.token.as_ref()) =>
                {
                    true
                }
                MayBinding::Target(target) if target.terminal => false,
                MayBinding::Target(target) => self.resolve_target_may_be_unknown(
                    &target.command,
                    &self.alias_target_namespace_key(target, namespace),
                    visiting,
                ),
            });
            visiting.remove(key);
        }
        unknown
    }

    /// Whether Tcl's registry-declared unresolved-command fallback can reach
    /// an implementation whose effects are not statically bounded. A missing
    /// handler never dispatches recursively through itself.
    fn unresolved_target_may_be_unknown(
        &self,
        missing_name: &str,
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> bool {
        let missing_key = nqn(missing_name);
        let Some(root) = self.source_root_namespace_key() else {
            return true;
        };
        self.baseline
            .semantics
            .unresolved_command_handlers()
            .iter()
            .filter(|handler| **handler != missing_key)
            .any(|handler| self.resolve_target_may_be_unknown(handler, &root, visiting))
    }

    /// Resolve Tcl's registry-declared unresolved-command fallback, retaining
    /// the missing command name as the argument Tcl appends to the handler's
    /// command prefix. A missing handler is terminal failure, not recursion.
    fn unresolved_targets(
        &self,
        missing_name: &str,
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> BTreeSet<ResolvedCommandTarget> {
        let missing_key = nqn(missing_name);
        let Some(root) = self.source_root_namespace_key() else {
            return BTreeSet::new();
        };
        let mut resolved = BTreeSet::new();
        for handler in self
            .baseline
            .semantics
            .unresolved_command_handlers()
            .iter()
            .filter(|handler| **handler != missing_key)
        {
            for mut terminal in self.resolve_targets(handler, &root, visiting) {
                terminal.prepended.push(
                    crate::registry_invocation::EffectiveInvocationWord::Literal(
                        missing_name.to_owned(),
                    ),
                );
                resolved.insert(terminal);
            }
        }
        resolved
    }

    fn resolve_targets(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> BTreeSet<ResolvedCommandTarget> {
        self.resolve_targets_in(name, namespace, visiting, CommandTargetLookup::WithFallback)
    }

    /// Named-slot lookup and executable fallback share ordering and alias
    /// traversal. A fallback handler is not the written command's operation
    /// and cannot supply original compilation-layout advice.
    fn resolve_targets_in(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        visiting: &mut BTreeSet<SourceCommandKey>,
        lookup: CommandTargetLookup,
    ) -> BTreeSet<ResolvedCommandTarget> {
        self.resolve_targets_with_prefixes_in(name, namespace, visiting, lookup)
            .targets
    }

    pub(in super::super) fn resolve_targets_with_prefixes_in(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        visiting: &mut BTreeSet<SourceCommandKey>,
        lookup: CommandTargetLookup,
    ) -> logical_prefix::LogicalPrefixResolution {
        let mut resolved = logical_prefix::LogicalPrefixResolution::default();
        let keys = self.source_keys(name, namespace);
        for (index, key) in keys.iter().enumerate() {
            if !visiting.insert(key.clone()) {
                continue;
            }
            let bindings = self.binding_alternatives(key);
            for binding in &bindings {
                match binding {
                    MayBinding::Imported(token) => {
                        if let Some(implementations) = self.objects.get(&token.origin) {
                            for implementation in implementations {
                                if let MayBinding::Target(target) = implementation {
                                    if target.terminal {
                                        resolved.insert(target.clone(), target.prepended.is_empty().then(|| logical_prefix::LogicalCapturedCommandPrefix::empty(self)).flatten());
                                    } else {
                                        let children = self.resolve_targets_with_prefixes_in(
                                            &target.command,
                                            &self.alias_target_namespace_key(target, namespace),
                                            visiting,
                                            lookup,
                                        );
                                        for mut terminal in children.targets {
                                            let prefix = self.compose_logical_alias_prefix(
                                                target,
                                                children.prefixes.get(&terminal).cloned().flatten(),
                                            );
                                            terminal
                                                .prepended
                                                .extend(target.prepended.iter().cloned());
                                            resolved.insert(terminal, prefix);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    MayBinding::Target(target) if target.terminal => {
                        resolved.insert(
                            target.clone(),
                            target
                                .prepended
                                .is_empty()
                                .then(|| logical_prefix::LogicalCapturedCommandPrefix::empty(self))
                                .flatten(),
                        );
                    }
                    MayBinding::Target(target) => {
                        let children = self.resolve_targets_with_prefixes_in(
                            &target.command,
                            &self.alias_target_namespace_key(target, namespace),
                            visiting,
                            lookup,
                        );
                        for mut terminal in children.targets {
                            let prefix = self.compose_logical_alias_prefix(
                                target,
                                children.prefixes.get(&terminal).cloned().flatten(),
                            );
                            terminal.prepended.extend(target.prepended.iter().cloned());
                            resolved.insert(terminal, prefix);
                        }
                    }
                    MayBinding::Missing
                        if index + 1 == keys.len()
                            && matches!(lookup, CommandTargetLookup::WithFallback)
                            && !self.namespace_unknown_handlers.contains(namespace) =>
                    {
                        for target in self.unresolved_targets(name, visiting) {
                            resolved.insert(target, None);
                        }
                    }
                    MayBinding::Missing | MayBinding::Unknown => {}
                }
            }
            visiting.remove(key);
        }
        resolved
    }
}
