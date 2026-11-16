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

use super::IndexReading::{At, Bad, HostLong};
use super::*;

/// `lsort -index <step> {{a b} {c d}}` against each release (tclsh 8.4.20,
/// 8.5.19, 8.6.18, 9.0.4, 9.1.0): a step before the first element names
/// the word from 9.0, the decoded index in 8.5/8.6 and 8.4's `end` = -2
/// encoding; a step past the end is the number everywhere.
#[test]
fn a_failed_step_is_named_as_each_release_names_it() {
    let rows: &[(&str, [&str; 5])] = &[
        ("end-5", ["-7", "-4", "-4", "end-5", "end-5"]),
        ("end-3", ["-5", "-2", "-2", "end-3", "end-3"]),
        ("7", ["7", "7", "7", "7", "7"]),
    ];
    let releases = [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ];
    for (spec, names) in rows {
        let step = PathEncoding::of(spec).expect("encodes");
        let decoded = step.decode(2);
        for (release, name) in releases.iter().zip(names) {
            assert_eq!(
                failed_step_name(step, decoded, *release),
                *name,
                "{spec} {release:?}"
            );
        }
    }
}

/// `TclCompileStringReplaceCmd` (8.6.18) settles the command on its encoded
/// indices without a replacement string, for a no-op it proves, for
/// `first` before the first element, and for `last` after the end with
/// `first` relative to `end`; otherwise `INST_STR_REPLACE` reads the words
/// at run time. Measured in a proc on `abcdefghijkl`: `string replace $s
/// end+2147483647 end X` is unchanged (the run-time reading replaces it
/// all), and so is `string replace $s 0 end+2147483647 X`, as at run time.
#[test]
fn string_replace_settles_on_its_encoded_indices_where_8_6_does() {
    use super::Encoded::{After, Before, FromEnd, Index};
    for (first, last, replacing, encoded) in [
        (After, FromEnd(0), true, true),
        (Index(0), After, true, false),
        (Index(0), After, false, true),
        (Before, After, true, true),
        (FromEnd(-1), After, true, true),
        (Index(2), Before, true, true),
        (Index(2), FromEnd(-1), true, false),
    ] {
        assert_eq!(
            string_replace_compiles_encoded(first, last, replacing),
            encoded,
            "{first:?} {last:?} replacing: {replacing}"
        );
    }
}

/// Each command's index words as 8.6's compile procedure encodes them, or
/// leaves them to the command. Measured on tclsh 8.6.18 in a proc with
/// `$l` `{a b c}` and `$i` 1: `lreplace $l end-2147483649 $i X` is
/// `X c` (read as it runs), `string range abcdef end-2147483649 $i` is
/// empty (a `first` after the end settles it), and `lrange $l 0
/// end+2147483647`, `lindex $l end-2147483649` and `linsert $l
/// end-2147483649 X` read their literal after the end.
#[test]
fn each_compile_procedure_encodes_its_own_index_words() {
    let words =
        |compiler, words: &[Option<&str>]| compiled_index_words(compiler, words, TclVersion::V8_6);
    let past = Some("end-2147483649");
    let after = Some("2147483647");
    assert_eq!(words(IndexCompiler::Lindex, &[past]), [after]);
    assert_eq!(words(IndexCompiler::Linsert, &[past]), [after]);
    assert_eq!(
        words(IndexCompiler::Lrange, &[Some("0"), Some("end+2147483647")]),
        [None, after]
    );
    assert_eq!(words(IndexCompiler::Lrange, &[past, None]), [None, None]);
    assert_eq!(words(IndexCompiler::Lreplace, &[past, None]), [None, None]);
    assert_eq!(
        words(IndexCompiler::Lreplace, &[Some("2"), past]),
        [None, after]
    );
    assert_eq!(
        words(IndexCompiler::StringRange, &[past, None]),
        [after, None]
    );
    assert_eq!(
        words(
            IndexCompiler::StringReplace { replacing: true },
            &[Some("0"), Some("end+2147483647")]
        ),
        [None, None]
    );
    assert_eq!(
        words(
            IndexCompiler::StringReplace { replacing: false },
            &[Some("0"), Some("end+2147483647")]
        ),
        [None, after]
    );
    for release in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V9_0] {
        assert_eq!(
            compiled_index_words(IndexCompiler::Lindex, &[past], release),
            [None]
        );
    }
}

