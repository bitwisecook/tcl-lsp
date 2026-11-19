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

use tcl_dialect::model::{Family, SurfaceQuery};

use super::{fold_is, fold_range, string_is_member_type, string_is_member_type_for_dialect};
use crate::hooks::TclVersion;

/// tclsh 8.4–9.1: `string range abcdef 1 3` is `bcd` and `1 end` is
/// `bcdef`; `1+1` is an index from 8.5 only (8.4: `bad index "1+1"`).
#[test]
fn the_range_fold_answers_every_release_and_declines_a_release_split_index() {
    for version in TclVersion::ALL {
        assert_eq!(
            fold_range(&["abcdef", "1", "3"], Some(version)).as_deref(),
            Some("bcd"),
            "{version:?}"
        );
        assert_eq!(
            fold_range(&["abcdef", "1", "end"], Some(version)).as_deref(),
            Some("bcdef"),
            "{version:?}"
        );
    }
    assert_eq!(
        fold_range(&["abcdef", "1+1", "3"], Some(TclVersion::V8_4)),
        None
    );
}
use crate::types::TclType;
use crate::{CommandRegistry, DispatchDependencies, DispatchDependencyDomain};

/// A `string is` test proves a type only where every value it accepts
/// holds it once tested: the numeric classes under `-strict` (`string is
/// integer {}` is 1 on every release, `-strict` makes it 0), `dict`
/// either way; `list`, which leaves `{}` a pure string, the boolean
/// classes, which leave `1` an integer, and a character class never; the
/// class reads with Tcl's abbreviations, and `-failindex` or an option
/// spelled otherwise proves nothing.
#[test]
fn a_string_is_test_proves_the_type_its_members_share() {
    let cases: [(&[&str], Option<TclType>); 11] = [
        (&["integer", "-strict", "$x"], Some(TclType::Int)),
        (&["int", "-strict", "$x"], Some(TclType::Int)),
        (&["integer", "$x"], None),
        (&["double", "-strict", "$x"], Some(TclType::Numeric)),
        (&["boolean", "-strict", "$x"], None),
        (&["list", "$x"], None),
        (&["dict", "-strict", "$x"], Some(TclType::Dict)),
        (&["alpha", "-strict", "$x"], None),
        (&["integer", "-failindex", "at", "$x"], None),
        (&["integer", "-str", "$x"], None),
        (&["integer"], None),
    ];
    for (args, expected) in cases {
        assert_eq!(string_is_member_type(args), expected, "{args:?}");
    }
}

#[test]
fn string_is_value_form_preserves_optional_write_effects() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let context = crate::model::ingress::static_context_for(profile);
        let dialect = crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
        let plain = [
            crate::InvocationWord::Literal("is"),
            crate::InvocationWord::Literal("integer"),
            crate::InvocationWord::Literal("42"),
        ];
        let invocation =
            crate::InvocationWords::structured(crate::InvocationWord::Literal("string"), &plain)
                .with_dialect(dialect);
        let resolved = context
            .commands()
            .resolve_structured_invocation(invocation, dialect.authoring_query())
            .resolved()
            .unwrap();
        let effects = resolved.effects();
        assert!(!effects.requires_world_barrier(), "{profile}: {effects:?}");
        assert!(effects.accesses().is_empty(), "{profile}: {effects:?}");
        for options in [
            ["is", "integer", "-failindex", "index", "not-integer"],
            ["is", "integer", "-failindex", "index", "42"],
        ] {
            let resolved = context
                .commands()
                .resolve_invocation("string", &options, dialect.authoring_query())
                .unwrap();
            assert!(resolved.effects().requires_world_barrier(), "{profile}");
            assert!(
                resolved
                    .effects()
                    .accesses()
                    .iter()
                    .any(|access| access.domain == crate::WorldStateDomain::VariableStore),
                "{profile}"
            );
        }
    }
}

