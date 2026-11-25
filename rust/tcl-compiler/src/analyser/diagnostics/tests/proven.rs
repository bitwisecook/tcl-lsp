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

//! Checks that read proven words: the literal-only checks, index sites over proven containers and W127.

use super::*;

/// The literal-only checks read proven words: each program's
/// checked word is a variable the lattice proves at the call, and the check
/// reports there — at the word the user wrote, once — where it had
/// abstained; the same call over an unknown value (a parameter) draws
/// nothing. W147 reports over the conflicting options, from the first to
/// the proven one, as the literal form reports over its pair, and the index
/// checks at the literal index a proven list or string makes checkable.
/// One row of [`literal_only_checks_read_proven_words`]: the code, the
/// dialect, the program whose checked word is proven, the same call over an
/// unknown value, and the text the finding lands on.
type ProvenRow = (
    DiagCode,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

/// The rows of [`literal_only_checks_read_proven_words`].
const PROVEN_ROWS: [ProvenRow; 14] = [
    (
        DiagCode::W121,
        "f5-irules",
        "when CLIENT_ACCEPTED {\n    set ip [IP::client_addr]\n    set m 255.0\n    \
             append m .255.0\n    IP::addr $ip mask $m\n}\n",
        "when CLIENT_ACCEPTED {\n    set ip [IP::client_addr]\n    \
             IP::addr $ip mask [IP::client_addr]\n}\n",
        "$m",
    ),
    (
        DiagCode::W127,
        "tk",
        "proc p {} {\n    set r bogus\n    button .b -relief $r\n}\n",
        "proc p {r} {\n    button .b -relief $r\n}\n",
        "$r",
    ),
    (
        DiagCode::W137,
        "tcl8.6",
        "proc p {config} {\n    set c dict\n    string is $c $config\n}\n",
        "proc p {c config} {\n    string is $c $config\n}\n",
        "$c",
    ),
    (
        DiagCode::W138,
        "tcl8.5",
        "proc p {mask} {\n    set f {flags: %b}\n    format $f $mask\n}\n",
        "proc p {f mask} {\n    format $f $mask\n}\n",
        "$f",
    ),
    (
        DiagCode::W141,
        "tcl8.6",
        "proc p {path} {\n    set es {CALL load extra}\n    \
             return -code error -errorstack $es \"cannot read $path\"\n}\n",
        "proc p {path es} {\n    return -code error -errorstack $es \"cannot read $path\"\n}\n",
        "$es",
    ),
    (
        DiagCode::W145,
        "tcl8.6",
        "proc p {l} {\n    set o -in\n    lsort $o $l\n}\n",
        "proc p {o l} {\n    lsort $o $l\n}\n",
        "$o",
    ),
    (
        DiagCode::W146,
        "tcl8.6",
        "proc logChange {args} {}\nproc p {} {\n    set ops {read rename write}\n    \
             trace add variable ::config(port) $ops logChange\n}\n",
        "proc logChange {args} {}\nproc p {ops} {\n    \
             trace add variable ::config(port) $ops logChange\n}\n",
        "$ops",
    ),
    (
        DiagCode::W147,
        "tcl9.0",
        "proc p {} {\n    set o -path\n    glob -directory root $o prefix *.tcl\n}\n",
        "proc p {o} {\n    glob -directory root $o prefix *.tcl\n}\n",
        "-directory root $o",
    ),
    (
        DiagCode::W152,
        "tcl8.6",
        "package require http\nproc p {cb} {\n    set opt -queryprogress\n    \
             ::http::geturl http://example.invalid/ $opt $cb\n}\n",
        "package require http\nproc p {opt cb} {\n    \
             ::http::geturl http://example.invalid/ $opt $cb\n}\n",
        "$opt",
    ),
    (
        DiagCode::W200,
        "tcl8.4",
        "proc p {} {\n    set t cu\n    binary format $t 5\n}\n",
        "proc p {t} {\n    binary format $t 5\n}\n",
        "$t",
    ),
    (
        DiagCode::W202,
        "tcl8.4",
        "proc p {b} {\n    set t t\n    binary scan $b $t v\n}\n",
        "proc p {b t} {\n    binary scan $b $t v\n}\n",
        "$t",
    ),
    (
        DiagCode::W230,
        "tcl8.6",
        "proc p {} {\n    set l {a b c}\n    lindex $l 9\n}\n",
        "proc p {l} {\n    lindex $l 9\n}\n",
        "9",
    ),
    (
        DiagCode::W232,
        "tcl8.6",
        "proc p {} {\n    set s abc\n    string index $s 9\n}\n",
        "proc p {s} {\n    string index $s 9\n}\n",
        "9",
    ),
    (
        DiagCode::W303,
        "tcl8.6",
        "proc p {s} {\n    set re {^((a+)+)\\1$}\n    regexp $re $s\n}\n",
        "proc p {re s} {\n    regexp $re $s\n}\n",
        "$re",
    ),
];

#[test]
fn literal_only_checks_read_proven_words() {
    for (code, dialect, proven, unknown, word) in PROVEN_ROWS {
        let reported = |src: &str| -> Vec<String> {
            Analyser::new()
                .analyse(src, dialect)
                .diagnostics
                .iter()
                .filter(|d| d.code == code)
                .map(|d| src[d.span.as_range()].to_owned())
                .collect()
        };
        assert_eq!(reported(proven), [word], "{code}: {proven}");
        assert_eq!(reported(unknown), Vec::<String>::new(), "{code}: {unknown}");
    }
}

/// A call that writes one bad word and reads a proven one reports each once:
/// the walk the written word, the proven-word pass the proven one.
#[test]
fn a_proven_word_is_reported_once_beside_a_written_one() {
    let both = "proc p {} {\n    set m 255.0.255.0\n    list 255.0.255.0 $m\n}\n";
    let reported: Vec<String> = Analyser::new()
        .analyse(both, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W121 && d.span.start() > 20)
        .map(|d| both[d.span.as_range()].to_owned())
        .collect();
    assert_eq!(reported, ["255.0.255.0", "255.0.255.0", "$m"], "{both}");
}

/// The source text each `code` finding of `src` is anchored at.
fn index_findings(src: &str, code: DiagCode) -> Vec<String> {
    Analyser::new()
        .analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code == code)
        .map(|d| src[d.span.as_range()].to_owned())
        .collect()
}

