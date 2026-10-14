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
