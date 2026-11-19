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

use super::{
    Number, NumberSyntax, Numbers, ParseFlags, is_expr_number, is_out_of_range_double, parse,
    parse_whole_with, runtime_syntax, set_runtime_syntax,
};
use tcl_dialect::TclVersion;

fn whole(s: &str, syntax: NumberSyntax) -> Option<Number> {
    parse_whole_with(s, ParseFlags::for_syntax(syntax))
}

/// A bare leading zero is octal up to 8.6 and decimal from 9.0
/// (`changes.md`: "`0NNN` format is no longer octal interpretation").
#[test]
fn leading_zero_is_octal_before_9_0() {
    for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
        assert_eq!(whole("0755", syntax), Some(Number::Int(493)), "{syntax:?}");
        assert_eq!(whole("010", syntax), Some(Number::Int(8)), "{syntax:?}");
    }
    assert_eq!(whole("0755", NumberSyntax::Tcl90), Some(Number::Int(755)));
    assert_eq!(whole("010", NumberSyntax::Tcl90), Some(Number::Int(10)));
    // `0` alone, and a zero run, are the same in every release.
    for syntax in [
        NumberSyntax::Tcl84,
        NumberSyntax::Tcl85,
        NumberSyntax::Tcl90,
    ] {
        assert_eq!(whole("0", syntax), Some(Number::Int(0)), "{syntax:?}");
        assert_eq!(whole("00", syntax), Some(Number::Int(0)), "{syntax:?}");
    }
}

/// Under the octal rule an invalid octal digit stops the scan, so `08` is a
/// *partial* parse (`0`, with `8` trailing) — `parse_whole` therefore
/// rejects it, which is how C reports `08` as a bad octal number. From 9.0
/// it is plain decimal 8.
#[test]
fn invalid_octal_digit_stops_the_scan_before_9_0() {
    for bad in ["08", "09"] {
        assert_eq!(whole(bad, NumberSyntax::Tcl85), None, "{bad}");
        let partial = parse(bad, ParseFlags::for_syntax(NumberSyntax::Tcl85)).unwrap();
        assert_eq!(partial.number, Number::Int(0));
        assert_eq!(partial.end, 1, "scan stops at the invalid octal digit");
    }
    assert_eq!(whole("08", NumberSyntax::Tcl90), Some(Number::Int(8)));
    assert_eq!(whole("09", NumberSyntax::Tcl90), Some(Number::Int(9)));
}

/// A digit run followed by `.` or an exponent is a decimal float in every
/// release — C backtracks out of its octal state rather than reading `07.5`
/// as octal.
#[test]
fn leading_zero_floats_stay_decimal() {
    for syntax in [
        NumberSyntax::Tcl84,
        NumberSyntax::Tcl85,
        NumberSyntax::Tcl90,
    ] {
        assert_eq!(
            whole("0.5", syntax),
            Some(Number::Double(0.5)),
            "{syntax:?}"
        );
        assert_eq!(
            whole("07.5", syntax),
            Some(Number::Double(7.5)),
            "{syntax:?}"
        );
        assert_eq!(
            whole("08.5", syntax),
            Some(Number::Double(8.5)),
            "{syntax:?}"
        );
        assert_eq!(
            whole("09e1", syntax),
            Some(Number::Double(90.0)),
            "{syntax:?}"
        );
    }
}

/// `0x` is universal; `0b`/`0o` arrive in 8.5; `0d` in 9.0. An unavailable
/// prefix is not a prefix, so the whole-string parse fails (the trailing
/// letters are junk) — which is what makes such a word a bareword in an
/// expression.
#[test]
fn radix_prefixes_appear_in_the_release_that_added_them() {
    for syntax in [
        NumberSyntax::Tcl84,
        NumberSyntax::Tcl85,
        NumberSyntax::Tcl90,
    ] {
        assert_eq!(whole("0x1f", syntax), Some(Number::Int(31)), "{syntax:?}");
    }
    // 0b / 0o: absent in 8.4, present from 8.5.
    assert_eq!(whole("0o17", NumberSyntax::Tcl84), None);
    assert_eq!(whole("0b101", NumberSyntax::Tcl84), None);
    for syntax in [NumberSyntax::Tcl85, NumberSyntax::Tcl90] {
        assert_eq!(whole("0o17", syntax), Some(Number::Int(15)), "{syntax:?}");
        assert_eq!(whole("0b101", syntax), Some(Number::Int(5)), "{syntax:?}");
    }
    // 0d: 9.0 only.
    for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
        assert_eq!(whole("0d99", syntax), None, "{syntax:?}");
    }
    assert_eq!(whole("0d99", NumberSyntax::Tcl90), Some(Number::Int(99)));
}

