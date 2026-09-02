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

//! Tcl expression evaluator (compile-time constant folding).
//!
//! The **tree-walk is shared** with the runtime: [`eval_tcl_expr`] drives the
//! one [`tcl_syntax::expr::eval()`] over the AST and supplies this const-folder's
//! value ops via [`FoldOps`] (an `ExprOps` impl) — the same way the lexer/parser
//! are shared. Only the value-type-specific bits (the `i64`/`f64`/`Str`
//! [`FoldValue`], the operator helpers below, env-var resolution) live here.
//! `None` means "can't fold" — a variable not in the environment, a command
//! substitution, a domain error, or a value past a wide — and callers fall
//! through to the runtime form.
//!
//! Semantics follow C Tcl 9.0.2 (`tclExecute.c`, `tclBasic.c`):
//!
//! - Integer division floors toward negative infinity.
//! - Integer modulo: sign follows divisor.
//! - Exponentiation: special rules for `|base| ≤ 1` and negative
//!   exponents.
//! - Comparisons always return `Int(0)` or `Int(1)`; `eq`/`ne`/`lt`… compare the
//!   operands' raw text (so `5.00 eq 5.0` → 0).
//! - `round()` ties away from zero (not banker's round-half-to-even rounding).
//!
//! The iRules
//! word operators (`contains`/`starts_with`/`matches_glob`/`matches_regex`/
//! `equals`/`in`/`ni`) fold via [`tcl_syntax::expr::ExprOps::binary_other`].

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

use std::collections::HashMap;

use tcl_dialect::{NumberSyntax, StringCharacterModel};

use crate::expr_ast::{BinOp, ExprNode, UnaryOp};

/// Result of evaluating a constant Tcl expression.
#[derive(Debug, Clone, PartialEq)]
pub enum TclValue {
    /// Integer value that fits a wide (Tcl booleans are `Int(0)` / `Int(1)`).
    Int(i64),
    /// IEEE-754 double.
    Float(f64),
    /// Integer beyond a wide — the bignum rung of the numeric tower. Folded
    /// exactly (`expr {2**64}` → `18446744073709551616`), mirroring the VM's
    /// `num-bigint` tower and C Tcl's seamless wide→bignum promotion; a
    /// bignum result that fits a wide is always demoted back to `Int`
    /// (`$big - $big` → `Int(0)`), so `Big` is canonical: only ever
    /// beyond-`i64` magnitudes.
    Big(num_bigint::BigInt),
}

impl TclValue {
    /// Wrap an arbitrary-precision integer, demoting to a wide when it fits
    /// (the canonical form — mirrors the VM's `big_value` and C Tcl's
    /// `mp_int` narrowing).
    #[must_use]
    pub fn from_big(value: num_bigint::BigInt) -> Self {
        use num_traits::ToPrimitive;
        value.to_i64().map_or(Self::Big(value), Self::Int)
    }

    /// Return the raw float representation (converting integer → float
    /// when necessary). Used by arithmetic that promotes mixed operands.
    /// A bignum converts with the same precision loss (to ±∞ past the
    /// double range) as C Tcl's `Tcl_GetDoubleFromObj` on a bignum.
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        use num_traits::ToPrimitive;
        match self {
            Self::Int(i) => *i as f64,
            Self::Float(f) => *f,
            Self::Big(b) => b.to_f64().unwrap_or(f64::NAN),
        }
    }

    /// True when the value is non-zero (Tcl truthiness).
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        use num_traits::Zero;
        match self {
            Self::Int(i) => *i != 0,
            Self::Float(f) => *f != 0.0,
            Self::Big(b) => !b.is_zero(),
        }
    }

    /// The exact arbitrary-precision view of an integer value (`None` for a
    /// float) — the promotion step of the integer arithmetic path.
    fn to_bigint(&self) -> Option<num_bigint::BigInt> {
        match self {
            Self::Int(i) => Some(num_bigint::BigInt::from(*i)),
            Self::Big(b) => Some(b.clone()),
            Self::Float(_) => None,
        }
    }
}

/// Environment value kind — what callers can bind a variable to.
#[derive(Debug, Clone)]
pub enum EnvValue {
    /// Integer binding.
    Int(i64),
    /// Float binding.
    Float(f64),
    /// String binding — decoded as a literal on read.
    Str(String),
}

/// Variable environment for evaluation.
pub type Env = HashMap<String, EnvValue>;

// Public API

/// Evaluate an expression AST against `env`. Returns `None` when the
/// expression depends on runtime state or triggers a domain error.
///
/// The tree-walk is the **shared** [`tcl_syntax::expr::eval()`] (the same one the
/// runtime evaluates with); this const-folder supplies only its value ops via
/// [`FoldOps`]. A `None` result means "can't fold".
#[must_use]
pub fn eval_tcl_expr(node: &ExprNode, env: &Env) -> Option<TclValue> {
    // No dialect context: decline the iRules word-operator fold rather than
    // assume plain Tcl (safe — see `FoldOps::is_irules`).
    eval_with_config(node, env, None, None, false)
}

/// Like [`eval_tcl_expr`] but resolves the *dialect* so a leading-zero decimal
/// (`08`, `010`) is classified correctly in `==`/`!=`/`<`/… comparisons:
/// octal in Tcl 8.x (`08`/`09` invalid → string; `010` → 8), decimal in
/// Tcl 9.0 (`08` → 8, `010` → 10). All non-9.x dialects (tcl8.4/8.5/8.6,
/// f5-irules ≈ 8.4, f5-iapps ≈ 8.5/8.6, EDA) use the 8.x octal rule.
///
/// The dialect also bounds the math functions that fold: `min`/`max` (8.5) or
/// an `is*` classification (9.0) used in an older core folds nothing, since
/// the `::tcl::mathfunc::*` command it would call does not exist there.
#[must_use]
pub fn eval_tcl_expr_in_dialect(
    node: &ExprNode,
    env: &Env,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Option<TclValue> {
    eval_with_config(
        node,
        env,
        leading_zero_is_octal(dialect),
        math_func_ceiling_for_dialect(dialect),
        dialect.is_irules(),
    )
}

/// Like [`eval_tcl_expr`] but takes the leading-zero octal policy directly:
/// `Some(true)` = tcl8.x octal rule, `Some(false)` = tcl9.0 decimal rule,
/// `None` = decline to fold dialect-ambiguous leading-zero operands. Callers
/// that hold a `CommandRegistry` rather than a dialect string derive the flag
/// via `CommandRegistry::leading_zero_is_octal`.
///
/// Without a dialect the math-function set is unbounded (any known function
/// folds) — the caller has already decided the octal policy but not the
/// version tier, so this path never over-restricts.
#[must_use]
pub fn eval_tcl_expr_with_octal(
    node: &ExprNode,
    env: &Env,
    octal: Option<bool>,
) -> Option<TclValue> {
    // The caller has resolved the octal policy but not a dialect string, so
    // (as with `eval_tcl_expr`) decline the iRules word-operator fold rather
    // than assume plain Tcl.
    eval_with_config(node, env, octal, None, false)
}

/// Like [`eval_tcl_expr_with_octal`] but for the (more common) optimiser call
/// sites that already have both an `octal` policy and a resolved dialect
/// profile in scope — so, unlike `eval_tcl_expr_with_octal`'s plain
/// `None`-profile callers, these can resolve [`FoldOps::is_irules`] precisely instead of
/// defaulting it to declined (issue #983/#985 residual: several of these
/// sites were passing the string on to `leading_zero_is_octal` for the octal
/// policy while never using it to gate the iRules word-operator fold).
#[must_use]
pub fn eval_tcl_expr_with_octal_and_dialect(
    node: &ExprNode,
    env: &Env,
    octal: Option<bool>,
    profile: Option<&tcl_dialect::DialectProfile>,
) -> Option<TclValue> {
    eval_tcl_expr_with_policy(node, env, FoldPolicy::for_profile(octal, profile))
}

/// The dialect-derived facts a constant fold needs: the leading-zero octal
/// rule, and whether the dialect's `expr` grammar carries the iRules word
/// operators (`contains`, `starts_with`, `equals`, …).
///
/// Bundled into one `Copy` value rather than threaded as two parallel
/// parameters, because the passes that need it — SCCP, the static-loop
/// simulator — already carry `octal` through a long chain of helpers, several
/// of which sit on the `clippy::too_many_arguments` ceiling.  Adding a
/// further dialect fact then means extending this struct in one place instead
/// of every signature on the chain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FoldPolicy {
    /// Leading-zero octal policy: `Some(true)` = the 8.x octal rule,
    /// `Some(false)` = the 9.0 decimal rule, `None` = decline to fold a
    /// dialect-ambiguous leading-zero operand.
    pub octal: Option<bool>,
    /// Whether the active dialect's `expr` grammar has the iRules word
    /// operators.  `false` (the default) declines that fold, which is always
    /// safe — see [`FoldOps::is_irules`].
    pub is_irules: bool,
    /// What the active dialect counts as a string character: `Some(model)`
    /// folds character counts under that model, `None` declines a fold whose
    /// answer the Tcl 8 and Tcl 9 models disagree on — the same
    /// dialect-ambiguity rule as [`Self::octal`].
    pub characters: Option<StringCharacterModel>,
    /// Which numeric literal forms the active dialect accepts: `Some(syntax)`
    /// parses operands under it, `None` falls back to the 9.0 grammar for a
    /// caller that knows no dialect.
    ///
    /// Distinct from [`Self::octal`], which is only the leading-zero rule:
    /// this also decides whether `0b`/`0o` exist at all (8.5+), whether `0d`
    /// does (9.0+), and whether `_` separators do (9.0+) — so folding `0o17`
    /// for an 8.4 target correctly yields nothing.
    pub numbers: Option<NumberSyntax>,
    /// The active dialect's word-value rules — how a word-shaped list of this
    /// document divides, and whether a braced `\<newline>` folds.  Carried
    /// beside the numeral axis for the same reason: a fold that re-splits a
    /// literal list must split it the way the document's own list parser
    /// does.  Defaults to C Tcl for a caller with no dialect.
    pub word_rules: tcl_syntax::word_rules::WordValueRules,
}

impl FoldPolicy {
    /// The policy for an explicit octal rule with no known dialect — the
    /// iRules word operators are declined.
    #[must_use]
    pub const fn from_octal(octal: Option<bool>) -> Self {
        Self {
            octal,
            is_irules: false,
            characters: None,
            numbers: None,
            word_rules: tcl_syntax::word_rules::WordValueRules::TCL,
        }
    }

    /// The policy for an octal rule plus a resolved profile. Name parsing and
    /// alias handling happen before this point, so every fact comes from one
    /// canonical profile.
    #[must_use]
    pub fn for_profile(octal: Option<bool>, profile: Option<&tcl_dialect::DialectProfile>) -> Self {
        Self {
            octal,
            is_irules: profile.is_some_and(tcl_dialect::DialectProfile::is_irules),
            characters: profile.and_then(tcl_dialect::DialectProfile::character_model),
            numbers: profile.map(|p| NumberSyntax::of_profile(Some(p))),
            word_rules: tcl_syntax::word_rules::WordValueRules::of_profile(profile),
        }
    }

    /// The policy a registry's own dialect profile implies — both facts from
    /// the one source of truth, for the pipeline entry points that hold a
    /// registry rather than a dialect string.
    #[must_use]
    pub fn from_registry(registry: &tcl_registry::CommandRegistry) -> Self {
        Self {
            octal: registry.octal_fold_policy(),
            is_irules: registry
                .profile()
                .is_some_and(tcl_dialect::DialectProfile::is_irules),
            characters: registry.character_model(),
            numbers: Some(registry.numbers()),
            word_rules: tcl_syntax::word_rules::WordValueRules::of_profile(registry.profile()),
        }
    }
}

/// Evaluate `node` under a bundled [`FoldPolicy`] — the entry point for
/// passes that thread the policy rather than the raw octal flag.
#[must_use]
pub fn eval_tcl_expr_with_policy(
    node: &ExprNode,
    env: &Env,
    policy: FoldPolicy,
) -> Option<TclValue> {
    eval_with_config(node, env, policy.octal, None, policy.is_irules)
}

/// Parse one Tcl expression arithmetic operand as an integer under `policy`.
///
/// Unlike evaluating a standalone literal expression and converting its final
/// string result, this applies the dialect's operand coercion directly. That
/// distinction is observable for a bare leading-zero spelling (`010` is octal
/// 8 in Tcl 8.x and decimal 10 in Tcl 9.x). Floating-point values and invalid
/// integer spellings return `None`; beyond-wide values remain
/// [`TclValue::Big`] so native-width proofs can decline without truncation.
#[must_use]
pub fn parse_integer_operand_with_policy(text: &str, policy: FoldPolicy) -> Option<TclValue> {
    let text = text.trim();
    if let Some(value) = tcl_syntax::boolean::parse_boolean_word(text) {
        return Some(TclValue::Int(i64::from(value)));
    }
    match strict_number_for_dialect(
        &FoldValue::Str(text.to_owned()),
        policy.octal,
        policy.numbers.unwrap_or_default(),
    )? {
        value @ (TclValue::Int(_) | TclValue::Big(_)) => Some(value),
        TclValue::Float(_) => None,
    }
}

/// Whether the dialect's *runtime* reads a bare leading-zero integer as
/// octal, from the dialect profile's runtime base: `Some(true)` for the 8.x
/// runtimes (the F5 and EDA shells included), `Some(false)` for 9.x
/// runtimes (TIP 114/472 dropped the rule — `bpf` embeds Tcl 9.0, D7), and
/// `None` — abstain rather than guess — for a profile with no Tcl runtime
/// (`f5-bigip`) or an unknown dialect string (the permissive fallback,
/// design doc §11.1).
#[must_use]
pub fn leading_zero_is_octal(profile: &tcl_dialect::DialectProfile) -> Option<bool> {
    profile.leading_zero_is_octal.as_bool()
}

