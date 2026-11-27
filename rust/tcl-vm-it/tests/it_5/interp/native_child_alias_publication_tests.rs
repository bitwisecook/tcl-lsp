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

//! Child-alias publication from complete original sources, against the
//! tclsh and jimsh captures.

fn original_result(rows: &str) -> (i64, Vec<u8>) {
    let fields: Vec<_> = rows
        .lines()
        .find(|line| line.starts_with("ORIGINAL|"))
        .unwrap()
        .split('|')
        .collect();
    let hex = fields[2].as_bytes();
    assert_eq!(hex.len() % 2, 0);
    (
        fields[1].parse().unwrap(),
        hex.as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect(),
    )
}

type OriginalControl = (&'static str, &'static [u8], [&'static str; 6]);

const CONTROLS: &[OriginalControl] = &[
    ("original-rooted-child-alias", include_bytes!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/original-rooted-child-alias.tcl").as_slice(), [
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.4/original-rooted-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.5/original-rooted-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.6/original-rooted-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.0/original-rooted-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.1/original-rooted-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/jim/original-rooted-child-alias/stdout"),
    ]),
    ("original-extra-root-child-alias", include_bytes!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/original-extra-root-child-alias.tcl").as_slice(), [
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.4/original-extra-root-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.5/original-extra-root-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.6/original-extra-root-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.0/original-extra-root-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.1/original-extra-root-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/jim/original-extra-root-child-alias/stdout"),
    ]),
    ("original-nul-before-suffix-child-alias", include_bytes!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/original-nul-before-suffix-child-alias.tcl").as_slice(), [
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.4/original-nul-before-suffix-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.5/original-nul-before-suffix-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.6/original-nul-before-suffix-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.0/original-nul-before-suffix-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.1/original-nul-before-suffix-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/jim/original-nul-before-suffix-child-alias/stdout"),
    ]),
    ("original-nul-after-suffix-child-alias", include_bytes!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/original-nul-after-suffix-child-alias.tcl").as_slice(), [
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.4/original-nul-after-suffix-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.5/original-nul-after-suffix-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.6/original-nul-after-suffix-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.0/original-nul-after-suffix-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.1/original-nul-after-suffix-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/jim/original-nul-after-suffix-child-alias/stdout"),
    ]),
    ("original-unicode-child-alias", include_bytes!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/original-unicode-child-alias.tcl").as_slice(), [
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.4/original-unicode-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.5/original-unicode-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.6/original-unicode-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.0/original-unicode-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.1/original-unicode-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/jim/original-unicode-child-alias/stdout"),
    ]),
    ("original-ff-child-alias", include_bytes!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/original-ff-child-alias.tcl").as_slice(), [
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.4/original-ff-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.5/original-ff-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl8.6/original-ff-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.0/original-ff-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/tcl9.1/original-ff-child-alias/stdout"),
        include_str!("../../../../tcl-registry/tests/data/native_child_alias_inventory298/jim/original-ff-child-alias/stdout"),
    ]),
];

#[test]
fn original_child_alias_publication_matches_all_36_native_public_windows() {
    // naming.alias.original-child-publication-and-inventory
    // docs/design/analysis/name-resolution-proofs/alias-original-child-publication-and-inventory.md
    // Replay each complete original source without altering counted input.
    // Literal U+00E9 is UTF-8 source; NUL and FF are produced by binary format.
    // NAME_BEFORE, definition, invocation, command inventory, actual alias API,
    // and the caught opposite alias API remain independently captured fields
    // within the whole result. Jim child forwarding commands are not core
    // aliases. C inventory's encoding-convertto channel differs from Jim raw
    // bytes. Every inventory has one matching alias; no hash-order claim follows.
    // These observations confer no original header, parent lifetime, physical
    // key identity, cross-provider string encoding or Rust execution authority.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let mut comparisons = 0;
    for &(case, source, columns) in CONTROLS {
        for (engine, column) in providers.iter().zip(columns) {
            let (expected_code, expected_result) = original_result(column);
            // jimsh's `binary`, which these sources call, is a C extension
            // the VM installs only through a private registration.
            if *engine == "jim" {
                continue;
            }
            let profile = tcl_vm::test_support::profile(engine);
            let mut vm = crate::fixture::interpreter(profile);
            let completion = crate::fixture::eval(&mut vm, std::str::from_utf8(source).unwrap());
            assert_eq!(
                completion.code.as_int(),
                expected_code,
                "{engine}/{case}: {completion:?}"
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                expected_result.as_slice(),
                "{engine}/{case}"
            );
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 30);
}
