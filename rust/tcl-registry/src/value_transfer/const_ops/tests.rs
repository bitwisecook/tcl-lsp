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

/// The detached context of a catalogue profile by name; `tcl` is the
/// lenient sink, which declares no release.
fn context(dialect: Option<&str>) -> AnalysisContext {
    AnalysisContext::detached(dialect.map(|name| {
        if name == "tcl" {
            DialectProfile::plain_tcl()
        } else {
            DialectProfile::find(name).expect("a catalogue profile")
        }
    }))
}

fn admit<'b>(dialect: Option<&str>, budget: &'b mut Budget, needs: Needs) -> ConstOps<'b> {
    ConstOps::admit(&context(dialect), budget, needs).expect("admissible")
}

#[test]
fn needs_compose_and_name_their_axes() {
    let needs = Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER;
    assert!(needs.contains(Needs::INT_TOWER));
    assert!(!needs.contains(Needs::CHAR_MODEL));
    assert_eq!(
        needs.axes().collect::<Vec<_>>(),
        vec![Axis::NumeralGrammar, Axis::IntTower]
    );
    assert!(Needs::NONE.is_empty());
}

#[test]
fn the_two_host_axes_never_admit() {
    let mut budget = Budget::evaluation();
    for needs in [Needs::PLATFORM, Needs::WALL_CLOCK] {
        let declined = ConstOps::admit(&context(None), &mut budget, needs).err();
        assert!(
            matches!(
                declined,
                Some(DeclineReason::ReleaseAmbiguous(
                    Axis::Platform | Axis::WallClock
                ))
            ),
            "{needs:?}: {declined:?}"
        );
    }
}

#[test]
fn a_core_call_outside_the_admitted_set_poisons_the_run() {
    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::NONE);
    let _ = ops.as_int(&ConstValue::text("5"));
    assert_eq!(
        ops.take(ConstValue::int(5)),
        Err(DeclineReason::MalformedAnswer)
    );
}

/// A leading zero is octal up to 8.6 and decimal from 9.0: `expr {010 +
/// 0}` is 8 on tclsh 8.4, 8.5 and 8.6 and 10 on 9.0 and 9.1. A dialect
/// that declares a base release reads under it (ruling 8): `f5-irules`,
/// `f5-iapps` and `f5-tmsh` on their 8.4-derived engine read 8, as does
/// `expect` on 8.6 and the Xilinx shell on 8.5, where before the ruling
/// they declined; a profile declaring no release (the lenient `tcl`
/// sink, and no profile at all) still declines.
#[test]
fn numerals_read_under_the_named_grammar_or_the_unanimous_one() {
    let text = ConstValue::text("010");
    for (dialect, want) in [
        (Some("tcl8.4"), Some(8)),
        (Some("tcl8.6"), Some(8)),
        (Some("tcl9.0"), Some(10)),
        (Some("tcl9.1"), Some(10)),
        (Some("f5-irules"), Some(8)),
        (Some("f5-iapps"), Some(8)),
        (Some("f5-tmsh"), Some(8)),
        (Some("expect"), Some(8)),
        (Some("xilinx-eda-tcl"), Some(8)),
        (Some("bpf"), Some(10)),
    ] {
        let mut budget = Budget::evaluation();
        let mut ops = admit(dialect, &mut budget, Needs::NUMERAL_GRAMMAR);
        assert_eq!(ops.as_int(&text).ok(), want, "{dialect:?}");
        assert!(ops.take(ConstValue::int(0)).is_ok(), "{dialect:?}");
    }
    for dialect in [None, Some("tcl")] {
        let mut budget = Budget::evaluation();
        let mut ops = admit(dialect, &mut budget, Needs::NUMERAL_GRAMMAR);
        assert!(ops.as_int(&text).is_err(), "{dialect:?}");
        assert_eq!(
            ops.take(ConstValue::int(0)),
            Err(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar)),
            "{dialect:?}"
        );
        let mut budget = Budget::evaluation();
        let mut ops = admit(dialect, &mut budget, Needs::NUMERAL_GRAMMAR);
        assert_eq!(
            ops.as_int(&ConstValue::text(" 5")).ok(),
            Some(5),
            "{dialect:?}"
        );
        assert_eq!(
            ops.as_int(&ConstValue::text("-7")).ok(),
            Some(-7),
            "{dialect:?}"
        );
        assert!(ops.as_int(&ConstValue::text("2.5")).is_err(), "{dialect:?}");
        assert!(ops.take(ConstValue::int(0)).is_ok(), "{dialect:?}");
    }
}

