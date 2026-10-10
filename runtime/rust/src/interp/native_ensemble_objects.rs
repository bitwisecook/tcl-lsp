// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configured ensemble roots and original native forwarding storage.

use super::{Command, Interp};
use crate::{
    dict,
    ensemble::{EnsembleConfig, NativeEnsembleObjects, NativeEnsembleRoot},
    list,
    namespace::GLOBAL,
    obj::{self, Owned, TclObj},
};
use tcl_cmd_core::ensemble::EnsembleObjectRole;
use tcl_syntax::value::ValueError;

impl Interp {
    pub(crate) fn retire_pending_native_ensemble_roles(&mut self) {
        self.retire_pending_children();
        loop {
            let retired = self.namespaces.borrow().take_retired_ensemble_roles();
            if retired.is_empty() {
                break;
            }
            for token in retired {
                token.mark_deleted();
            }
        }
    }

    /// Construct the real private objects installed by stock ensemble setup.
    /// Existing configured originals are never reconstructed from metadata.
    pub(super) fn prepare_original_ensemble_configuration(&self, cfg: &mut EnsembleConfig) {
        let Some(recipe) = self
            .eval_frame_dialect()
            .native_string_materialization(None)
        else {
            return;
        };
        let protocol = recipe.protocol();
        let make_list = |words: &[Vec<u8>]| {
            let members: Vec<_> = words
                .iter()
                .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)))
                .collect();
            let pointers: Vec<_> = members.iter().map(Owned::as_ptr).collect();
            Owned::fresh(list::new_list_obj_native(&pointers, protocol))
        };
        if cfg.originals.map.is_none() {
            if let Some(map) = &cfg.map {
                let members: Vec<_> = map
                    .iter()
                    .map(|(key, words)| {
                        (Owned::fresh(obj::new_string_bytes(key)), make_list(words))
                    })
                    .collect();
                let pairs: Vec<_> = members
                    .iter()
                    .map(|(k, v)| (k.as_ptr(), v.as_ptr()))
                    .collect();
                if let Ok(root) = dict::new_dict_obj_native(&pairs, None, protocol) {
                    cfg.originals.map = Some(EnsembleObjectRole::new(NativeEnsembleRoot::owned(
                        Owned::fresh(root),
                    )));
                }
            }
        }
        for (slot, words) in [
            (&mut cfg.originals.subcommands, cfg.subcommands.as_deref()),
            (
                &mut cfg.originals.parameters,
                (!cfg.parameters.is_empty()).then_some(cfg.parameters.as_slice()),
            ),
            (
                &mut cfg.originals.unknown,
                (!cfg.unknown.is_empty()).then_some(cfg.unknown.as_slice()),
            ),
        ] {
            if slot.is_none() {
                *slot = words.map(|words| {
                    EnsembleObjectRole::new(NativeEnsembleRoot::owned(make_list(words)))
                });
            }
        }
    }

    pub(super) fn build_original_ensemble_table(
        &mut self,
        cfg: &EnsembleConfig,
    ) -> Result<(), ValueError> {
        let epoch = self.namespaces.borrow().ensemble_export_epoch(cfg.ns);
        if !cfg.originals.table.needs_build(epoch) {
            return Ok(());
        }
        let protocol = self
            .eval_frame_dialect()
            .native_string_materialization(None)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "ensemble original prefix storage",
            ))?
            .protocol();
        let map = NativeEnsembleObjects::pointer(&cfg.originals.map)
            .map(|root| dict::native_dict_pairs(root, protocol))
            .transpose()?
            .unwrap_or_default();
        let configuration = self
            .native_invocation_dialect()
            .native_ensemble_configuration_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "ensemble member names",
            ))?;
        let counted_keys = map
            .iter()
            .map(|&(key, _)| dict::native_object_bytes(key, protocol))
            .collect::<Result<Vec<_>, _>>()?;
        let keys: Vec<_> = counted_keys.iter().map(Vec::as_slice).collect();
        let explicit = cfg
            .subcommands
            .as_ref()
            .map(|names| names.iter().map(Vec::as_slice).collect::<Vec<_>>());
        let exports = self.namespaces.borrow().exported_commands(cfg.ns);
        let exports: Vec<_> = exports.iter().map(Vec::as_slice).collect();
        let root = NativeEnsembleObjects::pointer(&cfg.originals.map);
        let same =
            root.is_some() && root == NativeEnsembleObjects::pointer(&cfg.originals.subcommands);
        let plan = configuration.table_plan(explicit.as_deref(), &keys, &exports, same);
        // BuildEnsembleConfig does not release overwritten map-only entries.
        // Transfer only these actually reached native references without a Drop.
        for index in plan.displaced_mappings {
            let _ = Owned::retain(map[index].1).into_raw();
        }
        let mut entries = Vec::new();
        for entry in plan.entries {
            let mapped = entry.mapping.map(|index| map[index].1);
            let name = entry.member;
            let prefix = if let Some(prefix) = mapped {
                Owned::retain(prefix)
            } else {
                let target = if cfg.subcommands.is_some() {
                    name.clone()
                } else {
                    let mut qualified = self.namespaces.borrow().qualified_name(cfg.ns);
                    if cfg.ns != GLOBAL {
                        qualified.extend_from_slice(b"::");
                    }
                    qualified.extend_from_slice(&name);
                    qualified
                };
                let head = Owned::fresh(obj::new_string_bytes(&target));
                Owned::fresh(list::new_list_obj_native(&[head.as_ptr()], protocol))
            };
            entries.push((name, prefix, mapped.is_some()));
        }
        cfg.originals.table.replace(entries, epoch);
        Ok(())
    }

    /// Borrow a resident original map target for the native compilation-entry
    /// observation owner. This does not convert/stringify or resolve a command.
    pub(crate) fn native_ensemble_original_target(
        &self,
        generation: u64,
        member: &[u8],
        position: usize,
    ) -> Option<*mut TclObj> {
        let Command::Ensemble(token) = self.raw_command_by_generation(generation)? else {
            return None;
        };
        if token.is_deleted() {
            return None;
        }
        let cfg = token.config();
        let root = NativeEnsembleObjects::pointer(&cfg.originals.map)?;
        if obj::obj_type_ptr(root) != &dict::TCL_DICT_TYPE {
            return None;
        }
        let prefix = dict::dict_get(root, member).ok()??;
        let backing = list::native_list_backing(prefix)?;
        let selected = backing.elements().ok()?.get(position).copied();
        selected
    }
}