#[test]
fn string_replace_settles_as_the_compiler_does() {
    use StringReplaceSettlement as S;
    let at = |index| Some(Encoded::Index(index));
    let end = Some(Encoded::FromEnd(0));
    assert_eq!(settle_string_replace(at(5), at(2), true), S::Unchanged);
    assert_eq!(settle_string_replace(at(0), at(2), true), S::Runtime);
    assert_eq!(
        settle_string_replace(Some(Encoded::Before), Some(Encoded::After), true),
        S::Replacement
    );
    assert_eq!(settle_string_replace(at(0), end, false), S::Empty);
    assert_eq!(settle_string_replace(at(1), end, false), S::Ranges);
    assert_eq!(settle_string_replace(None, end, false), S::Runtime);
}

#[test]
fn jim_literal_index_expressions_preserve_end_encoding() {
    let syntax = tcl_dialect::IndexSyntax {
        numbers: tcl_dialect::NumberSyntax::Jim080,
        grammar: tcl_dialect::IndexGrammar::Jim,
        width: tcl_dialect::IndexIntegerWidth::Jim32,
        end_abbreviations: false,
    };
    for (source, expected) in [
        ("end--1", i64::from(i32::MAX)),
        ("end+-1", 3),
        ("(1+2)*1", 3),
        ("1 ? 2 : 1/0", 2),
        ("--1", 1),
    ] {
        assert_eq!(
            resolve_opt_in(source, 5, syntax),
            Some(expected),
            "{source}"
        );
    }
    for source in [
        "$x",
        "1 ? 2 : $x",
        "[set x]",
        "abs(-2)",
        "1/0",
        "1 #ignored",
    ] {
        assert_eq!(resolve_opt_in(source, 5, syntax), None, "{source}");
    }
}

#[test]
fn resolve_integer_and_end_forms() {
    // Mirrors Tcl's `TclGetIntForIndex` grammar (end / signed integer /
    // base[+-]integer).
    assert_eq!(resolve("5", 10).unwrap(), 5);
    assert_eq!(resolve("0", 10).unwrap(), 0);
    assert_eq!(resolve("-1", 10).unwrap(), -1);
    assert_eq!(resolve("end", 10).unwrap(), 9);
    assert_eq!(resolve("end-2", 10).unwrap(), 7);
    assert_eq!(resolve("end+1", 10).unwrap(), 10);
    // `end--1` parses as `end - (-1)`.
    assert_eq!(resolve("end--1", 10).unwrap(), 10);
    assert_eq!(resolve("1+1", 10).unwrap(), 2);
    assert_eq!(resolve("0-1", 10).unwrap(), -1);
    // `end` of an empty container is -1.
    assert_eq!(resolve("end", 0).unwrap(), -1);
}

#[test]
fn resolve_rejects_garbage_and_resolve_opt_returns_none() {
    assert!(resolve("bad", 10).is_err());
    assert!(resolve("", 10).is_err());
    assert!(resolve("1.5", 10).is_err());
    assert_eq!(resolve_opt("bad", 10), None);
    assert_eq!(resolve_opt("end-1", 10), Some(8));
}