#[test]
fn the_integer_tower_widens_from_8_5_and_declines_elsewhere() {
    let max = ConstValue::int(i64::MAX);
    let one = ConstValue::int(1);
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut budget = Budget::evaluation();
        let mut ops = admit(
            Some(dialect),
            &mut budget,
            Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER,
        );
        let sum = ops.int_add(Some(&max), &one).expect("widens");
        assert_eq!(sum.as_utf8(), Some("9223372036854775808"), "{dialect}");
        let exact = ops.take(sum).expect("exact");
        assert_eq!(
            exact.numeric, None,
            "{dialect}: a bignum has no wide classification"
        );
    }
    let mut budget = Budget::evaluation();
    let mut ops = admit(
        Some("tcl8.4"),
        &mut budget,
        Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER,
    );
    let err = ops
        .int_add(Some(&max), &one)
        .expect_err("8.4 does not widen");
    assert_eq!(ops.decline_value(&err), DeclineReason::WrongRepresentation);
    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER);
    let err = ops.int_add(Some(&max), &one).expect_err("no release");
    assert_eq!(
        ops.decline_value(&err),
        DeclineReason::ReleaseAmbiguous(Axis::IntTower)
    );
    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER);
    assert_eq!(
        ops.int_add(None, &ConstValue::text("3"))
            .map(|v| v.as_utf8().map(str::to_owned)),
        Ok(Some("3".to_owned()))
    );
}

#[test]
fn an_integer_reads_as_the_nearest_double() {
    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::NUMERAL_GRAMMAR);
    // 2^53 + 1 has no double: the nearest, ties to even, is 2^53.
    assert_eq!(
        ops.as_double(&ConstValue::text("9007199254740993"))
            .map(f64::to_bits),
        Ok(9_007_199_254_740_992_f64.to_bits())
    );
    assert_eq!(
        ops.as_double(&ConstValue::text("-7")).map(f64::to_bits),
        Ok((-7.0_f64).to_bits())
    );
    assert!(ops.take(ConstValue::int(0)).is_ok());
}

#[test]
fn an_index_is_pre_resolved_under_the_admitted_grammar() {
    let spec = ConstValue::text("010");
    for (dialect, want) in [(Some("tcl8.6"), "8"), (Some("tcl9.0"), "10")] {
        let mut budget = Budget::evaluation();
        let mut ops = admit(dialect, &mut budget, Needs::INDEX_GRAMMAR);
        let resolved = ops.index(&spec, 12).expect("resolves");
        assert_eq!(resolved.as_utf8(), Some(want), "{dialect:?}");
    }
    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::INDEX_GRAMMAR);
    assert_eq!(
        ops.index(&spec, 12),
        Err(DeclineReason::ReleaseAmbiguous(Axis::IndexGrammar))
    );
    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::INDEX_GRAMMAR);
    assert_eq!(
        ops.index(&ConstValue::text("end-1"), 12).unwrap().as_utf8(),
        Some("10")
    );
    assert_eq!(
        ops.index(&ConstValue::text("0x2"), 12).unwrap().as_utf8(),
        Some("2")
    );
    assert_eq!(
        ops.index(&ConstValue::text("bogus"), 12),
        Err(DeclineReason::WrongRepresentation)
    );
}