#[test]
fn string_is_folds_tcl_faithful_classes() {
    // Helper: `string is <args...>`.
    // The char / boolean / list classes are version-invariant; pin them
    // under 9.0 (the number classes have their own version-aware tests).
    let is = |args: &[&str]| fold_is(args, Some(TclVersion::V9_0));

    // ASCII char classes — all-chars-in-class.
    assert_eq!(is(&["alpha", "abc"]).as_deref(), Some("1"));
    assert_eq!(is(&["alpha", "abc1"]).as_deref(), Some("0"));
    assert_eq!(is(&["alnum", "abc1"]).as_deref(), Some("1"));
    assert_eq!(is(&["digit", "123"]).as_deref(), Some("1"));
    assert_eq!(is(&["digit", "12a"]).as_deref(), Some("0"));
    assert_eq!(is(&["xdigit", "1aF"]).as_deref(), Some("1"));
    assert_eq!(is(&["space", "  \t"]).as_deref(), Some("1"));
    assert_eq!(is(&["wordchar", "a_b9"]).as_deref(), Some("1"));
    assert_eq!(is(&["wordchar", "a-b"]).as_deref(), Some("0"));

    // A divergence worth pinning: a naive "all lowercase" check would
    // pass `"abc1"`, but Tcl `string is lower` is 0
    // (a digit is not a lowercase *letter*).
    assert_eq!(is(&["lower", "abc1"]).as_deref(), Some("0"));
    assert_eq!(is(&["lower", "abc"]).as_deref(), Some("1"));
    assert_eq!(is(&["upper", "ABC"]).as_deref(), Some("1"));
    assert_eq!(is(&["upper", "ABc"]).as_deref(), Some("0"));

    // `ascii` is the membership test itself (defined for any input).
    assert_eq!(is(&["ascii", "abc"]).as_deref(), Some("1"));
    assert_eq!(is(&["ascii", "caf\u{e9}"]).as_deref(), Some("0"));
    // Other char classes bail on non-ASCII (membership not modelled).
    assert_eq!(is(&["alpha", "caf\u{e9}"]), None);

    // Boolean keyword + unique-prefix set.
    assert_eq!(is(&["boolean", "yes"]).as_deref(), Some("1"));
    assert_eq!(is(&["boolean", "t"]).as_deref(), Some("1"));
    assert_eq!(is(&["boolean", "of"]).as_deref(), Some("1"));
    assert_eq!(
        is(&["boolean", "o"]).as_deref(),
        Some("0"),
        "ambiguous on/off"
    );
    assert_eq!(is(&["boolean", "maybe"]).as_deref(), Some("0"));
    assert_eq!(is(&["true", "TRUE"]).as_deref(), Some("1"));
    assert_eq!(is(&["true", "no"]).as_deref(), Some("0"));
    assert_eq!(is(&["false", "off"]).as_deref(), Some("1"));

    // The selected list reader validates both escaped and braced elements.
    assert_eq!(is(&["list", "a b c"]).as_deref(), Some("1"));
    assert_eq!(is(&["list", "{a b} c"]).as_deref(), Some("1"));
    assert_eq!(is(&["list", "a {b"]).as_deref(), Some("0"), "unbalanced");
    assert_eq!(is(&["list", "a\\ b"]).as_deref(), Some("1"));

    // Empty string: non-strict member of every class, strict member
    // of none.
    assert_eq!(is(&["alpha", ""]).as_deref(), Some("1"));
    assert_eq!(is(&["alpha", "-strict", ""]).as_deref(), Some("0"));
    assert_eq!(
        is(&["integer", ""]).as_deref(),
        Some("1"),
        "even a deferred class"
    );

    // `-failindex` writes a var → never folds; an unknown class bails.
    assert_eq!(is(&["alpha", "-failindex", "v", "abc"]), None);
    assert_eq!(is(&["nonclass", "x"]), None, "unknown class bails");
    // Under 9.0 these classes fold (their version-gating
    // and caps are covered by `string_is_number_classes_are_version_aware`).
    assert_eq!(is(&["wideinteger", "42"]).as_deref(), Some("1"));
    assert_eq!(is(&["entier", "42"]).as_deref(), Some("1"));
    assert_eq!(is(&["dict", "a 1"]).as_deref(), Some("1"));
}

