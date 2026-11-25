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

//! `ExprServices`: the shared engine's `ExprOps` over the analysis inputs, and `evaluate_expression`.

use super::fold_ops::{FoldValue, Operand, Tower, classify_operand, make_fold_ops};
use super::fold_policy::FoldPolicy;
use super::numerals::strict_number_for_dialect;
use super::{Env, FoldNativeInputs, FoldOps, TclValue, operators_in_release, past_the_wide_tower};
use crate::expr_ast::{BinOp, ExprNode, UnaryOp};
use tcl_registry::value_transfer::{
    AnalysisInputs, Axis, Budget, BudgetLimit, CompletionOutcome, DeclineReason, EvalAnswer,
    EvaluationState, ExactValue, ExactValueOrUnavailable, FactDomain, NumericValue,
    RepresentationEvidence, WrittenPlace,
};

// The engine adapter: the shared walk over the analysis services

/// Whether the math function `name` reads or seeds the interpreter's random
/// generator, whose state no evaluation can know.
fn reads_the_generator(name: &str) -> bool {
    // value-transfer-ok: irreducible — `rand` and `srand` read and seed the
    // interpreter's generator, which no evaluation can know.
    matches!(name, "rand" | "srand")
}

/// Why an evaluation under the analysis services stopped short of a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExprStop {
    /// An input the solver has not reached yet.
    Pending,
    /// No value, for the recorded reason.
    Declined(DeclineReason),
    /// A nested invocation did not complete normally: the evaluation ends
    /// with that completion and the writes made so far.
    Ended,
}

impl ExprStop {
    /// The stop an input service's answer stands for.
    fn of_answer(answer: &EvalAnswer) -> Self {
        match answer {
            EvalAnswer::Pending => Self::Pending,
            EvalAnswer::Declined(reason) => Self::Declined(*reason),
            // A service that answers an outcome where a value was asked for
            // is malformed.
            EvalAnswer::Evaluated(_) => Self::Declined(DeclineReason::Unsupported),
        }
    }
}

/// The shared engine's answer for one expression under the analysis
/// services.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprAnswer {
    /// The expression's full value: the number Tcl normalises a numeric
    /// result to, or the string result itself.
    Value(ExactValue),
    /// An input has not reached a usable fact.
    Pending,
    /// No value, for the recorded reason.
    Declined(DeclineReason),
    /// The evaluation ended on a nested invocation that did not complete
    /// normally, with that completion: no value, and the writes the state
    /// holds are the ones that ran before it.
    Ended(Box<CompletionOutcome>),
}

/// The environment the services' value model never reads: every `$name`
/// goes through [`AnalysisInputs::variable`].
static NO_ENV: std::sync::LazyLock<Env> = std::sync::LazyLock::new(Env::new);

/// The engine's `ExprOps` over the analysis inputs: `var` reads through
/// `variable`, `command` through `nested`, `call` through `math_function`
/// and the shared dispatcher, and a quoted operand is substituted as
/// `expr` substitutes it. The value semantics — the numeral grammar, the
/// comparisons, the arithmetic — are the const-folder's ([`FoldOps`]), so
/// an expression answers here what it answers there; what differs is where
/// an operand's value comes from, and that a stop says why.
pub(crate) struct ExprServices<'a> {
    inputs: &'a dyn AnalysisInputs,
    state: &'a mut EvaluationState,
    budget: &'a mut Budget,
    fold: FoldOps<'static>,
    /// Whether the target widens an integer past a wide and reads `Inf`:
    /// `Some(true)` from 8.5 (and for a caller that names no dialect, which
    /// reads as 9.0), `Some(false)` for a 8.4 runtime, `None` for a profile
    /// whose runtime names no release.
    widens: Option<bool>,
    /// The first stop an infallible hook raised (a comparison over an
    /// operand the target's integer tower cannot read).
    stopped: Option<DeclineReason>,
    /// The completion a nested invocation ended the evaluation with.
    ended: Option<CompletionOutcome>,
    /// The document's lexer configuration, for a quoted operand's
    /// substitution.
    lexer: tcl_lexer::LexerConfig,
}

