// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original generic array workers, independent of source compiler admission.

use tcl_dialect::TclVersion;
use tcl_syntax::native_string::NativeStringProtocol;

/// Selected argument and publication order for the original array-set worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeArraySetProtocol {
    /// C resolves the original array name before List/parity/member stores.
    COriginalName,
    /// Jim checks List/parity before the original variable read; an absent
    /// target adopts that original List without reading its key members.
    JimOriginalVariable,
}

impl crate::InvocationDialect {
    /// Pure recipe selection only: no loaded extension, table, value, cache,
    /// successful read or current-operation authority is produced here.
    #[must_use]
    pub fn native_array_set_protocol(self) -> Option<NativeArraySetProtocol> {
        match self.native_string_protocol()? {
            NativeStringProtocol::C(_) => Some(NativeArraySetProtocol::COriginalName),
            NativeStringProtocol::Jim084 => Some(NativeArraySetProtocol::JimOriginalVariable),
        }
    }

    /// The actual C9 `ArrayForNRCmd` List-copy/script-retain worker. A shared
    /// release boundary with `ArrayDefault` does not grant its unrelated purpose.
    #[must_use]
    pub fn native_array_for_protocol(self) -> Option<NativeStringProtocol> {
        match self.native_string_protocol()? {
            protocol @ NativeStringProtocol::C(TclVersion::V9_0 | TclVersion::V9_1) => {
                Some(protocol)
            }
            NativeStringProtocol::C(TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6)
            | NativeStringProtocol::Jim084 => None,
        }
    }
}
