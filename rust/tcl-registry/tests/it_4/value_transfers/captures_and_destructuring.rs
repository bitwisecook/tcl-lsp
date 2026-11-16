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

//! Commands that store into variables: `lassign`, `regexp` and `regsub`, `scan`, and the byte and array writers.

use super::*;

/// `lassign` stores to its variables in order, each store failing where its
/// place holds an array, and its existence transfer says so by path
/// (`docs/design/compiler/value-transfers.md` § *`catch`, `try`, and
/// completion*): every target is bound on the normal path, which a target
/// proven an array removes; on the error path every target before the first
/// that is not proven writable is bound, that target and the rest are
/// may-bound where its kind is unknown, and untouched where it is certainly
/// an array, since the command stops there. An element may fail on its array,
/// so it is never proven writable.
#[test]
fn lassign_lists_its_completion_paths() {
    use tcl_registry::completion::{CompletionCode, CompletionCodeDomain};
    use tcl_registry::value_transfer::BindingKind;
    let reg = CommandRegistry::build_default();
    let resolved = resolve_semantics(reg.get("lassign").expect("lassign"), None, None);
    let semantics = resolved.semantics().expect("declared");
    let scalar = bound_as(BindingKind::Scalar);
    let array = bound_as(BindingKind::Array);
    let paths = |facts: &[(&str, FactView)], names: &[&'static str]| {
        let mut words = vec![literal("list", None)];
        words.extend(
            names
                .iter()
                .map(|name| literal(name, Some(ArgRole::VarWrite))),
        );
        let mut inputs = TestInputs::new("lassign", words);
        for (name, fact) in facts {
            inputs.prior.insert((*name).to_owned(), fact.clone());
        }
        match semantics.transfer(FactDomain::Existence, &inputs, &mut Budget::unbounded()) {
            TransferAnswer::Existence(transfer) => transfer.paths,
            other => panic!("{other:?}"),
        }
    };
    let bind = |at: usize| {
        (
            TargetId(OperandId(at)),
            ExistenceOutcome::Bind(BindingKind::Scalar),
        )
    };
    let may = |at: usize| {
        (
            TargetId(OperandId(at)),
            ExistenceOutcome::MayBind(BindingKind::Scalar),
        )
    };
    let normal = CompletionCodeDomain::Exact(&[CompletionCode::Ok]);
    let error = CompletionCodeDomain::Exact(&[CompletionCode::Error]);

    // Every place writable: the normal path alone.
    let all = paths(&[("a", scalar.clone()), ("b", absent())], &["a", "b"]);
    assert_eq!(all.len(), 1);
    assert_eq!(
        (all[0].completion, all[0].outcomes.clone()),
        (normal, vec![bind(1), bind(2)])
    );

    // `b` proven an array: the error path alone, `a` bound, `b` and `c`
    // untouched.
    let certain = paths(
        &[("a", scalar.clone()), ("b", array.clone())],
        &["a", "b", "c"],
    );
    assert_eq!(certain.len(), 1);
    assert_eq!(
        (certain[0].completion, certain[0].outcomes.clone()),
        (error, vec![bind(1)])
    );

    // `b` of a kind the rung does not state: both paths, `a` bound on the
    // error path and `b` and `c` may-bound.
    let unknown = paths(&[("a", scalar.clone())], &["a", "b", "c"]);
    assert_eq!(unknown.len(), 2);
    assert_eq!(unknown[0].outcomes, vec![bind(1), bind(2), bind(3)]);
    assert_eq!(
        (unknown[1].completion, unknown[1].outcomes.clone()),
        (error, vec![bind(1), may(2), may(3)])
    );

    // An element is never proven writable.
    let element = paths(&[], &["a", "k(1)"]);
    assert_eq!(element.len(), 2);
    assert_eq!(element[1].outcomes, vec![may(1), may(2)]);
    assert_eq!(
        semantics.transfer(
            FactDomain::Type,
            &TestInputs::new("lassign", vec![]),
            &mut Budget::unbounded()
        ),
        TransferAnswer::Generic
    );
}

