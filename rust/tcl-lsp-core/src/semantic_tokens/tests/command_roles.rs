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

//! Roles the registry gives argument words: options, option values, enum members and subcommand keywords.

use super::*;

#[test]
fn known_option_classified_as_decorator() {
    // `regexp -nocase {pat} $s` — `-nocase` is a real option → decorator.
    let ks = kinds("regexp -nocase {pat} $s\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::Decorator as u32)), "{ks:?}");
    // `puts -foo` — `-foo` is not an option of `puts` → not a decorator.
    let ks = kinds("puts -foo\n", tcl(), &reg());
    assert!(!ks.contains(&(TokenKind::Decorator as u32)), "{ks:?}");
}

#[test]
fn abbreviated_option_classified_as_decorator() {
    // Tcl option parsing accepts unique prefixes: `lsort -inc` ⇒
    // `-increasing`, `lsearch -ex` ⇒ `-exact`.
    for src in ["lsort -inc {3 1 2}\n", "lsearch -ex {a b} b\n"] {
        let ks = kinds(src, tcl(), &reg());
        assert!(
            ks.contains(&(TokenKind::Decorator as u32)),
            "expected decorator for {src:?}; got {ks:?}"
        );
    }
    // An ambiguous prefix (`lsort -i` → -index/-indices/-integer/…) is not
    // a recognised option and stays a string.
    let ks = kinds("lsort -i {3 1 2}\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "ambiguous prefix must not be a decorator; got {ks:?}"
    );
}

#[test]
fn subcommand_option_classified_as_decorator() {
    // `file delete -force filename`.  `-force`
    // is declared on the `delete` *subcommand* (not on `file` itself), so
    // it is only recognised once subcommand options are consulted.
    let ks = kinds("file delete -force filename\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -force decorator; got {ks:?}"
    );
    // A subcommand option on a different subcommand: `file link -symbolic`.
    let ks = kinds("file link -symbolic a b\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -symbolic decorator; got {ks:?}"
    );
    // A `-`-word that is not a declared option stays a plain string, even
    // on a command that has subcommand options elsewhere.
    let ks = kinds("file delete -bogus filename\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-bogus is not a real option; got {ks:?}"
    );
    // A `-$var` substitution word must never be treated as an option.
    let ks = kinds("file delete -$flag filename\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-$flag is a substitution, not an option; got {ks:?}"
    );
}

#[test]
fn unknown_head_options_classified_generically() {
    // The ngspice / ticklecharts pattern: `$chart Xaxis -name
    // {v(anode), V} -type value -min 0.4` — the head `$chart` is an object
    // handle, unknown to the registry, so its `-switch value` pairs are
    // highlighted by the generic heuristic.
    let ks = kinds(
        "$chart Xaxis -name {v} -type value -min 0.4\n",
        tcl(),
        &reg(),
    );
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -name/-type/-min decorators on an unknown head; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::OptionValue as u32)),
        "expected option values on an unknown head; got {ks:?}"
    );
    // A negative number is not an option: `$obj move -5 10`.
    let ks = kinds("$obj move -5 10\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-5 is a negative number, not an option; got {ks:?}"
    );
    // A `-$var` substitution word is not an option.
    let ks = kinds("$obj configure -$flag v\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-$flag is a substitution, not an option; got {ks:?}"
    );
    // A known command keeps the strict declared-option behaviour: `puts
    // -foo` stays a string even though the generic pass exists.
    let ks = kinds("puts -foo\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "a known command's undeclared -foo must stay a string; got {ks:?}"
    );
    // A plain bareword unknown head is a (possibly user-defined) command
    // name, not a computed dispatch: `mycmd -foo bar` stays a string so a
    // user proc's argument is not mistaken for an option.
    let ks = kinds("mycmd -foo bar\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "a bareword user command's -foo must stay a string; got {ks:?}"
    );
    // Negative special-float literals are numbers, not options: `-inf`,
    // `-Inf`, `-nan` all start with a letter but Tcl parses them as values.
    let ks = kinds("$obj set -inf\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-inf is a negative float literal, not an option; got {ks:?}"
    );
    let ks = kinds("$obj set -NaN\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-NaN is a float literal, not an option; got {ks:?}"
    );
}

