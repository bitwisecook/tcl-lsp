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

//! Transported original frame failure data, independent of command selection.

use crate::scalar_getter::NativeScalarGetterError;

/// A reached guest failure, independently of host capability refusals.
#[derive(Debug, Clone)]
pub enum NativeFrameLevelFailure {
    /// Native frame lookup failed; reporting uses the actual `CString` prefix.
    BadLevel {
        /// Original reported selector, or the omitted-level default `1`.
        name: Vec<u8>,
        /// Native C8.6+ publishes the structured lookup code.
        lookup_code: bool,
        /// Selected original append/format result producer, separate from text.
        string_result: Option<crate::native_string::NativeStringProtocol>,
    },
    /// Legacy digit-first lookup propagates a temporary Int getter failure.
    Primitive(Box<NativeScalarGetterError>),
}