/// The regexp owner's route for `command words…` over literal operands,
/// those marked `true` given the `VarWrite` role the resolver gives match
/// and result variables, with its store targets checked by the driver's
/// own validation before the answer is returned.
fn regex_route(command: &str, words: &[(&str, bool)], budget: &mut Budget) -> EvalAnswer {
    use tcl_registry::value_transfer::validate_outcome;
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get(command).expect(command), None, None);
    let semantics = semantics.semantics().expect("the regexp owner's route");
    let operands = words
        .iter()
        .map(|&(text, target)| literal(text, target.then_some(ArgRole::VarWrite)))
        .collect();
    let inputs = TestInputs::new(command, operands);
    let answer = semantics.evaluate(&inputs, budget);
    if let EvalAnswer::Evaluated(outcome) = &answer {
        assert_eq!(
            validate_outcome(
                &semantics.structure(&inputs),
                &semantics.store_targets(&inputs),
                outcome
            ),
            Ok(()),
            "{command} {words:?}"
        );
    }
    answer
}

/// The result text and each store as `(operand, Some(written text))` for a
/// write or `(operand, None)` for a preserve, of an evaluated answer.
fn regex_stores(answer: &EvalAnswer) -> (String, Vec<(usize, Option<String>)>) {
    let EvalAnswer::Evaluated(outcome) = answer else {
        panic!("not evaluated: {answer:?}");
    };
    let ExactValueOrUnavailable::Exact(result) = &outcome.result else {
        panic!("no exact result: {outcome:?}");
    };
    let stores = outcome
        .ordered_stores
        .iter()
        .map(|store| match store {
            StoreOutcome::Write { target, value } => (
                (target.0).0,
                Some(String::from_utf8(value.bytes.clone()).expect("text")),
            ),
            StoreOutcome::Preserve { target } => ((target.0).0, None),
            other => panic!("unexpected store {other:?}"),
        })
        .collect();
    (
        String::from_utf8(result.bytes.clone()).expect("text"),
        stores,
    )
}

