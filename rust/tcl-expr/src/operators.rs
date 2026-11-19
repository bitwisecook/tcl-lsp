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
//! [`BinOp`]/[`UnaryOp`] variants exist, which surfaces gate them, and how
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
//! helper, plus [`OperatorShape::Unary`] for the `~`/`!` commands that module
//! handles inline. An operator with no `::tcl::mathop` command form at all
//! (`&&`, `||`, every iRules word operator) has `mathop_shape: None`: the parser
//! already accepts it as expr grammar, but no bare/`tcl::mathop::`-prefixed
//! command exists for it (verified against `tclsh` 8.4 through 9.1 — `info
//! commands ::tcl::mathop::*` never lists `&&`, `||`, `and`, `or`, `not`,
//! `contains`, `starts_with`, `ends_with`, `equals`, `matches`,
//! `matches_glob`, or `matches_regex`, on any version).

use tcl_dialect::TclVersion;

use crate::{BinOp, UnaryOp};
use tcl_dialect::model::SpecSurface;

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
    /// A plain two-argument operation with no fold/chain behaviour at all
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
    /// (queried separately via `DialectProfile::expr_grammar_base`).
    pub surface: Option<&'static [SpecSurface]>,
    /// The `::tcl::mathop::*` command shape this operator shares, or `None`
    /// when the operator has no mathop command form at all (`&&`, `||`,
    /// every iRules word operator).
    pub mathop_shape: Option<OperatorShape>,
    /// A one-line human summary for hover text.
    pub summary: &'static str,
    /// The oldest Tcl release whose `expr` **grammar** parses this
    /// operator's syntax at all, or `None` when it's part of the base
    /// grammar every dialect's `expr` has always had. Distinct from
    /// [`Self::surface`] (which gates the *command-table* form, `surface:
    /// None` there just meaning "no extra gate beyond `::tcl::mathop`'s own
    /// 8.5+ requirement"): `**`/`in`/`ni` (TIP 123/201) need Tcl 8.5's
    /// grammar to parse at all but — like every other pre-9.0 operator —
    /// carry `surface: None`, so this field is the fact a *grammar*-level
    /// version diagnostic (W003) needs and `surface` doesn't give it.
    /// `None` for the iRules word operators too — they're gated by dialect
    /// *identity*, not a Tcl version threshold, an axis this field doesn't
    /// model (see [`Self::surface`] for that gate instead).
    pub expr_grammar_min_version: Option<TclVersion>,
}

/// The `(surface, mathop_shape, summary)` triple `BinOp::spec()`/
/// `UnaryOp::spec()` each assemble into a full [`OperatorSpec`].
type SpecFacts = (
    Option<&'static [SpecSurface]>,
    Option<OperatorShape>,
    &'static str,
);

impl BinOp {
    /// Static metadata for this operator — see the module docs for how
    /// [`OperatorShape`] and dialect gating are derived. Split into one
    /// helper per operator family purely to stay under clippy's
    /// function-length lint; the dispatch match below is still exhaustive
    /// with no wildcard arm, so a new `BinOp` variant is a compile error
    /// here, not a silent gap.
    #[must_use]
    pub const fn spec(self) -> OperatorSpec {
        let (surface, mathop_shape, summary) = match self {
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
            | Self::Matches
            | Self::MatchesGlob
            | Self::MatchesRegex => Self::spec_irules_words(self),
        };
        OperatorSpec {
            spelling: self.as_str(),
            surface,
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
                Some(SpecSurface::TCL90_PLUS),
                Some(OperatorShape::BoolChain { string_only: true }),
                "string less-than",
            ),
            Self::StrLe => (
                Some(SpecSurface::TCL90_PLUS),
                Some(OperatorShape::BoolChain { string_only: true }),
                "string less-or-equal",
            ),
            Self::StrGt => (
                Some(SpecSurface::TCL90_PLUS),
                Some(OperatorShape::BoolChain { string_only: true }),
                "string greater-than",
            ),
            Self::StrGe => (
                Some(SpecSurface::TCL90_PLUS),
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

    /// The 10 iRules word operators (well, 9 of the 10 — `not` is
    /// [`UnaryOp::WordNot`]): parsed as expr grammar only in the iRules
    /// dialect, with no `::tcl::mathop` command form at all (verified
    /// absent on every Tcl version — these aren't a version-gated *mathop*
    /// omission like `lt`/`le`/`gt`/`ge` above, they're not part of the
    /// `tcl::mathop` feature at all).
    const fn spec_irules_words(self) -> SpecFacts {
        match self {
            Self::WordAnd => (
                Some(SpecSurface::IRULES),
                None,
                "logical AND (iRules word form)",
            ),
            Self::WordOr => (
                Some(SpecSurface::IRULES),
                None,
                "logical OR (iRules word form)",
            ),
            Self::Contains => (Some(SpecSurface::IRULES), None, "substring containment"),
            Self::StartsWith => (Some(SpecSurface::IRULES), None, "string prefix test"),
            Self::EndsWith => (Some(SpecSurface::IRULES), None, "string suffix test"),
            Self::StrEquals => (
                Some(SpecSurface::IRULES),
                None,
                "string equality (iRules word form)",
            ),
            Self::Matches => (
                Some(SpecSurface::IRULES),
                None,
                "whole-string glob pattern match",
            ),
            Self::MatchesGlob => (Some(SpecSurface::IRULES), None, "glob pattern match"),
            Self::MatchesRegex => (Some(SpecSurface::IRULES), None, "regular-expression match"),
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
    /// the expression evaluator models: with a NaN operand `!=` is true and every
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
        let (surface, mathop_shape, summary) = match self {
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
                Some(SpecSurface::IRULES),
                None,
                "logical NOT (iRules word form)",
            ),
        };
        OperatorSpec {
            spelling: self.as_str(),
            surface,
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
    BinOp::Matches,
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
