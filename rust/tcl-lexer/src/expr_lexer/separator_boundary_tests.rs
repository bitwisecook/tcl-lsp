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

/// Tcl 8.4's `GetLexeme` takes an integer prefix before it reaches the
/// generic function-name/unknown scanner; it does not have the 8.5+
/// number-to-bareword rescan. The shared lower scanner must therefore keep
/// these token boundaries distinct for the last 8.4 profile.
#[test]
fn tcl84_keeps_legacy_integer_prefix_boundaries() {
    let (tokens, unknown) = tokenise_expr_checked("1_eq", Some("tcl8.4"));
    assert!(
        unknown,
        "`_` must be Tcl 8.4's invalid next lexeme: {tokens:?}"
    );
    assert_eq!(tokens[0].kind, ExprTokenType::Number);
    assert_eq!(tokens[0].text, "1");

    for (source, first, second) in [("12x", "12", "x"), ("0x1p2", "0x1", "p2")] {
        let tokens = tokenise_expr(source, Some("tcl8.4"));
        assert_eq!(
            tokens[0].kind,
            ExprTokenType::Number,
            "{source}: {tokens:?}"
        );
        assert_eq!(tokens[0].text, first, "{source}: {tokens:?}");
        assert_eq!(tokens[1].text, second, "{source}: {tokens:?}");
    }
}

/// Explicit-radix spelling validity comes from the shared `NumberSyntax`
/// owner. Once a prefix has valid digits, a following release-available
/// word operator is a separate lexeme; an invalid radix stays whole.
#[test]
fn explicit_radix_numbers_stop_before_word_operators() {
    for (dialect, source, number, operator) in [
        ("tcl8.6", "0x1ne 1", "0x1", "ne"),
        ("tcl8.6", "0xfne 1", "0xf", "ne"),
        ("tcl8.6", "0xffin {255}", "0xff", "in"),
        ("tcl9.0", "0xfge 15", "0xf", "ge"),
        ("tcl8.6", "0b1ne 1", "0b1", "ne"),
        ("tcl8.6", "0o7in {7}", "0o7", "in"),
        ("tcl8.6", "0b1ni {1}", "0b1", "ni"),
        ("tcl9.0", "0d9lt 10", "0d9", "lt"),
        ("tcl9.0", "0d9le 9", "0d9", "le"),
        ("tcl9.0", "0d9gt 8", "0d9", "gt"),
        ("tcl9.0", "0d9ge 9", "0d9", "ge"),
    ] {
        let tokens = tokenise_expr(source, Some(dialect));
        assert_eq!(
            tokens[0].kind,
            ExprTokenType::Number,
            "{dialect}: {source}: {tokens:?}"
        );
        assert_eq!(tokens[0].text, number, "{dialect}: {source}: {tokens:?}");
        assert_eq!(tokens[1].text, operator, "{dialect}: {source}: {tokens:?}");
    }
    for (dialect, source) in [
        ("tcl8.6", "0o8ne 1"),
        ("tcl8.6", "0d9ne 1"),
        ("tcl9.0", "0b2ne 1"),
    ] {
        let tokens = tokenise_expr(source, Some(dialect));
        assert_eq!(
            tokens[0].text,
            source.split_whitespace().next().unwrap(),
            "{dialect}: {tokens:?}"
        );
    }
}

#[test]
fn ordinary_number_barewords_stay_whole_after_a_nonzero_offset() {
    for source in ["0 + 1_eq", "0 + 12x"] {
        let expected = source.split(" + ").nth(1).unwrap();
        let tokens = tokenise_expr(source, Some("tcl9.0"));
        assert!(
            tokens.iter().any(|token| token.text == expected),
            "{source}: {tokens:?}"
        );
    }
}

#[test]
fn completed_nan_payload_is_a_number_before_following_bareword() {
    let tokens = tokenise_expr("NaN(1)x", Some("tcl9.0"));
    assert_eq!(tokens[0].kind, ExprTokenType::Number, "{tokens:?}");
    assert_eq!(tokens[0].text, "NaN(1)");
    assert_eq!(tokens[1].text, "x");
    assert_eq!(
        tokenise_expr("NaNx", Some("tcl9.0"))[0].kind,
        ExprTokenType::Function
    );
}

/// `_` inside a *fraction* changes the lexeme boundary, so unlike the
/// integer run it has to follow the release. C's bareword scan takes
/// `[A-Za-z0-9_]` and stops at `.`, which is why `1_0` is a bareword on
/// 8.6 while `1.0_2` is an `invalid character "_"` there — pinned on
/// tclsh 8.6.16 and 9.0.4.
#[test]
fn fraction_separators_follow_the_release() {
    // 9.0: one numeral.
    let t = tokenise_expr("1.0_2", Some("tcl9.0"));
    assert_eq!(t.len(), 1, "9.0 lexes `1.0_2` whole: {t:?}");
    assert_eq!(t[0].text, "1.0_2");

    // 8.6: the number ends at `1.0`, and `_` is its own (unknown) lexeme.
    let t = tokenise_expr("1.0_2", Some("tcl8.6"));
    assert!(t.len() > 1, "8.6 must split `1.0_2`: {t:?}");
    assert_eq!(t[0].text, "1.0");

    // The exponent behaves the same way.
    assert_eq!(tokenise_expr("1e1_0", Some("tcl9.0")).len(), 1);

    // The *integer* run absorbs `_` on every release — `1_0` is one
    // bareword candidate on 8.6 too, which is what C reports.
    assert_eq!(tokenise_expr("1_0", Some("tcl8.6"))[0].text, "1_0");
    assert_eq!(tokenise_expr("1_0", Some("tcl9.0"))[0].text, "1_0");
}