/// `regexp` writes or preserves its match variables (the Storage
/// row's "`regexp` no-match" and the Regexp row): a match writes one value
/// per match variable — an unmatched subgroup the empty string, or `-1 -1`
/// with `-indices` — and answers the count; a completed no-match preserves
/// every one and answers 0; `-inline` writes nothing and answers the list;
/// `-all` counts, the variables holding the last match; `-about` answers
/// the pattern's shape. Each answer is tclsh 8.4.20 to 9.1b0's
/// (`regexp_witnesses_match_every_release_on_path`).
#[test]
fn regexp_writes_or_preserves_its_match_variables() {
    use tcl_registry::TclType;
    let budget = || Budget::evaluation();
    let t = |text| (text, true);
    let w = |text| (text, false);

    // A completed no-match preserves every match variable.
    let answer = regex_route(
        "regexp",
        &[w("(x)(y)"), w("zz"), t("a"), t("b")],
        &mut budget(),
    );
    assert_eq!(
        regex_stores(&answer),
        ("0".into(), vec![(2, None), (3, None)])
    );

    // A match writes each one; the unmatched subgroup writes the empty
    // string, or `-1 -1` with `-indices`, typed as the value it built.
    let answer = regex_route(
        "regexp",
        &[w("(a)(b)?"), w("ac"), t("m"), t("g1"), t("g2")],
        &mut budget(),
    );
    assert_eq!(
        regex_stores(&answer),
        (
            "1".into(),
            vec![
                (2, Some("a".into())),
                (3, Some("a".into())),
                (4, Some(String::new()))
            ]
        )
    );
    let answer = regex_route(
        "regexp",
        &[
            w("-indices"),
            w("(a)(b)?"),
            w("ac"),
            t("m"),
            t("g1"),
            t("g2"),
        ],
        &mut budget(),
    );
    assert_eq!(
        regex_stores(&answer),
        (
            "1".into(),
            vec![
                (3, Some("0 0".into())),
                (4, Some("0 0".into())),
                (5, Some("-1 -1".into()))
            ]
        )
    );
    let EvalAnswer::Evaluated(outcome) = &answer else {
        unreachable!()
    };
    assert_eq!(outcome.types.result, Some(TclType::Int));
    assert!(
        outcome
            .types
            .per_target
            .iter()
            .all(|(_, ty)| *ty == TclType::List),
        "{:?}",
        outcome.types
    );

    // `-inline` writes nothing and answers the list; `-all` counts, its
    // variables holding the last match; `-about` answers the pattern.
    for (words, want) in [
        (
            &[w("-inline"), w("-indices"), w("(a)(b)?"), w("ac")][..],
            "{0 0} {0 0} {-1 -1}",
        ),
        (&[w("-all"), w("a*"), w("xaax")][..], "3"),
        (&[w("-about"), w("(?:a)")][..], "0 REG_UNONPOSIX"),
        (&[w("-about"), w("a(b)c")][..], "1 {}"),
        (
            &[w("-start"), w("2"), w("-inline"), w("."), w("abcdef")][..],
            "c",
        ),
    ] {
        assert_eq!(
            regex_stores(&regex_route("regexp", words, &mut budget())),
            (want.into(), Vec::new()),
            "{words:?}"
        );
    }
    let answer = regex_route(
        "regexp",
        &[w("-all"), w("(a)"), w("banana"), t("m"), t("g")],
        &mut budget(),
    );
    assert_eq!(
        regex_stores(&answer),
        (
            "3".into(),
            vec![(3, Some("a".into())), (4, Some("a".into()))]
        )
    );
}

/// A `regexp` that established neither a match nor a no-match declines the
/// whole answer, never a no-match: a search cut short by the budget is
/// `Approximate`; a malformed pattern is the command's error, never a
/// value; a `-start` index the releases read differently is not evaluated;
/// and the variables the core writes must be the operands the resolver
/// named, or nothing is published.
#[test]
fn a_regexp_that_established_nothing_declines() {
    let budget = || Budget::evaluation();
    let t = |text| (text, true);
    let w = |text| (text, false);

    // A search cut short declines the whole answer: never a no-match.
    let mut starved = Budget::evaluation();
    starved.fuel = 100;
    let long = "a".repeat(300);
    assert_eq!(
        regex_route("regexp", &[w("^(a+)+b$"), w(&long), t("m")], &mut starved),
        EvalAnswer::Declined(DeclineReason::Approximate)
    );
    // Under an ordinary budget the same search is a completed no-match, its
    // variable preserved — unanchored too, which once spent the whole budget
    // re-simulating the pattern from each of the 300 starts.
    for pattern in ["^(a+)+b$", "(a+)+b"] {
        assert_eq!(
            regex_stores(&regex_route(
                "regexp",
                &[w(pattern), w(&long), t("m")],
                &mut budget()
            )),
            ("0".to_owned(), vec![(2, None)]),
            "{pattern}"
        );
    }
    // The command's error is no value: it is the completion, after no store.
    assert_eq!(
        raised(&regex_route("regexp", &[w("("), w("x")], &mut budget())),
        Some((0, None, None))
    );
    // A `-start` index the releases read differently (8 up to 8.6, 10
    // from 9.0) is not evaluated.
    assert_eq!(
        regex_route(
            "regexp",
            &[
                w("-start"),
                w("010"),
                w("-inline"),
                w("."),
                w("abcdefghijkl")
            ],
            &mut budget()
        ),
        EvalAnswer::Declined(DeclineReason::Unsupported)
    );
    // The variables the core writes are the operands the resolver named,
    // or nothing is published: here `-nocase` is an option the roles took
    // for the pattern.
    assert_eq!(
        regex_route(
            "regexp",
            &[w("-nocase"), w("A"), t("a"), t("m")],
            &mut budget()
        ),
        EvalAnswer::Declined(DeclineReason::Unsupported)
    );
}

