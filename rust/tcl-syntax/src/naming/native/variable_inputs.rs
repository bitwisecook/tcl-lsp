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

//! Variable names by purpose: root, alias-local, combined and separate receivers, trace inputs, formal parameters and the `namespace upvar` local.

use super::super::is_qualified;
use super::{
    NameProjectionUnavailable, NativeNameProjection, NativeNameProtocol, NativeNamePurpose,
    NativeNameQualification, NativeVariableInputForm, NativeVariableProjection, projection,
    qualification,
};
use std::borrow::Cow;
use tcl_core_types::c_string_extent;
use tcl_dialect::TclVersion;

impl NativeNameProtocol {
    /// Root-variable receiver input. Qualification is tested before raw NUL in
    /// C; a later `::` must not turn a full unqualified scalar into a short key.
    #[must_use]
    pub fn variable_root_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        let prefix = c_string_extent(original);
        let selected = match self {
            Self::C(version) if version == TclVersion::V8_4 || is_qualified(prefix) => prefix,
            _ => original,
        };
        let mut result = projection(
            self,
            NativeNamePurpose::VariableRoot,
            original,
            Cow::Borrowed(selected),
            None,
        );
        result.qualification = match self {
            Self::C(_) => qualification(prefix),
            Self::Jim084 if original.starts_with(b"::") => NativeNameQualification::Absolute,
            Self::Jim084 => NativeNameQualification::Unqualified,
        };
        result
    }

    /// Scalar receiver used by `TclLookupSimpleVar` for an alias's local side.
    /// Its extent follows the simple-name API; it does not authorise a parsed
    /// or indexed cache on the original operand.
    #[must_use]
    pub fn variable_alias_local_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        let mut input = self.variable_root_input(original);
        input.purpose = NativeNamePurpose::VariableAliasLocal;
        input
    }

    /// Combined array spelling uses a native opening-parenthesis scanner and
    /// its own element extent. It does not strip a source `$` or `${...}`.
    #[must_use]
    pub fn combined_variable_input(self, original: &[u8]) -> NativeVariableProjection<'_> {
        let opening_input = match self {
            Self::C(version) if version < TclVersion::V9_0 => original,
            _ => c_string_extent(original),
        };
        let parts = if original.last() == Some(&b')') {
            opening_input
                .iter()
                .position(|byte| *byte == b'(')
                .map(|opening| {
                    (
                        &original[..opening],
                        &original[opening + 1..original.len() - 1],
                    )
                })
        } else {
            None
        };
        let (root, element) = match parts {
            Some((root, element)) => (
                self.variable_root_input(root),
                Some(self.element_input(element, NativeNamePurpose::ArrayElementCombined)),
            ),
            None => (self.variable_root_input(original), None),
        };
        NativeVariableProjection {
            original: NativeVariableInputForm::Combined(original),
            root,
            element,
        }
    }

    /// Separate native object operands preserve their input form. An already
    /// parsed array root plus a second element is a runtime guest error, not a
    /// reason for this pure selector to invent a different receiver.
    #[must_use]
    pub fn separate_variable_input<'a>(
        self,
        root: &'a [u8],
        element: Option<&'a [u8]>,
    ) -> NativeVariableProjection<'a> {
        NativeVariableProjection {
            original: NativeVariableInputForm::Separate { root, element },
            root: self.variable_root_input(root),
            element: element
                .map(|bytes| self.element_input(bytes, NativeNamePurpose::ArrayElementSeparated)),
        }
    }

    fn element_input(
        self,
        original: &[u8],
        purpose: NativeNamePurpose,
    ) -> NativeNameProjection<'_> {
        let truncate = match (self, purpose) {
            (Self::C(version), NativeNamePurpose::ArrayElementCombined) => {
                version < TclVersion::V9_0
            }
            (Self::C(TclVersion::V8_4), NativeNamePurpose::ArrayElementSeparated) => true,
            _ => false,
        };
        let selected = if truncate {
            c_string_extent(original)
        } else {
            original
        };
        let mut result = projection(self, purpose, original, Cow::Borrowed(selected), None);
        result.qualification = NativeNameQualification::Unqualified;
        result
    }

    /// C trace text extent is separate from the followed physical receiver.
    ///
    /// # Errors
    /// Returns `PurposeNotModelled` for an unaudited engine operation.
    pub fn trace_registration_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        self.trace_input(original, NativeNamePurpose::VariableTraceRegistration)
    }

    ///
    /// # Errors
    /// Returns `PurposeNotModelled` for an unaudited engine operation.
    pub fn trace_query_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        self.trace_input(original, NativeNamePurpose::VariableTraceQuery)
    }

    fn trace_input(
        self,
        original: &[u8],
        purpose: NativeNamePurpose,
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(projection(
            self,
            purpose,
            original,
            Cow::Borrowed(c_string_extent(original)),
            None,
        ))
    }

    /// A parsed formal operand's native name key, independently of list parsing.
    #[must_use]
    pub fn formal_name_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        self.formal_enumeration_name_input(original)
    }

    /// Native `info args`/`info default` name presentation and comparison.
    /// This input does not select the procedure's actual local storage key.
    #[must_use]
    pub fn formal_enumeration_name_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        let selected = if self.is_jim084() {
            original
        } else {
            c_string_extent(original)
        };
        projection(
            self,
            NativeNamePurpose::FormalEnumeration,
            original,
            Cow::Borrowed(selected),
            None,
        )
    }

    /// Input to each formal declaration list level. C8.4/8.5 split a
    /// `CString`; modern C and Jim retain counted list objects. This pure
    /// extent recipe grants neither successful parsing nor an activation.
    #[must_use]
    pub fn formal_parameter_list_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        let selected = if matches!(self, Self::C(version) if version <= TclVersion::V8_5) {
            c_string_extent(original)
        } else {
            original
        };
        projection(
            self,
            NativeNamePurpose::FormalParameterList,
            original,
            Cow::Borrowed(selected),
            None,
        )
    }

    /// Formal storage after the caller has parsed the native argument-list
    /// representation. C8.4/8.5 declarations use `CString` list splitting;
    /// C8.6+ and Jim retain the complete selected formal bytes.
    #[must_use]
    pub fn formal_storage_name_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        let selected = if matches!(self, Self::C(version) if version <= TclVersion::V8_5) {
            c_string_extent(original)
        } else {
            original
        };
        projection(
            self,
            NativeNamePurpose::FormalStorage,
            original,
            Cow::Borrowed(selected),
            None,
        )
    }

    /// The local receiver published by `namespace upvar`. Its C argument is
    /// a `CString` even when ordinary `upvar`, `global` and `variable` locals
    /// retain a counted object. C8.4 does not provide this command purpose.
    ///
    /// # Errors
    /// Returns `PurposeNotModelled` for C8.4.
    pub fn namespace_upvar_local_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if self == Self::C(TclVersion::V8_4) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let selected = if self.is_jim084() {
            original
        } else {
            c_string_extent(original)
        };
        Ok(projection(
            self,
            NativeNamePurpose::NamespaceUpvarLocal,
            original,
            Cow::Borrowed(selected),
            None,
        ))
    }
}
