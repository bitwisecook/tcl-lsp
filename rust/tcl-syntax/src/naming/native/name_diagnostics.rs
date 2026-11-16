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

//! Diagnostics for a refused rename, hide or expose and for a missing or conflicting namespace.

use super::{NameProjectionUnavailable, NativeNameProtocol};
use tcl_core_types::c_string_extent;
use tcl_dialect::TclVersion;

/// The failed stage selected by the actual rename operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRenameFailure {
    /// The source was absent when requesting a nonempty destination.
    MissingSource,
    /// The source was absent when requesting deletion.
    MissingDeleteSource,
    /// The destination already has a command binding.
    TargetExists,
}

/// Selected rename result bytes and independently selected explicit error code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRenameDiagnostic {
    /// Exact native diagnostic bytes.
    pub message: Vec<u8>,
    /// Explicit native tuple; absence uses the adapter's neutral error path.
    pub error_code: Option<Vec<u8>>,
}

impl NativeNameProtocol {
    /// Project an actual rename refusal without selecting or mutating bindings.
    /// Original counted operands remain independent of Jim's report extent.
    #[must_use]
    pub fn rename_diagnostic(
        self,
        failure: NativeRenameFailure,
        original: &[u8],
    ) -> NativeRenameDiagnostic {
        let reported = if self.is_jim084() {
            c_string_extent(original)
        } else {
            original
        };
        let (prefix, suffix) = match failure {
            NativeRenameFailure::MissingSource => (
                b"can't rename \"".as_slice(),
                b"\": command doesn't exist".as_slice(),
            ),
            NativeRenameFailure::MissingDeleteSource => (
                b"can't delete \"".as_slice(),
                b"\": command doesn't exist".as_slice(),
            ),
            NativeRenameFailure::TargetExists => (
                b"can't rename to \"".as_slice(),
                b"\": command already exists".as_slice(),
            ),
        };
        let message = [prefix, reported, suffix].concat();
        let error_code = if matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            Some(match failure {
                NativeRenameFailure::MissingSource | NativeRenameFailure::MissingDeleteSource => {
                    let mut code = b"TCL LOOKUP COMMAND ".to_vec();
                    crate::list::append_list_element(&mut code, original, false);
                    code
                }
                NativeRenameFailure::TargetExists => b"TCL OPERATION RENAME TARGET_EXISTS".to_vec(),
            })
        } else {
            None
        };
        NativeRenameDiagnostic {
            message,
            error_code,
        }
    }
}

/// Actual native hide/expose failure, independent of command lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCommandVisibilityFailure {
    /// Hidden destination token contains a namespace qualifier.
    InvalidHiddenToken,
    /// Selected command to hide is outside the global namespace.
    HideNonGlobal,
    /// Requested exposed destination contains a namespace qualifier.
    ExposeNonGlobal,
    /// Actual hidden source is outside the global namespace.
    HiddenSourceNonGlobal,
    /// Visible source command was absent.
    MissingCommand,
    /// Hidden source token was absent during exposure.
    MissingHiddenToken,
    /// Hidden invocation token was absent.
    InvokeHiddenMissing,
    /// Hidden destination token was occupied.
    AlreadyHidden,
    /// Visible destination command was occupied.
    CommandExists,
}

/// Native visibility diagnostic bytes with the selected explicit tuple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCommandVisibilityDiagnostic {
    /// Exact native diagnostic bytes.
    pub message: Vec<u8>,
    /// Explicit native tuple; absence uses the adapter's neutral error path.
    pub error_code: Option<Vec<u8>>,
}

