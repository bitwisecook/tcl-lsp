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

//! Actual engine-owned ensemble registrations and worker argument views.

use super::{
    native_operation_currency::NativeOperationScope, BuiltinFn, Code, Command, Interp, TclObj,
    GLOBAL,
};
use crate::ensemble::EnsembleConfig;
use std::collections::HashMap;
use tcl_registry::invocation_words::EnsembleImplementationFamily;

#[derive(Default)]
pub(super) struct StockEnsembles {
    families: HashMap<EnsembleImplementationFamily, InstalledFamily>,
}

pub(super) struct HandlerUsageAdapter {
    pub(super) birth: std::rc::Rc<()>,
    pub(super) parse_prefix: Vec<*mut TclObj>,
    pub(super) original_prefix: Vec<crate::obj::Owned>,
}

struct HandlerUsageScope {
    interpreter: Interp,
    previous_len: usize,
    currency: super::native_operation_currency::NativeOperationCurrency,
}

impl Drop for HandlerUsageScope {
    fn drop(&mut self) {
        let _ = self.currency.ensure_current_or_refuse();
        let retired = self
            .interpreter
            .handler_usage_adapters
            .borrow_mut()
            .split_off(self.previous_len);
        drop(retired);
        let _ = self.currency.ensure_current_or_refuse();
    }
}

#[derive(Clone)]
pub(super) struct StockRegistrationSlot {
    namespace: crate::namespace::NsId,
    simple: tcl_core_types::NameBytes,
    slot: tcl_core_types::NativeByteCommandSlot,
}

impl StockRegistrationSlot {
    fn capture(interpreter: &Interp, generation: u64) -> Option<Self> {
        let world = interpreter.namespaces();
        let (namespace, simple) = world.native_command_slot_at_node(generation)?;
        let path = world.native_context_path(namespace)?;
        Some(Self {
            namespace,
            simple: tcl_core_types::NameBytes::from(simple.clone()),
            slot: tcl_core_types::NativeByteCommandSlot {
                namespace: path,
                simple: tcl_core_types::NameBytes::from(simple),
            },
        })
    }
    fn same_physical_slot(&self, other: &Self) -> bool {
        self.namespace == other.namespace && self.simple == other.simple
    }
}

struct ScriptedHelperRegistration {
    declared: StockRegistrationSlot,
    generation: u64,
}

struct InstalledFamily {
    public: &'static [u8],
    generation: u64,
    config: Option<EnsembleConfig>,
    compiler: BuiltinFn,
    scripted: bool,
    scripted_helpers: Vec<ScriptedHelperRegistration>,
    members: Vec<(Vec<u8>, u64)>,
    namespaces: Vec<(Vec<u8>, crate::namespace::NsId)>,
    nested: Vec<(Vec<u8>, u64, EnsembleConfig, BuiltinFn)>,
}

/// Allocation/metadata-only current slots from actual published generations.
/// This does not retain a callable command or any native configured object.
struct OriginalStockSlots(Vec<(crate::namespace::NsId, Vec<u8>, u64)>);
impl OriginalStockSlots {
    fn capture(interpreter: &Interp, generations: impl IntoIterator<Item = u64>) -> Option<Self> {
        let namespaces = interpreter.namespaces();
        let mut slots = Vec::new();
        for generation in generations {
            let (namespace, tail) = namespaces.native_command_slot_at_node(generation)?;
            slots.push((namespace, tail, generation));
        }
        Some(Self(slots))
    }
    fn check(&self, interpreter: &mut Interp) -> bool {
        self.0.iter().all(|(namespace, tail, generation)| {
            interpreter.original_command_publication_is_current(*namespace, tail, *generation)
        })
    }
}

