// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independently authenticated physical C namespace-name object semantics.

use crate::InvocationDialect;
use tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe;

/// Actual C engine permission for physical namespace-name primary operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeNamespaceNameProtocol(NativeNamespaceNameRecipe);

/// Exact invocation route after global presence of the import preload helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNamespaceImportPreloadRoute {
    /// C8.4 invokes the already selected global command's objProc directly.
    GlobalDirect,
    /// C8.5+ evaluate the unqualified generated head in the current namespace.
    /// Their Tcl_EvalObjv flags are TCL_GLOBAL_ONLY (1), not TCL_EVAL_GLOBAL.
    CurrentEvaluation,
}

impl NativeNamespaceNameProtocol {
    /// Select only the actual C preload invocation route; helper spelling and
    /// source transfer knowledge remain in the existing NamespaceTransition.
    #[must_use]
    pub const fn import_preload_route(self) -> NativeNamespaceImportPreloadRoute {
        if matches!(self.0.version(), tcl_dialect::TclVersion::V8_4) {
            NativeNamespaceImportPreloadRoute::GlobalDirect
        } else {
            NativeNamespaceImportPreloadRoute::CurrentEvaluation
        }
    }

    /// C8.5 added a no-pattern query; C8.4 simply returns empty success.
    #[must_use]
    pub const fn import_queries_existing_commands(self) -> bool {
        !matches!(self.0.version(), tcl_dialect::TclVersion::V8_4)
    }

    /// Pure release recipe selected by this authenticated native engine.
    #[must_use]
    pub const fn recipe(self) -> NativeNamespaceNameRecipe {
        self.0
    }
}

impl InvocationDialect {
    /// Authenticate the actual C object issuer independently of logical grammar.
    /// Jim and vendor compatibility alone grant no C namespace primary.
    #[must_use]
    pub fn native_namespace_name_protocol(self) -> Option<NativeNamespaceNameProtocol> {
        self.native_string_protocol()?.tcl_version().map(|version| {
            NativeNamespaceNameProtocol(NativeNamespaceNameRecipe::for_tcl_version(version))
        })
    }
    /// Validate an explicitly authored F5 result recipe without issuing nsName
    /// cache permission for the physical host.
    #[must_use]
    pub fn authored_namespace_result_recipe(
        self,
        provider: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<NativeNamespaceNameRecipe> {
        self.authored_logical_name_simulation(provider)?;
        Some(NativeNamespaceNameRecipe::for_tcl_version(
            tcl_dialect::TclVersion::V8_4,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;
    use tcl_syntax::naming::NamePolicyProtocol;

    #[test]
    fn import_preload_routes_keep_global_direct_separate_from_current_evaluation() {
        for version in TclVersion::ALL {
            let protocol = InvocationDialect::for_version(version)
                .native_namespace_name_protocol()
                .unwrap();
            assert_eq!(
                protocol.import_preload_route(),
                if version == TclVersion::V8_4 {
                    NativeNamespaceImportPreloadRoute::GlobalDirect
                } else {
                    NativeNamespaceImportPreloadRoute::CurrentEvaluation
                }
            );
            assert_eq!(
                protocol.import_queries_existing_commands(),
                version != TclVersion::V8_4
            );
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert!(jim.native_namespace_name_protocol().is_none());
    }

    #[test]
    fn native_and_authored_namespace_permissions_are_independent() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let dialect = InvocationDialect::for_version(version);
            assert_eq!(
                dialect
                    .native_namespace_name_protocol()
                    .unwrap()
                    .recipe()
                    .version(),
                version
            );
            assert!(
                dialect
                    .authored_namespace_result_recipe(NamePolicyProtocol::authored_tcl(
                        TclVersion::V8_4
                    ))
                    .is_none()
            );
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert!(jim.native_namespace_name_protocol().is_none());
        let logical = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("f5-irules").unit_profile(),
        );
        assert!(logical.native_namespace_name_protocol().is_none());
        assert_eq!(
            logical
                .authored_namespace_result_recipe(NamePolicyProtocol::authored_tcl(
                    TclVersion::V8_4
                ))
                .unwrap()
                .version(),
            TclVersion::V8_4
        );
        assert!(
            logical
                .authored_namespace_result_recipe(NamePolicyProtocol::authored_tcl(
                    TclVersion::V9_0
                ))
                .is_none()
        );
    }
}