impl NativeNameProtocol {
    /// Project an actual C hide/expose refusal without selecting any binding.
    ///
    /// # Errors
    /// Jim has no C hidden-command operation.
    pub fn command_visibility_diagnostic(
        self,
        failure: NativeCommandVisibilityFailure,
        original: &[u8],
    ) -> Result<NativeCommandVisibilityDiagnostic, NameProjectionUnavailable> {
        let Self::C(version) = self else {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        };
        let name = c_string_extent(original);
        let (message, code) = match failure {
            NativeCommandVisibilityFailure::InvalidHiddenToken => (
                b"cannot use namespace qualifiers in hidden command token (rename)".to_vec(),
                b"TCL VALUE HIDDENTOKEN".as_slice(),
            ),
            NativeCommandVisibilityFailure::HideNonGlobal => (
                b"can only hide global namespace commands (use rename then hide)".to_vec(),
                b"TCL HIDE NON_GLOBAL".as_slice(),
            ),
            NativeCommandVisibilityFailure::ExposeNonGlobal => (
                if version == TclVersion::V8_4 {
                    b"can not expose to a namespace (use expose to toplevel, then rename)".to_vec()
                } else {
                    b"cannot expose to a namespace (use expose to toplevel, then rename)".to_vec()
                },
                b"TCL EXPOSE NON_GLOBAL".as_slice(),
            ),
            NativeCommandVisibilityFailure::HiddenSourceNonGlobal => (
                if version <= TclVersion::V8_5 {
                    b"trying to expose a non global command name space command".to_vec()
                } else {
                    b"trying to expose a non-global command namespace command".to_vec()
                },
                b"NONE".as_slice(),
            ),
            NativeCommandVisibilityFailure::MissingCommand => (
                [b"unknown command \"".as_slice(), name, b"\""].concat(),
                b"TCL LOOKUP COMMAND".as_slice(),
            ),
            NativeCommandVisibilityFailure::MissingHiddenToken => (
                [b"unknown hidden command \"".as_slice(), name, b"\""].concat(),
                b"TCL LOOKUP HIDDENTOKEN".as_slice(),
            ),
            NativeCommandVisibilityFailure::InvokeHiddenMissing => (
                [b"invalid hidden command name \"".as_slice(), name, b"\""].concat(),
                b"TCL LOOKUP HIDDENTOKEN".as_slice(),
            ),
            NativeCommandVisibilityFailure::AlreadyHidden => (
                [
                    b"hidden command named \"".as_slice(),
                    name,
                    b"\" already exists",
                ]
                .concat(),
                b"TCL HIDE ALREADY_HIDDEN".as_slice(),
            ),
            NativeCommandVisibilityFailure::CommandExists => (
                [b"exposed command \"".as_slice(), name, b"\" already exists"].concat(),
                b"TCL EXPOSE COMMAND_EXISTS".as_slice(),
            ),
        };
        let error_code = ((version >= TclVersion::V8_6
            || (version == TclVersion::V8_5
                && failure == NativeCommandVisibilityFailure::MissingCommand))
            && failure != NativeCommandVisibilityFailure::HiddenSourceNonGlobal)
            .then(|| {
                let mut tuple = code.to_vec();
                if matches!(
                    failure,
                    NativeCommandVisibilityFailure::MissingCommand
                        | NativeCommandVisibilityFailure::MissingHiddenToken
                        | NativeCommandVisibilityFailure::InvokeHiddenMissing
                ) {
                    tuple.push(b' ');
                    crate::list::append_list_element(&mut tuple, name, false);
                }
                tuple
            });
        Ok(NativeCommandVisibilityDiagnostic {
            message,
            error_code,
        })
    }
}

/// Native namespace lookup failure, with a complete current-context report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNamespaceLookupError {
    /// Native diagnostic bytes.
    pub message: Vec<u8>,
    /// Native error code as Tcl list bytes.
    pub error_code: Vec<u8>,
}

/// Report a failed native namespace address lookup without using display as identity.
///
/// # Errors
/// Tcl 8.4 requires the caller's operation; Jim uses distinct script helpers.
pub fn report_native_namespace_lookup_error(
    protocol: NativeNameProtocol,
    original: &[u8],
    current_fullname: &[u8],
) -> Result<NativeNamespaceLookupError, NameProjectionUnavailable> {
    if protocol.is_jim084() || protocol == NativeNameProtocol::C(TclVersion::V8_4) {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    Ok(namespace_lookup_error_parts(original, current_fullname))
}

fn namespace_lookup_error_parts(
    original: &[u8],
    current_fullname: &[u8],
) -> NativeNamespaceLookupError {
    let written = c_string_extent(original);
    let mut message = b"namespace \"".to_vec();
    message.extend_from_slice(written);
    message.extend_from_slice(b"\" not found");
    if !written.starts_with(b"::") {
        message.extend_from_slice(b" in \"");
        message.extend_from_slice(c_string_extent(current_fullname));
        message.push(b'"');
    }
    let mut error_code = b"TCL LOOKUP NAMESPACE ".to_vec();
    crate::list::append_list_element(&mut error_code, written, false);
    NativeNamespaceLookupError {
        message,
        error_code,
    }
}

/// The native namespace operation that selected a missing address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNamespaceLookupOperation {
    /// Later-C `TclGetNamespaceFromObj` reporting used by namespace upvar/path.
    ObjectLookup,
    /// Namespace parent query.
    Parent,
    /// Namespace children query.
    Children,
    /// Namespace deletion.
    Delete,
    /// Namespace inscope target selection.
    Inscope,
}

