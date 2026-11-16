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

//! The import graph after publication: origins, retirement nodes, replacing a source's implementation, retargeting on rename, and `namespace forget`.

use super::super::commands::CommandSidecarKey;
use super::super::{CommandTokenIdentity, ImportBinding, ImportRetirementNode, Vm};
use crate::command::Command;
use crate::value::Value;
use tcl_runtime_api::Completion;

impl Vm {
    /// The display name of the command `key` ultimately came from, following
    /// the `namespace import` chain to its source — C's
    /// `TclGetOriginalCommand`, which backs `namespace origin`. The import
    /// graph itself keeps the exact private placement key; rendering happens
    /// only at this presentation boundary.
    #[cfg(test)]
    pub(crate) fn command_origin_key(&self, key: &str) -> String {
        match self.ultimate_import_origin(&CommandSidecarKey::visible(key)) {
            CommandSidecarKey::Visible(origin)
                if self
                    .command_token_identity(&CommandSidecarKey::visible(&origin))
                    .is_some() =>
            {
                self.command_display_key(&origin)
            }
            CommandSidecarKey::Visible(origin) | CommandSidecarKey::Hidden(origin) => origin,
        }
    }

    /// Follow a namespace-import chain without collapsing the visible/hidden
    /// identity domain. A visible replacement may legitimately have the same
    /// spelling as a hidden source token, so lifecycle maintenance must compare
    /// the complete key rather than only [`CommandSidecarKey::name`].
    pub(in super::super) fn ultimate_import_origin(
        &self,
        start: &CommandSidecarKey,
    ) -> CommandSidecarKey {
        let Some(mut current) = self.command_token_identity(start) else {
            return start.clone();
        };
        // A valid chain cannot contain more edges than there are imported
        // bindings. Keep malformed/cyclic provenance finite as a defensive
        // invariant at this internal boundary.
        let retained_imports = self
            .name_world
            .borrow()
            .ns_deferral
            .retained
            .values()
            .map(|record| record.imported.len())
            .sum::<usize>();
        for _ in 0..self.imported_commands.len() + self.hidden.imports.len() + retained_imports {
            let next = match &current.key {
                CommandSidecarKey::Visible(visible) => self.visible_import_binding(visible),
                CommandSidecarKey::Hidden(hidden) => self.hidden.imports.get(hidden).cloned(),
            };
            let Some(next) = next.filter(|binding| binding.generation == current.generation) else {
                break;
            };
            current = next.origin.clone();
        }
        current.key
    }

    /// Snapshot the direct `ImportRef` list of one exact source token in C's
    /// newest-first order. Callers query this *after* the source token's delete
    /// trace, so an import created by that callback against the still-live
    /// dying token joins the cascade. A same-name replacement has a different
    /// source generation and its imports do not.
    pub(in super::super) fn direct_import_retirement_nodes(
        &self,
        source: &CommandTokenIdentity,
    ) -> Vec<ImportRetirementNode> {
        let mut direct: Vec<ImportRetirementNode> = self
            .imported_commands
            .iter()
            .filter(|(_, binding)| binding.origin == *source)
            .filter_map(|(name, binding)| {
                let key = CommandSidecarKey::visible(name);
                self.command_token_identity(&key)
                    .map(|token| ImportRetirementNode {
                        token,
                        binding: binding.clone(),
                    })
            })
            .chain(
                self.hidden
                    .imports
                    .iter()
                    .filter(|(_, binding)| binding.origin == *source)
                    .filter_map(|(name, binding)| {
                        let key = CommandSidecarKey::hidden(name);
                        self.command_token_identity(&key)
                            .map(|token| ImportRetirementNode {
                                token,
                                binding: binding.clone(),
                            })
                    }),
            )
            .collect();
        for record in self.name_world.borrow().ns_deferral.retained.values() {
            direct.extend(
                record
                    .imported
                    .iter()
                    .filter(|(_, binding)| binding.origin == *source)
                    .filter_map(|(name, binding)| {
                        let key = CommandSidecarKey::visible(name);
                        self.command_token_identity(&key)
                            .map(|token| ImportRetirementNode {
                                token,
                                binding: binding.clone(),
                            })
                    }),
            );
        }
        direct.sort_by_key(|node| std::cmp::Reverse(node.binding.generation));
        direct
    }

