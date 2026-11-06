// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
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
