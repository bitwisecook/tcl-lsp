// SPDX-License-Identifier: AGPL-3.0-or-later
//! Argument-count error metadata selected independently of result-message bytes.

use tcl_dialect::TclVersion;

/// Deliberately authored logical argument-count error provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalWrongArgumentsProvider {
    /// F5 iRules/iApps core uses the C Tcl 8.4 command error recipe.
    Tcl84CoreSimulation,
}

/// Authority retained separately from the selected command error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeWrongArgumentsOrigin {
    /// Independently authenticated actual engine and build.
    Native,
    /// Explicit logical simulation; no native execution proof is granted.
    Logical(LogicalWrongArgumentsProvider),
}

/// Selected command-completion wrong-arguments error metadata.
/// This does not describe a context-free primitive API's error-state timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeWrongArgumentsProtocol {
    code: &'static str,
    origin: NativeWrongArgumentsOrigin,
}

impl NativeWrongArgumentsProtocol {
    /// Exact native list bytes published for an argument-count command failure.
    #[must_use]
    pub const fn error_code(self) -> &'static str {
        self.code
    }

    /// Actual engine versus explicitly authored logical provider.
    #[must_use]
    pub const fn origin(self) -> NativeWrongArgumentsOrigin {
        self.origin
    }
}

impl crate::InvocationDialect {
    /// Authenticate actual argument-count metadata without reading message text.
    #[must_use]
    pub fn native_wrong_arguments_protocol(self) -> Option<NativeWrongArgumentsProtocol> {
        let string = self.native_string_protocol()?;
        let code = match string.tcl_version() {
            Some(TclVersion::V8_4 | TclVersion::V8_5) => "NONE",
            Some(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1) => "TCL WRONGARGS",
            None if string.is_jim084() => "NONE",
            None => return None,
        };
        Some(NativeWrongArgumentsProtocol {
            code,
            origin: NativeWrongArgumentsOrigin::Native,
        })
    }

    /// Select actual native metadata or a deliberately requested logical provider.
    #[must_use]
    pub fn wrong_arguments_protocol(
        self,
        provider: Option<LogicalWrongArgumentsProvider>,
    ) -> Option<NativeWrongArgumentsProtocol> {
        if let Some(native) = self.native_wrong_arguments_protocol() {
            return Some(native);
        }
        let provider = provider?;
        self.authored_f5_tcl84_core()?;
        Some(NativeWrongArgumentsProtocol {
            code: "NONE",
            origin: NativeWrongArgumentsOrigin::Logical(provider),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logical_wrong_arguments_never_claim_native_authority() {
        let logical = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert!(logical.native_wrong_arguments_protocol().is_none());
        assert!(logical.wrong_arguments_protocol(None).is_none());
        let protocol = logical
            .wrong_arguments_protocol(Some(LogicalWrongArgumentsProvider::Tcl84CoreSimulation))
            .unwrap();
        assert_eq!(
            protocol.origin(),
            NativeWrongArgumentsOrigin::Logical(LogicalWrongArgumentsProvider::Tcl84CoreSimulation)
        );
        assert_eq!(protocol.error_code(), "NONE");
        let mut unknown = crate::InvocationDialect::for_version(TclVersion::V9_0);
        unknown.native_family = None;
        unknown.core_point = None;
        assert!(
            unknown
                .wrong_arguments_protocol(Some(LogicalWrongArgumentsProvider::Tcl84CoreSimulation))
                .is_none()
        );
    }

    #[test]
    fn measured_native_codes_have_independent_origin() {
        for (name, expected) in [
            ("tcl8.4", "NONE"),
            ("tcl8.5", "NONE"),
            ("tcl8.6", "TCL WRONGARGS"),
            ("tcl9.0", "TCL WRONGARGS"),
            ("tcl9.1", "TCL WRONGARGS"),
            ("jim", "NONE"),
        ] {
            let environment = crate::model::ingress::resolve_environment(name);
            let protocol = crate::InvocationDialect::of_profile(environment.analyser_profile())
                .native_wrong_arguments_protocol()
                .unwrap();
            assert_eq!(protocol.error_code(), expected, "{name}");
            assert_eq!(protocol.origin(), NativeWrongArgumentsOrigin::Native);
        }
    }
}
