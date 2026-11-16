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

//! Registering builtins: host handlers, stock builtins and stock namespace ensembles, guarded builtins and spec builtins.

use super::super::Vm;
use crate::command::{BuiltinFn, Command, EnsembleDef, NativeEnsembleImplementation};
use std::collections::BTreeSet;
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_runtime_api::Namespaces;
use tcl_runtime_api::guard::GuardIdentity;

impl Vm {
    /// Register a host handler without claiming a stock native implementation.
    /// Semantic contracts require the explicit guarded or spec-backed doors;
    /// none of these public doors grant C Tcl compiler-hook provenance.
    pub fn register(&mut self, name: &str, f: BuiltinFn) {
        self.register_host_builtin(name, f);
    }

    fn register_host_builtin(&mut self, name: &str, f: BuiltinFn) -> bool {
        if self.execution_refusal.is_some() {
            return false;
        }
        self.register_command_with_host_input(name, Command::Builtin(f))
            .into_key()
            .is_some()
    }

    /// Engine bootstrap alone can identify an actual stock implementation.
    pub(crate) fn register_stock_builtin(&mut self, name: &str, f: BuiltinFn) {
        if !self.register_host_builtin(name, f) {
            return;
        }
        let canonical = tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name);
        if tcl_registry::mathfunc::is_in_mathfunc_namespace(canonical) {
            if self.fixed_math.handlers.contains_key(canonical) {
                self.note_native_compiler_cache_mutation(
                    tcl_registry::native_procedure::NativeCompilerCacheMutation::FixedMath,
                );
            }
            self.fixed_math.handlers.insert(canonical.to_owned(), f);
            self.fixed_math.generation = self
                .fixed_math
                .generation
                .checked_add(1)
                .expect("fixed math registration generation fits u64");
            let next = self.fixed_math.generation;
            let token = self
                .fixed_math
                .registrations
                .get(canonical)
                .map_or(next, |(token, _, _)| *token);
            self.fixed_math
                .registrations
                .insert(canonical.to_owned(), (token, next, name.to_owned()));
        }
        self.builtin_identities
            .insert(canonical.to_owned(), canonical.to_owned());
    }

    /// Bootstrap a real stock ensemble and its actual private member tokens.
    /// The caller selects the measured release surface before this ingress.
    pub(crate) fn register_stock_namespace_ensemble(
        &mut self,
        identity: &'static str,
        implementation_namespace: &str,
        members: &[(&str, BuiltinFn)],
        admitted_members: &[&str],
    ) {
        self.register_stock_namespace_ensemble_with_prefixes(
            identity,
            implementation_namespace,
            members,
            admitted_members,
            true,
        );
    }

    /// Native nested codec ensembles have exact member lookup rather than prefixes.
    pub(crate) fn register_stock_namespace_ensemble_with_prefixes(
        &mut self,
        identity: &'static str,
        implementation_namespace: &str,
        members: &[(&str, BuiltinFn)],
        admitted_members: &[&str],
        prefixes: bool,
    ) {
        let born_here = !self.namespace_exists(implementation_namespace);
        self.declare_namespace_key(
            tcl_syntax::naming::unroot_rooted_key(implementation_namespace)
                .unwrap_or(implementation_namespace),
        );
        let namespace = self
            .find_namespace(Namespaces::current(self), implementation_namespace)
            .expect("native implementation namespace was declared");
        if born_here {
            self.name_world
                .borrow_mut()
                .ns_deferral
                .bootstrap_ensembles
                .insert(identity, namespace);
        }
        let mut map = Vec::new();
        for &(member, handler) in members {
            if admitted_members.contains(&member) {
                let nested = (identity == "info")
                    .then(|| {
                        self.actual_native_invocation_dialect()
                            .info_oo_ensemble_namespace(member)
                    })
                    .flatten();
                let target = nested.map_or_else(
                    || tcl_syntax::naming::rooted_member_key(implementation_namespace, member),
                    str::to_owned,
                );
                let key = tcl_syntax::naming::unroot_rooted_key(&target).unwrap_or(&target);
                if nested.is_none()
                    && (!self.commands.contains_key(key)
                        || self.stock_native_identity(&target).as_deref() == Some(key))
                {
                    self.register_stock_builtin(&target, handler);
                }
                map.push((member.to_owned(), vec![target]));
            }
        }
        let native = Rc::new(NativeEnsembleImplementation {
            identity,
            namespace,
            prefixes,
            map: map.clone(),
        });
        let mut definition = EnsembleDef {
            originals: crate::command::native_ensemble_objects::NativeEnsembleObjects::default(),
            native: Some(native),
            namespace,
            map: map
                .into_iter()
                .map(|(name, words)| {
                    (
                        NameBytes::from(name),
                        words
                            .into_iter()
                            .map(|word| Some(NameBytes::from(word)))
                            .collect(),
                    )
                })
                .collect(),
            subcommands: None,
            prefixes,
            parameters: Vec::new(),
            unknown: None,
        };
        self.prepare_original_ensemble_configuration(&mut definition);
        definition.originals.activate();
        self.register_namespace_ensemble(
            identity,
            &Rc::new(
                tcl_cmd_core::ensemble::EnsembleToken::with_configuration_retirement(
                    definition,
                    NameBytes::from(tcl_syntax::naming::root_unrooted_key(identity)),
                    self.native_hook_for_registry_identity(identity),
                    crate::command::native_ensemble_objects::retire_configuration,
                ),
            ),
        );
    }

    /// Release a bootstrap-owned namespace only when its actual token has no
    /// remaining user state. User-created same-name recreations are independent.
    pub(crate) fn retire_unused_stock_ensemble_namespace(&mut self, identity: &'static str) {
        let Some(namespace) = self
            .name_world
            .borrow()
            .ns_deferral
            .bootstrap_ensembles
            .get(identity)
            .copied()
        else {
            return;
        };
        let path = self.ns_path(namespace);
        if self.name_world.borrow().ns_intern.get(&path) != Some(&namespace) {
            self.name_world
                .borrow_mut()
                .ns_deferral
                .bootstrap_ensembles
                .remove(identity);
            return;
        }
        if !self.names_directly_in_token(namespace, false).is_empty()
            || self
                .ns_vars
                .get(&namespace)
                .is_some_and(|variables| !variables.is_empty())
            || !self.child_namespace_ids(namespace).is_empty()
            || self
                .ns_exports
                .get(&namespace)
                .is_some_and(|exports| !exports.is_empty())
            || self
                .name_world
                .borrow()
                .ns_paths
                .get(&namespace)
                .is_some_and(|paths| !paths.is_empty())
            || self.ns_unknowns.contains_key(&namespace)
            || self
                .namespace_ensembles
                .get(&namespace)
                .is_some_and(|tokens| tokens.iter().any(|token| token.upgrade().is_some()))
            || self
                .name_world
                .borrow()
                .ns_deferral
                .counts
                .get(&namespace)
                .is_some_and(|count| *count > 0)
        {
            return;
        }
        self.name_world
            .borrow_mut()
            .ns_deferral
            .bootstrap_ensembles
            .remove(identity);
        self.delete_namespace_token(namespace, false);
    }

    /// Register a builtin together with a stable semantic identity that
    /// offline-generated code may request in a runtime guard.
    ///
    /// Ordinary builtins deliberately have no such identity. Adding one is an
    /// explicit runtime implementation decision, not an inference from the
    /// command's spelling or handler address.
    ///
    /// The handler is the embedder's, whatever its name: compiled code
    /// specialised for the shipped builtin at the same registry name is not
    /// admitted over it ([`Self::command_binding_matches`]), so the handler
    /// answers.
    pub fn register_guarded_builtin(&mut self, name: &str, f: BuiltinFn, identity: GuardIdentity) {
        let canonical = tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name);
        self.register(canonical, f);
        self.builtin_identities.remove(canonical);
        if let Some(generation) = self.visible_command_generation(canonical) {
            self.guarded_commands.host.insert(generation);
            self.attest(generation, None, BTreeSet::from([identity]));
        }
    }

    pub fn register_spec_builtin(&mut self, spec: &tcl_registry::CommandSpec, f: BuiltinFn) {
        self.register(spec.name, f);
        self.install_spec_semantic_guards(spec);
    }

    fn install_spec_semantic_guards(&mut self, spec: &tcl_registry::CommandSpec) {
        let identities: BTreeSet<_> = spec
            .intrinsic_ids()
            .into_iter()
            .flat_map(|id| {
                id.guard_semantics_variants().iter().map(move |semantics| {
                    GuardIdentity::registry_intrinsic_with_semantics(id.stable_id(), *semantics)
                })
            })
            .collect();
        if let Some(generation) = self.visible_command_generation(
            tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name),
        ) {
            self.attest(generation, None, identities);
        }
    }
}