/// A malformed index is the program's error, worded by the release
/// (measured, tclsh 8.4 to 9.1): 8.4 `must be integer or end?-integer?`,
/// 8.5 on `must be integer?[+-]integer? or end?[+-]integer?`, and the
/// `-errorcode` `TCL VALUE INDEX` from 8.6, `NONE` before it. A target
/// naming no release proves the message only where the releases agree.
#[test]
fn a_malformed_index_is_the_releases_error() {
    let old = "bad index \"bogus\": must be integer or end?-integer?";
    let new = "bad index \"bogus\": must be integer?[+-]integer? or end?[+-]integer?";
    let raised = |dialect: Option<&str>| {
        let mut budget = Budget::evaluation();
        let mut ops = admit(dialect, &mut budget, Needs::INDEX_GRAMMAR);
        let declined = ops.index(&ConstValue::text("bogus"), 3);
        assert_eq!(declined, Err(DeclineReason::WrongRepresentation));
        ops.raised().expect("the program's error")
    };
    let exact = ExactValueOrUnavailable::exact_text;
    for (dialect, message, code) in [
        (Some("tcl8.4"), old, "NONE"),
        (Some("tcl8.5"), new, "NONE"),
        (Some("tcl8.6"), new, "TCL VALUE INDEX"),
        (Some("tcl9.0"), new, "TCL VALUE INDEX"),
    ] {
        let expected = Raised {
            message: exact(message),
            error_code: exact(code),
        };
        assert_eq!(raised(dialect), expected, "{dialect:?}");
    }
    let unnamed = raised(None);
    assert_eq!(unnamed, Raised::unproven(), "no release: nothing agrees");
}

/// A failed core call is the program's own error, which a route that
/// models completions reads off the ops: the parsers' messages are every
/// release's, with the `-errorcode` of the release; a failure the route
/// cannot word is unproven; and a fault the run recorded wins, leaving
/// nothing raised. The fixed-width tower's overflow is no error: 8.4
/// wraps where 8.5 widens, so it stays a decline.
#[test]
fn a_failed_call_is_the_programs_error_unless_a_fault_stopped_the_run() {
    let exact = ExactValueOrUnavailable::exact_text;
    let unmatched = |dialect: Option<&str>| {
        let mut budget = Budget::evaluation();
        let mut ops = admit(dialect, &mut budget, Needs::LIST_RENDERING);
        let bad = ops
            .list_elements(&ConstValue::text("{"))
            .expect_err("not a list");
        assert_eq!(ops.decline_value(&bad), DeclineReason::WrongRepresentation);
        ops.raised()
    };
    for (dialect, code) in [
        (Some("tcl8.5"), exact("NONE")),
        (Some("tcl8.6"), exact("TCL VALUE LIST BRACE")),
        (Some("tcl9.1"), exact("TCL VALUE LIST BRACE")),
        (None, ExactValueOrUnavailable::unproven_string()),
    ] {
        let expected = Raised {
            message: exact("unmatched open brace in list"),
            error_code: code,
        };
        assert_eq!(unmatched(dialect), Some(expected), "{dialect:?}");
    }

    let mut budget = Budget::evaluation();
    let mut ops = admit(Some("tcl8.6"), &mut budget, Needs::NONE);
    let odd = ValueError::BadList("missing value to go with key".to_owned());
    assert_eq!(ops.decline_value(&odd), DeclineReason::WrongRepresentation);
    let dict = Raised {
        message: exact("missing value to go with key"),
        error_code: exact("TCL VALUE DICTIONARY"),
    };
    assert_eq!(ops.raised(), Some(dict));
    assert_eq!(ops.raised(), None, "the error is read once");
    let unworded = ValueError::BadList("no such wording".to_owned());
    assert_eq!(
        ops.decline_value(&unworded),
        DeclineReason::WrongRepresentation
    );
    assert_eq!(ops.raised(), Some(Raised::unproven()));
    let command = CmdError::new("unmatched open quote in list");
    assert_eq!(ops.decline(&command), DeclineReason::WrongRepresentation);
    let quote = Raised {
        message: exact("unmatched open quote in list"),
        error_code: exact("TCL VALUE LIST QUOTE"),
    };
    assert_eq!(ops.raised(), Some(quote), "a core's own error is worded");

    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::NONE);
    assert_eq!(
        ops.decline_value(&ValueError::IntegerOverflow),
        DeclineReason::WrongRepresentation
    );
    assert_eq!(ops.raised(), None, "an overflow is a model limit");

    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::NONE);
    let _ = ops.as_int(&ConstValue::text("5"));
    let bad = ValueError::NotInteger("x".to_owned());
    assert_eq!(ops.decline_value(&bad), DeclineReason::MalformedAnswer);
    assert_eq!(ops.raised(), None, "a fault is never an error");

    // A fault recorded after the error stops the run all the same.
    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::NONE);
    assert_eq!(ops.decline_value(&bad), DeclineReason::WrongRepresentation);
    let _ = ops.as_int(&ConstValue::text("5"));
    assert_eq!(ops.raised(), None, "the run's fault wins");
}