    /// Find an imported command token after a re-entrant callback may have
    /// renamed, hidden, or exposed it. Command generations are interpreter-
    /// unique and travel with those relocations.
    pub(in super::super) fn import_retirement_node_by_generation(
        &self,
        generation: u64,
    ) -> Option<ImportRetirementNode> {
        self.imported_commands
            .iter()
            .find(|(_, binding)| binding.generation == generation)
            .and_then(|(name, binding)| {
                let key = CommandSidecarKey::visible(name);
                self.command_token_identity(&key)
                    .filter(|token| token.generation == generation)
                    .map(|token| ImportRetirementNode {
                        token,
                        binding: binding.clone(),
                    })
            })
            .or_else(|| {
                self.hidden
                    .imports
                    .iter()
                    .find(|(_, binding)| binding.generation == generation)
                    .and_then(|(name, binding)| {
                        let key = CommandSidecarKey::hidden(name);
                        self.command_token_identity(&key)
                            .filter(|token| token.generation == generation)
                            .map(|token| ImportRetirementNode {
                                token,
                                binding: binding.clone(),
                            })
                    })
            })
            .or_else(|| {
                self.name_world
                    .borrow()
                    .ns_deferral
                    .retained
                    .values()
                    .find_map(|record| {
                        record
                            .imported
                            .iter()
                            .find(|(_, binding)| binding.generation == generation)
                            .and_then(|(name, binding)| {
                                let key = CommandSidecarKey::visible(name);
                                self.command_token_identity(&key)
                                    .filter(|token| token.generation == generation)
                                    .map(|token| ImportRetirementNode {
                                        token,
                                        binding: binding.clone(),
                                    })
                            })
                    })
            })
    }

    pub(in super::super) fn downstream_import_bindings(
        &self,
        source: &CommandTokenIdentity,
    ) -> Vec<CommandSidecarKey> {
        fn append(vm: &Vm, source: &CommandTokenIdentity, keys: &mut Vec<CommandSidecarKey>) {
            for node in vm.direct_import_retirement_nodes(source) {
                keys.push(node.token.key.clone());
                append(vm, &node.token, keys);
            }
        }
        let mut keys = Vec::new();
        append(self, source, &mut keys);
        keys
    }

