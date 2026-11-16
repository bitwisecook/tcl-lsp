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
use crate::InvocationWords;
use tcl_dialect::model::Family;

use ArgRole::{Body, Expr, Keyword};
use ClauseShapeError::{ExtraWords, MissingBody, MissingExpr};

type Roles = &'static [(usize, ArgRole)];

fn shipped(name: &str) -> &'static ClauseGrammarSpec {
    crate::cache::default_registry()
        .get(name)
        .and_then(|spec| spec.clause_grammar)
        .unwrap_or_else(|| panic!("`{name}` declares a clause grammar"))
}

fn shipped_sub(
    name: &str,
    sub: &str,
) -> (&'static ClauseGrammarSpec, &'static [RepeatedArgLayout]) {
    let sub = crate::cache::default_registry()
        .get(name)
        .and_then(|spec| spec.subcommands.iter().find(|found| found.name == sub))
        .unwrap_or_else(|| panic!("`{name} {sub}` is shipped"));
    (
        sub.clause_grammar
            .unwrap_or_else(|| panic!("`{name} {}` declares a clause grammar", sub.name)),
        sub.repeated_args,
    )
}

fn walk(grammar: &ClauseGrammarSpec, call: &str) -> ClausePlan {
    let words: Vec<&str> = call.split_whitespace().collect();
    grammar.walk(&words, &[])
}

/// Every call shape `if_.rs`'s retired walk was tested on — its 23 shape
/// cases and the full chain of the port's matrix — with the roles and the
/// defect that walk answered, captured before it retired. Each case was
/// cross-checked against tclsh 8.6 and Tcl 9.0.4's `TclNRIfObjCmd` /
/// `IfConditionCallback` (`generic/tclCmdIL.c`): well-formed shapes, and
/// shapes that fail only at runtime (an invalid-bareword condition), have
/// no defect.
const IF_CORPUS: &[(&str, Roles, Option<ClauseShapeError>)] = &[
    ("", &[], Some(MissingExpr { after: None })),
    ("1", &[(0, Expr)], Some(MissingBody { after: 0 })),
    (
        "1 then",
        &[(0, Expr), (1, Keyword)],
        Some(MissingBody { after: 1 }),
    ),
    ("1 a", &[(0, Expr), (1, Body)], None),
    ("1 then a", &[(0, Expr), (1, Keyword), (2, Body)], None),
    ("1 a b", &[(0, Expr), (1, Body), (2, Body)], None),
    (
        "1 a b c",
        &[(0, Expr), (1, Body), (2, Body)],
        Some(ExtraWords { first_extra: 3 }),
    ),
    (
        "1 a b c d",
        &[(0, Expr), (1, Body), (2, Body)],
        Some(ExtraWords { first_extra: 3 }),
    ),
    (
        "1 a else b",
        &[(0, Expr), (1, Body), (2, Keyword), (3, Body)],
        None,
    ),
    (
        "1 a else",
        &[(0, Expr), (1, Body), (2, Keyword)],
        Some(MissingBody { after: 2 }),
    ),
    (
        "1 a else b c",
        &[(0, Expr), (1, Body), (2, Keyword), (3, Body)],
        Some(ExtraWords { first_extra: 4 }),
    ),
    (
        "a x elseif b y",
        &[(0, Expr), (1, Body), (2, Keyword), (3, Expr), (4, Body)],
        None,
    ),
    (
        "a x elseif b y else z",
        &[
            (0, Expr),
            (1, Body),
            (2, Keyword),
            (3, Expr),
            (4, Body),
            (5, Keyword),
            (6, Body),
        ],
        None,
    ),
    (
        "1 a elseif",
        &[(0, Expr), (1, Body), (2, Keyword)],
        Some(MissingExpr { after: Some(2) }),
    ),
    (
        "1 a elseif 2",
        &[(0, Expr), (1, Body), (2, Keyword), (3, Expr)],
        Some(MissingBody { after: 3 }),
    ),
    (
        "1 a elseif 2 then",
        &[(0, Expr), (1, Body), (2, Keyword), (3, Expr), (4, Keyword)],
        Some(MissingBody { after: 4 }),
    ),
    (
        "1 a elseif 2 b c",
        &[
            (0, Expr),
            (1, Body),
            (2, Keyword),
            (3, Expr),
            (4, Body),
            (5, Body),
        ],
        None,
    ),
    (
        "1 a elseif 2 b c d",
        &[
            (0, Expr),
            (1, Body),
            (2, Keyword),
            (3, Expr),
            (4, Body),
            (5, Body),
        ],
        Some(ExtraWords { first_extra: 6 }),
    ),
    ("else a", &[(0, Expr), (1, Body)], None),
    ("elseif a", &[(0, Expr), (1, Body)], None),
    (
        "1 a elseif else b",
        &[(0, Expr), (1, Body), (2, Keyword), (3, Expr), (4, Body)],
        None,
    ),
    (
        "1 a then b",
        &[(0, Expr), (1, Body), (2, Body)],
        Some(ExtraWords { first_extra: 3 }),
    ),
    (
        "1 a else then",
        &[(0, Expr), (1, Body), (2, Keyword), (3, Body)],
        None,
    ),
    (
        "1 then a elseif 2 then b else c",
        &[
            (0, Expr),
            (1, Keyword),
            (2, Body),
            (3, Keyword),
            (4, Expr),
            (5, Keyword),
            (6, Body),
            (7, Keyword),
            (8, Body),
        ],
        None,
    ),
];

