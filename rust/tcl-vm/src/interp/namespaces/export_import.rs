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

//! `namespace export` and `namespace import`: export patterns, import candidates, the C import preload, publishing imported commands and Jim's import aliases.

use super::super::commands::{CommandSidecarKey, NativeCommandLookupUnavailable};
use super::super::namespace_context::NamespaceExecutionContext;
use super::super::{
    CommandSlot, CommandTokenIdentity, ImportBinding, ImportedSourceCommand,
    OriginalCImportPreload, Vm, native_name_consumers, native_numeric_ops,
    native_operation_currency, ok,
};
use crate::command::Command;
use crate::value::Value;
use std::collections::HashSet;
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{Code, Completion, NsId, ROOT_NS};

impl OriginalCImportPreload {
    pub(crate) fn command(&self) -> Command {
        self.command.clone()
    }

    pub(crate) fn key(&self) -> CommandSidecarKey {
        self.token.key.clone()
    }

    pub(crate) fn install_invoked_head(
        &self,
        vm: &mut Vm,
        head: &Value,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        if vm.command_token_identity(&self.token.key).as_ref() != Some(&self.token) {
            return Err(vm.retain_name_query_refusal("original C import preload command token"));
        }
        head.check_native_header()?;
        vm.native_invocation.name = head
            .existing_string_representation()
            .map(|text| text.to_string());
        vm.native_invocation.original_name = Some(head.native_lifetime_lease().into_value());
        vm.native_invocation.builtin_identity = match &self.token.key {
            CommandSidecarKey::Visible(key) => vm.stock_native_identity(key),
            CommandSidecarKey::Hidden(_) => None,
        };
        Ok(())
    }
}

/// A namespace-import graph mutation rejected before the destination binding
/// changes. The command layer maps each variant to Tcl's stable error code.
#[derive(Debug)]
pub(crate) enum NamespaceImportError {
    Overwrite {
        command: NameBytes,
    },
    Loop {
        pattern: NameBytes,
        command: NameBytes,
    },
    Host(String),
    Original(tcl_cmd_core::CmdError),
}

impl NamespaceImportError {
    pub(crate) fn completion(self, vm: &mut Vm) -> Completion<Value> {
        let (bytes, code) = match self {
            Self::Overwrite { command } => {
                let mut bytes = b"can't import command \"".to_vec();
                bytes.extend_from_slice(command.as_bytes());
                bytes.extend_from_slice(b"\": already exists");
                (bytes, "TCL IMPORT OVERWRITE")
            }
            Self::Loop { pattern, command } => {
                let mut bytes = b"import pattern \"".to_vec();
                bytes.extend_from_slice(pattern.as_bytes());
                bytes.extend_from_slice(b"\" would create a loop containing command \"::");
                bytes.extend_from_slice(command.as_bytes());
                bytes.push(b'"');
                (bytes, "TCL IMPORT LOOP")
            }
            Self::Host(reason) => return vm.refuse_host_command(reason),
            Self::Original(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        crate::command::completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::with_error_code_bytes(bytes, code.as_bytes()),
        )
    }
}

impl Vm {
    /// Record `namespace export` patterns for the current namespace. C's
    /// `Tcl_Export` skips a pattern already in the array, so the list is a
    /// set in insertion order — `namespace export a; namespace export a`
    /// still reports `a`.
    pub(crate) fn add_exports(&mut self, patterns: &[NameBytes]) {
        self.invalidate_namespace_structure_guards();
        let ns = self.current_ns_id();
        let world_handle = Rc::clone(&self.name_world);
        let mut world = world_handle.borrow_mut();
        let root = world.ns_deferral.owners.get(&ns).copied();
        let retained = root.and_then(|root| world.ns_deferral.retained.get_mut(&root));
        let entry = match retained {
            Some(record) => record.exports.entry(ns).or_default(),
            None => self.ns_exports.entry(ns).or_default(),
        };
        let mut changed = false;
        for pattern in patterns {
            if !entry.contains(pattern) {
                entry.push(pattern.clone());
                changed = true;
            }
        }
        drop(world);
        if changed {
            self.advance_native_ensemble_export_epoch(ns);
        }
    }

