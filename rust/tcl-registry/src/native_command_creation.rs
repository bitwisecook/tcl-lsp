// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected native object-command creation retirement recipes.
//!
//! Original public C and Jim creation callbacks overwrite an idle command
//! recreated by the original delete procedure without calling that intermediate
//! command's delete procedure. The recipe supplies no table, live holder,
//! unique object ownership, active-command or procedure lifetime evidence.

use tcl_syntax::native_string::NativeStringProtocol;

/// Original producer purpose retained on a runtime object-command record.
/// The tag describes issuance; it grants no live table or address authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeObjectCommandCreationPurpose {
    /// Legacy public C object-command constructor and its original callback ABI.
    LegacyPublicObjectCommand,
    /// Counted Engine callback publication, with its independent receipt contract.
    CountedEngineObjectCommand,
    /// Runtime-owned Rust shim storage, whose cleanup remains independently owned.
    OwnedRuntimeShim,
}

/// Retirement of an independently owned idle object-command record overwritten
/// by the original public object-command creation operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRecreatedObjectRetirement {
    /// Discard the intermediate record without another delete-procedure call.
    DiscardWithoutDeleteProcedure,
}

/// Public registration's selected name producer. Actual namespace, key object
/// lifetime and command-slot ownership remain independent runtime obligations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePublicCommandRegistrationRecipe {
    /// C public registration projects unqualified names globally and qualified
    /// names through the original current namespace.
    CNamespacePublication,
    /// `Jim_CreateCommand` constructs a fresh `CString` name in the root table;
    /// `Jim_RegisterCommand` then retains its independently owned table key.
    JimRootCStringKey,
}

/// Native creation rules selected independently of actual binding authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCommandCreationProtocol(NativeStringProtocol);

impl NativeCommandCreationProtocol {
    /// Independently selected engine recipe; this grants no live command slot.
    #[must_use]
    pub const fn protocol(self) -> NativeStringProtocol {
        self.0
    }

    /// Original C deleteProc/import ordering. Jim's hash-entry/refcount owner
    /// has a separate schedule and receives no C import-retirement recipe.
    /// Actual dying-token/recursive deletion guards remain embedding evidence.
    #[must_use]
    pub const fn c_deletion_order(
        self,
    ) -> Option<tcl_cmd_core::native_command_retirement::NativeCommandRetirementOrder> {
        use tcl_cmd_core::native_command_retirement::NativeCommandRetirementOrder;
        use tcl_dialect::TclVersion;
        match self.0 {
            NativeStringProtocol::C(TclVersion::V8_4 | TclVersion::V8_5) => {
                Some(NativeCommandRetirementOrder::ImplementationBeforeImports)
            }
            NativeStringProtocol::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1) => {
                Some(NativeCommandRetirementOrder::ImportsBeforeImplementation)
            }
            NativeStringProtocol::Jim084 => None,
        }
    }

    /// Select the actual public constructor facade, independently of a stored
    /// command's cleanup purpose. Jim has no counted Engine constructor recipe.
    #[must_use]
    pub const fn public_registration_recipe(
        self,
        purpose: NativeObjectCommandCreationPurpose,
    ) -> Option<NativePublicCommandRegistrationRecipe> {
        match (self.0, purpose) {
            (
                NativeStringProtocol::C(_),
                NativeObjectCommandCreationPurpose::LegacyPublicObjectCommand
                | NativeObjectCommandCreationPurpose::CountedEngineObjectCommand,
            ) => Some(NativePublicCommandRegistrationRecipe::CNamespacePublication),
            (
                NativeStringProtocol::Jim084,
                NativeObjectCommandCreationPurpose::LegacyPublicObjectCommand,
            ) => Some(NativePublicCommandRegistrationRecipe::JimRootCStringKey),
            _ => None,
        }
    }

    /// Project the genuine Jim public API's root `CString` name through the
    /// existing name owner. This describes key units, never a live key header.
    #[must_use]
    pub fn public_jim_registration_key(
        self,
        purpose: NativeObjectCommandCreationPurpose,
        original: &[u8],
    ) -> Option<tcl_syntax::naming::NativeJimCommandTableKey> {
        use tcl_syntax::naming::{NativeJimCommandTableKey, NativeNameContext, NativeNameProtocol};
        if self.public_registration_recipe(purpose)
            != Some(NativePublicCommandRegistrationRecipe::JimRootCStringKey)
        {
            return None;
        }
        let selected = NativeNameProtocol::Jim084
            .command_publication_input(
                NativeNameContext::root(),
                tcl_core_types::c_string_extent(original),
            )
            .ok()?;
        NativeJimCommandTableKey::from_projection(&selected)
    }

    /// The measured idle object-command overwrite purpose. Actual operation,
    /// callback provenance and unique record ownership remain caller checks.
    #[must_use]
    pub const fn idle_recreated_object_retirement(
        self,
        purpose: NativeObjectCommandCreationPurpose,
    ) -> Option<NativeRecreatedObjectRetirement> {
        match purpose {
            NativeObjectCommandCreationPurpose::LegacyPublicObjectCommand => {
                Some(NativeRecreatedObjectRetirement::DiscardWithoutDeleteProcedure)
            }
            NativeObjectCommandCreationPurpose::CountedEngineObjectCommand
            | NativeObjectCommandCreationPurpose::OwnedRuntimeShim => None,
        }
    }
}

