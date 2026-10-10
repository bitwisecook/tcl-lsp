// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native compiled-local comparison and source selection.
//!
//! Original local names remain counted byte keys. A compiler comparison may
//! select another retained slot; it never rewrites that slot's primary name.

use tcl_core_types::c_string_extent;
use tcl_dialect::{
    TclVersion,
    model::{BuildProfileId, DialectPoint, Family, Release},
};

/// The independently selected compiler implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompiledVariableRecipe {
    /// Canonical C Tcl's indexed procedure-local compiler.
    C(TclVersion),
    /// Jim's counted names, without a C indexed-local compiler.
    Jim084,
}

/// Native `LocalScalar` preparation from an original counted literal name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCompiledScalarName<'a> {
    /// Local base reserved even when an array operand later declines.
    pub declaration: Option<&'a [u8]>,
    /// Whether the original operand selects a scalar local.
    pub scalar: bool,
}

impl NativeCompiledVariableRecipe {
    /// Project `LocalScalar`'s declaration side effect without issuing a slot.
    /// C8.4 validates scalar names before allocation; C8.5+ can reserve an
    /// array base before declining. Jim has no indexed-local compiler.
    #[must_use]
    pub fn scalar_name(self, original: &[u8]) -> Option<NativeCompiledScalarName<'_>> {
        let Self::C(version) = self else {
            return None;
        };
        let original = if version == TclVersion::V8_4 {
            c_string_extent(original)
        } else {
            original
        };
        let array = original
            .last()
            .filter(|byte| **byte == b')')
            .and_then(|_| original.iter().position(|byte| *byte == b'('));
        let base = array.map_or(original, |open| &original[..open]);
        let qualified = base.windows(2).any(|bytes| bytes == b"::");
        Some(NativeCompiledScalarName {
            declaration: (!qualified && (array.is_none() || version > TclVersion::V8_4))
                .then_some(base),
            scalar: !qualified && array.is_none(),
        })
    }
    /// Compare source-local candidates exactly as the native compiler does.
    /// C checks full length before its bounded `CString` comparison. Jim compares
    /// every counted byte. Neither operation normalises a primary name.
    #[must_use]
    pub fn compiled_local_names_equal(self, existing: &[u8], requested: &[u8]) -> bool {
        if existing.len() != requested.len() {
            return false;
        }
        match self {
            NativeCompiledVariableRecipe::C(_) => {
                c_string_extent(existing) == c_string_extent(requested)
            }
            NativeCompiledVariableRecipe::Jim084 => existing == requested,
        }
    }

    /// Compare names during dynamic lookup of already compiled frame locals.
    /// C8.4/8.5 use `CString` equality without a length guard; C8.6+ and Jim use
    /// counted equality. Dynamic hash-table keys have their own input purpose
    /// and must not inherit this frame-local comparison.
    #[must_use]
    pub fn dynamic_local_names_equal(self, existing: &[u8], requested: &[u8]) -> bool {
        match self {
            NativeCompiledVariableRecipe::C(TclVersion::V8_4 | TclVersion::V8_5) => {
                c_string_extent(existing) == c_string_extent(requested)
            }
            NativeCompiledVariableRecipe::C(_) | NativeCompiledVariableRecipe::Jim084 => {
                existing == requested
            }
        }
    }
}

/// Origin of a compiler-name recipe, independent of name lookup authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompiledVariableAuthority {
    /// An actual audited native engine/build point.
    Native,
    /// A separately requested logical compiler simulation.
    AuthoredSimulation,
}

/// Purpose-selected compiler recipe; no frame or slot identity is supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeCompiledVariableProtocol {
    recipe: NativeCompiledVariableRecipe,
    authority: NativeCompiledVariableAuthority,
}

/// Actual source-local compilation environment, separate from runtime frame kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompiledVariableEnvironment {
    /// No indexed-local declaration or retained frame-layout receipt.
    None,
    /// Compile a procedure declaration and permit new local slots.
    DeclareProcedure,
    /// Compile a script against an existing frame local cache, without mutation.
    BorrowFrameSlots,
}

/// The native source compiler's local-variable selection rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompiledVariableLookup {
    /// Emit the original name and perform dynamic lookup at execution.
    DynamicName,
    /// Reuse an existing source local, otherwise emit the original name.
    ExistingLocalOnly,
    /// Reuse an existing source local or allocate a new counted local slot.
    CreateLocal,
}