/// `regsub` answers the substituted text, or the count with the text
/// written to its variable — whether or not anything matched, as tclsh 8.4
/// to 9.1 do — and its callback form (`-command`, from 9.0) has no route.
#[test]
fn regsub_writes_its_variable_and_declines_its_callback() {
    let budget = || Budget::evaluation();
    let t = |text| (text, true);
    let w = |text| (text, false);

    // `regsub`: the text, or the count with the text written — whether or
    // not anything matched; the callback form has no route.
    assert_eq!(
        regex_stores(&regex_route(
            "regsub",
            &[w("-all"), w(""), w("abc"), w("-")],
            &mut budget()
        )),
        ("-a-b-c".into(), Vec::new())
    );
    assert_eq!(
        regex_stores(&regex_route(
            "regsub",
            &[w("-all"), w("a"), w("banana"), w("o"), t("v")],
            &mut budget()
        )),
        ("3".into(), vec![(4, Some("bonono".into()))])
    );
    assert_eq!(
        regex_stores(&regex_route(
            "regsub",
            &[w("z"), w("abc"), w("X"), t("v")],
            &mut budget()
        )),
        ("0".into(), vec![(3, Some("abc".into()))])
    );
    assert_eq!(
        regex_route(
            "regsub",
            &[w("-command"), w("a"), w("abc"), w("string toupper")],
            &mut budget()
        ),
        EvalAnswer::Declined(DeclineReason::NoRoute(NoRouteReason::Callback))
    );
}

/// A writing route's answer for `command words…` under `dialect`'s profile,
/// the words marked `true` given the `VarWrite` role, rendered for
/// comparison: the result, then each store in order as `write N value`,
/// `element N key value` or `preserve N` — or the decline.
pub(super) fn destructured(
    command: &str,
    sub: Option<&str>,
    words: &[(&str, bool)],
    dialect: Option<&str>,
) -> Result<(String, Vec<String>), DeclineReason> {
    use tcl_registry::value_transfer::validate_outcome;
    let reg = CommandRegistry::build_default();
    let spec = reg.get(command).expect(command);
    let semantics = match sub {
        Some(name) => resolve_semantics(spec, Some(spec.subcommand(name).expect(name)), None),
        None => resolve_semantics(spec, None, None),
    };
    let semantics = semantics.semantics().expect("a destructuring route");
    let operands = words
        .iter()
        .map(|&(text, target)| literal(text, target.then_some(ArgRole::VarWrite)))
        .collect();
    let mut inputs = TestInputs::new(command, operands);
    inputs.context = AnalysisContext::detached(
        dialect.map(|name| tcl_dialect::DialectProfile::find(name).expect(name)),
    );
    match semantics.evaluate(&inputs, &mut Budget::evaluation()) {
        EvalAnswer::Evaluated(outcome) => {
            assert_eq!(
                validate_outcome(
                    &semantics.structure(&inputs),
                    &semantics.store_targets(&inputs),
                    &outcome
                ),
                Ok(()),
                "{command} {words:?}"
            );
            let text = |value: &ExactValue| String::from_utf8(value.bytes.clone()).expect("text");
            let result = match &outcome.result {
                ExactValueOrUnavailable::Exact(result) => text(result),
                ExactValueOrUnavailable::Unavailable(_) => {
                    error_label(&outcome).unwrap_or_else(|| panic!("no exact result: {outcome:?}"))
                }
            };
            let stores = outcome
                .ordered_stores
                .iter()
                .map(|store| match store {
                    StoreOutcome::Write { target, value } => {
                        format!("write {} {}", (target.0).0, text(value))
                    }
                    StoreOutcome::WriteElement { target, key, value } => {
                        format!("element {} {key} {}", (target.0).0, text(value))
                    }
                    StoreOutcome::Preserve { target } => format!("preserve {}", (target.0).0),
                    other => panic!("unexpected store {other:?}"),
                })
                .collect();
            Ok((result, stores))
        }
        EvalAnswer::Declined(reason) => Err(reason),
        EvalAnswer::Pending => panic!("pending over literal words"),
    }
}

