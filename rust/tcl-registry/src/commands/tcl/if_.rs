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

//! `if` — conditional execution with optional elseif/else clauses.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

/// Result of one pass over an `if` invocation's argument words: the
/// per-word [`ArgRole`] assignments [`if_layout_roles`] returns, and — for
/// the compiler's E004 diagnostic — the first structural defect, if any.
/// Walking the grammar exactly once for both consumers is what keeps
/// them from drifting apart the way two independent re-implementations
/// eventually would.
struct IfWalk {
    roles: Vec<(u8, ArgRole)>,
    error: Option<ClauseShapeError>,
    repair: Option<ClauseShapeRepair>,
}

/// The slots of every condition clause: `expr ?then? body`.
const CONDITION_CLAUSE: &[ClauseSlot] = &[
    ClauseSlot::of(ArgRole::Expr),
    ClauseSlot::noise("then"),
    ClauseSlot::of(ArgRole::Body),
];

/// The final clause's one script word.
const FINAL_CLAUSE: &[ClauseSlot] = &[ClauseSlot::of(ArgRole::Body)];

/// `expr ?then? body (elseif expr ?then? body)* (?else? body)?`, the grammar
/// C Tcl's `Tcl_IfObjCmd` accepts — verified word for word against
/// `TclNRIfObjCmd` / `IfConditionCallback` in Tcl 9.0.4's
/// `generic/tclCmdIL.c` and cross-checked against tclsh 8.6 (the same
/// algorithm since at least Tcl 8.4).
///
/// Two properties fall out of declaring the real grammar rather than
/// keyword-matching every position:
///
/// - A condition slot (the head's, or the word right after `elseif`) is
///   *never* keyword-matched — `if else {a}` and `if elseif {a}` are
///   structurally well-formed `if`s whose single condition happens to be
///   the bareword `else` / `elseif`; real Tcl evaluates it as a boolean
///   expression and fails there (an invalid-bareword error), not as a
///   malformed `if`. Only `then` (right after a fresh condition) and
///   `elseif` / `else` (once a clause is complete) are ever
///   keyword-matched, matching `IfConditionCallback` exactly.
/// - Once a clause completes, at most one more bare word is accepted as
///   the implicit-else body; anything past it is `ExtraWords`, matching
///   Tcl's `wrong # args: extra words after "else" clause` — whether or
///   not an explicit `else` keyword introduced that body.
fn walk_if(args: &[&str]) -> IfWalk {
    walk_if_arguments(crate::InvocationArguments::literals(args)).expect("literal grammar controls")
}

fn walk_if_arguments(args: crate::InvocationArguments<'_>) -> Option<IfWalk> {
    let plan = GRAMMAR.walk_arguments(args, &[], None)?.ok()?;
    let issue = plan.shape_issue();
    Some(IfWalk {
        roles: plan
            .roles
            .iter()
            .filter_map(|&(index, role)| u8::try_from(index).ok().map(|index| (index, role)))
            .collect(),
        error: issue.map(ClauseShapeIssue::error),
        repair: issue.and_then(ClauseShapeIssue::repair),
    })
}

pub const GRAMMAR: ClauseGrammarSpec = ClauseGrammarSpec {
    head: ClauseRow::head(CONDITION_CLAUSE, ClauseTiming::Selected),
    rows: &[ClauseRow::repeated(
        "elseif",
        CONDITION_CLAUSE,
        ClauseTiming::Selected,
    )],
    tail: Some(
        ClauseRow::once(Some("else"), FINAL_CLAUSE, ClauseTiming::Selected).optional_keyword(),
    ),
    fallthrough_body: None,
    // The final body runs when no condition held, and nothing may follow it.
    default_clause: Some(DefaultClause {
        row: None,
        final_only: true,
    }),
    selection: ClauseSelection::FirstMatch,
    surface: None,
};

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?",
    ..FormSpec::DEFAULT
}];

/// Dynamic arg role resolver for `if`/`elseif`/`else` chains.
///
/// Walks the argument list recognising `then`, `elseif`, `else`
/// keywords and classifying each positional argument as either
/// `Expr` (conditions) or `Body` (scripts). The structural keyword
/// words themselves (`then`/`elseif`/`else`) carry `ArgRole::Keyword`
/// so the semantic-token layer highlights them as keywords rather
/// than strings. Delegates to [`walk_if`] — the same grammar walk
/// that drives [`check_if_shape`], so highlighting and the E004
/// diagnostic can never disagree about where a clause starts.
fn if_layout_roles(
    args: crate::InvocationArguments<'_>,
    _options: crate::resolved_invocation::InvocationOptions<'_, '_>,
) -> Option<Vec<(u8, ArgRole)>> {
    let count = args.exact_argv_len()?;
    if count > usize::from(u8::MAX) + 1 {
        return None;
    }
    Some(walk_if_arguments(args)?.roles)
}

