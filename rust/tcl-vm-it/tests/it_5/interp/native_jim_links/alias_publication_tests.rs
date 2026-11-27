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

//! Jim alias reads and `upvar` error publication, against the tclsh and
//! jimsh captures.

use crate::prelude::*;

#[test]
fn original_upvar_error_publication_matches_all_six_native_sources() {
    // naming.variable.original-upvar-error-publication
    // docs/design/analysis/name-resolution-proofs/variable-original-upvar-error-publication.md
    // Public script results only; C9 legacy trace refusal is a separate row.
    let source = include_str!(
        "../../../../../tcl-registry/tests/data/native_upvar_error_publication307/source.tcl"
    );
    for (engine, stdout) in [
        (
            "tcl8.4",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_upvar_error_publication307/tcl8.4/original-upvar-errors/stdout"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_upvar_error_publication307/tcl8.5/original-upvar-errors/stdout"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_upvar_error_publication307/tcl8.6/original-upvar-errors/stdout"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_upvar_error_publication307/tcl9.0/original-upvar-errors/stdout"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_upvar_error_publication307/tcl9.1/original-upvar-errors/stdout"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_upvar_error_publication307/jim/original-upvar-errors/stdout"
            ),
        ),
    ] {
        let original = stdout
            .lines()
            .find_map(|line| line.strip_prefix("ORIGINAL|0|"))
            .unwrap();
        let expected: Vec<u8> = original
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let profile = tcl_vm::test_support::profile(engine);
        // The original public source needs the independently attached C
        // byte compiler; genuine Jim enters its original Script owner.
        // Neither capability supplies a scripted distribution library.
        let mut vm = crate::fixture::interpreter(profile);
        let completion = crate::fixture::eval(&mut vm, source);
        assert_eq!(
            completion.code,
            Code::Ok,
            "{engine}: {:?}",
            completion.result.string_bytes()
        );
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            expected.as_slice(),
            "{engine}"
        );
    }
}
