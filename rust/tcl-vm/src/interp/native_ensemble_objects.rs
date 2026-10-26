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

#[derive(Clone, Copy)]
pub(super) struct OriginalEnsembleInvocation<'a> {
    pub namespace: super::NsId,
    pub parameters: &'a [Value],
    pub arguments: &'a [Value],
    pub original_word_count: usize,
    pub usage: &'a [crate::command::NativeArgumentUsageRewrite],
    pub default_target: bool,
}

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
        let mut epochs = self
            .compilation_epochs
            .native_ensemble_namespace_epochs
            .borrow_mut();
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

    fn original_ensemble_invocation_list(
        prefix: &Value,
        selected: OriginalEnsembleInvocation<'_>,
        always_copy: bool,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Value, ValueError> {
        if always_copy || selected.original_word_count == 2 {
            prefix.native_list_copy(protocol)
        } else {
            let members = prefix.native_object_list_elements(protocol)?;
            let mut words = members.to_vec();
            words.extend_from_slice(selected.parameters);
            words.extend_from_slice(selected.arguments);
            Ok(Value::native_list_constructor(words, protocol))
        }
    }

    fn original_ensemble_target_missing(
        &mut self,
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
        namespace: super::NsId,
        lookup: super::NsId,
        words: &[Value],
        default_target: bool,
    ) -> Result<bool, tcl_core_types::Completion<Value>> {
        let selected_current = self.native_namespace_token_has_owner(namespace);
        if let Err(error) = original_names.require(
            self,
            selected_current,
            "selected original ensemble invocation namespace",
        ) {
            return Err(crate::command::completion_from_cmd_error(self, error));
        }
        let missing = if default_target {
            let bytes = match original_names.read(self, &words[0]) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return Err(crate::command::completion_from_cmd_error(self, error));
                }
            };
            let selected_current = self.native_namespace_token_has_owner(namespace);
            if let Err(error) = original_names.require(
                self,
                selected_current,
                "selected original ensemble namespace after head getter",
            ) {
                return Err(crate::command::completion_from_cmd_error(self, error));
            }
            match self.resolve_command_bytes_checked(lookup, &bytes, true) {
                Ok(selected) => selected.is_none(),
                Err(error) => return Err(self.refuse_host_command(error.to_string())),
            }
        } else {
            false
        };
        if let Err(error) = original_names.check(self) {
            return Err(crate::command::completion_from_cmd_error(self, error));
        }
        Ok(missing)
    }

    pub(super) fn invoke_original_ensemble_prefix(
        &mut self,
        prefix: Value,
        selected: OriginalEnsembleInvocation<'_>,
    ) -> (tcl_core_types::Completion<Value>, bool) {
        let original_names =
            match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self) {
                Ok(scope) => scope,
                Err(error) => {
                    return (
                        crate::command::completion_from_cmd_error(self, error),
                        false,
                    );
                }
            };
        let reached = (|| {
            let dialect = self.actual_native_invocation_dialect();
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
            let invocation = match Self::original_ensemble_invocation_list(
                &prefix,
                selected,
                dispatch.always_copy_prefix,
                protocol,
            ) {
                Ok(invocation) => invocation,
                Err(error) => return (self.refuse_host_command(error.to_string()), false),
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
                    selected
                        .parameters
                        .iter()
                        .chain(selected.arguments)
                        .map(|word| word.native_lifetime_lease().into_value()),
                );
            }
            let lookup = if dispatch.use_ensemble_namespace {
                selected.namespace
            } else {
                super::ROOT_NS
            };
            let missing = match self.original_ensemble_target_missing(
                &original_names,
                selected.namespace,
                lookup,
                &words,
                selected.default_target,
            ) {
                Ok(missing) => missing,
                Err(completion) => return (completion, false),
            };
            let result = self.invoke_command_value_internal_at(
                lookup,
                &words[0],
                &words[1..],
                selected.usage,
                tcl_registry::command_lookup::CommandLookupOrigin::EnsembleInvocation,
            );
            drop(words);
            drop(members);
            drop(invocation);
            drop(prefix);
            (result, missing)
        })();
        if let Err(error) = original_names.finish(self) {
            return (
                crate::command::completion_from_cmd_error(self, error),
                false,
            );
        }
        reached
    }
    pub(crate) fn prepare_original_ensemble_configuration(&self, def: &mut EnsembleDef) {
        let Some(recipe) = self
            .actual_native_invocation_dialect()
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
            if let Some(pairs) = pairs
                && let Ok(root) = Value::native_dictionary_constructor(pairs, None, protocol)
            {
                def.originals.map = Some(EnsembleObjectRole::new(NativeEnsembleRoot::owned(root)));
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
        configuration: tcl_registry::native_ensemble::NativeEnsembleConfigurationProtocol,
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
        configuration.table_plan(explicit.as_deref(), &keys, &exports, same)
    }

    pub(crate) fn build_original_ensemble_table(
        &self,
        def: &EnsembleDef,
        names: &[NameBytes],
    ) -> Result<(), ValueError> {
        // Physical ensemble objects use their actual engine, independently of source grammar.
        let dialect = self.actual_native_invocation_dialect();
        let configuration = dialect.native_ensemble_configuration_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("ensemble original configuration table"),
        )?;
        let protocol = dialect
            .native_string_materialization(None)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "ensemble original prefix table",
            ))?
            .protocol();
        let epoch = self
            .compilation_epochs
            .native_ensemble_namespace_epochs
            .borrow()
            .get(&def.namespace)
            .copied()
            .unwrap_or(0);
        if !def.originals.table.needs_build(epoch) {
            return Ok(());
        }
        if let Some(map) = &def.originals.map {
            drop(
                map.inspect(|root| root.inspect(|root| root.native_object_dict_pairs(protocol)))
                    .flatten()
                    .transpose()?,
            );
        }
        let plan = Self::native_original_ensemble_table_plan(configuration, def, names);
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

