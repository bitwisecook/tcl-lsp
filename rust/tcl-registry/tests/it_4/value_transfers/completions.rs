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

//! Completions of a route: errors, writes to arrays, `unset`, `error`, `break`, `continue` and `return`.

use super::*;

/// An error is a completion, not a decline (the prefix rule, `Error {
/// written, … }`): a route that proves the command raises answers the
/// completion, after the stores that ran. `lassign {new second} a b` over
/// an array `b` writes `a` and raises on `b` — tclsh 8.5 to 9.1 leave `a`
/// at `new` and say `can't set "b": variable is array` — so it is `Error {
/// written: 1 }` with both stores planned; the message is the same in every
/// release and the `-errorcode` is 8.6's (`TCL WRITE VARNAME`, `NONE`
/// before), each exact where the target names a release. A place the rung
/// does not prove an array is no error: the answer stays the normal
/// completion's, as it was.
#[test]
fn a_route_error_is_a_completion() {
    use tcl_registry::value_transfer::BindingKind;
    let (t, w) = (target, word);
    let array = || bound_as(BindingKind::Array);
    let message = |text: &str| Some(text.to_owned());

    for (dialect, code) in [
        ("tcl8.5", "NONE"),
        ("tcl8.6", "TCL WRITE VARNAME"),
        ("tcl9.0", "TCL WRITE VARNAME"),
        ("tcl9.1", "TCL WRITE VARNAME"),
    ] {
        let answer = completed(
            "lassign",
            None,
            &[w("new second"), t("a"), t("b")],
            &[("b", array())],
            Some(dialect),
        );
        assert_eq!(
            raised(&answer),
            Some((
                1,
                message("can't set \"b\": variable is array"),
                message(code)
            )),
            "{dialect}"
        );
        assert_eq!(planned(&answer), ["write 1", "write 2"], "{dialect}");
        let EvalAnswer::Evaluated(outcome) = &answer else {
            unreachable!()
        };
        assert!(matches!(
            outcome.result,
            ExactValueOrUnavailable::Unavailable(_)
        ));
    }
    // The failing step is the first write to an array: here the second of
    // three, so the third did not run (`c` stays as it was).
    let three = completed(
        "lassign",
        None,
        &[w("x y z"), t("a"), t("b"), t("c")],
        &[("b", array())],
        Some("tcl8.6"),
    );
    assert_eq!(raised(&three).map(|raised| raised.0), Some(1));
    assert_eq!(planned(&three), ["write 1", "write 2", "write 3"]);
    let first = completed(
        "lassign",
        None,
        &[w("x y"), t("a"), t("b")],
        &[("a", array())],
        Some("tcl8.6"),
    );
    assert_eq!(raised(&first).map(|raised| raised.0), Some(0));
    // A place that is not proven an array is no proof of an error: a bound
    // scalar, an absent place, a fact the rung does not state.
    for fact in [
        bound_as(BindingKind::Scalar),
        absent(),
        bound_as(BindingKind::Either),
        FactView::Top(DeclineReason::NotExact),
    ] {
        let answer = completed(
            "lassign",
            None,
            &[w("new second"), t("a"), t("b")],
            &[("b", fact)],
            Some("tcl8.6"),
        );
        assert_eq!(raised(&answer), None);
        let EvalAnswer::Evaluated(outcome) = &answer else {
            panic!("{answer:?}");
        };
        assert_eq!(outcome.completion, CompletionOutcome::Normal);
        assert_eq!(outcome.ordered_stores.len(), 2);
    }
    // A list that does not parse is the command's error, after no store; the
    // parser's message is every release's and its `-errorcode` the
    // release's (`TCL VALUE LIST BRACE` from 8.6, `NONE` before).
    for (dialect, code) in [
        (Some("tcl8.5"), message("NONE")),
        (Some("tcl8.6"), message("TCL VALUE LIST BRACE")),
        (Some("tcl9.1"), message("TCL VALUE LIST BRACE")),
    ] {
        let malformed = completed("lassign", None, &[w("{x"), t("a")], &[], dialect);
        assert_eq!(
            raised(&malformed),
            Some((0, message("unmatched open brace in list"), code)),
            "{dialect:?}"
        );
    }
    // Before 8.5 there is no `lassign`: that is no error this route proves.
    assert_eq!(
        completed("lassign", None, &[w("x"), t("a")], &[], Some("tcl8.4")),
        EvalAnswer::Declined(DeclineReason::ReleaseAmbiguous(Axis::Availability(
            tcl_dialect::model::SpecSurface::TCL85_PLUS[0]
        )))
    );
}

