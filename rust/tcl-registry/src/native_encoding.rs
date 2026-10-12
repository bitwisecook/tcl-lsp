// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual encoding command and source defaults, independent of Name receipts.

use crate::InvocationDialect;
use tcl_cmd_core::encoding::{NativeEncodingNameProtocol, NativeSourceFileProtocol};
use tcl_dialect::{TclVersion, model::Family};

impl InvocationDialect {
    /// Actual C encoding lookup presentation; Jim has no installed encoding command.
    #[must_use]
    pub fn native_encoding_name_protocol(self) -> Option<NativeEncodingNameProtocol> {
        (self.family()? == Family::Tcl)
            .then_some(NativeEncodingNameProtocol::for_c(self.tcl_version?))
    }

    /// Selected source channel default, without a file or conversion grant.
    #[must_use]
    pub fn native_source_file_protocol(self) -> Option<NativeSourceFileProtocol> {
        self.source_file_grammar()?;
        match self.family()? {
            Family::Tcl => match self.tcl_version? {
                version @ (TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6) => {
                    Some(NativeSourceFileProtocol::CSystem(version))
                }
                version @ (TclVersion::V9_0 | TclVersion::V9_1) => {
                    Some(NativeSourceFileProtocol::CUtf8(version))
                }
            },
            Family::Jim => Some(NativeSourceFileProtocol::Jim),
            _ => None,
        }
    }
}