#[test]
fn resolve_matches_tclsh_oracle() {
    // Ground truth captured from real tclsh 8.6 and 9.0 (identical) via
    // `string index`/`lindex` over a length-5 container (end = 4). Each
    // `resolve_opt` returns the raw resolved index (callers clamp for
    // out-of-range); a `None` is a `bad index` error at runtime.
    let ok: &[(&str, i64)] = &[
        // integers, signs, whitespace
        ("0", 0),
        ("1", 1),
        ("4", 4),
        ("5", 5),
        ("-1", -1),
        ("-2", -2),
        ("-0", 0),
        (" 1", 1),
        ("1 ", 1),
        // end and end offsets (`end--1` = end - (-1) = end + 1)
        ("end", 4),
        ("end-1", 3),
        ("end+1", 5),
        ("end-4", 0),
        ("end-5", -1),
        ("end--1", 5),
        ("end+-1", 3),
        ("end+0", 4),
        // arithmetic operands
        ("1+1", 2),
        ("0-1", -1),
        ("2+2", 4),
        ("3-1", 2),
        ("end-2", 2),
        // radix prefixes (hex / octal / binary / 0d) — oracle accepts these
        ("0x2", 2),
        ("0xa", 10),
        ("0xf", 15),
        ("0o7", 7),
        ("0b101", 5),
        ("0d5", 5),
        ("+0x2", 2),
        ("-0x1", -1),
        // radices inside the arithmetic operands
        ("end-0x2", 2),
        ("end-0o3", 1),
        ("0x2+1", 3),
        ("1+0x1", 2),
    ];
    for &(spec, want) in ok {
        assert_eq!(resolve_opt(spec, 5), Some(want), "resolve_opt({spec:?})");
    }
    // Syntactically bad — a `bad index` error at runtime.
    for spec in ["1.0", "1+", "end-", "foo", "", "1 1", "0x", "0xg", "2e0"] {
        assert_eq!(
            resolve_opt(spec, 5),
            None,
            "resolve_opt({spec:?}) should be None"
        );
    }
}

#[test]
fn encodable_classifies_specs() {
    // `TclIndexEncode` scale (used by `lsearch`/`lsort -index` parse-time
    // validation): Some(true) = encodable, Some(false) = out of range,
    // None = syntactically bad.
    assert_eq!(encodable("5"), Some(true)); // absolute, non-negative
    assert_eq!(encodable("0"), Some(true));
    assert_eq!(encodable("-1"), Some(false)); // absolute negative → out of range
    assert_eq!(encodable("end"), Some(true)); // offset 0 ≤ 0
    assert_eq!(encodable("end-2"), Some(true)); // offset -2 ≤ 0
    assert_eq!(encodable("end+1"), Some(false)); // offset +1 > 0 → out of range
    assert_eq!(encodable("bad"), None); // syntactically bad
}

/// An index word is read by `Tcl_GetIntForIndex`, so it inherits every
/// version difference in the numeral grammar. Pinned on tclsh 8.6.16 /
/// 9.0.4 against a 12-element list:
///
/// ```text
/// lindex $l 010      -> i (index 8, octal)  | k (index 10, decimal)
/// lindex $l end-010  -> d (end-8)           | b (end-10)
/// lindex $l 1_0      -> bad index           | k
/// lindex $l 0d1      -> bad index           | b
/// ```
#[test]
fn index_numerals_follow_the_named_release() {
    use tcl_syntax::number::NumberSyntax::{Tcl85, Tcl90};
    // (spec, len, 8.6 answer, 9.0 answer)
    let matrix = [
        ("010", 12, Some(8), Some(10)),
        ("end-010", 12, Some(3), Some(1)),
        ("1_0", 12, None, Some(10)),
        ("0d1", 12, None, Some(1)),
        // Unanimous spellings resolve the same either way.
        ("1", 12, Some(1), Some(1)),
        ("0x1", 12, Some(1), Some(1)),
        ("007", 12, Some(7), Some(7)),
        ("end", 12, Some(11), Some(11)),
        ("end-2", 12, Some(9), Some(9)),
    ];
    for (spec, len, on_86, on_90) in matrix {
        assert_eq!(
            resolve_opt_with(spec, len, Tcl85),
            on_86,
            "index {spec} on 8.6"
        );
        assert_eq!(
            resolve_opt_with(spec, len, Tcl90),
            on_90,
            "index {spec} on 9.0"
        );
    }
}