#[test]
fn counted_clause_walk_uses_values_only_at_comparisons() {
    use crate::{InvocationArguments, InvocationWord};

    // Source grammar projection only: unknown positional payloads retain
    // their ordinals; a compared unknown never acquires a keyword value.
    let grammar = shipped("if");
    let words = [
        InvocationWord::Dynamic,
        InvocationWord::KnownBytes(b"then"),
        InvocationWord::Dynamic,
        InvocationWord::KnownBytes(b"elseif"),
        InvocationWord::KnownBytes(b"elseif"),
        InvocationWord::KnownBytes(b"then"),
        InvocationWord::KnownBytes(b"puts \xff"),
    ];
    let plan = grammar
        .walk_arguments(InvocationArguments::structured(&words), &[], None)
        .unwrap()
        .unwrap();
    assert_eq!(
        plan.roles,
        [
            (0, Expr),
            (1, Keyword),
            (2, Body),
            (3, Keyword),
            (4, Expr),
            (5, Keyword),
            (6, Body)
        ]
    );
    assert_eq!(plan.defect, None);
    for word in [
        InvocationWord::Dynamic,
        InvocationWord::KnownBytes(b"then\0tail"),
    ] {
        let words = [InvocationWord::Literal("1"), word];
        let result = grammar
            .walk_arguments(InvocationArguments::structured(&words), &[], None)
            .unwrap();
        assert_eq!(result.unwrap_err().word, 1);
    }
    let words = [InvocationWord::Literal("1"), InvocationWord::Expanded];
    assert!(
        grammar
            .walk_arguments(InvocationArguments::structured(&words), &[], None)
            .is_none()
    );
    let words = ["1", "body", "elseif", "elseif"];
    let plan = grammar
        .walk_arguments(InvocationArguments::literals(&words), &[], None)
        .unwrap()
        .unwrap();
    let issue = plan.shape_issue().unwrap();
    assert_eq!(issue.error(), MissingBody { after: 3 });
    assert_eq!(
        issue.repair(),
        Some(ClauseShapeRepair::RemoveTrailingClause { keyword: 2 })
    );
}

