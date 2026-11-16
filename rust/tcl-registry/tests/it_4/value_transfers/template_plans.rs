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

//! Substitution template plans: the witnesses, the 9.1 switches and joined plans.

use super::*;

/// `subst switches… {template}` under `dialect`'s profile through `subst`'s
/// declared template plan: the template braced, its content from 1, and
/// each switch overridden by the fact the driver would prove.
fn template_plan_of(
    dialect: &str,
    switches: &[(&'static str, Option<FactView>)],
    template: &'static str,
) -> PlanAnswer {
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("subst").expect("subst"), None, None);
    let semantics = semantics.semantics().expect("the template plan");
    let mut operands: Vec<OperandView<'static>> = switches
        .iter()
        .map(|&(text, _)| literal(text, None))
        .collect();
    operands.push(literal(template, None));
    let last = operands.len() - 1;
    let mut inputs = TestInputs::new("subst", operands);
    for (index, (_, fact)) in switches.iter().enumerate() {
        if let Some(fact) = fact {
            inputs.operands.insert(index, fact.clone());
        }
    }
    inputs.structures.insert(
        last,
        WordStructure {
            braced: true,
            quoted: false,
            parts: vec![WordPart::Literal {
                span: tcl_lexer::Span::new(1, 1 + small(template.len())),
                text: template.to_owned(),
            }],
        },
    );
    // `tcl` is the permissive sink, which names no release; an empty name
    // is no profile at all.
    let profile = tcl_dialect::DialectProfile::find(dialect)
        .or_else(|| (dialect == "tcl").then(tcl_dialect::DialectProfile::plain_tcl));
    inputs.context = AnalysisContext::detached(profile);
    semantics.structure(&inputs)
}

/// The kinds `backslashes`, `commands`, `variables`.
fn kinds(
    backslashes: bool,
    commands: bool,
    variables: bool,
) -> tcl_registry::substitution::SubstitutionKinds {
    tcl_registry::substitution::SubstitutionKinds {
        backslashes,
        commands,
        variables,
    }
}

/// The `[script]` region at word offset `start`.
fn region(start: usize, script: &str) -> tcl_registry::value_transfer::ScriptRegion {
    tcl_registry::value_transfer::ScriptRegion {
        span: tcl_lexer::Span::new(small(start), small(start + script.len() + 2)),
        script: BodyRegion {
            script: script.to_owned(),
            base_offset: start + 1,
            frame: tcl_registry::FrameLevel::Relative(0),
        },
    }
}

/// A braced template's plan, the template operand at `operand`.
fn braced_plan(
    operand: usize,
    kinds: tcl_registry::substitution::SubstitutionKinds,
    script_regions: Vec<tcl_registry::value_transfer::ScriptRegion>,
    reads: Vec<tcl_registry::value_transfer::VariableRead>,
    escapes: Vec<tcl_lexer::Span>,
) -> PlanAnswer {
    PlanAnswer::TemplateWord(tcl_registry::value_transfer::TemplateWordPlan {
        operand: OperandId(operand),
        kinds,
        braced: true,
        dynamic: false,
        script_regions,
        reads,
        escapes,
    })
}

/// The page's first five template programs under `dialect`, each the
/// plan and the plan the program must have: the kinds the switches run
/// and the regions, reads and escapes they leave.
fn switch_witnesses(dialect: &str) -> Vec<(PlanAnswer, PlanAnswer)> {
    let span = tcl_lexer::Span::new;
    vec![
        // `a$b5`: the bracket still runs.
        (
            template_plan_of(dialect, &[switch("-novariables")], "a$b[set b]"),
            braced_plan(
                1,
                kinds(true, true, false),
                vec![region(4, "set b")],
                vec![],
                vec![],
            ),
        ),
        // `a5[set b]`.
        (
            template_plan_of(dialect, &[switch("-nocommands")], "a$b[set b]"),
            braced_plan(
                1,
                kinds(true, false, true),
                vec![],
                vec![read(2, "b")],
                vec![],
            ),
        ),
        // `x6`: `$b` inside the region substitutes, as the region's own.
        (
            template_plan_of(dialect, &[switch("-novariables")], "x[expr {$b+1}]"),
            braced_plan(
                1,
                kinds(true, true, false),
                vec![region(2, "expr {$b+1}")],
                vec![],
                vec![],
            ),
        ),
        // `a1`.
        (
            template_plan_of(dialect, &[switch("-novariables")], "a[string length $b]"),
            braced_plan(
                1,
                kinds(true, true, false),
                vec![region(2, "string length $b")],
                vec![],
                vec![],
            ),
        ),
        // `a$b[set b]A`: only the escape materialises.
        (
            template_plan_of(
                dialect,
                &[switch("-novariables"), switch("-nocommands")],
                "a$b[set b]\\x41",
            ),
            braced_plan(
                2,
                kinds(true, false, false),
                vec![],
                vec![],
                vec![span(11, 15)],
            ),
        ),
    ]
}

/// The page's next six template programs under `dialect`, as
/// [`switch_witnesses`]: escapes, a proven switch, the regions that run in
/// the caller's frame, and the two families together.
fn template_witnesses(dialect: &str) -> Vec<(PlanAnswer, PlanAnswer)> {
    let span = tcl_lexer::Span::new;
    let proven = |text: &str| Some(FactView::Exact(ExactValue::from_literal(text), None));
    vec![
        // `a\tb`, four characters.
        (
            template_plan_of(dialect, &[switch("-nobackslashes")], "a\\tb"),
            braced_plan(1, kinds(false, true, true), vec![], vec![], vec![]),
        ),
        // `a$b5`: the escape protects the `$`.
        (
            template_plan_of(dialect, &[], "a\\$b[set b]"),
            braced_plan(
                0,
                kinds(true, true, true),
                vec![region(5, "set b")],
                vec![],
                vec![span(2, 4)],
            ),
        ),
        // `set opt -novariables; subst $opt {hello $name}` is `hello
        // $name`: the proven switch reads as its spelling.
        (
            template_plan_of(dialect, &[("$opt", proven("-novariables"))], "hello $name"),
            braced_plan(1, kinds(true, true, false), vec![], vec![], vec![]),
        ),
        // `p` returns 2: the region runs in the caller's frame.
        (
            template_plan_of(dialect, &[], "[set c 2]"),
            braced_plan(
                0,
                kinds(true, true, true),
                vec![region(1, "set c 2")],
                vec![],
                vec![],
            ),
        ),
        // `q` returns `2 2`.
        (
            template_plan_of(dialect, &[switch("-novariables")], "[incr c]"),
            braced_plan(
                1,
                kinds(true, true, false),
                vec![region(1, "incr c")],
                vec![],
                vec![],
            ),
        ),
        // The two families together: an error on every release.
        (
            template_plan_of(
                dialect,
                &[switch("-nocommands"), switch("-variables")],
                "a$b",
            ),
            PlanAnswer::Declined(DeclineReason::WrongRepresentation),
        ),
    ]
}

/// `subst`'s template-word plan answers the page's fourteen
/// programs (`docs/design/compiler/value-transfers.md` § *The template-word
/// plan*): the kinds its switches run, read over their proven values, and
/// the braced template's script regions, variable reads and escapes under
/// those kinds, each at its offset in the word (the content from 1). A
/// template the parser substitutes reaches the command computed. The two
/// families together are an error on every release.
#[test]
fn the_template_plan_answers_the_fourteen_witnesses() {
    let span = tcl_lexer::Span::new;
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let rows = switch_witnesses(dialect)
            .into_iter()
            .chain(template_witnesses(dialect));
        for (index, (got, want)) in rows.enumerate() {
            assert_eq!(got, want, "{dialect}: row {index}");
        }
    }
    // `set t {a$b}; subst -nocommands $t` is `a5`: a template the parser
    // substitutes reaches the command computed.
    let mut dynamic = TestInputs::new(
        "subst",
        vec![literal("-nocommands", None), literal("$t", None)],
    );
    dynamic.structures.insert(
        1,
        WordStructure {
            braced: false,
            quoted: false,
            parts: vec![WordPart::VariableRead {
                span: span(0, 2),
                name: "t".to_owned(),
                element: None,
            }],
        },
    );
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("subst").expect("subst"), None, None);
    assert_eq!(
        semantics
            .semantics()
            .expect("the template plan")
            .structure(&dynamic),
        PlanAnswer::TemplateWord(tcl_registry::value_transfer::TemplateWordPlan {
            operand: OperandId(1),
            kinds: kinds(true, false, true),
            braced: false,
            dynamic: true,
            script_regions: vec![],
            reads: vec![],
            escapes: vec![],
        })
    );
}

