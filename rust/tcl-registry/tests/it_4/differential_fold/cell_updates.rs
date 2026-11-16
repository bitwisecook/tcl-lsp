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

//! Index reading and cell updates: list, keyed and variable updates and their storage outcomes on every release.

use super::*;

/// The route's answer for `command v values…` with `v` holding `prior`,
/// under `profile`: the registry's cell update over literal words.
fn cell_update_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    command: &str,
    prior: &str,
    values: &[&str],
) -> Option<String> {
    use tcl_registry::ArgRole;
    use tcl_registry::value_transfer::{
        Budget, EvalAnswer, ExactValue, ExactValueOrUnavailable, LiteralInputs, OperandId,
        resolve_semantics,
    };

    let spec = reg.get(command).expect(command);
    let semantics = resolve_semantics(spec, None, None);
    let semantics = semantics.semantics().expect("a storage route");
    let mut args = vec!["v"];
    args.extend_from_slice(values);
    let mut inputs = LiteralInputs::new(command, None, &args, profile)
        .with_prior("v", ExactValue::from_literal(prior));
    // The roles the resolver gives the same words, so a route that finds
    // its place by role (`set`'s write and read forms) runs as it does
    // over the lattice.
    for role in [ArgRole::VarWrite, ArgRole::VarRead] {
        for index in reg.arg_indices_for_role(command, &args, role) {
            inputs = inputs.with_role(OperandId(index), role);
        }
    }
    match semantics.evaluate(&inputs, &mut Budget::evaluation()) {
        EvalAnswer::Evaluated(outcome) => match outcome.result {
            ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes).ok(),
            ExactValueOrUnavailable::Unavailable(_) => None,
        },
        EvalAnswer::Pending | EvalAnswer::Declined(_) => None,
    }
}

/// What `tclsh` leaves in `v` after `command v values…` with `v` holding
/// `prior`, each value passed as a word that carries it exactly.
fn cell_update_oracle(tclsh: &str, command: &str, prior: &str, values: &[&str]) -> Option<String> {
    let mut script = format!("set v {}; {command} v", tcl_quoted_word(prior));
    for value in values {
        script.push(' ');
        script.push_str(&tcl_quoted_word(value));
    }
    script.push_str("; puts -nonewline $v");
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out),
        (false, _) => None,
    }
}

/// `string range` through its route under `version`, against `tclsh`.
fn check_range_witnesses(tclsh: &str, reg: &CommandRegistry, version: tcl_dialect::TclVersion) {
    let range = reg
        .get("string")
        .expect("string")
        .subcommand("range")
        .expect("range");
    let range_cases: &[[&str; 3]] = &[
        ["hello", "1", "3"],
        ["abcdefghijkl", "010", "end"],
        ["abcdefghijkl", "3", "6"],
        ["abcdef", "-2", "2"],
        ["abc", "end-1", "end"],
        ["abc", "3", "1"],
        ["abcdefghijkl", "0x2", "end-010"],
        [" a ", "0", "end"],
    ];
    for case in range_cases {
        let want = tcl_value(tclsh, &tcl_command("string", Some("range"), case));
        let got = range.run_const_fold(case, Some(version));
        match (want, got) {
            (Some(want), Some(got)) => assert_eq!(
                got,
                want,
                "tclsh{}: string range {case:?}",
                version.version_string()
            ),
            (None, Some(got)) => panic!(
                "tclsh{} raises on string range {case:?}, the fold answered {got:?}",
                version.version_string()
            ),
            (_, None) => {}
        }
    }
}