    /// Rebind all imports of a real command after an atomic implementation
    /// replacement at the same source key.
    ///
    /// Tcl keeps the source command token and makes existing imports observe
    /// the replacement implementation. The VM stores dispatcher clones, so it
    /// mirrors that update explicitly while leaving provenance, traces, and
    /// visible/hidden placement untouched.
    pub(in super::super) fn replace_import_implementations(
        &mut self,
        replaced: &CommandTokenIdentity,
        installed: &CommandTokenIdentity,
        command: &Command,
    ) {
        let bindings = self.downstream_import_bindings(replaced);
        let builtin_identity = matches!(command, Command::Builtin(_) | Command::Native(_))
            .then(|| installed.key.name().to_owned());
        for binding in self.imported_commands.values_mut() {
            if binding.origin == *replaced {
                binding.origin.clone_from(installed);
            }
        }
        for binding in self.hidden.imports.values_mut() {
            if binding.origin == *replaced {
                binding.origin.clone_from(installed);
            }
        }
        for record in self
            .name_world
            .borrow_mut()
            .ns_deferral
            .retained
            .values_mut()
        {
            for binding in record.imported.values_mut() {
                if binding.origin == *replaced {
                    binding.origin.clone_from(installed);
                }
            }
        }
        for key in bindings {
            match key {
                CommandSidecarKey::Visible(name) => {
                    let retained_root = self.retained_root_for_visible_key(&name).filter(|root| {
                        self.name_world
                            .borrow()
                            .ns_deferral
                            .retained
                            .get(root)
                            .is_some_and(|record| record.commands.contains_key(&name))
                    });
                    if let Some(root) = retained_root {
                        let mut world = self.name_world.borrow_mut();
                        let record = world
                            .ns_deferral
                            .retained
                            .get_mut(&root)
                            .expect("retained command owner exists");
                        record.commands.insert(name.clone(), command.clone());
                        if let Some(identity) = &builtin_identity {
                            record.builtin_identities.insert(name, identity.clone());
                        } else {
                            record.builtin_identities.remove(&name);
                        }
                    } else {
                        self.commands.insert(name.clone(), command.clone());
                        if let Some(identity) = &builtin_identity {
                            self.builtin_identities.insert(name, identity.clone());
                        } else {
                            self.builtin_identities.remove(&name);
                        }
                    }
                }
                CommandSidecarKey::Hidden(name) => {
                    self.hidden.commands.insert(name.clone(), command.clone());
                    if let Some(identity) = &builtin_identity {
                        self.hidden
                            .builtin_identities
                            .insert(name, identity.clone());
                    } else {
                        self.hidden.builtin_identities.remove(&name);
                    }
                }
            }
        }
    }

    /// Remove one imported binding and every location-keyed sidecar without
    /// touching the shared implementation token. The source command owns that
    /// token; deleting an alias alone must not mark the ensemble dead.
    pub(in super::super) fn retire_import_binding(&mut self, key: &CommandSidecarKey) {
        let Some(binding) = (match key {
            CommandSidecarKey::Visible(name) => self.visible_import_binding(name),
            CommandSidecarKey::Hidden(name) => self.hidden.imports.get(name).cloned(),
        }) else {
            return;
        };

        let Some(token) = self.command_token_identity(key) else {
            return;
        };
        let node = ImportRetirementNode { token, binding };
        if let CommandSidecarKey::Visible(name) = key {
            self.materialise_retained_binding(name);
        }
        self.retire_import_tree_snapshot(&node);
    }

    /// Retarget imports when their source command is renamed.  The command
    /// table stores an import as a cloned dispatcher, whereas C stores the
    /// source command token; rewrite the provenance explicitly so dispatch,
    /// visibility, and `namespace origin` continue to identify the final
    /// builtin under its new name. True deletion does not retarget: it follows
    /// the separate downstream-import retirement path above.
    pub(in super::super) fn retarget_imports_key(
        &mut self,
        old_key: &CommandSidecarKey,
        new_key: &CommandSidecarKey,
    ) {
        for binding in self.imported_commands.values_mut() {
            if binding.origin.key == *old_key {
                binding.origin.key.clone_from(new_key);
            }
        }
        for binding in self.hidden.imports.values_mut() {
            if binding.origin.key == *old_key {
                binding.origin.key.clone_from(new_key);
            }
        }
        for record in self
            .name_world
            .borrow_mut()
            .ns_deferral
            .retained
            .values_mut()
        {
            for binding in record.imported.values_mut() {
                if binding.origin.key == *old_key {
                    binding.origin.key.clone_from(new_key);
                }
            }
        }
    }

    /// Restore a renamed imported command's binding identity and origin edge.
    pub(in super::super) fn restore_import_binding(&mut self, key: &str, binding: ImportBinding) {
        self.set_visible_import_binding(key, binding);
    }

    /// Restore a renamed builtin's stable registry identity.
    pub(crate) fn restore_builtin_identity(&mut self, key: &str, identity: String) {
        self.set_visible_builtin_identity(key, identity);
    }