#[test]
fn literal_string_options_and_classes_follow_the_receivers() {
    let reg = CommandRegistry::build_default();
    let string = reg.get("string").unwrap();
    let (compare, equal, classify) = (
        string.subcommand("compare").unwrap(),
        string.subcommand("equal").unwrap(),
        string.subcommand("is").unwrap(),
    );
    for (profile, octal_equal, double_08, boolean_t) in [
        ("tcl8.4", Some("1"), "1", "1"),
        ("tcl8.5", Some("1"), "0", "1"),
        ("tcl8.6", Some("1"), "0", "1"),
        ("tcl9.0", Some("0"), "1", "1"),
        ("tcl9.1", Some("0"), "1", "1"),
        ("jimtcl", None, "1", "0"),
    ] {
        let dialect =
            crate::InvocationDialect::of_profile(crate::native_test_provider::profile(profile));
        assert_eq!(compare.run_const_fold_in(&["--", "a", "b"], dialect), None);
        assert_eq!(equal.run_const_fold_in(&["--", "a", "a"], dialect), None);
        assert_eq!(
            equal
                .run_const_fold_in(&["-length", "010", "abcdefghX", "abcdefghY"], dialect)
                .as_deref(),
            octal_equal,
            "{profile}"
        );
        assert_eq!(
            classify
                .run_const_fold_in(&["double", "08"], dialect)
                .as_deref(),
            Some(double_08),
            "{profile}"
        );
        assert_eq!(
            classify
                .run_const_fold_in(&["boolean", "t"], dialect)
                .as_deref(),
            Some(boolean_t),
            "{profile}"
        );
    }
}

#[test]
fn string_is_double_uses_the_selected_number_grammar() {
    // The selected C 9 grammar accepts both explicit radix and decimal forms.
    let is_dbl = |v: &str| fold_is(&["double", v], Some(TclVersion::V9_0));

    // Decimal / scientific / Inf / NaN forms fold to "1" — Rust's f64
    // parser agrees with Tcl 8.4 → 9.0 across this matrix.
    for v in [
        "3.14", "-2.5", "+.5", ".5", "5.", "5.e3", "42", "0", "1e10", "1E-10", "1e+5", "0.0",
        "00.5", "inf", "Inf", "infinity", "nan", "NaN", "-inf", " 1.5 ", "-3.14",
    ] {
        assert_eq!(is_dbl(v).as_deref(), Some("1"), "{v:?} is a double");
    }
    // Clear non-doubles fold to "0".
    for v in ["abc", "1e", "e5", "1.5e", ".", "1.2.3", "1d0", "1f", " "] {
        assert_eq!(is_dbl(v).as_deref(), Some("0"), "{v:?} is not a double");
    }
    for (literal, expected) in [
        ("1_000", "1"),
        ("0x1f", "1"),
        ("0o17", "1"),
        ("0x1.8p3", "0"),
        ("nan(0)", "1"),
    ] {
        assert_eq!(is_dbl(literal).as_deref(), Some(expected), "{literal}");
    }
    assert_eq!(is_dbl("caf\u{e9}"), None, "non-ASCII bails");
}

/// A spelling outside the double range is a double from 8.5 and not on
/// 8.4; with no core release (Jim, or none named) the fold abstains. The
/// boundary doubles and zero mantissas fold the same everywhere. Rows
/// measured on tclsh 8.4.20 to 9.1.0 and jimsh 0.84 (#2435).
#[test]
fn string_is_double_range_errors_follow_the_release() {
    let is_dbl = |v: &str, version| fold_is(&["double", v], version);
    for v in [
        "1e400",
        "-1e400",
        "1e-400",
        "1.8e308",
        "4.9e-324",
        "1e-320",
        "2.2250738585072009e-308",
    ] {
        assert_eq!(
            is_dbl(v, Some(TclVersion::V8_4)).as_deref(),
            Some("0"),
            "{v}"
        );
        assert_eq!(is_dbl(v, None), None, "{v}");
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            assert_eq!(is_dbl(v, Some(version)).as_deref(), Some("1"), "{v}");
        }
    }
    for v in [
        "1.7976931348623157e308",
        "2.2250738585072014e-308",
        "0e999",
        "0.0e-400",
        "Inf",
        "NaN",
        "1e5",
    ] {
        for version in [None, Some(TclVersion::V8_4), Some(TclVersion::V9_0)] {
            assert_eq!(is_dbl(v, version).as_deref(), Some("1"), "{v}");
        }
    }
    assert_eq!(
        fold_is(&["double", "-strict", "1e400"], Some(TclVersion::V8_4)).as_deref(),
        Some("0")
    );
}

