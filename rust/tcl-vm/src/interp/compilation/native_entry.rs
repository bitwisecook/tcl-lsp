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

//! The native compilation entry: exporting the visible interpreter world (namespaces, commands, aliases, builtins, compiler hooks, variable tables) at a compilation boundary.

use super::super::commands::CommandSidecarKey;
use super::super::{NamespacePath, Vm};
use crate::command::{Command, EnsembleDef};
use crate::error::TclError;
use crate::value::Value;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{NsId, ROOT_NS};

impl Vm {
    /// Export the actual visible interpreter world at a compilation boundary.
    /// Catalogue availability never supplies a missing runtime binding.
    pub(in super::super) fn native_compilation_entry_for_namespace(
        &self,
        namespace: &str,
        procedure: bool,
    ) -> Result<tcl_runtime_api::NativeCompilationEntry, TclError> {
        let current = if namespace.is_empty() {
            Some(ROOT_NS)
        } else if let Some(current) = self
            .current_namespace_execution_context()
            .resolved_token()
            .filter(|token| self.native_namespace_token_has_owner(*token))
            && tcl_syntax::naming::native_namespace_key_bytes(&self.ns_path(current))
                == namespace.as_bytes()
        {
            Some(current)
        } else {
            let world = self.name_world.borrow();
            let mut matches = world.ns_intern.iter().filter_map(|(path, token)| {
                (tcl_syntax::naming::native_namespace_key_bytes(path) == namespace.as_bytes())
                    .then_some(*token)
            });
            matches.next().filter(|_| matches.next().is_none())
        };
        let current = current
            .filter(|token| self.native_namespace_token_has_owner(*token))
            .ok_or_else(Self::unavailable_native_compilation_namespace)?;
        Ok(self.native_compilation_entry_for_namespace_token(current, procedure))
    }

    pub(in super::super) fn current_native_compilation_namespace(&self) -> Result<NsId, TclError> {
        self.current_namespace_execution_context()
            .resolved_token()
            .filter(|token| self.native_namespace_token_has_owner(*token))
            .ok_or_else(Self::unavailable_native_compilation_namespace)
    }

