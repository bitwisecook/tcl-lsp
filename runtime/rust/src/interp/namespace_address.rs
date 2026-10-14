// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected byte-name geometry for physical namespace lookup and publication.

use super::Interp;
use crate::namespace::{NsId, GLOBAL};
use tcl_core_types::NameBytes;
use tcl_syntax::{
    naming::{NameProjectionUnavailable, NativeNameContext, NativeNameProtocol},
    value::ValueError,
};

pub(crate) enum NamespaceAddressError {
    Unavailable(ValueError),
    Projection {
        protocol: NativeNameProtocol,
        reason: NameProjectionUnavailable,
    },
}

impl NamespaceAddressError {
    pub(crate) fn into_value_error(self) -> ValueError {
        match self {
            Self::Unavailable(error) => error,
            Self::Projection { .. } => {
                ValueError::CommandProtocolUnavailable("namespace address geometry")
            }
        }
    }
    fn into_creation_error(self) -> tcl_cmd_core::CmdError {
        match self {
            Self::Projection {
                protocol,
                reason: NameProjectionUnavailable::EmptyNamespaceOutsideRoot,
            } => {
                let report = match tcl_syntax::naming::report_native_namespace_creation_error(
                    protocol,
                    NameProjectionUnavailable::EmptyNamespaceOutsideRoot,
                ) {
                    Ok(report) => report,
                    Err(_) => {
                        return ValueError::CommandProtocolUnavailable(
                            "namespace creation diagnostic purpose",
                        )
                        .into()
                    }
                };
                let error = match report.error_code {
                    Some(code) => {
                        tcl_cmd_core::CmdError::with_error_code_bytes(report.message, code)
                    }
                    None => tcl_cmd_core::CmdError::new_bytes(report.message),
                };
                error.with_native_string_result(report.string_protocol)
            }
            error => error.into_value_error().into(),
        }
    }
}

impl Interp {
    pub(crate) fn namespace_address_components(
        &self,
        current: NsId,
        original: &[u8],
    ) -> Result<(NsId, Vec<NameBytes>), NamespaceAddressError> {
        let unavailable = |purpose| {
            NamespaceAddressError::Unavailable(ValueError::CommandProtocolUnavailable(purpose))
        };
        let policy = self
            .name_policy_protocol()
            .ok_or_else(|| unavailable("namespace address issuer"))?;
        let protocol = policy.recipe();
        if protocol.is_jim084() {
            return Err(unavailable("Jim flat namespace object storage"));
        }
        let path = self
            .namespaces()
            .native_context_path(current)
            .ok_or_else(|| unavailable("namespace address context"))?;
        let context = NativeNameContext::new(&path);
        let selected = protocol
            .namespace_address_input(context, original)
            .map_err(|reason| NamespaceAddressError::Projection { protocol, reason })?;
        if let Some(refusal) = self.native_access_refusal() {
            return Err(NamespaceAddressError::Unavailable(refusal.into()));
        }
        let geometry = protocol
            .namespace_address_path(context, original)
            .map_err(|reason| NamespaceAddressError::Projection { protocol, reason })?;
        if selected.qualification() == tcl_syntax::naming::NativeNameQualification::Absolute {
            Ok((GLOBAL, geometry.as_segments().to_vec()))
        } else {
            Ok((
                current,
                geometry.as_segments()[path.as_segments().len()..].to_vec(),
            ))
        }
    }

    pub(crate) fn ensure_global_namespace_address(
        &mut self,
        original: &[u8],
    ) -> Result<NsId, tcl_cmd_core::CmdError> {
        self.invalidate_command_environment();
        let (anchor, components) = self
            .namespace_address_components(GLOBAL, original)
            .map_err(NamespaceAddressError::into_creation_error)?;
        Ok(self
            .namespaces_mut()
            .ensure_namespace_at_components(anchor, &components))
    }

    pub(crate) fn ensure_namespace_address(
        &mut self,
        original: &[u8],
    ) -> Result<NsId, tcl_cmd_core::CmdError> {
        self.invalidate_command_environment();
        self.publish_namespace_address(original)
    }

