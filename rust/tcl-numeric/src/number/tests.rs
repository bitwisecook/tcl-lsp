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

use std::fmt::Write as _;
#[test]
fn nonfinite_double_spelling_matches_every_native_engine() {
    use tcl_dialect::DoubleStringPolicy;
    let fixtures = [
        (
            DoubleStringPolicy::Tcl84Precision,
            include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/8.4.20.tsv"),
        ),
        (
            DoubleStringPolicy::TclPrecision,
            include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/8.5.19.tsv"),
        ),
        (
            DoubleStringPolicy::TclPrecision,
            include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/8.6.18.tsv"),
        ),
        (
            DoubleStringPolicy::Shortest,
            include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/9.0.4.tsv"),
        ),
        (
            DoubleStringPolicy::Shortest,
            include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/9.1.0.tsv"),
        ),
        (
            DoubleStringPolicy::JimTwelve,
            include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/jim.tsv"),
        ),
    ];
    let bits = [
        0x7ff8_0000_0000_0000,
        0xfff8_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0,
        0x8000_0000_0000_0000,
    ];
    let mut observations = 0;
    for (policy, fixture) in fixtures {
        for row in fixture.lines() {
            let (case, expected) = row.split_once('\t').unwrap();
            let value = f64::from_bits(bits[case.parse::<usize>().unwrap()]);
            let rendered = super::format_double_native_selected(
                value,
                policy,
                policy.format(policy.default_precision()).unwrap(),
            );
            let hex: String = rendered.bytes().fold(String::new(), |mut output, byte| {
                write!(output, "{byte:02x}").unwrap();
                output
            });
            assert_eq!(hex, expected, "{policy:?}/{case}");
            observations += 1;
        }
    }
    assert_eq!(observations, 36);
}

#[test]
fn scalar_numeric_bytes_use_actual_native_extent_before_unicode() {
    use tcl_dialect::model::{DialectPoint, Release};
    let original = vec![b'1', 0, 0xff];
    let jim = super::NativeScalarNumericInputPolicy::for_point(Some(DialectPoint::canonical(
        Release::JIM_0_84,
    )))
    .unwrap();
    assert!(std::str::from_utf8(&original).is_err());
    assert_eq!(jim.input_bytes(&original), b"1");
    assert_eq!(jim.input_bytes(b"1.5\0X"), b"1.5");
    assert_eq!(jim.input_bytes(b"\0\xff"), b"");
    assert_eq!(jim.input_bytes(b"\xff\0"), b"\xff");
    assert_eq!(original, b"1\0\xff");
    for release in [
        Release::TCL_8_4,
        Release::TCL_8_5,
        Release::TCL_8_6,
        Release::TCL_9_0,
        Release::TCL_9_1,
    ] {
        let c = super::NativeScalarNumericInputPolicy::for_point(Some(DialectPoint::canonical(
            release,
        )))
        .unwrap();
        assert_eq!(c.input_bytes(&original), original);
        assert_eq!(c.input_bytes(b"1\0X"), b"1\0X");
    }
    assert!(super::NativeScalarNumericInputPolicy::for_point(None).is_none());
    for release in [Release::F5_TCL_TMOS, Release::F5_IRULES_TMM] {
        assert!(
            super::NativeScalarNumericInputPolicy::for_point(Some(DialectPoint::canonical(
                release
            )))
            .is_none()
        );
    }
}

use super::*;

fn n(s: &str) -> Option<Number> {
    parse_whole(s)
}

#[test]
fn decimal_integers() {
    assert_eq!(n("0"), Some(Number::Int(0)));
    assert_eq!(n("42"), Some(Number::Int(42)));
    assert_eq!(n("-42"), Some(Number::Int(-42)));
    assert_eq!(n("+7"), Some(Number::Int(7)));
    // leading zero is DECIMAL in 9.0, not octal
    assert_eq!(n("0755"), Some(Number::Int(755)));
    assert_eq!(n("  13  "), Some(Number::Int(13))); // surrounding whitespace ok
}

