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

//! Original ordinary procedure-body creation, separate from compilation and formal grammar.

use tcl_syntax::native_string::NativeStringProtocol;

/// The physical definition owner selected before acquiring any temporary body handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeProcedureBodyAction {
    /// The definition acquires a reference to the same original object.
    RetainOriginal,
    /// Materialize the original through its actual string owner, then make a counted string-only copy.
    CopyCountedString,
}

/// Selected actual ordinary body creation. This does not admit opaque precompiled `ProcBody` objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeProcedureBodyCreationProtocol {
    strings: NativeStringProtocol,
}
impl NativeProcedureBodyCreationProtocol {
    pub(crate) const fn for_string_recipe(strings: NativeStringProtocol) -> Self {
        Self { strings }
    }

    /// Exact native materialization recipe, independent of source parser settings.
    #[must_use]
    pub const fn strings(self) -> NativeStringProtocol {
        self.strings
    }

    /// Choose from the original native sharing observation made before temporary ownership.
    #[must_use]
    pub const fn action(self, shared: bool) -> NativeProcedureBodyAction {
        if shared && matches!(self.strings, NativeStringProtocol::C(_)) {
            NativeProcedureBodyAction::CopyCountedString
        } else {
            NativeProcedureBodyAction::RetainOriginal
        }
    }
}
impl crate::InvocationDialect {
    /// Issue ordinary procedure-body creation only for an actual supported native core.
    #[must_use]
    pub fn native_procedure_body_creation_protocol(
        self,
    ) -> Option<NativeProcedureBodyCreationProtocol> {
        Some(NativeProcedureBodyCreationProtocol::for_string_recipe(
            self.native_string_protocol()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;
    #[test]
    fn body_ownership_is_separate_from_compilation_and_source_grammar() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let recipe = crate::InvocationDialect::for_version(version)
                .native_procedure_body_creation_protocol()
                .unwrap();
            assert_eq!(
                recipe.action(false),
                NativeProcedureBodyAction::RetainOriginal
            );
            assert_eq!(
                recipe.action(true),
                NativeProcedureBodyAction::CopyCountedString
            );
            assert_eq!(recipe.strings(), NativeStringProtocol::C(version));
        }
        let jim = crate::InvocationDialect::of_profile(
            crate::model::resolve_environment("jim").unit_profile(),
        )
        .native_procedure_body_creation_protocol()
        .unwrap();
        assert_eq!(jim.action(true), NativeProcedureBodyAction::RetainOriginal);
    }
}
