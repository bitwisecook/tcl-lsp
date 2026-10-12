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

//! Codec/result-presentation assertions, independent of native header authority.

#[test]
fn selected_utf8_units_match_all_55_original_c_conversion_windows() {
    // naming.encoding.original-utf8-convertto-object-storage
    // docs/design/analysis/name-resolution-proofs/encoding-original-utf8-convertto-object-storage.md
    // naming.encoding.original-utf8-convertto-error-offset
    // docs/design/analysis/name-resolution-proofs/encoding-original-utf8-convertto-error-offset.md
    // This pure comparison owns converted bytes/structured error presentation;
    // backend tests independently exercise actual original getters/result storage.
    use tcl_syntax::native_string::{NativeStringInput, NativeStringProtocol};
    use tcl_test_support::encoding_utf8::{CONTROLS, decode_hex, original_input};
    let mut comparisons = 0;
    for control in CONTROLS {
        for (version, column) in tcl_dialect::TclVersion::ALL
            .into_iter()
            .zip(control.columns)
        {
            let original = original_input(column);
            let source = if control.binary {
                NativeStringProtocol::C(version)
                    .materialize(NativeStringInput::PureByteArray(&original))
                    .unwrap()
                    .into_owned()
            } else {
                original
            };
            match super::external_utf8_octets(version, &source) {
                Ok(bytes) => {
                    let raw = column
                        .lines()
                        .find(|line| line.starts_with("RAW|"))
                        .unwrap();
                    assert_eq!(
                        bytes,
                        decode_hex(raw.rsplit('|').next().unwrap()),
                        "{version:?}/{}",
                        control.case
                    );
                }
                Err(failure) => {
                    let message = column
                        .lines()
                        .find(|line| line.starts_with("MESSAGE|"))
                        .unwrap();
                    let code = column
                        .lines()
                        .find(|line| line.starts_with("ERROR_CODE|"))
                        .unwrap();
                    let details = failure.command_error(version).into_byte_details();
                    assert_eq!(
                        details.message,
                        decode_hex(message.split_once('|').unwrap().1)
                    );
                    assert_eq!(
                        details.error_code,
                        crate::CmdErrorCodeUpdate::Set(decode_hex(code.split_once('|').unwrap().1))
                    );
                    if control.case == "string-eacute-single-high-surrogate" {
                        assert_eq!(failure.character_index, 1);
                        assert_eq!(failure.byte_offset, 2);
                    }
                }
            }
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 55);
}