impl Interp {
    /// Adopt reached bookkeeping, retire its actual displaced configuration
    /// owners outside the map borrow, then check the original continuation.
    fn publish_stock_family(
        &mut self,
        family: EnsembleImplementationFamily,
        installed: InstalledFamily,
        scope: &NativeOperationScope,
        publication: Option<&OriginalStockSlots>,
    ) -> bool {
        // Bookkeeping settlement is mandatory, even if the original entry
        // already refuses. It does not authorize another installer.
        let admitted = match scope.currency().ensure_current_or_refuse() {
            Ok(()) => publication.is_none_or(|publication| publication.check(self)),
            Err(first) => {
                self.refuse_native_execution(first);
                false
            }
        };
        let retired = self
            .stock_ensembles
            .borrow_mut()
            .families
            .insert(family, installed);
        drop(retired);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            self.refuse_native_execution(first);
            return false;
        }
        admitted && publication.is_none_or(|publication| publication.check(self))
    }

    fn require_stock_publication(&mut self, publication: Option<&OriginalStockSlots>) -> bool {
        if let Some(publication) = publication {
            return publication.check(self);
        }
        self.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original stock family publication unavailable",
            ),
        );
        false
    }

    /// Failed refreshes preserve reached records, but cannot resume installation.
    fn restore_stock_families(
        &mut self,
        families: impl IntoIterator<Item = (EnsembleImplementationFamily, InstalledFamily)>,
        scope: &NativeOperationScope,
    ) {
        for (family, installed) in families {
            self.publish_stock_family(family, installed, scope, None);
        }
    }

    pub(crate) fn native_ensemble_profile_name(&self) -> &'static str {
        if self.dialect_profile().is_fallback() {
            self.runtime_version().dialect_profile_name()
        } else {
            self.dialect_profile().name
        }
    }

    /// Install an actual public token and admitted private worker callbacks.
    /// The immutable callback roster is authored by the handler implementation.
    pub(crate) fn register_stock_ensemble(
        &mut self,
        family: EnsembleImplementationFamily,
        public: &'static [u8],
        compiler: BuiltinFn,
        members: &[(&'static [u8], BuiltinFn)],
        admitted: &[&[u8]],
    ) {
        let scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return;
            }
        };
        let mut installed = InstalledFamily {
            public,
            generation: 0,
            config: None,
            compiler,
            scripted: false,
            scripted_helpers: Vec::new(),
            members: Vec::new(),
            namespaces: Vec::new(),
            nested: Vec::new(),
        };
        let generation = if let Some(namespace) = self
            .native_invocation_dialect()
            .ensemble_implementation_namespace(family)
        {
            let ns = self.ensure_stock_namespace(namespace.as_bytes(), &mut installed);
            let mut map = Vec::new();
            for &(member, callback) in members {
                if !admitted.contains(&member) {
                    continue;
                }
                let nested = (family == EnsembleImplementationFamily::Info)
                    .then(|| std::str::from_utf8(member).ok())
                    .flatten()
                    .and_then(|member| {
                        self.native_invocation_dialect()
                            .info_oo_ensemble_namespace(member)
                    });
                let mut target = nested.map_or_else(
                    || namespace.as_bytes().to_vec(),
                    |name| name.as_bytes().to_vec(),
                );
                if nested.is_none() {
                    target.extend_from_slice(b"::");
                    target.extend_from_slice(member);
                }
                let _ = self.register_stock_worker(&target, callback, &mut installed);
                if let Err(first) = scope.currency().ensure_current_or_refuse() {
                    self.refuse_native_execution(first);
                    return;
                }
                map.push((member.to_vec(), vec![target]));
            }
            for &member in self
                .native_invocation_dialect()
                .ensemble_bootstrap_deferred_members(family)
            {
                let mut target = namespace.as_bytes().to_vec();
                target.extend_from_slice(b"::");
                target.extend_from_slice(member);
                map.push((member.to_vec(), vec![target]));
            }
            let config = EnsembleConfig {
                originals: Default::default(),
                ns,
                map: Some(map),
                subcommands: None,
                prefixes: true,
                parameters: Vec::new(),
                unknown: Vec::new(),
            };
            let Some(generation) = self.create_ensemble(public, config.clone()) else {
                return;
            };
            installed.config = Some(config);
            generation
        } else {
            let generation = self.register_stock_builtin(public, compiler);
            if generation == 0 {
                return;
            }
            generation
        };
        installed.generation = generation;
        self.record_stock_compiler_registration(installed.generation, public);
        let publication = OriginalStockSlots::capture(self, [generation]);
        if !self.publish_stock_family(family, installed, &scope, publication.as_ref())
            || !self.require_stock_publication(publication.as_ref())
        {
            return;
        }
        if family == EnsembleImplementationFamily::String {
            self.attest_stock_string_implementation();
        }
    }

    /// Replace only the engine-owned root with the distribution's actual
    /// procedure and allocate its exact compound helper commands separately.
    pub(crate) fn install_stock_scripted_binary(
        &mut self,
        ingress: tcl_registry::native_binary_value::NativeBinaryScriptedIngress,
        callbacks: &[BuiltinFn],
    ) {
        let scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return;
            }
        };
        let family = EnsembleImplementationFamily::Binary;
        let Some(mut installed) = self.stock_ensembles.borrow_mut().families.remove(&family) else {
            return;
        };
        let reached = (|| {
            if self.resolve_cmd_token(installed.public) != Some(installed.generation)
                || !self.stock_ensemble_config_matches(&installed)
                || ingress.members.len() != callbacks.len()
            {
                return None;
            }
            let parameters = crate::cmd_proc::parse_params_in(self, ingress.parameters.as_bytes())
                .expect("audited distribution formal grammar");
            for (name, callback) in ingress.members.iter().zip(callbacks) {
                let token = self.register_stock_worker(name.as_bytes(), *callback, &mut installed);
                if let Err(first) = scope.currency().ensure_current_or_refuse() {
                    self.refuse_native_execution(first);
                    return None;
                }
                if let Some(token) = token {
                    // These helpers belong to the scripted distribution; no
                    // C primitive compiler identity is donated to them.
                    self.native_compilation.borrow_mut().stock.remove(&token);
                    let declared = StockRegistrationSlot::capture(self, token)?;
                    installed.scripted_helpers.push(ScriptedHelperRegistration {
                        declared,
                        generation: token,
                    });
                }
            }
            let body = crate::obj::Owned::fresh(super::new_string(ingress.body.as_bytes()));
            let reached = self.install_proc_original_storage_publication(
                ingress.name.as_bytes(),
                parameters,
                None,
                body.as_ptr(),
                None,
                None,
            );
            // This is the caller's actual temporary, independent of the Proc's
            // body role. Its cleanup cannot issue a different publication.
            drop(body);
            if let Err(first) = scope.currency().ensure_current_or_refuse() {
                self.refuse_native_execution(first);
                return None;
            }
            let publication = reached?;
            self.check_original_procedure_publication(
                publication.namespace,
                &publication.tail,
                publication.generation,
                &publication.binding,
            )
            .ok()?;
            installed.generation = publication.generation;
            installed.scripted = true;
            Some(publication)
        })();
        // Reached installer bookkeeping is mandatory even on refusal.
        if !self.publish_stock_family(family, installed, &scope, None) {
            return;
        }
        if let Some(publication) = reached {
            let _ = self.check_original_procedure_publication(
                publication.namespace,
                &publication.tail,
                publication.generation,
                &publication.binding,
            );
        }
    }

    /// Publish the actual selected distribution helper, preserving an existing slot.
    pub(crate) fn install_stock_jim_namespace_helpers(&mut self) {
        for kind in tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::ALL {
            self.install_stock_jim_namespace_helper(kind);
        }
    }

    fn install_stock_jim_namespace_helper(
        &mut self,
        kind: tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind,
    ) {
        let Some(helper) = self
            .native_invocation_dialect()
            .native_jim_namespace_helper(kind)
        else {
            return;
        };
        let scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return;
            }
        };
        let family = EnsembleImplementationFamily::Namespace;
        let Some(mut installed) = self.stock_ensembles.borrow_mut().families.remove(&family) else {
            return;
        };
        let reached = (|| {
            if self.resolve_cmd_token(installed.public) != Some(installed.generation)
                || !self.stock_ensemble_config_matches(&installed)
            {
                return None;
            }
            let existing = self
                .namespaces()
                .resolve_generation(GLOBAL, helper.name().as_bytes());
            let generation = if let Some(generation) = existing {
                generation
            } else {
                if self.current_ns() != GLOBAL {
                    self.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "Jim namespace helper global installation context",
                        )
                        .into(),
                    );
                    return None;
                }
                let parameters_obj =
                    crate::obj::Owned::fresh(super::new_string(helper.parameters().as_bytes()));
                let parameters = match crate::cmd_proc::parse_params_object(
                    self,
                    parameters_obj.as_ptr(),
                    b"",
                ) {
                    Ok(parameters) => parameters,
                    Err(error) => {
                        self.report_cmd_error(error);
                        return None;
                    }
                };
                let body = crate::obj::Owned::fresh(super::new_string(helper.body().as_bytes()));
                let reached = self.install_proc_original_storage_publication(
                    helper.name().as_bytes(),
                    parameters,
                    Some(parameters_obj.as_ptr()),
                    body.as_ptr(),
                    None,
                    None,
                );
                drop(body);
                drop(parameters_obj);
                if let Err(first) = scope.currency().ensure_current_or_refuse() {
                    self.refuse_native_execution(first);
                    return None;
                }
                let publication = reached?;
                self.check_original_procedure_publication(
                    publication.namespace,
                    &publication.tail,
                    publication.generation,
                    &publication.binding,
                )
                .ok()?;
                publication.generation
            };
            let declared = StockRegistrationSlot::capture(self, generation)?;
            let publication =
                OriginalStockSlots::capture(self, [installed.generation, generation])?;
            installed.scripted_helpers.push(ScriptedHelperRegistration {
                declared,
                generation,
            });
            Some(publication)
        })();
        // The public Namespace token remains its actual primitive allocation.
        let _ = self.publish_stock_family(family, installed, &scope, reached.as_ref());
    }

    /// Jim's public fallback obtains a fresh helper header and performs real lookup.
    pub(crate) fn dispatch_stock_jim_namespace_helper(
        &mut self,
        argv: &[*mut TclObj],
        helper: tcl_registry::native_jim_lookup::NativeJimNamespaceHelper,
    ) -> Code {
        if self
            .native_invocation_dialect()
            .native_jim_namespace_helper(helper.kind())
            != Some(helper)
        {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "Jim namespace helper issuer",
                )
                .into(),
            );
        }
        if !helper.accepts_public_arguments(argv.len().saturating_sub(2)) {
            return self.wrong_args_for_prefix(argv, 2, helper.usage().as_bytes());
        }
        let arguments = match super::native_arguments::OriginalArguments::enter(self, argv) {
            Ok(arguments) => arguments,
            Err(error) => return self.report_cmd_error(error),
        };
        let code = (|| {
            let context = match self.native_jim_object_context() {
                Ok(context) => context,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let head = crate::obj::Owned::fresh(super::new_string(helper.name().as_bytes()));
            if let Err(error) = crate::native_source::bind_context(head.as_ptr(), &context) {
                return self.report_cmd_error(error.into());
            }
            let mut words = vec![head.as_ptr()];
            words.extend_from_slice(&argv[2..]);
            self.dispatch(&words)
        })();
        arguments.finish(code)
    }

    fn ensure_stock_namespace(
        &self,
        name: &[u8],
        installed: &mut InstalledFamily,
    ) -> crate::namespace::NsId {
        let mut namespaces = self.namespaces.borrow_mut();
        let existed = namespaces.find_namespace(GLOBAL, name);
        let ns = namespaces.ensure_namespace(GLOBAL, name);
        if existed.is_none() {
            installed.namespaces.push((name.to_vec(), ns));
        }
        ns
    }

    fn register_stock_worker(
        &mut self,
        name: &[u8],
        callback: BuiltinFn,
        installed: &mut InstalledFamily,
    ) -> Option<u64> {
        let scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return None;
            }
        };
        // A host or script may already own the private spelling when a host
        // changes release. Its live handler remains the map's actual target.
        if self.resolve_cmd_token(name).is_some() {
            return None;
        }
        let publication = self.register_stock_builtin_publication(name, callback);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            self.refuse_native_execution(first);
            return None;
        }
        let publication = publication?;
        if !self.original_command_publication_is_current(
            publication.namespace,
            &publication.tail,
            publication.generation,
        ) {
            return None;
        }
        let generation = publication.generation;
        self.record_stock_compiler_registration(generation, name);
        installed.members.push((name.to_vec(), generation));
        Some(generation)
    }

    /// Replace a just-installed codec-root worker with its real nested ensemble.
    /// A preexisting host-owned token is preserved rather than overwritten.
    pub(crate) fn register_stock_nested_ensemble(
        &mut self,
        family: EnsembleImplementationFamily,
        name: &[u8],
        callback: BuiltinFn,
        members: &[(&[u8], BuiltinFn)],
    ) {
        self.register_stock_nested_ensemble_with_prefixes(family, name, callback, members, false);
    }

    pub(crate) fn register_stock_nested_ensemble_with_prefixes(
        &mut self,
        family: EnsembleImplementationFamily,
        name: &[u8],
        callback: BuiltinFn,
        members: &[(&[u8], BuiltinFn)],
        prefixes: bool,
    ) {
        let scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return;
            }
        };
        let Some(mut installed) = self.stock_ensembles.borrow_mut().families.remove(&family) else {
            return;
        };
        let owned = installed.members.iter().position(|(slot, generation)| {
            slot == name && self.resolve_cmd_token(slot) == Some(*generation)
        });
        let mut publication = None;
        if let Some(index) = owned {
            let ns = self.ensure_stock_namespace(name, &mut installed);
            let mut map = Vec::new();
            for &(member, worker) in members {
                let mut target = name.to_vec();
                target.extend_from_slice(b"::");
                target.extend_from_slice(member);
                let _ = self.register_stock_worker(&target, worker, &mut installed);
                if let Err(first) = scope.currency().ensure_current_or_refuse() {
                    self.refuse_native_execution(first);
                    self.publish_stock_family(family, installed, &scope, None);
                    return;
                }
                map.push((member.to_vec(), vec![target]));
            }
            let config = EnsembleConfig {
                originals: Default::default(),
                ns,
                map: Some(map),
                subcommands: None,
                prefixes,
                parameters: Vec::new(),
                unknown: Vec::new(),
            };
            let Some(generation) = self.create_ensemble(name, config.clone()) else {
                // Keep reached installer records, including the old exact worker
                // observation. Refusal cannot label a callback-created slot.
                self.publish_stock_family(family, installed, &scope, None);
                return;
            };
            installed.members.remove(index);
            self.record_stock_compiler_registration(generation, name);
            installed.members.push((name.to_vec(), generation));
            installed
                .nested
                .push((name.to_vec(), generation, config, callback));
            publication = OriginalStockSlots::capture(self, [installed.generation, generation]);
        }
        if self.publish_stock_family(family, installed, &scope, publication.as_ref())
            && owned.is_some()
        {
            self.require_stock_publication(publication.as_ref());
        }
    }

    pub(super) fn refresh_stock_ensembles(&mut self) {
        let scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return;
            }
        };
        let families = std::mem::take(&mut self.stock_ensembles.borrow_mut().families);
        let mut remaining = families.into_iter();
        while let Some((family, installed)) = remaining.next() {
            if self.resolve_cmd_token(installed.public) != Some(installed.generation)
                || !self.stock_ensemble_config_matches(&installed)
                || installed.nested.iter().any(|(_, token, config, _)| {
                    !matches!(self.command_by_generation(*token),
                        super::CommandGenerationLookup::Found { command: Command::Ensemble(ensemble), .. }
                            if ensemble.config() == *config)
                })
            {
                if !self.publish_stock_family(family, installed, &scope, None) {
                    self.restore_stock_families(remaining, &scope);
                    return;
                }
                continue;
            }
            let mut refused = false;
            for (name, generation) in &installed.members {
                if self.resolve_cmd_token(name) == Some(*generation) {
                    self.delete_command(name);
                    if let Err(first) = scope.currency().ensure_current_or_refuse() {
                        self.refuse_native_execution(first);
                        refused = true;
                        break;
                    }
                }
            }
            if !refused {
                for (name, ns) in installed.namespaces.iter().rev() {
                    let empty = {
                        let namespaces = self.namespaces.borrow();
                        namespaces.find_namespace(GLOBAL, name) == Some(*ns)
                            && namespaces.command_names(*ns).is_empty()
                            && namespaces.var_names(*ns).is_empty()
                            && namespaces.children(*ns).is_empty()
                    };
                    if empty {
                        self.delete_namespace_by_id(*ns);
                        if let Err(first) = scope.currency().ensure_current_or_refuse() {
                            self.refuse_native_execution(first);
                            refused = true;
                            break;
                        }
                    }
                }
            }
            if refused {
                self.restore_stock_families(
                    std::iter::once((family, installed)).chain(remaining),
                    &scope,
                );
                return;
            }
            match family {
                EnsembleImplementationFamily::Clock => crate::cmd_clock::install(self),
                EnsembleImplementationFamily::Info => crate::cmd_info::install(self),
                EnsembleImplementationFamily::File => crate::cmd_fs::install_file(self),
                EnsembleImplementationFamily::Array => crate::cmd_array::install(self),
                EnsembleImplementationFamily::Namespace => crate::cmd_namespace::install(self),
                EnsembleImplementationFamily::Binary => crate::cmd_binary::install(self),
                EnsembleImplementationFamily::String => crate::cmd_string::install_ensemble(self),
                EnsembleImplementationFamily::Dict => crate::cmd_dict::install(self),
            }
            // This is the old extracted record's genuine final configuration
            // role, outside every stock/namespace/frame borrow. Capture the new
            // actual publication before it retires; no post-callback lookup.
            let publication = {
                let registrations = self.stock_ensembles.borrow();
                registrations.families.get(&family).and_then(|current| {
                    OriginalStockSlots::capture(
                        self,
                        std::iter::once(current.generation).chain(
                            current
                                .nested
                                .iter()
                                .map(|(_, generation, _, _)| *generation),
                        ),
                    )
                })
            };
            let admitted = self.require_stock_publication(publication.as_ref());
            drop(installed);
            if let Err(first) = scope.currency().ensure_current_or_refuse() {
                self.refuse_native_execution(first);
                self.restore_stock_families(remaining, &scope);
                return;
            }
            if !admitted || !self.require_stock_publication(publication.as_ref()) {
                self.restore_stock_families(remaining, &scope);
                return;
            }
        }
        self.attest_stock_string_implementation();
    }

    pub(super) fn attest_stock_string_implementation(&self) {
        let registrations = self.stock_ensembles.borrow();
        let Some(installed) = registrations
            .families
            .get(&EnsembleImplementationFamily::String)
        else {
            return;
        };
        if self.resolve_cmd_token(installed.public) == Some(installed.generation)
            && self.stock_ensemble_config_matches(installed)
            && installed
                .members
                .iter()
                .all(|(name, generation)| self.resolve_cmd_token(name) == Some(*generation))
        {
            let registry = crate::environment::store_for_profile(self.dialect_profile());
            if let Some(spec) = registry.get_exact("string") {
                self.retire_guarded_command_identities();
                self.attest_spec_at(spec, installed.generation);
            }
        }
    }

    fn stock_ensemble_config_matches(&self, installed: &InstalledFamily) -> bool {
        let command = match self.command_by_generation(installed.generation) {
            super::CommandGenerationLookup::Found { command, .. } => Some(command),
            _ => None,
        };
        match (&installed.config, command) {
            (Some(config), Some(Command::Ensemble(token))) => token.config() == *config,
            (None, Some(Command::Builtin(callback))) => {
                std::ptr::fn_addr_eq(callback, installed.compiler)
            }
            (None, Some(Command::Proc(_))) => installed.scripted,
            _ => false,
        }
    }

    /// Installed worker generations required by unchanged stock roots. A new
    /// command at a former worker address never inherits the original receipt.
    pub(crate) fn stock_implementation_generations(&self, roots: &[u64]) -> Vec<u64> {
        let mut retained = roots.to_vec();
        let registrations = self.stock_ensembles.borrow();
        let stock = self.native_compilation.borrow();
        let registry = crate::environment::store_for_profile(self.dialect_profile());
        let mut cursor = 0;
        while cursor < retained.len() {
            let root = retained[cursor];
            cursor += 1;
            if self
                .namespaces()
                .native_command_slot_at_node(root)
                .is_none()
            {
                continue;
            }
            let expression = stock.stock.get(&root).is_some_and(|identity| {
                matches!(self.raw_command_by_generation(root), Some(Command::Builtin(_)))
                    && std::str::from_utf8(identity).ok().is_some_and(|identity| {
                        registry.native_compilation_for_registration(
                            identity, self.native_invocation_dialect(),
                        ).is_some_and(|spec| {
                            spec.grammar == tcl_registry::native_compilation::NativeCompilationGrammar::Expression
                        })
                    })
            });
            if expression {
                for (&generation, identity) in &stock.stock {
                    if matches!(
                        self.raw_command_by_generation(generation),
                        Some(Command::Builtin(_))
                    ) && std::str::from_utf8(identity)
                        .ok()
                        .is_some_and(tcl_registry::mathfunc::restricted_expression_command_identity)
                        && self
                            .namespaces()
                            .native_command_slot_at_node(generation)
                            .is_some()
                        && !retained.contains(&generation)
                    {
                        retained.push(generation);
                    }
                }
            }
            for installed in registrations.families.values() {
                if installed.generation != root || !self.stock_ensemble_config_matches(installed) {
                    continue;
                }
                for generation in installed
                    .members
                    .iter()
                    .map(|(_, generation)| *generation)
                    .chain(
                        installed
                            .scripted_helpers
                            .iter()
                            .map(|helper| helper.generation),
                    )
                {
                    if self
                        .namespaces()
                        .native_command_slot_at_node(generation)
                        .is_some()
                        && !retained.contains(&generation)
                    {
                        retained.push(generation);
                    }
                }
            }
            // A nested stock ensemble can itself be an explicitly retained root.
            for installed in registrations.families.values() {
                for (_, generation, config, _) in &installed.nested {
                    if *generation != root
                        || !matches!(self.command_by_generation(root),
                        super::CommandGenerationLookup::Found { command: Command::Ensemble(ensemble), .. }
                            if ensemble.config() == *config)
                    {
                        continue;
                    }
                    for (_, prefix) in config.map.iter().flatten() {
                        let Some(head) = prefix.first() else { continue };
                        let Some(selected) = self.namespaces().resolve_generation(config.ns, head)
                        else {
                            continue;
                        };
                        if installed
                            .members
                            .iter()
                            .any(|(_, token)| *token == selected)
                            && !retained.contains(&selected)
                        {
                            retained.push(selected);
                        }
                    }
                }
            }
        }
        retained
    }

    /// Actual distribution helper allocations retain their own runtime
    /// admission, independently of the primitive catalogue's fresh roster.
    pub(super) fn is_stock_scripted_worker(&self, generation: u64) -> bool {
        self.stock_ensembles
            .borrow()
            .families
            .values()
            .any(|installed| {
                installed
                    .scripted_helpers
                    .iter()
                    .any(|helper| helper.generation == generation)
            })
    }

    /// Runtime registration replaces an actual declared compound slot; retain
    /// its new allocation's admission without giving it a stock compiler ID.
    pub(super) fn record_scripted_helper_registration(&self, generation: u64) {
        let Some(current) = StockRegistrationSlot::capture(self, generation) else {
            return;
        };
        for installed in self.stock_ensembles.borrow_mut().families.values_mut() {
            if installed
                .scripted_helpers
                .iter()
                .any(|helper| helper.declared.same_physical_slot(&current))
            {
                installed.scripted_helpers.push(ScriptedHelperRegistration {
                    declared: current.clone(),
                    generation,
                });
            }
        }
    }

    pub(super) fn remember_stock_registration(&self, generation: u64) -> bool {
        let Some(current) = StockRegistrationSlot::capture(self, generation) else {
            return false;
        };
        self.native_compilation
            .borrow_mut()
            .stock_registrations
            .entry(generation)
            .or_insert(current);
        true
    }

    pub(super) fn record_stock_compiler_registration(
        &self,
        generation: u64,
        identity: &[u8],
    ) -> bool {
        if !self.remember_stock_registration(generation) {
            return false;
        }
        self.native_compilation
            .borrow_mut()
            .stock
            .insert(generation, identity.to_vec());
        true
    }

    fn stock_registration_matches(&self, generation: u64, authored: &[u8]) -> bool {
        let Some(protocol) = self.name_policy_protocol() else {
            return false;
        };
        let Ok(slot) = protocol
            .recipe()
            .command_lookup_slot(tcl_syntax::naming::NativeNameContext::root(), authored)
        else {
            return false;
        };
        let state = self.native_compilation.borrow();
        state.stock.contains_key(&generation)
            && state
                .stock_registrations
                .get(&generation)
                .is_some_and(|registered| registered.slot == slot)
    }

    /// Original stock root identity follows its allocation, including a rename.
    /// Scripted distribution roots have their independent selected admission.
    pub(super) fn stock_ensemble_registry_identity(&self, generation: u64) -> Option<Vec<u8>> {
        self.stock_ensembles
            .borrow()
            .families
            .values()
            .find_map(|installed| {
                if installed.generation == generation && !installed.scripted {
                    Some(installed.public.to_vec())
                } else {
                    installed.nested.iter().find_map(|(name, token, _, _)| {
                        (*token == generation).then(|| name.clone())
                    })
                }
            })
    }

    /// Writable maps and private worker replacements cannot retain a semantic
    /// attestation merely because the public root token still exists.
    pub(super) fn stock_ensemble_implementation_is_current(&self, generation: u64) -> bool {
        let registrations = self.stock_ensembles.borrow();
        registrations.families.values().any(|installed| {
            if installed.generation == generation {
                !installed.scripted
                    && self.stock_ensemble_config_matches(installed)
                    && installed.members.iter().all(|(name, token)| {
                        self.namespaces().resolve_generation(GLOBAL, name) == Some(*token)
                    })
                    && installed.nested.iter().all(|(_, token, config, _)| {
                        matches!(self.raw_command_by_generation(*token),
                            Some(Command::Ensemble(ensemble)) if ensemble.config() == *config)
                    })
            } else {
                installed.nested.iter().any(|(_, token, config, _)| {
                    *token == generation
                        && installed.members.iter().all(|(name, worker)| {
                            self.namespaces().resolve_generation(GLOBAL, name) == Some(*worker)
                        })
                        && matches!(self.raw_command_by_generation(*token),
                            Some(Command::Ensemble(ensemble)) if ensemble.config() == *config)
                })
            }
        })
    }

    /// Compiler identity comes from the installed token and unchanged real map.
    pub(super) fn stock_ensemble_compiler(&self, generation: u64) -> Option<BuiltinFn> {
        let registrations = self.stock_ensembles.borrow();
        for installed in registrations.families.values() {
            if installed.generation == generation {
                if installed.scripted {
                    return None;
                }
                return self
                    .stock_ensemble_config_matches(installed)
                    .then_some(installed.compiler);
            }
            for (_, token, config, callback) in &installed.nested {
                if *token == generation {
                    return match self.command_by_generation(*token) {
                        super::CommandGenerationLookup::Found {
                            command: Command::Ensemble(ensemble),
                            ..
                        } if ensemble.config() == *config => Some(*callback),
                        _ => None,
                    };
                }
            }
        }
        None
    }

    /// Actual installed ensemble CPP and its current mutable raw operands.
    /// A stock roster alone cannot attest a replaced public command token.
    pub(crate) fn native_ensemble_compiler_configuration(
        &self,
        generation: u64,
    ) -> Option<tcl_runtime_api::native_compilation::NativeCommandCompiler> {
        use tcl_runtime_api::native_compilation::{NativeCommandCompiler, NativeEnsembleCompiler};
        let dialect = self.native_invocation_dialect();
        dialect.native_ensemble_compiler_attachment_protocol()?;
        if self.namespaces.borrow().native_compiler_recipe(generation).is_some_and(|recipe| {
            matches!(recipe, crate::namespace::NativeCompilerRecipe::Registered { spec, .. } if spec.is_named_invocation_compiler())
        }) { return None; }
        let identity = {
            let registrations = self.stock_ensembles.borrow();
            registrations.families.values().find_map(|installed| {
                if installed.generation == generation && !installed.scripted {
                    Some(installed.public.to_vec())
                } else {
                    installed
                        .nested
                        .iter()
                        .find(|(_, token, _, _)| *token == generation)
                        .map(|(name, _, _, _)| name.clone())
                }
            })?
        };
        let Some(Command::Ensemble(ensemble)) = self.raw_command_by_generation(generation) else {
            return None;
        };
        let config = ensemble.config();
        let map = config
            .map?
            .into_iter()
            .map(|(member, prefix)| {
                (
                    tcl_runtime_api::NameBytes::from(member),
                    prefix
                        .into_iter()
                        .map(|word| Some(tcl_runtime_api::NameBytes::from(word)))
                        .collect(),
                )
            })
            .collect();
        Some(NativeCommandCompiler {
            registry_identity: String::from_utf8(identity).ok()?,
            ensemble: Some(NativeEnsembleCompiler {
                namespace_token: config.ns as u64,
                map,
                subcommands: config.subcommands.map(|members| {
                    members
                        .into_iter()
                        .map(tcl_runtime_api::NameBytes::from)
                        .collect()
                }),
                prefixes: config.prefixes,
                parameters: config
                    .parameters
                    .into_iter()
                    .map(tcl_runtime_api::NameBytes::from)
                    .collect(),
                unknown_handler: (!config.unknown.is_empty()).then(|| {
                    config
                        .unknown
                        .into_iter()
                        .map(|word| Some(tcl_runtime_api::NameBytes::from(word)))
                        .collect()
                }),
            }),
        })
    }

    /// Retain the exact raw command lookup node without a reporting-name lookup.
    pub(crate) fn native_compilation_binding_at(
        &self,
        namespace: crate::namespace::NsId,
        bytes: &[u8],
    ) -> Result<
        Option<tcl_runtime_api::native_compilation::NativeCompilationBinding>,
        tcl_syntax::value::ValueError,
    > {
        let Some(cache) = self.native_command_name_from_binding(namespace, bytes)? else {
            return Ok(None);
        };
        Ok(Some(self.native_compilation_binding_from_cache(cache)))
    }

    /// Resolve the same original mapping object and retain its selected node.
    pub(crate) fn native_compilation_binding_from_original(
        &mut self,
        namespace: crate::namespace::NsId,
        original: *mut crate::obj::TclObj,
    ) -> Result<
        Option<tcl_runtime_api::native_compilation::NativeCompilationBinding>,
        tcl_syntax::value::ValueError,
    > {
        if !matches!(
            crate::obj::native_object_snapshot(original)?.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
                | tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
                | tcl_syntax::native_object::NativeObjectCacheSnapshot::CommandName { .. }
        ) {
            return Ok(None);
        }
        let Some((_, token)) = self.resolve_original_command_at(namespace, original)? else {
            return Ok(None);
        };
        let Some(cache) = crate::obj::native_command_name_cache(original) else {
            return Ok(None);
        };
        if token != Some(cache.token) {
            return Ok(None);
        }
        Ok(Some(self.native_compilation_binding_from_cache(cache)))
    }

    fn native_compilation_binding_from_cache(
        &self,
        cache: tcl_runtime_api::native_command_name::NativeCommandNameCache,
    ) -> tcl_runtime_api::native_compilation::NativeCompilationBinding {
        use tcl_runtime_api::native_compilation::{
            NativeCommandCompiler, NativeCommandImplementation, NativeCompilationBinding,
            NativeCompilerHookPresence,
        };
        let ensemble = self.native_ensemble_compiler_configuration(cache.token);
        let world = self.namespaces.borrow();
        let compiler =
            ensemble
                .clone()
                .or_else(|| match world.native_compiler_recipe(cache.token) {
                    Some(crate::namespace::NativeCompilerRecipe::Registered {
                        registration,
                        ..
                    }) => String::from_utf8(registration)
                        .ok()
                        .map(|registry_identity| NativeCommandCompiler {
                            registry_identity,
                            ensemble: None,
                        }),
                    _ => None,
                });
        NativeCompilationBinding {
            slot: cache.slot,
            namespace_token: cache.namespace_token,
            token: cache.token,
            implementation_generation: cache.implementation_generation,
            implementation: NativeCommandImplementation::Opaque,
            compiler_hook: if ensemble.is_some() {
                NativeCompilerHookPresence::Present
            } else {
                world
                    .native_compiler_hook(cache.token)
                    .unwrap_or(NativeCompilerHookPresence::Unknown)
            },
            compiler,
            procedure_header: None,
            has_execution_trace: !self.traces.borrow().step_active.is_empty()
                || self.traces.borrow().cmd_traces.iter().any(|trace| {
                    trace.token == Some(cache.token)
                        && trace.ops & crate::cmd_trace::ops::EXEC_ANY != 0
                }),
        }
    }

    /// Close compiler-owned path prerequisites against real tokens and maps.
    /// Direct worker compilation needs its own registration, independently of
    /// a public ensemble's current configuration.
    pub(super) fn native_implementation_path_holds(
        &self,
        generation: u64,
        path: &[tcl_registry::native_compilation::NativeCompilerImplementationLookup],
    ) -> bool {
        if !matches!(
            self.command_by_generation(generation),
            super::CommandGenerationLookup::Found { .. }
        ) {
            return false;
        }
        if path
            .last()
            .is_some_and(|edge| self.stock_registration_matches(generation, edge.slot.as_bytes()))
        {
            return true;
        }
        let mut current = generation;
        for (index, edge) in path.iter().enumerate() {
            if index == 0 && !self.stock_registration_matches(current, edge.ensemble.as_bytes()) {
                let Ok(Some(binding)) =
                    self.native_compilation_binding_at(GLOBAL, edge.ensemble.as_bytes())
                else {
                    return false;
                };
                current = binding.token;
            }
            let super::CommandGenerationLookup::Found {
                command: Command::Ensemble(ensemble),
                ..
            } = self.command_by_generation(current)
            else {
                return false;
            };
            let config = ensemble.config();
            if !config.parameters.is_empty() || !config.unknown.is_empty() {
                return false;
            }
            let Some(prefix) = config.map.as_ref().and_then(|map| {
                map.iter()
                    .find(|(member, _)| member == edge.member.as_bytes())
                    .map(|(_, prefix)| prefix)
            }) else {
                return false;
            };
            if prefix.len() != 1 {
                return false;
            }
            let Ok(Some(mapped)) = self.native_compilation_binding_at(config.ns, &prefix[0]) else {
                return false;
            };
            let Ok(Some(required)) =
                self.native_compilation_binding_at(GLOBAL, edge.slot.as_bytes())
            else {
                return false;
            };
            if mapped.token != required.token
                || mapped.namespace_token != required.namespace_token
                || mapped.implementation_generation != required.implementation_generation
                || !self.stock_registration_matches(required.token, edge.slot.as_bytes())
            {
                return false;
            }
            current = required.token;
        }
        true
    }

    /// Adapt a frozen private worker invocation to the existing handler parser.
    /// Dispatch, evaluation frames and actual argv remain owned by the caller;
    /// only this handler's parse view and usage prefix are projected.
    pub(crate) fn invoke_stock_worker(
        &mut self,
        arguments: &[*mut TclObj],
        prefix: &[&[u8]],
        callback: BuiltinFn,
    ) -> Code {
        let originals = match super::native_arguments::OriginalArguments::enter(self, arguments) {
            Ok(originals) => originals,
            Err(error) => return self.report_cmd_error(error),
        };
        let code = (|| {
            let parse_prefix: Vec<_> = prefix
                .iter()
                .map(|word| crate::obj::Owned::fresh(super::new_string(word)))
                .collect();
            let mut view: Vec<_> = parse_prefix.iter().map(crate::obj::Owned::as_ptr).collect();
            view.extend_from_slice(&arguments[1..]);
            let previous_len = self.handler_usage_adapters.borrow().len();
            self.handler_usage_adapters
                .borrow_mut()
                .push(HandlerUsageAdapter {
                    birth: std::rc::Rc::new(()),
                    parse_prefix: view[..prefix.len()].to_vec(),
                    original_prefix: arguments
                        .first()
                        .map(|word| crate::obj::Owned::retain(*word))
                        .into_iter()
                        .collect(),
                });
            let _usage = HandlerUsageScope {
                interpreter: self.clone(),
                previous_len,
                currency: originals.currency().clone(),
            };
            callback(self, &view)
        })();
        originals.finish(code)
    }
}