/// The 9.1 positive family: it answers under a 9.1 profile, is the
/// command's error below it (`bad switch "-variables"` on tclsh 8.4 and
/// 8.5, `bad option` on 8.6 and 9.0), and declines as release-ambiguous
/// under a profile that spans both, while a question with no profile reads
/// every switch; the two families together raise on every release, so
/// they raise under the spanning profile too.
#[test]
fn the_positive_switches_are_9_1s() {
    let span = tcl_lexer::Span::new;
    let spanning = PlanAnswer::Declined(DeclineReason::ReleaseAmbiguous(Axis::Availability(
        tcl_dialect::model::SpecSurface::TCL91[0],
    )));
    let positive = [
        (
            "-variables",
            "a$b[set b]",
            braced_plan(
                1,
                kinds(false, false, true),
                vec![],
                vec![read(2, "b")],
                vec![],
            ),
        ),
        (
            "-backslashes",
            "a$b[set b]\\x41",
            braced_plan(
                1,
                kinds(true, false, false),
                vec![],
                vec![],
                vec![span(11, 15)],
            ),
        ),
    ];
    for (word, template, answered) in positive {
        assert_eq!(
            template_plan_of("tcl9.1", &[switch(word)], template),
            answered
        );
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert_eq!(
                template_plan_of(dialect, &[switch(word)], template),
                PlanAnswer::Declined(DeclineReason::WrongRepresentation),
                "{dialect} {word}"
            );
        }
        assert_eq!(template_plan_of("tcl", &[switch(word)], template), spanning);
        // With no profile at all the question is surface-blind, as a
        // profile-less registry's own: every switch is available.
        assert_eq!(
            template_plan_of("", &[switch(word)], template),
            template_plan_of("tcl9.1", &[switch(word)], template)
        );
    }
    assert_eq!(
        template_plan_of("tcl", &[switch("-nocommands"), switch("-variables")], "a$b"),
        PlanAnswer::Declined(DeclineReason::WrongRepresentation)
    );
}

