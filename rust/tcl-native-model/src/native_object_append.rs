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

//! Actual-native and explicitly authored logical issuers for shared append recipes.
use tcl_dialect::TclVersion;
pub use tcl_syntax::native_object_append::{
    NativeObjectAppendAction, NativeObjectAppendUnavailable,
};
use tcl_syntax::native_string::NativeStringProtocol;

/// Explicit logical provider; it cannot attest execution by a native engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalAppendProvider {
    /// Authored Tcl8.4 core append behavior within the F5 logical simulator.
    Tcl84CoreSimulation,
}

/// A recipe issued by an actual engine or an explicitly authored logical provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeObjectAppendProtocol {
    recipe: tcl_syntax::native_object_append::NativeObjectAppendProtocol,
    logical: Option<LogicalAppendProvider>,
}

impl NativeObjectAppendProtocol {
    /// Shared pure shape recipe; this descriptor does not supply native attestation.
    #[must_use]
    pub const fn recipe(self) -> tcl_syntax::native_object_append::NativeObjectAppendProtocol {
        self.recipe
    }

    /// Explicit authored logical provider; absent for an actual native engine.
    #[must_use]
    pub const fn logical_provider(self) -> Option<LogicalAppendProvider> {
        self.logical
    }
}

impl crate::InvocationDialect {
    /// Select an actual append engine or the explicitly requested F5 core provider.
    /// Unknown and vendor native engines do not borrow C compatibility recipes.
    #[must_use]
    pub fn native_object_append_protocol(
        self,
        logical: Option<LogicalAppendProvider>,
    ) -> Option<NativeObjectAppendProtocol> {
        if let Some(string) = self.native_string_protocol() {
            return Some(NativeObjectAppendProtocol { recipe: tcl_syntax::native_object_append::NativeObjectAppendProtocol::for_string_protocol(string), logical: None });
        }
        match logical? {
            LogicalAppendProvider::Tcl84CoreSimulation => {
                self.authored_f5_tcl84_core()?;
                Some(NativeObjectAppendProtocol { recipe: tcl_syntax::native_object_append::NativeObjectAppendProtocol::for_string_protocol(NativeStringProtocol::C(TclVersion::V8_4)), logical })
            }
        }
    }
}

/// Actual-native C9 concatenation issuer, independent of append simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeObjectCatProtocol {
    recipe: tcl_syntax::native_object_append::NativeObjectCatProtocol,
}
impl NativeObjectCatProtocol {
    /// Pure selected recipe, without execution or compiler attestation.
    #[must_use]
    pub const fn recipe(self) -> tcl_syntax::native_object_append::NativeObjectCatProtocol {
        self.recipe
    }
}
impl crate::InvocationDialect {
    /// Issue C9 `TclStringCat` only from an actual audited native string engine.
    #[must_use]
    pub fn native_object_cat_protocol(self) -> Option<NativeObjectCatProtocol> {
        let recipe =
            tcl_syntax::native_object_append::NativeObjectCatProtocol::for_string_protocol(
                self.native_string_protocol()?,
            )?;
        Some(NativeObjectCatProtocol { recipe })
    }
}