#[cfg(test)]
mod tests {
    use super::super::obj_bytes;
    use super::*;

    fn invoke(interpreter: &mut Interp, words: &[&[u8]]) -> (Code, Vec<u8>) {
        let argv: Vec<_> = words
            .iter()
            .map(|word| super::super::new_string(word))
            .collect();
        for &word in &argv {
            // SAFETY: the invocation retains each newly allocated argument.
            unsafe { crate::obj::incr_ref_count(word) };
        }
        let code = interpreter.dispatch(&argv);
        let result = obj_bytes(interpreter.get_obj_result());
        super::super::release_all(&argv);
        (code, result)
    }

    #[test]
    fn actual_private_families_follow_native_release_floors() {
        for (version, info, modern) in [
            (tcl_dialect::TclVersion::V8_4, false, false),
            (tcl_dialect::TclVersion::V8_5, true, false),
            (tcl_dialect::TclVersion::V8_6, true, true),
            (tcl_dialect::TclVersion::V9_0, true, true),
            (tcl_dialect::TclVersion::V9_1, true, true),
        ] {
            let mut interpreter = Interp::new();
            interpreter.set_runtime_version(version);
            assert_eq!(
                interpreter
                    .resolve_cmd_token(b"::tcl::info::exists")
                    .is_some(),
                info
            );
            for name in [
                b"::tcl::array::exists".as_slice(),
                b"::tcl::namespace::current",
                b"::tcl::binary::encode::hex",
            ] {
                assert_eq!(
                    interpreter.resolve_cmd_token(name).is_some(),
                    modern,
                    "{version:?}: {name:?}"
                );
            }
            if modern {
                assert!(interpreter.is_ensemble(b"binary"));
                assert!(interpreter.is_ensemble(b"::tcl::binary::encode"));
                assert_eq!(
                    invoke(&mut interpreter, &[b"binary", b"encode", b"hex", b"A"]),
                    (Code::Ok, b"41".to_vec())
                );
                assert_eq!(
                    invoke(&mut interpreter, &[b"::tcl::binary::encode::hex", b"B"]),
                    (Code::Ok, b"42".to_vec())
                );
                let (code, message) =
                    invoke(&mut interpreter, &[b"::tcl::binary::encode", b"h", b"A"]);
                assert_eq!(code, Code::Error);
                assert!(message.starts_with(b"unknown subcommand \"h\""));
            }
        }
        let mut interpreter = Interp::new();
        interpreter.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        assert!(!interpreter.is_ensemble(b"binary"));
        assert!(interpreter
            .resolve_cmd_token(b"::tcl::binary::encode::hex")
            .is_none());
        assert!(interpreter
            .resolve_cmd_token(b"::tcl::info::exists")
            .is_none());
    }

