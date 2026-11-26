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

use crate::prelude::*;
use tcl_syntax::value::ValueOps;

fn bytes_from_hex(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

// Native proof: naming.variable.namespace-upvar-target-and-empty-pair-grammar
// docs/design/analysis/name-resolution-proofs/variable.namespace-upvar-target-and-empty-pair-grammar.md
#[test]
fn namespace_upvar_grammar_and_target_cells_match_36_native_results() {
    const CASES: &str =
        include_str!("../../../../tcl-vm/tests/data/native_namespace_upvar/cases.tsv");
    let engines = [
        (
            "tcl8.4",
            include_str!("../../../../tcl-vm/tests/data/native_namespace_upvar/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../tcl-vm/tests/data/native_namespace_upvar/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../tcl-vm/tests/data/native_namespace_upvar/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../tcl-vm/tests/data/native_namespace_upvar/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../tcl-vm/tests/data/native_namespace_upvar/9.1.0.tsv"),
        ),
        (
            "jim",
            include_str!("../../../../tcl-vm/tests/data/native_namespace_upvar/Jim.tsv"),
        ),
    ];
    let mut compared = 0;
    for (engine, expected) in engines {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        assert_eq!(CASES.lines().count(), expected.lines().count());
        for (input, expected) in CASES.lines().zip(expected.lines()) {
            let (name, source) = tcl_test_support::fixture::tsv_split_once(input).unwrap();
            let (expected_name, remainder) =
                tcl_test_support::fixture::tsv_split_once(expected).unwrap();
            assert_eq!(expected_name, name);
            let (code, result) = tcl_test_support::fixture::tsv_split_once(remainder).unwrap();
            let code: i64 = code.parse().unwrap();
            let result = bytes_from_hex(result);
            let mut vm = crate::fixture::distribution(profile);
            let completion = vm
                .try_eval_source_bytes(&bytes_from_hex(source))
                .unwrap_or_else(|error| panic!("{engine}/{name}: {error:?}"));
            assert_eq!(completion.code.as_int(), code, "{engine}/{name}");
            let actual = ValueOps::native_string_bytes(&mut vm, &completion.result).unwrap();
            assert_eq!(actual.as_ref(), result.as_slice(), "{engine}/{name}");
            compared += 1;
        }
    }
    assert_eq!(compared, 36);
}
#[test]
fn namespace_upvar_original_vectors_match_eighteen_native_arity_windows() {
    let rows = include_str!(
        "../../../../tcl-registry/tests/data/native_namespace_upvar_arguments/rows.tsv"
    );
    let mut compared = 0;
    for row in rows.lines() {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        let profile = tcl_registry::model::ingress::resolve_environment(fields[0]).unit_profile();
        let mut vm = crate::fixture::distribution(profile);
        assert_eq!(
            vm.eval_source("namespace eval n {}").unwrap().code,
            Code::Ok
        );
        let mut args = vec![Value::string("upvar"), Value::string("n")];
        match fields[1] {
            "0" => {}
            "1" => args.push(Value::string("a")),
            "2" => args.extend([Value::string("a"), Value::string("b")]),
            other => panic!("unknown original case {other}"),
        }
        let result = vm
            .try_invoke_command("namespace", &args)
            .unwrap_or_else(|refusal| panic!("{refusal:?}"));
        assert_eq!(
            result.code.as_int(),
            fields[2].parse::<i64>().unwrap(),
            "{row}"
        );
        assert_eq!(
            ValueOps::native_string_bytes(&mut vm, &result.result)
                .unwrap()
                .as_ref(),
            bytes_from_hex(fields[3]).as_slice(),
            "{row}"
        );
        compared += 1;
    }
    assert_eq!(compared, 18);
}

#[test]
fn jim_namespace_upvar_forwards_root_worker_without_changing_variable_frame() {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut vm = crate::fixture::distribution(profile);
    let source = include_str!(
        "../../../../tcl-registry/tests/data/native_namespace_upvar_arguments/jim-forwarding.tcl"
    );
    let completion = vm.eval_source(source).unwrap();
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(
        ValueOps::native_string_bytes(&mut vm, &completion.result)
            .unwrap()
            .as_ref(),
        b"VALUE {FORWARD 0 ::missing::a {}}"
    );
}
