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

//! Registering builtins and object commands: publication, attestation of semantic identities and the backing report.

use super::super::{
    native_operation_currency, BuiltinFn, Interp, ObjCommand, OriginalBuiltinPublication,
    OriginalCommandPublication,
};
use super::Command;
#[cfg(not(have_tommath))]
use crate::builtins;
#[cfg(test)]
use crate::interp::constructor_diagnostics::ConstructorDiagnostics;
#[cfg(test)]
use crate::interp::constructor_diagnostics::Kind;
use crate::namespace::GLOBAL;
use std::rc::Rc;
use tcl_runtime_api::guard::GuardIdentity;
use tcl_runtime_api::RegisteredBacking;

impl Interp {
    // command registry

    /// Register a host handler at a possibly-qualified name. This replaces the
    /// actual binding without claiming stock dialect or compiler identity.
    pub fn register_builtin(&mut self, name: &[u8], f: BuiltinFn) {
        self.bind_builtin(name, f);
    }

    /// Only engine installation identifies an actual stock handler token.
    pub(crate) fn register_stock_builtin(&mut self, name: &[u8], f: BuiltinFn) -> u64 {
        self.register_stock_builtin_publication(name, f)
            .map_or(0, |publication| publication.generation)
    }

    /// Publish a stock object handler with its actual client-data deletion owner.
    pub(crate) fn register_stock_obj_command(
        &mut self,
        name: &[u8],
        command: ObjCommand,
    ) -> Option<u64> {
        let generation = self.create_obj_command(name, command)?;
        self.check_entered_native_operation().ok()?;
        let (fqn, _) = self.raw_command_location_by_generation(generation)?;
        self.registry_builtin_names
            .borrow_mut()
            .insert(generation, fqn.clone());
        self.remember_stock_registration(generation);
        self.record_scripted_helper_registration(generation);
        Some(generation)
    }

    pub(in super::super) fn register_stock_builtin_publication(
        &mut self,
        name: &[u8],
        f: BuiltinFn,
    ) -> Option<OriginalBuiltinPublication> {
        let publication = self.bind_builtin_publication(name, f)?;
        self.check_entered_native_operation().ok()?;
        if !self.original_command_publication_is_current(
            publication.namespace,
            &publication.tail,
            publication.generation,
        ) {
            return None;
        }
        let (fqn, _) = self.raw_command_location_by_generation(publication.generation)?;
        self.registry_builtin_names
            .borrow_mut()
            .insert(publication.generation, fqn.clone());
        self.remember_stock_registration(publication.generation);
        self.record_scripted_helper_registration(publication.generation);
        Some(publication)
    }

    /// Bind a builtin through its actual publication owner.
    fn bind_builtin(&mut self, name: &[u8], f: BuiltinFn) -> u64 {
        self.bind_builtin_publication(name, f)
            .map_or(0, |publication| publication.generation)
    }

    fn bind_builtin_publication(
        &mut self,
        name: &[u8],
        f: BuiltinFn,
    ) -> Option<OriginalBuiltinPublication> {
        let original_key = match self.generated_jim_command_key(
            GLOBAL,
            name,
            tcl_syntax::naming::NativeNamePurpose::CommandCApiPublication,
        ) {
            Ok(key) => key,
            Err(error) => {
                self.report_cmd_error(error.into());
                return None;
            }
        };
        let Some((ns, tail)) = self.namespaces_mut().command_publication_at(GLOBAL, name) else {
            let _ = self.refuse_host_command("builtin publication has no actual namespace");
            return None;
        };
        let report = self.namespaces().jim_command_table_key_at(
            GLOBAL,
            name,
            tcl_syntax::naming::NativeNamePurpose::AliasPublication,
        );
        let OriginalCommandPublication::Published(generation) = self
            .bind_command_replacement_with_original_jim_key(
                ns,
                &tail,
                Command::Builtin(f),
                report,
                original_key,
            )
        else {
            return None;
        };
        if self.host_refusal_pending() {
            return None;
        }
        self.remember_original_byte_command_registration(name, ns, &tail, generation);
        Some(OriginalBuiltinPublication {
            namespace: ns,
            tail,
            generation,
        })
    }

    /// The one writer of the attestation table: attest `identities` for the
    /// command token `generation` and drop the entry of the token it displaced.
    /// What is already attested for `generation` stays.
    pub(in super::super) fn attest(
        &self,
        generation: u64,
        displaced: Option<u64>,
        identities: std::collections::BTreeSet<GuardIdentity>,
    ) {
        let mut attested = self.guarded_commands.borrow_mut();
        if let Some(displaced) = displaced {
            attested.remove(&displaced);
        }
        if !identities.is_empty() {
            attested.entry(generation).or_default().extend(identities);
        }
    }

