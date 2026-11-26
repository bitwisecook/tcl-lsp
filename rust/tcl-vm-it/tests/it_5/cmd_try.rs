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

mod native_fixture_tests {
    use tcl_syntax::value::ValueOps;

    fn bytes_from_hex(hex: &str) -> Vec<u8> {
        assert_eq!(hex.len() % 2, 0);
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let text = core::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }

    #[test]
    fn try_grammar_binding_and_finally_match_fixed_native_results() {
        const CASES: &str = include_str!("../../../tcl-vm/tests/data/native_try/cases.tsv");
        let engines = [
            (
                "tcl8.6",
                include_str!("../../../tcl-vm/tests/data/native_try/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../../tcl-vm/tests/data/native_try/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../../tcl-vm/tests/data/native_try/9.1.0.tsv"),
            ),
            (
                "jim",
                include_str!("../../../tcl-vm/tests/data/native_try/Jim.tsv"),
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
                let source = bytes_from_hex(source);
                // Retain genuine core/bootstrap and compiler owners for this native expectation.
                let mut vm = crate::fixture::interpreter(profile);
                let completion = vm
                    .try_eval_source_bytes(&source)
                    .unwrap_or_else(|error| panic!("{engine}/{name}: {error:?}"));
                assert_eq!(completion.code.as_int(), code, "{engine}/{name}");
                let actual = ValueOps::native_string_bytes(&mut vm, &completion.result).unwrap();
                assert_eq!(actual.as_ref(), result.as_slice(), "{engine}/{name}");
                compared += 1;
            }
        }
        assert_eq!(compared, 60);
    }

    #[test]
    fn original_try_source_without_actual_core_keeps_native_preflight_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // An authored profile/compiler is not a genuine native-core constructor.
        let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
        let mut vm = tcl_vm::test_support::native_vm();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        let source = include_str!("../../../tcl-vm/tests/data/native_try/cases.tsv")
            .lines()
            .find_map(|row| row.strip_prefix("malformed-unmatched-vars\t"))
            .unwrap();
        let source = bytes_from_hex(source);
        // Without a genuine core the source must be refused before it runs:
        // at compile admission, or, now that `format` compiles without
        // native preflight, when `format` requests its numeric input.
        assert!(matches!(
            vm.try_eval_source_bytes(&source),
            Err(tcl_runtime_api::NativeExecutionError::CompilationAdmission(
                tcl_runtime_api::NativeCompilationAdmissionError::NativePreflightRequired
            ) | tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable
            ))
        ));
    }
}