#[test]
fn radix_prefixes() {
    assert_eq!(n("0xff"), Some(Number::Int(255)));
    assert_eq!(n("0XFF"), Some(Number::Int(255)));
    assert_eq!(n("0o17"), Some(Number::Int(15)));
    assert_eq!(n("0b1010"), Some(Number::Int(10)));
    assert_eq!(n("0d0755"), Some(Number::Int(755)));
    assert_eq!(n("-0x10"), Some(Number::Int(-16)));
    // a bare `0x` (no hex digit) is not a whole number
    assert_eq!(n("0x"), None);
}

#[test]
fn process_exit_integer_width_and_overflow_are_runtime_policies() {
    use tcl_dialect::ProcessExitConversion::{Narrow32, SaturatingWide, WholeInteger};
    let modern = Numbers::Target(NumberSyntax::Tcl90);
    assert_eq!(modern.parse_exit_status("4294967295", Narrow32), Some(-1));
    assert_eq!(modern.parse_exit_status("-4294967295", Narrow32), Some(1));
    assert_eq!(modern.parse_exit_status("4294967296", Narrow32), None);
    assert_eq!(
        modern.parse_exit_status("18446744073709551619", WholeInteger),
        Some(3)
    );
    assert_eq!(
        modern.parse_exit_status("-18446744073709551619", WholeInteger),
        Some(-3)
    );
    assert_eq!(
        modern.parse_exit_status("18446744073709551619", SaturatingWide),
        Some(-1)
    );
    assert_eq!(
        modern.parse_exit_status("-18446744073709551619", SaturatingWide),
        Some(0)
    );
    assert_eq!(modern.parse_exit_status("1.0", WholeInteger), None);
}

#[test]
fn underscores() {
    assert_eq!(n("1_000_000"), Some(Number::Int(1_000_000)));
    assert_eq!(n("1__000___000"), Some(Number::Int(1_000_000)));
    assert_eq!(n("0xff_ff"), Some(Number::Int(0xffff)));
    assert_eq!(n("0xff__ff"), Some(Number::Int(0xffff)));
    assert_eq!(n("0b1__0"), Some(Number::Int(2)));
    assert_eq!(n("0o7__7"), Some(Number::Int(63)));
    assert_eq!(n("1_0"), Some(Number::Int(10)));
    // misplaced underscores → not a whole number
    assert_eq!(n("_1"), None);
    assert_eq!(n("1_"), None);
    assert_eq!(n("1__"), None);
    assert_eq!(n("0x_f"), None);
}

#[test]
fn i64_bounds_and_bignum() {
    assert_eq!(n("9223372036854775807"), Some(Number::Int(i64::MAX)));
    assert_eq!(n("-9223372036854775808"), Some(Number::Int(i64::MIN)));
    // one past i64::MAX → Big
    assert_eq!(
        n("9223372036854775808"),
        Some(Number::Big {
            negative: false,
            radix: Radix::Dec,
            digits: "9223372036854775808".to_owned()
        })
    );
    // a clearly huge value → Big
    assert_eq!(
        n("123456789012345678901234567890"),
        Some(Number::Big {
            negative: false,
            radix: Radix::Dec,
            digits: "123456789012345678901234567890".to_owned()
        })
    );
    // huge hex with separators → Big with cleaned digits
    assert_eq!(
        n("0xffff_ffff_ffff_ffff_f"),
        Some(Number::Big {
            negative: false,
            radix: Radix::Hex,
            digits: "fffffffffffffffff".to_owned()
        })
    );
}

#[test]
fn floats() {
    assert_eq!(n("1.5"), Some(Number::Double(1.5)));
    assert_eq!(n(".5"), Some(Number::Double(0.5)));
    assert_eq!(n("1."), Some(Number::Double(1.0)));
    assert_eq!(n("-2.5e3"), Some(Number::Double(-2500.0)));
    assert_eq!(n("1E-3"), Some(Number::Double(0.001)));
    assert_eq!(n("1_0.5"), Some(Number::Double(10.5)));
    assert_eq!(n("1__0.5"), Some(Number::Double(10.5)));
    assert_eq!(n("1.0__2"), Some(Number::Double(1.02)));
    assert_eq!(n("1_.0"), None);
    assert_eq!(n("1._0"), None);
}

