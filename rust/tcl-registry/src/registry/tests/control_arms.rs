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

//! Typed control arms, `try` completion selectors and case-list invocation layouts.

use super::*;

#[test]
fn typed_control_arms_are_registry_owned() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.control_arm_semantics("try", &["{}", "finally", "{}"], 2),
        Some(ControlArmSemantics::Always)
    );
    assert_eq!(
        reg.control_arm_semantics("try", &["{}", "on", "error", "{m o}", "{}"], 4,),
        Some(ControlArmSemantics::Selected)
    );
    assert_eq!(
        reg.try_control_invocation("try", &["{}", "on", "error", "{m o}", "{}"], None,),
        Some(TryControlInvocation {
            body_index: 0,
            clauses: vec![TryControlClause {
                kind: TryClauseKind::On(TryCompletionSelector::Error),
                selector_index: Some(2),
                variable_list_index: Some(3),
                body_index: 4,
                fallthrough: false,
            }],
        })
    );
    assert_eq!(
        reg.control_arm_semantics("namespace", &["eval", "::ns", "{}"], 2),
        Some(ControlArmSemantics::FrameBoundary)
    );
    assert_eq!(
        reg.control_arm_semantics("try", &["{}", "finally", "{}", "orphan"], 0),
        None,
        "a malformed trailing clause invalidates even the main arm"
    );
    for args in [
        &["{}", "on", "bogus", "{}", "{}"][..],
        &["{}", "on", "ok", "a b c", "{}"][..],
        &["{}", "on", "ok", "{}", "-"][..],
    ] {
        assert_eq!(reg.control_arm_semantics("try", args, 0), None);
    }
    assert_eq!(
        reg.control_invocation_valid("try", &["{}", "on", "ok", "{}", "-"], None,),
        Some(false),
        "a terminal handler fallthrough has no following script"
    );
    assert_eq!(
        reg.control_arm_semantics("try", &["{}", "finally", "-"], 2),
        Some(ControlArmSemantics::Always),
        "a finally body is a script, so `-` has no fallthrough meaning there"
    );
    let chained_fallthrough = [
        "{}",
        "on",
        "error",
        "{}",
        "-",
        "on",
        "error",
        "{}",
        "{set r fell}",
    ];
    assert_eq!(
        reg.control_invocation_valid("try", &chained_fallthrough, None),
        Some(true)
    );
    assert_eq!(
        reg.control_arm_semantics("try", &chained_fallthrough, 8),
        Some(ControlArmSemantics::Selected)
    );
    assert_eq!(
        reg.control_invocation_valid("if", &["1", "then"], None),
        Some(false)
    );
    assert_eq!(
        reg.control_invocation_valid("if", &["1", "a", "b"], None),
        Some(true)
    );
    assert_eq!(
        reg.control_arm_semantics("if", &["1", "a", "b"], 2),
        Some(ControlArmSemantics::Selected)
    );
}

#[test]
fn try_completion_selectors_use_the_profiled_tcl_number_and_completion_owners() {
    // `try_control_invocation` receives resolved word values. In source,
    // `{ 1 }` becomes the `" 1 "` value below; a literal value of `{1}`
    // would instead come from nested braces and is not a completion code.
    let selector = |registry: &CommandRegistry, selector: &str| {
        let invocation = registry.try_control_invocation(
            "try",
            &["{body}", "on", selector, "{message}", "{handler}"],
            None,
        )?;
        let [clause] = invocation.clauses.as_slice() else {
            return None;
        };
        match clause.kind {
            TryClauseKind::On(selector) => Some(selector),
            TryClauseKind::Trap | TryClauseKind::Finally => None,
        }
    };

    let tcl9 = crate::model::ingress::static_context_for("tcl9.0").commands();
    for spelling in ["+1", "01", "0x1", " 1 "] {
        assert_eq!(
            selector(tcl9, spelling),
            Some(TryCompletionSelector::Error),
            "Tcl 9 completion selector {spelling:?}"
        );
    }
    assert_eq!(
        selector(tcl9, "{1}"),
        None,
        "the registry must not mistake nested literal braces for source-word delimiters"
    );

    // The number owner selects grammar from the registry's target
    // profile. A leading-zero spelling is octal through Tcl 8.6 and
    // decimal from Tcl 9.0.
    let tcl8 = crate::model::ingress::static_context_for("tcl8.6").commands();
    assert_eq!(
        selector(tcl8, "010"),
        Some(TryCompletionSelector::Numeric(8))
    );
    assert_eq!(
        selector(tcl9, "010"),
        Some(TryCompletionSelector::Numeric(10))
    );

    // Completion codes use Tcl's C-compatible signed-int domain: the
    // final unsigned range wraps, while values outside it are invalid.
    assert_eq!(
        selector(tcl9, "4294967295"),
        Some(TryCompletionSelector::Numeric(-1))
    );
    for out_of_range in ["-2147483649", "4294967296", "9223372036854775808"] {
        assert_eq!(
            selector(tcl9, out_of_range),
            None,
            "out-of-range completion selector {out_of_range:?}"
        );
    }
}

