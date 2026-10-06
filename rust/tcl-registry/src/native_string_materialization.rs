// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authenticated native object string recipes and explicit logical providers.

use tcl_syntax::native_string::NativeStringProtocol;

/// Explicit logical provider; selecting it grants no native engine proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalStringProvider {
    /// Explicit F5 Tcl8.4 core object-string simulation, independent of native authority.
    Tcl84CoreSimulation,
}

/// A native object-string recipe or explicitly authored logical counterpart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeStringMaterialization {
    protocol: NativeStringProtocol,
    logical: Option<LogicalStringProvider>,
}

impl NativeStringMaterialization {
    /// Independently selected object-string recipe, including Jim's native object parser.
    #[must_use]
    pub const fn protocol(self) -> NativeStringProtocol {
        self.protocol
    }
    /// Explicit logical origin; absent for authentic native issuance.
    #[must_use]
    pub const fn logical_provider(self) -> Option<LogicalStringProvider> {
        self.logical
    }
}

/// A supported C byte-array string recipe, sealed before allocating its backing.
/// Retain this packet with the object; updater callbacks have no interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteArrayStringRecipe {
    protocol: NativeStringProtocol,
    logical: Option<LogicalStringProvider>,
}
impl ByteArrayStringRecipe {
    /// Supported C string recipe retained by this sealed backing capability.
    #[must_use]
    pub const fn protocol(self) -> NativeStringProtocol {
        self.protocol
    }
    /// Independently retained logical origin; absence identifies native issuance.
    #[must_use]
    pub const fn logical_provider(self) -> Option<LogicalStringProvider> {
        self.logical
    }
    /// C 9's empty byte-array updater installs canonical empty-string storage;
    /// C 8.4–8.6 allocate an empty buffer instead.
    #[must_use]
    pub fn canonical_empty(self) -> bool {
        self.protocol
            .tcl_version()
            .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
    }
}
impl crate::InvocationDialect {
    /// Actual native string recipe, separately audited from numeric support.
    #[must_use]
    pub fn native_string_protocol(self) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        let point = self.execution_point()?;
        if self
            .tcl_version
            .is_some_and(|version| point.tcl_version() != Some(version))
        {
            return None;
        }
        tcl_syntax::native_string::NativeStringProtocol::for_point(point)
    }

    /// Actual native name recipe, issued independently of numeral grammars.
    #[must_use]
    pub fn native_name_protocol(self) -> Option<tcl_syntax::naming::NativeNameProtocol> {
        let point = self.execution_point()?;
        if self
            .tcl_version
            .is_some_and(|version| point.tcl_version() != Some(version))
        {
            return None;
        }
        tcl_syntax::naming::NativeNameProtocol::for_point(point)
    }

    /// Select a pure authored naming recipe, without issuing native authority.
    /// Unversioned Tcl uses the explicit C8.6 analysis abstraction; vendor
    /// compatibility and absent F5 providers do not select a C recipe.
    #[must_use]
    pub fn authored_name_policy(self) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        use tcl_syntax::naming::{NamePolicyProtocol, NativeNameProtocol};
        self.native_name_protocol()
            .map(|recipe| match recipe {
                NativeNameProtocol::C(version) => NamePolicyProtocol::authored_tcl(version),
                NativeNameProtocol::Jim084 => NamePolicyProtocol::authored_jim084(),
            })
            .or_else(|| {
                (self.family() == Some(tcl_dialect::model::Family::Tcl)
                    && self.tcl_version.is_none()
                    && self.core_point.is_none())
                .then(|| NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6))
            })
    }

    /// Validate a separately authored logical F5 naming provider. Actual host
    /// attestation and native naming authority remain independent obligations.
    #[must_use]
    pub fn authored_logical_name_simulation(
        self,
        provider: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        use tcl_syntax::naming::{NamePolicyAuthority, NativeNameProtocol};
        self.authored_f5_tcl84_core()?;
        (provider.authority() == NamePolicyAuthority::AuthoredSimulation
            && provider.recipe() == NativeNameProtocol::C(tcl_dialect::TclVersion::V8_4))
        .then_some(provider)
    }

    /// Select the general object-string owner independently of byte-array support.
    /// Jim's string/list protocol is available even though it has no C byte-array updater.
    #[must_use]
    pub fn native_string_materialization(
        self,
        provider: Option<LogicalStringProvider>,
    ) -> Option<NativeStringMaterialization> {
        if let Some(protocol) = self.native_string_protocol() {
            return Some(NativeStringMaterialization {
                protocol,
                logical: None,
            });
        }
        let provider = provider?;
        self.authored_f5_tcl84_core()?;
        Some(NativeStringMaterialization {
            protocol: NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
            logical: Some(provider),
        })
    }

    /// Select a fully supported byte-array updater before creating physical storage.
    /// Jim has no C byte-array updater. Vendor compatibility alone grants nothing.
    #[must_use]
    pub fn byte_array_string_recipe(
        self,
        provider: Option<LogicalStringProvider>,
    ) -> Option<ByteArrayStringRecipe> {
        if let Some(protocol) = self.native_string_protocol() {
            protocol.tcl_version()?;
            return Some(ByteArrayStringRecipe {
                protocol,
                logical: None,
            });
        }
        let provider = provider?;
        self.authored_f5_tcl84_core()?;
        Some(ByteArrayStringRecipe {
            protocol: NativeStringProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_4),
            logical: Some(provider),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_name_recipes_never_issue_actual_native_authority() {
        use tcl_syntax::naming::{NamePolicyAuthority, NativeNameProtocol};
        for version in tcl_dialect::TclVersion::ALL {
            let policy = crate::InvocationDialect::for_version(version)
                .authored_name_policy()
                .unwrap();
            assert_eq!(policy.recipe(), NativeNameProtocol::C(version));
            assert_eq!(policy.authority(), NamePolicyAuthority::AuthoredSimulation);
        }
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        let policy = jim.authored_name_policy().unwrap();
        assert_eq!(policy.recipe(), NativeNameProtocol::Jim084);
        assert_eq!(policy.authority(), NamePolicyAuthority::AuthoredSimulation);
        assert_eq!(
            crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules())
                .authored_name_policy(),
            None
        );
        let plain = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("tcl").analyser_profile(),
        );
        assert_eq!(plain.native_name_protocol(), None);
        assert_eq!(
            plain.authored_name_policy().unwrap().recipe(),
            NativeNameProtocol::C(tcl_dialect::TclVersion::V8_6)
        );
    }

    #[test]
    fn byte_array_recipes_decline_jim_and_unknown_native_points() {
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        assert!(jim.native_string_protocol().unwrap().is_jim084());
        assert_eq!(jim.byte_array_string_recipe(None), None);
        let mut unknown = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        unknown.core_point = None;
        unknown.native_family = None;
        assert_eq!(unknown.native_string_protocol(), None);
        assert_eq!(
            unknown.byte_array_string_recipe(Some(LogicalStringProvider::Tcl84CoreSimulation)),
            None
        );
        let logical = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert_eq!(logical.byte_array_string_recipe(None), None);
        let recipe = logical
            .byte_array_string_recipe(Some(LogicalStringProvider::Tcl84CoreSimulation))
            .unwrap();
        assert_eq!(
            recipe.logical_provider(),
            Some(LogicalStringProvider::Tcl84CoreSimulation)
        );
        assert!(!recipe.canonical_empty());
    }
}
