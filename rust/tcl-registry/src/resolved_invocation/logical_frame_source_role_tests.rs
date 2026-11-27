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

use crate::value_transfer::OperandId;
use crate::{
    ArgRole, CommandRegistry, FrameLevel, InvocationArguments, InvocationResolutionUnresolved,
    InvocationWord, InvocationWords, TclType,
};
use tcl_native_model::InvocationDialect;

#[test]
fn logical_frame_source_roles_use_consensus_without_native_frame_facts() {
    // naming.source.original-produced-command-prefix
    // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = crate::model::ingress::context_for_profile(profile);
    let dialect = InvocationDialect::of_profile(profile);
    assert!(dialect.native_name_protocol().is_none());
    for (head, arguments, roles) in [
        (
            "upvar",
            vec![
                InvocationWord::Literal("#0"),
                InvocationWord::Literal("original"),
                InvocationWord::Literal("local"),
            ],
            vec![(1, ArgRole::VarCell), (2, ArgRole::VarWrite)],
        ),
        (
            "uplevel",
            vec![InvocationWord::Literal("#0"), InvocationWord::Dynamic],
            vec![(1, ArgRole::Body)],
        ),
    ] {
        let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            InvocationWords::structured(InvocationWord::Literal(head), &arguments)
                .with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        let schema = resolution.resolved().unwrap();
        assert_eq!(
            schema.authored_logical_source_argument_roles(),
            (roles, true)
        );
        assert_eq!(
            schema.authored_logical_source_script_arguments(),
            Some(if head == "uplevel" {
                vec![1]
            } else {
                Vec::new()
            }),
        );
        assert_eq!(
            schema.authored_logical_source_plain_script_arguments(),
            schema.authored_logical_source_script_arguments(),
        );
        assert!(schema.authored_source_script_arguments().is_none());
        assert!(schema.authored_source_plain_script_arguments().is_none());
        assert!(!schema.authored_source_argument_roles().1);
        let facts = schema.facts();
        assert!(!facts.arg_roles_complete);
        assert_eq!(
            facts
                .frame_effect
                .unwrap()
                .resolve_arguments(schema.words.arguments()),
            crate::frame_effect::FrameArgumentResolution::Unknown,
        );
        if head == "uplevel" {
            assert!(facts.effects.requires_world_barrier());
        } else {
            // A callback barrier is not an unknown alias target. The
            // registry retains the unresolved cell/trace transition;
            // Logical source roles do not install a native frame link.
            let transitions = facts.state_transitions.declared().unwrap();
            assert_eq!(transitions.facts().len(), 1);
            assert!(matches!(
                &transitions.facts()[0].transition,
                crate::StateTransition::Widen(widening)
                    if widening.domains == [
                        crate::StateTransitionDomain::VariableCells,
                        crate::StateTransitionDomain::VariableTraces,
                    ]
            ));
        }
    }
}

#[test]
fn logical_frame_source_roles_keep_dynamic_divergent_and_expanded_layouts_unknown() {
    // naming.source.original-produced-command-prefix
    // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = crate::model::ingress::context_for_profile(profile);
    for arguments in [
        vec![
            InvocationWord::Dynamic,
            InvocationWord::Literal("original"),
            InvocationWord::Literal("local"),
        ],
        vec![
            InvocationWord::Literal("+1"),
            InvocationWord::Literal("original"),
            InvocationWord::Literal("local"),
        ],
        vec![InvocationWord::Expanded],
    ] {
        let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            InvocationWords::structured(InvocationWord::Literal("upvar"), &arguments)
                .with_dialect(InvocationDialect::of_profile(profile)),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        let schema = resolution.resolved().unwrap();
        assert_eq!(
            schema.authored_logical_source_argument_roles(),
            (Vec::new(), false)
        );
        assert!(!schema.facts().arg_roles_complete);
    }
}

/// The context a derived-query test resolves under: `profile`'s.
fn derived_context(profile: &str) -> crate::value_transfer::AnalysisContext {
    crate::value_transfer::AnalysisContext::detached(Some(
        tcl_dialect::DialectProfile::find(profile).expect("a catalogued profile"),
    ))
}

