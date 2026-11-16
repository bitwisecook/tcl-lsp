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

//! Arm selection plans for `switch` and `case` on every release.

use super::*;

/// `switch words…` under `dialect` (`tcl` is the lenient sink, which names
/// no release) over literal words, the operand at `subject` holding `fact`
/// when one is given: the inputs a selection runs over.
fn switch_inputs<'a>(
    dialect: &str,
    words: &[&'a str],
    subject: Option<(usize, FactView)>,
) -> TestInputs<'a> {
    let operands = words.iter().map(|&text| literal(text, None)).collect();
    let mut inputs = TestInputs::new("switch", operands);
    if let Some((index, fact)) = subject {
        inputs.operands.insert(index, fact);
    }
    let profile = tcl_dialect::DialectProfile::find(dialect)
        .or_else(|| (dialect == "tcl").then(tcl_dialect::DialectProfile::plain_tcl));
    inputs.context = AnalysisContext::detached(profile);
    inputs
}

/// A selection as the selected arms, the arms whose bodies run, and each
/// member's writes as `(operand, written text)`.
type SelectionRead = (
    Vec<Option<usize>>,
    Vec<Option<usize>>,
    Vec<Vec<(usize, String)>>,
);

/// The `Selection` transfer of `switch words…` under `dialect`, read as a
/// [`SelectionRead`], or the decline.
fn switch_selection(
    dialect: &str,
    words: &[&str],
    subject: Option<(usize, FactView)>,
) -> Result<SelectionRead, DeclineReason> {
    selection_of("switch", dialect, words, subject, Vec::new())
}

/// A literal operand's structure, delimited as said: its text as one run
/// from offset 0, or from 1 past a brace or a quote.
fn delimited(text: &str, braced: bool, quoted: bool) -> WordStructure {
    let start = u32::from(braced || quoted);
    WordStructure {
        braced,
        quoted,
        parts: vec![WordPart::Literal {
            span: tcl_lexer::Span::new(start, start + small(text.len())),
            text: text.to_owned(),
        }],
    }
}

/// The `Selection` transfer of `command words…` under `dialect`, with the
/// operands' structures where given, read as a [`SelectionRead`], or the
/// decline.
fn selection_of(
    command: &str,
    dialect: &str,
    words: &[&str],
    subject: Option<(usize, FactView)>,
    structures: Vec<(usize, WordStructure)>,
) -> Result<SelectionRead, DeclineReason> {
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get(command).expect(command), None, None);
    let semantics = semantics.semantics().expect("a selection contract");
    let mut inputs = switch_inputs(dialect, words, subject);
    inputs.view.canonical_command = if command == "case" { "case" } else { "switch" };
    inputs.structures.extend(structures);
    match semantics.transfer(FactDomain::Selection, &inputs, &mut Budget::evaluation()) {
        TransferAnswer::Selection(fact) => Ok((
            fact.selected,
            fact.bodies,
            fact.writes
                .into_iter()
                .map(|writes| {
                    writes
                        .into_iter()
                        .map(|store| match store {
                            StoreOutcome::Write { target, value } => {
                                ((target.0).0, String::from_utf8(value.bytes).expect("text"))
                            }
                            other => panic!("unexpected store {other:?}"),
                        })
                        .collect()
                })
                .collect(),
        )),
        TransferAnswer::Declined(reason) => Err(reason),
        other => panic!("not a selection: {other:?}"),
    }
}

/// No write for any member of a one-member selection.
fn no_writes() -> Vec<Vec<(usize, String)>> {
    vec![Vec::new()]
}