#[test]
fn inf_and_nan() {
    assert_eq!(n("Inf"), Some(Number::Double(f64::INFINITY)));
    assert_eq!(n("infinity"), Some(Number::Double(f64::INFINITY)));
    assert_eq!(n("-Inf"), Some(Number::Double(f64::NEG_INFINITY)));
    assert_eq!(
        n("NaN"),
        Some(Number::Nan {
            negative: false,
            payload: None
        })
    );
    assert_eq!(
        n("NaN(1ff)"),
        Some(Number::Nan {
            negative: false,
            payload: Some(0x1ff)
        })
    );
    assert_eq!(
        n("NaN( 1 2 3 )"),
        Some(Number::Nan {
            negative: false,
            payload: Some(0x123)
        })
    );
    assert_eq!(n("NaN(123456789abcde)"), None);
}

#[test]
fn integer_only_flag() {
    // integer_only stops at the `.` so a whole-parse of a float fails
    let f = ParseFlags {
        integer_only: true,
        ..Default::default()
    };
    let p = parse("12.5", f).unwrap();
    assert_eq!(p.number, Number::Int(12));
    assert_eq!(p.end, 2); // stopped before the `.`
}

#[test]
fn not_numbers() {
    assert_eq!(n(""), None);
    assert_eq!(n("   "), None);
    assert_eq!(n("abc"), None);
    assert_eq!(n("12x"), None);
    assert_eq!(n("0x1.5"), None); // no hex floats
    assert_eq!(n("1.2.3"), None);
    assert_eq!(n("--5"), None);
    assert_eq!(n("+-5"), None);
}

#[test]
fn partial_parse_reports_end() {
    let p = parse("42 rest", ParseFlags::default()).unwrap();
    assert_eq!(p.number, Number::Int(42));
    assert_eq!(p.end, 2);
}

#[test]
fn normalised_numeric_string_language_preserves_special_values() {
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        0.0,
        f64::MIN,
        f64::MAX,
        f64::MIN_POSITIVE,
    ] {
        for format in [
            tcl_dialect::DoubleFormat::Shortest,
            tcl_dialect::DoubleFormat::General(12),
            tcl_dialect::DoubleFormat::TclSignificant(17),
        ] {
            assert!(canonical_numeric_bytes_may_equal(
                format_double_selected(value, format).as_bytes()
            ));
        }
    }
    for value in [b"hello".as_slice(), b"true", b"", b" 7", b"\xff"] {
        assert!(!canonical_numeric_bytes_may_equal(value));
    }
    assert!(
        canonical_numeric_bytes_may_equal(b"not-a-number"),
        "the conservative language is not successful numeric parsing"
    );
}

#[test]
fn format_double_matches_tcl_print_double() {
    // Specials and signed zero (the `-0.0` sign must survive — regression).
    assert_eq!(format_double(f64::NAN), "NaN");
    assert_eq!(format_double(f64::INFINITY), "Inf");
    assert_eq!(format_double(f64::NEG_INFINITY), "-Inf");
    assert_eq!(format_double(0.0), "0.0");
    assert_eq!(format_double(-0.0), "-0.0");
    // Ordinary fixed-notation values keep the distinguishing `.0`.
    assert_eq!(format_double(2.0), "2.0");
    assert_eq!(format_double(1.5), "1.5");
    assert_eq!(format_double(0.5), "0.5");
    assert_eq!(format_double(0.0001), "0.0001");
    // Integer-valued doubles ≥ 1e16 keep `.0` (re-parse as Double, not Int):
    // tclsh renders these with the trailing `.0`, not as a bare integer.
    assert_eq!(format_double(1e16), "10000000000000000.0");
    assert_eq!(format_double(1.5e16), "15000000000000000.0");
    assert_eq!(format_double(-1e16), "-10000000000000000.0");
    // At 1e17 tclsh switches to scientific (`1e+17`, single-digit mantissa
    // stays bare, exponent has an explicit sign and natural width).
    assert_eq!(format_double(1e17), "1e+17");
    assert_eq!(format_double(-1e17), "-1e+17");
    assert_eq!(format_double(1e-5), "1e-5");
    assert_eq!(format_double(6.022e23), "6.022e+23");
}

