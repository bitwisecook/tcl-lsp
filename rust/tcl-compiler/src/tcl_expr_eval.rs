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
//! `equals`) fold via [`tcl_syntax::expr::ExprOps::binary_other`]; an operator
//! the target release's `expr` lacks (`**`, `in`, `ni` before 8.5, the F5
//! core included) folds nothing ([`operators_in_release`]).

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

use std::{cell::RefCell, collections::HashMap};

use tcl_dialect::{NumberSpelling, NumberSyntax, StringCharacterModel};
use tcl_registry::value_transfer::{DeclineReason, ExactValue, NumericValue};
#[cfg(test)]
use tcl_registry::value_transfer::{DomainFact, FactView};

use crate::expr_ast::{BinOp, ExprNode, UnaryOp};
mod native_constant;
#[doc(hidden)]
pub use native_constant::{NativeConstantResult, eval_native_constant};

mod arithmetic;
mod condition_facts;
mod fold_ops;
mod fold_policy;
mod numerals;
mod services;

pub(crate) use arithmetic::split_tcl_list;
#[cfg(test)]
pub(crate) use condition_facts::EdgeFacts;
#[cfg(test)]
use condition_facts::ordering_facts;
pub(crate) use condition_facts::{
    ConditionReading, PlaceFact, condition_edge_facts, condition_read_guards,
};
pub(crate) use fold_ops::FoldValue;
use fold_ops::{FoldPurpose, Tower, make_fold_ops};
pub(crate) use fold_policy::fold_math_ceiling;
pub use fold_policy::{
    FoldPolicy, is_known_mathfunc_in_dialect, leading_zero_is_octal, math_func_ceiling_for_dialect,
    mathfunc_command_wrappers_available_in_dialect,
};
pub use numerals::{
    format_tcl_value, format_tcl_value_with_policy, parse_literal, parse_literal_in,
};
use numerals::{strict_number, strict_number_for_dialect};
pub use services::ExprAnswer;
pub(crate) use services::{ExprServices, evaluate_expression};

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

    fn canonical_clone(&self) -> Self {
        match self {
            Self::Big(value) => Self::from_big(value.clone()),
            _ => self.clone(),
        }
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
    /// Numeric binding without a native object proof.
    Number(TclValue),
    /// Exact string binding, read under the selected numeral grammar.
    Str(ExactValue<String>),
}

impl EnvValue {
    /// A text binding without a numeric reading or representation evidence.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Str(ExactValue::utf8(text))
    }
}

/// Project lattice contents into expression bindings without an object proof.
pub(crate) fn const_to_env_value(c: &crate::analyses::ConstValue) -> EnvValue {
    use crate::analyses::ConstValue;

    match c {
        ConstValue::Number(NumericValue::Int(i)) => EnvValue::Number(TclValue::Int(*i)),
        ConstValue::Number(NumericValue::Float(f)) => EnvValue::Number(TclValue::Float(*f)),
        ConstValue::Number(NumericValue::Bool(b)) => EnvValue::Number(TclValue::Int(i64::from(*b))),
        ConstValue::String(s) => EnvValue::Str(s.clone().into_exact()),
    }
}

/// Variable environment for evaluation.
pub type Env = HashMap<String, EnvValue>;

/// Exact retained numeric object read without conversion at the reached site.
/// The owner supplying this evidence must prove the object's live identity,
/// current native numeric representation and absence of read observers. Value
/// constants, type shapes and a physical variable cell alone are insufficient.
#[derive(Debug, Clone)]
pub enum NativeOperandObjectIdentity {
    /// One actual interpreter-owned runtime object and representation generation.
    Runtime {
        /// Actual interpreter identity.
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
        /// Native object token.
        object: u64,
        /// Native representation generation.
        generation: u64,
    },
    /// Source-produced numeric family at an exact retained physical read.
    /// Distinct cells remain distinct; this supplies no unique object aliasing.
    Source {
        /// Original numeric producer and native preparation.
        producer: std::sync::Arc<crate::native_numeric::SourceNativeNumericObject>,
        /// Exact original read extent.
        read: tcl_source::words::SourceSite,
        /// Every actual physical alternative at this read.
        cells: Vec<crate::place::CellIdentity>,
    },
}

