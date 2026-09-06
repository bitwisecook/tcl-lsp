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

//! Grammar-aware Tcl script generator for differential fuzzing.
//!
//! Scoped to the surface the native VM implements so a divergence points at a
//! real miscompile rather than an unimplemented command. Every emitted script:
//!
//! * is syntactically valid Tcl — a *complete script*, which is not the same
//!   as balanced delimiter counts once a generated word's value can be `{`;
//! * is **pure** — no I/O, file, socket, exec, clock, or `after` commands;
//! * has **bounded** loops (literal integer bounds), so neither backend hangs;
//! * prints deterministic output via `puts`, so the differential has something
//!   to compare.
//!
//! Two productions exist because the generator was *structurally* unable to
//! reach a bug class, rather than reaching it rarely — a distinction worth
//! keeping in mind before reading anything into a clean campaign. A
//! deliberately [`Malformed`] expression, because every generated expression
//! was well-formed by construction; and a [`WordShape`], because every
//! generated word came from [`WORDS`], where the spelling *is* the value. The
//! second was added after a 1500-script campaign returned 0 findings against a
//! build that answered `string index "{}" 0` as the empty string.
//!
//! The generator is parameterised by a [`GenConfig`] and a seed, so any finding
//! replays exactly.

use std::fmt::Write as _;

use crate::rng::Rng;

/// Tunables for the generator.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_field_names)] // `max_*` reads naturally for bound knobs.
pub struct GenConfig {
    /// Maximum nesting depth of control structures.
    pub max_depth: u32,
    /// Maximum number of top-level statements.
    pub max_stmts: usize,
    /// Maximum generated list length.
    pub max_list_len: usize,
    /// Maximum expression nesting depth.
    pub max_expr_depth: u32,
    /// How often a top-level statement is a deliberately **malformed**
    /// expression, in parts per thousand (so `20` = 2% of top-level
    /// statements). `0` disables the production entirely.
    ///
    /// Until this existed, `Gen::expr` and `Gen::expr_leaf` could only ever
    /// build a *well-formed* expression, so the fuzzer was structurally
    /// incapable of generating an `expr` syntax error and that whole bug class
    /// was invisible to every campaign — see the `Malformed` shape list.
    ///
    /// The default is deliberately low. A malformed expression is emitted
    /// uncaught, so it aborts the script and the statements after it never
    /// run; at 2% per statement (mean ~5 top-level statements per script)
    /// roughly one script in ten carries one, which reaches the class often
    /// enough for a routine campaign to cover it while leaving the large
    /// majority of the budget on well-formed semantics. Turn it up for a
    /// campaign aimed at this class, or to `0` to opt out — at `0` no random
    /// draw is consumed either, so a rate-`0` campaign reproduces the
    /// pre-existing generator stream exactly and historical findings still
    /// replay from their recorded seeds.
    pub malformed_expr_permille: u32,
    /// How often a generated word is given a **delimiter-shaped value** — a
    /// value that looks like a braced word without being one (`"{}"`, `"{$x}"`,
    /// `\{\}`) — in parts per thousand.
    ///
    /// Until this existed the generator drew its words from [`WORDS`], none of
    /// which contains a brace, bracket, quote or backslash. It was therefore
    /// structurally incapable of producing a word whose *value* is delimiter
    /// shaped, and the whole word-value class was invisible: a 1500-script
    /// campaign found 0 findings against a build that provably answered
    /// `string index "{}" 0` as the empty string and `set v "{}"` as 0
    /// characters. Same failure mode as `malformed_expr_permille`, same
    /// remedy — see [`WordShape`] for what the shapes are and what each one
    /// caught.
    ///
    /// This class is much cheaper than a malformed expression: every shape is
    /// well-formed, so it does not abort the script and the statements after
    /// it still run. The rate is correspondingly higher — but only to the
    /// point where it still leaves most of the corpus to the rest of the
    /// grammar. At 60‰ a third of scripts carry a shape; 120‰ put it over half,
    /// which is more of the budget than one class should own.
    ///
    /// `0` opts out entirely, and consumes no random draw, so a rate-`0`
    /// campaign reproduces the pre-existing generator stream exactly and
    /// historical findings still replay from their recorded seeds.
    pub word_shape_permille: u32,
}

impl Default for GenConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_stmts: 10,
            max_list_len: 5,
            max_expr_depth: 3,
            malformed_expr_permille: 20,
            word_shape_permille: 60,
        }
    }
}

/// Variable names the generator draws from (kept small so reads usually hit a
/// previously-assigned variable).
const VARS: &[&str] = &["a", "b", "c", "d", "i", "j", "x", "y", "z"];

/// Short literal words used as list elements / string operands.
const WORDS: &[&str] = &["foo", "bar", "baz", "qux", "hello", "world", "abc", "x", ""];

/// Proc names the generator defines and later calls. Kept distinct from any Tcl
/// builtin so a redefinition never shadows a command both engines rely on.
const PROC_NAMES: &[&str] = &["p0", "p1", "p2", "helper", "compute"];

/// Single-segment namespace names. Deliberately no `::` prefixes / trailing
/// runs: namespace-name canonicalisation of multiple/trailing `::` is a known
/// VM gap, so steering clear of it keeps a divergence pointed at a real
/// miscompile rather than name-normalisation noise.
const NS_NAMES: &[&str] = &["n1", "n2", "ns"];

/// One way an expression can be malformed.
///
/// The list is **derived from C**, not invented: every `errCode = "..."` that
/// `ParseExpr` in `generic/tclCompExpr.c` can reach has a representative here
/// — `BADCHAR`, `PARTOP`, `BAREWORD`, `BADNUMBER`, `MISSING`, `UNBALANCED`,
/// `EMPTY`, `SURPRISE` — the sole omission being `NOMEM`, an out-of-memory
/// path no generated source can provoke. C's own coverage of these is the
/// `parseExpr-21.*` case family in `tests/parseExpr.test`. Two further shapes
/// cover the failures `expr` reports from the `::tcl::mathfunc` *command*
/// layer rather than the parser ([`Self::FunctionArity`],
/// [`Self::UnknownFunction`]): from outside, a caller cannot tell which layer
/// refused the expression, and both are `expr` failure modes a real script
/// hits.
///
/// **Every shape is a well-formed Tcl word.** The malformed-ness is confined
/// to the *expression* grammar. A shape that instead broke `Tcl_ParseCommand`
/// — an unterminated brace or quote, or a trailing backslash — would make the
/// whole *script* unparsable, which is a different layer and a uniformly
/// uninteresting whole-script failure, so no shape here contains a brace,
/// bracket, double quote, or backslash. That is asserted over the real
/// generated corpus by `malformed_expressions_stay_inside_the_expr_grammar`.
///
/// Every shape's rendered form was checked against a real `tclsh9.0.4`: each
/// is `[info complete]` as a script *and* raises the C error family its
/// documentation names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Malformed {
    /// `1 +` — a binary operator with no right operand. C: `MISSING`,
    /// "missing operand" (parseExpr-21.22, -21.25).
    TrailingOperator,
    /// `1 2` — two adjacent operands with no operator between them. C:
    /// `MISSING`, "missing operator" (parseExpr-21.6).
    MissingOperator,
    /// The empty expression. C: `EMPTY`, "empty expression" (parseExpr-21.19).
    EmptyExpression,
    /// `foo bar baz` — an unquoted bareword. C: `BAREWORD`, "invalid
    /// bareword" (parseExpr-21.3, -21.4).
    Bareword,
    /// `(1 + 2` — an unclosed open paren. C: `UNBALANCED`, "unbalanced open
    /// paren" (parseExpr-21.17, -21.26).
    UnbalancedOpenParen,
    /// `1 + 2)` — a close paren with no opener. C: `UNBALANCED`, "unbalanced
    /// close paren" (parseExpr-21.20, -21.29).
    UnbalancedCloseParen,
    /// `!` — a unary operator with no operand at all. C: `MISSING`, "missing
    /// operand".
    LoneUnary,
    /// `1 @ 2` — a character the expression lexer maps to `INVALID`. C:
    /// `BADCHAR`, "invalid character" (parseExpr-21.1, -21.34..-21.36).
    InvalidCharacter,
    /// `=` — the one-character prefix of a two-character operator. C:
    /// `PARTOP`, "incomplete operator" (parseExpr-21.2).
    IncompleteOperator,
    /// `()` — parens around nothing, outside a function argument list. C:
    /// `EMPTY`, "empty subexpression" (parseExpr-21.16).
    EmptySubexpression,
    /// `max(1, )` — a function argument list with a hole in it. C: `MISSING`
    /// / `UNBALANCED`, "missing function argument" (parseExpr-21.18, -21.21).
    MissingFunctionArgument,
    /// `sin(1, 2)` / `sin()` — a known math function called with the wrong
    /// number of arguments. Reported by the `::tcl::mathfunc` command layer:
    /// `TCL WRONGARGS`, "too many/not enough arguments for math function".
    FunctionArity,
    /// `nosuchfn(1)` — a function that does not exist. Also the command
    /// layer: `TCL LOOKUP COMMAND`, "invalid command name
    /// `tcl::mathfunc::…`".
    UnknownFunction,
    /// `1 : 2` — a `:` with no preceding `?`. C: `SURPRISE`, "unexpected
    /// operator \":\" without preceding \"?\"" (parseExpr-21.28).
    StrayColon,
    /// `1 , 2` — a `,` outside a function argument list. C: `SURPRISE`,
    /// "unexpected \",\" outside function argument list" (parseExpr-21.30..-21.32).
    StrayComma,
    /// `1 ? 2` — a ternary missing its `:` arm. C: `MISSING`, "missing
    /// operator \":\"" (parseExpr-21.27).
    MissingColon,
    /// `0o8` — a numeric literal whose digits do not fit its radix. C:
    /// `BADNUMBER`, "invalid octal/binary number" (parseExpr-21.7, -21.8).
    BadNumber,
    /// `# nope` — a body that is nothing but a TIP 582 expression comment.
    /// The comment is skipped, leaving nothing, so C reports `EMPTY`, "empty
    /// expression". Note this shape is Tcl-9-specific in its *wording*: 8.6
    /// has no expression comments and calls `#` an invalid character
    /// instead. Both releases error, which is what the status comparison
    /// needs; only an error-text comparison is version-sensitive here.
    CommentOnly,
}

impl Malformed {
    /// Every shape, so the generator draws uniformly over them and the tests
    /// can assert each one is reachable.
    const ALL: &'static [Self] = &[
        Self::TrailingOperator,
        Self::MissingOperator,
        Self::EmptyExpression,
        Self::Bareword,
        Self::UnbalancedOpenParen,
        Self::UnbalancedCloseParen,
        Self::LoneUnary,
        Self::InvalidCharacter,
        Self::IncompleteOperator,
        Self::EmptySubexpression,
        Self::MissingFunctionArgument,
        Self::FunctionArity,
        Self::UnknownFunction,
        Self::StrayColon,
        Self::StrayComma,
        Self::MissingColon,
        Self::BadNumber,
        Self::CommentOnly,
    ];
}

/// Barewords for [`Malformed::Bareword`]. Deliberately excludes every word C
/// Tcl *accepts* as a bareword operand — the boolean literals
/// (`true`/`false`/`yes`/`no`/`on`/`off`) and `inf`/`nan` — since those parse
/// fine (checked against `tclsh9.0.4`: `expr {true}` is `1`, not an error),
/// which would make the shape silently well-formed and the coverage claim
/// false.
const MALFORMED_BAREWORDS: &[&str] = &["foo", "bar", "baz", "qux", "hello"];

/// Characters C Tcl's expression lexer maps to `INVALID`, for
/// [`Malformed::InvalidCharacter`]. Restricted to characters that are also
/// safe inside a braced Tcl word: no brace or bracket (word-level breakage),
/// no double quote, and no backslash (which would escape the word's closing
/// brace and break `Tcl_ParseCommand` instead). `$` is excluded for a
/// different reason: `$` *followed by a name* is a variable read, so it only
/// reaches `BADCHAR` in some positions — these three do so in every position.
const MALFORMED_BADCHARS: &[&str] = &["@", "`", "'"];

/// Binary operators used to build [`Malformed::TrailingOperator`]. Includes
/// the word operators (`eq`/`ne`/`in`/`ni`/`lt`/`gt`) as well as the symbolic
/// ones, so the dangling-operand shape covers both lexeme kinds.
const MALFORMED_BINARY_OPS: &[&str] = &[
    "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "&&", "||", "&", "|", "^", "<<",
    ">>", "**", "eq", "ne", "in", "ni", "lt", "gt",
];

