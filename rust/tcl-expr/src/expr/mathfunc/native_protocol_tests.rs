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

use super::*;

fn jim(name: &str, args: &[Num]) -> Result<Num, MathFuncError> {
    try_dispatch_with_backend_protocol(name, args, IntWidth::Windowed, NativeMathProtocol::Jim084)
}

#[test]
fn jim_integer_power_and_conversion_follow_measured_signed_wide_protocol() {
    for (args, expected) in [
        ([Num::Int(2), Num::Int(2)], 4),
        ([Num::Int(2), Num::Int(-1)], 0),
        ([Num::Int(-1), Num::Int(-3)], -1),
        ([Num::Int(2), Num::Int(63)], i64::MIN),
        ([Num::Int(2), Num::Int(64)], 0),
    ] {
        assert_eq!(jim("pow", &args), Ok(Num::Int(expected)));
    }
    assert_eq!(
        jim("pow", &[Num::Float(2.0), Num::Int(2)]),
        Ok(Num::Float(4.0))
    );
    assert_eq!(
        jim("pow", &[Num::Int(0), Num::Int(-1)]),
        Err(MathFuncError::JimZeroToNegativePower)
    );
    assert_eq!(jim("abs", &[Num::Int(i64::MIN)]), Ok(Num::Int(i64::MIN)));
    for (name, value, expected) in [
        ("round", 0.499_999_999_999_999_94, 1),
        ("round", 4_503_599_627_370_497.0, 4_503_599_627_370_498),
        ("int", 1e20, i64::MIN),
        ("round", 1e20, i64::MIN),
        ("int", f64::NAN, i64::MIN),
        ("int", f64::INFINITY, i64::MIN),
    ] {
        assert_eq!(jim(name, &[Num::Float(value)]), Ok(Num::Int(expected)));
    }
}

#[test]
fn jim_float_domain_results_do_not_acquire_c_math_rejections() {
    for (name, args) in [
        ("sqrt", vec![Num::Int(-1)]),
        ("acos", vec![Num::Int(2)]),
        ("fmod", vec![Num::Int(1), Num::Int(0)]),
    ] {
        assert!(matches!(jim(name, &args), Ok(Num::Float(value)) if value.is_nan()));
        assert_eq!(
            try_dispatch_with_backend_protocol(
                name,
                &args,
                IntWidth::Windowed,
                NativeMathProtocol::Tcl
            ),
            Err(MathFuncError::Domain)
        );
    }
    assert_eq!(
        jim("log", &[Num::Int(0)]),
        Ok(Num::Float(f64::NEG_INFINITY))
    );
    assert_eq!(
        try_dispatch_with_backend_protocol(
            "pow",
            &[Num::Int(2), Num::Int(2)],
            IntWidth::Windowed,
            NativeMathProtocol::Tcl
        ),
        Ok(Num::Float(4.0))
    );
    assert_eq!(
        jim("future_function", &[Num::Int(1)]),
        Err(MathFuncError::UnknownFunction)
    );
}
