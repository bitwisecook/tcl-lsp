// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native string materialisation from independently known object storage.
//!
//! Resident bytes are authoritative. Materialising a pure byte array is a
//! different operation from decoding written escapes or projecting a name.
//! These recipes grant no object, compiler, engine-effect or cache proof.

use std::borrow::Cow;

mod name_display;
pub use name_display::resident_name_label;

use tcl_dialect::{
    EscapeSyntax, TclVersion,
    model::{BuildProfileId, DialectPoint, Family, Release},
};

/// Selected native string recipe, independently of numeric engine support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeStringProtocol {
    /// Independently selected canonical C Tcl release.
    C(TclVersion),
    /// Audited Jim 0.84 implementation.
    Jim084,
}

/// Original-header duplication selected before primary-specific cache hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeObjectHeaderDuplicateAction {
    /// Duplicate the reached primary representation using its selected owner.
    TypeSpecific,
    /// Jim duplicates resident zero-length objects to canonical empty NULL storage.
    CanonicalEmptyString,
}

/// Independently retained identity of an existing native string allocation.
/// Equal bytes do not establish the canonical empty allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeStringStorageIdentity {
    /// The actual native canonical empty storage address or constructor receipt.
    CanonicalEmpty,
    /// An independently known ordinary allocation, including allocated empties.
    Allocated,
    /// Imported bytes retain no authenticated allocation identity.
    Unknown,
}

impl NativeStringProtocol {
    /// Public C `Tcl_Eval` mirrors its result through the legacy `CString`
    /// result on C8. A subsequent object-result getter imports that extent.
    /// This pure reporting projection does not alter counted object results
    /// from `Tcl_EvalEx`/`Tcl_EvalObjv` or their stored variable keys.
    #[must_use]
    pub fn legacy_eval_result_input(self, original: &[u8]) -> Option<&[u8]> {
        match self {
            Self::C(TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6) => {
                Some(tcl_core_types::c_string_extent(original))
            }
            Self::C(TclVersion::V9_0 | TclVersion::V9_1) => Some(original),
            Self::Jim084 => None,
        }
    }
    /// `Tcl_NewUnicodeObj` retains an empty Unicode array on C86+, while
    /// C84/85 retain only the zero character count until a getter is reached.
    #[must_use]
    pub const fn unicode_constructor_has_unicode(self, length: usize) -> bool {
        length != 0
            || matches!(
                self,
                Self::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
            )
    }

    /// Whether duplicating a String primary preserves its retained character
    /// cache. C86+ discards an unknown-count primary; C84/85 and Jim copy it.
    #[must_use]
    pub const fn string_primary_survives_duplicate(self, count: Option<usize>) -> bool {
        matches!(
            self,
            Self::C(TclVersion::V8_4 | TclVersion::V8_5) | Self::Jim084
        ) || count.is_some()
    }
    /// Pure C recipe selection; does not authenticate a live native object.
    #[must_use]
    pub const fn for_tcl_version(version: TclVersion) -> Self {
        Self::C(version)
    }

    /// Select the actual engine/build point. Vendor compatibility is insufficient.
    #[must_use]
    pub fn for_point(point: DialectPoint) -> Option<Self> {
        match (point.family(), point.build()) {
            (Family::Tcl, BuildProfileId::Canonical) => point.tcl_version().map(Self::C),
            (Family::Jim, BuildProfileId::Canonical | BuildProfileId::JimFull)
                if point.release() == Release::JIM_0_84 =>
            {
                Some(Self::Jim084)
            }
            _ => None,
        }
    }

    /// Actual C release when this is a C recipe.
    #[must_use]
    pub const fn tcl_version(self) -> Option<TclVersion> {
        match self {
            Self::C(version) => Some(version),
            Self::Jim084 => None,
        }
    }

    /// Whether this is the audited Jim recipe.
    #[must_use]
    pub const fn is_jim084(self) -> bool {
        matches!(self, Self::Jim084)
    }

    /// Escape grammar belonging to this independently selected recipe.
    #[must_use]
    pub const fn escape_syntax(self) -> EscapeSyntax {
        match self {
            Self::C(TclVersion::V8_4 | TclVersion::V8_5) => EscapeSyntax::Tcl84,
            Self::C(TclVersion::V8_6) => EscapeSyntax::Tcl86,
            Self::C(TclVersion::V9_0 | TclVersion::V9_1) => EscapeSyntax::Tcl90,
            Self::Jim084 => EscapeSyntax::Jim,
        }
    }

