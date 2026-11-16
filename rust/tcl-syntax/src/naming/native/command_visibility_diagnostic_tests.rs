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

fn hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn command_visibility_diagnostics_match_original_eight_reason_c_releases() {
    for (version, original) in [
        (
            TclVersion::V8_4,
            include_str!("../native/original_visibility/c8_4.stdout"),
        ),
        (
            TclVersion::V8_5,
            include_str!("../native/original_visibility/c8_5.stdout"),
        ),
        (
            TclVersion::V8_6,
            include_str!("../native/original_visibility/c8_6.stdout"),
        ),
        (
            TclVersion::V9_0,
            include_str!("../native/original_visibility/c9_0.stdout"),
        ),
        (
            TclVersion::V9_1,
            include_str!("../native/original_visibility/c9_1.stdout"),
        ),
    ] {
        let mut count = 0;
        for row in original.lines() {
            let fields: Vec<_> = row.split_whitespace().collect();
            assert_eq!(fields.len(), 4);
            assert_eq!(fields[1], "1");
            let (failure, name) = match fields[0] {
                "invalid-token" => (
                    NativeCommandVisibilityFailure::InvalidHiddenToken,
                    b"N::held".as_slice(),
                ),
                "hide-non-global" => (
                    NativeCommandVisibilityFailure::HideNonGlobal,
                    b"N::p".as_slice(),
                ),
                "expose-non-global" => (
                    NativeCommandVisibilityFailure::ExposeNonGlobal,
                    b"::destination".as_slice(),
                ),
                "missing-command" => (
                    NativeCommandVisibilityFailure::MissingCommand,
                    b"missing".as_slice(),
                ),
                "missing-hidden" => (
                    NativeCommandVisibilityFailure::MissingHiddenToken,
                    b"N::missing".as_slice(),
                ),
                "already-hidden" => (
                    NativeCommandVisibilityFailure::AlreadyHidden,
                    b"held".as_slice(),
                ),
                "command-exists" => (
                    NativeCommandVisibilityFailure::CommandExists,
                    b"other".as_slice(),
                ),
                "invoke-hidden-missing" => (
                    NativeCommandVisibilityFailure::InvokeHiddenMissing,
                    b"N::missing".as_slice(),
                ),
                unknown => panic!("unexpected original reason {unknown}"),
            };
            let error = NativeNameProtocol::C(version)
                .command_visibility_diagnostic(failure, name)
                .unwrap();
            assert_eq!(error.message, hex(fields[2]), "{version:?} {}", fields[0]);
            assert_eq!(
                error.error_code.as_deref().unwrap_or(b"NONE"),
                hex(fields[3]),
                "{version:?} {}",
                fields[0]
            );
            count += 1;
        }
        assert_eq!(count, 8);
    }
    assert_eq!(
        NativeNameProtocol::Jim084.command_visibility_diagnostic(
            NativeCommandVisibilityFailure::MissingCommand,
            b"missing"
        ),
        Err(NameProjectionUnavailable::PurposeNotModelled)
    );
}
