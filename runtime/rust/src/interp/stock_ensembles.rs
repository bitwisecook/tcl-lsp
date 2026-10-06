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

use super::{BuiltinFn, Code, Command, Interp, TclObj, GLOBAL};
use crate::ensemble::EnsembleConfig;
use std::collections::HashMap;
use tcl_registry::invocation_words::EnsembleImplementationFamily;

#[derive(Default)]
pub(super) struct StockEnsembles {
    families: HashMap<EnsembleImplementationFamily, InstalledFamily>,
}

pub(super) struct HandlerUsageAdapter {
    pub(super) parse_prefix: Vec<*mut TclObj>,
    pub(super) original_prefix: Vec<crate::obj::Owned>,
}

struct HandlerUsageScope {
    interpreter: Interp,
    previous_len: usize,
}

impl Drop for HandlerUsageScope {
    fn drop(&mut self) {
        self.interpreter
            .handler_usage_adapters
            .borrow_mut()
            .truncate(self.previous_len);
    }
}

struct InstalledFamily {
    public: &'static [u8],
    generation: u64,
    config: Option<EnsembleConfig>,
    compiler: BuiltinFn,
    scripted: bool,
    scripted_helpers: Vec<(Vec<u8>, u64)>,
    members: Vec<(Vec<u8>, u64)>,
    namespaces: Vec<(Vec<u8>, crate::namespace::NsId)>,
    nested: Vec<(Vec<u8>, u64, EnsembleConfig, BuiltinFn)>,
}