/// One way a word's **value** can differ from its source spelling.
///
/// The generator's own word pool ([`WORDS`]) is nine short alphanumeric words:
/// for every one of them the spelling *is* the value, so an emitter that
/// confuses the two answers correctly by accident. Every shape here breaks
/// that identity, which is the only way the confusion becomes observable.
///
/// Three groups, each derived from measured divergences rather than invented:
///
/// * **Delimiter-shaped** — the value looks like a braced word without being
///   one. A de-quoted word is a value, and handing it back to the VM's
///   `subst_word` reads it as source a second time, so a value that merely
///   looks braced lost a brace layer and one that looks braced *and* still
///   carries a marker had the marker eaten (#1893).
/// * **Escape-bearing** — the value is what the escapes decode to, not the
///   text that spells them. This is the release-parameterised escape grammar
///   (#1479), and it is also where the word *boundary* moves: `a\ b` is one
///   word, not two.
/// * **Non-ASCII and whitespace-bearing** — the value's character count is not
///   its byte count, and a value carrying a space, a `#`, or nothing at all is
///   the case list quoting exists for.
///
/// Each group's variants say what they caught. Where a shape is a control
/// rather than a catch, it says that too.
///
/// **Every shape is a single well-formed Tcl word**, so it drops into any word
/// position without changing the shape of the surrounding script — the same
/// discipline [`Malformed`] keeps for the expression grammar. Shapes that are
/// not brace-balanced (`"{"`, `"}"`) are additionally kept out of the contexts
/// that wrap the word in braces; see [`Self::brace_balanced`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordShape {
    /// `"{}"` — the value `{}`. `string index "{}" 0` answered the empty
    /// string where both oracles say `{`, and `set v "{}"` stored 0 characters
    /// where both say 2.
    QuotedEmptyBraces,
    /// `"{abc}"` — the value `{abc}`. `string length` answered 3 for 5.
    QuotedBracedWord,
    /// `"{$x}"` — braced-shaped *and* still carrying a live variable. Pushed
    /// raw, the VM stripped the braces and handed back the inside
    /// unsubstituted: `puts "{$x}"` printed `${x}`.
    QuotedBracedVar,
    /// `"{[…]}"` — the command-substitution half of the same shape.
    QuotedBracedCmd,
    /// `"{}$x"` — a composite word whose *literal fragment* is brace-shaped.
    /// The fragment is already decoded, so re-substituting it stripped the
    /// braces off it: `string length "{}$x"` answered 1 for 3.
    QuotedFragmentBraces,
    /// `"{} {}"` — brace-shaped but *not* a whole braced word, so the VM would
    /// never have stripped it. The negative control for every shape above.
    QuotedBraceRun,
    /// `"{"` — an unbalanced single brace as a value.
    QuotedOpenBrace,
    /// `"}"` — the same, closing.
    QuotedCloseBrace,
    /// `\{\}` — braces reached by escape rather than by quoting: the same word
    /// one step later, finished once its escapes are decoded. `[format %s \{\}]`
    /// froze the 4-byte source spelling where both oracles give 2.
    EscapedBraces,
    /// `"\{\}"` — both routes at once.
    QuotedEscapedBraces,
    /// `{{}}` — a *genuinely* braced word, which must still lose exactly one
    /// layer (issue #1602). The positive control: the fix for every shape
    /// above must not disturb this one.
    BracedDoubleGroup,
    /// `{a[…]}` — a braced word whose brackets are data. A engine that treats
    /// the de-braced value as source *runs* the command, so the shape names a
    /// command that does not exist and the divergence is loud.
    BracedCmdIsData,
    /// `{a${…}}` — the variable half of the same control, naming a variable
    /// that does not exist.
    BracedVarIsData,
    /// `"{a}b{c}"` — two brace groups in one value, so the first `{` does not
    /// match the last `}`. Distinguishes a real balance walk from a two-byte
    /// `starts_with('{') && ends_with('}')` test, which is the bug that made
    /// `switch -- "{}$x" …` refuse a script both oracles run.
    QuotedTwoGroups,

    // -- escape-bearing --
    /// `a\ b` — one word whose value carries a space, because the backslash
    /// escapes the separator. The word *boundary* is the thing under test:
    /// `string length a\ b` answers `wrong # args` where both oracles say 3,
    /// so the escaped space is being read as a word break.
    EscapedSpace,
    /// `\x41` — a hex escape; the value is one character, the spelling four.
    /// Hex escape width is a release axis (#1479).
    EscapedHex,
    /// `\101` — the octal spelling of the same character, whose width is a
    /// release axis of its own.
    EscapedOctal,
    /// `\\` — an escaped backslash: the value is one byte the *next* decode
    /// would consume if a path decoded twice.
    EscapedBackslash,
    /// `a\tb` — an escape in the middle of a word, so the value carries a
    /// control character no spelling of it contains.
    EscapedTab,

    // -- non-ASCII and whitespace-bearing --
    /// `"café"` — a value whose character count (4) is not its byte count (5).
    /// `[format %s "café"]` answered 5 characters and printed mojibake, having
    /// walked the quoted word a byte at a time.
    NonAscii,
    /// `"{café}"` — non-ASCII crossed with the brace shape, so a byte-wise
    /// walk and a brace walk have to be right at the same time.
    NonAsciiBraced,
    /// `"a b"` — a value carrying a space, which is what list quoting exists
    /// for: `[list "a b"]` is `{a b}`, and that fold has been wrong before.
    QuotedSpace,
    /// `" "` — nothing but a separator.
    QuotedSpaceOnly,
    /// `""` — the empty value, which a list or dict element can collapse.
    QuotedEmpty,
    /// `"#"` — the character list quoting brace-wraps at element position 0
    /// and nowhere else (`[list #]` is `{#}`, `[list a #]` is `a #`), a rule
    /// the `dict create` fold's own documentation calls out.
    QuotedHash,
}

impl WordShape {
    /// Every shape, so the generator draws uniformly over them and the tests
    /// can assert each one is reachable.
    const ALL: &'static [Self] = &[
        Self::QuotedEmptyBraces,
        Self::QuotedBracedWord,
        Self::QuotedBracedVar,
        Self::QuotedBracedCmd,
        Self::QuotedFragmentBraces,
        Self::QuotedBraceRun,
        Self::QuotedOpenBrace,
        Self::QuotedCloseBrace,
        Self::EscapedBraces,
        Self::QuotedEscapedBraces,
        Self::BracedDoubleGroup,
        Self::BracedCmdIsData,
        Self::BracedVarIsData,
        Self::QuotedTwoGroups,
        Self::EscapedSpace,
        Self::EscapedHex,
        Self::EscapedOctal,
        Self::EscapedBackslash,
        Self::EscapedTab,
        Self::NonAscii,
        Self::NonAsciiBraced,
        Self::QuotedSpace,
        Self::QuotedSpaceOnly,
        Self::QuotedEmpty,
        Self::QuotedHash,
    ];

    /// The word's source spelling. `$x` is the generator's own seeded
    /// variable, which [`generate`] sets before any statement runs and
    /// [`Gen::word_shape_stmt`] re-seeds inside a proc body.
    const fn spelling(self) -> &'static str {
        match self {
            Self::QuotedEmptyBraces => "\"{}\"",
            Self::QuotedBracedWord => "\"{abc}\"",
            Self::QuotedBracedVar => "\"{$x}\"",
            Self::QuotedBracedCmd => "\"{[string length ab]}\"",
            Self::QuotedFragmentBraces => "\"{}$x\"",
            Self::QuotedBraceRun => "\"{} {}\"",
            Self::QuotedOpenBrace => "\"{\"",
            Self::QuotedCloseBrace => "\"}\"",
            Self::EscapedBraces => r"\{\}",
            Self::QuotedEscapedBraces => "\"\\{\\}\"",
            Self::BracedDoubleGroup => "{{}}",
            Self::BracedCmdIsData => "{a[nosuchcmd]}",
            Self::BracedVarIsData => "{a${nosuchvar}}",
            Self::QuotedTwoGroups => "\"{a}b{c}\"",
            Self::EscapedSpace => r"a\ b",
            Self::EscapedHex => r"\x41",
            Self::EscapedOctal => r"\101",
            Self::EscapedBackslash => r"\\",
            Self::EscapedTab => r"a\tb",
            Self::NonAscii => "\"café\"",
            Self::NonAsciiBraced => "\"{café}\"",
            Self::QuotedSpace => "\"a b\"",
            Self::QuotedSpaceOnly => "\" \"",
            Self::QuotedEmpty => "\"\"",
            Self::QuotedHash => "\"#\"",
        }
    }

    /// Whether the spelling's braces balance, which decides if it can be
    /// nested inside a braced word. `"{"` inside `expr {…}` would close the
    /// *expression's* brace and break `Tcl_ParseCommand`, turning a
    /// word-value probe into a uniformly uninteresting unparsable script —
    /// the same trap [`Malformed`] documents for its own shapes.
    const fn brace_balanced(self) -> bool {
        !matches!(self, Self::QuotedOpenBrace | Self::QuotedCloseBrace)
    }
}

/// Where a [`WordShape`] is placed. The shape decides what the value *is*; the
/// context decides which emitter resolves it, and the two cross — which is the
/// point. Each context was reached by a real defect: the assignment emitter,
/// the two command-word emitters, the fold path, and the `expr`-operand arm
/// each had their own copy of the same mistake, so a shape that only ever
/// appeared in one position would have found one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordContext {
    /// `set _wv <word>` — the assignment value emitter.
    Assign,
    /// A builtin's argument — the outer command-word emitter.
    CommandArg,
    /// An argument of a command nested in `[…]` — the *inner* word emitter,
    /// which carried its own copy of the hole.
    NestedArg,
    /// An argument to a generated proc, so the value crosses a call boundary.
    ProcArg,
    /// Assigned inside a proc body, where the local-variable table is in play.
    ProcLocal,
    /// A `list` element, then read back out.
    ListElement,
    /// A `dict create` value, then read back out.
    DictValue,
    /// Appended to a variable.
    Append,
    /// A `switch` subject, which reaches codegen as an expression operand.
    SwitchSubject,
    /// Inside `expr {…}` — brace-balanced shapes only.
    ExprOperand,
    /// `[format %s <word>]` in a value position, which constant-folds.
    FormatFold,
    /// `[list <word>]` in a value position, which constant-folds.
    ListFold,
}

impl WordContext {
    const ALL: &'static [Self] = &[
        Self::Assign,
        Self::CommandArg,
        Self::NestedArg,
        Self::ProcArg,
        Self::ProcLocal,
        Self::ListElement,
        Self::DictValue,
        Self::Append,
        Self::SwitchSubject,
        Self::ExprOperand,
        Self::FormatFold,
        Self::ListFold,
    ];

    /// Whether this context nests the word inside a braced word, and so can
    /// only take a brace-balanced shape.
    ///
    /// Two do, for different reasons: `expr {…}` braces its expression, and a
    /// proc *body* is itself a braced word. Miss either and the shape's brace
    /// closes the enclosing word instead of being content — `proc _wl {} {…
    /// set v "{" …}` stops being a parsable script at all, which is a
    /// uniformly uninteresting whole-script failure rather than a word-value
    /// probe. The contexts that merely bracket the word (`[list …]`,
    /// `[format …]`, `[string length …]`) are unaffected: a brace inside a
    /// bracket word is content already.
    const fn braces_the_word(self) -> bool {
        matches!(self, Self::ExprOperand | Self::ProcLocal)
    }
}

/// Generate one script from `seed`.
#[must_use]
pub fn generate(seed: u64, config: &GenConfig) -> String {
    let mut g = Gen {
        rng: Rng::new(seed),
        config: *config,
        out: String::new(),
        procs: Vec::new(),
    };
    // Seed a few variables so later reads resolve, matching how real scripts
    // initialise state before using it.
    for v in ["a", "b", "i", "x"] {
        let _ = writeln!(g.out, "set {v} {}", g.rng.below(20));
    }
    let stmts = 1 + g.rng.below(g.config.max_stmts);
    for _ in 0..stmts {
        g.statement(0);
    }
    g.out
}

struct Gen {
    rng: Rng,
    config: GenConfig,
    out: String,
    /// Procs defined so far, as `(name, arity)`. Calls draw from this so a
    /// generated call always matches a real definition; a redefinition with a
    /// different arity replaces the old entry rather than accumulating.
    procs: Vec<(&'static str, usize)>,
}

impl Gen {
    fn var(&mut self) -> &'static str {
        self.rng.pick(VARS)
    }