/// Two shortest candidates both round-trip and sit equally far from the
/// double's exact value; `TclDoubleDigits` takes the even last digit.
/// tclsh 8.5.19–9.1.0 (and Python's `repr`) print the left column;
/// Rust's bare `{:e}` printed the odd neighbour (#2432).
#[test]
fn format_double_breaks_a_shortest_tie_to_even() {
    assert_eq!(format_double(1e15 + 0.3), "1000000000000000.2");
    assert_eq!(format_double(-1e15 - 0.3), "-1000000000000000.2");
    for (bits, tcl) in [
        (0x430c_6bf5_2634_016a_u64, "1000000000000045.2"),
        (0x430c_6bf5_2634_0732, "1000000000000230.2"),
        (0x42d6_bcc4_1e90_0508, "100000000000020.12"),
    ] {
        assert_eq!(format_double(f64::from_bits(bits)), tcl);
    }
    assert_eq!(format_double(0.1 + 0.2), "0.30000000000000004");
    assert_eq!(format_double(1e23), "1e+23");
    assert_eq!(format_double(5e-324), "5e-324");
}

#[test]
fn format_double_round_trips_to_double_not_int() {
    // The point of the `.0`: every `format_double` output must re-parse as a
    // `Double` (never an `Int`), so the numeric tower stays put across a
    // string round-trip. 1e16/1e17 are the regression cases.
    for &v in &[2.0, 1e16, 1.5e16, 1e17, -1e16, -1e17, 1e-5, 1.0, 0.0, -0.0] {
        let s = format_double(v);
        match parse_whole(&s) {
            Some(Number::Double(_)) => {}
            other => panic!("format_double({v}) = {s:?} re-parsed as {other:?}, not Double"),
        }
    }
}

/// Every normal power of two, against tclsh: the 516 spellings 8.5.19 to
/// 9.1.0 print that are not the shortest are tabled (byte-identical
/// across the four releases), and the other 1530 are the shortest
/// round-tripping digits.
#[test]
fn format_double_spells_every_power_of_two_as_tclsh_does() {
    let table: std::collections::HashMap<i32, &str> =
        include_str!("../../../tcl-syntax/tests/data/tcl_pow2_spellings.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .map(|line| {
                let (k, text) = line.split_once(' ').unwrap();
                (k.parse().unwrap(), text)
            })
            .collect();
    assert_eq!(table.len(), 516);
    let mut lossy = 0;
    for k in -1022..=1023 {
        let value = 2f64.powi(k);
        let text = format_double(value);
        if let Some(&tcl) = table.get(&k) {
            assert_eq!(text, tcl, "2^{k}");
            lossy += usize::from(text.parse::<f64>().unwrap().to_bits() != value.to_bits());
        } else {
            let shortest = shortest_nearest_scientific(value);
            let digits = |s: &str| -> String {
                s.split(['e', 'E'])
                    .next()
                    .unwrap()
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect::<String>()
                    .trim_matches('0')
                    .to_owned()
            };
            assert_eq!(digits(&text), digits(&shortest), "2^{k}: {text}");
            assert_eq!(
                text.parse::<f64>().unwrap().to_bits(),
                value.to_bits(),
                "2^{k}: {text}"
            );
        }
        assert_eq!(format_double(-value), format!("-{text}"));
    }
    assert_eq!(lossy, 263);
    // The pinned cases: 2^64 and 2^-530 do not read back, 2^-1016 is
    // longer than its shortest, 2^0 to 2^3 and 2^-1 are plain.
    assert_eq!(format_double(2f64.powi(64)), "1.844674407370955e+19");
    assert_eq!(format_double(2f64.powi(-530)), "2.845131199340899e-160");
    assert_eq!(format_double(2f64.powi(-1016)), "1.4240472694446089e-306");
    assert_eq!(format_double(2f64.powi(-24)), "5.960464477539062e-8");
    for (value, text) in [(1.0, "1.0"), (2.0, "2.0"), (8.0, "8.0"), (0.5, "0.5")] {
        assert_eq!(format_double(value), text);
    }
    // A subnormal power of two is not in the bw == 1 path.
    assert_eq!(format_double(f64::from_bits(1)), "5e-324");
    assert_eq!(
        format_double(f64::from_bits(1 << 51)),
        "1.1125369292536007e-308"
    );
}

