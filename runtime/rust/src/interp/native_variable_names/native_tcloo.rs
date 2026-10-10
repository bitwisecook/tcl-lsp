// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original TclOO explicit variable lookup, separate from automatic local binding.

use super::*;

impl Interp {
    pub(crate) fn link_original_c_oo_variable(
        &mut self,
        original: *mut TclObj,
        namespace: crate::namespace::NsId,
        local: &[u8],
    ) -> Code {
        let purpose = NativeVariableNameLookupPurpose::Define;
        let selected = match self.prepare_original_c_name_in(original, purpose, Some(namespace)) {
            Ok(Some(selected)) => selected,
            Ok(None) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original TclOO variable target").into(),
                );
            }
            Err(code) => return code,
        };
        if selected.element.is_some() {
            return self.original_c_variable_failure_for_object(
                original,
                purpose,
                tcl_syntax::naming::NativeVariableDiagnosticReason::ArrayElement,
                tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
            );
        }
        let target = crate::vars::original_namespace_link_target(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            namespace,
            &selected.root,
            None,
        );
        let mut target = match target {
            Ok(target) => target,
            Err(error) => return self.original_c_lookup_var_error(original, purpose, error),
        };
        if let Err(error) = self.prepare_upvar_target(&mut target) {
            return self.original_c_lookup_var_error(original, purpose, error);
        }
        if let crate::frame::VarHome::Namespace(owner) = target.home {
            self.namespaces
                .borrow_mut()
                .var_table_mut(owner)
                .mark_namespace_declared(&target.name);
        }
        // PtrMakeUpvar supplies a fresh local CString object, not original part1.
        let local = Owned::fresh(obj::new_string_bytes(local));
        self.bind_original_c_alias_local(local.as_ptr(), target)
    }

    pub(crate) fn original_c_oo_varname(
        &mut self,
        original: *mut TclObj,
        namespace: crate::namespace::NsId,
        storage: &[u8],
    ) -> Code {
        let bytes = match self.native_string_bytes(&original) {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let version = self
            .native_c_variable_name_protocol()
            .expect("selected native TclOO names")
            .version();
        let protocol = tcl_syntax::naming::NativeNameProtocol::C(version);
        let namespace_name = self.namespaces.borrow().qualified_name(namespace);
        let lookup = match tcl_syntax::naming::native_oo_varname_lookup_bytes(
            protocol,
            &namespace_name,
            &bytes,
            storage,
        ) {
            Ok(lookup) => lookup,
            Err(_) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("TclOO varname construction").into(),
                );
            }
        };
        let owned =
            (!bytes.starts_with(b"::")).then(|| Owned::fresh(obj::new_string_bytes(&lookup)));
        let lookup = owned.as_ref().map_or(original, Owned::as_ptr);
        let purpose = NativeVariableNameLookupPurpose::Refer;
        let selected = match self.prepare_original_c_name_in(lookup, purpose, Some(namespace)) {
            Ok(Some(selected)) => selected,
            Ok(None) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original TclOO varname target").into(),
                );
            }
            Err(code) => {
                if code == Code::Error && !self.host_refusal_pending() {
                    let error_code = super::super::error_code_list(&[
                        b"TCL",
                        b"LOOKUP",
                        b"VARIABLE",
                        tcl_core_types::c_string_extent(&bytes),
                    ]);
                    self.replace_native_error_code(&error_code);
                }
                return code;
            }
        };
        let captured = match self.capture_original_c_selection(lookup, &selected, purpose) {
            Ok(Some(captured)) => captured,
            Ok(None) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original TclOO varname cell").into(),
                );
            }
            Err(code) => return code,
        };
        let (_, home) = captured;
        let Some(owner) = home.ns else {
            return self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("actual TclOO variable namespace owner")
                    .into(),
            );
        };
        if home.link_elem.is_none() {
            self.namespaces
                .borrow_mut()
                .var_table_mut(owner)
                .mark_namespace_declared(&home.base);
        }
        let mut result = self.namespaces.borrow().qualified_name(owner);
        if owner != crate::namespace::GLOBAL {
            result.extend_from_slice(b"::");
        }
        result.extend_from_slice(
            tcl_syntax::naming::native_oo_variable_key_report(protocol, &home.base)
                .expect("selected TclOO reporting"),
        );
        if let Some(key) = home.link_elem {
            result.push(b'(');
            result.extend_from_slice(tcl_core_types::c_string_extent(&key));
            result.push(b')');
        }
        self.set_result(obj::new_string_bytes(&result));
        Code::Ok
    }
}