    #[test]
    fn actual_maps_and_redefined_private_workers_control_runtime_dispatch() {
        fn replacement(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
            interpreter.set_result_bytes(b"REPLACED");
            Code::Ok
        }
        let mut interpreter = Interp::new();
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        interpreter.register_builtin(b"::tcl::binary::encode::hex", replacement);
        assert_eq!(
            invoke(&mut interpreter, &[b"binary", b"encode", b"hex", b"A"]),
            (Code::Ok, b"REPLACED".to_vec())
        );
        let Command::Ensemble(token) = interpreter
            .namespaces
            .borrow()
            .resolve(GLOBAL, b"::tcl::binary::encode")
            .unwrap()
        else {
            panic!("actual codec ensemble");
        };
        // The public configuration owner retains the replacement native map
        // and invalidates its selected prefix table on the same command token.
        assert_eq!(
            interpreter.eval_str(b"namespace ensemble configure ::tcl::binary::encode -map {hex ::tcl::binary::encode::base64}"),
            Code::Ok
        );
        assert_eq!(
            token.config().map,
            Some(vec![(
                b"hex".to_vec(),
                vec![b"::tcl::binary::encode::base64".to_vec()],
            )])
        );
        assert_eq!(
            invoke(&mut interpreter, &[b"binary", b"encode", b"hex", b"A"]),
            (Code::Ok, b"QQ==".to_vec())
        );
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        assert_eq!(
            invoke(&mut interpreter, &[b"::tcl::binary::encode::hex", b"A"]),
            (Code::Ok, b"REPLACED".to_vec())
        );
    }

