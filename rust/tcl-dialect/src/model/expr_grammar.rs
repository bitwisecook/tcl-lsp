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

//! The full `ExprGrammar` contract of design doc
//! `docs/design/dialect-and-package-registry-redesign.md` §3.1, as data —
//! no function pointers.
//!
//! The word-operators/comments/numbers triple is not enough for a non-Tcl
//! family: precedence is a per-family fact (Jim binds `eq` and `==` at
//! different levels where C Tcl merges them), symbolic operators need
//! lexer recognition (`<<<`, `=~`), the mathfunc surface is a set rather
//! than a floor, `expr`'s own arity is release-keyed, and whether the
//! expr engine interpolates at all varies in the wild (picol 2). Each of
//! those axes is a typed field here, resolved per
//! `(family, release, build)` through
//! [`CoreProfile::expr`](crate::model::family::CoreProfile).
//!
//! Seeded: the Tcl family per release, iRules (the ten word operators on
//! an 8.4 base), and — since P6 — the Jim ladder read out of the
//! upstream `jim.c` at every tag from 0.76 to 0.84: the full `OPRINIT`
//! binding-power tables, the release at which each word and symbolic
//! operator arrives, the twenty-six mathfuncs with the seven that
//! survive a `--minimal` build, the expr-comment and arity flips at
//! 0.81. Five struct-update values cover the whole ladder where the old
//! model needed nine profiles.
//!
//! **The Jim rows are transcripts, not readings.** Five `jimsh` binaries
//! were built from the upstream tags for P6 — 0.76 `--full`, 0.79
//! `--full`, 0.81 default, 0.84 default, 0.84 `--minimal` — and each
//! divergence below was run:
//!
//! | probe | 0.76 | 0.79 | 0.81 | 0.84 | 0.84 `--minimal` |
//! |---|---|---|---|---|---|
//! | `expr {"abc" lt "abd"}` | error | error | 1 | 1 | 1 |
//! | `expr {"abc" =* "a*"}` | error | error | error | 1 | 1 |
//! | `expr 1 + 2` | 3 | 3 | wrong # args | wrong # args | wrong # args |
//! | `expr {-2 ** 2}` | -4 | 4 | 4 | 4 | 4 |
//! | `expr {2 ** 3 ** 2}` | 64 | 512 | 512 | 512 | 512 |
//! | `expr {atan2(1,1)}` | error | 0.785… | 0.785… | 0.785… | error |
//! | `expr {sqrt(4)}` | 2.0 | 2.0 | error | 2.0 | error |
//! | `expr {int(4.7)}` | 4 | 4 | 4 | 4 | 4 |
//! | `expr {min(1,2)}` | error | error | error | error | error |
//! | `expr {010}` | 10 | 10 | 10 | 10 | 10 |
//! | `expr {1_000}` | error | error | error | error | error |

use crate::grammar::{ExprCommentStyle, NumberSyntax};
use crate::model::family::{Family, Release};

/// One word-shaped binary (or, for iRules' `not`, unary) expr operator,
/// with the oldest release on its family's ladder whose lexeme table
/// contains it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordOperator {
    /// The operator spelling (`"eq"`, `"contains"`).
    pub spelling: &'static str,
    /// The oldest release whose expr lexeme table contains it. For the
    /// iRules extension operators this is the family's own release line;
    /// for the operators iRules inherits from its embedded Tcl 8.4.6 core
    /// it is the embedded core's release.
    pub since: Release,
}

const fn word(spelling: &'static str, since: Release) -> WordOperator {
    WordOperator { spelling, since }
}

/// A per-family binding-power table for binary infix operators:
/// `(left_bp, right_bp)` per spelling, left-associative rows encoded as
/// `right = left + 1` and right-associative as `right = left`.
///
/// Precedence is NOT derivable from the operator set (§3.1): Jim and Tcl
/// share `eq`/`ne`/`lt`/`in` yet bind them at different levels, so two
/// cores accepting the identical operator set can produce different parse
/// trees. The table is a per-family fact; release gating of individual
/// operators lives in [`ExprGrammar::word_operators`] /
/// [`ExprGrammar::symbolic_operators`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrecedenceTable {
    rows: &'static [(&'static str, u16, u16)],
}

impl PrecedenceTable {
    /// The binding powers of the binary operator `op`, or `None` when the
    /// family's ladder never binds `op` as a binary infix operator
    /// (unary operators such as iRules' `not` are deliberately absent).
    #[must_use]
    pub fn lookup(&self, op: &str) -> Option<(u16, u16)> {
        self.rows
            .iter()
            .find_map(|&(spelling, left, right)| (spelling == op).then_some((left, right)))
    }

    /// Every `(spelling, left_bp, right_bp)` row.
    #[must_use]
    pub const fn rows(&self) -> &'static [(&'static str, u16, u16)] {
        self.rows
    }
}

/// One expr math function with the oldest release on its family's ladder
/// that ships it, and whether shipping it also depends on the build's
/// math extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MathFunc {
    /// The function name, matched verbatim (mathfunc lookup is
    /// case-sensitive).
    pub name: &'static str,
    /// The oldest release whose canonical build ships it.
    pub since: Release,
    /// Whether the function exists only when the build compiles in the
    /// expr math extension
    /// ([`CapabilitySet::math_extension`](crate::model::family::CapabilitySet::math_extension)).
    ///
    /// False for every C Tcl and F5 row — their math functions are a
    /// fixed part of the core. True for the nineteen Jim functions
    /// guarded by `#ifdef JIM_MATH_FUNCTIONS` in `Jim_ExprOperators`, and
    /// false for the seven that sit outside the guard (`int`, `wide`,
    /// `abs`, `double`, `round`, `rand`, `srand`), which is why a Jim
    /// `--minimal` build rejects `sqrt(4)` and still evaluates
    /// `int(4)`.
    pub needs_math_extension: bool,
}

const fn func(name: &'static str, since: Release) -> MathFunc {
    MathFunc {
        name,
        since,
        needs_math_extension: false,
    }
}

/// A math function that exists only when the build's math extension is
/// compiled in — Jim's `#ifdef JIM_MATH_FUNCTIONS` block.
const fn math_ext_func(name: &'static str, since: Release) -> MathFunc {
    MathFunc {
        name,
        since,
        needs_math_extension: true,
    }
}

/// The mathfunc surface of one resolved core, as a set (§3.1): membership
/// is answered against the resolved release, so a family that simply
/// never had a function is expressible — a floor model is not enough.
///
/// Build gating (a `--minimal` build that rejects `sqrt(4)` outright)
/// rides above this set, on
/// [`CoreProfile::mathfunc`](crate::model::family::CoreProfile::mathfunc)
/// via the capability record — this type answers for the canonical build
/// only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MathFuncSet {
    rows: &'static [MathFunc],
    ceiling: Release,
}

impl MathFuncSet {
    /// Whether `name` is in the set at the resolved release.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// The set's row for `name` at the resolved release, or `None` when
    /// the family never had it (or not yet at this release).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&'static MathFunc> {
        self.rows.iter().find(|f| {
            f.name == name
                && f.since.family() == self.ceiling.family()
                && f.since.ordinal() <= self.ceiling.ordinal()
        })
    }

    /// The member names at the resolved release, in table order.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.rows.iter().filter_map(|f| {
            (f.since.family() == self.ceiling.family()
                && f.since.ordinal() <= self.ceiling.ordinal())
            .then_some(f.name)
        })
    }
}

