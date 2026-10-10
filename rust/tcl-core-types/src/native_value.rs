// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Engine-neutral retained scalar and resident storage carriers.
//! A carrier describes payload and recorded origin; it grants no live object
//! authority, getter permission, or string updater capability.

use alloc::rc::Rc;

/// Physical resident-string storage, independent of its byte contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStringStorageIdentity {
    /// The selected native engine's canonical empty-string allocation.
    CanonicalEmpty,
    /// A distinct allocated string buffer, including allocated empty buffers.
    Allocated,
    /// Exact bytes are known but their native allocation identity is unavailable.
    Unknown,
}

/// C release belonging to an independently retained native cache descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCVersion {
    /// C Tcl 8.4.
    V8_4,
    /// C Tcl 8.5.
    V8_5,
    /// C Tcl 8.6.
    V8_6,
    /// C Tcl 9.0.
    V9_0,
    /// C Tcl 9.1.
    V9_1,
}

/// Radix of a full cached integer magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeIntegerRadix {
    /// Binary magnitude digits.
    Binary,
    /// Octal magnitude digits.
    Octal,
    /// Decimal magnitude digits.
    Decimal,
    /// Hexadecimal magnitude digits.
    Hexadecimal,
}

/// Full primary scalar cache. This transports storage, not getter permission.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeScalarCache {
    /// C Tcl8.4's native long cache, distinct from its wide-integer descriptor.
    Tcl84Long(i64),
    /// Exact signed integer cache, independently of a narrower getter return.
    Integer(i64),
    /// Full integer magnitude, without wrapping or a floating-point conversion.
    BigInteger {
        /// Whether the magnitude is negative.
        negative: bool,
        /// Base of the cleaned magnitude digits.
        radix: NativeIntegerRadix,
        /// Magnitude digits without a sign, prefix or separators.
        digits: Rc<str>,
    },
    /// Exact floating-point payload, including NaN payload and signed zero.
    Double(f64),
    /// A parsed NaN cache with its original sign and optional mantissa payload.
    Nan {
        /// The parsed sign.
        negative: bool,
        /// The independently retained mantissa payload.
        payload: Option<u64>,
    },
    /// C's non-integer word-Boolean cache with its original descriptor release.
    WordBoolean {
        /// Boolean interpretation.
        value: bool,
        /// Actual release of the original native descriptor.
        origin: NativeCVersion,
    },
    /// Jim's exact integer retained under a coerced-double representation.
    JimCoercedInteger(i64),
}