/// The newest `expr` math-function release available in `dialect`, or `None`
/// when the dialect has no expr-grammar base (don't restrict).
///
/// The dialect-name-keyed form of
/// [`tcl_registry::mathfunc::expr_grammar_ceiling`], which owns the mapping —
/// the registry is where mathfunc facts live, so the const-folder, the
/// availability diagnostic, and the LSP's hover/completion all read one
/// table.
#[must_use]
pub fn math_func_ceiling_for_dialect(
    dialect: &'static tcl_dialect::DialectProfile,
) -> Option<tcl_syntax::expr::mathfunc::MathFuncSince> {
    tcl_registry::mathfunc::expr_grammar_ceiling(dialect)
}

/// Whether `name` is a genuine built-in `expr` math function (`sin`, `max`,
/// …) available under `dialect` — a real name in
/// [`tcl_syntax::expr::mathfunc`] whose introducing release is at or before
/// [`math_func_ceiling_for_dialect`]'s ceiling for this dialect.  Free
/// function (rather than an `Analyser` method) so both the W123
/// unresolved-command check and the cross-namespace invocation resettlement
/// (`finalise_invocation_resolutions`, which runs after `self.result` is
/// borrowed mutably and so cannot call back through `&self`) share one
/// answer without either duplicating the other's logic.  Delegates to
/// [`tcl_registry::mathfunc::available_in_expr`].
#[must_use]
pub fn is_known_mathfunc_in_dialect(
    name: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> bool {
    tcl_registry::mathfunc::available_in_expr(name, dialect)
}

/// Whether `dialect` exposes math functions as literal `::tcl::mathfunc::*`
/// **commands** — TIP 232's own wrapper mechanism, landed in 8.5 alongside
/// `bool`/`entier`/`isqrt`/`min`/`max` (see
/// [`tcl_syntax::expr::mathfunc::MathFuncSince::Tcl85`]'s doc comment). This
/// is a coarser, single fact than [`is_known_mathfunc_in_dialect`]: it does
/// not vary per function name, because every wrapper command — even one
/// backing an 8.4-vintage function like `sin` — only exists from 8.5
/// onward. An 8.4-based dialect supports `expr {sin(1)}` (the internal
/// expr-grammar dispatch predates TIP 232) but not a bareword
/// `::tcl::mathfunc::sin 1` call (the command itself does not exist there) —
/// [`crate::analyser::diagnostics::unresolved`]'s W123 check uses this to
/// keep an *ordinary* call that happens to resolve to a `tcl::mathfunc`-
/// shaped qualified name from being waved through by the `expr`
/// function-call shortcut, which only reflects the first, narrower fact.
/// Delegates to [`tcl_registry::mathfunc::command_wrappers_available`].
#[must_use]
pub fn mathfunc_command_wrappers_available_in_dialect(
    dialect: &'static tcl_dialect::DialectProfile,
) -> bool {
    tcl_registry::mathfunc::command_wrappers_available(dialect)
}

fn eval_with_config(
    node: &ExprNode,
    env: &Env,
    octal: Option<bool>,
    math_since: Option<tcl_syntax::expr::mathfunc::MathFuncSince>,
    is_irules: bool,
) -> Option<TclValue> {
    let mut ops = FoldOps {
        env,
        ambiguous: false,
        octal,
        // Without an explicit grammar, infer from the leading-zero policy the
        // caller did resolve: the 8.x octal rule implies the 8.x numeric
        // grammar, and anything else is read as 9.0.
        numbers: if octal == Some(true) {
            NumberSyntax::Tcl85
        } else {
            NumberSyntax::default()
        },
        math_since,
        is_irules,
    };
    // The final value must reduce to a number (a bare string like `expr {"x"}`
    // doesn't fold) — `to_number` maps a `Str` result through `parse_literal`.
    let result = tcl_syntax::expr::eval(node, &mut ops).ok()?;
    if ops.ambiguous {
        // A comparison hit a leading-zero operand whose octal-vs-decimal
        // reading is dialect-dependent and the dialect is unknown — decline
        // to fold rather than pick one.
        return None;
    }
    result.to_number(ops.numbers)
}

// FoldOps — the const-folder's value ops for the shared expr walk

/// A const-fold value. `Str` keeps the operand's **raw text** and is parsed
/// lazily per context (numeric ops via [`parse_literal`]; string ops use it
/// verbatim) — exactly the `eval`-vs-`eval_as_string` split, so the raw-text
/// string-compare behaviour (`5.00 eq 5.0` → 0) is preserved.
#[derive(Clone)]
enum FoldValue {
    Int(i64),
    Big(num_bigint::BigInt),
    Float(f64),
    Str(String),
}

impl FoldValue {
    /// Interpret as a number, or `None` when the text isn't numeric.
    fn to_number(&self, numbers: NumberSyntax) -> Option<TclValue> {
        match self {
            FoldValue::Int(i) => Some(TclValue::Int(*i)),
            FoldValue::Big(b) => Some(TclValue::from_big(b.clone())),
            FoldValue::Float(f) => Some(TclValue::Float(*f)),
            FoldValue::Str(s) => parse_literal_in(s, numbers),
        }
    }
    /// Render as a string: raw for `Str`, canonical for numbers.
    fn to_string_val(&self) -> String {
        match self {
            FoldValue::Str(s) => s.clone(),
            FoldValue::Int(i) => format_tcl_value(&TclValue::Int(*i)),
            FoldValue::Big(b) => b.to_string(),
            FoldValue::Float(f) => format_tcl_value(&TclValue::Float(*f)),
        }
    }
    fn from_tcl(v: TclValue) -> FoldValue {
        match v {
            TclValue::Int(i) => FoldValue::Int(i),
            TclValue::Big(b) => FoldValue::Big(b),
            TclValue::Float(f) => FoldValue::Float(f),
        }
    }
}

/// The const-folder's [`ExprOps`](tcl_syntax::expr::ExprOps). `Error = ()` is the
/// "can't fold" signal (mapped to the public `Option`); `$var` resolves from the
/// `env`, `[cmd]`/`Raw` are opaque.
struct FoldOps<'a> {
    env: &'a Env,
    /// Set when a comparison's folded result would be unreliable, so
    /// [`eval_tcl_expr`] declines to fold rather than risk a false
    /// I230 unreachable-branch. Two triggers: a leading-zero integer operand
    /// whose octal-vs-decimal reading is dialect-dependent while
    /// [`Self::octal`] is unknown (`None`), and the wide-vs-2⁶³-double
    /// comparison whose answer is platform-dependent in C Tcl (see
    /// [`numeric_cmp`]).
    ambiguous: bool,
    /// How a bare leading-zero integer (`08`, `010`) is read in `==`/`!=`/`<`/…
    /// numeric eligibility: `Some(true)` = octal (Tcl 8.x — `08`/`09` invalid →
    /// string, `010` → 8), `Some(false)` = decimal (Tcl 9.0 — `08` → 8,
    /// `010` → 10), `None` = dialect unknown → decline (see [`Self::ambiguous`]).
    octal: Option<bool>,
    /// The release's numeric-literal grammar for reading operands — which radix
    /// prefixes exist and whether `_` separates digits. Broader than
    /// [`Self::octal`], which is only the leading-zero rule.
    numbers: NumberSyntax,
    /// The newest math-function release the active dialect provides.  A call
    /// to a function introduced *after* this tier folds nothing — the
    /// `::tcl::mathfunc::*` command it would dispatch to does not exist in that
    /// core, so the runtime would error rather than produce a constant.
    /// `None` leaves the set unbounded (dialect not resolved).
    math_since: Option<tcl_syntax::expr::mathfunc::MathFuncSince>,
    /// Whether the active dialect is iRules — the only dialect the iRules
    /// word operators (`contains`/`starts_with`/`equals`/`matches_glob`/
    /// `matches_regex`/…) are real in (see [`Self::binary_other`]).
    /// Lexing already gates which operators can appear in the AST at all
    /// (`irules_ops()`), but several call sites into this evaluator (the
    /// optimiser's `parse_expr(text, None)` sites) have no dialect to hand,
    /// so this is a defence-in-depth check at the fold site itself rather
    /// than trusting the lexer gate alone (issue #983/#985 residual).
    /// `false` — including when the dialect is genuinely unknown — declines
    /// the fold; that is always safe, it just forgoes an optimisation.
    is_irules: bool,
}

/// A comparison operand's numeric classification under the active dialect.
enum Operand {
    /// A definite number (used for a numeric comparison).
    Num(TclValue),
    /// Not a number in this dialect (used for a string comparison).
    Str,
    /// A leading-zero integer whose reading is dialect-dependent and the
    /// dialect is unknown — the whole fold must be declined.
    Ambiguous,
}

/// Classify a comparison operand as [`Operand::Num`] / [`Operand::Str`] /
/// [`Operand::Ambiguous`], applying the dialect's leading-zero rule.
fn classify_operand(value: &FoldValue, octal: Option<bool>, numbers: NumberSyntax) -> Operand {
    let s = match value {
        FoldValue::Int(_) | FoldValue::Big(_) | FoldValue::Float(_) => {
            // Already numeric — `to_number` cannot reach the literal parser
            // here, so the grammar it is given is immaterial.
            return Operand::Num(value.to_number(NumberSyntax::default()).unwrap());
        }
        FoldValue::Str(s) => s.as_str(),
    };
    if is_bare_leading_zero(s) {
        return match octal {
            None => Operand::Ambiguous,
            // 8.x octal: a valid octal (`010`) is a number; an invalid one
            // (`08`/`09`) is not — Tcl treats it as a string.
            Some(true) => parse_octal_literal(s).map_or(Operand::Str, Operand::Num),
            // 9.0 decimal: the shared number grammar already reads it as decimal.
            Some(false) => strict_number(value, numbers).map_or(Operand::Str, Operand::Num),
        };
    }
    strict_number(value, numbers).map_or(Operand::Str, Operand::Num)
}

/// Whether `s` is a bare leading-zero integer (`08`, `-010`) — the only
/// dialect-dependent number form. Excludes `0` alone, `0x`/`0o`/`0b` prefixes
/// (a non-digit follows the `0`), and floats (`0.5`).
fn is_bare_leading_zero(s: &str) -> bool {
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    digits.len() > 1 && digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit())
}