    /// Register a builtin with a stable semantic identity understood by
    /// offline-generated code. Ordinary registration never infers identity
    /// from a spelling or function address.
    pub fn register_guarded_builtin(&mut self, name: &[u8], f: BuiltinFn, identity: GuardIdentity) {
        let generation = self.bind_builtin(name, f);
        if generation != 0 {
            self.retire_guarded_command_identities();
            self.attest(
                generation,
                None,
                std::collections::BTreeSet::from([identity]),
            );
        }
    }

    /// Register a builtin and derive every semantic identity from its registry
    /// command, subcommand, and invocation-form descriptors.
    pub fn register_spec_builtin(&mut self, spec: &tcl_registry::CommandSpec, f: BuiltinFn) {
        let generation = self.bind_builtin(spec.name.as_bytes(), f);
        if generation != 0 {
            self.retire_guarded_command_identities();
            self.attest_spec_at(spec, generation);
        }
    }

    pub(in super::super) fn attest_spec_at(
        &self,
        spec: &tcl_registry::CommandSpec,
        generation: u64,
    ) {
        let identities: std::collections::BTreeSet<_> = spec
            .intrinsic_ids()
            .into_iter()
            .flat_map(|id| {
                id.guard_semantics_variants().iter().map(move |semantics| {
                    GuardIdentity::registry_intrinsic_with_semantics(id.stable_id(), *semantics)
                })
            })
            .chain(
                spec.native_lowering_at(
                    self.native_invocation_dialect().authoring_query().as_ref(),
                )
                .definition_guard_identity(self.native_invocation_dialect()),
            )
            .collect();
        self.attest(generation, None, identities);
    }

    /// Register a builtin that only refuses: a call answers "not supported"
    /// where an unregistered command would answer `invalid command name`. The
    /// backing report says so ([`RegisteredBacking::Unsupported`]) rather than
    /// counting it as a handler.
    pub(crate) fn register_unsupported(&mut self, name: &[u8], f: BuiltinFn) {
        let generation = self.register_stock_builtin(name, f);
        if generation != 0 {
            self.unsupported_builtins.borrow_mut().insert(generation);
        }
    }

    /// Attach the registry's intrinsic identities to the builtins this
    /// interpreter registered, from the generation it is pinned to.
    ///
    /// The sweep reads that generation's shipped store and nothing an overlay
    /// installed: a pack's command is not a runtime implementation, and only
    /// the runtime may attest. A name is attested only while the command bound
    /// at it is still the builtin registered there, so a name a script has
    /// since redefined or renamed over gains no attestation for a procedure,
    /// and a builtin moved to another name keeps the one it already has (the
    /// table is keyed by the command token's generation, which a rename
    /// carries). The sweep runs after stock installation and retains its
    /// attestations through a profile pin. Public host registration never adds
    /// a shipped token, even when its spelling belongs to this registry.
    pub(crate) fn attach_identities(&mut self) {
        let registry = crate::environment::store_for_profile(self.dialect_profile());
        self.attach_identities_from(registry);
    }

    /// The sweep over one generation's store. Only [`Self::attach_identities`]
    /// chooses the store in production: the generation the interpreter is
    /// pinned to.
    pub(in super::super) fn attach_identities_from(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        #[cfg(test)]
        let mut constructor_diagnostic = ConstructorDiagnostics::new(Kind::Identities);
        let c_lookup = matches!(
            self.namespaces.borrow().variable_name_protocol,
            Some(tcl_syntax::naming::NativeNameProtocol::C(_))
        );
        for name in registry.command_names() {
            let Some(spec) = registry.get_exact(name) else {
                continue;
            };
            let early_generation = if c_lookup {
                let Some(generation) = self.shipped_builtin_generation(spec.name.as_bytes()) else {
                    continue;
                };
                Some(generation)
            } else {
                None
            };
            let identities: std::collections::BTreeSet<GuardIdentity> = spec
                .intrinsic_ids()
                .into_iter()
                .flat_map(|id| {
                    id.guard_semantics_variants().iter().map(move |semantics| {
                        GuardIdentity::registry_intrinsic_with_semantics(id.stable_id(), *semantics)
                    })
                })
                .chain(
                    spec.native_lowering_at(
                        self.native_invocation_dialect().authoring_query().as_ref(),
                    )
                    .definition_guard_identity(self.native_invocation_dialect()),
                )
                .collect();
            if identities.is_empty() {
                continue;
            }
            let Some(generation) =
                early_generation.or_else(|| self.shipped_builtin_generation(spec.name.as_bytes()))
            else {
                continue;
            };
            self.attest(generation, None, identities);
        }
        #[cfg(test)]
        constructor_diagnostic.stage("identities_ready");
    }