/// The index spellings `an_index_reads_as_each_release_reads_it` reads.
const INDEX_SPECS: &[&str] = &[
    "1",
    " 1",
    "1 ",
    "+1",
    "- 1",
    "-1",
    "0x2",
    "010",
    "0o10",
    "0b10",
    "1_0",
    "1e0",
    "end",
    "e",
    "en",
    "endx",
    " end",
    "end ",
    "end-1",
    "end+1",
    "end--1",
    "end-+1",
    "end- 1",
    "end+ 1",
    "end-1 ",
    "end-0x1",
    "end-010",
    "end-",
    "end-1-1",
    "end -1",
    "1+1",
    "1-1",
    "1+-1",
    "1--1",
    "1++1",
    "+1+1",
    "1+ 1",
    "1 +1",
    " 1+1",
    "1+1 ",
    "0x1+1",
    "010+0",
    "-1+2",
    "1+end",
    "end-1+1",
    "1 0",
    "",
    "2147483647",
    "2147483648",
    "4294967295",
    "4294967296",
    "-2147483649",
    "-4294967295",
    "-4294967296",
    "0xffffffff",
    "0x100000000",
    "end-2147483648",
    "end-4294967295",
    "end-4294967296",
    "end+2147483647",
    "end+2147483636",
    "end-2147483649",
    "end--2147483647",
    "end+4294967295",
    "1+2147483647",
    "4294967295+1",
    "-2147483648-1",
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775809",
    "0x8000000000000000",
    "0x1ffffffffffffffff",
    "1+9223372036854775807",
    "9223372036854775807+9223372036854775807",
    "end+9223372036854775807",
    "end+9223372036854775808",
    "end+99999999999999999999",
    "end-9223372036854775808",
];

/// Magnitudes from 2^64 − 2^32 + 1 to 2^64 − 1, which 8.4 and 8.5 read only
/// where `long` is 64 bits.
const HOST_LONG_SPECS: &[&str] = &[
    "18446744073709551615",
    "-0xffffffffffffffff",
    "18446744069414584321",
    "end-18446744073709551615",
    "0xffffffffffffffff+1",
];

/// Words 8.6 encodes as literals after the end and reads as values before
/// the first element.
const COMPILED_APART_86: &[&str] = &["end+2147483647", "end-2147483649", "end--2147483647"];

/// Each spelling of an index, read by `string range` under each release on
/// `PATH`: the fold answers as that release's `tclsh` does, and declines where
/// it raises (`bad index`) — 8.4 reads no `end+1` or `1+1`, 8.4 to 8.6 read
/// `e` as `end`, and no release reads ` end` or `end ` or a space after an
/// operator but 8.4's `end- 1`. Past 32 bits: 8.4 to 8.6 wrap an
/// integer within ±4294967295 to 32 bits, sums and `end` offsets alike, and
/// raise past it; 9.0 reads the wide, a bignum and a sum that reaches the
/// widest as `end+1` past `end`. A magnitude from 2^64 − 2^32 + 1 to
/// 2^64 − 1, which these 64-bit oracles read under 8.4 and 8.5 and a 32-bit
/// `long` cannot, declines there, and an `end` offset 8.6 reads apart
/// as a literal and as a value declines under 8.6. Each spec is read
/// as both ends of `string range` and as its last against the first
/// character.
#[test]
fn an_index_reads_as_each_release_reads_it() {
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let reg = tcl_registry::model::ingress::static_context_for(version.dialect_profile_name())
            .commands();
        let range = reg
            .get("string")
            .expect("string")
            .subcommand("range")
            .expect("range");
        for spec in INDEX_SPECS.iter().chain(HOST_LONG_SPECS) {
            for case in [["abcdefghijkl", spec, spec], ["abcdefghijkl", "0", spec]] {
                let want = tcl_value(&tclsh, &tcl_command("string", Some("range"), &case));
                let got = range.run_const_fold(&case, Some(version));
                if HOST_LONG_SPECS.contains(spec) && version < tcl_dialect::TclVersion::V8_6 {
                    assert_eq!(
                        got,
                        None,
                        "tclsh{}: string range {case:?} turns on the host's long",
                        version.version_string()
                    );
                    continue;
                }
                if COMPILED_APART_86.contains(spec) && version == tcl_dialect::TclVersion::V8_6 {
                    assert_eq!(got, None, "tclsh8.6: string range {case:?} compiles apart");
                    if case[1] == "0" {
                        let as_value = tcl_value(
                            &tclsh,
                            &format!("string range {{{}}} 0 [set i {{{spec}}}]", case[0]),
                        );
                        assert_ne!(want, as_value, "tclsh8.6: string range {case:?}");
                    }
                    continue;
                }
                check_index_fold(version, &case, want.as_ref(), got.as_ref());
            }
        }
    }
}