/// What writing to an array raises by command and release (tclsh 8.4 to
/// 9.1): `scan`, `regexp` and `regsub` say `couldn't set variable "b"` up to
/// 8.5 and `can't set "b": variable is array` from 8.6; `binary scan`, `set`,
/// `append` and `lappend` the latter in every release; `incr` says `can't
/// read` up to 8.4. The `-errorcode` is `NONE` before 8.6. The stores before
/// the failing one ran: `scan {1 2} {%d %d} a b` and `regexp {(x)(y)} xy a
/// b` write `a` first, and the outcome types only the store that ran.
#[test]
fn writing_to_an_array_is_the_commands_error_with_its_prefix() {
    use tcl_registry::value_transfer::BindingKind;
    let (t, w) = (target, word);
    let array = || bound_as(BindingKind::Array);
    let sub = |text: &str| Some(text.to_owned());
    let cant_set = |name: &str| Some(format!("can't set \"{name}\": variable is array"));
    let couldnt = |name: &str| Some(format!("couldn't set variable \"{name}\""));
    let b = [("b", array())];

    for (dialect, scan_message, code) in [
        ("tcl8.4", couldnt("b"), "NONE"),
        ("tcl8.5", couldnt("b"), "NONE"),
        ("tcl8.6", cant_set("b"), "TCL WRITE VARNAME"),
        ("tcl9.0", cant_set("b"), "TCL WRITE VARNAME"),
        ("tcl9.1", cant_set("b"), "TCL WRITE VARNAME"),
    ] {
        let scan = completed(
            "scan",
            None,
            &[w("1 2"), w("%d %d"), t("a"), t("b")],
            &b,
            Some(dialect),
        );
        assert_eq!(
            raised(&scan),
            Some((2, scan_message.clone(), sub(code))),
            "scan under {dialect}"
        );
        assert_eq!(planned(&scan), ["write 2", "preserve 3"], "{dialect}");
        assert_eq!(
            typed_targets(&scan),
            [2],
            "the type of the store that ran, not of the one that did not"
        );
        let regexp = completed(
            "regexp",
            None,
            &[w("(x)(y)"), w("xy"), t("a"), t("b")],
            &b,
            Some(dialect),
        );
        assert_eq!(
            raised(&regexp),
            Some((1, scan_message.clone(), sub(code))),
            "regexp under {dialect}"
        );
        let regsub = completed(
            "regsub",
            None,
            &[w("x"), w("xyz"), w("Q"), t("b")],
            &b,
            Some(dialect),
        );
        assert_eq!(
            raised(&regsub),
            Some((0, scan_message, sub(code))),
            "regsub under {dialect}"
        );
        let set_like = completed(
            "binary",
            Some("scan"),
            &[w("scan"), w("abcd"), w("a2a2"), t("a"), t("b")],
            &b,
            Some(dialect),
        );
        assert_eq!(
            raised(&set_like),
            Some((1, cant_set("b"), sub(code))),
            "binary scan under {dialect}"
        );
        let incr_message = if dialect == "tcl8.4" {
            Some("can't read \"b\": variable is array".to_owned())
        } else {
            cant_set("b")
        };
        let incr = completed("incr", None, &[t("b")], &b, Some(dialect));
        assert_eq!(
            raised(&incr),
            Some((0, incr_message, sub(code))),
            "incr under {dialect}"
        );
        for command in ["append", "lappend"] {
            let appended = completed(command, None, &[t("b"), w("v")], &b, Some(dialect));
            assert_eq!(
                raised(&appended),
                Some((0, cant_set("b"), sub(code))),
                "{command} under {dialect}"
            );
        }
        let set = completed("set", None, &[t("b"), w("1")], &b, Some(dialect));
        assert_eq!(
            raised(&set),
            Some((0, cant_set("b"), sub(code))),
            "set under {dialect}"
        );
    }
}