    pub(super) fn unavailable_native_compilation_namespace() -> TclError {
        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native compilation namespace coordinates",
        )
        .into()
    }

    pub(in super::super) fn native_compilation_entry_for_namespace_token(
        &self,
        current: NsId,
        procedure: bool,
    ) -> tcl_runtime_api::NativeCompilationEntry {
        self.native_compilation_entry_with_literal_observation(
            current,
            procedure,
            |owner, epoch| {
                self.native_literal_world
                    .borrow()
                    .capture_empty_world(owner, epoch)
            },
        )
    }

    pub(in super::super) fn native_compilation_entry_with_literal_observation(
        &self,
        current: NsId,
        procedure: bool,
        literal_observation: impl FnOnce(
            tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
            u64,
        ) -> Option<
            tcl_runtime_api::native_literal::NativeEmptyLiteralWorld,
        >,
    ) -> tcl_runtime_api::NativeCompilationEntry {
        use tcl_runtime_api::native_compilation::{
            NativeCompilationEntry, NativeCompilationFrame, NativeInterpreterIdentity,
        };
        let interpreter = NativeInterpreterIdentity {
            owner: self.owner_nonce,
            interpreter: u64::try_from(self.cur.0).expect("interpreter slot fits u64"),
        };
        let (commands, complete) = self.native_compilation_commands(interpreter);
        let namespaces = self.native_compilation_namespaces();
        let frame = if procedure {
            NativeCompilationFrame::Procedure
        } else {
            match self.variable_alias_frame() {
                tcl_registry::VariableAliasFrame::Global => NativeCompilationFrame::Global,
                tcl_registry::VariableAliasFrame::Namespace => NativeCompilationFrame::Namespace,
                tcl_registry::VariableAliasFrame::Procedure => NativeCompilationFrame::Procedure,
                tcl_registry::VariableAliasFrame::Unknown => NativeCompilationFrame::Unknown,
            }
        };

        let mut entry = NativeCompilationEntry {
            interpreter,
            epoch: self.compilation_epochs.trace_deopt_epoch.get(),
            profile: self.source_profile().cache_key(),
            execution_point: self.actual_native_invocation_dialect().execution_point(),
            execution_name_policy: self.execution_name_policy(),
            name_protocol: self.name_policy_protocol(),
            compiled_variable_protocol: self.compiled_variable_protocol(),
            oo_classes: None,
            ensemble_target_objects: Some(self.native_ensemble_target_observations(interpreter)),
            source_string_protocol: self.source_string_protocol(),
            compiled_local_layout: if procedure {
                None
            } else {
                self.frames
                    .last()
                    .and_then(|frame| frame.compiled_local_layout.clone())
            },
            invocation_policy: Some(self.native_execution_profile().cache_key()),
            expression_policy: self.expression_evaluation_policy(),
            lexer_grammar: Some(
                self.lexer_config()
                    .grammar_over(self.source_profile().grammar),
            ),
            inline_compilation_disabled: self.step_trace_active(),
            direct_evaluation: false,
            math_functions: self.native_math_function_table(),
            closed: complete,
            commands,
            namespaces,
            // VM command lookup visits its own namespace/table arena only;
            // it has no interpreter or namespace resolver callback registry.
            // This observation does not describe handler execution or traces.
            command_resolvers: Some(
                tcl_runtime_api::native_compilation::NativeCommandResolverInventory::captured(
                    interpreter,
                    self.compilation_epochs.trace_deopt_epoch.get(),
                    tcl_runtime_api::native_compilation::NativeCommandResolverPresence::Absent,
                ),
            ),
            current_namespace: u64::from(current.0),
            authored_tmm_static: self.authored_static_compilation_context(),
            variable_observers: if !self.variable_observers.script_traces.is_empty()
                || !self.active_traces.is_empty()
            {
                tcl_runtime_api::native_compilation::NativeVariableObserverPresence::Present
            } else if self.variable_observers.precision_cell.is_some()
                || self.variable_observers.env_cell.is_some()
                || !self.variable_observers.native_error_cells.is_empty()
            {
                tcl_runtime_api::native_compilation::NativeVariableObserverPresence::BoundedNativeOnly
            } else {
                tcl_runtime_api::native_compilation::NativeVariableObserverPresence::Absent
            },
            namespace_variable_tables: self.native_namespace_variable_tables(interpreter),
            compiler_pass_environment: self.capture_native_compiler_pass_environment(None),
            empty_literal_world: literal_observation(
                interpreter,
                self.compilation_epochs.trace_deopt_epoch.get(),
            ),
            frame,
        };
        entry.oo_classes = crate::cmd_oo::native_bootstrap_inventory(self, &entry);
        entry
    }

    pub(in super::super) fn capture_native_compiler_pass_environment(
        &self,
        procedure: Option<tcl_runtime_api::native_compiler_pass::NativeCompilerPassProcedure>,
    ) -> Option<tcl_runtime_api::native_compiler_pass::NativeCompilerPassEnvironment> {
        if self.actual_native_invocation_dialect().native_family
            != Some(tcl_dialect::model::Family::Tcl)
            || self
                .actual_native_invocation_dialect()
                .execution_point()
                .is_none()
        {
            return None;
        }
        Some(
            self.native_compiler_pass_owner
                .capture_with_ordinary_entry_observation(
                    self.interps.get(self.cur.0)?.is_root(),
                    self.limits.cmd_value.is_some(),
                    self.limits.time_value.is_some(),
                    procedure,
                    tcl_runtime_api::native_compiler_pass::NativeOrdinaryEntryObservers::from(
                        self.exec_traces.values().all(Vec::is_empty)
                            && self.exec_step_scopes.is_empty()
                            && !self.trace_in_progress.get()
                            && self.firing_cmd_traces.is_empty()
                            && self.pending_exec_leave.is_none()
                            && self.debug_hooks.debug_hook.is_none()
                            && !self.debug_hooks.debug_hook_active.get()
                            && !self.interps.get(self.cur.0).is_none_or(|slot| slot.dying),
                    ),
                ),
        )
    }

    fn native_namespace_variable_tables(
        &self,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    ) -> Option<Vec<tcl_runtime_api::native_compilation::NativeNamespaceVariableTable>> {
        use tcl_runtime_api::native_compilation::{
            NativeNamespaceContext, NativeNamespaceVariableTable,
        };
        // C namespace tables and Jim's single flat global table have
        // separate storage owners. Neither is issued by Logical simulation.
        if self.name_policy_protocol()?.authority()
            != tcl_syntax::naming::NamePolicyAuthority::Native
        {
            return None;
        }
        let family = self.actual_native_invocation_dialect().native_family;
        if family == Some(tcl_dialect::model::Family::Jim)
            && self.name_policy_protocol()?.recipe().is_jim084()
        {
            return self.native_jim_global_variable_table(interpreter);
        }
        if family != Some(tcl_dialect::model::Family::Tcl) {
            return None;
        }
        let mut tables = Vec::new();
        for namespace in self.native_compilation_namespaces() {
            if !namespace.visible {
                continue;
            }
            let token = NsId(u32::try_from(namespace.token).ok()?);
            let table = if token == ROOT_NS {
                self.frames.first().map(|frame| &frame.locals)
            } else {
                self.ns_vars.get(&token)
            };
            let Some(table) = table else {
                continue;
            };
            let mut roots = table.keys().cloned().collect::<Vec<_>>();
            // Inventory equality needs no native iteration-order claim.
            roots.sort();
            tables.push(NativeNamespaceVariableTable {
                namespace: NativeNamespaceContext {
                    interpreter,
                    token: namespace.token,
                    path: namespace.path,
                },
                roots,
            });
        }
        Some(tables)
    }

    fn native_jim_global_variable_table(
        &self,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    ) -> Option<Vec<tcl_runtime_api::native_compilation::NativeNamespaceVariableTable>> {
        use tcl_runtime_api::native_compilation::{
            NativeNamespaceContext, NativeNamespaceVariableTable,
        };
        let namespace = self
            .native_compilation_namespaces()
            .into_iter()
            .find(|namespace| namespace.token == u64::from(ROOT_NS.0))?;
        if !namespace.visible
            || !namespace.path.is_root()
            || namespace.jim_namespace_object.as_ref()?.as_bytes() != b""
        {
            return None;
        }
        // Both Jim's global lookup and qualified namespace variable lookup
        // select Namespace(ROOT_NS), whose physical table is frame zero.
        // Keep its counted flat keys, including links and undefined cells;
        // do not project them through a C namespace or follow cell values.
        let mut roots = self
            .frames
            .first()?
            .locals
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        roots.sort();
        Some(vec![NativeNamespaceVariableTable {
            namespace: NativeNamespaceContext {
                interpreter,
                token: namespace.token,
                path: namespace.path,
            },
            roots,
        }])
    }

    /// [`Self::project_native_compilation_commands`], reused while no command
    /// table, binding, trace, policy or surface change has advanced the epochs
    /// that guard command resolution.
    fn native_compilation_commands(
        &self,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    ) -> (
        tcl_runtime_api::native_compilation::NativeCompilationCommands,
        bool,
    ) {
        let key = (
            interpreter,
            self.compilation_epochs.cmd_epoch.get(),
            self.trace_deopt_epoch(),
            self.native_compiler_policy_stamp(),
            std::ptr::from_ref(self.command_surface_profile).addr(),
        );
        if let Some((memo, commands, complete)) = self.compilation_commands_memo.borrow().as_ref()
            && *memo == key
        {
            return (commands.clone(), *complete);
        }
        let (commands, complete) = self.project_native_compilation_commands(interpreter);
        let commands: tcl_runtime_api::native_compilation::NativeCompilationCommands =
            commands.into();
        *self.compilation_commands_memo.borrow_mut() = Some((key, commands.clone(), complete));
        (commands, complete)
    }

    fn project_native_compilation_commands(
        &self,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    ) -> (
        Vec<tcl_runtime_api::native_compilation::NativeCompilationBinding>,
        bool,
    ) {
        let mut complete = true;
        let mut commands = self
            .commands
            .iter()
            .chain(
                self.name_world
                    .borrow()
                    .ns_deferral
                    .retained
                    .values()
                    .flat_map(|record| record.commands.iter()),
            )
            .filter_map(|(key, command)| {
                if !self.builtin_command_visible_for_surface(key, command) {
                    return None;
                }
                let binding = self.native_compilation_binding(key, command, interpreter);
                complete &= binding.is_some();
                binding
            })
            .collect::<Vec<_>>();
        commands.sort_by(|a, b| {
            a.slot
                .namespace
                .cmp(&b.slot.namespace)
                .then(a.slot.simple.cmp(&b.slot.simple))
                .then(a.token.cmp(&b.token))
        });
        (commands, complete)
    }

    fn native_compilation_namespaces(
        &self,
    ) -> Vec<tcl_runtime_api::native_compilation::NativeCompilationNamespace> {
        let mut namespaces = self
            .name_world
            .borrow()
            .ns_intern
            .iter()
            .map(|(path, token)| self.native_compilation_namespace(path, *token))
            .collect::<Vec<_>>();
        let jim_contexts = {
            let world = self.name_world.borrow();
            world
                .jim_namespace_objects
                .keys()
                .filter_map(|token| {
                    world
                        .ns_arena
                        .get(token.0 as usize)
                        .map(|path| (path.clone(), *token))
                })
                .collect::<Vec<_>>()
        };
        namespaces.extend(
            jim_contexts
                .into_iter()
                .map(|(path, token)| self.native_compilation_namespace(&path, token)),
        );
        for record in self.name_world.borrow().ns_deferral.retained.values() {
            namespaces.extend(
                record
                    .subtree
                    .iter()
                    .map(|(path, token)| self.native_compilation_namespace(path, *token)),
            );
        }
        namespaces.sort_by(|a, b| a.path.cmp(&b.path).then(a.token.cmp(&b.token)));
        namespaces.dedup_by(|a, b| a.token == b.token);
        namespaces
    }

    fn native_compilation_namespace(
        &self,
        path: &NamespacePath,
        token: NsId,
    ) -> tcl_runtime_api::native_compilation::NativeCompilationNamespace {
        let (retained_exports, retained_paths, retained_unknown) =
            self.retained_record_of(token).map_or_default(|record| {
                (
                    record.exports.get(&token).cloned(),
                    record.paths.get(&token).cloned(),
                    record.unknowns.get(&token).cloned(),
                )
            });
        let exports = retained_exports
            .or_else(|| self.ns_exports.get(&token).cloned())
            .unwrap_or_default();
        let command_path = retained_paths
            .or_else(|| self.name_world.borrow().ns_paths.get(&token).cloned())
            .unwrap_or_default()
            .into_iter()
            .map(|token| u64::from(token.0))
            .collect();
        let unknown_handler = retained_unknown
            .or_else(|| self.ns_unknowns.get(&token).cloned())
            .map(|handler| {
                handler.root.cached_list_representation().map_or_else(
                    || vec![None; handler.length],
                    |(words, _)| Self::native_compilation_prefix(&words),
                )
            });
        tcl_runtime_api::native_compilation::NativeCompilationNamespace {
            path: path.clone(),
            jim_namespace_object: self
                .name_world
                .borrow()
                .jim_namespace_objects
                .get(&token)
                .cloned(),
            token: u64::from(token.0),
            visible: self.namespace_reachable_from_root(token),
            exports: exports.into_iter().collect(),
            command_path,
            unknown_handler,
        }
    }

    /// Whether a namespace is reachable from the root: it and every ancestor
    /// are live. C unlinks a deleted namespace from its parent, so a retained
    /// child of a dying namespace is no longer found by path.
    fn namespace_reachable_from_root(&self, token: NsId) -> bool {
        let mut namespace = token;
        loop {
            let parent = {
                let world = self.name_world.borrow();
                if world.dead_namespaces.contains(&namespace) {
                    return false;
                }
                world.physical_namespace_parent(namespace)
            };
            match parent {
                Some(parent) => namespace = parent,
                // A detached retained namespace has no parent either.
                None => return self.ns_path(namespace).is_root(),
            }
        }
    }

    /// Capture only existing prefix bytes, preserving actual argument count.
    fn native_compilation_prefix(values: &[Value]) -> Vec<Option<tcl_core_types::NameBytes>> {
        values
            .iter()
            .map(|value| {
                value
                    .resident_string_bytes()
                    .map(|bytes| tcl_core_types::NameBytes::from(bytes.as_ref()))
            })
            .collect()
    }

    /// Capture aliases without materialising stored value objects. Argument
    /// bytes are an execution observation, not a compiler-admission operation.
    fn native_compilation_alias(
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
        values: &[Value],
        target_lookup: tcl_runtime_api::native_compilation::NativeAliasTargetLookup,
    ) -> tcl_runtime_api::native_compilation::NativeCommandImplementation {
        use tcl_runtime_api::native_compilation::NativeCommandImplementation;
        let words = Self::native_compilation_prefix(values);
        if let Some(complete) = words.iter().cloned().collect::<Option<Vec<_>>>() {
            NativeCommandImplementation::Alias {
                interpreter,
                words: complete,
                target_lookup,
            }
        } else {
            NativeCommandImplementation::PartialAlias {
                interpreter,
                words,
                target_lookup,
            }
        }
    }

    pub(in super::super) fn native_compilation_builtin(
        &self,
        key: &str,
    ) -> tcl_runtime_api::native_compilation::NativeCommandImplementation {
        use tcl_runtime_api::native_compilation::NativeCommandImplementation;
        let identity = self
            .retained_root_for_visible_key(key)
            .and_then(|root| {
                self.name_world
                    .borrow()
                    .ns_deferral
                    .retained
                    .get(&root)
                    .and_then(|record| record.builtin_identities.get(key))
                    .cloned()
            })
            .or_else(|| self.builtin_identities.get(key).cloned());
        let Some(identity) = identity else {
            return NativeCommandImplementation::Opaque;
        };
        let registry = {
            tcl_registry::model::ingress::static_context_for_profile(
                self.actual_native_execution_profile(),
            )
            .commands()
        };
        let compiler_hook = registry
            .native_compilation_for_registration(&identity, self.actual_native_invocation_dialect())
            .and_then(|spec| spec.compiler_hook_presence(self.actual_native_invocation_dialect()))
            == Some(true);
        NativeCommandImplementation::Registry {
            identity,
            compiler_hook,
        }
    }

    pub(in super::super) fn native_hook_for_registry_identity(
        &self,
        identity: &str,
    ) -> tcl_runtime_api::native_compilation::NativeCompilerHookPresence {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
        let registry = {
            tcl_registry::model::ingress::static_context_for_profile(
                self.actual_native_execution_profile(),
            )
            .commands()
        };
        let dialect = self.actual_native_invocation_dialect();
        match registry
            .native_compilation_for_registration(identity, dialect)
            .and_then(|spec| spec.compiler_hook_presence(dialect))
        {
            Some(true) => NativeCompilerHookPresence::Present,
            Some(false) => NativeCompilerHookPresence::Absent,
            None if created_without_compile_procedure(identity) => {
                NativeCompilerHookPresence::Absent
            }
            None => NativeCompilerHookPresence::Unknown,
        }
    }

    pub(in super::super) fn native_hook_for_implementation(
        &self,
        implementation: &tcl_runtime_api::native_compilation::NativeCommandImplementation,
        command: &Command,
    ) -> tcl_runtime_api::native_compilation::NativeCompilerHookPresence {
        use tcl_runtime_api::native_compilation::{
            NativeCommandImplementation, NativeCompilerHookPresence,
        };
        if let Command::Ensemble(token) = command {
            return token.compiler_hook();
        }
        match implementation {
            NativeCommandImplementation::Registry { identity, .. } => {
                self.native_hook_for_registry_identity(identity)
            }
            NativeCommandImplementation::Opaque
                if matches!(
                    command,
                    Command::Builtin(_)
                        | Command::Native(_)
                        | Command::Alias { .. }
                        | Command::CallerAlias(_)
                        | Command::CrossAlias { .. }
                        | Command::ChildInterp(_)
                        | Command::Object(_)
                ) =>
            {
                NativeCompilerHookPresence::Absent
            }
            NativeCommandImplementation::Opaque if matches!(command, Command::Proc(_)) => {
                let Command::Proc(procedure) = command else {
                    unreachable!()
                };
                match procedure.native_header {
                    tcl_dialect::NativeProcedureHeaderCompilation::Absent => {
                        NativeCompilerHookPresence::Absent
                    }
                    tcl_dialect::NativeProcedureHeaderCompilation::NoOp => {
                        NativeCompilerHookPresence::Present
                    }
                    tcl_dialect::NativeProcedureHeaderCompilation::Unknown => {
                        NativeCompilerHookPresence::Unknown
                    }
                }
            }
            NativeCommandImplementation::Alias { .. }
            | NativeCommandImplementation::PartialAlias { .. } => {
                NativeCompilerHookPresence::Absent
            }
            _ => NativeCompilerHookPresence::Unknown,
        }
    }

    fn native_compiler_registration(
        &self,
        command: &Command,
        implementation: &tcl_runtime_api::native_compilation::NativeCommandImplementation,
    ) -> Option<tcl_runtime_api::native_compilation::NativeCommandCompiler> {
        use tcl_runtime_api::native_compilation::{
            NativeCommandCompiler, NativeCommandImplementation,
        };
        match command {
            Command::Ensemble(token) => {
                let config = token.config();
                let native = config.native.as_deref()?;
                let dialect = self.actual_native_invocation_dialect();
                let registry = tcl_registry::model::ingress::static_context_for_profile(
                    self.actual_native_execution_profile(),
                )
                .commands();
                if registry
                    .native_compilation_for_registration(native.identity, dialect)
                    .is_some_and(tcl_registry::native_compilation::NativeCompilationSpec::is_named_invocation_compiler)
                {
                    return Some(NativeCommandCompiler {
                        registry_identity: native.identity.to_owned(),
                        ensemble: None,
                    });
                }
                Some(Self::native_ensemble_compiler_registration(
                    token,
                    native.identity,
                ))
            }
            Command::Builtin(_) => match implementation {
                NativeCommandImplementation::Registry { identity, .. } => {
                    Some(NativeCommandCompiler {
                        registry_identity: identity.clone(),
                        ensemble: None,
                    })
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn native_ensemble_compiler_registration(
        token: &tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>,
        registry_identity: &str,
    ) -> tcl_runtime_api::native_compilation::NativeCommandCompiler {
        use tcl_runtime_api::native_compilation::{NativeCommandCompiler, NativeEnsembleCompiler};
        let config = token.config();
        NativeCommandCompiler {
            registry_identity: registry_identity.to_owned(),
            ensemble: Some(NativeEnsembleCompiler {
                namespace_token: u64::from(config.namespace.0),
                map: config.map.clone(),
                subcommands: config.subcommands.clone(),
                prefixes: config.prefixes,
                parameters: config.parameters.clone(),
                unknown_handler: config
                    .unknown
                    .as_ref()
                    .map(|words| words.iter().cloned().map(Some).collect()),
            }),
        }
    }

    fn native_ensemble_target_observations(
        &self,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    ) -> Vec<tcl_runtime_api::native_compilation::NativeEnsembleTargetObservation> {
        use tcl_runtime_api::native_compilation::{
            NativeEnsembleTargetObservation, NativeEnsembleTargetPrimary,
        };
        let mut observations = Vec::new();
        let world = self.name_world.borrow();
        for (key, command) in self.commands.iter().chain(
            world
                .ns_deferral
                .retained
                .values()
                .flat_map(|record| record.commands.iter()),
        ) {
            let Command::Ensemble(token) = command else {
                continue;
            };
            let Some(binding) = self.native_compilation_binding(key, command, interpreter) else {
                continue;
            };
            let configuration = token.config();
            for (member, prefix) in &configuration.map {
                for position in 0..prefix.len() {
                    let observation = configuration.with_original_map_target(member.as_bytes(), position, |original| {
                        let snapshot = original.native_object_snapshot();
                        let primary = match snapshot.cache {
                            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
                                | tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. } => NativeEnsembleTargetPrimary::StockString,
                            tcl_syntax::native_object::NativeObjectCacheSnapshot::CommandName { version, .. } => {
                                let cache = original.native_command_name_cache();
                                let lookup = cache.as_ref().map(|cache| self.native_command_name_lookup_state(cache, configuration.namespace));
                                NativeEnsembleTargetPrimary::CommandName { origin: version, cache, lookup }
                            }
                            _ => NativeEnsembleTargetPrimary::Unavailable,
                        };
                        NativeEnsembleTargetObservation { ensemble_token: binding.token, member: member.clone(), prefix_index: position,
                            object_identity: u64::try_from(original.native_object_identity()).expect("native object identity fits u64"),
                            resident_name: snapshot.resident.as_ref().map(|name| NameBytes::from(name.as_ref())), primary }
                    });
                    if let Some(observation) = observation {
                        observations.push(observation);
                    }
                }
            }
        }
        observations
    }

    fn native_command_implementation(
        &self,
        key: &str,
        command: &Command,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    ) -> Option<tcl_runtime_api::native_compilation::NativeCommandImplementation> {
        use tcl_runtime_api::native_compilation::{
            NativeCommandImplementation, NativeInterpreterIdentity,
        };
        let implementation = if let Some(import) = self.visible_import_binding(key) {
            let source = import.origin.key.name();
            NativeCommandImplementation::Imported {
                interpreter,
                token: import.origin.generation,
                slot: self.command_slot(source).map(|slot| {
                    tcl_core_types::NativeByteCommandSlot::new(
                        self.ns_path(slot.namespace),
                        slot.simple.clone(),
                    )
                }),
            }
        } else {
            match command {
                Command::Builtin(_) => self.native_compilation_builtin(key),
                Command::Ensemble(ensemble) => {
                    let configuration = ensemble.config();
                    if let Some(native) = configuration.native_implementation() {
                        NativeCommandImplementation::Registry {
                            identity: native.identity.to_owned(),
                            compiler_hook: true,
                        }
                    } else {
                        NativeCommandImplementation::Opaque
                    }
                }
                Command::Object(_) if self.registry_object_roots.contains_key(key) => {
                    NativeCommandImplementation::Registry {
                        identity: self.registry_object_roots[key].clone(),
                        compiler_hook: false,
                    }
                }
                Command::Alias { words, .. } => Self::native_compilation_alias(
                    interpreter,
                    words,
                    tcl_runtime_api::native_compilation::NativeAliasTargetLookup::Global,
                ),
                Command::CallerAlias(original) => {
                    let words = original
                        .native_list_backing_in(
                            tcl_syntax::native_string::NativeStringProtocol::Jim084,
                        )
                        .ok()??;
                    Self::native_compilation_alias(
                        interpreter,
                        &words,
                        tcl_runtime_api::native_compilation::NativeAliasTargetLookup::CallerNamespace,
                    )
                }
                Command::CrossAlias { target, words, .. } => Self::native_compilation_alias(
                    NativeInterpreterIdentity {
                        owner: interpreter.owner,
                        interpreter: u64::try_from(target.0).expect("interpreter slot fits u64"),
                    },
                    words,
                    tcl_runtime_api::native_compilation::NativeAliasTargetLookup::Global,
                ),
                _ => NativeCommandImplementation::Opaque,
            }
        };
        Some(implementation)
    }

    pub(crate) fn native_compilation_binding(
        &self,
        key: &str,
        command: &Command,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    ) -> Option<tcl_runtime_api::native_compilation::NativeCompilationBinding> {
        use tcl_runtime_api::native_compilation::NativeCompilationBinding;
        let slot = self.command_slot(key)?;
        if !self.builtin_command_visible_for_surface(key, command) {
            return None;
        }
        let token = self.visible_command_generation(key)?;
        let implementation = self.native_command_implementation(key, command, interpreter)?;
        let compiler_hook = self.native_hook_at_sidecar(&CommandSidecarKey::visible(key));
        let compiler = self.visible_import_binding(key).map_or_else(
            || self.native_compiler_registration(command, &implementation),
            |import| {
                import.compiler.map(|compiler| match command {
                    Command::Ensemble(token) if compiler.ensemble.is_some() => {
                        Self::native_ensemble_compiler_registration(
                            token,
                            &compiler.registry_identity,
                        )
                    }
                    _ => compiler,
                })
            },
        );
        Some(NativeCompilationBinding {
            slot: tcl_core_types::NativeByteCommandSlot::new(
                self.ns_path(slot.namespace),
                slot.simple.clone(),
            ),
            namespace_token: u64::from(slot.namespace.0),
            token,
            implementation_generation: token,
            implementation,
            compiler_hook,
            compiler,
            procedure_header: self.visible_import_binding(key).map_or_else(
                || match command {
                    Command::Proc(procedure) => Some(procedure.native_header),
                    _ => None,
                },
                |binding| binding.procedure_header,
            ),
            has_execution_trace: self
                .command_has_execution_trace(&CommandSidecarKey::visible(key.to_owned())),
        })
    }
}

/// A command C creates with `Tcl_CreateObjCommand`, so with no compile
/// procedure: the Thread extension's `thread::`, `tsv::` and `tpool::`
/// commands, and 9.0's `ClockConfigureObjCmd`.
fn created_without_compile_procedure(identity: &str) -> bool {
    let name = tcl_syntax::naming::unroot_rooted_key(identity).unwrap_or(identity);
    name == "tcl::unsupported::clock::configure"
        || ["thread::", "tsv::", "tpool::"]
            .iter()
            .any(|prefix| name.starts_with(prefix))
}
