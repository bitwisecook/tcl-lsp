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

//! Shared operator grammar and the F5 predicate adapters.

use super::ast::BinOp;
#[cfg(test)]
use super::ast::UnaryOp;
pub use tcl_expr::operators::*;

/// Appliance contexts reached by the exact BIG-IP 21.1.0.1 build 0.0.26
/// expression payload. This observation supplies no runtime activation capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeasuredBigIpExpressionObservation {
    /// Dynamically evaluated expression in the `HTTP_REQUEST` event.
    TmmHttpRequest,
    /// A tmsh CLI script.
    TmshCliScript,
    /// An iApp implementation action; presentation/APL remains separate.
    IAppImplementationAction,
    /// A script reached through a triggered iCall handler.
    TriggeredICallScript,
}

/// Explicit authored F5 expression semantics, separate from native C/Jim
/// evaluation and from the measured appliance's build/context receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthoredF5StringPredicateProvider {
    /// The F5 word grammar over checked string operands.
    F5Trunk,
}

/// Shared string operation after independent operand and dialect admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum F5StringPredicate {
    /// Case-sensitive substring containment.
    Contains,
    /// Case-sensitive prefix comparison.
    StartsWith,
    /// Case-sensitive suffix comparison.
    EndsWith,
    /// Case-sensitive, whole-string glob matching; subject precedes pattern.
    Glob,
}

impl AuthoredF5StringPredicateProvider {
    /// Select a pure recipe after the caller admits the authored F5 surface.
    /// This grants no native compiler, native getter or appliance capability.
    #[must_use]
    pub const fn predicate(self, operator: BinOp) -> Option<F5StringPredicate> {
        match (self, operator) {
            (Self::F5Trunk, BinOp::Contains) => Some(F5StringPredicate::Contains),
            (Self::F5Trunk, BinOp::StartsWith) => Some(F5StringPredicate::StartsWith),
            (Self::F5Trunk, BinOp::EndsWith) => Some(F5StringPredicate::EndsWith),
            (Self::F5Trunk, BinOp::Matches | BinOp::MatchesGlob) => Some(F5StringPredicate::Glob),
            (Self::F5Trunk, _) => None,
        }
    }
}

impl MeasuredBigIpExpressionObservation {
    /// Measured bare `matches` discriminators in this exact build/context.
    /// This is observational metadata, not a live expression policy.
    #[must_use]
    pub const fn bare_matches(self) -> F5StringPredicate {
        match self {
            Self::TmmHttpRequest
            | Self::TmshCliScript
            | Self::IAppImplementationAction
            | Self::TriggeredICallScript => F5StringPredicate::Glob,
        }
    }
}