#[test]
fn explicit_control_surface_selects_completion_code_numeral_grammar() {
    let registry = CommandRegistry::build_default();
    let args = ["{body}", "on", "1_0", "{message}", "{handler}"];
    let tcl86 = SurfaceQuery::core(Family::Tcl, "8.6");
    let tcl90 = SurfaceQuery::core(Family::Tcl, "9.0");

    assert_eq!(
        registry.control_invocation_valid("try", &args, Some(tcl86)),
        Some(false),
        "Tcl 8.6 does not admit underscore-separated completion codes"
    );
    assert_eq!(
        registry.try_control_invocation("try", &args, Some(tcl86)),
        None
    );
    assert_eq!(
        registry.control_invocation_valid("try", &args, Some(tcl90)),
        Some(true),
        "Tcl 9 admits underscore-separated completion codes"
    );
    let invocation = registry
        .try_control_invocation("try", &args, Some(tcl90))
        .expect("Tcl 9 try invocation");
    assert!(matches!(
        invocation.clauses.as_slice(),
        [TryControlClause {
            kind: TryClauseKind::On(TryCompletionSelector::Numeric(10)),
            ..
        }]
    ));
}

#[test]
#[allow(clippy::too_many_lines)] // descriptor layout matrix
fn case_list_invocation_layout_is_registry_owned() {
    use crate::spec::CaseMatchMode;
    let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
    let Some((_, two_arg)) = reg.case_invocation(
        "switch",
        &["-glob", "default {}"],
        Some(SurfaceQuery::core(Family::Tcl, "9.0")),
    ) else {
        panic!("two-argument case form must parse");
    };
    assert_eq!(two_arg.subject_index, Some(0));
    assert_eq!(two_arg.mode, CaseMatchMode::Exact);
    assert_eq!(two_arg.clause_list_index, Some(1));

    let Some((_, options)) = reg.case_invocation(
        "switch",
        &["-glob", "-nocase", "--", "subject", "p {}"],
        Some(SurfaceQuery::core(Family::Tcl, "9.0")),
    ) else {
        panic!("option-bearing case form must parse");
    };
    assert_eq!(options.subject_index, Some(3));
    assert_eq!(options.mode, CaseMatchMode::Glob);
    assert!(options.nocase);

    let tcl91 = crate::model::ingress::static_context_for("tcl9.1").commands();
    let Some((_, integer)) = tcl91.case_invocation(
        "switch",
        &["-integer", "--", "1", "1 {set x 1}"],
        Some(SurfaceQuery::core(Family::Tcl, "9.1")),
    ) else {
        panic!("Tcl 9.1 integer switch must retain its case-list body");
    };
    assert_eq!(integer.mode, CaseMatchMode::Other);
    assert_eq!(integer.clause_list_index, Some(3));
    for args in [
        ["-integer", "-nocase", "1", "1 {set x 1}"],
        ["-nocase", "-integer", "1", "1 {set x 1}"],
    ] {
        assert!(
            tcl91
                .case_invocation(
                    "switch",
                    &args,
                    Some(SurfaceQuery::core(Family::Tcl, "9.1"))
                )
                .is_none(),
            "integer and nocase must be incompatible: {args:?}"
        );
        assert!(
            tcl91
                .arg_indices_for_role("switch", &args, ArgRole::Body)
                .is_empty(),
            "invalid integer/nocase form must expose no body: {args:?}"
        );
    }
    for args in [
        ["-integer", "-glob", "1", "1 {set x 1}"],
        ["-glob", "-integer", "1", "1 {set x 1}"],
        ["-integer", "-integer", "1", "1 {set x 1}"],
    ] {
        assert!(
            tcl91
                .case_invocation(
                    "switch",
                    &args,
                    Some(SurfaceQuery::core(Family::Tcl, "9.1"))
                )
                .is_none(),
            "multiple match modes must be rejected: {args:?}"
        );
        assert!(
            tcl91
                .arg_indices_for_role("switch", &args, ArgRole::Body)
                .is_empty(),
            "multiple match modes must expose no body: {args:?}"
        );
    }
    assert_eq!(
        tcl91.arg_indices_for_role(
            "switch",
            &["-integer", "--", "1", "1 {set x 1}"],
            ArgRole::Body,
        ),
        vec![3]
    );
    assert!(
        reg.case_invocation(
            "switch",
            &["subject", "pattern", "body", "orphan"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .is_none()
    );
    assert!(
        reg.case_invocation(
            "switch",
            &["subject", "pattern {} orphan"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0"))
        )
        .is_none()
    );

    let tcl84 = crate::model::ingress::static_context_for("tcl8.4").commands();
    for two_arg_switch in [
        ["-regexp", "default {puts hit}"],
        ["--", "default {puts hit}"],
    ] {
        assert!(
            tcl84
                .case_invocation(
                    "switch",
                    &two_arg_switch,
                    Some(SurfaceQuery::core(Family::Tcl, "8.4"))
                )
                .is_none(),
            "Tcl 8.4 scans the option-like first word and rejects the missing subject: {two_arg_switch:?}"
        );
        assert!(
            tcl84
                .arg_indices_for_role("switch", &two_arg_switch, ArgRole::Body)
                .is_empty(),
            "Tcl 8.4 must not assign the second word a body role: {two_arg_switch:?}"
        );
        assert_eq!(
            tcl84
                .resolve_option_terminator(
                    "switch",
                    &two_arg_switch,
                    Some(SurfaceQuery::core(Family::Tcl, "8.4"))
                )
                .expect("switch declares --")
                .reserved_trailing_words,
            0,
            "Tcl 8.4 must scan both words in the option-like two-word form: {two_arg_switch:?}"
        );
    }
    for args in [
        &["subject", "default {puts hit}"][..],
        &["-regexp", "subject", "default {puts hit}"][..],
        &["--", "-subject", "default {puts hit}"][..],
    ] {
        assert!(
            tcl84
                .case_invocation("switch", args, Some(SurfaceQuery::core(Family::Tcl, "8.4")))
                .is_some(),
            "Tcl 8.4 accepts the unambiguous switch case-list form: {args:?}"
        );
        assert_eq!(
            tcl84.arg_indices_for_role("switch", args, ArgRole::Body),
            vec![args.len() - 1],
            "Tcl 8.4 assigns the final case-list word a body role: {args:?}"
        );
    }
    assert!(
        tcl84
            .case_invocation(
                "switch",
                &["-nocase", "subject", "pattern {}"],
                Some(SurfaceQuery::core(Family::Tcl, "8.4")),
            )
            .is_none()
    );
    let tcl85 = crate::model::ingress::static_context_for("tcl8.5").commands();
    for (dialect, availability) in [
        ("tcl8.5", Some(SurfaceQuery::core(Family::Tcl, "8.5"))),
        ("tcl8.6", Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
        ("tcl9.0", Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
    ] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        for two_arg_switch in [
            ["-regexp", "default {puts hit}"],
            ["--", "default {puts hit}"],
        ] {
            assert!(
                registry
                    .case_invocation("switch", &two_arg_switch, availability)
                    .is_some(),
                "{dialect} accepts the optionless two-word switch form: {two_arg_switch:?}"
            );
            assert_eq!(
                registry.arg_indices_for_role("switch", &two_arg_switch, ArgRole::Body),
                vec![1],
                "{dialect} assigns the clause-list word a body role: {two_arg_switch:?}"
            );
            assert_eq!(
                registry
                    .resolve_option_terminator("switch", &two_arg_switch, availability)
                    .expect("switch declares --")
                    .reserved_trailing_words,
                2,
                "{dialect} reserves the subject and clause-list words: {two_arg_switch:?}"
            );
        }
    }
    assert!(
        tcl85
            .case_invocation(
                "switch",
                &["-nocase", "subject", "pattern {}"],
                Some(SurfaceQuery::core(Family::Tcl, "8.5")),
            )
            .is_some()
    );
    let default = CommandRegistry::build_default();
    assert!(
        default
            .case_invocation(
                "switch",
                &["subject", "pattern {}"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .is_some()
    );
    assert!(
        default
            .case_invocation(
                "switch",
                &["--", "-x", "-x {puts hit} default {puts miss}"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .is_some(),
        "the descriptor's -- terminator keeps a hyphenated switch subject positional"
    );
    assert!(
        reg.case_invocation(
            "switch",
            &["subject", "pattern -"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0"))
        )
        .is_none()
    );

    let expect = crate::model::ingress::static_context_for("expect").commands();
    assert!(
        expect
            .case_invocation(
                "expect",
                &["\"password:\" {send pw} -re {ye+s} {send yes} timeout {puts slow}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                ),
            )
            .is_some(),
        "clause-leading flags must not break valid Expect pattern/body pairs"
    );
    assert!(
        expect
            .case_invocation(
                "expect",
                &["{-re} {send literal}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                ),
            )
            .is_some(),
        "a braced flag-shaped pattern is literal text, not a clause flag"
    );
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-re {ye+s}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                )
            )
            .is_some(),
        "Expect permits a final pattern without an action"
    );
    for args in [
        ["-timeout", "5", "ready {action}"],
        ["-i", "spawn", "ready {action}"],
    ] {
        let (_, invocation) = expect
            .case_invocation(
                "expect",
                &args,
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")]),
                ),
            )
            .expect("outer Expect value option followed by a final pattern");
        assert_eq!(invocation.clause_list_index, None, "{args:?}");
        assert_eq!(invocation.inline_clause_start, Some(2), "{args:?}");
        let clauses = crate::CaseListSpec::EXPECT
            .inline_clauses(&args, invocation.inline_clause_start.unwrap())
            .expect("one action-less inline clause");
        assert_eq!(clauses.len(), 1, "{args:?}");
        assert_eq!(clauses[0].pattern_index, 2, "{args:?}");
        assert_eq!(clauses[0].body_index, None, "{args:?}");
    }
    for pattern in ["#", ";"] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &[&format!(
                        "{pattern} {{send literal}} default {{send other}}"
                    )],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    ),
                )
                .is_some(),
            "{pattern:?} is a literal Tcl list pattern, not script syntax"
        );
    }
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-timeout"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                )
            )
            .is_none(),
        "a value-taking clause flag without a value/pattern/body is invalid"
    );
}

#[test]
fn inline_expect_case_flags_and_actions_are_registry_owned() {
    let expect = crate::model::ingress::static_context_for("expect").commands();
    let args = [
        "-re",
        "{ye+s}",
        "{send yes}",
        "-timeout",
        "5",
        "timeout",
        "{send slow}",
    ];
    let (_, inline) = expect
        .case_invocation(
            "expect",
            &args,
            Some(
                SurfaceQuery::core(Family::Tcl, "8.6")
                    .with_packages(&[PackageFloor::named("expect")]),
            ),
        )
        .expect("inline Expect flags and value flags must parse");
    let clauses = crate::CaseListSpec::EXPECT
        .inline_clauses(&args, inline.inline_clause_start.expect("inline form"))
        .expect("inline clauses");
    assert_eq!(clauses.len(), 2);
    assert_eq!(clauses[0].pattern_index, 1);
    assert_eq!(clauses[0].body_index, Some(2));
    assert_eq!(clauses[0].mode, crate::spec::CaseMatchMode::Regexp);
    assert_eq!(clauses[1].pattern_index, 5);
    assert_eq!(clauses[1].body_index, Some(6));
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-not", "ready", "{send ok}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                )
            )
            .is_some(),
        "unique Expect flag abbreviations must retain the action body"
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Expect oracle layout matrix
fn expect_inline_flag_table_matches_the_oracle() {
    let expect = crate::model::ingress::static_context_for("expect").commands();
    for flag in [
        "-glob",
        "-regexp",
        "-exact",
        "-notransfer",
        "-nocase",
        "-i",
        "-indices",
        "-iread",
        "-timestamp",
        "-nobrace",
    ] {
        let args = if matches!(flag, "-i") {
            vec![flag, "spawn", "pattern", "{action}"]
        } else {
            vec![flag, "pattern", "{action}"]
        };
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &args,
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_some(),
            "canonical {flag} must parse"
        );
    }
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-timeout", "5", "pattern", "{action}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                )
            )
            .is_some()
    );
    for flag in ["-gl", "-re", "-ex", "-not"] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &[flag, "pattern", "{action}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_some(),
            "unique abbreviation {flag} must parse"
        );
    }
    for flag in ["-g", "-r", "-e", "-noc", "-nob"] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &[flag, "pattern", "{action}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_some(),
            "unique canonical-prefix {flag} must remain an inline clause flag"
        );
    }
    for flag in ["-n", "-bogus"] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &[flag, "pattern", "{action}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_none(),
            "ambiguous or unknown {flag} must invalidate the invocation"
        );
    }
    let args = ["--", "-re", "{action}"];
    let (_, invocation) = expect
        .case_invocation(
            "expect",
            &args,
            Some(
                SurfaceQuery::core(Family::Tcl, "8.6")
                    .with_packages(&[PackageFloor::named("expect")]),
            ),
        )
        .expect("-- makes -re a pattern");
    let clauses = crate::CaseListSpec::EXPECT
        .inline_clauses(&args, invocation.inline_clause_start.expect("inline"))
        .expect("clause");
    assert_eq!(clauses[0].pattern_index, 1);
    assert_eq!(clauses[0].body_index, Some(2));
    let (_, omitted) = expect
        .case_invocation(
            "expect",
            &["-re", "pattern"],
            Some(
                SurfaceQuery::core(Family::Tcl, "8.6")
                    .with_packages(&[PackageFloor::named("expect")]),
            ),
        )
        .expect("omitted final action is valid");
    assert_eq!(
        crate::CaseListSpec::EXPECT
            .inline_clauses(&["-re", "pattern"], omitted.inline_clause_start.unwrap())
            .unwrap()[0]
            .body_index,
        None
    );
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-nobrace", "{pattern}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                )
            )
            .is_some(),
        "-nobrace makes one braced word an action-less pattern"
    );
    let (_, brace) = expect
        .case_invocation(
            "expect",
            &["-brace", "{default {return FOLDED}}"],
            Some(
                SurfaceQuery::core(Family::Tcl, "8.6")
                    .with_packages(&[PackageFloor::named("expect")]),
            ),
        )
        .expect("exact -brace selects a clause list");
    assert_eq!(brace.clause_list_index, Some(1));
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-b", "{default {return FOLDED}}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                ),
            )
            .is_none(),
        "-brace is exact-only, so -b is not a clause flag abbreviation"
    );
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-brac", "{default {return FOLDED}}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                ),
            )
            .is_none(),
        "-brace must not accept a near-complete prefix either"
    );
    for args in [
        &["-timeout", "5", "-brace", "{default {return FOLDED}}"] as &[&str],
        &["-i", "spawn", "-brace", "{default {return FOLDED}}"],
        &["-brace", "{default {return FOLDED}}", "extra"],
    ] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    args,
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_none(),
            "force-list selector must be first and have one remainder: {args:?}"
        );
    }
}