    /// The token generation of the builtin this interpreter registered at
    /// `name`, when that is still the command `name` resolves to from the
    /// global namespace.
    pub(in super::super) fn shipped_builtin_generation(&self, name: &[u8]) -> Option<u64> {
        let namespaces = self.namespaces.borrow();
        let generation = namespaces.resolve_generation(GLOBAL, name)?;
        let fqn = namespaces.resolve_fqn(GLOBAL, name)?;
        drop(namespaces);
        if self.0.registry_builtin_names.borrow().get(&generation) == Some(&fqn) {
            return Some(generation);
        }
        self.stock_ensemble_registry_identity(generation)
            .filter(|identity| identity == name)
            .filter(|_| self.stock_ensemble_implementation_is_current(generation))
            .map(|_| generation)
    }

    /// What this interpreter's handler table backs, by command name: the
    /// runtime's own answer to "what did you register", which a spec's
    /// `runtime_backing` declaration is held to.
    ///
    /// Every command bound in any namespace is classified by what it is — a
    /// native handler, an engine-installed `TclOO` root, or a handler that only
    /// refuses. Two kinds of name are reported that no table entry holds: the
    /// commands the object system binds in every object's namespace, which
    /// exist once the roots do; and, in a build that embeds the Tcl library,
    /// the commands that library defines, which exist once `init_library` has
    /// sourced it. A build without the numeric tower reports the commands a
    /// build with it registers as needing the tower. Names are without a
    /// constructed root marker; literal colon components remain part of each
    /// unrooted key. Use `BackingReport::from_unrooted_entries` to store these
    /// keys. This Unicode catalogue omits opaque byte names; lookup and
    /// publication retain those names independently through their counted APIs.
    #[must_use]
    pub fn backing_report(&self) -> Vec<(String, RegisteredBacking)> {
        let mut report = std::collections::BTreeMap::new();
        {
            let namespaces = self.namespaces.borrow();
            let unsupported = self.0.unsupported_builtins.borrow();
            let roots = self.0.registry_object_roots.borrow();
            for ns in namespaces.descendant_ids(GLOBAL) {
                for name in namespaces.command_names(ns) {
                    let backing = match namespaces.command_in(ns, name) {
                        Some(Command::Builtin(_)) => {
                            let refuses = namespaces
                                .command_generation(ns, name)
                                .is_some_and(|generation| unsupported.contains(&generation));
                            if refuses {
                                RegisteredBacking::Unsupported
                            } else {
                                RegisteredBacking::Builtin
                            }
                        }
                        Some(Command::ObjCmd(_))
                            if namespaces.command_generation(ns, name).is_some_and(
                                |generation| {
                                    self.registry_builtin_names
                                        .borrow()
                                        .contains_key(&generation)
                                },
                            ) =>
                        {
                            RegisteredBacking::Builtin
                        }
                        Some(Command::Ensemble(_))
                            if namespaces.command_generation(ns, name).is_some_and(
                                |generation| {
                                    self.stock_ensemble_registry_identity(generation).is_some()
                                },
                            ) =>
                        {
                            RegisteredBacking::Builtin
                        }
                        Some(Command::OoObject(id)) if roots.contains_key(&id) => {
                            RegisteredBacking::Object
                        }
                        _ => continue,
                    };
                    let fqn = namespaces.command_fqn_at(ns, name);
                    let Ok(text) = std::str::from_utf8(&fqn) else {
                        continue;
                    };
                    let key = tcl_syntax::naming::unroot_rooted_key(text)
                        .expect("actual command FQN contains its constructed root");
                    report.insert(key.to_owned(), backing);
                }
            }
            if !roots.is_empty() {
                for name in crate::cmd_oo::object_namespace_command_names() {
                    report.insert(name.to_owned(), RegisteredBacking::Object);
                }
            }
        }
        #[cfg(feature = "wasm_stdlib")]
        for (name, file) in crate::embedded_stdlib::DEFINED_COMMANDS {
            report
                .entry((*name).to_owned())
                .or_insert(RegisteredBacking::Stdlib { file });
        }
        #[cfg(not(have_tommath))]
        for name in builtins::tower_command_names() {
            report
                .entry(name)
                .or_insert(RegisteredBacking::NeedsNumericTower);
        }
        report.into_iter().collect()
    }

    /// The identities attested for the command `name` resolves to from the
    /// current namespace. The name is resolved afresh, so a command that was
    /// replaced, renamed away, hidden, or is no longer admitted by the pinned
    /// surface answers `None`, and one restored by `rename` or `expose` answers
    /// what it did before.
    #[cfg(test)]
    pub(in super::super) fn attested_identities(
        &self,
        name: &[u8],
    ) -> Option<std::collections::BTreeSet<GuardIdentity>> {
        self.attested_command_identity(name)
            .map(|(_, identities)| identities)
    }