/// Parse a Tcl 8.x octal integer (`010` → 8, `-077` → -63). Returns `None`
/// for an invalid octal (`08`/`09`), which Tcl 8.x treats as a string.
fn parse_octal_literal(s: &str) -> Option<TclValue> {
    let (neg, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let v = i64::from_str_radix(digits, 8).ok()?;
    Some(TclValue::Int(if neg { -v } else { v }))
}

impl tcl_syntax::expr::ExprOps for FoldOps<'_> {
    type Value = FoldValue;
    type Error = ();

    fn literal(&mut self, text: &str) -> Result<FoldValue, ()> {
        Ok(FoldValue::Str(text.to_owned()))
    }
    fn string(&mut self, inner: &str) -> Result<FoldValue, ()> {
        Ok(FoldValue::Str(inner.to_owned()))
    }
    fn var(&mut self, name: &str) -> Result<FoldValue, ()> {
        match self.env.get(name) {
            Some(EnvValue::Int(i)) => Ok(FoldValue::Int(*i)),
            Some(EnvValue::Float(f)) => Ok(FoldValue::Float(*f)),
            Some(EnvValue::Str(s)) => Ok(FoldValue::Str(s.clone())),
            None => Err(()), // unbound → can't fold
        }
    }
    fn command(&mut self, _script: &str) -> Result<FoldValue, ()> {
        Err(()) // command substitution is opaque at compile time
    }
    fn call(&mut self, function: &str, args: Vec<FoldValue>) -> Result<FoldValue, ()> {
        use tcl_syntax::expr::mathfunc::{Num, accepts_boolean_operand, added_in, dispatch};
        let name = function.to_ascii_lowercase();
        if matches!(name.as_str(), "rand" | "srand") {
            return Err(()); // non-deterministic
        }
        // A function newer than the dialect provides has no `::tcl::mathfunc`
        // command to run — folding it would invent a value the real
        // interpreter never yields (it would error), so decline.
        if let (Some(ceiling), Some(since)) = (self.math_since, added_in(&name))
            && since > ceiling
        {
            return Err(());
        }
        // Math functions are the shared `tcl_syntax::expr::mathfunc` (the same
        // dispatch the runtime evaluates). Map `TclValue` → `Num` → result.
        // Every function except `bool` reads its operand as a strict number —
        // `Tcl_GetBoolean` coercion (`true`→1) would let the folder turn an
        // error (`abs(true)`) into a value, so parse strictly unless the
        // function itself accepts boolean words (the registry of that fact is
        // the mathfunc module, not a name check here).
        let boolean_ok = accepts_boolean_operand(&name);
        let octal = self.octal;
        let numbers = self.numbers;
        let nums: Option<Vec<Num>> = args
            .iter()
            .map(|v| {
                let parsed = if boolean_ok {
                    v.to_number(numbers)
                } else {
                    strict_number_for_dialect(v, octal, numbers)
                };
                parsed.and_then(|t| match t {
                    TclValue::Int(i) => Some(Num::Int(i)),
                    TclValue::Float(f) => Some(Num::Float(f)),
                    // A beyond-wide integer argument: the math functions
                    // dispatch over the wide/double pair, so decline rather
                    // than approximate.
                    TclValue::Big(_) => None,
                })
            })
            .collect();
        match dispatch(&name, &nums.ok_or(())?).ok_or(())? {
            Num::Int(i) => Ok(FoldValue::Int(i)),
            Num::Float(f) => Ok(FoldValue::Float(f)),
        }
    }

    fn arith(&mut self, op: BinOp, left: FoldValue, right: FoldValue) -> Result<FoldValue, ()> {
        // Arithmetic operands are strict numbers: Tcl's `+`/`-`/`*`/… read
        // them with `Tcl_GetNumberFromObj`, which rejects boolean words, so
        // `expr {true + 0}` is an error, not `1`. `strict_number_for_dialect`
        // omits the boolean coercion `to_number`/`parse_literal` add, and
        // additionally honours the dialect's leading-zero rule (see its doc).
        let a = strict_number_for_dialect(&left, self.octal, self.numbers).ok_or(())?;
        let b = strict_number_for_dialect(&right, self.octal, self.numbers).ok_or(())?;
        apply_binary(op, a, b).map(FoldValue::from_tcl).ok_or(())
    }
    fn unary(&mut self, op: UnaryOp, value: FoldValue) -> Result<FoldValue, ()> {
        match op {
            // Logical negation *does* take a boolean (`expr {!true}` → 0), so
            // it keeps the boolean-accepting `to_number` coercion. Truthiness
            // of a bare leading-zero operand is dialect-invariant (a run of
            // zero digits is zero under either reading, and any other
            // leading-zero run is non-zero under both), so no dialect
            // handling is needed here.
            UnaryOp::Not | UnaryOp::WordNot => {
                // `!NaN` is the same boolean-context domain error as `?:` on
                // NaN — decline, never fold a truth value.
                let truthy = match value.to_number(self.numbers).ok_or(())? {
                    TclValue::Float(f) if f.is_nan() => return Err(()),
                    v => v.is_truthy(),
                };
                Ok(FoldValue::Int(i64::from(!truthy)))
            }
            // Arithmetic/bitwise unaries reject boolean words like the binary
            // arithmetic path (`expr {-true}`, `expr {~yes}` are errors), and
            // are dialect-sensitive the same way `arith` is (`expr {-010}`
            // is `-8` in tcl8.x, `-10` in tcl9.0).
            UnaryOp::Pos => {
                match strict_number_for_dialect(&value, self.octal, self.numbers).ok_or(())? {
                    // `+NaN` is "can't use non-numeric floating-point value as
                    // operand" in C — never a foldable value.
                    TclValue::Float(f) if f.is_nan() => Err(()),
                    v => Ok(FoldValue::from_tcl(v)),
                }
            }
            UnaryOp::Neg => match strict_number_for_dialect(&value, self.octal, self.numbers)
                .ok_or(())?
            {
                TclValue::Int(i) => Ok(match i.checked_neg() {
                    Some(n) => FoldValue::Int(n),
                    // −i64::MIN promotes to the bignum tier, exactly as C.
                    None => FoldValue::from_tcl(TclValue::from_big(-num_bigint::BigInt::from(i))),
                }),
                TclValue::Big(b) => Ok(FoldValue::from_tcl(TclValue::from_big(-b))),
                // `-NaN` is the same operand error as `+NaN` — decline.
                TclValue::Float(f) if f.is_nan() => Err(()),
                TclValue::Float(f) => Ok(FoldValue::Float(-f)),
            },
            UnaryOp::BitNot => {
                match strict_number_for_dialect(&value, self.octal, self.numbers).ok_or(())? {
                    TclValue::Int(i) => Ok(FoldValue::Int(!i)),
                    // Two's-complement `~x` is `-x - 1` at any width.
                    TclValue::Big(b) => Ok(FoldValue::from_tcl(TclValue::from_big(-b - 1))),
                    TclValue::Float(_) => Err(()),
                }
            }
        }
    }

    fn compare_numeric(
        &mut self,
        left: &FoldValue,
        right: &FoldValue,
    ) -> Option<tcl_syntax::expr::NumericCompare> {
        // `==` / `!=` / `<` / … are polymorphic: Tcl compares numerically only
        // when *both* operands are valid numbers, otherwise as strings. The
        // *strict* number grammar (no boolean words — `parse_literal` would
        // coerce `true`/`yes`/… to `1`/`0`, but Tcl does NOT treat them as
        // numbers for comparison: `expr {"true" == "1"}` → 0 string compare,
        // `expr {"true" + 0}` errors) is applied via `classify_operand`, which
        // also resolves the dialect's leading-zero rule (`08` octal in 8.x,
        // decimal in 9.0). Returning `None` falls the shared evaluator back to
        // `compare_string`, matching Tcl. A leading-zero operand under an
        // unknown dialect is `Ambiguous` → mark the fold unreliable so
        // `eval_tcl_expr` declines entirely rather than pick a dialect.
        let (lo, ro) = (
            classify_operand(left, self.octal, self.numbers),
            classify_operand(right, self.octal, self.numbers),
        );
        if matches!(lo, Operand::Ambiguous) || matches!(ro, Operand::Ambiguous) {
            self.ambiguous = true;
            return None;
        }
        match (lo, ro) {
            (Operand::Num(a), Operand::Num(b)) => {
                let outcome = numeric_cmp(a, b);
                if outcome.is_none() {
                    // The comparison itself can't be folded reliably (the 2⁶³
                    // C-UB sliver — see `numeric_cmp`): decline the whole
                    // fold. Returning bare `None` would instead fall back to
                    // a string comparison of two numbers, computing a wrong
                    // value.
                    self.ambiguous = true;
                }
                outcome
            }
            _ => None,
        }
    }
    fn compare_string(&mut self, left: &FoldValue, right: &FoldValue) -> std::cmp::Ordering {
        left.to_string_val().cmp(&right.to_string_val())
    }
    fn in_list(&mut self, needle: &FoldValue, list: &FoldValue) -> Result<bool, ()> {
        let n = needle.to_string_val();
        Ok(split_tcl_list(&list.to_string_val()).contains(&n))
    }

    fn to_bool(&mut self, value: &FoldValue) -> Result<bool, ()> {
        // Boolean contexts (`?:`, `&&`, `||`) reject NaN — a domain error in
        // C Tcl ("floating point value is Not a Number"), so the fold
        // declines rather than pick a truth value.
        match value.to_number(self.numbers).ok_or(())? {
            TclValue::Float(f) if f.is_nan() => Err(()),
            v => Ok(v.is_truthy()),
        }
    }
    fn bool_value(&mut self, b: bool) -> FoldValue {
        FoldValue::Int(i64::from(b))
    }
    fn unsupported(&mut self, _what: &str) {}

    /// The iRules dialect string operators (`contains`/`starts_with`/`equals`/
    /// `matches_glob`/`matches_regex`/…) — apply to the operands as strings.
    /// Declines the fold outright unless [`Self::is_irules`] is set: these
    /// operators are only real Tcl outside iRules by way of a lexer bug the
    /// lexer's own `irules_ops()` gate already prevents, but this is the
    /// defence-in-depth check for the call sites that reach this evaluator
    /// with no dialect context to gate on at lex time.
    fn binary_other(
        &mut self,
        op: BinOp,
        left: FoldValue,
        right: FoldValue,
    ) -> Result<FoldValue, ()> {
        if !self.is_irules {
            return Err(());
        }
        apply_irules_string_op(op, &left.to_string_val(), &right.to_string_val())
            .map(FoldValue::from_tcl)
            .ok_or(())
    }
}

/// Render a `TclValue` as a Tcl source literal. Matches Tcl's
/// `Tcl_GetStringFromObj` output for numbers.
#[must_use]
pub fn format_tcl_value(v: &TclValue) -> String {
    match v {
        TclValue::Int(i) => i.to_string(),
        // The shared canonical double formatter (also the runtime's `double`
        // string rep): integer-valued floats get `.0`, plus `Inf`/`NaN`.
        TclValue::Float(f) => tcl_syntax::number::format_double(*f),
        // A bignum's canonical string rep is its decimal spelling.
        TclValue::Big(b) => b.to_string(),
    }
}

// Core dispatch

// Math function calls

// Literals and variables

/// Parse a numeric/boolean literal. Supports `0x`/`0o`/`0b` prefixes,
/// Tcl-style leading-zero decimals (e.g. `0005`), floats, and the
/// Tcl boolean spellings.
#[must_use]
pub fn parse_literal(text: &str) -> Option<TclValue> {
    parse_literal_in(text, NumberSyntax::default())
}

/// [`parse_literal`] under an explicit release grammar — the form the folder
/// uses, so an operand is read for the dialect being compiled for (`0o17` is
/// nothing under 8.4, and `0755` is 493 up to 8.6).
#[must_use]
pub fn parse_literal_in(text: &str, numbers: NumberSyntax) -> Option<TclValue> {
    use tcl_syntax::number::Number;
    // Boolean keywords (`Tcl_GetBoolean`, not part of the number grammar).
    // Accepts unique-prefix spellings (`tr`, `ye`, `of`) like real Tcl.
    if let Some(b) = tcl_syntax::boolean::parse_boolean_word(text) {
        return Some(TclValue::Int(i64::from(b)));
    }
    // The numeric grammar is the shared `tcl_syntax::number` (the same
    // `TclParseNumber` port the runtime const-folds with): `0x`/`0o`/`0b`,
    // leading-zero decimal, `_` separators, `Inf`/`NaN`. A magnitude past a
    // wide builds the exact bignum (P4 of type-tracking.md), matching C
    // Tcl's seamless promotion.
    match tcl_syntax::number::parse_whole_with(
        text,
        tcl_syntax::number::ParseFlags::for_syntax(numbers),
    )? {
        Number::Int(v) => Some(TclValue::Int(v)),
        Number::Double(d) => Some(TclValue::Float(d)),
        Number::Nan { .. } => Some(TclValue::Float(f64::NAN)),
        Number::Big {
            negative,
            radix,
            digits,
        } => big_from_parts(negative, radix, &digits),
    }
}

/// Build the exact [`TclValue`] of a beyond-wide integer literal from the
/// shared grammar's `Number::Big` parts (mirrors the VM's `value_as_bigint`).
fn big_from_parts(
    negative: bool,
    radix: tcl_syntax::number::Radix,
    digits: &str,
) -> Option<TclValue> {
    let b = num_bigint::BigInt::parse_bytes(digits.as_bytes(), radix as u32)?;
    Some(TclValue::from_big(if negative { -b } else { b }))
}

/// Parse a *strict* Tcl number — the number grammar only, **without** the
/// boolean-word coercion [`parse_literal`] adds. Used for the polymorphic
/// comparison operators, whose numeric-vs-string decision follows Tcl's number
/// rules: `true`/`yes`/`off`/… are strings, not numbers.
///
/// Reads the numeral under `numbers`, so every release-dependent spelling is
/// judged against the fold's own target: `1_0` and `0d1` are numbers only from
/// 9.0, `0o17`/`0b101` only from 8.5. The bare-leading-zero case is decided one
/// level up by [`strict_number_for_dialect`], which can also abstain when the
/// two readings disagree.
fn strict_number(value: &FoldValue, numbers: NumberSyntax) -> Option<TclValue> {
    use tcl_syntax::number::Number;
    match value {
        FoldValue::Int(i) => Some(TclValue::Int(*i)),
        FoldValue::Float(f) => Some(TclValue::Float(*f)),
        FoldValue::Big(b) => Some(TclValue::from_big(b.clone())),
        FoldValue::Str(s) => match tcl_syntax::number::parse_whole_with(
            s,
            tcl_syntax::number::ParseFlags::for_syntax(numbers),
        )? {
            Number::Int(v) => Some(TclValue::Int(v)),
            Number::Double(d) => Some(TclValue::Float(d)),
            Number::Nan { .. } => Some(TclValue::Float(f64::NAN)),
            Number::Big {
                negative,
                radix,
                digits,
            } => big_from_parts(negative, radix, &digits),
        },
    }
}

/// Like [`strict_number`] but for the arithmetic/unary/math-function
/// operand path, which — unlike [`strict_number`]'s comparison callers —
/// must honour the active dialect's leading-zero rule: `Some(true)` = tcl8.x
/// octal (`010` → 8; `08`/`09` are invalid octal, and Tcl raises an error
/// for them in arithmetic context, so this declines rather than guess),
/// `Some(false)` = tcl9.0 decimal (`010` → 10, matching the shared grammar
/// [`strict_number`] already applies).
///
/// `None` (dialect unknown) folds only when the octal and decimal readings
/// *agree* — e.g. `07` is `7` under both (`7` is a valid octal digit), so
/// it isn't actually ambiguous — and declines when they disagree (`010`:
/// octal 8 vs decimal 10) or either reading is invalid (`08`/`09`: invalid
/// octal, valid decimal). This is more precise than blanket-declining every
/// bare leading-zero operand under an unknown dialect while staying sound:
/// the two candidate dialects are the only readings a real Tcl interpreter
/// could apply.
///
/// Unlike [`classify_operand`], a leading-zero operand that fails to parse
/// under the active dialect always declines (`None`) here rather than
/// falling back to a string classification — arithmetic has no string
/// fallback in Tcl (`expr {08 + 1}` is a runtime error under the 8.x octal
/// rule, not a string operation), so "can't fold" is the only sound outcome.
fn strict_number_for_dialect(
    value: &FoldValue,
    octal: Option<bool>,
    numbers: NumberSyntax,
) -> Option<TclValue> {
    let FoldValue::Str(s) = value else {
        return strict_number(value, numbers);
    };
    if is_bare_leading_zero(s) {
        return match octal {
            Some(true) => parse_octal_literal(s),
            Some(false) => strict_number(value, numbers),
            None => match (parse_octal_literal(s), strict_number(value, numbers)) {
                (Some(o), Some(d)) if o == d => Some(o),
                _ => None,
            },
        };
    }
    strict_number(value, numbers)
}

// Binary operators