/// The derived-query layer on a literal call: every query answers, under
/// the query the invocation resolved at — the one the context fixes.
#[test]
fn derived_queries_answer_a_literal_call() {
    let context = derived_context("tcl9.0");
    let registry = CommandRegistry::build_default();
    let resolve = |name, args: &'static [&'static str]| {
        registry
            .invocation(InvocationWords::literals(name, args), &context)
            .resolved()
            .expect("a shipped command resolves")
    };

    let array_for = resolve("array", &["for", "{k v}", "a", "{body}"]);
    assert_eq!(array_for.dialect, context.surface_query());
    let plan = array_for.clause_plan().expect("`array for` walks at 9.0");
    assert!(plan.roles.contains(&(3, ArgRole::Body)), "{plan:?}");

    let subst = resolve("subst", &["-nocommands", "$x"]);
    let effects = subst.option_effects();
    assert!(effects.complete);
    assert_eq!(effects.option_end, 1);
    let kinds = subst.substitutions_performed().expect("subst substitutes");
    assert!(kinds.variables && kinds.backslashes && !kinds.commands);

    let upvar = resolve("upvar", &["1", "a", "b"]);
    let roles = upvar.arg_roles().expect("a literal layout");
    assert!(roles.contains(&(2, ArgRole::VarWrite)), "{roles:?}");
    assert_eq!(
        upvar.frame_effect(),
        Some((FrameLevel::Relative(1), vec![OperandId(1), OperandId(2)]))
    );

    let lsearch = resolve("lsearch", &["-regexp", "$list", "a.*"]);
    let patterns = lsearch.pattern_args();
    assert_eq!(patterns.len(), 1, "{patterns:?}");
    assert_eq!(patterns[0].index, 2);

    let switch = resolve("switch", &["-glob", "x", "a*", "body"]);
    let (case, clauses) = switch.case_invocation().expect("a case list");
    assert_eq!(case.subject_index, Some(1));
    assert_eq!(case.mode, crate::spec::CaseMatchMode::Glob);
    assert_eq!(clauses.len(), 1);
    assert_eq!(clauses[0].body_index, Some(3));

    assert_eq!(
        resolve("string", &["length", "abc"]).return_type(),
        Some(TclType::Int)
    );
    assert_ne!(resolve("set", &["x", "1"]).effects().accesses().len(), 0);
}

/// Two resolutions no query can answer from: a computed head resolves
/// nothing, and neither does a command the context's release lacks.
#[test]
fn derived_queries_abstain_on_a_computed_head_and_an_absent_command() {
    let registry = CommandRegistry::build_default();
    let context = derived_context("tcl8.6");
    let arguments = [crate::InvocationWord::Literal("x")];
    let computed_head = registry.invocation(
        InvocationWords::structured(crate::InvocationWord::Dynamic, &arguments),
        &context,
    );
    assert!(computed_head.resolved().is_none());
    assert!(matches!(
        computed_head.unresolved(),
        Some(InvocationResolutionUnresolved::ComputedHead { .. })
    ));
    let absent = registry.invocation(InvocationWords::literals("lpop", &["l"]), &context);
    assert!(absent.resolved().is_none(), "`lpop` is Tcl 9.0");
    assert!(matches!(
        absent.unresolved(),
        Some(InvocationResolutionUnresolved::UnknownLiteralHead { spelling: "lpop" })
    ));
}

