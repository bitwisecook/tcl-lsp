// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual `namespace code` handler recognition, independent of its compiler.

/// Audited recognition rule for returning the original command-prefix object.
/// This is a byte predicate, not a list parser or a command-name resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeNamespaceCodePolicy {
    rule: PrefixRule,
}

/// A deliberately installed logical handler simulation, not native evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalNamespaceCodeProvider {
    /// F5 iRules/iApps use this explicitly authored C Tcl 8.4 core simulation.
    Tcl84CoreSimulation,
}

/// Selected normal-handler behavior, retaining native versus simulation origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceCodeHandlerPolicy {
    /// Behavior selected from independently retained actual engine facts.
    Native(NativeNamespaceCodePolicy),
    /// Behavior deliberately supplied by a logical dialect simulation.
    Logical(LogicalNamespaceCodePolicy),
}

/// Authored logical recognition recipe. It grants no native compiler authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalNamespaceCodePolicy {
    provider: LogicalNamespaceCodeProvider,
}

impl NamespaceCodeHandlerPolicy {
    /// The selected handler's recognition predicate on exact argument bytes.
    #[must_use]
    pub fn preserves_argument(self, bytes: &[u8]) -> bool {
        match self {
            Self::Native(policy) => policy.preserves_argument(bytes),
            Self::Logical(policy) => match policy.provider {
                LogicalNamespaceCodeProvider::Tcl84CoreSimulation => NativeNamespaceCodePolicy {
                    rule: PrefixRule::Tcl84,
                }
                .preserves_argument(bytes),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrefixRule {
    Tcl84,
    Tcl85Plus,
    Jim084,
}

impl NativeNamespaceCodePolicy {
    /// Whether the handler returns the original argument object unchanged.
    /// A false result constructs a four-element list retaining that same object
    /// as its last element. Neither path decodes or reparses its bytes.
    #[must_use]
    pub fn preserves_argument(self, bytes: &[u8]) -> bool {
        const PREFIX: &[u8] = b"::namespace inscope ";
        match self.rule {
            PrefixRule::Tcl84 => {
                // NamespaceCodeCmd, C8.4: leading colons are skipped, the
                // residual size must exceed 17, and only ASCII spaces after
                // "namespace" are skipped. No token delimiter is required.
                let offset = bytes
                    .iter()
                    .position(|byte| *byte != b':')
                    .unwrap_or(bytes.len());
                let residual = &bytes[offset..];
                if residual.len() <= 17 || !residual.starts_with(b"namespace") {
                    return false;
                }
                let suffix = &residual[9..];
                let offset = suffix
                    .iter()
                    .position(|byte| *byte != b' ')
                    .unwrap_or(suffix.len());
                suffix[offset..].starts_with(b"inscope")
            }
            PrefixRule::Tcl85Plus => bytes.len() > PREFIX.len() && bytes.starts_with(PREFIX),
            PrefixRule::Jim084 => bytes.starts_with(PREFIX),
        }
    }
}

impl crate::InvocationDialect {
    /// Select the actual handler rule from independently retained engine facts.
    /// Unknown engines, unaudited builds and vendor compatibility projections
    /// do not acquire a C or Jim handler implementation.
    #[must_use]
    pub fn native_namespace_code_policy(self) -> Option<NativeNamespaceCodePolicy> {
        use tcl_dialect::TclVersion;
        let engine = self.native_string_protocol()?;
        let rule = match engine.tcl_version() {
            Some(TclVersion::V8_4) => PrefixRule::Tcl84,
            Some(TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1) => {
                PrefixRule::Tcl85Plus
            }
            None if engine.is_jim084() => PrefixRule::Jim084,
            None => return None,
        };
        Some(NativeNamespaceCodePolicy { rule })
    }

    /// Select native behavior first, or an explicitly requested authored
    /// logical simulation. A missing provider supplies no vendor behavior.
    #[must_use]
    pub fn namespace_code_handler_policy(
        self,
        provider: Option<LogicalNamespaceCodeProvider>,
    ) -> Option<NamespaceCodeHandlerPolicy> {
        if let Some(policy) = self.native_namespace_code_policy() {
            return Some(NamespaceCodeHandlerPolicy::Native(policy));
        }
        let provider = provider?;
        self.authored_f5_tcl84_core()?;
        Some(NamespaceCodeHandlerPolicy::Logical(
            LogicalNamespaceCodePolicy { provider },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPECIMENS: [&[u8]; 11] = [
        b"::namespace inscope ",
        b"::namespace inscope :: cmd",
        b"namespace inscope :: cmd",
        b":namespace inscope :: cmd",
        b"::::namespace inscope :: cmd",
        b"namespace    inscope :: cmd",
        b"namespace\tinscope :: cmd",
        b"namespaceinscopeXX",
        b"namespaceinscopeX",
        b"::namespace inscopeX",
        b"::namespace inscope \0",
    ];

    #[test]
    fn handler_recognition_matches_all_66_measured_native_rows() {
        let jim = crate::model::ingress::resolve_environment("jim").analyser_profile();
        let mut rows = 0;
        for dialect in tcl_dialect::TclVersion::ALL
            .map(crate::InvocationDialect::for_version)
            .into_iter()
            .chain(std::iter::once(crate::InvocationDialect::of_profile(jim)))
        {
            let preserved: &[usize] = if dialect.tcl_version == Some(tcl_dialect::TclVersion::V8_4)
            {
                &[0, 1, 2, 3, 4, 5, 7, 9, 10]
            } else if dialect.native_scalar_getter_protocol().unwrap().is_jim084() {
                &[0, 1, 10]
            } else {
                &[1, 10]
            };
            let policy = dialect
                .native_namespace_code_policy()
                .expect("audited native engine");
            for (index, specimen) in SPECIMENS.into_iter().enumerate() {
                assert_eq!(
                    policy.preserves_argument(specimen),
                    preserved.contains(&index),
                    "{dialect:?}: specimen {index}"
                );
                rows += 1;
            }
        }
        assert_eq!(rows, 66);
    }

    #[test]
    fn recognition_retains_raw_bytes_and_declines_unknown_engine_facts() {
        for rule in [PrefixRule::Tcl84, PrefixRule::Tcl85Plus, PrefixRule::Jim084] {
            let policy = NativeNamespaceCodePolicy { rule };
            assert!(policy.preserves_argument(b"::namespace inscope \xff\0"));
            assert!(!policy.preserves_argument(b"::namespace\0inscope :: cmd"));
        }
        let mut unknown = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        unknown.core_point = None;
        unknown.tcl_version = None;
        assert!(unknown.native_namespace_code_policy().is_none());
        let f5 = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert!(f5.native_namespace_code_policy().is_none());
        let mut conflict = crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::TCL_8_4),
        );
        conflict.tcl_version = Some(tcl_dialect::TclVersion::V9_0);
        assert!(conflict.native_namespace_code_policy().is_none());
    }

    #[test]
    fn logical_f5_simulation_is_explicit_and_never_native_engine_evidence() {
        let provider = Some(LogicalNamespaceCodeProvider::Tcl84CoreSimulation);
        let dialect = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert!(dialect.native_namespace_code_policy().is_none());
        assert!(dialect.namespace_code_handler_policy(None).is_none());
        let policy = dialect
            .namespace_code_handler_policy(provider)
            .expect("authored F5 provider");
        assert!(matches!(policy, NamespaceCodeHandlerPolicy::Logical(_)));
        assert!(policy.preserves_argument(b"namespaceinscopeXX"));
        assert!(!policy.preserves_argument(b"namespace\tinscope :: cmd"));
        let mut conflict = dialect;
        conflict.tcl_version = Some(tcl_dialect::TclVersion::V9_0);
        assert!(conflict.namespace_code_handler_policy(provider).is_none());
        let unknown = crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        );
        assert!(unknown.namespace_code_handler_policy(provider).is_none());
        let native = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        assert!(matches!(
            native.namespace_code_handler_policy(provider),
            Some(NamespaceCodeHandlerPolicy::Native(_))
        ));
        for release in [
            tcl_dialect::model::Release::F5_TCL_TMOS,
            tcl_dialect::model::Release::F5_IRULES_TMM,
        ] {
            let dialect = crate::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::canonical(release),
            );
            assert!(dialect.native_namespace_code_policy().is_none());
            assert!(matches!(
                dialect.namespace_code_handler_policy(provider),
                Some(NamespaceCodeHandlerPolicy::Logical(_))
            ));
        }
    }
}