impl Interp {
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
        if let Some(namespace) = self
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
                self.register_stock_worker(&target, callback, &mut installed);
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
            self.create_ensemble(public, config.clone());
            installed.config = Some(config);
        } else {
            self.register_builtin(public, compiler);
        }
        let Some(generation) = self.resolve_cmd_token(public) else {
            return;
        };
        installed.generation = generation;
        self.native_compilation
            .borrow_mut()
            .stock
            .insert(installed.generation, public.to_vec());
        self.stock_ensembles
            .borrow_mut()
            .families
            .insert(family, installed);
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
        let family = EnsembleImplementationFamily::Binary;
        let Some(mut installed) = self.stock_ensembles.borrow_mut().families.remove(&family) else {
            return;
        };
        if self.resolve_cmd_token(installed.public) == Some(installed.generation)
            && self.stock_ensemble_config_matches(&installed)
            && ingress.members.len() == callbacks.len()
        {
            let parameters = crate::cmd_proc::parse_params_in(self, ingress.parameters.as_bytes())
                .expect("audited distribution formal grammar");
            for (name, callback) in ingress.members.iter().zip(callbacks) {
                self.register_stock_worker(name.as_bytes(), *callback, &mut installed);
                if let Some(token) = self.resolve_cmd_token(name.as_bytes()) {
                    // These helpers belong to the scripted distribution; no
                    // C primitive compiler identity is donated to them.
                    self.native_compilation.borrow_mut().stock.remove(&token);
                    installed
                        .scripted_helpers
                        .push((name.as_bytes().to_vec(), token));
                }
            }
            let body = crate::obj::Owned::fresh(super::new_string(ingress.body.as_bytes()));
            self.define_proc_storage(
                ingress.name.as_bytes(),
                parameters,
                body.as_ptr(),
                None,
                None,
            );
            installed.generation = self
                .resolve_cmd_token(ingress.name.as_bytes())
                .expect("installed actual distribution procedure");
            installed.scripted = true;
        }
        self.stock_ensembles
            .borrow_mut()
            .families
            .insert(family, installed);
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
    ) {
        // A host or script may already own the private spelling when a host
        // changes release. Its live handler remains the map's actual target.
        if self.resolve_cmd_token(name).is_some() {
            return;
        }
        self.register_builtin(name, callback);
        let generation = self
            .resolve_cmd_token(name)
            .expect("installed private token");
        self.native_compilation
            .borrow_mut()
            .stock
            .insert(generation, name.to_vec());
        installed.members.push((name.to_vec(), generation));
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
        let Some(mut installed) = self.stock_ensembles.borrow_mut().families.remove(&family) else {
            return;
        };
        let owned = installed.members.iter().position(|(slot, generation)| {
            slot == name && self.resolve_cmd_token(slot) == Some(*generation)
        });
        if let Some(index) = owned {
            installed.members.remove(index);
            let ns = self.ensure_stock_namespace(name, &mut installed);
            let mut map = Vec::new();
            for &(member, worker) in members {
                let mut target = name.to_vec();
                target.extend_from_slice(b"::");
                target.extend_from_slice(member);
                self.register_stock_worker(&target, worker, &mut installed);
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
            self.create_ensemble(name, config.clone());
            let generation = self
                .resolve_cmd_token(name)
                .expect("installed codec ensemble");
            self.native_compilation
                .borrow_mut()
                .stock
                .insert(generation, name.to_vec());
            installed.members.push((name.to_vec(), generation));
            installed
                .nested
                .push((name.to_vec(), generation, config, callback));
        }
        self.stock_ensembles
            .borrow_mut()
            .families
            .insert(family, installed);
    }

    pub(super) fn refresh_stock_ensembles(&mut self) {
        let families = std::mem::take(&mut self.stock_ensembles.borrow_mut().families);
        for (family, installed) in families {
            if self.resolve_cmd_token(installed.public) != Some(installed.generation)
                || !self.stock_ensemble_config_matches(&installed)
                || installed.nested.iter().any(|(_, token, config, _)| {
                    !matches!(self.command_by_generation(*token),
                        super::CommandGenerationLookup::Found { command: Command::Ensemble(ensemble), .. }
                            if ensemble.config() == *config)
                })
            {
                self.stock_ensembles
                    .borrow_mut()
                    .families
                    .insert(family, installed);
                continue;
            }
            for (name, generation) in &installed.members {
                if self.resolve_cmd_token(name) == Some(*generation) {
                    self.delete_command(name);
                }
            }
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
                }
            }
            match family {
                EnsembleImplementationFamily::Info => crate::cmd_info::install(self),
                EnsembleImplementationFamily::File => crate::cmd_fs::install_file(self),
                EnsembleImplementationFamily::Array => crate::cmd_array::install(self),
                EnsembleImplementationFamily::Namespace => crate::cmd_namespace::install(self),
                EnsembleImplementationFamily::Binary => crate::cmd_binary::install(self),
                EnsembleImplementationFamily::String => crate::cmd_string::install_ensemble(self),
                EnsembleImplementationFamily::Dict => crate::cmd_dict::install(self),
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
            let registry = tcl_registry::default_registry();
            if let Some(spec) = registry.get("string") {
                self.attest_spec_implementation(spec);
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

    /// Actual distribution helper allocations retain their own runtime
    /// admission, independently of the primitive catalogue's fresh roster.
    pub(super) fn is_stock_scripted_worker(&self, generation: u64) -> bool {
        self.stock_ensembles
            .borrow()
            .families
            .values()
            .any(|installed| {
                installed.scripted
                    && installed
                        .scripted_helpers
                        .iter()
                        .any(|(_, token)| *token == generation)
            })
    }

    /// Runtime registration replaces an actual declared compound slot; retain
    /// its new allocation's admission without giving it a stock compiler ID.
    pub(super) fn record_scripted_helper_registration(&self, name: &[u8], generation: u64) {
        let normalise = |word: &[u8]| word.strip_prefix(b"::").unwrap_or(word).to_vec();
        for installed in self.stock_ensembles.borrow_mut().families.values_mut() {
            if installed.scripted
                && installed
                    .scripted_helpers
                    .iter()
                    .any(|(slot, _)| normalise(slot) == normalise(name))
            {
                installed.scripted_helpers.push((name.to_vec(), generation));
            }
        }
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
        identity: &[u8],
        path: &[tcl_registry::native_compilation::NativeCompilerImplementationLookup],
    ) -> bool {
        let normalise = |name: &[u8]| name.strip_prefix(b"::").unwrap_or(name).to_vec();
        if path
            .last()
            .is_some_and(|edge| normalise(identity) == normalise(edge.slot.as_bytes()))
        {
            return true;
        }
        let mut current = generation;
        for (index, edge) in path.iter().enumerate() {
            if index == 0 && normalise(identity) != normalise(edge.ensemble.as_bytes()) {
                let Some(token) = self.resolve_cmd_token(edge.ensemble.as_bytes()) else {
                    return false;
                };
                current = token;
            }
            let super::CommandGenerationLookup::Found {
                command: Command::Ensemble(ensemble),
                ..
            } = self.command_by_generation(current)
            else {
                return false;
            };
            let config = ensemble.config();
            if !config.parameters.is_empty()
                || !config.unknown.is_empty()
                || !config.map.as_ref().is_some_and(|map| {
                    map.iter().any(|(member, prefix)| {
                        member == edge.member.as_bytes()
                            && prefix.len() == 1
                            && normalise(&prefix[0]) == normalise(edge.slot.as_bytes())
                    })
                })
            {
                return false;
            }
            let Some(token) = self.resolve_cmd_token(edge.slot.as_bytes()) else {
                return false;
            };
            if !self
                .native_compilation
                .borrow()
                .stock
                .get(&token)
                .is_some_and(|name| normalise(name) == normalise(edge.slot.as_bytes()))
            {
                return false;
            }
            current = token;
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
        };
        callback(self, &view)
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
        let mut config = token.config();
        config.map = Some(vec![(
            b"hex".to_vec(),
            vec![b"::tcl::binary::encode::base64".to_vec()],
        )]);
        token.configure(config);
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