/// The plan `switch`'s `CaseListSpec` reads: the inline form names each
/// pair, a `-` arm with no body; the one-word form names its clause list.
fn the_case_list_plan_names_each_form() {
    use tcl_registry::value_transfer::CaseArms;
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("switch").expect("switch"), None, None);
    let semantics = semantics.semantics().expect("switch's selection contract");
    let PlanAnswer::CaseList { subject, arms, .. } = semantics.structure(&switch_inputs(
        "tcl8.6",
        &["-glob", "--", "x", "a*", "-", "b", "B"],
        None,
    )) else {
        panic!("a case list");
    };
    assert_eq!(subject, OperandId(2));
    assert_eq!(
        arms,
        CaseArms::Words(vec![
            (OperandId(3), None),
            (OperandId(5), Some(OperandId(6)))
        ])
    );
    let PlanAnswer::CaseList { arms, .. } =
        semantics.structure(&switch_inputs("tcl8.6", &["x", "a A"], None))
    else {
        panic!("a case list");
    };
    assert_eq!(arms, CaseArms::List(OperandId(1)));
}

/// What every release selects alike: ordered first match, the final
/// `default` (a non-final one is a literal pattern), a `-` arm supplying
/// the next body, a `ConstSet` subject per member, and a malformed regexp
/// declining.
fn every_release_selects_alike(dialect: &str) {
    assert_eq!(
        switch_selection(
            dialect,
            &["-glob", "--", "abc", "a* A ab* B default D"],
            None
        ),
        Ok((vec![Some(0)], vec![Some(0)], no_writes())),
        "{dialect}"
    );
    assert_eq!(
        switch_selection(
            dialect,
            &["-exact", "--", "zzz", "a", "A", "default", "D"],
            None
        ),
        Ok((vec![Some(1)], vec![Some(1)], no_writes())),
        "{dialect}"
    );
    assert_eq!(
        switch_selection(dialect, &["--", "zzz", "default", "A", "b", "B"], None),
        Ok((vec![None], vec![None], no_writes())),
        "{dialect}"
    );
    assert_eq!(
        switch_selection(dialect, &["-glob", "--", "x", "x - y Y"], None),
        Ok((vec![Some(0)], vec![Some(1)], no_writes())),
        "{dialect}"
    );
    let members = FactView::Finite(
        vec![
            ExactValue::from_literal("a"),
            ExactValue::from_literal("b"),
            ExactValue::from_literal("z"),
        ],
        None,
    );
    assert_eq!(
        switch_selection(
            dialect,
            &["-exact", "--", "$s", "a", "A", "b", "B"],
            Some((2, members))
        ),
        Ok((
            vec![Some(0), Some(1), None],
            vec![Some(0), Some(1), None],
            vec![Vec::new(), Vec::new(), Vec::new()]
        )),
        "{dialect}"
    );
    assert_eq!(
        switch_selection(dialect, &["-regexp", "--", "abc", "( A"], None),
        Err(DeclineReason::WrongRepresentation),
        "{dialect}"
    );
}

/// What 8.5 onwards selects: the captures — the index variable's write,
/// then the match variable's; the default arm's empty write — `-nocase`,
/// and a subject spelled like an option in the one-word form.
fn from_85_the_captures_and_nocase_select(dialect: &str) {
    assert_eq!(
        switch_selection(
            dialect,
            &[
                "-regexp",
                "-matchvar",
                "m",
                "-indexvar",
                "i",
                "--",
                "abc",
                "(a)(x)?b M"
            ],
            None
        ),
        Ok((
            vec![Some(0)],
            vec![Some(0)],
            vec![vec![
                (4, "{0 1} {0 0} {-1 -1}".to_owned()),
                (2, "ab a {}".to_owned())
            ]]
        )),
        "{dialect}"
    );
    assert_eq!(
        switch_selection(
            dialect,
            &["-regexp", "-matchvar", "m", "--", "abc", "x X default D"],
            None
        ),
        Ok((vec![Some(1)], vec![Some(1)], vec![vec![(2, String::new())]])),
        "{dialect}"
    );
    assert_eq!(
        switch_selection(dialect, &["-nocase", "--", "ABC", "abc A"], None),
        Ok((vec![Some(0)], vec![Some(0)], no_writes())),
        "{dialect}"
    );
    assert_eq!(
        switch_selection(
            dialect,
            &["-glob", "$s", "a A default D"],
            Some((1, FactView::Exact(ExactValue::from_literal("-x"), None)))
        ),
        Ok((vec![Some(1)], vec![Some(1)], no_writes())),
        "{dialect}"
    );
}

