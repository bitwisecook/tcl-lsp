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

//! Invocation completion: `return` options, process exit, completion routes and source words.

use super::*;

#[test]
fn invocation_completion_is_registry_owned() {
    let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
    assert_eq!(
        reg.invocation_completion("return", &["-code", "error", "$w"], None,),
        InvocationCompletion::Terminates
    );
    assert_eq!(
        reg.invocation_completion("return", &["$w"], None),
        InvocationCompletion::ReturnsResult(Some(0))
    );
    assert_eq!(
        reg.invocation_completion("return", &["-code"], None),
        InvocationCompletion::ReturnsResult(Some(0)),
        "a trailing option-shaped word is return's result, not a missing option value"
    );
    assert_eq!(
        reg.invocation_completion("return", &["--", "x"], None),
        InvocationCompletion::ReturnsResult(None),
        "`--` is no end of options: `return -- x` is the pair `-- x` and the empty result"
    );
    assert_eq!(
        reg.invocation_completion("return", &["-level", "0", "$w"], None,),
        InvocationCompletion::FallsThrough
    );
    for args in [
        &["-level", "0", "-code", "error", "$w"][..],
        &["-code", "error", "-level", "0", "$w"][..],
        &["-level", "2", "$w"][..],
        &["-level", "-1", "$w"][..],
    ] {
        assert_eq!(
            reg.invocation_completion("return", args, None),
            InvocationCompletion::Terminates,
            "{args:?}"
        );
    }
    assert_eq!(
        reg.invocation_completion("return", &["-level", "$dynamic", "$w"], None),
        InvocationCompletion::Unknown,
    );
    {
        let invalid = &["-level", "-1", "value"][..];
        assert_eq!(
            reg.invocation_completion("return", invalid, None),
            InvocationCompletion::Terminates,
            "native validation error ends the current path"
        );
    }
    // Native worker383 ordinary_pair: C85–C91 catch=2/result empty;
    // C84 catch=1. Custom option pairs are valid in this selected C9 context.
    assert_eq!(
        reg.invocation_completion("return", &["value", "extra"], None),
        InvocationCompletion::ReturnsResult(None)
    );
    // A registry that names no release answers only what every release
    // reads alike: 8.4 rejects `-level`, which 8.5 reads.
    assert_eq!(
        CommandRegistry::build_default().invocation_completion(
            "return",
            &["-level", "0", "$w"],
            None
        ),
        InvocationCompletion::Unknown
    );
    assert_eq!(
        reg.invocation_completion("not-a-command", &[], None),
        InvocationCompletion::Unknown
    );
    assert_eq!(
        reg.invocation_completion("set", &[], None),
        InvocationCompletion::Terminates
    );
}

#[test]
fn source_completion_preserves_dynamic_options_and_pending_return_depth() {
    use crate::InvocationWord::{Dynamic, Literal};
    let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
    let result = [Literal("-level"), Literal("1"), Dynamic];
    assert_eq!(
        registry.invocation_completion_words(
            "return",
            InvocationArguments::structured(&result),
            None
        ),
        InvocationCompletion::ReturnsResult(Some(2))
    );
    let deeper = [Literal("-level"), Literal("2"), Dynamic];
    assert_eq!(
        registry.invocation_completion_words(
            "return",
            InvocationArguments::structured(&deeper),
            None
        ),
        InvocationCompletion::Terminates,
        "pending return does not become a normal result at this procedure boundary"
    );
    let option = [Literal("-code"), Dynamic, Dynamic];
    assert_eq!(
        registry.invocation_completion_words(
            "return",
            InvocationArguments::structured(&option),
            None
        ),
        InvocationCompletion::Unknown
    );
    assert_eq!(
        CommandRegistry::build_default().invocation_completion(
            "return",
            &["-level", "0", "value"],
            None
        ),
        InvocationCompletion::Unknown,
        "an unselected native grammar cannot license modern return options"
    );
}