    #[test]
    fn jim_binary_uses_actual_procedure_and_mutable_compound_helpers() {
        let mut interpreter = Interp::new();
        interpreter.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        assert!(matches!(
            interpreter.namespaces.borrow().resolve(GLOBAL, b"binary"),
            Some(Command::Proc(_))
        ));
        assert_eq!(
            invoke(&mut interpreter, &[b"binary", b"format", b"H*", b"41"]),
            (Code::Ok, b"A".to_vec())
        );
        assert_eq!(interpreter.eval_str(b"rename binary saved; proc {binary format} {args} {return CHANGED}; saved format H* 41"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"CHANGED");
        assert_eq!(interpreter.eval_str(b"saved f H* 41"), Code::Error);
        assert_eq!(
            interpreter.result_bytes(),
            b"invalid command name \"binary f\""
        );
        assert_eq!(interpreter.eval_str(b"saved encode hex A"), Code::Error);
        assert_eq!(
            interpreter.result_bytes(),
            b"invalid command name \"binary encode\""
        );
    }

    #[test]
    fn jim_scripted_binary_preserves_host_helpers_and_later_replacements() {
        fn replacement(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
            interpreter.set_result_bytes(b"HOST");
            Code::Ok
        }
        let mut interpreter = Interp::new();
        interpreter.register_builtin(b"binary format", replacement);
        interpreter.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        assert_eq!(
            invoke(&mut interpreter, &[b"binary", b"format", b"H*", b"41"]),
            (Code::Ok, b"HOST".to_vec())
        );
        interpreter.register_builtin(b"binary scan", replacement);
        assert_eq!(
            invoke(&mut interpreter, &[b"binary", b"scan", b"A", b"H*", b"h"]),
            (Code::Ok, b"HOST".to_vec())
        );
    }

    #[test]
    fn worker_usage_retains_public_and_private_invocation_headers() {
        for version in [tcl_dialect::TclVersion::V8_6, tcl_dialect::TclVersion::V9_0] {
            let mut interpreter = Interp::new();
            interpreter.set_runtime_version(version);
            assert_eq!(
                invoke(&mut interpreter, &[b"binary", b"encode", b"hex"]),
                (
                    Code::Error,
                    b"wrong # args: should be \"binary encode hex data\"".to_vec()
                )
            );
            assert_eq!(
                invoke(&mut interpreter, &[b"::tcl::binary::encode::hex"]),
                (
                    Code::Error,
                    b"wrong # args: should be \"::tcl::binary::encode::hex data\"".to_vec()
                )
            );
            assert_eq!(
                invoke(&mut interpreter, &[b"::tcl::namespace::current", b"extra"]),
                (
                    Code::Error,
                    b"wrong # args: should be \"::tcl::namespace::current\"".to_vec()
                )
            );
        }
    }

    #[test]
    fn parser_usage_adapter_preserves_callback_reset_and_original_header_identity() {
        fn inspect(interpreter: &mut Interp, arguments: &[*mut TclObj]) -> Code {
            assert_eq!(
                interpreter.argument_usage_prefix(arguments, 1).unwrap(),
                b"parser"
            );
            let before = interpreter.argument_usage_prefix(arguments, 2).unwrap();
            assert_eq!(interpreter.eval_str(b"set scratch 1"), Code::Ok);
            assert!(interpreter.ensemble_rewrite().is_none());
            let after = interpreter.argument_usage_prefix(arguments, 2).unwrap();
            let callback = crate::obj::Owned::fresh(super::super::new_string(b"set"));
            let callback_header = interpreter
                .argument_usage_prefix(&[callback.as_ptr()], 1)
                .unwrap();
            let mut result = before;
            result.push(b'\n');
            result.extend_from_slice(&after);
            result.push(b'\n');
            result.extend_from_slice(&callback_header);
            interpreter.set_result_bytes(&result);
            Code::Ok
        }
        let mut interpreter = Interp::new();
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        interpreter.begin_ensemble_rewrite(
            vec![
                crate::obj::Owned::fresh(super::super::new_string(b"public")),
                crate::obj::Owned::fresh(super::super::new_string(b"member")),
            ],
            2,
            1,
        );
        let original = crate::obj::Owned::fresh(super::super::new_string(b"private"));
        assert_eq!(
            interpreter.invoke_stock_worker(&[original.as_ptr()], &[b"parser", b"member"], inspect),
            Code::Ok
        );
        assert_eq!(interpreter.result_bytes(), b"public member\nprivate\nset");
        assert!(interpreter.handler_usage_adapters.borrow().is_empty());
        assert!(interpreter.ensemble_rewrite().is_none());
    }
}

#[cfg(test)]
mod publication_tests;

#[cfg(test)]
mod retirement_tests;