impl<'a> ExprServices<'a> {
    /// Services over `inputs`, recording nested evidence in `state` and
    /// charging `budget`.
    pub(crate) fn new(
        inputs: &'a dyn AnalysisInputs,
        state: &'a mut EvaluationState,
        budget: &'a mut Budget,
    ) -> Self {
        let lexer = inputs
            .source_lexer_config()
            .unwrap_or_else(|| tcl_lexer::LexerConfig::from_grammar(inputs.context().grammar));
        Self {
            inputs,
            state,
            budget,
            fold: FoldOps::for_services(FoldPolicy::default()),
            widens: Some(true),
            stopped: None,
            ended: None,
            lexer,
        }
    }

    fn nested_result(&mut self, answer: EvalAnswer) -> Result<FoldValue, ExprStop> {
        let outcome = match answer {
            EvalAnswer::Evaluated(outcome) => outcome,
            answer => return Err(ExprStop::of_answer(&answer)),
        };
        if outcome.completion != CompletionOutcome::Normal {
            self.ended = Some(outcome.completion);
            return Err(ExprStop::Ended);
        }
        match &outcome.result {
            ExactValueOrUnavailable::Exact(value) => Self::operand_of(value),
            ExactValueOrUnavailable::Unavailable(_) => {
                Err(ExprStop::Declined(DeclineReason::NotExact))
            }
        }
    }

    fn string_parts(
        &mut self,
        inner: &str,
        substitutes: bool,
        inner_start: Option<u32>,
    ) -> Result<FoldValue, ExprStop> {
        use tcl_lexer::word_parts::{SubstFlags, WordPart as Part, decompose_spanned_checked};
        if !substitutes {
            return tcl_syntax::expr::fixed_string_body(inner, false)
                .map(FoldValue::text)
                .ok_or(ExprStop::Declined(DeclineReason::NotExact));
        }
        let parts = decompose_spanned_checked(inner.as_bytes(), SubstFlags::default(), self.lexer)
            .map_err(|_| ExprStop::Declined(DeclineReason::Unsupported))?;
        let mut bytes = Vec::with_capacity(inner.len());
        let format = self
            .fold
            .invocation_dialect
            .and_then(tcl_registry::InvocationDialect::double_string_policy)
            .and_then(tcl_dialect::DoubleStringPolicy::constant_format);
        for component in parts {
            match component.part {
                Part::Text(text) => bytes.extend_from_slice(&text),
                Part::Variable(reference) => {
                    let name = crate::value_transfer::variable_name(&reference)
                        .map_err(ExprStop::Declined)?;
                    let value = tcl_syntax::expr::ExprOps::var(self, &name)?;
                    bytes.extend_from_slice(
                        self.fold
                            .text_of(&value, format)
                            .ok_or_else(|| self.fold_refused())?
                            .as_bytes(),
                    );
                }
                Part::Command(script) => {
                    let value = if let Some(base) = inner_start {
                        let start = u32::try_from(component.start)
                            .ok()
                            .and_then(|offset| base.checked_add(offset))
                            .ok_or(ExprStop::Declined(DeclineReason::NotExact))?;
                        let end = u32::try_from(component.end)
                            .ok()
                            .and_then(|offset| base.checked_add(offset))
                            .ok_or(ExprStop::Declined(DeclineReason::NotExact))?;
                        tcl_syntax::expr::ExprOps::command_bytes_at(
                            self,
                            script,
                            start,
                            end.checked_sub(1)
                                .ok_or(ExprStop::Declined(DeclineReason::NotExact))?,
                        )?
                    } else {
                        let script = std::str::from_utf8(script)
                            .map_err(|_| ExprStop::Declined(DeclineReason::NotText))?;
                        tcl_syntax::expr::ExprOps::command(self, script)?
                    };
                    bytes.extend_from_slice(
                        self.fold
                            .text_of(&value, format)
                            .ok_or_else(|| self.fold_refused())?
                            .as_bytes(),
                    );
                }
                Part::Expression(_) => return Err(ExprStop::Declined(DeclineReason::Unsupported)),
                Part::ParseError(_) => {
                    return Err(ExprStop::Declined(DeclineReason::WrongRepresentation));
                }
            }
        }
        String::from_utf8(bytes)
            .map(FoldValue::text)
            .map_err(|_| ExprStop::Declined(DeclineReason::NotText))
    }

