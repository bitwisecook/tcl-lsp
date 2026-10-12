// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Variable table arithmetic selected independently of key extent and ABI authority.

use tcl_core_types::{NativeHashAbi, NativeHashBytePromotion, NativeHashRecipe};
use tcl_dialect::TclVersion;
use tcl_syntax::naming::{NamePolicyAuthority, NamePolicyProtocol, NativeNameProtocol};

/// An authenticated engine policy combined with independently supplied ABI facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeVariableTableProtocol {
    recipe: NativeHashRecipe,
    names: NativeNameProtocol,
    logical: bool,
}

impl NativeVariableTableProtocol {
    fn select(names: NativeNameProtocol, abi: NativeHashAbi, logical: bool) -> Option<Self> {
        let recipe = match names {
            NativeNameProtocol::C(version) => NativeHashRecipe::Tcl {
                promotion: if version <= TclVersion::V8_5 {
                    abi.plain_char
                } else {
                    NativeHashBytePromotion::Unsigned
                },
                width: if version < TclVersion::V9_0 {
                    abi.unsigned_int
                } else {
                    abi.size_t
                },
            },
            NativeNameProtocol::Jim084 => NativeHashRecipe::Jim {
                seed: abi.jim_seed?,
            },
        };
        Some(Self {
            recipe,
            names,
            logical,
        })
    }
    /// Hash and bucket recipe for an already selected stored key.
    #[must_use]
    pub const fn recipe(self) -> NativeHashRecipe {
        self.recipe
    }
    /// Original key policy; hashing never reinterprets the key's byte extent.
    #[must_use]
    pub const fn names(self) -> NativeNameProtocol {
        self.names
    }
    /// Whether arrays use Jim's independently owned Dictionary entry vector.
    #[must_use]
    pub const fn dictionary_arrays(self) -> bool {
        matches!(self.names, NativeNameProtocol::Jim084)
    }
    /// An explicit authored simulation, separate from native engine authority.
    #[must_use]
    pub const fn is_logical(self) -> bool {
        self.logical
    }
}

impl crate::InvocationDialect {
    /// Select the actual audited native variable table with independent ABI facts.
    #[must_use]
    pub fn native_variable_table_protocol(
        self,
        abi: NativeHashAbi,
    ) -> Option<NativeVariableTableProtocol> {
        NativeVariableTableProtocol::select(self.native_name_protocol()?, abi, false)
    }
    /// Select an explicitly installed logical F5 provider with independent ABI facts.
    #[must_use]
    pub fn authored_variable_table_protocol(
        self,
        names: NamePolicyProtocol,
        abi: NativeHashAbi,
    ) -> Option<NativeVariableTableProtocol> {
        self.authored_logical_name_simulation(names)?;
        (names.authority() == NamePolicyAuthority::AuthoredSimulation).then_some(())?;
        NativeVariableTableProtocol::select(names.recipe(), abi, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_core_types::NativeHashWordWidth;

    #[test]
    fn actual_variable_hash_uses_independent_abi_and_jim_seed() {
        let abi = NativeHashAbi {
            plain_char: NativeHashBytePromotion::Signed,
            unsigned_int: NativeHashWordWidth::Bits32,
            size_t: NativeHashWordWidth::Bits64,
            jim_seed: None,
        };
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = crate::InvocationDialect::for_version(version)
                .native_variable_table_protocol(abi)
                .unwrap();
            assert_eq!(
                protocol.recipe(),
                NativeHashRecipe::Tcl {
                    promotion: if version <= TclVersion::V8_5 {
                        NativeHashBytePromotion::Signed
                    } else {
                        NativeHashBytePromotion::Unsigned
                    },
                    width: if version < TclVersion::V9_0 {
                        NativeHashWordWidth::Bits32
                    } else {
                        NativeHashWordWidth::Bits64
                    },
                }
            );
        }
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        assert!(jim.native_variable_table_protocol(abi).is_none());
        let seeded = NativeHashAbi {
            jim_seed: Some(7),
            ..abi
        };
        assert_eq!(
            jim.native_variable_table_protocol(seeded).unwrap().recipe(),
            NativeHashRecipe::Jim { seed: 7 }
        );
        let f5 = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert!(f5.native_variable_table_protocol(seeded).is_none());
        assert!(
            f5.authored_variable_table_protocol(
                NamePolicyProtocol::authored_tcl(TclVersion::V8_4),
                seeded
            )
            .unwrap()
            .is_logical()
        );
    }
}
impl crate::InvocationDialect {
    /// Select actual audited C search handlers; Jim has no array-search commands.
    #[must_use]
    pub fn native_array_search_protocol(
        self,
        abi: tcl_core_types::NativeArraySearchAbi,
    ) -> Option<tcl_syntax::native_array_search::NativeArraySearchProtocol> {
        let NativeNameProtocol::C(version) = self.native_name_protocol()? else {
            return None;
        };
        tcl_syntax::native_array_search::NativeArraySearchProtocol::for_tcl_version(version, abi)
    }
}

impl crate::InvocationDialect {
    /// Actual audited presence of the four C array-search members, independently
    /// of an inherited advisory surface or the selected integer ABI.
    #[must_use]
    pub fn native_array_search_member_present(self, member: &[u8]) -> Option<bool> {
        if !matches!(
            member,
            b"anymore" | b"donesearch" | b"nextelement" | b"startsearch"
        ) {
            return None;
        }
        Some(matches!(
            self.native_name_protocol()?,
            NativeNameProtocol::C(_)
        ))
    }
}

#[cfg(test)]
mod array_search_tests {
    use crate::InvocationDialect;
    use tcl_core_types::{NativeArraySearchAbi, NativeHashWordWidth};
    use tcl_dialect::TclVersion;
    #[test]
    fn actual_search_presence_and_abi_are_independent_of_compatibility() {
        let abi = NativeArraySearchAbi {
            unsigned_long: NativeHashWordWidth::Bits64,
            int_bits: 32,
        };
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let dialect = InvocationDialect::for_version(version);
            assert_eq!(
                dialect.native_array_search_member_present(b"startsearch"),
                Some(true)
            );
            assert!(dialect.native_array_search_protocol(abi).is_some());
            assert!(
                dialect
                    .native_array_search_protocol(NativeArraySearchAbi {
                        int_bits: 64,
                        ..abi
                    })
                    .is_none()
            );
        }
        let jim =
            InvocationDialect::of_profile(crate::model::resolve_environment("jim").unit_profile());
        assert_eq!(
            jim.native_array_search_member_present(b"anymore"),
            Some(false)
        );
        assert!(jim.native_array_search_protocol(abi).is_none());
        let f5 = InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert_eq!(f5.native_array_search_member_present(b"startsearch"), None);
        assert!(f5.native_array_search_protocol(abi).is_none());
    }
}