#[test]
fn typed_host_decline_preserves_original_cause_and_first_fault_without_guest_raise() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software constant-analysis adapter; no original-provider observation.
    let profile = DialectProfile::find("tcl8.6").unwrap();
    let cause = NativeExecutionError::HostCommandRefusal(Box::new(
        tcl_runtime_api::NativeHostCommandRefusal {
            reason: "unmatched open quote in list".to_owned(),
            source_profile: profile.cache_key(),
            native_profile: profile.cache_key(),
            interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                owner: tcl_runtime_api::native_compilation::NativeInterpreterIdentity::fresh_owner(
                ),
                interpreter: 11,
            },
            frame: 3,
            namespace: "::original".into(),
            namespace_token: Some(17),
        },
    ));
    let original = CmdError::from_execution_refusal(cause.clone());
    let mut budget = Budget::evaluation();
    let mut ops = admit(Some("tcl8.6"), &mut budget, Needs::NONE);
    assert_eq!(ops.decline(&original), DeclineReason::Unsupported);
    assert_eq!(ops.raised(), None);
    assert_eq!(original.native_execution_refusal(), Some(&cause));

    let later = CmdError::from(NativeValueAccessRefusal::Unicode(UnicodeAccessError {
        valid_up_to: 0,
        error_len: Some(1),
    }));
    assert_eq!(ops.decline(&later), DeclineReason::Unsupported);
    assert_eq!(ops.fault(), Some(DeclineReason::Unsupported));
    assert_eq!(ops.raised(), None);
    assert_eq!(original.native_execution_refusal(), Some(&cause));

    let mut budget = Budget::evaluation();
    let mut fresh = admit(Some("tcl8.6"), &mut budget, Needs::NONE);
    assert_eq!(fresh.decline(&later), DeclineReason::NotText);
    assert_eq!(fresh.raised(), None);
}

/// The error every release the target names agrees on, field by field:
/// the named release's own, or the ladder's shared answer, and a field
/// the releases disagree on is unproven.
#[test]
fn a_raised_error_is_exact_only_where_every_release_agrees() {
    let exact = ExactValueOrUnavailable::exact_text;
    let per_release = |release: TclVersion| {
        let code = if release >= TclVersion::V8_6 {
            "NEW"
        } else {
            "NONE"
        };
        ("always".to_owned(), code.to_owned())
    };
    let mut budget = Budget::evaluation();
    let ops = admit(None, &mut budget, Needs::NONE);
    let ladder = Raised::unanimous(&ops.target, per_release);
    assert_eq!(ladder.message, exact("always"));
    assert_eq!(
        ladder.error_code,
        ExactValueOrUnavailable::unproven_string()
    );
    let mut budget = Budget::evaluation();
    let ops = admit(Some("tcl8.6"), &mut budget, Needs::NONE);
    let named = Raised::unanimous(&ops.target, per_release);
    assert_eq!(named.error_code, exact("NEW"));
}