    /// Emit one statement at nesting `depth`.
    fn statement(&mut self, depth: u32) {
        // A deliberately malformed expression, at the configured rate.
        //
        // Top-level only, for two reasons that both decide whether the
        // comparison is meaningful. Nested inside a `proc` body it would not
        // fire at all in C Tcl (which compiles a body lazily, at first call,
        // so a syntax error in an *uncalled* proc is never reported) while an
        // eagerly-compiling engine would error at definition time — a
        // status mismatch about compile *timing*, not about the expression.
        // Nested inside a `catch`/`try` body the error would be swallowed,
        // erasing the very status difference this production exists to
        // expose. At depth 0 it is neither deferred nor caught.
        //
        // Both guards are checked before `chance`, so a rate of 0 consumes no
        // random draw and leaves the generated stream byte-identical to what
        // it was before this production existed (see
        // `GenConfig::malformed_expr_permille`).
        if depth == 0
            && self.config.malformed_expr_permille > 0
            && self.rng.chance(self.config.malformed_expr_permille, 1000)
        {
            self.malformed_expr_stmt();
            return;
        }
        // A word-shape probe, at the configured rate. Top-level only, for one
        // of the two reasons the malformed production is: a probe nested in a
        // `catch`/`try` body would have its status divergence swallowed, and
        // `BracedCmdIsData` / `BracedVarIsData` announce themselves precisely
        // by erroring on an engine that substitutes what should be data.
        //
        // Checked before `chance`, so a rate of 0 consumes no random draw and
        // leaves the generated stream byte-identical to what it was before
        // this production existed (see `GenConfig::word_shape_permille`).
        if depth == 0
            && self.config.word_shape_permille > 0
            && self.rng.chance(self.config.word_shape_permille, 1000)
        {
            self.word_shape_stmt();
            return;
        }
        // At max depth, only emit leaf (non-nesting) statements.
        let leaf = depth >= self.config.max_depth || self.rng.chance(2, 3);
        if leaf {
            self.leaf_statement(depth);
        } else {
            // proc / namespace definitions are top-level only (matching the
            // oracle): nesting them changes scope semantics in ways the
            // generator doesn't model.
            let arms = if depth == 0 { 12 } else { 7 };
            match self.rng.below(arms) {
                0 => self.if_stmt(depth),
                1 => self.while_stmt(depth),
                2 => self.for_stmt(depth),
                3 => self.foreach_stmt(depth),
                4 => self.switch_stmt(depth),
                5 => self.catch_stmt(depth),
                6 => self.try_stmt(depth),
                7 => self.proc_stmt(depth),
                8 => self.namespace_stmt(depth),
                9 => self.inscope_stmt(),
                10 => self.dynamic_name_stmt(),
                11 => self.caller_frame_stmt(),
                _ => self.leaf_statement(depth),
            }
        }
    }

    fn leaf_statement(&mut self, depth: u32) {
        // A call to an already-defined proc, emitted only at top level so a
        // recursive definition can't drive unbounded generated recursion. The
        // call exercises the proc's return value through the differential.
        if depth == 0 && !self.procs.is_empty() && self.rng.chance(1, 4) {
            self.proc_call_stmt();
            return;
        }
        match self.rng.below(18) {
            0 => {
                let v = self.var();
                let e = self.expr(0);
                let _ = writeln!(self.out, "set {v} [expr {{{e}}}]");
            }
            1 => {
                let v = self.var();
                let _ = writeln!(self.out, "incr {v} {}", self.rng.below(5));
            }
            2 => {
                let v = self.var();
                let w = self.rng.pick(WORDS);
                let _ = writeln!(self.out, "append {v} {w}");
            }
            3 => {
                let v = self.var();
                let list = self.list_literal();
                let _ = writeln!(self.out, "set {v} {list}");
            }
            4 => {
                let v = self.var();
                let w = self.rng.pick(WORDS);
                let _ = writeln!(self.out, "lappend {v} {w}");
            }
            5 => {
                let s = self.string_op();
                let _ = writeln!(self.out, "puts [{s}]");
            }
            6 => {
                let l = self.list_op();
                let _ = writeln!(self.out, "puts [{l}]");
            }
            7 => {
                let e = self.expr(0);
                let _ = writeln!(self.out, "puts [expr {{{e}}}]");
            }
            8 => self.dict_mutator(),
            9 => {
                let d = self.dict_op();
                let _ = writeln!(self.out, "puts [{d}]");
            }
            10 => {
                let f = self.format_op();
                let _ = writeln!(self.out, "puts [{f}]");
            }
            11 => {
                let s = self.scan_op();
                let _ = writeln!(self.out, "puts [{s}]");
            }
            12 => self.array_stmt(),
            13 => {
                // `apply` a pure lambda to a small integer — result deterministic.
                let n = self.rng.below(20);
                let body =
                    *self
                        .rng
                        .pick(&["expr {$_p * 2}", "expr {$_p + 1}", "string length $_p"]);
                let _ = writeln!(self.out, "puts [apply {{{{_p}} {{{body}}}}} {n}]");
            }
            14 => {
                // `eval` a small assignment script (a list of words is a command).
                let v = self.var();
                let n = self.rng.below(20);
                let _ = writeln!(self.out, "eval [list set {v} {n}]");
            }
            15 => {
                // `subst` a string with an embedded variable read and command.
                let v = self.var();
                let _ = writeln!(self.out, "puts [subst {{${v}-[expr {{1+1}}]}}]");
            }
            16 => self.scope_stmt(),
            _ => {
                let r = self.regex_op();
                let _ = writeln!(self.out, "puts [{r}]");
            }
        }
    }

    /// A scoping statement: a one-shot proc that reaches an enclosing/global
    /// variable through `global` or `upvar`, then is called. The referenced
    /// variables (`a`/`b`) are seeded at top level, so the result is
    /// deterministic.
    fn scope_stmt(&mut self) {
        if self.rng.chance(1, 2) {
            // `global`: read module-global scalars from inside a proc.
            self.out
                .push_str("proc _gp {} {global a b\n return [list $a $b]}\n");
            self.out.push_str("puts [_gp]\n");
        } else {
            // `upvar`: alias the caller's variable (the global, at call level 1).
            let v = *self.rng.pick(&["a", "b", "i", "x"]);
            self.out
                .push_str("proc _up {vn} {upvar 1 $vn v\n return $v}\n");
            let _ = writeln!(self.out, "puts [_up {v}]");
        }
    }

    /// A `regexp` / `regsub` over a fixed, safe pattern and a word/variable
    /// subject — deterministic 0/1 (match) or substituted-string output.
    fn regex_op(&mut self) -> String {
        let pat = *self.rng.pick(&["[a-z]+", "[0-9]", "o+", "^ba", "a|o"]);
        let subject = if self.rng.chance(1, 2) {
            format!("${}", self.var())
        } else {
            self.nonempty_word().to_string()
        };
        match self.rng.below(3) {
            0 => format!("regexp {{{pat}}} {subject}"),
            1 => format!("regexp -all {{{pat}}} {subject}"),
            // `regsub` with an explicit result var (portable across 8.x/9.x),
            // printing the substituted string.
            _ => format!("regsub -all {{{pat}}} {subject} X _rs; set _rs"),
        }
    }

    /// An `array` statement: seed a scratch array with a literal, then read it
    /// back through a deterministic op. Enumeration (`array names`/`get`) is
    /// wrapped in `lsort` because Tcl's hash-iteration order is unspecified and
    /// legitimately differs between engines — only a *sorted* view is comparable.
    fn array_stmt(&mut self) {
        let k1 = self.nonempty_word();
        let k2 = self.nonempty_word();
        let (v1, v2) = (self.rng.below(20), self.rng.below(20));
        let _ = writeln!(self.out, "array set _arr {{{k1} {v1} {k2} {v2}}}");
        match self.rng.below(5) {
            0 => {
                let _ = writeln!(self.out, "puts [array size _arr]");
            }
            1 => {
                let _ = writeln!(self.out, "puts [array exists _arr]");
            }
            2 => {
                // A read of a key the seed guarantees is present.
                let _ = writeln!(self.out, "puts [set _arr({k1})]");
            }
            3 => {
                let _ = writeln!(self.out, "puts [lsort [array names _arr]]");
            }
            _ => {
                let _ = writeln!(self.out, "puts [lsort [array get _arr]]");
            }
        }
        // Drop the scratch array so a later re-seed with different keys doesn't
        // accumulate stale elements (which would desync the two engines' views).
        self.out.push_str("array unset _arr\n");
    }

    fn if_stmt(&mut self, depth: u32) {
        let cond = self.expr(0);
        let _ = writeln!(self.out, "if {{{cond}}} {{");
        self.block(depth + 1);
        if self.rng.chance(1, 2) {
            self.out.push_str("} else {\n");
            self.block(depth + 1);
        }
        self.out.push_str("}\n");
    }

    fn while_stmt(&mut self, depth: u32) {
        // Bounded: a fresh counter var counts up to a small literal.
        //
        // The counter name is qualified by `depth` — a hardcoded `_w` used
        // to collide across nesting levels, which was a genuine (if rare)
        // infinite-loop generator bug, not just a divergence risk: an inner
        // `while` also named `_w` resets the shared counter to 0 on every
        // outer iteration, so the outer loop's own `incr _w` (which only
        // ever sees the inner loop's fixed exit value + 1) can end up never
        // reaching the outer bound at all. Found via the differential
        // fuzzer's own generator running against a real `tclsh` while
        // verifying issue #983's mathfunc/operator additions — same-depth
        // sequential loops still safely reuse the same name, since they
        // never overlap in time.
        let n = 1 + self.rng.below(4);
        let _ = writeln!(self.out, "set _w{depth} 0\nwhile {{$_w{depth} < {n}}} {{");
        self.block(depth + 1);
        let _ = writeln!(self.out, "incr _w{depth}\n}}");
    }

    fn for_stmt(&mut self, depth: u32) {
        // Qualified by `depth` for the same nesting-collision reason as
        // `while_stmt` above (a `for` loop's own condition/increment clauses
        // make a bare collision self-correcting rather than an infinite
        // loop, but it still silently truncates the outer loop's iteration
        // count — still worth avoiding rather than documenting as fine).
        let n = 1 + self.rng.below(4);
        let _ = writeln!(
            self.out,
            "for {{set _f{depth} 0}} {{$_f{depth} < {n}}} {{incr _f{depth}}} {{"
        );
        self.block(depth + 1);
        self.out.push_str("}\n");
    }

    fn foreach_stmt(&mut self, depth: u32) {
        let v = self.var();
        let list = self.list_literal();
        let _ = writeln!(self.out, "foreach {v} {list} {{");
        self.block(depth + 1);
        self.out.push_str("}\n");
    }

    fn switch_stmt(&mut self, depth: u32) {
        // `switch -- <val>` over a couple of literal patterns plus a default,
        // so exactly one arm fires and the output is deterministic.
        let val = self.simple_value();
        let _ = writeln!(self.out, "switch -- {val} {{");
        let ncases = 1 + self.rng.below(3);
        for _ in 0..ncases {
            // Non-empty pattern word so the brace-form parse is unambiguous.
            let pat = self.nonempty_word();
            let _ = writeln!(self.out, "{pat} {{");
            self.block(depth + 1);
            self.out.push_str("}\n");
        }
        self.out.push_str("default {\n");
        self.block(depth + 1);
        self.out.push_str("}\n}\n");
    }

    fn catch_stmt(&mut self, depth: u32) {
        // `catch` of a small body, printing the return code (0/1) so the
        // differential compares the caught status, not just side effects.
        self.out.push_str("puts [catch {\n");
        self.block(depth + 1);
        self.out.push_str("} _cm]\n");
    }

    fn try_stmt(&mut self, depth: u32) {
        self.out.push_str("try {\n");
        self.block(depth + 1);
        self.out.push_str("} on error {_e} {\n");
        self.block(depth + 1);
        if self.rng.chance(1, 2) {
            self.out.push_str("} finally {\n");
            self.block(depth + 1);
        }
        self.out.push_str("}\n");
    }

    fn proc_stmt(&mut self, depth: u32) {
        let name = *self.rng.pick(PROC_NAMES);
        let nparams = self.rng.below(3);
        // Distinct parameter names so the signature is well-formed.
        let mut params: Vec<&'static str> = Vec::new();
        for &v in VARS {
            if params.len() >= nparams {
                break;
            }
            params.push(v);
        }
        let param_str = params.join(" ");
        let _ = writeln!(self.out, "proc {name} {{{param_str}}} {{");
        self.block(depth + 1);
        // A literal return value keeps the proc's result deterministic.
        let ret = self.simple_value();
        let _ = writeln!(self.out, "return {ret}");
        self.out.push_str("}\n");
        // Record (or refresh) the proc's arity for later calls.
        self.procs.retain(|(n, _)| *n != name);
        self.procs.push((name, params.len()));
    }

