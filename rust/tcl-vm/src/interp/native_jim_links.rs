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

//! Jim links retain their original recursive target getter and actual frame.

use super::{Local, UpvarLinkError, Value, Vm};
use tcl_runtime_api::Completion;
use tcl_syntax::native_jim_lookup::NativeJimLinkTargetInput;

impl Vm {
    fn attach_original_jim_link(
        &mut self,
        local: &[u8],
        target_level: usize,
        original: &Value,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let target_frame = original_names
            .capture_frame(self, target_level)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let Some(protocol) = self
                .actual_native_invocation_dialect()
                .native_jim_lookup_protocol()
            else {
                return Ok(());
            };
            let selected_local = self.var_binding_from_bytes(local, self.current_level());
            let selected_cell = selected_local
                .as_ref()
                .map(|binding| original_names.capture_binding(self, binding))
                .transpose()
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let bytes = original_names
                .read(self, original)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            target_frame
                .check(self, &original_names)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            if let Some(selected) = &selected_cell {
                selected
                    .check(self, &original_names)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            }
            let result = (|| -> Result<(), UpvarLinkError> {
                let (level, name) = match protocol.link_target_input(&bytes) {
                    NativeJimLinkTargetInput::Original => (target_level, original.clone()),
                    NativeJimLinkTargetInput::StrippedGlobal(tail) => {
                        (0, Value::new_native_string_bytes(tail))
                    }
                };
                let frame = self
                    .frames
                    .get(level)
                    .ok_or(UpvarLinkError::TargetNamespace)?;
                let receipt = crate::vars::JimNameLinkOriginal {
                    name,
                    frame: frame.jim_storage.reference(),
                };
                let binding = self
                    .var_binding_from_bytes(local, self.current_level())
                    .ok_or(UpvarLinkError::LocalNamespace)?;
                let raw = self
                    .raw_variable_binding(&binding)
                    .ok_or(UpvarLinkError::LocalNamespace)?;
                let mut link = match self.var_arena.get(raw).map(crate::vars::VarCell::state) {
                    Some(Local::NameLink(link)) => link.clone(),
                    _ => return Ok(()),
                };
                link.original_target = Some(receipt);
                self.var_arena.replace_state(raw, Local::NameLink(link));
                Ok(())
            })();
            result.map_err(|error| self.original_jim_link_failure(error, &bytes, local))
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }
    pub(crate) fn link_upvar_original(
        &mut self,
        target_level: usize,
        other: &Value,
        local: &Value,
    ) -> Result<(), tcl_runtime_api::Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let target_frame = original_names
            .capture_frame(self, target_level)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            if self.native_c_variable_name_protocol().is_some() {
                return self.link_original_c_variable_objects(other, target_level, None, local);
            }
            let bytes = original_names
                .read(self, other)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            target_frame
                .check(self, &original_names)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let local_bytes = original_names
                .read(self, local)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            target_frame
                .check(self, &original_names)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            (|| {
                self.link_upvar_bytes(target_level, &bytes, &local_bytes)
                    .map_err(|error| self.original_jim_link_failure(error, &bytes, &local_bytes))?;
                self.attach_original_jim_link(&local_bytes, target_level, other)?;
                self.retain_original_jim_link_local(local, &local_bytes, &original_names)
            })()
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }
    pub(crate) fn add_link_original(
        &mut self,
        local: &[u8],
        level: usize,
        target: &Value,
    ) -> Result<(), tcl_runtime_api::Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let target_frame = original_names
            .capture_frame(self, level)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let bytes = original_names
                .read(self, target)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            target_frame
                .check(self, &original_names)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            if self.native_c_variable_name_protocol().is_some() {
                let original = if self
                    .native_c_variable_name_protocol()
                    .expect("actual C alias")
                    .alias_local_input(&bytes)
                    .qualification()
                    == tcl_syntax::naming::NativeNameQualification::Unqualified
                {
                    target.native_lifetime_lease().into_value()
                } else {
                    Value::new_native_string_bytes(local)
                };
                return self.link_original_c_variable_objects(target, level, None, &original);
            }
            (|| {
                self.add_link_bytes(local, level, &bytes)
                    .map_err(|error| self.original_jim_link_failure(error, &bytes, local))?;
                self.attach_original_jim_link(local, level, target)?;
                if self.uses_native_jim_lookup() {
                    self.retain_original_jim_link_local(target, &bytes, &original_names)?;
                }
                Ok(())
            })()
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }
    fn original_jim_link_failure(
        &mut self,
        error: UpvarLinkError,
        other: &[u8],
        local: &[u8],
    ) -> Completion<Value> {
        use tcl_syntax::native_jim_lookup::NativeJimAliasFailure as Failure;
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .native_jim_lookup_protocol()
        else {
            return crate::command::upvar_link_error_bytes(error, other, local);
        };
        let failure = match error {
            UpvarLinkError::LocalElement => Failure::LocalElement,
            UpvarLinkError::Exists | UpvarLinkError::Traced => Failure::Exists,
            UpvarLinkError::Inverted => Failure::Inverted,
            UpvarLinkError::SelfLink => Failure::SelfLink,
            UpvarLinkError::TargetNamespace | UpvarLinkError::LocalNamespace => {
                return self.refuse_host_command("actual Jim alias frame unavailable".into());
            }
        };
        crate::command::completion_from_cmd_error(
            self,
            tcl_cmd_core::var::native_jim_alias_error(protocol, failure, local),
        )
    }

    fn retain_original_jim_link_local(
        &mut self,
        local: &Value,
        bytes: &[u8],
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<(), Completion<Value>> {
        if !self.uses_native_jim_lookup() {
            return Ok(());
        }
        let binding = self
            .var_binding_from_bytes(bytes, self.current_level())
            .ok_or_else(|| {
                self.original_jim_link_failure(UpvarLinkError::LocalNamespace, bytes, bytes)
            })?;
        let protocol = self.name_policy_protocol().ok_or_else(|| {
            self.refuse_host_command("original Jim link local name policy unavailable".into())
        })?;
        let selected = protocol.recipe().variable_alias_local_input(bytes);
        let key =
            if selected.qualification() == tcl_syntax::naming::NativeNameQualification::Absolute {
                Value::new_native_string_bytes(binding.name.as_bytes())
            } else {
                local.clone()
            };
        let Some(table) = self.var_table_mut(binding.owner) else {
            return Err(self.original_jim_link_failure(
                UpvarLinkError::LocalNamespace,
                bytes,
                bytes,
            ));
        };
        table.retain_original_native_key(&binding.name, key);
        self.install_original_jim_variable(local, bytes, original_names)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))
    }
    pub(super) fn original_jim_alias_target(
        &self,
        original: &Value,
    ) -> Option<(crate::value::NativeObjectLifetimeLease, usize)> {
        self.original_jim_alias_target_checked(original)
            .ok()
            .flatten()
    }

    pub(super) fn original_jim_alias_target_checked(
        &self,
        original: &Value,
    ) -> Result<
        Option<(crate::value::NativeObjectLifetimeLease, usize)>,
        tcl_syntax::value::ValueError,
    > {
        let Some(raw) = self.current_jim_variable_cell(original)? else {
            return Ok(None);
        };
        let Some(cell) = self.var_arena.get(raw) else {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim alias cell",
            ));
        };
        let Local::NameLink(link) = cell.state() else {
            return Ok(None);
        };
        let target = link.original_target.as_ref().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim alias target object",
            ),
        )?;
        let level = self
            .frames
            .iter()
            .position(|frame| frame.jim_storage.matches(&target.frame))
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim alias target frame",
            ))?;
        target.name.check_native_header()?;
        Ok(Some((target.name.native_lifetime_lease(), level)))
    }
    fn original_jim_unset_binding_is_confined(
        &mut self,
        original: &Value,
        bytes: &[u8],
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<bool, Completion<Value>> {
        if Self::is_original_jim_dictionary_name(original, bytes) {
            // The cached tuple's actual root object selects storage independently
            // of the combined name's reporting bytes. Do not read its value.
            let (root, _) = self
                .original_jim_tuple_children_in_context(original, original_names)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let root_bytes = original_names
                .read(self, root.value())
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            return self.original_jim_unset_binding_is_confined(
                root.value(),
                &root_bytes,
                original_names,
            );
        }
        let binding = self
            .var_binding_from_bytes(bytes, self.current_level())
            .ok_or_else(|| {
                self.refuse_host_command("original Jim unset binding is unavailable".into())
            })?;
        if self.store_binding_confined(&binding, self.current_level()) {
            return Ok(true);
        }
        self.install_original_jim_variable(original, bytes, original_names)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let cached = self
            .current_jim_variable_cell(original)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        Ok(self.store_binding_confined(&binding, self.current_level())
            || cached != self.raw_variable_binding(&binding))
    }

    pub(crate) fn unset_original_named_variable(
        &mut self,
        original: &Value,
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            if self.native_c_variable_name_protocol().is_some() {
                return self.unset_original_c_variable(original, complain);
            }
            let bytes = original_names
                .read(self, original)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            if self.uses_native_jim_lookup() {
                if self.limits.confined_stores
                    && self.original_jim_unset_binding_is_confined(
                        original,
                        &bytes,
                        &original_names,
                    )?
                {
                    return Err(Self::confined_store_error(&bytes, true));
                }
                if Self::is_original_jim_dictionary_name(original, &bytes) {
                    return match self.unset_original_jim_dictionary_name(original) {
                        Err(_) if !complain && self.refused_completion().is_none() => Ok(()),
                        result => result,
                    };
                }
                if !self.limits.confined_stores {
                    self.install_original_jim_variable(original, &bytes, &original_names)
                        .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                }
                if let Some((target, level)) = self.original_jim_alias_target(original) {
                    let frame = self.select_execution_frame(level).map_err(super::err)?;
                    let result = self.unset_original_named_variable(target.value(), complain);
                    self.restore_execution_frame(frame);
                    return match result {
                        Err(_) if self.refused_completion().is_none() => Err(
                            self.variable_access_error_bytes("unset", &bytes, "no such variable")
                        ),
                        result => result,
                    };
                }
            }
            self.unset_one_bytes(&bytes, complain)
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }
    pub(super) fn read_original_jim_alias(
        &mut self,
        original: &Value,
        target: &Value,
        level: usize,
        report: super::native_jim_lookup::OriginalJimVariableReport,
        result: super::native_jim_lookup::OriginalJimVariableResult,
    ) -> Result<Value, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let frame = self
                .select_execution_frame_checked(level)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let result = self.read_original_named_variable_using(target, report, result);
            self.restore_execution_frame(frame);
            match result {
                Err(error) if self.refused_completion().is_some() => Err(error),
                Err(error)
                    if report == super::native_jim_lookup::OriginalJimVariableReport::Silent =>
                {
                    Err(error)
                }
                Err(_) => {
                    let bytes = original_names
                        .read(self, original)
                        .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                    let protocol = self
                        .actual_native_invocation_dialect()
                        .native_jim_lookup_protocol()
                        .expect("actual Jim link read");
                    let message = protocol
                        .linked_variable_read_error(&bytes)
                        .map_err(|error| {
                            self.refuse_host_command(format!("Jim alias diagnostic: {error:?}"))
                        })?;
                    Err(super::err(message))
                }
                Ok(value) => Ok(value),
            }
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }
    pub(super) fn store_original_jim_alias(
        &mut self,
        target: &Value,
        level: usize,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        let caller = super::native_name_consumers::OriginalNameConsumerScope::enter(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        caller
            .current_for_value(self, target)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let frame = self
            .select_execution_frame_checked(level)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let result = self.store_original_named_variable(target, value);
        self.restore_execution_frame(frame);
        caller
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_core_types::ROOT_NS;
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    #[test]
    fn confined_original_jim_unset_joins_retained_cache_to_the_current_binding() {
        struct RetainedName(Value);
        impl crate::NativeCommand for RetainedName {
            fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
                super::super::ok(self.0.clone())
            }
        }
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        vm.push_call_frame(Some("outer".into()), Vec::new());
        let outer = vm.current_level();
        assert_eq!(
            vm.try_eval_source("set held OUTER").unwrap().code,
            tcl_runtime_api::Code::Ok
        );
        vm.push_call_frame(Some("inner".into()), Vec::new());
        vm.add_link("alias", outer, "held")
            .unwrap_or_else(|_| panic!("link"));
        let original = Value::new_native_string_bytes(b"alias".as_slice());
        vm.register_native_command(
            "cached_jim_name",
            std::rc::Rc::new(RetainedName(original.clone())),
        );
        // `info exists` owns a byte-name existence probe, not an original
        // Jim variable cache. The actual original getter installs that cache.
        let fetched = vm.try_eval_source("set [cached_jim_name]").unwrap();
        assert_eq!(fetched.code, tcl_runtime_api::Code::Ok);
        assert_eq!(fetched.result.to_str().as_ref(), "OUTER");
        let binding = vm
            .var_binding_from_bytes(b"alias", vm.current_level())
            .unwrap();
        let old = vm.raw_variable_binding(&binding).unwrap();
        assert_eq!(vm.current_jim_variable_cell(&original).unwrap(), Some(old));
        // Retain the old arena cell/cache while removing its table binding.
        // A missing current binding must not authorise the cached foreign link.
        assert_eq!(
            vm.var_table_mut(binding.owner)
                .unwrap()
                .remove(binding.name.as_bytes()),
            Some(old)
        );
        assert_eq!(vm.current_jim_variable_cell(&original).unwrap(), Some(old));
        vm.set_stores_confined(true);
        let result = vm
            .try_eval_source("unset -nocomplain [cached_jim_name]")
            .unwrap();
        assert_eq!(result.code, tcl_runtime_api::Code::Error);
        assert!(
            result
                .result
                .to_str()
                .contains("stores are confined to the activation")
        );
        assert_eq!(
            vm.get_var_from(outer, "held").unwrap().to_str().as_ref(),
            "OUTER"
        );
        assert!(vm.refused_completion().is_none());
        // Rebinding to a genuine local refreshes the original cache and remains
        // a valid confined unset, rather than following the old retained link.
        vm.set_var("alias", Value::string("LOCAL")).unwrap();
        let result = vm
            .try_eval_source("unset -nocomplain [cached_jim_name]")
            .unwrap();
        assert_eq!(result.code, tcl_runtime_api::Code::Ok);
        assert!(vm.get_var("alias").is_none());
        assert_eq!(
            vm.get_var_from(outer, "held").unwrap().to_str().as_ref(),
            "OUTER"
        );
        assert!(vm.refused_completion().is_none());
    }

    fn decode(hex: &str) -> Vec<u8> {
        assert_eq!(hex.len() % 2, 0);
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn original_alias_read_errors_match_thirty_six_native_results() {
        let cases = include_str!("../../../tcl-syntax/tests/data/native_jim_alias_read/cases.tsv");
        let engines = [
            (
                "tcl8.4",
                include_str!("../../../tcl-syntax/tests/data/native_jim_alias_read/8.4.tsv"),
            ),
            (
                "tcl8.5",
                include_str!("../../../tcl-syntax/tests/data/native_jim_alias_read/8.5.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../../../tcl-syntax/tests/data/native_jim_alias_read/8.6.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../../tcl-syntax/tests/data/native_jim_alias_read/9.0.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../../tcl-syntax/tests/data/native_jim_alias_read/9.1.tsv"),
            ),
            (
                "jim",
                include_str!("../../../tcl-syntax/tests/data/native_jim_alias_read/jim.tsv"),
            ),
        ];
        let mut count = 0;
        for (engine, expected) in engines {
            assert_eq!(cases.lines().count(), expected.lines().count());
            for (case, result) in cases.lines().zip(expected.lines()) {
                let (name, source) = case.split_once('\t').unwrap();
                let fields: Vec<_> = result.split('\t').collect();
                assert_eq!(fields[0], name);
                let profile =
                    tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
                let mut vm = crate::native_fixture::interpreter(profile);
                let completion = vm.try_eval_source_bytes(&decode(source)).unwrap();
                assert_eq!(
                    completion.code.as_int(),
                    fields[1].parse::<i64>().unwrap(),
                    "{engine}/{name}"
                );
                let bytes =
                    tcl_syntax::value::ValueOps::native_string_bytes(&mut vm, &completion.result)
                        .unwrap();
                assert_eq!(bytes.as_ref(), decode(fields[2]), "{engine}/{name}");
                count += 1;
            }
        }
        assert_eq!(count, 36);
    }

    #[test]
    fn aliases_keep_original_target_getters_across_target_frame_invalidation() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = Vm::new();
        assert!(vm.set_native_engine_profile(profile));
        vm.set_dialect_profile(profile);
        let key = Value::new_native_string_bytes(b"v\0tail".as_slice());
        vm.store_original_named_variable(&key, Value::int(1))
            .unwrap();
        vm.push_call_frame(Some("caller".into()), Vec::new());
        let target = Value::new_native_string_bytes(b"v\0tail".as_slice());
        let local = Value::new_native_string_bytes(b"a\0tail".as_slice());
        vm.link_upvar_original(0, &target, &local).unwrap();
        assert_eq!(target.native_object_reference_count(), 2);
        assert_eq!(local.native_object_reference_count(), 2);
        assert!(matches!(
            local.native_object_snapshot().cache,
            Cache::JimVariable { global: false, .. }
        ));
        let alias_frame = local.with_jim_variable_cache(|cache| cache.frame).unwrap();
        assert_eq!(
            vm.read_original_named_variable(&local)
                .unwrap()
                .as_int()
                .unwrap(),
            1
        );
        let old_target_frame = target.with_jim_variable_cache(|cache| cache.frame).unwrap();
        vm.unset_original_named_variable(&local, true).unwrap();
        assert_eq!(
            local.with_jim_variable_cache(|cache| cache.frame),
            Some(alias_frame)
        );
        vm.store_original_named_variable(&local, Value::int(2))
            .unwrap();
        assert_ne!(
            target.with_jim_variable_cache(|cache| cache.frame),
            Some(old_target_frame)
        );
        assert_eq!(target.native_object_reference_count(), 3);
        assert_eq!(
            vm.read_original_named_variable(&local)
                .unwrap()
                .as_int()
                .unwrap(),
            2
        );
        vm.pop_call_frame();
        assert_eq!(target.native_object_reference_count(), 2);
        assert_eq!(vm.current_ns_id(), ROOT_NS);
    }
    #[test]
    fn absolute_link_targets_own_a_distinct_stripped_original() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = Vm::new();
        assert!(vm.set_native_engine_profile(profile));
        vm.set_dialect_profile(profile);
        vm.push_call_frame(Some("caller".into()), Vec::new());
        let target = Value::new_native_string_bytes(b"::::v\0tail".as_slice());
        let local = Value::new_native_string_bytes(b"alias".as_slice());
        vm.link_upvar_original(0, &target, &local).unwrap();
        assert_eq!(target.native_object_reference_count(), 1);
        assert_eq!(target.native_object_snapshot().cache, Cache::None);
        let (stored, level) = vm.original_jim_alias_target(&local).unwrap();
        assert_eq!(level, 0);
        assert!(!stored.value().is_same_object(&target));
        assert_eq!(
            stored.value().resident_string_bytes().unwrap().as_ref(),
            b"v\0tail"
        );
    }
}