/// A `$var` index the interval checks bound is theirs, anchored at the
/// statement; the proven-word re-run reports only the sites they cannot
/// bound — a `split` list, a statement-level `string index`.
#[test]
fn an_index_site_is_reported_once() {
    let rows: [(&str, DiagCode, &str); 6] = [
        (
            "set l {a b c}\nset i 9\nlindex $l $i\n",
            DiagCode::W230,
            "lindex $l $i",
        ),
        (
            "set l [list a b c]\nset i 9\nlindex $l $i\n",
            DiagCode::W230,
            "lindex $l $i",
        ),
        (
            "set l {a b c}\nset i 9\nputs [lindex $l $i]\n",
            DiagCode::W230,
            "puts [lindex $l $i]",
        ),
        (
            "set l [split a,b,c ,]\nset i 9\nlindex $l $i\n",
            DiagCode::W230,
            "$i",
        ),
        (
            "set s abc\nset i 9\nstring index $s $i\n",
            DiagCode::W232,
            "$i",
        ),
        (
            "set s abc\nset i 9\nputs [string index $s $i]\n",
            DiagCode::W232,
            "puts [string index $s $i]",
        ),
    ];
    for (src, code, anchor) in rows {
        assert_eq!(index_findings(src, code), [anchor], "{code}: {src}");
    }
}

