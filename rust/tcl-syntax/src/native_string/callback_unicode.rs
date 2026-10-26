// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unicode-required host projection of already obtained counted String bytes.

use super::NativeStringProtocol;
use crate::native_tcl_utf::NativeTclUtf;
use tcl_dialect::TclVersion;

impl NativeStringProtocol {
    /// Project the selected public native units into a Rust Unicode string.
    ///
    /// This pure projection neither calls a getter nor installs Unicode storage.
    /// Literal NUL participates. C8 pairs its 16-bit surrogate units, C9 requires
    /// scalar units, and Jim retains its independent decoder's byte rules.
    /// An unrepresentable unit refuses rather than replacing it with U+FFFD.
    #[must_use]
    pub fn counted_unicode_text(self, bytes: &[u8]) -> Option<String> {
        match self {
            Self::C(version) => {
                let units = NativeTclUtf::for_version(version).decode_units(bytes);
                if version < TclVersion::V9_0 {
                    let units: Option<Vec<u16>> = units
                        .into_iter()
                        .map(|unit| u16::try_from(unit).ok())
                        .collect();
                    String::from_utf16(&units?).ok()
                } else {
                    units.into_iter().map(char::from_u32).collect()
                }
            }
            Self::Jim084 => {
                let mut result = String::new();
                let mut offset = 0;
                while let Some((unit, width)) = crate::raw_string::jim084_decode(&bytes[offset..]) {
                    result.push(char::from_u32(unit)?);
                    offset += width;
                }
                Some(result)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_unicode_keeps_counted_zero_and_actual_selected_units() {

        // Pure returned-byte projection, not a native getter or Unicode cache receipt.
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeStringProtocol::C(version);
            assert_eq!(protocol.counted_unicode_text(b"A\0B"), Some("A\0B".into()));
            assert_eq!(
                protocol.counted_unicode_text(b"A\xc0\x80B"),
                Some("A\0B".into())
            );
            assert_eq!(protocol.counted_unicode_text(b"\xff"), Some("ÿ".into()));
            assert_eq!(protocol.counted_unicode_text(b"\xed\xa0\x80"), None);
        }
        assert_eq!(
            NativeStringProtocol::Jim084.counted_unicode_text(b"A\0B"),
            Some("A\0B".into())
        );
        assert_eq!(
            NativeStringProtocol::Jim084.counted_unicode_text(b"\xc0\x80"),
            Some("À\u{80}".into())
        );
        assert_eq!(
            NativeStringProtocol::Jim084.counted_unicode_text(b"\xed\xa0\x80"),
            None
        );
    }

    #[test]
    fn callback_unicode_pairs_c8_units_without_forgiving_c9_surrogates() {

        // Fixed selected-unit inputs; no runtime or platform conversion grant.
        let cesu = b"\xed\xa0\xbd\xed\xb9\x82";
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            assert_eq!(
                NativeStringProtocol::C(version).counted_unicode_text(cesu),
                Some("🙂".into())
            );
        }
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let protocol = NativeStringProtocol::C(version);
            assert_eq!(protocol.counted_unicode_text(cesu), None);
            assert_eq!(
                protocol.counted_unicode_text("🙂".as_bytes()),
                Some("🙂".into())
            );
        }
        assert_eq!(
            NativeStringProtocol::C(TclVersion::V8_6).counted_unicode_text("🙂".as_bytes()),
            Some("🙂".into())
        );
        assert_eq!(
            NativeStringProtocol::Jim084.counted_unicode_text("🙂".as_bytes()),
            Some("🙂".into())
        );
        assert_eq!(
            NativeStringProtocol::Jim084.counted_unicode_text(cesu),
            None
        );
    }
}