/// One `string range` fold against its `tclsh`'s answer.
fn check_index_fold(
    version: tcl_dialect::TclVersion,
    case: &[&str],
    want: Option<&String>,
    got: Option<&String>,
) {
    match (want, got) {
        (Some(want), Some(got)) => assert_eq!(
            got,
            want,
            "tclsh{}: string range {case:?}",
            version.version_string()
        ),
        (None, Some(got)) => panic!(
            "tclsh{} raises on string range {case:?}, the fold answered {got:?}",
            version.version_string()
        ),
        (Some(want), None) => panic!(
            "tclsh{} answers {want:?} for string range {case:?}, the fold declined",
            version.version_string()
        ),
        (None, None) => {}
    }
}

/// `dict <sub> d <words…>` through the keyed update the resolver selects
/// under `profile`, with `d` holding `prior`; `None` for a decline or when
/// the resolver finds no such command.
fn keyed_update_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    sub: &str,
    prior: &str,
    words: &[&str],
) -> Option<String> {
    use tcl_registry::ArgRole;
    use tcl_registry::invocation_words::{InvocationWord, InvocationWords};
    use tcl_registry::value_transfer::{
        Budget, EvalAnswer, ExactValue, ExactValueOrUnavailable, LiteralInputs, OperandId,
    };

    let mut args = vec![sub, "d"];
    args.extend_from_slice(words);
    let literal_words: Vec<InvocationWord<'_>> =
        args.iter().copied().map(InvocationWord::Literal).collect();
    let resolved = reg
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("dict"), &literal_words),
            reg.own_surface_query(),
        )
        .resolved()?;
    let semantics = resolved.semantics.value.semantics()?;
    let mut inputs = LiteralInputs::new("dict", Some(sub), &args[1..], profile)
        .with_prior("d", ExactValue::from_literal(prior));
    for index in reg.arg_indices_for_role("dict", &args, ArgRole::VarWrite) {
        inputs = inputs.with_role(OperandId(index), ArgRole::VarWrite);
    }
    match semantics.evaluate(&inputs, &mut Budget::evaluation()) {
        EvalAnswer::Evaluated(outcome) => match outcome.result {
            ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes).ok(),
            ExactValueOrUnavailable::Unavailable(_) => None,
        },
        EvalAnswer::Pending | EvalAnswer::Declined(_) => None,
    }
}