    pub(super) fn attested_command_identity(
        &self,
        name: &[u8],
    ) -> Option<(u64, std::collections::BTreeSet<GuardIdentity>)> {
        let (command, generation) =
            self.resolve_dispatchable_with_generation(self.current_ns.get(), name)?;
        let generation = generation?;
        if matches!(command, Command::Ensemble(_))
            && self.stock_ensemble_registry_identity(generation).is_some()
            && !self.stock_ensemble_implementation_is_current(generation)
        {
            return None;
        }
        let identities = self.guarded_commands.borrow().get(&generation).cloned()?;
        Some((generation, identities))
    }

    /// Select the actual public object-command registration recipe. C qualified
    /// names use the current namespace; Jim constructs a root CString key.
    /// Answers the installed token without relooking up a name.
    pub(crate) fn create_obj_command(&mut self, name: &[u8], command: ObjCommand) -> Option<u64> {
        self.create_obj_command_for_purpose(
            name,
            command,
            tcl_registry::native_command_creation::NativeObjectCommandCreationPurpose::LegacyPublicObjectCommand,
        )
    }

    /// Counted Engine creation retains its own receipt and callback purpose,
    /// independently of the measured legacy public C object-command call.
    pub(crate) fn create_counted_engine_obj_command(
        &mut self,
        name: &[u8],
        command: ObjCommand,
    ) -> Option<u64> {
        self.create_obj_command_for_purpose(
            name,
            command,
            tcl_registry::native_command_creation::NativeObjectCommandCreationPurpose::CountedEngineObjectCommand,
        )
    }

    fn create_obj_command_for_purpose(
        &mut self,
        name: &[u8],
        mut command: ObjCommand,
        purpose: tcl_registry::native_command_creation::NativeObjectCommandCreationPurpose,
    ) -> Option<u64> {
        use tcl_registry::native_command_creation::NativePublicCommandRegistrationRecipe;
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return None;
            }
        };
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_command_creation_protocol()
        else {
            self.refuse_host_command("public command registration has no selected native protocol");
            return None;
        };
        let (namespace, simple, report, original_key) =
            match protocol.public_registration_recipe(purpose) {
                Some(NativePublicCommandRegistrationRecipe::CNamespacePublication) => {
                    let selected = self
                        .namespaces_mut()
                        .command_c_api_publication_at(self.current_ns(), name);
                    let Some((namespace, simple)) = selected else {
                        self.refuse_host_command(
                            "C command publication has no selected original namespace",
                        );
                        return None;
                    };
                    (namespace, simple, None, None)
                }
                Some(NativePublicCommandRegistrationRecipe::JimRootCStringKey) => {
                    let Some(report) = protocol.public_jim_registration_key(purpose, name) else {
                        self.refuse_host_command(
                            "Jim public registration has no original CString key projection",
                        );
                        return None;
                    };
                    let original_key = self.generated_jim_command_key(
                        GLOBAL,
                        name,
                        tcl_syntax::naming::NativeNamePurpose::CommandCApiPublication,
                    );
                    if scope.currency().ensure_current_or_refuse().is_err() {
                        return None;
                    }
                    let original_key = match original_key {
                        Ok(Some(key)) => key,
                        Ok(None) => {
                            self.refuse_host_command(
                                "Jim public registration has no original key owner",
                            );
                            return None;
                        }
                        Err(error) => {
                            self.report_cmd_error(error.into());
                            return None;
                        }
                    };
                    (
                        GLOBAL,
                        report.comparison_bytes().to_vec(),
                        Some(report),
                        Some(original_key),
                    )
                }
                None => {
                    self.refuse_host_command(
                        "public command constructor purpose has no selected native recipe",
                    );
                    return None;
                }
            };
        // A caller-provided owned-shim purpose remains independently sealed
        // even when its record passes through a public constructor facade.
        if command.creation_purpose.is_none() {
            command.creation_purpose = Some(purpose);
        }
        let OriginalCommandPublication::Published(generation) = self
            .bind_command_replacement_with_original_jim_key(
                namespace,
                &simple,
                Command::ObjCmd(Rc::new(command)),
                report,
                original_key,
            )
        else {
            return None;
        };
        if scope.currency().ensure_current_or_refuse().is_err() {
            return None;
        }
        self.remember_original_byte_command_registration(name, namespace, &simple, generation);
        Some(generation)
    }

    /// Register `cmd` under the (possibly qualified) name `name` — for the OO
    /// object/class commands.
    pub(crate) fn ns_register(&mut self, name: &[u8], cmd: Command) {
        let (namespace, tail) = self
            .namespaces
            .borrow_mut()
            .support_command_publication_at(self.current_ns.get(), name);
        self.bind_command_replacement(namespace, &tail, cmd);
    }
}