/// How a multi-word `expr` invocation parses — whether the words are
/// concatenated with spaces before parsing, or exactly one expression
/// word is accepted (§3.1: Jim 0.81 adopted the single-argument form; C
/// Tcl still concatenates in 9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExprArity {
    /// `expr 1 + 2` concatenates its words before parsing.
    Concatenating,
    /// `expr` takes exactly one expression word.
    ExactlyOne,
}

/// Whether `$var` / `[cmd]` interpolate inside the expr engine itself
/// (§3.1: invisible while every modelled family substitutes; picol 2
/// proves the axis varies in the wild).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExprSubstitution {
    /// The expr engine performs `$`/`[…]` substitution (tcl, jim,
    /// irules).
    Interpolating,
    /// The expr engine performs no interpolation; only ordinary word
    /// substitution outside it applies (picol 2's `expr`).
    NonInterpolating,
}

/// The full expr grammar of one `(family, release)` — §3.1's contract,
/// every field data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExprGrammar {
    /// The numeral grammar, including the special-float set.
    pub numbers: NumberSyntax,
    /// Whether `#` begins a comment inside an `[expr]` body.
    pub comments: ExprCommentStyle,
    /// The word-shaped operators available at this release, resolved —
    /// the slice already reflects the release, and each row carries its
    /// introducing release as provenance.
    pub word_operators: &'static [WordOperator],
    /// The family's binding-power table.
    pub precedence: PrecedenceTable,
    /// Symbolic (non-word) operators beyond the shared C-Tcl set,
    /// release-gated within the family: Jim's `<<<` / `>>>` rotates
    /// (every modelled release) and `=*` / `=~` (0.84 only).
    pub symbolic_operators: &'static [(&'static str, Release)],
    /// The mathfunc surface as a set for the canonical build.
    pub mathfuncs: MathFuncSet,
    /// How a multi-word `expr` invocation parses.
    pub arity: ExprArity,
    /// Whether the expr engine interpolates `$var` / `[cmd]` itself.
    pub substitution: ExprSubstitution,
}

impl ExprGrammar {
    /// Whether the word operator `spelling` exists at this resolved
    /// grammar.
    #[must_use]
    pub fn has_word_operator(&self, spelling: &str) -> bool {
        self.word_operators.iter().any(|w| w.spelling == spelling)
    }
}

// Word-operator slices. The Tcl rows and their evidence mirror
// `EXPR_WORD_OPERATORS` in `grammar.rs` (eq/ne 8.4; in/ni TIP 201, 8.5;
// lt/le/gt/ge TIP 461, 9.0); each release's slice lists exactly the
// operators its lexeme table has.

const TCL_WORDS_84: &[WordOperator] = &[word("eq", Release::TCL_8_4), word("ne", Release::TCL_8_4)];

const TCL_WORDS_85: &[WordOperator] = &[
    word("eq", Release::TCL_8_4),
    word("ne", Release::TCL_8_4),
    word("in", Release::TCL_8_5),
    word("ni", Release::TCL_8_5),
];

const TCL_WORDS_90: &[WordOperator] = &[
    word("eq", Release::TCL_8_4),
    word("ne", Release::TCL_8_4),
    word("in", Release::TCL_8_5),
    word("ni", Release::TCL_8_5),
    word("lt", Release::TCL_9_0),
    word("le", Release::TCL_9_0),
    word("gt", Release::TCL_9_0),
    word("ge", Release::TCL_9_0),
];

/// The F5 **trunk**: the fork point's word operators (`eq`/`ne` from the
/// embedded Tcl 8.4.6 core) plus the ten word-form extension operators
/// (`and`/`or`/`not`/`contains`/`starts_with`/`ends_with`/`equals`/
/// `matches`/`matches_glob`/`matches_regex` — the set the expr lexer's
/// `irules_ops` recognises). Measured **byte-identical in tmsh and iApp
/// contexts, not iRules-only**
/// (`docs/design/bigip-irule-parser-measurements.md` §4a: `expr {"abc"
/// starts_with "a"}` and `expr {1 and 1}` answer `1` in all three F5
/// contexts), so the ten carry the trunk's own release as their
/// provenance. `not` is unary and therefore absent from the precedence
/// table. The iRules offshoot overrides nothing here — it answers with
/// this same slice along the fork edge.
///
/// The bare `matches` is the tenth and was added last: §4a's `e_matches`
/// case (`expr {"abc" matches "abc"}`) answered `1` in all three F5
/// contexts and failed on both host builds, and §4b's model
/// recommendation lists it beside `matches_glob`/`matches_regex` among
/// the trunk's `expr` extensions.
const F5_TCL_WORDS: &[WordOperator] = &[
    word("eq", Release::TCL_8_4),
    word("ne", Release::TCL_8_4),
    word("and", Release::F5_TCL_TMOS),
    word("or", Release::F5_TCL_TMOS),
    word("not", Release::F5_TCL_TMOS),
    word("contains", Release::F5_TCL_TMOS),
    word("starts_with", Release::F5_TCL_TMOS),
    word("ends_with", Release::F5_TCL_TMOS),
    word("equals", Release::F5_TCL_TMOS),
    word("matches", Release::F5_TCL_TMOS),
    word("matches_glob", Release::F5_TCL_TMOS),
    word("matches_regex", Release::F5_TCL_TMOS),
];

/// Jim through 0.79: `eq`/`ne`/`in`/`ni` only.
///
/// **This corrects the design's §3.1 reading.** The prose says Jim shares
/// `eq`/`ne`/`in`/`ni`/`lt`/`le`/`gt`/`ge` "across every modelled
/// release"; the `OPRINIT` table says otherwise. `lt`/`gt`/`le`/`ge` are
/// absent from `Jim_ExprOperators` at 0.76, 0.77, 0.78 and 0.79 and
/// appear at **0.80**, carrying the comment "Precedence must be higher
/// than ==, !=, eq, ne but lower than <, >, <=, >=". Jim reached the TIP
/// 461 word operators five Tcl years before C Tcl shipped them in 9.0,
/// but it did not have them from the start of the modelled ladder, and a
/// completion offering `lt` under `jim 0.78` would be offering a syntax
/// error.
const JIM_WORDS_0_76: &[WordOperator] = &[
    word("eq", Release::JIM_0_76),
    word("ne", Release::JIM_0_76),
    word("in", Release::JIM_0_76),
    word("ni", Release::JIM_0_76),
];

/// Jim from 0.80: the four string relationals join the table.
const JIM_WORDS_0_80: &[WordOperator] = &[
    word("eq", Release::JIM_0_76),
    word("ne", Release::JIM_0_76),
    word("in", Release::JIM_0_76),
    word("ni", Release::JIM_0_76),
    word("lt", Release::JIM_0_80),
    word("le", Release::JIM_0_80),
    word("gt", Release::JIM_0_80),
    word("ge", Release::JIM_0_80),
];