#[test]
fn exact_return_options_and_process_exit_are_registry_owned() {
    use crate::completion::CompletionCode;

    let reg = crate::model::ingress::static_context_for("tcl8.6").commands();
    // Oracle (tclsh 8.6/9.0): the default -level 1 return is caught by
    // `try on return`, even when its eventual procedure result is error.
    assert_eq!(
        reg.exact_invocation_completion("return", &["-code", "error", "payload"], None,),
        Some(ExactInvocationCompletion::Tcl(CompletionCode::Return))
    );
    assert_eq!(
        reg.exact_invocation_completion(
            "return",
            &["-level", "0", "-code", "error", "payload"],
            None,
        ),
        Some(ExactInvocationCompletion::Tcl(CompletionCode::Error))
    );
    for args in [
        &["-foo", "bar", "payload"][..],
        &["-c", "error", "payload"][..],
        &["-level", "0x0", "-code", "error", "payload"][..],
    ] {
        assert_eq!(
            reg.exact_invocation_completion("return", args, None),
            Some(ExactInvocationCompletion::Tcl(if args[0] == "-level" {
                CompletionCode::Error
            } else {
                CompletionCode::Return
            }))
        );
    }
    for (spelling, expected) in [("2147483648", i32::MIN), ("4294967295", -1)] {
        assert_eq!(
            reg.exact_invocation_completion(
                "return",
                &["-level", "0", "-code", spelling, "payload"],
                None,
            ),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Other(
                expected
            )))
        );
    }
    for (spelling, expected) in [
        ("00", CompletionCode::Ok),
        ("+1", CompletionCode::Error),
        ("02", CompletionCode::Return),
        ("0x3", CompletionCode::Break),
        ("04", CompletionCode::Continue),
    ] {
        assert_eq!(
            reg.exact_invocation_completion(
                "return",
                &["-level", "0", "-code", spelling, "payload"],
                None,
            ),
            Some(ExactInvocationCompletion::Tcl(expected)),
            "{spelling}"
        );
    }
    assert_eq!(
        reg.exact_invocation_completion("exit", &["0"], None),
        Some(ExactInvocationCompletion::ProcessExit)
    );
    assert_eq!(
        reg.exact_invocation_completion("return", &["-options", "$dynamic", "payload"], None,),
        None
    );
    assert_eq!(
        reg.exact_invocation_completion("return", &["-code", "$dynamic", "payload"], None,),
        None
    );
    assert_eq!(
        reg.exact_invocation_completion("return", &["--", "-code", "error"], None,),
        Some(ExactInvocationCompletion::Tcl(CompletionCode::Return))
    );
}

#[test]
fn exit_status_completion_follows_the_release_specific_tcl_oracle() {
    use crate::completion::CompletionCode;
    use crate::invocation_words::{InvocationArguments, InvocationWord};

    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        // Tcl 8.x's `exit` calls `Tcl_GetIntFromObj`. Despite the `int`
        // destination, its 64-bit implementation accepts the asymmetric
        // `-UINT_MAX..=UINT_MAX` range before casting to `int`.
        for args in [
            &[][..],
            &["0"][..],
            &["-7"][..],
            &["0x10"][..],
            &["-4294967295"][..],
            &["4294967295"][..],
        ] {
            assert_eq!(
                reg.exact_invocation_completion("exit", args, None),
                Some(ExactInvocationCompletion::ProcessExit),
                "{dialect}: exit {args:?}"
            );
        }
        // One value beyond either Tcl 8.x conversion endpoint never gets
        // as far as process termination: it is a catchable Tcl error.
        for args in [
            &["nope"][..],
            &["1.5"][..],
            &["-4294967296"][..],
            &["4294967296"][..],
        ] {
            assert_eq!(
                reg.exact_invocation_completion("exit", args, None),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                "{dialect}: exit {args:?}"
            );
        }
    }

    for dialect in ["tcl9.0", "tcl9.1"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        // Tcl 9.0+ changed `exit` to `TclGetWideBitsFromObj`: every
        // integer, including a bignum beyond wide range, reaches
        // `Tcl_Exit` and has its low bits cast to `int` there.
        for args in [
            &[][..],
            &["0"][..],
            &["-4294967296"][..],
            &["4294967296"][..],
            &["18446744073709551616"][..],
        ] {
            assert_eq!(
                reg.exact_invocation_completion("exit", args, None),
                Some(ExactInvocationCompletion::ProcessExit),
                "{dialect}: exit {args:?}"
            );
        }
        for args in [&["nope"][..], &["1.5"][..], &["0", "extra"][..]] {
            assert_eq!(
                reg.exact_invocation_completion("exit", args, None),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                "{dialect}: exit {args:?}"
            );
        }
    }

    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        let dynamic = [InvocationWord::Dynamic];
        assert_eq!(
            reg.invocation_completion_knowledge(
                "exit",
                InvocationArguments::structured(&dynamic),
                None,
            ),
            Some(InvocationCompletionKnowledge::ExitOrError),
            "{dialect}: exit $status is only process-exit or Tcl error"
        );
        let expanded = [InvocationWord::Expanded];
        assert_eq!(
            reg.invocation_completion_knowledge(
                "exit",
                InvocationArguments::structured(&expanded),
                None,
            ),
            Some(InvocationCompletionKnowledge::ExitOrError),
            "{dialect}: exit {{*}}$statuses has no normal completion either"
        );
    }
}

