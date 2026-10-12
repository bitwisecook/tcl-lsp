// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! ABI issuance for supported in-process native-equivalent backends.
//!
//! These backends execute the selected hash recipe using the C ABI of their
//! process. This issuer describes that ABI; it does not select a Tcl release,
//! grant native interpreter authority, or recover a foreign table's seed.

use tcl_core_types::{NativeHashAbi, NativeHashBytePromotion, NativeHashWordWidth};

/// Independently issue the supported backend's actual host C ABI layout.
///
/// `jim_seed` must be the seed chosen by the owner of the actual Jim table.
/// Foreign/oracle tables supply their own measured receipt instead. Unsupported
/// integer widths withdraw the recipe rather than substituting host defaults.
#[must_use]
pub fn supported_backend_hash_abi(jim_seed: Option<u32>) -> Option<NativeHashAbi> {
    Some(NativeHashAbi {
        plain_char: if i64::from(std::ffi::c_char::MIN) < 0 {
            NativeHashBytePromotion::Signed
        } else {
            NativeHashBytePromotion::Unsigned
        },
        unsigned_int: word_width(std::mem::size_of::<std::ffi::c_uint>())?,
        size_t: word_width(std::mem::size_of::<usize>())?,
        jim_seed,
    })
}

fn word_width(bytes: usize) -> Option<NativeHashWordWidth> {
    match bytes {
        4 => Some(NativeHashWordWidth::Bits32),
        8 => Some(NativeHashWordWidth::Bits64),
        _ => None,
    }
}
/// Issue actual C strtoul/int layouts for this supported backend independently
/// of the Tcl release. Unsupported data models withdraw the recipe.
#[must_use]
pub fn supported_backend_array_search_abi() -> Option<tcl_core_types::NativeArraySearchAbi> {
    let unsigned_long = match core::mem::size_of::<std::ffi::c_ulong>() {
        4 => NativeHashWordWidth::Bits32,
        8 => NativeHashWordWidth::Bits64,
        _ => return None,
    };
    Some(tcl_core_types::NativeArraySearchAbi {
        unsigned_long,
        int_bits: u8::try_from(core::mem::size_of::<std::ffi::c_int>() * 8).ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_native_layout_withdraws_without_unsigned_or_width_default() {
        assert_eq!(word_width(4), Some(NativeHashWordWidth::Bits32));
        assert_eq!(word_width(8), Some(NativeHashWordWidth::Bits64));
        for bytes in [0, 1, 2, 3, 16] {
            assert_eq!(word_width(bytes), None);
        }
        let receipt = supported_backend_hash_abi(None).unwrap();
        assert_eq!(receipt.jim_seed, None);
    }
}