/// `scan` goes on past a write it cannot make where `regexp` stops (tclsh
/// 8.4 to 9.1): `scan {1 2} {%d %d} b a` with `b` an array writes `a` after
/// failing on `b`, its error comes after both stores, and the failing store
/// preserves its place and types nothing; with two failures the message is
/// the first from 8.6 and both, run together, before.
#[test]
fn scan_goes_on_past_a_write_to_an_array() {
    use tcl_registry::value_transfer::BindingKind;
    let (t, w) = (target, word);
    let b = [("b", bound_as(BindingKind::Array))];
    let sub = |text: &str| Some(text.to_owned());
    let couldnt = |name: &str| format!("couldn't set variable \"{name}\"");
    let cant_set = |name: &str| format!("can't set \"{name}\": variable is array");
    for (dialect, code) in [
        ("tcl8.4", "NONE"),
        ("tcl8.5", "NONE"),
        ("tcl8.6", "TCL WRITE VARNAME"),
        ("tcl9.0", "TCL WRITE VARNAME"),
        ("tcl9.1", "TCL WRITE VARNAME"),
    ] {
        let before_8_6 = dialect == "tcl8.4" || dialect == "tcl8.5";
        let one = if before_8_6 {
            couldnt("b")
        } else {
            cant_set("b")
        };
        let past = completed(
            "scan",
            None,
            &[w("1 2"), w("%d %d"), t("b"), t("a")],
            &b,
            Some(dialect),
        );
        assert_eq!(
            (raised(&past), planned(&past), typed_targets(&past)),
            (
                Some((2, Some(one.clone()), sub(code))),
                vec!["preserve 2".to_owned(), "write 3".to_owned()],
                vec![3]
            ),
            "scan goes on past the array under {dialect}"
        );
        let twice = completed(
            "scan",
            None,
            &[w("1 2"), w("%d %d"), t("b"), t("b")],
            &b,
            Some(dialect),
        );
        let both = if before_8_6 { one.repeat(2) } else { one };
        assert_eq!(
            raised(&twice),
            Some((2, Some(both), sub(code))),
            "two failures under {dialect}"
        );
    }
}

/// A profile that names no release proves a field of the array-write error
/// only where every release agrees: `scan` is worded differently by 8.5 and
/// 8.6, so neither field is proven, and `set` is worded alike but its
/// `-errorcode` is not. Where no write reaches the array nothing is raised:
/// a no-match and a scan that stopped before the second conversion preserve
/// it.
#[test]
fn a_write_to_an_array_is_proven_only_where_every_release_agrees() {
    use tcl_registry::value_transfer::BindingKind;
    let (t, w) = (target, word);
    let cant_set = |name: &str| Some(format!("can't set \"{name}\": variable is array"));
    let b = [("b", bound_as(BindingKind::Array))];
    let spanning = completed(
        "scan",
        None,
        &[w("1 2"), w("%d %d"), t("a"), t("b")],
        &b,
        None,
    );
    assert_eq!(raised(&spanning), Some((2, None, None)));
    let set = completed("set", None, &[t("b"), w("1")], &b, None);
    assert_eq!(raised(&set), Some((0, cant_set("b"), None)));
    // Nothing written to the array, nothing raised: a no-match and a scan
    // that stopped before the second conversion preserve it.
    for answer in [
        completed(
            "regexp",
            None,
            &[w("zzz"), w("abc"), t("a"), t("b")],
            &b,
            Some("tcl8.6"),
        ),
        completed(
            "scan",
            None,
            &[w("1 x"), w("%d %d"), t("a"), t("b")],
            &b,
            Some("tcl8.6"),
        ),
    ] {
        assert_eq!(raised(&answer), None, "{answer:?}");
        assert!(matches!(
            &answer,
            EvalAnswer::Evaluated(outcome) if outcome.completion == CompletionOutcome::Normal
        ));
    }
}