/// Retained object evidence supplied independently of mathematical values.
#[derive(Debug, Clone)]
pub struct RetainedNativeOperandProof {
    /// Actual native grammar/protocol of the object evidence's interpreter.
    pub dialect: tcl_native_model::InvocationDialect,
    /// Actual runtime allocation or retained source producer and physical reads.
    pub identity: NativeOperandObjectIdentity,
    /// Numeric value read from its already native representation.
    pub value: TclValue,
    /// Existing cached bytes, if known without materialising a string.
    pub existing_string: Option<String>,
}

/// Reached operands whose real object state licenses a read without coercion.
pub type NativeOperandProofs = HashMap<String, RetainedNativeOperandProof>;

/// Native object conversion required by a reached expression operation.
/// These are execution obligations, independent of the known mathematical value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCoercionKind {
    /// Arithmetic or native math-function numeric ingress.
    Number,
    /// Boolean ingress for conditions and logical operators.
    Boolean,
    /// Numeric classification for a polymorphic comparison.
    NumericComparison,
    /// List conversion for membership.
    List,
    /// String materialisation from an actual retained operand object.
    String,
    /// C Tcl's final expression result normalization.
    ResultNormalization,
}

/// An unresolved conversion of an actual retained operand object.
/// The site belongs to the original expression AST, before constant substitution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCoercionObligation {
    /// Authored variable reference, preserving bracing and index syntax.
    pub reference: String,
    /// Parser offset in the original expression; absent for an unpositioned lookup.
    pub start: Option<u32>,
    /// Required native conversion.
    pub kind: NativeCoercionKind,
}

/// A native expression result whose object or existing string must be retained.
/// Its numeric interpretation alone cannot establish a numeric result spelling.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeExpressionResultDependency {
    /// Jim returns the selected retained operand object unchanged.
    SelectedOperand {
        /// Original reference identifying the selected read.
        reference: String,
        /// Original parser offset of the selected reference.
        start: Option<u32>,
        /// Exact contents, if already known without materialising the object.
        existing_value: Option<ExactValue<String>>,
    },
    /// The native engine retains string bytes without numeric normalization.
    StringResult {
        /// Exact returned string contents.
        value: ExactValue<String>,
    },
}

/// Analysis value together with execution effects which must remain represented.
/// Knowing this value does not license replacing its source expression.
#[derive(Debug, Clone, PartialEq)]
pub struct FoldEvaluation {
    /// Known numeric result under the selected native semantics.
    pub value: TclValue,
    /// Reached conversions lacking actual live-object representation evidence.
    pub coercions: Vec<NativeCoercionObligation>,
    /// Native object/string result which must not be replaced by `value`.
    pub result_dependency: Option<NativeExpressionResultDependency>,
}

impl FoldEvaluation {
    /// Whether both reached object conversions and native result preservation
    /// are discharged. Dispatch and observer dependencies still require proof.
    #[must_use]
    pub fn native_value_effects_are_proved(&self) -> bool {
        self.coercions.is_empty() && self.result_dependency.is_none()
    }
    /// Whether replacing evaluation erases no unresolved native object conversion.
    /// Native dispatch dependencies and observer proofs remain separate obligations.
    #[must_use]
    pub fn coercions_are_proved(&self) -> bool {
        self.coercions.is_empty()
    }
}

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
    // assume plain Tcl (safe — see `FoldOps::f5_predicates`).
    eval_with_config(
        node,
        env,
        FoldPolicy::from_octal(None).with_intrinsic_math(),
    )
}