impl crate::InvocationDialect {
    /// Select original C/Jim creation rules from the supported native point.
    /// Authored compatibility, logical simulation and unknown points abstain.
    #[must_use]
    pub fn native_command_creation_protocol(self) -> Option<NativeCommandCreationProtocol> {
        self.native_string_protocol()
            .map(NativeCommandCreationProtocol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_public_registration_selects_c_namespace_and_jim_root_cstring_purposes() {
        // Pure source-selected registration units; no live table/header or
        // counted Jim API equivalence is inferred from the retirement rows.
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let dialect = crate::InvocationDialect::of_profile(
                crate::model::ingress::resolve_environment(environment).analyser_profile(),
            );
            let selected = dialect.native_command_creation_protocol().unwrap();
            let jim = selected.protocol() == NativeStringProtocol::Jim084;
            assert_eq!(
                selected.public_registration_recipe(
                    NativeObjectCommandCreationPurpose::LegacyPublicObjectCommand
                ),
                Some(if jim {
                    NativePublicCommandRegistrationRecipe::JimRootCStringKey
                } else {
                    NativePublicCommandRegistrationRecipe::CNamespacePublication
                })
            );
            assert_eq!(
                selected.public_registration_recipe(
                    NativeObjectCommandCreationPurpose::CountedEngineObjectCommand
                ),
                (!jim).then_some(NativePublicCommandRegistrationRecipe::CNamespacePublication)
            );
            assert!(
                selected
                    .public_registration_recipe(
                        NativeObjectCommandCreationPurpose::OwnedRuntimeShim
                    )
                    .is_none()
            );
            let key = selected.public_jim_registration_key(
                NativeObjectCommandCreationPurpose::LegacyPublicObjectCommand,
                b"::rootEntry\0ignored",
            );
            if jim {
                let key = key.unwrap();
                assert_eq!(key.report_bytes(), b"::rootEntry");
                assert_eq!(key.comparison_bytes(), b"rootEntry");
            } else {
                assert!(key.is_none());
            }
            assert!(
                selected
                    .public_jim_registration_key(
                        NativeObjectCommandCreationPurpose::CountedEngineObjectCommand,
                        b"::rootEntry"
                    )
                    .is_none()
            );
        }
        for environment in ["tcl", "irules"] {
            let dialect = crate::InvocationDialect::of_profile(
                crate::model::ingress::resolve_environment(environment).analyser_profile(),
            );
            assert!(dialect.native_command_creation_protocol().is_none());
        }
    }

    #[test]
    fn original_idle_recreation_recipe_requires_a_supported_native_point() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let dialect = crate::InvocationDialect::of_profile(
                crate::model::ingress::resolve_environment(environment).analyser_profile(),
            );
            let selected = dialect.native_command_creation_protocol().unwrap();
            assert_eq!(
                selected.protocol(),
                dialect.native_string_protocol().unwrap()
            );
            assert_eq!(
                selected.idle_recreated_object_retirement(
                    NativeObjectCommandCreationPurpose::LegacyPublicObjectCommand
                ),
                Some(NativeRecreatedObjectRetirement::DiscardWithoutDeleteProcedure)
            );
            for purpose in [
                NativeObjectCommandCreationPurpose::CountedEngineObjectCommand,
                NativeObjectCommandCreationPurpose::OwnedRuntimeShim,
            ] {
                assert_eq!(selected.idle_recreated_object_retirement(purpose), None);
            }
        }
        let authored = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("tcl").analyser_profile(),
        );
        assert!(authored.native_command_creation_protocol().is_none());
        let logical = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert!(logical.native_command_creation_protocol().is_none());
        let mut missing = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        missing.core_point = None;
        missing.native_family = None;
        assert!(missing.native_command_creation_protocol().is_none());
        let mut foreign = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        foreign.tcl_version = Some(tcl_dialect::TclVersion::V9_1);
        assert!(foreign.native_command_creation_protocol().is_none());
    }
}

#[cfg(test)]
mod original_deletion_order_tests {
    #[test]
    fn original_c_deletion_order_preserves_release_split_and_independent_jim_owner() {
        // Pure source recipe; no live command/Namespace, callback or Native role.
        use tcl_cmd_core::native_command_retirement::NativeCommandRetirementOrder::*;
        for (environment, expected) in [
            ("tcl8.4", Some(ImplementationBeforeImports)),
            ("tcl8.5", Some(ImplementationBeforeImports)),
            ("tcl8.6", Some(ImportsBeforeImplementation)),
            ("tcl9.0", Some(ImportsBeforeImplementation)),
            ("tcl9.1", Some(ImportsBeforeImplementation)),
            ("jim", None),
        ] {
            let dialect = crate::InvocationDialect::of_profile(
                crate::model::ingress::resolve_environment(environment).analyser_profile(),
            );
            assert_eq!(
                dialect
                    .native_command_creation_protocol()
                    .unwrap()
                    .c_deletion_order(),
                expected
            );
        }
        for environment in ["tcl", "irules"] {
            let dialect = crate::InvocationDialect::of_profile(
                crate::model::ingress::resolve_environment(environment).analyser_profile(),
            );
            assert!(dialect.native_command_creation_protocol().is_none());
        }
    }
}