/// `unset p nosuch q` raises on the absent name — `can't unset "nosuch": no
/// such variable` in every release (tclsh 8.4 to 9.1), with the
/// `-errorcode` `TCL LOOKUP VARNAME nosuch` from 8.6. `unset` unbinds in
/// order, so the names before the absent one are gone and the names after
/// it are not touched: `p` is unbound and `q` is 2 after `catch {unset p
/// nosuch q}`. With `-nocomplain` an absent name is no error, and a name an
/// earlier word of the command unset is absent by then. A name whose
/// existence the rung does not prove decides nothing.
#[test]
fn unset_raises_on_the_first_absent_name() {
    use tcl_registry::value_transfer::BindingKind;
    let (t, w) = (target, word);
    let scalar = || bound_as(BindingKind::Scalar);
    let facts = || [("p", scalar()), ("nosuch", absent()), ("q", scalar())];
    for (dialect, code) in [
        (Some("tcl8.4"), Some("NONE")),
        (Some("tcl8.5"), Some("NONE")),
        (Some("tcl8.6"), Some("TCL LOOKUP VARNAME nosuch")),
        (Some("tcl9.0"), Some("TCL LOOKUP VARNAME nosuch")),
        (Some("tcl9.1"), Some("TCL LOOKUP VARNAME nosuch")),
        (None, None),
    ] {
        let answer = completed(
            "unset",
            None,
            &[t("p"), t("nosuch"), t("q")],
            &facts(),
            dialect,
        );
        assert_eq!(
            raised(&answer),
            Some((
                1,
                Some("can't unset \"nosuch\": no such variable".to_owned()),
                code.map(str::to_owned)
            )),
            "{dialect:?}"
        );
        assert_eq!(
            planned(&answer),
            ["unbind 0", "unbind 1", "unbind 2"],
            "{dialect:?}"
        );
    }
    let nocomplain = completed(
        "unset",
        None,
        &[w("-nocomplain"), t("p"), t("nosuch"), t("q")],
        &facts(),
        Some("tcl8.6"),
    );
    assert_eq!(raised(&nocomplain), None);
    assert_eq!(planned(&nocomplain), ["unbind 1", "preserve 2", "unbind 3"]);
    let twice = completed(
        "unset",
        None,
        &[t("p"), t("p")],
        &[("p", scalar())],
        Some("tcl8.6"),
    );
    assert_eq!(raised(&twice).map(|raised| raised.0), Some(1));
    // Every name bound: the command completes and leaves each unbound.
    let all = completed(
        "unset",
        None,
        &[t("p"), t("q")],
        &[("p", scalar()), ("q", bound_as(BindingKind::Array))],
        Some("tcl8.6"),
    );
    assert_eq!(raised(&all), None);
    assert_eq!(planned(&all), ["unbind 0", "unbind 1"]);
    // A name the rung does not prove bound or unbound decides nothing.
    for fact in [
        bound_as(BindingKind::Either),
        FactView::Top(DeclineReason::NotExact),
    ] {
        let unproven = completed(
            "unset",
            None,
            &[t("p"), t("q")],
            &[("p", scalar()), ("q", fact)],
            Some("tcl8.6"),
        );
        assert!(matches!(
            unproven,
            EvalAnswer::Evaluated(_) | EvalAnswer::Declined(DeclineReason::NotExact)
        ));
    }
}