    /// Take the value semantics `policy` states.
    fn configure(&mut self, policy: FoldPolicy) {
        self.fold = FoldOps::for_services(policy);
        self.fold.spelling = self
            .fold
            .spelling
            .with_precision_written(self.inputs.context().precision_written);
        self.widens = policy.dialect.map_or(Some(true), |profile| {
            profile
                .runtime_base
                .map(|release| release >= tcl_dialect::TclVersion::V8_5)
        });
        self.stopped = None;
    }

    /// A cancellation point: every service call is one.
    fn checkpoint(&self) -> Result<(), ExprStop> {
        if self.budget.is_cancelled() {
            return Err(ExprStop::Declined(DeclineReason::Budget(
                BudgetLimit::Cancelled,
            )));
        }
        Ok(())
    }

    /// Why a beyond-wide integer or an infinity has no value here: 8.4
    /// computes something else (it wraps, or raises on `Inf`), and a
    /// profile that names no release cannot say which.
    fn tower_reason(&self) -> DeclineReason {
        match self.widens {
            Some(_) => DeclineReason::WrongRepresentation,
            None => DeclineReason::ReleaseAmbiguous(Axis::IntTower),
        }
    }

    /// `Err` when an operand or result is a beyond-wide integer or an
    /// infinity the target does not widen to — an intermediate one too:
    /// `expr {(1e308 * 10) > 0}` is 1 from 8.5, and tclsh 8.4 raises
    /// `floating-point value too large to represent` at the product.
    pub(super) fn tower(&self, values: &[&FoldValue]) -> Result<(), ExprStop> {
        if self.widens != Some(true)
            && values
                .iter()
                .any(|value| past_the_wide_tower(value, self.fold.octal, self.fold.numbers))
        {
            return Err(ExprStop::Declined(self.tower_reason()));
        }
        Ok(())
    }

    /// An exact input as an engine operand: its numeric classification when
    /// it has one, else its text.
    fn operand_of(value: &ExactValue) -> Result<FoldValue, ExprStop> {
        match value.numeric {
            Some(NumericValue::Int(i)) => Ok(FoldValue::Int(i)),
            Some(NumericValue::Float(f)) => Ok(FoldValue::Float(f)),
            Some(NumericValue::Bool(b)) => Ok(FoldValue::Int(i64::from(b))),
            None => value
                .as_str()
                .map(FoldValue::text)
                .map_err(ExprStop::Declined),
        }
    }

    /// Whether `left / right` and `left % right` are integer operations
    /// over a zero divisor: the one arithmetic failure whose completion every
    /// release words alike. A float operand makes the operation floating
    /// point (`1 / 0.0` is `Inf`), and an operand that reads as no integer
    /// raises another error the engine does not word.
    fn divides_integers_by_zero(&self, left: &FoldValue, right: &FoldValue) -> bool {
        let integer = |value: &FoldValue| {
            strict_number_for_dialect(value, self.fold.octal, self.fold.numbers)
        };
        matches!(
            (integer(left), integer(right)),
            (
                Some(TclValue::Int(_) | TclValue::Big(_)),
                Some(TclValue::Int(0))
            )
        )
    }

    /// The error a division by an integer zero raises: `divide by zero`,
    /// `-errorcode` `ARITH DIVZERO {divide by zero}`, in every release
    /// (tclsh 8.4 to 9.1), raised before any write the expression's own
    /// commands have not already made.
    fn divide_by_zero(&mut self) -> ExprStop {
        self.ended = Some(CompletionOutcome::Error {
            written: self.state.writes.len(),
            message: ExactValueOrUnavailable::exact_text("divide by zero"),
            error_code: ExactValueOrUnavailable::exact_text("ARITH DIVZERO {divide by zero}"),
        });
        ExprStop::Ended
    }