#[test]
fn string_is_integer_folds_dialect_invariant_subset() {
    // `None` version → the dialect-invariant subset every release shares.
    let is_int = |v: &str| fold_is(&["integer", v], None);

    // Non-leading-zero decimals within the 32-bit-unsigned bound every
    // 8.4 → 9.0 shares fold to "1" (verified against all four tclsh).
    assert_eq!(is_int("42").as_deref(), Some("1"));
    assert_eq!(is_int("-7").as_deref(), Some("1"));
    assert_eq!(is_int("+5").as_deref(), Some("1"));
    assert_eq!(is_int("0").as_deref(), Some("1"));
    assert_eq!(
        is_int(" 42 ").as_deref(),
        Some("1"),
        "surrounding ws trimmed"
    );
    assert_eq!(is_int("2147483648").as_deref(), Some("1"), "past 2^31");
    assert_eq!(
        is_int("4294967295").as_deref(),
        Some("1"),
        "2^32-1 boundary"
    );
    assert_eq!(is_int("-4294967295").as_deref(), Some("1"));

    // Clear non-integers fold to "0" — no Tcl version reads them as one.
    assert_eq!(is_int("abc").as_deref(), Some("0"));
    assert_eq!(is_int("1.5").as_deref(), Some("0"));
    assert_eq!(is_int("1e5").as_deref(), Some("0"));
    assert_eq!(is_int("12x").as_deref(), Some("0"));
    assert_eq!(is_int("1 2").as_deref(), Some("0"), "internal space");
    assert_eq!(is_int(".5").as_deref(), Some("0"));
    assert_eq!(is_int("-").as_deref(), Some("0"), "lone sign");
    assert_eq!(is_int("0abc").as_deref(), Some("0"));
    assert_eq!(is_int(" ").as_deref(), Some("0"), "whitespace-only");

    // Divergence / ambiguity bails (a miss is never wrong).
    assert_eq!(
        is_int("4294967296"),
        None,
        "past the 32-bit cap (8.x reject)"
    );
    assert_eq!(is_int("9999999999999999999999"), None, "huge → diverges");
    assert_eq!(
        is_int("010").as_deref(),
        Some("1"),
        "integer in every release"
    );
    assert_eq!(is_int("08"), None, "leading zero, invalid octal in 8.x");
    assert_eq!(is_int("0x1f").as_deref(), Some("1"));
    assert_eq!(is_int("0o17"), None, "0o raises in 8.4");
    assert_eq!(is_int("0b101"), None, "0b raises in 8.4");
    assert_eq!(is_int("caf\u{e9}"), None, "non-ASCII bails");

    // Option grammar: the last arg is the value, the rest are options
    // (verified against tclsh 8.4 → 9.0).
    assert_eq!(
        fold_is(&["integer", "-strict", "42"], None).as_deref(),
        Some("1"),
        "-strict option, value 42"
    );
    assert_eq!(
        fold_is(&["integer", "-strict", "-7"], None).as_deref(),
        Some("1"),
        "-strict option, negative value"
    );
    assert_eq!(
        fold_is(&["integer", "-strict"], None).as_deref(),
        Some("0"),
        "lone `-strict` is the value, not an option"
    );
    assert_eq!(
        fold_is(&["integer", "-failindex", "v", "42"], None),
        None,
        "-failindex writes a var → bail"
    );
}