// Binding powers for the Tcl family, transcribed from `binary_bp` in
// `rust/tcl-syntax/src/expr/parser.rs` so the parser can later source
// them from here. C Tcl merges the comparisons into two levels
// (`tclCompExpr.c`): `== != eq ne` at one, the relationals and
// membership at the other. `**` is right-associative (right == left).
const TCL_PRECEDENCE_ROWS: &[(&str, u16, u16)] = &[
    ("||", 4, 5),
    ("&&", 6, 7),
    ("|", 8, 9),
    ("^", 10, 11),
    ("&", 12, 13),
    ("==", 14, 15),
    ("!=", 14, 15),
    ("eq", 14, 15),
    ("ne", 14, 15),
    ("<", 16, 17),
    (">", 16, 17),
    ("<=", 16, 17),
    (">=", 16, 17),
    ("in", 16, 17),
    ("ni", 16, 17),
    ("lt", 16, 17),
    ("le", 16, 17),
    ("gt", 16, 17),
    ("ge", 16, 17),
    ("<<", 18, 19),
    (">>", 18, 19),
    ("+", 20, 21),
    ("-", 20, 21),
    ("*", 22, 23),
    ("/", 22, 23),
    ("%", 22, 23),
    ("**", 23, 23),
];

// The F5 trunk: the Tcl rows plus the word forms `binary_bp` binds —
// `or` with `||`, `and` with `&&`, and the seven binary comparison
// extensions at the equality level. A trunk fact: tmsh and iApp accept
// the identical operator set (measurements §4a).
//
// `matches` sits at the equality level with its siblings by
// **inference, not measurement**: the transcripts pin only that the
// operator parses and answers `1` for `expr {"abc" matches "abc"}` (§4a
// `e_matches`), a single-operator expression that exercises no binding
// power at all. §12 carries the discriminating re-probe; until it is
// run, the operator takes the class every other F5 string-comparison
// word form was measured at.
const F5_TCL_PRECEDENCE_ROWS: &[(&str, u16, u16)] = &[
    ("||", 4, 5),
    ("or", 4, 5),
    ("&&", 6, 7),
    ("and", 6, 7),
    ("|", 8, 9),
    ("^", 10, 11),
    ("&", 12, 13),
    ("==", 14, 15),
    ("!=", 14, 15),
    ("eq", 14, 15),
    ("ne", 14, 15),
    ("contains", 14, 15),
    ("starts_with", 14, 15),
    ("ends_with", 14, 15),
    ("equals", 14, 15),
    ("matches", 14, 15),
    ("matches_glob", 14, 15),
    ("matches_regex", 14, 15),
    ("<", 16, 17),
    (">", 16, 17),
    ("<=", 16, 17),
    (">=", 16, 17),
    ("<<", 18, 19),
    (">>", 18, 19),
    ("+", 20, 21),
    ("-", 20, 21),
    ("*", 22, 23),
    ("/", 22, 23),
    ("%", 22, 23),
    ("**", 23, 23),
];

// Jim's binary binding powers, transcribed from `Jim_ExprOperators`'
// `OPRINIT` rows at the upstream tags 0.76 … 0.84. Jim's own scale runs
// `||` 9 … `**` 120 (250 at 0.76); each row is encoded as `(2p, 2p + 1)`
// for a left-associative operator and `(2p, 2p)` for a right-associative
// one (`**` carries `OP_RIGHT_ASSOC`), the same convention the Tcl table
// above uses, so Jim's ordering is preserved exactly and every number
// here is twice a number in the C source.
//
// The unary rows (`!`, `~`, and the unary `+`/`-` spelled `" +"`/`" -"`,
// all at 150) and the ternary `?`/`:` (5) are deliberately absent: this
// is a binary infix table, exactly as `not` is absent from the F5 one.
//
// The comparison block is §3.1's motivating divergence, and the sources
// bear the design's numbers out: `in ni` 55, `eq ne` 60, `== !=` 70,
// `lt gt le ge` 75, `< > <= >=` 80. Where C Tcl merges the comparisons
// into two levels, Jim splits them into four.

/// Jim 0.76. `**` sits at **250** here — above the unary operators — and
/// is **left**-associative (`OPRINIT("**", 250, 2, JimExprOpBin)`, no
/// `OP_RIGHT_ASSOC`). 0.77 lowered it to 120 *and* made it
/// right-associative, with the comment "Precedence is higher than * and
/// / but lower than ! and ~". Both halves are measurable on a built
/// `jimsh`, and both were measured for P6: `expr {-2 ** 2}` is **-4** at
/// 0.76 and **4** at 0.79 (the unary minus overtakes `**`), and
/// `expr {2 ** 3 ** 2}` is **64** at 0.76 and **512** at 0.79. This is
/// the one row Jim ever moved, and it is why the table is release-keyed
/// rather than the per-family constant the design sketched.
const JIM_PRECEDENCE_ROWS_0_76: &[(&str, u16, u16)] = &[
    ("||", 18, 19),
    ("&&", 20, 21),
    ("|", 96, 97),
    ("^", 98, 99),
    ("&", 100, 101),
    ("in", 110, 111),
    ("ni", 110, 111),
    ("eq", 120, 121),
    ("ne", 120, 121),
    ("==", 140, 141),
    ("!=", 140, 141),
    ("<", 160, 161),
    (">", 160, 161),
    ("<=", 160, 161),
    (">=", 160, 161),
    ("<<", 180, 181),
    (">>", 180, 181),
    ("<<<", 180, 181),
    (">>>", 180, 181),
    ("+", 200, 201),
    ("-", 200, 201),
    ("*", 220, 221),
    ("/", 220, 221),
    ("%", 220, 221),
    ("**", 500, 501),
];

/// Jim 0.77 through 0.79: identical but for `**`, now 120 and
/// right-associative.
const JIM_PRECEDENCE_ROWS_0_77: &[(&str, u16, u16)] = &[
    ("||", 18, 19),
    ("&&", 20, 21),
    ("|", 96, 97),
    ("^", 98, 99),
    ("&", 100, 101),
    ("in", 110, 111),
    ("ni", 110, 111),
    ("eq", 120, 121),
    ("ne", 120, 121),
    ("==", 140, 141),
    ("!=", 140, 141),
    ("<", 160, 161),
    (">", 160, 161),
    ("<=", 160, 161),
    (">=", 160, 161),
    ("<<", 180, 181),
    (">>", 180, 181),
    ("<<<", 180, 181),
    (">>>", 180, 181),
    ("+", 200, 201),
    ("-", 200, 201),
    ("*", 220, 221),
    ("/", 220, 221),
    ("%", 220, 221),
    ("**", 240, 240),
];