#[test]
fn if_grammar_agrees_with_the_retired_walk_on_its_corpus() {
    assert_eq!(IF_CORPUS.len(), 24);
    let grammar = shipped("if");
    for (call, roles, defect) in IF_CORPUS {
        let plan = walk(grammar, call);
        assert_eq!(plan.roles, *roles, "roles for `if {call}`");
        assert_eq!(plan.defect, *defect, "defect for `if {call}`");
    }
}

/// Well-formed `try` calls, with the roles the retired `try_arg_roles`
/// answered: a handler's pattern and variable list carry no flat role, and
/// a `-` body carries no `Body`.
const TRY_CORPUS: &[(&[&str], Roles)] = &[
    (
        &[
            "{...}", "on", "ok", "result", "-", "trap", "NONE", "result", "{...}",
        ],
        &[(0, Body), (1, Keyword), (5, Keyword), (8, Body)],
    ),
    (
        &[
            "{...}", "on", "ok", "a", "{-}", "trap", "NONE", "b", "{...}",
        ],
        &[(0, Body), (1, Keyword), (5, Keyword), (8, Body)],
    ),
    (
        &[
            "{...}", "on", "ok", "a", "\"-\"", "trap", "NONE", "b", "{...}",
        ],
        &[(0, Body), (1, Keyword), (5, Keyword), (8, Body)],
    ),
    (
        &["{...}", "on", "error", "msg", "{puts $msg}"],
        &[(0, Body), (1, Keyword), (4, Body)],
    ),
    (
        &["{b}", "on", "ok", "{}", "{h}"],
        &[(0, Body), (1, Keyword), (4, Body)],
    ),
    (
        &["{b}", "trap", "{POSIX ENOENT}", "{}", "{h}"],
        &[(0, Body), (1, Keyword), (4, Body)],
    ),
    (
        &[
            "{b}",
            "trap",
            "{POSIX ENOENT}",
            "{m o}",
            "{h}",
            "finally",
            "{f}",
        ],
        &[(0, Body), (1, Keyword), (4, Body), (5, Keyword), (6, Body)],
    ),
    (
        &["{b}", "finally", "{f}"],
        &[(0, Body), (1, Keyword), (2, Body)],
    ),
    (&["{b}"], &[(0, Body)]),
    (
        &[
            "{b}", "on", "error", "{m o}", "-", "on", "break", "{}", "{h}", "finally", "{f}",
        ],
        &[
            (0, Body),
            (1, Keyword),
            (5, Keyword),
            (8, Body),
            (9, Keyword),
            (10, Body),
        ],
    ),
];

#[test]
fn try_grammar_agrees_with_the_retired_walk() {
    let grammar = shipped("try");
    for (call, roles) in TRY_CORPUS {
        let plan = grammar.walk(call, &[]);
        assert_eq!(plan.roles, *roles, "roles for `try {call:?}`");
        assert_eq!(plan.defect, None, "defect for `try {call:?}`");
    }
    // `on ok`: the handler's pattern selects by completion code; the
    // variable list is a clause fact.
    let plan = grammar.walk(&["{b}", "on", "ok", "{r o}", "{h}"], &[]);
    let handler = &plan.clauses[1];
    assert_eq!(handler.keyword_index, Some(1));
    assert_eq!(handler.timing, ClauseTiming::Selected);
    assert_eq!(
        handler.operands[0].1.handler,
        Some(HandlerMatch::CompletionCode)
    );
    assert_eq!(handler.operands[1].1.role, ArgRole::LoopVarList);
    assert_eq!(plan.clauses[0].timing, ClauseTiming::Protected);
    // `trap {POSIX ENOENT}` selects by errorcode prefix.
    let plan = grammar.walk(&["{b}", "trap", "{POSIX ENOENT}", "{}", "{h}"], &[]);
    assert_eq!(
        plan.clauses[1].operands[0].1.handler,
        Some(HandlerMatch::ErrorCodePrefix)
    );
    // A `-` body runs the next handler's body; `finally` always runs.
    let plan = grammar.walk(
        &[
            "{b}", "on", "error", "{}", "-", "trap", "{X}", "{}", "{h}", "finally", "{f}",
        ],
        &[],
    );
    assert_eq!(plan.clauses[1].falls_through_to, Some(2));
    assert_eq!(plan.clauses[2].falls_through_to, None);
    assert_eq!(plan.clauses[3].row, ClauseRowId::Tail);
    assert_eq!(plan.clauses[3].timing, ClauseTiming::Always);
    assert!(plan.clauses.iter().all(|clause| !clause.is_default));
}