#[test]
fn original_class_type_projection_requires_its_own_family_and_release() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Software class-policy selection only; no Native cache/frame/store
    // observation or successful command evaluation is asserted.
    let c84 = crate::InvocationDialect::for_version(TclVersion::V8_4);
    let c86 = crate::InvocationDialect::for_version(TclVersion::V8_6);
    let mut c90 = crate::InvocationDialect::for_version(TclVersion::V9_0);
    c90.numbers = tcl_dialect::NumberSyntax::Tcl85;
    assert_eq!(
        string_is_member_type_for_dialect(&["wideinteger", "-strict", "VALUE"], c84),
        None
    );
    assert_eq!(
        string_is_member_type_for_dialect(&["wideinteger", "-strict", "VALUE"], c86),
        Some(TclType::Int)
    );
    assert_eq!(
        string_is_member_type_for_dialect(&["dict", "VALUE"], c86),
        None
    );
    assert_eq!(
        string_is_member_type_for_dialect(&["dict", "VALUE"], c90),
        Some(TclType::Dict)
    );
    assert_eq!(
        string_is_member_type_for_dialect(&["integer", "-failindex", "out", "VALUE"], c90),
        None
    );
    let jim = crate::InvocationDialect::of_profile(crate::native_test_provider::profile("jimtcl"));
    assert_eq!(
        string_is_member_type_for_dialect(&["integer", "-strict", "VALUE"], jim),
        None
    );
    let mut foreign = c90;
    foreign.native_family = Some(tcl_dialect::model::Family::Jim);
    assert_eq!(
        string_is_member_type_for_dialect(&["dict", "VALUE"], foreign),
        None
    );
}

#[test]
fn string_is_number_classes_are_version_aware() {
    use TclVersion::{V8_4, V8_5, V8_6, V9_0};
    let is = |class: &str, v: &str, ver| fold_is(&[class, v], ver);

    // `integer`: 8.x cap at 2^32-1, 9.0 unbounded.  All verified vs the
    // four tclsh.
    assert_eq!(
        is("integer", "4294967296", Some(V8_6)).as_deref(),
        Some("0")
    );
    assert_eq!(
        is("integer", "4294967296", Some(V9_0)).as_deref(),
        Some("1")
    );
    assert_eq!(is("integer", "1__0", Some(V9_0)).as_deref(), Some("1"));
    assert_eq!(is("integer", "0xff__ff", Some(V9_0)).as_deref(), Some("1"));
    assert_eq!(is("integer", "0d1__0", Some(V9_0)).as_deref(), Some("1"));
    assert_eq!(is("integer", "0x_f", Some(V9_0)).as_deref(), Some("0"));
    assert_eq!(is("integer", "1__0", None), None, "Tcl 9-only separators");
    assert_eq!(is("integer", "42", Some(V8_4)).as_deref(), Some("1"));
    assert_eq!(is("integer", "abc", Some(V9_0)).as_deref(), Some("0"));

    // `wideinteger`: raises on 8.4 (bail), unsigned-64 on 8.5/8.6,
    // signed-64 on 9.0.
    assert_eq!(is("wideinteger", "42", Some(V8_4)), None, "raises in 8.4");
    assert_eq!(is("wideinteger", "42", None), None, "unknown version");
    assert_eq!(
        is("wideinteger", "9223372036854775808", Some(V8_6)).as_deref(),
        Some("1"),
        "2^63 fits unsigned-64 on 8.6"
    );
    assert_eq!(
        is("wideinteger", "9223372036854775808", Some(V9_0)).as_deref(),
        Some("0"),
        "2^63 overflows signed-64 on 9.0"
    );
    assert_eq!(
        is("wideinteger", "-9223372036854775808", Some(V9_0)).as_deref(),
        Some("1"),
        "i64::MIN fits signed wide on 9.0"
    );
    assert_eq!(
        is("wideinteger", "-9223372036854775809", Some(V9_0)).as_deref(),
        Some("0"),
        "below i64::MIN overflows signed wide on 9.0"
    );
    assert_eq!(
        is("wideinteger", "0xff__ff", Some(V9_0)).as_deref(),
        Some("1")
    );
    assert_eq!(
        is("wideinteger", "18446744073709551616", Some(V8_6)).as_deref(),
        Some("0"),
        "2^64 overflows unsigned-64"
    );

    // `entier`: raises on 8.4/8.5, unbounded on 8.6/9.0.
    assert_eq!(is("entier", "42", Some(V8_5)), None, "raises in 8.5");
    assert_eq!(
        is("entier", "99999999999999999999999999", Some(V8_6)).as_deref(),
        Some("1"),
        "arbitrary precision"
    );
    assert_eq!(is("entier", "1__0", Some(V9_0)).as_deref(), Some("1"));
    assert_eq!(is("entier", "0xff__ff", Some(V9_0)).as_deref(), Some("1"));
    assert_eq!(is("entier", "0x_f", Some(V9_0)).as_deref(), Some("0"));
    assert_eq!(is("entier", "abc", Some(V9_0)).as_deref(), Some("0"));

    // `dict`: 9.0 only — even-length valid list.
    assert_eq!(is("dict", "a 1 b 2", Some(V8_6)), None, "raises before 9.0");
    assert_eq!(is("dict", "a 1 b 2", Some(V9_0)).as_deref(), Some("1"));
    assert_eq!(is("dict", "a 1 b", Some(V9_0)).as_deref(), Some("0"), "odd");
    assert_eq!(is("dict", "a b c", Some(V9_0)).as_deref(), Some("0"));
    // The empty-string shortcut is also version-gated.
    assert_eq!(is("dict", "", Some(V8_6)), None, "dict \"\" raises pre-9.0");
    assert_eq!(is("dict", "", Some(V9_0)).as_deref(), Some("1"));

    // Tcl9.1 must fold identically to 9.0 (it is a 9.0+
    // superset), not degrade to the dialect-invariant 8.x subset. Assert
    // parity with 9.0 across the version-sensitive classes.
    let v91 = TclVersion::from_dialect(Some("tcl9.1"));
    assert_eq!(v91, Some(TclVersion::V9_1));
    for (class, val) in [
        ("integer", "4294967296"),
        ("integer", "1__0"),
        ("wideinteger", "9223372036854775808"),
        ("entier", "0xff__ff"),
        ("dict", "a 1 b 2"),
    ] {
        assert_eq!(
            is(class, val, v91),
            is(class, val, Some(V9_0)),
            "9.1 must match 9.0 for `string is {class} {val}`",
        );
    }
}