    /// Jim's resident-empty shortcut precedes every primary-specific duplicate.
    /// Absent string storage and counted NUL bytes do not establish emptiness.
    #[must_use]
    pub const fn object_header_duplicate_action(
        self,
        resident_length: Option<usize>,
    ) -> NativeObjectHeaderDuplicateAction {
        if matches!(self, Self::Jim084) && matches!(resident_length, Some(0)) {
            NativeObjectHeaderDuplicateAction::CanonicalEmptyString
        } else {
            NativeObjectHeaderDuplicateAction::TypeSpecific
        }
    }

    /// Canonical flag retained when a shared native List backing is copied.
    /// C 8.6 carries the prior flag; the other native copies initialise it.
    #[must_use]
    pub const fn copied_list_canonical(self, previous: bool) -> bool {
        matches!(self, Self::C(TclVersion::V8_6)) && previous
    }

    /// Canonical flag after a reached native List string updater.
    /// C 9 leaves shared backing flags unchanged; C 8.5 and 8.6 set them.
    #[must_use]
    pub const fn updated_list_canonical(self, previous: bool, backing_shared: bool) -> bool {
        match self {
            Self::C(TclVersion::V8_5 | TclVersion::V8_6) => true,
            Self::C(TclVersion::V9_0 | TclVersion::V9_1) => !backing_shared || previous,
            Self::C(TclVersion::V8_4) | Self::Jim084 => false,
        }
    }

    /// Storage installed by the native List and Dictionary string updaters.
    /// This receipt describes a reached updater, rather than imported bytes.
    #[must_use]
    pub const fn compound_updater_storage(self) -> NativeStringStorageIdentity {
        match self {
            Self::C(TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1) => {
                NativeStringStorageIdentity::CanonicalEmpty
            }
            Self::C(TclVersion::V8_4) | Self::Jim084 => NativeStringStorageIdentity::Allocated,
        }
    }

    /// Materialise actual storage. The adapter installs owned output lazily on
    /// the original object, retaining byte-array backing and its selected recipe.
    ///
    /// # Errors
    /// Returns `JimByteArray` for pure byte-array storage under Jim; resident strings never fail.
    pub fn materialize(
        self,
        input: NativeStringInput<'_>,
    ) -> Result<Cow<'_, [u8]>, NativeStringUnavailable> {
        materialize_native_string(self, input)
    }
}

/// Physical source of a native object's string, independent of its other reps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStringInput<'a> {
    /// Existing length-delimited string, including a byte array's resident rep.
    ResidentString(&'a [u8]),
    /// C byte-array bytes with no resident string representation.
    PureByteArray(&'a [u8]),
}

/// Unsupported host recipe; this is never a native Tcl guest error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStringUnavailable {
    /// No independently authenticated native recipe or explicit logical provider.
    ProtocolUnavailable,
    /// No equivalent pure Tcl byte-array representation is audited for Jim.
    JimByteArray,
    /// The original object's native descriptor has no string updater.
    StringUpdater,
}

impl std::fmt::Display for NativeStringUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::ProtocolUnavailable => "native string protocol is unavailable",
            Self::JimByteArray => "Jim byte-array materialisation is unavailable",
            Self::StringUpdater => "native object string updater is unavailable",
        })
    }
}

impl std::error::Error for NativeStringUnavailable {}

