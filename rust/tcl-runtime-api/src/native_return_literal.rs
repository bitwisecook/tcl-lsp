// SPDX-License-Identifier: AGPL-3.0-or-later
//! Construction data for original private compiler return-option objects.

use tcl_syntax::native_string::NativeStringProtocol;

/// `TclWordKnownAtCompileTime`'s original counted append operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeKnownWordLiteral {
    /// Decoded TEXT/BS pieces, retaining their original append boundaries.
    pub pieces: Vec<Vec<u8>>,
    /// Non-SIMPLE_WORD builds a temporary object before `AppendObjToObj`.
    pub composite: bool,
}

/// Manufacture one fresh private options header, independent of registration.
/// The backend checks the actual issuer and independently merges these words.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeReturnOptionsLiteral {
    /// Actual C object construction/merge issuer.
    pub protocol: NativeStringProtocol,
    /// Original known option words in argv order; result is not included.
    pub words: Vec<NativeKnownWordLiteral>,
    /// Independently projected native control out parameters.
    pub code: i32,
    /// Independently projected native return-level out parameter.
    pub level: i32,
    /// Remaining member count checked against the manufactured header.
    pub size: usize,
}
