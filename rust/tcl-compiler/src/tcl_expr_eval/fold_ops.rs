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

//! The constant folder's `ExprOps`: fold values, the numeric tower, operand classification and the operator callbacks.

use super::arithmetic::{apply_binary, apply_irules_string_op, numeric_cmp, split_tcl_list};
use super::fold_policy::{FoldPolicy, math_func_ceiling_for_dialect, widens_past_a_wide};
use super::numerals::{parse_literal_in, strict_number, strict_number_for_dialect};
use super::{
    ConstantCompilation, Env, EnvValue, FoldNativeInputs, FoldOps, IntrinsicMathQuery,
    NativeCoercionKind, NativeCoercionObligation, ResolvedMathQuery, RetainedNativeOperandProof,
    TclValue, past_the_wide_tower,
};
use crate::expr_ast::{BinOp, UnaryOp};
use std::cell::RefCell;
use tcl_dialect::{NumberSyntax, StringCharacterModel};
use tcl_registry::value_transfer::{Axis, DeclineReason};

pub(super) fn make_fold_ops<'a>(
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
        declined: None,
        tower: if widens_past_a_wide(policy.dialect)
            || policy
                .arithmetic
                .is_some_and(|arithmetic| arithmetic != tcl_dialect::NativeArithmetic::Tcl84Wide)
        {
            Tower::Widens
        } else {
            Tower::Wide
        },
        octal: policy.octal,
        numbers: policy.numbers.unwrap_or_else(|| {
            if policy.octal == Some(true) {
                NumberSyntax::Tcl85
            } else {
                NumberSyntax::default()
            }
        }),
        spelling: policy.spelling(),
        math_since: policy.dialect.and_then(math_func_ceiling_for_dialect),
        f5_predicates: policy.f5_predicates,
        word_rules: policy.word_rules,
        native_family: policy.native_family,
        constant_compilation: ConstantCompilation::default(),
    }
}

// FoldOps — the const-folder's value ops for the shared expr walk