/// A radix-invalid digit never makes the whole string a number, in any
/// release — the basis of C's `invalid bareword "0o8"` report.
#[test]
fn radix_invalid_digits_are_never_whole_numbers() {
    for syntax in [
        NumberSyntax::Tcl84,
        NumberSyntax::Tcl85,
        NumberSyntax::Tcl90,
    ] {
        for bad in ["0o8", "0o9", "0b2", "0b3", "0x", "0xg", "0x_", "0xZZZ"] {
            assert_eq!(whole(bad, syntax), None, "{bad} under {syntax:?}");
        }
    }
}

/// `_` digit separators are 9.0+, and the `no_underscore` flag bit still
/// overrides even there.
#[test]
fn digit_separators_are_9_0_only() {
    assert_eq!(
        whole("1__000", NumberSyntax::Tcl90),
        Some(Number::Int(1000))
    );
    assert_eq!(
        whole("0xff__ff", NumberSyntax::Tcl90),
        Some(Number::Int(65535))
    );
    for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
        assert_eq!(whole("1__000", syntax), None, "{syntax:?}");
    }
    let no_sep = ParseFlags {
        no_underscore: true,
        syntax: NumberSyntax::Tcl90,
        ..ParseFlags::default()
    };
    assert_eq!(parse_whole_with("1__000", no_sep), None);
}

/// Expression classification consumes the lower shared boundary scanner
/// before it asks this module's value parser. These paired rows fail if the
/// lexer and syntax parser ever start using different junction rules.
#[test]
fn expression_number_validation_shares_the_boundary_owner() {
    assert!(is_expr_number(
        "1.0_2",
        NumberSyntax::Tcl90,
        Some(TclVersion::V9_0)
    ));
    assert!(!is_expr_number(
        "1.0_2",
        NumberSyntax::Tcl85,
        Some(TclVersion::V8_6)
    ));
    assert!(!is_expr_number(
        "1_eq",
        NumberSyntax::Tcl90,
        Some(TclVersion::V9_0)
    ));
    // The value parser consumes the hexadecimal prefix, while the shared
    // lower boundary scanner restarts at `ne`/`in`/`ge` rather than using
    // the preceding alpha hex digit as a suffix boundary.
    for (source, syntax, base, end) in [
        ("0xfne 1", NumberSyntax::Tcl85, Some(TclVersion::V8_6), 3),
        (
            "0xffin {255}",
            NumberSyntax::Tcl85,
            Some(TclVersion::V8_6),
            4,
        ),
        ("0xfge 15", NumberSyntax::Tcl90, Some(TclVersion::V9_0), 3),
    ] {
        assert_eq!(
            tcl_dialect::scan_expr_number(source.as_bytes(), 0, syntax, base,)
                .unwrap()
                .end(),
            end,
            "{source}"
        );
        assert_eq!(
            parse(source, ParseFlags::for_syntax(syntax)).unwrap().end,
            end,
            "{source}"
        );
    }
    assert!(is_expr_number(
        "NaN(1)",
        NumberSyntax::Tcl85,
        Some(TclVersion::V8_6)
    ));
    assert!(is_expr_number(
        "NaN( 1 2 3 )",
        NumberSyntax::Tcl90,
        Some(TclVersion::V9_0)
    ));
    assert!(!is_expr_number(
        "NaN(123456789abcde)",
        NumberSyntax::Tcl90,
        Some(TclVersion::V9_0)
    ));
    assert!(!is_expr_number(
        "NaN(1)x",
        NumberSyntax::Tcl90,
        Some(TclVersion::V9_0)
    ));
    assert!(!is_expr_number(
        "NaN(1)",
        NumberSyntax::Tcl84,
        Some(TclVersion::V8_4)
    ));
    assert!(!is_expr_number(
        "Infinityeq",
        NumberSyntax::Tcl90,
        Some(TclVersion::V9_0)
    ));
}

/// The three ways a consumer can stand in relation to the release, and what
/// each answers for a spelling the releases read differently.
#[test]
fn numbers_variants_answer_their_own_use_case() {
    // A named target reads under exactly that release.
    assert_eq!(
        Numbers::Target(NumberSyntax::Tcl85).parse_wide("010"),
        Some(8)
    );
    assert_eq!(
        Numbers::Target(NumberSyntax::Tcl90).parse_wide("010"),
        Some(10)
    );
    // No release in hand: the readings disagree, so no answer.
    assert_eq!(Numbers::Unknown.parse_wide("010"), None);
    // ...but a spelling every release agrees on still resolves.
    assert_eq!(Numbers::Unknown.parse_wide("007"), Some(7));
    assert_eq!(Numbers::Unknown.parse_wide("0x1f"), Some(31));
    // The runtime variant follows whatever was installed.
    let restore = runtime_syntax();
    set_runtime_syntax(NumberSyntax::Tcl85);
    assert_eq!(Numbers::Runtime.parse_wide("010"), Some(8));
    set_runtime_syntax(NumberSyntax::Tcl90);
    assert_eq!(Numbers::Runtime.parse_wide("010"), Some(10));
    set_runtime_syntax(restore);
}