#[cfg(test)]
mod physical_ensemble_owner_tests {
    use super::*;

    fn answer(_vm: &mut Vm, _args: &[Value]) -> tcl_core_types::Completion<Value> {
        crate::interp::ok(Value::new_native_string_bytes(b"ANSWER".as_slice()))
    }

    #[test]
    fn original_physical_ensemble_table_and_prefix_keep_source_policy_independent() {
        // Actual software core and original table/prefix; no external C object measurement.
        let actual = crate::environment::profile_for_dialect("tcl9.0");
        let source = crate::environment::profile_for_dialect("tcl8.4");
        let mut vm = crate::native_fixture::core(actual);
        vm.set_dialect_profile(source);
        let original_config = vm.lexer_config();
        vm.register_stock_namespace_ensemble(
            "physical538",
            "::Physical538",
            &[("answer", answer)],
            &["answer"],
        );
        let completion = vm
            .try_invoke_command("physical538", &[Value::string("ans")])
            .unwrap();
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok);
        assert_eq!(
            completion.result.resident_string_bytes().unwrap().as_ref(),
            b"ANSWER"
        );
        assert_eq!(vm.lexer_config(), original_config);
        assert_eq!(vm.source_profile().cache_key(), source.cache_key());
        assert_eq!(
            vm.actual_native_execution_profile().cache_key(),
            actual.cache_key()
        );
        assert_eq!(vm.source_is_complete("set x ${a{}"), Ok(true));
        let completion = vm
            .try_invoke_command(
                "info",
                &[Value::string("complete"), Value::string("set x ${a{}")],
            )
            .unwrap();
        assert_eq!(
            completion.result.resident_string_bytes().unwrap().as_ref(),
            b"0"
        );
    }

    #[test]
    fn original_ensemble_table_refuses_source_profile_as_physical_configuration_owner() {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
        vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
        let def = EnsembleDef {
            originals: crate::command::native_ensemble_objects::NativeEnsembleObjects::default(),
            native: None,
            namespace: tcl_core_types::ROOT_NS,
            map: Vec::new(),
            subcommands: Some(vec![NameBytes::from("answer")]),
            prefixes: true,
            parameters: Vec::new(),
            unknown: None,
        };
        assert!(
            vm.native_invocation_dialect()
                .native_ensemble_configuration_protocol()
                .is_some()
        );
        assert!(
            vm.actual_native_invocation_dialect()
                .native_ensemble_configuration_protocol()
                .is_none()
        );
        assert!(matches!(
            vm.build_original_ensemble_table(&def, &[]),
            Err(ValueError::CommandProtocolUnavailable(
                "ensemble original configuration table"
            ))
        ));
    }
}
