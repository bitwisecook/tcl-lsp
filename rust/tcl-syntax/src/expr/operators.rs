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

//! `expr` operator metadata — the single source of truth for which
//! [`BinOp`]/[`UnaryOp`] variants exist, which dialects gate them, and how
//! (if at all) they surface as a `::tcl::mathop::*` command.
//!
//! `BinOp`/`UnaryOp` are a **closed grammar production**, unlike math
//! functions (`mathfunc.rs`, open and string-keyed because TIP 232 makes
//! `::tcl::mathfunc::*` a real overridable command table). Metadata attaches
//! via an exhaustive `match` in [`BinOp::spec`]/[`UnaryOp::spec`] — a
//! compile error, not a silent gap, the day a new variant lands without one.
//!
//! [`OperatorShape`] reifies the fold/chain taxonomy `tcl_cmd_core::mathop`'s
//! helper functions (`fold`, `left_fold`, `pow_fold`, `binary`, `bool_chain`,
//! `ne_binary`, `membership`) already implement by hand — one shape per
//! helper. An operator with no `::tcl::mathop` command form at all (`&&`,
//! `||`, every iRules word operator) has `mathop_shape: None`: the parser
//! already accepts it as expr grammar, but no bare/`tcl::mathop::`-prefixed
//! command exists for it (verified against `tclsh` 8.4 through 9.1 — `info
//! commands ::tcl::mathop::*` never lists `&&`, `||`, `and`, `or`, `not`,
//! `contains`, `starts_with`, `ends_with`, `equals`, `matches_glob`, or
//! `matches_regex`, on any version).

use tcl_dialect::DialectSet;

use super::ast::{BinOp, UnaryOp};

/// A command's argument-count contract — deliberately not `tcl_registry::Arity`
/// (`tcl-registry` depends on `tcl-syntax`, never the reverse).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandArity {
    /// Minimum argument count.
    pub min: u8,
    /// Maximum argument count, or `None` for unbounded (variadic).
    pub max: Option<u8>,
}

impl CommandArity {
    /// Exactly `n` arguments.
    #[must_use]
    pub const fn exact(n: u8) -> Self {
        Self {
            min: n,
            max: Some(n),
        }
    }

    /// At least `n` arguments, unbounded above.
    #[must_use]
    pub const fn at_least(n: u8) -> Self {
        Self { min: n, max: None }
    }
}

/// The composition rule a `::tcl::mathop::*` command's spelling follows —
/// one variant per shape helper already implemented (by hand) in
/// `tcl_cmd_core::mathop`. A generic dispatcher switches on this variant
/// alone, never on the operator's name, mirroring
/// `tcl_registry::definer::MemberKind`'s "one shape enum, one generic
/// walker" precedent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorShape {
    /// Left-fold over `0..=N` arguments with an algebraic identity for the
    /// zero-argument and one-argument cases (`+`, `*`, `&`, `|`, `^`).
    Fold {
        /// The fold's identity element (`0` for `+`/`|`/`^`, `1` for `*`,
        /// `-1` — all bits set — for `&`).
        identity: i64,
    },
    /// `-`/`/`'s shared shape: one argument means "apply the single-arg
    /// operation" (negate or reciprocal), two or more means "left-fold the
    /// binary operation". Exactly one of the two flags is ever `true` for a
    /// given operator — they're mutually exclusive readings of the same
    /// one-argument case, not independent switches.
    SubtractOrDivide {
        /// `true` for `-` (one argument negates).
        negate: bool,
        /// `true` for `/` (one argument takes the reciprocal).
        reciprocal: bool,
    },
    /// `**`'s right-associative fold (`2 ** 3 ** 2 == 2 ** (3 ** 2)`), with
    /// the same zero-argument identity (`1`) as `Fold` but a different
    /// associativity, so it can't share that variant.
    PowFold,
    /// A plain two-argument operation with no fold/chain behavior at all
    /// (`%`, `<<`, `>>`) — always exactly 2 arguments.
    Binary,
    /// A plain one-argument operation (`!`, `~`) that shares no command
    /// spelling with any binary operator.
    Unary,
    /// Adjacent-pairs chained comparison (`a < b < c` means `a < b && b <
    /// c`), variadic down to 0/1 arguments (trivially true) — every
    /// comparison operator except `!=`/`ne` (see [`Self::NeBinary`]).
    BoolChain {
        /// `true` for the string-ordering family (`eq`/`lt`/`le`/`gt`/`ge`),
        /// `false` for the numeric family (`==`/`<`/`<=`/`>`/`>=`).
        string_only: bool,
    },
    /// `!=`/`ne`'s shape: unlike every other comparison, chaining has no
    /// unambiguous meaning, so the command is a plain, always-exactly-2-args
    /// binary test — kept distinct from [`Self::Binary`] only so a consumer
    /// can still recognise it as "the negation of a `BoolChain` operator".
    NeBinary {
        /// `true` for `ne` (string), `false` for `!=` (numeric).
        string_only: bool,
    },
    /// List membership (`in`/`ni`), always exactly 2 arguments.
    Membership {
        /// `true` for `ni` (negated membership).
        negate: bool,
    },
}

