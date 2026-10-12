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
    string_result: Option<tcl_syntax::native_string::NativeStringProtocol>,
}

impl NativeWrongArgumentsProtocol {
    /// Exact native list bytes published for an argument-count command failure.
    #[must_use]
    pub const fn error_code(self) -> &'static str {
        self.code
    }

    /// C Tcl's append-based arity presenter produces an unknown-count String.
    /// Jim and logical code recipes confer no native C String producer.
    #[must_use]
    pub const fn string_result(self) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        self.string_result
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
            string_result: string
                .tcl_version()
                .map(tcl_syntax::native_string::NativeStringProtocol::C),
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
            string_result: None,
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
        assert!(protocol.string_result().is_none());
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
        // naming.diagnostics.original-error-code-metadata-not-message
        // docs/design/analysis/name-resolution-proofs/diagnostics-original-error-code-metadata-not-message.md
        // Only the original public `catch {set}` error-code field supplies the
        // expectation. The separate origin/String assertions below exercise
        // software selection; these streams do not observe physical headers.
        for (name, original) in [
            (
                "tcl8.4",
                include_str!(
                    "../tests/data/native_error_code_metadata_original/providers/8.4.20/execute.stdout"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../tests/data/native_error_code_metadata_original/providers/8.5.19/execute.stdout"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../tests/data/native_error_code_metadata_original/providers/8.6.18/execute.stdout"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../tests/data/native_error_code_metadata_original/providers/9.0.4/execute.stdout"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../tests/data/native_error_code_metadata_original/providers/9.1.0/execute.stdout"
                ),
            ),
            (
                "jim",
                include_str!(
                    "../tests/data/native_error_code_metadata_original/providers/jim/execute.stdout"
                ),
            ),
        ] {
            let records = original
                .lines()
                .filter_map(|line| {
                    line.strip_prefix("CASE genuine-set-wrongargs code 1 result_hex ")
                })
                .collect::<Vec<_>>();
            assert_eq!(records.len(), 1, "{name}: exact original set-arity record");
            let (_, code_hex) = records[0]
                .split_once(" errorCode_available 1 errorCode_hex ")
                .expect("the original process observed a public errorCode value");
            assert_eq!(code_hex.len() % 2, 0, "{name}: complete counted hex");
            let expected = code_hex
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| {
                    u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16)
                        .expect("original emitted hexadecimal bytes")
                })
                .collect::<Vec<_>>();
            let environment = crate::model::ingress::resolve_environment(name);
            let protocol = crate::InvocationDialect::of_profile(environment.analyser_profile())
                .native_wrong_arguments_protocol()
                .unwrap();
            assert_eq!(protocol.error_code().as_bytes(), expected, "{name}");
            assert_eq!(protocol.origin(), NativeWrongArgumentsOrigin::Native);
            let dialect = crate::InvocationDialect::of_profile(environment.analyser_profile());
            assert_eq!(
                protocol.string_result(),
                dialect
                    .tcl_version
                    .map(tcl_syntax::native_string::NativeStringProtocol::C),
                "{name}"
            );
        }
    }
}
