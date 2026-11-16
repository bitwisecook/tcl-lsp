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

//! Variable failure diagnostics: the operation, reason and failure site, and the projection of the original operand at that site.

use super::{NameProjectionUnavailable, NativeNameProtocol, NativeVariableInputForm};
use tcl_core_types::c_string_extent;
use tcl_dialect::TclVersion;

/// The variable operation whose diagnostic retains original operand parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVariableDiagnosticOperation {
    Read,
    Write,
    Unset,
    /// Native increment constant-write failure; reached reads use `Read`.
    Increment,
    /// Native C9 constant declaration failure on the selected variable cell.
    MakeConstant,
    /// Namespace declaration lookup, independently of a following value write.
    Define,
}

/// The actual variable failure selected by the physical runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVariableDiagnosticReason {
    NoSuchVariable,
    NoSuchElement,
    NotArray,
    IsArray,
    MissingParentNamespace,
    Constant,
    DetachedElement,
    RetiredNamespace,
    /// A constant declaration targets an existing non-constant variable.
    AlreadyExists,
    /// A constant or namespace declaration targets an array element.
    ArrayElement,
}

impl NativeVariableDiagnosticReason {
    /// Native reason text after operation-specific selection.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::NoSuchVariable => "no such variable",
            Self::NoSuchElement => "no such element in array",
            Self::NotArray => "variable isn't array",
            Self::IsArray => "variable is array",
            Self::MissingParentNamespace => "parent namespace doesn't exist",
            Self::Constant => "variable is a constant",
            Self::DetachedElement => "upvar refers to element in deleted array",
            Self::RetiredNamespace => "upvar refers to variable in deleted namespace",
            Self::AlreadyExists => "variable already exists",
            Self::ArrayElement => "name refers to an element in an array",
        }
    }
}

/// Failure site retained by the physical variable operation. Identical reason
/// text at two sites does not imply identical reporting or error-code tuples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVariableFailureSite {
    NameLookup,
    /// Early element lookup by an actually admitted indexed local instruction.
    IndexedNameLookup,
    ValueRead,
    ValueWrite,
    ValueUnset,
}

/// Purpose-selected diagnostic output, independent of cell identity and argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeVariableDiagnosticProjection {
    /// Operation-selected name spelling for the diagnostic.
    pub name: Vec<u8>,
    /// Effective reason, including Jim's unset-on-scalar dictionary behaviour.
    pub reason: NativeVariableDiagnosticReason,
    /// Explicit native error-code words. `None` means this operation does not
    /// issue a tuple; the interpreter's error initialisation owns its default.
    pub error_code: Option<Vec<Vec<u8>>>,
    /// Compatibility lookup-root view, only for actual LOOKUP VARNAME tuples.
    pub missing_lookup_root: Option<Vec<u8>>,
}

/// Original operand name for a reached C variable value access. Combined
/// inputs retain their original diagnostic extent; separate operands retain
/// their own extents before parentheses are added for presentation only.
/// This selects no variable cell, alias or native execution capability.
///
/// # Errors
/// Returns `PurposeNotModelled` for a non-C protocol.
pub fn report_native_c_variable_value_name(
    protocol: NativeNameProtocol,
    input: NativeVariableInputForm<'_>,
) -> Result<Vec<u8>, NameProjectionUnavailable> {
    if !matches!(protocol, NativeNameProtocol::C(_)) {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    Ok(match input {
        NativeVariableInputForm::Combined(original) => c_string_extent(original).to_vec(),
        NativeVariableInputForm::Separate { root, element } => {
            let mut name = c_string_extent(root).to_vec();
            if let Some(element) = element {
                name.push(b'(');
                name.extend_from_slice(c_string_extent(element));
                name.push(b')');
            }
            name
        }
    })
}

/// Report a failure occurring while selecting a variable name-table binding.
/// Late cell reads/writes must use [`report_native_variable_diagnostic_at`].
///
/// # Errors
/// Returns `PurposeNotModelled` for an unmeasured failure/input combination.
pub fn report_native_variable_diagnostic(
    protocol: NativeNameProtocol,
    operation: NativeVariableDiagnosticOperation,
    reason: NativeVariableDiagnosticReason,
    input: NativeVariableInputForm<'_>,
) -> Result<NativeVariableDiagnosticProjection, NameProjectionUnavailable> {
    report_native_variable_diagnostic_at(
        protocol,
        operation,
        reason,
        NativeVariableFailureSite::NameLookup,
        input,
    )
}

/// Select the measured C9 constant failure verb at its physical site.
/// Early binding failures report `const`; late receiver failures report
/// `make constant`. This does not select a variable receiver or caller opcode.
///
/// # Errors
/// Refuses other engines and unsupported failure sites.
pub fn native_constant_failure_verb(
    protocol: NativeNameProtocol,
    site: NativeVariableFailureSite,
) -> Result<&'static str, NameProjectionUnavailable> {
    if !matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V9_0) {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    match site {
        NativeVariableFailureSite::NameLookup => Ok("const"),
        NativeVariableFailureSite::ValueWrite => Ok("make constant"),
        _ => Err(NameProjectionUnavailable::PurposeNotModelled),
    }
}