impl NativeCompiledVariableProtocol {
    /// Select an actual audited compiler independently of compatibility claims.
    #[must_use]
    pub fn for_native_point(point: DialectPoint) -> Option<Self> {
        let recipe = match (point.family(), point.build()) {
            (Family::Tcl, BuildProfileId::Canonical) => {
                NativeCompiledVariableRecipe::C(point.tcl_version()?)
            }
            (Family::Jim, BuildProfileId::Canonical | BuildProfileId::JimFull)
                if point.release() == Release::JIM_0_84 =>
            {
                NativeCompiledVariableRecipe::Jim084
            }
            _ => return None,
        };
        Some(Self {
            recipe,
            authority: NativeCompiledVariableAuthority::Native,
        })
    }

    /// Select a pure authored C recipe without authenticating its physical host.
    #[must_use]
    pub const fn authored_tcl(version: TclVersion) -> Self {
        Self {
            recipe: NativeCompiledVariableRecipe::C(version),
            authority: NativeCompiledVariableAuthority::AuthoredSimulation,
        }
    }

    /// Retained compiler implementation, separate from its provider origin.
    #[must_use]
    pub const fn recipe(self) -> NativeCompiledVariableRecipe {
        self.recipe
    }

    /// Independent actual-native or logical provider origin.
    #[must_use]
    pub const fn authority(self) -> NativeCompiledVariableAuthority {
        self.authority
    }

    /// Whether this recipe has C's indexed procedure-local compiler.
    #[must_use]
    pub const fn has_indexed_locals(self) -> bool {
        matches!(self.recipe, NativeCompiledVariableRecipe::C(_))
    }

    /// Apply the selected compiler's local-name comparison without changing keys.
    #[must_use]
    pub fn compiled_local_names_equal(self, existing: &[u8], requested: &[u8]) -> bool {
        self.recipe.compiled_local_names_equal(existing, requested)
    }

    /// Whether a compiler match necessarily retains these exact requested
    /// bytes. C's count plus `CString` comparison has that property before any
    /// raw NUL; Jim's counted comparator has it for every name. This is a
    /// comparison-purpose fact, not a global name admission restriction.
    #[must_use]
    pub fn compiled_local_name_is_byte_unique(self, requested: &[u8]) -> bool {
        match self.recipe {
            NativeCompiledVariableRecipe::C(_) => !requested.contains(&0),
            NativeCompiledVariableRecipe::Jim084 => true,
        }
    }

    /// Apply the independent dynamic frame-local comparison for this engine.
    #[must_use]
    pub fn dynamic_local_names_equal(self, existing: &[u8], requested: &[u8]) -> bool {
        self.recipe.dynamic_local_names_equal(existing, requested)
    }

    /// Select the native source substitution path from its full counted name.
    /// `single_component` is the original native variable-token geometry,
    /// rather than a reconstructed name or a runtime array classification.
    /// The first qualifier or array opener decides the outcome in source order.
    /// Borrowed frame slots are read-only, and only C8.6+ has that compiler path.
    /// `ExistingLocalOnly` compares the complete name, including array-looking names.
    #[must_use]
    pub fn substitution_lookup(
        self,
        original: &[u8],
        single_component: bool,
        environment: NativeCompiledVariableEnvironment,
    ) -> NativeCompiledVariableLookup {
        if !self.supports_environment(environment) {
            return NativeCompiledVariableLookup::DynamicName;
        }
        for (index, byte) in original.iter().enumerate() {
            if *byte == b':' && original.get(index + 1) == Some(&b':') {
                return NativeCompiledVariableLookup::DynamicName;
            }
            if *byte == b'(' && single_component && original.last() == Some(&b')') {
                return NativeCompiledVariableLookup::ExistingLocalOnly;
            }
        }
        match environment {
            NativeCompiledVariableEnvironment::DeclareProcedure => {
                NativeCompiledVariableLookup::CreateLocal
            }
            NativeCompiledVariableEnvironment::BorrowFrameSlots => {
                NativeCompiledVariableLookup::ExistingLocalOnly
            }
            NativeCompiledVariableEnvironment::None => NativeCompiledVariableLookup::DynamicName,
        }
    }