/// The doubles beside a power of two keep the shortest spelling.
#[test]
fn format_double_neighbours_of_a_power_of_two_are_shortest() {
    for (value, text) in [
        (
            f64::from_bits(2f64.powi(64).to_bits() + 1),
            "1.8446744073709556e+19",
        ),
        (
            f64::from_bits(2f64.powi(64).to_bits() - 1),
            "1.844674407370955e+19",
        ),
        (3.0 * 2f64.powi(64), "5.5340232221128655e+19"),
        (0.1, "0.1"),
        (1.0 / 3.0, "0.3333333333333333"),
    ] {
        assert_eq!(format_double(value), text);
    }
}

/// `%.12g` and the `.0` rule, measured on tclsh 8.4.20 and jimsh 0.84
/// (identical on every row).
#[test]
fn precision_12_matches_tcl_8_4_and_jim() {
    for (value, text) in [
        (1.0 / 3.0, "0.333333333333"),
        (1e15 + 0.3, "1e+15"),
        (0.1 + 0.2, "0.3"),
        (1e-5, "1e-05"),
        (1.5e-7, "1.5e-07"),
        (-1e-5, "-1e-05"),
        (123_456_789_012.5, "123456789012.0"),
        (123_456_789_012.0, "123456789012.0"),
        (999_999_999_999.4, "999999999999.0"),
        (999_999_999_999.5, "1e+12"),
        (9.999_999_999_995e15, "1e+16"),
        (9_999_999_999_995.0, "1e+13"),
        (99_999_999_999.95, "99999999999.9"),
        (100_000_000_000.0, "100000000000.0"),
        (1e12, "1e+12"),
        (2.0, "2.0"),
        (100.0, "100.0"),
        (0.5, "0.5"),
        (-2.5, "-2.5"),
        (0.0001, "0.0001"),
        (0.000_099_999_999_999_995, "0.0001"),
        (0.000_123_456_789_012_34, "0.000123456789012"),
        (-5e-11, "-5e-11"),
        (1e16, "1e+16"),
        (1e17, "1e+17"),
        (1e22, "1e+22"),
        (1e100, "1e+100"),
        (1e301, "1e+301"),
        (1e-310, "1e-310"),
        (2.5e-308 / 2.0, "1.25e-308"),
        (4.611_686_018_427_388e18, "4.61168601843e+18"),
        (2f64.powi(64), "1.84467440737e+19"),
        (12_345_678_901_234_567.0, "1.23456789012e+16"),
        (core::f64::consts::SQRT_2, "1.41421356237"),
        (core::f64::consts::E, "2.71828182846"),
        (5e-324, "4.94065645841e-324"),
        (0.0, "0.0"),
        (-0.0, "-0.0"),
        (f64::INFINITY, "Inf"),
        (f64::NEG_INFINITY, "-Inf"),
        (f64::NAN, "NaN"),
    ] {
        assert_eq!(
            format_double_as(value, DoubleSpelling::Precision12),
            text,
            "{value:e}"
        );
    }
}