fn special_variable_failure_projection(
    protocol: NativeNameProtocol,
    operation: NativeVariableDiagnosticOperation,
    reason: NativeVariableDiagnosticReason,
    site: NativeVariableFailureSite,
    input: NativeVariableInputForm<'_>,
) -> Result<Option<NativeVariableDiagnosticProjection>, NameProjectionUnavailable> {
    use NativeVariableDiagnosticOperation::MakeConstant;
    use NativeVariableDiagnosticReason::{
        IsArray, MissingParentNamespace, NoSuchElement, NotArray,
    };
    use NativeVariableFailureSite::{NameLookup, ValueWrite};
    if operation == NativeVariableDiagnosticOperation::Define {
        if !matches!(protocol, NativeNameProtocol::C(_)) || site != NameLookup {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        if reason == NativeVariableDiagnosticReason::ArrayElement {
            let NativeVariableInputForm::Combined(original) = input else {
                return Err(NameProjectionUnavailable::PurposeNotModelled);
            };
            return Ok(Some(NativeVariableDiagnosticProjection {
                name: c_string_extent(original).to_vec(),
                reason,
                error_code: Some(vec![
                    b"TCL".to_vec(),
                    b"UPVAR".to_vec(),
                    b"LOCAL_ELEMENT".to_vec(),
                ]),
                missing_lookup_root: None,
            }));
        }
    }
    if operation == MakeConstant {
        native_constant_failure_verb(protocol, site)?;
        if site == NameLookup && matches!(reason, MissingParentNamespace | NotArray | NoSuchElement)
        {
            // Binding failures use the ordinary independently split name parts.
        } else if site == ValueWrite
            && matches!(
                reason,
                IsArray
                    | NativeVariableDiagnosticReason::AlreadyExists
                    | NativeVariableDiagnosticReason::ArrayElement
            )
        {
            let name = match input {
                NativeVariableInputForm::Combined(original) => c_string_extent(original).to_vec(),
                NativeVariableInputForm::Separate { root, element } => {
                    let mut name = c_string_extent(root).to_vec();
                    if let Some(element) = element {
                        name.push(b'(');
                        name.extend_from_slice(c_string_extent(element));
                        name.push(b')');
                    }
                    name
                }
            };
            return Ok(Some(NativeVariableDiagnosticProjection {
                name,
                reason,
                error_code: Some(vec![b"TCL".to_vec(), b"LOOKUP".to_vec(), b"CONST".to_vec()]),
                missing_lookup_root: None,
            }));
        } else {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
    }
    Ok(None)
}

fn native_variable_failure_code(
    protocol: NativeNameProtocol,
    operation: NativeVariableDiagnosticOperation,
    reason: NativeVariableDiagnosticReason,
    site: NativeVariableFailureSite,
    root: &[u8],
    element: Option<&[u8]>,
) -> Result<Option<Vec<Vec<u8>>>, NameProjectionUnavailable> {
    use NativeVariableDiagnosticOperation::Increment;
    use NativeVariableDiagnosticReason::{
        Constant, DetachedElement, IsArray, MissingParentNamespace, NoSuchElement, NoSuchVariable,
        NotArray, RetiredNamespace,
    };
    use NativeVariableFailureSite::{
        IndexedNameLookup, NameLookup, ValueRead, ValueUnset, ValueWrite,
    };
    let error_code = if matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V8_6)
    {
        let words: &[&[u8]] = match (site, reason) {
            (IndexedNameLookup, RetiredNamespace)
                if operation == NativeVariableDiagnosticOperation::Write && element.is_some() =>
            {
                &[b"TCL", b"LOOKUP", b"VARNAME"]
            }
            (NameLookup, RetiredNamespace)
                if operation == NativeVariableDiagnosticOperation::Write && element.is_some() =>
            {
                &[b"TCL", b"LOOKUP", b"VARNAME", root]
            }
            (NameLookup, NoSuchVariable | MissingParentNamespace | NotArray) => {
                &[b"TCL", b"LOOKUP", b"VARNAME", root]
            }
            (NameLookup, NoSuchElement) if element.is_some() => &[
                b"TCL",
                b"LOOKUP",
                b"ELEMENT",
                element.expect("selected element"),
            ],
            (ValueRead, NoSuchVariable | NoSuchElement | IsArray | DetachedElement) => {
                &[b"TCL", b"READ", b"VARNAME"]
            }
            (ValueUnset, NoSuchVariable | NoSuchElement) => &[b"TCL", b"UNSET", b"VARNAME"],
            (ValueWrite, Constant) if operation == Increment => &[b"TCL", b"WRITE", b"CONST"],
            (
                ValueWrite,
                NoSuchVariable | IsArray | Constant | DetachedElement | RetiredNamespace,
            ) => &[b"TCL", b"WRITE", b"VARNAME"],
            (ValueUnset, Constant) if matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V9_0) => {
                &[b"TCL", b"UNSET", b"CONST"]
            }
            _ => return Err(NameProjectionUnavailable::PurposeNotModelled),
        };
        Some(words.iter().map(|word| word.to_vec()).collect::<Vec<_>>())
    } else if protocol == NativeNameProtocol::C(TclVersion::V8_5)
        && operation == NativeVariableDiagnosticOperation::Unset
        && reason == NoSuchElement
        && site == NameLookup
    {
        // C8.5's element lookup tags only the unset miss.
        element.map(|element| {
            vec![
                b"TCL".to_vec(),
                b"LOOKUP".to_vec(),
                b"ELEMENT".to_vec(),
                element.to_vec(),
            ]
        })
    } else {
        None
    };
    Ok(error_code)
}

