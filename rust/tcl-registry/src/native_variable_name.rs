// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independently authenticated C variable-name primary recipes.

impl crate::InvocationDialect {
    /// Actual C cache origin, independent of logical/source compatibility versions.
    #[must_use]
    pub fn native_variable_name_protocol(
        self,
    ) -> Option<tcl_syntax::native_variable_name::NativeVariableNameProtocol> {
        match self.native_name_protocol()? {
            tcl_syntax::naming::NativeNameProtocol::C(version) => Some(
                tcl_syntax::native_variable_name::NativeVariableNameProtocol::for_tcl_version(
                    version,
                ),
            ),
            tcl_syntax::naming::NativeNameProtocol::Jim084 => None,
        }
    }
}