#[test]
fn subjectless_case_list_defers_clause_flags_after_outer_value_options() {
    use crate::hover::{OptionSpec, OptionValue};
    use crate::spec::CaseListSpec;

    // This deliberately does not name `expect`: a future descriptor with
    // one value-taking outer option and one overlapping clause flag must
    // use the same precedence rule. The body is the observable proof that
    // `-clause` was not swallowed by the outer scan.
    let case = CaseListSpec {
        subject_args: 0,
        two_arg_optionless_surface: None,
        fallthrough_body: None,
        value_options_require_regex: &[],
        special_match_options: &[],
        clause_flags: &["-clause", "--"],
        clause_regex_flag: None,
        clause_value_flags: &[],
        clause_end_options_flag: Some("--"),
        clause_force_inline_flag: None,
        clause_force_list_flag: None,
        clause_force_list_shape: None,
        allow_omitted_final_body: true,
        keyword_patterns: &[],
        keyword_patterns_require_final: false,
        exhaustive_keyword_patterns: &[],
        optional_subject_separator: None,
        warn_unbraced_bodies: false,
        default_mode: crate::spec::CaseMatchMode::Exact,
        pattern_words: crate::spec::PatternWords::Single,
    };
    let options = [
        OptionSpec {
            name: "-outer",
            value: OptionValue::value("value"),
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-clause",
            ..OptionSpec::DEFAULT
        },
    ];
    let option_refs: Vec<&OptionSpec> = options.iter().collect();
    let args = ["-outer", "value", "-clause", "pattern", "{body}"];
    let invocation = case
        .invocation(
            &args,
            &option_refs,
            Some(SurfaceQuery::any_release(Family::Tcl)),
        )
        .expect("the outer value must precede, not consume, the clause flag");
    assert_eq!(invocation.inline_clause_start, Some(2));
    let clauses = case
        .inline_clauses(&args, invocation.inline_clause_start.unwrap())
        .expect("the deferred flag must retain the action body");
    assert_eq!(clauses[0].pattern_index, 3);
    assert_eq!(clauses[0].body_index, Some(4));
}