#[cfg(test)]
mod alias_publication_tests {
    #[test]
    fn original_upvar_error_publication_matches_all_six_native_sources() {
        // naming.variable.original-upvar-error-publication
        // docs/design/analysis/name-resolution-proofs/variable-original-upvar-error-publication.md
        // Public script results only; C9 legacy trace refusal is a separate row.
        let source = include_str!(
            "../../../tcl-registry/tests/data/native_upvar_error_publication307/source.tcl"
        );
        for (engine, stdout) in [
            (
                "tcl8.4",
                include_str!(
                    "../../../tcl-registry/tests/data/native_upvar_error_publication307/8.4.20/original-upvar-errors/stdout"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../tcl-registry/tests/data/native_upvar_error_publication307/8.5.19/original-upvar-errors/stdout"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../tcl-registry/tests/data/native_upvar_error_publication307/8.6.18/original-upvar-errors/stdout"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../tcl-registry/tests/data/native_upvar_error_publication307/9.0.4/original-upvar-errors/stdout"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../tcl-registry/tests/data/native_upvar_error_publication307/9.1.0/original-upvar-errors/stdout"
                ),
            ),
            (
                "jim",
                include_str!(
                    "../../../tcl-registry/tests/data/native_upvar_error_publication307/jim/original-upvar-errors/stdout"
                ),
            ),
        ] {
            let original = stdout
                .lines()
                .find_map(|line| line.strip_prefix("ORIGINAL|0|"))
                .unwrap();
            let expected: Vec<u8> = original
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            // The original public source needs the independently attached C
            // byte compiler; genuine Jim enters its original Script owner.
            // Neither capability supplies a scripted distribution library.
            let mut vm = crate::native_fixture::interpreter(profile);
            let completion = vm.eval_source(source).unwrap();
            assert_eq!(
                completion.code,
                crate::Code::Ok,
                "{engine}: {:?}",
                completion.result.string_bytes()
            );
            assert!(vm.refused_completion().is_none(), "{engine}");
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                expected.as_slice(),
                "{engine}"
            );
        }
    }
}