#[test]
fn character_counts_need_a_release_only_off_the_basic_plane() {
    let plain = ConstValue::text("héllo");
    let astral = ConstValue::text("\u{1F600}");
    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::CHAR_MODEL);
    assert_eq!(
        ops.native_char_len(&plain).expect("known character count"),
        5
    );
    assert!(ops.fault().is_none());
    let _ = ops.native_char_len(&astral);
    assert_eq!(
        ops.take(ConstValue::int(1)),
        Err(DeclineReason::ReleaseAmbiguous(Axis::CharacterModel))
    );
    for (dialect, want) in [("tcl8.6", 2), ("tcl9.0", 1), ("expect", 2)] {
        let mut budget = Budget::evaluation();
        let mut ops = admit(Some(dialect), &mut budget, Needs::CHAR_MODEL);
        assert_eq!(
            ops.native_char_len(&astral)
                .expect("selected character count"),
            want,
            "{dialect}"
        );
        assert!(ops.take(ConstValue::int(1)).is_ok());
    }
    // The F5 dialects declare a character model of their own
    // (`DialectProfile::character_model`) that their 8.4 base does not
    // share, so the declared divergence blocks the base's answer and the
    // axis is unanimous: the basic plane counts, an astral character
    // declines.
    for dialect in ["f5-irules", "f5-iapps"] {
        let mut budget = Budget::evaluation();
        let mut ops = admit(Some(dialect), &mut budget, Needs::CHAR_MODEL);
        assert_eq!(
            ops.native_char_len(&plain).expect("known character count"),
            5,
            "{dialect}"
        );
        let _ = ops.native_char_len(&astral);
        assert_eq!(
            ops.take(ConstValue::int(1)),
            Err(DeclineReason::ReleaseAmbiguous(Axis::CharacterModel)),
            "{dialect}"
        );
    }
}

/// Ruling 8 in one table: the release each profile evaluates under, and
/// the axes a declared divergence leaves unanimous.
#[test]
fn a_declared_base_release_is_the_release() {
    for (dialect, release) in [
        ("tcl8.4", Some(TclVersion::V8_4)),
        ("tcl9.1", Some(TclVersion::V9_1)),
        ("f5-irules", Some(TclVersion::V8_4)),
        ("f5-iapps", Some(TclVersion::V8_4)),
        ("expect", Some(TclVersion::V8_6)),
        ("cadence-eda-tcl", Some(TclVersion::V8_4)),
        ("intel-quartus-eda-tcl", Some(TclVersion::V8_5)),
        ("synopsys-eda-tcl", Some(TclVersion::V8_6)),
        ("f5-bigip", None),
    ] {
        let target = TargetSemantics::of(DialectProfile::find(dialect));
        assert_eq!(target.release, release, "{dialect}");
        assert_eq!(
            target.numerals,
            release.map(TclVersion::number_syntax),
            "{dialect}"
        );
        assert_eq!(
            target.quotes_leading_hash,
            release.map(|v| v >= TclVersion::V8_5),
            "{dialect}"
        );
    }
    let lenient = TargetSemantics::of(Some(DialectProfile::plain_tcl()));
    assert_eq!(lenient.release, None);
    assert_eq!(lenient.numerals, None);
    let irules = TargetSemantics::of(DialectProfile::find("f5-irules"));
    assert_eq!(
        irules.character_model, None,
        "the F5 character model is declared, and not 8.4's"
    );
    let expect = TargetSemantics::of(DialectProfile::find("expect"));
    assert_eq!(
        expect.character_model,
        Some(StringCharacterModel::Utf16CodeUnits)
    );
}