/// `error message ?info? ?code?` is the `TCL_ERROR` completion after no
/// store: the message its first word gives and the `-errorcode` its third,
/// `NONE` where none is given, in every release (tclsh 8.4 to 9.1). A word
/// the analysis does not prove leaves its field unproven and the
/// completion certain; a count `error` does not take is not worded.
#[test]
fn error_raises_its_message_and_code() {
    let w = word;
    let message = |text: &str| Some(text.to_owned());
    let raise = |words: &[(&str, bool)]| completed("error", None, words, &[], Some("tcl8.6"));
    assert_eq!(
        raised(&raise(&[w("boom")])),
        Some((0, message("boom"), message("NONE")))
    );
    assert_eq!(
        raised(&raise(&[w("boom"), w("some info"), w("CODE1")])),
        Some((0, message("boom"), message("CODE1")))
    );
    assert_eq!(
        raised(&raise(&[w("boom"), w("some info")])),
        Some((0, message("boom"), message("NONE")))
    );
    for dialect in ["tcl8.4", "tcl8.5", "tcl9.0", "tcl9.1"] {
        assert_eq!(
            raised(&completed("error", None, &[w("boom")], &[], Some(dialect))),
            Some((0, message("boom"), message("NONE"))),
            "{dialect}"
        );
    }
    let answer = raise(&[w("boom")]);
    let EvalAnswer::Evaluated(outcome) = &answer else {
        panic!("{answer:?}");
    };
    assert_eq!(outcome.ordered_stores.len(), 0);
    assert!(matches!(
        outcome.result,
        ExactValueOrUnavailable::Unavailable(_)
    ));
    // A message the analysis does not prove: the error stays certain.
    let mut inputs = TestInputs::new("error", vec![literal("$msg", None)]);
    inputs
        .operands
        .insert(0, FactView::Top(DeclineReason::NotExact));
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("error").expect("error"), None, None);
    let semantics = semantics.semantics().expect("the error route");
    assert_eq!(
        raised(&semantics.evaluate(&inputs, &mut Budget::evaluation())),
        Some((0, None, message("NONE")))
    );
    inputs.operands.insert(0, FactView::Pending);
    assert_eq!(
        semantics.evaluate(&inputs, &mut Budget::evaluation()),
        EvalAnswer::Pending
    );
    assert_eq!(raise(&[]), EvalAnswer::Declined(DeclineReason::Unsupported));
    assert_eq!(
        raise(&[w("a"), w("b"), w("c"), w("d")]),
        EvalAnswer::Declined(DeclineReason::Unsupported)
    );
}

/// What a completion route proves of a command's completion: the code a
/// caller observes (what `catch` returns), the `-code` and `-level` of its
/// return options, and the result where it is exact.
fn code_completion(answer: &EvalAnswer) -> Option<(i64, i64, u32, Option<String>)> {
    let EvalAnswer::Evaluated(outcome) = answer else {
        return None;
    };
    let (code, level) = outcome.completion.options_code_and_level();
    let result = match &outcome.result {
        ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes.clone()).ok(),
        ExactValueOrUnavailable::Unavailable(_) => None,
    };
    Some((outcome.completion.observed_code(), code, level, result))
}

/// `break` and `continue` complete with the code of the same name, which a
/// caller observes as `catch` does (tclsh 8.4 to 9.0): `catch {break}` is 3
/// and `catch {continue}` 4, each with the empty result and the options
/// `-code N -level 0`. The commands take no word, and a word is `wrong #
/// args`, which the route does not word.
#[test]
fn break_and_continue_complete_with_their_codes() {
    let w = word;
    for dialect in [Some("tcl8.4"), Some("tcl8.6"), Some("tcl9.0"), None] {
        let run =
            |command: &str, words: &[(&str, bool)]| completed(command, None, words, &[], dialect);
        assert_eq!(
            code_completion(&run("break", &[])),
            Some((3, 3, 0, Some(String::new())))
        );
        assert_eq!(
            code_completion(&run("continue", &[])),
            Some((4, 4, 0, Some(String::new())))
        );
        for command in ["break", "continue"] {
            assert_eq!(
                run(command, &[w("extra")]),
                EvalAnswer::Declined(DeclineReason::Unsupported)
            );
        }
    }
}