    fn proc_call_stmt(&mut self) {
        let procs = self.procs.clone();
        let (name, arity) = *self.rng.pick(&procs);
        let mut call = String::from(name);
        for _ in 0..arity {
            let _ = write!(call, " {}", self.rng.below(20));
        }
        let _ = writeln!(self.out, "puts [{call}]");
    }

    fn namespace_stmt(&mut self, depth: u32) {
        let ns = *self.rng.pick(NS_NAMES);
        let _ = writeln!(self.out, "namespace eval {ns} {{");
        self.block(depth + 1);
        self.out.push_str("}\n");
    }

    /// `namespace inscope ns script ?arg ...?` — the one member of the
    /// `Tcl_ConcatObj` eval family whose trailing words are appended as **list
    /// elements** rather than space-joined (`NamespaceInscopeCmd`,
    /// `generic/tclNamesp.c`), so a word holding whitespace or list punctuation
    /// must reach the invoked command as exactly one argument.
    ///
    /// The differential regression seed for issue #1056: the VM space-joined
    /// the tail, so `namespace inscope ns {p} {x y}` called `p` with two
    /// arguments instead of one. The probe proc reports both the argument
    /// *count* and each argument's text, so a wrong split is a stdout mismatch
    /// against the reference `tclsh` rather than a silent pass. A zero-word
    /// tail is drawn too, covering C's `objc == 3` arm (script evaluated
    /// verbatim, nothing appended).
    fn inscope_stmt(&mut self) {
        let ns = *self.rng.pick(NS_NAMES);
        let _ = writeln!(
            self.out,
            "namespace eval {ns} {{proc _shape {{args}} \
             {{return [llength $args]:[join $args ,]}}}}"
        );
        let mut call = format!("namespace inscope {ns} {{_shape}}");
        for _ in 0..self.rng.below(4) {
            let arg = self.inscope_arg();
            let _ = write!(call, " {arg}");
        }
        let _ = writeln!(self.out, "puts [{call}]");
    }

    /// Variable access whose *name* is computed at run time — `set $n v`,
    /// `[set $n]`, `lappend $n w`, `unset $n`, and a `subst` over a
    /// variable-held template.
    ///
    /// The compiler's SSA / dataflow treats these as whole-name-space
    /// barriers (issue #923 audit cluster C10,
    /// `tcl_compiler::dynamic_names`), which gates the constant-branch fold
    /// and dead-store elimination feeding this backend's input IR — so the
    /// differential needs the shapes present to catch a VM that resolves a
    /// computed name differently from C Tcl.
    ///
    /// Every arm keeps to a private `_dn*` name space the other productions
    /// never touch, so the emitted lines stay deterministic wherever they land
    /// in the script.  Output is the resolved value plus an `info exists`
    /// probe, so a wrong resolution is a stdout mismatch rather than a silent
    /// pass.  tclsh 9.0.4 / 8.6.14 agree on all five arms.
    fn dynamic_name_stmt(&mut self) {
        match self.rng.below(5) {
            // `set $n v` writes, `[set $n]` reads back — the argparse idiom.
            0 => self.out.push_str(
                "set _dnn _dnv\nset $_dnn hello\nputs [set $_dnn]\nputs [info exists _dnv]\n",
            ),
            // A read-modify-write through a computed name.
            1 => self.out.push_str(
                "set _dnn _dna\nlappend $_dnn one\nlappend $_dnn two\nputs [set $_dnn]\n",
            ),
            // `append` through a computed name.
            2 => self
                .out
                .push_str("set _dnn _dnb\nappend $_dnn ab\nappend $_dnn cd\nputs [set $_dnn]\n"),
            // A template held in a variable: `subst` expands names that only
            // exist in run-time data.
            3 => self
                .out
                .push_str("set _dnv 7\nset _dnt {$_dnv}\nputs [subst $_dnt]\n"),
            // A computed-name destroy, probed either side.
            _ => self.out.push_str(
                "set _dnn _dnu\nset $_dnn gone\nputs [info exists _dnu]\n\
                 unset $_dnn\nputs [info exists _dnu]\n",
            ),
        }
    }

    /// A procedure that writes its **caller's** frame — `upvar`'s three
    /// resolvable shapes, `uplevel`'s literal / constructed / opaque bodies,
    /// and the one-frame forward `uplevel 1 [list callee …]`.
    ///
    /// The compiler summarises these per procedure (issue #923 audit cluster
    /// C1, `tcl_compiler::cfg_builder::upvar_info`) and applies the summary
    /// at every call site, widening the caller's `defs` or raising a
    /// caller-frame barrier.  Both gate the constant-branch fold and
    /// dead-store elimination that feed this backend's input IR, so the
    /// differential needs the shapes present to catch a VM whose frame
    /// targeting differs from C Tcl's.
    ///
    /// Every arm keeps to a private `_cf*` name space the other productions
    /// never touch, and every proc is defined and called inside the same
    /// emitted run, so the lines stay deterministic wherever they land in the
    /// script.  Output is the injected value plus an `info exists` probe, so
    /// a write landing in the wrong frame is a stdout mismatch rather than a
    /// silent pass.  tclsh 9.0.4 / 8.6.14 agree on all six arms.
    fn caller_frame_stmt(&mut self) {
        match self.rng.below(6) {
            // `upvar 1 $param local` — the by-reference out-parameter.
            0 => self.out.push_str(
                "proc _cfa {n} {upvar 1 $n v; set v made}\n\
                 proc _cfa0 {} {_cfa _cfx; puts $_cfx; puts [info exists _cfx]}\n_cfa0\n",
            ),
            // `upvar 1 literal local` — a fixed caller-side name.
            1 => self.out.push_str(
                "proc _cfb {} {upvar 1 _cfy v; set v lit}\n\
                 proc _cfb0 {} {_cfb; puts $_cfy}\n_cfb0\n",
            ),
            // `upvar #0` reaches the global frame from any depth.
            2 => self.out.push_str(
                "proc _cfc {} {upvar #0 _cfg g; set g glob}\n\
                 proc _cfc0 {} {_cfc}\n_cfc0\nputs $::_cfg\n",
            ),
            // `uplevel 1 {literal}` — the body runs one frame up.
            3 => self.out.push_str(
                "proc _cfd {} {uplevel 1 {set _cfz up}}\n\
                 proc _cfd0 {} {_cfd; puts $_cfz}\n_cfd0\n",
            ),
            // `uplevel 1 [list set $name …]` — a constructed body naming its
            // command statically.
            4 => self.out.push_str(
                "proc _cfe {n} {uplevel 1 [list set $n cons]}\n\
                 proc _cfe0 {} {_cfe _cfw; puts $_cfw}\n_cfe0\n",
            ),
            // `uplevel 1 [list callee …]` forwards the callee's own `upvar 1`
            // one frame further out; the plain-call spelling does not.
            _ => self.out.push_str(
                "proc _cff {n} {upvar 1 $n v; set v fwd}\n\
                 proc _cff1 {n} {uplevel 1 [list _cff $n]}\n\
                 proc _cff2 {n} {_cff $n}\n\
                 proc _cff0 {} {_cff1 _cfv; puts $_cfv; \
_cff2 _cfu; puts [info exists _cfu]}\n_cff0\n",
            ),
        }
    }

