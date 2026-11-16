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

//! Commands that store into variables: `regexp` captures, `lassign` and `scan` destructuring, and the keys `dict with` binds.

use super::*;

/// One regexp-owner witness: a [`StorageWitness`] with no subcommand.
type RegexWitness = (
    &'static str,
    &'static [&'static str],
    &'static [(&'static str, &'static str)],
    &'static [&'static str],
);

/// The regexp owner's witnesses, each measured identical on tclsh 8.4.20,
/// 8.5.19, 8.6.18, 9.0.4 and 9.1b0 except where noted; every release
/// answers the first five.
const REGEX_WITNESSES: &[RegexWitness] = &[
    (
        "regexp",
        &["(x)(y)", "zz", "a", "b"],
        &[("a", "before"), ("b", "before")],
        &["a", "b"],
    ),
    (
        "regexp",
        &["-inline", "-indices", "(a)(b)?", "ac"],
        &[],
        &[],
    ),
    ("regexp", &["-all", "a*", "xaax"], &[], &[]),
    ("regsub", &["-all", "", "abc", "-"], &[], &[]),
    ("regexp", &["-about", "a"], &[], &[]),
    ("regexp", &["-about", "(?:a)"], &[], &[]),
    ("regexp", &["-about", "a(b)c"], &[], &[]),
    (
        "regexp",
        &["(a)(b)?", "ac", "m", "g1", "g2"],
        &[],
        &["m", "g1", "g2"],
    ),
    (
        "regexp",
        &["-indices", "(a)(b)?", "ac", "m", "g1", "g2"],
        &[],
        &["m", "g1", "g2"],
    ),
    (
        "regexp",
        &["-all", "(a)", "banana", "m", "g"],
        &[("m", "x")],
        &["m", "g"],
    ),
    (
        "regexp",
        &["-all", "z", "abc", "m"],
        &[("m", "keep")],
        &["m"],
    ),
    ("regexp", &["-nocase", "B+", "abBbc", "m"], &[], &["m"]),
    (
        "regexp",
        &["-start", "2", "-inline", ".", "abcdef"],
        &[],
        &[],
    ),
    (
        "regexp",
        &["-start", "010", "-inline", ".", "abcdefghijkl"],
        &[],
        &[],
    ),
    ("regexp", &["-all", "-inline", "a|(b)", "ab"], &[], &[]),
    // The list rendering differs: `#a a` on 8.4, `{#a} a` from 8.5.
    ("regexp", &["-inline", "#(a)", "#a"], &[], &[]),
    ("regexp", &["-inline", "a", "a", "m"], &[], &[]),
    ("regexp", &["(", "x"], &[], &[]),
    ("regsub", &["-all", "a", "banana", "o", "v"], &[], &["v"]),
    ("regsub", &["z", "abc", "X", "v"], &[("v", "old")], &["v"]),
    ("regsub", &["(b)(c)?", "abd", "[\\2|\\1]"], &[], &[]),
    (
        "regsub",
        &["-command", "a", "abc", "string toupper"],
        &[],
        &[],
    ),
];

/// The regexp owner's witnesses, per release found on `PATH`, each
/// under that release's profile against the real `tclsh`: a no-match leaves
/// its match variables as they were, a match writes them — an unmatched
/// subgroup the empty string, or `-1 -1` with `-indices` — `-inline`
/// answers the list, `-all` counts, `-about` answers the pattern's shape,
/// and `regsub` substitutes and writes its variable whether or not anything
/// matched. When the route answers it must match; when `tclsh` raises the
/// route must decline; a decline where `tclsh` answers is allowed (`-start
/// 010` is 8 up to 8.6 and 10 from 9.0, the `-command` callback form), but
/// the first five witnesses answer under every release.
#[test]
fn regexp_witnesses_match_every_release_on_path() {
    // The first five witnesses, which every release answers.
    const REQUIRED: [usize; 5] = [0, 1, 2, 3, 4];
    let reg = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        let mut agreed = 0usize;
        for (index, &witness) in REGEX_WITNESSES.iter().enumerate() {
            let (command, args, priors, observed) = witness;
            let want = storage_oracle(&tclsh, (command, None, args, priors, observed));
            let got = storage_route(&reg, profile, (command, None, args, priors, observed));
            match (&want, &got) {
                (Some(want), Some(got)) => {
                    assert_eq!(
                        got,
                        want,
                        "tclsh{}: {} {:?}",
                        version.version_string(),
                        witness.0,
                        witness.1
                    );
                    agreed += 1;
                }
                (None, Some(got)) => panic!(
                    "tclsh{} raises on {} {:?}, the route answered {got:?}",
                    version.version_string(),
                    witness.0,
                    witness.1
                ),
                (_, None) => assert!(
                    !REQUIRED.contains(&index),
                    "tclsh{}: the route declined the witness {} {:?}",
                    version.version_string(),
                    witness.0,
                    witness.1
                ),
            }
        }
        assert!(
            agreed >= 18,
            "tclsh{}: only {agreed} witnesses agreed",
            version.version_string()
        );
    }
}