#[test]
fn actual_context_grammar_withdraws_mismatched_numerals_and_keeps_escapes() {
    let mut actual = context(Some("tcl9.0"));
    actual.grammar.numbers = NumberSyntax::Tcl85;
    actual.grammar.escapes = tcl_dialect::EscapeSyntax::Tcl84;
    let target = TargetSemantics::for_context(&actual);
    assert_eq!(target.release, Some(TclVersion::V9_0));
    assert_eq!(target.numerals, None);
    assert_eq!(target.grammar, actual.grammar);

    let mut budget = Budget::evaluation();
    let mut ops = ConstOps::admit(&actual, &mut budget, Needs::NUMERAL_GRAMMAR).unwrap();
    assert!(ops.as_int(&ConstValue::text("010")).is_err());
    assert_eq!(
        ops.fault(),
        Some(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar))
    );

    let mut budget = Budget::evaluation();
    let mut ops = ConstOps::admit(&actual, &mut budget, Needs::LIST_RENDERING).unwrap();
    let elements = ops.list_elements(&ConstValue::text(r"a\x41b c")).unwrap();
    assert_eq!(
        elements,
        vec![ConstValue::text("a\u{1b}"), ConstValue::text("c")]
    );
    assert!(ops.fault().is_none());
}

#[test]
fn non_ascii_text_is_admissible_only_where_source_is_utf8() {
    let value = ConstValue::text("café");
    let mut budget = Budget::evaluation();
    let mut ops = admit(Some("tcl9.0"), &mut budget, Needs::SOURCE_ENCODING);
    assert_eq!(ops.admissible_text(&value).as_deref(), Ok("café"));
    for dialect in [None, Some("tcl8.6")] {
        let mut budget = Budget::evaluation();
        let mut ops = admit(dialect, &mut budget, Needs::SOURCE_ENCODING);
        assert_eq!(
            ops.admissible_text(&value),
            Err(DeclineReason::ReleaseAmbiguous(Axis::SourceEncoding)),
            "{dialect:?}"
        );
        let mut budget = Budget::evaluation();
        let mut ops = admit(dialect, &mut budget, Needs::SOURCE_ENCODING);
        assert_eq!(
            ops.admissible_text(&ConstValue::text("cafe")).as_deref(),
            Ok("cafe"),
            "{dialect:?}"
        );
    }
}

/// A declared script encoding makes the analysed text the text every
/// release reads, so 8.x admits the non-ASCII operand it declines under
/// `auto` (#2129).
#[test]
fn a_declared_script_encoding_admits_non_ascii_text_under_every_release() {
    use crate::value_transfer::ScriptEncoding;
    let value = ConstValue::text("Ã\u{89}");
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for name in ["utf-8", "iso8859-1"] {
            let mut declared = context(Some(dialect));
            declared.script_encoding = ScriptEncoding::parse(name).expect("a script encoding");
            let mut budget = Budget::evaluation();
            let mut ops = ConstOps::admit(&declared, &mut budget, Needs::SOURCE_ENCODING)
                .expect("admissible");
            assert_eq!(
                ops.admissible_text(&value).as_deref(),
                Ok("Ã\u{89}"),
                "{dialect} {name}"
            );
        }
    }
    assert_eq!(ScriptEncoding::parse("auto"), Some(ScriptEncoding::Auto));
    assert_eq!(ScriptEncoding::parse("unicode"), None);
    assert_eq!(ScriptEncoding::parse("koi8-r"), None);
    for name in ScriptEncoding::NAMES {
        assert_eq!(
            ScriptEncoding::parse(name).map(ScriptEncoding::as_str),
            Some(name)
        );
    }
}