#[test]
fn spell_double_declines_a_spelling_that_does_not_read_back() {
    let tcl86 = NumberSpelling::of(NumberSyntax::Tcl85, Some(tcl_dialect::TclVersion::V8_6));
    let tcl84 = NumberSpelling::of(NumberSyntax::Tcl84, Some(tcl_dialect::TclVersion::V8_4));
    let jim = NumberSpelling::of(NumberSyntax::Jim080, None);
    let tcl90 = NumberSpelling::default();
    for spelling in [tcl86, tcl90] {
        assert_eq!(spell_double(2f64.powi(64), spelling), None);
        assert_eq!(
            spell_double(2f64.powi(63), spelling).as_deref(),
            Some("9.223372036854776e+18")
        );
        assert_eq!(
            spell_double(1.0 / 3.0, spelling).as_deref(),
            Some("0.3333333333333333")
        );
        assert_eq!(spell_double(1e-310, spelling).as_deref(), Some("1e-310"));
        assert_eq!(
            spell_double(f64::INFINITY, spelling).as_deref(),
            Some("Inf")
        );
        assert_eq!(spell_double(f64::NAN, spelling), None);
    }
    for spelling in [tcl84, jim] {
        assert_eq!(spell_double(1.0 / 3.0, spelling), None);
        assert_eq!(spell_double(0.1 + 0.2, spelling), None);
        assert_eq!(spell_double(1e-5, spelling).as_deref(), Some("1e-05"));
        assert_eq!(spell_double(2.0, spelling).as_deref(), Some("2.0"));
        assert_eq!(spell_double(1e16, spelling).as_deref(), Some("1e+16"));
        assert_eq!(spell_double(-0.0, spelling).as_deref(), Some("-0.0"));
    }
    // 8.4 prints a computed subnormal but cannot read one back.
    assert_eq!(spell_double(1e-310, tcl84), None);
    assert_eq!(spell_double(1e-310, jim).as_deref(), Some("1e-310"));
    // A program that may set `tcl_precision` has no known spelling
    // through 8.6; 9.0 and Jim have no such variable.
    assert_eq!(spell_double(2.0, tcl84.with_precision_written(true)), None);
    assert_eq!(spell_double(2.0, tcl86.with_precision_written(true)), None);
    assert_eq!(
        spell_double(2.0, tcl90.with_precision_written(true)).as_deref(),
        Some("2.0")
    );
    assert_eq!(
        spell_double(2.0, jim.with_precision_written(true)).as_deref(),
        Some("2.0")
    );
}

#[test]
fn native_integer_string_garbles_only_int64_min_where_the_release_does() {
    use super::native_integer_string;
    assert_eq!(native_integer_string(i64::MIN, true), "-8");
    assert_eq!(
        native_integer_string(i64::MIN, false),
        "-9223372036854775808"
    );
    assert_eq!(
        native_integer_string(i64::MIN + 1, true),
        "-9223372036854775807"
    );
    assert_eq!(native_integer_string(-8, true), "-8");
    assert_eq!(native_integer_string(i64::MAX, true), "9223372036854775807");
}

#[test]
fn spell_wide_declines_int64_min_where_the_release_garbles_it() {
    use tcl_dialect::TclVersion;
    let min = i64::MIN;
    for (spelling, garbled) in [
        (
            NumberSpelling::of(NumberSyntax::Tcl84, Some(TclVersion::V8_4)),
            true,
        ),
        (
            NumberSpelling::of(NumberSyntax::Tcl85, Some(TclVersion::V8_5)),
            true,
        ),
        (
            NumberSpelling::of(NumberSyntax::Tcl85, Some(TclVersion::V8_6)),
            false,
        ),
        (NumberSpelling::of(NumberSyntax::Tcl85, None), true),
        (
            NumberSpelling::of(NumberSyntax::Tcl90, Some(TclVersion::V9_0)),
            false,
        ),
        (NumberSpelling::of(NumberSyntax::Jim080, None), false),
        (NumberSpelling::default(), false),
    ] {
        let expected = (!garbled).then(|| "-9223372036854775808".to_owned());
        assert_eq!(spell_wide(min, spelling), expected, "{spelling:?}");
        assert_eq!(
            spell_wide(min + 1, spelling).as_deref(),
            Some("-9223372036854775807")
        );
    }
}

