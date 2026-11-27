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

mod tests {
    use tcl_runtime_api::{Code, Completion};
    use tcl_syntax::value::ValueOps;
    use tcl_vm::{Vm, value::Value};
    const CASES: &[(&str, &[u8], &[u8])] = &[
        ("PLAIN", b"k", b"k"),
        ("RAW_ZERO", b"k\0tail", b"k\0tail"),
        ("QUALIFIED_AFTER_ZERO", b"k\0::tail", b"k\0::tail"),
        ("SURROGATE", b"k\xed\xa0\x80", b"k\xed\xa0\x80"),
        ("OPAQUE_FF", b"k\xff", b"k\xff"),
        ("PREFIX_DECLARATION_NEGATIVE", b"k", b"k\0tail"),
    ];

    fn call(vm: &mut Vm, words: &[&[u8]]) -> Completion<Value> {
        let arguments: Vec<_> = words
            .iter()
            .map(|word| Value::new_native_string_bytes(*word))
            .collect();
        vm.try_invoke_command(std::str::from_utf8(words[0]).unwrap(), &arguments[1..])
            .expect("no host refusal")
    }

    fn assert_row(vm: &mut Vm, result: &Completion<Value>, rows: &str, label: &str) {
        let protocol = tcl_native_model::InvocationDialect::of_static_profile(vm.dialect_profile())
            .native_string_protocol()
            .unwrap();
        let fields: Vec<_> = rows
            .lines()
            .find(|row| row.starts_with(&format!("{label}|")))
            .unwrap()
            .split('|')
            .collect();
        let expected: Vec<_> = fields[2]
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(
            result.code.as_int(),
            fields[1].parse::<i64>().unwrap(),
            "{label}: {result:?}"
        );
        let actual = ValueOps::native_string_bytes(vm, &result.result).unwrap();
        let observed = if label.ends_with("_OBJECT_VARS_AFTER_STATIC")
            || label.ends_with("_OBJECT_VARS_AFTER_DYNAMIC")
        {
            protocol.legacy_eval_result_input(&actual).unwrap()
        } else {
            &actual
        };
        assert_eq!(observed, expected, "{label}");
    }

    #[test]
    fn compiled_and_dynamic_declared_variables_match_original_native_controls() {
        // Native proof: naming.interpreter.legacy-eval-object-result-boundary
        // docs/design/analysis/name-resolution-proofs/legacy-eval-object-result-boundary.md
        // Native proof: naming.tcloo.resident-variable-public-api-counted-keys
        // docs/design/analysis/name-resolution-proofs/tcloo-resident-variable-public-api-counted-keys.md
        // Native proof: naming.tcloo.declared-variable-compiled-and-runtime-resolver
        // docs/design/analysis/name-resolution-proofs/declared-variable-compiled-and-runtime-resolver.md
        for (engine, rows, counted_rows) in [
            (
                "tcl8.6",
                include_str!(
                    "../../../../tcl-vm/tests/data/native_oo_variable_resolver/tcl8.6/stdout.tsv"
                ),
                include_str!(
                    "../../../../tcl-vm/tests/data/native_oo_resident_keys/v5/tcl8.6/stdout.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../tcl-vm/tests/data/native_oo_variable_resolver/tcl9.0/stdout.tsv"
                ),
                include_str!(
                    "../../../../tcl-vm/tests/data/native_oo_resident_keys/v5/tcl9.0/stdout.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../tcl-vm/tests/data/native_oo_variable_resolver/tcl9.1/stdout.tsv"
                ),
                include_str!(
                    "../../../../tcl-vm/tests/data/native_oo_resident_keys/v5/tcl9.1/stdout.tsv"
                ),
            ),
        ] {
            for &(label, declaration, primary) in CASES {
                let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
                assert_eq!(
                    vm.try_eval_source_bytes(b"oo::class create C {};C create c")
                        .unwrap()
                        .code,
                    Code::Ok
                );
                let declared = call(&mut vm, &[b"oo::define", b"C", b"variable", declaration]);
                assert_row(&mut vm, &declared, rows, &format!("{label}_DECLARE"));
                let writer = [
                    b"set {".as_slice(),
                    primary,
                    b"} STATIC;return [list [info locals] [info vars]]",
                ]
                .concat();
                let reader = [b"return ${".as_slice(), primary, b"}"].concat();
                for (name, parameters, body) in [
                    (b"writer".as_slice(), b"".as_slice(), writer.as_slice()),
                    (b"reader", b"", reader.as_slice()),
                    (
                        b"dynamic",
                        b"target",
                        b"set $target DYNAMIC;return [list [info locals] [info vars]]".as_slice(),
                    ),
                    (b"dynamicread", b"target", b"set $target".as_slice()),
                ] {
                    assert_eq!(
                        call(
                            &mut vm,
                            &[b"oo::define", b"C", b"method", name, parameters, body]
                        )
                        .code,
                        Code::Ok
                    );
                }
                for (suffix, words) in [
                    ("STATIC_WRITE", vec![b"c".as_slice(), b"writer"]),
                    ("STATIC_READ", vec![b"c".as_slice(), b"reader"]),
                    (
                        "OBJECT_VARS_AFTER_STATIC",
                        vec![b"info".as_slice(), b"object", b"vars", b"c"],
                    ),
                    ("DYNAMIC_WRITE", vec![b"c".as_slice(), b"dynamic", primary]),
                    (
                        "STATIC_READ_AFTER_DYNAMIC",
                        vec![b"c".as_slice(), b"reader"],
                    ),
                    (
                        "DYNAMIC_READ",
                        vec![b"c".as_slice(), b"dynamicread", primary],
                    ),
                    (
                        "OBJECT_VARS_AFTER_DYNAMIC",
                        vec![b"info".as_slice(), b"object", b"vars", b"c"],
                    ),
                ] {
                    let result = call(&mut vm, &words);
                    assert_row(&mut vm, &result, rows, &format!("{label}_{suffix}"));
                    if suffix == "OBJECT_VARS_AFTER_STATIC" {
                        assert_row(
                            &mut vm,
                            &result,
                            counted_rows,
                            &format!("{label}_OBJECT_VARS_AFTER_STATIC_OBJECT_VECTOR"),
                        );
                    }
                }
            }
        }
    }
}