/// The rows past 32 bits of [`each_release_reads_an_index_as_its_tclsh_does`]:
/// (spec, 8.4, 8.5, 8.6, 9.0 and 9.1).
const PAST_32_BITS: &[(&str, [IndexReading; 4])] = &[
    (
        "2147483647",
        [
            At(2_147_483_647),
            At(2_147_483_647),
            At(2_147_483_647),
            At(2_147_483_647),
        ],
    ),
    (
        "2147483648",
        [
            At(-2_147_483_648),
            At(-2_147_483_648),
            At(-2_147_483_648),
            At(2_147_483_648),
        ],
    ),
    ("4294967295", [At(-1), At(-1), At(-1), At(4_294_967_295)]),
    ("0xffffffff", [At(-1), At(-1), At(-1), At(4_294_967_295)]),
    ("4294967296", [Bad, Bad, Bad, At(4_294_967_296)]),
    ("0x100000000", [Bad, Bad, Bad, At(4_294_967_296)]),
    (
        "-2147483649",
        [
            At(2_147_483_647),
            At(2_147_483_647),
            At(2_147_483_647),
            At(-2_147_483_649),
        ],
    ),
    ("-4294967295", [At(1), At(1), At(1), At(-4_294_967_295)]),
    ("-4294967296", [Bad, Bad, Bad, At(-4_294_967_296)]),
    (
        "end-2147483648",
        [
            At(-2_147_483_637),
            At(-2_147_483_637),
            At(-2_147_483_637),
            At(-2_147_483_637),
        ],
    ),
    (
        "end-4294967295",
        [At(12), At(12), At(12), At(-4_294_967_284)],
    ),
    ("end-4294967296", [Bad, Bad, Bad, At(-4_294_967_285)]),
    (
        "end+2147483647",
        [
            Bad,
            At(-2_147_483_638),
            At(-2_147_483_638),
            At(2_147_483_658),
        ],
    ),
    ("end+4294967295", [Bad, At(10), At(10), At(4_294_967_306)]),
    (
        "1+2147483647",
        [
            Bad,
            At(-2_147_483_648),
            At(-2_147_483_648),
            At(2_147_483_648),
        ],
    ),
    ("4294967295+1", [Bad, At(0), At(0), At(4_294_967_296)]),
    (
        "-2147483648-1",
        [
            Bad,
            At(2_147_483_647),
            At(2_147_483_647),
            At(-2_147_483_649),
        ],
    ),
    ("9223372036854775807", [Bad, Bad, Bad, At(i64::MAX)]),
    ("9223372036854775808", [Bad, Bad, Bad, At(i64::MAX)]),
    ("-9223372036854775809", [Bad, Bad, Bad, At(i64::MIN)]),
    ("0x8000000000000000", [Bad, Bad, Bad, At(i64::MAX)]),
    ("0x1ffffffffffffffff", [Bad, Bad, Bad, At(i64::MAX)]),
    ("1+9223372036854775807", [Bad, Bad, Bad, At(12)]),
    (
        "9223372036854775807+9223372036854775807",
        [Bad, Bad, Bad, At(12)],
    ),
    ("-9223372036854775808-1", [Bad, Bad, Bad, At(i64::MIN)]),
    ("end+9223372036854775807", [Bad, Bad, Bad, At(i64::MAX)]),
    ("end+9223372036854775808", [Bad, Bad, Bad, At(12)]),
    ("end+99999999999999999999", [Bad, Bad, Bad, At(12)]),
    ("end-9223372036854775808", [Bad, Bad, Bad, At(-1)]),
    (
        "18446744073709551615",
        [HostLong(-1), HostLong(-1), Bad, At(i64::MAX)],
    ),
    (
        "-0xffffffffffffffff",
        [HostLong(1), HostLong(1), Bad, At(i64::MIN)],
    ),
    (
        "18446744069414584321",
        [HostLong(1), HostLong(1), Bad, At(i64::MAX)],
    ),
    (
        "end-18446744073709551615",
        [HostLong(12), HostLong(12), Bad, At(-1)],
    ),
    ("0xffffffffffffffff+1", [Bad, HostLong(0), Bad, At(12)]),
];