#[test]
fn exact_return_option_shapes_follow_the_tcl86_and_tcl90_oracle() {
    use crate::completion::CompletionCode;

    // Tcl parses an option only when a following argv word can supply its
    // value. A lone option-shaped word is therefore return's sole result;
    // with `-level 0`, it exposes the default TCL_OK instead.
    for dialect in ["tcl8.6", "tcl9.0"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        for (args, expected) in [
            (&["-code"][..], CompletionCode::Return),
            (&["-level"][..], CompletionCode::Return),
            (&["-options"][..], CompletionCode::Return),
            (&["-foo"][..], CompletionCode::Return),
            (&["-level", "0", "-code"][..], CompletionCode::Ok),
        ] {
            assert_eq!(
                reg.exact_invocation_completion("return", args, None),
                Some(ExactInvocationCompletion::Tcl(expected)),
                "{dialect}: {args:?}"
            );
        }
        // Once a following word exists, the same spellings are options;
        // malformed option values remain TCL_ERROR.
        for args in [
            &["-code", "bogus", "payload"][..],
            &["-level", "-1", "payload"][..],
            &["-code", "-level"][..],
            &["-level", "-code"][..],
        ] {
            assert_eq!(
                reg.exact_invocation_completion("return", args, None),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                "{dialect}: {args:?}"
            );
        }
        // Native worker383 confirms ordinary_pair catch=2/result empty
        // for these exact C86/C90 workers. Two words are an option and its value, whatever the first
        // spells, which the options dictionary keeps: `proc p {} {return
        // payload extra}` returns the empty string (tclsh 8.6, 9.0).
        assert_eq!(
            reg.exact_invocation_completion("return", &["payload", "extra"], None),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Return)),
            "{dialect}"
        );
        // Past `INT_MIN` the conversion's range is the release's own:
        // 8.6 wraps `-2147483649` to 2147483647 and 9.0 rejects it.
        assert_eq!(
            reg.exact_invocation_completion(
                "return",
                &["-level", "0", "-code", "-2147483649", "payload"],
                None,
            ),
            None,
            "{dialect}"
        );
    }
}

#[test]
fn exact_invalid_arity_of_known_terminators_is_tcl_error() {
    use crate::completion::CompletionCode;

    // The descriptor of a well-formed call must never hide Tcl's prior
    // arity validation.  This table covers every fixed-arity terminating
    // core primitive, including ones whose normal effect is process exit
    // or a loop transfer rather than TCL_ERROR.
    for dialect in ["tcl8.6", "tcl9.0"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        for (name, args) in [
            ("error", &[][..]),
            ("error", &["message", "info", "code", "extra"][..]),
            ("throw", &[][..]),
            ("throw", &["type"][..]),
            ("throw", &["type", "message", "extra"][..]),
            ("break", &["extra"][..]),
            ("continue", &["extra"][..]),
            ("exit", &["0", "extra"][..]),
        ] {
            assert_eq!(
                reg.exact_invocation_completion(name, args, None),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                "{dialect}: {name} {args:?}"
            );
        }
    }
}