/// `return` is `TCL_RETURN`, 2, with the result its last word gives and the
/// `-code` and `-level` it carries (tclsh 8.4 to 9.0, measured through
/// `catch`): `return -code error boom` is 2 with `-code 1 -level 1` until a
/// procedure consumes the level, and from 8.5, where `-level` begins,
/// `return -level 0 val` is a normal completion, `return -level 0 -code
/// error msg` the error, and `-level 0 -code break` the code 3 itself. A
/// lone word is the result whatever it starts with: `return -x` is `-x`.
#[test]
fn return_completes_with_its_code_and_level() {
    let w = word;
    let text = |text: &str| Some(text.to_owned());
    for dialect in [Some("tcl8.4"), Some("tcl8.6"), Some("tcl9.0"), None] {
        let seen = |words: &[(&str, bool)]| {
            code_completion(&completed("return", None, words, &[], dialect))
        };
        // Under every target a level the form does not name is 1.
        assert_eq!(seen(&[]), Some((2, 0, 1, text(""))));
        assert_eq!(seen(&[w("abc")]), Some((2, 0, 1, text("abc"))));
        assert_eq!(seen(&[w("-x")]), Some((2, 0, 1, text("-x"))));
        for (words, code, result) in [
            (&[w("-code"), w("error")][..], 1, ""),
            (&[w("-code"), w("error"), w("boom")][..], 1, "boom"),
            (&[w("-code"), w("5"), w("custom")][..], 5, "custom"),
            (&[w("-code"), w("break")][..], 3, ""),
            (&[w("-code"), w("continue"), w("x")][..], 4, "x"),
            (&[w("-code"), w("ok"), w("val")][..], 0, "val"),
            (&[w("-code"), w("-3"), w("neg")][..], -3, "neg"),
        ] {
            assert_eq!(seen(words), Some((2, code, 1, text(result))), "{words:?}");
        }
    }

    for dialect in [
        Some("tcl8.5"),
        Some("tcl8.6"),
        Some("tcl9.0"),
        Some("tcl9.1"),
    ] {
        let run = |words: &[(&str, bool)]| completed("return", None, words, &[], dialect);
        let seen = |words: &[(&str, bool)]| code_completion(&run(words));
        assert_eq!(
            seen(&[w("-level"), w("0"), w("val")]),
            Some((0, 0, 0, text("val"))),
            "{dialect:?}"
        );
        assert_eq!(
            seen(&[w("-code"), w("ok"), w("-level"), w("0"), w("val")]),
            Some((0, 0, 0, text("val")))
        );
        assert_eq!(
            seen(&[w("-level"), w("2"), w("val")]),
            Some((2, 0, 2, text("val")))
        );
        assert_eq!(
            seen(&[w("-level"), w("0"), w("-code"), w("break")]),
            Some((3, 3, 0, text("")))
        );
        assert_eq!(
            seen(&[w("-level"), w("0"), w("-code"), w("7"), w("seven")]),
            Some((7, 7, 0, text("seven")))
        );
        assert_eq!(
            raised(&run(&[
                w("-level"),
                w("0"),
                w("-code"),
                w("error"),
                w("msg")
            ])),
            Some((0, text("msg"), text("NONE")))
        );
    }

    // 8.4 reads three options and rejects `-level` ("bad option "-level":
    // must be -code, -errorcode, or -errorinfo"), an error the route does not
    // word; a target that names no release leaves it to the release.
    assert_eq!(
        raised(&completed(
            "return",
            None,
            &[w("-level"), w("0")],
            &[],
            Some("tcl8.4")
        )),
        Some((0, None, None))
    );
    assert!(matches!(
        completed("return", None, &[w("-level"), w("0")], &[], None),
        EvalAnswer::Declined(DeclineReason::ReleaseAmbiguous(_))
    ));
}