/// Each release reads one index as its own `string index` does, measured on
/// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 over `abcdefghijkl` (8.5
/// and 8.6 answer alike, as do 9.0 and 9.1, but for the integers the second
/// table reads): `None` is `bad index`. Past 32 bits each release reads in
/// its own range, measured with `string index`, `string
/// range` from each side and `lset`: 8.4 to 8.6 wrap an integer within
/// ±4294967295 to 32 bits and their sums and `end` offsets alike, 8.4 and
/// 8.5 read a magnitude from 2^64 − 2^32 + 1 to 2^64 − 1 as a 64-bit `long`
/// wraps it, which a 32-bit one cannot hold, and 9.0 reads a wide, a bignum
/// alone as the nearest wide and a sum that reaches the widest, or a bignum
/// offset after `end`, as `end+1`.
#[test]
fn each_release_reads_an_index_as_its_tclsh_does() {
    use tcl_syntax::number::NumberSyntax::{Tcl84, Tcl85, Tcl90};
    // (spec, 8.4, 8.5 and 8.6, 9.0 and 9.1)
    type Row = (&'static str, Option<i64>, Option<i64>, Option<i64>);
    let table: &[Row] = &[
        ("1", Some(1), Some(1), Some(1)),
        (" 1", Some(1), Some(1), Some(1)),
        ("1 ", Some(1), Some(1), Some(1)),
        ("+1", Some(1), Some(1), Some(1)),
        (" +1", Some(1), Some(1), Some(1)),
        ("- 1", None, None, None),
        ("-1", Some(-1), Some(-1), Some(-1)),
        ("0x2", Some(2), Some(2), Some(2)),
        ("010", Some(8), Some(8), Some(10)),
        ("0o10", None, Some(8), Some(8)),
        ("0b10", None, Some(2), Some(2)),
        ("1_0", None, None, Some(10)),
        ("1e0", None, None, None),
        ("end", Some(11), Some(11), Some(11)),
        ("e", Some(11), Some(11), None),
        ("en", Some(11), Some(11), None),
        ("endx", None, None, None),
        (" end", None, None, None),
        ("end ", None, None, None),
        ("end-1", Some(10), Some(10), Some(10)),
        ("end+1", None, Some(12), Some(12)),
        ("end--1", Some(12), Some(12), Some(12)),
        ("end-+1", Some(10), Some(10), Some(10)),
        ("end- 1", Some(10), None, None),
        ("end+ 1", None, None, None),
        ("end-1 ", Some(10), Some(10), Some(10)),
        ("end-0x1", Some(10), Some(10), Some(10)),
        ("end-010", Some(3), Some(3), Some(1)),
        ("end-", None, None, None),
        ("end+", None, None, None),
        ("end-1-1", None, None, None),
        ("end -1", None, None, None),
        ("1+1", None, Some(2), Some(2)),
        ("1-1", None, Some(0), Some(0)),
        ("1+-1", None, Some(0), Some(0)),
        ("1--1", None, Some(2), Some(2)),
        ("1++1", None, Some(2), Some(2)),
        ("+1+1", None, Some(2), Some(2)),
        ("1+ 1", None, None, None),
        ("1 +1", None, None, None),
        (" 1+1", None, Some(2), Some(2)),
        ("1+1 ", None, Some(2), Some(2)),
        ("0x1+1", None, Some(2), Some(2)),
        ("010+0", None, Some(8), Some(10)),
        ("-1+2", None, Some(1), Some(1)),
        ("1+end", None, None, None),
        ("end-1+1", None, None, None),
        ("1 0", None, None, None),
        ("{1}", None, None, None),
        ("", None, None, None),
        (" ", None, None, None),
    ];
    for &(spec, on_84, on_85, on_90) in table {
        assert_eq!(resolve_opt_with(spec, 12, Tcl84), on_84, "{spec:?} on 8.4");
        assert_eq!(resolve_opt_with(spec, 12, Tcl85), on_85, "{spec:?} on 8.5");
        assert_eq!(resolve_opt_with(spec, 12, Tcl90), on_90, "{spec:?} on 9.0");
    }

    for &(spec, readings) in PAST_32_BITS {
        for (release, want) in TclVersion::ALL.into_iter().zip([
            readings[0],
            readings[1],
            readings[2],
            readings[3],
            readings[3],
        ]) {
            assert_eq!(
                read_under(spec, 12, release),
                want,
                "{spec:?} on {release:?}"
            );
        }
    }
    // The grammar 8.5 and 8.6 share names neither, so it cannot say which
    // reading a 64-bit `long` gives one of them.
    assert_eq!(
        read_with("18446744073709551615", 12, Tcl85),
        IndexReading::Unsure
    );
    assert_eq!(
        read_with("0xffffffffffffffff+1", 12, Tcl85),
        IndexReading::Unsure
    );
    assert_eq!(resolve_opt_with("18446744073709551615", 12, Tcl84), None);
}

/// 8.6 encodes a literal `end` offset that is positive after the 32-bit
/// wrap as after the end, where the word read at run time sums with `end`
/// in an `int`; the two part where that sum passes `INT_MAX`.
/// Measured on a 12-element string and list, a literal against the same
/// word in a variable (`programs/s1/litvar.tcl`): only 8.6 parts, and
/// only on these rows.
#[test]
fn a_literal_end_offset_past_int_compiles_apart_in_8_6() {
    for (spec, apart) in [
        ("end+2147483647", true),
        ("end+2147483637", true),
        ("end+2147483636", false),
        ("end-2147483649", true),
        ("end--2147483647", true),
        ("end-2147483648", false),
        ("end-2147483647", false),
        ("end+4294967296", false),
        ("end+1", false),
        ("2147483647", false),
        ("1+2147483647", false),
    ] {
        assert_eq!(compiles_apart(spec, 12, TclVersion::V8_6), apart, "{spec}");
        for release in TclVersion::ALL {
            if release != TclVersion::V8_6 {
                assert!(!compiles_apart(spec, 12, release), "{spec} on {release:?}");
            }
        }
    }
}

/// A literal index word fixed as each release reads it whatever the length,
/// compiled in a body and read by its command at run time. Measured on
/// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 with `string index`,
/// `string range` from each side, `lindex` and `lrange` over a 12-element
/// string and list, each word a literal inside a proc against the same
/// word in a variable (`end+2147483647` in a proc ranges the whole string
/// on 8.6 and nothing through a variable; 8.5 ranges nothing either way,
/// 9.0 and 9.1 the whole string either way). `None` is a reading that turns
/// on the length or the host, or `bad index`.
#[test]
fn a_literal_index_is_fixed_as_each_release_compiles_or_reads_it() {
    use Encoded::{After, Before, FromEnd, Index};
    use TclVersion::{V8_4, V8_5, V8_6, V9_0, V9_1};
    // (spec, release, compiled, read at run time)
    type Row = (&'static str, TclVersion, Option<Encoded>, Option<Encoded>);
    let table: &[Row] = &[
        ("end+2147483647", V8_6, Some(After), None),
        ("end+2147483646", V8_6, Some(After), None),
        ("end+2147483637", V8_6, Some(After), None),
        ("end+2147483636", V8_6, Some(After), None),
        ("end-2147483649", V8_6, Some(After), None),
        ("end--2147483647", V8_6, Some(After), None),
        ("end+1", V8_6, Some(After), None),
        (
            "end-2147483647",
            V8_6,
            Some(Before),
            Some(FromEnd(-2_147_483_647)),
        ),
        ("end-2147483648", V8_6, Some(Before), None),
        ("end+2147483648", V8_6, Some(Before), None),
        ("end-1", V8_6, Some(FromEnd(-1)), Some(FromEnd(-1))),
        ("end", V8_6, Some(FromEnd(0)), Some(FromEnd(0))),
        ("e", V8_6, Some(FromEnd(0)), Some(FromEnd(0))),
        ("end+4294967295", V8_6, Some(FromEnd(-1)), Some(FromEnd(-1))),
        ("end+4294967296", V8_6, None, None),
        (
            "2147483647",
            V8_6,
            Some(Index(2_147_483_647)),
            Some(Index(2_147_483_647)),
        ),
        ("2147483648", V8_6, Some(Before), Some(Before)),
        (
            "-2147483649",
            V8_6,
            Some(Index(2_147_483_647)),
            Some(Index(2_147_483_647)),
        ),
        ("1+2147483647", V8_6, Some(Before), Some(Before)),
        ("010", V8_6, Some(Index(8)), Some(Index(8))),
        ("-1", V8_6, Some(Before), Some(Before)),
        ("end+2147483647", V8_5, None, None),
        ("end-2147483649", V8_5, None, None),
        (
            "end-2147483647",
            V8_5,
            Some(FromEnd(-2_147_483_647)),
            Some(FromEnd(-2_147_483_647)),
        ),
        ("end-1", V8_5, Some(FromEnd(-1)), Some(FromEnd(-1))),
        ("2147483648", V8_5, Some(Before), Some(Before)),
        ("end-2147483649", V8_4, None, None),
        ("end+1", V8_4, None, None),
        ("end-1", V8_4, Some(FromEnd(-1)), Some(FromEnd(-1))),
        (
            "end+2147483647",
            V9_0,
            Some(FromEnd(2_147_483_647)),
            Some(FromEnd(2_147_483_647)),
        ),
        (
            "end-2147483649",
            V9_0,
            Some(FromEnd(-2_147_483_649)),
            Some(FromEnd(-2_147_483_649)),
        ),
        (
            "2147483648",
            V9_0,
            Some(Index(2_147_483_648)),
            Some(Index(2_147_483_648)),
        ),
        ("end+1", V9_1, Some(FromEnd(1)), Some(FromEnd(1))),
        ("010", V9_1, Some(Index(10)), Some(Index(10))),
    ];
    for &(spec, release, compiled, run_time) in table {
        assert_eq!(
            encode(spec, release, true),
            compiled,
            "{spec:?} compiled on {release:?}"
        );
        assert_eq!(
            encode(spec, release, false),
            run_time,
            "{spec:?} read on {release:?}"
        );
    }
}

/// The version-less entry takes the grammar this runtime was built for. The
/// ambient starts at 9.0, so an unpinned process reads `010` as decimal —
/// and pinning it to 8.5 flips the same spec to octal.
#[test]
fn the_versionless_entry_follows_the_installed_grammar() {
    use tcl_syntax::number::{NumberSyntax, runtime_syntax, set_runtime_syntax};
    let restore = runtime_syntax();
    set_runtime_syntax(NumberSyntax::Tcl90);
    assert_eq!(resolve_opt("010", 12), Some(10));
    set_runtime_syntax(NumberSyntax::Tcl85);
    assert_eq!(resolve_opt("010", 12), Some(8));
    set_runtime_syntax(restore);
}

/// The grammar-taking entry ignores the installed grammar. Measured on a
/// 12-character subject with `string index` (LANG=C.UTF-8): `010` is 8
/// and `end-010` is 3 on tclsh 8.4.20, 8.5.19 and 8.6.18, 10 and 1 on
/// 9.0.4 and 9.1.0; `1_0` is `bad index` up to 8.6 and 10 from 9.0.
#[test]
fn the_grammar_taking_entry_ignores_the_installed_grammar() {
    use tcl_syntax::number::{NumberSyntax, runtime_syntax, set_runtime_syntax};
    let restore = runtime_syntax();
    for ambient in [NumberSyntax::Tcl85, NumberSyntax::Tcl90] {
        set_runtime_syntax(ambient);
        for numbers in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
            assert_eq!(resolve_with("010", 12, numbers).ok(), Some(8));
            assert_eq!(resolve_with("end-010", 12, numbers).ok(), Some(3));
            assert!(resolve_with("1_0", 12, numbers).is_err());
        }
        assert_eq!(resolve_with("010", 12, NumberSyntax::Tcl90).ok(), Some(10));
        assert_eq!(
            resolve_with("end-010", 12, NumberSyntax::Tcl90).ok(),
            Some(1)
        );
        assert_eq!(resolve_with("1_0", 12, NumberSyntax::Tcl90).ok(), Some(10));
    }
    set_runtime_syntax(restore);
}