#[test]
fn string_is_subcommand_carries_const_fold() {
    // `string is` is version-aware, so it registers a `const_fold_versioned`.
    let reg = CommandRegistry::build_default();
    let sub = reg
        .get("string")
        .and_then(|s| s.subcommand("is"))
        .expect("string is subcommand");
    assert!(sub.const_fold_versioned.is_some());
    assert_eq!(
        sub.run_const_fold(&["alpha", "abc"], Some(tcl_dialect::TclVersion::V9_0))
            .as_deref(),
        Some("1")
    );
}

#[test]
fn tcl9_string_is_fold_does_not_read_the_runtime_number_grammar() {
    let previous = tcl_syntax::number::runtime_syntax();
    tcl_syntax::number::set_runtime_syntax(tcl_dialect::NumberSyntax::Tcl85);
    let folded = fold_is(&["integer", "08"], Some(TclVersion::V9_0));
    tcl_syntax::number::set_runtime_syntax(previous);

    assert_eq!(folded.as_deref(), Some("1"));
}

#[test]
fn pure_string_subcommands_carry_const_fold() {
    // toupper / tolower / reverse expose a
    // const_fold callback for the optimiser's O129 path.
    let reg = CommandRegistry::build_default();
    let spec = reg.get("string").expect("string spec");

    let toupper = spec
        .subcommand("toupper")
        .and_then(|s| s.const_fold)
        .expect("toupper const_fold");
    assert_eq!(toupper(&["foo"]).as_deref(), Some("FOO"));
    // The range form (`string toupper s first last`) must not fold.
    assert_eq!(toupper(&["foo", "0", "0"]), None);
    // Non-ASCII bails (conservative — Rust/Tcl case maps diverge).
    assert_eq!(toupper(&["caf\u{e9}"]), None);

    let tolower = spec
        .subcommand("tolower")
        .and_then(|s| s.const_fold)
        .expect("tolower const_fold");
    assert_eq!(tolower(&["FOO"]).as_deref(), Some("foo"));

    let reverse = spec
        .subcommand("reverse")
        .and_then(|s| s.const_fold)
        .expect("reverse const_fold");
    assert_eq!(reverse(&["abc"]).as_deref(), Some("cba"));

    let length = spec
        .subcommand("length")
        .and_then(|s| s.const_fold)
        .expect("length const_fold");
    assert_eq!(length(&["abcde"]).as_deref(), Some("5"));
    assert_eq!(length(&[""]).as_deref(), Some("0"));
    assert_eq!(length(&["caf\u{e9}"]), None, "non-ASCII bails");
}