/// `return` reads its options as the release does (tclsh 8.6 and 9.0,
/// measured through `catch`): from 8.5 any pair is kept in the options
/// dictionary and changes nothing (`return a b` is 2 with the empty result),
/// `-errorcode` beside `-code error` leaves the completion pending a level, a
/// code or level is read with the release's numerals (`-code 010` is 8 in 8.6
/// and 10 from 9.0, `0x5` and `+5` are 5, `2147483648` wraps to
/// `-2147483648`, `-level 01` is 1), and a code or level the release rejects
/// is the error it raises, which the route does not word. `-options`, whose
/// dictionary the route does not merge, declines; so does a word that is not
/// exact. A result the analysis does not prove leaves the completion certain,
/// and a word not yet reached leaves the answer pending.
#[test]
fn return_reads_its_options_as_the_release_does() {
    let w = word;
    let text = |text: &str| Some(text.to_owned());
    for (dialect, octal) in [(Some("tcl8.6"), 8), (Some("tcl9.0"), 10)] {
        let run = |words: &[(&str, bool)]| completed("return", None, words, &[], dialect);
        let seen = |words: &[(&str, bool)]| code_completion(&run(words));
        for (words, code, result) in [
            (
                &[w("-errorcode"), w("A"), w("-code"), w("error"), w("m")][..],
                1,
                "m",
            ),
            (&[w("a"), w("b")][..], 0, ""),
            (&[w("-foo"), w("bar")][..], 0, ""),
            (&[w("-code"), w("010"), w("x")][..], octal, "x"),
            (&[w("-code"), w("0x5"), w("x")][..], 5, "x"),
            (&[w("-code"), w("+5"), w("x")][..], 5, "x"),
            (
                &[w("-code"), w("2147483648"), w("x")][..],
                -2_147_483_648,
                "x",
            ),
            (&[w("-level"), w("01"), w("x")][..], 0, "x"),
        ] {
            assert_eq!(
                seen(words),
                Some((2, code, 1, text(result))),
                "{dialect:?}: {words:?}"
            );
        }
        for words in [
            &[w("-code"), w("notacode"), w("x")][..],
            &[w("-level"), w("-1"), w("x")][..],
        ] {
            assert_eq!(
                raised(&run(words)),
                Some((0, None, None)),
                "{dialect:?}: {words:?}"
            );
        }
        // An `-options` value that is no dictionary is rejected at level 0
        // (tclsh 8.6 to 9.1: `expected dict but got "{-code 1}"`).
        assert_eq!(
            raised(&run(&[w("-options"), w("{-code 1}"), w("m")])),
            Some((0, None, None))
        );
        // `-code return` is `ok` one level further out: `catch {return -code
        // return x} m o` leaves `o` holding `-code 0 -level 2`.
        assert_eq!(
            seen(&[w("-code"), w("return"), w("x")]),
            Some((2, 0, 2, text("x")))
        );
        // From 8.5 an `-errorcode` that is no list is rejected ("bad
        // -errorcode value: expected a list").
        assert_eq!(
            raised(&run(&[w("-errorcode"), w("a {b"), w("x")])),
            Some((0, None, None))
        );
        // At level 0 the error carries the `-errorcode` given.
        assert_eq!(
            raised(&run(&[
                w("-level"),
                w("0"),
                w("-code"),
                w("error"),
                w("-errorcode"),
                w("A B"),
                w("msg")
            ])),
            Some((0, text("msg"), text("A B")))
        );
    }

    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("return").expect("return"), None, None);
    let semantics = semantics.semantics().expect("the return route");
    let mut inputs = TestInputs::new(
        "return",
        vec![
            literal("-code", None),
            literal("5", None),
            literal("$v", None),
        ],
    );
    inputs
        .operands
        .insert(2, FactView::Top(DeclineReason::NotExact));
    assert_eq!(
        code_completion(&semantics.evaluate(&inputs, &mut Budget::evaluation())),
        Some((2, 5, 1, None))
    );
    inputs
        .operands
        .insert(1, FactView::Top(DeclineReason::NotExact));
    assert_eq!(
        semantics.evaluate(&inputs, &mut Budget::evaluation()),
        EvalAnswer::Declined(DeclineReason::NotExact)
    );
    inputs.operands.insert(1, FactView::Pending);
    assert_eq!(
        semantics.evaluate(&inputs, &mut Budget::evaluation()),
        EvalAnswer::Pending
    );
}

