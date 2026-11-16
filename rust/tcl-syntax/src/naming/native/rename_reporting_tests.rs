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

use super::*;

#[test]
fn rename_diagnostic_codes_follow_original_c_releases_and_jim() {
    let original = b"::missing\0suffix";
    for protocol in TclVersion::ALL
        .into_iter()
        .map(NativeNameProtocol::C)
        .chain([NativeNameProtocol::Jim084])
    {
        for (failure, prefix, suffix) in [
            (
                NativeRenameFailure::MissingSource,
                b"can't rename \"".as_slice(),
                b"\": command doesn't exist".as_slice(),
            ),
            (
                NativeRenameFailure::MissingDeleteSource,
                b"can't delete \"".as_slice(),
                b"\": command doesn't exist".as_slice(),
            ),
            (
                NativeRenameFailure::TargetExists,
                b"can't rename to \"".as_slice(),
                b"\": command already exists".as_slice(),
            ),
        ] {
            let error = protocol.rename_diagnostic(failure, original);
            let extent = if protocol.is_jim084() {
                b"::missing".as_slice()
            } else {
                original
            };
            assert_eq!(error.message, [prefix, extent, suffix].concat());
            let expected = match (protocol, failure) {
                (NativeNameProtocol::C(version), _) if version <= TclVersion::V8_5 => None,
                (NativeNameProtocol::Jim084, _) => None,
                (_, NativeRenameFailure::TargetExists) => {
                    Some(b"TCL OPERATION RENAME TARGET_EXISTS".as_slice())
                }
                _ => Some(b"TCL LOOKUP COMMAND ::missing\0suffix".as_slice()),
            };
            assert_eq!(error.error_code.as_deref(), expected);
        }
    }
}

#[test]
fn rename_diagnostics_preserve_counted_lookup_operands_and_selected_report_extent() {
    let original = b"::alias\0old";
    for protocol in tcl_dialect::TclVersion::ALL
        .into_iter()
        .map(NativeNameProtocol::C)
        .chain([NativeNameProtocol::Jim084])
    {
        assert_eq!(
            report_native_name_bytes(
                protocol,
                NativeNameReportPurpose::RenameCommandError,
                original
            )
            .unwrap(),
            if protocol.is_jim084() {
                b"::alias".as_slice()
            } else {
                original
            }
        );
        let lookup = protocol
            .rename_source_input(NativeNameContext::root(), original)
            .unwrap();
        assert_eq!(lookup.original(), original);
        if protocol.is_jim084() {
            assert_eq!(lookup.selected(), original);
        }
    }
}