    /// Record `key` as an engine-installed `TclOO` root object command, so the
    /// release-availability gate treats it like a builtin (see
    /// [`InterpState::registry_object_roots`]).
    pub(crate) fn declare_registry_object_root(&mut self, key: &str) {
        self.declare_registry_object_root_as(key, key);
    }

    /// [`Self::declare_registry_object_root`] with an explicit registry
    /// identity, for a root that has been renamed away from the name the
    /// registry dates it by.
    pub(crate) fn declare_registry_object_root_as(&mut self, key: &str, identity: &str) {
        self.registry_object_roots
            .insert(key.to_owned(), identity.to_owned());
    }

    /// `namespace forget pattern` — remove previously imported commands matching
    /// `pattern` from the current namespace (C `Tcl_ForgetImport`). Only commands
    /// created by `namespace import` are removed; a real command of the same name
    /// is left intact. A simple (unqualified) pattern matches imported commands
    /// in the current namespace by name; a qualified `ns::pat` pattern matches
    /// those whose origin lives in `ns` and whose origin tail matches `pat`.
    /// Returns `Err` for an unknown namespace in a qualified pattern.
    pub(crate) fn forget_imports(
        &mut self,
        original: impl AsRef<[u8]>,
    ) -> Result<(), Completion<Value>> {
        self.invalidate_namespace_structure_guards();
        let parts = self
            .namespace_pattern_parts(
                original.as_ref(),
                tcl_syntax::naming::NativeNamePurpose::NamespaceForgetPattern,
            )
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let pattern = parts.selected.as_bytes();
        let cur = self.current_ns_id();
        let mut current_imports: Vec<String> = self
            .imported_commands
            .keys()
            .cloned()
            .chain(
                self.retained_record_of(cur)
                    .into_iter()
                    .flat_map(|record| record.imported.keys().cloned().collect::<Vec<_>>()),
            )
            .filter(|key| {
                self.command_slot(key)
                    .is_some_and(|slot| slot.namespace == cur)
            })
            .collect();
        current_imports.sort();
        current_imports.dedup();
        let victims: Vec<String> = if parts.source.is_some() {
            let simple = parts.tail.as_bytes();
            let Some(src_ns) = self.namespace_pattern_source(&parts) else {
                let mut message = b"unknown namespace in namespace forget pattern \"".to_vec();
                message.extend_from_slice(pattern);
                message.push(b'"');
                return Err(crate::command::completion_from_cmd_error(
                    self,
                    tcl_cmd_core::CmdError::new_bytes(message),
                ));
            };
            current_imports
                .iter()
                .filter(|key| {
                    // C's TclGetOriginalCommand follows an import chain;
                    // qualified forget matches that ultimate source, not
                    // merely the immediately imported alias.
                    let origin =
                        self.ultimate_import_origin(&CommandSidecarKey::visible(key.as_str()));
                    matches!(origin, CommandSidecarKey::Visible(ref origin_key)
                            if self.command_slot(origin_key).is_some_and(|slot|
                                slot.namespace == src_ns
                                    && self.native_namespace_match(tcl_syntax::native_glob::NativeNameGlobPurpose::ForgetOriginFilter, simple, slot.simple.as_bytes()).expect("audited C CString forget filter")))
                })
                .cloned()
                .collect()
        } else {
            // Simple pattern: imported commands in the current namespace whose
            // own name matches.
            current_imports
                .iter()
                .filter(|key| {
                    self.command_slot(key).is_some_and(|slot| {
                        self.native_namespace_match(
                            tcl_syntax::native_glob::NativeNameGlobPurpose::ForgetOwnSearch,
                            pattern,
                            slot.simple.as_bytes(),
                        )
                        .expect("audited C CString forget search")
                    })
                })
                .cloned()
                .collect()
        };
        for key in victims {
            self.retire_import_binding(&CommandSidecarKey::visible(key));
        }
        Ok(())
    }
}