/// A call nested in a command substitution is checked over the value its
/// host proves, wherever the host sits: a call's words, an assignment's, a
/// branch condition or a `return` value.
#[test]
fn a_nested_index_reads_the_proven_container() {
    let rows: [(&str, DiagCode); 9] = [
        ("set l {a b c}\nputs [lindex $l 9]\n", DiagCode::W230),
        ("set l [list a b c]\nputs [lindex $l 9]\n", DiagCode::W230),
        (
            "set l [split a,b,c ,]\nputs [lindex $l 9]\n",
            DiagCode::W230,
        ),
        (
            "set l {}\nlappend l a b c\nputs [lindex $l 9]\n",
            DiagCode::W230,
        ),
        ("set l {a b c}\nset x [lindex $l 9]\n", DiagCode::W230),
        (
            "set l {a b c}\nif {[lindex $l 9] eq {}} {puts e}\n",
            DiagCode::W230,
        ),
        (
            "proc p {} {\n    set l {a b c}\n    return [lindex $l 9]\n}\n",
            DiagCode::W230,
        ),
        ("set s abc\nputs [string index $s 9]\n", DiagCode::W232),
        (
            "set s abc\nputs \"at [string index $s 9]\"\n",
            DiagCode::W232,
        ),
    ];
    for (src, code) in rows {
        assert_eq!(index_findings(src, code), ["9"], "{code}: {src}");
    }
}

/// A container written as a command substitution, or built by one into a
/// variable, is checked over the value the registry's folds prove (#2438,
/// #2437), at every read: reading a range built from a literal list runs no
/// object method the module cannot see. An index inside it is not reported.
#[test]
fn a_container_built_by_a_command_is_checked() {
    let rows: [(&str, DiagCode, &[&str]); 10] = [
        ("lindex [list a b c] 9\n", DiagCode::W230, &["9"]),
        ("puts [lindex [list a b c] 9]\n", DiagCode::W230, &["9"]),
        (
            "string index [string repeat a 3] 9\n",
            DiagCode::W232,
            &["9"],
        ),
        ("set l [list a b c]\nlindex $l 9\n", DiagCode::W230, &["9"]),
        (
            "set l [lrange {a b c d e} 0 2]\nlindex $l 9\n",
            DiagCode::W230,
            &["9"],
        ),
        (
            "set l [lrange {a b c d e} 0 2]\nlindex $l 9\nputs [lindex $l 9]\n",
            DiagCode::W230,
            &["9", "9"],
        ),
        (
            "llength [lrange {a b c d e} 0 2]\nlindex {a b c} 9\n",
            DiagCode::W230,
            &["9"],
        ),
        ("lindex [list a b c] 2\n", DiagCode::W230, &[]),
        ("string index [string repeat a 3] 2\n", DiagCode::W232, &[]),
        ("lindex [list a $argv0] 9\n", DiagCode::W230, &[]),
    ];
    for (src, code, anchors) in rows {
        assert_eq!(index_findings(src, code), anchors, "{code}: {src}");
    }
}

/// An `lrepeat` result is an ordinary list (8.5–9.0 `Tcl_LrepeatObjCmd`, 9.1
/// `Tcl_ListObjRepeat` below 100 elements), so reading it twice runs no object
/// method the module cannot see and both reads are checked (#2437).
#[test]
fn a_repeated_list_is_checked_at_every_read() {
    let src = "set l [lrepeat 3 a]\nlindex $l 9\nputs [lindex $l 9]\n";
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let findings = Analyser::new()
            .analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W230)
            .map(|d| src[d.span.as_range()].to_owned())
            .collect::<Vec<_>>();
        assert_eq!(findings, ["9", "9"], "{dialect}");
    }
}

/// A reversed literal is an ordinary list on 8.5–9.0, so both reads are checked.
/// 9.1 reverses a literal of unknown intrep through its abstract
/// `lreverseType`, but copies a list proved current (#2437).
#[test]
fn a_reversed_list_is_checked_at_every_read() {
    let w230 = |src: &str, dialect: &str| {
        Analyser::new()
            .analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W230)
            .map(|d| src[d.span.as_range()].to_owned())
            .collect::<Vec<_>>()
    };
    let literal = "set l [lreverse {a b c}]\nlindex $l 9\nputs [lindex $l 9]\n";
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        assert_eq!(w230(literal, dialect), ["9", "9"], "{dialect}");
    }
    let built = "set l [lreverse [list a b c]]\nlindex $l 9\nputs [lindex $l 9]\n";
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert_eq!(w230(built, dialect), ["9", "9"], "{dialect}");
    }
}