/// Jim 0.80 through 0.83: the four string relationals arrive at 75 —
/// "higher than ==, !=, eq, ne but lower than <, >, <=, >=".
const JIM_PRECEDENCE_ROWS_0_80: &[(&str, u16, u16)] = &[
    ("||", 18, 19),
    ("&&", 20, 21),
    ("|", 96, 97),
    ("^", 98, 99),
    ("&", 100, 101),
    ("in", 110, 111),
    ("ni", 110, 111),
    ("eq", 120, 121),
    ("ne", 120, 121),
    ("==", 140, 141),
    ("!=", 140, 141),
    ("lt", 150, 151),
    ("gt", 150, 151),
    ("le", 150, 151),
    ("ge", 150, 151),
    ("<", 160, 161),
    (">", 160, 161),
    ("<=", 160, 161),
    (">=", 160, 161),
    ("<<", 180, 181),
    (">>", 180, 181),
    ("<<<", 180, 181),
    (">>>", 180, 181),
    ("+", 200, 201),
    ("-", 200, 201),
    ("*", 220, 221),
    ("/", 220, 221),
    ("%", 220, 221),
    ("**", 240, 240),
];

/// Jim 0.84: `=*` and `=~` join `eq`/`ne` at 60 — the same semantic
/// operation iRules spells `matches_glob`/`matches_regex`, at Jim's
/// spelling and Jim's level.
const JIM_PRECEDENCE_ROWS_0_84: &[(&str, u16, u16)] = &[
    ("||", 18, 19),
    ("&&", 20, 21),
    ("|", 96, 97),
    ("^", 98, 99),
    ("&", 100, 101),
    ("in", 110, 111),
    ("ni", 110, 111),
    ("eq", 120, 121),
    ("ne", 120, 121),
    ("=*", 120, 121),
    ("=~", 120, 121),
    ("==", 140, 141),
    ("!=", 140, 141),
    ("lt", 150, 151),
    ("gt", 150, 151),
    ("le", 150, 151),
    ("ge", 150, 151),
    ("<", 160, 161),
    (">", 160, 161),
    ("<=", 160, 161),
    (">=", 160, 161),
    ("<<", 180, 181),
    (">>", 180, 181),
    ("<<<", 180, 181),
    (">>>", 180, 181),
    ("+", 200, 201),
    ("-", 200, 201),
    ("*", 220, 221),
    ("/", 220, 221),
    ("%", 220, 221),
    ("**", 240, 240),
];

const TCL_PRECEDENCE: PrecedenceTable = PrecedenceTable {
    rows: TCL_PRECEDENCE_ROWS,
};

const F5_TCL_PRECEDENCE: PrecedenceTable = PrecedenceTable {
    rows: F5_TCL_PRECEDENCE_ROWS,
};

const JIM_PRECEDENCE_0_76: PrecedenceTable = PrecedenceTable {
    rows: JIM_PRECEDENCE_ROWS_0_76,
};

const JIM_PRECEDENCE_0_77: PrecedenceTable = PrecedenceTable {
    rows: JIM_PRECEDENCE_ROWS_0_77,
};

const JIM_PRECEDENCE_0_80: PrecedenceTable = PrecedenceTable {
    rows: JIM_PRECEDENCE_ROWS_0_80,
};

const JIM_PRECEDENCE_0_84: PrecedenceTable = PrecedenceTable {
    rows: JIM_PRECEDENCE_ROWS_0_84,
};

/// Jim's symbolic extension operators (design §3.1): the 64-bit rotates
/// on every modelled release, and the glob/regexp matches — iRules'
/// `matches_glob`/`matches_regex` at Jim's spelling — on 0.84 only.
const JIM_SYMBOLIC: &[(&str, Release)] = &[
    ("<<<", Release::JIM_0_76),
    (">>>", Release::JIM_0_76),
    ("=*", Release::JIM_0_84),
    ("=~", Release::JIM_0_84),
];

// The Tcl mathfunc rows, transcribed from `added_in` in
// `rust/tcl-syntax/src/expr/mathfunc.rs` (the single source there for
// which names are expr functions and when each appeared): the 8.4 fixed C
// table, TIP 232's 8.5 additions, TIP 521's 9.0 classifications, and TIP
// 745's 9.1 C99 batch. `tcl-syntax` sits above this crate, so equality is
// pinned by count/spot tests here and by the P6+ migration that makes
// `mathfunc.rs` read this table instead.
const TCL_MATHFUNCS: &[MathFunc] = &[
    func("abs", Release::TCL_8_4),
    func("acos", Release::TCL_8_4),
    func("asin", Release::TCL_8_4),
    func("atan", Release::TCL_8_4),
    func("atan2", Release::TCL_8_4),
    func("ceil", Release::TCL_8_4),
    func("cos", Release::TCL_8_4),
    func("cosh", Release::TCL_8_4),
    func("double", Release::TCL_8_4),
    func("exp", Release::TCL_8_4),
    func("floor", Release::TCL_8_4),
    func("fmod", Release::TCL_8_4),
    func("hypot", Release::TCL_8_4),
    func("int", Release::TCL_8_4),
    func("log", Release::TCL_8_4),
    func("log10", Release::TCL_8_4),
    func("pow", Release::TCL_8_4),
    func("rand", Release::TCL_8_4),
    func("round", Release::TCL_8_4),
    func("sin", Release::TCL_8_4),
    func("sinh", Release::TCL_8_4),
    func("sqrt", Release::TCL_8_4),
    func("srand", Release::TCL_8_4),
    func("tan", Release::TCL_8_4),
    func("tanh", Release::TCL_8_4),
    func("wide", Release::TCL_8_4),
    func("bool", Release::TCL_8_5),
    func("entier", Release::TCL_8_5),
    func("isqrt", Release::TCL_8_5),
    func("max", Release::TCL_8_5),
    func("min", Release::TCL_8_5),
    func("isfinite", Release::TCL_9_0),
    func("isinf", Release::TCL_9_0),
    func("isnan", Release::TCL_9_0),
    func("isnormal", Release::TCL_9_0),
    func("issubnormal", Release::TCL_9_0),
    func("isunordered", Release::TCL_9_0),
    func("acosh", Release::TCL_9_1),
    func("asinh", Release::TCL_9_1),
    func("atanh", Release::TCL_9_1),
    func("cbrt", Release::TCL_9_1),
    func("copysign", Release::TCL_9_1),
    func("dim", Release::TCL_9_1),
    func("erf", Release::TCL_9_1),
    func("erfc", Release::TCL_9_1),
    func("exp2", Release::TCL_9_1),
    func("expm1", Release::TCL_9_1),
    func("fma", Release::TCL_9_1),
    func("gamma", Release::TCL_9_1),
    func("ldexp", Release::TCL_9_1),
    func("lgamma", Release::TCL_9_1),
    func("log1p", Release::TCL_9_1),
    func("log2", Release::TCL_9_1),
    func("logb", Release::TCL_9_1),
    func("nextafter", Release::TCL_9_1),
    func("remainder", Release::TCL_9_1),
    func("signbit", Release::TCL_9_1),
    func("trunc", Release::TCL_9_1),
];