/// The destructuring writers run the shared cores (the Storage row's
/// "partial `scan`; … repeated targets; array and base overlap"): a
/// converted field writes its variable and a field the input did not reach
/// preserves it (`scan {12 nope} {%d %d} a b` is 1, `a` 12, `b` as it was);
/// `lassign` writes in order, a repeated variable twice, and returns the
/// rest; `binary scan` writes each field it scanned; `array set` writes one
/// element per key. Every answer is tclsh's under 8.4 to 9.1 (8.5 on for
/// `lassign`, `destructuring_witnesses_match_every_release_on_path`), and a
/// form a release reads differently declines on its axis.
#[test]
fn destructuring_writers_run_the_shared_cores() {
    let (t, w) = (target, word);
    let tcl90 = Some("tcl9.0");

    assert_eq!(
        destructured(
            "scan",
            None,
            &[w("12 nope"), w("%d %d"), t("a"), t("b")],
            tcl90
        ),
        Ok(answered("1", &["write 2 12", "preserve 3"]))
    );
    assert_eq!(
        destructured("scan", None, &[w(""), w("%d %d"), t("a"), t("b")], tcl90),
        Ok(answered("-1", &["preserve 2", "preserve 3"])),
        "the input ended before any conversion"
    );
    assert_eq!(
        destructured("scan", None, &[w("12 34"), w("%d %d")], None),
        Ok(answered("12 34", &[])),
        "the inline form"
    );
    assert_eq!(
        destructured("scan", None, &[w("abc"), w("%d")], None),
        Ok(answered("{}", &[])),
        "a failed inline field is the empty string"
    );
    // Past the 32-bit range the releases disagree: `4294967296` is kept up
    // to 8.6 and clamps to `2147483647` from 9.0, so no release is taken.
    assert_eq!(
        destructured("scan", None, &[w("4294967296"), w("%d"), t("a")], tcl90),
        Err(DeclineReason::ReleaseAmbiguous(Axis::IntTower))
    );
    // `%b` arrives in 8.6.
    assert!(matches!(
        destructured("scan", None, &[w("101"), w("%b"), t("a")], Some("tcl8.5")),
        Err(DeclineReason::ReleaseAmbiguous(Axis::Availability(_)))
    ));
    assert_eq!(
        destructured("scan", None, &[w("101"), w("%b"), t("a")], Some("tcl8.6")),
        Ok(answered("1", &["write 2 5"]))
    );

    assert_eq!(
        destructured(
            "lassign",
            None,
            &[w("first second extra"), t("a"), t("a")],
            tcl90
        ),
        Ok(answered("extra", &["write 1 first", "write 2 second"]))
    );
    assert_eq!(
        destructured("lassign", None, &[w("a b"), t("x"), t("y"), t("z")], tcl90),
        Ok(answered("", &["write 1 a", "write 2 b", "write 3 "])),
        "past the end the variable is the empty string"
    );
    assert!(matches!(
        destructured("lassign", None, &[w("a b"), t("x")], Some("tcl8.4")),
        Err(DeclineReason::ReleaseAmbiguous(Axis::Availability(_)))
    ));

    scan_declines_what_the_matcher_reads_apart();
    the_byte_and_array_writers_run_the_shared_cores();
}

