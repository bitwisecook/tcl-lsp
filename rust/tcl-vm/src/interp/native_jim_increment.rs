// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Jim update lookup, with root publication before member conversion.

use super::{Completion, Local, PreparedIncrementAmount, Value, Vm};
use crate::NativeObjectLifetimeLease;
use tcl_syntax::native_string::NativeStringProtocol;

impl Vm {
    pub(super) fn is_original_jim_dictionary_name(original: &Value, bytes: &[u8]) -> bool {
        original
            .with_native_jim_dictionary_substitution(|_, _| ())
            .is_some()
            || (bytes.last() == Some(&b')')
                && bytes
                    .split(|byte| *byte == 0)
                    .next()
                    .unwrap_or_default()
                    .contains(&b'('))
    }

    fn original_jim_dictionary_children(
        &mut self,
        original: &Value,
    ) -> Result<(NativeObjectLifetimeLease, NativeObjectLifetimeLease), Completion<Value>> {
        let context = self
            .native_jim_object_context()
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        original
            .ensure_native_jim_dictionary_substitution(&context)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        Ok(original
            .with_native_jim_dictionary_substitution(|name, key| {
                (name.native_lifetime_lease(), key.native_lifetime_lease())
            })
            .expect("installed original tuple"))
    }

    fn read_original_jim_update(
        &mut self,
        original: &Value,
    ) -> Result<Option<NativeObjectLifetimeLease>, Completion<Value>> {
        if let Some(cell) = self
            .current_jim_variable_cell(original)
            .map_err(|error| self.refuse_host_command(error.to_string()))?
            && let Some(Local::Scalar(value)) =
                self.var_arena.get(cell).map(crate::vars::VarCell::state)
        {
            return Ok(Some(value.native_lifetime_lease()));
        }
        if let Some((target, level)) = self.original_jim_alias_target(original) {
            let frame = self.select_execution_frame(level).map_err(super::err)?;
            let result = self.read_original_jim_update(target.value());
            self.restore_execution_frame(frame);
            return result;
        }
        let bytes = self
            .native_name_operand_bytes(original)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        if Self::is_original_jim_dictionary_name(original, &bytes) {
            let (name, key) = self.original_jim_dictionary_children(original)?;
            let found = match self.read_native_jim_dictionary_member(name.value(), key.value()) {
                Ok(found) => found,
                Err(error) if self.refused_completion().is_some() => return Err(error),
                Err(_) => return Ok(None),
            };
            let member = found.native_lifetime_lease();
            drop(found);
            // Remove the ordinary getter's temporary native reference before
            // testing genuine root sharing. Only the variable/aliases own it.
            let root = self.read_original_named_variable(name.value())?;
            let root_lease = root.native_lifetime_lease();
            drop(root);
            if root_lease.value().native_object_is_shared() {
                let duplicate = root_lease
                    .value()
                    .duplicate_native_object_in(NativeStringProtocol::Jim084);
                drop(self.store_original_named_variable(name.value(), duplicate)?);
            }
            return Ok(Some(member));
        }
        self.install_original_jim_variable(original, &bytes)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        if let Some((target, level)) = self.original_jim_alias_target(original) {
            let frame = self.select_execution_frame(level).map_err(super::err)?;
            let result = self.read_original_jim_update(target.value());
            self.restore_execution_frame(frame);
            return result;
        }
        let raw = self
            .current_jim_variable_cell(original)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        Ok(raw.and_then(
            |cell| match self.var_arena.get(cell).map(crate::vars::VarCell::state) {
                Some(Local::Scalar(value)) => Some(value.native_lifetime_lease()),
                _ => None,
            },
        ))
    }

    pub(super) fn store_original_jim_dictionary_name(
        &mut self,
        original: &Value,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        let (name, key) = self.original_jim_dictionary_children(original)?;
        let existing = match self.read_original_named_variable(name.value()) {
            Ok(root) => {
                let lease = root.native_lifetime_lease();
                drop(root);
                Some(lease)
            }
            Err(error) if self.refused_completion().is_some() => return Err(error),
            Err(_) => None,
        };
        let fresh = if existing.is_none() {
            let root = Value::native_dictionary_constructor(
                Vec::new(),
                None,
                NativeStringProtocol::Jim084,
            )
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
            let lease = root.native_lifetime_lease();
            drop(self.store_original_named_variable(name.value(), root)?);
            Some(lease)
        } else {
            None
        };
        let root = existing
            .as_ref()
            .or(fresh.as_ref())
            .expect("native dictionary root");
        let result = value.native_lifetime_lease();
        let updated = root
            .value()
            .native_dictionary_set_member(key.value().clone(), value, NativeStringProtocol::Jim084)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        drop(self.store_original_named_variable(name.value(), updated)?);
        Ok(result.into_value())
    }