fn apply_binary(op: BinOp, a: TclValue, b: TclValue) -> Option<TclValue> {
    match op {
        // Arithmetic.
        BinOp::Add => Some(arith(&a, &b, i64::checked_add, |x, y| x + y, |x, y| x + y)?),
        BinOp::Sub => Some(arith(&a, &b, i64::checked_sub, |x, y| x - y, |x, y| x - y)?),
        BinOp::Mul => Some(arith(&a, &b, i64::checked_mul, |x, y| x * y, |x, y| x * y)?),
        BinOp::Div => tcl_div(&a, &b),
        BinOp::Mod => tcl_mod(&a, &b),
        BinOp::Pow => tcl_pow(&a, &b),

        // Shifts and bitwise — integer only (wide or bignum; exact).
        BinOp::LShift => match (&a, &b) {
            (TclValue::Int(0), TclValue::Int(y)) if *y >= 0 => Some(TclValue::Int(0)),
            // The shift count is capped so a folded literal stays small
            // (`1 << 100000` is a real Tcl value but not one worth
            // materialising into source text — decline past the cap).
            (TclValue::Int(_) | TclValue::Big(_), TclValue::Int(y)) if (0..=256).contains(y) => {
                let x = a.to_bigint()?;
                Some(TclValue::from_big(x << u32::try_from(*y).ok()?))
            }
            _ => None,
        },
        BinOp::RShift => match (&a, &b) {
            (TclValue::Int(x), TclValue::Int(y)) if *y >= 0 => {
                // At y == 64 the value has been fully shifted out, so an
                // arithmetic right shift yields the sign bit replicated
                // (0 for non-negative, -1 for negative). Executing `x >> 64`
                // here would panic on shift-overflow in debug builds and mask
                // to `x >> 0` in release. The boundary is therefore `>= 64`,
                // not `> 64` (i64 has 64 bits).
                if *y >= 64 {
                    Some(TclValue::Int(if *x >= 0 { 0 } else { -1 }))
                } else {
                    Some(TclValue::Int(x >> y))
                }
            }
            (TclValue::Big(x), TclValue::Int(y)) if *y >= 0 => {
                use num_traits::Signed;
                // A count past the operand's width collapses to the sign.
                match usize::try_from(*y) {
                    Ok(count) if count <= x.bits() as usize => {
                        Some(TclValue::from_big(x.clone() >> count))
                    }
                    _ => Some(TclValue::Int(if x.is_negative() { -1 } else { 0 })),
                }
            }
            _ => None,
        },
        BinOp::BitAnd => bitwise(&a, &b, |x, y| x & y, |x, y| x & y),
        BinOp::BitOr => bitwise(&a, &b, |x, y| x | y, |x, y| x | y),
        BinOp::BitXor => bitwise(&a, &b, |x, y| x ^ y, |x, y| x ^ y),

        // Numeric comparison — always returns Int(0) or Int(1). A NaN operand
        // is unordered: unequal to everything, ordered against nothing.
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            use std::cmp::Ordering;
            use tcl_syntax::expr::NumericCompare;
            let outcome = numeric_cmp(a, b)?;
            let result = match (op, outcome) {
                (BinOp::Ne, NumericCompare::Unordered) => true,
                (_, NumericCompare::Unordered) => false,
                (BinOp::Eq, NumericCompare::Ordered(o)) => o == Ordering::Equal,
                (BinOp::Ne, NumericCompare::Ordered(o)) => o != Ordering::Equal,
                (BinOp::Lt, NumericCompare::Ordered(o)) => o == Ordering::Less,
                (BinOp::Le, NumericCompare::Ordered(o)) => o != Ordering::Greater,
                (BinOp::Gt, NumericCompare::Ordered(o)) => o == Ordering::Greater,
                (_, NumericCompare::Ordered(o)) => o != Ordering::Less,
            };
            Some(TclValue::Int(i64::from(result)))
        }

        // String-comparison ops (eq/ne/lt/le/gt/ge) are routed through
        // `apply_string_compare` from `eval_binary` before they reach here
        // (they need string, not numeric, operands), as are the
        // short-circuit and iRules string ops.
        BinOp::StrEq
        | BinOp::StrNe
        | BinOp::StrLt
        | BinOp::StrLe
        | BinOp::StrGt
        | BinOp::StrGe
        | BinOp::And
        | BinOp::Or
        | BinOp::WordAnd
        | BinOp::WordOr
        | BinOp::Contains
        | BinOp::StartsWith
        | BinOp::EndsWith
        | BinOp::StrEquals
        | BinOp::Matches
        | BinOp::MatchesGlob
        | BinOp::MatchesRegex
        | BinOp::In
        | BinOp::Ni => None,
    }
}

/// Integer-only bitwise operator over the wide/bignum tiers (`num-bigint`'s
/// negative-operand semantics are two's-complement, matching Tcl).
fn bitwise<F, B>(a: &TclValue, b: &TclValue, int_op: F, big_op: B) -> Option<TclValue>
where
    F: FnOnce(i64, i64) -> i64,
    B: FnOnce(&num_bigint::BigInt, &num_bigint::BigInt) -> num_bigint::BigInt,
{
    match (a, b) {
        (TclValue::Int(x), TclValue::Int(y)) => Some(TclValue::Int(int_op(*x, *y))),
        (TclValue::Big(_) | TclValue::Int(_), TclValue::Big(_) | TclValue::Int(_)) => {
            Some(TclValue::from_big(big_op(&a.to_bigint()?, &b.to_bigint()?)))
        }
        _ => None,
    }
}

/// Whether a value's `f64` view is exact — always true for wides and floats
/// (the wide→double rounding is C's own conversion), and true for a bignum
/// only when the conversion round-trips. An inexact bignum→double declines
/// the fold rather than depend on rounding-parity with `TclBignumToDouble`.
fn big_to_f64_exact(v: &TclValue) -> bool {
    match v {
        TclValue::Big(b) => num_traits::FromPrimitive::from_f64(v.as_f64())
            .is_some_and(|back: num_bigint::BigInt| back == *b),
        TclValue::Int(_) | TclValue::Float(_) => true,
    }
}

fn arith<F, B, G>(a: &TclValue, b: &TclValue, int_op: F, big_op: B, float_op: G) -> Option<TclValue>
where
    F: FnOnce(i64, i64) -> Option<i64>,
    B: FnOnce(&num_bigint::BigInt, &num_bigint::BigInt) -> num_bigint::BigInt,
    G: FnOnce(f64, f64) -> f64,
{
    match (a, b) {
        // Wide fast path; overflow promotes to the exact bignum tier
        // (never wraps, never declines — C Tcl's seamless promotion).
        (TclValue::Int(x), TclValue::Int(y)) => Some(match int_op(*x, *y) {
            Some(r) => TclValue::Int(r),
            None => TclValue::from_big(big_op(
                &num_bigint::BigInt::from(*x),
                &num_bigint::BigInt::from(*y),
            )),
        }),
        // Any float operand contaminates to double arithmetic — including a
        // bignum operand, with C's same double-conversion rounding. Two
        // divergence guards (oracle-pinned):
        // - a NaN *result* is C's "domain error" (`Inf - Inf`, `Inf * 0`),
        //   and a NaN *operand* is "can't use non-numeric floating-point
        //   value" — both decline (the NaN result covers both);
        // - a bignum whose double conversion is inexact declines rather
        //   than bet on rounding parity with C's `TclBignumToDouble`.
        (TclValue::Float(_), _) | (_, TclValue::Float(_)) => {
            if !big_to_f64_exact(a) || !big_to_f64_exact(b) {
                return None;
            }
            let r = float_op(a.as_f64(), b.as_f64());
            if r.is_nan() {
                return None;
            }
            Some(TclValue::Float(r))
        }
        // At least one bignum, no float: exact arbitrary precision.
        _ => Some(TclValue::from_big(big_op(&a.to_bigint()?, &b.to_bigint()?))),
    }
}

/// Numeric comparison outcome for two folded numbers, exact across the whole
/// wide range (C Tcl's `TclCompareTwoNumbers` — a both-as-`f64` comparison
/// merges distinct wides above 2⁵³), with NaN as `Unordered` (Tcl's "`!=` is
/// true, every other comparison false" rule).
///
/// `None` declines the fold: a double equal to exactly 2⁶³ compared against a
/// wide that rounds onto it hits undefined behaviour in C Tcl's double→wide
/// cast, and real interpreters answer platform-dependently (x86-64 tclsh 8.6
/// says the wide is *greater*; the saturating ARM64 conversion says *equal*;
/// the exact answer is *less*). No fold can match every runtime, so none is
/// made.
fn numeric_cmp(a: TclValue, b: TclValue) -> Option<tcl_syntax::expr::NumericCompare> {
    use tcl_syntax::expr::NumericCompare;
    Some(match (a, b) {
        (TclValue::Int(x), TclValue::Int(y)) => NumericCompare::Ordered(x.cmp(&y)),
        (TclValue::Float(x), TclValue::Float(y)) => NumericCompare::from_partial(x.partial_cmp(&y)),
        (TclValue::Int(x), TclValue::Float(y)) => int_vs_double(x, y)?,
        (TclValue::Float(x), TclValue::Int(y)) => match int_vs_double(y, x)? {
            NumericCompare::Ordered(ord) => NumericCompare::Ordered(ord.reverse()),
            NumericCompare::Unordered => NumericCompare::Unordered,
        },
        // Integer-vs-integer with a bignum side is exact at any width; a
        // bignum is canonical (beyond i64), so against a wide only the sign
        // matters and against a double the bignum's double view is what C
        // compares (`mp_int` → double, same rounding).
        (TclValue::Big(x), TclValue::Big(y)) => NumericCompare::Ordered(x.cmp(&y)),
        (TclValue::Big(x), TclValue::Int(y)) => {
            NumericCompare::Ordered(x.cmp(&num_bigint::BigInt::from(y)))
        }
        (TclValue::Int(x), TclValue::Big(y)) => {
            NumericCompare::Ordered(num_bigint::BigInt::from(x).cmp(&y))
        }
        (TclValue::Big(x), TclValue::Float(y)) => big_vs_double(&x, y),
        (TclValue::Float(x), TclValue::Big(y)) => match big_vs_double(&y, x) {
            NumericCompare::Ordered(o) => NumericCompare::Ordered(o.reverse()),
            NumericCompare::Unordered => NumericCompare::Unordered,
        },
    })
}