/// `switch` declares its selection contract (§ *`switch`* of the interface
/// page): the case-list plan its `CaseListSpec` reads — each arm a pair of words,
/// or the elements of one clause-list word — and, per member of a proven
/// subject, the arm the shared core selects: ordered first match, the final
/// `default` (a non-final one is a literal pattern), a `-` arm supplying
/// the body of an arm whose pattern never matches, the regexp mode's
/// captures as `Write`s of the `-indexvar` and `-matchvar` words, and a
/// `ConstSet` subject one entry per member. A malformed regexp, an option
/// the release lacks, a subject that release scans as an option, and 9.1's
/// `-integer` decline. Each answer is tclsh 8.4.20 to 9.1b0's
/// (`switch_witnesses_match_every_release_on_path`).
#[test]
fn switch_selection_runs_the_shared_core() {
    the_case_list_plan_names_each_form();
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl9.1", "tcl"] {
        every_release_selects_alike(dialect);
    }
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        from_85_the_captures_and_nocase_select(dialect);
    }
    // 8.4 has no `-nocase` and scans every leading word spelled like an
    // option, and a profile naming no release reads both ways.
    let dashed = || Some((1, FactView::Exact(ExactValue::from_literal("-x"), None)));
    for dialect in ["tcl8.4", "tcl"] {
        assert!(
            switch_selection(dialect, &["-nocase", "--", "ABC", "abc A"], None).is_err(),
            "{dialect}"
        );
        assert!(
            switch_selection(dialect, &["-glob", "$s", "a A default D"], dashed()).is_err(),
            "{dialect}"
        );
    }
    // With two arms after the subject every release scans it.
    assert!(
        switch_selection(
            "tcl8.6",
            &["-glob", "$s", "a", "A", "default", "D"],
            dashed()
        )
        .is_err()
    );
    // 9.1's `-integer` is not the core's comparison.
    assert_eq!(
        switch_selection("tcl9.1", &["-integer", "--", "1", "1 A"], None),
        Err(DeclineReason::Unsupported)
    );
}