/// Jim's mathfunc rows, read out of `Jim_ExprOperators`' `OP_FUNC` block
/// at the upstream tags — no longer the empty placeholder the design
/// forbade guessing at.
///
/// Twenty-six at 0.77 and later, exactly the count §3.1 pins, and the
/// five absentees it names are confirmed: **`entier`, `bool`, `min`,
/// `max` and `isqrt` appear nowhere in the table at any modelled tag**,
/// so a floor model keyed on "available since Tcl 8.5" would offer all
/// five under Jim and every one of them is a syntax error there.
///
/// Two facts the design did not have:
///
/// 1. The set is **release-gated within the family**: 0.76 ships
///    twenty-three, and `atan2`, `hypot` and `fmod` arrive at 0.77.
/// 2. The set splits on the **build** axis. Seven rows — `int`, `wide`,
///    `abs`, `double`, `round`, `rand`, `srand` — sit *outside*
///    `#ifdef JIM_MATH_FUNCTIONS`; the other nineteen sit inside it. A
///    `--minimal` build therefore has a mathfunc surface of seven, not
///    of zero, which is why the guard is per row
///    ([`MathFunc::needs_math_extension`]) rather than a single
///    build-wide veto.
const JIM_MATHFUNCS: &[MathFunc] = &[
    // Always compiled in — no `JIM_MATH_FUNCTIONS` guard.
    func("int", Release::JIM_0_76),
    func("wide", Release::JIM_0_76),
    func("abs", Release::JIM_0_76),
    func("double", Release::JIM_0_76),
    func("round", Release::JIM_0_76),
    func("rand", Release::JIM_0_76),
    func("srand", Release::JIM_0_76),
    // `#ifdef JIM_MATH_FUNCTIONS` — absent from a `--minimal` build.
    math_ext_func("sin", Release::JIM_0_76),
    math_ext_func("cos", Release::JIM_0_76),
    math_ext_func("tan", Release::JIM_0_76),
    math_ext_func("asin", Release::JIM_0_76),
    math_ext_func("acos", Release::JIM_0_76),
    math_ext_func("atan", Release::JIM_0_76),
    math_ext_func("sinh", Release::JIM_0_76),
    math_ext_func("cosh", Release::JIM_0_76),
    math_ext_func("tanh", Release::JIM_0_76),
    math_ext_func("ceil", Release::JIM_0_76),
    math_ext_func("floor", Release::JIM_0_76),
    math_ext_func("exp", Release::JIM_0_76),
    math_ext_func("log", Release::JIM_0_76),
    math_ext_func("log10", Release::JIM_0_76),
    math_ext_func("sqrt", Release::JIM_0_76),
    math_ext_func("pow", Release::JIM_0_76),
    // The three two-argument functions 0.77 added.
    math_ext_func("atan2", Release::JIM_0_77),
    math_ext_func("hypot", Release::JIM_0_77),
    math_ext_func("fmod", Release::JIM_0_77),
];

const fn jim_set(ceiling: Release) -> MathFuncSet {
    MathFuncSet {
        rows: JIM_MATHFUNCS,
        ceiling,
    }
}

const fn tcl_set(ceiling: Release) -> MathFuncSet {
    MathFuncSet {
        rows: TCL_MATHFUNCS,
        ceiling,
    }
}

const EXPR_TCL84: ExprGrammar = ExprGrammar {
    numbers: NumberSyntax::Tcl84,
    comments: ExprCommentStyle::None,
    word_operators: TCL_WORDS_84,
    precedence: TCL_PRECEDENCE,
    symbolic_operators: &[],
    mathfuncs: tcl_set(Release::TCL_8_4),
    arity: ExprArity::Concatenating,
    substitution: ExprSubstitution::Interpolating,
};

const EXPR_TCL85: ExprGrammar = ExprGrammar {
    numbers: NumberSyntax::Tcl85,
    word_operators: TCL_WORDS_85,
    mathfuncs: tcl_set(Release::TCL_8_5),
    ..EXPR_TCL84
};

const EXPR_TCL86: ExprGrammar = ExprGrammar {
    mathfuncs: tcl_set(Release::TCL_8_6),
    ..EXPR_TCL85
};

const EXPR_TCL90: ExprGrammar = ExprGrammar {
    numbers: NumberSyntax::Tcl90,
    comments: ExprCommentStyle::Hash,
    word_operators: TCL_WORDS_90,
    mathfuncs: tcl_set(Release::TCL_9_0),
    ..EXPR_TCL86
};

const EXPR_TCL91: ExprGrammar = ExprGrammar {
    mathfuncs: tcl_set(Release::TCL_9_1),
    ..EXPR_TCL90
};

/// The F5 trunk: the ten word operators over the fork point's 8.4 base
/// — 8.4 numerals (`0b101` fails in every F5 context, measurements §4a),
/// no expr comments, the 8.4 mathfunc set (expressed on the fork
/// parent's ladder), concatenating arity. The `expr` sub-parser is
/// otherwise **unmodified** from stock 8.4 (measurements R7/§7).
const EXPR_F5_TCL: ExprGrammar = ExprGrammar {
    numbers: NumberSyntax::Tcl84,
    comments: ExprCommentStyle::None,
    word_operators: F5_TCL_WORDS,
    precedence: F5_TCL_PRECEDENCE,
    symbolic_operators: &[],
    mathfuncs: tcl_set(Release::TCL_8_4),
    arity: ExprArity::Concatenating,
    substitution: ExprSubstitution::Interpolating,
};

/// The iRules offshoot overrides no expr axis: its grammar answers from
/// the trunk along the fork edge (measurements §4a — the word operators
/// are present in tmsh and iApps too). What iRules adds to `expr` is
/// load-time math-function *validation*, a compiler strictness rule, not
/// grammar.
const EXPR_IRULES: ExprGrammar = EXPR_F5_TCL;

/// Jim 0.76 — the oldest modelled release, and the base every later Jim
/// value is a struct update over. Four struct updates replace the nine
/// near-identical `jim0.76`–`jim0.84` profiles the old model needed,
/// because a profile could carry exactly one resolved grammar.
///
/// `numbers` is `NumberSyntax::Tcl90` for the reason
/// [`crate::model::family::grammar`]'s Jim value documents: Jim's own
/// numeral grammar is a fifth enum value that does not exist yet, and
/// `Tcl90` is right about the load-bearing half (`010` is ten, not
/// eight).
const EXPR_JIM_0_76: ExprGrammar = ExprGrammar {
    // Jim's own numeral grammar. It matches Tcl 9.0 on decimal leading
    // zero and differs on three counts: no `0d` before 0.80, `_` digit
    // separators never accepted, and a six-spelling special-float set with
    // no `Infinity` and no `NaN(payload)` — `expr {Infinity}` is a syntax
    // error on jimsh 0.84 where tclsh 9.0 answers `Inf`.
    numbers: NumberSyntax::Jim,
    comments: ExprCommentStyle::None,
    word_operators: JIM_WORDS_0_76,
    precedence: JIM_PRECEDENCE_0_76,
    symbolic_operators: JIM_SYMBOLIC,
    mathfuncs: jim_set(Release::JIM_0_76),
    arity: ExprArity::Concatenating,
    substitution: ExprSubstitution::Interpolating,
};

/// Jim 0.77 through 0.79: `**` drops from 250 to 120, and `atan2`,
/// `hypot` and `fmod` join the mathfunc set.
const EXPR_JIM_0_77: ExprGrammar = ExprGrammar {
    precedence: JIM_PRECEDENCE_0_77,
    mathfuncs: jim_set(Release::JIM_0_77),
    ..EXPR_JIM_0_76
};