/// Each query abstains rather than answering for words it cannot read:
/// an expansion abstains the role table, the frame and a return-type
/// hook; a computed word where an option could stand abstains the role
/// table and the case-list reading; a computed word where none can is an
/// operand like any other.
#[test]
fn derived_queries_abstain_on_words_they_cannot_read() {
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let registry = CommandRegistry::build_default();
    let context = derived_context("tcl8.6");
    let resolve = |head: &'static str, arguments: &'static [crate::InvocationWord<'static>]| {
        registry
            .invocation(
                InvocationWords::structured(Literal(head), arguments),
                &context,
            )
            .resolved()
            .expect("a shipped command resolves")
    };

    let upvar = resolve("upvar", &[Expanded, Literal("b")]);
    assert_eq!(upvar.arg_roles(), None);
    assert_eq!(upvar.frame_effect(), None);
    assert_eq!(
        resolve("regexp", &[Expanded, Literal("s")]).return_type(),
        None
    );

    let switch = resolve("switch", &[Dynamic, Literal("x"), Literal("a {}")]);
    assert_eq!(switch.case_invocation(), None);
    assert_eq!(switch.arg_roles(), None);

    let switch = resolve("switch", &[Dynamic, Literal("a {}")]);
    let (case, _) = switch.case_invocation().expect("the reading holds");
    assert_eq!(case.subject_index, Some(0));
    assert_eq!(case.clause_list_index, Some(1));
    // A computed pattern is a pattern: `switch` has no clause flags.
    let switch = resolve("switch", &[Literal("x"), Dynamic, Literal("{b}")]);
    let (_, clauses) = switch.case_invocation().expect("the reading holds");
    assert_eq!(clauses[0].pattern_index, 1);

    let uplevel = resolve("uplevel", &[Dynamic, Literal("{set x 1}")]);
    assert_eq!(
        uplevel.frame_effect(),
        None,
        "the unknown first value can select a level or remain part of the script"
    );
    let uplevel = resolve("uplevel", &[Dynamic]);
    assert_eq!(uplevel.frame_effect(), None);
    let uplevel = resolve("uplevel", &[Literal("{set x 1}")]);
    assert_eq!(
        uplevel.frame_effect(),
        Some((FrameLevel::DEFAULT, vec![OperandId(0)])),
        "a known non-level value is the script"
    );
}

/// Every answer is keyed on the context: the same words answer
/// differently under releases that read them differently.
#[test]
fn derived_queries_answer_under_the_context_release() {
    let registry = CommandRegistry::build_default();
    let at = |profile: &str, name: &'static str, args: &'static [&'static str]| {
        let context = derived_context(profile);
        let invocation = registry
            .invocation(InvocationWords::literals(name, args), &context)
            .resolved()
            .expect("a shipped command resolves");
        (
            invocation.clause_plan().is_some(),
            invocation.frame_effect().map(|(level, _)| level),
            invocation.substitutions_performed(),
        )
    };
    // `array for` is Tcl 9.0.
    assert!(!at("tcl8.6", "array", &["for", "{k v}", "a", "{}"]).0);
    assert!(at("tcl9.0", "array", &["for", "{k v}", "a", "{}"]).0);
    // A leading-zero level is octal on 8.6 and decimal on 9.0.
    assert_eq!(
        at("tcl8.6", "upvar", &["010", "a", "b"]).1,
        Some(FrameLevel::Relative(8))
    );
    assert_eq!(
        at("tcl9.0", "upvar", &["010", "a", "b"]).1,
        Some(FrameLevel::Relative(10))
    );
    // `subst`'s positive switches are Tcl 9.1: below it the call is
    // unreadable and every kind runs.
    let positive = at("tcl9.1", "subst", &["-variables", "$x"])
        .2
        .expect("subst substitutes");
    assert!(positive.variables && !positive.commands && !positive.backslashes);
    let below = at("tcl9.0", "subst", &["-variables", "$x"])
        .2
        .expect("subst substitutes");
    assert!(below.variables && below.commands && below.backslashes);
}

/// The one place the derived queries and the by-name answers part: the
/// resolution selects its subcommand under its release, so a prefix the
/// release makes unique selects it — `array d` is `donesearch` at 8.6,
/// where a release-blind lookup finds it ambiguous with 9.0's `default`.
#[test]
fn a_prefix_the_release_makes_unique_selects_its_subcommand() {
    let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
    let invocation = registry
        .invocation(
            InvocationWords::literals("array", &["d", "a", "s"]),
            &derived_context("tcl8.6"),
        )
        .resolved()
        .expect("`array` resolves");
    assert_eq!(
        invocation
            .subcommand
            .resolved()
            .map(|sub| sub.canonical_name),
        Some("donesearch")
    );
    let roles = invocation.arg_roles().expect("a literal layout");
    assert!(roles.contains(&(1, ArgRole::VarRead)), "{roles:?}");
    assert!(
        CommandRegistry::build_default()
            .arg_indices_for_role("array", &["d", "a", "s"], ArgRole::VarRead)
            .is_empty(),
        "the release-blind lookup finds `d` ambiguous"
    );
}