    /// The current namespace's `namespace export` pattern list (C's
    /// `Tcl_AppendExportList`, the `namespace export` query form).
    pub(crate) fn exports_get(&self) -> Vec<NameBytes> {
        let current = self.current_ns_id();
        self.retained_current_record()
            .and_then(|record| record.exports.get(&current).cloned())
            .or_else(|| self.ns_exports.get(&current).cloned())
            .unwrap_or_default()
    }

    /// Drop the current namespace's export patterns — the state change
    /// `namespace export -clear` makes (`Tcl_Export`'s `resetListFirst`).
    pub(crate) fn clear_exports(&mut self) {
        self.invalidate_namespace_structure_guards();
        let ns = self.current_ns_id();
        if !self.exports_get().is_empty() {
            self.advance_native_ensemble_export_epoch(ns);
        }
        let world_handle = Rc::clone(&self.name_world);
        let mut world = world_handle.borrow_mut();
        match world.ns_deferral.owners.get(&ns).copied() {
            Some(root) => {
                if let Some(record) = world.ns_deferral.retained.get_mut(&root) {
                    record.exports.remove(&ns);
                }
            }
            None => {
                self.ns_exports.remove(&ns);
            }
        }
    }

    /// The unqualified names of the current namespace's commands that
    /// `namespace import` created — C's `NamespaceImportCmd` introspection
    /// form (`objc == 1`), a `cmdTable` walk for `deleteProc ==
    /// DeleteImportedCmd`. Sorted, since the VM's table has no hash order to
    /// reproduce.
    pub(crate) fn imported_command_tails(&self) -> Vec<NameBytes> {
        let ns = self
            .resolution_stacks
            .ns_id_stack
            .last()
            .and_then(NamespaceExecutionContext::resolved_token)
            .expect("operation requires a materialized current namespace");
        let mut names: Vec<NameBytes> = Vec::new();
        for key in self.imported_commands.keys().cloned().chain(
            self.retained_record_of(ns)
                .into_iter()
                .flat_map(|record| record.imported.keys().cloned().collect::<Vec<_>>()),
        ) {
            if let Some(slot) = self.command_slot(&key)
                && slot.namespace == ns
            {
                names.push(slot.simple.clone());
            }
        }
        names.sort();
        names.dedup();
        names
    }

    /// Declare a built-in namespace (`ns`, unrooted) and record its
    /// `namespace export` patterns directly. Used for namespaces whose commands
    /// are created in Rust rather than by a script `namespace export` — e.g.
    /// `::tcl::mathop`, which C exports so `namespace import ::tcl::mathop::*`
    /// works.
    pub(crate) fn declare_namespace_exports(&mut self, ns: &str, patterns: &[&str]) {
        self.declare_namespace(ns);
        let id = self
            .namespace_id_from_written(ROOT_NS, ns)
            .unwrap_or(ROOT_NS);
        let entry = self.ns_exports.entry(id).or_default();
        for p in patterns {
            if !entry.iter().any(|e| e == p) {
                entry.push(NameBytes::from(*p));
            }
        }
    }