/// Jim 0.80: `lt`/`gt`/`le`/`ge` arrive, at their own level between
/// `== !=` and `< > <= >=`.
const EXPR_JIM_0_80: ExprGrammar = ExprGrammar {
    // `0d` arrives with the `lt`/`ge` operators: `expr 0d10` is a syntax
    // error on jimsh 0.79 and 10 on 0.80.
    numbers: NumberSyntax::Jim080,
    word_operators: JIM_WORDS_0_80,
    precedence: JIM_PRECEDENCE_0_80,
    mathfuncs: jim_set(Release::JIM_0_80),
    ..EXPR_JIM_0_77
};

/// Jim 0.81 through 0.83: `expr` takes exactly one expression word
/// (`Jim_ExprCoreCommand` gained the `Jim_WrongNumArgs(interp, 1, argv,
/// "expression")` arm at this tag) and `#` begins a comment inside an
/// expr body.
///
/// One honest residue: both halves are `#ifndef JIM_COMPAT` /
/// `#ifdef JIM_COMPAT` in the C, so a `--compat` build still
/// concatenates. `--compat` is off unless asked for (a plain `opt-bool
/// compat` in `auto.def`), so the ladder value is the default build's;
/// expressing the other column needs a `BuildProfileId::JimCompat` and a
/// build-keyed `expr` resolution, which is P6's recorded next probe.
const EXPR_JIM_0_81: ExprGrammar = ExprGrammar {
    comments: ExprCommentStyle::Hash,
    arity: ExprArity::ExactlyOne,
    mathfuncs: jim_set(Release::JIM_0_81),
    ..EXPR_JIM_0_80
};

/// Jim 0.84: `=*` and `=~` join the symbolic set and the precedence
/// table at `eq`/`ne`'s level.
const EXPR_JIM_0_84: ExprGrammar = ExprGrammar {
    precedence: JIM_PRECEDENCE_0_84,
    mathfuncs: jim_set(Release::JIM_0_84),
    ..EXPR_JIM_0_81
};