#[test]
fn generic_option_scan_stops_at_double_dash() {
    // Tcl's `--` ends option processing: `-real` before it is an option,
    // the `--` is the marker, and `-literal` after it is a positional
    // operand (a plain string), not an option.
    // Columns: `-real` at 9, `1` at 15, `--` at 17, `-literal` at 20.
    let mut deco: Vec<u32> = decode_full("$obj cfg -real 1 -- -literal\n", tcl(), &reg())
        .iter()
        .filter(|(_, _, _, k, _)| *k == TokenKind::Decorator as u32)
        .map(|&(_, c, _, _, _)| c)
        .collect();
    deco.sort_unstable();
    // Exactly `-real` (9) and `--` (17) — never `-literal` (20).
    assert_eq!(
        deco,
        vec![9, 17],
        "expected -real and -- as the only decorators, not -literal after --; got {deco:?}"
    );
}

#[test]
fn value_taking_option_value_classified_as_option_value() {
    // `lsort -index 2 $l` — `-index` takes a value, so the literal `2` is
    // an option value (distinct from the `-index` decorator).
    let ks = kinds("lsort -index 2 $l\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -index decorator; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::OptionValue as u32)),
        "expected the `2` value to be an OptionValue; got {ks:?}"
    );
    // A boolean option takes no value — the following word is not recoloured.
    // `lsort -unique $l`: `$l` stays a variable, not an OptionValue.
    let ks = kinds("lsort -unique $l\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::OptionValue as u32)),
        "boolean -unique must not mark a following value; got {ks:?}"
    );
    // A `$var` value keeps its variable highlight, not OptionValue.
    let ks = kinds("lsort -index $i $l\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::OptionValue as u32)),
        "a $var option value keeps its variable highlight; got {ks:?}"
    );
}

#[test]
fn argparse_global_switch_values_classified_as_option_value() {
    // The `argparse` package command's value-taking global switches
    // (`-template`, `-level`, …) colour their following literal as an
    // OptionValue, while boolean switches (`-inline`) do not. argparse is
    // registered (package-gated), so the classifier resolves its spec.
    let ks = kinds(
        "argparse -template foo -level 2 -inline {d}\n",
        tcl(),
        &reg(),
    );
    let n_val = ks
        .iter()
        .filter(|&&k| k == TokenKind::OptionValue as u32)
        .count();
    assert_eq!(
        n_val, 2,
        "expected `foo` and `2` as OptionValues (not boolean -inline's `{{d}}`); got {ks:?}"
    );
    // A boolean global switch does not recolour the following word.
    let ks = kinds("argparse -inline {d}\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::OptionValue as u32)),
        "boolean -inline must not mark a following value; got {ks:?}"
    );
}

#[test]
fn subcommand_enum_value_classified_as_enum_member() {
    // `string is alnum $s` — `alnum` is a closed-set value declared on
    // the `is` subcommand → enumMember, not a plain string.
    let ks = kinds("string is alnum $s\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::EnumMember as u32)),
        "expected an enumMember token; got {ks:?}"
    );
    // A value not in the set stays a string.
    let ks = kinds("string is bogusclass $s\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::EnumMember as u32)),
        "bogusclass is not a class; got {ks:?}"
    );
}