impl OperatorShape {
    /// The `::tcl::mathop` command's argument-count contract — a pure
    /// function of the shape, since every operator sharing a shape shares
    /// its arity too (verified against `tclsh9.1`: every `Fold`/`PowFold`
    /// command accepts 0+ args, `SubtractOrDivide` commands require at
    /// least 1, and `Binary`/`Unary`/`NeBinary`/`Membership` commands take
    /// exactly the shape's fixed count).
    #[must_use]
    pub const fn command_arity(self) -> CommandArity {
        match self {
            Self::Fold { .. } | Self::PowFold | Self::BoolChain { .. } => CommandArity::at_least(0),
            Self::SubtractOrDivide { .. } => CommandArity::at_least(1),
            Self::Unary => CommandArity::exact(1),
            Self::Binary | Self::NeBinary { .. } | Self::Membership { .. } => {
                CommandArity::exact(2)
            }
        }
    }
}

/// Static facts about one `expr` operator — the `BinOp`/`UnaryOp`
/// counterpart to `mathfunc::MathFuncSpec`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorSpec {
    /// Source-text spelling (identical to `as_str()` — kept as a field so a
    /// consumer can work from the spec alone, mirroring `MathFuncSpec::name`).
    pub spelling: &'static str,
    /// The dialect(s) this operator is gated to, or `None` when it's
    /// available in every dialect since the expr grammar's own base version
    /// (queried separately via `DialectSet::expr_grammar_base_version`).
    pub dialects: Option<DialectSet>,
    /// The `::tcl::mathop::*` command shape this operator shares, or `None`
    /// when the operator has no mathop command form at all (`&&`, `||`,
    /// every iRules word operator).
    pub mathop_shape: Option<OperatorShape>,
    /// A one-line human summary for hover text.
    pub summary: &'static str,
}