#[test]
fn string_length_dispatch_uses_only_irreducible_live_domains() {
    let registry = CommandRegistry::build_default();
    for dialect in [
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        Some(SurfaceQuery::core(Family::Tcl, "9.0")),
    ] {
        let facts = registry
            .resolve_invocation("string", &["length", "value"], dialect)
            .expect("string length resolves")
            .facts();
        assert_eq!(facts.dispatch_dependencies, DispatchDependencies::BASE);
        assert!(
            facts
                .dispatch_dependencies
                .contains(DispatchDependencyDomain::InterpreterPolicy)
        );
        assert!(
            !facts
                .dispatch_dependencies
                .contains(DispatchDependencyDomain::ObjectDispatch)
        );
        assert!(
            !facts
                .dispatch_dependencies
                .contains(DispatchDependencyDomain::UnknownHandling)
        );
    }
}

#[test]
fn string_match_const_fold_matches_tcl_glob_semantics() {
    let registry = crate::CommandRegistry::build_default();
    let spec = registry.get("string").unwrap();
    let fold = |args: &[&str]| {
        spec.subcommand("match")
            .unwrap()
            .run_const_fold(args, Some(tcl_dialect::TclVersion::V9_0))
    };
    assert_eq!(fold(&["::*", "::clay"]), Some("1".to_owned()));
    assert_eq!(fold(&["::*", "clay"]), Some("0".to_owned()));
    assert_eq!(fold(&["-noc", "A*", "alpha"]), Some("1".to_owned()));
    assert_eq!(fold(&["-bogus", "*", "x"]), None);
}

#[test]
fn string_value_folds_match_tcl() {
    // The cat / repeat / trim* / totitle folds.
    let reg = CommandRegistry::build_default();
    let spec = reg.get("string").expect("string spec");
    let f = |sub: &str| spec.subcommand(sub).and_then(|s| s.const_fold).unwrap();

    assert_eq!(f("cat")(&["a", "b", "c"]).as_deref(), Some("abc"));
    assert_eq!(f("cat")(&[]).as_deref(), Some(""));
    assert_eq!(f("repeat")(&["ab", "3"]).as_deref(), Some("ababab"));
    assert_eq!(f("repeat")(&["x", "0"]).as_deref(), Some(""));
    assert_eq!(f("repeat")(&["x", "-1"]), None, "negative count bails");
    assert_eq!(f("repeat")(&["x", "999999"]), None, "over-cap bails");
    assert_eq!(f("trim")(&["  hi  "]).as_deref(), Some("hi"));
    assert_eq!(f("trim")(&["xxhixx", "x"]).as_deref(), Some("hi"));
    assert_eq!(f("trimleft")(&["  hi  "]).as_deref(), Some("hi  "));
    assert_eq!(f("trimright")(&["  hi  "]).as_deref(), Some("  hi"));
    assert_eq!(f("trim")(&["\0hi\0"]), None);
    assert_eq!(f("trim")(&["\0hi\0", "\0"]).as_deref(), Some("hi"));
    assert_eq!(f("totitle")(&["hELLO"]).as_deref(), Some("Hello"));
    assert_eq!(f("totitle")(&[""]).as_deref(), Some(""));
    assert_eq!(f("totitle")(&["caf\u{e9}"]), None, "non-ASCII bails");
}