/// `concat` and `join` build a fresh stock string, which the first index read
/// converts through the ordinary parser, so the second read is checked too (#2437).
#[test]
fn a_concatenated_or_joined_string_is_checked_at_every_read() {
    for src in [
        "set l [concat {a b} {c}]\nlindex $l 9\nputs [lindex $l 9]\n",
        "set l [join {a b c} { }]\nlindex $l 9\nputs [lindex $l 9]\n",
    ] {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let findings = Analyser::new()
                .analyse(src, dialect)
                .diagnostics
                .iter()
                .filter(|d| d.code == DiagCode::W230)
                .map(|d| src[d.span.as_range()].to_owned())
                .collect::<Vec<_>>();
            assert_eq!(findings, ["9", "9"], "{dialect}: {src}");
        }
    }
}

/// A plain `lsort` of a literal has known bytes, so its out-of-range reads
/// are checked; a comparator command could run any script and is not (#2437).
#[test]
fn a_sorted_literal_is_checked_at_every_read() {
    let w230 = |src: &str, dialect: &str| {
        Analyser::new()
            .analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W230)
            .map(|d| src[d.span.as_range()].to_owned())
            .collect::<Vec<_>>()
    };
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert_eq!(
            w230("lindex [lsort {c b a}] 9\n", dialect),
            ["9"],
            "{dialect}"
        );
        assert_eq!(
            w230(
                "set l [lsort -decreasing {c b a}]\nlindex $l 9\nputs [lindex $l 9]\n",
                dialect
            ),
            ["9", "9"],
            "{dialect}"
        );
        assert_eq!(
            w230("lindex [lsort -command cmp {c b a}] 9\n", dialect),
            Vec::<String>::new(),
            "{dialect}"
        );
    }
}

/// C8.6–9.1 compile a constant `list` in a procedure body into one shared
/// literal list object, which every later read finds as stock as a source
/// literal; C8.4/8.5 build a fresh list. Either way both reads are checked (#2437).
#[test]
fn a_compiled_constant_list_is_checked_at_every_read() {
    let src = "proc p {} {\n    set l [list a b]\n    lindex $l 9\n    puts [lindex $l 9]\n}\n";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let findings = Analyser::new()
            .analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W230)
            .map(|d| src[d.span.as_range()].to_owned())
            .collect::<Vec<_>>();
        assert_eq!(findings, ["9", "9"], "{dialect}");
    }
}

/// Over pool-literal operands these builders return a stock-class object: an
/// operand, a part of one, or a new string, list or dictionary. A later
/// literal index conversion keeps that receipt, so both reads are checked (#2437).
#[test]
fn a_stock_built_container_is_checked_at_every_read() {
    for (build, from) in [
        ("lindex {{a b c} x} 0", "tcl8.4"),
        ("split a,b,c ,", "tcl8.4"),
        ("string repeat {a } 3", "tcl8.4"),
        ("string map {, { }} a,b,c", "tcl8.4"),
        ("string trim { a b c }", "tcl8.4"),
        ("dict keys {a 1 b 2}", "tcl8.5"),
        ("dict values {a 1 b 2}", "tcl8.5"),
        ("dict get {k {a b}} k", "tcl8.5"),
    ] {
        let src = format!("set l [{build}]\nlindex $l 9\nputs [lindex $l 9]\n");
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            if dialect < from {
                continue;
            }
            let findings = Analyser::new()
                .analyse(&src, dialect)
                .diagnostics
                .iter()
                .filter(|d| d.code == DiagCode::W230)
                .map(|d| src[d.span.as_range()].to_owned())
                .collect::<Vec<_>>();
            assert_eq!(findings, ["9", "9"], "{dialect}: {build}");
        }
    }
}

