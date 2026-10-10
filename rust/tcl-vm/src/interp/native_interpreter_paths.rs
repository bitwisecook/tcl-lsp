// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C child paths, independent of command and namespace addresses.

use super::{InterpId, Vm, ok};
use crate::{Completion, Value, command::completion_from_cmd_error};
use tcl_core_types::NameBytes;
use tcl_syntax::value::ValueError;

impl Vm {
    pub(crate) fn create_child_original(&mut self, path: &Value, safe: bool) -> Completion<Value> {
        let elements = match self.native_interpreter_path_elements(path) {
            Ok(elements) => elements,
            Err(error) => return completion_from_cmd_error(self, error.into()),
        };
        // ChildCreate retains the whole original spelling when its list has
        // fewer than two elements; this differs from GetInterp's element loop.
        let (parent, name) = if elements.len() < 2 {
            let bytes = match self.native_name_operand_bytes(path) {
                Ok(bytes) => bytes,
                Err(error) => return self.refuse_host_command(error.to_string()),
            };
            let Some(policy) = self.name_policy_protocol() else {
                return self.refuse_host_command("interpreter creation protocol".into());
            };
            let name = match policy.recipe().interpreter_child_input(&bytes) {
                Ok(name) => NameBytes::from(name.selected()),
                Err(_) => return self.refuse_host_command("interpreter creation extent".into()),
            };
            (self.cur, name)
        } else {
            let mut parent = self.cur;
            for element in &elements[..elements.len() - 1] {
                let next = self
                    .st_of(parent)
                    .and_then(|state| state.children.get(element.as_bytes()).copied());
                let Some(next) = next.filter(|next| self.interp_alive(*next)) else {
                    return self.interpreter_path_missing(path);
                };
                parent = next;
            }
            (parent, elements.last().expect("multi-element path").clone())
        };
        if self
            .st_of(parent)
            .is_some_and(|state| state.children.contains_key(name.as_bytes()))
        {
            return completion_from_cmd_error(
                self,
                tcl_cmd_core::CmdError::new_bytes(
                    [
                        b"interpreter named \"".as_slice(),
                        name.as_bytes(),
                        b"\" already exists, cannot create",
                    ]
                    .concat(),
                ),
            );
        }
        match self.in_interp(parent, |vm| vm.create_child_named_bytes(name, safe)) {
            Ok(_) => ok(path.clone()),
            Err(error) => self.refuse_host_command(error.to_string()),
        }
    }

    pub(crate) fn native_interpreter_path_elements(
        &self,
        original: &Value,
    ) -> Result<Vec<NameBytes>, ValueError> {
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "interpreter path protocol",
            ))?;
        if policy.recipe().is_jim084() {
            return Err(ValueError::CommandProtocolUnavailable(
                "C interpreter path on Jim",
            ));
        }
        let elements = self.native_object_list_elements_in(original, policy.string_protocol())?;
        elements
            .iter()
            .map(|element| {
                let bytes = self.native_name_operand_bytes(element).map_err(|_| {
                    ValueError::CommandProtocolUnavailable("interpreter path element getter")
                })?;
                let selected = policy
                    .recipe()
                    .interpreter_child_input(&bytes)
                    .map_err(|_| ValueError::CommandProtocolUnavailable("interpreter child key"))?;
                Ok(NameBytes::from(selected.selected()))
            })
            .collect()
    }

    pub(crate) fn resolve_interp_path_original(
        &mut self,
        original: &Value,
    ) -> Result<InterpId, Completion<Value>> {
        let elements = self
            .native_interpreter_path_elements(original)
            .map_err(|error| completion_from_cmd_error(self, error.into()))?;
        let mut selected = self.cur;
        for element in elements {
            let next = self
                .st_of(selected)
                .and_then(|state| state.children.get(element.as_bytes()).copied());
            let Some(next) = next.filter(|next| self.interp_alive(*next)) else {
                return Err(self.interpreter_path_missing(original));
            };
            selected = next;
        }
        Ok(selected)
    }

    pub(crate) fn interpreter_path_missing(&mut self, original: &Value) -> Completion<Value> {
        let bytes = match self.native_name_operand_bytes(original) {
            Ok(bytes) => bytes,
            Err(error) => return self.refuse_host_command(error.to_string()),
        };
        let Some(policy) = self.name_policy_protocol() else {
            return self.refuse_host_command("interpreter path diagnostic protocol".into());
        };
        let reported = match policy.recipe().interpreter_child_input(&bytes) {
            Ok(input) => input.selected().to_vec(),
            Err(_) => return self.refuse_host_command("interpreter path diagnostic extent".into()),
        };
        let code = tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
            policy.string_protocol(),
        )
        .render(&[b"TCL".as_slice(), b"LOOKUP", b"INTERP", reported.as_slice()]);
        completion_from_cmd_error(
            self,
            tcl_cmd_core::CmdError::with_error_code_bytes(
                [
                    b"could not find interpreter \"".as_slice(),
                    &reported,
                    b"\"",
                ]
                .concat(),
                code,
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{Value, Vm};
    use tcl_runtime_api::{Code, Completion};

    fn call(vm: &mut Vm, subcommand: &[u8], arguments: &[Value]) -> Completion<Value> {
        let mut words = vec![Value::new_native_string_bytes(subcommand)];
        words.extend_from_slice(arguments);
        vm.invoke_host_original_object_vector(
            &Value::new_native_string_bytes(b"interp".as_slice()),
            &words,
        )
    }

    #[test]
    fn original_interpreter_paths_keep_counted_objects_and_child_cstring_keys() {
        // Native proof: naming.interpreter.counted-list-path-cstring-child-keys
        // docs/design/analysis/name-resolution-proofs/interpreter-counted-list-path-cstring-child-keys.md
        // Native proof: naming.interpreter.singleton-create-original-spelling
        // docs/design/analysis/name-resolution-proofs/interpreter-singleton-create-original-spelling.md
        // Native proof: naming.interpreter.malformed-path-list-error
        // docs/design/analysis/name-resolution-proofs/interpreter-malformed-path-list-error.md
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm =
                crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
            for bytes in [
                b"child\0suffix".as_slice(),
                b"opaque\xff",
                b"surrogate\xed\xa0\x80",
                b"encoded\xc0\x80tail",
            ] {
                let path = Value::new_native_string_bytes(bytes);
                let created = call(&mut vm, b"create", std::slice::from_ref(&path));
                assert_eq!(created.code, Code::Ok, "{engine}: {created:?}");
                assert!(created.result.is_same_object(&path));
                let selected = vm.resolve_interp_path_original(&path).unwrap();
                assert_ne!(selected, vm.cur);
                let deleted = call(&mut vm, b"delete", std::slice::from_ref(&path));
                assert_eq!(deleted.code, Code::Ok);
                assert!(vm.resolve_interp_path_original(&path).is_err());
            }
            let original = Value::new_native_string_bytes(b"{with space}".as_slice());
            assert_eq!(call(&mut vm, b"create", &[original.clone()]).code, Code::Ok);
            assert!(vm.resolve_interp_path_original(&original).is_err());
            let quoted = Value::new_native_string_bytes(b"{{with space}}".as_slice());
            assert!(vm.resolve_interp_path_original(&quoted).is_ok());
            let malformed = Value::new_native_string_bytes(b"{".as_slice());
            assert_eq!(call(&mut vm, b"create", &[malformed]).code, Code::Error);
            assert!(vm.execution_refusal.is_none());
        }
    }
}