/// Validate an `if` invocation's structural shape.
///
/// Runs the same clause-chain walk [`if_layout_roles`] uses for
/// highlighting and returns the first defect [`walk_if`] finds, or
/// `None` for any shape C Tcl accepts — including a bare leading
/// `else` / `elseif` (`if else {a}`): see [`walk_if`]'s doc comment for
/// why that is structurally well-formed rather than a malformed `if`.
pub(crate) fn check_if_shape(args: &[&str]) -> Option<ClauseShapeError> {
    check_if_shape_words(crate::InvocationArguments::literals(args)).map(ClauseShapeIssue::error)
}

/// Validate the selected clause grammar without inventing unknown values.
/// Dynamic conditions and bodies retain cardinality; unknown control words
/// cannot select a branch of the grammar.
fn check_if_shape_words(args: crate::InvocationArguments<'_>) -> Option<ClauseShapeIssue> {
    // naming.diagnostic.original-control-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
    let walk = walk_if_arguments(args)?;
    Some(ClauseShapeIssue::with_repair(walk.error?, walk.repair))
}

/// BPF syntax uses the same clause grammar, independently of Tcl execution
/// admission. Every returned position addresses an original argument word.
pub(crate) fn bpf_conditional_operands(args: &[&str]) -> Option<Vec<(Option<usize>, usize)>> {
    // The shared role carrier cannot represent positions beyond this bound.
    if args.len() > usize::from(u8::MAX) {
        return None;
    }
    let walk = walk_if(args);
    if walk.error.is_some() {
        return None;
    }
    let mut condition = None;
    let mut clauses = Vec::new();
    for (position, role) in walk.roles {
        match role {
            ArgRole::Expr => condition = Some(usize::from(position)),
            ArgRole::Body => clauses.push((condition.take(), usize::from(position))),
            _ => {}
        }
    }
    Some(clauses)
}

/// Present an authored C Tcl 8.4 compiler clause defect. Expression words
/// retain their source delimiters; grammar keywords use their decoded value.
pub(crate) fn native_compile_shape_message(
    arguments: &[&str],
    written_head: &str,
    spellings: &[&str],
) -> Option<String> {
    let walk = walk_if(arguments);
    let (prefix, token) = match walk.error? {
        ClauseShapeError::MissingExpr { after } => (
            "wrong # args: no expression after",
            after.map_or(Some(written_head), |index| arguments.get(index).copied())?,
        ),
        ClauseShapeError::MissingBody { after } => {
            let expression = walk
                .roles
                .iter()
                .any(|(index, role)| usize::from(*index) == after && *role == ArgRole::Expr);
            (
                "wrong # args: no script following",
                if expression {
                    spellings.get(after).copied()?
                } else {
                    arguments.get(after).copied()?
                },
            )
        }
        ClauseShapeError::ExtraWords { .. } => {
            return Some(
                "wrong # args: extra words after \"else\" clause in \"if\" command".into(),
            );
        }
    };
    let mut end = token.len().min(50);
    while !token.is_char_boundary(end) {
        end -= 1;
    }
    Some(format!("{prefix} \"{}\" argument", &token[..end]))
}

