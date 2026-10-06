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

use std::{cell::RefCell, collections::HashMap};

use tcl_dialect::{NumberSyntax, StringCharacterModel};

use crate::expr_ast::{BinOp, ExprNode, UnaryOp};
mod native_constant;
pub(crate) use native_constant::{NativeConstantResult, eval_native_constant};

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
        read: crate::ir::SourceSite,
        /// Every actual physical alternative at this read.
        cells: Vec<crate::place::CellIdentity>,
    },
}

/// Retained object evidence supplied independently of mathematical values.
#[derive(Debug, Clone)]
pub struct RetainedNativeOperandProof {
    /// Actual native grammar/protocol of the object evidence's interpreter.
    pub dialect: tcl_registry::InvocationDialect,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExpressionResultDependency {
    /// Jim returns the selected retained operand object unchanged.
    SelectedOperand {
        /// Original reference identifying the selected read.
        reference: String,
        /// Original parser offset of the selected reference.
        start: Option<u32>,
        /// Existing bytes, if already known without materialising the object.
        existing_bytes: Option<String>,
    },
    /// The native engine retains string bytes without numeric normalization.
    StringResult {
        /// Exact returned string bytes.
        bytes: String,
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
    /// Exact invocation snapshot for engine-owned native function tables.
    pub invocation_dialect: Option<tcl_registry::InvocationDialect>,
    /// Integer representation and overflow policy of the selected engine.
    /// Legacy callers without an engine retain the modern Tcl tower.
    pub arithmetic: Option<tcl_dialect::NativeArithmetic>,
    /// Leading-zero octal policy: `Some(true)` = the 8.x octal rule,
    /// `Some(false)` = the 9.0 decimal rule, `None` = decline to fold a
    /// dialect-ambiguous leading-zero operand.
    pub octal: Option<bool>,
    /// The active dialect's profile, for the passes that re-parse a nested
    /// `[expr …]` while folding: the parse must use the same grammar the
    /// document was lexed under, never the ambient one.
    pub dialect: Option<&'static tcl_dialect::DialectProfile>,
    /// Explicit authored F5 predicate recipe for this fold. It grants no
    /// native C/Jim execution or measured appliance-context capability.
    pub f5_predicates: Option<tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider>,
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
    /// Explicit engine identity, including registry-less native snapshots.
    pub native_family: Option<tcl_dialect::model::Family>,
    /// Pure mathematical evaluation explicitly requested by a caller. This is
    /// not a Tcl execution proof; execution consumers leave it false and supply
    /// the reached native function-binding query instead.
    pub intrinsic_math: bool,
}

impl FoldPolicy {
    /// Select mathematical intrinsic semantics without claiming Tcl dispatch.
    /// Use only for an explicitly mathematical evaluation API or after another
    /// owner has proved the exact native implementation for every reached call.
    #[must_use]
    pub const fn with_intrinsic_math(mut self) -> Self {
        self.intrinsic_math = true;
        self
    }

    /// Render a floating result only when native precision is immutable.
    /// Numeric folding itself can proceed under mutable precision; string
    /// materialisation requires this independent interpreter-world proof.
    #[must_use]
    pub fn double_format(self) -> Option<tcl_dialect::DoubleFormat> {
        self.invocation_dialect
            .and_then(tcl_registry::InvocationDialect::double_string_policy)
            .and_then(tcl_dialect::DoubleStringPolicy::constant_format)
    }

    /// The policy for an explicit octal rule with no known dialect — the
    /// iRules word operators are declined.
    #[must_use]
    pub const fn from_octal(octal: Option<bool>) -> Self {
        Self {
            invocation_dialect: None,
            arithmetic: None,
            dialect: None,
            native_family: None,
            octal,
            f5_predicates: None,
            characters: None,
            numbers: None,
            word_rules: tcl_syntax::word_rules::WordValueRules::TCL,
            intrinsic_math: false,
        }
    }

    /// The policy for an octal rule plus a resolved profile. Name parsing and
    /// alias handling happen before this point, so every fact comes from one
    /// canonical profile.
    #[must_use]
    pub fn for_profile(
        octal: Option<bool>,
        profile: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Self {
        Self {
            invocation_dialect: profile.map(tcl_registry::InvocationDialect::of_profile),
            arithmetic: profile.and_then(|profile| {
                tcl_registry::InvocationDialect::of_profile(profile).arithmetic()
            }),
            octal,
            dialect: profile,
            native_family: profile
                .and_then(|profile| tcl_registry::InvocationDialect::of_profile(profile).family()),
            f5_predicates: profile
                .is_some_and(tcl_dialect::DialectProfile::is_irules)
                .then_some(tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider::F5Trunk),
            characters: profile.and_then(tcl_dialect::DialectProfile::character_model),
            numbers: profile.map(|p| NumberSyntax::of_profile(Some(p))),
            word_rules: tcl_syntax::word_rules::WordValueRules::of_profile(profile),
            intrinsic_math: false,
        }
    }

    /// Complete parser axes expected by a native expression preparation proof.
    /// Missing engine and source profile remain unknown; display names and
    /// mathematical intrinsic mode do not establish executable preparation.
    #[must_use]
    pub fn preparation_context(self) -> Option<tcl_syntax::expr::parser::ExprParseContext> {
        self.invocation_dialect
            .map(|dialect| dialect.expression_parse_context(self.dialect))
            .or_else(|| {
                self.dialect
                    .map(tcl_syntax::expr::parser::ExprParseContext::for_profile)
            })
    }

    /// Preserve native execution policy independently of the catalogue's
    /// authoring expression profile, applying only the retained lexer overlay.
    #[must_use]
    pub fn for_retained_entry(
        registry: &tcl_registry::CommandRegistry,
        invocation: Option<tcl_registry::InvocationDialect>,
        config: &tcl_lexer::LexerConfig,
    ) -> Self {
        let policy = Self::from_registry(registry);
        let Some(mut actual) = invocation else {
            return policy;
        };
        actual.lexer_grammar = config.grammar_over(actual.lexer_grammar);
        actual.word_values =
            tcl_syntax::word_rules::WordValueRules::from_grammar(&actual.lexer_grammar);
        policy.with_invocation_dialect(actual)
    }

    /// Retain the exact invocation's engine and grammar instead of inferring
    /// them from a catalogue profile which may be absent or unrelated.
    #[must_use]
    pub fn with_invocation_dialect(mut self, dialect: tcl_registry::InvocationDialect) -> Self {
        self.invocation_dialect = Some(dialect);
        self.arithmetic = dialect.arithmetic();
        self.numbers = Some(dialect.numbers);
        self.octal = Some(dialect.numbers.leading_zero_is_octal());
        self.word_rules = dialect.word_values;
        self.native_family = dialect.family();
        self.characters = dialect.characters;
        if self.native_family == Some(tcl_dialect::model::Family::Jim) {
            self.characters = None;
            self.f5_predicates = None;
        }
        self
    }

    /// The policy a registry's own dialect profile implies — both facts from
    /// the one source of truth, for the pipeline entry points that hold a
    /// registry rather than a dialect string.
    #[must_use]
    pub fn from_registry(registry: &tcl_registry::CommandRegistry) -> Self {
        Self {
            invocation_dialect: registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile),
            arithmetic: registry.profile().and_then(|profile| {
                tcl_registry::InvocationDialect::of_profile(profile).arithmetic()
            }),
            octal: registry.octal_fold_policy(),
            dialect: registry.profile(),
            native_family: registry
                .profile()
                .and_then(|profile| tcl_registry::InvocationDialect::of_profile(profile).family()),
            f5_predicates: registry
                .profile()
                .is_some_and(tcl_dialect::DialectProfile::is_irules)
                .then_some(tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider::F5Trunk),
            characters: registry.character_model(),
            numbers: Some(registry.numbers()),
            word_rules: tcl_syntax::word_rules::WordValueRules::of_profile(registry.profile()),
            intrinsic_math: false,
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
    eval_with_config(node, env, policy)
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

fn make_fold_ops<'a>(
    env: &'a Env,
    policy: FoldPolicy,
    bindings: Option<&'a IntrinsicMathQuery<'a>>,
    calls: Option<&'a ResolvedMathQuery<'a>>,
    native_inputs: FoldNativeInputs<'a>,
    analysis: bool,
) -> FoldOps<'a> {
    FoldOps {
        purpose: if analysis {
            FoldPurpose::AnalysisValue
        } else {
            FoldPurpose::Executable
        },
        coercions: RefCell::new(Vec::new()),
        intrinsic_math: policy.intrinsic_math,
        math_bindings: bindings,
        math_calls: calls,
        operand_proofs: native_inputs.operands,
        integer_contents: native_inputs.integer_contents,
        invocation_dialect: policy.invocation_dialect,
        characters: policy.characters,
        arithmetic: policy
            .arithmetic
            .unwrap_or(tcl_dialect::NativeArithmetic::TclBignum),
        env,
        ambiguous: false,
        octal: policy.octal,
        numbers: policy.numbers.unwrap_or_else(|| {
            if policy.octal == Some(true) {
                NumberSyntax::Tcl85
            } else {
                NumberSyntax::default()
            }
        }),
        math_since: policy.dialect.and_then(math_func_ceiling_for_dialect),
        f5_predicates: policy.f5_predicates,
        word_rules: policy.word_rules,
        native_family: policy.native_family,
        constant_compilation: ConstantCompilation::default(),
    }
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
    // The final value must reduce to a number (a bare string like `expr {"x"}`
    // doesn't fold) — `to_number` maps a `Str` result through `parse_literal`.
    let result = tcl_syntax::expr::eval(node, &mut ops).ok()?;
    if ops.ambiguous {
        // A comparison hit a leading-zero operand whose octal-vs-decimal
        // reading is dialect-dependent and the dialect is unknown — decline
        // to fold rather than pick one.
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
        FoldValue::Str(bytes) => Some(NativeExpressionResultDependency::StringResult {
            bytes: bytes.clone(),
        }),
        FoldValue::RetainedNativeObject {
            value,
            proof,
            reference,
            start,
            ..
        } => {
            let existing_bytes = proof
                .as_ref()
                .and_then(|proof| proof.existing_string.clone())
                .or_else(|| {
                    if let FoldValue::Str(bytes) = value.as_ref() {
                        Some(bytes.clone())
                    } else {
                        None
                    }
                });
            Some(NativeExpressionResultDependency::SelectedOperand {
                reference: reference.clone(),
                start: *start,
                existing_bytes,
            })
        }
        _ => None,
    }
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
    /// An environment value retains an actual runtime object. Missing evidence
    /// cannot erase a representation-changing conversion of a shared object.
    RetainedNativeObject {
        value: Box<FoldValue>,
        proof: Option<Box<RetainedNativeOperandProof>>,
        integer_contents: Option<Box<crate::native_numeric::SourceIntegerContentsRead>>,
        reference: String,
        start: Option<u32>,
    },
}

impl FoldValue {
    /// Interpret as a number, or `None` when the text isn't numeric.
    fn to_number(&self, numbers: NumberSyntax) -> Option<TclValue> {
        match self {
            FoldValue::Int(i) => Some(TclValue::Int(*i)),
            FoldValue::Big(b) => Some(TclValue::from_big(b.clone())),
            FoldValue::Float(f) => Some(TclValue::Float(*f)),
            FoldValue::Str(s) => parse_literal_in(s, numbers),
            FoldValue::RetainedNativeObject {
                proof,
                integer_contents,
                ..
            } => proof.as_ref().map(|proof| proof.value.clone()).or_else(|| {
                integer_contents
                    .as_ref()
                    .map(|receipt| receipt.number().clone())
            }),
        }
    }
    /// Render as a string: raw for `Str`, canonical for numbers.
    fn to_string_val(&self, format: Option<tcl_dialect::DoubleFormat>) -> Option<String> {
        Some(match self {
            FoldValue::Str(s) => s.clone(),
            FoldValue::Int(i) => i.to_string(),
            FoldValue::Big(b) => b.to_string(),
            FoldValue::Float(f) => tcl_syntax::number::format_double_selected(*f, format?),
            FoldValue::RetainedNativeObject {
                proof,
                integer_contents,
                ..
            } => proof
                .as_ref()
                .and_then(|proof| proof.existing_string.clone())
                .or_else(|| {
                    integer_contents
                        .as_ref()
                        .map(|receipt| receipt.contents().to_owned())
                })?,
        })
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
#[derive(Clone, Copy, PartialEq, Eq)]
enum FoldPurpose {
    Executable,
    AnalysisValue,
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
    invocation_dialect: Option<tcl_registry::InvocationDialect>,
    arithmetic: tcl_dialect::NativeArithmetic,
    env: &'a Env,
    native_family: Option<tcl_dialect::model::Family>,
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
    /// Explicit authored string-predicate policy for an admitted iRules fold.
    f5_predicates: Option<tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider>,
    /// The active dialect's word-value rules — how a list-shaped value
    /// divides.  `in` / `ni` membership and the `llength` folds split a list
    /// here, so they must split it the way the document's own runtime does.
    word_rules: tcl_syntax::word_rules::WordValueRules,
}

impl FoldOps<'_> {
    fn operand_for<'a>(&self, value: &'a FoldValue, kind: NativeCoercionKind) -> &'a FoldValue {
        if self.purpose == FoldPurpose::AnalysisValue
            && let FoldValue::RetainedNativeObject {
                value,
                proof: None,
                integer_contents,
                reference,
                start,
            } = value
        {
            if kind == NativeCoercionKind::Number && integer_contents.is_some() {
                return value;
            }
            let obligation = NativeCoercionObligation {
                reference: reference.clone(),
                start: *start,
                kind,
            };
            let mut coercions = self.coercions.borrow_mut();
            if !coercions.contains(&obligation) {
                coercions.push(obligation);
            }
            value
        } else {
            value
        }
    }

    fn normalize_number(&self, value: TclValue) -> Option<TclValue> {
        match value {
            TclValue::Big(value) if self.arithmetic != tcl_dialect::NativeArithmetic::TclBignum => {
                tcl_syntax::expr::wide::literal(self.arithmetic, &value)
                    .ok()
                    .map(TclValue::Int)
            }
            value => Some(value),
        }
    }

    fn number_for(&self, value: &FoldValue, kind: NativeCoercionKind) -> Option<TclValue> {
        let value = self.operand_for(value, kind);
        self.normalize_number(value.to_number(self.numbers)?)
    }

    fn strict_number(&self, value: &FoldValue) -> Option<TclValue> {
        let value = self.operand_for(value, NativeCoercionKind::Number);
        self.normalize_number(strict_number_for_dialect(value, self.octal, self.numbers)?)
    }

    fn classify(&self, value: &FoldValue) -> Operand {
        let value = self.operand_for(value, NativeCoercionKind::NumericComparison);
        match classify_operand(value, self.octal, self.numbers) {
            Operand::Num(value) => self
                .normalize_number(value)
                .map_or(Operand::Str, Operand::Num),
            result => result,
        }
    }
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
        FoldValue::RetainedNativeObject {
            proof,
            integer_contents,
            ..
        } => {
            return proof
                .as_ref()
                .map(|proof| Operand::Num(proof.value.clone()))
                .or_else(|| {
                    integer_contents
                        .as_ref()
                        .map(|receipt| Operand::Num(receipt.number().clone()))
                })
                .unwrap_or(Operand::Ambiguous);
        }
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
        if self.arithmetic == tcl_dialect::NativeArithmetic::JimWide
            && let Some(number) = self.strict_number(&FoldValue::Str(text.to_owned()))
        {
            return Ok(FoldValue::from_tcl(number));
        }
        Ok(FoldValue::Str(text.to_owned()))
    }
    fn string(&mut self, inner: &str, substitutes: bool) -> Result<FoldValue, ()> {
        // A `"…"` operand substitutes `$var`, `[cmd]` and backslashes, and a
        // folded constant must be the value Tcl computes, not the spelling.
        // Taking the text as-is folded `expr {"pre$x"}` to `pre$x` and
        // `if {"$x" eq "5"}` to false, and O112 then deleted the live branch
        // (#2227). Declining costs an optimisation; folding wrong costs the
        // program.
        // A `{…}` operand's backslash-newline folds to a space in Tcl and
        // stays as written in Jim; the folder does not know which, so it
        // declines that too rather than guess.
        tcl_syntax::expr::fixed_string_body(inner, substitutes)
            .map(|body| FoldValue::Str(body.to_owned()))
            .ok_or(())
    }
    fn var(&mut self, name: &str) -> Result<FoldValue, ()> {
        let value = match self.env.get(name) {
            Some(EnvValue::Int(i)) => FoldValue::Int(*i),
            Some(EnvValue::Float(f)) => FoldValue::Float(*f),
            Some(EnvValue::Str(s)) => FoldValue::Str(s.clone()),
            None => return Err(()), // unbound → can't fold
        };
        if self.intrinsic_math {
            return Ok(value);
        }
        let proof = self
            .operand_proofs
            .and_then(|proofs| proofs.get(name))
            .cloned();
        if proof.as_ref().is_some_and(|proof| {
            Some(proof.dialect) != self.invocation_dialect
                || value.to_number(self.numbers).as_ref() != Some(&proof.value)
        }) {
            return Err(());
        }
        let integer_contents = self
            .integer_contents
            .and_then(|proofs| proofs.get(name))
            .cloned();
        if let Some(receipt) = &integer_contents {
            let FoldValue::Str(contents) = &value else {
                return Err(());
            };
            if proof.is_some() || !receipt.accepts_number(self.invocation_dialect, contents) {
                return Err(());
            }
        }
        Ok(FoldValue::RetainedNativeObject {
            value: Box::new(value),
            proof: proof.map(Box::new),
            integer_contents: integer_contents.map(Box::new),
            reference: name.to_owned(),
            start: None,
        })
    }
    fn variable_reference_at(&mut self, reference: &str, start: u32) -> Result<FoldValue, ()> {
        let name = if let Some(dialect) = self.invocation_dialect {
            crate::native_lowering::cells::variable_reference_place(
                reference,
                tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            )
            .map_err(|_| ())?
            .spelling()
        } else if self.intrinsic_math {
            // Explicit mathematical compatibility evaluation grants no native
            // effects or erasure proof. Execution needs the actual grammar.
            tcl_syntax::naming::var_reference(reference).to_owned()
        } else {
            return Err(());
        };
        let mut value = self.var(&name)?;
        if let FoldValue::RetainedNativeObject {
            reference: original,
            start: site,
            ..
        } = &mut value
        {
            reference.clone_into(original);
            *site = Some(start);
        }
        Ok(value)
    }
    fn command(&mut self, _script: &str) -> Result<FoldValue, ()> {
        Err(()) // command substitution is opaque at compile time
    }
    fn call_at(
        &mut self,
        function: &str,
        args: Vec<FoldValue>,
        start: u32,
    ) -> Result<FoldValue, ()> {
        if let Some(query) = self.math_calls {
            let target = query(function, start).ok_or(())?;
            let mut composed = target
                .prepended
                .into_iter()
                .map(FoldValue::Str)
                .collect::<Vec<_>>();
            composed.extend(args);
            return self.call(&target.function, composed);
        }
        if !self.intrinsic_math
            && !self
                .math_bindings
                .is_some_and(|proof| proof(function, start))
        {
            return Err(());
        }
        self.call(function, args)
    }

    fn call(&mut self, function: &str, args: Vec<FoldValue>) -> Result<FoldValue, ()> {
        use tcl_syntax::expr::mathfunc::{
            IntWidth, NativeMathProtocol, Num, accepts_boolean_operand, added_in,
            try_dispatch_with_backend_protocol,
        };
        let name = function.to_ascii_lowercase();
        if self.native_family == Some(tcl_dialect::model::Family::Jim) {
            let functions = self
                .invocation_dialect
                .and_then(tcl_registry::mathfunc::jim_fixed_math_function_names)
                .ok_or(())?;
            if !functions.contains(&function) {
                return Err(());
            }
        }
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
        let nums: Option<Vec<Num>> = args
            .iter()
            .map(|v| {
                let parsed = if boolean_ok {
                    self.number_for(v, NativeCoercionKind::Boolean)
                } else {
                    self.strict_number(v)
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
        let protocol = self
            .invocation_dialect
            .and_then(tcl_registry::mathfunc::native_math_protocol)
            .or_else(|| self.intrinsic_math.then_some(NativeMathProtocol::Tcl))
            .ok_or(())?;
        match try_dispatch_with_backend_protocol(
            &name,
            &nums.ok_or(())?,
            IntWidth::for_native_arithmetic(self.arithmetic),
            protocol,
        )
        .map_err(|_| ())?
        {
            Num::Int(i) => Ok(FoldValue::Int(i)),
            Num::Float(f) => Ok(FoldValue::Float(f)),
            Num::Big(never) => match never {},
        }
    }

    fn arith(&mut self, op: BinOp, left: FoldValue, right: FoldValue) -> Result<FoldValue, ()> {
        self.check_native_constant_arithmetic(op, &left, &right)?;
        // Arithmetic operands are strict numbers: Tcl's `+`/`-`/`*`/… read
        // them with `Tcl_GetNumberFromObj`, which rejects boolean words, so
        // `expr {true + 0}` is an error, not `1`. `strict_number_for_dialect`
        // omits the boolean coercion `to_number`/`parse_literal` add, and
        // additionally honours the dialect's leading-zero rule (see its doc).
        let a = self.strict_number(&left).ok_or(())?;
        let b = self.strict_number(&right).ok_or(())?;
        if self.arithmetic != tcl_dialect::NativeArithmetic::TclBignum
            && let (TclValue::Int(x), TclValue::Int(y)) = (&a, &b)
        {
            return tcl_syntax::expr::wide::binary(self.arithmetic, op, *x, *y)
                .map(FoldValue::Int)
                .map_err(|_| ());
        }
        apply_binary(op, a, b).map(FoldValue::from_tcl).ok_or(())
    }
    fn unary(&mut self, op: UnaryOp, value: FoldValue) -> Result<FoldValue, ()> {
        self.check_native_constant_unary(op, &value)?;
        if self.arithmetic != tcl_dialect::NativeArithmetic::TclBignum
            && matches!(op, UnaryOp::Pos | UnaryOp::Neg | UnaryOp::BitNot)
            && let TclValue::Int(value) = self.strict_number(&value).ok_or(())?
        {
            return tcl_syntax::expr::wide::unary(self.arithmetic, op, value)
                .map(FoldValue::Int)
                .map_err(|_| ());
        }
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
                let truthy = match self
                    .number_for(&value, NativeCoercionKind::Boolean)
                    .ok_or(())?
                {
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
                match self.strict_number(&value).ok_or(())? {
                    // `+NaN` is "can't use non-numeric floating-point value as
                    // operand" in C — never a foldable value.
                    TclValue::Float(f) if f.is_nan() => Err(()),
                    v => Ok(FoldValue::from_tcl(v)),
                }
            }
            UnaryOp::Neg => match self.strict_number(&value).ok_or(())? {
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
                match self.strict_number(&value).ok_or(())? {
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
        let (lo, ro) = (self.classify(left), self.classify(right));
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
    fn compare_string(
        &mut self,
        left: &FoldValue,
        right: &FoldValue,
    ) -> Result<std::cmp::Ordering, ()> {
        let left = self.operand_for(left, NativeCoercionKind::String);
        let right = self.operand_for(right, NativeCoercionKind::String);
        let format = self
            .invocation_dialect
            .and_then(tcl_registry::InvocationDialect::double_string_policy)
            .and_then(tcl_dialect::DoubleStringPolicy::constant_format);
        let result = (|| {
            let model = self.characters?;
            // Jim ordering consumes cached native character counts. Known
            // bytes alone cannot reconstruct those original object caches.
            if model == StringCharacterModel::Jim084Utf8 {
                return None;
            }
            let left = tcl_syntax::raw_string::RawString::from_unicode(left.to_string_val(format)?);
            let right =
                tcl_syntax::raw_string::RawString::from_unicode(right.to_string_val(format)?);
            left.compare_character_units(model, &right, (0, 0)).ok()
        })();
        result.ok_or_else(|| {
            self.ambiguous = true;
        })
    }
    fn equal_string(&mut self, left: &FoldValue, right: &FoldValue) -> Result<bool, ()> {
        if self
            .invocation_dialect
            .and_then(|dialect| dialect.characters)
            != Some(StringCharacterModel::Jim084Utf8)
        {
            return self
                .compare_string(left, right)
                .map(std::cmp::Ordering::is_eq);
        }
        // Jim's eq/ne operation compares byte lengths and bytes directly;
        // unlike ordering it does not require a retained character cache.
        let left = self.operand_for(left, NativeCoercionKind::String);
        let right = self.operand_for(right, NativeCoercionKind::String);
        let format = self
            .invocation_dialect
            .and_then(tcl_registry::InvocationDialect::double_string_policy)
            .and_then(tcl_dialect::DoubleStringPolicy::constant_format);
        let left = left.to_string_val(format).ok_or(())?;
        let right = right.to_string_val(format).ok_or(())?;
        Ok(left.as_bytes() == right.as_bytes())
    }
    fn in_list(&mut self, needle: &FoldValue, list: &FoldValue) -> Result<bool, ()> {
        // Membership converts its list operand. Numeric/value-only evidence
        // cannot prove that changing a retained shared object's list
        // representation is unobserved.
        if self.purpose == FoldPurpose::Executable
            && matches!(list, FoldValue::RetainedNativeObject { .. })
        {
            return Err(());
        }
        let needle = self.operand_for(needle, NativeCoercionKind::String);
        let list = self.operand_for(list, NativeCoercionKind::List);
        let format = self
            .invocation_dialect
            .and_then(tcl_registry::InvocationDialect::double_string_policy)
            .and_then(tcl_dialect::DoubleStringPolicy::constant_format);
        let n = needle.to_string_val(format).ok_or(())?;
        let list = list.to_string_val(format).ok_or(())?;
        Ok(split_tcl_list(&list, self.word_rules).contains(&n))
    }

    fn to_bool(&mut self, value: &FoldValue) -> Result<bool, ()> {
        self.check_native_constant_boolean(value)?;
        // Boolean contexts (`?:`, `&&`, `||`) reject NaN — a domain error in
        // C Tcl ("floating point value is Not a Number"), so the fold
        // declines rather than pick a truth value.
        match self
            .number_for(value, NativeCoercionKind::Boolean)
            .ok_or(())?
        {
            TclValue::Float(f) if f.is_nan() => Err(()),
            v => Ok(v.is_truthy()),
        }
    }
    fn bool_value(&mut self, b: bool) -> FoldValue {
        FoldValue::Int(i64::from(b))
    }
    fn unsupported(&mut self, _what: &str) {}

    /// Evaluate shared F5 predicates only under the selected authored policy.
    /// Native C/Jim and absent dialect policies decline these operations.
    fn binary_other(
        &mut self,
        op: BinOp,
        left: FoldValue,
        right: FoldValue,
    ) -> Result<FoldValue, ()> {
        let provider = self.f5_predicates.ok_or(())?;
        let left = self.operand_for(&left, NativeCoercionKind::String);
        let right = self.operand_for(&right, NativeCoercionKind::String);
        let format = self
            .invocation_dialect
            .and_then(tcl_registry::InvocationDialect::double_string_policy)
            .and_then(tcl_dialect::DoubleStringPolicy::constant_format);
        apply_irules_string_op(
            provider,
            op,
            &left.to_string_val(format).ok_or(())?,
            &right.to_string_val(format).ok_or(())?,
        )
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

/// Render a folded numeric value using its exact native string policy.
/// Mutable or unknown floating precision remains a runtime operation.
#[must_use]
pub fn format_tcl_value_with_policy(value: &TclValue, policy: FoldPolicy) -> Option<String> {
    match value {
        TclValue::Float(value) => Some(tcl_syntax::number::format_double_selected(
            *value,
            policy.double_format()?,
        )),
        _ => Some(format_tcl_value(value)),
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
        FoldValue::RetainedNativeObject {
            proof,
            integer_contents,
            ..
        } => proof.as_ref().map(|proof| proof.value.clone()).or_else(|| {
            integer_contents
                .as_ref()
                .map(|receipt| receipt.number().clone())
        }),
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
        // divergence guards, pinned against tclsh:
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
/// A thin wrapper over the word-value owner's tolerant split
/// ([`tcl_syntax::word_rules::WordValueRules::split_list_tolerant`]):
/// membership (`in` / `ni`) compares against decoded element values, and
/// element *counts* (`llength` folds) need the full grammar so an escaped
/// separator like `a\ b` counts as one element.  Tolerant of a malformed tail
/// so folding a partial list still yields a best-effort element list rather
/// than nothing.
///
/// `rules` are the document dialect's word-value rules, so a `JimTcl`
/// document's list divides the way its own runtime divides it.
pub(crate) fn split_tcl_list(
    text: &str,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Vec<String> {
    rules
        .split_list_tolerant(text)
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
fn apply_irules_string_op(
    provider: tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider,
    op: BinOp,
    left: &str,
    right: &str,
) -> Option<TclValue> {
    let predicate = provider.predicate(op)?;
    Some(TclValue::Int(i64::from(predicate.evaluate(left, right))))
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

    #[test]
    fn string_ordering_uses_selected_native_units_and_jim_equality_uses_bytes() {
        for (profile, ordered) in [
            ("tcl8.4", Some(0)),
            ("tcl8.5", Some(0)),
            ("tcl8.6", Some(0)),
            ("tcl9.0", Some(1)),
            ("tcl9.1", Some(1)),
            ("jim", None),
        ] {
            let supplied = tcl_registry::model::ingress::static_context_for(profile);
            let registry = supplied.commands();
            let selected = registry.profile().unwrap();
            let policy = FoldPolicy::for_retained_entry(
                registry,
                Some(tcl_registry::InvocationDialect::of_profile(selected)),
                &tcl_lexer::LexerConfig::from_grammar(selected.grammar),
            );
            for (expression, expected) in [
                ("\"\u{10000}\" > \"\u{e000}\"", ordered),
                ("\"\u{10000}\" eq \"\u{10000}\"", Some(1)),
                ("\"\u{10000}\" eq \"\u{e000}\"", Some(0)),
            ] {
                let tree = tcl_syntax::expr::parser::parse_expr_with_syntax_context(
                    expression,
                    &policy.preparation_context().unwrap(),
                );
                assert_eq!(
                    eval_tcl_expr_with_policy(&tree, &Env::new(), policy),
                    expected.map(TclValue::Int),
                    "{profile}: {expression}"
                );
            }
        }
    }

    /// Parse + evaluate using the iRules dialect, which enables
    /// `contains`/`starts_with`/`ends_with`/`equals`/`matches_glob`/
    /// `matches_regex`/`in`/`ni` word operators. Must use the
    /// dialect-threading evaluator, not the bare [`eval_tcl_expr`] — the
    /// word operators parse under any dialect gate, but only actually
    /// *fold* when [`FoldOps::f5_predicates`] is set, which only
    /// [`eval_tcl_expr_in_dialect`] does.
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
            let profile =
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
            // This test selects mathematical implementation grammar explicitly;
            // it supplies no executable command-table proof.
            eval_tcl_expr_with_policy(
                &parse_expr(expr, Some(dialect)),
                &env,
                FoldPolicy::for_profile(leading_zero_is_octal(profile), Some(profile))
                    .with_intrinsic_math(),
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
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
            assert_eq!(
                eval_tcl_expr_in_dialect(&parse_expr("abs(-5)", Some(dialect)), &env, profile),
                None,
                "a profile alone cannot establish a reached native function"
            );
        }
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
            assert_eq!(eval_d("abs(-010)", d), None, "unproved function: {d}");
            let profile = tcl_registry::model::ingress::resolve_environment(d).analyser_profile();
            assert_eq!(
                eval_tcl_expr_with_policy(
                    &parse_expr("abs(-010)", Some(d)),
                    &Env::new(),
                    FoldPolicy::for_profile(leading_zero_is_octal(profile), Some(profile))
                        .with_intrinsic_math(),
                ),
                Some(TclValue::Int(8)),
                "selected mathematical grammar: {d}"
            );
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

    // matches_regex is never constant-folded.

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

    // Simple iRules string ops.

    /// `FoldOps::binary_other` must only fold the iRules word operators under
    /// an iRules dialect, and decline (not panic, not silently misfold)
    /// everywhere else — the defence-in-depth check for
    /// call sites that reach this evaluator without a dialect string
    /// (`eval_tcl_expr`/`eval_tcl_expr_with_octal`).
    #[test]
    fn measured_bigip_bare_matches_glob_discriminators_fold_only_under_authored_policy() {
        let env = Env::new();
        for (row_index, row) in
            include_str!("../../tcl-syntax/tests/data/f5-matches-21.1.0.1-0.0.26.tsv")
                .lines()
                .enumerate()
        {
            let fields = row.split('\t').collect::<Vec<_>>();
            let expression = fields[0];
            let expected = if fields[1] == "0" {
                Some(TclValue::Int(fields[2].parse().unwrap()))
            } else {
                None
            };
            assert_eq!(eval_irules(expression), expected, "{expression}");
            let node = parse_expr(expression, Some("f5-irules"));
            if row_index >= 6 {
                continue;
            }
            assert_eq!(eval_tcl_expr(&node, &env), None);
            for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
                assert_eq!(
                    eval_tcl_expr_in_dialect(
                        &node,
                        &env,
                        tcl_registry::model::ingress::resolve_environment(engine)
                            .analyser_profile()
                    ),
                    None,
                    "{engine}: {expression}"
                );
            }
        }
    }

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

    // matches_glob + in/ni.

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
        // A class is written braced: inside `"…"` the `[bxy]` is a command
        // substitution, which the folder declines (#2227).
        assert_eq!(
            eval_irules(r#""abc" matches_glob {a[bxy]c}"#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""axc" matches_glob {a[bxy]c}"#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""azc" matches_glob {a[bxy]c}"#),
            Some(TclValue::Int(0))
        );
        assert_eq!(eval_irules(r#""abc" matches_glob "a[bxy]c""#), None);
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
        assert_eq!(
            split_tcl_list("a b c", tcl_syntax::word_rules::WordValueRules::default()),
            vec!["a", "b", "c"]
        );
        assert_eq!(
            split_tcl_list(
                "{hello world} foo",
                tcl_syntax::word_rules::WordValueRules::default()
            ),
            vec!["hello world", "foo"]
        );
        assert_eq!(
            split_tcl_list("", tcl_syntax::word_rules::WordValueRules::default()),
            Vec::<String>::new()
        );
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

    // Math function dispatch.

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

    /// The float-edge reference table (tclsh 8.6.14; 9.0 agrees). Errors fold
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

    /// Values verified on tclsh 8.6.14/9.0 (see type-tracking.md):
    /// exact integer arithmetic at and beyond the wide
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

    /// The fold is C's `modf` form, not `floor(d + 0.5)`, which rounds twice
    /// and would bake `1` and `4503599627370498` into the compiled code where
    /// tclsh 8.6.16/9.0.4 answer `0` and `4503599627370497`.
    #[test]
    fn math_round_folds_the_exact_value_not_d_plus_a_half() {
        assert_eq!(
            eval_str("round(0.49999999999999994)"),
            Some(TclValue::Int(0))
        );
        assert_eq!(
            eval_str("round(-0.49999999999999994)"),
            Some(TclValue::Int(0))
        );
        assert_eq!(
            eval_str("round(4503599627370497.0)"),
            Some(TclValue::Int(4_503_599_627_370_497))
        );
        assert_eq!(
            eval_str("round(-4503599627370497.0)"),
            Some(TclValue::Int(-4_503_599_627_370_497))
        );
        assert_eq!(
            eval_str("round(2251799813685248.5)"),
            Some(TclValue::Int(2_251_799_813_685_249))
        );
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
        // A `Float` operand must not fall to
        // `tcl_syntax::expr::mathfunc::dispatch`'s catch-all `None` (treated
        // as a domain error): real Tcl accepts one, truncating
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

#[cfg(test)]
mod double_string_policy_tests {
    use super::*;

    fn selected(version: tcl_dialect::TclVersion) -> FoldPolicy {
        FoldPolicy::default()
            .with_invocation_dialect(tcl_registry::InvocationDialect::for_version(version))
    }

    #[test]
    fn floating_string_folds_require_proved_native_precision() {
        let value = TclValue::Float(1.0 / 3.0);
        for version in [
            tcl_dialect::TclVersion::V8_4,
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
        ] {
            assert_eq!(
                format_tcl_value_with_policy(&value, selected(version)),
                None
            );
        }
        assert_eq!(
            format_tcl_value_with_policy(&value, selected(tcl_dialect::TclVersion::V9_1)),
            Some("0.3333333333333333".to_owned())
        );
        let jim = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        assert_eq!(
            format_tcl_value_with_policy(
                &value,
                FoldPolicy::default().with_invocation_dialect(jim)
            ),
            Some("0.333333333333".to_owned())
        );
    }

    #[test]
    fn mutable_precision_blocks_string_comparison_but_keeps_numeric_comparison() {
        let policy = selected(tcl_dialect::TclVersion::V8_6);
        let string = crate::expr_parser::parse_expr(r#"(1.0/3) eq "0.3333333333333333""#, None);
        let numeric = crate::expr_parser::parse_expr("(1.0/3) < 1", None);
        assert_eq!(
            eval_tcl_expr_with_policy(&string, &Env::new(), policy),
            None
        );
        assert_eq!(
            eval_tcl_expr_with_policy(&numeric, &Env::new(), policy),
            Some(TclValue::Int(1))
        );
    }
}

#[cfg(test)]
mod reached_math_binding_tests {
    use super::*;

    #[test]
    fn analysis_preserves_jim_selected_operand_result_bytes_and_sharing() {
        let jim = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        let policy = FoldPolicy::default().with_invocation_dialect(jim);
        let env = Env::from([("x".into(), EnvValue::Str("003".into()))]);
        let original = crate::expr_parser::parse_expr("1?$x:0", None);
        let result = analyse_tcl_expr_with_resolved_math_bindings(
            &original,
            &env,
            policy,
            &|_, _| None,
            None,
        )
        .unwrap();
        assert_eq!(result.value, TclValue::Int(3));
        assert_eq!(
            result.coercions,
            [] as [crate::tcl_expr_eval::NativeCoercionObligation; 0]
        );
        assert_eq!(
            result.result_dependency,
            Some(NativeExpressionResultDependency::SelectedOperand {
                reference: "$x".into(),
                start: Some(2),
                existing_bytes: Some("003".into()),
            },)
        );
        assert!(!result.native_value_effects_are_proved());
        assert_eq!(
            eval_tcl_expr_with_resolved_math_bindings(&original, &env, policy, &|_, _| None,),
            None
        );
    }

    #[test]
    fn analysis_retains_values_and_only_reached_coercion_obligations() {
        let policy = FoldPolicy::default().with_invocation_dialect(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        let env = Env::from([("x".into(), EnvValue::Int(3))]);
        let calls = |function: &str, _| {
            Some(NativeMathFunctionTarget {
                function: function.to_owned(),
                prepended: Vec::new(),
            })
        };
        for (source, kind, start) in [
            ("$x+1", NativeCoercionKind::Number, 0),
            ("!!$x", NativeCoercionKind::Boolean, 2),
            ("$x==3", NativeCoercionKind::NumericComparison, 0),
            ("abs($x)", NativeCoercionKind::Number, 4),
            ("$x", NativeCoercionKind::ResultNormalization, 0),
        ] {
            let original = crate::expr_parser::parse_expr(source, None);
            let result =
                analyse_tcl_expr_with_resolved_math_bindings(&original, &env, policy, &calls, None)
                    .unwrap();
            assert_eq!(
                result.coercions,
                vec![NativeCoercionObligation {
                    reference: "$x".into(),
                    start: Some(start),
                    kind,
                }],
                "{source}"
            );
            assert!(!result.coercions_are_proved());
        }
        let original = crate::expr_parser::parse_expr("$x+1", None);
        assert_eq!(
            analyse_tcl_expr_with_resolved_math_bindings(&original, &env, policy, &calls, None,)
                .unwrap()
                .value,
            TclValue::Int(4)
        );
        let skipped = crate::expr_parser::parse_expr("1?abs(-3):$x+1", None);
        let result =
            analyse_tcl_expr_with_resolved_math_bindings(&skipped, &env, policy, &calls, None)
                .unwrap();
        assert_eq!(result.value, TclValue::Int(3));
        assert!(result.coercions_are_proved());
        // Mathematical constants cannot turn the original retained read into a
        // fresh object simply because its value happens to be known.
        assert_eq!(
            eval_tcl_expr_with_resolved_math_bindings(&original, &env, policy, &calls,),
            None
        );
    }

    #[test]
    fn known_string_bytes_do_not_erase_an_unproved_operand_materialisation() {
        let policy = FoldPolicy::default().with_invocation_dialect(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        let original = crate::expr_parser::parse_expr("$x eq \"hello\"", None);
        let environment = Env::from([("x".into(), EnvValue::Str("hello".into()))]);
        let analysis = analyse_tcl_expr_with_resolved_math_bindings(
            &original,
            &environment,
            policy,
            &|_, _| None,
            None,
        )
        .unwrap();
        assert_eq!(analysis.value, TclValue::Int(1));
        assert_eq!(
            analysis.coercions,
            vec![NativeCoercionObligation {
                reference: "$x".into(),
                start: Some(0),
                kind: NativeCoercionKind::String,
            }]
        );
        assert!(!analysis.native_value_effects_are_proved());
        assert_eq!(
            eval_tcl_expr_with_resolved_math_bindings(&original, &environment, policy, &|_, _| {
                None
            },),
            None
        );
    }

    #[test]
    fn retained_value_constants_do_not_prove_required_native_coercions() {
        let policy = FoldPolicy::default().with_invocation_dialect(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        let env = Env::from([("x".into(), EnvValue::Int(3))]);
        for source in ["$x", "$x+0", "!!$x", "$x==3", "abs($x)"] {
            let expression = crate::expr_parser::parse_expr(source, None);
            assert_eq!(
                eval_tcl_expr_with_resolved_math_bindings(
                    &expression,
                    &env,
                    policy,
                    &|function, _| {
                        Some(NativeMathFunctionTarget {
                            function: function.to_owned(),
                            prepended: Vec::new(),
                        })
                    },
                ),
                None,
                "a value constant cannot erase a retained-object conversion: {source}",
            );
        }
        let lazy = crate::expr_parser::parse_expr("1?abs(-3):$x+0", None);
        assert_eq!(
            eval_tcl_expr_with_resolved_math_bindings(&lazy, &env, policy, &|function, _| {
                Some(NativeMathFunctionTarget {
                    function: function.to_owned(),
                    prepended: Vec::new(),
                })
            }),
            Some(TclValue::Int(3)),
        );
    }

    #[test]
    fn already_native_object_proof_licenses_only_matching_numeric_reads() {
        let expression = crate::expr_parser::parse_expr("abs($x)", None);
        let policy = FoldPolicy::default().with_invocation_dialect(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        let env = Env::from([("x".into(), EnvValue::Int(-3))]);
        let mut operands = NativeOperandProofs::from([(
            "x".into(),
            RetainedNativeOperandProof {
                dialect: tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6,
                ),
                identity: NativeOperandObjectIdentity::Runtime {
                    interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                        owner: 12,
                        interpreter: 2,
                    },
                    object: 7,
                    generation: 4,
                },
                value: TclValue::Int(-3),
                existing_string: None,
            },
        )]);
        let calls = |function: &str, _| {
            Some(NativeMathFunctionTarget {
                function: function.to_owned(),
                prepended: Vec::new(),
            })
        };
        assert_eq!(
            eval_tcl_expr_with_proved_operands(&expression, &env, policy, &calls, &operands),
            Some(TclValue::Int(3))
        );
        operands.get_mut("x").unwrap().value = TclValue::Int(8);
        assert_eq!(
            eval_tcl_expr_with_proved_operands(&expression, &env, policy, &calls, &operands),
            None
        );
    }

    #[test]
    fn resolved_native_alias_uses_its_terminal_handler_and_frozen_prefix() {
        let expression = crate::expr_parser::parse_expr("chosen()", None);
        let policy = FoldPolicy::default().with_invocation_dialect(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        let folded = eval_tcl_expr_with_resolved_math_bindings(
            &expression,
            &Env::new(),
            policy,
            &|function, start| {
                (function == "chosen" && start == 0).then(|| NativeMathFunctionTarget {
                    function: "abs".to_owned(),
                    prepended: vec!["-3".to_owned()],
                })
            },
        );
        assert_eq!(folded, Some(TclValue::Int(3)));
        assert_eq!(
            eval_tcl_expr_with_resolved_math_bindings(&expression, &Env::new(), policy, &|_, _| {
                None
            }),
            None
        );
    }

    #[test]
    fn execution_folding_requires_the_reached_native_implementation() {
        let expression = crate::expr_parser::parse_expr("abs(-3)", None);
        let policy = FoldPolicy::default().with_invocation_dialect(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert_eq!(
            eval_tcl_expr_with_policy(&expression, &Env::new(), policy),
            None
        );
        assert_eq!(
            eval_tcl_expr_with_math_bindings(
                &expression,
                &Env::new(),
                policy,
                &|function, start| { function == "abs" && start == 0 }
            ),
            Some(TclValue::Int(3)),
        );
        assert_eq!(
            eval_tcl_expr_with_math_bindings(&expression, &Env::new(), policy, &|_, _| false),
            None,
        );
    }

    #[test]
    fn unproved_unreached_function_does_not_suppress_a_valid_lazy_fold() {
        let expression = crate::expr_parser::parse_expr("1 ? abs(-3) : abs(-7)", None);
        let reached = std::cell::RefCell::new(Vec::new());
        let folded = eval_tcl_expr_with_math_bindings(
            &expression,
            &Env::new(),
            FoldPolicy::default().with_invocation_dialect(
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
            ),
            &|function, start| {
                reached.borrow_mut().push((function.to_owned(), start));
                function == "abs" && start == 4
            },
        );
        assert_eq!(folded, Some(TclValue::Int(3)));
        assert_eq!(*reached.borrow(), [("abs".to_owned(), 4)]);
    }
}
