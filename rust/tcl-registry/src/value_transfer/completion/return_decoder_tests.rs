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
use crate::completion_route::ReturnInvocationGrammar as Grammar;
use crate::invocation_words::InvocationWord::{KnownBytes as B, Literal as L};

// Post-evaluation values of the exact native request cases. Counted NUL
// values are binary-produced in the probe, not source NUL characters.
const CASES: &[(&str, &[InvocationWord<'static>])] = &[
    ("empty", &[]),
    ("result", &[L("RESULT")]),
    ("ordinary_pair", &[L("foo"), L("bar")]),
    ("ordinary_pair_result", &[L("foo"), L("bar"), L("RESULT")]),
    ("ordinary_pairs", &[L("foo"), L("bar"), L("baz"), L("quux")]),
    ("lone_code", &[L("-code")]),
    ("lone_level", &[L("-level")]),
    ("lone_options", &[L("-options")]),
    ("lone_dashdash", &[L("--")]),
    ("dashdash_pair", &[L("--"), L("VALUE")]),
    ("code_ok", &[L("-code"), L("ok"), L("RESULT")]),
    ("code_error", &[L("-code"), L("error"), L("RESULT")]),
    ("code_return", &[L("-code"), L("return"), L("RESULT")]),
    ("code_break", &[L("-code"), L("break"), L("RESULT")]),
    ("code_continue", &[L("-code"), L("continue"), L("RESULT")]),
    ("code_signal", &[L("-code"), L("signal"), L("RESULT")]),
    ("code_exit", &[L("-code"), L("exit"), L("RESULT")]),
    ("code_eval", &[L("-code"), L("eval"), L("RESULT")]),
    ("code_prefix", &[L("-code"), L("o"), L("RESULT")]),
    ("code_010", &[L("-code"), L("010"), L("RESULT")]),
    ("code_uintmax", &[L("-code"), L("4294967295"), L("RESULT")]),
    (
        "code_negative_uintmax",
        &[L("-code"), L("-4294967295"), L("RESULT")],
    ),
    (
        "code_over_uintmax",
        &[L("-code"), L("4294967296"), L("RESULT")],
    ),
    ("code_intmin", &[L("-code"), L("-2147483648"), L("RESULT")]),
    (
        "code_bad_then_ok",
        &[L("-code"), L("BAD"), L("-code"), L("ok"), L("RESULT")],
    ),
    (
        "code_ok_then_bad",
        &[L("-code"), L("ok"), L("-code"), L("BAD"), L("RESULT")],
    ),
    ("level_zero", &[L("-level"), L("0"), L("RESULT")]),
    ("level_two", &[L("-level"), L("2"), L("RESULT")]),
    ("level_negative", &[L("-level"), L("-1"), L("RESULT")]),
    ("level_010", &[L("-level"), L("010"), L("RESULT")]),
    ("level_intmax", &[L("-level"), L("2147483647"), L("RESULT")]),
    (
        "level_over_intmax",
        &[L("-level"), L("2147483648"), L("RESULT")],
    ),
    (
        "level_uintmax",
        &[L("-level"), L("4294967295"), L("RESULT")],
    ),
    (
        "level_bad_then_zero",
        &[L("-level"), L("BAD"), L("-level"), L("0"), L("RESULT")],
    ),
    (
        "level_negative_then_zero",
        &[L("-level"), L("-1"), L("-level"), L("0"), L("RESULT")],
    ),
    (
        "errorcode_valid",
        &[
            L("-code"),
            L("error"),
            L("-errorcode"),
            L("A B"),
            L("RESULT"),
        ],
    ),
    ("errorcode_bad", &[L("-errorcode"), L("{"), L("RESULT")]),
    (
        "errorcode_bad_then_good",
        &[
            L("-errorcode"),
            L("{"),
            L("-errorcode"),
            L("A B"),
            L("RESULT"),
        ],
    ),
    ("errorinfo_any", &[L("-errorinfo"), L("{"), L("RESULT")]),
    (
        "errorstack_odd",
        &[L("-errorstack"), L("INNER"), L("RESULT")],
    ),
    (
        "errorstack_even",
        &[L("-errorstack"), L("INNER command"), L("RESULT")],
    ),
    ("errorstack_bad", &[L("-errorstack"), L("{"), L("RESULT")]),
    (
        "errorstack_bad_then_good",
        &[
            L("-errorstack"),
            L("{"),
            L("-errorstack"),
            L("INNER command"),
            L("RESULT"),
        ],
    ),
    ("custom_dash", &[L("-custom"), L("VALUE"), L("RESULT")]),
    ("custom_bare", &[L("custom"), L("VALUE"), L("RESULT")]),
    ("custom_dynamic_value", &[L("-custom"), L("V"), L("RESULT")]),
    ("options_empty", &[L("-options"), L(""), L("RESULT")]),
    (
        "options_code_ok",
        &[L("-options"), L("-code ok -level 0"), L("RESULT")],
    ),
    ("options_odd", &[L("-options"), L("-code"), L("RESULT")]),
    (
        "options_bad_code",
        &[L("-options"), L("-code BAD"), L("RESULT")],
    ),
    (
        "options_bad_overridden",
        &[
            L("-options"),
            L("-code BAD"),
            L("-code"),
            L("ok"),
            L("RESULT"),
        ],
    ),
    (
        "options_bad_level_overridden",
        &[
            L("-options"),
            L("-level -1"),
            L("-level"),
            L("0"),
            L("RESULT"),
        ],
    ),
    (
        "options_overrides_bad_code",
        &[
            L("-code"),
            L("BAD"),
            L("-options"),
            L("-code ok"),
            L("RESULT"),
        ],
    ),
    (
        "options_nested",
        &[
            L("-options"),
            L("-options {-code ok -level 0}"),
            L("RESULT"),
        ],
    ),
    (
        "options_code_key_bare",
        &[L("-options"), L("code error"), L("RESULT")],
    ),
    (
        "code_return_level_zero",
        &[L("-code"), L("return"), L("-level"), L("0"), L("RESULT")],
    ),
    (
        "code_return_level_two",
        &[L("-code"), L("return"), L("-level"), L("2"), L("RESULT")],
    ),
    (
        "code_break_level_zero",
        &[L("-code"), L("break"), L("-level"), L("0"), L("RESULT")],
    ),
    (
        "nul_key_code",
        &[B(b"-code\0tail"), L("error"), L("RESULT")],
    ),
    ("nul_code_ok", &[L("-code"), B(b"ok\0tail"), L("RESULT")]),
    (
        "nul_custom_key",
        &[B(b"custom\0tail"), L("VALUE"), L("RESULT")],
    ),
    (
        "level_negative_uintmax",
        &[L("-level"), L("-4294967295"), L("RESULT")],
    ),
    (
        "level_negative_over_intmin",
        &[L("-level"), L("-2147483649"), L("RESULT")],
    ),
    (
        "level_negative_over_uintmax",
        &[L("-level"), L("-4294967296"), L("RESULT")],
    ),
    (
        "level_over_uintmax",
        &[L("-level"), L("4294967296"), L("RESULT")],
    ),
    ("level_named_ok", &[L("-level"), L("ok"), L("RESULT")]),
    (
        "options_nested_before_outer_code",
        &[
            L("-options"),
            L("-options {-code error} -code ok"),
            L("RESULT"),
        ],
    ),
    (
        "options_nested_after_outer_code",
        &[
            L("-options"),
            L("-code ok -options {-code error}"),
            L("RESULT"),
        ],
    ),
    (
        "options_repeated_nested_bad_then_good",
        &[
            L("-options"),
            L("-options {-code BAD} -options {-code ok}"),
            L("RESULT"),
        ],
    ),
    (
        "options_repeated_nested_invalid_then_good",
        &[
            L("-options"),
            L("-options \\{ -options {-code ok}"),
            L("RESULT"),
        ],
    ),
    (
        "options_repeated_code_bad_then_good",
        &[L("-options"), L("-code BAD -code ok"), L("RESULT")],
    ),
    (
        "options_odd_then_empty",
        &[L("-options"), L("-code"), L("-options"), L(""), L("RESULT")],
    ),
    (
        "options_bad_list_then_empty",
        &[L("-options"), L("{"), L("-options"), L(""), L("RESULT")],
    ),
    (
        "options_errorcode_origin",
        &[
            L("-options"),
            L("-code error -level 0 -errorcode {MERGED CODE}"),
            L("RESULT"),
        ],
    ),
    (
        "nul_numeric_code",
        &[L("-code"), B(b"1\0tail"), L("RESULT")],
    ),
    (
        "nul_numeric_level",
        &[L("-level"), B(b"1\0tail"), L("RESULT")],
    ),
];

struct Provider {
    profile: &'static str,
    source: &'static str,
    extension: &'static str,
    worker: &'static str,
}

const PROVIDERS: &[Provider] = &[
    Provider {
        profile: "tcl8.4",
        source: include_str!(
            "../../../tests/data/native_return_option_pair_grammar379/tcl8.4/stdout"
        ),
        extension: include_str!(
            "../../../tests/data/native_return_option_pair_grammar380/tcl8.4/stdout"
        ),
        worker: include_str!("../../../tests/data/native_return_evaluated_worker383/tcl8.4/stdout"),
    },
    Provider {
        profile: "tcl8.5",
        source: include_str!(
            "../../../tests/data/native_return_option_pair_grammar379/tcl8.5/stdout"
        ),
        extension: include_str!(
            "../../../tests/data/native_return_option_pair_grammar380/tcl8.5/stdout"
        ),
        worker: include_str!("../../../tests/data/native_return_evaluated_worker383/tcl8.5/stdout"),
    },
    Provider {
        profile: "tcl8.6",
        source: include_str!(
            "../../../tests/data/native_return_option_pair_grammar379/tcl8.6/stdout"
        ),
        extension: include_str!(
            "../../../tests/data/native_return_option_pair_grammar380/tcl8.6/stdout"
        ),
        worker: include_str!("../../../tests/data/native_return_evaluated_worker383/tcl8.6/stdout"),
    },
    Provider {
        profile: "tcl9.0",
        source: include_str!(
            "../../../tests/data/native_return_option_pair_grammar379/tcl9.0/stdout"
        ),
        extension: include_str!(
            "../../../tests/data/native_return_option_pair_grammar380/tcl9.0/stdout"
        ),
        worker: include_str!("../../../tests/data/native_return_evaluated_worker383/tcl9.0/stdout"),
    },
    Provider {
        profile: "tcl9.1",
        source: include_str!(
            "../../../tests/data/native_return_option_pair_grammar379/tcl9.1/stdout"
        ),
        extension: include_str!(
            "../../../tests/data/native_return_option_pair_grammar380/tcl9.1/stdout"
        ),
        worker: include_str!("../../../tests/data/native_return_evaluated_worker383/tcl9.1/stdout"),
    },
    Provider {
        profile: "jim",
        source: include_str!("../../../tests/data/native_return_option_pair_grammar379/jim/stdout"),
        extension: include_str!(
            "../../../tests/data/native_return_option_pair_grammar380/jim/stdout"
        ),
        worker: include_str!("../../../tests/data/native_return_evaluated_worker383/jim/stdout"),
    },
];

fn decoded(
    provider: &Provider,
    words: &[InvocationWord<'_>],
    facet: ReturnInvocationFacet,
) -> ReturnDecoding {
    let profile = crate::native_test_provider::profile(provider.profile);
    let dialect = crate::InvocationDialect::of_profile(profile);
    let numbers = if provider.profile == "jim" {
        Numbers::Target(NumberSyntax::Jim)
    } else {
        Numbers::Target(dialect.numbers)
    };
    decode_return_words_in(
        Grammar::for_dialect(dialect),
        numbers,
        InvocationArguments::structured(words).with_dialect(dialect),
        facet,
    )
}

fn hex(text: &str) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    for byte in text.bytes() {
        write!(out, "{byte:02x}").expect("String formatting");
    }
    out
}

fn compare_public_fields(provider: &Provider, row: &str, facet: ReturnInvocationFacet) -> bool {
    let fields: Vec<_> = row.split('|').collect();
    assert_eq!(fields.len(), 8, "{}: {row}", provider.profile);
    let label = fields[1];
    let words = CASES
        .iter()
        .find(|(name, _)| *name == label)
        .expect("exact native case label")
        .1;
    let answer = decoded(provider, words, facet);
    let context = format!("{} {label} {facet:?}", provider.profile);
    if provider.profile == "jim"
        && matches!(
            label,
            "level_over_intmax" | "level_uintmax" | "level_over_uintmax"
        )
    {
        // The original public Jim rows are retained in full. This API has
        // no signed-wide level field, so none is fabricated from the u32
        // field, the observed catch cast or a C recipe.
        assert_eq!(
            answer,
            ReturnDecoding::Unknown(ReturnUnknown::Release),
            "{context}"
        );
        return false;
    }
    if provider.profile == "tcl8.5"
        && facet == ReturnInvocationFacet::OriginalSource
        && matches!(
            label,
            "options_nested_before_outer_code" | "options_repeated_nested_invalid_then_good"
        )
    {
        // Source380 and original-worker383 disagree at these exact rows.
        // Source values do not identify a selected compiler artifact.
        assert_eq!(
            answer,
            ReturnDecoding::Unknown(ReturnUnknown::Release),
            "{context}"
        );
        return false;
    }
    if provider.profile != "tcl8.4"
        && provider.profile != "jim"
        && label == "options_errorcode_origin"
    {
        // An accepted level-zero error also has catch code one. Its
        // supplied result and errorcode distinguish acceptance from a
        // grammar rejection; code one alone must not pass this control.
        assert!(matches!(answer, ReturnDecoding::Completes(_)), "{context}");
    }
    match answer {
        ReturnDecoding::Rejects => assert_eq!(fields[2], "1", "{context}"),
        ReturnDecoding::Unknown(reason) => {
            panic!("unresolved measured fields {context}: {reason:?}")
        }
        ReturnDecoding::Completes(returned) => {
            assert_eq!(
                returned.own_code().as_int().to_string(),
                fields[2],
                "{context}"
            );
            if fields[4] != "ABSENT" {
                assert_eq!(returned.code.as_int().to_string(), fields[4], "{context}");
                assert_eq!(returned.level.to_string(), fields[5], "{context}");
            }
            let result = returned.result.map_or("", |at| {
                return_word_text(match words[at] {
                    L(text) => ReturnWord::Text(text),
                    B(bytes) => ReturnWord::Bytes(bytes),
                    _ => panic!("exact result fixture"),
                })
                .expect("known result")
            });
            assert_eq!(hex(result), fields[3], "{context}");
            if returned.code == CompletionCode::Error
                && fields[6] != "ABSENT"
                && fields[4] != "ABSENT"
            {
                let error_code = returned
                    .error_code
                    .and_then(|at| match words[at] {
                        L(text) => Some(text),
                        _ => None,
                    })
                    .or(returned.merged_error_code.as_deref())
                    .unwrap_or("NONE");
                assert_eq!(hex(error_code), fields[6], "{context}");
            }
        }
    }
    true
}

#[test]
fn source_return_decoder_matches_measured_public_fields_with_explicit_refusals() {
    // naming.completion.return-option-pair-grammar
    // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
    // Original source379/380 only: own code, accepted result, and available
    // catch options. No compiler instruction, object, frame or message claim.
    for provider in PROVIDERS {
        let mut compared = 0;
        let mut count = 0;
        for row in provider
            .source
            .lines()
            .chain(provider.extension.lines())
            .filter(|line| line.starts_with("ROW|"))
        {
            compared += usize::from(compare_public_fields(
                provider,
                row,
                ReturnInvocationFacet::OriginalSource,
            ));
            count += 1;
        }
        assert_eq!(count, 76);
        assert_eq!(
            compared,
            if provider.profile == "jim" {
                73
            } else if provider.profile == "tcl8.5" {
                74
            } else {
                76
            }
        );
    }
}

#[test]
fn evaluated_return_decoder_matches_original_worker_public_fields() {
    // naming.completion.return-evaluated-argument-worker-grammar
    // docs/design/analysis/name-resolution-proofs/completion-return-evaluated-argument-worker-grammar.md
    // Alias-to-original-worker383 keeps every original operand. This compares
    // public fields only, independently of source379/380 compilation.
    for provider in PROVIDERS {
        let rows: Vec<_> = provider
            .worker
            .lines()
            .filter(|line| line.starts_with("ROW|"))
            .collect();
        assert_eq!(rows.len(), 76);
        let compared = rows
            .into_iter()
            .filter(|row| {
                compare_public_fields(provider, row, ReturnInvocationFacet::EvaluatedArguments)
            })
            .count();
        assert_eq!(compared, if provider.profile == "jim" { 73 } else { 76 });
    }
}

#[test]
fn return_decoder_preserves_unknown_values_count_and_original_ordinals() {
    // naming.completion.return-option-pair-grammar
    // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
    // Software contract: constructed source carriers test uncertainty and
    // provenance. They are not independently observed Native entry/header facts.
    let profile = crate::native_test_provider::profile("tcl8.6");
    let dialect = crate::InvocationDialect::of_profile(profile);
    let read = |words: &[InvocationWord<'_>]| {
        decode_return_words_in(
            Grammar::OptionsTcl,
            Numbers::Target(dialect.numbers),
            InvocationArguments::structured(words).with_dialect(dialect),
            ReturnInvocationFacet::OriginalSource,
        )
    };
    assert_eq!(
        read(&[L("-code"), InvocationWord::Dynamic]),
        ReturnDecoding::Unknown(ReturnUnknown::Word)
    );
    assert_eq!(
        read(&[InvocationWord::Expanded]),
        ReturnDecoding::Unknown(ReturnUnknown::Word)
    );
    assert_eq!(
        read(&[B(&[0xff]), L("ok")]),
        ReturnDecoding::Unknown(ReturnUnknown::Word)
    );
    assert_eq!(
        read(&[L("-options"), InvocationWord::Dynamic]),
        ReturnDecoding::Unknown(ReturnUnknown::Options)
    );
    let ReturnDecoding::Completes(custom) = read(&[
        InvocationWord::DynamicNonOption,
        InvocationWord::Dynamic,
        L("RESULT"),
    ]) else {
        panic!("non-reserved custom C pair");
    };
    assert_eq!(custom.result, Some(2));
    let ReturnDecoding::Completes(overridden) = read(&[
        L("-code"),
        InvocationWord::Dynamic,
        L("-code"),
        L("ok"),
        L("-errorcode"),
        L("A B"),
        L("RESULT"),
    ]) else {
        panic!("known final values");
    };
    assert_eq!(overridden.code, CompletionCode::Ok);
    assert_eq!(overridden.error_code, Some(5));
    assert_eq!(overridden.merged_error_code, None);
    assert_eq!(overridden.result, Some(6));
    let ReturnDecoding::Completes(merged) = read(&[
        L("-options"),
        L("-code error -level 0 -errorcode {MERGED CODE}"),
        L("RESULT"),
    ]) else {
        panic!("exact dictionary");
    };
    assert_eq!(merged.error_code, None);
    assert_eq!(merged.merged_error_code.as_deref(), Some("MERGED CODE"));
    assert_eq!(merged.result, Some(2));
}

#[test]
fn return_value_transfer_keeps_actual_numeric_context_and_merged_error_code() {
    // naming.completion.return-option-pair-grammar
    // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
    // Software overlay/route contract. A retained mixed numeral grammar is
    // not a newly measured C90 engine or a Native object/handler receipt.
    let profile = crate::native_test_provider::profile("tcl9.0");
    let mut context = super::super::context::AnalysisContext::detached(Some(profile));
    context.grammar.numbers = NumberSyntax::Tcl85;
    let inputs = super::super::literal::LiteralInputs::new(
        "return",
        None,
        &["-code", "010", "RESULT"],
        Some(profile),
    )
    .with_context(context);
    let EvalAnswer::Evaluated(outcome) = RETURN.evaluate(&inputs, &mut Budget::evaluation()) else {
        panic!("exact source value route");
    };
    assert_eq!(outcome.completion.options_code_and_level(), (8, 1));
    let inputs = super::super::literal::LiteralInputs::new(
        "return",
        None,
        &[
            "-options",
            "-code error -level 0 -errorcode {MERGED CODE}",
            "RESULT",
        ],
        Some(profile),
    );
    let EvalAnswer::Evaluated(outcome) = RETURN.evaluate(&inputs, &mut Budget::evaluation()) else {
        panic!("exact dictionary route");
    };
    let CompletionOutcome::Error { error_code, .. } = outcome.completion else {
        panic!("selected level-zero error");
    };
    assert_eq!(
        error_code,
        ExactValueOrUnavailable::exact_text("MERGED CODE")
    );
}

#[test]
fn return_decoder_release_and_invocation_facet_are_independent_premises() {
    // naming.completion.return-option-pair-grammar
    // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
    // Software contract: a missing release/entry facet cannot be replaced by
    // numeral grammar or a source word. Public disagreement is in380/383.
    let values = [L("-errorstack"), L("INNER"), L("RESULT")];
    assert_eq!(
        decode_return_words_in(
            Grammar::OptionsTcl,
            Numbers::Target(NumberSyntax::Tcl85),
            InvocationArguments::structured(&values),
            ReturnInvocationFacet::OriginalSource
        ),
        ReturnDecoding::Unknown(ReturnUnknown::Release)
    );
    let nested = [
        L("-options"),
        L("-options {-code error} -code ok"),
        L("RESULT"),
    ];
    let dialect = crate::InvocationDialect::for_version(TclVersion::V8_5);
    let args = InvocationArguments::structured(&nested).with_dialect(dialect);
    assert_eq!(
        decode_return_words_in(
            Grammar::OptionsTcl,
            Numbers::Target(NumberSyntax::Tcl85),
            args,
            ReturnInvocationFacet::OriginalSource
        ),
        ReturnDecoding::Unknown(ReturnUnknown::Release)
    );
    let ReturnDecoding::Completes(worker) = decode_return_words_in(
        Grammar::OptionsTcl,
        Numbers::Target(NumberSyntax::Tcl85),
        args,
        ReturnInvocationFacet::EvaluatedArguments,
    ) else {
        panic!("explicit worker reading");
    };
    assert_eq!(worker.code, CompletionCode::Error);
    assert_eq!(worker.result, Some(2));
}

#[test]
fn return_conversion_width_keeps_release_independent_of_numeral_overlay() {
    // naming.completion.return-option-pair-grammar
    // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
    // Source379/380 measures the C8/C9 conversion-width distinction.
    // These mixed carriers separately test the software axis contract;
    // they do not claim a newly observed mixed Native engine or entry.
    let numbers = Numbers::Target(NumberSyntax::Tcl85);
    let c90 = crate::InvocationDialect::for_version(TclVersion::V9_0);
    let c86 = crate::InvocationDialect::for_version(TclVersion::V8_6);
    for option in ["-code", "-level"] {
        let values = [L(option), L("-4294967295"), L("RESULT")];
        let arguments = InvocationArguments::structured(&values);
        assert_eq!(
            decode_return_words_in(
                Grammar::OptionsTcl,
                numbers,
                arguments.with_dialect(c90),
                ReturnInvocationFacet::OriginalSource,
            ),
            ReturnDecoding::Rejects,
        );
        assert!(matches!(
            decode_return_words_in(
                Grammar::OptionsTcl,
                numbers,
                arguments.with_dialect(c86),
                ReturnInvocationFacet::OriginalSource,
            ),
            ReturnDecoding::Completes(_),
        ));
        assert_eq!(
            decode_return_words_in(
                Grammar::OptionsTcl,
                numbers,
                arguments,
                ReturnInvocationFacet::OriginalSource,
            ),
            ReturnDecoding::Unknown(ReturnUnknown::Release),
        );
    }
    let values = [L("-code"), L("010"), L("RESULT")];
    let ReturnDecoding::Completes(parsed) = decode_return_words_in(
        Grammar::OptionsTcl,
        numbers,
        InvocationArguments::structured(&values).with_dialect(c90),
        ReturnInvocationFacet::OriginalSource,
    ) else {
        panic!("the retained numeral overlay still reads octal");
    };
    assert_eq!(parsed.code, CompletionCode::Other(8));
}

#[test]
fn registry_return_routes_keep_release_grammar_and_numerals_independent() {
    use crate::completion_route::{InvocationCompletionRoute as Route, ReturnCompletionRoute};

    // naming.completion.return-option-pair-grammar
    // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
    // Actual Registry consumer contracts under explicit mixed carriers.
    // These source advice queries prove no Native entry or object state.
    let values = [L("-options"), L("-code ok -level 0"), L("RESULT")];
    for (name, version, numbers, expected) in [
        (
            "tcl9.0",
            TclVersion::V9_0,
            NumberSyntax::Tcl84,
            Route::Tcl(CompletionCode::Ok),
        ),
        (
            "tcl8.4",
            TclVersion::V8_4,
            NumberSyntax::Tcl90,
            Route::Tcl(CompletionCode::Error),
        ),
    ] {
        let registry = crate::model::ingress::static_context_for(name).commands();
        let mut dialect = crate::InvocationDialect::for_version(version);
        dialect.numbers = numbers;
        let arguments = InvocationArguments::structured(&values).with_dialect(dialect);
        assert_eq!(
            registry.invocation_completion_route("return", arguments, None),
            Some(expected),
        );
        assert_eq!(
            matches!(
                registry.return_completion(arguments),
                ReturnDecoding::Completes(_)
            ),
            version != TclVersion::V8_4,
        );
    }
    let values = [L("-code"), L("-4294967295"), L("RESULT")];
    for (name, version, numbers, expected) in [
        (
            "tcl9.0",
            TclVersion::V9_0,
            NumberSyntax::Tcl85,
            Route::Tcl(CompletionCode::Error),
        ),
        (
            "tcl8.6",
            TclVersion::V8_6,
            NumberSyntax::Tcl90,
            Route::Return(ReturnCompletionRoute {
                eventual_code: CompletionCode::Error,
                remaining_level: 1,
            }),
        ),
    ] {
        let registry = crate::model::ingress::static_context_for(name).commands();
        let mut dialect = crate::InvocationDialect::for_version(version);
        dialect.numbers = numbers;
        let arguments = InvocationArguments::structured(&values).with_dialect(dialect);
        assert_eq!(
            registry.invocation_completion_route("return", arguments, None),
            Some(expected),
        );
    }
}

#[test]
fn return_decoder_keeps_unknown_grammar_and_numbers_as_separate_premises() {
    // naming.completion.return-option-pair-grammar
    // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
    // Software consensus contract: provider grammar and numeral knowledge
    // are independent; agreement supplies no worker or compiler entry.
    let values = [L("RESULT")];
    let ReturnDecoding::Completes(common) = decode_return_words_in(
        Grammar::Unknown,
        Numbers::Unknown,
        InvocationArguments::structured(&values),
        ReturnInvocationFacet::OriginalSource,
    ) else {
        panic!("a result-only return needs neither numeric nor option selection");
    };
    assert_eq!(common.code, CompletionCode::Ok);
    assert_eq!(common.level, 1);
    assert_eq!(common.result, Some(0));
    let values = [L("-code"), L("return"), L("RESULT")];
    assert_eq!(
        decode_return_words_in(
            Grammar::Unknown,
            Numbers::Target(NumberSyntax::Tcl85),
            InvocationArguments::structured(&values),
            ReturnInvocationFacet::OriginalSource,
        ),
        ReturnDecoding::Unknown(ReturnUnknown::Release),
    );
    let values = [L("-code"), L("010"), L("RESULT")];
    assert_eq!(
        decode_return_words_in(
            Grammar::OptionsTcl,
            Numbers::Unknown,
            InvocationArguments::structured(&values)
                .with_dialect(crate::InvocationDialect::for_version(TclVersion::V8_6)),
            ReturnInvocationFacet::OriginalSource,
        ),
        ReturnDecoding::Unknown(ReturnUnknown::Release),
    );
    let ReturnDecoding::Completes(common) = decode_return_words_in(
        Grammar::Unknown,
        Numbers::Target(NumberSyntax::Tcl85),
        InvocationArguments::structured(&values),
        ReturnInvocationFacet::OriginalSource,
    ) else {
        panic!("the explicit numeral overlay agrees at this narrow code");
    };
    assert_eq!(common.code, CompletionCode::Other(8));
}