#[test]
fn compare_int_double_is_exact_past_2_pow_53() {
    use core::cmp::Ordering::{Equal, Greater, Less};
    // The motivating cases (tclsh oracle): a both-as-f64 comparison calls
    // 9007199254740993 equal to 9007199254740992.0; Tcl compares exactly.
    //   expr {9007199254740993 == 9007199254740992.0} → 0
    //   expr {9007199254740993 >  9007199254740992.0} → 1
    assert_eq!(
        compare_int_double(9_007_199_254_740_993, 9_007_199_254_740_992.0),
        Some(Greater)
    );
    // The float literal ...993.0 itself rounds to ...992.0, so the wide
    // still orders above it: expr {9007199254740993 > 9007199254740993.0} → 1.
    assert_eq!(
        compare_int_double(9_007_199_254_740_993, 9_007_199_254_740_993.0),
        Some(Greater)
    );
    assert_eq!(
        compare_int_double(9_007_199_254_740_992, 9_007_199_254_740_992.0),
        Some(Equal)
    );
    // 20000000000000003 < 20000000000000004.0 — the exact case the C
    // comment in TclCompareTwoNumbers cites.
    assert_eq!(
        compare_int_double(20_000_000_000_000_003, 20_000_000_000_000_004.0),
        Some(Less)
    );
}

#[test]
fn compare_int_double_small_and_fractional() {
    use core::cmp::Ordering::{Equal, Greater, Less};
    assert_eq!(compare_int_double(2, 2.5), Some(Less));
    assert_eq!(compare_int_double(3, 2.5), Some(Greater));
    assert_eq!(compare_int_double(-2, -2.5), Some(Greater));
    assert_eq!(compare_int_double(-3, -2.5), Some(Less));
    assert_eq!(compare_int_double(0, 0.0), Some(Equal));
    assert_eq!(compare_int_double(0, -0.0), Some(Equal));
    assert_eq!(compare_int_double(0, -0.5), Some(Greater));
    assert_eq!(compare_int_double(0, 0.5), Some(Less));
    assert_eq!(compare_int_double(5, 5.0), Some(Equal));
    assert_eq!(compare_int_double(-5, -5.0), Some(Equal));
    // Subnormals: integer part 0, fraction non-zero.
    assert_eq!(compare_int_double(0, f64::MIN_POSITIVE / 2.0), Some(Less));
    assert_eq!(
        compare_int_double(0, -f64::MIN_POSITIVE / 2.0),
        Some(Greater)
    );
}

#[test]
fn compare_int_double_extremes() {
    use core::cmp::Ordering::{Equal, Greater, Less};
    // ±2¹²⁷: i128::MAX = 2¹²⁷−1 sits below 2¹²⁷; i128::MIN is exactly
    // −2¹²⁷. 2⁶³ marks the i64-boundary sliver.
    const TWO_POW_127: f64 = 170_141_183_460_469_231_731_687_303_715_884_105_728.0;
    const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;
    // NaN is unordered — the caller applies Tcl's "!= true, rest false".
    assert_eq!(compare_int_double(1, f64::NAN), None);
    // Infinities order around every integer.
    assert_eq!(compare_int_double(i128::MAX, f64::INFINITY), Some(Less));
    assert_eq!(
        compare_int_double(i128::MIN, f64::NEG_INFINITY),
        Some(Greater)
    );
    assert_eq!(compare_int_double(i128::MAX, TWO_POW_127), Some(Less));
    assert_eq!(compare_int_double(i128::MIN, -TWO_POW_127), Some(Equal));
    assert_eq!(compare_int_double(0, -TWO_POW_127), Some(Greater));
    // 2⁶³ exactly vs the neighbouring wides (the i64-boundary sliver).
    assert_eq!(
        compare_int_double(i128::from(i64::MAX), TWO_POW_63),
        Some(Less)
    );
    assert_eq!(
        compare_int_double(i128::from(i64::MIN), -TWO_POW_63),
        Some(Equal)
    );
    // A wide whose f64 rounding lands ON 2⁶³ still orders exactly.
    assert_eq!(
        compare_int_double(9_223_372_036_854_775_807, 9.3e18),
        Some(Less)
    );
    assert_eq!(
        compare_int_double(-9_223_372_036_854_775_807, -9.3e18),
        Some(Greater)
    );
}