/// Exact bignum-vs-double comparison — C converts the double to a bignum
/// (`Tcl_InitBignumFromDouble`) and compares exactly, so
/// `18446744073709551617 == 1.8446744073709552e19` is FALSE even though the
/// bignum's rounded double view equals the float. NaN is unordered; ±Inf
/// orders past every integer; a finite double compares by exact integer
/// part with the fraction as tiebreak.
fn big_vs_double(x: &num_bigint::BigInt, d: f64) -> tcl_syntax::expr::NumericCompare {
    use std::cmp::Ordering;
    use tcl_syntax::expr::NumericCompare;
    if d.is_nan() {
        return NumericCompare::Unordered;
    }
    if d.is_infinite() {
        return NumericCompare::Ordered(if d > 0.0 {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let trunc = <num_bigint::BigInt as num_traits::FromPrimitive>::from_f64(d.trunc())
        .expect("finite double truncation is an exact integer");
    match x.cmp(&trunc) {
        Ordering::Equal => {
            let frac = d.fract();
            NumericCompare::Ordered(if frac > 0.0 {
                Ordering::Less
            } else if frac < 0.0 {
                Ordering::Greater
            } else {
                Ordering::Equal
            })
        }
        other => NumericCompare::Ordered(other),
    }
}

/// Exact wide-vs-double comparison, declining (`None`) in the C-UB sliver
/// documented on [`numeric_cmp`].
fn int_vs_double(w: i64, d: f64) -> Option<tcl_syntax::expr::NumericCompare> {
    use tcl_syntax::expr::NumericCompare;
    // 2⁶³ as a double. A wide in (2⁶³−1024, 2⁶³) rounds onto it, which is the
    // window where C Tcl's `(Tcl_WideInt) d2` conversion is undefined.
    const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;
    if d == TWO_POW_63 && w as f64 == TWO_POW_63 {
        return None;
    }
    Some(
        tcl_syntax::number::compare_int_double(i128::from(w), d)
            .map_or(NumericCompare::Unordered, NumericCompare::Ordered),
    )
}

fn tcl_div(a: &TclValue, b: &TclValue) -> Option<TclValue> {
    use num_integer::Integer;
    match (a, b) {
        (TclValue::Int(_), TclValue::Int(0)) => None,
        (TclValue::Int(x), TclValue::Int(y)) => {
            // Floor division: r and y must have the same sign,
            // otherwise subtract 1 from the truncated quotient.
            // `i64::MIN / -1` overflows a wide — promote to the bignum tier
            // like every other integer overflow.
            match x.checked_div(*y) {
                Some(q) => {
                    let r = x.checked_rem(*y)?;
                    if r != 0 && (r.signum() != y.signum()) {
                        Some(TclValue::Int(q.checked_sub(1)?))
                    } else {
                        Some(TclValue::Int(q))
                    }
                }
                None => Some(TclValue::from_big(
                    num_bigint::BigInt::from(*x).div_floor(&num_bigint::BigInt::from(*y)),
                )),
            }
        }
        // Exact floor division on the bignum tier — the shared tower
        // semantics (`tcl_syntax::number_tower`).
        (TclValue::Big(_) | TclValue::Int(_), TclValue::Big(_) | TclValue::Int(_)) => {
            let (x, y) = (a.to_bigint()?, b.to_bigint()?);
            tcl_syntax::number_tower::int_div(&x, &y).map(TclValue::from_big)
        }
        _ => {
            // A non-zero numerator over a zero divisor is Inf/-Inf, a real
            // (foldable) Tcl value — only 0.0/0.0 is a domain error
            // (tclsh: `expr {5.0/0.0}` -> Inf, `expr {0.0/0.0}` errors).
            // IEEE-754 division naturally produces NaN for exactly that
            // case, so decline on NaN rather than blanket-declining
            // whenever the divisor is zero — the same pattern `tcl_pow`'s
            // float path already uses below.
            let r = a.as_f64() / b.as_f64();
            if r.is_nan() {
                return None;
            }
            Some(TclValue::Float(r))
        }
    }
}

fn tcl_mod(a: &TclValue, b: &TclValue) -> Option<TclValue> {
    match (a, b) {
        (TclValue::Int(_), TclValue::Int(0)) => None,
        (TclValue::Int(x), TclValue::Int(y)) => {
            // Sign follows divisor. `i64::MIN % -1` overflows the checked
            // rem — the true result is 0.
            // `i64::MIN % -1` overflows the checked rem — the true result is 0.
            let r = x.checked_rem(*y).unwrap_or_default();
            if r != 0 && (r.signum() != y.signum()) {
                Some(TclValue::Int(r.checked_add(*y)?))
            } else {
                Some(TclValue::Int(r))
            }
        }
        // Floor modulus on the bignum tier — the shared tower semantics.
        (TclValue::Big(_) | TclValue::Int(_), TclValue::Big(_) | TclValue::Int(_)) => {
            let (x, y) = (a.to_bigint()?, b.to_bigint()?);
            tcl_syntax::number_tower::int_mod(&x, &y).map(TclValue::from_big)
        }
        _ => None, // Tcl 9.0 rejects floats for `%`.
    }
}

fn tcl_pow(a: &TclValue, b: &TclValue) -> Option<TclValue> {
    if matches!(a, TclValue::Float(_)) || matches!(b, TclValue::Float(_)) {
        let fa = a.as_f64();
        let fb = b.as_f64();
        if fa == 0.0 && fb < 0.0 {
            return None;
        }
        if fa < 0.0 && (!fb.is_finite() || fb.fract() != 0.0) {
            return None;
        }
        let r = fa.powf(fb);
        if r.is_nan() {
            return None;
        }
        return Some(TclValue::Float(r));
    }
    // Integer path — the shared tower semantics (`INST_EXPON`'s collapses,
    // the 2^28 exponent ceiling, exact bignum powers). The exponent must be
    // a wide: a beyond-wide exponent with |base| > 1 is C's "exponent too
    // large" runtime error, and with a negative bignum exponent the result
    // collapses to 0 (base magnitude ≥ 2 by bignum canonicality).
    let y = match b {
        TclValue::Int(y) => *y,
        TclValue::Big(yy) => {
            use num_traits::{One, Signed, Zero};
            // The 0 / ±1 base collapses hold for ANY exponent magnitude and
            // precede the negative-exponent rule: `0 ** -big` is C's
            // domain error (decline the fold so it surfaces), `1 ** ±big`
            // is 1, `(-1) ** ±big` is ±1 by exponent parity. Only then
            // does |base| ≥ 2 collapse a negative exponent to 0.
            let xb = a.to_bigint()?;
            if xb.is_zero() {
                return None;
            }
            if xb.is_one() {
                return Some(TclValue::Int(1));
            }
            if (-&xb).is_one() {
                let even = (yy % 2u8).is_zero();
                return Some(TclValue::Int(if even { 1 } else { -1 }));
            }
            return if yy.is_negative() {
                Some(TclValue::Int(0))
            } else {
                None
            };
        }
        TclValue::Float(_) => return None,
    };
    let x = a.to_bigint()?;
    tcl_syntax::number_tower::int_pow(&x, y).map(TclValue::from_big)
}

// Unary operators

// iRules string ops

/// Split a Tcl list string into its decoded element values.
///
/// A thin wrapper over the shared list grammar
/// [`tcl_syntax::list::split_list_lenient`]: membership (`in` / `ni`) compares
/// against decoded element values, and element *counts* (`llength` folds) need
/// the full grammar so an escaped separator like `a\ b` counts as one element.
/// Tolerant of a malformed tail so folding a partial list still yields a
/// best-effort element list rather than nothing.
pub(crate) fn split_tcl_list(text: &str) -> Vec<String> {
    // dialect-drift-ok: the *tolerant* element split, whose Jim counterpart
    // `WordValueRules` does not expose (it owns strict-vs-Jim, not
    // strict-vs-tolerant); the four call sites outside this lane are the
    // analyser's. Tracked for the `WordValueRules` owner.
    tcl_syntax::list::split_list_lenient(text)
        .into_iter()
        .map(std::borrow::Cow::into_owned)
        .collect()
}

/// Apply an iRules string operator to two rendered string operands.
///
/// Reached only through [`FoldOps::binary_other`], which the shared
/// [`tcl_syntax::expr::eval`] tree-walk calls for the operators its own
/// `match` does not handle.  That match already covers `equals`
/// ([`BinOp::StrEquals`], via `compare_string` alongside `eq`) and `in` /
/// `ni` (via `in_list`), so those three never arrive here — the arms that
/// re-implemented them were unreachable, and re-implementing `in`/`ni` with
/// a private list split risked disagreeing with the shared `in_list`
/// semantics if either drifted.
fn apply_irules_string_op(op: BinOp, left: &str, right: &str) -> Option<TclValue> {
    let res = match op {
        BinOp::Contains => left.contains(right),
        BinOp::StartsWith => left.starts_with(right),
        BinOp::EndsWith => left.ends_with(right),
        BinOp::MatchesGlob => tcl_syntax::glob::string_match(right, left),
        // `matches_regex` is deliberately *not* constant-folded (along
        // with any other / unsupported operator): the Rust `regex` crate
        // is not Tcl's ARE engine — classes, anchors, word boundaries,
        // embedded options and greediness all differ — so folding here
        // could disagree with runtime.  Decline to evaluate and defer to
        // the runtime regex engine.
        //
        // The bare `matches` ([`BinOp::Matches`]) declines through the
        // same arm, for a different reason: only its *presence* is
        // measured (`docs/design/bigip-irule-parser-measurements.md` §4a
        // `e_matches`), and the probe — `expr {"abc" matches "abc"}` — is
        // an exact-equality case that discriminates none of the
        // string-match readings.  The VM answers it as a string equality
        // so the measured cell reproduces; folding it here would bake an
        // unmeasured semantics into a rewrite, which §12's outstanding
        // re-probe has not yet earned.
        _ => return None,
    };
    Some(TclValue::Int(i64::from(res)))
}

// Tests

#[cfg(test)]
mod tests {
    /// Adversarial-review regressions (tclsh 8.6/9.0 verified): exact
    /// bignum↔double comparison folds, the `**` base collapses for bignum
    /// exponents, and NaN branch conditions declining.
    #[test]
    fn adversarial_fold_regressions() {
        // B7: C compares a bignum and a double EXACTLY — never through the
        // bignum's rounded double view.
        assert_eq!(
            eval_str("18446744073709551617 == 1.8446744073709552e19"),
            Some(TclValue::Int(0)),
            "exact compare: 2^64+1 != its rounded double"
        );
        assert_eq!(
            eval_str("18446744073709551617 > 1.8446744073709552e19"),
            Some(TclValue::Int(1))
        );
        assert_eq!(eval_str("10**308 == 1e308"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("(2**1024) == inf"), Some(TclValue::Int(0)));
        // B8: 0/±1 base collapses precede the negative-bignum-exponent rule.
        assert_eq!(
            eval_str("0**(-(2**64))"),
            None,
            "0 ** -big is a runtime domain error — never fold"
        );
        assert_eq!(eval_str("1**(-(2**64))"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("(-1)**(-(2**64))"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("(-1)**(-(2**64)-1)"), Some(TclValue::Int(-1)));
    }

    use super::*;
    use crate::expr_parser::parse_expr;

    fn eval_str(expr: &str) -> Option<TclValue> {
        let env = Env::new();
        eval_tcl_expr(&parse_expr(expr, None), &env)
    }

    fn eval_str_env(expr: &str, env: &Env) -> Option<TclValue> {
        eval_tcl_expr(&parse_expr(expr, None), env)
    }

    /// Parse + evaluate using the iRules dialect, which enables
    /// `contains`/`starts_with`/`ends_with`/`equals`/`matches_glob`/
    /// `matches_regex`/`in`/`ni` word operators. Must use the
    /// dialect-threading evaluator, not the bare [`eval_tcl_expr`] — the
    /// word operators parse under any dialect gate, but only actually
    /// *fold* when [`FoldOps::is_irules`] is set (issue #983/#985's
    /// defence-in-depth fix), which only [`eval_tcl_expr_in_dialect`] does.
    fn eval_irules(expr: &str) -> Option<TclValue> {
        let env = Env::new();
        eval_tcl_expr_in_dialect(
            &parse_expr(expr, Some("f5-irules")),
            &env,
            tcl_dialect::DialectProfile::irules(),
        )
    }

    fn eval_irules_env(expr: &str, env: &Env) -> Option<TclValue> {
        eval_tcl_expr_in_dialect(
            &parse_expr(expr, Some("f5-irules")),
            env,
            tcl_dialect::DialectProfile::irules(),
        )
    }

    #[test]
    fn math_functions_fold_only_from_their_introducing_release() {
        let env = Env::new();
        let fold = |expr: &str, dialect: &str| {
            eval_tcl_expr_in_dialect(
                &parse_expr(expr, Some(dialect)),
                &env,
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
        };
        // `min`/`max` are 8.5+: fold from 8.5, decline under 8.4.
        assert_eq!(fold("min(3, 1, 2)", "tcl8.6"), Some(TclValue::Int(1)));
        assert_eq!(fold("min(3, 1, 2)", "tcl8.4"), None);
        // The `is*` classification family is 9.0+.
        assert_eq!(fold("isinf(1.0)", "tcl9.0"), Some(TclValue::Int(0)));
        assert_eq!(fold("isinf(1.0)", "tcl8.6"), None);
        // An 8.4-era function folds everywhere.
        assert_eq!(fold("abs(-5)", "tcl8.4"), Some(TclValue::Int(5)));
    }

    #[test]
    fn literal_int() {
        assert_eq!(eval_str("42"), Some(TclValue::Int(42)));
        assert_eq!(eval_str("0"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("0x1a"), Some(TclValue::Int(26)));
        assert_eq!(eval_str("0b101"), Some(TclValue::Int(5)));
        assert_eq!(eval_str("0o17"), Some(TclValue::Int(15)));
    }

    #[test]
    fn literal_float() {
        assert_eq!(eval_str("1.5"), Some(TclValue::Float(1.5)));
        assert_eq!(eval_str("2e3"), Some(TclValue::Float(2000.0)));
    }

    #[test]
    fn const_fold_integers_match_tclsh() {
        // The const-folder drives O101 expr-folding, so a divergence from C Tcl
        // is a miscompile. Each (expr, value) pair is verified against
        // tclsh8.6/9.0 via `expr {<expr>}`:
        //   0x1a=26 0b101=5 0o17=15 7/2=3 7%3=1 2**10=1024 3*4+5=17
        //   (3+4)*2=14 10-3-2=5 5>3=1 5==5=1 5!=5=0 abs(-7)=7 max(3,9)=9
        //   min(3,9)=3 (1<<4)=16 (255&15)=15 (5|2)=7 (6^3)=5
        for (expr, want) in [
            ("0x1a", 26),
            ("0b101", 5),
            ("0o17", 15),
            ("7/2", 3),
            ("7%3", 1),
            ("2**10", 1024),
            ("3*4+5", 17),
            ("(3+4)*2", 14),
            ("10-3-2", 5),
            ("5>3", 1),
            ("5==5", 1),
            ("5!=5", 0),
            ("abs(-7)", 7),
            ("max(3,9)", 9),
            ("min(3,9)", 3),
            ("1<<4", 16),
            ("255&15", 15),
            ("5|2", 7),
            ("6^3", 5),
        ] {
            assert_eq!(eval_str(expr), Some(TclValue::Int(want)), "expr {{{expr}}}");
        }
    }

    #[test]
    fn const_fold_floats_and_logicals_match_tclsh() {
        // tclsh-verified floats: 1.5+2.5=4.0, 10.0/4=2.5, 3.0*2=6.0.
        assert_eq!(eval_str("1.5+2.5"), Some(TclValue::Float(4.0)));
        assert_eq!(eval_str("10.0/4"), Some(TclValue::Float(2.5)));
        assert_eq!(eval_str("3.0*2"), Some(TclValue::Float(6.0)));
        // tclsh-verified comparisons / logicals → 0/1 integers:
        //   2.5>1.5=1, "abc" eq "abc"=1, "abc" eq "abd"=0, "a" ne "b"=1,
        //   1&&0=0, 1||0=1, !0=1.
        for (expr, want) in [
            ("2.5>1.5", 1),
            (r#""abc" eq "abc""#, 1),
            (r#""abc" eq "abd""#, 0),
            (r#""a" ne "b""#, 1),
            ("1 && 0", 0),
            ("1 || 0", 1),
            ("!0", 1),
        ] {
            assert_eq!(eval_str(expr), Some(TclValue::Int(want)), "expr {{{expr}}}");
        }
    }

    #[test]
    fn const_fold_integer_division_floors_toward_negative_infinity() {
        // tclsh: integer `/` and `%` floor toward -inf (not truncate):
        //   -7/2 = -4 (not -3); -7%2 = 1; 7/-2 = -4; 7%-2 = -1.
        assert_eq!(eval_str("-7/2"), Some(TclValue::Int(-4)));
        assert_eq!(eval_str("-7%2"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("7/-2"), Some(TclValue::Int(-4)));
        assert_eq!(eval_str("7%-2"), Some(TclValue::Int(-1)));
    }

    #[test]
    fn polymorphic_equality_uses_strict_numbers() {
        // `==` / `!=` compare numerically only when *both* operands are valid
        // Tcl numbers; otherwise as strings. Boolean words are NOT numbers for
        // comparison (tclsh: `expr {"true" == "1"}` → 0), so the comparison
        // must fall back to a string compare rather than coercing `true`→1.
        assert_eq!(eval_str(r#""true" == "1""#), Some(TclValue::Int(0)));
        assert_eq!(eval_str(r#""yes" == "1""#), Some(TclValue::Int(0)));
        assert_eq!(eval_str(r#""true" != "1""#), Some(TclValue::Int(1)));
        // Genuine numbers still compare numerically (incl. mixed int/float and
        // hex), and non-numeric strings string-compare.
        assert_eq!(eval_str(r#""5" == "5.0""#), Some(TclValue::Int(1)));
        assert_eq!(eval_str(r#""0x10" == "16""#), Some(TclValue::Int(1)));
        assert_eq!(eval_str(r#""foo" == "foo""#), Some(TclValue::Int(1)));
        assert_eq!(eval_str(r#""foo" == "bar""#), Some(TclValue::Int(0)));
    }

    #[test]
    fn dialect_ambiguous_operand_declines_to_fold() {
        // A leading-zero decimal is octal in tcl8.x but decimal in tcl9.0, so
        // its numeric-vs-string comparison is dialect-dependent — the
        // const-folder declines rather than risk a wrong answer (false I230).
        assert_eq!(eval_str(r#""08" == "8""#), None);
        assert_eq!(eval_str(r#""010" == "8""#), None);
        assert_eq!(eval_str(r#""8" != "08""#), None);
        // But unambiguous numbers and non-numbers still fold.
        assert_eq!(eval_str(r#""0" == "0""#), Some(TclValue::Int(1)));
        assert_eq!(eval_str(r#""0x10" == "16""#), Some(TclValue::Int(1)));
        assert_eq!(eval_str(r#""08" == "foo""#), None); // 08 still ambiguous
    }

    #[test]
    fn dialect_aware_leading_zero_folds_per_dialect() {
        let eval_d = |expr: &str, dialect: &str| {
            eval_tcl_expr_in_dialect(
                &parse_expr(expr, None),
                &Env::new(),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
        };
        // tcl8.x octal: `08`/`09` are *invalid* octal → treated as strings, so
        // `"08" == "8"` is a string compare → 0. `010` is valid octal (8), so
        // `"010" == "8"` compares numerically → 1.
        for d in ["tcl8.4", "tcl8.5", "tcl8.6", "f5-irules", "f5-iapps"] {
            assert_eq!(eval_d(r#""08" == "8""#, d), Some(TclValue::Int(0)), "{d}");
            assert_eq!(eval_d(r#""010" == "8""#, d), Some(TclValue::Int(1)), "{d}");
            assert_eq!(eval_d(r#""08" != "8""#, d), Some(TclValue::Int(1)), "{d}");
        }
        // tcl9.0 decimal (TIP 472): `08` → 8, `010` → 10, both numeric compares.
        assert_eq!(eval_d(r#""08" == "8""#, "tcl9.0"), Some(TclValue::Int(1)));
        assert_eq!(eval_d(r#""010" == "8""#, "tcl9.0"), Some(TclValue::Int(0)));
        assert_eq!(eval_d(r#""010" == "10""#, "tcl9.0"), Some(TclValue::Int(1)));
    }

    #[test]
    fn dialect_aware_leading_zero_folds_in_arithmetic() {
        // TP: the octal-vs-decimal split must also apply to arithmetic /
        // unary / math-function operands, not just comparisons. Each value
        // verified against real tclsh8.6: `010 + 1` = 9 (octal 8+1),
        // `010 * 2` = 16, `-010` = -8, `abs(-010)` = 8.
        let eval_d = |expr: &str, dialect: &str| {
            eval_tcl_expr_in_dialect(
                &parse_expr(expr, None),
                &Env::new(),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
        };
        for d in ["tcl8.4", "tcl8.5", "tcl8.6", "f5-irules", "f5-iapps"] {
            assert_eq!(eval_d("010 + 1", d), Some(TclValue::Int(9)), "{d}");
            assert_eq!(eval_d("010 * 2", d), Some(TclValue::Int(16)), "{d}");
            assert_eq!(eval_d("-010", d), Some(TclValue::Int(-8)), "{d}");
            assert_eq!(eval_d("abs(-010)", d), Some(TclValue::Int(8)), "{d}");
        }
        // tcl9.0 decimal (TIP 472): `010 + 1` = 11 (decimal 10+1).
        assert_eq!(eval_d("010 + 1", "tcl9.0"), Some(TclValue::Int(11)));
        assert_eq!(eval_d("-010", "tcl9.0"), Some(TclValue::Int(-10)));
    }

    #[test]
    fn dialect_blind_arithmetic_declines_on_bare_leading_zero() {
        // FN guard (regression for the fix): when the dialect is genuinely
        // unknown (plain `eval_tcl_expr`, no octal info at all), arithmetic
        // on a bare leading-zero literal whose octal and decimal readings
        // *disagree* must decline rather than silently pick one — `010` is
        // octal 8 vs decimal 10.
        assert_eq!(eval_str("010 + 1"), None);
        assert_eq!(eval_str("-010"), None);
        assert_eq!(eval_str("abs(-010)"), None);
        // Unaffected: no leading-zero operand, arithmetic still folds.
        assert_eq!(eval_str("10 + 1"), Some(TclValue::Int(11)));
        // Unaffected: `07` is 7 under both readings (7 is a valid octal
        // digit), so a dialect-blind fold is still sound here.
        assert_eq!(eval_str("07 + 1"), Some(TclValue::Int(8)));
    }

    #[test]
    fn invalid_octal_arithmetic_declines_under_octal_dialect() {
        // TN: `08 + 1` is a genuine Tcl *error* under the 8.x octal rule
        // (`08` is not valid octal — tclsh: "invalid bareword \"08\"") —
        // the const-folder has no error channel, so it must decline (defer
        // to the runtime, which will raise the real error) rather than
        // guess a decimal reading.
        let eval_d = |expr: &str, dialect: &str| {
            eval_tcl_expr_in_dialect(
                &parse_expr(expr, None),
                &Env::new(),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
        };
        assert_eq!(eval_d("08 + 1", "tcl8.6"), None);
        // Decimal dialect reads `08` as 8, so this is fine there.
        assert_eq!(eval_d("08 + 1", "tcl9.0"), Some(TclValue::Int(9)));
    }

    #[test]
    fn bpf_folds_leading_zero_as_decimal_like_its_tcl_9_runtime() {
        let eval_d = |expr: &str, dialect: &str| {
            eval_tcl_expr_in_dialect(
                &parse_expr(expr, None),
                &Env::new(),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
        };
        // bpf embeds Tcl 9.0 (dialect-profile-model.md D7): `010` is decimal
        // 10 — tclsh9.0-verified (`expr {010 + 1}` → 11, `expr {08 + 1}` →
        // 9). The old string-prefix heuristic (!starts_with("tcl9")) wrongly
        // read bpf as octal.
        assert_eq!(eval_d("010 + 1", "bpf"), Some(TclValue::Int(11)));
        assert_eq!(eval_d("08 + 1", "bpf"), Some(TclValue::Int(9)));
        // 8.x runtimes keep the octal rule: tclsh8.6-verified
        // (`expr {010 + 1}` → 9).
        assert_eq!(eval_d("010 + 1", "tcl8.6"), Some(TclValue::Int(9)));
        assert_eq!(eval_d("010 + 1", "f5-irules"), Some(TclValue::Int(9)));
    }

    #[test]
    fn no_runtime_profiles_abstain_from_octal_sensitive_folds() {
        let eval_d = |expr: &str, dialect: &str| {
            eval_tcl_expr_in_dialect(
                &parse_expr(expr, None),
                &Env::new(),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
        };
        // f5-bigip has no Tcl runtime and an unknown dialect resolves to the
        // permissive fallback: leading-zero-sensitive folds abstain (§11.1)
        // rather than guessing a base, while octal-insensitive arithmetic
        // still folds.
        for d in ["f5-bigip", "no-such-dialect"] {
            assert_eq!(eval_d("010 + 1", d), None, "{d}: abstain on 010");
            assert_eq!(eval_d("1 + 1", d), Some(TclValue::Int(2)), "{d}");
        }
    }

    #[test]
    fn leading_zero_octal_policy_follows_the_runtime_base() {
        // 8.x runtimes (plain and vendor) read leading zeros as octal.
        for d in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "f5-irules",
            "f5-iapps",
            "f5-tmsh",
            "expect",
        ] {
            assert_eq!(
                leading_zero_is_octal(
                    tcl_registry::model::ingress::resolve_environment(d).analyser_profile()
                ),
                Some(true),
                "{d}"
            );
        }
        // 9.x runtimes dropped the rule (TIP 114/472) — bpf embeds Tcl 9.0
        // (D7), so `010` is not octal there either.
        for d in ["tcl9.0", "tcl9.1", "bpf"] {
            assert_eq!(
                leading_zero_is_octal(
                    tcl_registry::model::ingress::resolve_environment(d).analyser_profile()
                ),
                Some(false),
                "{d}"
            );
        }
        // No Tcl runtime / unknown dialect: abstain, never guess (§11.1).
        assert_eq!(
            leading_zero_is_octal(
                tcl_registry::model::ingress::resolve_environment("f5-bigip").analyser_profile()
            ),
            None
        );
        assert_eq!(
            leading_zero_is_octal(
                tcl_registry::model::ingress::resolve_environment("no-such-dialect")
                    .analyser_profile()
            ),
            None
        );
    }

    #[test]
    fn literal_bool() {
        assert_eq!(parse_literal("true"), Some(TclValue::Int(1)));
        assert_eq!(parse_literal("yes"), Some(TclValue::Int(1)));
        assert_eq!(parse_literal("false"), Some(TclValue::Int(0)));
        assert_eq!(parse_literal("off"), Some(TclValue::Int(0)));
    }

    #[test]
    fn literal_bool_unique_prefix() {
        // `Tcl_GetBoolean` accepts unique prefixes in a boolean-coercion
        // context (verified against tclsh: `expr {1 && tr}` => 1,
        // `expr {ye && of}` => 0).
        for w in ["t", "tr", "tru", "y", "ye", "on"] {
            assert_eq!(parse_literal(w), Some(TclValue::Int(1)), "{w}");
        }
        for w in ["f", "fa", "n", "no", "of", "off"] {
            assert_eq!(parse_literal(w), Some(TclValue::Int(0)), "{w}");
        }
        // Ambiguous `o` (on/off) is not a boolean.
        assert_eq!(parse_literal("o"), None);
    }

    #[test]
    fn arithmetic_int() {
        assert_eq!(eval_str("1 + 2"), Some(TclValue::Int(3)));
        assert_eq!(eval_str("5 - 3"), Some(TclValue::Int(2)));
        assert_eq!(eval_str("4 * 5"), Some(TclValue::Int(20)));
        assert_eq!(eval_str("10 / 3"), Some(TclValue::Int(3)));
        assert_eq!(eval_str("10 % 3"), Some(TclValue::Int(1)));
    }

    #[test]
    fn arithmetic_int_floor_division_negative() {
        // Tcl: floor toward -inf.
        assert_eq!(eval_str("-7 / 2"), Some(TclValue::Int(-4)));
        assert_eq!(eval_str("7 / -2"), Some(TclValue::Int(-4)));
        // Modulo sign follows divisor.
        assert_eq!(eval_str("-7 % 2"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("7 % -2"), Some(TclValue::Int(-1)));
    }

    #[test]
    fn arithmetic_float_promotion() {
        assert_eq!(eval_str("1 + 2.5"), Some(TclValue::Float(3.5)));
        assert_eq!(eval_str("10.0 / 4"), Some(TclValue::Float(2.5)));
    }

    #[test]
    fn division_by_zero() {
        assert_eq!(eval_str("1 / 0"), None);
        assert_eq!(eval_str("1 % 0"), None);
        // tclsh: `expr {0.0/0.0}` is a domain error (NaN) — declines.
        assert_eq!(eval_str("0.0 / 0.0"), None);
    }

    #[test]
    fn float_division_by_zero_with_nonzero_numerator_folds_to_infinity() {
        // TP (fixes a previous over-conservative decline): tclsh:
        // `expr {1.0/0.0}` -> Inf, `expr {-1.0/0.0}` -> -Inf,
        // `expr {1.0/-0.0}` -> -Inf. A non-zero numerator over a zero
        // divisor is a real, foldable IEEE value, not a domain error —
        // only 0.0/0.0 itself errors.
        assert_eq!(eval_str("1.0 / 0.0"), Some(TclValue::Float(f64::INFINITY)));
        assert_eq!(
            eval_str("-1.0 / 0.0"),
            Some(TclValue::Float(f64::NEG_INFINITY))
        );
        assert_eq!(
            eval_str("1.0 / -0.0"),
            Some(TclValue::Float(f64::NEG_INFINITY))
        );
    }

    #[test]
    fn pow_integer() {
        assert_eq!(eval_str("2 ** 10"), Some(TclValue::Int(1024)));
        assert_eq!(eval_str("2 ** 0"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("0 ** 5"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("(-1) ** 3"), Some(TclValue::Int(-1)));
        assert_eq!(eval_str("(-1) ** 4"), Some(TclValue::Int(1)));
        // |base| > 1 with negative exp → 0 (Tcl integer rules).
        assert_eq!(eval_str("2 ** -5"), Some(TclValue::Int(0)));
        // 0 ** negative → error.
        assert_eq!(eval_str("0 ** -1"), None);
    }

    #[test]
    fn comparisons_return_0_or_1() {
        assert_eq!(eval_str("1 < 2"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("2 < 1"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("3 == 3"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("3 != 3"), Some(TclValue::Int(0)));
    }

    #[test]
    fn mixed_int_double_comparisons_are_exact_past_2_pow_53() {
        // tclsh (8.6 and 9.0): a wide compares against a double at full
        // precision, not through a lossy both-as-f64 conversion.
        //   expr {9007199254740993 == 9007199254740992.0} → 0
        //   expr {9007199254740993 >  9007199254740992.0} → 1
        //   expr {9007199254740993 >  9007199254740993.0} → 1  (the float
        //   literal itself rounds down to …992.0)
        assert_eq!(
            eval_str("9007199254740993 == 9007199254740992.0"),
            Some(TclValue::Int(0))
        );
        assert_eq!(
            eval_str("9007199254740993 > 9007199254740992.0"),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_str("9007199254740993 > 9007199254740993.0"),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_str("20000000000000003 < 20000000000000004.0"),
            Some(TclValue::Int(1))
        );
        // Order flipped: the double on the left.
        assert_eq!(
            eval_str("9007199254740992.0 < 9007199254740993"),
            Some(TclValue::Int(1))
        );
    }

    #[test]
    fn nan_comparisons_follow_tcl_unordered_rule() {
        // tclsh: `set x NaN; expr {$x == 1}` → 0, `!=` → 1, every ordering
        // comparison → 0 — numeric-unordered, NOT a string comparison.
        let mut env = Env::new();
        env.insert("x".to_owned(), EnvValue::Str("NaN".to_owned()));
        assert_eq!(eval_str_env("$x == 1", &env), Some(TclValue::Int(0)));
        assert_eq!(eval_str_env("$x != 1", &env), Some(TclValue::Int(1)));
        assert_eq!(eval_str_env("$x < 1", &env), Some(TclValue::Int(0)));
        assert_eq!(eval_str_env("$x <= 1", &env), Some(TclValue::Int(0)));
        assert_eq!(eval_str_env("$x > 1", &env), Some(TclValue::Int(0)));
        assert_eq!(eval_str_env("$x >= 1", &env), Some(TclValue::Int(0)));
        // NaN != NaN numerically… but `eq` is a string comparison, so the
        // spelling matters there instead (both verified against tclsh).
        assert_eq!(eval_str_env("$x == $x", &env), Some(TclValue::Int(0)));
        assert_eq!(eval_str_env("$x eq $x", &env), Some(TclValue::Int(1)));
    }

    #[test]
    fn wide_vs_2_pow_63_double_declines_to_fold() {
        // `9223372036854775807 < 9223372036854775808.0` is answered
        // platform-dependently by C Tcl (UB in its double→wide cast: x86-64
        // says 0, a saturating conversion says 1, exact maths says 1) — the
        // folder must decline rather than bake in any one answer.
        assert_eq!(
            eval_str("9223372036854775807 < 9223372036854775808.0"),
            None
        );
        assert_eq!(
            eval_str("9223372036854775807 == 9223372036854775808.0"),
            None
        );
        // The negative mirror is well-defined (−2⁶³ is representable): fold.
        assert_eq!(
            eval_str("-9223372036854775807-1 == -9223372036854775808.0"),
            Some(TclValue::Int(1))
        );
        // And a double clearly past the boundary is well-defined: 2⁶³−1 < 9.3e18.
        assert_eq!(
            eval_str("9223372036854775807 < 9.3e18"),
            Some(TclValue::Int(1))
        );
    }

    #[test]
    fn string_comparisons() {
        assert_eq!(eval_str("1 eq 1"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("1 ne 2"), Some(TclValue::Int(1)));
    }

    #[test]
    fn string_comparisons_fold_string_operands() {
        // `eq`/`ne`/`lt`/`gt`/`le`/`ge` compare
        // operands AS strings, so string-only operands fold
        // (an earlier numeric parse returned None for them).
        assert_eq!(eval_str("\"x\" eq \"y\""), Some(TclValue::Int(0)));
        assert_eq!(eval_str("\"x\" ne \"y\""), Some(TclValue::Int(1)));
        assert_eq!(eval_str("\"x\" eq \"x\""), Some(TclValue::Int(1)));
        // Lexical ordering on string operands.
        assert_eq!(eval_str("\"abc\" lt \"abd\""), Some(TclValue::Int(1)));
        assert_eq!(eval_str("\"b\" gt \"a\""), Some(TclValue::Int(1)));
        // `5 eq 5.0` compares the strings "5" vs "5.0" (→ 0), matching
        // C Tcl 9 — a regression guard for the numeric-looking case.
        assert_eq!(eval_str("5 eq 5.0"), Some(TclValue::Int(0)));
    }

    #[test]
    fn short_circuit_and() {
        // Second operand is an unbound variable — short-circuit must
        // avoid evaluating it when the first operand is falsy.
        let env = Env::new();
        assert_eq!(eval_str_env("0 && $undef", &env), Some(TclValue::Int(0)));
    }

    #[test]
    fn short_circuit_or() {
        let env = Env::new();
        assert_eq!(eval_str_env("1 || $undef", &env), Some(TclValue::Int(1)));
    }

    #[test]
    fn ternary_selects_correct_branch() {
        assert_eq!(eval_str("1 ? 10 : 20"), Some(TclValue::Int(10)));
        assert_eq!(eval_str("0 ? 10 : 20"), Some(TclValue::Int(20)));
    }

    #[test]
    fn unary_operators() {
        assert_eq!(eval_str("-5"), Some(TclValue::Int(-5)));
        assert_eq!(eval_str("+5"), Some(TclValue::Int(5)));
        assert_eq!(eval_str("!0"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("!1"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("~0"), Some(TclValue::Int(-1)));
    }

    #[test]
    fn bitwise_operators() {
        assert_eq!(eval_str("0xff & 0x0f"), Some(TclValue::Int(0x0f)));
        assert_eq!(eval_str("0xf0 | 0x0f"), Some(TclValue::Int(0xff)));
        assert_eq!(eval_str("0xff ^ 0x0f"), Some(TclValue::Int(0xf0)));
    }

    #[test]
    fn shifts() {
        assert_eq!(eval_str("1 << 4"), Some(TclValue::Int(16)));
        assert_eq!(eval_str("16 >> 2"), Some(TclValue::Int(4)));
        // Negative shift count is undefined.
        assert_eq!(eval_str("1 << -1"), None);
    }

    #[test]
    fn right_shift_at_and_past_width() {
        // `x >> 64` must not execute a 64-bit shift (which
        // panics in debug / masks to `x >> 0` in release). At >= 64 the
        // result is the replicated sign bit.
        assert_eq!(eval_str("5 >> 64"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("5 >> 65"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("-5 >> 64"), Some(TclValue::Int(-1)));
        assert_eq!(eval_str("-5 >> 100"), Some(TclValue::Int(-1)));
        // Just below the boundary still shifts normally.
        assert_eq!(eval_str("-1 >> 63"), Some(TclValue::Int(-1)));
    }

    #[test]
    fn variable_resolution_from_env() {
        let mut env = Env::new();
        env.insert("x".into(), EnvValue::Int(42));
        assert_eq!(eval_str_env("$x + 8", &env), Some(TclValue::Int(50)));
    }

    #[test]
    fn unbound_variable_is_none() {
        let env = Env::new();
        assert_eq!(eval_str_env("$undef + 1", &env), None);
    }

    #[test]
    fn command_substitution_is_none() {
        // Raw text `[foo]` is parsed as an ExprCommand, which is opaque.
        assert_eq!(eval_str("[clock seconds] + 1"), None);
    }

    #[test]
    fn format_tcl_value_int_and_float() {
        assert_eq!(format_tcl_value(&TclValue::Int(42)), "42");
        assert_eq!(format_tcl_value(&TclValue::Int(-7)), "-7");
        assert_eq!(format_tcl_value(&TclValue::Float(1.5)), "1.5");
        // Integer-valued floats render with trailing .0.
        assert_eq!(format_tcl_value(&TclValue::Float(3.0)), "3.0");
    }

    #[test]
    fn overflow_promotes_to_exact_bignum() {
        // 10 ** 100 overflows a wide: C Tcl promotes to a bignum and so does
        // the folder (P4, type-tracking.md) — exactly, never wrapped.
        let want = format!("1{}", "0".repeat(100));
        assert_eq!(
            eval_str("10 ** 100").map(|v| format_tcl_value(&v)),
            Some(want)
        );
    }

    // -- matches_regex is never constant-folded --

    #[test]
    fn irules_matches_regex_is_not_folded() {
        // `matches_regex` is deferred to the runtime ARE engine rather
        // than folded via the Rust `regex` crate (whose syntax/semantics
        // differ from Tcl ARE), so the constant evaluator returns None
        // for every pattern — even ones the Rust engine could match.
        for expr in [
            r#""hello world" matches_regex "world""#,
            r#""hello" matches_regex "^bye""#,
            r#""abc123" matches_regex "^[a-z]+[0-9]+$""#,
            r#""apple" matches_regex "apple|orange""#,
            r#""prefix-world-suffix" matches_regex "world""#,
            r#""abc" matches_regex "(?=a)abc""#,
            r#""abc" matches_regex "[unterminated""#,
        ] {
            assert_eq!(eval_irules(expr), None, "{expr}");
        }
    }

    // -- simple iRules string ops --

    /// #983/#985 residual: `FoldOps::binary_other` must only fold the iRules
    /// word operators under an iRules dialect, and decline (not panic, not
    /// silently misfold) everywhere else — the defence-in-depth check for
    /// call sites that reach this evaluator without a dialect string
    /// (`eval_tcl_expr`/`eval_tcl_expr_with_octal`).
    #[test]
    fn irules_contains_folds_under_irules_and_declines_under_plain_tcl() {
        let node_irules = parse_expr(r#""abc" contains "b""#, Some("f5-irules"));
        let env = Env::new();

        // Folds to true under the iRules dialect.
        assert_eq!(
            eval_tcl_expr_in_dialect(&node_irules, &env, tcl_dialect::DialectProfile::irules()),
            Some(TclValue::Int(1))
        );

        // The bare, dialect-less entry points decline rather than assume
        // plain Tcl — no fold, no crash.
        assert_eq!(eval_tcl_expr(&node_irules, &env), None);
        assert_eq!(eval_tcl_expr_with_octal(&node_irules, &env, None), None);

        // And explicitly asking for a plain-Tcl dialect also declines.
        assert_eq!(
            eval_tcl_expr_in_dialect(
                &node_irules,
                &env,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile()
            ),
            None
        );
    }

    #[test]
    fn irules_contains() {
        assert_eq!(
            eval_irules(r#""hello world" contains "world""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""hello" contains "bye""#),
            Some(TclValue::Int(0))
        );
    }

    #[test]
    fn irules_starts_with() {
        assert_eq!(
            eval_irules(r#""foobar" starts_with "foo""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""foobar" starts_with "bar""#),
            Some(TclValue::Int(0))
        );
    }

    #[test]
    fn irules_ends_with() {
        assert_eq!(
            eval_irules(r#""foobar" ends_with "bar""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""foobar" ends_with "foo""#),
            Some(TclValue::Int(0))
        );
    }

    #[test]
    fn irules_str_equals() {
        assert_eq!(eval_irules(r#""abc" equals "abc""#), Some(TclValue::Int(1)));
        assert_eq!(eval_irules(r#""abc" equals "xyz""#), Some(TclValue::Int(0)));
    }

    #[test]
    fn irules_string_op_with_bound_variable() {
        let mut env = Env::new();
        env.insert("name".into(), EnvValue::Str("production".into()));
        assert_eq!(
            eval_irules_env(r#"$name contains "prod""#, &env),
            Some(TclValue::Int(1))
        );
    }

    // -- matches_glob + in/ni --

    #[test]
    fn irules_matches_glob_star() {
        assert_eq!(
            eval_irules(r#""hello world" matches_glob "hello*""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""hello world" matches_glob "*world""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""hello world" matches_glob "*lo w*""#),
            Some(TclValue::Int(1))
        );
    }

    #[test]
    fn irules_matches_glob_question_and_class() {
        assert_eq!(
            eval_irules(r#""abc" matches_glob "a?c""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""abc" matches_glob "a[bxy]c""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""axc" matches_glob "a[bxy]c""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""azc" matches_glob "a[bxy]c""#),
            Some(TclValue::Int(0))
        );
    }

    #[test]
    fn irules_matches_glob_rejects_on_mismatch() {
        assert_eq!(
            eval_irules(r#""hello" matches_glob "world""#),
            Some(TclValue::Int(0))
        );
    }

    #[test]
    fn irules_in_list_membership() {
        assert_eq!(eval_irules(r#""b" in "a b c""#), Some(TclValue::Int(1)));
        assert_eq!(eval_irules(r#""d" in "a b c""#), Some(TclValue::Int(0)));
        // Braced element grouping.
        assert_eq!(
            eval_irules(r#""b c" in "{a b} {b c} d""#),
            Some(TclValue::Int(1))
        );
    }

    #[test]
    fn irules_ni_negated_membership() {
        assert_eq!(eval_irules(r#""d" ni "a b c""#), Some(TclValue::Int(1)));
        assert_eq!(eval_irules(r#""b" ni "a b c""#), Some(TclValue::Int(0)));
    }

    #[test]
    fn split_tcl_list_handles_braces_and_quotes() {
        assert_eq!(split_tcl_list("a b c"), vec!["a", "b", "c"]);
        assert_eq!(
            split_tcl_list("{hello world} foo"),
            vec!["hello world", "foo"]
        );
        assert_eq!(split_tcl_list(""), Vec::<String>::new());
    }

    #[test]
    fn matches_glob_folds_through_shared_string_match() {
        // `matches_glob` (iRules dialect) now folds through the shared
        // `tcl_syntax::glob` (one `string match` dialect); see that module for
        // the full grammar.
        assert_eq!(
            eval_irules(r#""abc" matches_glob "a*c""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""ab" matches_glob "a*c""#),
            Some(TclValue::Int(0))
        );
        assert_eq!(
            eval_irules(r#""abc" matches_glob "a?c""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""abc" matches_glob "a?d""#),
            Some(TclValue::Int(0))
        );
    }

    // (string-delimiter stripping now lives in the shared `tcl_syntax::expr`
    // walk — `strip_delims` — and is exercised by its tests.)

    // -- Math function dispatch --

    #[test]
    fn math_abs_int_and_float() {
        assert_eq!(eval_str("abs(-5)"), Some(TclValue::Int(5)));
        assert_eq!(eval_str("abs(-1.5)"), Some(TclValue::Float(1.5)));
    }

    #[test]
    fn lshift_overflowing_a_wide_promotes_exactly() {
        // → P4: `1 << 63` overflows a wide; Tcl promotes to
        // the bignum 9223372036854775808 and the folder now computes it
        // exactly (never the wrapped `i64::MIN`).
        assert_eq!(
            eval_str("1 << 63").map(|v| format_tcl_value(&v)),
            Some("9223372036854775808".to_owned())
        );
        assert_eq!(
            eval_str("1 << 64").map(|v| format_tcl_value(&v)),
            Some("18446744073709551616".to_owned())
        );
        assert_eq!(
            eval_str("2 << 62").map(|v| format_tcl_value(&v)),
            Some("9223372036854775808".to_owned())
        );
        // (TN / FP-guard) In-range shifts still fold to the exact wide.
        assert_eq!(eval_str("1 << 62"), Some(TclValue::Int(1i64 << 62)));
        assert_eq!(eval_str("1 << 0"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("3 << 4"), Some(TclValue::Int(48)));
        // `0 << y` never overflows, for any non-negative count.
        assert_eq!(eval_str("0 << 63"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("0 << 100"), Some(TclValue::Int(0)));
        // A negative-magnitude shift that stays in range folds normally.
        assert_eq!(eval_str("-1 << 4"), Some(TclValue::Int(-16)));
        // A count past the smallness cap declines — a folded literal of
        // thousands of digits helps nobody.
        assert_eq!(eval_str("1 << 100000"), None);
    }

    /// NaN in a boolean context is a C Tcl domain error ("floating point
    /// value is Not a Number"), so the folder declines — it must never pick
    /// a truth value (tclsh-verified; `Inf` IS truthy).
    #[test]
    fn nan_in_boolean_context_declines() {
        assert_eq!(eval_str("NaN ? 1 : 0"), None);
        assert_eq!(eval_str("!NaN"), None);
        assert_eq!(eval_str("NaN && 1"), None);
        assert_eq!(eval_str("Inf ? 1 : 0"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("!Inf"), Some(TclValue::Int(0)));
    }

    /// The float-edge oracle table (tclsh 8.6.14; 9.0 agrees). Errors fold
    /// to `None`; values fold to C's exact canonical text:
    ///
    /// ```text
    /// NaN + 1      => error (non-numeric operand)      -NaN / +NaN => error
    /// NaN == NaN   => 0     NaN != 1.0 => 1   NaN < 1 => 0   (IEEE unordered,
    ///                                                          NOT an error)
    /// Inf - Inf    => error (domain)   Inf * 0 => error (domain)
    /// Inf + 1      => Inf   -Inf * -1 => Inf   Inf == Inf => 1
    /// 5.0 / 0      => Inf   -5.0 / 0 => -Inf   0.0/0.0 => error
    /// 1e309        => Inf   4.9e-324 => 5e-324 (denormal round-trip)
    /// -0.0         => -0.0  0.0 == -0.0 => 1   -0.0 + 0.0 => 0.0
    /// isqrt(4611686018427387903) => 2147483647 (exact, not the f64 2^31)
    /// ```
    #[test]
    fn float_edge_oracle_table() {
        let fold = |e: &str| eval_str(e).map(|v| format_tcl_value(&v));
        // NaN in arithmetic / unary: operand errors — decline.
        assert_eq!(eval_str("NaN + 1"), None);
        assert_eq!(eval_str("-NaN"), None);
        assert_eq!(eval_str("+NaN"), None);
        // NaN in comparisons: IEEE unordered values, NOT errors.
        assert_eq!(eval_str("NaN == NaN"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("NaN != 1.0"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("NaN < 1"), Some(TclValue::Int(0)));
        // NaN results are domain errors — decline.
        assert_eq!(eval_str("Inf - Inf"), None);
        assert_eq!(eval_str("Inf * 0"), None);
        // Inf propagates as a real value.
        assert_eq!(fold("Inf + 1"), Some("Inf".into()));
        assert_eq!(fold("-Inf * -1"), Some("Inf".into()));
        assert_eq!(eval_str("Inf == Inf"), Some(TclValue::Int(1)));
        assert_eq!(
            eval_str("Inf > 9223372036854775807"),
            Some(TclValue::Int(1))
        );
        // Float division by (any) zero: ±Inf; 0.0/0.0 is the domain error.
        assert_eq!(fold("5.0 / 0"), Some("Inf".into()));
        assert_eq!(fold("-5.0 / 0"), Some("-Inf".into()));
        assert_eq!(eval_str("0.0 / 0.0"), None);
        // Overflowing literals parse to Inf; denormals round-trip.
        assert_eq!(fold("1e309"), Some("Inf".into()));
        assert_eq!(fold("4.9e-324"), Some("5e-324".into()));
        assert_eq!(fold("1e-330"), Some("0.0".into()));
        // Signed zero.
        assert_eq!(fold("-0.0"), Some("-0.0".into()));
        assert_eq!(eval_str("0.0 == -0.0"), Some(TclValue::Int(1)));
        assert_eq!(fold("-0.0 + 0.0"), Some("0.0".into()));
        // Exact integer square root at the f64-rounding edge.
        assert_eq!(
            eval_str("isqrt(4611686018427387903)"),
            Some(TclValue::Int(2_147_483_647))
        );
        assert_eq!(
            eval_str("isqrt(9223372036854775806)"),
            Some(TclValue::Int(3_037_000_499))
        );
        // int()/round() beyond a wide: int() is dialect-divergent (8.6 wraps
        // mod 2^64, 9.0 is exact) — decline; int(Inf)/int(NaN) are errors.
        assert_eq!(eval_str("int(1e300)"), None);
        assert_eq!(eval_str("int(Inf)"), None);
        assert_eq!(eval_str("int(NaN)"), None);
    }

    /// The P4 oracle corpus (tclsh 8.6.14/9.0-verified values from
    /// type-tracking.md): exact integer arithmetic at and beyond the wide
    /// boundary, floor div/mod, double contamination, and bignum demotion.
    #[test]
    fn bignum_oracle_corpus() {
        let fold = |e: &str| eval_str(e).map(|v| format_tcl_value(&v));
        // Exact integer arithmetic at 2^53 (f64 would merge these).
        assert_eq!(
            fold("9007199254740992 + 1"),
            Some("9007199254740993".into())
        );
        // One double operand contaminates — with f64's genuine rounding.
        assert_eq!(
            fold("9007199254740992 + 1.0"),
            Some("9007199254740992.0".into())
        );
        // Wide → bignum promotion by result magnitude.
        assert_eq!(fold("2 ** 64"), Some("18446744073709551616".into()));
        assert_eq!(
            fold("9223372036854775807 + 1"),
            Some("9223372036854775808".into())
        );
        assert_eq!(
            fold("9223372036854775807 * 2"),
            Some("18446744073709551614".into())
        );
        // Bignum arithmetic is exact, and demotes back to a wide when the
        // result fits.
        assert_eq!(
            fold("18446744073709551616 + 1"),
            Some("18446744073709551617".into())
        );
        assert_eq!(
            eval_str("18446744073709551616 - 18446744073709551616"),
            Some(TclValue::Int(0))
        );
        // Floor division / modulus, both tiers.
        assert_eq!(eval_str("7 / 2"), Some(TclValue::Int(3)));
        assert_eq!(eval_str("-7 / 2"), Some(TclValue::Int(-4)));
        assert_eq!(eval_str("-7 % 2"), Some(TclValue::Int(1)));
        assert_eq!(
            fold("-18446744073709551616 / 3"),
            Some("-6148914691236517206".into())
        );
        assert_eq!(eval_str("18446744073709551617 % 2"), Some(TclValue::Int(1)));
        // Negation off the wide edge promotes; double negation demotes back.
        assert_eq!(
            fold("0 - (-9223372036854775807 - 1)"),
            Some("9223372036854775808".into())
        );
        // Bignum comparisons are exact (distinct beyond-2^53 values).
        assert_eq!(
            eval_str("18446744073709551617 > 18446744073709551616"),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_str("9007199254740993 == 9007199254740992"),
            Some(TclValue::Int(0))
        );
        // Bitwise on the bignum tier (two's-complement).
        assert_eq!(
            fold("18446744073709551616 | 1"),
            Some("18446744073709551617".into())
        );
        assert_eq!(
            fold("~18446744073709551616"),
            Some("-18446744073709551617".into())
        );
    }

    #[test]
    fn arith_rejects_boolean_words() {
        // Arithmetic/unary/mathfunc reject boolean words the
        // way Tcl's numeric context does — folding them would replace an error
        // with a value. All of these are Tcl errors, so the folder declines.
        assert_eq!(eval_str("true + 0"), None); // (TP)
        assert_eq!(eval_str("yes * 2"), None);
        assert_eq!(eval_str("off - 1"), None);
        assert_eq!(eval_str("-true"), None);
        assert_eq!(eval_str("~yes"), None);
        assert_eq!(eval_str("abs(true)"), None);
        assert_eq!(eval_str("int(no)"), None);
        // (TN / FP-guard) Genuine numbers still fold, and the boolean-accepting
        // constructs (`!`, `bool()`) keep working.
        assert_eq!(eval_str("1 + 0"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("abs(-5)"), Some(TclValue::Int(5)));
        assert_eq!(eval_str("!true"), Some(TclValue::Int(0))); // logical not takes a bool
        assert_eq!(eval_str("!0"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("bool(true)"), Some(TclValue::Int(1))); // bool() accepts words
        assert_eq!(eval_str("bool(no)"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("bool(42)"), Some(TclValue::Int(1)));
    }

    #[test]
    fn math_int_conversion_truncates() {
        assert_eq!(eval_str("int(3.7)"), Some(TclValue::Int(3)));
        assert_eq!(eval_str("int(-3.7)"), Some(TclValue::Int(-3)));
        assert_eq!(eval_str("entier(2.9)"), Some(TclValue::Int(2)));
        assert_eq!(eval_str("wide(1)"), Some(TclValue::Int(1)));
    }

    #[test]
    fn math_double_promotes_ints() {
        assert_eq!(eval_str("double(3)"), Some(TclValue::Float(3.0)));
    }

    #[test]
    fn math_bool_normalises_to_01() {
        assert_eq!(eval_str("bool(42)"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("bool(0)"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("bool(0.0)"), Some(TclValue::Int(0)));
    }

    #[test]
    fn math_round_ties_away_from_zero() {
        // Tcl round: 0.5 → 1, -0.5 → -1 (NOT banker's rounding).
        assert_eq!(eval_str("round(0.5)"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("round(-0.5)"), Some(TclValue::Int(-1)));
        assert_eq!(eval_str("round(1.5)"), Some(TclValue::Int(2)));
        assert_eq!(eval_str("round(-1.5)"), Some(TclValue::Int(-2)));
        assert_eq!(eval_str("round(2.5)"), Some(TclValue::Int(3)));
    }

    #[test]
    fn math_ceil_and_floor_return_floats() {
        assert_eq!(eval_str("ceil(1.2)"), Some(TclValue::Float(2.0)));
        assert_eq!(eval_str("floor(1.8)"), Some(TclValue::Float(1.0)));
    }

    #[test]
    fn math_min_max_preserve_int_width() {
        assert_eq!(eval_str("min(3, 1, 2)"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("max(3, 1, 2)"), Some(TclValue::Int(3)));
        // Adversarial-review finding: a mixed int/float call returns the
        // *winning* argument's own value, preserving its type — it does not
        // widen to float just because a float appeared among the operands.
        // `min(1, 2.5)` is `1` (an Int, since 1 wins), not `1.0` (confirmed
        // tclsh8.6/9.0); `max(1, 2.5)` is `2.5` (a Float, since 2.5 wins).
        assert_eq!(eval_str("min(1, 2.5)"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("max(1, 2.5)"), Some(TclValue::Float(2.5)));
    }

    #[test]
    fn math_sqrt_and_pow() {
        assert_eq!(eval_str("sqrt(16)"), Some(TclValue::Float(4.0)));
        assert_eq!(eval_str("pow(2, 10)"), Some(TclValue::Float(1024.0)));
    }

    #[test]
    fn math_sqrt_negative_is_domain_error() {
        assert_eq!(eval_str("sqrt(-1)"), None);
    }

    #[test]
    fn math_log_zero_and_negative_domain_error() {
        assert_eq!(eval_str("log(-1)"), None);
        // log(0) → -inf, treated as success (Tcl returns -inf too).
        let v = eval_str("log(0)");
        assert!(matches!(v, Some(TclValue::Float(f)) if f.is_infinite()));
    }

    #[test]
    fn math_atan2_and_hypot() {
        // atan2(0, 1) = 0, hypot(3, 4) = 5
        assert_eq!(eval_str("atan2(0, 1)"), Some(TclValue::Float(0.0)));
        assert_eq!(eval_str("hypot(3, 4)"), Some(TclValue::Float(5.0)));
    }

    #[test]
    fn math_trig_approx() {
        // sin(0) == 0.
        assert!(matches!(
            eval_str("sin(0)"),
            Some(TclValue::Float(f)) if f == 0.0
        ));
    }

    #[test]
    fn math_classification() {
        assert_eq!(eval_str("isinf(1)"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("isnan(1.0)"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("isfinite(1.0)"), Some(TclValue::Int(1)));
    }

    #[test]
    fn math_isqrt_accepts_float_truncating_first() {
        assert_eq!(eval_str("isqrt(16)"), Some(TclValue::Int(4)));
        assert_eq!(eval_str("isqrt(17)"), Some(TclValue::Int(4)));
        // Adversarial-review finding: a `Float` operand used to fall to
        // `tcl_syntax::expr::mathfunc::dispatch`'s catch-all `None` (treated
        // as a domain error) even though real Tcl accepts one, truncating
        // toward zero first (`expr {isqrt(4.0)}` -> `2`, `isqrt(4.9)` -> `2`
        // same as `isqrt(4)`; confirmed tclsh8.6/9.0) — this const-folder
        // must fold the same value the real interpreter evaluates to, or a
        // W-series "simplify this constant expression" fix would propose a
        // wrong replacement.
        assert_eq!(eval_str("isqrt(4.0)"), Some(TclValue::Int(2)));
        assert_eq!(eval_str("isqrt(4.9)"), Some(TclValue::Int(2)));
        assert_eq!(eval_str("isqrt(-1.0)"), None); // domain error, same as isqrt(-1)
    }

    #[test]
    fn math_rand_and_srand_always_none() {
        // Non-deterministic — callers must not constant-fold.
        assert_eq!(eval_str("rand()"), None);
        assert_eq!(eval_str("srand(42)"), None);
    }

    #[test]
    fn math_unknown_function_is_none() {
        assert_eq!(eval_str("thereisnosuchfn(1)"), None);
    }
}
