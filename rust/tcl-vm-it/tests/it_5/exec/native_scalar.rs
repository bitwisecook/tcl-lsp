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
    use tcl_syntax::value::ValueOps;
    use tcl_vm::Value;

    include!("../../../../tcl-registry/tests/data/native_scalar_compilation/cases.rs");

    const SCALAR_COMPILATION_FIXTURES: [(&str, &str); 5] = [
        (
            "tcl8.4",
            include_str!(
                "../../../../tcl-registry/tests/data/native_scalar_compilation/8.4.20.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../tcl-registry/tests/data/native_scalar_compilation/8.5.19.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../tcl-registry/tests/data/native_scalar_compilation/8.6.18.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!("../../../../tcl-registry/tests/data/native_scalar_compilation/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../tcl-registry/tests/data/native_scalar_compilation/9.1.0.tsv"),
        ),
    ];

    /// The 41 string and list scalar rows: completion code and result bytes
    /// as the pinned releases report them.
    #[test]
    fn compiled_scalars_preserve_forty_one_native_original_header_windows() {
        let mut windows = 0;
        for (engine, table) in SCALAR_COMPILATION_FIXTURES {
            let mut vm =
                crate::fixture::interpreter(tcl_dialect::DialectProfile::find(engine).unwrap());
            for row in table.lines() {
                let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
                if !fields[9]
                    .split(',')
                    .any(|op| matches!(op, "streq" | "strlen" | "listlength" | "listLength"))
                {
                    continue;
                }
                let case = fields[0].parse::<usize>().unwrap();
                vm.try_eval_source(&format!("proc p {{left right}} {{{}}}", CASES[case]))
                    .unwrap();
                let left = if matches!(case, 7..=9) {
                    b"a b".as_slice()
                } else {
                    b"A\0x".as_slice()
                };
                let arguments = [
                    Value::new_native_string_bytes(left),
                    Value::new_native_string_bytes(b"A\0y".as_slice()),
                ];
                let completion = crate::fixture::call(&mut vm, "p", &arguments);
                assert_eq!(
                    completion.code.as_int().to_string(),
                    fields[1],
                    "{engine}/{case}"
                );
                assert_eq!(
                    vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                    crate::fixture::unhex(fields[8]),
                    "{engine}/{case}"
                );
                windows += 1;
            }
        }
        assert_eq!(windows, 41);
    }
}
