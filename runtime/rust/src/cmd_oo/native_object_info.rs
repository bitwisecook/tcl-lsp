// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected TclOO bytecode getters consume the same original command operand.

use super::{obj, Code, Command, Interp, OoId, TclObj};
use std::collections::HashSet;
use tcl_registry::native_tcloo_compilation::NativeTclOoObjectInfo;
use tcl_syntax::value::{ValueError, ValueOps};

pub(super) fn selected_object_info_handler(
    interp: &mut Interp,
    member: &[u8],
    original: &[*mut TclObj],
) -> Option<Code> {
    let protocol = interp
        .native_invocation_dialect()
        .native_namespace_name_protocol()?;
    if original.len() != 4 || protocol.recipe().version() < tcl_dialect::TclVersion::V8_6 {
        return None;
    }
    let operation = match member {
        b"class" => NativeTclOoObjectInfo::Class,
        b"namespace" => NativeTclOoObjectInfo::Namespace,
        b"creationid" if protocol.recipe().version() >= tcl_dialect::TclVersion::V9_1 => {
            NativeTclOoObjectInfo::CreationId
        }
        _ => return None,
    };
    Some(interp.execute_native_oo_object_info(operation, original[3]))
}

impl Interp {
    fn native_object_from_original(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<OoId>, ValueError> {
        let Some((mut command, _)) =
            self.resolve_original_command_at(self.oo_outer_namespace(), original)?
        else {
            return Ok(None);
        };
        let mut imports = HashSet::new();
        loop {
            match command {
                Command::OoObject(object) => {
                    return Ok(self
                        .oo
                        .borrow()
                        .objects
                        .contains_key(&object)
                        .then_some(object))
                }
                Command::Imported {
                    source_generation, ..
                } => {
                    if !imports.insert(source_generation) {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "cyclic original imported object command",
                        ));
                    }
                    let next = self.namespaces().native_command_at_node(source_generation);
                    let Some((next, _)) = next else {
                        return Ok(None);
                    };
                    command = next;
                }
                _ => return Ok(None),
            }
        }
    }

    fn native_object_lookup_error(&mut self, original: *mut TclObj) -> Code {
        let bytes = match self.native_string_bytes(&original) {
            Ok(bytes) => bytes,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let name = tcl_core_types::c_string_extent(&bytes);
        let mut message = name.to_vec();
        message.extend_from_slice(b" does not refer to an object");
        let code = crate::interp::error_code_list(&[b"TCL", b"LOOKUP", b"OBJECT", name]);
        self.error_with_code(&message, &code)
    }

    pub(crate) fn execute_native_oo_object_info(
        &mut self,
        operation: NativeTclOoObjectInfo,
        original: *mut TclObj,
    ) -> Code {
        let object = match self.native_object_from_original(original) {
            Ok(object) => object,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        if object.is_none() {
            let code = self.native_object_lookup_error(original);
            if operation != NativeTclOoObjectInfo::IsObject || self.host_refusal_pending() {
                return code;
            }
        }
        let result = match operation {
            NativeTclOoObjectInfo::Class => {
                let class =
                    self.oo.borrow().objects[&object.expect("selected original object")].class;
                Ok(self.oo_original_name(class))
            }
            NativeTclOoObjectInfo::Namespace => {
                let namespace =
                    self.oo.borrow().objects[&object.expect("selected original object")].var_ns;
                match u32::try_from(namespace) {
                    Ok(namespace) => tcl_cmd_core::namespace::NamespaceObjectBackend::produce_namespace_object(
                        self, tcl_runtime_api::NsId(namespace),
                        tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer::ObjectNamespace,
                    ),
                    Err(_) => Err(ValueError::CommandProtocolUnavailable("original TclOO namespace token width")),
                }
            }
            NativeTclOoObjectInfo::IsObject => match self.native_invocation_dialect().tcl_version {
                Some(version) if operation.uses_execution_constant(version) => {
                    self.native_execution_boolean_constant(object.is_some())
                }
                _ => Err(ValueError::CommandProtocolUnavailable(
                    "native TclOO predicate result producer",
                )),
            },
            NativeTclOoObjectInfo::CreationId => {
                let epoch = self.oo.borrow().objects[&object.expect("selected original object")]
                    .creation_id;
                match i64::try_from(epoch) {
                    Ok(epoch) => Ok(obj::new_wide_int_obj(epoch)),
                    Err(_) => Err(ValueError::CommandProtocolUnavailable(
                        "original TclOO creation epoch width",
                    )),
                }
            }
        };
        match result {
            Ok(result) => {
                self.set_result(result);
                Code::Ok
            }
            Err(error) => self.report_cmd_error(error.into()),
        }
    }
}
