// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
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