    /// Mint a new namespace-import command-token identity. Moving or renaming
    /// an existing imported binding carries its generation instead.
    fn fresh_import_binding(
        &self,
        alias: &CommandSidecarKey,
        origin: CommandTokenIdentity,
    ) -> ImportBinding {
        let compiler_hook = self.native_hook_at_sidecar(&origin.key);
        let procedure_header = self.import_binding_at_sidecar_key(&origin.key).map_or_else(
            || match self.command_at_sidecar_key(&origin.key) {
                Some(Command::Proc(procedure)) => Some(procedure.native_header),
                _ => None,
            },
            |binding| binding.procedure_header,
        );
        let compiler = (compiler_hook
            == tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Present)
            .then(|| {
                let command = self.command_at_sidecar_key(&origin.key)?;
                self.native_compilation_binding(
                    origin.key.name(),
                    &command,
                    self.native_interpreter_identity(),
                )?
                .compiler
            })
            .flatten();
        ImportBinding {
            origin,
            compiler_hook,
            procedure_header,
            compiler,
            generation: self
                .command_token_identity(alias)
                .expect("an imported command was just registered")
                .generation,
        }
    }

    /// Snapshot exported source commands with the exact token identity their
    /// eventual import edges must reference. A retained namespace contributes
    /// its private table directly; no rendered name is inverted to find it.
    fn import_candidates(
        &self,
        source: NsId,
        glob: &[u8],
        exports: &[NameBytes],
    ) -> Result<Vec<ImportedSourceCommand>, NamespaceImportError> {
        let candidates = self
            .commands
            .iter()
            .filter(|(key, _)| {
                self.command_slot(key)
                    .is_some_and(|slot| slot.namespace == source)
            })
            .map(|(key, command)| (key.clone(), command.clone()))
            .chain(
                self.retained_record_of(source)
                    .into_iter()
                    .flat_map(|record| {
                        record
                            .commands
                            .iter()
                            .map(|(key, command)| (key.clone(), command.clone()))
                            .collect::<Vec<_>>()
                    }),
            );
        let mut selected = Vec::new();
        for (command_key, command) in candidates {
            let Some(slot) = self.command_slot(&command_key) else {
                continue;
            };
            if slot.namespace != source
                || slot.simple.is_empty()
                || !self.builtin_command_visible_for_surface(&command_key, &command)
            {
                continue;
            }
            if !self
                .native_namespace_match(
                    tcl_syntax::native_glob::NativeNameGlobPurpose::ImportSearch,
                    glob,
                    slot.simple.as_bytes(),
                )
                .map_err(|error| NamespaceImportError::Host(error.to_string()))?
            {
                continue;
            }
            let mut exported = false;
            for pattern in exports {
                if self
                    .native_namespace_match(
                        tcl_syntax::native_glob::NativeNameGlobPurpose::ExportFilter,
                        pattern.as_bytes(),
                        slot.simple.as_bytes(),
                    )
                    .map_err(|error| NamespaceImportError::Host(error.to_string()))?
                {
                    exported = true;
                    break;
                }
            }
            if !exported {
                continue;
            }
            let builtin_identity = matches!(command, Command::Builtin(_) | Command::Native(_))
                .then(|| {
                    self.retained_record_of(source)
                        .and_then(|record| record.builtin_identities.get(&command_key).cloned())
                        .or_else(|| self.builtin_identities.get(&command_key).cloned())
                        .unwrap_or_else(|| command_key.clone())
                });
            let source_token = self
                .command_token_identity(&CommandSidecarKey::visible(&command_key))
                .expect("every registered command has a token generation");
            selected.push((slot.simple.clone(), command, builtin_identity, source_token));
        }
        Ok(selected)
    }