/// C checks a link into a deleted namespace or array only when storing; a
/// read or unset finds the dead cell undefined.
fn retired_namespace_failure(
    protocol: NativeNameProtocol,
    operation: NativeVariableDiagnosticOperation,
    reason: NativeVariableDiagnosticReason,
    site: NativeVariableFailureSite,
) -> (NativeVariableDiagnosticReason, NativeVariableFailureSite) {
    use NativeVariableDiagnosticOperation::{Read, Unset};
    use NativeVariableDiagnosticReason::{DetachedElement, NoSuchVariable, RetiredNamespace};
    use NativeVariableFailureSite::{IndexedNameLookup, ValueRead, ValueUnset};
    if !matches!(protocol, NativeNameProtocol::C(_))
        || !matches!(reason, RetiredNamespace | DetachedElement)
        || site == IndexedNameLookup
    {
        return (reason, site);
    }
    match operation {
        Read => (NoSuchVariable, ValueRead),
        Unset => (NoSuchVariable, ValueUnset),
        _ => (reason, site),
    }
}

/// Project the original input at the actual physical failure site. C lookup
/// diagnostics format `CString` root/element parts independently. A late read
/// uses the original combined `CString` operand, so an embedded NUL can remove
/// its closing parenthesis. Separate object inputs retain their own boundaries.
///
/// # Errors
/// Returns `PurposeNotModelled` for an unaudited site, operation or Jim input form.
pub fn report_native_variable_diagnostic_at(
    protocol: NativeNameProtocol,
    operation: NativeVariableDiagnosticOperation,
    reason: NativeVariableDiagnosticReason,
    site: NativeVariableFailureSite,
    input: NativeVariableInputForm<'_>,
) -> Result<NativeVariableDiagnosticProjection, NameProjectionUnavailable> {
    use NativeVariableDiagnosticOperation::{Increment, Read, Unset, Write};
    use NativeVariableDiagnosticReason::{Constant, NoSuchElement, NoSuchVariable, NotArray};
    use NativeVariableFailureSite::{IndexedNameLookup, ValueRead, ValueUnset, ValueWrite};
    let (reason, site) = retired_namespace_failure(protocol, operation, reason, site);
    if site == IndexedNameLookup
        && !(matches!(protocol, NativeNameProtocol::C(_))
            && operation == Write
            && reason == NativeVariableDiagnosticReason::RetiredNamespace)
    {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    if let Some(projection) =
        special_variable_failure_projection(protocol, operation, reason, site, input)?
    {
        return Ok(projection);
    }
    if matches!(
        reason,
        NativeVariableDiagnosticReason::AlreadyExists
            | NativeVariableDiagnosticReason::ArrayElement
    ) {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    if matches!(
        (site, operation),
        (ValueRead, Write | Unset | Increment)
            | (ValueWrite, Read | Unset)
            | (ValueUnset, Read | Write | Increment)
    ) {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    if operation == Increment
        && !(site == ValueWrite
            && reason == Constant
            && matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V9_0))
    {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    let parts = match input {
        NativeVariableInputForm::Combined(original) => protocol.combined_variable_input(original),
        NativeVariableInputForm::Separate { root, element } => {
            protocol.separate_variable_input(root, element)
        }
    };
    let root = c_string_extent(parts.root().original());
    let element = parts.element().map(|part| c_string_extent(part.original()));
    if site == IndexedNameLookup && element.is_none() {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    let reconstructed = || {
        let mut name = root.to_vec();
        if let Some(element) = element {
            name.push(b'(');
            name.extend_from_slice(element);
            name.push(b')');
        }
        name
    };
    let mut effective_reason = reason;
    let name = if protocol.is_jim084() {
        match input {
            NativeVariableInputForm::Separate { .. } => match (operation, reason) {
                (Read, NoSuchVariable) => root.to_vec(),
                (Read, NoSuchElement | NotArray) if element.is_some() => reconstructed(),
                _ => return Err(NameProjectionUnavailable::PurposeNotModelled),
            },
            NativeVariableInputForm::Combined(original) => match (operation, reason) {
                (Read, NoSuchVariable) if element.is_some() => root.to_vec(),
                (Read, NoSuchElement | NotArray) => reconstructed(),
                (Read | Write | Unset, NoSuchVariable)
                | (Write, NotArray)
                | (Unset, NoSuchElement) => c_string_extent(original).to_vec(),
                (Unset, NotArray) => {
                    effective_reason = NoSuchElement;
                    c_string_extent(original).to_vec()
                }
                _ => return Err(NameProjectionUnavailable::PurposeNotModelled),
            },
        }
    } else if matches!(site, ValueRead | ValueWrite) {
        report_native_c_variable_value_name(protocol, input)?
    } else {
        reconstructed()
    };
    let error_code =
        native_variable_failure_code(protocol, operation, reason, site, root, element)?;
    let missing_lookup_root = error_code
        .as_ref()
        .filter(|words| words.len() == 4 && words[1] == b"LOOKUP" && words[2] == b"VARNAME")
        .map(|words| words[3].clone());
    Ok(NativeVariableDiagnosticProjection {
        name,
        reason: effective_reason,
        error_code,
        missing_lookup_root,
    })
}