/// Converting a variable index keeps the stock objects it may share, so a
/// second read through the same index is checked as the first is (#2387),
/// a loop counter `incr` left numeric included.
#[test]
fn a_repeated_index_read_is_checked_each_time() {
    for src in [
        "set i 5\nlindex {a b c} $i\nlindex {a b c} $i\n",
        "set l {a b c}\nset i 5\nset r [lindex $l $i]\nset s [lindex $l $i]\n",
        "set i 5\nputs \"<[lindex {a b c} $i]>\"\nputs \"<[lindex {a b c} $i]>\"\n",
        "set l {a b c}\nfor {set i 0} {$i < 5} {incr i} {}\nlindex $l $i\nlindex $l $i\n",
        "set l {a b c}\nfor {set i 0} {$i < 5} {incr i} {}\n\
         puts \"<[lindex $l $i]>\"\nset r [lindex $l $i]\n",
    ] {
        assert_eq!(index_findings(src, DiagCode::W230).len(), 2, "{src}");
    }
}

/// No finding where the call is not run by the substitution — a braced
/// word, a script run later — or where an earlier substitution in the same
/// command writes the variable the word reads: `$i` is 0 there, in range.
#[test]
fn a_nested_index_reads_only_what_the_substitution_sees() {
    for src in [
        "set l {a b c}\nputs {[lindex $l 9]}\n",
        "set l {a b c}\nafter 100 {puts [lindex $l 9]}\n",
        "set i 9\nlindex {{a b} c} [set i 0] $i\n",
        "set i 9\nputs [lindex {{a b} c} [set i 0] $i]\n",
        "set l {a b c}\nputs [lindex [set l {a b c d e f g h i j k}] 0][lindex $l 9]\n",
        "proc p {} {\n    set l {a b c}\n    \
             return [lindex [set l {a b c d e f g h i j k}] 0][lindex $l 9]\n}\n",
        "set l {a b c}\nif {[set l {a b c d e f g h i j k}] ne {} && [lindex $l 9] eq {}} {puts e}\n",
    ] {
        assert_eq!(
            index_findings(src, DiagCode::W230),
            Vec::<String>::new(),
            "{src}"
        );
    }
}

#[test]
fn w127_fires_on_invalid_option_enum_value() {
    // `-relief` carries a closed Tk value set; a literal outside it is W127.
    let bad = Analyser::new().analyse("button .b -relief bogus", "tk");
    assert!(
        bad.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W127 && d.message.contains("-relief")),
        "expected W127 on `-relief bogus`; got {:?}",
        bad.diagnostics
    );
    // A member value is accepted.
    let good = Analyser::new().analyse("button .b -relief raised", "tk");
    assert!(
        !good.diagnostics.iter().any(|d| d.code == DiagCode::W127),
        "`raised` is a valid relief; got {:?}",
        good.diagnostics
    );
    // A dynamic value is skipped (can't be checked statically).
    let dynamic = Analyser::new().analyse("button .b -relief $r", "tk");
    assert!(
        !dynamic.diagnostics.iter().any(|d| d.code == DiagCode::W127),
        "dynamic `-relief $r` must be skipped; got {:?}",
        dynamic.diagnostics
    );
}

#[test]
fn w127_fires_on_invalid_subcommand_closed_value() {
    fn has_w127(src: &str) -> bool {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W127)
    }
    // FN fix: `string is <class>` carries an exhaustive class set on the `is`
    // subcommand; a non-member is a runtime `bad class` error.
    assert!(has_w127("string is booleanx 1\n"));
    assert!(has_w127("string is xyz 1\n"));
    // C Tcl accepts a unique prefix — must NOT fire.
    assert!(!has_w127("string is boolean 1\n"));
    assert!(!has_w127("string is boo 1\n"));
    assert!(!has_w127("string is b 1\n"));
    assert!(!has_w127("string is dig 1\n"));
    // FN fix: an *ambiguous* prefix — one matching two or more classes — is a
    // runtime error in C Tcl (only a *unique* prefix abbreviates), so W127 must
    // fire, not silently accept it.
    assert!(has_w127("string is a 1\n")); // alnum / alpha / ascii
    assert!(has_w127("string is d 1\n")); // digit / double
    assert!(has_w127("string is w 1\n")); // wideinteger / wordchar
    // Dynamic class is skipped.
    assert!(!has_w127("string is $c 1\n"));
    // The message names the subcommand.
    let d = Analyser::new().analyse("string is booleanx 1\n", "tcl8.6");
    assert!(
        d.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W127 && d.message.contains("string is")),
        "message must name `string is`; got {:?}",
        d.diagnostics,
    );
}