/// `_` never starts a lexeme, on any release — `ParseLexeme`'s final gate
/// is `if (!TclIsBareword(*start) || *start == '_')`, which yields INVALID
/// rather than BAREWORD (9.0.4 `tclCompExpr.c:2136`, 8.6.16 `:2068`,
/// 8.5.19 `:1980`; 8.4.20 `tclParseExpr.c:1852` requires `isalpha` to
/// start a lexeme at all). Verified on tclsh 8.4.20, 8.5.19, 8.6.16,
/// 9.0.4 and 9.1b0: every one reports `_`, `_1`, `_abc`, `1+_2` and
/// `$x + _y` as an invalid *character*, never a bareword.
#[test]
fn underscore_never_starts_a_lexeme() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for src in ["_", "_1", "_abc", "__"] {
            let (toks, unknown) = tokenise_expr_checked(src, Some(dialect));
            assert!(
                unknown,
                "{dialect}: `{src}` must be an invalid character, got {toks:?}"
            );
        }
        // …but `_` *inside* a bareword that starts alphanumeric is fine:
        // `abc_1` and `a__b` are barewords on every release.
        assert_eq!(tokenise_expr("abc_1", Some(dialect))[0].text, "abc_1");
        assert_eq!(tokenise_expr("a__b", Some(dialect))[0].text, "a__b");
    }
}

/// The word-operator set is release-dependent, and that moves the lexeme
/// boundary *where a bareword byte follows the operator*. tclsh-verified:
/// `1 lt_ 2` is `invalid bareword "lt_"` on 8.5/8.6 (no `lt` lexeme before
/// TIP 461) but `invalid character "_"` on 9.0/9.1, and `1 lt2 2` is
/// `invalid bareword "lt2"` on 8.6 against `missing operator` on 9.0.
/// `eq` exists throughout, so `eq_` splits on every release.
#[test]
fn word_operator_availability_moves_the_boundary() {
    // `lt` is 9.0+: before it, `lt_` / `lt2` are one bareword.
    for old in ["tcl8.5", "tcl8.6"] {
        for src in ["1 lt_ 2", "1 lt2 2"] {
            let t = tokenise_expr(src, Some(old));
            assert!(
                t[2].text.starts_with("lt") && t[2].text.len() > 2,
                "{old} must keep `{src}`'s `lt…` whole: {t:?}"
            );
        }
    }
    for new in ["tcl9.0", "tcl9.1"] {
        let t = tokenise_expr("1 lt_ 2", Some(new));
        assert_eq!(t[2].text, "lt", "{new} must split `lt_`: {t:?}");
    }
    // `eq` is in every modelled release, so `eq_` splits in all of them.
    for d in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert_eq!(tokenise_expr("1 eq_ 2", Some(d))[2].text, "eq");
    }
    // C guards the trailing side with `isalpha` only, so a digit after the
    // operator still leaves it an operator: `1 eq2` is `1 eq 2` (tclsh: 0).
    let t = tokenise_expr("1 eq2", Some("tcl9.0"));
    assert_eq!(t[2].text, "eq", "a digit must not glue onto `eq`: {t:?}");
}

/// The release gate must NOT fire where it changes nothing about the
/// boundary — a word operator followed by whitespace is one lexeme in
/// every release, and the version check belongs above the lexer (the
/// parser reports `invalid bareword "lt"` under 8.6, which is what tclsh
/// prints). W003 reads these tokens to say "this operator is Tcl 9.0+
/// (TIP 461)", so suppressing them here silenced the diagnostic on exactly
/// the dialects it targets — nine e2e tests caught it.
#[test]
fn a_standalone_word_operator_stays_visible_to_the_analyser() {
    for d in ["tcl8.4", "tcl8.5", "tcl8.6", "f5-irules"] {
        for (src, op) in [("$a lt $b", "lt"), ("$a in $b", "in"), ("1 ge 2", "ge")] {
            let t = tokenise_expr(src, Some(d));
            assert!(
                t.iter()
                    .any(|k| k.kind == ExprTokenType::Operator && k.text == op),
                "{d}: `{src}` must still expose `{op}` as an Operator token: {t:?}"
            );
        }
    }
}

/// A numeric run butted against bareword characters is ONE bareword —
/// C rescans `[A-Za-z0-9_]*` from the start rather than emitting a number
/// (`tclCompExpr.c:2080-2126`). Two exceptions keep it a number: a
/// non-bareword character in the run (`1.5abc` names only `abc`), and a
/// following binary word operator (`1eq 2` is `1 eq 2`). All tclsh-checked
/// on 8.5, 8.6, 9.0 and 9.1.
#[test]
fn a_number_against_barewords_is_one_bareword() {
    for d in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for src in ["1_eq", "1abc", "12x", "1e_0", "9_ne", "1_2_eq", "1_abc"] {
            let t = tokenise_expr(src, Some(d));
            assert_eq!(t[0].text, src, "{d}: `{src}` must stay one lexeme: {t:?}");
        }
        // Exception 1 — the run holds a `.`, which no bareword can.
        let t = tokenise_expr("1.5abc", Some(d));
        assert_eq!(
            t[0].text, "1.5",
            "{d}: `1.5abc` splits at the double: {t:?}"
        );
        // Exception 2 — a binary word operator follows.
        let t = tokenise_expr("1eq 2", Some(d));
        assert_eq!(t[0].text, "1", "{d}: `1eq 2` is `1 eq 2`: {t:?}");
        assert_eq!(t[1].text, "eq");
    }
    // Release-sensitive: `lt` is not an operator before 9.0, so `1lt 2`
    // is one bareword there and a number + operator after.
    assert_eq!(tokenise_expr("1lt 2", Some("tcl8.6"))[0].text, "1lt");
    assert_eq!(tokenise_expr("1lt 2", Some("tcl9.0"))[0].text, "1");
}