    pub(super) fn namespace_pattern_parts(
        &self,
        original: &[u8],
        purpose: tcl_syntax::naming::NativeNamePurpose,
    ) -> Result<tcl_syntax::naming::NativeNamespacePatternParts, NativeCommandLookupUnavailable>
    {
        let protocol = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        let current = self.current_ns_id();
        let path = self.ns_path(current);
        let jim = self
            .name_world
            .borrow()
            .jim_namespace_objects
            .get(&current)
            .cloned();
        let context = if protocol.is_jim084() {
            tcl_syntax::naming::NativeNameContext::with_jim_namespace(
                &path,
                jim.as_ref()
                    .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)?
                    .as_bytes(),
            )
        } else {
            tcl_syntax::naming::NativeNameContext::new(&path)
        };
        protocol
            .namespace_pattern_parts(context, original, purpose)
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)
    }

    pub(super) fn namespace_pattern_source(
        &self,
        parts: &tcl_syntax::naming::NativeNamespacePatternParts,
    ) -> Option<NsId> {
        use tcl_syntax::naming::{NativeNameQualification, NativeNamespacePatternSource};
        match parts.source.as_ref() {
            None => Some(self.current_ns_id()),
            Some(NativeNamespacePatternSource::C(path)) => self.namespace_token_at_path(
                path,
                parts.qualification == NativeNameQualification::Absolute,
            ),
            Some(NativeNamespacePatternSource::Jim(object)) => self
                .name_world
                .borrow()
                .jim_namespace_objects
                .iter()
                .find_map(|(token, candidate)| (candidate == object).then_some(*token)),
        }
    }

    /// Enter the original optional C import preload stage before validation.
    pub(crate) fn original_c_import_preload(
        &mut self,
        pattern: &[u8],
        original: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<Option<Completion<Value>>, tcl_cmd_core::CmdError> {
        use tcl_registry::native_namespace_name::NativeNamespaceImportPreloadRoute;
        use tcl_syntax::value::ValueError;

        original.check(self)?;
        let protocol = self
            .actual_native_invocation_dialect()
            .native_namespace_name_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C import preload issuer",
            ))?;
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C import preload naming issuer",
            ))?;
        let helper =
            tcl_registry::state_transition::NamespaceTransition::import_preload_command(policy)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original C import preload head",
                ))?;
        // Tcl_FindCommand(..., TCL_GLOBAL_ONLY) observes this exact global slot.
        // It does not enter unknown or a namespace path while testing absence.
        let slot = CommandSlot::new(ROOT_NS, NameBytes::from(helper));
        let key = self
            .command_at_exact_slot_checked(&slot, true)
            .map_err(|_| self.retain_name_query_refusal("original global import preload lookup"))?;
        let Some(key) = key else {
            return Ok(None);
        };
        let head = Value::new_native_string_bytes(helper.as_bytes());
        let operand = Value::new_native_string_bytes(pattern);
        original.current_for_value(self, &head)?;
        original.current_for_value(self, &operand)?;
        let mut completion = match protocol.import_preload_route() {
            NativeNamespaceImportPreloadRoute::GlobalDirect => {
                let command = self.visible_command_at_key(&key).ok_or_else(|| {
                    self.retain_name_query_refusal("original global import preload callable")
                })?;
                let token = self
                    .command_token_identity(&CommandSidecarKey::visible(&key))
                    .ok_or_else(|| {
                        self.retain_name_query_refusal("original global import preload generation")
                    })?;
                self.invoke_original_c_import_preload_direct(
                    OriginalCImportPreload { token, command },
                    &head,
                    &operand,
                    original,
                )
            }
            NativeNamespaceImportPreloadRoute::CurrentEvaluation => self.invoke_command_value_at(
                self.current_ns_id(),
                &head,
                std::slice::from_ref(&operand),
                &[],
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            ),
        };
        let returned = (|| {
            original.current_for_value(self, &head)?;
            original.current_for_value(self, &operand)?;
            original.check(self)
        })();
        // Native Tcl decrements objv[0], then objv[1], before inspecting the
        // helper status or resetting its result. A callback can have changed
        // either primary, so both cleanup stages run even after first refusal.
        drop(head);
        let head_cleanup = original.check(self);
        drop(operand);
        let operand_cleanup = original.check(self);
        returned?;
        head_cleanup?;
        operand_cleanup?;
        if completion.code != Code::Ok {
            completion.code = Code::Error;
            return Ok(Some(completion));
        }
        drop(completion);
        self.reset_native_jim_result()?;
        original.check(self)?;
        Ok(None)
    }

    /// `namespace import` for `pattern` (e.g. `::tcltest::*`): alias every
    /// exported command of the source namespace matching the glob into the
    /// current namespace under its tail name. Returns the imported tail names.
    #[cfg(test)]
    pub(crate) fn import_commands(
        &mut self,
        pattern: impl AsRef<[u8]>,
        allow_overwrite: bool,
    ) -> Result<Vec<NameBytes>, NamespaceImportError> {
        let original = native_name_consumers::OriginalNameConsumerScope::enter(self)
            .map_err(NamespaceImportError::Original)?;
        let parts = self
            .namespace_pattern_parts(
                pattern.as_ref(),
                tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern,
            )
            .map_err(|error| NamespaceImportError::Host(error.to_string()))?;
        let source = self
            .namespace_pattern_source(&parts)
            .ok_or_else(|| NamespaceImportError::Host("original test import source".into()))?;
        let selected = tcl_cmd_core::namespace::ImportPattern {
            source,
            tail: parts.tail.as_bytes().to_vec(),
        };
        let reached = self.import_commands_selected(
            parts.selected.as_bytes(),
            &selected,
            allow_overwrite,
            &original,
        );
        original
            .finish(self)
            .map_err(NamespaceImportError::Original)?;
        reached
    }

    /// Publish the commands of the one source already selected by the shared
    /// checked import gate. Neither report bytes nor a second lookup selects it.
    pub(crate) fn import_commands_selected(
        &mut self,
        pattern: &[u8],
        selected: &tcl_cmd_core::namespace::ImportPattern,
        allow_overwrite: bool,
        original: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<Vec<NameBytes>, NamespaceImportError> {
        original
            .check(self)
            .map_err(NamespaceImportError::Original)?;
        let source_scope = native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
            self,
            Some(selected.source),
        )
        .map_err(NamespaceImportError::Original)?;
        let reached = (|| {
            let glob = selected.tail.as_slice();
            let src_ns_id = selected.source;
            let exports = self
                .retained_record_of(src_ns_id)
                .and_then(|record| record.exports.get(&src_ns_id).cloned())
                .or_else(|| self.ns_exports.get(&src_ns_id).cloned())
                .unwrap_or_default();
            // Candidate commands: those in the source namespace whose tail matches
            // the import glob and an export pattern.
            let to_import = self.import_candidates(src_ns_id, glob, &exports)?;
            let mut imported = Vec::new();
            for candidate in to_import {
                if let Some(tail) = self.import_selected_command(
                    candidate,
                    pattern,
                    allow_overwrite,
                    original,
                    &source_scope,
                )? {
                    imported.push(tail);
                }
            }
            if !imported.is_empty() {
                self.invalidate_namespace_structure_guards();
            }
            Ok(imported)
        })();
        source_scope
            .finish(self)
            .map_err(NamespaceImportError::Original)?;
        original
            .check(self)
            .map_err(NamespaceImportError::Original)?;
        reached
    }

    fn import_selected_command(
        &mut self,
        candidate: ImportedSourceCommand,
        pattern: &[u8],
        allow_overwrite: bool,
        original: &native_name_consumers::OriginalNameConsumerScope,
        source_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<Option<NameBytes>, NamespaceImportError> {
        let (tail, cmd, builtin_identity, source_token) = candidate;
        original
            .check(self)
            .map_err(NamespaceImportError::Original)?;
        source_scope
            .check(self)
            .map_err(NamespaceImportError::Original)?;
        let source_current =
            self.command_token_identity(&source_token.key).as_ref() == Some(&source_token);
        source_scope
            .require(
                self,
                source_current,
                "original C import source command token",
            )
            .map_err(NamespaceImportError::Original)?;
        let alias_display = NameBytes::from(self.command_slot_display_bytes(&CommandSlot {
            namespace: self.current_ns_id(),
            simple: tail.clone(),
        }));
        let destination = CommandSlot {
            namespace: self
                .resolution_stacks
                .ns_id_stack
                .last()
                .and_then(NamespaceExecutionContext::resolved_token)
                .expect("operation requires a materialized current namespace"),
            simple: tail.clone(),
        };
        let existing_key = self.command_storage_key_at_slot(&destination);
        let overwritten = existing_key
            .as_ref()
            .and_then(|key| self.command_token_identity(&CommandSidecarKey::visible(key.as_str())));
        if let Some(overwritten) = overwritten.as_ref()
            && !allow_overwrite
        {
            let same_source = existing_key.as_ref().is_some_and(|key| {
                self.visible_import_binding(key).is_some_and(|binding| {
                    binding.generation == overwritten.generation && binding.origin == source_token
                })
            });
            if same_source {
                return Ok(None);
            }
            return Err(NamespaceImportError::Overwrite { command: tail });
        }
        // C follows an imported source token's real-command chain before
        // replacing an existing destination binding. If that chain
        // reaches the token about to be overwritten, the new edge would
        // form an ImportRef cycle (Tcl_Import, tclNamesp.c). Reject it at
        // the graph-owner seam so origin lookup, replacement propagation,
        // and depth-first retirement all retain their acyclic invariant.
        if let Some(overwritten) = overwritten
            && self.import_source_chain_contains(&source_token, &overwritten)
        {
            return Err(NamespaceImportError::Loop {
                pattern: NameBytes::from(pattern),
                command: alias_display,
            });
        }
        let destination_is_retained = self
            .name_world
            .borrow()
            .dead_namespaces
            .contains(&destination.namespace);
        let alias = self.register_command_in_slot(destination, cmd);
        // Replacement deletion callbacks run inside the existing command
        // publisher. Preserve first refusal before stamping any ImportRef.
        original
            .check(self)
            .map_err(NamespaceImportError::Original)?;
        source_scope
            .check(self)
            .map_err(NamespaceImportError::Original)?;
        let source_current =
            self.command_token_identity(&source_token.key).as_ref() == Some(&source_token);
        source_scope
            .require(
                self,
                source_current,
                "original C import source after publication",
            )
            .map_err(NamespaceImportError::Original)?;
        if destination_is_retained {
            // Tcl accepts `namespace import` while the current namespace
            // token is retained, but the dying namespace cannot acquire a
            // new ImportRef. Drop the unstamped dispatcher clone without
            // retiring the source implementation it shares.
            self.materialise_retained_binding(&alias);
            self.take_command_unchecked_key(&alias);
            return Ok(Some(tail));
        }
        // `register_command` cleared any stale provenance; now stamp this key
        // as an import so `namespace forget` can target it.
        let binding =
            self.fresh_import_binding(&CommandSidecarKey::visible(alias.as_str()), source_token);
        self.set_visible_import_binding(&alias, binding);
        if let Some(identity) = builtin_identity {
            self.set_visible_builtin_identity(&alias, identity);
        }
        Ok(Some(tail))
    }

    /// The pinned Jim helper queries the root alias table, owns the same
    /// stored Prefix List, compares that whole List, then queries it as the
    /// next command name. A multiword prefix does not become its first member.
    fn check_jim_import_alias_cycle(
        &mut self,
        source: &Value,
        destination: &Value,
        original: &[u8],
        operation: &native_operation_currency::NativeOperationScope,
    ) -> Result<(), Completion<Value>> {
        let mut visited = HashSet::new();
        let mut alias = source.clone();
        loop {
            let lookup =
                tcl_runtime_api::Aliases::alias_prefix_original_value_at(self, ROOT_NS, &alias);
            if let Err(error) = operation.check(self) {
                return Err(crate::command::completion_from_cmd_error(self, error));
            }
            let next = match lookup {
                Ok(
                    tcl_runtime_api::AliasPrefixLookup::MissingCommand
                    | tcl_runtime_api::AliasPrefixLookup::NotAlias,
                ) => break,
                Ok(tcl_runtime_api::AliasPrefixLookup::Prefix(next)) => next,
                Err(error) => {
                    return Err(crate::command::completion_from_cmd_error(self, error));
                }
            };
            let resident = alias.native_object_snapshot().resident.ok_or_else(|| {
                crate::command::completion_from_cmd_error(
                    self,
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim import reached original alias spelling",
                    )
                    .into(),
                )
            })?;
            if !visited.insert(resident) {
                return Err(crate::command::completion_from_cmd_error(
                    self,
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim namespace import cyclic alias chain",
                    )
                    .into(),
                ));
            }
            alias = next;
            let equal = tcl_syntax::native_equality::jim_string_object_equality_using(
                self,
                &alias,
                destination,
                &|vm, original| {
                    native_numeric_ops::OriginalNumericOps::in_original_scope(vm, operation)?
                        .current_for_value(original)
                },
                &mut |vm, original| {
                    vm.native_name_operand_bytes_checked_using(original, &|vm| operation.check(vm))
                },
            );
            if let Err(error) = operation.check(self) {
                return Err(crate::command::completion_from_cmd_error(self, error));
            }
            if equal.map_err(|error| crate::command::completion_from_cmd_error(self, error))? {
                let mut message = b"import pattern \"".to_vec();
                message.extend_from_slice(original);
                message.extend_from_slice(b"\" would create a loop");
                return Err(crate::command::completion_from_cmd_error(
                    self,
                    tcl_cmd_core::CmdError::new_bytes(message),
                ));
            }
        }
        Ok(())
    }

    fn jim_import_candidates(
        &mut self,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        qualified: &[u8],
    ) -> Result<Vec<NameBytes>, Completion<Value>> {
        let matcher = tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy);
        let mut candidates = Vec::new();
        let keys: Vec<_> = self.commands.keys().cloned().collect();
        for key in keys {
            let Some(slot) = self.command_slot(&key) else {
                continue;
            };
            match matcher.match_name_pattern(
                tcl_syntax::native_glob::NativeNameGlobPurpose::InfoCommandsSearch,
                qualified,
                slot.simple.as_bytes(),
            ) {
                Ok(true) => candidates.push(slot.simple),
                Ok(false) => {}
                Err(error) => return Err(self.refuse_host_command(error.to_string())),
            }
        }
        Ok(candidates)
    }

    fn jim_import_pattern(
        &mut self,
        context: NsId,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        namespace: &[u8],
        original: &[u8],
    ) -> Result<Vec<u8>, Completion<Value>> {
        let path = self.ns_path(context);
        let qualified = match policy.recipe().jim_namespace_canonical_input(
            tcl_syntax::naming::NativeNameContext::with_jim_namespace(&path, namespace),
            original,
        ) {
            Ok(projected) => projected.selected().to_vec(),
            Err(error) => {
                return Err(self.refuse_host_command(format!(
                    "native Jim import canonicalisation unavailable: {error:?}"
                )));
            }
        };
        Ok(qualified)
    }

    pub(crate) fn import_source_names(
        &mut self,
        original: &[u8],
        operation: &native_operation_currency::NativeOperationScope,
    ) -> Completion<Value> {
        let context = self.current_ns_id();
        let Some(policy) = self.name_policy_protocol() else {
            return self
                .refuse_host_command("native Jim import name protocol is unavailable".into());
        };
        let holder =
            self.with_jim_namespace_at(context, |original| Ok(original.native_lifetime_lease()));
        if let Err(error) = operation.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }
        let holder = match holder {
            Ok(holder) => holder,
            Err(error) => return crate::command::completion_from_cmd_error(self, error.into()),
        };
        let namespace = tcl_syntax::value::ValueOps::native_string_bytes(self, holder.value());
        if let Err(error) = operation.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }
        let namespace = match namespace {
            Ok(namespace) => namespace,
            Err(error) => return crate::command::completion_from_cmd_error(self, error.into()),
        };
        let qualified = match self.jim_import_pattern(context, policy, namespace.as_ref(), original)
        {
            Ok(qualified) => qualified,
            Err(error) => return error,
        };
        let candidates = match self.jim_import_candidates(policy, &qualified) {
            Ok(candidates) => candidates,
            Err(error) => return error,
        };
        for source in candidates {
            let holder = policy.recipe().namespace_qualifier_bytes(source.as_bytes());
            let tail = policy.recipe().namespace_tail_bytes(source.as_bytes());
            if holder == namespace.as_ref() {
                let mut message = b"import pattern \"".to_vec();
                message.extend_from_slice(original);
                message.extend_from_slice(b"\" tries to import from namespace \"");
                message.extend_from_slice(namespace.as_ref());
                message.extend_from_slice(b"\" into itself");
                return crate::command::completion_from_cmd_error(
                    self,
                    tcl_cmd_core::CmdError::new_bytes(message),
                );
            }
            // The pinned Jim namespace helper forms ${current}::tail from
            // its actual canonical namespace object, without another root.
            let mut written = namespace.as_ref().to_vec();
            written.extend_from_slice(b"::");
            written.extend_from_slice(tail);
            let publication = match policy
                .recipe()
                .alias_publication_input(tcl_syntax::naming::NativeNameContext::root(), &written)
            {
                Ok(projected) => {
                    NameBytes::from(projected.jim_flat_key().expect("actual Jim publication"))
                }
                Err(error) => {
                    return self.refuse_host_command(format!(
                        "native Jim import alias publication unavailable: {error:?}"
                    ));
                }
            };
            let destination = CommandSlot {
                namespace: ROOT_NS,
                simple: publication,
            };
            let source_object = Value::from_native_string_bytes(source.as_bytes());
            let destination_object = Value::from_native_string_bytes(written.as_slice());
            if let Err(error) = self.check_jim_import_alias_cycle(
                &source_object,
                &destination_object,
                original,
                operation,
            ) {
                return error;
            }
            if let Err(error) = operation.check(self) {
                return crate::command::completion_from_cmd_error(self, error);
            }
            let report = match self.jim_command_table_key_for_original(
                &written,
                tcl_syntax::naming::NativeNamePurpose::AliasPublication,
            ) {
                Ok(report) => report,
                Err(error) => return self.refuse_host_command(error.to_string()),
            };
            self.register_command_in_slot_with_jim_key(
                destination,
                Command::CallerAlias(Value::native_list_constructor(
                    vec![source_object],
                    tcl_syntax::native_string::NativeStringProtocol::Jim084,
                )),
                report,
            );
            if let Err(error) = operation.check(self) {
                return crate::command::completion_from_cmd_error(self, error);
            }
        }
        ok(Value::empty())
    }

    /// Whether following imported-command origins from `source` reaches
    /// `needle`. A visited-token set keeps this invariant check finite even if
    /// handed malformed legacy provenance; normal construction rejects such a
    /// cycle before installing it.
    fn import_source_chain_contains(
        &self,
        source: &CommandTokenIdentity,
        needle: &CommandTokenIdentity,
    ) -> bool {
        let mut current = source.clone();
        let mut visited = HashSet::new();
        while visited.insert(current.clone()) {
            let next = match &current.key {
                CommandSidecarKey::Visible(name) => self.visible_import_binding(name),
                CommandSidecarKey::Hidden(name) => self.hidden.imports.get(name).cloned(),
            };
            let Some(binding) = next.filter(|binding| binding.generation == current.generation)
            else {
                return false;
            };
            current = binding.origin.clone();
            if current == *needle {
                return true;
            }
        }
        false
    }
}