/// The destructuring writers' witnesses: the first four are `scan`'s
/// partial conversion, `lassign`'s repeated variable, `binary scan`'s two
/// fields and `array set`'s two elements, then come the forms each route
/// reads and the ones it declines.
const DESTRUCTURE_WITNESSES: &[StorageWitness] = &[
    (
        "scan",
        None,
        &["12 nope", "%d %d", "a", "b"],
        &[("a", "A"), ("b", "B")],
        &["a", "b"],
    ),
    (
        "lassign",
        None,
        &["first second extra", "a", "a"],
        &[("a", "A")],
        &["a"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{1}\u{2}", "cc", "a", "b"],
        &[],
        &["a", "b"],
    ),
    (
        "array",
        Some("set"),
        &["arr", "k1 v1 k2 v2"],
        &[],
        &["arr(k1)", "arr(k2)"],
    ),
    ("scan", None, &["12 34", "%d %d"], &[], &[]),
    ("scan", None, &["abc", "%d"], &[], &[]),
    ("scan", None, &["12 ab", "%d %d %s"], &[], &[]),
    (
        "scan",
        None,
        &["", "%d %d", "a", "b"],
        &[("a", "A"), ("b", "B")],
        &["a", "b"],
    ),
    (
        "scan",
        None,
        &["hi x", "%s %c%n", "w", "c", "n"],
        &[],
        &["w", "c", "n"],
    ),
    (
        "scan",
        None,
        &["12345", "%2d%3d", "a", "b"],
        &[],
        &["a", "b"],
    ),
    (
        "scan",
        None,
        &["abc123", "%[a-z]%d", "w", "n"],
        &[],
        &["w", "n"],
    ),
    ("scan", None, &["12 34", "%*d %d", "a"], &[], &["a"]),
    (
        "scan",
        None,
        &["1.5 2", "%f %d", "f", "d"],
        &[],
        &["f", "d"],
    ),
    (
        "scan",
        None,
        &["ff 017", "%x %i", "h", "i"],
        &[],
        &["h", "i"],
    ),
    ("scan", None, &["-0xff", "%x", "h"], &[("h", "H")], &["h"]),
    ("scan", None, &["101", "%b", "a"], &[("a", "A")], &["a"]),
    (
        "scan",
        None,
        &["4294967296", "%d", "a"],
        &[("a", "A")],
        &["a"],
    ),
    (
        "scan",
        None,
        &["12 34", "%2$d %1$d", "a", "b"],
        &[],
        &["a", "b"],
    ),
    ("scan", None, &["12", "%d %d", "a"], &[], &[]),
    ("scan", None, &["-1 12", "%u %u"], &[], &[]),
    ("scan", None, &["-0 -0.0", "%f %f"], &[], &[]),
    ("scan", None, &["-inf 1", "%f %d"], &[], &[]),
    (
        "scan",
        None,
        &["Infinity", "%g", "g"],
        &[("g", "G")],
        &["g"],
    ),
    ("scan", None, &["nan 1.5", "%f %f"], &[], &[]),
    (
        "binary",
        Some("scan"),
        &["\u{1}", "cc", "a", "b"],
        &[("b", "B")],
        &["a", "b"],
    ),
    ("binary", Some("scan"), &["\u{1}\u{2}", "cc", "a"], &[], &[]),
    (
        "binary",
        Some("scan"),
        &["abc", "a2A*", "x", "y"],
        &[],
        &["x", "y"],
    ),
    ("binary", Some("scan"), &["ab  ", "A*", "x"], &[], &["x"]),
    (
        "binary",
        Some("scan"),
        &["abc", "H*b8", "h", "b"],
        &[],
        &["h", "b"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{1}\u{2}", "c*", "l"],
        &[],
        &["l"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{1}\u{2}", "c0x1c", "e", "c"],
        &[],
        &["e", "c"],
    ),
    (
        "binary",
        Some("scan"),
        &["abc", "a4", "x"],
        &[("x", "X")],
        &["x"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{0}\u{0} @", "r", "v"],
        &[],
        &["v"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{7f}\u{7e}", "su", "v"],
        &[],
        &["v"],
    ),
    ("binary", Some("scan"), &["abc", "Z", "v"], &[], &[]),
    (
        "binary",
        Some("scan"),
        &["\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}\u{f0}\u{7f}", "d", "v"],
        &[],
        &["v"],
    ),
    (
        "binary",
        Some("scan"),
        &[
            "\u{ff}\u{ff}\u{ff}\u{ff}\u{ff}\u{ff}\u{ff}\u{ff}",
            "wuw",
            "u",
            "s",
        ],
        &[],
        &["u", "s"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{cd}\u{cc}\u{cc}\u{3d}", "f", "v"],
        &[],
        &["v"],
    ),
    (
        "lassign",
        None,
        &["a b", "x", "y", "z"],
        &[("z", "Z")],
        &["x", "y", "z"],
    ),
    ("lassign", None, &["a {b c} d", "x"], &[], &["x"]),
    ("lassign", None, &["x #a b", "y"], &[], &["y"]),
    ("lassign", None, &["a b"], &[], &[]),
    ("lassign", None, &["a {b", "x"], &[], &[]),
    ("array", Some("set"), &["arr", "k 1 k 2"], &[], &["arr(k)"]),
    ("array", Some("set"), &["arr", "k1 v1 k2"], &[], &[]),
    ("array", Some("set"), &["arr", ""], &[], &[]),
];

/// `scan`'s conversion count: a `%n` and a
/// suppressed success are conversions, as C's `nconversions` counts them,
/// so an input that runs out after one is not the underflow. Measured
/// alike on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0: `scan {} %n%d
/// n a` is 1 with `n` written 0 and `a` preserved, `scan abc %s%n a n` is
/// 2, the suppressed forms are 0 with `a` preserved, and only `scan {}
/// %*d%d a` is -1. Every row answers on every release.
const SCAN_COUNT_WITNESSES: &[StorageWitness] = &[
    (
        "scan",
        None,
        &["", "%n%d", "n", "a"],
        &[("n", "5"), ("a", "7")],
        &["n", "a"],
    ),
    (
        "scan",
        None,
        &["abc", "%s%n", "a", "n"],
        &[("a", "7"), ("n", "5")],
        &["a", "n"],
    ),
    ("scan", None, &["", "%*n%d", "a"], &[("a", "7")], &["a"]),
    ("scan", None, &["5", "%*d%d", "a"], &[("a", "7")], &["a"]),
    ("scan", None, &["", "%*d%d", "a"], &[("a", "7")], &["a"]),
    ("scan", None, &["", "%n%d"], &[], &[]),
    ("scan", None, &["", "%*n%d"], &[], &[]),
    ("scan", None, &["5", "%*d%d"], &[], &[]),
    ("scan", None, &["", "%*d%d"], &[], &[]),
];

/// The destructuring writers' witnesses, per release found on
/// `PATH`, each under that release's profile against the real `tclsh`
/// (8.5 on for `lassign`, which 8.4 lacks): a converted field writes its
/// variable and one the input did not reach keeps its value, a repeated
/// variable composes, `array set` writes one element per key. When the
/// route answers it must match; when `tclsh` raises the route must
/// decline; a decline where `tclsh` answers is allowed — a 32-bit overflow,
/// a positional conversion, a float or `0x` spelling 8.4 reads another way,
/// a field a release lacks — but the first four and the conversion count's
/// witnesses answer on every release that has the command.
#[test]
fn destructuring_witnesses_match_every_release_on_path() {
    let reg = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        let mut agreed = 0usize;
        let witnesses = DESTRUCTURE_WITNESSES.iter().chain(SCAN_COUNT_WITNESSES);
        for (index, &witness) in witnesses.enumerate() {
            let want = storage_oracle(&tclsh, witness);
            let got = storage_route(&reg, profile, witness);
            let (command, sub, args, _, _) = witness;
            let answers = index < 4 || index >= DESTRUCTURE_WITNESSES.len();
            match (&want, &got) {
                (Some(want), Some(got)) => {
                    assert_eq!(
                        got,
                        want,
                        "tclsh{}: {command} {sub:?} {args:?}",
                        version.version_string()
                    );
                    agreed += 1;
                }
                (None, Some(got)) => panic!(
                    "tclsh{} raises on {command} {sub:?} {args:?}, the route answered {got:?}",
                    version.version_string()
                ),
                (Some(_), None) => assert!(
                    !answers,
                    "tclsh{}: the route declined the witness {command} {sub:?} {args:?}",
                    version.version_string()
                ),
                (None, None) => {}
            }
        }
        assert!(
            agreed >= 21 + SCAN_COUNT_WITNESSES.len(),
            "tclsh{}: only {agreed} witnesses agreed",
            version.version_string()
        );
    }
}

/// `dict with`'s key projection against the real `tclsh`, from 8.5,
/// the release `dict` arrives in: the variables a body sees on entry, beyond
/// the dictionary variable, are the plan's binders — every key of the
/// dictionary, a repeated key once, or of the nested one a key path names —
/// and the page's program answers `done` and leaves `d` as `a 2`.
#[test]
fn dict_with_binds_the_keys_tclsh_binds() {
    use tcl_registry::value_transfer::{
        BinderName, ExactValue, LiteralInputs, PlanAnswer, resolve_semantics,
    };
    const DICTS: &[(&str, &[&str])] = &[
        ("a 1", &[]),
        ("a 1 b 2 a 3", &[]),
        ("", &[]),
        ("x {p 1 q 2} y 3", &["x"]),
        ("x {y {k v}}", &["x", "y"]),
    ];
    let reg = CommandRegistry::build_default();
    let spec = reg.get("dict").expect("dict");
    let semantics = resolve_semantics(spec, Some(spec.subcommand("with").expect("with")), None);
    let semantics = semantics.semantics().expect("the dict with plan");
    for version in tcl_dialect::TclVersion::ALL {
        if version < tcl_dialect::TclVersion::V8_5 {
            continue;
        }
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        for &(dict, path) in DICTS {
            let mut words = vec!["d"];
            words.extend_from_slice(path);
            words.push("return [lsort [info locals]]");
            let inputs = LiteralInputs::new("dict", Some("with"), &words, profile)
                .with_prior("d", ExactValue::from_literal(dict));
            let PlanAnswer::Body { binders, .. } = semantics.structure(&inputs) else {
                panic!("{dict:?} {path:?}: no body plan");
            };
            let mut planned: Vec<String> = binders
                .iter()
                .map(|binder| match &binder.name {
                    BinderName::Declared(name) => name.clone(),
                    BinderName::Operand(id) => words[id.0 - 1].to_owned(),
                })
                .collect();
            planned.sort();
            let mut script = format!(
                "proc p {{}} {{\n    set d {}\n    dict with d",
                tcl_quoted_word(dict)
            );
            for key in path {
                script.push(' ');
                script.push_str(key);
            }
            script.push_str(" {return [lsort [info locals]]}\n}\nputs -nonewline [p]\n");
            let (true, locals) = run_tcl(&tclsh, &script).expect("tclsh runs") else {
                panic!("tclsh{}: {script}", version.version_string());
            };
            let bound: Vec<&str> = locals
                .split(' ')
                .filter(|name| *name != "d" && !name.is_empty())
                .collect();
            assert_eq!(
                planned,
                bound,
                "tclsh{}: dict with over {dict:?} {path:?}",
                version.version_string()
            );
        }
        let program =
            "set d {a 1}\nputs [dict with d {incr a; set result done}]\nputs -nonewline $d\n";
        assert_eq!(
            run_tcl(&tclsh, program),
            Some((true, "done\na 2".to_owned())),
            "tclsh{}",
            version.version_string()
        );
    }
}
