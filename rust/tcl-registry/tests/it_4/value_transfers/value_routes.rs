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

//! Value routes that run the shared cores: list and length, `format`, `append` and `string range`.

use super::*;

/// `list`, `llength` and `string length` run the shared cores over
/// `ConstOps` on registry-owned routes (tclsh 8.4 to 9.1 give `a {b c} {}`
/// and 2; `llength "a {b"` raises; `string length héllo` read from a UTF-8
/// file is 6 up to 8.6 and 5 from 9.0, so a non-ASCII subject declines
/// where the target does not decode source as UTF-8).
#[test]
fn list_and_length_routes_run_the_shared_cores() {
    use tcl_registry::value_transfer::builtins::{LIST_LENGTH, LIST_OF_ARGS, STRING_LENGTH};
    let run = |semantics: &dyn CommandSemantics,
               command: &'static str,
               words: &[&'static str],
               dialect: Option<&str>| {
        let mut inputs = TestInputs::new(
            command,
            words.iter().map(|word| literal(word, None)).collect(),
        );
        inputs.context =
            AnalysisContext::detached(dialect.and_then(tcl_dialect::DialectProfile::find));
        evaluated(semantics.evaluate(&inputs, &mut Budget::evaluation())).map(|outcome| {
            if let Some(label) = error_label(&outcome) {
                return label;
            }
            assert!(
                outcome.ordered_stores.is_empty(),
                "{command} writes nothing"
            );
            match outcome.result {
                ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes).unwrap(),
                ExactValueOrUnavailable::Unavailable(_) => panic!("unavailable"),
            }
        })
    };
    assert_eq!(
        run(&LIST_OF_ARGS, "list", &["a", "b c", ""], None).as_deref(),
        Ok("a {b c} {}")
    );
    // A first element starting with `#` is brace-quoted from 8.5 and bare
    // in 8.4 (tclsh 8.4 prints `# a` for `puts [list # a]`, 8.5 to 9.1
    // print `{#} a`), so a profile naming no release cannot render it; a
    // `#` anywhere else is data in every release. `f5-irules` renders as
    // its 8.4 base does (ruling 8).
    for (dialect, want) in [
        (Some("tcl8.4"), Ok("# a")),
        (Some("tcl8.5"), Ok("{#} a")),
        (Some("tcl9.1"), Ok("{#} a")),
        (
            None,
            Err(DeclineReason::ReleaseAmbiguous(Axis::ListRendering)),
        ),
        (Some("f5-irules"), Ok("# a")),
    ] {
        assert_eq!(
            run(&LIST_OF_ARGS, "list", &["#", "a"], dialect),
            want.map(str::to_owned),
            "{dialect:?}"
        );
    }
    assert_eq!(
        run(&LIST_OF_ARGS, "list", &["a", "#b"], None).as_deref(),
        Ok("a #b")
    );
    assert_eq!(
        run(&LIST_LENGTH, "llength", &["a {b c}"], None).as_deref(),
        Ok("2")
    );
    assert_eq!(
        run(&LIST_LENGTH, "llength", &["a {b"], None).as_deref(),
        Ok("error after 0"),
        "a value that is no list is the command's error, after no store"
    );
    assert_eq!(
        run(
            &STRING_LENGTH,
            "string",
            &["length", "héllo"],
            Some("tcl9.0")
        )
        .as_deref(),
        Ok("5")
    );
    for dialect in [Some("tcl8.6"), None] {
        assert_eq!(
            run(&STRING_LENGTH, "string", &["length", "héllo"], dialect),
            Err(DeclineReason::ReleaseAmbiguous(Axis::SourceEncoding)),
            "{dialect:?}"
        );
    }
    assert_eq!(
        run(&STRING_LENGTH, "string", &["length", " a "], None).as_deref(),
        Ok("3")
    );
    for id in [
        NativeEvalId::ListOfArgs,
        NativeEvalId::ListLength,
        NativeEvalId::StringLength,
        NativeEvalId::FormatTemplate,
    ] {
        assert_eq!(id.owner(), EvaluatorOwner::Registry, "{id:?}");
    }
}

/// `format` over literal words under `dialect`'s release, through the
/// registry-owned route.
fn format_under(dialect: Option<&str>, words: &[&str]) -> Result<String, DeclineReason> {
    use tcl_registry::value_transfer::builtins::FORMAT_TEMPLATE;
    let profile = dialect.map(|name| tcl_dialect::DialectProfile::find(name).expect(name));
    let operands = words.iter().map(|word| literal(word, None)).collect();
    let mut inputs = TestInputs::new("format", operands);
    inputs.context = AnalysisContext::detached(profile);
    let outcome = evaluated(FORMAT_TEMPLATE.evaluate(&inputs, &mut Budget::evaluation()))?;
    match outcome.result {
        ExactValueOrUnavailable::Exact(value) => Ok(String::from_utf8(value.bytes).expect("text")),
        ExactValueOrUnavailable::Unavailable(_) => panic!("unavailable"),
    }
}