    /// One trailing word for [`Gen::inscope_stmt`], chosen to exercise the
    /// list-element quoting on the way back out: whitespace (inner and
    /// padding), an empty element, and `$`/`[`/`;`/`"` — each of which forces
    /// quoting, so the word must not substitute, split, or terminate the
    /// command inside the built script.
    ///
    /// Every shape keeps `{}`/`[]` **byte-balanced**. An unbalanced-brace
    /// element (`a\{b`, the backslash-quoting path) is valid Tcl but trips the
    /// generator's balanced-delimiter invariant, which is a raw byte count —
    /// that path is pinned by the VM's own unit tests instead.
    fn inscope_arg(&mut self) -> &'static str {
        self.rng.pick(&[
            "{x y}",
            "{}",
            "{a b c}",
            "{  padded  }",
            "{a;b}",
            "{$nosub}",
            "{[nosub]}",
            "{a\"b}",
            "plain",
        ])
    }

    /// Mutate a dict-valued variable in place. An unset target is created by
    /// the first `dict set`, matching Tcl semantics; `dict incr` on a
    /// non-integer errors in both engines (so it stays a match, not a finding).
    fn dict_mutator(&mut self) {
        let v = self.var();
        let key = self.nonempty_word();
        match self.rng.below(5) {
            0 => {
                let val = self.nonempty_word();
                let _ = writeln!(self.out, "dict set {v} {key} {val}");
            }
            1 => {
                let val = self.nonempty_word();
                let _ = writeln!(self.out, "dict append {v} {key} {val}");
            }
            2 => {
                let _ = writeln!(self.out, "dict incr {v} {key} {}", self.rng.below(5));
            }
            3 => {
                let val = self.nonempty_word();
                let _ = writeln!(self.out, "dict lappend {v} {key} {val}");
            }
            _ => {
                let _ = writeln!(self.out, "dict unset {v} {key}");
            }
        }
    }

    /// A `dict` ensemble subcommand producing a value, over a literal dict.
    fn dict_op(&mut self) -> String {
        let d = self.dict_literal();
        match self.rng.below(5) {
            0 => format!("dict size {d}"),
            1 => format!("dict keys {d}"),
            2 => format!("dict values {d}"),
            3 => format!("dict exists {d} {}", self.nonempty_word()),
            // `dict get` of a present key (the literal always contains `foo`).
            _ => format!("dict get {d} foo"),
        }
    }

    /// A small literal dict. Always includes the `foo` key so a `dict get foo`
    /// resolves; remaining pairs are short non-empty words.
    fn dict_literal(&mut self) -> String {
        let mut s = String::from("{foo 1");
        let extra = self.rng.below(3);
        for _ in 0..extra {
            let k = self.nonempty_word();
            let val = self.rng.below(20);
            let _ = write!(s, " {k} {val}");
        }
        s.push('}');
        s
    }

    /// A non-empty short word (`""` excluded so it can't collapse a list/dict
    /// element or swallow a switch pattern).
    fn nonempty_word(&mut self) -> &'static str {
        loop {
            let w = *self.rng.pick(WORDS);
            if !w.is_empty() {
                break w;
            }
        }
    }

    /// A simple scalar value: a variable read or a small literal word/integer.
    ///
    /// At the configured rate this is a [`WordShape`] instead, which is how the
    /// shapes reach the two positions this producer feeds — a `switch` subject
    /// and a proc's return value — without the dedicated probe having to model
    /// either.
    ///
    /// Balanced shapes only, unconditionally: both positions can sit *inside*
    /// a braced word (a proc body always does, and a `switch` nested at any
    /// depth does), and this producer is not told its depth. Requiring balance
    /// is the cheap way to be right in every caller rather than the two that
    /// happen to be top-level today. The unbalanced shapes are not lost — they
    /// reach the ten non-bracing contexts through
    /// [`word_shape_stmt`](Self::word_shape_stmt).
    ///
    /// Checked before `chance`, so a rate of 0 consumes no draw.
    fn simple_value(&mut self) -> String {
        if self.config.word_shape_permille > 0
            && self.rng.chance(self.config.word_shape_permille, 1000)
        {
            return self.word_shape(true).spelling().to_owned();
        }
        match self.rng.below(3) {
            0 => format!("${}", self.var()),
            1 => self.rng.below(20).to_string(),
            _ => self.nonempty_word().to_string(),
        }
    }

    /// Emit a small block of 1–3 statements at `depth`.
    fn block(&mut self, depth: u32) {
        let n = 1 + self.rng.below(3);
        for _ in 0..n {
            self.statement(depth);
        }
    }

    /// A braced list literal of short words.
    fn list_literal(&mut self) -> String {
        let n = self.rng.below(self.config.max_list_len + 1);
        let mut s = String::from("{");
        for k in 0..n {
            if k > 0 {
                s.push(' ');
            }
            // Use a non-empty word so empty elements don't collapse.
            let w = loop {
                let w = self.rng.pick(WORDS);
                if !w.is_empty() {
                    break *w;
                }
            };
            s.push_str(w);
        }
        s.push('}');
        s
    }

    /// A `string` ensemble subcommand producing a value. Every form is
    /// deterministic (no locale/encoding-sensitive output) so a divergence is a
    /// real bug.
    fn string_op(&mut self) -> String {
        let w = self.nonempty_word();
        let w2 = self.nonempty_word();
        match self.rng.below(14) {
            0 => format!("string length {w}"),
            1 => format!("string toupper {w}"),
            2 => format!("string tolower {w}"),
            3 => format!("string totitle {w}"),
            4 => format!("string reverse {w}"),
            5 => format!("string index {w} {}", self.rng.below(4)),
            6 => format!("string range {w} 0 {}", self.rng.below(4)),
            7 => format!("string compare {w} {w2}"),
            8 => format!("string equal {w} {w2}"),
            9 => format!("string first {w2} {w}"),
            10 => format!("string last {w2} {w}"),
            11 => format!("string repeat {w} {}", self.rng.below(4)),
            12 => format!("string map {{{w} {w2}}} {w}"),
            _ => format!("string trim {w}{w2} {w}"),
        }
    }

    /// A `list`/`l*` operation producing a value.
    fn list_op(&mut self) -> String {
        let list = self.list_literal();
        match self.rng.below(11) {
            0 => format!("llength {list}"),
            1 => format!("lindex {list} {}", self.rng.below(self.config.max_list_len)),
            2 => format!("lreverse {list}"),
            3 => format!("lsort {list}"),
            4 => format!(
                "lrange {list} 0 {}",
                self.rng.below(self.config.max_list_len)
            ),
            5 => format!("lsearch {list} {}", self.nonempty_word()),
            6 => format!("concat {list} {}", self.list_literal()),
            7 => format!("join {list} -"),
            8 => format!(
                "linsert {list} {} {}",
                self.rng.below(self.config.max_list_len),
                self.nonempty_word()
            ),
            9 => format!(
                "lreplace {list} 0 {} {}",
                self.rng.below(self.config.max_list_len),
                self.nonempty_word()
            ),
            // `lmap` over a bounded literal list, mapping each element with a pure
            // expression — deterministic element order and values.
            _ => format!("lmap _e {list} {{string length $_e}}"),
        }
    }

    /// A `format` over integer / string conversions only (float conversions are
    /// left out so shortest-`double` formatting can't introduce spurious
    /// divergences); every conversion here is byte-deterministic across engines.
    fn format_op(&mut self) -> String {
        let n = self.rng.below(256);
        let w = self.nonempty_word();
        match self.rng.below(8) {
            0 => format!("format %d {n}"),
            1 => format!("format %05d {n}"),
            2 => format!("format %x {n}"),
            3 => format!("format %o {n}"),
            4 => format!("format %-6s|%s {w} {w}"),
            5 => format!("format %c {}", 65 + self.rng.below(26)),
            6 => format!("format {{%d-%d}} {n} {}", self.rng.below(256)),
            _ => format!("format %%{n}"),
        }
    }

    /// A `scan` returning its conversion count (a deterministic small integer);
    /// the scanned-into variables are scratch and not read back.
    fn scan_op(&mut self) -> String {
        match self.rng.below(3) {
            0 => format!("scan {} %d _s0", self.rng.below(256)),
            1 => format!(
                "scan {{{} {}}} {{%d %d}} _s0 _s1",
                self.rng.below(256),
                self.rng.below(256)
            ),
            _ => format!("scan {} %c _s0", self.nonempty_word()),
        }
    }

    /// A (possibly nested) expression. Division/modulo guard against a zero
    /// divisor so both engines agree on the (non-error) result. The leaves mix
    /// integers, floats, and variable reads; a fraction of interior nodes are
    /// string-relational (`eq`/`ne`/`in`/`ni`/TIP 461 `lt`/`le`/`gt`/`ge`) so
    /// those operators are exercised, and a fraction are a
    /// builtin math-function call (see [`Self::mathfunc_call`]).
    ///
    /// Issue #983's plan: this operator list used to be missing every TIP 461
    /// string-ordering word and every bitwise/shift symbol (`&`/`|`/`^`/`<<`/
    /// `>>`) and `**` entirely — a real fuzz-coverage gap, since none of that
    /// operator surface was ever differentially tested.
    fn expr(&mut self, depth: u32) -> String {
        if depth >= self.config.max_expr_depth || self.rng.chance(1, 2) {
            return self.expr_leaf();
        }
        // A minority of interior nodes are string/list relational (`eq`/`ne`/
        // `in`/`ni`/`lt`/`le`/`gt`/`ge`), which take word operands rather
        // than nested arithmetic.
        if self.rng.chance(1, 5) {
            let op = *self
                .rng
                .pick(&["eq", "ne", "in", "ni", "lt", "le", "gt", "ge"]);
            let left = self.nonempty_word();
            return match op {
                // `in`/`ni` test membership in a list literal.
                "in" | "ni" => format!("{{{left}}} {op} {}", self.list_literal()),
                _ => format!("{{{left}}} {op} {{{}}}", self.nonempty_word()),
            };
        }
        if self.rng.chance(1, 6) {
            return self.mathfunc_call();
        }
        let op = *self.rng.pick(&[
            "+", "-", "*", "/", "%", "<", ">", "<=", ">=", "==", "!=", "&&", "||", "**", "&", "|",
            "^", "<<", ">>",
        ]);
        let left = self.expr(depth + 1);
        let right = self.expr(depth + 1);
        match op {
            // Guard divide/modulo so a zero divisor can't make one engine error
            // and the other not — that is a divergence in the *test*, not the VM.
            // `int(...)` keeps both operands integral so `%` (which rejects a
            // floating-point operand) never errors on a generated float leaf.
            "/" | "%" => {
                format!("int({left}) {op} (int({right}) == 0 ? 1 : int({right}))")
            }
            // Bitwise operators reject floating-point operands outright.
            "&" | "|" | "^" => format!("int({left}) {op} int({right})"),
            // A negative shift count is a hard error in both engines, unlike
            // a negative arithmetic operand — force it non-negative. The
            // count must also be bounded, not just sign-guarded: `right` is
            // itself an arbitrary nested `expr(depth+1)`, which can be
            // another `**`/`<<`/`>>` node, so an unbounded count can reach
            // magnitudes (~1e5+) that make bignum shift/stringification take
            // multiple seconds in both engines — a generator-caused hang,
            // not a VM divergence. `% 64` keeps the count in-range for a
            // real shift while still exercising the operator.
            "<<" | ">>" => format!("int({left}) {op} (abs(int({right})) % 64)"),
            // `**` errors on `0 ** negative` and on a negative base with a
            // non-integer exponent (`domain error`) — forcing a non-negative
            // base and a non-negative integer exponent avoids both without
            // narrowing coverage of the operator itself. The exponent is
            // additionally bounded to `% 20` for the same reason as the
            // shift count above: an unbounded nested exponent can make the
            // result's digit count explode (`2**300000` alone takes ~7s on
            // a real `tclsh`), which is a hang risk independent of, and
            // compounding with, chained `**` nesting.
            "**" => format!("abs({left}) {op} (int(abs({right})) % 20)"),
            _ => format!("({left}) {op} ({right})"),
        }
    }

    /// A call to a builtin `::tcl::mathfunc` — until now the generator never
    /// emitted a single one, leaving that whole command-table dispatch path
    /// (and its TIP 232 override semantics) completely unexercised by the
    /// differential fuzzer.
    ///
    /// Deliberately scoped to:
    /// - functions available by Tcl 9.0 only. The differential fuzzer's
    ///   default reference is `tclsh9.0` (see `main.rs`); a TIP 745 (Tcl 9.1)
    ///   function like `cbrt`/`trunc`/`asinh` would be a guaranteed
    ///   divergence against that reference (it simply doesn't exist there),
    ///   not a real bug — that would make every generated occurrence a false
    ///   finding.
    /// - operands transformed so the call can never raise a domain/range
    ///   error itself (verified against `tclsh8.6` for every Tcl 8.4/8.5
    ///   function here across a matrix of representative operand values,
    ///   including negative/zero/fractional): every argument is a plain
    ///   `expr_leaf()` (a var read or a small bounded literal), so none of
    ///   `hypot`/`atan2`/`isunordered`/`max`/`min`/the arity-1 set below can
    ///   ever see anything but a small finite double.
    fn mathfunc_call(&mut self) -> String {
        const ARITY1: &[&str] = &[
            "abs",
            "ceil",
            "floor",
            "round",
            "sin",
            "cos",
            "tan",
            "sinh",
            "cosh",
            "tanh",
            "atan",
            "int",
            "double",
            "wide",
            "bool",
            "isfinite",
            "isinf",
            "isnan",
            "isnormal",
            "issubnormal",
        ];
        const ARITY2: &[&str] = &["hypot", "atan2", "max", "min", "isunordered"];
        if self.rng.chance(1, 6) {
            // `fmod` needs the same guarded non-zero divisor as `/`/`%`.
            let a = self.expr_leaf();
            let b = self.expr_leaf();
            format!("fmod({a}, (({b}) == 0 ? 1 : ({b})))")
        } else if self.rng.chance(1, 2) {
            let f = *self.rng.pick(ARITY2);
            let a = self.expr_leaf();
            let b = self.expr_leaf();
            format!("{f}({a}, {b})")
        } else {
            let f = *self.rng.pick(ARITY1);
            let a = self.expr_leaf();
            format!("{f}({a})")
        }
    }

    /// An expression leaf: a variable read, a small integer, or a simple float
    /// (shortest-`double` formatting of the *result* is itself under test).
    fn expr_leaf(&mut self) -> String {
        match self.rng.below(4) {
            0 | 1 => format!("${}", self.var()),
            2 => self.rng.below(20).to_string(),
            // A terminating decimal so the value is exact in binary where
            // possible; the engines must still agree on how they print it.
            _ => format!("{}.{}", self.rng.below(10), self.rng.below(10)),
        }
    }

    /// Emit a statement whose expression is deliberately malformed, so the
    /// differential actually sees an `expr` syntax error.
    ///
    /// Emitted **uncaught**. C Tcl raises an error here, so an engine that
    /// instead evaluates the broken source to some value diverges as
    /// [`crate::harness::Verdict::StatusMismatch`] — one engine errored, the
    /// other did not. That is precisely how the real bug this production
    /// exists for would have been caught: an unparsable runtime expression
    /// evaluated to its own source text, so `expr {1+}` quietly returned the
    /// string `1+` instead of failing. Wrapping the statement in `catch` would
    /// swallow exactly that signal.
    ///
    /// Three contexts, all of which reach the same C `ParseExpr`: `expr` as a
    /// command argument, an `if` condition, and an `expr` result being
    /// assigned. Loop conditions (`while`/`for`) are deliberately excluded: an
    /// engine that wrongly accepted a malformed condition as true would spin
    /// until the harness timeout, degrading a crisp status mismatch into a slow
    /// and much less informative `Timeout` finding.
    fn malformed_expr_stmt(&mut self) {
        let e = self.malformed_expr();
        match self.rng.below(3) {
            0 => {
                let _ = writeln!(self.out, "puts [expr {{{e}}}]");
            }
            1 => {
                let _ = writeln!(self.out, "if {{{e}}} {{puts _mx}}");
            }
            // Assigns through a private `_mx*` name so that, whichever
            // statement slot this lands in, it cannot perturb the shared
            // variable pool the rest of the script reads.
            _ => {
                let _ = writeln!(self.out, "set _mxv [expr {{{e}}}]");
            }
        }
    }

    /// One delimiter-shaped word, drawn uniformly from [`WordShape::ALL`].
    ///
    /// `braceable` restricts the draw to shapes that can be nested inside a
    /// braced word; a caller that does not brace the word passes `false` and
    /// gets the whole table.
    fn word_shape(&mut self, must_balance: bool) -> WordShape {
        loop {
            let shape = *self.rng.pick(WordShape::ALL);
            if !must_balance || shape.brace_balanced() {
                break shape;
            }
        }
    }

    /// Emit one word-shape probe: a [`WordShape`] crossed with a
    /// [`WordContext`], printing an observable so the differential has
    /// something to compare.
    ///
    /// Every probe prints the value's *length* as well as the value itself.
    /// Length is what makes a lost brace layer visible at all — the two
    /// oracles and a broken engine can print byte sequences that look alike in
    /// a terminal, and `{}` versus the empty string is exactly such a pair.
    ///
    /// Private `_w*` names throughout, so whichever statement slot this lands
    /// in it cannot perturb the shared variable pool the rest of the script
    /// reads — the same containment `malformed_expr_stmt` keeps.
    fn word_shape_stmt(&mut self) {
        let ctx = *self.rng.pick(WordContext::ALL);
        let shape = self.word_shape(ctx.braces_the_word());
        let w = shape.spelling();
        match ctx {
            WordContext::Assign => {
                let _ = writeln!(self.out, "set _wv {w}");
                let _ = writeln!(self.out, "puts \"[string length $_wv]:$_wv\"");
            }
            WordContext::CommandArg => {
                let _ = writeln!(self.out, "puts [string length {w}]");
            }
            WordContext::NestedArg => {
                let _ = writeln!(self.out, "puts [string length [format %s {w}]]");
            }
            WordContext::ProcArg => {
                let _ = writeln!(self.out, "proc _wp {{s}} {{return [string length $s]:$s}}");
                let _ = writeln!(self.out, "puts [_wp {w}]");
            }
            // The body seeds its own `x`: `$x` inside a proc is a *local*
            // read, so a shape carrying one would otherwise raise
            // `can't read "x"` on both engines and compare error text
            // instead of a word value.
            WordContext::ProcLocal => {
                let _ = writeln!(
                    self.out,
                    "proc _wl {{}} {{set x 7; set v {w}; return [string length $v]:$v}}"
                );
                let _ = writeln!(self.out, "puts [_wl]");
            }
            WordContext::ListElement => {
                let _ = writeln!(self.out, "set _wl [list {w} q]");
                let _ = writeln!(
                    self.out,
                    "puts [llength $_wl]:[string length [lindex $_wl 0]]"
                );
            }
            WordContext::DictValue => {
                let _ = writeln!(self.out, "set _wd [dict create k {w}]");
                let _ = writeln!(self.out, "puts [string length [dict get $_wd k]]");
            }
            WordContext::Append => {
                let _ = writeln!(self.out, "set _wa \"\"");
                let _ = writeln!(self.out, "append _wa {w}");
                let _ = writeln!(self.out, "puts [string length $_wa]");
            }
            WordContext::SwitchSubject => {
                let _ = writeln!(
                    self.out,
                    "switch -- {w} zz {{puts _ws_hit}} default {{puts _ws_def}}"
                );
            }
            WordContext::ExprOperand => {
                let _ = writeln!(self.out, "puts [expr {{[string length {w}] + 0}}]");
            }
            WordContext::FormatFold => {
                let _ = writeln!(self.out, "set _wf [format %s {w}]");
                let _ = writeln!(self.out, "puts [string length $_wf]:$_wf");
            }
            WordContext::ListFold => {
                let _ = writeln!(self.out, "set _wg [list {w}]");
                let _ = writeln!(self.out, "puts [string length $_wg]:$_wg");
            }
        }
    }

    /// The source text of one malformed expression, drawn uniformly from
    /// [`Malformed::ALL`].
    fn malformed_expr(&mut self) -> String {
        let mode = *self.rng.pick(Malformed::ALL);
        self.render_malformed(mode)
    }

    /// Render `mode` as expression source. [`Malformed`] documents the C error
    /// family each shape provokes and where `tests/parseExpr.test` covers it.
    fn render_malformed(&mut self, mode: Malformed) -> String {
        match mode {
            Malformed::TrailingOperator => {
                let op = *self.rng.pick(MALFORMED_BINARY_OPS);
                let a = self.expr_leaf();
                format!("{a} {op}")
            }
            Malformed::MissingOperator => {
                let a = self.expr_leaf();
                let b = self.expr_leaf();
                format!("{a} {b}")
            }
            Malformed::EmptyExpression => String::new(),
            Malformed::Bareword => {
                let n = 1 + self.rng.below(3);
                let mut words = Vec::with_capacity(n);
                for _ in 0..n {
                    words.push(*self.rng.pick(MALFORMED_BAREWORDS));
                }
                words.join(" ")
            }
            Malformed::UnbalancedOpenParen => {
                let a = self.expr_leaf();
                let b = self.expr_leaf();
                if self.rng.chance(1, 2) {
                    format!("({a} + {b}")
                } else {
                    format!("(({a} + {b})")
                }
            }
            Malformed::UnbalancedCloseParen => {
                let a = self.expr_leaf();
                let b = self.expr_leaf();
                if self.rng.chance(1, 2) {
                    format!("{a} + {b})")
                } else {
                    format!("{a}) + {b}")
                }
            }
            Malformed::LoneUnary => (*self.rng.pick(&["!", "~", "-", "+"])).to_owned(),
            Malformed::InvalidCharacter => {
                let c = *self.rng.pick(MALFORMED_BADCHARS);
                if self.rng.chance(1, 2) {
                    c.to_owned()
                } else {
                    let a = self.expr_leaf();
                    let b = self.expr_leaf();
                    format!("{a} {c} {b}")
                }
            }
            Malformed::IncompleteOperator => {
                if self.rng.chance(1, 2) {
                    "=".to_owned()
                } else {
                    let a = self.expr_leaf();
                    let b = self.expr_leaf();
                    format!("{a} = {b}")
                }
            }
            Malformed::EmptySubexpression => {
                if self.rng.chance(1, 2) {
                    "()".to_owned()
                } else {
                    format!("{} + ()", self.expr_leaf())
                }
            }
            Malformed::StrayColon | Malformed::StrayComma | Malformed::MissingColon => {
                let punct = match mode {
                    Malformed::StrayColon => ":",
                    Malformed::StrayComma => ",",
                    _ => "?",
                };
                let a = self.expr_leaf();
                let b = self.expr_leaf();
                format!("{a} {punct} {b}")
            }
            Malformed::BadNumber => {
                let n = *self.rng.pick(&["0o8", "0o9", "0b2", "0b3"]);
                if self.rng.chance(1, 2) {
                    n.to_owned()
                } else {
                    format!("{} + {n}", self.expr_leaf())
                }
            }
            Malformed::CommentOnly => {
                if self.rng.chance(1, 2) {
                    "#".to_owned()
                } else {
                    format!("# {}", self.rng.pick(MALFORMED_BAREWORDS))
                }
            }
            Malformed::MissingFunctionArgument
            | Malformed::FunctionArity
            | Malformed::UnknownFunction => self.render_malformed_call(mode),
        }
    }

    /// The malformed *function-call* shapes, split out of
    /// [`Self::render_malformed`] to keep each rendering function small.
    fn render_malformed_call(&mut self, mode: Malformed) -> String {
        match mode {
            // A hole in the argument list — a parser error (`MISSING` /
            // `UNBALANCED`), reached before the function is ever looked up.
            Malformed::MissingFunctionArgument => {
                let a = self.expr_leaf();
                if self.rng.chance(1, 2) {
                    format!("max({a}, )")
                } else {
                    format!("max(, {a})")
                }
            }
            // A real, arity-1 function called with two arguments or none, so
            // both branches are a genuine arity error.
            //
            // Operands are literals, not `expr_leaf()`, and that matters: this
            // shape and `UnknownFunction` below are the only two that parse
            // cleanly and fail at *run* time, in the `::tcl::mathfunc` command
            // layer. `expr_leaf()` can emit a read of a variable the script
            // never assigned (only `a`/`b`/`i`/`x` are seeded), which would
            // raise `can't read "y"` *first* and mask the arity error this
            // shape exists to provoke — measured against a real `tclsh9.0.4`,
            // that pre-empted 54 of 1461 generated statements before the
            // operands here were pinned to literals.
            Malformed::FunctionArity => {
                let f = *self.rng.pick(&["sin", "cos", "abs", "double", "round"]);
                if self.rng.chance(1, 2) {
                    let (a, b) = (self.rng.below(20), self.rng.below(20));
                    format!("{f}({a}, {b})")
                } else {
                    format!("{f}()")
                }
            }
            // A name with no `::tcl::mathfunc` command behind it. Kept clear
            // of every name the generator itself defines, so a generated proc
            // can never accidentally make the call resolve. Literal operand
            // for the same reason as `FunctionArity`.
            _ => {
                let f = *self.rng.pick(&["nosuchfn", "notafunc", "bogusfn"]);
                format!("{f}({})", self.rng.below(20))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bare [`Gen`], for driving one production directly instead of through
    /// a whole script.
    fn gen_with(seed: u64, config: GenConfig) -> Gen {
        Gen {
            rng: Rng::new(seed),
            config,
            out: String::new(),
            procs: Vec::new(),
        }
    }

    /// Whether `text` has the shape [`Malformed`] documents for `mode`, so the
    /// per-mode expectation lives next to the assertion that checks it.
    fn matches_mode(mode: Malformed, text: &str) -> bool {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let starts_with_call =
            |names: &[&str]| names.iter().any(|f| text.starts_with(&format!("{f}(")));
        match mode {
            Malformed::TrailingOperator => {
                tokens.len() == 2 && MALFORMED_BINARY_OPS.contains(&tokens[1])
            }
            Malformed::MissingOperator => {
                tokens.len() == 2 && !MALFORMED_BINARY_OPS.contains(&tokens[1])
            }
            Malformed::EmptyExpression => text.is_empty(),
            Malformed::Bareword => {
                !tokens.is_empty() && tokens.iter().all(|t| MALFORMED_BAREWORDS.contains(t))
            }
            Malformed::UnbalancedOpenParen => text.matches('(').count() > text.matches(')').count(),
            Malformed::UnbalancedCloseParen => {
                text.matches(')').count() > text.matches('(').count()
            }
            Malformed::LoneUnary => ["!", "~", "-", "+"].contains(&text),
            Malformed::InvalidCharacter => MALFORMED_BADCHARS.iter().any(|c| text.contains(*c)),
            Malformed::IncompleteOperator => text.contains('=') && !text.contains("=="),
            Malformed::EmptySubexpression => text.contains("()"),
            Malformed::MissingFunctionArgument => {
                text.starts_with("max(") && (text.contains(", )") || text.contains("(, "))
            }
            Malformed::FunctionArity => starts_with_call(&["sin", "cos", "abs", "double", "round"]),
            Malformed::UnknownFunction => starts_with_call(&["nosuchfn", "notafunc", "bogusfn"]),
            Malformed::StrayColon => tokens.len() == 3 && tokens[1] == ":",
            Malformed::StrayComma => tokens.len() == 3 && tokens[1] == ",",
            Malformed::MissingColon => tokens.len() == 3 && tokens[1] == "?",
            Malformed::BadNumber => ["0o8", "0o9", "0b2", "0b3"]
                .iter()
                .any(|n| text.contains(n)),
            Malformed::CommentOnly => text.starts_with('#'),
        }
    }

    /// Malformed shapes whose status on our *own* engine is deliberately not
    /// asserted, because a real divergence against C Tcl is open for them.
    ///
    /// `tclvm` still evaluates a radix-invalid numeric literal to its own
    /// source text — `expr {0o8}` yields the string `0o8`, and likewise `0o9`,
    /// `0b2`, `0b3` (and, outside this generator's set, `0x` and `0xg`) —
    /// where C Tcl 9.0.4 raises `invalid bareword "0o8" … (invalid octal
    /// number?)`. That is the same silent-wrong-answer shape as the `expr {1+}`
    /// bug, on the numeric-literal path rather than the operator path. It was
    /// found *by this production* (6 seeds in a 1000-iteration campaign at
    /// 300‰, 2 in 3000 at the default rate) and is reported for triage, not
    /// fixed here.
    ///
    /// Recorded as "not asserted" rather than "must not error" on purpose:
    /// fixing the engine must not break this test.
    const MALFORMED_MODES_WITH_OPEN_DIVERGENCE: &[Malformed] = &[Malformed::BadNumber];

    /// Run `src` on `tcl-vm` in-process; `true` if it reported an error.
    /// Mirrors `wasm_diff`'s direct arm. Output is discarded — only the
    /// completion status matters here.
    fn errors_on_our_engine(src: &str) -> bool {
        let registry = tcl_registry::CommandRegistry::build_default();
        let ir = tcl_compiler::lowering::lower_to_ir(src, &registry);
        let cfg = tcl_compiler::cfg_builder::build_cfg_codegen_with_registry(&ir, false, &registry);
        let asm = tcl_compiler::codegen::codegen_module(&cfg, &ir, &registry);
        let mut vm = tcl_vm::Vm::with_output(Box::new(std::io::sink()));
        vm.run_module(&asm).code == tcl_vm::Code::Error
    }

    /// Whether `out` is one of the three statement forms
    /// [`Gen::malformed_expr_stmt`] emits.
    fn is_malformed_stmt(out: &str) -> bool {
        let line = out.trim_end();
        line.starts_with("set _mxv [expr {")
            || (line.starts_with("if {") && line.ends_with("} {puts _mx}"))
            || (line.starts_with("puts [expr {") && line.ends_with("}]"))
    }

    #[test]
    fn deterministic_for_seed() {
        let cfg = GenConfig::default();
        assert_eq!(generate(42, &cfg), generate(42, &cfg));
        assert_ne!(generate(1, &cfg), generate(2, &cfg));
    }

    /// Regression: `while`/`for` counters used to be a flat `_w`/`_f`
    /// regardless of nesting depth, so a `while` nested inside another
    /// `while` reset the *same* counter on every outer iteration — a
    /// genuine infinite loop, not just a divergence risk (found by actually
    /// running the generator's output through a real `tclsh` while
    /// verifying issue #983's mathfunc/operator additions below — seed 127
    /// under a deepened `GenConfig` hung indefinitely). Depth-qualifying
    /// the name (`_w0`, `_w1`, …) fixes it structurally: nested loops can
    /// never share a counter, so this checks the old un-qualified form
    /// never reappears.
    #[test]
    fn while_and_for_counters_are_depth_qualified() {
        let cfg = GenConfig {
            max_depth: 5,
            max_stmts: 20,
            max_expr_depth: 4,
            ..GenConfig::default()
        };
        for seed in 0..500u64 {
            let src = generate(seed, &cfg);
            assert!(
                !src.contains("set _w 0") && !src.contains("set _f 0"),
                "seed {seed}: un-qualified loop counter reappeared:\n{src}"
            );
        }
    }

    /// Adversarial-review finding: `**`/`<<`/`>>` guarded operand *sign*
    /// (never negative) but not *magnitude*. Since `left`/`right` are
    /// arbitrary nested `expr(depth+1)` calls — which can themselves be
    /// another `**`/`<<`/`>>` node — an unbounded exponent/shift-count could
    /// reach magnitudes that make bignum exponentiation/shift and its
    /// decimal stringification take multiple seconds in both engines
    /// (confirmed directly against a real `tclsh8.6`: `2**300000` and
    /// `1<<300000` each take ~7s), a generator-caused hang that shows up as
    /// a spurious `Timeout`/`StatusMismatch` finding, not a real VM bug.
    ///
    /// A live `tclsh` sweep isn't a reliable regression guard here: the
    /// magnitude collision that triggers the hang is probabilistic (an
    /// adversarial-review simulation found it in ~1.6% of expression-root
    /// draws), so a seed sweep small enough to run as a fast unit test can
    /// pass on the *old*, unbounded code purely by chance — this asserts
    /// the actual generated Tcl source instead: every `**` exponent operand
    /// is deterministically reduced by `% 20` and every `<<`/`>>` count
    /// operand by `% 64`, which bounds the operand to a small range no
    /// matter how large the sub-expression feeding it evaluates to.
    #[test]
    fn power_and_shift_operands_are_magnitude_bounded() {
        let cfg = GenConfig {
            max_depth: 4,
            max_stmts: 16,
            max_expr_depth: 4,
            ..GenConfig::default()
        };
        let corpus: Vec<String> = (0..400u64).map(|s| generate(s, &cfg)).collect();
        assert!(
            corpus.iter().any(|s| s.contains("**")),
            "no `**` was ever generated in this sweep"
        );
        assert!(
            corpus.iter().any(|s| s.contains("<<") || s.contains(">>")),
            "no `<<`/`>>` was ever generated in this sweep"
        );
        assert!(
            corpus.iter().any(|s| s.contains("% 20)")),
            "`**` exponent is no longer bounded by `% 20` — magnitude hang risk reintroduced"
        );
        assert!(
            corpus.iter().any(|s| s.contains("% 64)")),
            "`<<`/`>>` count is no longer bounded by `% 64` — magnitude hang risk reintroduced"
        );
    }

    #[test]
    fn balanced_delimiters() {
        let cfg = GenConfig::default();
        for seed in 0..200 {
            let src = generate(seed, &cfg);
            // Asked of the owner of the question (`info complete`'s engine),
            // not of a byte counter. A counter was an adequate proxy only
            // while no generated word could contain a delimiter; the
            // `WordShape` productions emit words whose *value* is `{` or `}`,
            // and `set _w "{"` is a complete script whose braces do not
            // balance by count. The owner knows a delimiter inside a quoted
            // word is content, so it answers what this test always meant.
            assert!(
                tcl_lexer::script_is_complete(&src),
                "seed {seed}: generated script is not a complete script\n{src}"
            );
        }
    }

    #[test]
    fn nonempty_and_prints() {
        let cfg = GenConfig::default();
        // Most scripts should contain a `puts` so the differential has output.
        let with_puts = (0..100)
            .filter(|s| generate(*s, &cfg).contains("puts"))
            .count();
        assert!(
            with_puts > 50,
            "too few scripts print output: {with_puts}/100"
        );
    }

    #[test]
    fn broadened_grammar_is_exercised() {
        // Over a decent seed range each newly-added production should appear at
        // least once, so the broadened surface is actually generated (not dead).
        let cfg = GenConfig {
            max_depth: 4,
            max_stmts: 16,
            ..GenConfig::default()
        };
        let corpus: Vec<String> = (0..600).map(|s| generate(s, &cfg)).collect();
        let appears = |needle: &str| corpus.iter().any(|s| s.contains(needle));
        for needle in [
            "proc ",
            "namespace eval ",
            "dict ",
            "switch -- ",
            "catch {",
            "try {",
            // Command families added for.
            "format ",
            "scan ",
            "array set ",
            "apply {",
            "eval [list",
            "subst {",
            "string compare ",
            "concat ",
            "lmap ",
            "global ",
            "upvar ",
            "regexp ",
            "regsub ",
        ] {
            assert!(appears(needle), "production never generated: {needle:?}");
        }
        // The string/list relational operators reach the expression grammar.
        assert!(
            corpus
                .iter()
                .any(|s| s.contains(" eq ") || s.contains(" in ")),
            "string/list relational operators never generated"
        );
        // A defined proc should sometimes be called back (`puts [p0 ...]`).
        let proc_called = corpus.iter().any(|s| {
            ["p0", "p1", "p2", "helper", "compute"]
                .iter()
                .any(|p| s.contains(&format!("puts [{p}")))
        });
        assert!(proc_called, "no generated proc was ever called");
    }

    /// Issue #1056's regression seed. `namespace inscope` was never generated
    /// at all, so the whole list-element tail rule went differentially
    /// untested and the VM's space-join (which turned `{x y}` into two
    /// arguments) could not be caught by a campaign. Proves the production is
    /// live and that every interesting tail shape — the whitespace-bearing
    /// word that is the minimal reproducer, an empty element, and the
    /// substitution/terminator characters that force list quoting — actually
    /// reaches a generated script.
    #[test]
    fn namespace_inscope_list_args_are_exercised() {
        let cfg = GenConfig {
            max_depth: 4,
            max_stmts: 16,
            ..GenConfig::default()
        };
        let corpus: Vec<String> = (0..600u64).map(|s| generate(s, &cfg)).collect();
        let inscope_lines: Vec<&str> = corpus
            .iter()
            .flat_map(|s| s.lines())
            .filter(|l| l.contains("namespace inscope "))
            .collect();
        assert!(
            !inscope_lines.is_empty(),
            "`namespace inscope` is never generated"
        );
        // The probe proc must accompany every call, else the differential has
        // nothing to compare.
        assert!(
            corpus
                .iter()
                .any(|s| s.contains("proc _shape {args}") && s.contains("namespace inscope ")),
            "the inscope probe proc is not emitted alongside the call"
        );
        for arg in [
            "{x y}",
            "{  padded  }",
            "{}",
            "{a;b}",
            "{$nosub}",
            "{[nosub]}",
        ] {
            assert!(
                inscope_lines.iter().any(|l| l.contains(arg)),
                "inscope tail shape never generated: {arg:?}"
            );
        }
        // The bug's shape specifically: a trailing word containing whitespace.
        assert!(
            inscope_lines
                .iter()
                .any(|l| l.contains("{_shape} {x y}") || l.contains("{_shape} {a b c}")),
            "no whitespace-bearing first tail word — the #1056 reproducer is not covered"
        );
    }

    /// Issue #983's plan: `expr()`'s operator menu used to omit the TIP 461
    /// string-ordering words, every bitwise/shift symbol, and `**` entirely,
    /// and the generator never emitted a single `::tcl::mathfunc` call — all
    /// real fuzz-coverage gaps (that whole surface went differentially
    /// untested). Proves each newly-added production actually gets generated
    /// (not dead code) over a decent seed sweep, mirroring
    /// `broadened_grammar_is_exercised` above.
    #[test]
    fn expr_and_mathfunc_additions_are_exercised() {
        let cfg = GenConfig {
            max_depth: 4,
            max_stmts: 16,
            max_expr_depth: 4,
            ..GenConfig::default()
        };
        let corpus: Vec<String> = (0..1500).map(|s| generate(s, &cfg)).collect();
        for op in ["lt", "le", "gt", "ge"] {
            assert!(
                corpus.iter().any(|s| s.contains(&format!(" {op} "))),
                "TIP 461 operator never generated: {op:?}"
            );
        }
        for op in ["**", "<<", ">>"] {
            assert!(
                corpus.iter().any(|s| s.contains(op)),
                "operator never generated: {op:?}"
            );
        }
        // `&`/`|` search on `& int(`/`| int(` rather than the bare symbol:
        // the bitwise shape always renders as `int(...) & int(...)` with no
        // extra parens around the right operand, whereas `&&`/`||` (already
        // in the pre-existing pool) render as `(...) && (...)` — a nested
        // right operand starting with `int(` would make the bare `&`/`|`
        // substring check a false pass via `&&`/`||` instead.
        for op in ["& int(", "| int(", "^ int("] {
            assert!(
                corpus.iter().any(|s| s.contains(op)),
                "operator never generated: {op:?}"
            );
        }
        for func in [
            "abs(", "sin(", "hypot(", "atan2(", "max(", "min(", "fmod(", "isnan(", "bool(",
        ] {
            assert!(
                corpus.iter().any(|s| s.contains(func)),
                "mathfunc call never generated: {func:?}"
            );
        }
        // Every TIP 745 (Tcl 9.1) function must never appear: the
        // differential fuzzer's default reference is `tclsh9.0`, which
        // doesn't have them, so generating one would be a guaranteed false
        // finding rather than a real bug.
        for func in ["cbrt(", "trunc(", "asinh(", "acosh(", "exp2(", "gamma("] {
            assert!(
                !corpus.iter().any(|s| s.contains(func)),
                "TIP 745 (9.1-only) function must never be generated: {func:?}"
            );
        }
    }

    /// Issue #923 audit cluster C10's coverage seed. Variable access through
    /// a **computed name** was never generated, so the whole dynamic-name
    /// path — which the compiler now treats as a dataflow barrier gating the
    /// constant-branch fold and dead-store elimination that feed this
    /// backend's input IR — went differentially untested. Proves every arm
    /// reaches a generated script.
    #[test]
    fn dynamic_name_shapes_are_exercised() {
        let cfg = GenConfig {
            max_depth: 4,
            max_stmts: 16,
            ..GenConfig::default()
        };
        let corpus: Vec<String> = (0..600u64).map(|s| generate(s, &cfg)).collect();
        for needle in [
            "set $_dnn hello",
            "puts [set $_dnn]",
            "lappend $_dnn one",
            "append $_dnn ab",
            "puts [subst $_dnt]",
            "unset $_dnn",
        ] {
            assert!(
                corpus.iter().any(|s| s.contains(needle)),
                "dynamic-name production never generated: {needle:?}"
            );
        }
        // The shapes must stay inside their private `_dn*` name space, so
        // wherever they land they cannot perturb the rest of a script.
        for line in corpus
            .iter()
            .flat_map(|s| s.lines())
            .filter(|l| l.contains("_dn"))
        {
            for v in VARS {
                assert!(
                    !line.contains(&format!("${v} ")) && !line.ends_with(&format!("${v}")),
                    "dynamic-name line touches shared variable {v:?}: {line:?}"
                );
            }
        }
    }

    /// Issue #923 audit cluster C1's coverage seed. A procedure writing its
    /// **caller's** frame was never generated, so the whole caller-frame
    /// path — which the compiler now summarises per procedure and applies at
    /// every call site, gating the constant-branch fold and dead-store
    /// elimination that feed this backend's input IR — went differentially
    /// untested. Proves every arm reaches a generated script.
    #[test]
    fn caller_frame_shapes_are_exercised() {
        let cfg = GenConfig {
            max_depth: 4,
            max_stmts: 16,
            ..GenConfig::default()
        };
        let corpus: Vec<String> = (0..600u64).map(|s| generate(s, &cfg)).collect();
        for needle in [
            "upvar 1 $n v",
            "upvar 1 _cfy v",
            "upvar #0 _cfg g",
            "uplevel 1 {set _cfz up}",
            "uplevel 1 [list set $n cons]",
            "uplevel 1 [list _cff $n]",
        ] {
            assert!(
                corpus.iter().any(|s| s.contains(needle)),
                "caller-frame production never generated: {needle:?}"
            );
        }
        // The shapes must stay inside their private `_cf*` name space, so
        // wherever they land they cannot perturb the rest of a script.
        for line in corpus
            .iter()
            .flat_map(|s| s.lines())
            .filter(|l| l.contains("_cf"))
        {
            for v in VARS {
                assert!(
                    !line.contains(&format!("${v} ")) && !line.ends_with(&format!("${v}")),
                    "caller-frame line touches shared variable {v:?}: {line:?}"
                );
            }
        }
    }

    /// Every [`Malformed`] shape renders the form its documentation promises,
    /// so a mode can never silently degrade into a *different* (or, worse, a
    /// well-formed) expression while still counting towards the coverage claim.
    #[test]
    fn malformed_modes_each_render_their_documented_shape() {
        for mode in Malformed::ALL {
            for seed in 0..200u64 {
                let text = gen_with(seed, GenConfig::default()).render_malformed(*mode);
                assert!(
                    matches_mode(*mode, &text),
                    "{mode:?} rendered an unexpected shape: {text:?}"
                );
            }
        }
    }

    /// The single most important invariant of this production: a malformed
    /// expression must still be a well-formed Tcl **word**.
    ///
    /// The malformed-ness has to stay inside the *expression* grammar. A brace,
    /// bracket, double quote, or backslash in the rendered text would instead
    /// break `Tcl_ParseCommand`, making the whole *script* unparsable — a
    /// different layer of Tcl entirely, and a uniformly uninteresting
    /// whole-script failure rather than the `expr` syntax error being hunted.
    /// (A trailing backslash is the subtle one: inside `expr {…}` it would
    /// escape the word's own closing brace.)
    #[test]
    fn malformed_expressions_stay_inside_the_expr_grammar() {
        for mode in Malformed::ALL {
            for seed in 0..200u64 {
                let text = gen_with(seed, GenConfig::default()).render_malformed(*mode);
                for bad in ['{', '}', '[', ']', '"', '\\'] {
                    assert!(
                        !text.contains(bad),
                        "{mode:?} emitted {bad:?}, which breaks the script parser: {text:?}"
                    );
                }
            }
        }
    }

    /// The production is emitted at top level only, and that is load-bearing
    /// twice over. Nested in a `proc` body C Tcl would not report it at all
    /// (bodies compile lazily, at first call, so a syntax error in an uncalled
    /// proc is never raised) while an eagerly-compiling engine would, turning
    /// the finding into one about compile *timing*. Nested in a `catch`/`try`
    /// body the error would be swallowed, erasing the very status difference
    /// the production exists to expose.
    ///
    /// With the rate pinned to always fire, depth 0 must always take the
    /// branch and no deeper depth ever may. The nested half is checked through
    /// the `_mx` marker, which two of the three statement contexts carry: over
    /// 600 nested samples that would all be malformed if the guard were
    /// dropped, missing every marker is vanishingly unlikely.
    #[test]
    fn malformed_expr_is_top_level_only() {
        let always = GenConfig {
            malformed_expr_permille: 1000,
            ..GenConfig::default()
        };
        for seed in 0..200u64 {
            let mut top = gen_with(seed, always);
            top.statement(0);
            assert!(
                is_malformed_stmt(&top.out),
                "depth 0 did not emit the malformed production: {:?}",
                top.out
            );
            for depth in 1..4 {
                let mut nested = gen_with(seed, always);
                nested.statement(depth);
                assert!(
                    !nested.out.contains("_mx"),
                    "depth {depth} emitted a malformed expression: {:?}",
                    nested.out
                );
            }
        }
    }

    /// Every shape renders the spelling its documentation names, and every one
    /// is a single Tcl word — the invariant that lets a shape drop into any
    /// word position without reshaping the script around it.
    #[test]
    fn word_shapes_each_render_their_documented_spelling() {
        for &shape in WordShape::ALL {
            let w = shape.spelling();
            assert!(!w.is_empty(), "{shape:?} renders nothing");
            // Exactly one word, asked of the segmentation owner rather than of
            // a "contains a space" guess. `EscapedSpace` is precisely the
            // shape a guess gets wrong — `a\ b` reads as two words and is one
            // — and being one word is the property under test for it, so the
            // assertion has to be the real boundary rule.
            let probe = format!("puts {w}\n");
            let tokens = tcl_lexer::Lexer::new(&probe)
                .tokenise_all()
                .unwrap_or_else(|e| panic!("{shape:?} does not lex: {e:?}"));
            let commands =
                tcl_lexer::group_commands(&tokens, &probe, tcl_lexer::LexerConfig::default());
            assert_eq!(
                commands.len(),
                1,
                "{shape:?} renders `{w}`, which is not one command"
            );
            assert_eq!(
                commands[0].words.len(),
                2,
                "{shape:?} renders `{w}`, which is not exactly one word"
            );
            // And it is a complete script on its own as a command argument, so
            // it cannot be what makes a generated script unparsable.
            assert!(
                tcl_lexer::script_is_complete(&probe),
                "{shape:?} does not form a complete script: {probe}"
            );
        }
        assert_eq!(WordShape::QuotedEmptyBraces.spelling(), "\"{}\"");
        assert_eq!(WordShape::EscapedBraces.spelling(), r"\{\}");
        assert_eq!(WordShape::BracedDoubleGroup.spelling(), "{{}}");
        assert_eq!(WordShape::QuotedOpenBrace.spelling(), "\"{\"");
    }

    /// `brace_balanced` agrees with an actual brace walk, so the guard that
    /// keeps `"{"` out of `expr {…}` and out of a proc body cannot drift from
    /// the spellings it is guarding.
    #[test]
    fn brace_balanced_matches_a_real_walk() {
        for &shape in WordShape::ALL {
            let mut depth = 0i32;
            let bytes = shape.spelling().as_bytes();
            let mut i = 0;
            let mut ok = true;
            while i < bytes.len() {
                match bytes[i] {
                    b'\\' => i += 1,
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth < 0 {
                            ok = false;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            assert_eq!(
                shape.brace_balanced(),
                ok && depth == 0,
                "{shape:?} declares brace_balanced()={} but walks to depth {depth}",
                shape.brace_balanced()
            );
        }
    }

    /// The invariant that decides whether a probe is a probe: a context that
    /// nests the word inside a braced word must only ever be handed a
    /// brace-balanced shape. Get it wrong and the shape's brace closes the
    /// enclosing word — `proc _wl {} {… set v "{" …}` — and the script stops
    /// parsing, which is a whole-script failure rather than a word-value
    /// comparison. Asserted over the real corpus, not over the table, because
    /// `script_is_complete` is what actually catches the mistake.
    #[test]
    fn unbalanced_shapes_never_reach_a_bracing_context() {
        let cfg = GenConfig {
            word_shape_permille: 1000,
            malformed_expr_permille: 0,
            ..GenConfig::default()
        };
        for seed in 0..400u64 {
            let src = generate(seed, &cfg);
            assert!(
                tcl_lexer::script_is_complete(&src),
                "seed {seed}: a word shape broke the script\n{src}"
            );
        }
    }

    /// Every shape and every context is reachable from the default config, so
    /// no row of the cross is dead.
    #[test]
    fn every_word_shape_and_context_is_reachable() {
        let cfg = GenConfig {
            word_shape_permille: 400,
            ..GenConfig::default()
        };
        let corpus: Vec<String> = (0..600u64).map(|s| generate(s, &cfg)).collect();
        for &shape in WordShape::ALL {
            let w = shape.spelling();
            assert!(
                corpus.iter().any(|s| s.contains(w)),
                "shape {shape:?} (`{w}`) never appears in the corpus"
            );
        }
        // One marker per context's emitted form.
        for (ctx, marker) in [
            (WordContext::Assign, "set _wv "),
            (WordContext::CommandArg, "puts [string length "),
            (WordContext::NestedArg, "[format %s "),
            (WordContext::ProcArg, "proc _wp "),
            (WordContext::ProcLocal, "proc _wl "),
            (WordContext::ListElement, "set _wl [list "),
            (WordContext::DictValue, "set _wd [dict create k "),
            (WordContext::Append, "append _wa "),
            (WordContext::SwitchSubject, "_ws_def"),
            (WordContext::ExprOperand, "puts [expr {[string length "),
            (WordContext::FormatFold, "set _wf [format %s "),
            (WordContext::ListFold, "set _wg [list "),
        ] {
            assert!(
                corpus.iter().any(|s| s.contains(marker)),
                "context {ctx:?} never appears in the corpus (marker `{marker}`)"
            );
        }
    }

    /// The word-shape production reaches scripts at the default rate without
    /// dominating them — the same balance `malformed_expressions_reach_...`
    /// keeps for its own class.
    #[test]
    fn word_shapes_reach_scripts_at_the_default_rate() {
        let cfg = GenConfig::default();
        let corpus: Vec<String> = (0..1000u64).map(|s| generate(s, &cfg)).collect();
        let carrying = corpus
            .iter()
            .filter(|s| WordShape::ALL.iter().any(|sh| s.contains(sh.spelling())))
            .count();
        assert!(
            carrying > 0,
            "the word-shape production is dead at the default rate"
        );
        // A band, not a floor: the class is cheap (every shape is well-formed,
        // so it neither aborts the script nor displaces the statements after
        // it), but it still has to leave most of the budget on the rest of the
        // grammar. Measured 332/1000 at the default rate — the rate was set
        // *to* that, having started at 120‰ and reached 534, which is more of
        // the corpus than one class should own however productive it is. The
        // bounds are wide enough that ordinary generator changes do not trip
        // them and narrow enough that losing or dominating the class does.
        assert!(
            (150..=550).contains(&carrying),
            "word shapes reach {carrying}/1000 scripts at the default rate, outside the intended band"
        );
    }

    /// At rate 0 the production is gone *and* consumes no random draw, so the
    /// stream is byte-identical to a build that never had it — which is what
    /// lets a historical finding still replay from its recorded seed.
    #[test]
    fn word_shape_rate_zero_opts_out_and_leaves_the_stream_alone() {
        let off = GenConfig {
            word_shape_permille: 0,
            ..GenConfig::default()
        };
        for seed in 0..400u64 {
            let src = generate(seed, &off);
            for &shape in WordShape::ALL {
                assert!(
                    !src.contains(shape.spelling()),
                    "rate 0 still emitted {shape:?} (seed {seed}):\n{src}"
                );
            }
            assert!(
                !src.contains("_wv") && !src.contains("_ws_def"),
                "rate 0 still emitted a word-shape probe (seed {seed}):\n{src}"
            );
        }
    }

    /// A rate of 0 opts out completely, so a campaign can keep the generator's
    /// pre-existing behaviour (and a registry's historical seeds replaying)
    /// while this production exists.
    #[test]
    fn malformed_expr_rate_zero_opts_out() {
        let off = GenConfig {
            malformed_expr_permille: 0,
            ..GenConfig::default()
        };
        for seed in 0..400u64 {
            let src = generate(seed, &off);
            assert!(
                !src.contains("_mx"),
                "rate 0 still emitted a malformed statement (seed {seed}):\n{src}"
            );
        }
    }

    /// The production must be live at the **default** rate: a knob nobody sets
    /// is a knob that never runs, and the point of this work was that the
    /// malformed-expression class was unreachable in ordinary campaigns. Also
    /// pins the rate's order of magnitude from above, so a routine campaign
    /// keeps spending most of its budget on well-formed semantics — a
    /// malformed statement is uncaught, so it aborts the script and everything
    /// after it goes unexercised.
    #[test]
    fn malformed_expressions_reach_scripts_at_the_default_rate() {
        let cfg = GenConfig::default();
        let corpus: Vec<String> = (0..1000u64).map(|s| generate(s, &cfg)).collect();
        let carrying = corpus.iter().filter(|s| s.contains("_mx")).count();
        assert!(
            carrying > 0,
            "the malformed production is dead at the default rate"
        );
        assert!(
            carrying < corpus.len() / 4,
            "malformed statements dominate the corpus at the default rate: {carrying}/1000"
        );
    }

    /// The coverage claim, evidenced rather than asserted: a malformed
    /// expression from this production really does make *our own* engine report
    /// an error, so the differential's status comparison has two comparable
    /// outcomes rather than one engine quietly succeeding.
    ///
    /// This is the regression guard for the bug that motivated the whole
    /// production — an unparsable runtime expression that evaluated to its own
    /// source text, so `expr {1+}` returned the string `1+` — which no
    /// generated script could reach before, and which a campaign therefore
    /// could not have caught.
    ///
    /// Drives `tcl-vm` in-process (lower → CFG → codegen → run) exactly as
    /// `wasm_diff`'s direct arm does, so no external binary is needed.
    #[test]
    fn malformed_expressions_error_on_our_engine() {
        for mode in Malformed::ALL {
            if MALFORMED_MODES_WITH_OPEN_DIVERGENCE.contains(mode) {
                continue;
            }
            for seed in 0..12u64 {
                let text = gen_with(seed, GenConfig::default()).render_malformed(*mode);
                let script = format!("set a 1\nset b 2\nputs [expr {{{text}}}]\n");
                assert!(
                    errors_on_our_engine(&script),
                    "{mode:?} did not error on our engine — the differential would \
                     see one engine succeed where C Tcl raises a syntax error: {text:?}"
                );
            }
        }
    }
}