#[test]
fn exact_return_source_words_distinguish_literals_from_substitution() {
    use crate::completion::CompletionCode;
    use crate::invocation_words::{InvocationArguments, InvocationWord};

    // Tcl 8.6 and 9.0 oracle: braces keep `$code` and `\\x32` literal,
    // while a bare/quoted substitution remains unknown and a decoded bare
    // escape can be supplied to this API as its effective static value.
    for dialect in ["tcl8.6", "tcl9.0"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        let trailing_option = [InvocationWord::Literal("-code")];
        assert_eq!(
            reg.exact_invocation_completion_words(
                "return",
                InvocationArguments::structured(&trailing_option),
                None
            ),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Return)),
            "{dialect}: source-aware lone -code remains the result word"
        );
        for words in [
            vec![
                InvocationWord::Literal("-code"),
                InvocationWord::Literal("$code"),
            ],
            vec![
                InvocationWord::Literal("-code"),
                InvocationWord::Literal("\\x32"),
            ],
        ] {
            assert_eq!(
                reg.exact_invocation_completion_words(
                    "return",
                    InvocationArguments::structured(&words),
                    None
                ),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                "{dialect}: {words:?}"
            );
        }
        let decoded = [
            InvocationWord::Literal("-code"),
            InvocationWord::Literal("2"),
        ];
        assert_eq!(
            reg.exact_invocation_completion_words(
                "return",
                InvocationArguments::structured(&decoded),
                None
            ),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Return)),
        );
        for dynamic in [InvocationWord::Dynamic, InvocationWord::Dynamic] {
            let words = [InvocationWord::Literal("-code"), dynamic];
            assert_eq!(
                reg.exact_invocation_completion_words(
                    "return",
                    InvocationArguments::structured(&words),
                    None
                ),
                None,
            );
        }
        let dynamic_option = [InvocationWord::Dynamic, InvocationWord::Literal("error")];
        assert_eq!(
            reg.invocation_completion_knowledge(
                "return",
                InvocationArguments::structured(&dynamic_option),
                None,
            ),
            Some(InvocationCompletionKnowledge::Dynamic),
        );
    }
}

#[test]
fn exact_completion_counts_positionals_after_known_selector_and_terminator() {
    use crate::InvocationWord::{Dynamic, Literal};
    let registry = CommandRegistry::build_default();
    let extra_names = [Literal("create"), Literal("--"), Dynamic, Dynamic];
    assert_eq!(
        registry.exact_invocation_completion_words(
            "interp",
            InvocationArguments::structured(&extra_names),
            None,
        ),
        Some(ExactInvocationCompletion::Tcl(crate::CompletionCode::Error)),
        "dynamic positional bytes do not erase a known selector and excess count"
    );
    let unknown_option = [Literal("create"), Dynamic, Dynamic];
    assert_eq!(
        registry.exact_invocation_completion_words(
            "interp",
            InvocationArguments::structured(&unknown_option),
            None,
        ),
        None,
        "an unknown leading option cannot establish rejection by count"
    );
    let one_name = [Literal("create"), Literal("--"), Dynamic];
    assert_eq!(
        registry.exact_invocation_completion_words(
            "interp",
            InvocationArguments::structured(&one_name),
            None,
        ),
        None,
        "accepted arity does not prove successful creation"
    );
}

#[test]
fn exact_process_exit_obeys_explicit_snapshot_without_catalogue_profile() {
    let registry = CommandRegistry::build_default();
    for release in tcl_dialect::TclVersion::ALL {
        let arguments = crate::InvocationArguments::literals(&["4294967296"])
            .with_dialect(tcl_native_model::InvocationDialect::for_version(release));
        assert_eq!(
            registry.exact_invocation_completion_words("exit", arguments, None),
            Some(if release >= tcl_dialect::TclVersion::V9_0 {
                ExactInvocationCompletion::ProcessExit
            } else {
                ExactInvocationCompletion::Tcl(crate::completion::CompletionCode::Error)
            })
        );
    }
    let arguments = crate::InvocationArguments::literals(&["18446744073709551616"]).with_dialect(
        tcl_native_model::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        )),
    );
    assert_eq!(
        registry.exact_invocation_completion_words("exit", arguments, None),
        Some(ExactInvocationCompletion::Tcl(
            crate::completion::CompletionCode::Other(6)
        ))
    );
}
