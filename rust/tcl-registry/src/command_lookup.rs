// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native missing-command handler selection, separate from target lookup.

/// Actual dispatcher entry that selected the command lookup namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandLookupOrigin {
    /// Ordinary evaluated object-vector dispatch.
    Ordinary,
    /// An interpreter alias invokes its prefix through the native invoke flag.
    AliasInvocation,
    /// An ensemble forwards its selected original command prefix.
    EnsembleInvocation,
}

/// Actual C ensemble forwarding allocation and command-reference context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeEnsembleDispatchProtocol {
    /// C8.5 forwards through the global invoke context; C8.6+ selects the
    /// ensemble namespace without changing the caller's variable frame.
    pub use_ensemble_namespace: bool,
    /// C8.5 borrows trailing arguments beside a shared prefix List copy.
    /// C8.6+ copies only a two-word invocation; otherwise a fresh List owns
    /// the prefix members and the supplied parameter/argument objects.
    pub always_copy_prefix: bool,
}

impl crate::InvocationDialect {
    /// Select the authentic ensemble dispatch protocol independently of the
    /// assistance catalogue. Unknown and non-C issuers supply no native recipe.
    #[must_use]
    pub fn native_ensemble_dispatch_protocol(self) -> Option<NativeEnsembleDispatchProtocol> {
        let version = self.native_command_name_protocol()?.version();
        if version < tcl_dialect::TclVersion::V8_5 {
            return None;
        }
        Some(NativeEnsembleDispatchProtocol {
            use_ensemble_namespace: version >= tcl_dialect::TclVersion::V8_6,
            always_copy_prefix: version == tcl_dialect::TclVersion::V8_5,
        })
    }
}

/// Namespace from which a native missing-command handler is selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnknownHandlerNamespace {
    /// The actual caller's variable-frame namespace, including retained tokens.
    Caller,
    /// The independently selected command lookup namespace.
    Lookup,
}

/// Measured missing-command dispatch policy. Handler selection and handler-head
/// resolution are separate; the head is always resolved in the selected lookup
/// namespace and a missing handler reports the original missing command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeLookupFallbackPolicy {
    /// Namespace-handler selection; absent on monolithic C 8.4 and current Jim.
    pub namespace_handler: Option<UnknownHandlerNamespace>,
    /// Default prefix when no namespace handler is configured.
    pub default_handler: &'static str,
}

/// Resolve actual native policy without inferring an implementation from a
/// catalogue name or a platform's unmeasured compiler/runtime snapshot.
#[must_use]
pub fn native_lookup_fallback_policy(
    dialect: crate::InvocationDialect,
    origin: CommandLookupOrigin,
) -> Option<NativeLookupFallbackPolicy> {
    use tcl_dialect::{TclVersion, model::Family};
    let namespace_handler = match dialect.family()? {
        Family::Tcl => match dialect.tcl_version? {
            TclVersion::V8_4 => None,
            TclVersion::V8_5 if origin != CommandLookupOrigin::Ordinary => {
                Some(UnknownHandlerNamespace::Lookup)
            }
            TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1 => {
                Some(UnknownHandlerNamespace::Caller)
            }
        },
        Family::F5Irules
            if dialect.interpreter_protocol == Some(tcl_dialect::InterpreterProtocol::Tcl)
                && dialect.tcl_version == Some(TclVersion::V8_4) =>
        {
            // The platform's authored interpreter contract retains its 8.4
            // missing-command dispatcher independently of unknown compiler hooks.
            return Some(NativeLookupFallbackPolicy {
                namespace_handler: None,
                default_handler: "::unknown",
            });
        }
        Family::Jim
            if dialect
                .core_point
                .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84) =>
        {
            return Some(NativeLookupFallbackPolicy {
                namespace_handler: None,
                default_handler: "unknown",
            });
        }
        _ => return None,
    };
    Some(NativeLookupFallbackPolicy {
        namespace_handler,
        default_handler: "::unknown",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensemble_forwarding_preserves_actual_version_context_and_allocation() {
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            let protocol = dialect.native_ensemble_dispatch_protocol();
            if version == tcl_dialect::TclVersion::V8_4 {
                assert!(protocol.is_none());
                continue;
            }
            let protocol = protocol.unwrap();
            assert_eq!(
                protocol.use_ensemble_namespace,
                version >= tcl_dialect::TclVersion::V8_6
            );
            assert_eq!(
                protocol.always_copy_prefix,
                version == tcl_dialect::TclVersion::V8_5
            );
            let fallback =
                native_lookup_fallback_policy(dialect, CommandLookupOrigin::EnsembleInvocation)
                    .unwrap();
            assert_eq!(
                fallback.namespace_handler,
                Some(if version == tcl_dialect::TclVersion::V8_5 {
                    UnknownHandlerNamespace::Lookup
                } else {
                    UnknownHandlerNamespace::Caller
                })
            );
        }
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        let jim = crate::InvocationDialect::of_profile(&profile);
        assert!(jim.native_ensemble_dispatch_protocol().is_none());
    }

    #[test]
    fn alias_entry_selection_is_independent_of_handler_head_resolution() {
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            let ordinary =
                native_lookup_fallback_policy(dialect, CommandLookupOrigin::Ordinary).unwrap();
            let alias =
                native_lookup_fallback_policy(dialect, CommandLookupOrigin::AliasInvocation)
                    .unwrap();
            assert_eq!(alias.default_handler, "::unknown");
            assert_eq!(
                ordinary.namespace_handler,
                (version != tcl_dialect::TclVersion::V8_4)
                    .then_some(UnknownHandlerNamespace::Caller)
            );
            assert_eq!(
                alias.namespace_handler,
                match version {
                    tcl_dialect::TclVersion::V8_4 => None,
                    tcl_dialect::TclVersion::V8_5 => Some(UnknownHandlerNamespace::Lookup),
                    _ => Some(UnknownHandlerNamespace::Caller),
                }
            );
        }
        let f5 = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert_eq!(
            native_lookup_fallback_policy(f5, CommandLookupOrigin::AliasInvocation),
            Some(NativeLookupFallbackPolicy {
                namespace_handler: None,
                default_handler: "::unknown",
            })
        );
    }
}
