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
/// per-word [`ArgRole`] assignments [`if_arg_roles`] returns, and — for
/// the compiler's E004 diagnostic — the first structural defect, if any.
/// Walking the grammar exactly once for both consumers is what keeps
/// them from drifting apart the way two independent re-implementations
/// eventually would.
struct IfWalk {
    roles: Vec<(u8, ArgRole)>,
    error: Option<ClauseShapeError>,
}

/// Walk an `if` invocation's argument words against the grammar C Tcl's
/// `Tcl_IfObjCmd` accepts: `expr ?then? body (elseif expr ?then? body)*
/// (else body)?`. Verified word-for-word against `TclNRIfObjCmd` /
/// `IfConditionCallback` in Tcl 9.0.4's `generic/tclCmdIL.c`, and
/// cross-checked against tclsh 8.6 (the same algorithm, unchanged since
/// at least Tcl 8.4).
///
/// Two properties fall out of mirroring the real grammar rather than
/// keyword-matching every position:
///
/// - A condition slot (`args[0]`, or the word right after `elseif`) is
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
    let mut roles = Vec::new();
    let n = args.len();

    let push_role = |roles: &mut Vec<(u8, ArgRole)>, index: usize, role: ArgRole| {
        if let Ok(idx) = u8::try_from(index) {
            roles.push((idx, role));
        }
    };

    if n == 0 {
        return IfWalk {
            roles,
            error: Some(ClauseShapeError::MissingExpr { after: None }),
        };
    }

    // Mandatory first condition — position 0, never keyword-matched.
    let mut i: usize = 0;
    push_role(&mut roles, i, ArgRole::Expr);
    i += 1;
    if i < n && args[i] == "then" {
        push_role(&mut roles, i, ArgRole::Keyword);
        i += 1;
    }
    if i >= n {
        return IfWalk {
            roles,
            error: Some(ClauseShapeError::MissingBody { after: i - 1 }),
        };
    }
    push_role(&mut roles, i, ArgRole::Body);
    i += 1;

    loop {
        if i >= n {
            return IfWalk { roles, error: None };
        }
        if args[i] == "elseif" {
            let kw_idx = i;
            push_role(&mut roles, i, ArgRole::Keyword);
            i += 1;
            if i >= n {
                return IfWalk {
                    roles,
                    error: Some(ClauseShapeError::MissingExpr {
                        after: Some(kw_idx),
                    }),
                };
            }
            // The elseif's condition — position-only, never keyword-matched
            // (see the `if elseif else {a}` case in the doc comment above).
            push_role(&mut roles, i, ArgRole::Expr);
            i += 1;
            if i < n && args[i] == "then" {
                push_role(&mut roles, i, ArgRole::Keyword);
                i += 1;
            }
            if i >= n {
                return IfWalk {
                    roles,
                    error: Some(ClauseShapeError::MissingBody { after: i - 1 }),
                };
            }
            push_role(&mut roles, i, ArgRole::Body);
            i += 1;
            continue;
        }
        if args[i] == "else" {
            let kw_idx = i;
            push_role(&mut roles, i, ArgRole::Keyword);
            i += 1;
            if i >= n {
                return IfWalk {
                    roles,
                    error: Some(ClauseShapeError::MissingBody { after: kw_idx }),
                };
            }
            push_role(&mut roles, i, ArgRole::Body);
            i += 1;
            let error = (i < n).then_some(ClauseShapeError::ExtraWords { first_extra: i });
            return IfWalk { roles, error };
        }
        // Implicit else: the first bare word is the body; anything past it
        // is unreachable in real Tcl and a structural error here.
        push_role(&mut roles, i, ArgRole::Body);
        i += 1;
        let error = (i < n).then_some(ClauseShapeError::ExtraWords { first_extra: i });
        return IfWalk { roles, error };
    }
}

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
fn if_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    walk_if(args).roles
}

/// Validate an `if` invocation's structural shape.
///
/// Runs the same clause-chain walk [`if_arg_roles`] uses for
/// highlighting and returns the first defect [`walk_if`] finds, or
/// `None` for any shape C Tcl accepts — including a bare leading
/// `else` / `elseif` (`if else {a}`): see [`walk_if`]'s doc comment for
/// why that is structurally well-formed rather than a malformed `if`.
fn check_if_shape(args: &[&str]) -> Option<ClauseShapeError> {
    walk_if(args).error
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
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "if",
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
        // minimum is enforced by `clause_shape_check`, which also covers the
        // `elseif`/`else` chain shape a plain range can't express.
        arity: Arity::at_least(2),
        arg_role_resolver: Some(if_arg_roles),
        arg_role_resolver_roles: &[ArgRole::Expr, ArgRole::Body, ArgRole::Keyword],
        clause_shape_check: Some(check_if_shape),
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
    fn if_arg_roles_matches_walk_if_roles() {
        // `if_arg_roles` must be a pure delegation to `walk_if` — assert
        // it, so the two can never drift back apart.
        for src in [
            "1 a",
            "1 then a",
            "1 a else b",
            "a x elseif b y else z",
            "1 a b c",
            "else a",
        ] {
            let words: Vec<&str> = src.split_whitespace().collect();
            assert_eq!(if_arg_roles(&words), walk_if(&words).roles, "src={src:?}");
        }
    }
}