/// The keyed updates of `dict`, per release found on `PATH`, against the
/// real `tclsh`: when the route answers it must match, and when `tclsh`
/// raises it must decline. Under 8.4, which has no `dict`, the resolver
/// finds no route at all.
#[test]
fn keyed_update_witnesses_match_every_release_on_path() {
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let dialect = version.dialect_profile_name();
        let reg = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = tcl_dialect::DialectProfile::find(dialect);
        if version == tcl_dialect::TclVersion::V8_4 {
            assert_eq!(
                keyed_update_route(reg, profile, "set", "", &["a", "1"]),
                None,
                "8.4 has no dict, so the resolver finds no route"
            );
            continue;
        }
        let cases: &[(&str, &str, &[&str])] = &[
            ("set", "", &["a", "1"]),
            ("set", "a 1 b 2", &["a", "3"]),
            ("set", "a {x 1}", &["a", "y", "2"]),
            ("set", "b 2 a 1", &["c", "3"]),
            ("set", " a  1 ", &["b", "2"]),
            ("set", "a 1 a 2", &["b", "3"]),
            ("set", "a 1 b", &["c", "3"]),
            ("set", "a b", &["a", "c", "d"]),
            ("unset", "a 1 b 2", &["a"]),
            ("unset", " a  1 ", &["zz"]),
            ("unset", "a {x 1}", &["a", "x"]),
            ("unset", "a 1", &["x", "y"]),
            ("incr", "k 5", &["k"]),
            ("incr", "k 5", &["k", "-7"]),
            ("incr", "k 010", &["k"]),
            ("incr", "k 5", &["k", "010"]),
            ("incr", "k { 5 }", &["k"]),
            ("incr", "k 9223372036854775807", &["k"]),
            ("incr", "k abc", &["k"]),
            ("incr", "", &["k", "010"]),
            ("incr", "", &["k", "2.5"]),
            ("append", "k foo", &["k", "bar"]),
            ("append", "", &["k"]),
            ("lappend", "", &["k", "a", "b c"]),
            ("lappend", "k v", &["k"]),
            ("lappend", "k \\{", &["k", "v"]),
        ];
        let mut agreed = 0usize;
        for &(sub, prior, words) in cases {
            let mut script = format!("set d {}; dict {sub} d", tcl_quoted_word(prior));
            for word in words {
                script.push(' ');
                script.push_str(&tcl_quoted_word(word));
            }
            script.push_str("; puts -nonewline $d");
            let want = match run_tcl(&tclsh, &script) {
                Some((true, out)) => Some(out),
                _ => None,
            };
            let got = keyed_update_route(reg, profile, sub, prior, words);
            match (want, got) {
                (Some(want), Some(got)) => {
                    assert_eq!(
                        got,
                        want,
                        "tclsh{}: dict {sub} over {prior:?} with {words:?}",
                        version.version_string()
                    );
                    agreed += 1;
                }
                (None, Some(got)) => panic!(
                    "tclsh{} raises on dict {sub} over {prior:?} with {words:?}, the route answered {got:?}",
                    version.version_string()
                ),
                (_, None) => {}
            }
        }
        assert!(
            agreed >= 18,
            "tclsh{}: only {agreed} keyed-update witnesses agreed",
            version.version_string()
        );
    }
}

/// The storage-outcome witnesses of the direct route, per release found on
/// `PATH`: `incr`, `append`, and `lappend` through the registry's cell
/// update, `set`'s write and read forms through the cell write, and `string
/// range` through its route, each under the release's profile, against the
/// real `tclsh`. When the route answers it must match;
/// when `tclsh` raises the route must decline; a decline where `tclsh`
/// answers is allowed (an unfolded value is never wrong).
#[test]
fn storage_outcome_witnesses_match_every_release_on_path() {
    let reg = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        let cases: &[(&str, &str, &[&str])] = &[
            ("incr", "5", &[]),
            ("incr", "3", &["10"]),
            ("incr", "10", &["-2"]),
            ("incr", "010", &[]),
            ("incr", "1", &[" 5"]),
            ("incr", "9223372036854775807", &[]),
            ("incr", "1_000", &[]),
            ("incr", "0x10", &["1"]),
            ("incr", "abc", &[]),
            ("incr", "1", &["2.5"]),
            ("append", "foo", &["bar", " baz"]),
            ("append", " a ", &["b"]),
            ("append", "", &["x"]),
            ("lappend", "a b", &["c", "d e"]),
            ("lappend", "", &["c"]),
            ("lappend", "{", &["v"]),
            ("lappend", "a", &["{", "b"]),
            // A first element starting with `#` is brace-quoted from 8.5
            // and bare in 8.4.
            ("lappend", "", &["#", "b"]),
            ("lappend", "#", &["b"]),
            ("lappend", "#x", &["b"]),
            ("set", "old", &[" a "]),
            ("set", "old", &["a\\b"]),
            ("set", " 7 ", &[]),
            ("set", "010", &[]),
        ];
        let mut agreed = 0usize;
        let mut commands_agreeing = std::collections::BTreeSet::new();
        for &(command, prior, values) in cases {
            let want = cell_update_oracle(&tclsh, command, prior, values);
            let got = cell_update_route(&reg, profile, command, prior, values);
            match (want, got) {
                (Some(want), Some(got)) => {
                    assert_eq!(
                        got,
                        want,
                        "tclsh{}: {command} over {prior:?} with {values:?}",
                        version.version_string()
                    );
                    agreed += 1;
                    commands_agreeing.insert(command);
                }
                (None, Some(got)) => panic!(
                    "tclsh{} raises on {command} over {prior:?} with {values:?}, the route answered {got:?}",
                    version.version_string()
                ),
                (_, None) => {}
            }
        }
        assert!(
            agreed >= 8,
            "tclsh{}: only {agreed} witnesses agreed",
            version.version_string()
        );
        assert_eq!(
            commands_agreeing.into_iter().collect::<Vec<_>>(),
            ["append", "incr", "lappend", "set"],
            "tclsh{}: every route answers somewhere",
            version.version_string()
        );
        check_range_witnesses(&tclsh, &reg, version);
    }
}

