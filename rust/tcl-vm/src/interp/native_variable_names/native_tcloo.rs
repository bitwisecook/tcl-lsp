// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original `TclOO` explicit variable lookup, separate from automatic local binding.

use super::{Completion, NativeVariableNameLookupPurpose, Value, VarTableOwner, Vm};

impl Vm {
    pub(crate) fn link_original_c_oo_variable(
        &mut self,
        original: &Value,
        namespace: tcl_core_types::NsId,
        local: &[u8],
    ) -> Result<(), Completion<Value>> {
        let (_root, bytes, element, resolved) = self.prepare_native_original_variable_in(
            original,
            NativeVariableNameLookupPurpose::Define,
            Some(namespace),
        )?;
        if element.is_some() {
            let reported = self
                .native_name_operand_bytes(original)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            return Err(self.variable_access_error_input(
                "define",
                tcl_syntax::naming::NativeVariableInputForm::Combined(&reported),
                "name refers to an element in an array",
            ));
        }
        let target = resolved.id.ok_or_else(|| {
            self.refuse_host_command("original TclOO namespace variable cell unavailable".into())
        })?;
        self.var_arena.mark_namespace_declared(target);
        // PtrMakeUpvar creates a fresh CString local object, not the counted target.
        let local = Value::new_native_string_bytes(local);
        self.bind_original_c_alias_local(&local, target, false, &bytes)
    }

    pub(crate) fn original_c_oo_varname(
        &mut self,
        original: &Value,
        namespace: tcl_core_types::NsId,
        storage: &[u8],
    ) -> Result<Value, Completion<Value>> {
        let bytes = self
            .native_name_operand_bytes(original)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let protocol = self
            .name_policy_protocol()
            .expect("selected native TclOO names")
            .recipe();
        let lookup = tcl_syntax::naming::native_oo_varname_lookup_bytes(
            protocol,
            &tcl_runtime_api::Namespaces::name_bytes(self, namespace),
            &bytes,
            storage,
        )
        .map_err(|error| self.refuse_host_command(format!("{error:?}")))?;
        let lookup = if bytes.starts_with(b"::") {
            original.clone()
        } else {
            Value::new_native_string_bytes(lookup)
        };
        let (_root, _bytes, _element, resolved) = self
            .prepare_native_original_variable_in(
                &lookup,
                NativeVariableNameLookupPurpose::Refer,
                Some(namespace),
            )
            .map_err(|mut failure| {
                if self.refused_completion().is_none() {
                    let code = Value::list(vec![
                        Value::string("TCL"),
                        Value::string("LOOKUP"),
                        Value::string("VARIABLE"),
                        Value::new_native_string_bytes(tcl_core_types::c_string_extent(&bytes)),
                    ]);
                    failure.options = match crate::command::with_return_option(
                        self,
                        &failure.options,
                        "-errorcode",
                        code,
                    ) {
                        Ok(options) => options,
                        Err(error) => {
                            return crate::command::completion_from_tcl_error(self, error);
                        }
                    };
                }
                failure
            })?;
        let target = self.settle_original_c_link_target(&lookup, &resolved)?;
        let parent = self
            .var_arena
            .element_parent(target)
            .map(|(array, key)| (array, key.clone()));
        let base = parent.as_ref().map_or(target, |(array, _)| *array);
        if parent.is_none() {
            self.var_arena.mark_namespace_declared(target);
        }
        let owner = self
            .var_arena
            .get(base)
            .and_then(crate::vars::VarCell::namespace_owner)
            .ok_or_else(|| {
                self.refuse_host_command("actual TclOO variable namespace owner unavailable".into())
            })?;
        let key = self
            .var_table(VarTableOwner::Namespace(owner))
            .and_then(|table| {
                table
                    .iter()
                    .find(|(_, cell)| **cell == base)
                    .map(|(key, _)| key.clone())
            })
            .ok_or_else(|| {
                self.refuse_host_command("actual TclOO variable table key unavailable".into())
            })?;
        let mut result = tcl_runtime_api::Namespaces::name_bytes(self, owner);
        if owner != tcl_core_types::ROOT_NS {
            result.extend_from_slice(b"::");
        }
        result.extend_from_slice(
            tcl_syntax::naming::native_oo_variable_key_report(protocol, key.as_bytes())
                .map_err(|error| self.refuse_host_command(format!("{error:?}")))?,
        );
        if let Some((_, key)) = parent {
            result.push(b'(');
            result.extend_from_slice(tcl_core_types::c_string_extent(key.as_bytes()));
            result.push(b')');
        }
        Ok(Value::new_native_string_bytes(result))
    }
}