/// 9.1b0's byte-compiled `switch` reads only a bare `-` as the fall-through
/// body, its interpreted path the word's value: measured on tclsh
/// 9.1b0, `switch -glob -- a a "-" b {…}` runs `-` as a command inside a
/// procedure and falls through at a script's top level, where 8.4.20 to
/// 9.0.4 fall through on both paths. So under a profile that may be 9.1 a
/// member whose selection reaches a quoted or braced `-` body — selecting
/// its arm, or falling through into it — declines the whole fact on the
/// availability axis; a bare `-`, a substituted word, the one-word form's
/// elements, and a member that never reaches the arm still decide.
#[test]
fn a_delimited_fallthrough_body_reads_two_ways_under_91() {
    let words = ["-glob", "--", "a", "a", "-", "b", "B"];
    let ambiguous = Err(DeclineReason::ReleaseAmbiguous(Axis::Availability(
        tcl_dialect::model::SpecSurface::TCL91[0],
    )));
    let decided = Ok((vec![Some(0)], vec![Some(1)], vec![Vec::new()]));
    for (braced, quoted) in [(false, true), (true, false)] {
        let body = || vec![(4, delimited("-", braced, quoted))];
        for dialect in ["tcl9.1", "tcl"] {
            assert_eq!(
                selection_of("switch", dialect, &words, None, body()),
                ambiguous,
                "{dialect} braced {braced} quoted {quoted}"
            );
        }
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert_eq!(
                selection_of("switch", dialect, &words, None, body()),
                decided,
                "{dialect} braced {braced} quoted {quoted}"
            );
        }
        // A member that selects the next arm directly never reads it.
        let direct = ["-glob", "--", "b", "a", "-", "b", "B"];
        assert_eq!(
            selection_of("switch", "tcl9.1", &direct, None, body()),
            Ok((vec![Some(1)], vec![Some(1)], vec![Vec::new()]))
        );
    }
    // A bare `-` reads alike on both paths.
    for dialect in ["tcl9.1", "tcl", "tcl8.6"] {
        assert_eq!(
            selection_of(
                "switch",
                dialect,
                &words,
                None,
                vec![(4, delimited("-", false, false))]
            ),
            decided,
            "{dialect}"
        );
    }
    // A word with a substitution sends the command to the interpreted path.
    let substituted = WordStructure {
        braced: false,
        quoted: true,
        parts: vec![WordPart::VariableRead {
            span: tcl_lexer::Span::new(1, 3),
            name: "d".to_owned(),
            element: None,
        }],
    };
    assert_eq!(
        selection_of(
            "switch",
            "tcl9.1",
            &["-glob", "--", "a", "a", "\"$d\"", "b", "B"],
            Some((4, FactView::Exact(ExactValue::from_literal("-"), None))),
            vec![(4, substituted)]
        ),
        decided
    );
    // The one-word form's elements are read by content on both paths, and
    // a body whose structure is not given is not known to be bare.
    assert_eq!(
        selection_of(
            "switch",
            "tcl9.1",
            &["-glob", "--", "a", "a {-} b B"],
            None,
            Vec::new()
        ),
        decided
    );
    assert_eq!(
        selection_of("switch", "tcl9.1", &words, None, Vec::new()),
        ambiguous
    );
}

/// `case` declares its own selection contract: no options, glob
/// matching, a pattern word holding whitespace or a backslash a list of
/// patterns, a `default` fallback wherever it stands and still matched
/// literally, the first match winning, and no fall-through body. Each
/// answer is tclsh 8.4.20 to 8.6.18's (`case_witnesses_match_every_release_on_path`);
/// `case` is not a command from 9.0.
#[test]
fn case_selection_runs_tcl_case_obj_cmd() {
    let case = |words: &[&str]| selection_of("case", "tcl8.6", words, None, Vec::new());
    let arm = |arm: usize| Ok((vec![Some(arm)], vec![Some(arm)], vec![Vec::new()]));
    assert_eq!(case(&["abc", "in", "a*", "Y", "default", "N"]), arm(0));
    assert_eq!(case(&["abc", "a*", "Y", "default", "N"]), arm(0));
    assert_eq!(case(&["abc", "a* Y default N"]), arm(0));
    assert_eq!(case(&["zzz", "in", "a*", "Y", "default", "N"]), arm(1));
    assert_eq!(case(&["abc", "in", "abc", "X"]), arm(0));
    assert_eq!(case(&["abc", "in", "x a*", "L", "default", "N"]), arm(0));
    assert_eq!(case(&["abc", "in", "a\\*", "E", "default", "N"]), arm(0));
    assert_eq!(case(&["zzz", "in", "default", "D", "a", "A"]), arm(0));
    assert_eq!(
        case(&["default", "in", "default", "D", "def*", "X"]),
        arm(0)
    );
    assert_eq!(case(&["abc", "in", "-", "D", "default", "N"]), arm(1));
    assert_eq!(
        case(&["abc", "in", "q", "Q"]),
        Ok((vec![None], vec![None], vec![Vec::new()]))
    );
    // A word after the subject whose value is the separator is one the
    // command skips: the plan read it as a pattern, so the fact declines.
    assert_eq!(
        selection_of(
            "case",
            "tcl8.6",
            &["abc", "$w", "a*", "Y"],
            Some((1, FactView::Exact(ExactValue::from_literal("in"), None))),
            Vec::new()
        ),
        Err(DeclineReason::Unsupported)
    );
}