    /// Select an already-resolved literal command operand's scalar or array-base
    /// name, as native `PushVarName` does. The caller supplies the complete scalar
    /// or extracted base, never an element or reconstructed combined spelling.
    /// Qualification scans every counted byte, including bytes after raw NUL.
    /// Procedure declarations may create a local; borrowed frame layouts only
    /// reuse an existing indexed slot. The primary key is never shortened.
    #[must_use]
    pub fn command_lookup(
        self,
        original_base: &[u8],
        environment: NativeCompiledVariableEnvironment,
    ) -> NativeCompiledVariableLookup {
        if !self.supports_environment(environment)
            || original_base.windows(2).any(|pair| pair == b"::")
        {
            return NativeCompiledVariableLookup::DynamicName;
        }
        match environment {
            NativeCompiledVariableEnvironment::DeclareProcedure => {
                NativeCompiledVariableLookup::CreateLocal
            }
            NativeCompiledVariableEnvironment::BorrowFrameSlots => {
                NativeCompiledVariableLookup::ExistingLocalOnly
            }
            NativeCompiledVariableEnvironment::None => NativeCompiledVariableLookup::DynamicName,
        }
    }

    /// Whether this native compiler can use the supplied local environment.
    /// Borrowing requires a separate retained layout receipt from the runtime.
    #[must_use]
    pub const fn supports_environment(
        self,
        environment: NativeCompiledVariableEnvironment,
    ) -> bool {
        match environment {
            NativeCompiledVariableEnvironment::None => false,
            NativeCompiledVariableEnvironment::DeclareProcedure => self.has_indexed_locals(),
            NativeCompiledVariableEnvironment::BorrowFrameSlots => matches!(
                self.recipe,
                NativeCompiledVariableRecipe::C(
                    TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
                )
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Native proof: naming.variable.compiled-versus-runtime-root-purpose
    // docs/design/analysis/name-resolution-proofs/variable.compiled-versus-runtime-root-purpose.md
    fn compiler_and_dynamic_frame_comparisons_keep_separate_extents() {
        let controls: &[(&[u8], &[u8], bool, bool)] = &[
            (b"k\0a", b"k\0b", true, true),
            (b"k\0a", b"k\0bb", false, true),
            (b"k\0a", b"k", false, true),
            (b"k\0a", b"q\0a", false, false),
            (b"k\xc0\x80a", b"k\xc0\x80b", false, false),
            (b"k\xffa", b"k\xffb", false, false),
            (b"k\xffa", b"k\xffa", true, true),
            (b"", b"", true, true),
        ];
        for version in TclVersion::ALL {
            let protocol = NativeCompiledVariableProtocol::for_native_point(
                DialectPoint::for_tcl_version(version),
            )
            .unwrap();
            for &(existing, requested, compiler, old_dynamic) in controls {
                assert_eq!(
                    protocol.compiled_local_names_equal(existing, requested),
                    compiler,
                    "{version:?}: {existing:?}, {requested:?}"
                );
                assert_eq!(
                    protocol.dynamic_local_names_equal(existing, requested),
                    if version <= TclVersion::V8_5 {
                        old_dynamic
                    } else {
                        existing == requested
                    },
                    "{version:?}: {existing:?}, {requested:?}"
                );
            }
        }
    }

    #[test]
    fn source_selection_scans_counted_bytes_in_original_token_order() {
        use NativeCompiledVariableLookup::{CreateLocal, DynamicName, ExistingLocalOnly};
        let controls: &[(&[u8], bool, NativeCompiledVariableLookup)] = &[
            (b"k\0a", true, CreateLocal),
            (b"k\0z::x", true, DynamicName),
            (b"n::k\0x", true, DynamicName),
            (b"a(k)", true, ExistingLocalOnly),
            (b"a(k)", false, CreateLocal),
            (b"a(k::q)", true, ExistingLocalOnly),
            (b"n::a(k)", true, DynamicName),
            (b"k\0z(q)", true, ExistingLocalOnly),
            (b"a(k", true, CreateLocal),
            (b"", true, CreateLocal),
        ];
        for version in TclVersion::ALL {
            let protocol = NativeCompiledVariableProtocol::for_native_point(
                DialectPoint::for_tcl_version(version),
            )
            .unwrap();
            for &(name, single, expected) in controls {
                assert_eq!(
                    protocol.substitution_lookup(
                        name,
                        single,
                        NativeCompiledVariableEnvironment::DeclareProcedure
                    ),
                    expected
                );
                assert_eq!(
                    protocol.substitution_lookup(
                        name,
                        single,
                        NativeCompiledVariableEnvironment::None
                    ),
                    DynamicName
                );
            }
        }
    }

    #[test]
    fn literal_command_bases_do_not_use_substitution_array_shortcut() {
        use NativeCompiledVariableEnvironment::{BorrowFrameSlots, DeclareProcedure};
        use NativeCompiledVariableLookup::{CreateLocal, DynamicName, ExistingLocalOnly};
        let modern = NativeCompiledVariableProtocol::authored_tcl(TclVersion::V9_0);
        assert_eq!(modern.command_lookup(b"a", DeclareProcedure), CreateLocal);
        assert_eq!(
            modern.command_lookup(b"a", BorrowFrameSlots),
            ExistingLocalOnly
        );
        assert_eq!(
            modern.command_lookup(b"k\0z::x", DeclareProcedure),
            DynamicName
        );
        assert_eq!(
            modern.command_lookup(b"a(k)", DeclareProcedure),
            CreateLocal
        );
        assert_eq!(
            modern.substitution_lookup(b"a(k)", true, DeclareProcedure),
            ExistingLocalOnly
        );
        let old = NativeCompiledVariableProtocol::authored_tcl(TclVersion::V8_4);
        assert_eq!(old.command_lookup(b"a", BorrowFrameSlots), DynamicName);
    }

    #[test]
    fn borrowed_frame_slots_are_read_only_and_release_specific() {
        use NativeCompiledVariableEnvironment::{BorrowFrameSlots, DeclareProcedure, None};
        use NativeCompiledVariableLookup::{CreateLocal, DynamicName, ExistingLocalOnly};
        for version in TclVersion::ALL {
            let protocol = NativeCompiledVariableProtocol::for_native_point(
                DialectPoint::for_tcl_version(version),
            )
            .unwrap();
            let supported = version >= TclVersion::V8_6;
            assert_eq!(protocol.supports_environment(BorrowFrameSlots), supported);
            assert_eq!(
                protocol.substitution_lookup(b"k\0a", true, BorrowFrameSlots),
                if supported {
                    ExistingLocalOnly
                } else {
                    DynamicName
                }
            );
            assert_eq!(
                protocol.substitution_lookup(b"k\0a", true, DeclareProcedure),
                CreateLocal
            );
            assert_eq!(
                protocol.substitution_lookup(b"n::k", true, BorrowFrameSlots),
                DynamicName
            );
            assert_eq!(protocol.substitution_lookup(b"k", true, None), DynamicName);
        }
        let logical = NativeCompiledVariableProtocol::authored_tcl(TclVersion::V8_4);
        assert!(!logical.supports_environment(BorrowFrameSlots));
    }

    #[test]
    fn jim_counted_names_do_not_attest_a_c_local_compiler() {
        let jim = NativeCompiledVariableProtocol::for_native_point(DialectPoint::canonical(
            Release::JIM_0_84,
        ))
        .unwrap();
        assert!(!jim.has_indexed_locals());
        assert!(!jim.compiled_local_names_equal(b"k\0a", b"k\0b"));
        assert!(!jim.dynamic_local_names_equal(b"k\0a", b"k"));
        assert_eq!(
            jim.substitution_lookup(
                b"a",
                true,
                NativeCompiledVariableEnvironment::DeclareProcedure
            ),
            NativeCompiledVariableLookup::DynamicName
        );
    }

    #[test]
    fn compiler_authority_is_independent_of_name_and_compatibility_policies() {
        let vendor = DialectPoint::canonical(Release::F5_IRULES_TMM);
        assert_eq!(
            NativeCompiledVariableProtocol::for_native_point(vendor),
            None
        );
        let authored = NativeCompiledVariableProtocol::authored_tcl(TclVersion::V8_4);
        assert_eq!(
            authored.authority(),
            NativeCompiledVariableAuthority::AuthoredSimulation
        );
        assert_eq!(
            authored.recipe(),
            NativeCompiledVariableRecipe::C(TclVersion::V8_4)
        );
        let native = NativeCompiledVariableProtocol::for_native_point(
            DialectPoint::for_tcl_version(TclVersion::V9_0),
        )
        .unwrap();
        assert_eq!(native.authority(), NativeCompiledVariableAuthority::Native);
    }
}
