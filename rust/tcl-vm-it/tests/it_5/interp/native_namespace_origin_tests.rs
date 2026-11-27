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

//! Original public origin and helper-root reports from retained references.

fn compare(engine: &str, oracle: &[u8], input: &str, expected_rows: usize) {
    let profile = tcl_vm::test_support::profile(engine);
    let output = crate::fixture::Output::default();
    let mut vm = crate::fixture::distribution_with_output(profile, Box::new(output.clone()));
    let completion = vm.try_eval_source(input).unwrap_or_else(|error| {
        panic!(
            "{engine}: {error:?}; output before refusal:\n{}",
            String::from_utf8_lossy(&output.0.borrow())
        )
    });
    assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
    let actual = output.0.borrow();
    let (actual_identity, actual_rows) =
        actual.split_at(actual.iter().position(|byte| *byte == b'\n').unwrap() + 1);
    let (oracle_identity, oracle_rows) =
        oracle.split_at(oracle.iter().position(|byte| *byte == b'\n').unwrap() + 1);
    if engine == "jim" {
        assert_eq!(actual_identity, b"IDENTITY 0.84\n");
        assert_eq!(oracle_identity, b"IDENTITY 0.84-9-g5bac7c9\n");
    } else {
        assert_eq!(
            actual_identity, oracle_identity,
            "{engine}: provider reporting"
        );
    }
    assert_eq!(
        std::str::from_utf8(oracle_rows).unwrap().lines().count(),
        expected_rows
    );
    assert_eq!(
        actual_rows, oracle_rows,
        "{engine}: exact original origin/helper-root output, including alias prefix objects and replacement generations"
    );
    eprintln!(
        "{engine}:{expected_rows} original observation rows; reports never recover command identity"
    );
}

macro_rules! origins {
    ($test:ident, $engine:literal, $provider:literal) => {
        #[test]
        fn $test() {
            compare($engine, include_bytes!(concat!("../../../../../experiments/name-handling/namespace-origin-query/", $provider, "/stdout.bin")),
                include_str!("../../../../../experiments/name-handling/namespace-origin-query/input.tcl"),37);
            compare($engine, include_bytes!(concat!("../../../../../experiments/name-handling/namespace-origin-query/helper-root/", $provider, "/stdout.bin")),
                include_str!("../../../../../experiments/name-handling/namespace-origin-query/helper-root-input.tcl"),5);
        }
    };
}
origins!(original_namespace_origin_c84, "tcl8.4", "8.4.20");
origins!(original_namespace_origin_c85, "tcl8.5", "8.5.19");
origins!(original_namespace_origin_c86, "tcl8.6", "8.6.18");
origins!(original_namespace_origin_c90, "tcl9.0", "9.0.4");
origins!(original_namespace_origin_c91, "tcl9.1", "9.1.0");
origins!(original_namespace_origin_jim, "jim", "jim");