/// A const-fold value. `Str` keeps the operand's **raw text** and is parsed
/// lazily per context (numeric ops via [`parse_literal`]; string ops use it
/// verbatim) — exactly the `eval`-vs-`eval_as_string` split, so the raw-text
/// string-compare behaviour (`5.00 eq 5.0` → 0) is preserved.
#[derive(Clone)]
pub(crate) enum FoldValue {
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
    pub(super) fn to_number(&self, numbers: NumberSyntax) -> Option<TclValue> {
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
    pub(super) fn to_string_val(
        &self,
        format: Option<tcl_dialect::DoubleFormat>,
    ) -> Option<String> {
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

/// The integer and floating-point tower a constant fold keeps to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tower {
    /// The target widens past a wide integer and reads an infinity as a
    /// value: from 8.5, or no dialect named.
    Widens,
    /// It does not — an 8.4 runtime, or a profile naming no release — so an
    /// operand or result past a wide declines.
    Wide,
    /// As [`Self::Wide`], and a comparison met such an operand: the fold
    /// declines once the walk is done.
    Breached,
}

/// The const-folder's [`ExprOps`](tcl_syntax::expr::ExprOps). `Error = ()` is the
/// "can't fold" signal (mapped to the public `Option`); `$var` resolves from the
/// `env`, `[cmd]`/`Raw` are opaque.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FoldPurpose {
    Executable,
    AnalysisValue,
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

    pub(super) fn normalize_number(&self, value: TclValue) -> Option<TclValue> {
        match value {
            TclValue::Big(value) if self.arithmetic != tcl_dialect::NativeArithmetic::TclBignum => {
                tcl_syntax::expr::wide::literal(self.arithmetic, &value)
                    .ok()
                    .map(TclValue::Int)
            }
            value => Some(value),
        }
    }

    pub(super) fn number_for(
        &self,
        value: &FoldValue,
        kind: NativeCoercionKind,
    ) -> Option<TclValue> {
        let value = self.operand_for(value, kind);
        self.normalize_number(value.to_number(self.numbers)?)
    }

    pub(super) fn strict_number(&self, value: &FoldValue) -> Option<TclValue> {
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
pub(super) enum Operand {
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
pub(super) fn classify_operand(
    value: &FoldValue,
    octal: Option<bool>,
    numbers: NumberSyntax,
) -> Operand {
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
pub(super) fn is_bare_leading_zero(s: &str) -> bool {
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    digits.len() > 1 && digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit())
}

/// Parse a Tcl 8.x octal integer (`010` → 8, `-077` → -63). Returns `None`
/// for an invalid octal (`08`/`09`), which Tcl 8.x treats as a string.
pub(super) fn parse_octal_literal(s: &str) -> Option<TclValue> {
    let (neg, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let v = i64::from_str_radix(digits, 8).ok()?;
    Some(TclValue::Int(if neg { -v } else { v }))
}

impl FoldOps<'_> {
    /// `value` as a string operation reads it: a string's own text, a
    /// number's string rep under the target's spelling
    /// ([`FoldValue::to_string_val`] under `format`). A number the spelling
    /// cannot give — a double under a precision the program may set, an
    /// `INT64_MIN` the release prints as `-8`
    /// ([`NumberSpelling::garbles_wide_min`]) — declines the fold
    /// (`Unspellable`).
    pub(super) fn text_of(
        &mut self,
        value: &FoldValue,
        format: Option<tcl_dialect::DoubleFormat>,
    ) -> Option<String> {
        let spelled = match value {
            FoldValue::Int(i) => tcl_syntax::number::spell_wide(*i, self.spelling),
            // A program that may set the precision leaves a computed double
            // no spelling known here ([`NumberSpelling::doubles`]).
            FoldValue::Float(_) if self.spelling.doubles().is_none() => None,
            value => value.to_string_val(format),
        };
        if spelled.is_none() && matches!(value, FoldValue::Int(_) | FoldValue::Float(_)) {
            self.decline(DeclineReason::Unspellable);
        }
        spelled
    }

    /// Record why the fold declines; the first reason stands.
    pub(super) fn decline(&mut self, reason: DeclineReason) {
        self.declined.get_or_insert(reason);
    }

    /// Whether `value` is past a tower the target does not widen to
    /// ([`Self::tower`]).
    pub(super) fn beyond_tower(&self, value: &FoldValue) -> bool {
        self.tower != Tower::Widens
            && past_the_wide_tower(value, self.octal, self.numbers)
            && !self.wide_model_reads(value)
    }

    /// A beyond-wide integer the selected fixed-width model still reads: an
    /// 8.4 literal up to 64 bits keeps its low bits.
    fn wide_model_reads(&self, value: &FoldValue) -> bool {
        self.arithmetic != tcl_dialect::NativeArithmetic::TclBignum
            && matches!(
                strict_number_for_dialect(value, self.octal, self.numbers),
                Some(TclValue::Big(big))
                    if tcl_syntax::expr::wide::literal(self.arithmetic, &big).is_ok()
            )
    }

    /// An operand or result past a tower the target does not widen to folds
    /// nothing: tclsh 8.4 raises `floating-point value too large to
    /// represent` for `1e308 * 10` and `integer value too large to represent`
    /// for `18446744073709551616`.
    fn within_tower(&self, values: &[&FoldValue]) -> Result<(), ()> {
        if values.iter().any(|value| self.beyond_tower(value)) {
            return Err(());
        }
        Ok(())
    }

    /// Whether `left % right` is the wide remainder `i64::MIN % -1`: `0` from
    /// 8.5, but a hardware divide trap that kills tclsh 8.4 and jimsh
    /// (`SIGFPE`, measured), so a target that does not widen declines it
    /// rather than fold a value the program never reaches.
    pub(super) fn is_trapping_wide_remainder(
        &self,
        op: BinOp,
        left: &FoldValue,
        right: &FoldValue,
    ) -> bool {
        let wide =
            |value: &FoldValue| match strict_number_for_dialect(value, self.octal, self.numbers) {
                Some(TclValue::Int(i)) => Some(i),
                _ => None,
            };
        op == BinOp::Mod && wide(left) == Some(i64::MIN) && wide(right) == Some(-1)
    }

    /// Run the math function `name` — spelled as the program calls it —
    /// over `args` through the shared dispatcher, the one the runtime
    /// evaluates with; `Err` when it declines.
    pub(super) fn math_call(&self, name: &str, args: &[FoldValue]) -> Result<FoldValue, ()> {
        use tcl_syntax::expr::mathfunc::{
            IntWidth, NativeMathProtocol, Num, accepts_boolean_operand, dispatch_under,
            try_dispatch_with_backend_protocol,
        };
        // Map `TclValue` → `Num` → result. Every function except `bool`
        // reads its operand as a strict number — `Tcl_GetBoolean` coercion
        // (`true`→1) would let the folder turn an error (`abs(true)`) into
        // a value, so parse strictly unless the function itself accepts
        // boolean words (the registry of that fact is the mathfunc module,
        // not a name check here).
        let boolean_ok = accepts_boolean_operand(name);
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
        let nums = nums.ok_or(())?;
        // The release's own semantics: Jim's `pow` is its `**` operator
        // ([`dispatch_under`]), which the Tcl table does not compute. The
        // Jim protocol computes it natively; the Tcl table declines it under
        // a grammar whose integer `pow` is an integer.
        if protocol == NativeMathProtocol::Tcl
            && dispatch_under(name, &nums, self.spelling.numbers()).is_none()
        {
            return Err(());
        }
        match try_dispatch_with_backend_protocol(
            name,
            &nums,
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
            IntWidth, NativeMathProtocol, Num, accepts_boolean_operand, added_in, dispatch_under,
            try_dispatch_with_backend_protocol,
        };
        let name = function.to_owned();
        self.within_tower(&args.iter().collect::<Vec<_>>())?;
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
        let nums = nums.ok_or(())?;
        // The release's own semantics: Jim's `pow` is its `**` operator
        // ([`dispatch_under`]), which the Tcl table does not compute. The
        // Jim protocol computes it natively; the Tcl table declines it under
        // a grammar whose integer `pow` is an integer.
        if protocol == NativeMathProtocol::Tcl
            && dispatch_under(&name, &nums, self.spelling.numbers()).is_none()
        {
            return Err(());
        }
        match try_dispatch_with_backend_protocol(
            &name,
            &nums,
            IntWidth::for_native_arithmetic(self.arithmetic),
            protocol,
        )
        .map_err(|_| ())?
        {
            Num::Int(i) => Ok(FoldValue::Int(i)),
            Num::Float(f) => {
                let value = FoldValue::Float(f);
                self.within_tower(&[&value])?;
                Ok(value)
            }
            Num::Big(never) => match never {},
        }
    }

    fn arith(&mut self, op: BinOp, left: FoldValue, right: FoldValue) -> Result<FoldValue, ()> {
        self.check_native_constant_arithmetic(op, &left, &right)?;
        self.within_tower(&[&left, &right])?;
        if self.tower != Tower::Widens && self.is_trapping_wide_remainder(op, &left, &right) {
            return Err(());
        }
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
        let value = apply_binary(op, a, b).map(FoldValue::from_tcl).ok_or(())?;
        self.within_tower(&[&value])?;
        Ok(value)
    }
    fn unary(&mut self, op: UnaryOp, value: FoldValue) -> Result<FoldValue, ()> {
        self.check_native_constant_unary(op, &value)?;
        self.within_tower(&[&value])?;
        if self.arithmetic != tcl_dialect::NativeArithmetic::TclBignum
            && matches!(op, UnaryOp::Pos | UnaryOp::Neg | UnaryOp::BitNot)
            && let TclValue::Int(value) = self.strict_number(&value).ok_or(())?
        {
            return tcl_syntax::expr::wide::unary(self.arithmetic, op, value)
                .map(FoldValue::Int)
                .map_err(|_| ());
        }
        let result = match op {
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
        }?;
        self.within_tower(&[&result])?;
        Ok(result)
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
        if self.beyond_tower(left) || self.beyond_tower(right) {
            self.tower = Tower::Breached;
            return None;
        }
        let (lo, ro) = (self.classify(left), self.classify(right));
        if matches!(lo, Operand::Ambiguous) || matches!(ro, Operand::Ambiguous) {
            self.decline(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar));
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
                    self.decline(DeclineReason::ReleaseAmbiguous(Axis::Platform));
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
            let left = tcl_syntax::raw_string::RawString::from_unicode(self.text_of(left, format)?);
            let right =
                tcl_syntax::raw_string::RawString::from_unicode(self.text_of(right, format)?);
            left.compare_character_units(model, &right, (0, 0)).ok()
        })();
        result.ok_or_else(|| {
            self.decline(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar));
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
        let left = self.text_of(left, format).ok_or(())?;
        let right = self.text_of(right, format).ok_or(())?;
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
        let n = self.text_of(needle, format).ok_or(())?;
        let list = self.text_of(list, format).ok_or(())?;
        Ok(split_tcl_list(&list, self.word_rules).contains(&n))
    }

    fn to_bool(&mut self, value: &FoldValue) -> Result<bool, ()> {
        self.check_native_constant_boolean(value)?;
        self.within_tower(&[value])?;
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
    fn to_bool_for_purpose(
        &mut self,
        value: &FoldValue,
        purpose: tcl_syntax::native_boolean_truth::NativeBooleanTruthPurpose,
    ) -> Result<bool, ()> {
        if purpose.requires_expression_result()
            || (self.purpose == FoldPurpose::Executable
                && matches!(value, FoldValue::RetainedNativeObject { .. }))
        {
            return Err(());
        }
        // Mathematical values are explicit here. A retained executable object
        // needs its independently reached native stage, not this value result.
        self.to_bool(value)
    }

    fn logical_right_truth(
        &mut self,
        left: &FoldValue,
        right: &FoldValue,
        _conjunction: bool,
    ) -> Result<bool, ()> {
        if self.purpose == FoldPurpose::Executable
            && [left, right]
                .into_iter()
                .any(|value| matches!(value, FoldValue::RetainedNativeObject { .. }))
        {
            return Err(());
        }
        self.to_bool(right)
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
            &self.text_of(left, format).ok_or(())?,
            &self.text_of(right, format).ok_or(())?,
        )
        .map(FoldValue::from_tcl)
        .ok_or(())
    }
}