    pub(crate) fn increment_original_jim_name(
        &mut self,
        original: &Value,
        amount: &Value,
    ) -> Completion<Value> {
        let prepared = match self.prepare_increment_amount(amount) {
            Ok(prepared) => prepared,
            Err(error) => return error,
        };
        let PreparedIncrementAmount::Native(prepared) = prepared else {
            return self.refuse_host_command("original Jim increment amount issuer".into());
        };
        let current = match self.read_original_jim_update(original) {
            Ok(current) => current,
            Err(error) => return error,
        };
        let objects = match crate::value_ops::VmLegacyIncrementObjects::selected(self) {
            Ok(objects) => objects,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let sum = match tcl_cmd_core::native_increment::increment_legacy(
            &objects,
            current.as_ref().map(NativeObjectLifetimeLease::value),
            prepared,
        ) {
            Ok(sum) => sum,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        match self.store_original_named_variable(original, sum) {
            Ok(value) => super::ok(value),
            Err(error) => error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;
    fn current_root(vm: &Vm) -> &Value {
        let selected = vm.resolve_var_from_bytes(b"d", vm.current_level()).unwrap();
        match vm.var_arena.get(selected.id.unwrap()).unwrap().state() {
            Local::Scalar(value) => value,
            _ => panic!("original scalar dictionary"),
        }
    }
    fn kind(value: &Value) -> &'static str {
        match value.native_object_type_name() {
            "none" => "NULL",
            name => name,
        }
    }
    #[test]
    fn original_dictionary_sugar_increment_matches_native_publication_windows() {
        let mut observed = String::new();
        for case in 0..7 {
            let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
            let mut vm = Vm::new();
            assert!(vm.set_native_engine_profile(profile));
            vm.set_dialect_profile(profile);
            let member = Value::new_native_string_bytes(if case == 2 || case == 3 {
                b"BAD".as_slice()
            } else {
                b"4".as_slice()
            });
            let member_lease = member.native_lifetime_lease();
            let original_member = (case == 4).then(|| member.clone());
            let root = Value::native_dictionary_constructor(
                vec![(Value::new_native_string_bytes(b"k".as_slice()), member)],
                None,
                NativeStringProtocol::Jim084,
            )
            .unwrap();
            let root_lease = root.native_lifetime_lease();
            let shared = matches!(case, 1 | 3 | 5 | 6);
            let original_root = shared.then(|| root.clone());
            vm.set_var_bytes(b"d", root).unwrap();
            let arguments = [
                Value::new_native_string_bytes(if case == 5 {
                    b"d(missing)".as_slice()
                } else {
                    b"d(k)".as_slice()
                }),
                if case == 6 {
                    Value::new_native_string_bytes(b"BAD".as_slice())
                } else {
                    Value::int(1)
                },
            ];
            writeln!(
                observed,
                "BEFORE\t{case}\t{}\t{}\t{}\t{}\t{}\t{}",
                root_lease.value().native_object_reference_count(),
                kind(root_lease.value()),
                usize::from(root_lease.value().resident_string_bytes().is_some()),
                member_lease.value().native_object_reference_count(),
                kind(member_lease.value()),
                usize::from(member_lease.value().resident_string_bytes().is_some())
            )
            .unwrap();
            let completion = vm.try_invoke_command("incr", &arguments).unwrap();
            let code = completion.code;
            drop(completion);
            let root = current_root(&vm);
            let key = if case == 5 {
                b"missing".as_slice()
            } else {
                b"k".as_slice()
            };
            root.with_cached_dictionary_member(&NameBytes::from(key), |current| {
                let current = current.unwrap();
                writeln!(
                    observed,
                    "AFTER\t{case}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    code.as_int(),
                    usize::from(root.is_same_object(root_lease.value())),
                    kind(root),
                    root.native_object_reference_count(),
                    usize::from(current.is_same_object(member_lease.value())),
                    kind(current),
                    current.native_object_reference_count(),
                    usize::from(current.resident_string_bytes().is_some())
                )
                .unwrap();
            })
            .unwrap();
            if shared {
                writeln!(
                    observed,
                    "OLD\t{case}\t{}\t{}\t{}",
                    member_lease.value().native_object_reference_count(),
                    kind(member_lease.value()),
                    usize::from(member_lease.value().resident_string_bytes().is_some())
                )
                .unwrap();
            }
            drop(original_root);
            drop(original_member);
        }
        let expected = include_str!(
            "../../../tcl-syntax/testdata/native_jim_sugar_increment/observations.tsv"
        );
        assert_eq!(expected.lines().count(), 18);
        assert_eq!(observed, expected);
    }
}