#[test]
fn return_code_option_classified_as_decorator_issue_967() {
    // `return -code error "bad"` must not highlight `-code` as a plain
    // string. `-code` is a declared OptionSpec
    // on `return` (a decorator) and `error` is one of its closed-set
    // values (an enumMember); `"bad"` stays a plain string.
    let ks = kinds("return -code error \"bad\"\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -code to be a decorator; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::EnumMember as u32)),
        "expected error to be an enumMember; got {ks:?}"
    );

    // `-level` is likewise a declared option (Tcl 8.5+) and its value
    // (`0`) is an OptionValue, not a plain number/string.
    let ks = kinds("return -level 0 \"bad\"\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -level to be a decorator; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::OptionValue as u32)),
        "expected 0 to be an OptionValue; got {ks:?}"
    );

    // `-options $opts` — `-options` is a decorator and its dict value
    // stays highlighted as the variable it is (not recoloured away).
    let ks = kinds("return -options $opts\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -options to be a decorator; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::Variable as u32)),
        "expected $opts to keep its variable highlight; got {ks:?}"
    );

    // TN: a plain word `-code` passed to a command with no declared
    // OptionSpec (`concat` takes no options at all) must not be painted
    // as an option — it is just a string argument.
    let ks = kinds("concat -code error\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-code is not a declared option of concat; got {ks:?}"
    );
}

#[test]
fn info_object_class_sub_subcommand_classified_as_keyword() {
    // In `info object class $obj`, the `class` word is a
    // second-level subcommand (OBJECT INTROSPECTION), not a string. Both the
    // first-level `object` and the second-level `class` must read as
    // keywords (the `info` head itself is a Function).
    let kind_at = |src: &str, col: u32| -> u32 {
        decode_full(src, tcl(), &reg())
            .into_iter()
            .find(|&(_, c, _, _, _)| c == col)
            .map_or_else(
                || panic!("no token at column {col} in {src:?}"),
                |(_, _, _, k, _)| k,
            )
    };

    // `info object class $obj` — column 12 is `class`.
    let src = "info object class $obj\n";
    assert_eq!(
        kind_at(src, 12),
        TokenKind::Keyword as u32,
        "`class` sub-subcommand should be a keyword"
    );

    // `info class superclasses $cls` — column 11 is `superclasses`.
    let src = "info class superclasses $cls\n";
    assert_eq!(
        kind_at(src, 11),
        TokenKind::Keyword as u32,
        "`superclasses` sub-subcommand should be a keyword"
    );

    // A non-subcommand third word stays a string: `info object frobnicate`
    // — `frobnicate` is not a recognised OBJECT INTROSPECTION operation.
    let src = "info object frobnicate $obj\n";
    assert_eq!(
        kind_at(src, 12),
        TokenKind::String as u32,
        "an unknown third word must stay a string, not a keyword"
    );

    // The issue's actual form: `info object class` nested inside a command
    // substitution within an `if` expression. Highlighting must recurse into
    // the bracketed inner command, not just top-level statements. The column
    // of `class` is located dynamically to stay robust.
    let src = "if {([info object class $element {::Foo::Analysis}]) && ([info exists C])} {\n}\n";
    let col = u32::try_from(src.find(" class ").expect("has ` class `") + 1).unwrap();
    assert_eq!(
        kind_at(src, col),
        TokenKind::Keyword as u32,
        "`class` must highlight as a keyword even nested in an if-expr command substitution"
    );

    // Unique-prefix abbreviation: `info object cl` is Tcl's
    // abbreviation of `class`; column 12 is `cl`.
    let src = "info object cl $obj\n";
    assert_eq!(
        kind_at(src, 12),
        TokenKind::Keyword as u32,
        "a unique-prefix sub-subcommand should highlight as a keyword"
    );
    // An ambiguous prefix stays a string: `info class c` matches both
    // `call` and `constructor`, so it must not be painted a keyword.
    let src = "info class c $cls\n";
    assert_eq!(
        kind_at(src, 11),
        TokenKind::String as u32,
        "an ambiguous prefix must not highlight as a keyword"
    );
}