#[test]
fn lists_split_and_render_through_the_owner() {
    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::LIST_RENDERING);
    let items = ops
        .list_elements(&ConstValue::text("a {b c} d"))
        .expect("a list");
    assert_eq!(
        items
            .iter()
            .map(|i| i.as_utf8().unwrap())
            .collect::<Vec<_>>(),
        ["a", "b c", "d"]
    );
    let rendered = ops.new_list(items);
    assert_eq!(rendered.as_utf8(), Some("a {b c} d"));
    assert_eq!(rendered.repr, Representation::List);
    let bad = ops
        .list_elements(&ConstValue::text("{"))
        .expect_err("not a list");
    assert_eq!(bad.message(), "unmatched open brace in list");
}

#[test]
fn bytes_that_are_not_text_decline_rather_than_substitute() {
    let mut budget = Budget::evaluation();
    let mut ops = admit(None, &mut budget, Needs::NONE);
    let raw = ConstValue::bytes(&[0xff, 0xfe], Representation::ByteArray);
    assert_eq!(&*ops.as_bytes(&raw), &[0xff, 0xfe]);
    assert!(ops.try_as_str(&raw).is_err());
    assert_eq!(ops.take(raw), Err(DeclineReason::NotText));
}

#[test]
fn logical_append_retains_counted_contents_and_refuses_unprojected_binary() {
    let mut budget = Budget::evaluation();
    let mut ops = admit(Some("tcl9.0"), &mut budget, Needs::NONE);
    let value = ops
        .logical_append(Some(ConstValue::text("a\0")), &[ConstValue::text("é")])
        .expect("decoded logical contents");
    assert_eq!(&*value.bytes, "a\0é".as_bytes());
    assert_eq!(value.repr, Representation::String);
    assert!(ops.take(value).is_ok());

    let mut budget = Budget::evaluation();
    let mut ops = admit(Some("tcl9.0"), &mut budget, Needs::NONE);
    let octets = ConstValue::bytes("é".as_bytes(), Representation::ByteArray);
    assert_eq!(
        ops.logical_append(Some(octets), &[ConstValue::text("x")]),
        Err(DeclineReason::Unsupported),
        "UTF-8-looking binary storage is not a logical string-value receipt",
    );
    assert_eq!(ops.raised(), None);
}

#[test]
fn unavailable_physical_protocol_is_a_decline_without_guest_completion() {
    let mut budget = Budget::evaluation();
    let mut ops = admit(Some("tcl9.0"), &mut budget, Needs::NONE);
    let unavailable = ValueError::CommandProtocolUnavailable("physical fixture owner");
    assert_eq!(ops.decline_value(&unavailable), DeclineReason::Unsupported);
    assert_eq!(ops.raised(), None);
    assert_eq!(
        ops.take(ConstValue::text("surrogate")),
        Err(DeclineReason::Unsupported)
    );
}

#[test]
fn take_classifies_by_representation() {
    let mut budget = Budget::evaluation();
    let ops = admit(None, &mut budget, Needs::NONE);
    let exact = ops.take(ConstValue::text("42")).expect("exact");
    assert_eq!(exact.numeric, Some(NumericValue::Int(42)));
    assert_eq!(
        exact.representation,
        RepresentationEvidence::Constructed(TclType::String)
    );
    let mut budget = Budget::evaluation();
    let ops = admit(None, &mut budget, Needs::LIST_RENDERING);
    let list = ops
        .take(ConstValue::bytes(b"1", Representation::List))
        .expect("exact");
    assert_eq!(list.numeric, None, "a one-element list is not a numeral");
    assert_eq!(
        list.representation,
        RepresentationEvidence::Constructed(TclType::List)
    );
}

#[test]
fn the_budget_bounds_before_allocation() {
    let mut budget = Budget::evaluation();
    budget.allocation_bytes = 4;
    let mut ops = admit(None, &mut budget, Needs::NONE);
    assert!(ops.charge_bytes(3).is_ok());
    assert_eq!(
        ops.charge_bytes(3),
        Err(DeclineReason::Budget(
            super::super::decline::BudgetLimit::AllocationBytes
        ))
    );
    assert!(ops.take(ConstValue::text("x")).is_err());
}