#[test]
fn string_index_comparison_folds_match_tcl() {
    // index / range / replace / first / last /
    // compare / equal, with `end` / `end-N` index parsing.
    let reg = CommandRegistry::build_default();
    let spec = reg.get("string").expect("string spec");
    let f = |sub: &str| spec.subcommand(sub).and_then(|s| s.const_fold).unwrap();

    assert_eq!(f("index")(&["abc", "1"]).as_deref(), Some("b"));
    assert_eq!(f("index")(&["abc", "end"]).as_deref(), Some("c"));
    assert_eq!(f("index")(&["abcde", "end--1"]).as_deref(), Some(""));
    assert_eq!(
        f("index")(&["abc", "9"]).as_deref(),
        Some(""),
        "OOB → empty"
    );
    assert_eq!(f("range")(&["abcde", "1", "3"]).as_deref(), Some("bcd"));
    assert_eq!(f("range")(&["abcde", "2", "end"]).as_deref(), Some("cde"));
    assert_eq!(f("range")(&["abcde", "3", "1"]).as_deref(), Some(""));
    assert_eq!(
        f("range")(&["abcde", "0", "end--1"]).as_deref(),
        Some("abcde")
    );
    assert_eq!(
        f("replace")(&["abcde", "end--1", "end", "X"]).as_deref(),
        Some("abcde")
    );
    assert_eq!(f("replace")(&["abcde", "1", "3"]).as_deref(), Some("ae"));
    assert_eq!(
        f("replace")(&["abcde", "1", "3", "XY"]).as_deref(),
        Some("aXYe")
    );
    assert_eq!(f("first")(&["b", "abcb"]).as_deref(), Some("1"));
    assert_eq!(f("first")(&["b", "abcb", "2"]).as_deref(), Some("3"));
    assert_eq!(f("first")(&["", "abc"]).as_deref(), Some("-1"));
    assert_eq!(f("first")(&["", "abc", "1"]).as_deref(), Some("-1"));
    assert_eq!(f("first")(&["z", "abc"]).as_deref(), Some("-1"));
    assert_eq!(f("last")(&["b", "abcb"]).as_deref(), Some("3"));
    assert_eq!(f("last")(&["", "abc"]).as_deref(), Some("-1"));
    assert_eq!(f("last")(&["", "abc", "1"]).as_deref(), Some("-1"));
    assert_eq!(f("last")(&["z", "abc"]).as_deref(), Some("-1"));
    assert_eq!(f("compare")(&["abc", "abc"]).as_deref(), Some("0"));
    assert_eq!(f("compare")(&["abc", "abd"]).as_deref(), Some("-1"));
    assert_eq!(f("compare")(&["abd", "abc"]).as_deref(), Some("1"));
    assert_eq!(
        spec.subcommand("compare")
            .unwrap()
            .run_const_fold(&["-nocase", "ABC", "abc"], Some(TclVersion::V8_6))
            .as_deref(),
        Some("0")
    );
    assert_eq!(
        spec.subcommand("compare")
            .unwrap()
            .run_const_fold(&["-length", "2", "abX", "abY"], Some(TclVersion::V8_6))
            .as_deref(),
        Some("0")
    );
    assert_eq!(f("equal")(&["abc", "abc"]).as_deref(), Some("1"));
    assert_eq!(f("equal")(&["abc", "abd"]).as_deref(), Some("0"));
}

#[test]
fn string_map_folds_match_tcl() {
    // `string map` greedy left-to-right replace.
    let reg = CommandRegistry::build_default();
    let m = reg
        .get("string")
        .and_then(|s| s.subcommand("map"))
        .and_then(|s| s.const_fold)
        .expect("map const_fold");
    assert_eq!(m(&["a b", "aaa"]).as_deref(), Some("bbb"));
    assert_eq!(m(&["ab AB", "xabx"]).as_deref(), Some("xABx"));
    assert_eq!(m(&["-nocase", "abc X", "ABCdef"]).as_deref(), Some("Xdef"));
    // Odd mapping → no fold; non-ASCII → no fold.
    assert_eq!(m(&["a b c", "x"]), None);
    assert_eq!(m(&["a b", "caf\u{e9}"]), None);
}

// `fold_cat` is wired through the `ConstFoldFn` callback contract
// (`-> Option<String>`) but pure concatenation never fails. Positive:
// several args concatenate. Edge: zero args fold to the empty string —
// it always returns `Some`, so the `Option` is a dispatch artefact.
#[test]
fn fold_cat_is_infallible() {
    assert_eq!(super::fold_cat(&["a", "b", "c"]).as_deref(), Some("abc"));
    assert_eq!(super::fold_cat(&[]).as_deref(), Some(""));
    // Non-ASCII is fine for cat (no per-char transform).
    assert_eq!(
        super::fold_cat(&["caf", "\u{e9}"]).as_deref(),
        Some("caf\u{e9}")
    );
    assert!(super::fold_cat(&["x"]).is_some());
}