    /// The expression's full value, as `expr` returns it: a string result
    /// that reads as a number under the target's grammar is that number's
    /// canonical form (`expr {"0x10"}` is 16, `expr {" 5 "}` is 5), and any
    /// other string is the result (`expr {"x"}` is `x`, `expr {"true"}` is
    /// `true`).
    fn full_value(&self, value: &FoldValue) -> Result<ExactValue, DeclineReason> {
        if self.widens != Some(true)
            && matches!(value, FoldValue::Str(text)
                if tcl_syntax::number::is_out_of_range_double(&text.bytes, self.fold.numbers))
        {
            return Err(self.tower_reason());
        }
        let number = match value {
            FoldValue::Str(text) => {
                match classify_operand(value, self.fold.octal, self.fold.numbers) {
                    Operand::Num(number) => number,
                    Operand::Str => return Ok(text.clone().into_byte_value()),
                    Operand::Ambiguous => {
                        return Err(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar));
                    }
                }
            }
            FoldValue::Int(i) => TclValue::Int(*i),
            FoldValue::Big(b) => TclValue::from_big(b.clone()),
            FoldValue::Float(f) => TclValue::Float(*f),
            FoldValue::RetainedNativeObject { .. } => {
                return Err(DeclineReason::WrongRepresentation);
            }
        };
        match number {
            // The release's own string rep, or none where it would not be
            // the value's: a computed `INT64_MIN` is `-8` on 8.4 and 8.5.
            TclValue::Int(i) => tcl_syntax::number::spell_wide(i, self.fold.spelling)
                .map(|_| ExactValue::int(i))
                .ok_or(DeclineReason::Unspellable),
            TclValue::Big(_) if self.widens != Some(true) => Err(self.tower_reason()),
            // A beyond-wide integer is its canonical decimal spelling, which
            // a later fold re-reads exactly.
            TclValue::Big(b) => Ok(ExactValue::text(b.to_string())),
            // `expr` raises on a NaN result: a domain error, never a value.
            TclValue::Float(f) if f.is_nan() => Err(DeclineReason::WrongRepresentation),
            TclValue::Float(f) if f.is_infinite() && self.widens != Some(true) => {
                Err(self.tower_reason())
            }
            // The release's spelling, when it reads back as the value: 2^64
            // is `1.844674407370955e+19` from 8.5, which reads back as
            // 2^64 - 2048, and `1.0/3` is `0.333333333333` under 8.4 and Jim.
            TclValue::Float(f) => tcl_syntax::number::spell_double(f, self.fold.spelling)
                .map(|text| ExactValue {
                    bytes: text.into_bytes(),
                    numeric: Some(NumericValue::Float(f)),
                    representation: RepresentationEvidence::Unknown,
                })
                .ok_or(DeclineReason::Unspellable),
        }
    }
}

impl FoldOps<'static> {
    /// The value semantics the analysis services evaluate under: `policy`'s
    /// numeral grammar and operator set, reading no environment. The math
    /// functions' availability is the `math_function` service's.
    fn for_services(policy: FoldPolicy) -> Self {
        let mut fold = make_fold_ops(
            &NO_ENV,
            policy.with_intrinsic_math(),
            None,
            None,
            FoldNativeInputs::objects(None),
            true,
        );
        fold.tower = Tower::Widens;
        fold.math_since = None;
        fold
    }
}

/// A fold operation's refusal: the program raises (a non-numeric operand,
/// division by zero, a domain error) or the model does not compute it, and
/// either way an error is never a value.
pub(super) const fn refused(_: ()) -> ExprStop {
    ExprStop::Declined(DeclineReason::WrongRepresentation)
}

impl ExprServices<'_> {
    /// A string operation's refusal: the reason the fold recorded
    /// ([`FoldOps::decline`], such as `Unspellable`), else [`refused`]'s.
    fn fold_refused(&self) -> ExprStop {
        ExprStop::Declined(
            self.fold
                .declined
                .unwrap_or(DeclineReason::WrongRepresentation),
        )
    }
}