/// Command spec for `if`.
///
/// Synopsis, grammar, and semantics are identical across Tcl 8.4, 8.5, 8.6,
/// 9.0, and 9.1: the if(n) manpage's SYNOPSIS, DESCRIPTION, and EXAMPLES
/// sections are word-for-word identical on all five version trees (fetched
/// and line-diffed directly, not paraphrased) — no wording, grammar,
/// option, or semantics change anywhere in the range. The five pages
/// differ only in cosmetic typesetting, split at the same 8.5/8.6
/// boundary: 8.4/8.5 use an ASCII troff-quoted "noise words" phrase and a
/// hyphen in the NAME line, where 8.6+ use a Unicode-quoted "noise words"
/// phrase and an em dash; and 8.4/8.5 render the EXAMPLES code blocks at 3-space indent
/// with the final multi-line-expression example kept on one line, where
/// 8.6+ use 4-space indent and wrap that same expression's `||` operators
/// one per line. `if` has never taken an option, never gained or lost a
/// form, and has no version-gated behaviour to model here.
static BPF_CONDITIONAL: crate::bpf_op::BpfOpSpec =
    crate::bpf_op::BpfOpSpec::structural(crate::bpf_op::BpfOpKind::Conditional);

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "if",
        bpf_op: Some(&BPF_CONDITIONAL),
        successful_handler: Some(crate::native_compilation::SuccessfulHandlerSpec::PossibleBodies),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::Conditional,
            operation: crate::SemanticOperationId::StructuredLowering(
                crate::hooks::LoweringHookId::If,
            ),
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("if"),
        // Present and unrestricted everywhere. `if` is a pure control-flow
        // keyword with no filesystem/process/network access, so its surface
        // carries an iRules row explicitly (`ALL_TCL.union(IRULES)`) and it
        // resolves under the bare `IRULES` mask, the same as under every other
        // dialect that hosts a real Tcl core (iapps, tmsh, the EDA shells,
        // expect, tk). iRules availability is fully explicit per spec now —
        // there is no `disabled_commands` list for a command to be absent
        // from.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_BOOLEAN_COND
            | Traits::NEVER_INLINE_BODY
            | Traits::BRANCH_SELECTED_BODY
            | Traits::STRUCTURALLY_CHECKED_ARITY,
        // The floor here is purely descriptive (hover / hint text): the real
        // minimum is enforced by the clause grammar's walk, which also covers
        // the `elseif`/`else` chain shape a plain range can't express.
        arity: Arity::at_least(2),
        arg_role_layout_resolver: Some(if_layout_roles),
        arg_role_resolver_roles: &[ArgRole::Expr, ArgRole::Body, ArgRole::Keyword],
        clause_shape_check: Some(check_if_shape_words),
        clause_grammar: Some(&GRAMMAR),
        lowering_hook: Some(crate::hooks::LoweringHookId::If),
        native_lowering: Some(NativeLowering::Structured(crate::hooks::LoweringHookId::If)),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Boolean),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Conditional execution with optional elseif/else branches.",
            synopsis: &[
                "if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?",
                "if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else bodyN?",
            ],
            snippet: "Each expr is evaluated left to right, the same way expr evaluates its argument, until one is true; that clause's body runs and no later expr or body is touched. `then` and `else` are optional noise words kept only for readability — `if {$x} then {body}` and `if {$x} {body}` are equivalent. A boolean value is either numeric (0 is false, anything else is true) or one of the strings true/yes/false/no. Any number of elseif clauses may appear, including none, and the final body may be introduced with `else` or left bare with no keyword at all; an `else` with no body is an error, but a bare trailing body needs no `else` to be recognised. With no true expr and no final body, `if` returns an empty string.",
            source: "Tcl if(n)",
            examples: "if {$vbl == 1} {\n    puts \"vbl is one\"\n} elseif {$vbl == 2} {\n    puts \"vbl is two\"\n} else {\n    puts \"vbl is not one or two\"\n}",
            return_value: "The result of whichever body script ran, or an empty string if no expr was true and no final body was given.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every case here was cross-checked against tclsh 8.6 and against
    /// Tcl 9.0.4's `TclNRIfObjCmd` / `IfConditionCallback` source
    /// (`generic/tclCmdIL.c`), which implement the identical structural
    /// walk.  TN cases (well-formed shapes) and FN-shaped cases
    /// (structurally valid shapes that fail for an unrelated reason at
    /// runtime — an invalid-bareword condition) are `None`; TP cases
    /// (genuinely malformed shapes) are `Some`.
    fn shape(src: &str) -> Option<ClauseShapeError> {
        let words: Vec<&str> = src.split_whitespace().collect();
        check_if_shape(&words)
    }

    #[test]
    fn empty_args_is_missing_expr_with_no_word() {
        assert_eq!(
            shape(""),
            Some(ClauseShapeError::MissingExpr { after: None })
        );
    }

    #[test]
    fn condition_only_is_missing_body_after_condition() {
        // `if {1}`
        assert_eq!(shape("1"), Some(ClauseShapeError::MissingBody { after: 0 }));
    }

    #[test]
    fn condition_then_is_missing_body_after_then() {
        // `if {1} then`
        assert_eq!(
            shape("1 then"),
            Some(ClauseShapeError::MissingBody { after: 1 })
        );
    }

    #[test]
    fn condition_body_is_well_formed() {
        assert_eq!(shape("1 a"), None);
    }

    #[test]
    fn bpf_conditionals_share_exact_clause_positions_and_shape_rejection() {
        assert_eq!(
            bpf_conditional_operands(&["else", "then", "A", "elseif", "test", "B", "C"]),
            Some(vec![(Some(0), 2), (Some(4), 5), (None, 6)]),
        );
        assert_eq!(bpf_conditional_operands(&["test", "then"]), None);
        assert_eq!(
            bpf_conditional_operands(&["test", "A", "else", "B", "extra"]),
            None
        );
    }

    #[test]
    fn condition_then_body_is_well_formed() {
        assert_eq!(shape("1 then a"), None);
    }

    #[test]
    fn implicit_else_single_body_is_well_formed() {
        // `if {1} {a} {b}`
        assert_eq!(shape("1 a b"), None);
    }

    #[test]
    fn implicit_else_extra_words_is_extra_words_at_first_extra() {
        // `if {1} {a} {b} {c}` — "c" (index 3) is the first extra word.
        assert_eq!(
            shape("1 a b c"),
            Some(ClauseShapeError::ExtraWords { first_extra: 3 })
        );
    }

    #[test]
    fn implicit_else_many_extra_words_still_anchors_first_extra() {
        // `if {1} {a} {b} {c} {d}`
        assert_eq!(
            shape("1 a b c d"),
            Some(ClauseShapeError::ExtraWords { first_extra: 3 })
        );
    }

    #[test]
    fn explicit_else_body_is_well_formed() {
        assert_eq!(shape("1 a else b"), None);
    }

    #[test]
    fn else_without_body_is_missing_body_after_else() {
        // `if {1} {a} else`
        assert_eq!(
            shape("1 a else"),
            Some(ClauseShapeError::MissingBody { after: 2 })
        );
    }

    #[test]
    fn else_body_extra_words_is_extra_words_at_first_extra() {
        // `if {1} {a} else {b} {c}`
        assert_eq!(
            shape("1 a else b c"),
            Some(ClauseShapeError::ExtraWords { first_extra: 4 })
        );
    }

    #[test]
    fn elseif_chain_no_else_is_well_formed() {
        assert_eq!(shape("a x elseif b y"), None);
    }

    #[test]
    fn elseif_chain_with_else_is_well_formed() {
        assert_eq!(shape("a x elseif b y else z"), None);
    }

    #[test]
    fn elseif_without_expr_is_missing_expr_after_elseif() {
        // `if {1} {a} elseif`
        assert_eq!(
            shape("1 a elseif"),
            Some(ClauseShapeError::MissingExpr { after: Some(2) })
        );
    }

    #[test]
    fn elseif_condition_without_body_is_missing_body_after_condition() {
        // `if {1} {a} elseif {2}`
        assert_eq!(
            shape("1 a elseif 2"),
            Some(ClauseShapeError::MissingBody { after: 3 })
        );
    }

    #[test]
    fn elseif_then_without_body_is_missing_body_after_then() {
        // `if {1} {a} elseif {2} then`
        assert_eq!(
            shape("1 a elseif 2 then"),
            Some(ClauseShapeError::MissingBody { after: 4 })
        );
    }

    #[test]
    fn elseif_chain_then_implicit_else_is_well_formed() {
        // `if {1} {a} elseif {2} {b} {c}` — an elseif clause followed by
        // a bare implicit-else body is well-formed (verified against
        // tclsh 8.6: runs body "a", not a wrong-#args error).
        assert_eq!(shape("1 a elseif 2 b c"), None);
    }

    #[test]
    fn elseif_chain_then_implicit_else_extra_words() {
        // `if {1} {a} elseif {2} {b} {c} {d}` — "d" (index 6) is the
        // first word to trail the implicit-else body.
        assert_eq!(
            shape("1 a elseif 2 b c d"),
            Some(ClauseShapeError::ExtraWords { first_extra: 6 })
        );
    }

    // -- Leading `else`/`elseif`: the condition slot is never
    // keyword-matched, so these are structurally well-formed (the
    // bareword condition fails at expression-evaluation time instead —
    // a distinct, unrelated problem this check does not own).

    #[test]
    fn leading_else_is_well_formed_condition_bareword() {
        // `if else {a}` — "else" is just the (ill-typed) condition text.
        assert_eq!(shape("else a"), None);
    }

    #[test]
    fn leading_elseif_is_well_formed_condition_bareword() {
        // `if elseif {a}`
        assert_eq!(shape("elseif a"), None);
    }

    #[test]
    fn else_as_elseif_condition_is_well_formed() {
        // `if {1} {a} elseif else {b}` — "else" sits in the *elseif's
        // condition* slot, never keyword-matched there either.
        assert_eq!(shape("1 a elseif else b"), None);
    }

    #[test]
    fn then_as_implicit_else_body_with_trailing_word_is_extra_words() {
        // `if {1} {a} then {b}` — "then" is only ever keyword-matched
        // right after a *fresh* condition; here it is the implicit-else
        // body candidate, and "b" (index 3) is the extra word.
        assert_eq!(
            shape("1 a then b"),
            Some(ClauseShapeError::ExtraWords { first_extra: 3 })
        );
    }

    #[test]
    fn else_as_implicit_else_body_is_well_formed() {
        // `if {1} {a} else then` — "then" here is just the else-branch
        // body text, not a keyword.
        assert_eq!(shape("1 a else then"), None);
    }

    #[test]
    fn if_layout_roles_matches_walk_if_roles() {
        // Literal shape checks and structured role projection share the same
        // keyword-selecting grammar; payload bytes are not grammar controls.
        for src in [
            "1 a",
            "1 then a",
            "1 a else b",
            "a x elseif b y else z",
            "1 a b c",
            "else a",
        ] {
            let words: Vec<&str> = src.split_whitespace().collect();
            assert_eq!(
                walk_if_arguments(crate::InvocationArguments::literals(&words))
                    .unwrap()
                    .roles,
                walk_if(&words).roles,
                "src={src:?}"
            );
        }
    }
}