/// Like [`eval_tcl_expr`] but resolves the *dialect* so a leading-zero decimal
/// (`08`, `010`) is classified correctly in `==`/`!=`/`<`/… comparisons:
/// octal in Tcl 8.x (`08`/`09` invalid → string; `010` → 8), decimal in
/// Tcl 9.0 (`08` → 8, `010` → 10). All non-9.x dialects (tcl8.4/8.5/8.6,
/// f5-irules ≈ 8.4, f5-iapps ≈ 8.5/8.6, EDA) use the 8.x octal rule.
///
/// A profile supplies numeric grammar, not installed function identity. Reached
/// math calls need the resolved binding API; this adapter declines them.
#[must_use]
pub fn eval_tcl_expr_in_dialect(
    node: &ExprNode,
    env: &Env,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Option<TclValue> {
    eval_with_config(
        node,
        env,
        FoldPolicy::for_profile(leading_zero_is_octal(dialect), Some(dialect)),
    )
}

/// Like [`eval_tcl_expr`] but takes the leading-zero octal policy directly:
/// `Some(true)` = tcl8.x octal rule, `Some(false)` = tcl9.0 decimal rule,
/// `None` = decline to fold dialect-ambiguous leading-zero operands. Callers
/// that hold a `CommandRegistry` rather than a dialect string derive the flag
/// via `CommandRegistry::leading_zero_is_octal`.
///
/// An octal policy supplies no installed math-function evidence. Reached
/// function calls require the separate resolved binding API.
#[must_use]
pub fn eval_tcl_expr_with_octal(
    node: &ExprNode,
    env: &Env,
    octal: Option<bool>,
) -> Option<TclValue> {
    // The caller has resolved the octal policy but not a dialect string, so
    // (as with `eval_tcl_expr`) decline the iRules word-operator fold rather
    // than assume plain Tcl.
    eval_with_config(node, env, FoldPolicy::from_octal(octal))
}

/// Like [`eval_tcl_expr_with_octal`] but for the (more common) optimiser call
/// sites that already have both an `octal` policy and a resolved dialect
/// profile in scope — so, unlike `eval_tcl_expr_with_octal`'s plain
/// `None`-profile callers, these can resolve [`FoldOps::f5_predicates`] precisely
/// instead of defaulting it to declined.  A profile used only for
/// `leading_zero_is_octal` and never to gate the iRules word-operator fold
/// leaves that fold silently off.
#[must_use]
pub fn eval_tcl_expr_with_octal_and_dialect(
    node: &ExprNode,
    env: &Env,
    octal: Option<bool>,
    profile: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<TclValue> {
    eval_tcl_expr_with_policy(node, env, FoldPolicy::for_profile(octal, profile))
}

/// Evaluate `node` under a bundled [`FoldPolicy`] — the entry point for
/// passes that thread the policy rather than the raw octal flag.
#[must_use]
pub fn eval_tcl_expr_with_policy(
    node: &ExprNode,
    env: &Env,
    policy: FoldPolicy,
) -> Option<TclValue> {
    eval_with_config(node, env, policy)
}

/// Evaluate an expression under `env` to an integer: `Some(int)` when the
/// result folds to an integer or an integer-valued float, `None` otherwise.
/// A boolean answers 0 or 1.
#[must_use]
pub fn evaluate_expr_with_constants(expr: &ExprNode, env: &Env, policy: FoldPolicy) -> Option<i64> {
    match eval_tcl_expr_with_policy(expr, env, policy)? {
        TclValue::Int(i) => Some(i),
        // A beyond-wide value is pathological here — decline rather than
        // saturate.
        TclValue::Big(_) => None,
        TclValue::Float(f) => {
            if !f.is_finite() || f.fract() != 0.0 {
                return None;
            }
            // Finite integral float: saturate to the `i64` range.
            Some(saturating_f64_to_i64(f))
        }
    }
}

/// Convert a finite, integer-valued `f64` to `i64`, saturating to
/// `i64::MIN` / `i64::MAX` when the value is out of range.
///
/// Avoids a lossy `as` cast: the value is rendered to its exact integer
/// decimal (`f` is integral by contract) and parsed. Out-of-range
/// magnitudes fail to parse and saturate by sign, as an `f as i64` cast
/// does.
fn saturating_f64_to_i64(f: f64) -> i64 {
    // `+ 0.0` normalises `-0.0` to `0.0` so it renders/parses as `0`, as an
    // `as i64` cast does.
    match format!("{:.0}", f + 0.0).parse::<i64>() {
        Ok(i) => i,
        Err(_) if f.is_sign_negative() => i64::MIN,
        Err(_) => i64::MAX,
    }
}

/// Whether `value` is past the wide tower under the grammar: a beyond-wide
/// integer, an infinity, or a double spelling outside the double range
/// (`1e-400`, `4.9e-324`), as an operand or a result — what a target that
/// does not widen ([`widens_past_a_wide`]) computes differently. tclsh 8.4
/// raises `ARITH OVERFLOW` / `UNDERFLOW` on such a literal where 8.5 onward
/// read the rounded value.
fn past_the_wide_tower(value: &FoldValue, octal: Option<bool>, numbers: NumberSyntax) -> bool {
    match value {
        FoldValue::Number(TclValue::Big(_)) => true,
        FoldValue::Number(TclValue::Float(f)) => f.is_infinite(),
        FoldValue::Number(TclValue::Int(_)) => false,
        FoldValue::RetainedNativeObject { value, .. } => past_the_wide_tower(value, octal, numbers),
        FoldValue::Str(text) => match strict_number_for_dialect(value, octal, numbers) {
            Some(TclValue::Big(_)) => true,
            Some(TclValue::Float(f)) => {
                f.is_infinite() || tcl_syntax::number::is_out_of_range_double(&text.bytes, numbers)
            }
            Some(TclValue::Int(_)) | None => false,
        },
    }
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
    // `ParseBoolean` matches a word untrimmed (`"yes "` is no boolean); only
    // the numeric reading skips surrounding whitespace.
    if let Some(value) = tcl_syntax::boolean::parse_boolean_word(text) {
        return Some(TclValue::Int(i64::from(value)));
    }
    match strict_number_for_dialect(
        &FoldValue::text(text.trim()),
        policy.octal,
        policy.numbers.unwrap_or_default(),
    )? {
        value @ (TclValue::Int(_) | TclValue::Big(_)) => Some(value),
        TclValue::Float(_) => None,
    }
}

fn eval_with_config(node: &ExprNode, env: &Env, policy: FoldPolicy) -> Option<TclValue> {
    eval_with_math_bindings(node, env, policy, None)
}

/// Fold only reached native function calls proved at their exact AST sites.
/// The query runs after the evaluator has reduced the call's operands and is
/// never consulted for a branch skipped by Tcl's lazy expression evaluation.
#[must_use]
pub fn eval_tcl_expr_with_math_bindings(
    node: &ExprNode,
    env: &Env,
    policy: FoldPolicy,
    bindings: &IntrinsicMathQuery<'_>,
) -> Option<TclValue> {
    eval_with_math_bindings(node, env, policy, Some(bindings))
}

/// Resolved native math handler with its already evaluated alias prefix.
/// Construction requires actual dispatch evidence, independently of the name
/// written in the expression and the catalogue's function roster.
pub struct NativeMathFunctionTarget {
    /// Bare stock implementation identity.
    pub function: String,
    /// Frozen prefix values whose existing bytes are known.
    pub prepended: Vec<String>,
}

/// Exact-site query licensing the unchanged native function and argument shape.
pub type IntrinsicMathQuery<'a> = dyn Fn(&str, u32) -> bool + 'a;

/// Exact-site query resolving the actual handler and proved operand prefix.
pub type ResolvedMathQuery<'a> = dyn Fn(&str, u32) -> Option<NativeMathFunctionTarget> + 'a;

/// Evaluate reached calls through their exact native handler and frozen prefix.
/// Unknown target words, handlers, or prefixes decline that reached call only.
#[must_use]
pub fn eval_tcl_expr_with_resolved_math_bindings(
    node: &ExprNode,
    env: &Env,
    policy: FoldPolicy,
    calls: &ResolvedMathQuery<'_>,
) -> Option<TclValue> {
    eval_with_math_queries(node, env, policy, None, Some(calls), None)
}

/// Fold with actual reached operand-object evidence and exact math dispatch.
/// Unknown object identity/representation remains a required runtime coercion;
/// this API never treats an environment constant as a freshly allocated value.
#[must_use]
pub fn eval_tcl_expr_with_proved_operands(
    node: &ExprNode,
    env: &Env,
    policy: FoldPolicy,
    calls: &ResolvedMathQuery<'_>,
    operands: &NativeOperandProofs,
) -> Option<TclValue> {
    eval_with_math_queries(node, env, policy, None, Some(calls), Some(operands))
}

/// Analyse the original expression without erasing reached native conversions.
/// Callers may use the value for semantic facts, but must retain `coercions` at
/// this execution point. Source replacement, branch deletion and constant native
/// emission require discharging those obligations or retaining residual evaluation.
/// Do not pass an AST whose retained references were substituted with literals.
#[must_use]
pub fn analyse_tcl_expr_with_resolved_math_bindings(
    node: &ExprNode,
    env: &Env,
    policy: FoldPolicy,
    calls: &ResolvedMathQuery<'_>,
    operands: Option<&NativeOperandProofs>,
) -> Option<FoldEvaluation> {
    evaluate_with_math_queries(
        node,
        env,
        policy,
        None,
        Some(calls),
        FoldNativeInputs::objects(operands),
        true,
    )
}

/// Analyse using independently accepted integer contents at exact original
/// source reads. The conversion receipt cannot stand in for a numeric primary.
pub(crate) fn analyse_tcl_expr_with_integer_contents(
    node: &ExprNode,
    env: &Env,
    policy: FoldPolicy,
    calls: &ResolvedMathQuery<'_>,
    operands: &NativeOperandProofs,
    integer_contents: &crate::native_numeric::SourceIntegerContentsReads,
) -> Option<FoldEvaluation> {
    evaluate_with_math_queries(
        node,
        env,
        policy,
        None,
        Some(calls),
        FoldNativeInputs {
            operands: Some(operands),
            integer_contents: Some(integer_contents),
        },
        true,
    )
}

#[derive(Clone, Copy)]
struct FoldNativeInputs<'a> {
    operands: Option<&'a NativeOperandProofs>,
    integer_contents: Option<&'a crate::native_numeric::SourceIntegerContentsReads>,
}
impl<'a> FoldNativeInputs<'a> {
    const fn objects(operands: Option<&'a NativeOperandProofs>) -> Self {
        Self {
            operands,
            integer_contents: None,
        }
    }
}

/// Whether every operator in `node` is one the release `policy` names has
/// ([`tcl_registry::expr_surface::validate_operators`]): tclsh 8.4.20
/// rejects `2**3` as a syntax error and `1 in {1 2}` as a bareword, and
/// 8.4 to 8.6 reject `lt`/`le`/`gt`/`ge`, so the program raises and no
/// fold may give it a value. A policy naming no dialect admits them all.
fn operators_in_release(node: &ExprNode, policy: FoldPolicy) -> bool {
    policy
        .dialect
        .is_none_or(|profile| tcl_registry::expr_surface::validate_operators(profile, node).is_ok())
}

fn eval_with_math_bindings(
    node: &ExprNode,
    env: &Env,
    policy: FoldPolicy,
    bindings: Option<&IntrinsicMathQuery<'_>>,
) -> Option<TclValue> {
    eval_with_math_queries(node, env, policy, bindings, None, None)
}

fn eval_with_math_queries(
    node: &ExprNode,
    env: &Env,
    policy: FoldPolicy,
    bindings: Option<&IntrinsicMathQuery<'_>>,
    calls: Option<&ResolvedMathQuery<'_>>,
    operands: Option<&NativeOperandProofs>,
) -> Option<TclValue> {
    evaluate_with_math_queries(
        node,
        env,
        policy,
        bindings,
        calls,
        FoldNativeInputs::objects(operands),
        false,
    )
    .map(|evaluation| evaluation.value)
}

fn evaluate_with_math_queries(
    node: &ExprNode,
    env: &Env,
    policy: FoldPolicy,
    bindings: Option<&IntrinsicMathQuery<'_>>,
    calls: Option<&ResolvedMathQuery<'_>>,
    native_inputs: FoldNativeInputs<'_>,
    analysis: bool,
) -> Option<FoldEvaluation> {
    let mut ops = make_fold_ops(env, policy, bindings, calls, native_inputs, analysis);
    if !operators_in_release(node, policy) {
        return None;
    }
    // The final value must reduce to a number (a bare string like `expr {"x"}`
    // doesn't fold) — `to_number` maps a `Str` result through `parse_literal`.
    let result = tcl_syntax::expr::eval(node, &mut ops).ok()?;
    if ops.declined.is_some() || ops.tower == Tower::Breached || ops.beyond_tower(&result) {
        // A comparison hit a leading-zero operand whose octal-vs-decimal
        // reading is dialect-dependent and the dialect is unknown, or a
        // comparison the platform decides, or an operand or result past a
        // tower the target does not widen to — decline to fold rather than
        // pick one.
        return None;
    }
    let result_dependency = native_result_dependency(&result, ops.arithmetic, ops.numbers);
    if !analysis && result_dependency.is_some() {
        return None;
    }
    let result = if result_dependency.is_some() {
        // This is a numeric analysis projection, not a native conversion or a
        // licence to emit the resulting number as the returned value.
        let value = match &result {
            FoldValue::RetainedNativeObject { value, .. } => value.as_ref(),
            value => value,
        };
        ops.normalize_number(value.to_number(ops.numbers)?)?
    } else {
        ops.number_for(&result, NativeCoercionKind::ResultNormalization)?
    };
    // Tcl 8.4's signed-minimum decimal formatter is not portable across
    // builds. Preserve runtime formatting rather than manufacture a string.
    if ops.arithmetic == tcl_dialect::NativeArithmetic::Tcl84Wide
        && matches!(result, TclValue::Int(i64::MIN))
    {
        return None;
    }
    Some(FoldEvaluation {
        value: result,
        coercions: ops.coercions.into_inner(),
        result_dependency,
    })
}

fn native_result_dependency(
    result: &FoldValue,
    arithmetic: tcl_dialect::NativeArithmetic,
    numbers: NumberSyntax,
) -> Option<NativeExpressionResultDependency> {
    let contents = match result {
        FoldValue::RetainedNativeObject { value, .. } => value.as_ref(),
        value => value,
    };
    // C's final conversion attempts the numeric grammar, not boolean-word
    // coercion. A bare `false` retains its original bytes and native object.
    if arithmetic.normalizes_expression_result() && strict_number(contents, numbers).is_some() {
        return None;
    }
    match result {
        FoldValue::Str(value) => Some(NativeExpressionResultDependency::StringResult {
            value: value.clone(),
        }),
        FoldValue::RetainedNativeObject {
            value,
            proof,
            reference,
            start,
            ..
        } => {
            let existing_value = proof
                .as_ref()
                .and_then(|proof| proof.existing_string.as_deref())
                .map(ExactValue::utf8)
                .or_else(|| {
                    if let FoldValue::Str(value) = value.as_ref() {
                        Some(value.clone())
                    } else {
                        None
                    }
                });
            Some(NativeExpressionResultDependency::SelectedOperand {
                reference: reference.clone(),
                start: *start,
                existing_value,
            })
        }
        FoldValue::Number(_) => None,
    }
}

/// Constant compiler evaluation mode and its retained guest failure.
#[derive(Default)]
struct ConstantCompilation {
    enabled: bool,
    failure: Option<tcl_registry::native_compilation::NativeCompilationFailure>,
}

struct FoldOps<'a> {
    constant_compilation: ConstantCompilation,
    /// Actual native ordering units; missing original Jim counts remain unknown.
    characters: Option<StringCharacterModel>,
    purpose: FoldPurpose,
    coercions: RefCell<Vec<NativeCoercionObligation>>,
    intrinsic_math: bool,
    math_bindings: Option<&'a IntrinsicMathQuery<'a>>,
    math_calls: Option<&'a ResolvedMathQuery<'a>>,
    operand_proofs: Option<&'a NativeOperandProofs>,
    integer_contents: Option<&'a crate::native_numeric::SourceIntegerContentsReads>,
    invocation_dialect: Option<tcl_native_model::InvocationDialect>,
    arithmetic: tcl_dialect::NativeArithmetic,
    env: &'a Env,
    native_family: Option<tcl_dialect::model::Family>,
    /// Why the walk's value is unreliable though every hook answered, so
    /// [`eval_tcl_expr`] declines to fold rather than risk a false I230
    /// unreachable-branch; the first reason stands
    /// ([`Self::decline`]). Three triggers: a leading-zero integer operand
    /// whose octal-vs-decimal reading is dialect-dependent while
    /// [`Self::octal`] is unknown (`ReleaseAmbiguous(NumeralGrammar)`); the
    /// wide-vs-2⁶³-double comparison whose answer is undefined behaviour in
    /// C Tcl, the platform's rather than a release's (see [`numeric_cmp`],
    /// `ReleaseAmbiguous(Platform)`); and a string operation over a number
    /// whose string rep the target's spelling cannot give
    /// ([`Self::text_of`], `Unspellable`).
    declined: Option<DeclineReason>,
    /// The tower the fold keeps to ([`widens_past_a_wide`]): past it, a
    /// beyond-wide or infinite operand or result folds nothing
    /// ([`Self::beyond_tower`]). The analysis services gate the tower
    /// themselves, with its decline reason, so their value ops leave it open.
    tower: Tower,
    /// How a bare leading-zero integer (`08`, `010`) is read in `==`/`!=`/`<`/…
    /// numeric eligibility: `Some(true)` = octal (Tcl 8.x — `08`/`09` invalid →
    /// string, `010` → 8), `Some(false)` = decimal (Tcl 9.0 — `08` → 8,
    /// `010` → 10), `None` = dialect unknown → decline (see [`Self::declined`]).
    octal: Option<bool>,
    /// The release's numeric-literal grammar for reading operands — which radix
    /// prefixes exist and whether `_` separates digits. Broader than
    /// [`Self::octal`], which is only the leading-zero rule.
    numbers: NumberSyntax,
    /// How the target spells a computed number — the string rep a string
    /// operation reads ([`Self::text_of`]).
    spelling: NumberSpelling,
    /// The newest math-function release the active dialect provides.  A call
    /// to a function introduced *after* this tier folds nothing — the
    /// `::tcl::mathfunc::*` command it would dispatch to does not exist in that
    /// core, so the runtime would error rather than produce a constant.
    /// `None` leaves the set unbounded (dialect not resolved).
    math_since: Option<tcl_syntax::expr::mathfunc::MathFuncSince>,
    /// Explicit authored string-predicate policy for an admitted iRules fold.
    f5_predicates: Option<tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider>,
    /// The active dialect's word-value rules — how a list-shaped value
    /// divides.  `in` / `ni` membership and the `llength` folds split a list
    /// here, so they must split it the way the document's own runtime does.
    word_rules: tcl_syntax::word_rules::WordValueRules,
}

// Core dispatch

// Math function calls

// Unary operators

// Tests

#[cfg(test)]
mod tests;

#[cfg(test)]
mod double_string_policy_tests;

#[cfg(test)]
mod reached_math_binding_tests;