/// [`destructuring_writers_run_the_shared_cores`]'s `scan` declines where
/// the shared matcher and the releases part: `%u` (the matcher reads it
/// signed, where `scan -1 %u` is `18446744073709551615` on every release),
/// an infinity spelling (`-Inf` from 8.5, no conversion in the matcher) and
/// a negative zero (`scan -0 %f` is `0.0` from 8.5).
fn scan_declines_what_the_matcher_reads_apart() {
    let (t, w) = (target, word);
    let tcl90 = Some("tcl9.0");
    for (subject, format) in [("-1", "%u"), ("-inf", "%f"), ("-0", "%f")] {
        assert_eq!(
            destructured("scan", None, &[w(subject), w(format), t("v")], tcl90),
            Err(DeclineReason::Unsupported),
            "scan {subject} {format}"
        );
    }
}

/// [`destructuring_writers_run_the_shared_cores`]'s `binary scan` and
/// `array set` half.
fn the_byte_and_array_writers_run_the_shared_cores() {
    let (t, w) = (target, word);
    let tcl90 = Some("tcl9.0");
    assert_eq!(
        destructured(
            "binary",
            Some("scan"),
            &[w("scan"), w("\u{1}\u{2}"), w("cc"), t("a"), t("b")],
            tcl90
        ),
        Ok(answered("2", &["write 3 1", "write 4 2"]))
    );
    assert_eq!(
        destructured(
            "binary",
            Some("scan"),
            &[w("scan"), w("\u{1}"), w("cc"), t("a"), t("b")],
            tcl90
        ),
        Ok(answered("1", &["write 3 1", "preserve 4"])),
        "the data ran out before the second field"
    );
    assert_eq!(
        destructured(
            "binary",
            Some("scan"),
            &[w("scan"), w("\u{1}\u{2}"), w("cc"), t("a")],
            tcl90
        ),
        Err(DeclineReason::WrongRepresentation),
        "a field without a variable raises once the scan reaches it with data left"
    );

    assert_eq!(
        destructured(
            "array",
            Some("set"),
            &[w("set"), t("arr"), w("k1 v1 k2 v2 k1 v3")],
            None
        ),
        Ok(answered("", &["element 1 k1 v3", "element 1 k2 v2"]))
    );
    assert_eq!(
        destructured(
            "array",
            Some("set"),
            &[w("set"), t("arr"), w("k1 v1 k2")],
            None
        ),
        Ok(answered("error after 0", &[])),
        "an odd-length list raises, after no store"
    );
}

/// `array set a {x}` raises `list must have an even number of elements` in
/// every release (tclsh 8.4 to 9.1), after no store, with the `-errorcode`
/// `TCL ARGUMENT FORMAT` from 8.6 and `NONE` before; a profile that names no
/// release proves the message alone. A list that does not parse raises the
/// parser's error first.
#[test]
fn an_odd_array_set_list_is_the_commands_error() {
    let (t, w) = (target, word);
    let message = |text: &str| Some(text.to_owned());
    for (dialect, code) in [
        (Some("tcl8.4"), message("NONE")),
        (Some("tcl8.5"), message("NONE")),
        (Some("tcl8.6"), message("TCL ARGUMENT FORMAT")),
        (Some("tcl9.0"), message("TCL ARGUMENT FORMAT")),
        (Some("tcl9.1"), message("TCL ARGUMENT FORMAT")),
        (None, None),
    ] {
        let odd = completed(
            "array",
            Some("set"),
            &[w("set"), t("a"), w("x")],
            &[],
            dialect,
        );
        assert_eq!(
            raised(&odd),
            Some((
                0,
                message("list must have an even number of elements"),
                code
            )),
            "{dialect:?}"
        );
        assert_eq!(planned(&odd), Vec::<String>::new(), "{dialect:?}");
    }
    let malformed = completed(
        "array",
        Some("set"),
        &[w("set"), t("a"), w("{x")],
        &[],
        Some("tcl8.6"),
    );
    assert_eq!(
        raised(&malformed),
        Some((
            0,
            message("unmatched open brace in list"),
            message("TCL VALUE LIST BRACE")
        ))
    );
}