/// The two abstention directions are genuinely different, and a consumer
/// picking the wrong one is the silent failure this API exists to prevent.
#[test]
fn unknown_abstains_in_both_directions() {
    // "Is this provably a number?" — `08` is one only from 9.0, so no.
    assert!(!Numbers::Unknown.is_whole_number("08"));
    // "Could this still be a number?" — yes, on 9.0.
    assert!(Numbers::Unknown.is_number_in_any_release("08"));
    // Something no release reads as a number answers false both ways.
    assert!(!Numbers::Unknown.is_whole_number("0o8"));
    assert!(!Numbers::Unknown.is_number_in_any_release("0o8"));
    // And something every release reads answers true both ways.
    assert!(Numbers::Unknown.is_whole_number("42"));
    assert!(Numbers::Unknown.is_number_in_any_release("42"));
}

/// `of_profile` / `of_dialect_name` are the only sanctioned ways to get from
/// a dialect to a grammar, so they must agree with the profile catalogue.
#[test]
fn resolvers_agree_with_the_profile_catalogue() {
    assert_eq!(
        NumberSyntax::of_dialect_name(Some("tcl8.6")),
        NumberSyntax::Tcl85
    );
    assert_eq!(
        NumberSyntax::of_dialect_name(Some("tcl9.0")),
        NumberSyntax::Tcl90
    );
    // No dialect named falls to the permissive default, not to 8.x.
    assert_eq!(NumberSyntax::of_dialect_name(None), NumberSyntax::default());
    assert_eq!(NumberSyntax::of_profile(None), NumberSyntax::default());
}

/// The ambient's `const` initialiser cannot call `Default`, so it names the
/// variant directly — two places asserting one fact. This pins them.
#[test]
fn numbers_default_matches_the_ambient_initialiser() {
    // A fresh thread has never had `set_runtime_syntax` called on it, so it
    // still holds the initialiser's value.
    let ambient = std::thread::spawn(runtime_syntax).join().unwrap();
    assert_eq!(ambient, NumberSyntax::default());
}

/// The `strtod` `ERANGE` set, measured on tclsh 8.4.20 and jimsh 0.84
/// (glibc): an overflow, and a non-zero mantissa that comes out zero or
/// subnormal. The boundary values around `DBL_MAX` / `DBL_MIN` are normal.
#[test]
fn out_of_range_is_the_strtod_erange_set() {
    let syntax = NumberSyntax::Tcl90;
    for range_error in [
        "1e400",
        "-1e400",
        "+1e400",
        "1E400",
        ".1e400",
        "1.e400",
        "100e398",
        "1.7976931348623159e308",
        "1.8e308",
        "1e-400",
        "-1e-400",
        "1e-325",
        "4.9e-324",
        "5e-324",
        "1e-320",
        "1e-310",
        "1e-308",
        "2.2250738585072009e-308",
        "1e999999999999",
        "0.5e-323",
        "123456789012345678901234567890e-340",
    ] {
        assert!(is_out_of_range_double(range_error, syntax), "{range_error}");
    }
    for in_range in [
        "1.7976931348623157e308",
        "1e308",
        "2.2250738585072014e-308",
        "2.2250738585072013e-308",
        "1e-307",
        "1e19",
        "0.0",
        "0e999",
        "0.0e-400",
        "00e999",
        "0e-999",
        "-0.0",
        "Inf",
        "-Infinity",
        "NaN",
        "1e5",
        "42",
        "1e400x",
        "abc",
    ] {
        assert!(
            !is_out_of_range_double(in_range, syntax),
            "{in_range} is in range"
        );
    }
    assert!(is_out_of_range_double(" 1e400 ", syntax));
}

#[test]
fn range_error_is_a_release_fact_not_a_parse_failure() {
    for syntax in NumberSyntax::ALL {
        let parsed = parse("1e400", ParseFlags::for_syntax(*syntax)).unwrap();
        assert_eq!(parsed.number, Number::Double(f64::INFINITY));
        assert!(parsed.out_of_range);
        assert_eq!(
            parsed.is_range_error(*syntax),
            syntax.string_is_double_rejects_range_error()
        );
        let inf = parse("Inf", ParseFlags::for_syntax(*syntax)).unwrap();
        assert!(!inf.is_range_error(*syntax));
    }
}