/// A finite set of switch values joins per member, a raising member
/// contributing nothing; an unproven switch runs every kind; a call without
/// its template, or a template holding a construct `subst` rejects, is the
/// command's error; and an array index substitutes
/// whatever the kinds say — `subst -nocommands {$a([set b])}` runs `set b`
/// (tclsh 8.4 to 9.1 read `a(5)`).
#[test]
fn a_template_plan_joins_proven_switches_and_reads_indexes() {
    let set = |members: &[&str]| {
        Some(FactView::Finite(
            members
                .iter()
                .map(|member| ExactValue::from_literal(member))
                .collect(),
            None,
        ))
    };
    assert_eq!(
        template_plan_of("tcl9.1", &[("$s", set(&["-variables", "-commands"]))], "x"),
        braced_plan(1, kinds(false, true, true), vec![], vec![], vec![])
    );
    assert_eq!(
        template_plan_of("tcl8.6", &[("$s", set(&["-novariables", "-bogus"]))], "x"),
        braced_plan(1, kinds(true, true, false), vec![], vec![], vec![])
    );
    assert_eq!(
        template_plan_of(
            "tcl8.6",
            &[("$s", Some(FactView::Top(DeclineReason::NotExact)))],
            "a$b"
        ),
        braced_plan(
            1,
            kinds(true, true, true),
            vec![],
            vec![read(2, "b")],
            vec![]
        )
    );
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("subst").expect("subst"), None, None);
    assert_eq!(
        semantics
            .semantics()
            .expect("the template plan")
            .structure(&TestInputs::new("subst", vec![])),
        PlanAnswer::Declined(DeclineReason::WrongRepresentation)
    );
    // A construct `subst` rejects is the command's error: `subst {a[set b}`
    // raises `missing close-bracket` on tclsh 8.4 to 9.1.
    assert_eq!(
        template_plan_of("tcl8.6", &[], "a[set b"),
        PlanAnswer::Declined(DeclineReason::WrongRepresentation)
    );
    assert_eq!(
        template_plan_of("tcl8.6", &[switch("-nocommands")], "a[set b"),
        braced_plan(1, kinds(true, false, true), vec![], vec![], vec![])
    );
    let PlanAnswer::TemplateWord(indexed) =
        template_plan_of("tcl8.6", &[switch("-nocommands")], "$a([set b])")
    else {
        panic!("a template plan");
    };
    assert_eq!(
        indexed.reads,
        [tcl_registry::value_transfer::VariableRead {
            span: tcl_lexer::Span::new(1, 12),
            name: "a".to_owned(),
            element: Some("[set b]".to_owned()),
        }]
    );
    assert_eq!(
        indexed
            .script_regions
            .iter()
            .map(|region| region.script.script.as_str())
            .collect::<Vec<_>>(),
        ["set b"]
    );
}
