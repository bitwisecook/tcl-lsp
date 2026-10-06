// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim links retain their original recursive target getter and actual frame.

use super::{Local, UpvarLinkError, Value, Vm};
use std::rc::Rc;
use tcl_runtime_api::Completion;
use tcl_syntax::native_jim_lookup::NativeJimLinkTargetInput;

impl Vm {
    fn attach_original_jim_link(
        &mut self,
        local: &[u8],
        target_level: usize,
        original: &Value,
    ) -> Result<(), UpvarLinkError> {
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .native_jim_lookup_protocol()
        else {
            return Ok(());
        };
        let bytes = self
            .native_name_operand_bytes(original)
            .map_err(|_| UpvarLinkError::TargetNamespace)?;
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
            frame: Rc::downgrade(&frame.activation),
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
    }
    pub(crate) fn link_upvar_original(
        &mut self,
        target_level: usize,
        other: &Value,
        local: &Value,
    ) -> Result<(), tcl_runtime_api::Completion<Value>> {
        if self.native_c_variable_name_protocol().is_some() {
            return self.link_original_c_variable_objects(other, target_level, None, local);
        }
        let bytes = self
            .native_name_operand_bytes(other)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let local_bytes = self
            .native_name_operand_bytes(local)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let result = (|| {
            self.link_upvar_bytes(target_level, &bytes, &local_bytes)?;
            self.attach_original_jim_link(&local_bytes, target_level, other)?;
            self.retain_original_jim_link_local(local, &local_bytes)
        })();
        result.map_err(|error| crate::command::upvar_link_error_bytes(error, &bytes, &local_bytes))
    }
    pub(crate) fn add_link_original(
        &mut self,
        local: &[u8],
        level: usize,
        target: &Value,
    ) -> Result<(), tcl_runtime_api::Completion<Value>> {
        let bytes = self
            .native_name_operand_bytes(target)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
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
        let result = (|| {
            self.add_link_bytes(local, level, &bytes)?;
            self.attach_original_jim_link(local, level, target)?;
            if self.uses_native_jim_lookup() {
                self.retain_original_jim_link_local(target, &bytes)?;
            }
            Ok(())
        })();
        result.map_err(|error| crate::command::upvar_link_error_bytes(error, &bytes, local))
    }
    fn retain_original_jim_link_local(
        &mut self,
        local: &Value,
        bytes: &[u8],
    ) -> Result<(), UpvarLinkError> {
        if !self.uses_native_jim_lookup() {
            return Ok(());
        }
        let binding = self
            .var_binding_from_bytes(bytes, self.current_level())
            .ok_or(UpvarLinkError::LocalNamespace)?;
        let key = if bytes.starts_with(b"::") {
            Value::new_native_string_bytes(binding.name.as_bytes())
        } else {
            local.clone()
        };
        self.var_table_mut(binding.owner)
            .ok_or(UpvarLinkError::LocalNamespace)?
            .retain_original_native_key(&binding.name, key);
        self.install_original_jim_variable(local, bytes)
            .map_err(|_| UpvarLinkError::LocalNamespace)
    }
    pub(super) fn original_jim_alias_target(
        &self,
        original: &Value,
    ) -> Option<(crate::value::NativeObjectLifetimeLease, usize)> {
        let raw = self.current_jim_variable_cell(original).ok()??;
        let Local::NameLink(link) = self.var_arena.get(raw)?.state() else {
            return None;
        };
        let target = link.original_target.as_ref()?;
        let actual = target.frame.upgrade()?;
        let level = self
            .frames
            .iter()
            .position(|frame| Rc::ptr_eq(&frame.activation, &actual))?;
        Some((target.name.native_lifetime_lease(), level))
    }
    pub(crate) fn unset_original_named_variable(
        &mut self,
        original: &Value,
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        if self.native_c_variable_name_protocol().is_some() {
            return self.unset_original_c_variable(original, complain);
        }
        let bytes = self
            .native_name_operand_bytes(original)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        if self.uses_native_jim_lookup() {
            self.install_original_jim_variable(original, &bytes)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            if let Some((target, level)) = self.original_jim_alias_target(original) {
                let frame = self.select_execution_frame(level).map_err(super::err)?;
                let result = self.unset_original_named_variable(target.value(), complain);
                self.restore_execution_frame(frame);
                return result;
            }
        }
        self.unset_one_bytes(&bytes, complain)
    }
    pub(super) fn read_original_jim_alias(
        &mut self,
        original: &Value,
        target: &Value,
        level: usize,
    ) -> Result<Value, Completion<Value>> {
        let frame = self.select_execution_frame(level).map_err(super::err)?;
        let result = self.read_original_named_variable(target);
        self.restore_execution_frame(frame);
        match result {
            Err(error) if self.refused_completion().is_some() => Err(error),
            Err(_) => {
                let bytes = self
                    .native_name_operand_bytes(original)
                    .map_err(|error| self.refuse_host_command(error.to_string()))?;
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
    }
    pub(super) fn store_original_jim_alias(
        &mut self,
        target: &Value,
        level: usize,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        let frame = self.select_execution_frame(level).map_err(super::err)?;
        let result = self.store_original_named_variable(target, value);
        self.restore_execution_frame(frame);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_core_types::ROOT_NS;
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    fn decode(hex: &str) -> Vec<u8> {
        assert_eq!(hex.len() % 2, 0);
        hex.as_bytes()
            .chunks_exact(2)
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