impl tcl_syntax::expr::ExprOps for ExprServices<'_> {
    type Value = FoldValue;
    type Error = ExprStop;

    fn literal(&mut self, text: &str) -> Result<FoldValue, ExprStop> {
        Ok(FoldValue::text(text))
    }

    /// An unpositioned string retains the explicit compatibility callback.
    fn string(&mut self, inner: &str, substitutes: bool) -> Result<FoldValue, ExprStop> {
        self.string_parts(inner, substitutes, None)
    }

    fn string_bytes_at(
        &mut self,
        inner: &[u8],
        substitutes: bool,
        start: u32,
        end: u32,
    ) -> Result<FoldValue, ExprStop> {
        let inner =
            std::str::from_utf8(inner).map_err(|_| ExprStop::Declined(DeclineReason::NotText))?;
        let inner_start = start
            .checked_add(1)
            .ok_or(ExprStop::Declined(DeclineReason::NotExact))?;
        if inner_start.checked_add(
            u32::try_from(inner.len()).map_err(|_| ExprStop::Declined(DeclineReason::NotExact))?,
        ) != Some(end)
        {
            return Err(ExprStop::Declined(DeclineReason::NotExact));
        }
        self.string_parts(inner, substitutes, Some(inner_start))
    }

    /// A read consults the state's own writes before the inputs at the
    /// program point: after `[incr x]` the next `$x` is the incremented
    /// value.
    fn var(&mut self, name: &str) -> Result<FoldValue, ExprStop> {
        self.checkpoint()?;
        let value = match self.state.written(name) {
            WrittenPlace::Exact(value) => value,
            WrittenPlace::Unknown => return Err(ExprStop::Declined(DeclineReason::NotExact)),
            WrittenPlace::Untouched => self
                .inputs
                .variable(name, FactDomain::ExactValue)
                .exact()
                .map_err(|answer| ExprStop::of_answer(&answer))?,
        };
        Self::operand_of(&value)
    }

    fn command(&mut self, script: &str) -> Result<FoldValue, ExprStop> {
        self.checkpoint()?;
        let answer = self.inputs.nested(script, self.state);
        self.nested_result(answer)
    }

    fn command_bytes_at(
        &mut self,
        script: &[u8],
        start: u32,
        end: u32,
    ) -> Result<FoldValue, ExprStop> {
        self.checkpoint()?;
        let script =
            std::str::from_utf8(script).map_err(|_| ExprStop::Declined(DeclineReason::NotText))?;
        let answer = self
            .inputs
            .nested_expression_at(script, start, end, self.state);
        self.nested_result(answer)
    }

    fn call(&mut self, function: &str, args: Vec<FoldValue>) -> Result<FoldValue, ExprStop> {
        self.checkpoint()?;
        if reads_the_generator(function) {
            return Err(ExprStop::Declined(DeclineReason::Unsupported));
        }
        let binding = self
            .inputs
            .math_function(function)
            .map_err(ExprStop::Declined)?;
        if !self.state.evidence.bindings.contains(&binding) {
            self.state.evidence.bindings.push(binding);
        }
        let operands: Vec<&FoldValue> = args.iter().collect();
        self.tower(&operands)?;
        let value = self.fold.math_call(function, &args).map_err(refused)?;
        self.tower(&[&value])?;
        Ok(value)
    }

    fn arith(
        &mut self,
        op: BinOp,
        left: FoldValue,
        right: FoldValue,
    ) -> Result<FoldValue, ExprStop> {
        self.tower(&[&left, &right])?;
        if self.widens != Some(true) && self.fold.is_trapping_wide_remainder(op, &left, &right) {
            return Err(ExprStop::Declined(self.tower_reason()));
        }
        if matches!(op, BinOp::Div | BinOp::Mod) && self.divides_integers_by_zero(&left, &right) {
            return Err(self.divide_by_zero());
        }
        let value = self.fold.arith(op, left, right).map_err(refused)?;
        self.tower(&[&value])?;
        Ok(value)
    }

    fn unary(&mut self, op: UnaryOp, value: FoldValue) -> Result<FoldValue, ExprStop> {
        self.tower(&[&value])?;
        let value = self.fold.unary(op, value).map_err(refused)?;
        self.tower(&[&value])?;
        Ok(value)
    }

    fn binary_other(
        &mut self,
        op: BinOp,
        left: FoldValue,
        right: FoldValue,
    ) -> Result<FoldValue, ExprStop> {
        self.fold
            .binary_other(op, left, right)
            .map_err(|()| ExprStop::Declined(DeclineReason::Unsupported))
    }

    fn compare_numeric(
        &mut self,
        left: &FoldValue,
        right: &FoldValue,
    ) -> Option<tcl_syntax::expr::NumericCompare> {
        if self.stopped.is_none()
            && let Err(ExprStop::Declined(reason)) = self.tower(&[left, right])
        {
            self.stopped = Some(reason);
        }
        self.fold.compare_numeric(left, right)
    }

    fn compare_string(
        &mut self,
        left: &FoldValue,
        right: &FoldValue,
    ) -> Result<std::cmp::Ordering, ExprStop> {
        self.fold
            .compare_string(left, right)
            .map_err(|()| self.fold_refused())
    }

    fn equal_string(&mut self, left: &FoldValue, right: &FoldValue) -> Result<bool, ExprStop> {
        self.fold
            .equal_string(left, right)
            .map_err(|()| self.fold_refused())
    }

    fn in_list(&mut self, needle: &FoldValue, list: &FoldValue) -> Result<bool, ExprStop> {
        self.fold
            .in_list(needle, list)
            .map_err(|()| self.fold_refused())
    }

    fn to_bool(&mut self, value: &FoldValue) -> Result<bool, ExprStop> {
        self.tower(&[value])?;
        self.fold.to_bool(value).map_err(refused)
    }

    fn to_bool_for_purpose(
        &mut self,
        value: &FoldValue,
        purpose: tcl_syntax::native_boolean_truth::NativeBooleanTruthPurpose,
    ) -> Result<bool, ExprStop> {
        self.tower(&[value])?;
        self.fold
            .to_bool_for_purpose(value, purpose)
            .map_err(refused)
    }

    fn logical_right_truth(
        &mut self,
        left: &FoldValue,
        right: &FoldValue,
        conjunction: bool,
    ) -> Result<bool, ExprStop> {
        self.tower(&[left, right])?;
        self.fold
            .logical_right_truth(left, right, conjunction)
            .map_err(refused)
    }

    fn bool_value(&mut self, b: bool) -> FoldValue {
        FoldValue::Int(i64::from(b))
    }

    fn unsupported(&mut self, what: &str) -> ExprStop {
        // An unparsed expression is the program's syntax error; any other
        // unsupported construct is one this model does not evaluate.
        ExprStop::Declined(if what == "syntax error in expression" {
            DeclineReason::WrongRepresentation
        } else {
            DeclineReason::Unsupported
        })
    }
}