/// The retired scan skipped a word it did not recognise and kept looking
/// for a keyword; the grammar stops there, as the chain does in Tcl
/// (`bad handler type`), and reports where the words stopped making sense.
#[test]
fn a_malformed_try_stops_at_its_first_defect() {
    let grammar = shipped("try");
    let plan = grammar.walk(&["{b}", "foo", "on", "e", "m", "h"], &[]);
    assert_eq!(plan.roles, [(0, Body)]);
    assert_eq!(plan.defect, Some(ExtraWords { first_extra: 1 }));
    let plan = grammar.walk(&["{b}", "on", "x", "y"], &[]);
    assert_eq!(plan.roles, [(0, Body), (1, Keyword)]);
    assert_eq!(plan.defect, Some(MissingBody { after: 3 }));
    let plan = grammar.walk(&["{b}", "finally", "f", "on", "e", "m", "h"], &[]);
    assert_eq!(plan.roles, [(0, Body), (1, Keyword), (2, Body)]);
    assert_eq!(plan.defect, Some(ExtraWords { first_extra: 3 }));
    assert_eq!(
        grammar.walk(&[], &[]).defect,
        Some(MissingBody { after: 0 })
    );
}

/// A marker runs the next *handler's* body: `finally` runs whatever the
/// outcome, so a marker with only `finally` after it has no target —
/// Tcl's "last non-finally clause must not have a body of `-`" — while
/// [`ClausePlan::falls_through`] still names the clause whose body is the
/// marker.
#[test]
fn a_marker_falls_through_to_the_next_handler_never_to_finally() {
    let grammar = shipped("try");
    let plan = grammar.walk(&["{b}", "on", "ok", "{}", "-", "finally", "{f}"], &[]);
    assert_eq!(plan.defect, None);
    assert_eq!(plan.clauses[1].falls_through_to, None);
    assert!(plan.falls_through(1));
    assert!(!plan.falls_through(2), "`finally` is a script");
    assert!(!plan.falls_through(0), "the protected body is a script");
    let plan = grammar.walk(
        &[
            "{b}", "on", "ok", "{}", "-", "on", "error", "{}", "-", "trap", "{X}", "{}", "{h}",
        ],
        &[],
    );
    assert_eq!(plan.clauses[1].falls_through_to, Some(3));
    assert_eq!(plan.clauses[2].falls_through_to, Some(3));
    assert!(plan.falls_through(1) && plan.falls_through(2));
    assert!(!plan.falls_through(3));
    assert!(!plan.falls_through(9), "no such clause");
}

/// The value walk compares the marker exactly: a word whose *value* is
/// `{-}` (the source `"{-}"`) is a script, where a source spelling `{-}`
/// is the braced marker.
#[test]
fn the_value_walk_compares_the_marker_exactly() {
    let grammar = shipped("try");
    let words = |body: &'static str| ["b", "on", "ok", "", body, "trap", "X", "", "h"];
    let plan = grammar
        .walk_words(&words("-"), &[false; 9], &[], None)
        .expect("every word is a value");
    assert_eq!(plan.clauses[1].falls_through_to, Some(2));
    let plan = grammar
        .walk_words(&words("{-}"), &[false; 9], &[], None)
        .expect("every word is a value");
    assert!(!plan.falls_through(1), "the value `{{-}}` is a script");
    assert!(plan.roles.contains(&(4, Body)));
    assert!(grammar.walk(&words("{-}"), &[]).falls_through(1));
    // A computed body word is where the marker is compared: no plan.
    let mut dynamic = [false; 9];
    dynamic[4] = true;
    assert_eq!(grammar.walk_words(&words(""), &dynamic, &[], None), None);
}