/// The `(dialects, mathop_shape, summary)` triple `BinOp::spec()`/
/// `UnaryOp::spec()` each assemble into a full [`OperatorSpec`].
type SpecFacts = (Option<DialectSet>, Option<OperatorShape>, &'static str);

impl BinOp {
    /// Static metadata for this operator — see the module docs for how
    /// [`OperatorShape`] and dialect gating are derived. Split into one
    /// helper per operator family (mirroring
    /// `mathop_generated.rs`'s `specs_0()..specs_19()` split) purely to stay
    /// under clippy's function-length lint; the dispatch match below is
    /// still exhaustive with no wildcard arm, so a new `BinOp` variant is a
    /// compile error here, not a silent gap.
    #[must_use]
    pub const fn spec(self) -> OperatorSpec {
        let (dialects, mathop_shape, summary) = match self {
            Self::Add
            | Self::Sub
            | Self::Mul
            | Self::Div
            | Self::Mod
            | Self::Pow
            | Self::LShift
            | Self::RShift
            | Self::BitAnd
            | Self::BitOr
            | Self::BitXor => Self::spec_arithmetic(self),
            Self::And
            | Self::Or
            | Self::Eq
            | Self::Ne
            | Self::Lt
            | Self::Le
            | Self::Gt
            | Self::Ge => Self::spec_logical_and_numeric_cmp(self),
            Self::StrEq
            | Self::StrNe
            | Self::StrLt
            | Self::StrLe
            | Self::StrGt
            | Self::StrGe
            | Self::In
            | Self::Ni => Self::spec_string_cmp_and_membership(self),
            Self::WordAnd
            | Self::WordOr
            | Self::Contains
            | Self::StartsWith
            | Self::EndsWith
            | Self::StrEquals
            | Self::MatchesGlob
            | Self::MatchesRegex => Self::spec_irules_words(self),
        };
        OperatorSpec {
            spelling: self.as_str(),
            dialects,
            mathop_shape,
            summary,
        }
    }

    /// Arithmetic and bitwise operators (`+ - * / % ** << >> & | ^`). Only
    /// ever called by [`Self::spec`] for these 11 variants — the trailing
    /// `unreachable!()` covers the rest of the enum so this stays a plain
    /// (non-wildcard-free) `match` without duplicating every arm across all
    /// four helpers.
    const fn spec_arithmetic(self) -> SpecFacts {
        match self {
            Self::Add => (None, Some(OperatorShape::Fold { identity: 0 }), "addition"),
            Self::Sub => (
                None,
                Some(OperatorShape::SubtractOrDivide {
                    negate: true,
                    reciprocal: false,
                }),
                "subtraction",
            ),
            Self::Mul => (
                None,
                Some(OperatorShape::Fold { identity: 1 }),
                "multiplication",
            ),
            Self::Div => (
                None,
                Some(OperatorShape::SubtractOrDivide {
                    negate: false,
                    reciprocal: true,
                }),
                "division",
            ),
            Self::Mod => (None, Some(OperatorShape::Binary), "modulo"),
            Self::Pow => (None, Some(OperatorShape::PowFold), "exponentiation"),
            Self::LShift => (None, Some(OperatorShape::Binary), "left shift"),
            Self::RShift => (None, Some(OperatorShape::Binary), "right shift"),
            Self::BitAnd => (
                None,
                Some(OperatorShape::Fold { identity: -1 }),
                "bitwise AND",
            ),
            Self::BitOr => (
                None,
                Some(OperatorShape::Fold { identity: 0 }),
                "bitwise OR",
            ),
            Self::BitXor => (
                None,
                Some(OperatorShape::Fold { identity: 0 }),
                "bitwise XOR",
            ),
            _ => unreachable!(),
        }
    }

    /// `&&`/`||` (expr grammar only) plus the numeric comparison family.
    const fn spec_logical_and_numeric_cmp(self) -> SpecFacts {
        match self {
            // `&&`/`||` are expr grammar only — deliberately excluded from
            // `::tcl::mathop` (TIP 174) since a command can't short-circuit
            // its own already-evaluated arguments.
            Self::And => (None, None, "logical AND (short-circuit)"),
            Self::Or => (None, None, "logical OR (short-circuit)"),
            Self::Eq => (
                None,
                Some(OperatorShape::BoolChain { string_only: false }),
                "numeric equality",
            ),
            Self::Ne => (
                None,
                Some(OperatorShape::NeBinary { string_only: false }),
                "numeric inequality",
            ),
            Self::Lt => (
                None,
                Some(OperatorShape::BoolChain { string_only: false }),
                "numeric less-than",
            ),
            Self::Le => (
                None,
                Some(OperatorShape::BoolChain { string_only: false }),
                "numeric less-or-equal",
            ),
            Self::Gt => (
                None,
                Some(OperatorShape::BoolChain { string_only: false }),
                "numeric greater-than",
            ),
            Self::Ge => (
                None,
                Some(OperatorShape::BoolChain { string_only: false }),
                "numeric greater-or-equal",
            ),
            _ => unreachable!(),
        }
    }

    /// String comparison (`eq`/`ne`/TIP 461's `lt`/`le`/`gt`/`ge`) plus list
    /// membership (`in`/`ni`).
    const fn spec_string_cmp_and_membership(self) -> SpecFacts {
        match self {
            Self::StrEq => (
                None,
                Some(OperatorShape::BoolChain { string_only: true }),
                "string equality",
            ),
            Self::StrNe => (
                None,
                Some(OperatorShape::NeBinary { string_only: true }),
                "string inequality",
            ),
            // TIP 461 (Tcl 9.0): string-ordering counterparts to `eq`/`ne` —
            // one release newer than the `::tcl::mathop` ensemble's own 8.5
            // baseline (verified absent under `::tcl::mathop::*` on tclsh
            // 8.4/8.5/8.6, present on 9.0/9.1).
            Self::StrLt => (
                Some(DialectSet::TCL90_PLUS),
                Some(OperatorShape::BoolChain { string_only: true }),
                "string less-than",
            ),
            Self::StrLe => (
                Some(DialectSet::TCL90_PLUS),
                Some(OperatorShape::BoolChain { string_only: true }),
                "string less-or-equal",
            ),
            Self::StrGt => (
                Some(DialectSet::TCL90_PLUS),
                Some(OperatorShape::BoolChain { string_only: true }),
                "string greater-than",
            ),
            Self::StrGe => (
                Some(DialectSet::TCL90_PLUS),
                Some(OperatorShape::BoolChain { string_only: true }),
                "string greater-or-equal",
            ),
            Self::In => (
                None,
                Some(OperatorShape::Membership { negate: false }),
                "list membership",
            ),
            Self::Ni => (
                None,
                Some(OperatorShape::Membership { negate: true }),
                "list non-membership",
            ),
            _ => unreachable!(),
        }
    }

    /// The 9 iRules word operators (well, 8 of the 9 — `not` is
    /// [`UnaryOp::WordNot`]): parsed as expr grammar only in the iRules
    /// dialect, with no `::tcl::mathop` command form at all (verified
    /// absent on every Tcl version — these aren't a version-gated *mathop*
    /// omission like `lt`/`le`/`gt`/`ge` above, they're not part of the
    /// `tcl::mathop` feature at all).
    const fn spec_irules_words(self) -> SpecFacts {
        match self {
            Self::WordAnd => (
                Some(DialectSet::IRULES),
                None,
                "logical AND (iRules word form)",
            ),
            Self::WordOr => (
                Some(DialectSet::IRULES),
                None,
                "logical OR (iRules word form)",
            ),
            Self::Contains => (Some(DialectSet::IRULES), None, "substring containment"),
            Self::StartsWith => (Some(DialectSet::IRULES), None, "string prefix test"),
            Self::EndsWith => (Some(DialectSet::IRULES), None, "string suffix test"),
            Self::StrEquals => (
                Some(DialectSet::IRULES),
                None,
                "string equality (iRules word form)",
            ),
            Self::MatchesGlob => (Some(DialectSet::IRULES), None, "glob pattern match"),
            Self::MatchesRegex => (Some(DialectSet::IRULES), None, "regular-expression match"),
            _ => unreachable!(),
        }
    }

    /// The operator's logical negation as a single operator, or `None` when
    /// no single operator expresses it (e.g. `&&`/`||` would need De
    /// Morgan's distribution over both operands, not a token swap).
    ///
    /// Used by refactors like "invert comparison" so they emit `$x >= $y`
    /// rather than `!($x < $y)`.
    #[must_use]
    pub const fn inverse(self) -> Option<Self> {
        match self {
            Self::Eq => Some(Self::Ne),
            Self::Ne => Some(Self::Eq),
            Self::Lt => Some(Self::Ge),
            Self::Le => Some(Self::Gt),
            Self::Gt => Some(Self::Le),
            Self::Ge => Some(Self::Lt),
            Self::StrEq => Some(Self::StrNe),
            Self::StrNe => Some(Self::StrEq),
            Self::StrLt => Some(Self::StrGe),
            Self::StrLe => Some(Self::StrGt),
            Self::StrGt => Some(Self::StrLe),
            Self::StrGe => Some(Self::StrLt),
            Self::In => Some(Self::Ni),
            Self::Ni => Some(Self::In),
            _ => None,
        }
    }
}

impl UnaryOp {
    /// Static metadata for this operator — see [`BinOp::spec`] for the
    /// shared-shape rationale (`Neg`/`Pos` alias the `-`/`+` mathop commands
    /// their `BinOp` counterparts also use).
    #[must_use]
    pub const fn spec(self) -> OperatorSpec {
        let (dialects, mathop_shape, summary) = match self {
            Self::Neg => (
                None,
                Some(OperatorShape::SubtractOrDivide {
                    negate: true,
                    reciprocal: false,
                }),
                "arithmetic negation",
            ),
            Self::Pos => (
                None,
                Some(OperatorShape::Fold { identity: 0 }),
                "arithmetic identity",
            ),
            Self::BitNot => (None, Some(OperatorShape::Unary), "bitwise complement"),
            Self::Not => (None, Some(OperatorShape::Unary), "logical NOT"),
            Self::WordNot => (
                Some(DialectSet::IRULES),
                None,
                "logical NOT (iRules word form)",
            ),
        };
        OperatorSpec {
            spelling: self.as_str(),
            dialects,
            mathop_shape,
            summary,
        }
    }
}

/// Every [`BinOp`] variant, in declaration order — the completeness anchor
/// for consumers that need to enumerate all binary operators (diagnostics,
/// completion, editor-generator sweeps).
pub const ALL_BIN_OPS: &[BinOp] = &[
    BinOp::Add,
    BinOp::Sub,
    BinOp::Mul,
    BinOp::Div,
    BinOp::Mod,
    BinOp::Pow,
    BinOp::LShift,
    BinOp::RShift,
    BinOp::BitAnd,
    BinOp::BitOr,
    BinOp::BitXor,
    BinOp::And,
    BinOp::Or,
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
    BinOp::WordAnd,
    BinOp::WordOr,
    BinOp::Contains,
    BinOp::StartsWith,
    BinOp::EndsWith,
    BinOp::StrEquals,
    BinOp::MatchesGlob,
    BinOp::MatchesRegex,
];

/// Every [`UnaryOp`] variant, in declaration order.
pub const ALL_UNARY_OPS: &[UnaryOp] = &[
    UnaryOp::Neg,
    UnaryOp::Pos,
    UnaryOp::BitNot,
    UnaryOp::Not,
    UnaryOp::WordNot,
];

#[cfg(test)]
mod tests {
    use super::*;

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
                | BinOp::MatchesGlob
                | BinOp::MatchesRegex => {}
            }
        }
        for &op in ALL_BIN_OPS {
            exhaustive(op);
        }
        assert_eq!(ALL_BIN_OPS.len(), 35);
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

        // Spot-check every shape's derived arity against the `tclsh9.1`
        // ground truth captured while designing this module: `&`, `|`, `^`,
        // `+`, `*`, `**`, `==`, `<`, `eq` all accept 0 args; `-`/`/` require
        // at least 1; `%`/`<<`/`>>`/`!=`/`ne`/`in`/`ni` are always exactly 2;
        // `!`/`~` are always exactly 1.
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
            assert_eq!(op.spec().dialects, Some(DialectSet::TCL90_PLUS));
        }
        // Their pre-existing (8.5+) counterparts are ungated.
        for op in [BinOp::StrEq, BinOp::StrNe] {
            assert_eq!(op.spec().dialects, None);
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
            BinOp::MatchesGlob,
            BinOp::MatchesRegex,
        ];
        for op in irules_ops {
            let spec = op.spec();
            assert_eq!(spec.dialects, Some(DialectSet::IRULES));
            assert_eq!(spec.mathop_shape, None);
        }
        let word_not = UnaryOp::WordNot.spec();
        assert_eq!(word_not.dialects, Some(DialectSet::IRULES));
        assert_eq!(word_not.mathop_shape, None);
    }

    #[test]
    fn short_circuit_ops_have_no_mathop_form() {
        for op in [BinOp::And, BinOp::Or] {
            let spec = op.spec();
            assert_eq!(spec.dialects, None);
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
}
