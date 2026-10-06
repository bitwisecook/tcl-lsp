// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native ensemble object production and prefix-table preparation.

use super::Vm;
use crate::{
    Value,
    command::{EnsembleDef, native_ensemble_objects::NativeEnsembleRoot},
};
use tcl_cmd_core::ensemble::EnsembleObjectRole;
use tcl_core_types::NameBytes;
use tcl_syntax::value::ValueError;

impl Vm {
    pub(super) fn borrow_native_argument_usage(
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Vec<crate::command::NativeArgumentUsageRewrite> {
        usage
            .iter()
            .map(|rewrite| crate::command::NativeArgumentUsageRewrite {
                original_prefix: rewrite
                    .original_prefix
                    .iter()
                    .map(|word| word.native_lifetime_lease().into_value())
                    .collect(),
                removed_words: rewrite.removed_words,
            })
            .collect()
    }
    pub(super) fn advance_native_ensemble_export_epoch(&self, namespace: super::NsId) {
        let mut epochs = self.native_ensemble_namespace_epochs.borrow_mut();
        let epoch = epochs.entry(namespace).or_default();
        *epoch = epoch
            .checked_add(1)
            .expect("native ensemble export epoch exhausted");
    }
    pub(super) fn native_ensemble_namespace_has_exports(&self, namespace: super::NsId) -> bool {
        let world = self.name_world.borrow();
        let retained = world
            .ns_deferral
            .owners
            .get(&namespace)
            .and_then(|root| world.ns_deferral.retained.get(root))
            .and_then(|record| record.exports.get(&namespace));
        retained
            .or_else(|| self.ns_exports.get(&namespace))
            .is_some_and(|exports| !exports.is_empty())
    }

    pub(super) fn invoke_original_ensemble_prefix(
        &mut self,
        prefix: Value,
        namespace: super::NsId,
        parameters: &[Value],
        arguments: &[Value],
        original_word_count: usize,
        usage: &[crate::command::NativeArgumentUsageRewrite],
        default_target: bool,
    ) -> (tcl_core_types::Completion<Value>, bool) {
        let dialect = self.native_invocation_dialect();
        let Some(dispatch) = dialect.native_ensemble_dispatch_protocol() else {
            return (
                self.refuse_host_command("native ensemble forwarding protocol".into()),
                false,
            );
        };
        let Some(recipe) = dialect.native_string_materialization(None) else {
            return (
                self.refuse_host_command("native ensemble invocation List".into()),
                false,
            );
        };
        let protocol = recipe.protocol();
        let invocation = if dispatch.always_copy_prefix || original_word_count == 2 {
            match prefix.native_list_copy(protocol) {
                Ok(copy) => copy,
                Err(error) => return (self.refuse_host_command(error.to_string()), false),
            }
        } else {
            let members = match prefix.native_object_list_elements(protocol) {
                Ok(members) => members,
                Err(error) => return (self.refuse_host_command(error.to_string()), false),
            };
            let mut words = members.to_vec();
            words.extend_from_slice(parameters);
            words.extend_from_slice(arguments);
            Value::native_list_constructor(words, protocol)
        };
        let members = match invocation.native_object_list_elements(protocol) {
            Ok(members) => members,
            Err(error) => return (self.refuse_host_command(error.to_string()), false),
        };
        let prefix = if dispatch.always_copy_prefix {
            Some(prefix)
        } else {
            drop(prefix);
            None
        };
        let mut words: Vec<_> = members
            .iter()
            .map(|word| word.native_lifetime_lease().into_value())
            .collect();
        if dispatch.always_copy_prefix {
            words.extend(
                parameters
                    .iter()
                    .chain(arguments)
                    .map(|word| word.native_lifetime_lease().into_value()),
            );
        }
        let lookup = if dispatch.use_ensemble_namespace {
            namespace
        } else {
            super::ROOT_NS
        };
        let missing = if default_target {
            let bytes = match self.native_name_operand_bytes(&words[0]) {
                Ok(bytes) => bytes,
                Err(error) => return (self.refuse_host_command(error.to_string()), false),
            };
            match self.resolve_command_bytes_checked(lookup, &bytes, true) {
                Ok(selected) => selected.is_none(),
                Err(error) => return (self.refuse_host_command(error.to_string()), false),
            }
        } else {
            false
        };
        let result = self.invoke_command_value_internal_at(
            lookup,
            &words[0],
            &words[1..],
            usage,
            tcl_registry::command_lookup::CommandLookupOrigin::EnsembleInvocation,
        );
        drop(words);
        drop(members);
        drop(invocation);
        drop(prefix);
        (result, missing)
    }
    pub(crate) fn prepare_original_ensemble_configuration(&self, def: &mut EnsembleDef) {
        let Some(recipe) = self
            .native_invocation_dialect()
            .native_string_materialization(None)
        else {
            return;
        };
        let protocol = recipe.protocol();
        let list = |words: &[NameBytes]| {
            Value::native_list_constructor(
                words
                    .iter()
                    .map(|word| Value::from_native_string_bytes(word.as_bytes()))
                    .collect(),
                protocol,
            )
        };
        if def.originals.map.is_none() && !def.map.is_empty() {
            let pairs = def
                .map
                .iter()
                .map(|(name, words)| {
                    let words = words.iter().cloned().collect::<Option<Vec<_>>>()?;
                    Some((
                        Value::from_native_string_bytes(name.as_bytes()),
                        list(&words),
                    ))
                })
                .collect::<Option<Vec<_>>>();
            if let Some(pairs) = pairs {
                if let Ok(root) = Value::native_dictionary_constructor(pairs, None, protocol) {
                    def.originals.map =
                        Some(EnsembleObjectRole::new(NativeEnsembleRoot::owned(root)));
                }
            }
        }
        for (slot, words) in [
            (&mut def.originals.subcommands, def.subcommands.as_deref()),
            (
                &mut def.originals.parameters,
                (!def.parameters.is_empty()).then_some(def.parameters.as_slice()),
            ),
            (&mut def.originals.unknown, def.unknown.as_deref()),
        ] {
            if slot.is_none() {
                *slot = words
                    .map(|words| EnsembleObjectRole::new(NativeEnsembleRoot::owned(list(words))));
            }
        }
    }

    pub(super) fn native_original_ensemble_table_plan(
        &self,
        def: &EnsembleDef,
        exports: &[NameBytes],
    ) -> tcl_registry::native_ensemble::NativeEnsembleTablePlan {
        let keys: Vec<_> = def.map.iter().map(|(key, _)| key.as_bytes()).collect();
        let explicit = def
            .subcommands
            .as_ref()
            .map(|names| names.iter().map(NameBytes::as_bytes).collect::<Vec<_>>());
        let exports: Vec<_> = exports.iter().map(NameBytes::as_bytes).collect();
        let identity = |role: &Option<EnsembleObjectRole<NativeEnsembleRoot>>| {
            role.as_ref().and_then(|role| {
                role.inspect(|root| root.inspect(Value::native_object_identity))
                    .flatten()
            })
        };
        let map = identity(&def.originals.map);
        let same = map.is_some() && map == identity(&def.originals.subcommands);
        self.native_invocation_dialect()
            .native_ensemble_configuration_protocol()
            .expect("selected ensemble configuration protocol")
            .table_plan(explicit.as_deref(), &keys, &exports, same)
    }

    pub(crate) fn build_original_ensemble_table(
        &self,
        def: &EnsembleDef,
        names: &[NameBytes],
    ) -> Result<(), ValueError> {
        let epoch = self
            .native_ensemble_namespace_epochs
            .borrow()
            .get(&def.namespace)
            .copied()
            .unwrap_or(0);
        if !def.originals.table.needs_build(epoch) {
            return Ok(());
        }
        let protocol = self
            .native_invocation_dialect()
            .native_string_materialization(None)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "ensemble original prefix table",
            ))?
            .protocol();
        if let Some(map) = &def.originals.map {
            drop(
                map.inspect(|root| root.inspect(|root| root.native_object_dict_pairs(protocol)))
                    .flatten()
                    .transpose()?,
            );
        }
        let plan = self.native_original_ensemble_table_plan(def, names);
        let lookup = |index: usize| {
            def.originals.map.as_ref().and_then(|role| {
                role.inspect(|root| {
                    root.inspect(|root| {
                        root.with_cached_dictionary_member(def.map[index].0.as_bytes(), |prefix| {
                            prefix.cloned()
                        })
                        .flatten()
                    })
                    .flatten()
                })
                .flatten()
            })
        };
        // Native map-only replacement leaves the previous actual +1 unreleased.
        for index in plan.displaced_mappings {
            if let Some(prefix) = lookup(index) {
                std::mem::forget(prefix);
            }
        }
        let mut entries = Vec::new();
        for entry in plan.entries {
            let mapped = entry.mapping.and_then(lookup);
            let is_mapped = mapped.is_some();
            let name = NameBytes::from(entry.member);
            let prefix = if let Some(prefix) = mapped {
                prefix
            } else {
                let target = if def.subcommands.is_some() {
                    name.as_bytes().to_vec()
                } else {
                    let mut target = b"::".to_vec();
                    target.extend_from_slice(&self.command_slot_display_bytes(
                        &super::CommandSlot {
                            namespace: def.namespace,
                            simple: name.clone(),
                        },
                    ));
                    target
                };
                Value::native_list_constructor(
                    vec![Value::from_native_string_bytes(target)],
                    protocol,
                )
            };
            entries.push((name, prefix, is_mapped));
        }
        def.originals.table.replace(entries, epoch);
        Ok(())
    }
}