#[test]
fn subcommand_prefix_resolution_is_dialect_aware() {
    let kind_at = |src: &str, dialect: &'static tcl_dialect::DialectProfile, col: u32| -> u32 {
        decode_full(src, dialect, &reg())
            .into_iter()
            .find(|&(_, c, _, _, _)| c == col)
            .map_or_else(
                || panic!("no token at col {col} in {src:?}"),
                |(_, _, _, k, _)| k,
            )
    };
    // `string rev` is `reverse` (added 8.5): a keyword in 8.6, but an
    // unknown word in 8.4 where `reverse` does not exist (verified: tclsh8.4
    // rejects `string rev`).  Column 7 is `rev`.
    let src = "string rev abc\n";
    assert_eq!(
        kind_at(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            7
        ),
        TokenKind::Keyword as u32
    );
    assert_eq!(
        kind_at(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
            7
        ),
        TokenKind::String as u32,
        "`string rev` is not a subcommand in 8.4"
    );

    // `info class def`: uniquely `definition` in 8.6 (keyword), but
    // ambiguous with `definitionnamespace` in 9.0 (verified against tclsh)
    // → stays a string.  Column 11 is `def`.
    let src = "info class def ::C\n";
    assert_eq!(
        kind_at(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            11
        ),
        TokenKind::Keyword as u32,
        "`info class def` is `definition` in 8.6"
    );
    assert_eq!(
        kind_at(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            11
        ),
        TokenKind::String as u32,
        "`info class def` is ambiguous in 9.0"
    );
}

#[test]
fn abbreviated_first_level_subcommand_highlights_as_keyword() {
    // `string le $s` — `le` is Tcl's unique-prefix abbreviation of
    // `length`; it must highlight as a subcommand keyword (column 7).
    let src = "string le $s\n";
    let toks = decode_full(src, tcl(), &reg());
    let kind = toks
        .into_iter()
        .find(|&(_, c, _, _, _)| c == 7)
        .map(|(_, _, _, k, _)| k)
        .expect("token at col 7");
    assert_eq!(
        kind,
        TokenKind::Keyword as u32,
        "abbreviated subcommand `le` should be a keyword"
    );
    // An ambiguous prefix (`string t`) stays a string.
    let src = "string t $s\n";
    let kind = decode_full(src, tcl(), &reg())
        .into_iter()
        .find(|&(_, c, _, _, _)| c == 7)
        .map(|(_, _, _, k, _)| k)
        .expect("token at col 7");
    assert_eq!(
        kind,
        TokenKind::String as u32,
        "ambiguous prefix `t` must not be a keyword"
    );
}

#[test]
fn oo_define_inline_keyword_classified_as_keyword() {
    // `oo::define Cls method foo {} {}` — the inline `method` keyword sits
    // at an argument position and must read as a keyword, not a string.
    let ks = kinds("oo::define Cls method foo {} {}\n", tcl(), &reg());
    let n_kw = ks
        .iter()
        .filter(|&&k| k == TokenKind::Keyword as u32)
        .count();
    // Two keyword tokens: the `oo::define` head and the inline `method`.
    assert!(n_kw >= 2, "expected >=2 keyword tokens; got {ks:?}");
    // `oo::define Cls self method foo {} {}` — the inner keyword after
    // `self` is highlighted too.
    let ks = kinds("oo::define Cls self method foo {} {}\n", tcl(), &reg());
    let n_kw = ks
        .iter()
        .filter(|&&k| k == TokenKind::Keyword as u32)
        .count();
    assert!(
        n_kw >= 3,
        "expected >=3 keyword tokens (head+self+method); got {ks:?}"
    );
}

#[test]
fn option_enum_value_is_enum_member() {
    // `button .b -relief raised` — the closed-set option value is coloured
    // as an EnumMember, not a generic OptionValue.
    let toks = decode_full(
        "button .b -relief raised\n",
        crate::profile_for_dialect("tk"),
        &reg(),
    );
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::EnumMember as u32),
        "expected `raised` as EnumMember; got {toks:?}"
    );
}

#[test]
fn option_textvariable_value_is_variable_declaration() {
    // `entry .e -textvariable myvar` — the value names a variable the widget
    // reads/writes (`ArgRole::VarWrite`), so it is a Variable
    // declaration, not a plain `OptionValue` string.
    let toks = decode_full(
        "entry .e -textvariable myvar\n",
        crate::profile_for_dialect("tk"),
        &reg(),
    );
    assert!(
        toks.iter()
            .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION),
        "expected myvar as a variable declaration; got {toks:?}"
    );
}
