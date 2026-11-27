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

//! Source list-length and concat rows against every captured provider.

use crate::prelude::*;

#[test]
fn original_source_length_and_concat_channels_match_all_native_providers() {
    // Native proof: naming.list.original-jim-source-length-conversion
    // docs/design/analysis/name-resolution-proofs/list-original-jim-source-length-conversion.md
    macro_rules! fixture {
        ($file:literal) => {
            include_bytes!(concat!(
                "../../../tcl-registry/tests/data/native_source_list_length241/",
                $file
            ))
        };
    }
    macro_rules! rows {
        ($provider:literal) => {
            include_str!(concat!(
                "../../../tcl-registry/tests/data/native_source_list_length241/",
                $provider,
                "/stdout"
            ))
        };
    }
    let sources: [&[u8]; 3] = [
        fixture!("original-0.tcl"),
        fixture!("original-1.tcl"),
        fixture!("original-2.tcl"),
    ];
    let mut compared = 0;
    for (engine, native) in [
        ("tcl8.4", rows!("tcl8.4")),
        ("tcl8.5", rows!("tcl8.5")),
        ("tcl8.6", rows!("tcl8.6")),
        ("tcl9.0", rows!("tcl9.0")),
        ("tcl9.1", rows!("tcl9.1")),
        ("jim", rows!("jim")),
    ] {
        // Original241 executes these three unchanged inputs in one selected
        // interpreter. C source compilation remains an independent supplied
        // capability; Jim uses its original Script engine at the public door.
        let profile = tcl_vm::test_support::profile(engine);
        let mut vm = crate::fixture::interpreter(profile);
        for (index, source) in sources.into_iter().enumerate() {
            let completion = vm.try_eval_source_bytes(source).unwrap();
            let label = format!("ORIGINAL_{index}");
            let row = native
                .lines()
                .find(|row| row.split('|').next() == Some(label.as_str()))
                .unwrap();
            let fields = row.split('|').collect::<Vec<_>>();
            assert_eq!(
                completion.code.as_int(),
                fields[1].parse::<i64>().unwrap(),
                "{engine}/{index}"
            );
            assert_eq!(
                vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                crate::fixture::unhex(fields[2]),
                "{engine}/{index}"
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 18);
}