/// `format` runs the shared format core over `ConstOps` on its
/// registry-owned route. Oracle, tclsh 8.4 to 9.1: `format %5.2f 3.14159`
/// is ` 3.14`, `format %x 255` is `ff`, `format %s-%d a 5` is `a-5`,
/// `format %c 65` is `A`, `format %5s hi` is `   hi`, and `format %d abc`
/// raises — in every release, so under a profile that names none too.
#[test]
fn format_runs_the_shared_core() {
    for dialect in [
        Some("tcl8.4"),
        Some("tcl8.5"),
        Some("tcl8.6"),
        Some("tcl9.0"),
        Some("tcl9.1"),
        Some("f5-irules"),
        None,
    ] {
        for (words, want) in [
            (&["%5.2f", "3.14159"][..], " 3.14"),
            (&["%x", "255"][..], "ff"),
            (&["%s-%d", "a", "5"][..], "a-5"),
            (&["%c", "65"][..], "A"),
            (&["%5s", "hi"][..], "   hi"),
        ] {
            assert_eq!(
                format_under(dialect, words),
                Ok(want.to_owned()),
                "{dialect:?} {words:?}"
            );
        }
        assert_eq!(
            format_under(dialect, &["%d", "abc"]),
            Err(DeclineReason::WrongRepresentation),
            "{dialect:?}"
        );
    }
}

/// `format` answers under the target release's grammar, and a profile that
/// names no release answers only where every release agrees. Oracle,
/// tclsh 8.4 to 9.1: `%b` raises before 8.6 and `%p` and `%llu` before
/// 9.0; `format %d 010` is 8 up to 8.6 and 10 from 9.0; an unmodified `%d`
/// of 2147483648 is itself up to 8.6 and wraps to -2147483648 from 9.0;
/// `%#o 8` is `010` against `0o10` and `%#d 5` is `5` against `0d5`; and
/// `%.0d 0` is empty under 8.4, which formats through C's `printf`, and `0`
/// from 8.5.
#[test]
fn format_answers_per_release() {
    let error = Err(DeclineReason::WrongRepresentation);
    for (words, eight_four, eight_five, eight_six, nine) in [
        (&["%b", "5"][..], error, error, Ok("101"), Ok("101")),
        (&["%p", "255"][..], error, error, error, Ok("0xff")),
        (&["%llu", "5"][..], error, error, error, Ok("5")),
        (&["%d", "010"][..], Ok("8"), Ok("8"), Ok("8"), Ok("10")),
        (
            &["%d", "2147483648"][..],
            Ok("2147483648"),
            Ok("2147483648"),
            Ok("2147483648"),
            Ok("-2147483648"),
        ),
        (
            &["%#o", "8"][..],
            Ok("010"),
            Ok("010"),
            Ok("010"),
            Ok("0o10"),
        ),
        (&["%#d", "5"][..], Ok("5"), Ok("5"), Ok("5"), Ok("0d5")),
        (&["%.0d", "0"][..], Ok(""), Ok("0"), Ok("0"), Ok("0")),
    ] {
        for (dialect, want) in [
            ("tcl8.4", eight_four),
            ("tcl8.5", eight_five),
            ("tcl8.6", eight_six),
            ("tcl9.0", nine),
            ("tcl9.1", nine),
        ] {
            assert_eq!(
                format_under(Some(dialect), words),
                want.map(str::to_owned),
                "{dialect} {words:?}"
            );
        }
        // `f5-irules` formats as its 8.4 base does (ruling 8): tclsh 8.4
        // raises `bad field specifier` for `%b`, `%p` and `%llu`, and prints
        // 8, 2147483648, `010` and 5 for the rest.
        assert_eq!(
            format_under(Some("f5-irules"), words),
            eight_four.map(str::to_owned),
            "f5-irules {words:?}"
        );
        let answer = format_under(None, words);
        assert!(
            matches!(
                answer,
                Err(DeclineReason::ReleaseAmbiguous(
                    Axis::FormatVerbs | Axis::NumeralGrammar
                ))
            ),
            "{words:?}: {answer:?}"
        );
    }
}

