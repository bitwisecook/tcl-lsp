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

use tcl_dialect::{DialectSet, TclVersion};

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
    /// The oldest Tcl release whose `expr` **grammar** parses this
    /// operator's syntax at all, or `None` when it's part of the base
    /// grammar every dialect's `expr` has always had. Distinct from
    /// [`Self::dialects`] (which gates the *command-table* form, `dialects:
    /// None` there just meaning "no extra gate beyond `::tcl::mathop`'s own
    /// 8.5+ requirement"): `**`/`in`/`ni` (TIP 123/201) need Tcl 8.5's
    /// grammar to parse at all but — like every other pre-9.0 operator —
    /// carry `dialects: None`, so this field is the fact a *grammar*-level
    /// version diagnostic (W003) needs and `dialects` doesn't give it.
    /// `None` for the iRules word operators too — they're gated by dialect
    /// *identity*, not a Tcl version threshold, an axis this field doesn't
    /// model (see [`Self::dialects`] for that gate instead).
    pub expr_grammar_min_version: Option<TclVersion>,
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
            expr_grammar_min_version: self.expr_grammar_min_version(),
        }
    }

    /// The oldest Tcl release whose `expr` grammar parses this operator at
    /// all — see [`OperatorSpec::expr_grammar_min_version`]. A wildcard
    /// match (unlike [`Self::spec`]'s deliberately exhaustive one): `None`
    /// — "part of the base grammar" — is a safe default for every variant
    /// this doesn't name, so a future `BinOp` addition doesn't need this
    /// function updated in lockstep, only [`Self::spec`] itself.
    const fn expr_grammar_min_version(self) -> Option<TclVersion> {
        match self {
            // TIP 123 (Tcl 8.5): exponentiation. TIP 201 (Tcl 8.5): list
            // membership operators.
            Self::Pow | Self::In | Self::Ni => Some(TclVersion::V8_5),
            // TIP 461 (Tcl 9.0): string-ordering operators.
            Self::StrLt | Self::StrLe | Self::StrGt | Self::StrGe => Some(TclVersion::V9_0),
            _ => None,
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
    ///
    /// # NaN precondition
    ///
    /// For the four **ordered numeric** comparisons (`<`, `<=`, `>`, `>=`)
    /// the identity `!(a OP b) == (a INV b)` holds only when neither operand
    /// can be the IEEE NaN. Tcl inherits C's rule (`tclExecute.c`: "NaN arg:
    /// NaN != to everything, other compares are false"), which
    /// [`crate::expr::eval`] models: with a NaN operand `!=` is true and every
    /// other comparison is false. So `expr {!(NaN < 1)}` is `1` while
    /// `expr {NaN >= 1}` is `0` — a rewriter that cannot prove both operands
    /// non-NaN must leave those four alone.
    ///
    /// Every other row is unconditional: `==`/`!=` are exact complements even
    /// under the NaN rule (`!=` is defined as the negation of `==`), and the
    /// string comparisons (`eq`/`ne`/`lt`/`le`/`gt`/`ge`) and membership tests
    /// (`in`/`ni`) never compare numerically, so no NaN rule reaches them.
    /// [`Self::inverse_needs_non_nan`] is the machine-readable form of this
    /// paragraph — consumers gate on it rather than re-deriving the split.
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

    /// Whether [`Self::inverse`]'s identity for this operator is conditional on
    /// both operands being provably non-NaN.
    ///
    /// True for exactly the four ordered numeric comparisons `<`, `<=`, `>`,
    /// `>=` — see [`Self::inverse`] for the rule. A consumer that cannot
    /// establish NaN-freedom (a source-text rewriter, an optimiser with no type
    /// facts for the operands) must decline the inversion for these, and may
    /// apply it unconditionally to every other operator the table covers.
    #[must_use]
    pub const fn inverse_needs_non_nan(self) -> bool {
        matches!(self, Self::Lt | Self::Le | Self::Gt | Self::Ge)
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
            // No `UnaryOp` needs a grammar-version gate: `-`/`+`/`~`/`!` are
            // base grammar, and `not` (like the other iRules word
            // operators) is gated by dialect identity, not a Tcl version —
            // see `OperatorSpec::expr_grammar_min_version`.
            expr_grammar_min_version: None,
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
    fn expr_grammar_min_version_matches_the_gated_operators_tclsh_ground_truth() {
        use tcl_dialect::TclVersion;

        // TIP 123/201 (Tcl 8.5): `**`/`in`/`ni` don't parse at all pre-8.5,
        // yet (unlike the TIP-461 four) carry `dialects: None` — no
        // *additional* mathop command-table gate beyond the ensemble's own
        // 8.5+ requirement. This is exactly the axis mismatch
        // `expr_grammar_min_version` exists to resolve for a grammar-level
        // consumer (W003) that `dialects` alone can't answer.
        for op in [BinOp::Pow, BinOp::In, BinOp::Ni] {
            assert_eq!(op.spec().expr_grammar_min_version, Some(TclVersion::V8_5));
            assert_eq!(
                op.spec().dialects,
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
    /// (`MULTI_OPS`/`is_single_op`/`irules_ops` in `expr_lexer.rs`).
    /// `tcl-lexer` sits *below* `tcl-syntax` in the dependency graph (see
    /// this crate's own architecture doc comment), so those tables can't
    /// derive from `ALL_BIN_OPS`/`ALL_UNARY_OPS` directly — this proves,
    /// from this (the higher) side of the boundary, that the lexer still
    /// tokenises every operator spelling this module knows about as a
    /// single `Operator` token, catching the day a new `BinOp`/`UnaryOp`
    /// variant lands here without a matching lexer update.
    #[test]
    fn tcl_lexer_recognises_every_operator_spelling_as_one_operator_token() {
        let check = |spelling: &str, dialects: Option<DialectSet>| {
            let dialect = (dialects == Some(DialectSet::IRULES)).then_some("f5-irules");
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
            check(spec.spelling, spec.dialects);
        }
        for &op in ALL_UNARY_OPS {
            let spec = op.spec();
            check(spec.spelling, spec.dialects);
        }
    }

    /// Drift guard for `tcl-lexer`'s `math_functions()` — the set
    /// `tcl-lsp-core::semantic_tokens` reads to decide which `Function`-kind
    /// expr tokens get the "known math function" modifier. Found this list
    /// missing every TIP 521 (9.0) / TIP 745 (9.1) addition (under-
    /// classifying `expr {gamma(2.5)}`/`expr {isfinite($x)}` in a 9.1
    /// document) — this guard is what would have caught it at the time.
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