/// Native namespace operation failure with its primitive error-code update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNamespaceOperationError {
    /// Complete diagnostic bytes.
    pub message: Vec<u8>,
    /// Explicit native tuple; absence retains the primitive error-code state.
    pub error_code: Option<Vec<u8>>,
    /// The native append/format result owns an unknown-count String primary.
    /// This pure recipe does not itself authorise physical cache installation.
    pub string_protocol: crate::native_string::NativeStringProtocol,
}

/// Report the selected invalid C namespace-creation address.
///
/// # Errors
/// Refuses unsupported recipes and failures without this exact address reason.
pub fn report_native_namespace_creation_error(
    protocol: NativeNameProtocol,
    reason: NameProjectionUnavailable,
) -> Result<NativeNamespaceOperationError, NameProjectionUnavailable> {
    let NativeNameProtocol::C(version) = protocol else {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    };
    if reason != NameProjectionUnavailable::EmptyNamespaceOutsideRoot {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    Ok(NativeNamespaceOperationError {
        message: b"can't create namespace \"\": only global namespace can have empty name".to_vec(),
        error_code: (version >= TclVersion::V8_6)
            .then(|| b"TCL OPERATION NAMESPACE CREATEGLOBAL".to_vec()),
        string_protocol: crate::native_string::NativeStringProtocol::C(version),
    })
}

/// Report C namespace-creation occupancy using its original C-string operand.
///
/// # Errors
/// Refuses unaudited Jim namespace-creation diagnostics.
pub fn report_native_namespace_creation_conflict(
    protocol: NativeNameProtocol,
    original: &[u8],
) -> Result<NativeNamespaceOperationError, NameProjectionUnavailable> {
    let NativeNameProtocol::C(version) = protocol else {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    };
    Ok(NativeNamespaceOperationError {
        message: [
            b"can't create namespace \"".as_slice(),
            c_string_extent(original),
            b"\": already exists",
        ]
        .concat(),
        error_code: (version >= TclVersion::V8_6)
            .then(|| b"TCL OPERATION NAMESPACE CREATEEXISTING".to_vec()),
        string_protocol: crate::native_string::NativeStringProtocol::C(version),
    })
}

/// Report a missing namespace using the actual operation and engine release.
///
/// # Errors
/// Refuses unaudited Jim failure operations and unavailable C84 object APIs.
pub fn report_native_namespace_operation_error(
    protocol: NativeNameProtocol,
    operation: NativeNamespaceLookupOperation,
    original: &[u8],
    current_fullname: &[u8],
) -> Result<NativeNamespaceOperationError, NameProjectionUnavailable> {
    use NativeNamespaceLookupOperation as Operation;
    if protocol.is_jim084()
        || (protocol == NativeNameProtocol::C(TclVersion::V8_4)
            && operation == Operation::ObjectLookup)
    {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    let report = namespace_lookup_error_parts(original, current_fullname);
    let legacy = protocol == NativeNameProtocol::C(TclVersion::V8_4);
    let message = if legacy || operation == Operation::Delete {
        let mut message = b"unknown namespace \"".to_vec();
        message.extend_from_slice(c_string_extent(original));
        message.extend_from_slice(b"\" in ");
        if operation == Operation::Inscope {
            message.extend_from_slice(b"inscope namespace command");
        } else {
            message.extend_from_slice(b"namespace ");
            message.extend_from_slice(match operation {
                Operation::Parent => b"parent",
                Operation::Children => b"children",
                Operation::Delete => b"delete",
                Operation::ObjectLookup | Operation::Inscope => unreachable!("selected above"),
            });
            message.extend_from_slice(b" command");
        }
        message
    } else {
        report.message
    };
    Ok(NativeNamespaceOperationError {
        message,
        error_code: (!legacy).then_some(report.error_code),
        string_protocol: crate::native_string::NativeStringProtocol::C(match protocol {
            NativeNameProtocol::C(version) => version,
            NativeNameProtocol::Jim084 => unreachable!("Jim operation refused above"),
        }),
    })
}