/// Source-value `append` and `lappend` use `var::logical_append_value` and
/// `var::lappend_value` with the lattice write as the store. List rendering is
/// canonical; appending to a non-list is an error rather than a value. Native
/// object representation, publication and observers have their own owner.
#[test]
fn append_and_list_append_run_the_shared_cores() {
    let reg = CommandRegistry::build_default();
    let evaluate = |command: &'static str, prior: FactView, values: &[&'static str]| {
        let cell = resolve_semantics(reg.get(command).expect(command), None, None);
        let cell = cell.semantics().expect("derived");
        let mut operands = vec![literal("v", Some(ArgRole::VarWrite))];
        operands.extend(values.iter().map(|value| literal(value, None)));
        let mut inputs = TestInputs::new(command, operands);
        inputs.prior.insert("v".to_owned(), prior);
        evaluated(cell.evaluate(&inputs, &mut Budget::evaluation()))
    };
    let text = |t: &str| FactView::Exact(ExactValue::text(t), None);

    let outcome = evaluate("append", text("foo"), &["bar", " baz"]).expect("appends");
    assert_eq!(
        outcome.result,
        ExactValueOrUnavailable::Exact(ExactValue {
            bytes: b"foobar baz".to_vec(),
            numeric: None,
            representation: tcl_registry::value_transfer::RepresentationEvidence::Constructed(
                tcl_registry::TclType::String
            ),
        })
    );
    assert_eq!(outcome.ordered_stores.len(), 1);
    assert_eq!(outcome.types.result, Some(tcl_registry::TclType::String));
    let padded = evaluate("append", text(" a "), &["b"]).expect("exact bytes");
    assert!(matches!(padded.result, ExactValueOrUnavailable::Exact(ref v) if v.bytes == b" a b"));
    let numeric = evaluate("append", text("4"), &["2"]).expect("appends digits");
    assert!(
        matches!(numeric.result, ExactValueOrUnavailable::Exact(ref v) if v.bytes == b"42" && v.numeric == Some(NumericValue::Int(42)))
    );

    let outcome = evaluate("lappend", text("a b"), &["c", "d e"]).expect("appends elements");
    assert!(
        matches!(outcome.result, ExactValueOrUnavailable::Exact(ref v) if v.bytes == b"a b c {d e}")
    );
    assert_eq!(outcome.types.result, Some(tcl_registry::TclType::List));
    let outcome = evaluate("lappend", text(""), &["c"]).expect("appends to the empty list");
    assert!(matches!(outcome.result, ExactValueOrUnavailable::Exact(ref v) if v.bytes == b"c"));
    let raises = evaluate("lappend", text("{"), &["v"]).expect("the error is an outcome");
    assert_eq!(
        raises.completion,
        CompletionOutcome::Error {
            written: 0,
            message: ExactValueOrUnavailable::exact_text("unmatched open brace in list"),
            error_code: ExactValueOrUnavailable::unproven_string(),
        },
        "`lappend` over `{{` raises `unmatched open brace in list`, after no store"
    );
    assert_eq!(raises.ordered_stores.len(), 0);
    assert!(matches!(
        raises.result,
        ExactValueOrUnavailable::Unavailable(_)
    ));
    let mut inputs = TestInputs::new(
        "lappend",
        vec![literal("v", Some(ArgRole::VarWrite)), literal("x", None)],
    );
    inputs.prior.insert("v".to_owned(), FactView::Pending);
    let cell = resolve_semantics(reg.get("lappend").expect("lappend"), None, None);
    assert_eq!(
        cell.semantics()
            .expect("derived")
            .evaluate(&inputs, &mut Budget::evaluation()),
        EvalAnswer::Pending
    );
}

