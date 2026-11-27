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

//! Behaviour tests moved from `tcl-vm`'s `cmd_mathop::tests`.

use crate::prelude::*;

#[test]
fn original_jim_namespace_info_guard_retains_its_original_headers() {
    // naming.mathop.original-jim-source-and-helper-context
    // docs/design/analysis/name-resolution-proofs/mathop-original-jim-source-and-helper-context.md
    // This compares the unchanged original guarded Jim source and public
    // completion. Optional header-access diagnostics supply no provider evidence.
    let source = include_str!(
        "../../../../tcl-registry/tests/data/native_mathop_jim_original_source/inputs/identity/0/source.tcl"
    );
    let observed = include_str!(
        "../../../../tcl-registry/tests/data/native_mathop_jim_original_source/providers/jim/cases/identity/0/execute.stdout"
    );
    let fields: Vec<_> = observed
        .lines()
        .find(|line| line.starts_with("CASE_RESULT "))
        .unwrap()
        .split_whitespace()
        .collect();
    let expected: Vec<_> = fields[2]
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    let profile = tcl_vm::test_support::profile("jim");
    let mut vm = crate::fixture::distribution(profile);
    let completion = vm
        .eval_source(source)
        .expect("original Jim guard completion");
    assert_eq!(completion.code.as_int(), fields[1].parse::<i64>().unwrap());
    assert_eq!(
        vm.native_string_bytes(&completion.result).unwrap().as_ref(),
        expected,
    );
}

#[test]
fn selected_mathop_identity_matches_all_60_native_name_and_argv_controls() {
    // naming.mathop.original-jim-source-and-helper-context
    // docs/design/analysis/name-resolution-proofs/mathop-original-jim-source-and-helper-context.md
    // The pinned current Jim distribution reconfirms these ten unchanged
    // Jim public code/result pairs with its actual NamespaceInfo library.
    // It does not establish the different old CLI's library roster.
    let rows = include_str!("../../../../tcl-cmd-core/tests/data/native_mathop_identity/rows.txt");
    let decode = |text: &str| {
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };
    let mut count = 0;
    for row in rows.lines() {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        let profile = tcl_vm::test_support::profile(fields[0]);
        let mut vm = if fields[0] == "jim" {
            crate::fixture::distribution(profile)
        } else {
            crate::fixture::interpreter(profile)
        };
        let source = String::from_utf8(decode(fields[4])).unwrap();
        let completion = vm.eval_source(&source).unwrap_or_else(|error| {
            panic!(
                "{}/{} native mathop source: {error:?}",
                fields[0], fields[1]
            )
        });
        assert_eq!(
            completion.code.as_int(),
            fields[2].parse::<i64>().unwrap(),
            "{}/{} original result: {:?}",
            fields[0],
            fields[1],
            completion.result.resident_string_bytes()
        );
        assert_eq!(
            vm.native_string_bytes(&completion.result).unwrap().as_ref(),
            decode(fields[3]),
            "{}/{}",
            fields[0],
            fields[1]
        );
        count += 1;
    }
    assert_eq!(count, 60);
}

#[test]
fn original_mathop_compilation_matches_all_84_native_controls() {
    // naming.mathop.original-jim-source-and-helper-context
    // docs/design/analysis/name-resolution-proofs/mathop-original-jim-source-and-helper-context.md
    // Current Jim independently reconfirms these fourteen unchanged guarded
    // script completions with NamespaceInfo. The unavailable mathop branch
    // does not observe original Jim specialised compilation or operators.
    let rows =
        include_str!("../../../../tcl-cmd-core/tests/data/native_mathop_compilation/rows.txt");
    let decode = |text: &str| {
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };
    let mut count = 0;
    for row in rows.lines() {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        let profile = tcl_vm::test_support::profile(fields[0]);
        let mut vm = if fields[0] == "jim" {
            crate::fixture::distribution(profile)
        } else {
            crate::fixture::interpreter(profile)
        };
        let source = String::from_utf8(decode(fields[4])).unwrap();
        let completion = vm.eval_source(&source).unwrap_or_else(|error| {
            panic!(
                "{}/{} native mathop source: {error:?}",
                fields[0], fields[1]
            )
        });
        assert_eq!(
            completion.code.as_int(),
            fields[2].parse::<i64>().unwrap(),
            "{}/{} original result: {:?}",
            fields[0],
            fields[1],
            completion.result.resident_string_bytes()
        );
        assert_eq!(
            vm.native_string_bytes(&completion.result).unwrap().as_ref(),
            decode(fields[3]),
            "{}/{}",
            fields[0],
            fields[1]
        );
        count += 1;
    }
    assert_eq!(count, 84);
}