/// The nodes of `node`, the work its evaluation is charged before it runs.
fn node_count(node: &ExprNode) -> u64 {
    1 + match node {
        ExprNode::Unary { operand, .. } => node_count(operand),
        ExprNode::Binary { left, right, .. } => node_count(left) + node_count(right),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => node_count(condition) + node_count(true_branch) + node_count(false_branch),
        ExprNode::Call { args, .. } => args.iter().map(node_count).sum(),
        ExprNode::Literal { .. }
        | ExprNode::String { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Var { .. }
        | ExprNode::Command { .. }
        | ExprNode::Raw { .. } => 0,
    }
}

/// Evaluate `node` under the analysis services and `policy`'s value
/// semantics: the engine's full value, or why there is none. One work unit
/// per node is charged before the walk; each service call is a
/// cancellation point.
pub(crate) fn evaluate_expression(
    node: &ExprNode,
    services: &mut ExprServices<'_>,
    policy: FoldPolicy,
) -> ExprAnswer {
    services.configure(policy);
    if !operators_in_release(node, policy) {
        return ExprAnswer::Declined(DeclineReason::WrongRepresentation);
    }
    if let Err(reason) = services.budget.charge_work(node_count(node)) {
        return ExprAnswer::Declined(reason);
    }
    let value = match tcl_syntax::expr::eval(node, services) {
        Ok(value) => value,
        Err(ExprStop::Pending) => return ExprAnswer::Pending,
        Err(ExprStop::Declined(reason)) => return ExprAnswer::Declined(reason),
        Err(ExprStop::Ended) => {
            return services.ended.take().map_or(
                ExprAnswer::Declined(DeclineReason::Unsupported),
                |completion| ExprAnswer::Ended(Box::new(completion)),
            );
        }
    };
    if let Some(reason) = services.stopped {
        return ExprAnswer::Declined(reason);
    }
    if let Some(reason) = services.fold.declined {
        return ExprAnswer::Declined(reason);
    }
    match services.full_value(&value) {
        Ok(value) => ExprAnswer::Value(value),
        Err(reason) => ExprAnswer::Declined(reason),
    }
}