/// `string range` on the direct route: the shared core, with the index
/// numerals pre-resolved under the target's grammar and a non-ASCII operand
/// admitted only where the target decodes source as UTF-8. The shipped
/// `const_fold` is the same evaluator.
#[test]
fn string_range_runs_the_shared_core_with_the_index_grammar() {
    use tcl_registry::value_transfer::builtins::STRING_RANGE;
    let evaluate = |dialect: Option<&str>, args: [&'static str; 3]| {
        let mut inputs = TestInputs::new(
            "string",
            vec![
                literal("range", None),
                literal(args[0], None),
                literal(args[1], None),
                literal(args[2], None),
            ],
        );
        inputs.context =
            AnalysisContext::detached(dialect.and_then(tcl_dialect::DialectProfile::find));
        evaluated(STRING_RANGE.evaluate(&inputs, &mut Budget::evaluation())).map(|outcome| {
            if let Some(label) = error_label(&outcome) {
                return label;
            }
            match outcome.result {
                ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes).unwrap(),
                ExactValueOrUnavailable::Unavailable(_) => panic!("unavailable"),
            }
        })
    };
    assert_eq!(evaluate(None, ["hello", "1", "3"]).as_deref(), Ok("ell"));
    assert_eq!(evaluate(None, [" a ", "0", "end"]).as_deref(), Ok(" a "));
    assert_eq!(evaluate(None, ["abcdef", "-2", "2"]).as_deref(), Ok("abc"));
    assert_eq!(evaluate(None, ["abc", "end-1", "end"]).as_deref(), Ok("bc"));
    assert_eq!(evaluate(None, ["abc", "3", "1"]).as_deref(), Ok(""));
    // tclsh 8.4, 8.5, 8.6: `ijkl`; tclsh 9.0, 9.1: `kl`.
    assert_eq!(
        evaluate(Some("tcl8.6"), ["abcdefghijkl", "010", "end"]).as_deref(),
        Ok("ijkl")
    );
    assert_eq!(
        evaluate(Some("tcl9.0"), ["abcdefghijkl", "010", "end"]).as_deref(),
        Ok("kl")
    );
    assert_eq!(
        evaluate(None, ["abcdefghijkl", "010", "end"]),
        Err(DeclineReason::ReleaseAmbiguous(Axis::IndexGrammar))
    );
    // A malformed index is the command's error, after no store; the message
    // is the release's, which `a_malformed_string_range_index_is_the_commands_error`
    // words.
    assert_eq!(
        evaluate(None, ["abc", "x", "1"]).as_deref(),
        Ok("error after 0")
    );
    assert_eq!(
        evaluate(Some("tcl9.0"), ["café", "0", "2"]).as_deref(),
        Ok("caf")
    );
    assert_eq!(
        evaluate(Some("tcl9.0"), ["café", "3", "3"]).as_deref(),
        Ok("é")
    );
    for dialect in [None, Some("tcl8.6")] {
        assert_eq!(
            evaluate(dialect, ["café", "0", "2"]),
            Err(DeclineReason::ReleaseAmbiguous(Axis::SourceEncoding)),
            "{dialect:?}"
        );
    }

    let reg = CommandRegistry::build_default();
    let range = reg
        .get("string")
        .expect("string")
        .subcommand("range")
        .expect("range");
    assert!(matches!(
        range.semantics,
        SemanticsDeclaration::Declared(semantics) if semantics.route() == EvalRoute::Direct { id: NativeEvalId::StringRange }
    ));
    assert_eq!(
        range.run_const_fold(&["hello", "1", "3"], None).as_deref(),
        Some("ell")
    );
    assert_eq!(
        range
            .run_const_fold(
                &["abcdefghijkl", "010", "end"],
                Some(tcl_dialect::TclVersion::V9_0)
            )
            .as_deref(),
        Some("kl")
    );
    assert_eq!(
        range
            .run_const_fold(
                &["abcdefghijkl", "010", "end"],
                Some(tcl_dialect::TclVersion::V8_6)
            )
            .as_deref(),
        Some("ijkl")
    );
    assert_eq!(
        range.run_const_fold(&["abcdefghijkl", "010", "end"], None),
        None
    );
    assert_eq!(range.run_const_fold(&["café", "0", "2"], None), None);
}

/// A malformed `string range` index is the command's error, after no store;
/// its message is the release's (8.4 `integer or end?-integer?`, 8.5 on
/// `integer?[+-]integer? or end?[+-]integer?`) and its `-errorcode` is 8.6's
/// `TCL VALUE INDEX`, so a profile naming no release proves neither.
#[test]
fn a_malformed_string_range_index_is_the_commands_error() {
    use tcl_registry::value_transfer::builtins::STRING_RANGE;
    let inputs = |dialect: Option<&str>| {
        let mut inputs = TestInputs::new(
            "string",
            vec![
                literal("range", None),
                literal("abc", None),
                literal("x", None),
                literal("1", None),
            ],
        );
        inputs.context =
            AnalysisContext::detached(dialect.and_then(tcl_dialect::DialectProfile::find));
        inputs
    };
    let run = |dialect: Option<&str>| {
        raised(&STRING_RANGE.evaluate(&inputs(dialect), &mut Budget::evaluation()))
    };
    for (dialect, wording, code) in [
        ("tcl8.4", "integer or end?-integer?", "NONE"),
        ("tcl8.5", "integer?[+-]integer? or end?[+-]integer?", "NONE"),
        (
            "tcl8.6",
            "integer?[+-]integer? or end?[+-]integer?",
            "TCL VALUE INDEX",
        ),
        (
            "tcl9.0",
            "integer?[+-]integer? or end?[+-]integer?",
            "TCL VALUE INDEX",
        ),
    ] {
        assert_eq!(
            run(Some(dialect)),
            Some((
                0,
                Some(format!("bad index \"x\": must be {wording}")),
                Some(code.to_owned())
            )),
            "{dialect}"
        );
    }
    assert_eq!(
        run(None),
        Some((0, None, None)),
        "the releases word it differently"
    );
}