/// Calls whose layout a computed word decides, or leaves decided: the
/// corpus the derived role table is held to the by-name one on.
const COMPUTED_WORD_CORPUS: &[(&str, &[crate::InvocationWord<'static>])] = {
    use crate::InvocationWord::{Dynamic, DynamicNonOption, Expanded, Literal};
    &[
        ("upvar", &[Dynamic, Literal("a"), Literal("b")]),
        ("upvar", &[Literal("1"), Dynamic, Literal("b"), Dynamic]),
        ("switch", &[Dynamic, Literal("{a {b}}")]),
        ("switch", &[Dynamic, Literal("x"), Literal("{a b}")]),
        (
            "switch",
            &[Literal("-glob"), Literal("--"), Dynamic, Literal("{a b}")],
        ),
        (
            "switch",
            &[DynamicNonOption, Literal("x"), Literal("{a b}")],
        ),
        ("case", &[Dynamic, Literal("in"), Literal("{a {b}}")]),
        ("lsearch", &[Dynamic, Literal("l"), Literal("a*")]),
        ("lsearch", &[Literal("-glob"), Dynamic, Dynamic]),
        ("regexp", &[Dynamic, Literal("s"), Literal("m")]),
        ("regexp", &[Literal("-inline"), Dynamic, Dynamic]),
        ("foreach", &[Dynamic, Dynamic, Literal("{body}")]),
        ("foreach", &[Literal("{a b}"), Dynamic, Literal("{body}")]),
        (
            "if",
            &[Dynamic, Literal("{a}"), Literal("else"), Literal("{b}")],
        ),
        ("if", &[Literal("{$c}"), Dynamic, Literal("{a}")]),
        (
            "try",
            &[
                Literal("{a}"),
                Literal("on"),
                Literal("error"),
                Dynamic,
                Literal("{b}"),
            ],
        ),
        (
            "try",
            &[
                Literal("{a}"),
                Dynamic,
                Literal("error"),
                Literal("m"),
                Literal("{b}"),
            ],
        ),
        (
            "dict",
            &[Literal("for"), Literal("{k v}"), Dynamic, Literal("{body}")],
        ),
        ("dict", &[Dynamic, Literal("d")]),
        ("string", &[Dynamic, Literal("abc")]),
        ("lsort", &[Literal("-command"), Dynamic, Dynamic]),
        ("after", &[Literal("100"), Dynamic]),
        ("format", &[Dynamic, Literal("a")]),
        ("scan", &[Dynamic, Literal("%d"), Literal("v")]),
        ("set", &[Expanded]),
        ("namespace", &[Literal("eval"), Dynamic, Literal("{body}")]),
        (
            "trace",
            &[
                Literal("add"),
                Literal("variable"),
                Literal("x"),
                Literal("write"),
                Dynamic,
            ],
        ),
        ("lassign", &[Dynamic, Literal("a"), Literal("b")]),
        (
            "interp",
            &[
                Literal("alias"),
                Literal("{}"),
                Literal("a"),
                Literal("{}"),
                Dynamic,
            ],
        ),
    ]
};

/// `arg_roles` is `arg_indices_for_role_words` over every role, re-keyed
/// on the resolution — on computed words too, where the layout proof
/// decides between a position and an abstention.
#[test]
fn arg_roles_agree_with_the_registry_role_answer_on_computed_words() {
    use crate::InvocationWord::Literal;
    let corpus = COMPUTED_WORD_CORPUS;
    for (profile, &(name, arguments)) in ["tcl8.6", "tcl9.0"]
        .into_iter()
        .flat_map(|profile| corpus.iter().map(move |row| (profile, row)))
    {
        let registry = crate::model::ingress::static_context_for(profile).commands();
        let context = derived_context(profile);
        let Some(invocation) = registry
            .invocation(
                InvocationWords::structured(Literal(name), arguments),
                &context,
            )
            .resolved()
        else {
            assert_eq!(
                name, "case",
                "{profile}: only `case` is absent (Tcl 9 dropped it)"
            );
            continue;
        };
        let words = InvocationArguments::structured(arguments);
        let expected = ArgRole::ALL
            .iter()
            .map(|&role| {
                registry
                    .arg_indices_for_role_words(name, words, role)
                    .map(|indices| indices.into_iter().map(move |index| (index, role)))
            })
            .collect::<Option<Vec<_>>>()
            .map(|per_role| {
                let mut roles: Vec<(usize, ArgRole)> = per_role.into_iter().flatten().collect();
                roles.sort_by_key(|&(index, _)| index);
                roles.dedup();
                roles
            });
        assert_eq!(
            invocation.arg_roles(),
            expected,
            "{profile} {name} {arguments:?}: the derived table"
        );
        assert_eq!(
            invocation.pattern_args(),
            registry.pattern_args_words(name, words),
            "{profile} {name} {arguments:?}: the pattern answer"
        );
    }
}

/// The return-type hooks read a computed word as its source spelling
/// would read — a value they cannot see — so the query answers what
/// `return_type_for_call` answers over the source text.
#[test]
fn return_type_reads_a_computed_word_as_its_source_spelling_reads() {
    use crate::InvocationWord::{Dynamic, Literal};
    let registry = CommandRegistry::build_default();
    let context = derived_context("tcl9.0");
    let corpus: &[(&str, &[crate::InvocationWord<'static>], &[&str])] = &[
        (
            "lsearch",
            &[Literal("-all"), Dynamic, Dynamic],
            &["-all", "$l", "$p"],
        ),
        (
            "regexp",
            &[Dynamic, Literal("a"), Literal("b")],
            &["$opt", "a", "b"],
        ),
        (
            "regexp",
            &[Literal("--"), Dynamic, Dynamic],
            &["--", "$re", "$s"],
        ),
        (
            "scan",
            &[Dynamic, Literal("%d"), Literal("v")],
            &["$s", "%d", "v"],
        ),
        (
            "regsub",
            &[Dynamic, Dynamic, Dynamic],
            &["$re", "$s", "$sub"],
        ),
    ];
    for &(name, arguments, source) in corpus {
        let invocation = registry
            .invocation(
                InvocationWords::structured(Literal(name), arguments),
                &context,
            )
            .resolved()
            .expect("a shipped command resolves");
        let spec = registry.get(name).expect("a shipped command");
        assert_eq!(
            invocation.return_type(),
            spec.return_type_for_call(source),
            "{name} {source:?}"
        );
    }
    // The C9 selected option schema includes `-about`; the legacy
    // descriptor-only text facade has no independent availability point.
    // Dynamic pattern bytes do not withdraw this selected source shape.
    let about = [Literal("-about"), Dynamic];
    let selected = registry
        .invocation(
            InvocationWords::structured(Literal("regexp"), &about),
            &context,
        )
        .resolved()
        .unwrap();
    assert_eq!(selected.return_type(), Some(TclType::List));
    assert_eq!(
        registry
            .get("regexp")
            .unwrap()
            .return_type_for_call(&["-about", "$re"]),
        Some(TclType::Int),
        "the scalar compatibility facade cannot donate a C-only option point"
    );
    let arguments = [Dynamic, Dynamic];
    let invocation = registry
        .invocation(
            InvocationWords::structured(Literal("lsearch"), &arguments),
            &context,
        )
        .resolved()
        .unwrap();
    assert_eq!(invocation.return_type(), Some(TclType::Int));
    assert_eq!(
        registry
            .get("lsearch")
            .unwrap()
            .return_type_for_call(&["$l", "$p"]),
        None,
        "legacy text does not retain the two mandatory argv slots"
    );
}