impl F5StringPredicate {
    /// Evaluate admitted checked operands. The right operand is the pattern.
    #[must_use]
    pub fn evaluate(self, subject: &str, operand: &str) -> bool {
        match self {
            Self::Contains => subject.contains(operand),
            Self::StartsWith => subject.starts_with(operand),
            Self::EndsWith => subject.ends_with(operand),
            Self::Glob => crate::glob::string_match(operand, subject),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::SpecSurface;

    #[test]
    fn measured_bigip_bare_matches_is_glob_in_exact_reached_contexts() {
        for scope in [
            MeasuredBigIpExpressionObservation::TmmHttpRequest,
            MeasuredBigIpExpressionObservation::TmshCliScript,
            MeasuredBigIpExpressionObservation::IAppImplementationAction,
            MeasuredBigIpExpressionObservation::TriggeredICallScript,
        ] {
            for row in include_str!("../../tests/data/f5-matches-21.1.0.1-0.0.26.tsv")
                .lines()
                .take(6)
            {
                let fields = row.split('\t').collect::<Vec<_>>();
                let (subject, pattern) = fields[0].split_once(" matches ").unwrap();
                assert_eq!(
                    scope.bare_matches().evaluate(
                        &subject[1..subject.len() - 1],
                        &pattern[1..pattern.len() - 1]
                    ),
                    fields[2] == "1"
                );
            }
        }
    }

    /// Adding a `BinOp` variant without extending this match is a compile
    /// error — the actual exhaustiveness guard. The length assert on
    /// `ALL_BIN_OPS` is the paired reminder to also extend the array (and
    /// `BinOp::spec()`) in the same change.
    #[test]
    fn all_bin_ops_lists_every_variant() {
        const fn exhaustive(op: BinOp) {
            match op {
                BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Mod
                | BinOp::Pow
                | BinOp::LShift
                | BinOp::RShift
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
                | BinOp::And
                | BinOp::Or
                | BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::StrEq
                | BinOp::StrNe
                | BinOp::StrLt
                | BinOp::StrLe
                | BinOp::StrGt
                | BinOp::StrGe
                | BinOp::In
                | BinOp::Ni
                | BinOp::WordAnd
                | BinOp::WordOr
                | BinOp::Contains
                | BinOp::StartsWith
                | BinOp::EndsWith
                | BinOp::StrEquals
                | BinOp::Matches
                | BinOp::MatchesGlob
                | BinOp::MatchesRegex => {}
            }
        }
        for &op in ALL_BIN_OPS {
            exhaustive(op);
        }
        assert_eq!(ALL_BIN_OPS.len(), 36);
    }

    /// See [`all_bin_ops_lists_every_variant`] — same guard for `UnaryOp`'s
    /// 5 variants.
    #[test]
    fn all_unary_ops_lists_every_variant() {
        const fn exhaustive(op: UnaryOp) {
            match op {
                UnaryOp::Neg | UnaryOp::Pos | UnaryOp::BitNot | UnaryOp::Not | UnaryOp::WordNot => {
                }
            }
        }
        for &op in ALL_UNARY_OPS {
            exhaustive(op);
        }
        assert_eq!(ALL_UNARY_OPS.len(), 5);
    }

    #[test]
    fn every_bin_op_spec_is_reachable() {
        // `spec()` itself has no wildcard arm, so this loop alone proves
        // every variant maps to real metadata (a missing arm is a compile
        // error, not a test failure) — this also exercises `as_str()`
        // agreement between the spelling field and the existing method.
        for &op in ALL_BIN_OPS {
            let spec = op.spec();
            assert_eq!(spec.spelling, op.as_str());
        }
        for &op in ALL_UNARY_OPS {
            let spec = op.spec();
            assert_eq!(spec.spelling, op.as_str());
        }
    }

    #[test]
    fn mathop_arity_matches_tclsh_verified_facts() {
        use OperatorShape::{
            Binary, BoolChain, Fold, Membership, NeBinary, PowFold, SubtractOrDivide, Unary,
        };

        // Every shape's derived arity, against the `tclsh9.1` ground truth:
        // `&`, `|`, `^`, `+`, `*`, `**`, `==`, `<`, `eq` all accept 0 args;
        // `-`/`/` require at least 1; `%`/`<<`/`>>`/`!=`/`ne`/`in`/`ni` are
        // always exactly 2; `!`/`~` are always exactly 1.
        assert_eq!(
            Fold { identity: 0 }.command_arity(),
            CommandArity::at_least(0)
        );
        assert_eq!(PowFold.command_arity(), CommandArity::at_least(0));
        assert_eq!(
            BoolChain { string_only: false }.command_arity(),
            CommandArity::at_least(0)
        );
        assert_eq!(
            SubtractOrDivide {
                negate: true,
                reciprocal: false
            }
            .command_arity(),
            CommandArity::at_least(1)
        );
        assert_eq!(Binary.command_arity(), CommandArity::exact(2));
        assert_eq!(
            NeBinary { string_only: false }.command_arity(),
            CommandArity::exact(2)
        );
        assert_eq!(
            Membership { negate: false }.command_arity(),
            CommandArity::exact(2)
        );
        assert_eq!(Unary.command_arity(), CommandArity::exact(1));
    }

    #[test]
    fn tip_461_string_ordering_ops_are_tcl90_plus() {
        for op in [BinOp::StrLt, BinOp::StrLe, BinOp::StrGt, BinOp::StrGe] {
            assert_eq!(op.spec().surface, Some(SpecSurface::TCL90_PLUS));
        }
        // Their 8.4 counterparts are ungated.
        for op in [BinOp::StrEq, BinOp::StrNe] {
            assert_eq!(op.spec().surface, None);
        }
    }

    #[test]
    fn expr_grammar_min_version_matches_the_gated_operators_tclsh_ground_truth() {
        use tcl_dialect::TclVersion;

        // TIP 123/201 (Tcl 8.5): `**`/`in`/`ni` don't parse at all pre-8.5,
        // yet (unlike the TIP-461 four) carry `surface: None` — no
        // *additional* mathop command-table gate beyond the ensemble's own
        // 8.5+ requirement. This is exactly the axis mismatch
        // `expr_grammar_min_version` exists to resolve for a grammar-level
        // consumer (W003) that `surface` alone can't answer.
        for op in [BinOp::Pow, BinOp::In, BinOp::Ni] {
            assert_eq!(op.spec().expr_grammar_min_version, Some(TclVersion::V8_5));
            assert_eq!(
                op.spec().surface,
                None,
                "{op:?}: unchanged from before this field existed"
            );
        }
        // TIP 461 (Tcl 9.0): both axes agree here.
        for op in [BinOp::StrLt, BinOp::StrLe, BinOp::StrGt, BinOp::StrGe] {
            assert_eq!(op.spec().expr_grammar_min_version, Some(TclVersion::V9_0));
        }
        // Base-grammar operators (arithmetic, `==`/`!=`, `eq`/`ne`, …) and
        // the iRules word operators (dialect-*identity* gated, not a
        // version threshold) both read `None`.
        for op in [
            BinOp::Add,
            BinOp::Sub,
            BinOp::Eq,
            BinOp::Ne,
            BinOp::StrEq,
            BinOp::StrNe,
            BinOp::And,
            BinOp::Or,
            BinOp::WordAnd,
            BinOp::Contains,
        ] {
            assert_eq!(op.spec().expr_grammar_min_version, None, "{op:?}");
        }
        for op in ALL_UNARY_OPS {
            assert_eq!(op.spec().expr_grammar_min_version, None, "{op:?}");
        }
    }

    #[test]
    fn irules_word_operators_are_gated_and_have_no_mathop_form() {
        let irules_ops = [
            BinOp::WordAnd,
            BinOp::WordOr,
            BinOp::Contains,
            BinOp::StartsWith,
            BinOp::EndsWith,
            BinOp::StrEquals,
            BinOp::Matches,
            BinOp::MatchesGlob,
            BinOp::MatchesRegex,
        ];
        for op in irules_ops {
            let spec = op.spec();
            assert_eq!(spec.surface, Some(SpecSurface::IRULES));
            assert_eq!(spec.mathop_shape, None);
        }
        let word_not = UnaryOp::WordNot.spec();
        assert_eq!(word_not.surface, Some(SpecSurface::IRULES));
        assert_eq!(word_not.mathop_shape, None);
    }

    #[test]
    fn short_circuit_ops_have_no_mathop_form() {
        for op in [BinOp::And, BinOp::Or] {
            let spec = op.spec();
            assert_eq!(spec.surface, None);
            assert_eq!(spec.mathop_shape, None);
        }
    }

    #[test]
    fn inverse_is_an_involution_for_comparisons() {
        let comparisons = [
            BinOp::Eq,
            BinOp::Ne,
            BinOp::Lt,
            BinOp::Le,
            BinOp::Gt,
            BinOp::Ge,
            BinOp::StrEq,
            BinOp::StrNe,
            BinOp::StrLt,
            BinOp::StrLe,
            BinOp::StrGt,
            BinOp::StrGe,
            BinOp::In,
            BinOp::Ni,
        ];
        for op in comparisons {
            let inv = op
                .inverse()
                .unwrap_or_else(|| panic!("{op:?} has no inverse"));
            assert_eq!(
                inv.inverse(),
                Some(op),
                "{op:?}.inverse().inverse() != {op:?}"
            );
            assert_ne!(inv, op);
        }
    }

    /// The NaN split of [`BinOp::inverse`]: only the four ordered numeric
    /// comparisons carry the precondition, and the rows that do not are pinned
    /// to their exact inverses so a future edit cannot quietly widen either set.
    #[test]
    fn inverse_needs_non_nan_only_for_ordered_numeric_comparisons() {
        for op in [BinOp::Lt, BinOp::Le, BinOp::Gt, BinOp::Ge] {
            assert!(
                op.inverse_needs_non_nan(),
                "{op:?} inverts only under a non-NaN precondition"
            );
        }
        // `==`/`!=` are exact complements even with a NaN operand; the string
        // and membership operators have no NaN rule at all. Their table rows
        // are unconditional, and are pinned here.
        let unconditional = [
            (BinOp::Eq, BinOp::Ne),
            (BinOp::Ne, BinOp::Eq),
            (BinOp::StrEq, BinOp::StrNe),
            (BinOp::StrNe, BinOp::StrEq),
            (BinOp::StrLt, BinOp::StrGe),
            (BinOp::StrLe, BinOp::StrGt),
            (BinOp::StrGt, BinOp::StrLe),
            (BinOp::StrGe, BinOp::StrLt),
            (BinOp::In, BinOp::Ni),
            (BinOp::Ni, BinOp::In),
        ];
        for (op, inv) in unconditional {
            assert!(
                !op.inverse_needs_non_nan(),
                "{op:?} has no NaN rule, so its inverse is unconditional"
            );
            assert_eq!(op.inverse(), Some(inv), "{op:?} inverse table drifted");
        }
    }

    /// An operator with no inverse never claims a NaN precondition — the flag
    /// is meaningless without a table row to guard.
    #[test]
    fn ops_without_an_inverse_claim_no_nan_precondition() {
        for op in ALL_BIN_OPS {
            if op.inverse().is_none() {
                assert!(
                    !op.inverse_needs_non_nan(),
                    "{op:?} has no inverse but claims a NaN precondition"
                );
            }
        }
    }

    #[test]
    fn arithmetic_and_logical_ops_have_no_inverse() {
        for op in [
            BinOp::Add,
            BinOp::Sub,
            BinOp::Mul,
            BinOp::Div,
            BinOp::And,
            BinOp::Or,
            BinOp::Contains,
            BinOp::WordAnd,
        ] {
            assert_eq!(
                op.inverse(),
                None,
                "{op:?} should have no single-operator inverse"
            );
        }
    }

    /// Drift guard for `tcl-lexer`'s hand-typed operator-spelling tables
    /// (`MULTI_OPS`/`is_single_op` in `expr_lexer.rs`; the F5 word
    /// operators are derived there from `tcl_dialect`'s family
    /// `ExprGrammar` word table rather than a lexer-local list).
    /// `tcl-lexer` sits *below* `tcl-syntax` in the dependency graph (see
    /// this crate's own architecture doc comment), so those tables can't
    /// derive from `ALL_BIN_OPS`/`ALL_UNARY_OPS` directly — this proves,
    /// from this (the higher) side of the boundary, that the lexer still
    /// tokenises every operator spelling this module knows about as a
    /// single `Operator` token, catching the day a new `BinOp`/`UnaryOp`
    /// variant lands here without a matching lexer update.
    #[test]
    fn tcl_lexer_recognises_every_operator_spelling_as_one_operator_token() {
        let check = |spelling: &str, surface: Option<&'static [SpecSurface]>| {
            let dialect = (surface == Some(SpecSurface::IRULES)).then_some("f5-irules");
            let tokens = tcl_lexer::tokenise_expr(spelling, dialect);
            assert_eq!(
                tokens.len(),
                1,
                "{spelling:?}: expected exactly one token, got {tokens:?}"
            );
            assert_eq!(
                tokens[0].kind,
                tcl_lexer::ExprTokenType::Operator,
                "{spelling:?}: tcl-lexer didn't tokenise this as an Operator"
            );
            assert_eq!(tokens[0].text, spelling);
        };
        for &op in ALL_BIN_OPS {
            let spec = op.spec();
            check(spec.spelling, spec.surface);
        }
        for &op in ALL_UNARY_OPS {
            let spec = op.spec();
            check(spec.spelling, spec.surface);
        }
    }

    /// Drift guard for `tcl-lexer`'s `expr_math_functions()` — the set
    /// `tcl-lsp-core::semantic_tokens` reads to decide which `Function`-kind
    /// expr tokens get the "known math function" modifier. A name missing
    /// from that set under-classifies its call site: a TIP 521 (9.0) or
    /// TIP 745 (9.1) addition such as `expr {gamma(2.5)}` or
    /// `expr {isfinite($x)}` would go unhighlighted in a 9.1 document.
    #[test]
    fn tcl_lexer_recognises_every_mathfunc_name() {
        let known = tcl_lexer::expr_math_functions();
        for spec in super::super::mathfunc::all() {
            assert!(
                known.contains(spec.name),
                "{}: tcl-lexer's math_functions() doesn't know this name (added_in {:?})",
                spec.name,
                spec.since
            );
        }
    }

    /// Drift guard for [`tcl_dialect::EXPR_WORD_OPERATORS`], which the *lexer*
    /// reads to decide whether a word-shaped spelling is an operator lexeme at
    /// all in a given release. Two tables, two layers, one fact — and they
    /// disagreeing is not a cosmetic problem: the lexer's copy moves the
    /// lexeme *boundary*, so a drift there silently changes what C reports
    /// (`1 lt_ 2` is `invalid bareword "lt_"` under 8.6 and
    /// `invalid character "_"` under 9.0).
    ///
    /// `expr_grammar_min_version` uses `None` for "in the base grammar of
    /// every release we model", which for the oldest modelled release (8.4)
    /// is the same statement as `EXPR_WORD_OPERATORS`' explicit `V8_4`.
    #[test]
    fn the_dialect_word_operator_table_matches_the_operator_specs() {
        for &(spelling, since) in tcl_dialect::EXPR_WORD_OPERATORS {
            let spec = ALL_BIN_OPS
                .iter()
                .map(|op| op.spec())
                .find(|s| s.spelling == spelling)
                .unwrap_or_else(|| panic!("{spelling}: no BinOp has this spelling"));
            let expected = (since != tcl_dialect::TclVersion::V8_4).then_some(since);
            assert_eq!(
                spec.expr_grammar_min_version, expected,
                "{spelling}: tcl-dialect says {since:?}, OperatorSpec says {:?}",
                spec.expr_grammar_min_version
            );
        }
    }

    /// The other direction: every version-gated *word* operator in the specs
    /// must appear in the lexer's table. Without this, adding a word operator
    /// with a version gate to `OperatorSpec` alone would leave the lexer
    /// treating it as a bareword in every release.
    #[test]
    fn every_version_gated_word_operator_is_in_the_dialect_table() {
        for op in ALL_BIN_OPS {
            let spec = op.spec();
            if spec.expr_grammar_min_version.is_none()
                || !spec.spelling.as_bytes()[0].is_ascii_alphabetic()
            {
                continue;
            }
            assert!(
                tcl_dialect::expr_word_operator_since(spec.spelling).is_some(),
                "{}: version-gated word operator missing from tcl_dialect::EXPR_WORD_OPERATORS",
                spec.spelling
            );
        }
    }
}
