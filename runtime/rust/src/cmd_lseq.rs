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

//! Native numeric sequence arguments and list result construction.
#![cfg(have_tommath)]

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};
use tcl_cmd_core::lseq::{self, LseqError};

/// Register `lseq`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"lseq", lseq);
}

fn lseq(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let actual = interp.eval_frame_dialect();
    let native = actual.native_string_protocol().filter(|protocol| {
        protocol
            .tcl_version()
            .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
    });
    let decoded = if let Some(protocol) = native {
        lseq::decode_original(argv.len() - 1, |index, numeric_allowed| {
            let argument = argv[index + 1];
            let number = if numeric_allowed {
                match crate::typed_value::native_number_probe(
                    argument,
                    actual,
                    tcl_syntax::scalar_getter::NativeNumberGetterKind::Number,
                ) {
                    Ok(Ok(number)) => Some(number),
                    Ok(Err(_)) => None,
                    Err(error) => return Err(LseqError::Command(error.into())),
                }
            } else {
                None
            };
            // Cached Int arguments keep their absent resident string. Double
            // precision and keyword/diagnostic stages request original bytes.
            let bytes = if number.is_some() && !obj::has_string_rep(argument) {
                Vec::new()
            } else {
                crate::dict::native_object_bytes(argument, protocol)
                    .map_err(|error| LseqError::Command(error.into()))?
            };
            let parsed = number
                .as_ref()
                .and_then(|number| lseq::number_argument(number, &bytes));
            let bytes = if parsed.is_none() {
                let end = bytes
                    .iter()
                    .position(|&byte| byte == 0)
                    .unwrap_or(bytes.len());
                bytes[..end].to_vec()
            } else {
                bytes
            };
            Ok((parsed, bytes))
        })
    } else {
        let args = argv[1..]
            .iter()
            .map(|&argument| obj_bytes(argument))
            .collect::<Vec<_>>();
        let refs = args.iter().map(Vec::as_slice).collect::<Vec<_>>();
        lseq::decode(&refs)
    };
    let mut plan = match decoded {
        Ok(plan) => plan,
        Err(LseqError::WrongArguments) => {
            return interp.wrong_args_for_invocation(argv, b"n ??op? n ??by? n??");
        }
        Err(LseqError::Command(error)) => return interp.report_cmd_error(error),
    };
    if let Some(protocol) = native {
        if let Err(error) = plan.prepare_double_precision(|index| {
            crate::dict::native_object_bytes(argv[index + 1], protocol).map_err(Into::into)
        }) {
            return interp.report_cmd_error(error);
        }
        let series = match lseq::prepare_series(&plan) {
            Ok(series) => series,
            Err(error) => return interp.report_cmd_error(error),
        };
        let value = match series {
            Some(series) => match crate::native_arithseries::new_series(series, protocol) {
                Ok(value) => value,
                Err(error) => return interp.report_cmd_error(error.into()),
            },
            None => crate::obj::new_obj(),
        };
        interp.set_result(value);
        return Code::Ok;
    }
    match lseq::generate(interp, &plan) {
        Ok(value) => {
            interp.set_result(value);
            Code::Ok
        }
        Err(error) => interp.report_cmd_error(error),
    }
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    /// Evaluate `src` leak-checked, asserting success, and return the result.
    fn ok(src: &[u8]) -> Vec<u8> {
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
            code = i.eval_str(src);
            bytes = i.result_bytes();
        }
        assert_eq!(counters::finalize(), 0, "leak");
        assert_eq!(counters::double_free_count(), 0);
        assert_eq!(
            code,
            Code::Ok,
            "result={:?}",
            String::from_utf8_lossy(&bytes)
        );
        bytes
    }

    fn err(src: &[u8]) -> Vec<u8> {
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
            code = i.eval_str(src);
            bytes = i.result_bytes();
        }
        assert_eq!(counters::finalize(), 0, "leak");
        assert_eq!(code, Code::Error);
        bytes
    }

    #[test]
    fn lseq_integer_forms() {
        assert_eq!(ok(b"lseq 5"), b"0 1 2 3 4");
        assert_eq!(ok(b"lseq 0"), b"");
        assert_eq!(ok(b"lseq -5"), b""); // negative count → empty
        assert_eq!(ok(b"lseq 1 .. 10"), b"1 2 3 4 5 6 7 8 9 10");
        assert_eq!(ok(b"lseq 10 .. 1"), b"10 9 8 7 6 5 4 3 2 1");
        assert_eq!(ok(b"lseq 1 to 10 by 2"), b"1 3 5 7 9");
        assert_eq!(ok(b"lseq 1 to 10 by -2"), b""); // wrong-sign step → empty
        assert_eq!(ok(b"lseq 1 to 5 by 0"), b"1"); // omitted count defaults to one
        assert_eq!(ok(b"lseq 5 count 5"), b"5 6 7 8 9");
        assert_eq!(ok(b"lseq 5 count 5 by -2"), b"5 3 1 -1 -3");
        assert_eq!(ok(b"lseq 3 by 2"), b"0 2 4");
        assert_eq!(ok(b"lseq 1 5"), b"1 2 3 4 5");
        assert_eq!(ok(b"lseq 5 1"), b"5 4 3 2 1");
    }

    #[test]
    fn lseq_double_precision() {
        // Precision matches the inputs' fractional digits (`maxObjPrecision`).
        assert_eq!(ok(b"lseq 0 0.5 by 0.1"), b"0.0 0.1 0.2 0.3 0.4 0.5");
        assert_eq!(ok(b"lseq 25. to 5. by -5"), b"25.0 20.0 15.0 10.0 5.0");
        assert_eq!(
            ok(b"lseq 3.5 18.5 1.5"),
            b"3.5 5.0 6.5 8.0 9.5 11.0 12.5 14.0 15.5 17.0 18.5"
        );
        // A double-valued count is used as an integer and stays an int sequence.
        assert_eq!(ok(b"lseq 5 count 5.0"), b"5 6 7 8 9");
    }

    #[test]
    fn lseq_numeric_arguments_do_not_evaluate_expressions() {
        assert_eq!(err(b"lseq 1+2 to 10"), b"expected number but got \"1+2\"");
        assert_eq!(
            ok(b"set changed BEFORE; catch {lseq {[set changed 3]}}; set changed"),
            b"BEFORE"
        );
        assert_eq!(ok(b"lseq 1 t 3"), b"1 2 3");
        assert_eq!(ok(b"lseq 1 c 3 by 0"), b"1 1 1");
        assert_eq!(ok(b"lseq 1.5 to 3 by 0"), b"1.5");
    }

    #[test]
    fn lseq_errors() {
        assert_eq!(
            err(b"lseq"),
            b"wrong # args: should be \"lseq n ??op? n ??by? n??\""
        );
        assert_eq!(
            err(b"lseq 12 to 24 by 2 count"),
            b"wrong # args: should be \"lseq n ??op? n ??by? n??\""
        );
    }

    #[test]
    fn lseq_materialization_refusal_bypasses_guest_capture_and_finally() {
        use tcl_syntax::raw_string::{NativeMaterializationLimitError, NativeValueAccessRefusal};
        for script in [
            b"set prior BEFORE; catch {join [lseq 100000001]} captured options; set after YES"
                .as_slice(),
            b"set prior BEFORE; try {join [lseq 100000001]} finally {set final YES}; set after YES",
        ] {
            counters::reset();
            {
                let mut interp = Interp::new();
                interp.set_runtime_version(tcl_dialect::TclVersion::V9_0);
                assert_eq!(interp.eval_str(script), Code::Error);
                assert_eq!(
                    interp.native_access_refusal(),
                    Some(NativeValueAccessRefusal::Materialization(
                        NativeMaterializationLimitError::new(100_000_001, 100_000_000)
                    ))
                );
                assert_eq!(
                    crate::interp::obj_bytes(interp.var_get(b"prior").unwrap()),
                    b"BEFORE"
                );
                for name in [b"captured".as_slice(), b"options", b"final", b"after"] {
                    assert!(!interp.var_exists(name));
                }
            }
            assert_eq!(counters::finalize(), 0, "leak");
            assert_eq!(counters::double_free_count(), 0);
        }
    }
}