/// The expr grammar of `release` on `family`'s ladder — the resolution
/// [`CoreProfile::expr`](crate::model::family::CoreProfile) returns.
///
/// # Panics
/// If `release` does not sit on `family`'s ladder, exactly as
/// [`crate::model::family::grammar`] panics.
#[must_use]
pub fn expr(family: Family, release: Release) -> &'static ExprGrammar {
    assert!(
        release.family() == family,
        "release is not on this family's ladder"
    );
    match family {
        Family::Tcl => match release.ordinal() {
            0 => &EXPR_TCL84,
            1 => &EXPR_TCL85,
            2 => &EXPR_TCL86,
            3 => &EXPR_TCL90,
            _ => &EXPR_TCL91,
        },
        Family::F5Tcl => &EXPR_F5_TCL,
        Family::F5Irules => &EXPR_IRULES,
        // The Jim ladder as a ladder: five values, each a struct update
        // over the one before, where the old model needed nine profiles.
        Family::Jim => match release.ordinal() {
            0 => &EXPR_JIM_0_76,
            1..=3 => &EXPR_JIM_0_77,
            4 => &EXPR_JIM_0_80,
            5..=7 => &EXPR_JIM_0_81,
            _ => &EXPR_JIM_0_84,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TclVersion;
    use crate::grammar::EXPR_WORD_OPERATORS;

    #[test]
    fn tcl_precedence_matches_the_parser_transcription() {
        // Spot rows against `binary_bp` in tcl-syntax's expr parser.
        let t = &TCL_PRECEDENCE;
        assert_eq!(t.lookup("||"), Some((4, 5)));
        assert_eq!(t.lookup("&&"), Some((6, 7)));
        assert_eq!(t.lookup("eq"), Some((14, 15)));
        assert_eq!(t.lookup("=="), Some((14, 15)));
        assert_eq!(t.lookup("in"), Some((16, 17)));
        assert_eq!(t.lookup("lt"), Some((16, 17)));
        assert_eq!(t.lookup("+"), Some((20, 21)));
        assert_eq!(t.lookup("**"), Some((23, 23)), "** is right-associative");
        // The Tcl family never binds the iRules extension words.
        assert_eq!(t.lookup("contains"), None);
        assert_eq!(t.lookup("and"), None);
        // `not` is unary everywhere; a binary table has no row for it.
        assert_eq!(t.lookup("not"), None);
    }

    #[test]
    fn f5_trunk_precedence_extends_tcl_with_the_word_forms() {
        let i = &F5_TCL_PRECEDENCE;
        for op in [
            "contains",
            "starts_with",
            "ends_with",
            "equals",
            "matches",
            "matches_glob",
            "matches_regex",
        ] {
            assert_eq!(i.lookup(op), Some((14, 15)), "{op}");
        }
        assert_eq!(i.lookup("or"), Some((4, 5)));
        assert_eq!(i.lookup("and"), Some((6, 7)));
        assert_eq!(i.lookup("not"), None, "not is unary");
        // The shared C-Tcl rows are unchanged — except the TIP 201/461
        // word operators, which the fork point's 8.4.6 core never binds
        // and the family table therefore never lists.
        for &(op, l, r) in TCL_PRECEDENCE_ROWS {
            if matches!(op, "in" | "ni" | "lt" | "le" | "gt" | "ge") {
                assert_eq!(i.lookup(op), None, "{op} is not an F5 operator");
                continue;
            }
            assert_eq!(i.lookup(op), Some((l, r)), "{op}");
        }
    }

    /// The §3.1 motivating divergence: `expr {"a" eq "b" == 1}` parses as
    /// `("a" eq "b") == 1` under Tcl (eq and == share a level, hence
    /// left-to-right) and as `"a" eq ("b" == 1)` under Jim (eq binds
    /// looser than ==).
    #[test]
    fn jim_splits_the_comparison_levels_tcl_merges() {
        let (tcl_eq, _) = TCL_PRECEDENCE.lookup("eq").unwrap();
        let (tcl_eqeq, _) = TCL_PRECEDENCE.lookup("==").unwrap();
        assert_eq!(tcl_eq, tcl_eqeq);

        let jim = expr(Family::Jim, Release::JIM_0_84).precedence;
        let (jim_in, _) = jim.lookup("in").unwrap();
        let (jim_eq, _) = jim.lookup("eq").unwrap();
        let (jim_eqeq, _) = jim.lookup("==").unwrap();
        let (jim_lt, _) = jim.lookup("lt").unwrap();
        let (jim_sym_lt, _) = jim.lookup("<").unwrap();
        assert!(jim_in < jim_eq);
        assert!(jim_eq < jim_eqeq);
        assert!(jim_eqeq < jim_lt);
        assert!(jim_lt < jim_sym_lt);
        // Four distinct comparison levels where C Tcl has two.
        let levels: std::collections::BTreeSet<u16> =
            [jim_in, jim_eq, jim_eqeq, jim_lt, jim_sym_lt]
                .into_iter()
                .collect();
        assert_eq!(levels.len(), 5, "Jim splits what Tcl merges");
        assert_eq!(
            [
                TCL_PRECEDENCE.lookup("eq").unwrap().0,
                TCL_PRECEDENCE.lookup("in").unwrap().0,
            ]
            .into_iter()
            .collect::<std::collections::BTreeSet<u16>>()
            .len(),
            2,
            "C Tcl has exactly two"
        );
    }

    /// Every Jim binding power is twice its `OPRINIT` precedence, and the
    /// whole table — not just the comparison block — is now present, so
    /// `lookup` no longer answers `None` for the arithmetic and bitwise
    /// scaffold the design left for P6.
    #[test]
    fn the_jim_table_is_the_whole_oprinit_table() {
        let g = expr(Family::Jim, Release::JIM_0_84);
        for (op, precedence) in [
            ("||", 9u16),
            ("&&", 10),
            ("|", 48),
            ("^", 49),
            ("&", 50),
            ("in", 55),
            ("eq", 60),
            ("=*", 60),
            ("=~", 60),
            ("==", 70),
            ("lt", 75),
            ("<", 80),
            ("<<", 90),
            ("<<<", 90),
            ("+", 100),
            ("*", 110),
        ] {
            assert_eq!(
                g.precedence.lookup(op),
                Some((2 * precedence, 2 * precedence + 1)),
                "{op}"
            );
        }
        // `**` is right-associative (`OP_RIGHT_ASSOC`), so right == left.
        assert_eq!(g.precedence.lookup("**"), Some((240, 240)));
        // The unary and ternary rows are not binary infix operators.
        for op in ["!", "~", "?", ":"] {
            assert_eq!(g.precedence.lookup(op), None, "{op}");
        }
    }

    /// The one row Jim ever moved: `**` was 250 and **left**-associative
    /// at 0.76, and 120 and right-associative from 0.77. A per-*family*
    /// table (which is what §3.1 sketched) could hold neither change.
    ///
    /// Measured on `jimsh` built from the upstream tags:
    /// `expr {-2 ** 2}` is -4 at 0.76 and 4 at 0.79 — the unary minus
    /// (150) overtakes `**` when it drops to 120 — and
    /// `expr {2 ** 3 ** 2}` is 64 at 0.76 and 512 at 0.79.
    #[test]
    fn jim_moved_exactly_one_precedence_on_its_ladder() {
        let old = expr(Family::Jim, Release::JIM_0_76).precedence;
        assert_eq!(
            old.lookup("**"),
            Some((500, 501)),
            "left-associative at 0.76: `2 ** 3 ** 2` is 64"
        );
        // Above the unary operators (150 → 300 on the doubled scale), so
        // `-2 ** 2` groups as -(2 ** 2).
        assert!(old.lookup("**").expect("row").0 > 300);
        for release in [Release::JIM_0_77, Release::JIM_0_80, Release::JIM_0_84] {
            let table = expr(Family::Jim, release).precedence;
            assert_eq!(
                table.lookup("**"),
                Some((240, 240)),
                "{release}: right-associative — `2 ** 3 ** 2` is 512"
            );
            assert!(
                table.lookup("**").expect("row").0 < 300,
                "{release}: below the unary operators — `-2 ** 2` is 4"
            );
        }
        // Every other row is stable across the whole ladder.
        let base = expr(Family::Jim, Release::JIM_0_76).precedence;
        for &(op, left, right) in base.rows() {
            if op == "**" {
                continue;
            }
            assert_eq!(
                expr(Family::Jim, Release::JIM_0_84).precedence.lookup(op),
                Some((left, right)),
                "{op}"
            );
        }
    }

    /// **Correction to §3.1's prose.** The design says Jim shares
    /// `lt`/`le`/`gt`/`ge` with Tcl "across every modelled release"; the
    /// `OPRINIT` table says they arrive at 0.80. Offering them under
    /// `jim 0.78` would be offering a syntax error.
    #[test]
    fn jim_string_relationals_arrive_at_0_80() {
        for release in [
            Release::JIM_0_76,
            Release::JIM_0_77,
            Release::JIM_0_78,
            Release::JIM_0_79,
        ] {
            let g = expr(Family::Jim, release);
            assert_eq!(g.word_operators.len(), 4, "{release}");
            for op in ["lt", "le", "gt", "ge"] {
                assert!(!g.has_word_operator(op), "{release}: {op}");
                assert_eq!(g.precedence.lookup(op), None, "{release}: {op}");
            }
            // What it does have, from the start.
            for op in ["eq", "ne", "in", "ni"] {
                assert!(g.has_word_operator(op), "{release}: {op}");
            }
        }
        for release in [Release::JIM_0_80, Release::JIM_0_84] {
            let g = expr(Family::Jim, release);
            assert_eq!(g.word_operators.len(), 8, "{release}");
            for op in ["lt", "le", "gt", "ge"] {
                assert!(g.has_word_operator(op), "{release}: {op}");
                assert_eq!(g.precedence.lookup(op), Some((150, 151)), "{release}");
            }
        }
    }

    /// The mathfunc **set**, from the `OP_FUNC` rows of
    /// `Jim_ExprOperators`: twenty-six from 0.77 (the count §3.1 pins),
    /// twenty-three at 0.76, and the five C Tcl 8.5 functions Jim simply
    /// never had.
    #[test]
    fn the_jim_mathfunc_set_is_measured_not_guessed() {
        let g84 = expr(Family::Jim, Release::JIM_0_84);
        assert_eq!(g84.mathfuncs.names().count(), 26);
        assert_eq!(
            expr(Family::Jim, Release::JIM_0_76)
                .mathfuncs
                .names()
                .count(),
            23,
            "atan2/hypot/fmod arrive at 0.77"
        );
        for late in ["atan2", "hypot", "fmod"] {
            assert!(
                !expr(Family::Jim, Release::JIM_0_76)
                    .mathfuncs
                    .contains(late)
            );
            assert!(
                expr(Family::Jim, Release::JIM_0_77)
                    .mathfuncs
                    .contains(late)
            );
        }
        // §3.1's five absentees, confirmed against the source table.
        for absent in ["entier", "bool", "min", "max", "isqrt"] {
            assert!(!g84.mathfuncs.contains(absent), "{absent}");
            // …and every one of them is a real C Tcl 8.5 function, which
            // is exactly why a floor model would have offered it.
            assert!(EXPR_TCL86.mathfuncs.contains(absent), "{absent}");
        }
        // The build split: seven rows outside `#ifdef
        // JIM_MATH_FUNCTIONS`, nineteen inside it.
        let unguarded: Vec<&str> = g84
            .mathfuncs
            .names()
            .filter(|name| !g84.mathfuncs.get(name).expect("row").needs_math_extension)
            .collect();
        assert_eq!(
            unguarded,
            ["int", "wide", "abs", "double", "round", "rand", "srand"]
        );
        // Nothing in the Tcl or F5 tables is build-gated.
        for g in [&EXPR_TCL91, &EXPR_F5_TCL] {
            for name in g.mathfuncs.names() {
                assert!(
                    !g.mathfuncs.get(name).expect("row").needs_math_extension,
                    "{name}"
                );
            }
        }
    }

    #[test]
    fn tcl_word_operators_match_the_shipping_table() {
        // The per-release slices agree with `EXPR_WORD_OPERATORS` and its
        // release floors.
        for &(spelling, since) in EXPR_WORD_OPERATORS {
            let release = match since {
                TclVersion::V8_4 => Release::TCL_8_4,
                TclVersion::V8_5 => Release::TCL_8_5,
                _ => Release::TCL_9_0,
            };
            for (grammar, ceiling) in [
                (&EXPR_TCL84, Release::TCL_8_4),
                (&EXPR_TCL85, Release::TCL_8_5),
                (&EXPR_TCL86, Release::TCL_8_6),
                (&EXPR_TCL90, Release::TCL_9_0),
                (&EXPR_TCL91, Release::TCL_9_1),
            ] {
                assert_eq!(
                    grammar.has_word_operator(spelling),
                    release.ordinal() <= ceiling.ordinal(),
                    "{spelling} at {ceiling}"
                );
            }
        }
        assert_eq!(EXPR_TCL84.word_operators.len(), 2);
        assert_eq!(EXPR_TCL85.word_operators.len(), 4);
        assert_eq!(EXPR_TCL91.word_operators.len(), 8);
    }

    /// The word operators are an `f5-tcl` **trunk** fact — measured in
    /// tmsh and iApp contexts too, not iRules-only (measurements §4a) —
    /// and the offshoot answers with the trunk's grammar along the fork
    /// edge.
    #[test]
    fn f5_trunk_words_are_the_ten_plus_the_fork_point_core() {
        let trunk = expr(Family::F5Tcl, Release::F5_TCL_TMOS);
        let offshoot = expr(Family::F5Irules, Release::F5_IRULES_TMM);
        assert_eq!(trunk, offshoot, "the offshoot overrides no expr axis");
        for g in [trunk, offshoot] {
            for op in [
                "and",
                "or",
                "not",
                "contains",
                "starts_with",
                "ends_with",
                "equals",
                "matches",
                "matches_glob",
                "matches_regex",
            ] {
                assert!(g.has_word_operator(op), "{op}");
            }
            assert!(g.has_word_operator("eq"));
            // The fork point is 8.4: no TIP 201/461 operators.
            assert!(!g.has_word_operator("in"));
            assert!(!g.has_word_operator("lt"));
            assert_eq!(g.word_operators.len(), 12);
            assert_eq!(g.arity, ExprArity::Concatenating);
            assert_eq!(g.numbers, NumberSyntax::Tcl84);
            assert_eq!(g.comments, ExprCommentStyle::None);
        }
        // The ten extension operators carry the trunk's own release as
        // provenance; `eq`/`ne` carry the fork parent's.
        for w in trunk.word_operators {
            let expected = if matches!(w.spelling, "eq" | "ne") {
                Release::TCL_8_4
            } else {
                Release::F5_TCL_TMOS
            };
            assert_eq!(w.since, expected, "{}", w.spelling);
        }
    }

    #[test]
    fn mathfunc_sets_are_sets_not_floors() {
        let s84 = tcl_set(Release::TCL_8_4);
        let s85 = tcl_set(Release::TCL_8_5);
        let s90 = tcl_set(Release::TCL_9_0);
        let s91 = tcl_set(Release::TCL_9_1);
        assert!(s84.contains("sqrt"));
        assert!(!s84.contains("min"));
        assert!(s85.contains("min"));
        assert!(!s85.contains("isnan"));
        assert!(s90.contains("isnan"));
        assert!(!s90.contains("cbrt"));
        assert!(s91.contains("cbrt"));
        assert!(!s91.contains("no-such"));
        // Counts per release, matching `added_in`'s grouping: 26 in the
        // 8.4 C table, +5 (TIP 232), +6 (TIP 521), +21 (TIP 745).
        assert_eq!(s84.names().count(), 26);
        assert_eq!(s85.names().count(), 31);
        assert_eq!(tcl_set(Release::TCL_8_6).names().count(), 31);
        assert_eq!(s90.names().count(), 37);
        assert_eq!(s91.names().count(), 58);
        // The F5 tree carries the fork point's 8.4 set.
        assert_eq!(EXPR_F5_TCL.mathfuncs.names().count(), 26);
        assert!(!EXPR_F5_TCL.mathfuncs.contains("min"));
    }

    #[test]
    fn jim_symbolic_operators_are_release_gated() {
        let g = expr(Family::Jim, Release::JIM_0_84);
        assert!(g.symbolic_operators.contains(&("<<<", Release::JIM_0_76)));
        assert!(g.symbolic_operators.contains(&("=*", Release::JIM_0_84)));
        assert!(g.symbolic_operators.contains(&("=~", Release::JIM_0_84)));
        // Tcl and the F5 tree have no symbolic extensions beyond the
        // shared C-Tcl set.
        assert!(
            expr(Family::Tcl, Release::TCL_9_1)
                .symbolic_operators
                .is_empty()
        );
        assert!(EXPR_F5_TCL.symbolic_operators.is_empty());
    }

    /// Measured: `expr 1 + 2` answers 3 on `jimsh 0.76` and `0.79` and
    /// is `wrong # args: should be "expr expression"` on 0.81 and 0.84.
    #[test]
    fn jim_arity_flips_at_0_81() {
        for release in [Release::JIM_0_76, Release::JIM_0_80] {
            let g = expr(Family::Jim, release);
            assert_eq!(g.arity, ExprArity::Concatenating, "{release}");
            assert_eq!(g.comments, ExprCommentStyle::None, "{release}");
        }
        for release in [Release::JIM_0_81, Release::JIM_0_84] {
            let g = expr(Family::Jim, release);
            assert_eq!(g.arity, ExprArity::ExactlyOne, "{release}");
            assert_eq!(g.comments, ExprCommentStyle::Hash, "{release}");
        }
        // C Tcl still concatenates in 9.1.
        assert_eq!(
            expr(Family::Tcl, Release::TCL_9_1).arity,
            ExprArity::Concatenating
        );
    }

    #[test]
    fn every_binary_word_operator_has_a_precedence_row() {
        for family in crate::model::family::Family::ALL {
            for &release in family.releases() {
                let g = expr(family, release);
                for w in g.word_operators {
                    if w.spelling == "not" {
                        continue; // unary
                    }
                    assert!(
                        g.precedence.lookup(w.spelling).is_some(),
                        "{family} {release}: {} has no binding power",
                        w.spelling
                    );
                }
                // A symbolic row carries its own introducing release, so
                // the table must bind it exactly from that release on —
                // and must *not* bind it before (a `=~` binding power
                // under `jim 0.80` would be an operator the core has no
                // lexeme for). P6's full `OPRINIT` transcription is what
                // lets this be an equality rather than an exemption
                // list.
                for &(spelling, since) in g.symbolic_operators {
                    assert_eq!(
                        g.precedence.lookup(spelling).is_some(),
                        since.ordinal() <= release.ordinal(),
                        "{family} {release}: {spelling}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_modelled_family_interpolates() {
        for family in Family::ALL {
            for &release in family.releases() {
                assert_eq!(
                    expr(family, release).substitution,
                    ExprSubstitution::Interpolating
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "not on this family's ladder")]
    fn expr_rejects_a_release_from_another_ladder() {
        let _ = expr(Family::Jim, Release::TCL_8_6);
    }
}