#[test]
fn case_list_invocation_abstains_on_empty_and_truncated_shapes() {
    let switch = crate::model::ingress::static_context_for("tcl9.0").commands();
    for args in [
        &[][..],
        &["subject"][..],
        &["subject", ""][..],
        &["-regexp"][..],
        &["-regexp", "subject"][..],
        &["--"][..],
    ] {
        assert!(
            switch
                .case_invocation("switch", args, Some(SurfaceQuery::core(Family::Tcl, "9.0")))
                .is_none(),
            "truncated switch invocation must abstain: {args:?}",
        );
    }

    let expect = crate::model::ingress::static_context_for("expect").commands();
    for args in [
        &[][..],
        &["-brace"][..],
        &["-nobrace"][..],
        &["-timeout"][..],
        &["-timeout", "5"][..],
        &[""][..],
    ] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    args,
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_none(),
            "truncated Expect invocation must abstain: {args:?}",
        );
    }

    // Exercise the generic selector branch independently of either
    // built-in descriptor. The selector is intentionally outside the
    // clause-flag vocabulary, so a complete call consumes it as shape
    // syntax while a missing subject/body must simply abstain.
    let custom = crate::CaseListSpec {
        clause_force_inline_flag: Some("-inline"),
        allow_omitted_final_body: false,
        ..crate::CaseListSpec::EXPECT
    };
    for args in [
        &[][..],
        &["-inline"][..],
        &["-inline", "subject"][..],
        &[""][..],
    ] {
        assert!(
            custom
                .invocation(args, &[], Some(SurfaceQuery::any_release(Family::Tcl)))
                .is_none(),
            "truncated custom selector invocation must abstain: {args:?}",
        );
    }
    assert!(
        custom
            .invocation(
                &["-inline", "pattern", "{body}"],
                &[],
                Some(SurfaceQuery::any_release(Family::Tcl)),
            )
            .is_some(),
        "complete custom inline-selector invocation remains valid",
    );

    // A dynamic two-word switch cannot expose a static body region, but
    // asking for roles must remain a conservative abstention rather than
    // a panic. The compiler's W304 regression separately proves Tcl's
    // optionless two-word rule is retained for diagnostics.
    assert_eq!(
        switch
            .arg_indices_for_role("switch", &["$subject", "$cases"], ArgRole::Body)
            .len(),
        0,
    );
}