    pub(crate) fn publish_namespace_address(
        &mut self,
        original: &[u8],
    ) -> Result<NsId, tcl_cmd_core::CmdError> {
        let (anchor, components) = self
            .namespace_address_components(self.current_ns(), original)
            .map_err(NamespaceAddressError::into_creation_error)?;
        let dying_name = {
            let namespaces = self.namespaces();
            namespaces
                .dying_namespace_at_components(anchor, &components)
                .map(|namespace| namespaces.qualified_name(namespace))
        };
        if let Some(name) = dying_name {
            let mut message = b"can't create namespace \"".to_vec();
            message.extend_from_slice(&name);
            message.extend_from_slice(b"\": already exists");
            return Err(tcl_cmd_core::CmdError::new_bytes(message));
        }
        Ok(self
            .namespaces_mut()
            .ensure_namespace_at_components(anchor, &components))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_address_keeps_selected_extent_and_ignores_the_callers_namespace() {
        for version in tcl_dialect::TclVersion::ALL {
            let mut interpreter = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(version.dialect_profile_name()),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let caller = interpreter.ensure_namespace(b"::Caller");
            interpreter.set_current_ns(caller);
            let selected = interpreter
                .ensure_global_namespace_address(b"Selected\0Ignored")
                .unwrap();
            assert_eq!(interpreter.current_ns(), caller);
            assert_eq!(
                interpreter.namespaces().find_namespace(GLOBAL, b"Selected"),
                Some(selected),
            );
            assert!(interpreter
                .namespaces()
                .find_namespace(caller, b"Selected")
                .is_none());
            assert!(interpreter
                .namespaces()
                .find_namespace(GLOBAL, b"Selected\0Ignored")
                .is_none());
            assert_eq!(
                interpreter.ensure_global_namespace_address(b"").unwrap(),
                GLOBAL,
            );
        }
    }

    #[test]
    fn hidden_global_address_keeps_a_dying_token_through_its_original_callback() {
        for version in tcl_dialect::TclVersion::ALL {
            if version == tcl_dialect::TclVersion::V8_4 {
                continue;
            }
            let mut interpreter = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(version.dialect_profile_name()),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let source = br#"
set ::observations {}
proc during {name1 name2 operation} {
    foreach script {
        {interp invokehidden {} -namespace retiring hiddenNamespace current}
        {interp invokehidden {} hiddenNamespace exists retiring}
        {interp invokehidden {} hiddenNamespace eval retiring {}}
    } {
        set code [catch {uplevel #0 $script} value]
        lappend ::observations [list $code $value]
    }
}
namespace eval retiring {
    variable sentinel HELD
    trace add variable sentinel unset ::during
}
interp hide {} namespace hiddenNamespace
set deleted [catch {interp invokehidden {} hiddenNamespace delete retiring} message]
list $deleted $message $::observations [interp invokehidden {} hiddenNamespace exists retiring]
"#;
            assert_eq!(interpreter.eval_str(source), crate::interp::Code::Ok);
            assert_eq!(
                interpreter.result_bytes(),
                br#"0 {} {{0 ::retiring} {0 0} {1 {can't create namespace "retiring": already exists}}} 0"#,
            );
            assert_eq!(interpreter.current_ns(), GLOBAL);
            assert!(interpreter.native_execution_refusal().is_none());
        }
    }

    #[test]
    fn global_address_refuses_missing_or_jim_authority_before_creation() {
        for environment in ["irules", "jim"] {
            let mut interpreter = if environment == "jim" {
                Interp::with_native_core(
                    super::super::default_host(),
                    crate::environment::profile_for_dialect(environment),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap()
            } else {
                let mut interpreter = Interp::new();
                interpreter
                    .set_dialect_profile(crate::environment::profile_for_dialect(environment));
                interpreter
            };
            assert!(interpreter
                .ensure_global_namespace_address(b"Unadmitted")
                .is_err());
            assert!(interpreter
                .namespaces()
                .find_namespace(GLOBAL, b"Unadmitted")
                .is_none());
        }
    }
}