#[test]
fn a_resolved_clause_names_its_operands_and_handler() {
    let plan = shipped("try").walk(&["{b}", "on", "ok", "{r o}", "{h}"], &[]);
    let handler = &plan.clauses[1];
    assert_eq!(handler.operand(Body), Some(4));
    assert_eq!(handler.operand(ArgRole::LoopVarList), Some(3));
    assert_eq!(handler.operand(Expr), None);
    assert_eq!(handler.handler(), Some((2, HandlerMatch::CompletionCode)));
    assert_eq!(plan.clauses[0].handler(), None);
    let plan = walk(shipped("if"), "{$c} then {a} else {b}");
    assert_eq!(plan.clauses[0].operand(Expr), Some(0));
    assert_eq!(plan.clauses[0].operand(Keyword), Some(1), "the noise word");
    assert_eq!(plan.clauses[0].operand(Body), Some(2));
    assert_eq!(plan.clauses[1].operand(Body), Some(4));
    assert!(plan.clauses[1].is_default);
}

#[test]
fn every_operand_of_a_role_is_named_in_word_order() {
    // `catch`'s tail has two `LoopVarList` slots: `operands` names both,
    // `operand` the first; a slot the call leaves unfilled names nothing.
    let plan = shipped("catch").walk(&["{s}", "r", "o"], &[]);
    let tail = plan.clauses.last().expect("the result words' clause");
    assert_eq!(
        tail.operands(ArgRole::LoopVarList).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(tail.operand(ArgRole::LoopVarList), Some(1));
    assert_eq!(tail.operands(Body).count(), 0);
    let plan = shipped("catch").walk(&["{s}", "r"], &[]);
    let tail = plan.clauses.last().expect("the result words' clause");
    assert_eq!(tail.operands(ArgRole::LoopVarList).collect::<Vec<_>>(), [1]);
}

#[test]
fn foreach_group_row_cites_layout_zero() {
    let spec = crate::cache::default_registry()
        .get("foreach")
        .expect("foreach is shipped");
    let grammar = spec.clause_grammar.expect("foreach declares a grammar");
    assert_eq!(grammar.rows[0].shape, ClauseRowShape::Group { layout: 0 });
    let layout = spec.repeated_args[0];
    assert_eq!(layout.role, ArgRole::LoopVarList);
    let plan = grammar.walk(&["x", "{a b}", "y", "$l", "{body}"], spec.repeated_args);
    // Two binder groups, then the body; the groups' words are the
    // layout's, so only the body is a flat role.
    assert_eq!(plan.roles, [(4, Body)]);
    assert_eq!(plan.defect, None);
    let groups: Vec<Vec<(usize, ArgRole)>> = plan
        .clauses
        .iter()
        .filter(|clause| clause.row == ClauseRowId::Row(0))
        .map(|clause| {
            clause
                .operands
                .iter()
                .map(|(index, slot)| (*index, slot.role))
                .collect()
        })
        .collect();
    assert_eq!(
        groups,
        [
            vec![(0, ArgRole::LoopVarList), (1, ArgRole::Value)],
            vec![(2, ArgRole::LoopVarList), (3, ArgRole::Value)],
        ]
    );
    assert!(
        plan.clauses
            .iter()
            .all(|clause| clause.timing == ClauseTiming::PerIteration)
    );
    // The retired resolver's rule: the last word is the body once there
    // are three words, and nothing is a body before that — including
    // the off-by-one shapes.
    for (call, body) in [
        ("", None),
        ("x", None),
        ("x l", None),
        ("x l b", Some(2)),
        ("x l y b", Some(3)),
        ("x l y m b", Some(4)),
    ] {
        let words: Vec<&str> = call.split_whitespace().collect();
        let plan = grammar.walk(&words, spec.repeated_args);
        let bodies: Vec<usize> = plan
            .roles
            .iter()
            .filter(|(_, role)| *role == Body)
            .map(|(index, _)| *index)
            .collect();
        assert_eq!(
            bodies,
            body.into_iter().collect::<Vec<_>>(),
            "`foreach {call}`"
        );
        assert_eq!(
            plan.defect.is_none(),
            matches!(call, "x l b" | "x l y m b"),
            "`foreach {call}`"
        );
    }
}

#[test]
fn array_for_is_absent_below_9_0() {
    let registry = crate::cache::default_registry();
    let words = InvocationWords::literals("array", &["for", "{k v}", "a", "{body}"]);
    let at = |release: &str| {
        registry
            .resolve_structured_invocation(words, Some(SurfaceQuery::core(Family::Tcl, release)))
            .resolved()
            .and_then(|invocation| invocation.clause_plan())
    };
    assert!(at("8.6").is_none(), "`array for` is Tcl 9.0");
    let plan = at("9.0").expect("`array for` walks at 9.0");
    // In the invocation's own coordinates: the subcommand word is 0.
    assert_eq!(plan.roles, [(3, Body)]);
    let (grammar, _) = shipped_sub("array", "for");
    assert!(!grammar.available(Some(SurfaceQuery::core(Family::Tcl, "8.6"))));
    assert!(grammar.available(Some(SurfaceQuery::core(Family::Tcl, "9.0"))));
}

#[test]
fn a_keyword_never_matches_inside_a_slot() {
    let grammar = shipped("if");
    // `if else {a}` — the condition is the bareword `else`.
    let plan = walk(grammar, "else a");
    assert_eq!(plan.roles, [(0, Expr), (1, Body)]);
    assert_eq!(plan.defect, None);
    assert_eq!(plan.clauses.len(), 1);
    assert_eq!(plan.clauses[0].keyword_index, None);
    // `elseif`'s own condition slot is positional too.
    let plan = walk(grammar, "1 a elseif else b");
    assert_eq!(plan.clauses[1].operands[0], (3, ClauseSlot::of(Expr)));
    // `then` is noise only right after a condition.
    let plan = walk(grammar, "1 a else then");
    assert_eq!(plan.clauses[1].operands[0].0, 3);
    assert!(plan.clauses[1].is_default, "the else body is the default");
}

#[test]
fn a_dynamic_word_where_a_keyword_could_be_abstains() {
    let grammar = shipped("if");
    let args = ["$c", "{a}", "$k", "{b}"];
    // The condition slot never compares, so a computed condition is fine.
    assert!(
        grammar
            .walk_words(&args[..2], &[true, false], &[], None)
            .is_some()
    );
    // A computed word where `elseif` / `else` could stand is Tcl's call.
    assert!(
        grammar
            .walk_words(&args, &[true, false, true, false], &[], None)
            .is_none()
    );
    let try_grammar = shipped("try");
    assert!(
        try_grammar
            .walk_words(
                &["{b}", "on", "error", "{}", "$h"],
                &[false, false, false, false, true],
                &[],
                None
            )
            .is_none(),
        "a computed handler body could be the `-` marker"
    );
}

#[test]
fn positional_grammars_fill_their_rows_in_declaration_order() {
    let for_grammar = shipped("for");
    let plan = walk(for_grammar, "{set i 0} {$i < 3} {incr i} {body}");
    assert_eq!(plan.roles, [(0, Body), (1, Expr), (2, Body), (3, Body)]);
    let timings: Vec<ClauseTiming> = plan.clauses.iter().map(|clause| clause.timing).collect();
    assert_eq!(
        timings,
        [
            ClauseTiming::LoopFixture(LoopPhase::Init),
            ClauseTiming::Selected,
            ClauseTiming::LoopFixture(LoopPhase::Next),
            ClauseTiming::PerIteration,
        ]
    );
    assert_eq!(
        walk(for_grammar, "a b").defect,
        Some(MissingBody { after: 1 })
    );
    assert_eq!(
        walk(for_grammar, "a").defect,
        Some(MissingExpr { after: Some(0) })
    );
    assert_eq!(
        walk(for_grammar, "a b c d e").defect,
        Some(ExtraWords { first_extra: 4 })
    );
    let while_grammar = shipped("while");
    assert_eq!(walk(while_grammar, "1 b").roles, [(0, Expr), (1, Body)]);
    assert_eq!(
        walk(while_grammar, "1").defect,
        Some(MissingBody { after: 0 })
    );
    let catch = shipped("catch");
    for (call, defect) in [
        ("s", None),
        ("s r", None),
        ("s r o", None),
        ("s r o x", Some(ExtraWords { first_extra: 3 })),
        ("", Some(MissingBody { after: 0 })),
    ] {
        let plan = walk(catch, call);
        assert_eq!(plan.defect, defect, "`catch {call}`");
        // The result and options words are clause facts; `catch`'s own
        // `arg_roles` carries their `VarWrite`.
        assert!(
            plan.roles.iter().all(|(_, role)| *role == Body),
            "`catch {call}`"
        );
    }
}

#[test]
fn dict_update_binds_its_pairs_conditionally() {
    let (grammar, layouts) = shipped_sub("dict", "update");
    let plan = grammar.walk(&["d", "k1", "v1", "k2", "v2", "{body}"], layouts);
    assert_eq!(plan.defect, None);
    assert_eq!(plan.roles, [(5, Body)]);
    let bound: Vec<usize> = plan
        .clauses
        .iter()
        .flat_map(|clause| clause.operands.iter())
        .filter(|(_, slot)| slot.role == ArgRole::LoopVarList)
        .map(|(index, slot)| {
            assert!(slot.conditional_binding);
            *index
        })
        .collect();
    assert_eq!(bound, [2, 4]);
    let body = plan.clauses.last().expect("the body clause");
    assert_eq!(body.timing, ClauseTiming::Always);
}

#[test]
fn the_shipped_clause_words_have_their_owners() {
    for (word, owner) in [
        ("else", "if"),
        ("elseif", "if"),
        ("then", "if"),
        ("on", "try"),
        ("trap", "try"),
        ("finally", "try"),
    ] {
        assert_eq!(owner_of_keyword(word), Some(owner), "`{word}`");
    }
    assert_eq!(owner_of_keyword("set"), None);
    let noise: Vec<&str> = shipped_clause_keywords()
        .iter()
        .filter(|keyword| keyword.noise)
        .map(|keyword| keyword.word)
        .collect();
    assert_eq!(noise, ["then"]);
}

#[test]
fn only_selected_first_match_grammars_evaluate_conditions_once_before_bodies() {
    const AGAIN_SLOTS: &[ClauseSlot] = &[ClauseSlot::of(Expr), ClauseSlot::of(Body)];
    const AGAIN: &[ClauseRow] = &[ClauseRow::repeated(
        "again",
        AGAIN_SLOTS,
        ClauseTiming::PerIteration,
    )];
    assert!(shipped("if").evaluates_conditions_once_before_bodies());
    for name in ["while", "for", "foreach", "try", "catch"] {
        assert!(
            !shipped(name).evaluates_conditions_once_before_bodies(),
            "`{name}`"
        );
    }
    // A first-match grammar is not enough: a loop-timed row re-evaluates
    // its condition after a body has run.
    let looping = ClauseGrammarSpec {
        rows: AGAIN,
        ..*shipped("if")
    };
    assert_eq!(looping.selection, ClauseSelection::FirstMatch);
    assert!(!looping.evaluates_conditions_once_before_bodies());
}