/// Materialise native string bytes without Unicode replacement or name parsing.
/// Existing strings always borrow their complete bytes. Pure C byte arrays map
/// each byte to U+00XX, with NUL encoded as C0 80 in every supported C release.
///
/// # Errors
/// Returns `JimByteArray` for pure byte-array storage under Jim; resident strings never fail.
pub fn materialize_native_string(
    protocol: NativeStringProtocol,
    input: NativeStringInput<'_>,
) -> Result<Cow<'_, [u8]>, NativeStringUnavailable> {
    let original = match input {
        NativeStringInput::ResidentString(bytes) => return Ok(Cow::Borrowed(bytes)),
        NativeStringInput::PureByteArray(bytes) => bytes,
    };
    let NativeStringProtocol::C(version) = protocol else {
        return Err(NativeStringUnavailable::JimByteArray);
    };
    let units = crate::native_tcl_utf::NativeTclUtf::for_version(version);
    let mut output = Vec::with_capacity(original.len());
    for &byte in original {
        units
            .encode_unit(u32::from(byte), &mut output)
            .expect("byte unit is valid in every C recipe");
    }
    Ok(Cow::Owned(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_eval_result_extent_keeps_encoded_zero_separate_from_counted_zero() {
        // Native proof: naming.interpreter.legacy-eval-object-result-boundary
        // docs/design/analysis/name-resolution-proofs/legacy-eval-object-result-boundary.md
        for version in TclVersion::ALL {
            let protocol = NativeStringProtocol::C(version);
            let original = b"k\0tail";
            assert_eq!(
                protocol.legacy_eval_result_input(original).unwrap(),
                if version <= TclVersion::V8_6 {
                    b"k".as_slice()
                } else {
                    original
                }
            );
            assert_eq!(
                protocol.legacy_eval_result_input(b"k\xc0\x80tail").unwrap(),
                b"k\xc0\x80tail"
            );
        }
        assert_eq!(
            NativeStringProtocol::Jim084.legacy_eval_result_input(b"k\0tail"),
            None
        );
    }

    #[test]
    fn header_duplicate_empty_shortcut_requires_jim_and_resident_zero_length() {
        use NativeObjectHeaderDuplicateAction::{CanonicalEmptyString, TypeSpecific};
        assert_eq!(
            NativeStringProtocol::Jim084.object_header_duplicate_action(Some(0)),
            CanonicalEmptyString
        );
        for length in [None, Some(1), Some(3)] {
            assert_eq!(
                NativeStringProtocol::Jim084.object_header_duplicate_action(length),
                TypeSpecific
            );
        }
        for version in TclVersion::ALL {
            assert_eq!(
                NativeStringProtocol::C(version).object_header_duplicate_action(Some(0)),
                TypeSpecific
            );
        }
    }

    #[test]
    fn empty_compound_updater_storage_matches_every_actual_native_engine() {
        let fixtures = [
            (
                NativeStringProtocol::C(TclVersion::V8_4),
                include_str!("../testdata/native_empty_compound_updaters/8.4.20.tsv"),
            ),
            (
                NativeStringProtocol::C(TclVersion::V8_5),
                include_str!("../testdata/native_empty_compound_updaters/8.5.19.tsv"),
            ),
            (
                NativeStringProtocol::C(TclVersion::V8_6),
                include_str!("../testdata/native_empty_compound_updaters/8.6.18.tsv"),
            ),
            (
                NativeStringProtocol::C(TclVersion::V9_0),
                include_str!("../testdata/native_empty_compound_updaters/9.0.4.tsv"),
            ),
            (
                NativeStringProtocol::C(TclVersion::V9_1),
                include_str!("../testdata/native_empty_compound_updaters/9.1.0.tsv"),
            ),
            (
                NativeStringProtocol::Jim084,
                include_str!("../testdata/native_empty_compound_updaters/jim.tsv"),
            ),
        ];
        let mut rows = 0;
        let mut actual = 0;
        for (protocol, fixture) in fixtures {
            for row in fixture.lines() {
                rows += 1;
                let fields: Vec<_> = row.split('\t').collect();
                if fields[1] == "unavailable" {
                    assert_eq!(protocol, NativeStringProtocol::C(TclVersion::V8_4));
                    continue;
                }
                actual += 1;
                assert_eq!(fields[1], fields[3]);
                assert_eq!(fields[4], "1");
                assert_eq!(fields[5], "0");
                if protocol.is_jim084() {
                    assert_eq!(
                        protocol.compound_updater_storage(),
                        NativeStringStorageIdentity::Allocated
                    );
                } else {
                    assert_eq!(
                        protocol.compound_updater_storage()
                            == NativeStringStorageIdentity::CanonicalEmpty,
                        fields[6] == "1"
                    );
                }
            }
        }
        assert_eq!((rows, actual), (12, 11));
    }

    #[test]
    fn resident_bytes_do_not_inherit_byte_array_materialisation() {
        let original = [0, 0xff, 0xc0, 0x80, 0xed, 0xa0, 0x80];
        for protocol in [
            NativeStringProtocol::C(TclVersion::V8_4),
            NativeStringProtocol::C(TclVersion::V8_5),
            NativeStringProtocol::C(TclVersion::V8_6),
            NativeStringProtocol::C(TclVersion::V9_0),
            NativeStringProtocol::C(TclVersion::V9_1),
            NativeStringProtocol::Jim084,
        ] {
            let output = protocol
                .materialize(NativeStringInput::ResidentString(&original))
                .unwrap();
            assert!(matches!(output, Cow::Borrowed(_)));
            assert_eq!(output.as_ref(), original);
        }
    }

    #[test]
    fn pure_c_byte_array_native_units_preserve_distinguishing_controls() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeStringProtocol::C(version);
            assert_eq!(
                protocol
                    .materialize(NativeStringInput::PureByteArray(&[0, 0xff]))
                    .unwrap()
                    .as_ref(),
                &[0xc0, 0x80, 0xc3, 0xbf]
            );
            assert_ne!(
                protocol
                    .materialize(NativeStringInput::PureByteArray(&[0xc0, 0x80]))
                    .unwrap()
                    .as_ref(),
                &[0xc0, 0x80]
            );
        }
        assert_eq!(
            NativeStringProtocol::Jim084.materialize(NativeStringInput::PureByteArray(&[])),
            Err(NativeStringUnavailable::JimByteArray)
        );
    }
}