/// The list a list cell update's witnesses start from.
const NESTED: &[(&str, &str)] = &[("x", "a {b1 b2} c")];

/// The list cell updates' witnesses, each with `x` holding `a {b1 b2} c`
/// unless it says otherwise: `lset` on every release, `ledit` and `lpop`
/// from 9.0. Measured on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0: an
/// index equal to a level's length appends from 8.6 (`lset x 3 D`, `lset x
/// 3 0 Q`) and raises before it; the sums read from 8.5 (`lset x 1+1 Q`);
/// a lone index word that is no index is a path (`lset x {1 0} Q`), and one
/// that is neither an index nor a list a bad index; `lpop` reads each word
/// as one index (`lpop x {1 0}` raises). Past 32 bits each release reads in
/// its own range: `lset x -4294967295 Z` writes element 1 up to
/// 8.6, `end-4294967295` appends on 8.6, and 9.0 appends at a sum that
/// reaches the widest wide or a bignum offset after `end`.
const LIST_UPDATE_WITNESSES: &[StorageWitness] = &[
    ("lset", None, &["x", "1", "B"], NESTED, &["x"]),
    ("lset", None, &["x", "end", "B"], NESTED, &["x"]),
    ("lset", None, &["x", "1 0", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", "1", "0", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", "", "NEW"], NESTED, &["x"]),
    ("lset", None, &["x", "NEW"], NESTED, &["x"]),
    ("lset", None, &["x", "0x1", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", " 1 ", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", "end-0x1", "Q"], NESTED, &["x"]),
    (
        "lset",
        None,
        &["x", "0", "0", "0", "Q"],
        &[("x", "a b")],
        &["x"],
    ),
    ("lset", None, &["x", "3", "D"], NESTED, &["x"]),
    ("lset", None, &["x", "end+1", "D"], NESTED, &["x"]),
    ("lset", None, &["x", "3", "0", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", "1", "2", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", "1+1", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", "-1", "Z"], NESTED, &["x"]),
    ("lset", None, &["x", "4", "E"], NESTED, &["x"]),
    ("lset", None, &["x", "1", "5", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", "bogus", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", "{a", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", "1 bogus", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", "010", "Q"], NESTED, &["x"]),
    ("lset", None, &["x", "0", "Q"], &[("x", "a \"b")], &["x"]),
    ("lset", None, &["x", "-4294967295", "Z"], NESTED, &["x"]),
    ("lset", None, &["x", "end-4294967295", "Z"], NESTED, &["x"]),
    ("lset", None, &["x", "2147483648", "Z"], NESTED, &["x"]),
    ("lset", None, &["x", "4294967296", "Z"], NESTED, &["x"]),
    (
        "lset",
        None,
        &["x", "1+9223372036854775807", "Z"],
        NESTED,
        &["x"],
    ),
    (
        "lset",
        None,
        &["x", "end+99999999999999999999", "Z"],
        NESTED,
        &["x"],
    ),
    (
        "lset",
        None,
        &["x", "end+9223372036854775807", "Z"],
        NESTED,
        &["x"],
    ),
    ("ledit", None, &["x", "1", "1", "X", "Y"], NESTED, &["x"]),
    ("ledit", None, &["x", "1", "0", "I"], NESTED, &["x"]),
    ("ledit", None, &["x", "5", "6", "Z"], NESTED, &["x"]),
    ("ledit", None, &["x", "-1", "-1", "Z"], NESTED, &["x"]),
    ("ledit", None, &["x", "end+1", "end+1", "Z"], NESTED, &["x"]),
    ("ledit", None, &["x", "0", "end"], NESTED, &["x"]),
    ("ledit", None, &["x", "bogus", "0", "A"], NESTED, &["x"]),
    (
        "ledit",
        None,
        &[
            "x",
            "end+9223372036854775808",
            "end+9223372036854775808",
            "Z",
        ],
        NESTED,
        &["x"],
    ),
    ("lpop", None, &["x"], NESTED, &["x"]),
    ("lpop", None, &["x", "0"], NESTED, &["x"]),
    ("lpop", None, &["x", "end"], NESTED, &["x"]),
    ("lpop", None, &["x", "1", "0"], NESTED, &["x"]),
    ("lpop", None, &["x", "0", "0"], NESTED, &["x"]),
    ("lpop", None, &["x", "1 0"], NESTED, &["x"]),
    ("lpop", None, &["x", "3"], NESTED, &["x"]),
    ("lpop", None, &["x", "1", "5"], NESTED, &["x"]),
    ("lpop", None, &["x", "bogus"], NESTED, &["x"]),
    ("lpop", None, &["x"], &[("x", "")], &["x"]),
    ("lpop", None, &["x", "4294967296"], NESTED, &["x"]),
];

/// `lset`, `ledit` and `lpop` through their routes, against each release on
/// `PATH` under that release's profile: the route answers every witness its
/// `tclsh` answers, with the result and the list it leaves, and declines every
/// one `tclsh` raises on; `ledit` and `lpop` decline before 9.0, which lacks
/// them. A profile that names no release answers only what every release
/// answers alike.
#[test]
fn list_cell_updates_match_every_release_on_path() {
    let reg = CommandRegistry::build_default();
    let mut answers: Vec<Vec<Option<Vec<String>>>> = vec![Vec::new(); LIST_UPDATE_WITNESSES.len()];
    let mut releases = 0usize;
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        releases += 1;
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        for (index, &witness) in LIST_UPDATE_WITNESSES.iter().enumerate() {
            let want = storage_oracle(&tclsh, witness);
            let got = storage_route(&reg, profile, witness);
            let (command, _, args, priors, _) = witness;
            assert_eq!(
                got,
                want,
                "tclsh{}: {command} {args:?} over {priors:?}",
                version.version_string()
            );
            answers[index].push(want);
        }
    }
    if releases < tcl_dialect::TclVersion::ALL.len() {
        return;
    }
    for (index, &witness) in LIST_UPDATE_WITNESSES.iter().enumerate() {
        let every = &answers[index];
        let agreed = every
            .first()
            .cloned()
            .flatten()
            .filter(|first| every.iter().all(|answer| answer.as_ref() == Some(first)));
        let got = storage_route(&reg, None, witness);
        let (command, _, args, priors, _) = witness;
        assert_eq!(
            got, agreed,
            "no release named: {command} {args:?} over {priors:?}"
        );
    }
}

/// A magnitude from 2^64 − 2^32 + 1 to 2^64 − 1 is an index on 8.4 and 8.5
/// only where `long` is 64 bits, as on these oracles, and `bad index` where it
/// is 32 bits: `lset x 18446744069414584321 Z` writes element 1 on tclsh
/// 8.4.20 and 8.5.19 and raises on 8.6.18, so the route declines under 8.4
/// and 8.5 and raises with `tclsh` from 8.6.
#[test]
fn an_index_the_hosts_long_decides_declines() {
    let reg = CommandRegistry::build_default();
    let witness: StorageWitness = (
        "lset",
        None,
        &["x", "18446744069414584321", "Z"],
        NESTED,
        &["x"],
    );
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        let want = storage_oracle(&tclsh, witness);
        let got = storage_route(&reg, profile, witness);
        let series = version.version_string();
        if version < tcl_dialect::TclVersion::V8_6 {
            assert!(
                want.is_some(),
                "tclsh{series} reads it as a 64-bit long does"
            );
            assert_eq!(got, None, "tclsh{series}: the host's long decides it");
        } else {
            assert_eq!(got, want, "tclsh{series}");
        }
    }
}