#[cfg(test)]
mod original_clause_repair_tests {
    use super::*;

    #[test]
    fn original_if_roles_keep_opaque_payloads_and_decline_unknown_controls() {
        // naming.cli.original-source-highlight-geometry
        // docs/design/analysis/name-resolution-proofs/cli-original-source-highlight-geometry.md
        for body in [&b"puts \xff"[..], &b"puts \xed\xa0\xbd\xed\xb9\x82"[..]] {
            let words = [
                crate::InvocationWord::KnownBytes(b"1"),
                crate::InvocationWord::KnownBytes(body),
            ];
            let args = crate::InvocationArguments::structured(&words);
            let walk = walk_if_arguments(args).unwrap();
            assert_eq!(walk.roles, [(0, ArgRole::Expr), (1, ArgRole::Body)]);
            assert_eq!(walk.error, None);
            let words = [
                crate::InvocationWord::KnownBytes(b"1"),
                crate::InvocationWord::KnownBytes(b"then"),
                crate::InvocationWord::KnownBytes(body),
                crate::InvocationWord::KnownBytes(b"else"),
                crate::InvocationWord::KnownBytes(body),
                crate::InvocationWord::KnownBytes(b"extra"),
            ];
            let args = crate::InvocationArguments::structured(&words);
            let issue = check_if_shape_words(args).unwrap();
            assert_eq!(
                issue.error(),
                ClauseShapeError::ExtraWords { first_extra: 5 }
            );
            assert_eq!(
                issue.repair(),
                Some(ClauseShapeRepair::MergeTrailingWords { body: 4 })
            );
        }
        for control in [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::KnownBytes(b"then\0tail"),
        ] {
            let words = [crate::InvocationWord::Literal("1"), control];
            let args = crate::InvocationArguments::structured(&words);
            assert!(walk_if_arguments(args).is_none());
        }
        let words = [
            crate::InvocationWord::Literal("1"),
            crate::InvocationWord::Expanded,
        ];
        let args = crate::InvocationArguments::structured(&words);
        assert_eq!(args.exact_argv_len(), None);
    }

    #[test]
    fn original_clause_repairs_distinguish_conditions_from_opening_keywords() {
        // naming.diagnostic.original-control-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
        let issue = check_if_shape_words(crate::InvocationArguments::literals(&[
            "1", "puts ok", "elseif", "elseif",
        ]))
        .unwrap();
        assert_eq!(issue.error(), ClauseShapeError::MissingBody { after: 3 });
        assert_eq!(
            issue.repair(),
            Some(ClauseShapeRepair::RemoveTrailingClause { keyword: 2 })
        );
        let issue = check_if_shape_words(crate::InvocationArguments::literals(&[
            "1", "puts ok", "else", "body", "trailing",
        ]))
        .unwrap();
        assert_eq!(
            issue.error(),
            ClauseShapeError::ExtraWords { first_extra: 4 }
        );
        assert_eq!(
            issue.repair(),
            Some(ClauseShapeRepair::MergeTrailingWords { body: 3 })
        );
        let unanchored = ClauseShapeIssue::diagnostic(issue.error());
        assert_eq!(unanchored.error(), issue.error());
        assert_eq!(unanchored.repair(), None);
    }
}
