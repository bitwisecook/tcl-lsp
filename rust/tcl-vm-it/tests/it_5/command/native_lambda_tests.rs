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

//! Original lambda conversion diagnostics, independent of successful activation.

use crate::prelude::*;

fn row<'a>(rows: &'a str, label: &str) -> Vec<&'a str> {
    rows.lines()
        .find(|row| row.split('|').next() == Some(label))
        .unwrap()
        .split('|')
        .collect()
}

#[test]
fn lambda_conversion_errors_match_original_counted_values_and_parameter_frames() {
    // Native proof: naming.lambda.original-conversion-diagnostic
    // docs/design/analysis/name-resolution-proofs/lambda-original-conversion-diagnostic.md
    // Native proof: naming.lambda.original-parameter-error-frame
    // docs/design/analysis/name-resolution-proofs/lambda-original-parameter-error-frame.md
    for (engine, captured) in [
        (
            "tcl8.5",
            include_str!(
                "../../../../tcl-registry/tests/data/native_lambda_diagnostics/v2/tcl8.5/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../tcl-registry/tests/data/native_lambda_diagnostics/v2/tcl8.6/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../tcl-registry/tests/data/native_lambda_diagnostics/v2/tcl9.0/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../tcl-registry/tests/data/native_lambda_diagnostics/v2/tcl9.1/stdout.tsv"
            ),
        ),
        (
            "jimtcl",
            include_str!(
                "../../../../tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/stdout.tsv"
            ),
        ),
    ] {
        for input in captured.lines().filter(|line| {
            line.starts_with("DIRECT_") && line.split('|').next().unwrap().ends_with("_INPUT")
        }) {
            let fields: Vec<_> = input.split('|').collect();
            let label = fields[0].strip_suffix("_INPUT").unwrap();
            let native = row(captured, label);
            if native[1] == "0" {
                continue;
            }
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
            let lambda = Value::new_native_string_bytes(crate::fixture::unhex(fields[2]));
            let completion = crate::fixture::call(&mut vm, "apply", &[lambda]);
            assert_eq!(
                completion.code.as_int(),
                native[1].parse::<i64>().unwrap(),
                "{engine}/{label}"
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                crate::fixture::unhex(native[2]).as_slice(),
                "{engine}/{label}"
            );
            if engine != "jimtcl" {
                let expected = row(captured, &format!("{label}_ERRORCODE"));
                assert_eq!(
                    crate::fixture::option(&completion.options, "-errorcode")
                        .unwrap()
                        .string_bytes()
                        .as_ref(),
                    crate::fixture::unhex(expected[2]).as_slice(),
                    "{engine}/{label}/errorcode"
                );
                if label == "DIRECT_PARAM_BODY_RAW_ZERO" {
                    let frame = b"\n    (parsing lambda expression \"{{a b c}} {x\")";
                    let info =
                        crate::fixture::call(&mut vm, "set", &[Value::string("::errorInfo")])
                            .result;
                    let actual = info.string_bytes();
                    assert!(
                        actual.windows(frame.len()).any(|part| part == frame),
                        "{engine}/CString parameter frame"
                    );
                }
            }
        }
    }
}