/// Reading an absent variable, and a value that is no integer, are the
/// command's error: `set x` over an unbound `x` is `can't read "x": no such
/// variable` in every release (tclsh 8.4 to 9.1) with `-errorcode` `NONE`
/// before 8.6 and `TCL LOOKUP VARNAME x` from it; `incr n abc` raises
/// `expected integer but got "abc"` after no store. Neither is a value, and
/// the integer tower's overflow — a value 8.4 computes and the model does
/// not — is no error.
#[test]
fn a_read_of_nothing_and_a_non_integer_are_errors() {
    let reg = CommandRegistry::build_default();
    let set = resolve_semantics(reg.get("set").expect("set"), None, None);
    let set = set.semantics().expect("the cell write");
    let read = |fact: FactView, dialect: Option<&str>| {
        let mut inputs = TestInputs::new("set", vec![literal("x", Some(ArgRole::VarRead))]);
        inputs.prior.insert("x".to_owned(), fact);
        inputs.context = AnalysisContext::detached(
            dialect.map(|name| tcl_dialect::DialectProfile::find(name).expect(name)),
        );
        set.evaluate(&inputs, &mut Budget::evaluation())
    };
    for (dialect, code) in [
        (Some("tcl8.4"), Some("NONE")),
        (Some("tcl8.5"), Some("NONE")),
        (Some("tcl8.6"), Some("TCL LOOKUP VARNAME x")),
        (Some("tcl9.1"), Some("TCL LOOKUP VARNAME x")),
        (None, None),
    ] {
        assert_eq!(
            raised(&read(absent(), dialect)),
            Some((
                0,
                Some("can't read \"x\": no such variable".to_owned()),
                code.map(str::to_owned)
            )),
            "{dialect:?}"
        );
    }
    // A place that may be bound is read for its value, not for an error.
    let either = bound_as(tcl_registry::value_transfer::BindingKind::Either);
    assert_eq!(raised(&read(either, Some("tcl8.6"))), None);
    // A whole array read as a scalar is the command's error too, with `TCL
    // READ VARNAME` from 8.6.
    for (dialect, code) in [
        (Some("tcl8.5"), Some("NONE")),
        (Some("tcl8.6"), Some("TCL READ VARNAME")),
        (None, None),
    ] {
        let array = bound_as(tcl_registry::value_transfer::BindingKind::Array);
        assert_eq!(
            raised(&read(array, dialect)),
            Some((
                0,
                Some("can't read \"x\": variable is array".to_owned()),
                code.map(str::to_owned)
            )),
            "{dialect:?}"
        );
    }
    // An element's message and code say whether its array exists, which the
    // fact does not: the error is certain and its wording unproven.
    let mut element = TestInputs::new("set", vec![literal("a(1)", Some(ArgRole::VarRead))]);
    element.prior.insert("a(1)".to_owned(), absent());
    element.context = AnalysisContext::detached(tcl_dialect::DialectProfile::find("tcl8.6"));
    assert_eq!(
        raised(&set.evaluate(&element, &mut Budget::evaluation())),
        Some((0, None, None))
    );
    // The overflow of 8.4's fixed-width increment is a value, not an error.
    let cell = resolve_semantics(
        CommandRegistry::build_default().get("incr").expect("incr"),
        None,
        None,
    );
    let cell = cell.semantics().expect("derived");
    assert_eq!(
        evaluate_increment(
            cell,
            Some("tcl8.4"),
            FactView::Exact(ExactValue::int(i64::MAX), None),
            None
        ),
        EvalAnswer::Declined(DeclineReason::WrongRepresentation)
    );
}
